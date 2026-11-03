use hyprland_editor_core::{
    lua::{
        config_inspection,
        registration::{self, Role},
        replay::{
            override_review::{self, Choice},
            scalar_edit::{self, Change, Selection},
            *,
        },
        source_hash,
    },
    workspace_graph::revisions,
    Draft, Value, AUDITED_MAIN, BASELINE,
};
fn program(version: &str, id: &str, literal: &str) -> Program {
    let key = &registration::find(version, Role::Global, id)
        .unwrap()
        .lua_key;
    let source = format!(
        "-- immutable\r\nhl.config({{[{}]={literal}}})\r\n",
        Value::String(key.clone()).lua()
    );
    Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            sha256: source_hash(&source),
            source,
            load_name: b"/fixture/complex.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn cases() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "misc:background_color",
            "'#123456'",
            Value::Color(0x80123456),
        ),
        (
            "general:col.active_border",
            "{colors={'#123','#456'},angle=45}",
            Value::Gradient {
                colors: vec![0x80123456, 0xffff0000],
                angle: 100000.125,
            },
        ),
        (
            "decoration:shadow:offset",
            "{0,0}",
            Value::Vec2([-0.0, 0.1]),
        ),
        (
            "general:gaps_out",
            "3.9",
            Value::CssGap([-3, 5, i64::MAX, i64::MIN]),
        ),
        (
            "group:groupbar:font_weight_active",
            "'bold'",
            Value::FontWeight(9007199254740992),
        ),
    ]
}
#[test]
fn every_compound_family_compares_and_keeps_native_validated_literals_without_changing_originals() {
    for version in [BASELINE, AUDITED_MAIN] {
        for (id, initial, value) in cases() {
            let p = program(version, id, initial);
            let before = replay(&p).unwrap();
            let inspection = config_inspection::inspect(&before.events, version).unwrap();
            let c = &inspection.batches[0].candidates[0];
            let comparison = scalar_edit::compare(
                &p,
                &Change {
                    event_order: c.event_order,
                    location: c.location.clone(),
                    setting_id: id.into(),
                    selection: Selection::SelectedExpression,
                    value: value.clone(),
                },
            )
            .unwrap();
            assert!(
                comparison.review.modeled_observations_match,
                "{version} {id}: {:?}",
                comparison.review.differences
            );
            let pieces = revisions::apply(&p, &[], &comparison.plan).unwrap();
            let after = revisions::program(p.clone(), &pieces).unwrap();
            assert!(after.documents[0].source.starts_with("-- immutable\r\n"));
            assert!(after.documents[0].source.ends_with("\r\n"));
            assert_eq!(p.documents[0].sha256, source_hash(&p.documents[0].source));
            let checked =
                config_inspection::inspect(&replay(&after).unwrap().events, version).unwrap();
            assert_eq!(
                serde_json::to_value(&checked.batches[0].candidates[0].model_value).unwrap(),
                serde_json::to_value(Some(&comparison.proposed_native_value)).unwrap()
            );
            let mut draft = Draft::new(version);
            draft.values.insert(id.into(), value);
            let review = override_review::prepare(&p, &draft).unwrap();
            assert!(review.rows[0].can_write);
            let result = override_review::resolve(
                &p,
                &draft,
                &review,
                &[(id.into(), Choice::WriteOverride)].into(),
            )
            .unwrap();
            assert!(result.draft.values.is_empty());
            assert_eq!(result.plans.len(), 1);
        }
    }
}
#[test]
fn vector_constraints_and_undefined_integer_casts_are_enforced_without_clamping() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(version, "decoration:shadow:offset", "{0,0}");
        let c = config_inspection::inspect(&replay(&p).unwrap().events, version)
            .unwrap()
            .batches
            .remove(0)
            .candidates
            .remove(0);
        let result = scalar_edit::compare(
            &p,
            &Change {
                event_order: c.event_order,
                location: c.location,
                setting_id: c.setting_id,
                selection: Selection::SelectedExpression,
                value: Value::Vec2([251.0, 0.0]),
            },
        );
        assert_eq!(result.is_ok(), version == BASELINE);
        let p = program(version, "group:groupbar:font_weight_active", "400");
        let c = config_inspection::inspect(&replay(&p).unwrap().events, version)
            .unwrap()
            .batches
            .remove(0)
            .candidates
            .remove(0);
        assert!(scalar_edit::compare(
            &p,
            &Change {
                event_order: c.event_order,
                location: c.location,
                setting_id: c.setting_id,
                selection: Selection::SelectedExpression,
                value: Value::FontWeight(i64::MAX)
            }
        )
        .is_err());
    }
}
