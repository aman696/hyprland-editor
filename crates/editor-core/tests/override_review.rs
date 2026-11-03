use hyprland_editor_core::{
    lua::{
        replay::{
            override_review::{self, *},
            Document, Profile, Program,
        },
        source_hash,
    },
    workspace_graph::revisions,
    Draft, Value, AUDITED_MAIN, BASELINE,
};
use std::collections::BTreeMap;
fn program(source: &str, version: &str) -> Program {
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
fn draft(version: &str, values: &[(&str, Value)]) -> Draft {
    let mut d = Draft::new(version);
    d.values = values
        .iter()
        .map(|(id, v)| (id.to_string(), v.clone()))
        .collect();
    d
}
fn choose(id: &str, choice: Choice) -> BTreeMap<String, Choice> {
    [(id.into(), choice)].into()
}
fn materialize(p: &Program, r: &override_review::Resolution) -> Program {
    let mut pieces = vec![];
    for plan in &r.plans {
        pieces = revisions::prepare_pieces(p, &pieces, plan).unwrap();
    }
    revisions::program(p.clone(), &pieces).unwrap()
}
#[test]
fn each_layer_stays_private_until_an_explicit_choice_and_originals_stay_exact() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            "-- original\r\nhl.config({general={border_size=6}})\r\n",
            version,
        );
        let d = draft(version, &[("general:border_size", Value::Int(9))]);
        let review = prepare(&p, &d).unwrap();
        assert!(review.model_complete);
        assert!(matches!(
            review.rows[0].source,
            SourceValue::Parsed {
                value: Value::Int(6),
                provisional: false,
                ..
            }
        ));
        let unchanged = resolve(&p, &d, &review, &BTreeMap::new()).unwrap();
        assert!(unchanged.plans.is_empty());
        assert_eq!(unchanged.draft.values, d.values);
        assert_eq!(unchanged.remaining, 1);
        let use_source = resolve(
            &p,
            &d,
            &review,
            &choose("general:border_size", Choice::UseSource),
        )
        .unwrap();
        assert!(use_source.plans.is_empty());
        assert!(use_source.draft.values.is_empty());
        let write = resolve(
            &p,
            &d,
            &review,
            &choose("general:border_size", Choice::WriteOverride),
        )
        .unwrap();
        assert_eq!(write.plans.len(), 1);
        assert!(write.draft.values.is_empty());
        assert!(!write.ownership_verified && !write.runtime_verified);
        assert_eq!(
            materialize(&p, &write).documents[0].source,
            "-- original\r\nhl.config({general={border_size=9}})\r\n"
        );
        assert!(p.documents[0].source.contains("border_size=6"));
        assert_eq!(d.values["general:border_size"], Value::Int(9));
    }
}
#[test]
fn incomplete_analysis_never_substitutes_defaults_or_claims_absence() {
    let d = draft(
        BASELINE,
        &[
            ("general:border_size", Value::Int(5)),
            ("general:allow_tearing", Value::Bool(true)),
        ],
    );
    let p = program("hl.config({general={border_size=2}})", BASELINE);
    let r = prepare(&p, &d).unwrap();
    assert!(matches!(
        r.rows
            .iter()
            .find(|r| r.setting_id == "general:allow_tearing")
            .unwrap()
            .source,
        SourceValue::Absent
    ));
    assert!(resolve(
        &p,
        &d,
        &r,
        &choose("general:allow_tearing", Choice::WriteOverride)
    )
    .is_err());
    for source in [
        "require('uncaptured')",
        "hl.config({general={border_size=2}});os.execute('never')",
        "hl.config({unknown_setting=2})",
    ] {
        let p = program(source, BASELINE);
        let r = prepare(&p, &d).unwrap();
        assert!(!r.model_complete);
        let absent = r
            .rows
            .iter()
            .find(|r| r.setting_id == "general:allow_tearing")
            .unwrap();
        assert!(matches!(absent.source, SourceValue::Unresolved { .. }));
        let removed = resolve(
            &p,
            &d,
            &r,
            &choose("general:allow_tearing", Choice::UseSource),
        )
        .unwrap();
        assert_eq!(removed.remaining, 1);
        assert!(!removed.model_complete);
        assert!(removed.plans.is_empty());
    }
    let d = draft(BASELINE, &[("general:border_size", Value::Int(2))]);
    let p = program(
        "hl.config({general={border_size=2}});os.execute('never')",
        BASELINE,
    );
    assert!(resolve(
        &p,
        &d,
        &prepare(&p, &d).unwrap(),
        &choose("general:border_size", Choice::WriteOverride)
    )
    .is_err());
}
#[test]
fn sequential_plans_track_shifted_spans_and_choose_the_last_modeled_declaration() {
    let p=program("hl.config({general={border_size=2,allow_tearing=false}});hl.config({general={border_size=3}})",BASELINE);
    let d = draft(
        BASELINE,
        &[
            ("general:border_size", Value::Int(8)),
            ("general:allow_tearing", Value::Bool(true)),
        ],
    );
    let r = prepare(&p, &d).unwrap();
    assert_eq!(
        r.rows
            .iter()
            .find(|r| r.setting_id == "general:border_size")
            .unwrap()
            .declaration_count,
        2
    );
    let choices = d
        .values
        .keys()
        .map(|id| (id.clone(), Choice::WriteOverride))
        .collect();
    let resolved = resolve(&p, &d, &r, &choices).unwrap();
    assert_eq!(resolved.plans.len(), 2);
    assert_eq!(materialize(&p,&resolved).documents[0].source,"hl.config({general={border_size=2,allow_tearing=true}});hl.config({general={border_size=8}})");
    assert!(resolved.draft.values.is_empty());
    assert!(resolved.model_complete);
}
#[test]
fn equality_uses_native_float_storage_and_preserves_the_sign_of_zero() {
    for (source, value, plans) in [("0.1", 0.1, 0), ("0.0", -0.0, 1), ("-0.0", -0.0, 0)] {
        let p = program(
            &format!("hl.config({{input={{sensitivity={source}}}}})"),
            BASELINE,
        );
        let d = draft(BASELINE, &[("input:sensitivity", Value::Float(value))]);
        let r = resolve(
            &p,
            &d,
            &prepare(&p, &d).unwrap(),
            &choose("input:sensitivity", Choice::WriteOverride),
        )
        .unwrap();
        assert_eq!(r.plans.len(), plans);
        assert!(r.draft.values.is_empty());
        let q = materialize(&p, &r);
        let check = draft(BASELINE, &[("input:sensitivity", Value::Float(value))]);
        let source = prepare(&q, &check).unwrap().rows.remove(0).source;
        let SourceValue::Parsed {
            native: hyprland_editor_core::lua::scalar_codec::NativeValue::FloatBits(bits),
            ..
        } = source
        else {
            panic!("Missing float")
        };
        assert_eq!(bits, (value as f32).to_bits());
    }
}
#[test]
fn shared_source_effects_and_unmodeled_codecs_are_not_silently_folded() {
    let d = draft(BASELINE, &[("general:border_size", Value::Int(8))]);
    let p = program(
        "local t={general={border_size=2}};hl.config(t);hl.config(t)",
        BASELINE,
    );
    let review = prepare(&p, &d).unwrap();
    assert!(resolve(
        &p,
        &d,
        &review,
        &choose("general:border_size", Choice::WriteOverride)
    )
    .unwrap_err()
    .contains("other modeled effects"));
    let p = program("hl.config({general={border_size='bad'}})", BASELINE);
    let review = prepare(&p, &d).unwrap();
    assert!(matches!(
        review.rows[0].source,
        SourceValue::Unresolved { .. }
    ));
    let fixed = resolve(
        &p,
        &d,
        &review,
        &choose("general:border_size", Choice::WriteOverride),
    )
    .unwrap();
    assert_eq!(
        materialize(&p, &fixed).documents[0].source,
        "hl.config({general={border_size=8}})"
    );
}
#[test]
fn stale_or_forged_review_program_draft_version_and_unknown_choices_are_refused() {
    let p = program("hl.config({general={border_size=2}})", BASELINE);
    let d = draft(BASELINE, &[("general:border_size", Value::Int(8))]);
    let r = prepare(&p, &d).unwrap();
    for case in 0..5 {
        let mut wrong = r.clone();
        match case {
            0 => wrong.rows[0].can_write = false,
            1 => wrong.rows[0].override_value = Value::Int(2),
            2 => wrong.compiler.library_identity = "foreign".into(),
            3 => wrong.rows.clear(),
            _ => wrong.program_sha256 = "0".repeat(64),
        }
        assert!(resolve(&p, &d, &wrong, &BTreeMap::new()).is_err());
    }
    let other = program("hl.config({general={border_size=3}})", BASELINE);
    assert!(resolve(&other, &d, &r, &BTreeMap::new()).is_err());
    let mut other = d.clone();
    other
        .values
        .insert("general:border_size".into(), Value::Int(9));
    assert!(resolve(&p, &other, &r, &BTreeMap::new()).is_err());
    other.version = AUDITED_MAIN.into();
    assert!(prepare(&p, &other).is_err());
    assert!(resolve(&p, &d, &r, &choose("foreign", Choice::KeepSeparate)).is_err());
}
#[test]
fn client_checks_reject_unselected_layer_changes_and_forged_resolution_claims() {
    let p = program("hl.config({general={border_size=2}})", BASELINE);
    let d = draft(
        BASELINE,
        &[
            ("general:border_size", Value::Int(8)),
            ("general:allow_tearing", Value::Bool(true)),
        ],
    );
    let review = prepare(&p, &d).unwrap();
    let choices = choose("general:border_size", Choice::WriteOverride);
    let result = resolve(&p, &d, &review, &choices).unwrap();
    validate_resolution(&d, &review, &choices, &result).unwrap();
    for case in 0..9 {
        let mut bad = result.clone();
        match case {
            0 => bad.draft.values.clear(),
            1 => bad.remaining += 1,
            2 => bad.runtime_verified = true,
            3 => bad.plans[0].edits[0].semantic_targets[0] = "general:allow_tearing".into(),
            4 => bad.plans.push(bad.plans[0].clone()),
            5 => bad.choices_sha256 = "0".repeat(64),
            6 => {
                bad.plans.clear();
                bad.program_sha256 = review.program_sha256.clone();
            }
            7 => bad.plans[0].structural_calls.push(
                hyprland_editor_core::lua::replay::edits::StructuralCall::RemoveSite {
                    document: "root".into(),
                    span: 0..1,
                    api: "hl.monitor".into(),
                },
            ),
            _ => bad.model_complete = false,
        }
        assert!(
            validate_resolution(&d, &review, &choices, &bad).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn successive_cross_document_choices_rebase_captured_loader_requests() {
    use hyprland_editor_core::lua::replay::{self, Binding, Halt};
    let mut p = program(
        "hl.config({general={allow_tearing=false}});require('part')",
        BASELINE,
    );
    let source = "hl.config({general={border_size=2}});return true";
    p.documents.push(Document {
        id: "part".into(),
        source: source.into(),
        sha256: source_hash(source),
        load_name: b"/fixture/part.lua".to_vec(),
    });
    let Some(Halt::NeedsResolution { request }) = replay::replay(&p).unwrap().halt else {
        panic!("Missing request")
    };
    p.bindings.push(Binding {
        request,
        resolution: replay::Resolution::Loaded {
            document: "part".into(),
            loader_data: b"/fixture/part.lua".to_vec(),
        },
    });
    let d = draft(
        BASELINE,
        &[
            ("general:allow_tearing", Value::Bool(true)),
            ("general:border_size", Value::Int(8)),
        ],
    );
    let choices = d
        .values
        .keys()
        .map(|id| (id.clone(), Choice::WriteOverride))
        .collect();
    let resolved = resolve(&p, &d, &prepare(&p, &d).unwrap(), &choices).unwrap();
    let current = materialize(&p, &resolved);
    assert_eq!(resolved.plans.len(), 2);
    assert!(resolved.draft.values.is_empty());
    assert_eq!(
        current.bindings[0].request.span.start,
        p.bindings[0].request.span.start - 1
    );
    assert_eq!(
        current.bindings[0].request.sha256,
        current.documents[0].sha256
    );
    assert!(replay::replay(&current).unwrap().halt.is_none());
    assert!(current.documents[1].source.contains("border_size=8"));
}

#[test]
fn interleaved_monitor_calls_keep_last_scalar_declarations_and_complete_absence_separate() {
    use hyprland_editor_core::lua::replay::replay;
    for version in [BASELINE, AUDITED_MAIN] {
        let p=program("hl.monitor{output='DP-1',transform=1};hl.config{general={border_size=2}};hl.monitor{output='DP-2',scale='auto'};hl.config{general={border_size=3}}",version);
        let d = draft(
            version,
            &[
                ("general:border_size", Value::Int(9)),
                ("general:allow_tearing", Value::Bool(true)),
            ],
        );
        let review = prepare(&p, &d).unwrap();
        assert!(review.model_complete);
        let border = review
            .rows
            .iter()
            .find(|r| r.setting_id == "general:border_size")
            .unwrap();
        assert_eq!(border.declaration_count, 2);
        assert!(matches!(
            border.source,
            SourceValue::Parsed {
                value: Value::Int(3),
                provisional: false,
                ..
            }
        ));
        assert_eq!(border.target.as_ref().unwrap().event_order, 3);
        assert!(matches!(
            review
                .rows
                .iter()
                .find(|r| r.setting_id == "general:allow_tearing")
                .unwrap()
                .source,
            SourceValue::Absent
        ));
        let result = resolve(
            &p,
            &d,
            &review,
            &choose("general:border_size", Choice::WriteOverride),
        )
        .unwrap();
        assert_eq!(result.plans.len(), 1);
        assert_eq!(result.plans[0].expected_calls[0].order, 3);
        assert_eq!(result.draft.values.len(), 1);
        let next = materialize(&p, &result);
        assert_eq!(
            next.documents[0].source,
            p.documents[0]
                .source
                .replacen("border_size=3", "border_size=9", 1)
        );
        let before = replay(&p).unwrap();
        let after = replay(&next).unwrap();
        for i in [0, 2] {
            assert_eq!(
                serde_json::to_value(&before.events[i].arguments).unwrap(),
                serde_json::to_value(&after.events[i].arguments).unwrap()
            );
        }
        let none = program("hl.monitor{output='DP-1'}", version);
        assert!(matches!(
            prepare(
                &none,
                &draft(version, &[("general:border_size", Value::Int(9))])
            )
            .unwrap()
            .rows[0]
                .source,
            SourceValue::Absent
        ));
    }
}
#[test]
fn removing_monitor_argument_effects_or_crossing_unknown_apis_never_reconciles_a_scalar_override() {
    for version in [BASELINE, AUDITED_MAIN] {
        let d = draft(version, &[("general:border_size", Value::Int(9))]);
        let p=program("local function value() hl.monitor{output='DP-1',transform=1};return 2 end;hl.config{general={border_size=value()}}",version);
        let review = prepare(&p, &d).unwrap();
        assert!(review.model_complete);
        assert!(resolve(
            &p,
            &d,
            &review,
            &choose("general:border_size", Choice::WriteOverride)
        )
        .is_err());
        for source in [
            "hl.monitor{output='DP-1'};hl.config{general={border_size=2}};unknown()",
            "hl.monitor{output='DP-1'};unknown();hl.config{general={border_size=2}}",
        ] {
            let p = program(source, version);
            let review = prepare(&p, &d).unwrap();
            assert!(!review.model_complete);
            assert!(resolve(
                &p,
                &d,
                &review,
                &choose("general:border_size", Choice::WriteOverride)
            )
            .is_err());
        }
    }
}
#[test]
fn monitor_arguments_that_share_scalar_locals_are_preserved_by_explicit_scalar_read_replacement() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p=program("local shared=2;hl.monitor{output='DP-1',transform=shared};hl.config{general={border_size=shared}}",version);
        let d = draft(version, &[("general:border_size", Value::Int(3))]);
        let review = prepare(&p, &d).unwrap();
        let result = resolve(
            &p,
            &d,
            &review,
            &choose("general:border_size", Choice::WriteOverride),
        )
        .unwrap();
        let next = materialize(&p, &result);
        assert_eq!(
            next.documents[0].source,
            p.documents[0]
                .source
                .replacen("border_size=shared", "border_size=3", 1)
        );
        assert!(result.draft.values.is_empty());
    }
}
