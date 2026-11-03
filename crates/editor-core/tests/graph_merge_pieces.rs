//! Graph merge fragments preserve captured dependency provenance.
use hyprland_editor_core::{
    lua::{
        self,
        replay::{self, *},
    },
    module_path::Context,
    source::{RecordedOwnership, SourceIdentity},
    source_merge,
    workspace::{SourceLanguage, SourceSnapshot},
    workspace_graph::{
        self,
        merge::{self, Decision},
        private::Inputs,
        revisions::{self, Buffer, Piece, Revision},
        Bundle, Graph, View,
    },
    AUDITED_MAIN, BASELINE,
};
use std::{collections::BTreeMap, sync::Arc};
fn source(id: &str, text: &str) -> SourceSnapshot {
    SourceSnapshot {
        id: id.into(),
        language: SourceLanguage::Lua,
        content: text.into(),
        origin: SourceIdentity {
            selected_path: format!("/captured/{id}.lua").into_bytes(),
            resolved_path: None,
            links: vec![],
            device: 1,
            inode: 2,
            owner: 1000,
            mode: 0o100600,
            size: text.len() as u64,
            modified_seconds: 0,
            modified_nanoseconds: 0,
            changed_seconds: 0,
            changed_nanoseconds: 0,
            sha256: lua::source_hash(text),
            recorded_ownership: RecordedOwnership::UserOwned,
        },
    }
}
fn bundle(version: &str, main: &str, part: &str) -> Bundle {
    let sources = vec![source("root", main), source("part", part)];
    let context = Context {
        main_file: sources[0].origin.selected_path.clone(),
        home: None,
        inherited_package_path: None,
    };
    let graph = Graph {
        private: Default::default(),
        revisions: vec![],
        format: "hye-workspace-graph-1".into(),
        schema_sha256: workspace_graph::schema_sha256(),
        compiler: replay::compiler_profile(),
        root: "root".into(),
        context,
        working_directory: None,
        views: sources
            .iter()
            .map(|s| View {
                document: s.id.clone(),
                load_name: s.origin.selected_path.clone(),
            })
            .collect(),
        bindings: vec![],
    };
    let mut b = Bundle {
        graph,
        sources: Arc::new(sources),
    };
    let program = b.graph.program(&b.sources, Some("root"), version).unwrap();
    let Some(Halt::NeedsResolution { request }) = replay::replay(&program).unwrap().halt else {
        panic!("module request")
    };
    b.graph.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: b.sources[1].origin.selected_path.clone(),
        },
    });
    assert!(
        replay::replay(&b.graph.program(&b.sources, Some("root"), version).unwrap())
            .unwrap()
            .halt
            .is_none()
    );
    b
}
fn revise(b: &mut Bundle, version: &str, id: &str, from: &str, to: &str) {
    let p = b.graph.program(&b.sources, Some("root"), version).unwrap();
    let current = revisions::program(p.clone(), &b.graph.revisions).unwrap();
    let d = current.documents.iter().find(|d| d.id == id).unwrap();
    let start = d.source.find(from).unwrap();
    let plan = edits::prepare(
        &current,
        vec![edits::SourceEdit {
            document: id.into(),
            base_sha256: d.sha256.clone(),
            span: start..start + from.len(),
            old_bytes_sha256: lua::source_hash(from),
            replacement: to.into(),
            semantic_targets: vec!["private".into()],
        }],
        vec![],
    )
    .unwrap();
    b.graph.revisions = revisions::prepare_pieces(&p, &b.graph.revisions, &plan).unwrap();
}
fn decisions(
    old: &Bundle,
    incoming: &Bundle,
    review: &merge::Review,
    selection: &merge::Selection,
    choice: source_merge::Choice,
) -> BTreeMap<String, Decision> {
    selection
        .pairs
        .iter()
        .filter(|(_, v)| v.is_some())
        .map(|(id, _)| {
            let inputs = merge::file_inputs(old, incoming, review, selection, id).unwrap();
            let r = inputs.text_review().unwrap();
            let choices = r
                .regions
                .iter()
                .enumerate()
                .filter(|(_, r)| r.conflict.is_some())
                .map(|(i, _)| (i, choice))
                .collect();
            (id.clone(), Decision { review: r, choices })
        })
        .collect()
}
fn selection(review: &merge::Review) -> merge::Selection {
    merge::Selection {
        pairs: review
            .rows
            .iter()
            .map(|r| (r.old.clone(), r.recommended.clone()))
            .collect(),
        discard_private: Default::default(),
        archive_private: Default::default(),
        accept_context_change: review.context_changed,
    }
}
const MAIN: &str = "local part=require('part');hl.config{general={border_size=part,gaps_in=2}}\n";
fn private_bundle(version: &str) -> Bundle {
    let mut b = bundle(version, MAIN, "return 3\n");
    let base = b.graph.program(&b.sources, Some("root"), version).unwrap();
    let text = MAIN.replace("'part'", "'extra'");
    b.graph.revisions.push(Revision {
        document: "root".into(),
        base_sha256: lua::source_hash(MAIN),
        sha256: lua::source_hash(&text),
        buffers: vec![Arc::from(text.as_str())],
        pieces: vec![Piece {
            buffer: Buffer::Inserted { index: 0 },
            span: 0..text.len(),
        }],
    });
    let pending = Inputs::default()
        .pending_program(&base, &b.graph.revisions)
        .unwrap();
    let Some(Halt::NeedsResolution { request }) = replay::replay(&pending).unwrap().halt else {
        panic!("private request")
    };
    let extra = source("extra", "return 7\n");
    let archive = source("unused", "invalid archived Lua =");
    b.graph.private = Inputs {
        views: vec![
            View {
                document: extra.id.clone(),
                load_name: extra.origin.selected_path.clone(),
            },
            View {
                document: archive.id.clone(),
                load_name: b"@historical-archive".to_vec(),
            },
        ],
        sources: Arc::new(vec![extra.clone(), archive]),
        bindings: vec![Binding {
            request,
            resolution: Resolution::Loaded {
                document: extra.id,
                loader_data: extra.origin.selected_path,
            },
        }],
        archived: ["unused".into()].into(),
    };
    let current = b.graph.private.program(&base, &b.graph.revisions).unwrap();
    let report = replay::replay(&current).unwrap();
    b.graph.private.check_reached(&current, &report).unwrap();
    let plan = edits::prepare(
        &current,
        vec![edits::SourceEdit {
            document: "extra".into(),
            base_sha256: lua::source_hash("return 7\n"),
            span: 7..8,
            old_bytes_sha256: lua::source_hash("7"),
            replacement: "9".into(),
            semantic_targets: vec!["reviewed".into()],
        }],
        vec![],
    )
    .unwrap();
    (b.graph.revisions, b.graph.private) = b
        .graph
        .private
        .apply(&base, &b.graph.revisions, &plan)
        .unwrap();
    b
}
fn archive_unmapped(review: &merge::Review) -> merge::Selection {
    let mut s = selection(review);
    s.archive_private = review
        .old_views
        .iter()
        .filter(|v| s.pairs[&v.document].is_none() && (v.private_changed || v.private_input))
        .map(|v| v.document.clone())
        .collect();
    s
}

#[test]
fn private_and_archived_views_participate_in_full_graph_correspondence() {
    for version in [BASELINE, AUDITED_MAIN] {
        let old = private_bundle(version);
        let incoming = bundle(version, MAIN, "return 3\n");
        let review = merge::prepare(&old, &incoming, version).unwrap();
        assert_eq!(review.old_views.len(), 4);
        let extra = review
            .old_views
            .iter()
            .find(|v| v.document == "extra")
            .unwrap();
        assert!(extra.private_input && extra.private_changed && !extra.archived);
        assert_eq!(extra.captured_sha256, lua::source_hash("return 7\n"));
        assert_eq!(extra.private_sha256, lua::source_hash("return 9\n"));
        let archive = review
            .old_views
            .iter()
            .find(|v| v.document == "unused")
            .unwrap();
        assert!(archive.private_input && archive.archived && !archive.private_changed);
        let mut s = selection(&review);
        assert!(merge::validate_selection(&review, &s).is_err());
        s.archive_private.insert("extra".into());
        assert!(
            merge::validate_selection(&review, &s).is_err(),
            "unchanged captured archive still needs a choice"
        );
        s.discard_private.insert("unused".into());
        merge::validate_selection(&review, &s).unwrap();
        for id in ["root", "part", "unknown", "unused"] {
            let mut bad = s.clone();
            bad.archive_private.insert(id.into());
            assert!(merge::validate_selection(&review, &bad).is_err());
        }
    }
}

#[test]
fn private_module_mapping_merges_bytes_onto_fresh_capture_without_old_resolution_rights() {
    for version in [BASELINE, AUDITED_MAIN] {
        let old = private_bundle(version);
        let mut incoming = bundle(
            version,
            &MAIN.replace("'part'", "'extra'"),
            "return 7 -- fresh\n",
        );
        Arc::make_mut(&mut incoming.sources)[1].origin.selected_path =
            b"/captured/extra.lua".to_vec();
        incoming.graph.views[1].load_name = b"/captured/extra.lua".to_vec();
        if let Resolution::Loaded { loader_data, .. } = &mut incoming.graph.bindings[0].resolution {
            *loader_data = b"/captured/extra.lua".to_vec();
        }
        let before = incoming.fingerprint().unwrap();
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let s = archive_unmapped(&review);
        assert_eq!(s.pairs["extra"], Some("part".into()));
        assert_eq!(s.pairs["part"], None);
        let input = merge::file_inputs(&old, &incoming, &review, &s, "extra").unwrap();
        assert_eq!(input.base, "return 7\n");
        assert_eq!(input.private, "return 9\n");
        let chosen = decisions(&old, &incoming, &review, &s, source_merge::Choice::Private);
        let c = merge::compose(&old, &incoming, &review, &s, &chosen).unwrap();
        assert_eq!(incoming.fingerprint().unwrap(), before);
        assert!(c.bundle.graph.private.bindings.is_empty());
        let base = c
            .bundle
            .graph
            .program(&c.bundle.sources, Some("root"), version)
            .unwrap();
        let p = c
            .bundle
            .graph
            .private
            .program(&base, &c.bundle.graph.revisions)
            .unwrap();
        assert_eq!(
            p.documents.iter().find(|d| d.id == "part").unwrap().source,
            "return 9 -- fresh\n"
        );
        assert_eq!(
            serde_json::to_vec(&p.bindings).unwrap(),
            serde_json::to_vec(&incoming.graph.bindings).unwrap()
        );
        let report = replay::replay(&p).unwrap();
        c.bundle.graph.private.check_reached(&p, &report).unwrap();
        assert!(report.halt.is_none());
        assert_eq!(c.archived_documents["unused"], "unused");
    }
}

#[test]
fn removed_sources_archive_captured_identity_and_pieces_with_collision_free_labels() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut old = private_bundle(version);
        // Give the original module a private edit and retain every unmapped
        // view. Incoming IDs deliberately collide with both old archive labels.
        let base = old
            .graph
            .program(&old.sources, Some("root"), version)
            .unwrap();
        let plan = edits::prepare(
            &old.graph
                .private
                .program(&base, &old.graph.revisions)
                .unwrap(),
            vec![edits::SourceEdit {
                document: "part".into(),
                base_sha256: lua::source_hash("return 3\n"),
                span: 7..8,
                old_bytes_sha256: lua::source_hash("3"),
                replacement: "4".into(),
                semantic_targets: vec!["reviewed".into()],
            }],
            vec![],
        )
        .unwrap();
        (old.graph.revisions, old.graph.private) = old
            .graph
            .private
            .apply(&base, &old.graph.revisions, &plan)
            .unwrap();
        let mut incoming = bundle(version, &MAIN.replace("'part'", "'incoming'"), "return 3\n");
        Arc::make_mut(&mut incoming.sources)[1].origin.selected_path =
            b"/incoming/other.lua".to_vec();
        incoming.graph.views[1].load_name = b"/incoming/other.lua".to_vec();
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let s = archive_unmapped(&review);
        assert_eq!(s.archive_private.len(), 3);
        let chosen = decisions(&old, &incoming, &review, &s, source_merge::Choice::Incoming);
        let c = merge::compose(&old, &incoming, &review, &s, &chosen).unwrap();
        assert_ne!(c.archived_documents["part"], "part");
        assert!(c.bundle.graph.private.bindings.is_empty());
        for id in &s.archive_private {
            let target = &c.archived_documents[id];
            let before = old
                .sources
                .iter()
                .chain(old.graph.private.sources.iter())
                .find(|x| x.id == *id)
                .unwrap();
            let after = c
                .bundle
                .graph
                .private
                .sources
                .iter()
                .find(|x| &x.id == target)
                .unwrap();
            assert_eq!(after.content, before.content);
            assert_eq!(
                serde_json::to_vec(&after.origin).unwrap(),
                serde_json::to_vec(&before.origin).unwrap()
            );
            assert!(c.bundle.graph.private.archived.contains(target));
            if let Some(r) = old.graph.revisions.iter().find(|r| r.document == *id) {
                let mut expected = r.clone();
                expected.document = target.clone();
                assert_eq!(
                    serde_json::to_vec(
                        c.bundle
                            .graph
                            .revisions
                            .iter()
                            .find(|r| &r.document == target)
                            .unwrap()
                    )
                    .unwrap(),
                    serde_json::to_vec(&expected).unwrap()
                );
            }
        }
        let base = c
            .bundle
            .graph
            .program(&c.bundle.sources, Some("root"), version)
            .unwrap();
        let p = c
            .bundle
            .graph
            .private
            .program(&base, &c.bundle.graph.revisions)
            .unwrap();
        assert_eq!(p.documents.len(), 2);
        assert!(!p
            .documents
            .iter()
            .any(|d| d.source.contains("archived Lua")));
        let report = replay::replay(&p).unwrap();
        assert!(report.halt.is_none());
        c.bundle.graph.private.check_reached(&p, &report).unwrap();
    }
}

#[test]
fn archived_inputs_cannot_silently_resolve_a_preserved_private_import() {
    for version in [BASELINE, AUDITED_MAIN] {
        let old = private_bundle(version);
        let incoming = bundle(version, &MAIN.replace("'part'", "'incoming'"), "return 3\n");
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let s = archive_unmapped(&review);
        let chosen = decisions(&old, &incoming, &review, &s, source_merge::Choice::Private);
        let c = merge::compose(&old, &incoming, &review, &s, &chosen).unwrap();
        assert!(c.dependency_renewal.is_some());
        let base = c
            .bundle
            .graph
            .program(&c.bundle.sources, Some("root"), version)
            .unwrap();
        let p = c
            .bundle
            .graph
            .private
            .program(&base, &c.bundle.graph.revisions)
            .unwrap();
        assert!(p.bindings.is_empty());
        assert!(!p
            .documents
            .iter()
            .any(|d| d.id == c.archived_documents["extra"]));
        assert!(
            matches!(replay::replay(&p).unwrap().halt,Some(Halt::NeedsResolution {request}) if request.module == b"extra")
        );
    }
}

#[test]
fn explicit_discard_removes_only_reviewed_private_data_and_stale_archive_reviews_are_refused() {
    for version in [BASELINE, AUDITED_MAIN] {
        let old = private_bundle(version);
        let incoming = bundle(version, &MAIN.replace("'part'", "'incoming'"), "return 3\n");
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let mut s = archive_unmapped(&review);
        s.archive_private.remove("extra");
        s.discard_private.insert("extra".into());
        let chosen = decisions(&old, &incoming, &review, &s, source_merge::Choice::Incoming);
        let c = merge::compose(&old, &incoming, &review, &s, &chosen).unwrap();
        assert_eq!(c.bundle.graph.private.sources.len(), 1);
        assert_eq!(c.bundle.graph.private.sources[0].id, "unused");
        assert!(!c
            .bundle
            .graph
            .revisions
            .iter()
            .any(|r| r.document == "extra"));
        let mut changed = old.clone();
        changed.graph.private.views[1].load_name.push(b'x');
        assert!(merge::compose(&changed, &incoming, &review, &s, &chosen).is_err());
        let mut forged = review.clone();
        forged.old_views[3].archived = false;
        assert!(merge::compose(&old, &incoming, &forged, &s, &chosen).is_err());
        assert!(
            merge::prepare(&old, &old, version).is_err(),
            "incoming private data is not a fresh capture"
        );
    }
}

#[test]
fn archival_overflow_refuses_the_candidate_without_dropping_preserved_data() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut old = private_bundle(version);
        for n in 0..28 {
            let s = source(&format!("extra-{n}"), "return 0");
            old.graph.private.views.push(View {
                document: s.id.clone(),
                load_name: s.origin.selected_path.clone(),
            });
            old.graph.private.archived.insert(s.id.clone());
            Arc::make_mut(&mut old.graph.private.sources).push(s);
        }
        let mut incoming = bundle(version, &MAIN.replace("'part'", "'incoming'"), "return 3\n");
        let s = source("added", "return 0");
        incoming.graph.views.push(View {
            document: s.id.clone(),
            load_name: s.origin.selected_path.clone(),
        });
        Arc::make_mut(&mut incoming.sources).push(s);
        let before = old.fingerprint().unwrap();
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let s = archive_unmapped(&review);
        let chosen = decisions(&old, &incoming, &review, &s, source_merge::Choice::Incoming);
        assert!(merge::compose(&old, &incoming, &review, &s, &chosen).is_err());
        assert_eq!(old.fingerprint().unwrap(), before);
    }
}
#[test]
fn incoming_captures_stay_immutable_and_private_root_and_module_changes_replay_together() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut old = bundle(version, MAIN, "return 3\n");
        revise(&mut old, version, "root", "gaps_in=2", "gaps_in=6");
        revise(&mut old, version, "part", "3", "5");
        let incoming = bundle(
            version,
            &format!("{MAIN}-- incoming root\n"),
            "return 3 -- incoming part\n",
        );
        let before = incoming.fingerprint().unwrap();
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let selected = selection(&review);
        let chosen = decisions(
            &old,
            &incoming,
            &review,
            &selected,
            source_merge::Choice::Private,
        );
        let c = merge::compose(&old, &incoming, &review, &selected, &chosen).unwrap();
        assert_eq!(incoming.fingerprint().unwrap(), before);
        assert!(Arc::ptr_eq(&c.bundle.sources, &incoming.sources));
        assert_eq!(c.bundle.graph.revisions.len(), 2);
        assert!(c.dependency_renewal.is_none());
        let p = c
            .bundle
            .graph
            .program(&c.bundle.sources, Some("root"), version)
            .unwrap();
        let p = revisions::program(p, &c.bundle.graph.revisions).unwrap();
        assert!(p.documents[0].source.contains("gaps_in=6"));
        assert!(p.documents[0].source.ends_with("-- incoming root\n"));
        assert!(p.documents[1].source.contains("return 5 -- incoming part"));
        assert_eq!(p.bindings.len(), 1);
        assert!(replay::replay(&p).unwrap().halt.is_none());
        assert!(!c.syntax_verified && !c.dependency_replay_verified && !c.ownership_verified);
        assert_eq!(c.fingerprint, c.bundle.fingerprint().unwrap());
    }
}
#[test]
fn changed_surviving_loader_requires_renewal_instead_of_borrowing_incoming_resolution() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut old = bundle(version, MAIN, "return 3\n");
        let private = MAIN.replace("'part'", "'private'");
        old.graph.revisions = vec![Revision {
            document: "root".into(),
            base_sha256: lua::source_hash(MAIN),
            sha256: lua::source_hash(&private),
            buffers: vec![Arc::from(private.as_str())],
            pieces: vec![Piece {
                buffer: Buffer::Inserted { index: 0 },
                span: 0..private.len(),
            }],
        }];
        let incoming = bundle(version, &MAIN.replace("'part'", "'incoming'"), "return 3\n");
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let selected = selection(&review);
        let chosen = decisions(
            &old,
            &incoming,
            &review,
            &selected,
            source_merge::Choice::Private,
        );
        let c = merge::compose(&old, &incoming, &review, &selected, &chosen).unwrap();
        assert!(c
            .dependency_renewal
            .as_ref()
            .is_some_and(|e| e.contains("renewed resolution")));
        let p = c
            .bundle
            .graph
            .program(&c.bundle.sources, Some("root"), version)
            .unwrap();
        assert!(revisions::program(p, &c.bundle.graph.revisions).is_err());
    }
}
#[test]
fn composition_refuses_missing_extra_stale_or_forged_text_decisions_and_retains_no_op_identity() {
    for version in [BASELINE, AUDITED_MAIN] {
        let old = bundle(version, MAIN, "return 3\n");
        let incoming = bundle(version, MAIN, "return 3\n");
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let selected = selection(&review);
        let chosen = decisions(
            &old,
            &incoming,
            &review,
            &selected,
            source_merge::Choice::Private,
        );
        let c = merge::compose(&old, &incoming, &review, &selected, &chosen).unwrap();
        assert!(c.bundle.graph.revisions.is_empty());
        assert_eq!(c.fingerprint, incoming.fingerprint().unwrap());
        let mut bad = chosen.clone();
        bad.remove("part");
        assert!(merge::compose(&old, &incoming, &review, &selected, &bad).is_err());
        bad = chosen.clone();
        bad.insert("unknown".into(), chosen["part"].clone());
        assert!(merge::compose(&old, &incoming, &review, &selected, &bad).is_err());
        bad = chosen.clone();
        bad.get_mut("root").unwrap().review.base.sha256 = "0".repeat(64);
        assert!(merge::compose(&old, &incoming, &review, &selected, &bad).is_err());
        bad = chosen.clone();
        bad.get_mut("root")
            .unwrap()
            .choices
            .insert(0, source_merge::Choice::Base);
        assert!(merge::compose(&old, &incoming, &review, &selected, &bad).is_err());
        let mut altered = review.clone();
        altered.context_changed = !altered.context_changed;
        assert!(merge::compose(&old, &incoming, &altered, &selected, &chosen).is_err());
        assert_eq!(incoming.fingerprint().unwrap(), old.fingerprint().unwrap());
    }
}
