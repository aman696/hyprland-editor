//! Frozen graph inputs, never parser reports, live observations or read/write authority.
use crate::{
    lua::{
        self,
        replay::{self, Binding, CompilerProfile, Document, Program},
    },
    module_path::Context,
    workspace::{SourceLanguage, SourceSnapshot, MAX_SOURCE_DOCUMENTS},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub document: String,
    pub load_name: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub format: String,
    pub schema_sha256: String,
    pub compiler: CompilerProfile,
    pub root: String,
    pub context: Context,
    pub working_directory: Option<Vec<u8>>,
    pub views: Vec<View>,
    /// Historical supplied inputs. Reopening cannot certify a current file or absence.
    pub bindings: Vec<Binding>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Bundle {
    pub graph: Graph,
    pub sources: Vec<SourceSnapshot>,
}
pub fn schema_sha256() -> String {
    lua::source_hash(include_str!("../../../data/graph-model.json"))
}
impl Graph {
    /// Structural validation only; no parsing, Lua execution or filesystem operations.
    pub fn program(
        &self,
        sources: &[SourceSnapshot],
        root: Option<&str>,
        version: &str,
    ) -> Result<Program, String> {
        if self.format != "hye-workspace-graph-1"
            || self.schema_sha256 != schema_sha256()
            || root != Some(self.root.as_str())
            || sources.is_empty()
            || sources.len() > MAX_SOURCE_DOCUMENTS
            || self.views.len() != sources.len()
            || self.bindings.len() > 128
        {
            return Err("Invalid or incompatible saved source graph".into());
        }
        let (path, unknown) = self.context.initial_path(version)?;
        let mut ids = BTreeSet::new();
        let mut documents = Vec::new();
        for view in &self.views {
            if !ids.insert(view.document.as_str()) {
                return Err("Duplicate saved source load view".into());
            }
            let source = sources
                .iter()
                .find(|s| s.id == view.document)
                .ok_or("Saved load view references an unknown source")?;
            if source.language != SourceLanguage::Lua {
                return Err("Saved graph contains a non-Lua source".into());
            }
            source.origin.validate(&source.content)?;
            documents.push(Document {
                id: source.id.clone(),
                source: source.content.clone(),
                sha256: source.origin.sha256.clone(),
                load_name: view.load_name.clone(),
            });
        }
        let source = sources
            .iter()
            .find(|s| s.id == self.root)
            .ok_or("Saved graph root missing")?;
        if source.origin.selected_path != self.context.main_file
            || documents
                .iter()
                .find(|d| d.id == self.root)
                .is_none_or(|d| d.load_name != self.context.main_file)
        {
            return Err("Saved graph root and path context disagree".into());
        }
        let program = Program {
            version: version.into(),
            profile: replay::Profile::HyprlandLua54,
            root: self.root.clone(),
            documents,
            bindings: self.bindings.clone(),
            initial_package_path: Some(path),
            initial_package_path_unknown_tail: unknown,
            working_directory: self.working_directory.clone(),
        };
        replay::validate_program(&program)?;
        Ok(program)
    }
}
impl Bundle {
    pub fn fingerprint(&self) -> Result<String, String> {
        let bytes = serde_json::to_string(self).map_err(|e| e.to_string())?;
        if bytes.len() > crate::workspace::MAX_WORKSPACE_BYTES {
            return Err("Saved graph exceeds its 8 MiB envelope".into());
        }
        Ok(lua::source_hash(&bytes))
    }
}
