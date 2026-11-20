use hyprland_editor_core::{
    lua::{
        replay::{
            edits,
            monitor_edit::declarations::{self, Anchor, Change},
            *,
        },
        source_hash,
    },
    monitor, Value, AUDITED_MAIN, BASELINE,
};
fn doc(id: &str, source: &str) -> Document {
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
        documents: vec![doc("main", source)],
        bindings: vec![],
        initial_package_path: Some(b"<capture>/?.lua".to_vec()),
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn rule() -> monitor::Rule {
    let mut r = monitor::Rule::new("source-rule");
    r.output = monitor::Output::Connector("DP-1".into());
    r.fields.insert("transform".into(), Value::Int(2));
    r
}
fn add(p: &Program, anchor: Anchor) -> hyprland_editor_core::lua::replay::monitor_edit::Comparison {
    declarations::compare(
        p,
        &Change::Insert {
            document: "main".into(),
            anchor,
            rule: rule(),
        },
    )
    .unwrap()
}
#[test]
fn first_declaration_can_be_inserted_in_empty_commented_or_returning_captures() {
    for version in [BASELINE, AUDITED_MAIN] {
        for source in [
            "",
            "-- keep café",
            "-- keep\r\nreturn 7",
            "local x=1;return x",
        ] {
            let p = program(source, version);
            let result = add(&p, Anchor::End);
            assert!(
                result.review.modeled_observations_match,
                "{source}: {:?}",
                result.review.differences
            );
            assert_eq!(result.review.structural_calls_matched, 1);
            assert!(!result.review.runtime_verified && !result.review.ownership_verified);
            if source.contains("return") {
                assert!(
                    result.review.revisions[0]
                        .content
                        .find("hl.monitor")
                        .unwrap()
                        < result.review.revisions[0].content.find("return").unwrap()
                );
            }
            if source.contains("-- keep") {
                assert!(result.review.revisions[0].content.contains("-- keep"));
            }
        }
    }
}
#[test]
fn insertion_tracks_repeated_sites_and_retained_calls_and_branch_boundaries() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p=program("for i=1,3 do if i>1 then hl.config{general={border_size=i}} end end;hl.config{general={border_size=4}}",version);
        let sites = declarations::inspect(&p, "main").unwrap();
        let site = sites
            .statements
            .iter()
            .find(|s| {
                &p.documents[0].source[s.span.clone()] == "hl.config{general={border_size=i}}"
            })
            .unwrap();
        let result = add(
            &p,
            Anchor::Before {
                span: site.span.clone(),
            },
        );
        assert!(
            result.review.modeled_observations_match,
            "{:?}",
            result.review.differences
        );
        assert_eq!(result.review.structural_calls_matched, 1);
        let site = sites
            .statements
            .iter()
            .find(|s| s.kind == "return_statement");
        assert!(site.is_none());
        let p = program(
            "hl.config{general={border_size=1}};hl.config{general={border_size=2}}",
            version,
        );
        let result = add(&p, Anchor::Beginning);
        assert!(
            result.review.modeled_observations_match,
            "{:?}",
            result.review.differences
        );
    }
}
#[test]
fn standalone_and_embedded_removal_keep_comments_zero_result_arity_and_other_calls() {
    for version in [BASELINE, AUDITED_MAIN] {
        for source in [
            "hl.monitor{output='DP-1', -- true comment\n mode='text\\n-- not comment'};hl.config{general={border_size=1}}",
            "return hl.monitor{output='DP-1'}",
            "local x,y=1,hl.monitor{output='DP-1'};hl.config{general={border_size=x}}",
            "for i=1,3 do hl.monitor{output='DP-1'};hl.config{general={border_size=i}} end",
        ] {
            let p=program(source,version);let sites=declarations::inspect(&p,"main").unwrap();
            let site=&sites.declarations[0];
            let result=declarations::compare(&p,&Change::Remove {document:"main".into(),span:site.span.clone()}).unwrap();
            assert!(result.review.modeled_observations_match,"{source}: {:?}",result.review.differences);
            assert_eq!(result.review.structural_calls_matched,1);
            assert!(!result.review.revisions[0].content.contains("hl.monitor"));
            assert!(!result.review.revisions[0].content.contains("not comment"));
            if source.contains("-- true comment") {assert!(result.review.revisions[0].content.contains("-- true comment"));}
        }
    }
}
#[test]
fn unobserved_declarations_are_visible_and_insertion_into_unreached_code_is_reported() {
    let p = program(
        "local function later() hl.monitor{output='DP-1'} end;hl.config{general={border_size=1}}",
        BASELINE,
    );
    let sites = declarations::inspect(&p, "main").unwrap();
    assert_eq!(sites.declarations.len(), 1);
    assert_eq!(sites.declarations[0].observed, 0);
    let result = declarations::compare(
        &p,
        &Change::Remove {
            document: "main".into(),
            span: sites.declarations[0].span.clone(),
        },
    )
    .unwrap();
    assert!(result.review.modeled_observations_match);
    let result = add(
        &p,
        Anchor::Before {
            span: sites.declarations[0].span.clone(),
        },
    );
    assert!(!result.review.modeled_observations_match);
    assert!(result
        .review
        .differences
        .iter()
        .any(|d| d.kind == edits::DifferenceKind::RequestedDeclaration));
}
#[test]
fn retained_module_loads_and_module_calls_are_not_misidentified_by_shifted_ordinals() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p=program("local value=require('part');if value==3 then hl.config{general={border_size=value}}end",version);
        p.documents
            .push(doc("part", "hl.config{general={border_size=2}};return 3"));
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
        let result = add(&p, Anchor::Beginning);
        assert!(
            result.review.modeled_observations_match,
            "{:?}",
            result.review.differences
        );
        let changed = declarations::compare(
            &p,
            &Change::Insert {
                document: "part".into(),
                anchor: Anchor::End,
                rule: rule(),
            },
        )
        .unwrap();
        assert!(
            changed.review.modeled_observations_match,
            "{:?}",
            changed.review.differences
        );
    }
}
#[test]
fn argument_side_effects_bad_anchors_and_invalid_or_unbound_structural_requests_remain_visible() {
    let p=program("hl.monitor{output='DP-1',outside=hl.config{general={border_size=2}}};hl.config{general={border_size=3}}",BASELINE);
    let sites = declarations::inspect(&p, "main").unwrap();
    let result = declarations::compare(
        &p,
        &Change::Remove {
            document: "main".into(),
            span: sites.declarations[0].span.clone(),
        },
    )
    .unwrap();
    assert!(
        !result.review.modeled_observations_match,
        "Removing argument effects must not be classified as only removing monitor dispatch"
    );
    let p = program("return 2", BASELINE);
    let sites = declarations::inspect(&p, "main").unwrap();
    assert!(declarations::compare(
        &p,
        &Change::Insert {
            document: "main".into(),
            anchor: Anchor::After {
                span: sites.statements[0].span.clone()
            },
            rule: rule()
        }
    )
    .is_err());
    let result = add(&p, Anchor::Beginning);
    let mut plan = result.plan;
    let edits::StructuralCall::Insert { arguments, .. } = &mut plan.structural_calls[0] else {
        panic!()
    };
    *arguments = vec![Observed::Int(123)];
    assert!(!edits::review(&p, &plan).unwrap().modeled_observations_match);
    plan.structural_calls.push(plan.structural_calls[0].clone());
    assert!(edits::review(&p, &plan).is_err());
}
#[test]
fn removed_loader_arguments_leave_original_capture_evidence_and_do_not_supply_new_resolutions() {
    use hyprland_editor_core::workspace_graph::revisions;
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = program(
            "hl.monitor{output='DP-1',outside=require('part')};hl.config{general={border_size=3}}",
            version,
        );
        p.documents
            .push(doc("part", "local child=require('child');return child"));
        p.documents.push(doc("child", "return 2"));
        loop {
            let r = replay(&p).unwrap();
            let Some(Halt::NeedsResolution { request }) = r.halt else {
                break;
            };
            let target = if request.document == "main" {
                "part"
            } else {
                "child"
            };
            p.bindings.push(Binding {
                request,
                resolution: Resolution::Loaded {
                    document: target.into(),
                    loader_data: vec![],
                },
            });
        }
        let original = serde_json::to_vec(&p).unwrap();
        let site = declarations::inspect(&p, "main").unwrap().declarations[0].clone();
        let result = declarations::compare(
            &p,
            &Change::Remove {
                document: "main".into(),
                span: site.span,
            },
        )
        .unwrap();
        assert!(
            !result.review.modeled_observations_match,
            "Removed loader side effects require review"
        );
        assert!(result
            .review
            .differences
            .iter()
            .any(|d| d.kind == edits::DifferenceKind::Loader));
        assert!(
            !result.review.differences.iter().any(|d| matches!(
                d.kind,
                edits::DifferenceKind::MissingCall | edits::DifferenceKind::AddedCall
            )),
            "Unchanged retained calls must align despite removed module loads"
        );
        let pieces = revisions::apply(&p, &[], &result.plan).unwrap();
        let next = revisions::program(p.clone(), &pieces).unwrap();
        assert!(next.bindings.is_empty());
        assert_eq!(next.documents.len(), 3);
        assert_eq!(serde_json::to_vec(&p).unwrap(), original);
        let added = add(&next, Anchor::Beginning);
        assert!(
            added.review.modeled_observations_match,
            "A following source operation must remain available"
        );
        // Ordinary value plans cannot silently reuse or drop a changed loader.
        let at = p.documents[0].source.find("'part'").unwrap();
        let plan = edits::prepare(
            &p,
            vec![edits::SourceEdit {
                document: "main".into(),
                base_sha256: p.documents[0].sha256.clone(),
                span: at..at + 6,
                old_bytes_sha256: source_hash("'part'"),
                replacement: "'other'".into(),
                semantic_targets: vec!["fixture".into()],
            }],
            vec![],
        )
        .unwrap();
        assert!(edits::review(&p, &plan).is_err());
        assert!(!pieces.is_empty());
    }
}
#[test]
fn overlapping_removal_intentions_cannot_count_the_same_observation_twice() {
    let p = program("hl.monitor{output='DP-1'}", BASELINE);
    let site = declarations::inspect(&p, "main").unwrap().declarations[0].clone();
    let result = declarations::compare(
        &p,
        &Change::Remove {
            document: "main".into(),
            span: site.span,
        },
    )
    .unwrap();
    let event = replay(&p).unwrap().events.remove(0);
    let mut plan = result.plan;
    plan.structural_calls.push(edits::StructuralCall::Remove {
        order: event.order,
        api: event.api,
        arguments: event.arguments,
    });
    assert!(edits::review(&p, &plan)
        .unwrap_err()
        .contains("Overlapping"));
    plan.structural_calls.reverse();
    assert!(edits::review(&p, &plan)
        .unwrap_err()
        .contains("Overlapping"));
}
