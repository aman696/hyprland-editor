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
    let root = std::env::temp_dir().join(format!(
        "he-graph-reopen-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    root
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
    for change in 0..7 {
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
fn reviewed_graph_migrations_rebuild_deleted_sources_and_private_pieces_without_writing() {
    for version in [BASELINE, AUDITED_MAIN] {
        for old in [
            "f7017978da7127ee461497d1021e418bb2c2e171b4d28a43b6671f8abb2531dd",
            "046fb098cb5c1f38d6922f37274cfcba56818ec35c68dbcd5d78fc1e5e4d7371",
            "7273a441eca7d6ae6424b92d1188be64e642d007a77ae9e6a909239284f23ffc",
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
    for change in 0..6 {
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
            _ => value["source_graph"]["views"][1]["document"] = "unknown".into(),
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
