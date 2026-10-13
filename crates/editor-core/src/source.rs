//! Source evidence is data, never permission to reopen or overwrite an imported path.
use crate::{lua::source_hash, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::{
    fs::Metadata,
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::Path,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordedOwnership {
    UserOwned,
    ReadOnly,
    OtherOwner,
    StorePath,
    SymlinkManaged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkIdentity {
    pub encountered_path: Vec<u8>,
    pub target: Vec<u8>,
    pub device: u64,
    pub inode: u64,
    pub owner: u32,
    pub mode: u32,
    pub changed_seconds: i64,
    pub changed_nanoseconds: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    /// Byte-safe display/provenance data. No deserializer opens this path.
    pub selected_path: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkIdentity>,
    pub device: u64,
    pub inode: u64,
    pub owner: u32,
    pub mode: u32,
    pub size: u64,
    pub modified_seconds: i64,
    pub modified_nanoseconds: i64,
    pub changed_seconds: i64,
    pub changed_nanoseconds: i64,
    pub sha256: String,
    pub recorded_ownership: RecordedOwnership,
}

#[derive(Clone, Debug)]
pub struct CapturedSource {
    pub content: String,
    pub identity: SourceIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reattachment {
    SameRevision,
    /// Identical bytes at another descriptor identity/path, requiring explicit review.
    RelocatedRevision,
}

impl SourceIdentity {
    pub(crate) fn captured(
        path: &Path,
        metadata: &Metadata,
        content: &str,
    ) -> Result<Self, String> {
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(path)
        };
        let recorded_ownership = if path.starts_with("/nix/store") {
            RecordedOwnership::StorePath
        } else if metadata.uid() != unsafe { libc::geteuid() } {
            RecordedOwnership::OtherOwner
        } else if metadata.mode() & 0o200 == 0 {
            RecordedOwnership::ReadOnly
        } else {
            RecordedOwnership::UserOwned
        };
        let identity = Self {
            selected_path: path.as_os_str().as_bytes().to_vec(),
            resolved_path: None,
            links: vec![],
            device: metadata.dev(),
            inode: metadata.ino(),
            owner: metadata.uid(),
            mode: metadata.mode(),
            size: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
            sha256: source_hash(content),
            recorded_ownership,
        };
        identity.validate(content)?;
        Ok(identity)
    }
    pub fn validate(&self, content: &str) -> Result<(), String> {
        if content.len() > MAX_DOCUMENT_BYTES
            || content.as_bytes().contains(&0)
            || self.selected_path.first() != Some(&b'/')
            || self.selected_path.len() > 4096
            || self.selected_path.contains(&0)
            || self.size != content.len() as u64
            || self.mode & libc::S_IFMT != libc::S_IFREG
            || !(0..1_000_000_000).contains(&self.modified_nanoseconds)
            || !(0..1_000_000_000).contains(&self.changed_nanoseconds)
            || self.sha256 != source_hash(content)
            || self.links.len() > 32
            || self
                .resolved_path
                .as_ref()
                .is_some_and(|p| !valid_absolute_path(p))
            || self.links.iter().any(|link| {
                !valid_absolute_path(&link.encountered_path)
                    || link.target.is_empty()
                    || link.target.len() > 4096
                    || link.target.contains(&0)
                    || link.mode & libc::S_IFMT != libc::S_IFLNK
                    || !(0..1_000_000_000).contains(&link.changed_nanoseconds)
            })
        {
            return Err("Invalid source identity or content hash".into());
        }
        Ok(())
    }
    pub fn display_path(&self) -> String {
        display_path_bytes(&self.selected_path)
    }
    /// Compares an explicitly user-selected, freshly captured file. Never reads its recorded path.
    pub fn compare_selected(&self, source: &CapturedSource) -> Result<Reattachment, String> {
        source.identity.validate(&source.content)?;
        if self.sha256 != source.identity.sha256 || self.size != source.identity.size {
            return Err("Selected source is a different revision. Import/reconcile it separately; the saved draft was not rebound.".into());
        }
        if self.selected_path == source.identity.selected_path
            && self.device == source.identity.device
            && self.inode == source.identity.inode
            && self.owner == source.identity.owner
            && self.mode == source.identity.mode
            && self.modified_seconds == source.identity.modified_seconds
            && self.modified_nanoseconds == source.identity.modified_nanoseconds
            && self.changed_seconds == source.identity.changed_seconds
            && self.changed_nanoseconds == source.identity.changed_nanoseconds
            && self.links == source.identity.links
            && self.resolved_path.as_deref().unwrap_or(&self.selected_path)
                == source
                    .identity
                    .resolved_path
                    .as_deref()
                    .unwrap_or(&source.identity.selected_path)
        {
            Ok(Reattachment::SameRevision)
        } else {
            Ok(Reattachment::RelocatedRevision)
        }
    }
}
/// Preserve readable Unicode but expose control/bidi bytes instead of letting them spoof rows.
pub fn display_path_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    for chunk in bytes.utf8_chunks() {
        for character in chunk.valid().chars() {
            if character.is_control()
                || character == '\\'
                || matches!(character,'\u{061c}'|'\u{200e}'|'\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
            {
                output.extend(character.escape_default());
            } else {
                output.push(character);
            }
        }
        for byte in chunk.invalid() {
            let _ = write!(output, "\\x{byte:02x}");
        }
    }
    output
}
fn valid_absolute_path(bytes: &[u8]) -> bool {
    bytes.first() == Some(&b'/') && bytes.len() <= 4096 && !bytes.contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::read_source;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn folder() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "he-source-{}-{}",
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
    fn captures_bytes_and_identity_and_refuses_changed_revision() {
        let root = folder();
        let path = root.join("config.lua");
        fs::write(&path, "hl.config{general={border_size=2}}\n").unwrap();
        let first = read_source(&path).unwrap();
        assert_eq!(first.identity.sha256, source_hash(&first.content));
        assert_eq!(
            first.identity.compare_selected(&first).unwrap(),
            Reattachment::SameRevision
        );
        fs::write(&path, "hl.config{general={border_size=4}}\n").unwrap();
        assert!(first
            .identity
            .compare_selected(&read_source(&path).unwrap())
            .is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn renamed_copy_is_a_reviewable_relocation_not_an_identity_match() {
        let root = folder();
        let old = root.join("old.lua");
        let new = root.join("new.lua");
        fs::write(&old, "-- identical bytes\n").unwrap();
        let first = read_source(&old).unwrap();
        fs::copy(&old, &new).unwrap();
        fs::remove_file(&old).unwrap();
        assert_eq!(
            first
                .identity
                .compare_selected(&read_source(&new).unwrap())
                .unwrap(),
            Reattachment::RelocatedRevision
        );
        // The recorded path no longer exists; comparison did not reopen it.
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn source_metadata_is_not_path_authority_and_accepts_non_utf8_names() {
        use std::os::unix::ffi::OsStringExt;
        let root = folder();
        let path = root.join(std::ffi::OsString::from_vec(b"source-\xff.lua".to_vec()));
        fs::write(&path, "-- inert\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let source = read_source(&path).unwrap();
        assert_eq!(source.identity.selected_path, path.as_os_str().as_bytes());
        assert_eq!(
            source.identity.recorded_ownership,
            RecordedOwnership::ReadOnly
        );
        let mut forged = source.identity.clone();
        forged.sha256 = "0".repeat(64);
        assert!(forged.validate(&source.content).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn path_display_exposes_newline_bidi_invalid_bytes_and_literal_backslashes() {
        assert_eq!(
            display_path_bytes(b"/source\nOwner: 0\\n-\xff"),
            "/source\\nOwner: 0\\\\n-\\xff"
        );
        assert_eq!(
            display_path_bytes("/café/\u{202e}source".as_bytes()),
            "/café/\\u{202e}source"
        );
    }
}
