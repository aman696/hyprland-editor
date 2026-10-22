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
