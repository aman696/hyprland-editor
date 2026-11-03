use hyprland_editor_core::{
    lua::{
        self,
        replay::{
            self, declaration_edit::submap, edits::DifferenceKind, Binding, Document, Profile,
            Program, Resolution,
        },
    },
    AUDITED_MAIN, BASELINE,
};
fn document(id: &str, source: &str) -> Document {
    Document {
        id: id.into(),
        source: source.into(),
        sha256: lua::source_hash(source),
        load_name: format!("/config/{id}").into_bytes(),
    }
}
fn program(source: &str, version: &str) -> Program {
    Program {
        version: version.into(),
        profile: if cfg!(feature = "native55") {
            Profile::HyprlandLua55
        } else {
            Profile::HyprlandLua54
        },
        root: "main.lua".into(),
        documents: vec![document("main.lua", source)],
        bindings: vec![],
        initial_package_path: Some(b"/config/?.lua".to_vec()),
        initial_package_path_unknown_tail: false,
        working_directory: Some(b"/config".to_vec()),
    }
}
#[test]
fn header_edits_keep_callbacks_module_loads_comments_and_mixed_declarations() {
    for version in [BASELINE, AUDITED_MAIN] {
        for reset in [None, Some("reset".to_string()), Some(String::new())] {
            let source="-- header\r\nhl.define_submap( -- name note\r\n'demo', -- callback note\r\nfunction()local part=require('part');hl.bind('Q',hl.dsp.no_op());hl.config{general={border_size=part}}end);hl.config{general={gaps_in=3}}";
            let mut p = program(source, version);
            p.documents
                .push(document("part.lua", "hl.device{name='mouse'};return 2"));
            let request = match replay::replay(&p).unwrap().halt.unwrap() {
                replay::Halt::NeedsResolution { request } => request,
                other => panic!("{other:?}"),
            };
            p.bindings.push(Binding {
                request,
                resolution: Resolution::Loaded {
                    document: "part.lua".into(),
                    loader_data: vec![],
                },
            });
            let old = submap::inspect(&p, 0).unwrap();
            assert_eq!(old.name, "demo");
            assert_eq!(old.reset, None);
            assert_eq!(old.occurrences, 1);
            let callback = &source[old.callback_span.clone()];
            let result = submap::compare(&p, 0, "renamed".into(), reset.clone()).unwrap();
            assert_eq!(result.review.expected_calls_matched, 1);
            assert!(result.review.before_model_complete && result.review.after_model_complete);
            assert!(
                result
                    .review
                    .differences
                    .iter()
                    .all(|d| d.kind == DifferenceKind::RegistrationScope),
                "{:?}",
                result.review.differences
            );
            assert!(!result.review.differences.is_empty());
            let revised = &result.review.revisions[0].content;
            assert!(revised.contains(callback));
            assert_eq!(revised.matches("-- callback note").count(), 1);
            assert_eq!(revised.matches("-- name note").count(), 1);
            assert!(revised.starts_with("-- header\r\n"));
            assert!(revised.ends_with(";hl.config{general={gaps_in=3}}"));
            let patch = &result.plan.edits[0];
            assert_eq!(patch.span.end, old.callback_span.start);
            assert!(patch.span.start < patch.span.end);
            assert_eq!(p.documents[0].source, source);
        }
    }
}
#[test]
fn named_callbacks_reset_removal_and_repeated_sites_retain_their_source() {
    for version in [BASELINE, AUDITED_MAIN] {
        let source="local callback=function()hl.bind('Q',hl.dsp.no_op())end;hl.define_submap('demo','reset',callback)";
        let p = program(source, version);
        let old = submap::inspect(&p, 0).unwrap();
        assert_eq!(old.reset.as_deref(), Some("reset"));
        let result = submap::compare(&p, 0, "changed".into(), None).unwrap();
        assert!(result.review.revisions[0]
            .content
            .starts_with("local callback=function()hl.bind('Q',hl.dsp.no_op())end;"));
        assert!(result.review.revisions[0].content.ends_with("callback)"));
        let p = program(
            "for i=1,2 do hl.define_submap('demo'..i,function()hl.bind('Q',hl.dsp.no_op())end)end",
            version,
        );
        let old = submap::inspect(&p, 0).unwrap();
        assert_eq!(old.occurrences, 2);
        let result = submap::compare(&p, 0, "shared".into(), Some("reset".into())).unwrap();
        assert_eq!(result.review.expected_calls_matched, 2);
        assert!(result
            .review
            .differences
            .iter()
            .all(|d| d.kind == DifferenceKind::RegistrationScope));
    }
}
#[test]
fn invalid_unchanged_and_expanded_headers_are_refused_and_unknown_callbacks_stay_partial() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            "hl.define_submap('demo',function()os.execute('never')end)",
            version,
        );
        let header = submap::inspect(&p, 0).unwrap();
        assert_eq!(header.name, "demo");
        for (name, reset) in [
            ("demo".to_string(), None),
            (String::new(), None),
            ("a\0b".into(), None),
            ("a".into(), Some("x".repeat(4097))),
        ] {
            assert!(submap::compare(&p, 0, name, reset).is_err());
        }
        let result = submap::compare(&p, 0, "changed".into(), None).unwrap();
        assert!(!result.review.before_model_complete && !result.review.after_model_complete);
        assert!(result.review.revisions[0]
            .content
            .contains("function()os.execute('never')end"));
        let p = program(
            "local function headers()return 'demo',function()end end;hl.define_submap(headers())",
            version,
        );
        assert!(submap::inspect(&p, 0).is_err());
    }
}

#[test]
fn empty_captured_names_can_be_repaired_without_rebuilding_the_callback() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            "hl.define_submap('',function()hl.device{name='mouse'}end)",
            version,
        );
        assert_eq!(submap::inspect(&p, 0).unwrap().name, "");
        let result = submap::compare(&p, 0, "repaired".into(), None).unwrap();
        assert!(result.review.revisions[0]
            .content
            .ends_with("function()hl.device{name='mouse'}end)"));
    }
}
