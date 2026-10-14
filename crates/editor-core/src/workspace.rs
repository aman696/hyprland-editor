//! Private draft snapshots persist source bytes, not trusted parser reports or path capabilities.
use crate::{source::SourceIdentity, storage, Draft, Setting, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub const MAX_WORKSPACE_BYTES: usize = 8 * MAX_DOCUMENT_BYTES;
pub const MAX_SOURCE_DOCUMENTS: usize = 32;
pub const WORKSPACE_FORMAT: &str = "hyprland-editor-workspace-1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLanguage {
    Lua,
    Legacy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaPin {
    pub upstream_commit: String,
    pub catalogue_sha256: String,
}
impl SchemaPin {
    fn for_version(version: &str) -> Self {
        Self {
            upstream_commit: match version {
                crate::BASELINE => crate::BASELINE_COMMIT,
                crate::AUDITED_MAIN => crate::AUDITED_MAIN_COMMIT,
                _ => "unsupported",
            }
            .into(),
            catalogue_sha256: crate::lua::source_hash(include_str!("../../../data/scalars.json")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshot {
    pub id: String,
    pub language: SourceLanguage,
    pub content: String,
    pub origin: SourceIdentity,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub format: String,
    pub schema: SchemaPin,
    pub draft: Draft,
    pub root_source: Option<String>,
    pub sources: Vec<SourceSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monitors: Vec<crate::monitor::Rule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor_schema_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<crate::device::Spec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_schema_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "crate::binding::Draft::is_empty")]
    pub bindings: crate::binding::Draft,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workspace_rules: Vec<crate::workspace_rule::Rule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<crate::rule::Rule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_schema_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_schema_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding_schema_sha256: Option<String>,
}
impl Workspace {
    pub fn new(draft: Draft) -> Self {
        Self {
            format: WORKSPACE_FORMAT.into(),
            schema: SchemaPin::for_version(&draft.version),
            draft,
            root_source: None,
            sources: vec![],
            monitors: vec![],
            monitor_schema_sha256: None,
            devices: vec![],
            device_schema_sha256: None,
            bindings: Default::default(),
            workspace_rules: vec![],
            rules: vec![],
            rule_schema_sha256: None,
            workspace_schema_sha256: None,
            binding_schema_sha256: None,
        }
    }
    pub fn with_root(mut self, content: String, origin: SourceIdentity) -> Self {
        self.root_source = Some("root".into());
        self.sources.push(SourceSnapshot {
            id: "root".into(),
            language: SourceLanguage::Lua,
            content,
            origin,
        });
        self
    }
    pub fn validate(&self, catalogue: &[Setting]) -> Result<(), String> {
        if self.format != WORKSPACE_FORMAT || self.sources.len() > MAX_SOURCE_DOCUMENTS {
            return Err("Unsupported workspace format or source count".into());
        }
        self.draft.validate(catalogue)?;
        crate::rule::validate(&self.rules, &self.draft.version)?;
        let rule_hash = crate::rule::schema_hash();
        if (!self.rules.is_empty() && self.rule_schema_sha256.as_deref() != Some(&rule_hash))
            || self
                .rule_schema_sha256
                .as_ref()
                .is_some_and(|h| *h != rule_hash)
        {
            return Err("Window/layer rule schema differs from the selected audited pack".into());
        }
        crate::monitor::validate(&self.monitors, &self.draft.version)?;
        crate::device::validate(&self.devices, &self.draft.version)?;
        let device_hash = crate::device::schema_hash();
        if (!self.devices.is_empty() && self.device_schema_sha256.as_deref() != Some(&device_hash))
            || self
                .device_schema_sha256
                .as_ref()
                .is_some_and(|h| *h != device_hash)
        {
            return Err("Device schema differs from the audited fields".into());
        }
        crate::workspace_rule::validate(&self.workspace_rules, &self.draft.version)?;
        let workspace_hash = crate::workspace_rule::schema_hash();
        if (!self.workspace_rules.is_empty()
            && self.workspace_schema_sha256.as_deref() != Some(&workspace_hash))
            || self
                .workspace_schema_sha256
                .as_ref()
                .is_some_and(|h| *h != workspace_hash)
        {
            return Err("Workspace-rule schema differs from the audited fields".into());
        }
        self.bindings.validate(&self.draft.version)?;
        let binding_hash = crate::binding::schema_hash();
        if (!self.bindings.is_empty()
            && self.binding_schema_sha256.as_deref() != Some(&binding_hash))
            || self
                .binding_schema_sha256
                .as_ref()
                .is_some_and(|h| *h != binding_hash)
        {
            return Err("Binding/action schema differs from the selected audited pack".into());
        }
        let monitor_hash = crate::monitor::schema_hash();
        if (!self.monitors.is_empty()
            && self.monitor_schema_sha256.as_deref() != Some(&monitor_hash))
            || self
                .monitor_schema_sha256
                .as_ref()
                .is_some_and(|hash| *hash != monitor_hash)
        {
            return Err("Monitor draft schema differs from the audited fields".into());
        }
        if self.schema != SchemaPin::for_version(&self.draft.version) {
            return Err("Workspace schema differs from this exact audited pack; an explicit migration is required".into());
        }
        let mut ids = BTreeSet::new();
        let mut bytes = 0;
        for source in &self.sources {
            if source.id.is_empty()
                || source.id.len() > 64
                || !source
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !ids.insert(source.id.as_str())
            {
                return Err("Invalid or duplicate source document ID".into());
            }
            source.origin.validate(&source.content)?;
            bytes += source.content.len();
            if bytes > 4 * MAX_DOCUMENT_BYTES {
                return Err("Workspace source bytes exceed 4 MiB".into());
            }
        }
        if self
            .root_source
            .as_deref()
            .is_some_and(|root| !ids.contains(root))
            || (self.root_source.is_none() && !self.sources.is_empty())
        {
            return Err("Invalid workspace source root".into());
        }
        Ok(())
    }
    pub fn to_json(&self, catalogue: &[Setting]) -> Result<Vec<u8>, String> {
        self.validate(catalogue)?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_WORKSPACE_BYTES {
            return Err("Workspace exceeds 8 MiB".into());
        }
        Ok(bytes)
    }
    pub fn from_json(text: &str, catalogue: &[Setting]) -> Result<Self, String> {
        if text.len() > MAX_WORKSPACE_BYTES {
            return Err("Workspace exceeds 8 MiB".into());
        }
        let workspace: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        workspace.validate(catalogue)?;
        Ok(workspace)
    }
    pub fn root(&self) -> Option<&SourceSnapshot> {
        self.root_source
            .as_deref()
            .and_then(|id| self.sources.iter().find(|s| s.id == id))
    }
    pub fn with_rules(mut self, rules: Vec<crate::rule::Rule>) -> Self {
        self.rule_schema_sha256 = (!rules.is_empty()).then(crate::rule::schema_hash);
        self.rules = rules;
        self
    }
    pub fn with_workspace_rules(mut self, rules: Vec<crate::workspace_rule::Rule>) -> Self {
        self.workspace_schema_sha256 = (!rules.is_empty()).then(crate::workspace_rule::schema_hash);
        self.workspace_rules = rules;
        self
    }
    pub fn with_bindings(mut self, bindings: crate::binding::Draft) -> Self {
        self.binding_schema_sha256 = (!bindings.is_empty()).then(crate::binding::schema_hash);
        self.bindings = bindings;
        self
    }
    pub fn with_devices(mut self, devices: Vec<crate::device::Spec>) -> Self {
        self.device_schema_sha256 = (!devices.is_empty()).then(crate::device::schema_hash);
        self.devices = devices;
        self
    }
    pub fn with_monitors(mut self, monitors: Vec<crate::monitor::Rule>) -> Self {
        self.monitor_schema_sha256 = (!monitors.is_empty()).then(crate::monitor::schema_hash);
        self.monitors = monitors;
        self
    }
}
pub fn save(
    state_root: &Path,
    workspace: &Workspace,
    catalogue: &[Setting],
) -> Result<PathBuf, String> {
    let bytes = workspace.to_json(catalogue)?;
    storage::save_private(state_root, &bytes, "workspace", "hye")
}
pub fn read(path: &Path, catalogue: &[Setting]) -> Result<Workspace, String> {
    let (text, _) = storage::read_bounded(path, MAX_WORKSPACE_BYTES)?;
    Workspace::from_json(&text, catalogue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue, storage::read_source, BASELINE};
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
        time::{SystemTime, UNIX_EPOCH},
    };
    fn folder() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "he-workspace-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        root
    }
    fn snapshot(root: &Path) -> Workspace {
        let path = root.join("synthetic.lua");
        fs::write(
            &path,
            "-- keep this comment\nhl.config{general={border_size=3}}\n",
        )
        .unwrap();
        let source = read_source(&path).unwrap();
        Workspace::new(Draft::new(BASELINE)).with_root(source.content, source.identity)
    }
    #[test]
    fn saves_private_immutable_source_and_restores_without_original_path() {
        let root = folder();
        let workspace = snapshot(&root);
        let saved = save(&root, &workspace, &catalogue()).unwrap();
        assert_eq!(
            fs::metadata(&saved).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::remove_file(root.join("synthetic.lua")).unwrap();
        let restored = read(&saved, &catalogue()).unwrap();
        assert_eq!(
            restored.root().unwrap().content,
            workspace.root().unwrap().content
        );
        assert_eq!(
            restored.root().unwrap().origin,
            workspace.root().unwrap().origin
        );
        let another = save(&root, &workspace, &catalogue()).unwrap();
        assert_ne!(saved, another);
        assert!(fs::read_dir(root.join("hyprland-editor"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_tampered_hash_unknown_fields_dangling_roots_and_duplicate_ids() {
        let root = folder();
        let base = snapshot(&root);
        let mut bad = base.clone();
        bad.sources[0].content.push_str("-- changed");
        assert!(bad.validate(&catalogue()).is_err());
        bad = base.clone();
        bad.root_source = Some("missing".into());
        assert!(bad.validate(&catalogue()).is_err());
        bad = base.clone();
        bad.sources.push(bad.sources[0].clone());
        assert!(bad.validate(&catalogue()).is_err());
        bad = base.clone();
        bad.schema.catalogue_sha256 = "0".repeat(64);
        assert!(bad.validate(&catalogue()).is_err());
        let mut json: serde_json::Value =
            serde_json::from_slice(&base.to_json(&catalogue()).unwrap()).unwrap();
        json["parser_report"] = serde_json::json!({"trusted":true});
        assert!(Workspace::from_json(&json.to_string(), &catalogue()).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_oversize_and_symlinked_workspaces() {
        let root = folder();
        let base = snapshot(&root);
        let mut bad = base.clone();
        bad.sources[0].content = "x".repeat(MAX_DOCUMENT_BYTES + 1);
        assert!(bad.validate(&catalogue()).is_err());
        assert!(Workspace::from_json(&" ".repeat(MAX_WORKSPACE_BYTES + 1), &catalogue()).is_err());
        let saved = save(&root, &base, &catalogue()).unwrap();
        let link = root.join("linked.hye");
        symlink(&saved, &link).unwrap();
        assert!(read(&link, &catalogue()).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn symlink_source_evidence_round_trips_without_reopening_the_link_or_target() {
        let root = folder();
        fs::write(root.join("target.lua"), "-- target\n").unwrap();
        symlink("target.lua", root.join("entry.lua")).unwrap();
        let source = crate::source_path::read_selected(&root.join("entry.lua")).unwrap();
        let snapshot =
            Workspace::new(Draft::new(BASELINE)).with_root(source.content, source.identity);
        let saved = save(&root, &snapshot, &catalogue()).unwrap();
        fs::remove_file(root.join("target.lua")).unwrap();
        fs::remove_file(root.join("entry.lua")).unwrap();
        let restored = read(&saved, &catalogue()).unwrap();
        assert_eq!(
            restored.root().unwrap().origin,
            snapshot.root().unwrap().origin
        );
        assert_eq!(restored.root().unwrap().content, "-- target\n");
        fs::remove_dir_all(root).unwrap();
    }
}
