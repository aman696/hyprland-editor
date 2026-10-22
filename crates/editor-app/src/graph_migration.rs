//! Explicit, reviewed dependency/storage/monitor model transitions. No writer or saved
//! analysis is trusted; current restricted workers rebuild the captured graph.
use hyprland_editor_core::{
    lua, storage,
    workspace::{Workspace, MAX_WORKSPACE_BYTES},
    workspace_graph::{self, Bundle},
    Setting,
};
use serde::Serialize;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
const FROM: &str = "f7017978da7127ee461497d1021e418bb2c2e171b4d28a43b6671f8abb2531dd";
const FROM_MIGRATION: &str = "7273a441eca7d6ae6424b92d1188be64e642d007a77ae9e6a909239284f23ffc";
const MIGRATION: &str = include_str!("../../../data/migrations/graph-model-migration.json");
const BEFORE_MONITORS: &str = "c1abcce3f3c001585eb7b39e13b7ab888791a3480877f7817bde26d66bba1427";
const PRE_MONITOR: &str = include_str!("../../../data/migrations/graph-model-before-monitors.json");
const TO: &str = "d7dd575308699007b7083b5776cfdfb6268b84b78fd4bd7f7f29c023c24f70cd";
const OLD: &str = include_str!("../../../data/migrations/graph-model-before-projection.json");
const FROM_PROJECTION: &str = "046fb098cb5c1f38d6922f37274cfcba56818ec35c68dbcd5d78fc1e5e4d7371";
const PROJECTION: &str = include_str!("../../../data/migrations/graph-model-projection.json");
const CURRENT: &str = include_str!("../../../data/graph-model.json");

#[derive(Clone, Debug, Serialize)]
pub(super) struct Summary {
    pub previous_model: String,
    pub current_model: String,
    pub input_sha256: String,
    pub rebuilt_workspace_sha256: String,
    pub captured_documents: usize,
    pub private_revisions: usize,
    pub resolution_inputs: usize,
    pub reanalysis_findings: usize,
    pub reanalysis_note: Option<String>,
}
pub(super) struct Prepared {
    pub workspace: Workspace,
    pub graph: Arc<super::source_graph::Saved>,
    pub summary: Summary,
}
fn transition(from: &str) -> Result<(), String> {
    let (previous, allowed): (&str, &[&str]) = match from {
        FROM => (
            OLD,
            &[
                "Cargo.lock",
                "Cargo.toml",
                "crates/editor-app/Cargo.toml",
                "crates/editor-core/src/storage.rs",
            ],
        ),
        FROM_PROJECTION => (
            PROJECTION,
            &[
                "Cargo.lock",
                "crates/editor-app/Cargo.toml",
                "crates/editor-core/src/storage.rs",
            ],
        ),
        FROM_MIGRATION => (MIGRATION, &["Cargo.lock", "crates/editor-app/Cargo.toml"]),
        BEFORE_MONITORS => (PRE_MONITOR, &[]),
        _ => return Err("This saved graph model has no reviewed migration".into()),
    };
    if lua::source_hash(previous) != from
        || lua::source_hash(PRE_MONITOR) != BEFORE_MONITORS
        || workspace_graph::schema_sha256() != TO
    {
        return Err("No reviewed graph migration to this model is available".into());
    }
    let old: serde_json::Value = serde_json::from_str(previous).map_err(|e| e.to_string())?;
    let intermediate: serde_json::Value =
        serde_json::from_str(PRE_MONITOR).map_err(|e| e.to_string())?;
    let old = old["inputs"]
        .as_object()
        .ok_or("Invalid previous model inventory")?;
    let current = intermediate["inputs"]
        .as_object()
        .ok_or("Invalid current model inventory")?;
    let keys: BTreeSet<_> = old.keys().collect();
    if keys != current.keys().collect() || keys.len() != 58 {
        return Err("Reviewed migration input set changed".into());
    }
    let changes: Vec<_> = keys
        .into_iter()
        .filter(|k| old[*k] != current[*k])
        .map(String::as_str)
        .collect();
    if changes != allowed {
        return Err("Graph migration no longer has only the reviewed dependency and storage-helper visibility differences".into());
    }
    // This transition changes the argument observer, not the saved byte/piece
    // format. It adds monitor calls with fixed zero results and primary-value
    // inspection; every reached loader and private revision is freshly rebuilt.
    // The new model can reach code that was previously unresolved. Such inputs
    // must pass restore's full saved-graph checks; no old analysis is reused.
    let next: serde_json::Value = serde_json::from_str(CURRENT).map_err(|e| e.to_string())?;
    let next = next["inputs"]
        .as_object()
        .ok_or("Invalid current model inventory")?;
    let added = next
        .keys()
        .filter(|k| !current.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if next.len() != 62
        || added
            != [
                "crates/editor-core/src/lua/monitor_inspection.rs",
                "crates/editor-core/src/lua/replay/monitor_edit.rs",
                "data/monitor-source-semantics.json",
                "data/monitors.json",
            ]
    {
        return Err("Reviewed monitor migration input set changed".into());
    }
    let changed = current
        .keys()
        .filter(|k| current[*k] != next.get(*k).cloned().unwrap_or_default())
        .map(String::as_str)
        .collect::<Vec<_>>();
    if changed
        != [
            "crates/editor-app/src/source_graph.rs",
            "crates/editor-app/src/worker.rs",
            "crates/editor-core/src/lua.rs",
            "crates/editor-core/src/lua/replay.rs",
            "crates/editor-core/src/lua/replay/scalar_edit.rs",
        ]
    {
        return Err(
            "Monitor migration no longer has only the reviewed observer and worker differences"
                .into(),
        );
    }
    Ok(())
}
pub(super) fn prepare(text: &str, catalogue: &[Setting]) -> Result<Prepared, String> {
    if text.len() > MAX_WORKSPACE_BYTES {
        return Err("Saved workspace exceeds 8 MiB".into());
    }
    let mut workspace: Workspace = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let previous = workspace
        .source_graph
        .as_ref()
        .ok_or("Workspace has no saved graph to migrate")?
        .schema_sha256
        .clone();
    transition(&previous)?;
    // Known old writer shape only. Whitespace/key order are immaterial, but
    // unknown/discarded payloads or changed numeric representations are refused.
    let raw: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if serde_json::to_value(&workspace).map_err(|e| e.to_string())? != raw {
        return Err(
            "Saved workspace cannot be rebuilt without discarding an unrecognized representation"
                .into(),
        );
    }
    drop(raw);
    let graph = workspace
        .source_graph
        .as_mut()
        .ok_or("Workspace has no saved graph to migrate")?;
    let mut summary = Summary {
        previous_model: previous.clone(),
        current_model: TO.into(),
        input_sha256: lua::source_hash(text),
        rebuilt_workspace_sha256: String::new(),
        captured_documents: workspace.sources.len(),
        private_revisions: graph.revisions.len(),
        resolution_inputs: graph.bindings.len(),
        reanalysis_findings: 0,
        reanalysis_note: None,
    };
    graph.schema_sha256 = TO.into();
    workspace.validate(catalogue)?;
    let graph = workspace.source_graph.as_ref().unwrap();
    let bundle = Bundle {
        graph: graph.clone(),
        sources: Arc::new(workspace.sources.clone()),
    };
    // Fresh restricted-worker replay verifies compiler identity, every reached
    // loader, captured view and private piece revision. No parser report was saved.
    let analysis = super::source_graph::restore(&bundle, &workspace.draft.version)?;
    summary.reanalysis_findings = analysis.report.logged_module_errors
        + usize::from(analysis.report.halt.is_some())
        + usize::from(analysis.blocked.is_some());
    summary.reanalysis_note = analysis.blocked.clone().or_else(|| analysis.report.halt.as_ref().map(|halt| match halt {
        lua::replay::Halt::NeedsResolution {..} => "Rebuilt analysis needs a dependency absent from the saved capture. Folder access has not been restored.".into(),
        lua::replay::Halt::Unresolved {..} => "Rebuilt analysis stopped at an unmodeled source operation.".into(),
        lua::replay::Halt::LuaError {..} => "Rebuilt analysis encountered a modeled Lua error.".into(),
        lua::replay::Halt::Budget {..} => "Rebuilt analysis reached a resource limit.".into(),
    }));
    let saved = Arc::new(super::source_graph::Saved::new(
        bundle,
        &workspace.draft.version,
    )?);
    let rebuilt = workspace.to_json(catalogue)?;
    // Reversing the metadata change must recover the same decoded workspace.
    // This comparison also covers original bytes, private buffers/pieces, all
    // typed declarations, resolution inputs and source identity evidence.
    let mut reversed: Workspace = serde_json::from_slice(&rebuilt).map_err(|e| e.to_string())?;
    reversed.source_graph.as_mut().unwrap().schema_sha256 = previous;
    let before: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if serde_json::to_value(&reversed).map_err(|e| e.to_string())? != before {
        return Err("Migration changed saved data beyond the reviewed model metadata".into());
    }
    summary.rebuilt_workspace_sha256 =
        lua::source_hash(std::str::from_utf8(&rebuilt).map_err(|e| e.to_string())?);
    Ok(Prepared {
        workspace,
        graph: saved,
        summary,
    })
}
pub(super) fn check(path: PathBuf) -> Result<serde_json::Value, String> {
    if path.extension().is_none_or(|p| p != "hye") {
        return Err("Select one saved .hye workspace".into());
    }
    let (text, _) = storage::read_bounded(&path, MAX_WORKSPACE_BYTES)?;
    let prepared = prepare(&text, &hyprland_editor_core::catalogue())?;
    Ok(
        serde_json::json!({"scope":"reviewed-model-transition-and-current-static-reanalysis", "migration":prepared.summary, "original_paths_opened":false,"folder_access_restored":false,"runtime_verified":false,"saved_file_written":false,"needs_review_and_new_private_save":true}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_transition_pins_exact_monitor_differences_and_refuses_unreviewed_builds() {
        transition(FROM).unwrap();
        transition(FROM_PROJECTION).unwrap();
        transition(FROM_MIGRATION).unwrap();
        transition(BEFORE_MONITORS).unwrap();
        assert!(transition(&"0".repeat(64)).is_err());
        assert_ne!(FROM, TO);
        assert_eq!(lua::source_hash(CURRENT), TO);
    }
}
