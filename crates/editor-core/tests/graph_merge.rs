use hyprland_editor_core::{
    lua::{
        replay::{self, edits},
        source_hash,
    },
    module_path::Context,
    source::{RecordedOwnership, SourceIdentity},
    workspace::{SourceLanguage, SourceSnapshot},
    workspace_graph::{
        self,
        merge::{self, Selection},
        revisions, Bundle, Graph, View,
    },
    AUDITED_MAIN, BASELINE,
};
use std::sync::Arc;
fn source(id: &str, path: &str, text: &str) -> SourceSnapshot {
    SourceSnapshot {
        id: id.into(),
        language: SourceLanguage::Lua,
        content: text.into(),
        origin: SourceIdentity {
            selected_path: path.as_bytes().to_vec(),
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
            sha256: source_hash(text),
            recorded_ownership: RecordedOwnership::UserOwned,
        },
    }
}
fn bundle(modules: &[(&str, &str, &str, &str)]) -> Bundle {
    let mut sources = vec![source("root", "/captured/main.lua", "return 1")];
    let mut views = vec![View {
        document: "root".into(),
        load_name: b"/captured/main.lua".to_vec(),
    }];
    for (id, path, name, text) in modules {
        sources.push(source(id, path, text));
        views.push(View {
            document: (*id).into(),
            load_name: name.as_bytes().to_vec(),
        });
    }
    Bundle {
        graph: Graph {
            format: "hye-workspace-graph-1".into(),
            schema_sha256: workspace_graph::schema_sha256(),
            compiler: replay::compiler_profile(),
            root: "root".into(),
            context: Context {
                main_file: b"/captured/main.lua".to_vec(),
                home: None,
                inherited_package_path: None,
            },
            working_directory: None,
            bindings: vec![],
            views,
            revisions: vec![],
        },
        sources: Arc::new(sources),
    }
}
fn recommended(review: &merge::Review) -> Selection {
    Selection {
        pairs: review
            .rows
            .iter()
            .map(|r| (r.old.clone(), r.recommended.clone()))
            .collect(),
        discard_private: Default::default(),
        accept_context_change: review.context_changed,
    }
}
#[test]
fn correspondence_uses_logical_paths_and_load_views_instead_of_sequential_module_ids() {
    for version in [BASELINE, AUDITED_MAIN] {
        let old = bundle(&[
            (
                "module-1",
                "/captured/part.lua",
                "/captured/part.lua",
                "return 2",
            ),
            (
                "module-2",
                "/captured/other.lua",
                "/captured/other.lua",
                "return 3",
            ),
        ]);
        let incoming = bundle(&[
            (
                "module-1",
                "/captured/new.lua",
                "/captured/new.lua",
                "return 4",
            ),
            (
                "module-2",
                "/captured/part.lua",
                "/captured/part.lua",
                "return 5",
            ),
            (
                "module-3",
                "/captured/other.lua",
                "/captured/other.lua",
                "return 3",
            ),
        ]);
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let selected = recommended(&review);
        assert_eq!(selected.pairs["module-1"], Some("module-2".into()));
        assert_eq!(selected.pairs["module-2"], Some("module-3".into()));
        merge::validate_selection(&review, &selected).unwrap();
        let inputs = merge::file_inputs(&old, &incoming, &review, &selected, "module-1").unwrap();
        assert_eq!(inputs.base, "return 2");
        assert_eq!(inputs.private, "return 2");
        assert_eq!(inputs.incoming, "return 5");
        assert!(
            !review.syntax_verified
                && !review.dependency_replay_verified
                && !review.ownership_verified
        );
    }
}
#[test]
fn removed_private_views_need_explicit_discard_and_ambiguous_or_relocated_views_need_mapping() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut old = bundle(&[(
            "module-1",
            "/captured/part.lua",
            "/captured/part.lua",
            "return 2",
        )]);
        let base = old
            .graph
            .program(&old.sources, Some("root"), version)
            .unwrap();
        let doc = &base.documents[1];
        let plan = edits::prepare(
            &base,
            vec![edits::SourceEdit {
                document: doc.id.clone(),
                base_sha256: doc.sha256.clone(),
                span: 7..8,
                old_bytes_sha256: source_hash("2"),
                replacement: "3".into(),
                semantic_targets: vec!["private".into()],
            }],
            vec![],
        )
        .unwrap();
        old.graph.revisions = revisions::prepare_pieces(&base, &[], &plan).unwrap();
        let incoming = bundle(&[]);
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let mut selected = recommended(&review);
        assert!(merge::validate_selection(&review, &selected).is_err());
        selected.discard_private.insert("module-1".into());
        merge::validate_selection(&review, &selected).unwrap();
        assert!(merge::file_inputs(&old, &incoming, &review, &selected, "module-1").is_err());
        let incoming = bundle(&[(
            "module-4",
            "/relocated/part.lua",
            "/relocated/part.lua",
            "return 4",
        )]);
        let review = merge::prepare(&old, &incoming, version).unwrap();
        assert!(review.rows[1].recommended.is_none());
        let mut selected = recommended(&review);
        selected
            .pairs
            .insert("module-1".into(), Some("module-4".into()));
        merge::validate_selection(&review, &selected).unwrap();
        let inputs = merge::file_inputs(&old, &incoming, &review, &selected, "module-1").unwrap();
        assert_eq!(inputs.private, "return 3");
        let incoming = bundle(&[
            (
                "one",
                "/captured/part.lua",
                "/captured/part.lua",
                "return 2",
            ),
            (
                "two",
                "/captured/part.lua",
                "/captured/part.lua",
                "return 2",
            ),
        ]);
        let review = merge::prepare(&old, &incoming, version).unwrap();
        assert_eq!(review.rows[1].candidates.len(), 2);
        assert!(review.rows[1].recommended.is_none());
    }
}
#[test]
fn context_root_forged_review_and_counterpart_reuse_are_explicitly_guarded() {
    for version in [BASELINE, AUDITED_MAIN] {
        let old = bundle(&[
            (
                "one",
                "/captured/part.lua",
                "/captured/part.lua",
                "return 2",
            ),
            (
                "two",
                "/captured/part.lua",
                "/different/load.lua",
                "return 2",
            ),
        ]);
        let mut incoming = old.clone();
        incoming.graph.context.home = Some(b"/different-home".to_vec());
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let mut selected = recommended(&review);
        selected.accept_context_change = false;
        assert!(merge::validate_selection(&review, &selected).is_err());
        selected.accept_context_change = true;
        merge::validate_selection(&review, &selected).unwrap();
        selected.pairs.insert("two".into(), Some("one".into()));
        assert!(merge::validate_selection(&review, &selected).is_err());
        let selected = recommended(&review);
        let mut forged = review.clone();
        forged.old_views[0].private_sha256 = "0".repeat(64);
        assert!(merge::file_inputs(&old, &incoming, &forged, &selected, "root").is_err());
        let mut selected = recommended(&review);
        selected.pairs.insert("root".into(), None);
        assert!(merge::validate_selection(&review, &selected).is_err());
    }
}
