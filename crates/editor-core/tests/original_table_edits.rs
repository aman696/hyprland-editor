use hyprland_editor_core::{
    lua::{
        config_inspection,
        replay::{
            scalar_edit::{self, Change, Selection},
            *,
        },
        source_hash,
    },
    workspace_graph::revisions,
    Value, AUDITED_MAIN, BASELINE,
};
fn program(version: &str, source: &str) -> Program {
    Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            source: source.into(),
            sha256: source_hash(source),
            load_name: b"/fixture/main.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn change(p: &Program) -> Change {
    let report = replay(p).unwrap();
    let inspection = config_inspection::inspect(&report.events, &p.version).unwrap();
    let c = inspection
        .batches
        .iter()
        .flat_map(|b| &b.candidates)
        .find(|c| c.setting_id == "general:gaps_out")
        .unwrap();
    Change {
        event_order: c.event_order,
        location: c.location.clone(),
        setting_id: c.setting_id.clone(),
        selection: Selection::OriginalTableLayout,
        value: Value::CssGap([9, 2, 3, 4]),
    }
}
#[test]
fn original_table_selection_is_explicit_and_preserves_definition_comments_and_alias_reads() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p=program(version,"local gaps={top=1 --[[memo]],right=2,bottom=3,left=4,note='keep'};local alias=gaps;hl.config({general={gaps_out=alias}})");
        let mut request = change(&p);
        request.selection = Selection::PreserveTableLayout;
        assert!(scalar_edit::compare(&p, &request).is_err());
        request.selection = Selection::OriginalTableLayout;
        let compared = scalar_edit::compare(&p, &request).unwrap();
        assert!(compared.table_layout_preserved);
        assert!(compared.review.modeled_observations_match);
        assert!(!compared.review.ownership_verified && !compared.review.runtime_verified);
        let pieces = revisions::apply(&p, &[], &compared.plan).unwrap();
        let after = revisions::program(p.clone(), &pieces).unwrap();
        assert_eq!(
            after.documents[0].source,
            p.documents[0].source.replace("top=1", "top=9")
        );
    }
}
#[test]
fn identical_tables_and_overwritten_aliases_select_the_reached_definition_by_provenance() {
    for version in [BASELINE, AUDITED_MAIN] {
        for alias in [
            "local selected=second",
            "local selected=first;selected=second",
            "local function take()return second end;local selected=take()",
        ] {
            let source=format!("local first={{top=1,right=2,bottom=3,left=4}};local second={{top=1,right=2,bottom=3,left=4}};{alias};hl.config({{general={{gaps_out=selected}}}})");
            let p = program(version, &source);
            let c = scalar_edit::compare(&p, &change(&p)).unwrap();
            assert!(c.review.modeled_observations_match);
            let span = c.plan.edits[0].span.clone();
            assert!(span.start > source.find("local second=").unwrap());
            let after = revisions::program(p.clone(), &revisions::apply(&p, &[], &c.plan).unwrap())
                .unwrap();
            assert!(after.documents[0]
                .source
                .contains("local first={top=1,right=2,bottom=3,left=4}"));
            assert!(after.documents[0]
                .source
                .contains("local second={top=9,right=2,bottom=3,left=4}"));
        }
    }
}
#[test]
fn shared_definition_changes_report_every_additional_reached_use() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p=program(version,"local gaps={top=1,right=2,bottom=3,left=4};hl.config({general={gaps_out=gaps}});hl.config({general={gaps_out=gaps}})");
        let c = scalar_edit::compare(&p, &change(&p)).unwrap();
        assert!(!c.review.modeled_observations_match);
        assert_eq!(c.review.expected_calls_matched, 1);
        assert!(c
            .review
            .differences
            .iter()
            .any(|d| d.kind == edits::DifferenceKind::Arguments && d.order == Some(1)));
        assert!(!c.review.ownership_verified);
        assert!(p.documents[0].source.contains("top=1"));
    }
}
#[test]
fn captured_module_table_edits_keep_main_bytes_and_historical_resolution() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = program(
            version,
            "local gaps=require('theme');hl.config({general={gaps_out=gaps}})",
        );
        let source =
            "-- module é\r\nreturn {top=1 --[[memo]],right=2,bottom=3,left=4,note='stay'}\r\n";
        p.documents.push(Document {
            id: "theme".into(),
            source: source.into(),
            sha256: source_hash(source),
            load_name: b"/fixture/theme.lua".to_vec(),
        });
        let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
            panic!("Expected exact module request")
        };
        p.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: "theme".into(),
                loader_data: b"/fixture/theme.lua".to_vec(),
            },
        });
        let c = scalar_edit::compare(&p, &change(&p)).unwrap();
        assert!(
            c.review.modeled_observations_match,
            "{:?}",
            c.review.differences
        );
        assert_eq!(c.plan.edits[0].document, "theme");
        let pieces = revisions::apply(&p, &[], &c.plan).unwrap();
        let after = revisions::program(p.clone(), &pieces).unwrap();
        assert_eq!(after.documents[0].source, p.documents[0].source);
        assert_eq!(after.documents[1].source, source.replace("top=1", "top=9"));
        assert_eq!(
            serde_json::to_vec(&after.bindings).unwrap(),
            serde_json::to_vec(&p.bindings).unwrap()
        );
        assert!(replay(&after).unwrap().halt.is_none());
    }
}
#[test]
fn constructor_selection_refuses_external_fields_and_reports_later_overwrite_conflicts() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            version,
            "local gaps={top=1};gaps.right=2;hl.config({general={gaps_out=gaps}})",
        );
        assert!(scalar_edit::compare(&p, &change(&p))
            .unwrap_err()
            .contains("fields created outside"));
        let p=program(version,"local gaps={top=1,right=2,bottom=3,left=4};gaps.top=2;hl.config({general={gaps_out=gaps}})");
        let c = scalar_edit::compare(&p, &change(&p)).unwrap();
        assert!(!c.review.modeled_observations_match);
        assert!(c
            .review
            .differences
            .iter()
            .any(|d| d.kind == edits::DifferenceKind::Arguments));
        assert!(!c.review.ownership_verified);
    }
}
