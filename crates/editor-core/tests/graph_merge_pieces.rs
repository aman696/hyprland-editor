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
        accept_context_change: review.context_changed,
    }
}
const MAIN: &str = "local part=require('part');hl.config{general={border_size=part,gaps_in=2}}\n";
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
