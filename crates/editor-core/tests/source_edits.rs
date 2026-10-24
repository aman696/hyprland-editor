use hyprland_editor_core::{
    lua::{
        replay::{edits::*, *},
        source_hash,
    },
    AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES,
};
fn program(source: &str) -> Program {
    Program {
        version: BASELINE.into(),
        profile: Profile::SyntheticEmit,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            source: source.into(),
            sha256: source_hash(source),
            load_name: b"/fixture/root.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn edit(p: &Program, id: &str, needle: &str, replacement: &str) -> SourceEdit {
    let d = p.documents.iter().find(|d| d.id == id).unwrap();
    let start = d.source.find(needle).unwrap();
    SourceEdit {
        document: id.into(),
        base_sha256: d.sha256.clone(),
        span: start..start + needle.len(),
        old_bytes_sha256: source_hash(needle),
        replacement: replacement.into(),
        semantic_targets: vec!["fixture.value".into()],
    }
}
fn expected(order: usize, before: Observed, after: Observed) -> ExpectedCall {
    ExpectedCall {
        order,
        before_arguments: vec![before],
        after_arguments: vec![after],
    }
}
#[test]
fn private_patch_preserves_utf8_comments_line_endings_and_unaffected_calls() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p =
            program("-- café\r\nlocal value = 2 -- keep\r\nemit(value);emit('unaffected')\r\n");
        p.version = version.into();
        let before = serde_json::to_vec(&p).unwrap();
        let plan = prepare(
            &p,
            vec![edit(&p, "root", "2", "900")],
            vec![expected(0, Observed::Int(2), Observed::Int(900))],
        )
        .unwrap();
        let r = review(&p, &plan).unwrap();
        assert!(r.modeled_observations_match, "{:?}", r.differences);
        assert_eq!(r.expected_calls_matched, 1);
        assert_eq!(
            r.revisions[0].content,
            p.documents[0].source.replacen('2', "900", 1)
        );
        assert_eq!(r.revisions[0].base_sha256, p.documents[0].sha256);
        assert_eq!(
            r.revisions[0].proposed_sha256,
            source_hash(&r.revisions[0].content)
        );
        assert!(!r.ownership_verified && !r.runtime_verified);
        assert_eq!(serde_json::to_vec(&p).unwrap(), before);
    }
}
#[test]
fn shared_values_unexpected_arguments_and_path_changes_are_visible_instead_of_authorized() {
    let p = program("local x=2;emit(x);emit(x)");
    let plan = prepare(
        &p,
        vec![edit(&p, "root", "2", "7")],
        vec![expected(0, Observed::Int(2), Observed::Int(7))],
    )
    .unwrap();
    let r = review(&p, &plan).unwrap();
    assert!(!r.modeled_observations_match);
    assert!(r
        .differences
        .iter()
        .any(|d| d.kind == DifferenceKind::Arguments && d.order == Some(1)));
    let change = r
        .differences
        .iter()
        .find(|d| d.kind == DifferenceKind::Arguments)
        .unwrap();
    assert_eq!(change.before_arguments, Some(vec![Observed::Int(2)]));
    assert_eq!(change.after_arguments, Some(vec![Observed::Int(7)]));
    let p = program("local x=2;if x==2 then emit(1)else emit(1)end");
    let r = review(
        &p,
        &prepare(&p, vec![edit(&p, "root", "2", "7")], vec![]).unwrap(),
    )
    .unwrap();
    assert!(!r.modeled_observations_match);
    assert!(r
        .differences
        .iter()
        .any(|d| d.kind == DifferenceKind::PathHistory));
    assert!(r
        .differences
        .iter()
        .any(|d| d.kind == DifferenceKind::CallSiteOrApi));
}
#[test]
fn source_graph_edits_rebase_only_unchanged_recorded_loader_sites_and_keep_logical_ids() {
    let mut p = program("local value=require('part');emit(value)");
    let module = "-- dependency\nreturn 3\n";
    p.documents.push(Document {
        id: "part".into(),
        source: module.into(),
        sha256: source_hash(module),
        load_name: b"/fixture/part.lua".to_vec(),
    });
    let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
        panic!("No request")
    };
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: b"/fixture/part.lua".to_vec(),
        },
    });
    let plan = prepare(
        &p,
        vec![
            edit(&p, "root", "", "-- inserted prefix\n"),
            edit(&p, "part", "3", "12"),
        ],
        vec![expected(1, Observed::Int(3), Observed::Int(12))],
    )
    .unwrap();
    let r = review(&p, &plan).unwrap();
    assert!(r.modeled_observations_match, "{:?}", r.differences);
    assert_eq!(r.revisions.len(), 2);
    assert_eq!(r.revisions[0].document, "root");
    assert_eq!(r.revisions[1].document, "part");
    let plan = prepare(&p, vec![edit(&p, "root", "'part'", "'other'")], vec![]).unwrap();
    assert!(review(&p, &plan).unwrap_err().contains("loader request"));
    let plan = prepare(&p, vec![edit(&p, "root", "'part'", "'part'..''")], vec![]).unwrap();
    assert!(
        review(&p, &plan).is_err(),
        "Even apparently equal loader edits need new resolution evidence"
    );
}
#[test]
fn stale_program_schema_compiler_old_bytes_and_noncanonical_or_overlapping_ranges_are_refused() {
    let p = program("local x=2;emit(x)");
    let plan = prepare(
        &p,
        vec![edit(&p, "root", "2", "9")],
        vec![expected(0, Observed::Int(2), Observed::Int(9))],
    )
    .unwrap();
    for field in ["program", "schema", "compiler", "base", "old", "document"] {
        let mut bad = plan.clone();
        match field {
            "program" => bad.base_program_sha256 = source_hash("other"),
            "schema" => bad.schema_sha256 = source_hash("other"),
            "compiler" => bad.compiler.library_identity = "other".into(),
            "base" => bad.edits[0].base_sha256 = source_hash("other"),
            "old" => bad.edits[0].old_bytes_sha256 = source_hash("other"),
            _ => bad.edits[0].document = "foreign".into(),
        }
        assert!(review(&p, &bad).is_err(), "{field}");
    }
    let mut changed = p.clone();
    changed.working_directory = Some(b"/other".to_vec());
    assert!(review(&changed, &plan).is_err());
    let repeated = vec![edit(&p, "root", "2", "9"), edit(&p, "root", "2", "4")];
    assert!(prepare(&p, repeated, vec![]).is_err());
    let plan = prepare(
        &p,
        vec![
            edit(&p, "root", "2", "9"),
            edit(&p, "root", "emit", "emit "),
        ],
        vec![],
    )
    .unwrap();
    let mut bad = plan.clone();
    bad.edits.reverse();
    assert!(review(&p, &bad).is_err());
    let p = program("-- café\nemit(2)");
    let mut e = edit(&p, "root", "é", "x");
    e.span.start += 1;
    assert!(prepare(&p, vec![e], vec![]).is_err());
}
#[test]
fn syntax_failure_in_an_unreached_document_and_new_effects_never_pass_review() {
    let mut p = program("emit(2)");
    let source = "return 4";
    p.documents.push(Document {
        id: "unused".into(),
        source: source.into(),
        sha256: source_hash(source),
        load_name: b"/fixture/unused.lua".to_vec(),
    });
    let plan = prepare(&p, vec![edit(&p, "unused", "4", "(")], vec![]).unwrap();
    assert!(review(&p, &plan).unwrap_err().contains("compilation"));
    let p = program("emit(2)");
    let plan = prepare(&p, vec![edit(&p, "root", "2", "2);emit(3")], vec![]).unwrap();
    let r = review(&p, &plan).unwrap();
    assert!(!r.modeled_observations_match);
    assert!(r
        .differences
        .iter()
        .any(|d| d.kind == DifferenceKind::AddedCall));
    let plan = prepare(
        &p,
        vec![edit(&p, "root", "2", "2);os.execute('never')--")],
        vec![],
    )
    .unwrap();
    let r = review(&p, &plan).unwrap();
    assert!(!r.after_model_complete);
    assert!(r.after_halt.is_some());
    assert!(!r.modeled_observations_match);
}
#[test]
fn insertion_deletion_negative_zero_and_partial_models_keep_their_distinctions() {
    let p = program("-- owned comment\nemit(0.0)");
    let plan = prepare(
        &p,
        vec![
            edit(&p, "root", "-- owned comment\n", ""),
            edit(&p, "root", "0.0", "-0.0"),
        ],
        vec![expected(0, Observed::Float(0.0), Observed::Float(-0.0))],
    )
    .unwrap();
    let r = review(&p, &plan).unwrap();
    assert!(r.modeled_observations_match, "{:?}", r.differences);
    assert_eq!(r.revisions[0].content, "emit(-0.0)");
    let plan = prepare(&p, vec![edit(&p, "root", "0.0", "-0.0")], vec![]).unwrap();
    assert!(!review(&p, &plan).unwrap().modeled_observations_match);
    let p = program("emit(2);os.execute('not executed')");
    let plan = prepare(
        &p,
        vec![edit(&p, "root", "2", "3")],
        vec![expected(0, Observed::Int(2), Observed::Int(3))],
    )
    .unwrap();
    let r = review(&p, &plan).unwrap();
    assert!(!r.before_model_complete && !r.after_model_complete);
    assert!(!r.modeled_observations_match);
    assert!(r.before_halt.is_some());
}
#[test]
fn patch_size_expectation_depth_and_argument_preconditions_have_bounded_failures() {
    let p = program("emit(2)");
    let e = edit(&p, "root", "2", "x");
    let mut big = e.clone();
    big.replacement = " ".repeat(MAX_DOCUMENT_BYTES + 1);
    assert!(prepare(&p, vec![big], vec![]).is_err());
    let mut deep = Observed::Int(1);
    for _ in 0..66 {
        deep = Observed::Table(vec![(Observed::Int(1), deep)]);
    }
    assert!(prepare(
        &p,
        vec![e.clone()],
        vec![expected(0, deep, Observed::Int(3))]
    )
    .is_err());
    let plan = prepare(
        &p,
        vec![edit(&p, "root", "2", "3")],
        vec![expected(0, Observed::Int(99), Observed::Int(3))],
    )
    .unwrap();
    assert!(review(&p, &plan).is_err());
    let mut insertion = edit(&p, "root", "", "-- prefix\n");
    let mut same = insertion.clone();
    same.replacement = "-- other\n".into();
    assert!(prepare(&p, vec![insertion.clone(), same], vec![]).is_err());
    insertion.replacement.push('\0');
    assert!(prepare(&p, vec![insertion], vec![]).is_err());
}

#[test]
fn unchanged_modeled_calls_do_not_certify_unexecuted_callable_effects() {
    let p = program("emit(2)");
    let d = &p.documents[0];
    let e = SourceEdit {
        document: d.id.clone(),
        base_sha256: d.sha256.clone(),
        span: d.source.len()..d.source.len(),
        old_bytes_sha256: source_hash(""),
        replacement: ";local function later()os.execute('never executed')end".into(),
        semantic_targets: vec!["fixture.future".into()],
    };
    let r = review(&p, &prepare(&p, vec![e], vec![]).unwrap()).unwrap();
    assert!(r.modeled_observations_match);
    assert!(
        !r.ownership_verified && !r.runtime_verified,
        "Matching executed observations is insufficient for future-effect ownership"
    );
}

#[test]
fn changed_computed_loader_context_does_not_reuse_a_different_modules_resolution() {
    let mut p = program("local name='part';local value=require(name);emit(value)");
    let source = "return 4";
    p.documents.push(Document {
        id: "part".into(),
        source: source.into(),
        sha256: source_hash(source),
        load_name: b"/fixture/part.lua".to_vec(),
    });
    let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
        panic!("No request")
    };
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: b"/fixture/part.lua".to_vec(),
        },
    });
    let r = review(
        &p,
        &prepare(&p, vec![edit(&p, "root", "'part'", "'other'")], vec![]).unwrap(),
    )
    .unwrap();
    assert!(!r.modeled_observations_match);
    assert!(
        matches!(r.after_halt,Some(Halt::NeedsResolution{ref request}) if request.module==b"other")
    );
    assert!(r
        .differences
        .iter()
        .any(|d| d.kind == DifferenceKind::Loader));
    let mut unreachable = p.clone();
    unreachable.documents[0].source = "emit(4) -- require(name)".into();
    unreachable.documents[0].sha256 = source_hash(&unreachable.documents[0].source);
    unreachable.bindings[0].request.sha256 = unreachable.documents[0].sha256.clone();
    unreachable.bindings[0].request.span = 11..24;
    let plan = prepare(
        &unreachable,
        vec![edit(&unreachable, "root", "4", "5")],
        vec![],
    )
    .unwrap();
    assert!(review(&unreachable, &plan)
        .unwrap_err()
        .contains("not reached"));
}
