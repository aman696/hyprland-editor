use hyprland_editor_core::{
    lua::{
        replay::{
            monitor_edit::{self, FieldChange, TableSelection},
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
        load_name: format!("<capture>/{id}.lua").into_bytes(),
    }
}
fn program(source: &str, version: &str) -> Program {
    Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "main".into(),
        documents: vec![document("main", source)],
        bindings: vec![],
        initial_package_path: Some(b"<capture>/?.lua".to_vec()),
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn remove(p: &Program, field: &[u8]) -> FieldChange {
    let report = replay(p).unwrap();
    let event = report
        .events
        .iter()
        .find(|e| e.api == "hl.monitor")
        .unwrap();
    let Observed::Table(rows) = &event.arguments[0] else {
        panic!()
    };
    let row = rows
        .iter()
        .position(|(k, _)| matches!(k, Observed::Bytes(b) if b == field))
        .unwrap();
    FieldChange::Remove {
        event_order: event.order,
        row,
        raw_key: field.to_vec(),
    }
}
fn add(p: &Program, field: &str, value: Value) -> FieldChange {
    let event = replay(p)
        .unwrap()
        .events
        .into_iter()
        .find(|e| e.api == "hl.monitor")
        .unwrap();
    FieldChange::Add {
        event_order: event.order,
        field: field.into(),
        value,
        selection: TableSelection::OriginalConstructor,
    }
}
fn content(result: &monitor_edit::Comparison) -> &str {
    &result.review.revisions[0].content
}
#[test]
fn field_removal_preserves_all_comments_unrelated_fields_and_separator_variants() {
    for version in [BASELINE, AUDITED_MAIN] {
        for source in [
            "hl.monitor{transform=2, output='DP-1'}",
            "hl.monitor{output='DP-1', transform=2}",
            "hl.monitor{output='DP-1'; transform=2;}",
            "hl.monitor{transform -- key\n = -- value\n 2, -- separator\n output='DP-1'}",
            "hl.monitor{output='DP-1', -- separator\n transform={ -- nested\n top=1 -- inner\n }, -- trailing\n}",
            "hl.monitor{output='DP-1', -- before\r\n transform=2 -- last\r\n}",
        ] {
            let p=program(source,version);
            let result=monitor_edit::compare_field(&p,&remove(&p,b"transform")).unwrap();
            assert!(result.review.modeled_observations_match,"{source}: {:?}",result.review.differences);
            assert!(!content(&result).contains("transform"));
            assert!(content(&result).contains("output='DP-1'"));
            for line in source.lines().filter(|line| line.contains("--")) {
                let comment=line.split_once("--").unwrap().1;
                assert!(content(&result).contains(&format!("--{comment}")),"Lost comment {comment}");
            }
            assert!(result.table_layout_preserved);
            assert!(!result.review.runtime_verified && !result.review.ownership_verified);
        }
    }
}
#[test]
fn adding_fields_preserves_loaders_array_indices_comments_and_nil_placeholders() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = program(
            "hl.monitor{ -- opening\noutput='DP-1',outside=require('part'), false, 77}",
            version,
        );
        p.documents.push(document("part", "return 123"));
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
        let result = monitor_edit::compare_field(&p, &add(&p, "transform", Value::Int(3))).unwrap();
        assert!(
            result.review.modeled_observations_match,
            "{:?}",
            result.review.differences
        );
        assert!(content(&result).contains("outside=require('part'), false, 77"));
        assert!(content(&result).contains("-- opening"));
        assert_eq!(result.plan.edits.len(), 1);
        assert!(result.plan.edits[0].span.is_empty());
        let p = program(
            "hl.monitor{output='DP-1',transform= -- placeholder\r\n nil}",
            version,
        );
        let result = monitor_edit::compare_field(&p, &add(&p, "transform", Value::Int(3))).unwrap();
        assert!(result.review.modeled_observations_match);
        assert_eq!(content(&result).matches("transform").count(), 1);
        assert!(content(&result).contains("-- placeholder\r\n"));
        let p = program("hl.monitor{ -- missing output\ntransform=2}", version);
        let result =
            monitor_edit::compare_field(&p, &add(&p, "output", Value::String("DP-1".into())))
                .unwrap();
        assert!(result.review.modeled_observations_match);
        assert!(content(&result).contains("-- missing output"));
        let p = program("hl.monitor{output=true}", version);
        let result = monitor_edit::compare(
            &p,
            &monitor_edit::Change {
                event_order: 0,
                row: 0,
                field: "output".into(),
                selection: monitor_edit::Selection::SelectedExpression,
                value: Value::String("DP-1".into()),
            },
        )
        .unwrap();
        assert!(result.review.modeled_observations_match);
    }
}
#[test]
fn removal_uses_recorded_dynamic_keys_and_latest_assignment_or_rawset_without_resurrecting_values()
{
    for version in [BASELINE, AUDITED_MAIN] {
        for source in [
            "local name='transform';hl.monitor{output='DP-1',[name]=2}",
            "local rule={output='DP-1',transform=1};rule.transform=2;hl.monitor(rule)",
            "local rule={output='DP-1',transform=1};rawset(rule,'transform',2);hl.monitor(rule)",
            "hl.monitor{output='DP-1',['transform\\0alias']=2}",
            "hl.monitor{output='DP-1',unknown_field=2}",
        ] {
            let p = program(source, version);
            let key: &[u8] = if source.contains("unknown_field") {
                b"unknown_field".as_slice()
            } else if source.contains("alias") {
                b"transform\0alias"
            } else {
                b"transform"
            };
            let result = monitor_edit::compare_field(&p, &remove(&p, key))
                .unwrap_or_else(|e| panic!("{source}: {e}"));
            assert!(
                result.review.modeled_observations_match,
                "{source}: {:?}",
                result.review.differences
            );
            if source.contains("rule.transform=") {
                assert!(content(&result).contains("rule.transform=nil"));
            }
            if source.contains("rawset") {
                assert!(content(&result).contains("rawset(rule,'transform',nil)"));
            }
        }
        let p = program(
            "hl.monitor{output='DP-1',reserved=1,reserved_area=2}",
            version,
        );
        let result = monitor_edit::compare_field(&p, &remove(&p, b"reserved_area")).unwrap();
        assert!(result.review.modeled_observations_match);
        assert!(content(&result).contains("reserved=1"));
    }
}
#[test]
fn structural_edits_report_shared_table_impact_and_later_writes_masking_an_insertion() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = program(
            "local part=require('part');hl.monitor(part);hl.monitor(part)",
            version,
        );
        p.documents.push(document(
            "part",
            "-- keep\nreturn {output='DP-1',transform=2}",
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
        let result = monitor_edit::compare_field(&p, &remove(&p, b"transform")).unwrap();
        assert!(!result.review.modeled_observations_match);
        assert_eq!(result.plan.edits[0].document, "part");
        assert!(result
            .review
            .differences
            .iter()
            .any(|d| d.kind == edits::DifferenceKind::Arguments));
        let result =
            monitor_edit::compare_field(&p, &add(&p, "scale", Value::String("auto".into())))
                .unwrap();
        assert!(!result.review.modeled_observations_match);
        let p = program(
            "local rule={output='DP-1'};rule.transform=nil;hl.monitor(rule)",
            version,
        );
        let result = monitor_edit::compare_field(&p, &add(&p, "transform", Value::Int(2))).unwrap();
        assert!(
            !result.review.modeled_observations_match,
            "Later deletion must not be reported as a successful field addition"
        );
    }
}
#[test]
fn field_coordinates_required_output_alias_conflicts_and_unknown_factories_are_refused() {
    let p = program("hl.monitor{output='DP-1',reserved=1}", BASELINE);
    assert!(monitor_edit::compare_field(&p, &remove(&p, b"output"))
        .unwrap_err()
        .contains("required output"));
    assert!(
        monitor_edit::compare_field(&p, &add(&p, "reserved_area", Value::CssGap([1; 4])))
            .unwrap_err()
            .contains("already present")
    );
    assert!(monitor_edit::compare_field(&p, &add(&p, "unknown", Value::Int(1))).is_err());
    let FieldChange::Remove {
        event_order, row, ..
    } = remove(&p, b"reserved")
    else {
        panic!()
    };
    assert!(monitor_edit::compare_field(
        &p,
        &FieldChange::Remove {
            event_order,
            row,
            raw_key: b"other".to_vec()
        }
    )
    .is_err());
    let p = program(
        "hl.monitor{output='DP-1',transform=1,transform=2}",
        BASELINE,
    );
    assert!(
        replay(&p).unwrap().halt.is_some(),
        "Duplicate constructor storage order remains unmodeled"
    );
}
#[test]
fn latest_nil_write_can_add_a_field_without_being_masked_or_guessing_a_future_write() {
    for version in [BASELINE, AUDITED_MAIN] {
        for source in ["local rule={output='DP-1',transform=1};rule.transform=nil;hl.monitor(rule)","local rule={output='DP-1'};rawset(rule,'transform',nil);hl.monitor(rule);rule.transform=nil"] {
            let p=program(source,version);
            let FieldChange::Add {event_order,field,value,..}=add(&p,"transform",Value::Int(2)) else {panic!()};
            let result=monitor_edit::compare_field(&p,&FieldChange::Add {event_order,field,value,selection:TableSelection::LatestWrite}).unwrap();
            assert!(result.review.modeled_observations_match,"{source}: {:?}",result.review.differences);
            assert_eq!(result.plan.edits.len(),1);
        }
        let p = program("hl.monitor{output='DP-1'}", version);
        assert!(monitor_edit::compare_field(
            &p,
            &FieldChange::Add {
                event_order: 0,
                field: "transform".into(),
                value: Value::Int(2),
                selection: TableSelection::LatestWrite
            }
        )
        .is_err());
    }
}
