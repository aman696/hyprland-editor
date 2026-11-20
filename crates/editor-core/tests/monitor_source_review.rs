use hyprland_editor_core::{
    lua::{
        replay::{
            self,
            monitor_review::{self, Choice},
            *,
        },
        source_hash,
    },
    monitor::{Output, Rule},
    workspace_graph::revisions,
    Value, AUDITED_MAIN, BASELINE,
};
fn program(source: &str, version: &str) -> Program {
    Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            source: source.into(),
            sha256: source_hash(source),
            load_name: b"/captured/main.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn rule(id: &str, output: &str, fields: impl IntoIterator<Item = (&'static str, Value)>) -> Rule {
    let mut r = Rule::new(id);
    r.output = Output::Connector(output.into());
    r.fields = fields.into_iter().map(|(k, v)| (k.into(), v)).collect();
    r
}
fn keep(p: &Program, result: &monitor_review::Resolution) -> Program {
    let mut pieces = vec![];
    for plan in &result.plans {
        pieces = revisions::prepare_pieces(p, &pieces, plan).unwrap();
    }
    let next = revisions::program(p.clone(), &pieces).unwrap();
    assert_eq!(
        hyprland_editor_core::workspace_graph::program_fingerprint(&next).unwrap(),
        result.program_sha256
    );
    assert!(!result.ownership_verified && !result.runtime_verified);
    next
}
#[test]
fn ordered_source_copy_requires_explicit_replacement_and_preserves_other_draft_rows() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            "hl.monitor{output='DP-1',transform=1};hl.monitor{output='DP-1',reserved={top=-3}}",
            version,
        );
        let drafts = vec![
            rule("first", "DP-2", [("transform", Value::Int(3))]),
            rule("second", "DP-3", []),
        ];
        let review = monitor_review::prepare(&p, &drafts).unwrap();
        assert!(review.model_complete);
        assert_eq!(review.sources.len(), 2);
        let choice = Choice::CopySource {
            event_order: 1,
            replace_id: Some("first".into()),
        };
        let result = monitor_review::resolve(&p, &drafts, &review, &choice).unwrap();
        assert_eq!(result.monitors[0].id, "first");
        assert_eq!(result.monitors[0].output, Output::Connector("DP-1".into()));
        assert_eq!(
            result.monitors[0].fields["reserved"],
            Value::CssGap([-3, 0, 0, 0])
        );
        assert_eq!(result.monitors[1], drafts[1]);
        assert!(result.plans.is_empty(), "{:?}", result.plans);
        keep(&p, &result);
        let all = monitor_review::resolve(&p, &drafts, &review, &Choice::CopyAllSources).unwrap();
        assert_eq!(all.monitors.len(), 2);
        assert_eq!(all.monitors[0].fields["transform"], Value::Int(1));
        assert_ne!(all.monitors[0].id, all.monitors[1].id);
        let appended = monitor_review::resolve(
            &p,
            &drafts,
            &review,
            &Choice::CopySource {
                event_order: 0,
                replace_id: None,
            },
        )
        .unwrap();
        assert_eq!(&appended.monitors[..2], drafts);
        assert_eq!(appended.monitors.len(), 3);
    }
}
#[test]
fn explicit_monitor_write_updates_adds_removes_and_keeps_original_comments_and_gap_layout() {
    for version in [BASELINE, AUDITED_MAIN] {
        let source="-- header é\r\nlocal area={output='DP-1',transform=1,mirror='DP-2',reserved_area={top=-1}};area.scale=nil;hl.monitor(area);hl.config{general={border_size=1}}\r\n-- footer\r\n";
        let p = program(source, version);
        let drafts = vec![rule(
            "draft",
            "DP-3",
            [
                ("transform", Value::Int(3)),
                ("scale", Value::String("auto".into())),
                ("reserved", Value::CssGap([-8, 2, 3, 4])),
            ],
        )];
        let review = monitor_review::prepare(&p, &drafts).unwrap();
        let choice = Choice::WriteDraft {
            draft_id: "draft".into(),
            event_order: 0,
        };
        let result = monitor_review::resolve(&p, &drafts, &review, &choice).unwrap();
        assert_eq!(result.monitors, drafts);
        assert!(result.plans.len() >= 4);
        let next = keep(&p, &result);
        let text = &next.documents[0].source;
        assert!(text.starts_with("-- header é\r\n"));
        assert!(text.ends_with("\r\n-- footer\r\n"));
        assert!(text.contains("area.scale=\"auto\""), "{text}");
        let event = &replay::replay(&next).unwrap().events[0];
        let Observed::Table(rows) = &event.arguments[0] else {
            panic!()
        };
        assert_eq!(rows.len(), 4);
        assert!(rows
            .iter()
            .any(|(k, _)| matches!(k,Observed::Bytes(b) if b==b"reserved")));
        assert!(!rows
            .iter()
            .any(|(k, _)| matches!(k,Observed::Bytes(b) if b==b"mirror"||b==b"reserved_area")));
        assert_eq!(p.documents[0].source, source);
        let p=program("hl.monitor{output='DP-1',reserved={top = -1 --[[memo]],right='2',bottom=3.0,left=4,note='stay'}}",version);
        let drafts = vec![rule(
            "draft",
            "DP-1",
            [("reserved", Value::CssGap([-8, 2, 3, 4]))],
        )];
        let review = monitor_review::prepare(&p, &drafts).unwrap();
        let result = monitor_review::resolve(&p, &drafts, &review, &choice).unwrap();
        assert_eq!(result.plans.len(), 1);
        assert_eq!(
            keep(&p, &result).documents[0].source,
            p.documents[0].source.replacen("top = -1", "top = -8", 1)
        );
    }
}
#[test]
fn equal_primary_values_keep_original_spellings_and_insertion_uses_a_selected_boundary() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program("hl.monitor{output=123,transform=2,scale='1'}", version);
        let drafts = vec![rule(
            "draft",
            "123",
            [
                ("transform", Value::Int(2)),
                ("scale", Value::String("1".into())),
            ],
        )];
        let review = monitor_review::prepare(&p, &drafts).unwrap();
        let result = monitor_review::resolve(
            &p,
            &drafts,
            &review,
            &Choice::WriteDraft {
                draft_id: "draft".into(),
                event_order: 0,
            },
        )
        .unwrap();
        assert!(result.plans.is_empty(), "{:?}", result.plans);
        let p = program("-- keep\nreturn 4\n", version);
        let review = monitor_review::prepare(&p, &drafts).unwrap();
        let choice = Choice::InsertDraft {
            draft_id: "draft".into(),
            document: "root".into(),
            anchor: monitor_edit::declarations::Anchor::End,
        };
        let result = monitor_review::resolve(&p, &drafts, &review, &choice).unwrap();
        let next = keep(&p, &result);
        assert!(next.documents[0].source.ends_with("return 4\n"));
        assert_eq!(replay::replay(&next).unwrap().events.len(), 1);
    }
}
#[test]
fn rejected_unknown_alias_and_partial_sources_remain_visible_without_silent_import() {
    for source in [
        "hl.monitor{output='DP-1',transform=99}",
        "hl.monitor{output='DP-1',extra=1}",
        "hl.monitor{output='DP-1',reserved=1,reserved_area=2}",
        "hl.monitor{output='DP-1',['scale\\0x']='1'}",
        "hl.monitor{output='DP-1',mode='unknown-native-helper-syntax'}",
    ] {
        let p = program(source, BASELINE);
        let review = monitor_review::prepare(&p, &[]).unwrap();
        assert_eq!(review.sources.len(), 1);
        assert!(review.sources[0].rule.is_none(), "{source}");
        assert!(review.sources[0].reason.is_some());
        assert!(monitor_review::resolve(&p, &[], &review, &Choice::CopyAllSources).is_err());
    }
    let p = program("hl.monitor{output='DP-1'};unknown()", BASELINE);
    let review = monitor_review::prepare(&p, &[]).unwrap();
    assert!(!review.model_complete);
    assert!(monitor_review::resolve(&p, &[], &review, &Choice::CopyAllSources).is_err());
    assert!(monitor_review::resolve(
        &p,
        &[],
        &review,
        &Choice::CopySource {
            event_order: 0,
            replace_id: None
        }
    )
    .is_ok());
}
#[test]
fn repeated_sites_and_shared_argument_effects_require_manual_source_review() {
    for version in [BASELINE, AUDITED_MAIN] {
        let drafts = vec![rule("draft", "DP-2", [("transform", Value::Int(3))])];
        for source in ["for i=1,2 do hl.monitor{output='DP-1',transform=1} end", "local t={output='DP-1',transform=1};hl.monitor(t);hl.monitor(t)", "local function effect() hl.config{general={border_size=1}};return 1 end;hl.monitor{output='DP-1',transform=effect()}"] {
            let p=program(source,version);let review=monitor_review::prepare(&p,&drafts).unwrap();let choice=Choice::WriteDraft {draft_id:"draft".into(),event_order:review.sources[0].event_order};assert!(monitor_review::resolve(&p,&drafts,&review,&choice).is_err(),"{source}");
        }
    }
}
#[test]
fn stale_choices_forged_layers_and_rule_count_limits_are_refused() {
    let p = program("hl.monitor{output='DP-1'}", BASELINE);
    let drafts = vec![rule("draft", "DP-2", [])];
    let review = monitor_review::prepare(&p, &drafts).unwrap();
    let choice = Choice::CopySource {
        event_order: 0,
        replace_id: Some("draft".into()),
    };
    let mut result = monitor_review::resolve(&p, &drafts, &review, &choice).unwrap();
    result.monitors[0].output = Output::Connector("unselected".into());
    assert!(monitor_review::validate_resolution(&drafts, &review, &choice, &result).is_err());
    let mut stale = review.clone();
    stale.program_sha256 = "0".repeat(64);
    assert!(monitor_review::resolve(&p, &drafts, &stale, &choice).is_err());
    assert!(monitor_review::resolve(
        &p,
        &drafts,
        &review,
        &Choice::WriteDraft {
            draft_id: "missing".into(),
            event_order: 0
        }
    )
    .is_err());
    let drafts = (0..64)
        .map(|i| rule(&format!("draft-{i}"), "DP-2", []))
        .collect::<Vec<_>>();
    let review = monitor_review::prepare(&p, &drafts).unwrap();
    assert!(monitor_review::resolve(
        &p,
        &drafts,
        &review,
        &Choice::CopySource {
            event_order: 0,
            replace_id: None
        }
    )
    .is_err());
}

#[test]
fn explicit_writes_repair_known_invalid_rows_but_refuse_unknown_field_erasure() {
    for version in [BASELINE, AUDITED_MAIN] {
        let drafts = vec![rule(
            "draft",
            "DP-1",
            [
                ("transform", Value::Int(3)),
                ("reserved", Value::CssGap([-1, 2, 3, 4])),
            ],
        )];
        for input in [
            "hl.monitor{transform=99}",
            "hl.monitor{output='DP-1',transform=99,reserved=1,reserved_area=2}",
        ] {
            let p = program(input, version);
            let review = monitor_review::prepare(&p, &drafts).unwrap();
            assert!(review.sources[0].rule.is_none());
            let result = monitor_review::resolve(
                &p,
                &drafts,
                &review,
                &Choice::WriteDraft {
                    draft_id: "draft".into(),
                    event_order: 0,
                },
            )
            .unwrap();
            let next = keep(&p, &result);
            assert!(monitor_review::prepare(&next, &drafts).unwrap().sources[0]
                .rule
                .is_some());
        }
        for input in [
            "hl.monitor{output='DP-1',future_field=true}",
            "hl.monitor{output='DP-1',[false]=true}",
        ] {
            let p = program(input, version);
            let review = monitor_review::prepare(&p, &drafts).unwrap();
            assert!(monitor_review::resolve(
                &p,
                &drafts,
                &review,
                &Choice::WriteDraft {
                    draft_id: "draft".into(),
                    event_order: 0
                }
            )
            .is_err());
        }
    }
}
#[test]
fn reconciliation_keeps_cross_document_private_pieces_and_captured_loader_evidence() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = program(
            "local r=require('part');hl.monitor(r);hl.config{general={border_size=1}}",
            version,
        );
        let source = "-- module memo\nreturn {output='DP-1',transform=1,mirror='DP-2'}\n";
        p.documents.push(Document {
            id: "part".into(),
            source: source.into(),
            sha256: source_hash(source),
            load_name: b"/captured/part.lua".to_vec(),
        });
        let Some(Halt::NeedsResolution { request }) = replay::replay(&p).unwrap().halt else {
            panic!()
        };
        p.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: "part".into(),
                loader_data: vec![],
            },
        });
        let drafts = vec![rule(
            "draft",
            "DP-1",
            [
                ("transform", Value::Int(3)),
                ("scale", Value::String("auto".into())),
            ],
        )];
        let review = monitor_review::prepare(&p, &drafts).unwrap();
        let result = monitor_review::resolve(
            &p,
            &drafts,
            &review,
            &Choice::WriteDraft {
                draft_id: "draft".into(),
                event_order: 1,
            },
        )
        .unwrap();
        let kept = keep(&p, &result);
        assert_eq!(kept.documents[0].source, p.documents[0].source);
        assert!(kept.documents[1].source.starts_with("-- module memo\n"));
        assert_eq!(kept.bindings.len(), 1);
        assert!(replay::replay(&kept).unwrap().halt.is_none());
        assert_eq!(p.documents[1].source, source);
    }
}

#[test]
fn inserted_monitor_resolution_refuses_hidden_code_and_changed_observation_arguments() {
    let p = program("-- header\nreturn 4", BASELINE);
    let monitors = vec![rule("draft", "DP-1", [("transform", Value::Int(3))])];
    let review = monitor_review::prepare(&p, &monitors).unwrap();
    let choice = Choice::InsertDraft {
        draft_id: "draft".into(),
        document: "root".into(),
        anchor: monitor_edit::declarations::Anchor::End,
    };
    let result = monitor_review::resolve(&p, &monitors, &review, &choice).unwrap();
    let mut forged = result.clone();
    forged.plans[0].edits[0]
        .replacement
        .push_str("; unexpected()");
    forged.previews[0].after = forged.plans[0].edits[0].replacement.clone();
    assert!(monitor_review::validate_resolution(&monitors, &review, &choice, &forged).is_err());
    let mut forged = result;
    if let edits::StructuralCall::Insert { arguments, .. } =
        &mut forged.plans[0].structural_calls[0]
    {
        arguments[0] = Observed::Nil;
    } else {
        panic!()
    }
    assert!(monitor_review::validate_resolution(&monitors, &review, &choice, &forged).is_err());
}
