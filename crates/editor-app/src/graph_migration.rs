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
const MONITOR_VALUES: &str = "d7dd575308699007b7083b5776cfdfb6268b84b78fd4bd7f7f29c023c24f70cd";
const VALUES_MODEL: &str = include_str!("../../../data/migrations/graph-model-monitor-values.json");
const MONITOR_FIELDS: &str = "05bb2e4e86ef118289b70942ff23ae80ef07ad6ea16c3f44dd676e542c0ece37";
const FIELDS_MODEL: &str = include_str!("../../../data/migrations/graph-model-monitor-fields.json");
const MONITOR_DECLARATIONS: &str =
    "cfa1292e40e8e8111d2f279f98cfeed192f35b05b7a6af49c024c23c47085783";
const DECLARATIONS_MODEL: &str =
    include_str!("../../../data/migrations/graph-model-monitor-declarations.json");
const MONITOR_LAYOUT: &str = "1dacfb662caa635f63047517d223d73ec0c1922b2d9f93b3a7fa35cecb91909e";
const LAYOUT_MODEL: &str = include_str!("../../../data/migrations/graph-model-monitor-layout.json");
const MONITOR_REVIEW: &str = "be2a3bceb486422b079705792d7dae67f4922f96fcfc304004df20fbc22c92f1";
const REVIEW_MODEL: &str = include_str!("../../../data/migrations/graph-model-monitor-review.json");
const MIXED_REVIEW: &str = "dbb7803fa4ea318a5b4511fd1f44eb422be73feebd8342ab449a8a4ffa27351d";
const MIXED_MODEL: &str = include_str!("../../../data/migrations/graph-model-mixed-review.json");
const TEXT_MERGE: &str = "071946f5f808e82ef8f38d67fa919d7a427299f2c41c0f5e73368d7c3ee0aae7";
const TEXT_MODEL: &str = include_str!("../../../data/migrations/graph-model-text-merge.json");
const COMPOSITION: &str = "71556836e091dccaaf231bc4510e448e545e588c175a56e59ad8c7d9cd2a3972";
const COMPOSITION_MODEL: &str =
    include_str!("../../../data/migrations/graph-model-composition.json");
const PRIVATE_INPUTS: &str = "9f77616b432badc333c5a3b70cfbca4d7ed9fe4557ba87b97b7da5348940044f";
const PRIVATE_MODEL: &str =
    include_str!("../../../data/migrations/graph-model-private-inputs.json");
const CORRESPONDENCE: &str = "e25d982f292e56135f0810194ae06ae6a93a45d4b6f6afd75bb0ada5ecf7b0fc";
const CORRESPONDENCE_MODEL: &str =
    include_str!("../../../data/migrations/graph-model-correspondence.json");
const RESOLUTION: &str = "aaa60b286d173a7d4b8818bea1461ce7e87c7a054788fcf90bbd58f8b9925d75";
const RESOLUTION_MODEL: &str = include_str!("../../../data/migrations/graph-model-resolution.json");
const WORKSPACE_STATE: &str = "a556ebf4459ee6ee9196ca2593faaa48c2d8c4d8b383a7b030c54e1cf951778c";
const STATE_MODEL: &str = include_str!("../../../data/migrations/graph-model-workspace-state.json");
const COMPILER_EXTRACTION: &str =
    "88ba4156fabecf287c34d467b80ba9db8e13a8682afca20af55c106cfc850922";
const COMPILER_MODEL: &str =
    include_str!("../../../data/migrations/graph-model-compiler-extraction.json");
const NUMERIC_CODEGEN: &str = "2f9e39f0f1efb1e7be298cfbc50c43ed754f7e3feb68d64d7cb7ed7fbca76128";
const NUMERIC_MODEL: &str =
    include_str!("../../../data/migrations/graph-model-numeric-codegen.json");
const TO: &str = "09e8ff83ee94a51dcd54a7c7cbabc543d956b4e1c50eb5369b3b56cf34b3b0b9";
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
    pub private_documents: usize,
    pub private_resolution_inputs: usize,
    pub archived_documents: usize,
    pub reanalysis_findings: usize,
    pub reanalysis_note: Option<String>,
    pub compiler_observation_renewed: bool,
    pub compiler_numeric_mode_known: bool,
}
pub(super) struct Prepared {
    pub workspace: Workspace,
    pub graph: Arc<super::source_graph::Saved>,
    pub summary: Summary,
}
fn before_field_transition(from: &str) -> Result<(), String> {
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
    let next: serde_json::Value = serde_json::from_str(VALUES_MODEL).map_err(|e| e.to_string())?;
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
fn before_declaration_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(VALUES_MODEL) != MONITOR_VALUES || workspace_graph::schema_sha256() != TO {
        return Err("No reviewed monitor field migration to this exact model is available".into());
    }
    if from != MONITOR_VALUES {
        before_field_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(VALUES_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(FIELDS_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid monitor value model inventory")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid monitor field model inventory")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 62
        || new.len() != 63
        || added != ["crates/editor-core/src/lua/replay/monitor_edit/fields.rs"]
    {
        return Err("Reviewed monitor field migration input set changed".into());
    }
    let changes = old
        .keys()
        .filter(|k| old[*k] != new.get(*k).cloned().unwrap_or_default())
        .map(String::as_str)
        .collect::<Vec<_>>();
    if changes
        != [
            "crates/editor-app/src/worker.rs",
            "crates/editor-core/src/lua/replay/monitor_edit.rs",
            "crates/editor-core/src/lua/replay/scalar_edit.rs",
            "crates/editor-core/src/lua/replay/scalar_edit/layout.rs",
        ]
    {
        return Err(
            "Monitor field migration no longer has only the reviewed editor and trivia changes"
                .into(),
        );
    }
    Ok(())
}
fn before_monitor_layout_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(FIELDS_MODEL) != MONITOR_FIELDS
        || lua::source_hash(DECLARATIONS_MODEL) != MONITOR_DECLARATIONS
    {
        return Err("No reviewed declaration transition to this exact model is available".into());
    }
    if from != MONITOR_FIELDS {
        before_declaration_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(FIELDS_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(DECLARATIONS_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous field model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid declaration model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 63
        || new.len() != 65
        || added
            != [
                "crates/editor-core/src/lua/replay/edits/structure.rs",
                "crates/editor-core/src/lua/replay/monitor_edit/declarations.rs",
            ]
    {
        return Err("Reviewed declaration model input set changed".into());
    }
    let changes = old
        .keys()
        .filter(|k| old[*k] != new.get(*k).cloned().unwrap_or_default())
        .map(String::as_str)
        .collect::<Vec<_>>();
    if changes
        != [
            "crates/editor-app/src/worker.rs",
            "crates/editor-core/src/lua/replay/edits.rs",
            "crates/editor-core/src/lua/replay/monitor_edit.rs",
            "crates/editor-core/src/workspace_graph/revisions.rs",
        ]
    {
        return Err("Declaration model no longer has only the reviewed structural editor and resolution preservation changes".into());
    }
    Ok(())
}
fn before_monitor_review_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(DECLARATIONS_MODEL) != MONITOR_DECLARATIONS
        || lua::source_hash(LAYOUT_MODEL) != MONITOR_LAYOUT
    {
        return Err(
            "No reviewed monitor layout transition to this exact model is available".into(),
        );
    }
    if from != MONITOR_DECLARATIONS {
        before_monitor_layout_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(DECLARATIONS_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(LAYOUT_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous declaration model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid monitor layout model")?;
    if old.len() != 65 || new.len() != 65 || old.keys().ne(new.keys()) {
        return Err("Reviewed monitor layout model input set changed".into());
    }
    let changes = old
        .keys()
        .filter(|k| old[*k] != new[*k])
        .map(String::as_str)
        .collect::<Vec<_>>();
    if changes
        != [
            "crates/editor-core/src/lua/replay/monitor_edit.rs",
            "crates/editor-core/src/lua/replay/scalar_edit/layout.rs",
        ]
    {
        return Err("Monitor layout model no longer has only the reviewed monitor editor and shared layout visibility changes".into());
    }
    Ok(())
}
fn before_mixed_review_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(LAYOUT_MODEL) != MONITOR_LAYOUT
        || lua::source_hash(REVIEW_MODEL) != MONITOR_REVIEW
    {
        return Err(
            "No reviewed monitor reconciliation transition to this exact model is available".into(),
        );
    }
    if from != MONITOR_LAYOUT {
        before_monitor_review_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(LAYOUT_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(REVIEW_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous monitor layout model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid monitor reconciliation model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 65
        || new.len() != 66
        || added != ["crates/editor-core/src/lua/replay/monitor_review.rs"]
        || changed
            != [
                "crates/editor-app/src/worker.rs",
                "crates/editor-core/src/lua/replay.rs",
                "crates/editor-core/src/lua/replay/monitor_edit.rs",
            ]
    {
        return Err("Monitor reconciliation model no longer has only the reviewed module, worker and helper visibility changes".into());
    }
    Ok(())
}
fn before_text_merge_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(REVIEW_MODEL) != MONITOR_REVIEW
        || lua::source_hash(MIXED_MODEL) != MIXED_REVIEW
    {
        return Err(
            "No reviewed mixed-family reconciliation transition to this exact model is available"
                .into(),
        );
    }
    if from != MONITOR_REVIEW {
        before_mixed_review_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(REVIEW_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(MIXED_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous monitor review model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid mixed reconciliation model")?;
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 66
        || new.len() != 66
        || old.keys().ne(new.keys())
        || changed != ["crates/editor-core/src/lua/replay/override_review.rs"]
    {
        return Err("Mixed reconciliation model no longer has only the reviewed scalar inspection prefix and structural-intent rejection changes".into());
    }
    Ok(())
}
fn before_composition_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(MIXED_MODEL) != MIXED_REVIEW || lua::source_hash(TEXT_MODEL) != TEXT_MERGE {
        return Err("No reviewed source merge transition to this exact model is available".into());
    }
    if from != MIXED_REVIEW {
        before_text_merge_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(MIXED_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value = serde_json::from_str(TEXT_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous mixed-call model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid source merge model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 66
        || new.len() != 70
        || added
            != [
                "crates/editor-core/src/source_merge.rs",
                "crates/editor-core/src/source_merge/diff.rs",
                "crates/editor-core/src/source_merge/tests.rs",
                "crates/editor-core/src/workspace_graph/merge.rs",
            ]
        || changed
            != [
                "crates/editor-app/src/worker.rs",
                "crates/editor-core/src/lib.rs",
                "crates/editor-core/src/workspace_graph.rs",
            ]
    {
        return Err("Source merge model no longer has only the reviewed merge modules, registrations and worker changes".into());
    }
    Ok(())
}
fn before_private_inputs_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(TEXT_MODEL) != TEXT_MERGE
        || lua::source_hash(COMPOSITION_MODEL) != COMPOSITION
    {
        return Err(
            "No reviewed graph composition transition to this exact model is available".into(),
        );
    }
    if from != TEXT_MERGE {
        before_composition_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(TEXT_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(COMPOSITION_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous text merge model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid graph composition model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 70
        || new.len() != 71
        || added != ["crates/editor-core/src/workspace_graph/merge/pieces.rs"]
        || changed != ["crates/editor-core/src/workspace_graph/merge.rs"]
    {
        return Err("Graph composition model no longer has only the reviewed provenance composition and bounded hashing changes".into());
    }
    Ok(())
}
fn before_private_correspondence_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(COMPOSITION_MODEL) != COMPOSITION
        || lua::source_hash(PRIVATE_MODEL) != PRIVATE_INPUTS
    {
        return Err("No reviewed private input transition to this exact model is available".into());
    }
    if from != COMPOSITION {
        before_private_inputs_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(COMPOSITION_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(PRIVATE_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous graph composition model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid private input model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 71
        || new.len() != 72
        || added != ["crates/editor-core/src/workspace_graph/private.rs"]
        || changed
            != [
                "crates/editor-app/src/source_graph.rs",
                "crates/editor-app/src/worker.rs",
                "crates/editor-core/src/lua/replay/edits.rs",
                "crates/editor-core/src/workspace_graph.rs",
                "crates/editor-core/src/workspace_graph/merge.rs",
                "crates/editor-core/src/workspace_graph/revisions.rs",
            ]
    {
        return Err("Private input model no longer has only the reviewed explicit source layer, restricted reconstruction, merge refusal and provenance-preserving piece changes".into());
    }
    Ok(())
}
fn before_resolution_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(PRIVATE_MODEL) != PRIVATE_INPUTS
        || lua::source_hash(CORRESPONDENCE_MODEL) != CORRESPONDENCE
    {
        return Err(
            "No reviewed private graph correspondence transition to this exact model is available"
                .into(),
        );
    }
    if from != PRIVATE_INPUTS {
        before_private_correspondence_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(PRIVATE_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(CORRESPONDENCE_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid private input model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid private correspondence model")?;
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 72
        || old.keys().collect::<BTreeSet<_>>() != new.keys().collect()
        || changed
            != [
                "crates/editor-core/src/workspace_graph/merge.rs",
                "crates/editor-core/src/workspace_graph/merge/pieces.rs",
            ]
    {
        return Err("Private correspondence model no longer has only the reviewed view mapping, explicit archival and provenance-preserving composition changes".into());
    }
    Ok(())
}
fn before_workspace_state_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(CORRESPONDENCE_MODEL) != CORRESPONDENCE
        || lua::source_hash(RESOLUTION_MODEL) != RESOLUTION
    {
        return Err(
            "No reviewed private resolution transition to this exact model is available".into(),
        );
    }
    if from != CORRESPONDENCE {
        before_resolution_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(CORRESPONDENCE_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(RESOLUTION_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous correspondence model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid private resolution model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 72
        || new.len() != 73
        || added != ["crates/editor-app/src/source_graph/private_inputs.rs"]
        || changed
            != [
                "crates/editor-app/src/source_graph.rs",
                "crates/editor-app/src/worker.rs",
            ]
    {
        return Err("Private resolution model no longer has only the reviewed cancellable reconstruction, owned parser cleanup and explicit pending-input coordinator changes".into());
    }
    Ok(())
}
fn before_compiler_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(RESOLUTION_MODEL) != RESOLUTION || workspace_graph::schema_sha256() != TO {
        return Err(
            "No reviewed workspace state transition to this exact model is available".into(),
        );
    }
    if from != RESOLUTION {
        before_workspace_state_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(RESOLUTION_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(STATE_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous resolution model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid workspace state model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 73
        || new.len() != 77
        || added
            != [
                "crates/editor-core/src/workspace_rule/source_state.rs",
                "crates/editor-core/src/workspace_rule/source_state/tests.rs",
                "data/workspace-argument-semantics.json",
                "data/workspace-object-semantics.json",
            ]
        || changed != ["crates/editor-core/src/workspace_rule.rs"]
    {
        return Err("Workspace state model no longer has only the reviewed internal state module, post-parse/native-reference tests and pinned factory/object metadata; Lua routing and saved representations must remain unchanged".into());
    }
    Ok(())
}
fn before_numeric_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(STATE_MODEL) != WORKSPACE_STATE || workspace_graph::schema_sha256() != TO {
        return Err(
            "No reviewed compiler-extraction transition to this exact model is available".into(),
        );
    }
    if from != WORKSPACE_STATE {
        before_compiler_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(STATE_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(COMPILER_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous state model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid current compiler model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 77
        || new.len() != 78
        || added != ["crates/editor-lua55/native/bridge.inc"]
        || changed
            != [
                "Cargo.lock",
                "Cargo.toml",
                "crates/editor-app/src/process_policy.rs",
                "crates/editor-core/build.rs",
                "crates/editor-core/native/lua_syntax.cpp",
            ]
    {
        return Err("Compiler extraction no longer has only the reviewed workspace dependency, legacy wrapper, shared adapter and separate-role policy additions; Lua 5.4 replay and saved representations must remain unchanged".into());
    }
    Ok(())
}
fn before_admission_transition(from: &str) -> Result<(), String> {
    if lua::source_hash(COMPILER_MODEL) != COMPILER_EXTRACTION
        || workspace_graph::schema_sha256() != TO
    {
        return Err(
            "No reviewed numeric compiler transition to this exact model is available".into(),
        );
    }
    if from != COMPILER_EXTRACTION {
        before_numeric_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(COMPILER_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value =
        serde_json::from_str(NUMERIC_MODEL).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous compiler model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid numeric compiler model")?;
    let added = new
        .keys()
        .filter(|k| !old.contains_key(*k))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let changed = old
        .keys()
        .filter(|k| new.get(*k) != Some(&old[*k]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 78
        || new.len() != 80
        || added
            != [
                "crates/editor-core/src/lua/replay/numeric_codegen.rs",
                "data/lua-numeric-codegen.json",
            ]
        || changed
            != [
                "crates/editor-core/src/lua/replay.rs",
                "crates/editor-core/src/lua/replay/eval.rs",
                "crates/editor-lua55/native/bridge.inc",
            ]
    {
        return Err("Numeric transition no longer has only the reviewed fixed compile/dump observation, bounded descriptor model, affected subtraction handling and optional compiler metadata".into());
    }
    Ok(())
}
fn transition(from: &str) -> Result<(), String> {
    if lua::source_hash(NUMERIC_MODEL) != NUMERIC_CODEGEN || workspace_graph::schema_sha256() != TO
    {
        return Err(
            "No reviewed native release-admission transition to this model is available".into(),
        );
    }
    if from != NUMERIC_CODEGEN {
        before_admission_transition(from)?;
    }
    let previous: serde_json::Value =
        serde_json::from_str(NUMERIC_MODEL).map_err(|e| e.to_string())?;
    let current: serde_json::Value = serde_json::from_str(CURRENT).map_err(|e| e.to_string())?;
    let old = previous["inputs"]
        .as_object()
        .ok_or("Invalid previous numeric model")?;
    let new = current["inputs"]
        .as_object()
        .ok_or("Invalid native admission model")?;
    let changed = old
        .keys()
        .filter(|key| new.get(*key) != Some(&old[*key]))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if old.len() != 80
        || old.keys().ne(new.keys())
        || changed
            != [
                "crates/editor-core/build.rs",
                "crates/editor-core/native/lua_syntax.cpp",
                "crates/editor-lua55/native/bridge.inc",
            ]
    {
        return Err("Native admission transition no longer contains only the reviewed legacy adapter/link ordering and shared preconstructor release guard".into());
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
    if previous != PRIVATE_INPUTS
        && previous != CORRESPONDENCE
        && previous != RESOLUTION
        && previous != WORKSPACE_STATE
        && previous != COMPILER_EXTRACTION
        && previous != NUMERIC_CODEGEN
        && raw["source_graph"].get("private").is_some()
    {
        return Err("Previous graph writers had no private input layer; new payloads cannot be introduced during migration".into());
    }
    if previous != NUMERIC_CODEGEN
        && raw["source_graph"]["compiler"]
            .get("numeric_codegen")
            .is_some()
    {
        return Err("Previous compiler writers had no numeric observation; saved claims cannot supply a new compiler observation".into());
    }
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
    let previous_compiler = graph.compiler.clone();
    let current_compiler = lua::replay::compiler_profile();
    let compiler_observation_renewed = previous != NUMERIC_CODEGEN;
    let mut expected_compiler = current_compiler.clone();
    if compiler_observation_renewed {
        expected_compiler.numeric_codegen = None;
    }
    if previous_compiler != expected_compiler {
        return Err("Saved compiler differs from the reviewed current identity; this review cannot change compiler family, release or an existing numeric observation".into());
    }
    let mut summary = Summary {
        previous_model: previous.clone(),
        current_model: TO.into(),
        input_sha256: lua::source_hash(text),
        rebuilt_workspace_sha256: String::new(),
        captured_documents: workspace.sources.len(),
        private_revisions: graph.revisions.len(),
        resolution_inputs: graph.bindings.len(),
        private_documents: graph.private.sources.len(),
        private_resolution_inputs: graph.private.bindings.len(),
        archived_documents: graph.private.archived.len(),
        reanalysis_findings: 0,
        reanalysis_note: None,
        compiler_observation_renewed,
        compiler_numeric_mode_known: current_compiler
            .numeric_codegen
            .as_ref()
            .is_some_and(|p| p.zero_subtraction.is_some()),
    };
    graph.schema_sha256 = TO.into();
    graph.compiler = current_compiler;
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
    reversed.source_graph.as_mut().unwrap().compiler = previous_compiler;
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
        transition(MONITOR_VALUES).unwrap();
        transition(MONITOR_FIELDS).unwrap();
        transition(MONITOR_DECLARATIONS).unwrap();
        transition(MONITOR_LAYOUT).unwrap();
        transition(MONITOR_REVIEW).unwrap();
        transition(MIXED_REVIEW).unwrap();
        transition(TEXT_MERGE).unwrap();
        transition(COMPOSITION).unwrap();
        transition(PRIVATE_INPUTS).unwrap();
        transition(CORRESPONDENCE).unwrap();
        transition(RESOLUTION).unwrap();
        transition(WORKSPACE_STATE).unwrap();
        transition(COMPILER_EXTRACTION).unwrap();
        transition(NUMERIC_CODEGEN).unwrap();
        assert!(transition(&"0".repeat(64)).is_err());
        assert_ne!(FROM, TO);
        assert_eq!(lua::source_hash(CURRENT), TO);
    }
}
