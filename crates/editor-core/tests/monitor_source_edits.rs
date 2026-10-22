use hyprland_editor_core::{
    lua::{
        replay::{
            monitor_edit::{self, Change, Selection},
            *,
        },
        source_hash,
    },
    Value, AUDITED_MAIN, BASELINE,
};
fn document(id: &str, source: &str) -> Document {
    Document {
        id: id.into(),
        source: source.into(),
        sha256: source_hash(source),
        load_name: format!("<captured>/{id}.lua").into_bytes(),
    }
}
fn program(source: &str, version: &str) -> Program {
    Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "main".into(),
        documents: vec![document("main", source)],
        bindings: vec![],
        initial_package_path: Some(b"<captured>/?.lua".to_vec()),
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn change(
    program: &Program,
    call: usize,
    field: &str,
    value: Value,
    selection: Selection,
) -> Change {
    let report = replay(program).unwrap();
    let event = &report.events[call];
    let Observed::Table(rows) = &event.arguments[0] else {
        panic!()
    };
    let row = rows
        .iter()
        .position(|(k, _)| matches!(k, Observed::Bytes(b) if b == field.as_bytes()))
        .unwrap();
    Change {
        event_order: event.order,
        row,
        field: field.into(),
        value,
        selection,
    }
}
#[test]
fn monitor_source_comparison_tracks_cross_file_alias_impacts_and_retains_unrelated_source_bytes() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p=program("local part=require('part');hl.monitor{output='DP-1', transform=part.turn};hl.monitor{output='DP-2', transform=part.turn};hl.config{general={border_size=part.turn}}",version);
        p.documents.push(document(
            "part",
            "-- keep comment\nreturn {turn = 2, other = 'unchanged'}\n",
        ));
        let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
            panic!()
        };
        p.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: "part".into(),
                loader_data: vec![],
            },
        });
        let c = change(
            &p,
            0,
            "transform",
            Value::Int(3),
            Selection::SelectedExpression,
        );
        let result = monitor_edit::compare(&p, &c).unwrap();
        assert!(
            result.review.modeled_observations_match,
            "{:?}",
            result.review.differences
        );
        assert_eq!(result.plan.edits[0].document, "main");
        assert_eq!(result.plan.edits[0].replacement, "3");
        assert!(result.review.revisions[0].content.contains("transform=3"));
        let c = Change {
            selection: Selection::OriginalLiteral,
            ..c
        };
        let result = monitor_edit::compare(&p, &c).unwrap();
        assert!(!result.review.modeled_observations_match);
        assert_eq!(result.plan.edits[0].document, "part");
        assert_eq!(
            result.review.revisions[0].content,
            "-- keep comment\nreturn {turn = 3, other = 'unchanged'}\n"
        );
        assert_eq!(
            result
                .review
                .differences
                .iter()
                .filter(|d| d.kind == edits::DifferenceKind::Arguments)
                .count(),
            2
        );
        assert!(!result.review.runtime_verified);
        assert!(!result.review.ownership_verified);
        assert_eq!(
            p.documents[1].source,
            "-- keep comment\nreturn {turn = 2, other = 'unchanged'}\n"
        );
    }
}
#[test]
fn monitor_output_and_compound_fields_use_exact_coordinates_and_product_validation() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p=program("hl.monitor{output=123, transform=99, reserved={top=-1,left=2}, icc='/unopened/old.icc'}",version);
        for (field, value) in [
            ("output", Value::String("desc:Panel".into())),
            ("transform", Value::Int(3)),
            ("reserved", Value::CssGap([-2, 3, 4, 5])),
            ("icc", Value::String("/unopened/new.icc".into())),
        ] {
            let c = change(&p, 0, field, value, Selection::SelectedExpression);
            let result = monitor_edit::compare(&p, &c).unwrap();
            assert!(
                result.review.modeled_observations_match,
                "{field}: {:?}",
                result.review.differences
            );
            assert_eq!(result.plan.edits.len(), 1);
        }
        for (field, value) in [
            ("transform", Value::Int(8)),
            ("output", Value::String("bad\nselector".into())),
            ("icc", Value::String("".into())),
        ] {
            let c = change(&p, 0, field, value, Selection::SelectedExpression);
            assert!(monitor_edit::compare(&p, &c).is_err(), "{field}");
        }
        let p = program(
            "hl.monitor{output='DP-1',reserved=1,reserved_area=2}",
            version,
        );
        let c = change(
            &p,
            0,
            "reserved",
            Value::CssGap([2; 4]),
            Selection::SelectedExpression,
        );
        assert!(monitor_edit::compare(&p, &c)
            .unwrap_err()
            .contains("aliases"));
        let p = program("hl.monitor{output='DP-1',transform=1}", version);
        let mut c = change(
            &p,
            0,
            "transform",
            Value::Int(2),
            Selection::SelectedExpression,
        );
        c.field = "output".into();
        c.value = Value::String("DP-2".into());
        assert!(monitor_edit::compare(&p, &c)
            .unwrap_err()
            .contains("exact required"));
    }
}
#[test]
fn scalar_comparison_after_monitor_keeps_monitor_changes_in_the_review() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p=program("local shared=2;hl.monitor{output='DP-1',transform=shared};hl.config{general={border_size=shared}}",version);
        let r = replay(&p).unwrap();
        assert!(r.halt.is_none());
        let result =
            hyprland_editor_core::lua::config_inspection::inspect(&[r.events[1].clone()], version)
                .unwrap();
        let c = &result.batches[0].candidates[0];
        let result = scalar_edit::compare(
            &p,
            &scalar_edit::Change {
                event_order: r.events[1].order,
                location: c.location.clone(),
                setting_id: c.setting_id.clone(),
                selection: scalar_edit::Selection::OriginalLiteral,
                value: Value::Int(3),
            },
        )
        .unwrap();
        assert!(!result.review.modeled_observations_match);
        assert!(result
            .review
            .differences
            .iter()
            .any(|d| d.kind == edits::DifferenceKind::Arguments
                && d.order == Some(r.events[0].order)));
    }
}

#[test]
fn monitor_gap_layout_keeps_signed_components_native_spelling_comments_and_unowned_fields() {
    for version in [BASELINE, AUDITED_MAIN] {
        for field in ["reserved", "reserved_area"] {
            let source=format!("-- header é\r\nhl.monitor{{output='DP-1',{field}={{top = -1 --[[top memo]] , right='2'; bottom=3.0,left=-4, note='-- literal é', [false]=true}}}}\r\n-- footer\r\n");
            let p = program(&source, version);
            let c = change(
                &p,
                0,
                field,
                Value::CssGap([-8, 2, 3, -4]),
                Selection::PreserveTableLayout,
            );
            let result = monitor_edit::compare(&p, &c).unwrap();
            assert!(result.table_layout_preserved);
            assert!(
                result.review.modeled_observations_match,
                "{:?}",
                result.review.differences
            );
            assert_eq!(
                result.review.revisions[0].content,
                source.replacen("top = -1", "top = -8", 1)
            );
            assert!(!result.review.ownership_verified && !result.review.runtime_verified);
            assert_eq!(p.documents[0].source, source);
            let pieces =
                hyprland_editor_core::workspace_graph::revisions::apply(&p, &[], &result.plan)
                    .unwrap();
            let kept =
                hyprland_editor_core::workspace_graph::revisions::program(p, &pieces).unwrap();
            assert_eq!(kept.documents[0].source, result.review.revisions[0].content);
        }
        for initial in [
            "{top=-1 -- trailing\r\n}",
            "{-- opening\r\n}",
            "{top=-1,right=nil; -- nil slot\r\n}",
        ] {
            let p = program(
                &format!("hl.monitor{{output='DP-1',reserved={initial}}}"),
                version,
            );
            let c = change(
                &p,
                0,
                "reserved",
                Value::CssGap([-1, 2, -3, 4]),
                Selection::PreserveTableLayout,
            );
            let result = monitor_edit::compare(&p, &c).unwrap();
            assert!(
                result.review.modeled_observations_match,
                "{:?}",
                result.review.differences
            );
            for comment in ["-- trailing", "-- opening", "-- nil slot"] {
                assert_eq!(
                    result.review.revisions[0].content.matches(comment).count(),
                    initial.matches(comment).count()
                );
            }
        }
    }
}

#[test]
fn monitor_original_gap_table_reports_shared_and_later_write_impacts_without_fallback() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p=program("local area=require('part');hl.monitor{output='DP-1',reserved=area};hl.monitor{output='DP-2',reserved=area}",version);
        p.documents.push(document(
            "part",
            "-- captured module\nreturn {top=-1, right='2', bottom=3, left=4, note='stay'}\n",
        ));
        let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
            panic!()
        };
        p.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: "part".into(),
                loader_data: vec![],
            },
        });
        let c = change(
            &p,
            0,
            "reserved",
            Value::CssGap([-8, 2, 3, 4]),
            Selection::OriginalTableLayout,
        );
        let result = monitor_edit::compare(&p, &c).unwrap();
        assert!(result.table_layout_preserved);
        assert_eq!(result.plan.edits[0].document, "part");
        assert_eq!(
            result.review.revisions[0].content,
            p.documents[1].source.replacen("top=-1", "top=-8", 1)
        );
        assert!(!result.review.modeled_observations_match);
        assert_eq!(
            result
                .review
                .differences
                .iter()
                .filter(|d| d.kind == edits::DifferenceKind::Arguments)
                .count(),
            1
        );
        let selected = Change {
            selection: Selection::PreserveTableLayout,
            ..c
        };
        assert!(monitor_edit::compare(&p, &selected)
            .unwrap_err()
            .contains("directly selected table"));

        let p=program("local area={top=-1,right=2,bottom=3,left=4};area.top=7;hl.monitor{output='DP-1',reserved=area}",version);
        let c = change(
            &p,
            0,
            "reserved",
            Value::CssGap([-8, 2, 3, 4]),
            Selection::OriginalTableLayout,
        );
        let result = monitor_edit::compare(&p, &c).unwrap();
        assert!(
            !result.review.modeled_observations_match,
            "A later write masked the requested change"
        );
        assert!(result
            .review
            .differences
            .iter()
            .any(|d| d.kind == edits::DifferenceKind::Arguments));
    }
}

#[test]
fn monitor_layout_refuses_unowned_shapes_and_keeps_removed_component_effects_visible() {
    for version in [BASELINE, AUDITED_MAIN] {
        for source in [
            "local key='top';hl.monitor{output='DP-1',reserved={[key]=-1}}",
            "local area={top=-1};area.right=2;hl.monitor{output='DP-1',reserved=area}",
            "hl.monitor{output='DP-1',reserved=-1}",
        ] {
            let p = program(source, version);
            let c = change(
                &p,
                0,
                "reserved",
                Value::CssGap([-8, 2, 3, 4]),
                Selection::OriginalTableLayout,
            );
            assert!(monitor_edit::compare(&p, &c).is_err(), "{source}");
        }
        let p = program("hl.monitor{output='DP-1',transform=1}", version);
        let c = change(
            &p,
            0,
            "transform",
            Value::Int(2),
            Selection::PreserveTableLayout,
        );
        assert!(monitor_edit::compare(&p, &c)
            .unwrap_err()
            .contains("reserved-area"));

        let p=program("local function effect() hl.config{general={border_size=1}};return -1 end;hl.monitor{output='DP-1',reserved={top=(effect() --[[effect memo]]),right=2,bottom=3,left=4}}",version);
        let c = change(
            &p,
            1,
            "reserved",
            Value::CssGap([-8, 2, 3, 4]),
            Selection::PreserveTableLayout,
        );
        let result = monitor_edit::compare(&p, &c).unwrap();
        assert!(result.table_layout_preserved);
        assert!(result.review.revisions[0]
            .content
            .contains("--[[effect memo]]"));
        assert!(!result.review.modeled_observations_match);
        assert!(result.review.differences.iter().any(|d| matches!(
            d.kind,
            edits::DifferenceKind::MissingCall | edits::DifferenceKind::AddedCall
        )));
    }
}
