use hyprland_editor_core::{
    lua::{
        config_inspection,
        replay::{scalar_edit::*, *},
        source_hash,
    },
    Value, AUDITED_MAIN, BASELINE,
};
fn program(source: &str) -> Program {
    Program {
        version: BASELINE.into(),
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
fn change(p: &Program, id: &str, value: Value, selection: Selection) -> Change {
    let report = replay(p).unwrap();
    let inspection = config_inspection::inspect(&report.events, &p.version).unwrap();
    let c = inspection
        .batches
        .iter()
        .flat_map(|b| &b.candidates)
        .find(|c| c.setting_id == id)
        .unwrap();
    Change {
        event_order: c.event_order,
        location: c.location.clone(),
        setting_id: id.into(),
        selection,
        value,
    }
}
#[test]
fn selected_read_and_original_shared_literal_have_distinct_source_and_impact() {
    let p=program("local size=2;hl.config({general={border_size=size}});hl.config({general={border_size=size}})");
    let selected = compare(
        &p,
        &change(
            &p,
            "general:border_size",
            Value::Int(4),
            Selection::SelectedExpression,
        ),
    )
    .unwrap();
    assert!(selected.review.modeled_observations_match);
    assert_eq!(
        &p.documents[0].source[selected.plan.edits[0].span.clone()],
        "size"
    );
    let original = compare(
        &p,
        &change(
            &p,
            "general:border_size",
            Value::Int(4),
            Selection::OriginalLiteral,
        ),
    )
    .unwrap();
    assert!(!original.review.modeled_observations_match);
    assert_eq!(
        &p.documents[0].source[original.plan.edits[0].span.clone()],
        "2"
    );
    assert_eq!(
        original.review.differences[0].kind,
        edits::DifferenceKind::Arguments
    );
    assert!(!selected.review.ownership_verified && !selected.review.runtime_verified);
}
#[test]
fn computed_expression_replacement_is_explicit_and_never_guesses_an_original_operand() {
    let p = program("local size=2;hl.config({general={border_size=size+1}})");
    assert!(
        compare(
            &p,
            &change(
                &p,
                "general:border_size",
                Value::Int(8),
                Selection::SelectedExpression
            )
        )
        .unwrap()
        .review
        .modeled_observations_match
    );
    assert!(compare(
        &p,
        &change(
            &p,
            "general:border_size",
            Value::Int(8),
            Selection::OriginalLiteral
        )
    )
    .unwrap_err()
    .contains("no single original"));
}
#[test]
fn primitive_printers_preserve_float_kind_signed_zero_integer_min_and_string_data() {
    for version in [BASELINE, AUDITED_MAIN] {
        for (source, id, value) in [
            (
                "hl.config({general={allow_tearing=false}})",
                "general:allow_tearing",
                Value::Bool(true),
            ),
            (
                "hl.config({decoration={rounding_power=2.0}})",
                "decoration:rounding_power",
                Value::Float(3.0),
            ),
            (
                "hl.config({input={sensitivity=0.0}})",
                "input:sensitivity",
                Value::Float(-0.0),
            ),
            (
                "hl.config({general={layout='dwindle'}})",
                "general:layout",
                Value::String("\";os.execute('never') --\n\\é".into()),
            ),
        ] {
            let mut p = program(source);
            p.version = version.into();
            let result =
                compare(&p, &change(&p, id, value, Selection::SelectedExpression)).unwrap();
            assert!(
                result.review.modeled_observations_match,
                "{:?}",
                result.review.differences
            );
        }
    }
    let p = program("hl.config({general={border_size=2}})");
    let result = compare(
        &p,
        &change(
            &p,
            "general:border_size",
            Value::Int(i64::MIN),
            Selection::SelectedExpression,
        ),
    )
    .unwrap();
    assert!(result.review.modeled_observations_match);
    assert_eq!(
        result.plan.edits[0].replacement,
        "(-9223372036854775807 - 1)"
    );
}
#[test]
fn native_version_constraints_wrong_types_nonfinite_and_unknown_coordinates_are_not_bypassed() {
    let mut p = program("hl.config({general={border_size=2}})");
    let request = change(
        &p,
        "general:border_size",
        Value::Int(21),
        Selection::SelectedExpression,
    );
    assert!(
        compare(&p, &request)
            .unwrap()
            .review
            .modeled_observations_match
    );
    p.version = AUDITED_MAIN.into();
    assert!(compare(&p, &request).is_err());
    let mut bad = request.clone();
    bad.value = Value::String("4".into());
    assert!(compare(&p, &bad).is_err());
    bad = request.clone();
    bad.setting_id = "foreign".into();
    assert!(compare(&p, &bad).is_err());
    bad = request.clone();
    bad.location = vec![999];
    assert!(compare(&p, &bad).is_err());
    let p = program("hl.config({input={sensitivity=0.0}})");
    for value in [f64::NAN, f64::INFINITY, 1e300] {
        let request = change(
            &p,
            "input:sensitivity",
            Value::Float(value),
            Selection::SelectedExpression,
        );
        assert!(compare(&p, &request).is_err());
    }
    let p = program("hl.config({general={layout='dwindle'}})");
    assert!(compare(
        &p,
        &change(
            &p,
            "general:layout",
            Value::String("a\0b".into()),
            Selection::SelectedExpression
        )
    )
    .is_err());
}
#[test]
fn a_cross_document_original_literal_exposes_changed_branch_effects() {
    let mut p = program(
        "local part=require('part');if part==4 then hl.config({general={border_size=part}})end",
    );
    let source = "return 4";
    p.documents.push(Document {
        id: "part".into(),
        source: source.into(),
        sha256: source_hash(source),
        load_name: b"/fixture/part.lua".to_vec(),
    });
    let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
        panic!("Missing request")
    };
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: b"/fixture/part.lua".to_vec(),
        },
    });
    let local = compare(
        &p,
        &change(
            &p,
            "general:border_size",
            Value::Int(6),
            Selection::SelectedExpression,
        ),
    )
    .unwrap();
    assert_eq!(local.plan.edits[0].document, "root");
    assert!(local.review.modeled_observations_match);
    let shared = compare(
        &p,
        &change(
            &p,
            "general:border_size",
            Value::Int(6),
            Selection::OriginalLiteral,
        ),
    )
    .unwrap();
    assert_eq!(shared.plan.edits[0].document, "part");
    assert!(!shared.review.modeled_observations_match);
    assert!(shared
        .review
        .differences
        .iter()
        .any(|d| d.kind == edits::DifferenceKind::PathHistory));
}
