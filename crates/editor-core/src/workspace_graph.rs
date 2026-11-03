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
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::Arc;
pub mod merge;
pub mod private;
pub mod revisions;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub document: String,
    pub load_name: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    #[serde(default, skip_serializing_if = "private::Inputs::is_empty")]
    pub private: private::Inputs,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revisions: Vec<revisions::Revision>,
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
    pub sources: Arc<Vec<SourceSnapshot>>,
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
        self.private.validate(&program, &self.revisions)?;
        Ok(program)
    }
}
impl Bundle {
    pub fn fingerprint(&self) -> Result<String, String> {
        self.fingerprint_with_size().map(|v| v.0)
    }
    pub fn fingerprint_with_size(&self) -> Result<(String, usize), String> {
        encoded(self)
    }
    pub fn component_sizes(&self) -> Result<(usize, usize, usize), String> {
        let mut metadata = self.graph.clone();
        metadata.private.sources = Arc::new(vec![]);
        Ok((
            encoded(self.sources.as_ref())?.1,
            encoded(self.graph.private.sources.as_ref())?.1,
            encoded(&metadata)?.1,
        ))
    }
}
fn encoded(value: &impl Serialize) -> Result<(String, usize), String> {
    struct Hash {
        hash: Sha256,
        bytes: usize,
    }
    impl std::io::Write for Hash {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes = self.bytes.saturating_add(bytes.len());
            if self.bytes > crate::workspace::MAX_WORKSPACE_BYTES {
                return Err(std::io::Error::other("saved graph envelope limit"));
            }
            self.hash.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut hash = Hash {
        hash: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut hash, value)
        .map_err(|_| "Saved graph exceeds its 8 MiB envelope".to_string())?;
    Ok((format!("{:x}", hash.hash.finalize()), hash.bytes))
}
pub fn program_fingerprint(program: &Program) -> Result<String, String> {
    replay::validate_program(program)?;
    encoded(program).map(|v| v.0)
}
