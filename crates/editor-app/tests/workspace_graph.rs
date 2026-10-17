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
        sources,
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
