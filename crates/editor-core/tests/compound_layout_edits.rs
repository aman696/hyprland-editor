use hyprland_editor_core::{
    lua::{
        config_inspection,
        registration::{self, Role},
        replay::{
            scalar_edit::{self, Change, Selection},
            *,
        },
        source_hash,
    },
    workspace_graph::revisions,
    Value, AUDITED_MAIN, BASELINE,
};
fn program(version: &str, id: &str, value: &str) -> Program {
    let meta = registration::find(version, Role::Global, id).unwrap();
    let source = format!(
        "-- header é\r\nhl.config({{[{}]={value}}})\r\n-- footer\r\n",
        Value::String(meta.lua_key.clone()).lua()
    );
    Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            sha256: source_hash(&source),
            source,
            load_name: b"/fixture/main.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn compare(p: &Program, id: &str, value: Value) -> Result<scalar_edit::Comparison, String> {
    let report = replay(p)?;
    let inspection = config_inspection::inspect(&report.events, &p.version)?;
    let c = inspection
        .batches
        .iter()
        .flat_map(|b| &b.candidates)
        .find(|c| c.setting_id == id)
        .ok_or("Missing candidate")?;
    scalar_edit::compare(
        p,
        &Change {
            event_order: c.event_order,
            location: c.location.clone(),
            setting_id: id.into(),
            selection: Selection::PreserveTableLayout,
            value,
        },
    )
}
fn kept(p: &Program, c: &scalar_edit::Comparison) -> Program {
    assert!(c.table_layout_preserved);
    assert!(
        c.review.modeled_observations_match,
        "{:?}",
        c.review.differences
    );
    assert!(!c.review.ownership_verified && !c.review.runtime_verified);
    let pieces = revisions::apply(p, &[], &c.plan).unwrap();
    let result = revisions::program(p.clone(), &pieces).unwrap();
    assert!(result.documents[0].source.starts_with("-- header é\r\n"));
    assert!(result.documents[0].source.ends_with("\r\n-- footer\r\n"));
    let after = config_inspection::inspect(&replay(&result).unwrap().events, &p.version).unwrap();
    assert!(after.stopped.is_none(), "{:?}", after.stopped);
    result
}
#[test]
fn changing_a_component_preserves_exact_unrelated_bytes_and_unused_fields() {
    for version in [BASELINE, AUDITED_MAIN] {
        for (id,initial,value,old,new) in [
            ("general:gaps_out","{top = 1 --[[keep ,}]] , right=2; bottom=3,left=4, note='-- literal é', [false]=true}",Value::CssGap([8,2,3,4]),"top = 1","top = 8"),
            ("decoration:shadow:offset","{1.0, -- x\r\n 2.0; description = 'untouched'}",Value::Vec2([5.0,2.0]),"{1.0","{5.0"),
        ] {
            let p=program(version,id,initial);
            let c=compare(&p,id,value).unwrap();
            let after=kept(&p,&c);
            assert_eq!(after.documents[0].source,p.documents[0].source.replacen(old,new,1));
        }
    }
}
#[test]
fn inserted_gap_fields_cannot_be_swallowed_by_line_comments_and_keep_named_nil_slots() {
    for version in [BASELINE, AUDITED_MAIN] {
        for initial in [
            "{top=1 -- trailing\r\n}",
            "{top=1, -- trailing\r\n note='stay'}",
            "{top=1,right=nil; -- right note\r\n}",
            "{ -- first\r\n}",
        ] {
            let p = program(version, "general:gaps_out", initial);
            let c = compare(&p, "general:gaps_out", Value::CssGap([1, 2, 3, 4])).unwrap();
            let after = kept(&p, &c);
            assert_eq!(
                after.documents[0].source.matches("-- trailing").count(),
                initial.matches("-- trailing").count()
            );
            assert_eq!(
                after.documents[0].source.matches("-- right note").count(),
                initial.matches("-- right note").count()
            );
            if initial.contains("note='stay'") {
                assert!(after.documents[0].source.contains("note='stay'"));
            }
        }
    }
}
#[test]
fn gradient_growth_and_tail_removal_preserve_comments_in_fields_and_between_separators() {
    for version in [BASELINE, AUDITED_MAIN] {
        for stops in [
            "'#123', -- between\r\n '#456' -- last\r\n",
            "[1]='#123';[2 --[[key note]] ]= --[[value note]] '#456' -- before separator\r\n,",
            "'#123', '#456', '#789'",
            "'#123', '#456'; '#789';",
            "'#123' -- before comma\r\n, '#456' -- after\r\n",
        ] {
            for colors in [
                vec![0xff112233],
                vec![0xff112233, 0xff445566, 0xff778899, 0xffabcdef],
            ] {
                let literal =
                    format!("{{colors={{{stops}}}, angle = 45, custom='stay', [false]=17}}");
                let p = program(version, "general:col.active_border", &literal);
                let c = compare(
                    &p,
                    "general:col.active_border",
                    Value::Gradient {
                        colors,
                        angle: 90.0,
                    },
                )
                .unwrap();
                let after = kept(&p, &c);
                for comment in [
                    "-- between",
                    "-- last",
                    "--[[key note]]",
                    "--[[value note]]",
                    "-- before separator",
                    "-- before comma",
                    "-- after",
                ] {
                    assert_eq!(
                        after.documents[0].source.matches(comment).count(),
                        p.documents[0].source.matches(comment).count(),
                        "{comment} {}",
                        after.documents[0].source
                    );
                }
                assert!(after.documents[0]
                    .source
                    .contains("custom='stay', [false]=17"));
            }
        }
    }
}
#[test]
fn comments_inside_replaced_arithmetic_remain_and_signed_zero_is_not_a_noop() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            version,
            "decoration:shadow:offset",
            "{(0.0 --[[memo]] + 0.0), 2.0}",
        );
        let c = compare(&p, "decoration:shadow:offset", Value::Vec2([-0.0, 2.0])).unwrap();
        let after = kept(&p, &c);
        assert!(after.documents[0].source.contains("-0.0 --[[memo]]"));
        assert!(
            matches!(&c.proposed_native_value,hyprland_editor_core::lua::scalar_codec::Outcome::Accepted {native:hyprland_editor_core::lua::scalar_codec::NativeValue::Vec2Bits(v),..} if v[0]==(-0.0f32).to_bits())
        );
    }
}
#[test]
fn layout_refuses_computed_keys_aliases_and_fields_created_outside_constructor() {
    for source in [
        "local k='top';hl.config({general={gaps_out={[k]=1}}})",
        "local t={top=1};hl.config({general={gaps_out=t}})",
        "local t={top=1};t.right=2;hl.config({general={gaps_out=t}})",
    ] {
        let mut p = program(BASELINE, "general:gaps_out", "1");
        p.documents[0].source = source.into();
        p.documents[0].sha256 = source_hash(source);
        let original = p.documents[0].source.clone();
        assert!(compare(&p, "general:gaps_out", Value::CssGap([2, 3, 4, 5])).is_err());
        assert_eq!(p.documents[0].source, original);
    }
}
#[test]
fn removing_component_effects_is_reported_instead_of_certifying_constructor_ownership() {
    let p = program(BASELINE, "general:gaps_out", "{top=(function() --[[retained explanation]]\n hl.config({general={border_size=2}}); return 1 end)(),right=2,bottom=3,left=4}");
    let c = compare(&p, "general:gaps_out", Value::CssGap([9, 2, 3, 4])).unwrap();
    assert!(!c.review.modeled_observations_match);
    assert!(!c.review.differences.is_empty());
    assert!(!c.review.ownership_verified && !c.review.runtime_verified);
    assert!(c.plan.edits[0]
        .replacement
        .contains("--[[retained explanation]]"));
    assert!(p.documents[0]
        .source
        .contains("hl.config({general={border_size=2}})"));
}

#[test]
fn gradient_array_separator_matrix_keeps_every_comment_for_all_length_changes() {
    for version in [BASELINE, AUDITED_MAIN] {
        for old_len in 1..=4 {
            for new_len in 1..=4 {
                for trailing in [false, true] {
                    let mut fields = vec![];
                    for i in 1..=old_len {
                        fields.push(format!("[{i} --[[key {i}]]] = '#123' -- value {i}\r\n"));
                    }
                    let mut stops = fields.join("; -- separator\r\n");
                    if trailing {
                        stops.push_str("; -- trailing\r\n");
                    }
                    let p = program(
                        version,
                        "general:col.active_border",
                        &format!("{{colors={{{stops}}},angle=0}}"),
                    );
                    let c = compare(
                        &p,
                        "general:col.active_border",
                        Value::Gradient {
                            colors: vec![0xffabcdef; new_len],
                            angle: 10.0,
                        },
                    )
                    .unwrap();
                    let after = kept(&p, &c);
                    for i in 1..=old_len {
                        for marker in [format!("--[[key {i}]]"), format!("-- value {i}")] {
                            assert_eq!(
                                after.documents[0].source.matches(&marker).count(),
                                1,
                                "{version} {old_len}->{new_len}: {marker}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn unchanged_native_components_keep_original_spelling_and_computations() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            version,
            "general:col.active_border",
            "{colors={'#123', 'rgba(445566FF)'},angle=45, note='stay'}",
        );
        // The displayed angle includes float32 storage conversion, yet retains
        // the same native radians. Appending a stop must not rewrite angle/colors.
        let c = compare(
            &p,
            "general:col.active_border",
            Value::Gradient {
                colors: vec![0xff112233, 0xff445566, 0xffabcdef],
                angle: 45.00000125223908,
            },
        )
        .unwrap();
        let after = kept(&p, &c);
        assert!(after.documents[0]
            .source
            .contains("'#123', 'rgba(445566FF)'"));
        assert!(after.documents[0].source.contains("angle=45, note='stay'"));
        let p = program(
            version,
            "general:gaps_out",
            "{top=1,right=2.9,bottom='-3',left=(1+1)}",
        );
        let c = compare(&p, "general:gaps_out", Value::CssGap([7, 0, -3, 2])).unwrap();
        let after = kept(&p, &c);
        assert_eq!(
            after.documents[0].source,
            p.documents[0].source.replace("top=1", "top=7")
        );
    }
}

#[test]
fn override_reconciliation_preserves_literal_layout_and_never_falls_back_after_refusal() {
    use hyprland_editor_core::{
        lua::replay::override_review::{self, Choice},
        Draft,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            version,
            "general:gaps_out",
            "{top=1 --[[memo]],right=2,bottom=3,left=4,note='stay'}",
        );
        let mut draft = Draft::new(version);
        draft
            .values
            .insert("general:gaps_out".into(), Value::CssGap([9, 2, 3, 4]));
        let review = override_review::prepare(&p, &draft).unwrap();
        assert!(
            review.rows[0]
                .target
                .as_ref()
                .unwrap()
                .preserve_table_layout
        );
        let choices = [("general:gaps_out".into(), Choice::WriteOverride)].into();
        let resolved = override_review::resolve(&p, &draft, &review, &choices).unwrap();
        assert!(resolved.draft.values.is_empty());
        assert_eq!(
            resolved.plans[0].edits[0].replacement,
            "{top=9 --[[memo]],right=2,bottom=3,left=4,note='stay'}"
        );
        let mut p = p;
        let source = "local key='top';hl.config({general={gaps_out={[key]=1}}})";
        p.documents[0].source = source.into();
        p.documents[0].sha256 = source_hash(source);
        let review = override_review::prepare(&p, &draft).unwrap();
        assert!(override_review::resolve(&p, &draft, &review, &choices).is_err());
        assert_eq!(draft.values.len(), 1);
        assert_eq!(p.documents[0].source, source);
    }
}
