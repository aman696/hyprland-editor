//! Actual application reopen from frozen bytes after deleting every original.
use hyprland_editor_core::{
    catalogue,
    lua::{
        self,
        replay::{self, *},
    },
    module_path::Context,
    storage,
    workspace::{self, SourceLanguage, SourceSnapshot, Workspace},
    workspace_graph::{Bundle, Graph, View},
    Draft, Value, AUDITED_MAIN, BASELINE,
};
use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn check(path: &Path) -> (bool, String) {
    let result = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
        .arg("--check-workspace")
        .arg(path)
        .env_clear()
        .output()
        .unwrap();
    (
        result.status.success(),
        String::from_utf8(if result.status.success() {
            result.stdout
        } else {
            result.stderr
        })
        .unwrap(),
    )
}
fn fixture(root: &Path, version: &str) -> Workspace {
    fixture_source(
        root,
        version,
        "local part=require('part');hl.config{general={border_size=part}}",
    )
}
fn fixture_source(root: &Path, version: &str, source: &str) -> Workspace {
    let originals = root.join("originals");
    fs::create_dir(&originals).unwrap();
    fs::write(originals.join("main.lua"), source).unwrap();
    fs::write(originals.join("part.lua"), "return 3").unwrap();
    let main = storage::read_source(&originals.join("main.lua")).unwrap();
    let part = storage::read_source(&originals.join("part.lua")).unwrap();
    let context = Context {
        main_file: main.identity.selected_path.clone(),
        home: None,
        inherited_package_path: None,
    };
    let (path, unknown) = context.initial_path(version).unwrap();
    let mut program = Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            source: source.into(),
            sha256: lua::source_hash(source),
            load_name: context.main_file.clone(),
        }],
        bindings: vec![],
        initial_package_path: Some(path),
        initial_package_path_unknown_tail: unknown,
        working_directory: None,
    };
    let report = loop {
        let report = replay::replay(&program).unwrap();
        let Some(Halt::NeedsResolution { request }) = report.halt.clone() else {
            break report;
        };
        assert!(program.documents.len() < 4);
        let id = if program.documents.len() == 1 {
            "part".into()
        } else {
            format!("part-{}", program.documents.len())
        };
        let direct = matches!(request.loader, Loader::Dofile | Loader::Loadfile);
        let load_name = if direct {
            request.module.clone()
        } else {
            part.identity.selected_path.clone()
        };
        let loader_data = if direct { vec![] } else { load_name.clone() };
        program.documents.push(Document {
            id: id.clone(),
            source: part.content.clone(),
            sha256: part.identity.sha256.clone(),
            load_name,
        });
        program.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: id,
                loader_data,
            },
        });
    };
    let sources = program
        .documents
        .iter()
        .map(|d| {
            let captured = if d.id == "root" { &main } else { &part };
            SourceSnapshot {
                id: d.id.clone(),
                language: SourceLanguage::Lua,
                content: captured.content.clone(),
                origin: captured.identity.clone(),
            }
        })
        .collect();
    let bundle = Bundle {
        graph: Graph {
            private: Default::default(),
            revisions: vec![],
            format: "hye-workspace-graph-1".into(),
            schema_sha256: hyprland_editor_core::workspace_graph::schema_sha256(),
            compiler: report.compiler_profile,
            root: "root".into(),
            context,
            working_directory: None,
            views: program
                .documents
                .iter()
                .map(|d| View {
                    document: d.id.clone(),
                    load_name: d.load_name.clone(),
                })
                .collect(),
            bindings: program.bindings,
        },
        sources: std::sync::Arc::new(sources),
    };
    let mut draft = Draft::new(version);
    draft
        .values
        .insert("general:border_size".into(), Value::Int(5));
    let workspace = Workspace::new(draft).with_graph(&bundle);
    fs::remove_dir_all(originals).unwrap();
    workspace
}
fn root() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "he-graph-reopen-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    root
}
#[test]
fn removed_declaration_dependencies_remain_captured_and_reopen_with_further_source_edits() {
    use hyprland_editor_core::{
        lua::replay::monitor_edit::declarations::{self, Anchor, Change},
        monitor,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let mut snapshot = fixture_source(
            &root,
            version,
            "hl.monitor{output='DP-1',outside=require('part')};hl.config{general={border_size=2}}",
        );
        let graph = snapshot.source_graph.as_ref().unwrap();
        let base = graph
            .program(&snapshot.sources, snapshot.root_source.as_deref(), version)
            .unwrap();
        let original_sources = serde_json::to_value(&snapshot.sources).unwrap();
        let site = declarations::inspect(&base, "root").unwrap().declarations[0].clone();
        let comparison = declarations::compare(
            &base,
            &Change::Remove {
                document: "root".into(),
                span: site.span,
            },
        )
        .unwrap();
        assert!(!comparison.review.modeled_observations_match);
        let revisions =
            hyprland_editor_core::workspace_graph::revisions::apply(&base, &[], &comparison.plan)
                .unwrap();
        let changed =
            hyprland_editor_core::workspace_graph::revisions::program(base.clone(), &revisions)
                .unwrap();
        assert!(changed.bindings.is_empty());
        assert_eq!(changed.documents.len(), 2);
        let mut rule = monitor::Rule::new("new-source");
        rule.output = monitor::Output::Connector("DP-2".into());
        let next = declarations::compare(
            &changed,
            &Change::Insert {
                document: "root".into(),
                anchor: Anchor::End,
                rule,
            },
        )
        .unwrap();
        let revisions =
            hyprland_editor_core::workspace_graph::revisions::apply(&base, &revisions, &next.plan)
                .unwrap();
        snapshot.source_graph.as_mut().unwrap().revisions = revisions;
        let saved = workspace::save(&root, &snapshot, &catalogue()).unwrap();
        let (ok, output) = check(&saved);
        assert!(ok, "{output}");
        let result: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(result["original_paths_opened"], false);
        assert_eq!(result["folder_access_restored"], false);
        let reopened = workspace::read(&saved, &catalogue()).unwrap();
        assert_eq!(
            serde_json::to_value(&reopened.sources).unwrap(),
            original_sources
        );
        assert_eq!(
            reopened.source_graph.as_ref().unwrap().bindings.len(),
            1,
            "Original dependency evidence must remain saved"
        );
        assert!(!root.join("originals").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
fn private_fixture(root: &Path, version: &str) -> Workspace {
    use hyprland_editor_core::workspace_graph::{
        private::Inputs,
        revisions::{Buffer, Piece, Revision},
    };
    let mut snapshot = fixture(root, version);
    let base = snapshot
        .source_graph
        .as_ref()
        .unwrap()
        .program(&snapshot.sources, Some("root"), version)
        .unwrap();
    let doc = &base.documents[0];
    let at = doc.source.find("'part'").unwrap() + 1;
    let content = format!("{}new{}", &doc.source[..at], &doc.source[at + 4..]);
    let revision = Revision {
        document: doc.id.clone(),
        base_sha256: doc.sha256.clone(),
        sha256: lua::source_hash(&content),
        buffers: vec![std::sync::Arc::from("new")],
        pieces: vec![
            Piece {
                buffer: Buffer::Original,
                span: 0..at,
            },
            Piece {
                buffer: Buffer::Inserted { index: 0 },
                span: 0..3,
            },
            Piece {
                buffer: Buffer::Original,
                span: at + 4..doc.source.len(),
            },
        ],
    };
    let input = root.join("new.lua");
    fs::write(&input, "return 7").unwrap();
    let capture = storage::read_source(&input).unwrap();
    fs::remove_file(input).unwrap();
    let source = SourceSnapshot {
        id: "private-new".into(),
        language: SourceLanguage::Lua,
        content: capture.content,
        origin: capture.identity,
    };
    let mut inputs = Inputs {
        sources: std::sync::Arc::new(vec![source.clone()]),
        views: vec![View {
            document: source.id.clone(),
            load_name: source.origin.selected_path.clone(),
        }],
        ..Inputs::default()
    };
    let pending = inputs
        .program(&base, std::slice::from_ref(&revision))
        .unwrap();
    let Some(Halt::NeedsResolution { request }) = replay::replay(&pending).unwrap().halt else {
        panic!("private loader request")
    };
    inputs.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: source.id.clone(),
            loader_data: source.origin.selected_path.clone(),
        },
    });
    snapshot.source_graph.as_mut().unwrap().revisions = vec![revision];
    snapshot.source_graph.as_mut().unwrap().private = inputs;
    snapshot
}
#[test]
fn private_dependency_inputs_reopen_and_reconstruct_preview_layers_without_original_paths() {
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let snapshot = private_fixture(&root, version);
        let base = snapshot
            .source_graph
            .as_ref()
            .unwrap()
            .program(&snapshot.sources, Some("root"), version)
            .unwrap();
        let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
        let bytes = fs::read(&path).unwrap();
        let (ok, output) = check(&path);
        assert!(ok, "{output}");
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["source_graph_private_documents"], 1);
        assert_eq!(value["source_graph_private_bindings"], 1);
        assert_eq!(value["source_graph_archived_private_documents"], 0);
        assert_eq!(value["original_paths_opened"], false);
        assert_eq!(value["folder_access_restored"], false);
        let result = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
            .arg("--check-preview-layers")
            .arg(&path)
            .env_clear()
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let review: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        let row = review["review"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["setting_id"] == "general:border_size")
            .unwrap();
        assert_eq!(
            row["captured"]["value"],
            serde_json::to_value(Value::Int(7)).unwrap()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(!root.join("originals").exists());
        let graph = snapshot.source_graph.as_ref().unwrap();
        let current = graph.private.program(&base, &graph.revisions).unwrap();
        let request = graph.private.bindings[0].request.clone();
        let replace = edits::prepare(
            &current,
            vec![edits::SourceEdit {
                document: request.document.clone(),
                base_sha256: request.sha256.clone(),
                span: request.span.clone(),
                old_bytes_sha256: lua::source_hash(
                    &current
                        .documents
                        .iter()
                        .find(|d| d.id == request.document)
                        .unwrap()
                        .source[request.span],
                ),
                replacement: "7".into(),
                semantic_targets: vec!["general:border_size".into()],
            }],
            vec![],
        )
        .unwrap();
        let (revisions, inputs) = graph
            .private
            .apply(&base, &graph.revisions, &replace)
            .unwrap();
        let mut archived = snapshot.clone();
        let graph = archived.source_graph.as_mut().unwrap();
        graph.revisions = revisions;
        graph.private = inputs;
        let archive_path = root.join("archived-private.hye");
        fs::write(&archive_path, archived.to_json(&catalogue()).unwrap()).unwrap();
        let (ok, output) = check(&archive_path);
        assert!(ok, "{output}");
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["source_graph_private_documents"], 1);
        assert_eq!(value["source_graph_private_bindings"], 0);
        assert_eq!(value["source_graph_archived_private_documents"], 1);
        let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for mutation in 0..4 {
            let mut value = original.clone();
            match mutation {
                0 => {
                    value["source_graph"]["private"]["bindings"][0]["request"]["module"] =
                        serde_json::json!([b'x'])
                }
                1 => value["source_graph"]["private"]["sources"][0]["content"] = "return 99".into(),
                2 => {
                    value["source_graph"]["private"]["archived"] =
                        serde_json::json!(["private-new"])
                }
                _ => value["source_graph"]["private"]["bindings"] = serde_json::json!([]),
            }
            let bad = root.join("bad-private.hye");
            fs::write(&bad, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(!check(&bad).0, "Accepted changed private input {mutation}");
        }
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn composed_incoming_graph_reopens_in_actual_application_after_both_capture_trees_are_deleted() {
    use hyprland_editor_core::workspace_graph::{merge, revisions};
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let old_folder = root.join("old");
        let new_folder = root.join("incoming");
        fs::create_dir(&old_folder).unwrap();
        fs::create_dir(&new_folder).unwrap();
        let old_workspace = fixture(&old_folder, version);
        let incoming_workspace = fixture_source(&new_folder, version,
            "local part=require('part');hl.config{general={border_size=part}} -- incoming capture\n");
        let mut old = Bundle {
            graph: old_workspace.source_graph.unwrap(),
            sources: std::sync::Arc::new(old_workspace.sources),
        };
        let incoming = Bundle {
            graph: incoming_workspace.source_graph.unwrap(),
            sources: std::sync::Arc::new(incoming_workspace.sources),
        };
        let base = old
            .graph
            .program(&old.sources, Some("root"), version)
            .unwrap();
        let part = &base.documents[1];
        let start = part.source.find('3').unwrap();
        let plan = edits::prepare(
            &base,
            vec![edits::SourceEdit {
                document: part.id.clone(),
                base_sha256: part.sha256.clone(),
                span: start..start + 1,
                old_bytes_sha256: lua::source_hash("3"),
                replacement: "7".into(),
                semantic_targets: vec!["general:border_size".into()],
            }],
            vec![],
        )
        .unwrap();
        old.graph.revisions = revisions::apply(&base, &[], &plan).unwrap();
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let selection = merge::Selection {
            pairs: old
                .sources
                .iter()
                .map(|s| (s.id.clone(), Some(s.id.clone())))
                .collect(),
            discard_private: Default::default(),
            archive_private: Default::default(),
            accept_context_change: true,
        };
        let decisions = old
            .sources
            .iter()
            .map(|s| {
                let input =
                    merge::file_inputs(&old, &incoming, &review, &selection, &s.id).unwrap();
                let review = input.text_review().unwrap();
                assert!(review.regions.iter().all(|r| r.conflict.is_none()));
                (
                    s.id.clone(),
                    merge::Decision {
                        review,
                        choices: Default::default(),
                    },
                )
            })
            .collect();
        let composed = merge::compose(&old, &incoming, &review, &selection, &decisions).unwrap();
        assert!(composed.dependency_renewal.is_none());
        let saved = Workspace::new(Draft::new(version)).with_graph(&composed.bundle);
        let path = workspace::save(&root, &saved, &catalogue()).unwrap();
        let (ok, output) = check(&path);
        assert!(ok, "{output}");
        let result: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(result["source_graph_replayed"], true);
        assert_eq!(result["source_graph_revisions"], 1);
        assert_eq!(result["original_paths_opened"], false);
        assert_eq!(result["folder_access_restored"], false);
        let reopened = workspace::read(&path, &catalogue()).unwrap();
        assert_eq!(
            serde_json::to_vec(&reopened.sources).unwrap(),
            serde_json::to_vec(incoming.sources.as_ref()).unwrap()
        );
        assert!(!old_folder.join("originals").exists());
        assert!(!new_folder.join("originals").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn external_merge_preserves_private_captures_edits_and_archives_through_actual_reopen_and_staging()
{
    use hyprland_editor_core::workspace_graph::merge;
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let old_folder = root.join("old");
        let new_folder = root.join("incoming");
        fs::create_dir(&old_folder).unwrap();
        fs::create_dir(&new_folder).unwrap();
        let old_workspace = private_fixture(&old_folder, version);
        let incoming_workspace = fixture_source(
            &new_folder,
            version,
            "local part=require('incoming');hl.config{general={border_size=part}} -- fresh\n",
        );
        let mut old = Bundle {
            graph: old_workspace.source_graph.unwrap(),
            sources: std::sync::Arc::new(old_workspace.sources),
        };
        let incoming = Bundle {
            graph: incoming_workspace.source_graph.unwrap(),
            sources: std::sync::Arc::new(incoming_workspace.sources),
        };
        let base = old
            .graph
            .program(&old.sources, Some("root"), version)
            .unwrap();
        let current = old
            .graph
            .private
            .program(&base, &old.graph.revisions)
            .unwrap();
        let change = edits::prepare(
            &current,
            vec![edits::SourceEdit {
                document: "private-new".into(),
                base_sha256: lua::source_hash("return 7"),
                span: 7..8,
                old_bytes_sha256: lua::source_hash("7"),
                replacement: "9".into(),
                semantic_targets: vec!["general:border_size".into()],
            }],
            vec![],
        )
        .unwrap();
        (old.graph.revisions, old.graph.private) = old
            .graph
            .private
            .apply(&base, &old.graph.revisions, &change)
            .unwrap();
        // An invalid historical file must survive as inert data without being
        // compiled, evaluated or rediscovered through its removed live path.
        let path = old_folder.join("unused.lua");
        fs::write(&path, "this archived Lua is invalid =").unwrap();
        let archived = storage::read_source(&path).unwrap();
        fs::remove_file(path).unwrap();
        std::sync::Arc::make_mut(&mut old.graph.private.sources).push(SourceSnapshot {
            id: "unused".into(),
            language: SourceLanguage::Lua,
            content: archived.content,
            origin: archived.identity.clone(),
        });
        old.graph.private.views.push(View {
            document: "unused".into(),
            load_name: archived.identity.selected_path,
        });
        old.graph.private.archived.insert("unused".into());
        let review = merge::prepare(&old, &incoming, version).unwrap();
        let selection = merge::Selection {
            pairs: [
                ("root".into(), Some("root".into())),
                ("part".into(), Some("part".into())),
                ("private-new".into(), None),
                ("unused".into(), None),
            ]
            .into(),
            archive_private: ["private-new".into(), "unused".into()].into(),
            discard_private: Default::default(),
            accept_context_change: true,
        };
        let decisions = ["root", "part"]
            .into_iter()
            .map(|id| {
                let input = merge::file_inputs(&old, &incoming, &review, &selection, id).unwrap();
                let review = input.text_review().unwrap();
                let choices = review
                    .regions
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.conflict.is_some())
                    .map(|(i, _)| (i, hyprland_editor_core::source_merge::Choice::Incoming))
                    .collect();
                (id.into(), merge::Decision { review, choices })
            })
            .collect();
        let composed = merge::compose(&old, &incoming, &review, &selection, &decisions).unwrap();
        assert!(composed.dependency_renewal.is_none());
        assert!(composed.bundle.graph.private.bindings.is_empty());
        assert_eq!(
            composed
                .bundle
                .graph
                .private
                .sources
                .iter()
                .find(|s| s.id == "private-new")
                .unwrap()
                .content,
            "return 7"
        );
        let kept = composed
            .bundle
            .graph
            .revisions
            .iter()
            .find(|r| r.document == "private-new")
            .unwrap();
        assert_eq!(kept.sha256, lua::source_hash("return 9"));
        let snapshot = Workspace::new(Draft::new(version)).with_graph(&composed.bundle);
        let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
        let bytes = fs::read(&path).unwrap();
        let (ok, output) = check(&path);
        assert!(ok, "{output}");
        let checked: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(checked["source_graph_private_documents"], 2);
        assert_eq!(checked["source_graph_archived_private_documents"], 2);
        assert_eq!(checked["source_graph_private_bindings"], 0);
        assert_eq!(checked["original_paths_opened"], false);
        assert_eq!(checked["folder_access_restored"], false);
        let staging = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
            .arg("--check-preview-layers")
            .arg(&path)
            .env_clear()
            .output()
            .unwrap();
        assert!(
            staging.status.success(),
            "{}",
            String::from_utf8_lossy(&staging.stderr)
        );
        let staged: serde_json::Value = serde_json::from_slice(&staging.stdout).unwrap();
        let row = staged["review"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["setting_id"] == "general:border_size")
            .unwrap();
        assert_eq!(
            row["captured"]["value"],
            serde_json::to_value(Value::Int(3)).unwrap()
        );
        let reopened = workspace::read(&path, &catalogue()).unwrap();
        assert_eq!(
            serde_json::to_vec(&reopened).unwrap(),
            serde_json::to_vec(&snapshot).unwrap()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        // A subsequent external review still sees both archives and cannot
        // forget them merely because their bytes are unchanged.
        let retained = Bundle {
            graph: reopened.source_graph.unwrap(),
            sources: std::sync::Arc::new(reopened.sources),
        };
        let next = merge::prepare(&retained, &incoming, version).unwrap();
        assert!(next.old_views.iter().filter(|v| v.archived).count() == 2);
        let mut missing = selection.clone();
        missing.archive_private.clear();
        assert!(merge::validate_selection(&next, &missing).is_err());
        assert!(!old_folder.join("originals").exists());
        assert!(!new_folder.join("originals").exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn private_input_model_migration_preserves_active_and_archived_data_with_fresh_reanalysis() {
    const PREVIOUS: &str = "9f77616b432badc333c5a3b70cfbca4d7ed9fe4557ba87b97b7da5348940044f";
    for previous in [
        PREVIOUS,
        "e25d982f292e56135f0810194ae06ae6a93a45d4b6f6afd75bb0ada5ecf7b0fc",
        "aaa60b286d173a7d4b8818bea1461ce7e87c7a054788fcf90bbd58f8b9925d75",
        "a556ebf4459ee6ee9196ca2593faaa48c2d8c4d8b383a7b030c54e1cf951778c",
        "88ba4156fabecf287c34d467b80ba9db8e13a8682afca20af55c106cfc850922",
        "2f9e39f0f1efb1e7be298cfbc50c43ed754f7e3feb68d64d7cb7ed7fbca76128",
        "09e8ff83ee94a51dcd54a7c7cbabc543d956b4e1c50eb5369b3b56cf34b3b0b9",
        "4ec820faed65d07698417a29f7738f56f8470e0fc7f9898643c646c291aae5af",
        "25888d9809cc642d8cc9d6980c5cd1f321576863d2f9660e36736b810a402814",
        "d569b9b70d887873926112bf736e8b481d21e93ff0fe9df8d390865b7ec4ff3f",
        "92435dde9f49b35a6a607abd548593e2ac944786411da37754ad8f1467568da2",
        "a89500f370d5687e920ba3140ab3307352764036084dbc0646a49450d32a4bf7",
        "155fa2b02986568d01d1caa025c6ca9920f241aed2c8c789b0d94b50a5ed2d1c",
    ] {
        for version in [BASELINE, AUDITED_MAIN] {
            for archive in [false, true] {
                let root = root();
                let mut snapshot = private_fixture(&root, version);
                if archive {
                    let graph = snapshot.source_graph.as_mut().unwrap();
                    let base = graph
                        .program(&snapshot.sources, Some("root"), version)
                        .unwrap();
                    let p = graph.private.program(&base, &graph.revisions).unwrap();
                    let request = &graph.private.bindings[0].request;
                    let d = p
                        .documents
                        .iter()
                        .find(|d| d.id == request.document)
                        .unwrap();
                    let plan = edits::prepare(
                        &p,
                        vec![edits::SourceEdit {
                            document: d.id.clone(),
                            base_sha256: d.sha256.clone(),
                            span: request.span.clone(),
                            old_bytes_sha256: lua::source_hash(&d.source[request.span.clone()]),
                            replacement: "7".into(),
                            semantic_targets: vec!["general:border_size".into()],
                        }],
                        vec![],
                    )
                    .unwrap();
                    (graph.revisions, graph.private) =
                        graph.private.apply(&base, &graph.revisions, &plan).unwrap();
                }
                snapshot.source_graph.as_mut().unwrap().schema_sha256 = previous.into();
                if !matches!(
                    previous,
                    "2f9e39f0f1efb1e7be298cfbc50c43ed754f7e3feb68d64d7cb7ed7fbca76128"
                        | "09e8ff83ee94a51dcd54a7c7cbabc543d956b4e1c50eb5369b3b56cf34b3b0b9"
                        | "4ec820faed65d07698417a29f7738f56f8470e0fc7f9898643c646c291aae5af"
                        | "25888d9809cc642d8cc9d6980c5cd1f321576863d2f9660e36736b810a402814"
                        | "d569b9b70d887873926112bf736e8b481d21e93ff0fe9df8d390865b7ec4ff3f"
                        | "92435dde9f49b35a6a607abd548593e2ac944786411da37754ad8f1467568da2"
                        | "a89500f370d5687e920ba3140ab3307352764036084dbc0646a49450d32a4bf7"
                        | "155fa2b02986568d01d1caa025c6ca9920f241aed2c8c789b0d94b50a5ed2d1c"
                ) {
                    snapshot
                        .source_graph
                        .as_mut()
                        .unwrap()
                        .compiler
                        .numeric_codegen = None;
                }
                let bytes = serde_json::to_vec_pretty(&snapshot).unwrap();
                let path = root.join("previous-private.hye");
                fs::write(&path, &bytes).unwrap();
                assert!(!check(&path).0);
                let (ok, output) = check_migration(&path);
                assert!(ok, "{output}");
                let checked: serde_json::Value = serde_json::from_str(&output).unwrap();
                assert_eq!(checked["migration"]["previous_model"], previous);
                assert_eq!(checked["migration"]["private_documents"], 1);
                assert_eq!(
                    checked["migration"]["private_resolution_inputs"],
                    usize::from(!archive)
                );
                assert_eq!(
                    checked["migration"]["archived_documents"],
                    usize::from(archive)
                );
                assert_eq!(checked["migration"]["reanalysis_findings"], 0);
                assert_eq!(checked["saved_file_written"], false);
                assert_eq!(checked["original_paths_opened"], false);
                assert_eq!(fs::read(&path).unwrap(), bytes);
                assert!(!root.join("originals").exists());
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
}
#[test]
fn private_piece_revisions_reopen_without_originals_and_preserve_captured_identities() {
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let mut snapshot = fixture(&root, version);
        let graph = snapshot.source_graph.as_ref().unwrap();
        let base = graph
            .program(&snapshot.sources, snapshot.root_source.as_deref(), version)
            .unwrap();
        let part = &base.documents[1];
        let start = part.source.find('3').unwrap();
        let plan = replay::edits::prepare(
            &base,
            vec![replay::edits::SourceEdit {
                document: part.id.clone(),
                base_sha256: part.sha256.clone(),
                span: start..start + 1,
                old_bytes_sha256: lua::source_hash("3"),
                replacement: "7".into(),
                semantic_targets: vec!["general:border_size".into()],
            }],
            vec![],
        )
        .unwrap();
        let revisions =
            hyprland_editor_core::workspace_graph::revisions::apply(&base, &[], &plan).unwrap();
        let expected =
            hyprland_editor_core::workspace_graph::revisions::program(base, &revisions).unwrap();
        assert_eq!(
            replay::replay(&expected).unwrap().events[0].arguments[0],
            Observed::Table(vec![(
                Observed::Bytes(b"general".to_vec()),
                Observed::Table(vec![(
                    Observed::Bytes(b"border_size".to_vec()),
                    Observed::Int(7)
                )])
            )])
        );
        snapshot.source_graph.as_mut().unwrap().revisions = revisions;
        let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
        let (ok, output) = check(&path);
        assert!(ok, "{output}");
        let result: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(result["source_graph_revisions"], 1);
        assert_eq!(result["original_paths_opened"], false);
        assert_eq!(result["folder_access_restored"], false);
        let reopened = workspace::read(&path, &catalogue()).unwrap();
        assert_eq!(reopened.sources[1].content, "return 3");
        assert_eq!(reopened.sources[1].origin, snapshot.sources[1].origin);
        assert_eq!(reopened.draft.values, snapshot.draft.values);
        let mut forged: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        forged["source_graph"]["revisions"][0]["buffers"][0] = "9".into();
        let bad = root.join("forged.hye");
        fs::write(&bad, serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(!check(&bad).0);
        assert!(!root.join("originals").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn application_replays_all_saved_graph_inputs_without_original_files_or_folder_access() {
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let snapshot = fixture(&root, version);
        let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
        let (ok, output) = check(&path);
        assert!(ok, "{output}");
        let result: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(result["source_graph_replayed"], true);
        assert_eq!(result["source_graph_documents"], 2);
        assert_eq!(result["source_graph_bindings"], 1);
        assert_eq!(result["original_paths_opened"], false);
        assert_eq!(result["folder_access_restored"], false);
        assert_eq!(result["runtime_verified"], false);
        let saved = workspace::read(&path, &catalogue()).unwrap();
        assert_eq!(saved.draft.values, snapshot.draft.values);
        assert_eq!(saved.sources[1].content, "return 3");
        assert!(!saved.sources[0].origin.selected_path.is_empty());
        assert!(!root.join("originals").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn graph_reopen_refuses_inconsistent_profiles_views_hashes_and_unreached_bindings() {
    let root = root();
    let snapshot = fixture(&root, BASELINE);
    let path = root.join("forged.hye");
    for change in 0..11 {
        let mut value = serde_json::to_value(&snapshot).unwrap();
        match change {
            0 => value["source_graph"]["compiler"]["release"] = "foreign".into(),
            1 => value["source_graph"]["views"][1]["document"] = "unknown".into(),
            2 => value["sources"][1]["content"] = "return 99".into(),
            3 => {
                value["source_graph"]["bindings"][0]["request"]["module"] =
                    serde_json::json!([b'x'])
            }
            4 => value["source_graph"]["schema_sha256"] = "0".repeat(64).into(),
            5 => {
                value["source_graph"]["context"]["main_file"] =
                    serde_json::json!(b"/different/main.lua".to_vec())
            }
            7 => {
                value["source_graph"]["compiler"]["numeric_codegen"]["probe_sha256"] =
                    "0".repeat(64).into()
            }
            8 => {
                value["source_graph"]["compiler"]["numeric_codegen"]["zero_subtraction"] =
                    serde_json::Value::Null
            }
            9 => value["source_graph"]["compiler"]["numeric_codegen"] = serde_json::Value::Null,
            10 => {
                value["source_graph"]["compiler"]
                    .as_object_mut()
                    .unwrap()
                    .remove("numeric_codegen");
            }
            _ => value["source_graph"]["bindings"] = serde_json::json!([]),
        }
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(!check(&path).0, "Accepted changed graph input {change}");
    }
    // Existing multi-file artifacts without explicit resolution inputs remain refused.
    let mut missing = snapshot;
    missing.source_graph = None;
    fs::write(&path, missing.to_json(&catalogue()).unwrap()).unwrap();
    assert!(!check(&path).0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn numeric_private_revisions_reopen_with_observed_compiler_and_original_zero_sign() {
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let mut snapshot = fixture_source(&root, version, "local nz=-0.0;local z=0;return nz-0");
        let graph = snapshot.source_graph.as_ref().unwrap();
        let base = graph
            .program(&snapshot.sources, snapshot.root_source.as_deref(), version)
            .unwrap();
        let doc = &base.documents[0];
        let end = doc.source.len();
        let plan = edits::prepare(
            &base,
            vec![edits::SourceEdit {
                document: doc.id.clone(),
                base_sha256: doc.sha256.clone(),
                span: end - 1..end,
                old_bytes_sha256: lua::source_hash("0"),
                replacement: "z".into(),
                semantic_targets: vec!["numeric-return".into()],
            }],
            vec![],
        )
        .unwrap();
        let original = replay(&base).unwrap();
        snapshot.source_graph.as_mut().unwrap().revisions =
            hyprland_editor_core::workspace_graph::revisions::apply(&base, &[], &plan).unwrap();
        let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
        let (ok, output) = check(&path);
        assert!(ok, "{output}");
        let saved = workspace::read(&path, &catalogue()).unwrap();
        let graph = saved.source_graph.as_ref().unwrap();
        assert_eq!(graph.compiler, compiler_profile());
        let reopened_base = graph
            .program(&saved.sources, saved.root_source.as_deref(), version)
            .unwrap();
        let revised = hyprland_editor_core::workspace_graph::revisions::program(
            reopened_base.clone(),
            &graph.revisions,
        )
        .unwrap();
        let report = replay(&revised).unwrap();
        let [Observed::Float(v)] = report.returns.as_slice() else {
            panic!("revised numeric return")
        };
        assert_eq!(v.to_bits(), (-0.0f64).to_bits());
        assert_eq!(saved.sources[0].content, base.documents[0].source);
        assert_eq!(saved.sources[0].origin, snapshot.sources[0].origin);
        let restored = replay(&reopened_base).unwrap();
        let [Observed::Float(before)] = original.returns.as_slice() else {
            panic!("original numeric return")
        };
        let [Observed::Float(after)] = restored.returns.as_slice() else {
            panic!("restored numeric return")
        };
        assert_eq!(before.to_bits(), after.to_bits());
        assert!(!report.runtime_verified && !root.join("originals").exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn saved_load_aliases_remain_distinct_and_a_stopped_model_stays_partial() {
    let root = root();
    let snapshot=fixture_source(&root,BASELINE,"local a=dofile('part.lua');local b=dofile('./part.lua');hl.config{general={border_size=a+b}};unknown_effect()");
    let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
    let (ok, output) = check(&path);
    assert!(ok, "{output}");
    let saved = workspace::read(&path, &catalogue()).unwrap();
    let graph = saved.source_graph.as_ref().unwrap();
    assert_eq!(graph.views.len(), 3);
    assert_ne!(graph.views[1].load_name, graph.views[2].load_name);
    assert_eq!(saved.sources[1].origin, saved.sources[2].origin);
    let report = replay::replay(
        &graph
            .program(&saved.sources, saved.root_source.as_deref(), BASELINE)
            .unwrap(),
    )
    .unwrap();
    assert!(matches!(report.halt, Some(Halt::Unresolved { .. })));
    assert!(!report.runtime_verified);
    assert_eq!(report.loads.len(), 2);
    assert_eq!(report.events.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

fn check_migration(path: &Path) -> (bool, String) {
    let result = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
        .arg("--check-workspace-migration")
        .arg(path)
        .env_clear()
        .output()
        .unwrap();
    (
        result.status.success(),
        String::from_utf8(if result.status.success() {
            result.stdout
        } else {
            result.stderr
        })
        .unwrap(),
    )
}
#[test]
fn preview_layer_worker_rebuilds_private_module_pieces_and_preserves_deleted_originals() {
    use hyprland_editor_projection::{
        layers::{Choice, Selection},
        Scenario,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let mut snapshot = fixture(&root, version);
        let graph = snapshot.source_graph.as_ref().unwrap();
        let base = graph
            .program(&snapshot.sources, snapshot.root_source.as_deref(), version)
            .unwrap();
        let part = &base.documents[1];
        let at = part.source.find('3').unwrap();
        let plan = replay::edits::prepare(
            &base,
            vec![replay::edits::SourceEdit {
                document: part.id.clone(),
                base_sha256: part.sha256.clone(),
                span: at..at + 1,
                old_bytes_sha256: lua::source_hash("3"),
                replacement: "7".into(),
                semantic_targets: vec!["general:border_size".into()],
            }],
            vec![],
        )
        .unwrap();
        snapshot.source_graph.as_mut().unwrap().revisions =
            hyprland_editor_core::workspace_graph::revisions::apply(&base, &[], &plan).unwrap();
        let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
        let bytes = fs::read(&path).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
            .arg("--check-preview-layers")
            .arg(&path)
            .env_clear()
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let row = value["review"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["setting_id"] == "general:border_size")
            .unwrap();
        assert_eq!(
            row["captured"]["value"],
            serde_json::to_value(Value::Int(7)).unwrap()
        );
        assert_eq!(row["pending"], serde_json::to_value(Value::Int(5)).unwrap());
        assert_eq!(value["review"]["model_complete"], true);
        let selection = Selection {
            review_sha256: value["review_sha256"].as_str().unwrap().into(),
            acknowledge_partial: false,
            choices: std::collections::BTreeMap::from([(
                "general:border_size".into(),
                Choice::Captured,
            )]),
        };
        let selected = root.join("selection.json");
        let scenario = root.join("scenario.json");
        fs::write(&selected, serde_json::to_vec(&selection).unwrap()).unwrap();
        fs::write(&scenario, serde_json::to_vec(&Scenario::default()).unwrap()).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
            .arg("--check-preview-projection-selected")
            .arg(&path)
            .arg(&scenario)
            .arg(&selected)
            .env_clear()
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let projected: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            projected["artifact"]["report"]["layers"]["source_value_roundtrips"],
            1
        );
        assert_eq!(projected["runtime_verified"], false);
        let parsed = lua::parse(
            projected["artifact"]["lua"].as_str().unwrap(),
            version,
            &catalogue(),
        )
        .unwrap();
        assert_eq!(parsed.draft.values["general:border_size"], Value::Int(7));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(!root.join("originals").exists());
        assert_eq!(snapshot.sources[1].content, "return 3");
        let mut corrupt: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        corrupt["source_graph"]["compiler"]["release"] = "foreign".into();
        let bad = root.join("foreign.hye");
        fs::write(&bad, serde_json::to_vec(&corrupt).unwrap()).unwrap();
        assert!(!Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
            .arg("--check-preview-layers")
            .arg(&bad)
            .env_clear()
            .output()
            .unwrap()
            .status
            .success());
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn monitor_model_migration_preserves_an_old_stopped_graph_without_guessing_new_dependencies() {
    for version in [BASELINE, AUDITED_MAIN] {
        let root = root();
        let mut snapshot = fixture_source(&root, version, "hl.monitor{output='DP-1',transform=2};local part=require('part');hl.config{general={border_size=part}}");
        let graph = snapshot.source_graph.as_mut().unwrap();
        // The preceding model stopped at hl.monitor, so a valid saved capture
        // from that model contains only its root, without a reached part loader.
        graph.schema_sha256 =
            "c1abcce3f3c001585eb7b39e13b7ab888791a3480877f7817bde26d66bba1427".into();
        graph.compiler.numeric_codegen = None;
        graph.bindings.clear();
        graph.views.retain(|v| v.document == graph.root);
        snapshot.sources.retain(|s| s.id == graph.root);
        let bytes = serde_json::to_vec_pretty(&snapshot).unwrap();
        let path = root.join("old-stopped.hye");
        fs::write(&path, &bytes).unwrap();
        let (ok, output) = check_migration(&path);
        assert!(ok, "{output}");
        let report: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(report["migration"]["captured_documents"], 1);
        assert_eq!(report["migration"]["resolution_inputs"], 0);
        assert!(report["migration"]["reanalysis_findings"].as_u64().unwrap() > 0);
        assert!(report["migration"]["reanalysis_note"].is_string());
        assert_eq!(report["original_paths_opened"], false);
        assert_eq!(report["folder_access_restored"], false);
        assert_eq!(report["runtime_verified"], false);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(!root.join("originals").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn reviewed_graph_migrations_rebuild_deleted_sources_and_private_pieces_without_writing() {
    for version in [BASELINE, AUDITED_MAIN] {
        for old in [
            "f7017978da7127ee461497d1021e418bb2c2e171b4d28a43b6671f8abb2531dd",
            "046fb098cb5c1f38d6922f37274cfcba56818ec35c68dbcd5d78fc1e5e4d7371",
            "7273a441eca7d6ae6424b92d1188be64e642d007a77ae9e6a909239284f23ffc",
            "c1abcce3f3c001585eb7b39e13b7ab888791a3480877f7817bde26d66bba1427",
            "d7dd575308699007b7083b5776cfdfb6268b84b78fd4bd7f7f29c023c24f70cd",
            "05bb2e4e86ef118289b70942ff23ae80ef07ad6ea16c3f44dd676e542c0ece37",
            "cfa1292e40e8e8111d2f279f98cfeed192f35b05b7a6af49c024c23c47085783",
            "1dacfb662caa635f63047517d223d73ec0c1922b2d9f93b3a7fa35cecb91909e",
            "be2a3bceb486422b079705792d7dae67f4922f96fcfc304004df20fbc22c92f1",
            "dbb7803fa4ea318a5b4511fd1f44eb422be73feebd8342ab449a8a4ffa27351d",
            "071946f5f808e82ef8f38d67fa919d7a427299f2c41c0f5e73368d7c3ee0aae7",
            "71556836e091dccaaf231bc4510e448e545e588c175a56e59ad8c7d9cd2a3972",
            "9f77616b432badc333c5a3b70cfbca4d7ed9fe4557ba87b97b7da5348940044f",
            "e25d982f292e56135f0810194ae06ae6a93a45d4b6f6afd75bb0ada5ecf7b0fc",
            "aaa60b286d173a7d4b8818bea1461ce7e87c7a054788fcf90bbd58f8b9925d75",
            "a556ebf4459ee6ee9196ca2593faaa48c2d8c4d8b383a7b030c54e1cf951778c",
            "88ba4156fabecf287c34d467b80ba9db8e13a8682afca20af55c106cfc850922",
            "2f9e39f0f1efb1e7be298cfbc50c43ed754f7e3feb68d64d7cb7ed7fbca76128",
            "09e8ff83ee94a51dcd54a7c7cbabc543d956b4e1c50eb5369b3b56cf34b3b0b9",
            "4ec820faed65d07698417a29f7738f56f8470e0fc7f9898643c646c291aae5af",
            "25888d9809cc642d8cc9d6980c5cd1f321576863d2f9660e36736b810a402814",
            "d569b9b70d887873926112bf736e8b481d21e93ff0fe9df8d390865b7ec4ff3f",
            "92435dde9f49b35a6a607abd548593e2ac944786411da37754ad8f1467568da2",
            "a89500f370d5687e920ba3140ab3307352764036084dbc0646a49450d32a4bf7",
            "155fa2b02986568d01d1caa025c6ca9920f241aed2c8c789b0d94b50a5ed2d1c",
        ] {
            let root = root();
            let mut snapshot = fixture(&root, version);
            let graph = snapshot.source_graph.as_ref().unwrap();
            let base = graph
                .program(&snapshot.sources, snapshot.root_source.as_deref(), version)
                .unwrap();
            let part = &base.documents[1];
            let at = part.source.find('3').unwrap();
            let plan = replay::edits::prepare(
                &base,
                vec![replay::edits::SourceEdit {
                    document: part.id.clone(),
                    base_sha256: part.sha256.clone(),
                    span: at..at + 1,
                    old_bytes_sha256: lua::source_hash("3"),
                    replacement: "7".into(),
                    semantic_targets: vec!["general:border_size".into()],
                }],
                vec![],
            )
            .unwrap();
            snapshot.source_graph.as_mut().unwrap().revisions =
                hyprland_editor_core::workspace_graph::revisions::apply(&base, &[], &plan).unwrap();
            let mut value = serde_json::to_value(&snapshot).unwrap();
            value["source_graph"]["schema_sha256"] = old.into();
            if !matches!(
                old,
                "2f9e39f0f1efb1e7be298cfbc50c43ed754f7e3feb68d64d7cb7ed7fbca76128"
                    | "09e8ff83ee94a51dcd54a7c7cbabc543d956b4e1c50eb5369b3b56cf34b3b0b9"
                    | "4ec820faed65d07698417a29f7738f56f8470e0fc7f9898643c646c291aae5af"
                    | "25888d9809cc642d8cc9d6980c5cd1f321576863d2f9660e36736b810a402814"
                    | "d569b9b70d887873926112bf736e8b481d21e93ff0fe9df8d390865b7ec4ff3f"
                    | "92435dde9f49b35a6a607abd548593e2ac944786411da37754ad8f1467568da2"
                    | "a89500f370d5687e920ba3140ab3307352764036084dbc0646a49450d32a4bf7"
                    | "155fa2b02986568d01d1caa025c6ca9920f241aed2c8c789b0d94b50a5ed2d1c"
            ) {
                value["source_graph"]["compiler"]
                    .as_object_mut()
                    .unwrap()
                    .remove("numeric_codegen");
            }
            let bytes = serde_json::to_vec_pretty(&value).unwrap();
            let path = root.join("old.hye");
            fs::write(&path, &bytes).unwrap();
            assert!(!check(&path).0, "Normal opening must not silently migrate");
            let (ok, output) = check_migration(&path);
            assert!(ok, "{output}");
            let report: serde_json::Value = serde_json::from_str(&output).unwrap();
            assert_eq!(report["migration"]["previous_model"], old);
            assert_eq!(report["migration"]["captured_documents"], 2);
            assert_eq!(report["migration"]["private_revisions"], 1);
            assert_eq!(report["migration"]["resolution_inputs"], 1);
            assert_eq!(
                report["migration"]["input_sha256"],
                lua::source_hash(std::str::from_utf8(&bytes).unwrap())
            );
            for key in [
                "saved_file_written",
                "original_paths_opened",
                "folder_access_restored",
                "runtime_verified",
            ] {
                assert_eq!(report[key], false, "{key}");
            }
            assert_eq!(report["needs_review_and_new_private_save"], true);
            assert_eq!(
                report["migration"]["compiler_observation_renewed"],
                !matches!(
                    old,
                    "2f9e39f0f1efb1e7be298cfbc50c43ed754f7e3feb68d64d7cb7ed7fbca76128"
                        | "09e8ff83ee94a51dcd54a7c7cbabc543d956b4e1c50eb5369b3b56cf34b3b0b9"
                        | "4ec820faed65d07698417a29f7738f56f8470e0fc7f9898643c646c291aae5af"
                        | "25888d9809cc642d8cc9d6980c5cd1f321576863d2f9660e36736b810a402814"
                        | "d569b9b70d887873926112bf736e8b481d21e93ff0fe9df8d390865b7ec4ff3f"
                        | "92435dde9f49b35a6a607abd548593e2ac944786411da37754ad8f1467568da2"
                        | "a89500f370d5687e920ba3140ab3307352764036084dbc0646a49450d32a4bf7"
                        | "155fa2b02986568d01d1caa025c6ca9920f241aed2c8c789b0d94b50a5ed2d1c"
                )
            );
            assert_eq!(report["migration"]["compiler_numeric_mode_known"], true);
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert!(!root.join("originals").exists());
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
            fs::remove_dir_all(root).unwrap();
        }
    }
}
#[test]
fn migration_refuses_unknown_models_profiles_corruption_unreached_inputs_and_hidden_payloads() {
    let root = root();
    let snapshot = fixture(&root, BASELINE);
    let path = root.join("old.hye");
    let mut original = serde_json::to_value(snapshot).unwrap();
    original["source_graph"]["schema_sha256"] =
        "f7017978da7127ee461497d1021e418bb2c2e171b4d28a43b6671f8abb2531dd".into();
    let numeric = original["source_graph"]["compiler"]
        .as_object_mut()
        .unwrap()
        .remove("numeric_codegen")
        .unwrap();
    // A valid legacy profile is rebuilt before adding each independent corruption.
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    assert!(check_migration(&path).0);
    for change in 0..9 {
        let mut value = original.clone();
        match change {
            0 => value["source_graph"]["schema_sha256"] = "0".repeat(64).into(),
            1 => value["source_graph"]["compiler"]["release"] = "foreign".into(),
            2 => value["sources"][1]["content"] = "return 999".into(),
            3 => {
                value["source_graph"]["bindings"][0]["request"]["module"] =
                    serde_json::json!([b'x'])
            }
            4 => {
                value["draft"]["values"]["general:border_size"]["hidden"] = "discarded field".into()
            }
            5 => value["source_graph"]["views"][1]["document"] = "unknown".into(),
            7 => value["source_graph"]["compiler"]["numeric_codegen"] = numeric.clone(),
            8 => value["source_graph"]["compiler"]["numeric_codegen"] = serde_json::Value::Null,
            _ => {
                value["source_graph"]["private"] =
                    serde_json::json!({"sources":[],"views":[],"bindings":[],"archived":[]})
            }
        }
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(
            !check_migration(&path).0,
            "Accepted invalid migration {change}"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn numeric_writer_migration_preserves_existing_observation_and_refuses_observation_renewal() {
    for previous in [
        "2f9e39f0f1efb1e7be298cfbc50c43ed754f7e3feb68d64d7cb7ed7fbca76128",
        "09e8ff83ee94a51dcd54a7c7cbabc543d956b4e1c50eb5369b3b56cf34b3b0b9",
        "4ec820faed65d07698417a29f7738f56f8470e0fc7f9898643c646c291aae5af",
        "25888d9809cc642d8cc9d6980c5cd1f321576863d2f9660e36736b810a402814",
        "d569b9b70d887873926112bf736e8b481d21e93ff0fe9df8d390865b7ec4ff3f",
        "92435dde9f49b35a6a607abd548593e2ac944786411da37754ad8f1467568da2",
        "a89500f370d5687e920ba3140ab3307352764036084dbc0646a49450d32a4bf7",
        "155fa2b02986568d01d1caa025c6ca9920f241aed2c8c789b0d94b50a5ed2d1c",
    ] {
        for version in [BASELINE, AUDITED_MAIN] {
            let root = root();
            let snapshot = fixture(&root, version);
            let mut original = serde_json::to_value(&snapshot).unwrap();
            original["source_graph"]["schema_sha256"] = previous.into();
            let path = root.join("numeric-writer.hye");
            let bytes = serde_json::to_vec(&original).unwrap();
            fs::write(&path, &bytes).unwrap();
            let (ok, output) = check_migration(&path);
            assert!(ok, "{output}");
            let reply: serde_json::Value = serde_json::from_str(&output).unwrap();
            assert_eq!(reply["migration"]["compiler_observation_renewed"], false);
            assert!(reply["migration"]["reanalysis_note"]
                .as_str()
                .unwrap()
                .contains("immediate submap registration callbacks"));
            assert_eq!(fs::read(&path).unwrap(), bytes);
            for change in 0..5 {
                let mut forged = original.clone();
                match change {
                    0 => {
                        forged["source_graph"]["compiler"]
                            .as_object_mut()
                            .unwrap()
                            .remove("numeric_codegen");
                    }
                    1 => {
                        forged["source_graph"]["compiler"]["numeric_codegen"] =
                            serde_json::Value::Null
                    }
                    2 => {
                        forged["source_graph"]["compiler"]["numeric_codegen"]["probe_sha256"] =
                            "0".repeat(64).into()
                    }
                    3 => {
                        forged["source_graph"]["compiler"]["numeric_codegen"]["zero_subtraction"] =
                            serde_json::Value::Null
                    }
                    _ => forged["source_graph"]["compiler"]["release"] = "5.5.1".into(),
                }
                let bytes = serde_json::to_vec(&forged).unwrap();
                fs::write(&path, &bytes).unwrap();
                assert!(
                    !check_migration(&path).0,
                    "Accepted numeric writer corruption {change}"
                );
                assert_eq!(fs::read(&path).unwrap(), bytes);
            }
            fs::remove_dir_all(root).unwrap();
        }
    }
}
