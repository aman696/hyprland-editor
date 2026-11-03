//! Held folder authority. Serialized identity/path evidence alone cannot construct a scope.
//! Application callers use this inside the capture process, never in a parser worker.
use crate::{
    source::{CapturedSource, LinkIdentity},
    source_path::{self, SelectedSource},
    MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
mod probe;
pub use probe::{Failure, FailureKind, NodeEvidence, Observation, Stage, Status};
use std::{
    ffi::CString,
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub selected_path: Vec<u8>,
    pub resolved_path: Vec<u8>,
    pub device: u64,
    pub inode: u64,
    pub mount_id: u64,
    pub owner: u32,
    pub group: u32,
    pub mode: u32,
    pub links: Vec<LinkIdentity>,
}
impl Identity {
    /// Data validation only; neither this nor candidate_path grants access or opens paths.
    pub fn validate(&self) -> Result<(), String> {
        if !path_ok(&self.selected_path)
            || !path_ok(&self.resolved_path)
            || self.mode & libc::S_IFMT != libc::S_IFDIR
            || self.links.len() > 32
            || self.links.iter().any(|l| {
                !path_ok(&l.encountered_path)
                    || l.target.is_empty()
                    || l.target.len() > 4096
                    || l.target.contains(&0)
                    || l.mode & libc::S_IFMT != libc::S_IFLNK
                    || !(0..1_000_000_000).contains(&l.changed_nanoseconds)
            })
        {
            return Err("Invalid folder identity evidence".into());
        }
        Ok(())
    }
    pub fn candidate_path(&self, candidate: &[u8]) -> Result<Vec<u8>, String> {
        if candidate.is_empty() || candidate.len() > 4096 || candidate.contains(&0) {
            return Err("Invalid dependency path".into());
        }
        join_absolute(&self.selected_path, self.relative(candidate)?)
    }
    fn relative<'a>(&self, candidate: &'a [u8]) -> Result<&'a [u8], String> {
        if !candidate.starts_with(b"/") {
            return Ok(candidate);
        }
        for root in [&self.selected_path, &self.resolved_path] {
            let root = root.strip_suffix(b"/").unwrap_or(root);
            if candidate == root {
                return Ok(b".");
            }
            if let Some(tail) = candidate
                .strip_prefix(root)
                .and_then(|p| p.strip_prefix(b"/"))
            {
                return Ok(tail);
            }
        }
        Err("Dependency is outside the approved folder; select its target separately".into())
    }
}
/// A live non-serializable descriptor is required in addition to the identity record.
pub struct Scope {
    node: File,
    identity: Identity,
}
fn path_ok(path: &[u8]) -> bool {
    path.first() == Some(&b'/') && path.len() <= 4096 && !path.contains(&0)
}
fn held_path(node: &File) -> Result<Vec<u8>, String> {
    let own = CString::new(format!("/proc/self/fd/{}", node.as_raw_fd())).unwrap();
    let mut bytes = vec![0; 4097];
    let n = unsafe { libc::readlink(own.as_ptr(), bytes.as_mut_ptr().cast(), bytes.len()) };
    if n <= 0 || n > 4096 {
        return Err("Cannot record approved folder location".into());
    }
    bytes.truncate(n as usize);
    Ok(bytes)
}
fn mount_id(node: &File) -> Result<u64, String> {
    let mut stat = std::mem::MaybeUninit::<libc::statx>::uninit();
    if unsafe {
        libc::statx(
            node.as_raw_fd(),
            c"".as_ptr(),
            libc::AT_EMPTY_PATH | libc::AT_SYMLINK_NOFOLLOW | libc::AT_NO_AUTOMOUNT,
            libc::STATX_MNT_ID,
            stat.as_mut_ptr(),
        )
    } != 0
    {
        return Err("Folder capture requires descriptor mount-identity support".into());
    }
    let stat = unsafe { stat.assume_init() };
    if stat.stx_mask & libc::STATX_MNT_ID == 0 {
        return Err("Folder mount identity is unavailable".into());
    }
    Ok(stat.stx_mnt_id)
}
impl Scope {
    /// Only the user's explicit folder selection may reach this operation.
    pub fn select(path: &Path) -> Result<Self, String> {
        let selected = source_path::resolve_node(path, true)?;
        let metadata = selected.node.metadata().map_err(|e| e.to_string())?;
        for (link, held) in selected.links.iter().zip(&selected.held_links) {
            let now = held.metadata().map_err(|e| e.to_string())?;
            if now.nlink() == 0
                || now.dev() != link.device
                || now.ino() != link.inode
                || now.uid() != link.owner
                || now.mode() != link.mode
                || now.ctime() != link.changed_seconds
                || now.ctime_nsec() != link.changed_nanoseconds
            {
                return Err("A selected folder link changed during approval".into());
            }
        }
        let identity = Identity {
            selected_path: selected.absolute.as_os_str().as_bytes().to_vec(),
            resolved_path: held_path(&selected.node)?,
            device: metadata.dev(),
            inode: metadata.ino(),
            mount_id: mount_id(&selected.node)?,
            owner: metadata.uid(),
            group: metadata.gid(),
            mode: metadata.mode(),
            links: selected.links,
        };
        // Probe the exact containment flags before issuing a folder grant. No file bytes open.
        let _probe = open_beneath(&selected.node, b".")?;
        Self::from_descriptor(selected.node, identity)
    }
    /// Identity records are not authority: the caller must supply a live O_PATH directory FD.
    /// The application obtains that FD only from its explicit approval operation.
    pub fn from_descriptor(node: File, identity: Identity) -> Result<Self, String> {
        identity.validate()?;
        let scope = Self { node, identity };
        let flags = unsafe { libc::fcntl(scope.node.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 || flags & libc::O_PATH == 0 {
            return Err("Folder capture requires a held O_PATH descriptor".into());
        }
        scope.validate()?;
        Ok(scope)
    }
    pub fn validate(&self) -> Result<(), String> {
        let now = self.node.metadata().map_err(|e| e.to_string())?;
        if !now.is_dir()
            || now.nlink() == 0
            || now.dev() != self.identity.device
            || now.ino() != self.identity.inode
            || mount_id(&self.node)? != self.identity.mount_id
            || now.uid() != self.identity.owner
            || now.gid() != self.identity.group
            || now.mode() != self.identity.mode
        {
            return Err("Approved folder identity or permissions changed; select it again".into());
        }
        Ok(())
    }
    pub fn node(&self) -> &File {
        &self.node
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn into_parts(self) -> (File, Identity) {
        (self.node, self.identity)
    }
    pub fn selected_path(&self, candidate: &[u8]) -> Result<Vec<u8>, String> {
        self.identity.candidate_path(candidate)
    }
    /// Resolve only beneath the held root, retaining each symlink without reading source bytes.
    /// Mount crossings need an independently selected scope, even for a bind mount.
    pub fn resolve(&self, candidate: &[u8]) -> Result<SelectedSource, String> {
        let mut chain = probe::Chain::default();
        let node = self
            .walk(candidate, &mut chain)
            .map_err(|e| e.to_string())?;
        let meta = node.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > MAX_DOCUMENT_BYTES as u64 {
            return Err("Select a regular dependency no larger than 1 MiB".into());
        }
        Ok(SelectedSource {
            node,
            absolute: PathBuf::from(std::ffi::OsStr::from_bytes(&self.selected_path(candidate)?)),
            links: chain.links,
            held_links: chain.held_links,
        })
    }
    pub fn capture(&self, candidate: &[u8]) -> Result<CapturedSource, String> {
        self.resolve(candidate)?.capture()
    }
    fn relative<'a>(&self, candidate: &'a [u8]) -> Result<&'a [u8], String> {
        self.identity.relative(candidate)
    }
    // Absolute source paths keep their actual pathname semantics. A retained, renamed root
    // must not turn an absolute reference to its replacement into a read of the old folder.
    fn verify_absolute_alias(&self, candidate: &[u8]) -> Result<(), String> {
        if !candidate.starts_with(b"/") {
            return Ok(());
        }
        let tail = self.relative(candidate)?;
        let root_path = if [&self.identity.selected_path, &self.identity.resolved_path]
            .iter()
            .any(|root| candidate == root.strip_suffix(b"/").unwrap_or(root))
        {
            candidate
        } else {
            &candidate[..candidate.len() - tail.len()]
        };
        let current = Self::select(Path::new(std::ffi::OsStr::from_bytes(root_path)))?;
        if current.identity.device != self.identity.device
            || current.identity.inode != self.identity.inode
            || current.identity.mount_id != self.identity.mount_id
        {
            return Err(
                "Absolute dependency folder alias changed; select the current target separately"
                    .into(),
            );
        }
        Ok(())
    }
}
fn join_relative(parts: &[Vec<u8>], leaf: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        out.extend(part);
        out.push(b'/');
    }
    out.extend(leaf);
    out
}
fn join_absolute(root: &[u8], relative: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = root.to_vec();
    if !out.ends_with(b"/") {
        out.push(b'/');
    }
    out.extend(relative);
    if out.len() > 4096 {
        return Err("Dependency evidence path exceeds 4096 bytes".into());
    }
    Ok(out)
}
fn open_beneath(root: &File, path: &[u8]) -> Result<File, String> {
    probe::open_node(root, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::{
            ffi::OsStringExt,
            fs::{symlink, PermissionsExt},
        },
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "he-folder-core-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            Self(root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn contained_relative_absolute_and_alias_paths_preserve_bytes_and_link_evidence() {
        let f = Fixture::new();
        let root = f.0.join("approved");
        fs::create_dir_all(root.join("nested")).unwrap();
        let leaf = root.join(std::ffi::OsString::from_vec(b"theme-\xff.lua".to_vec()));
        fs::write(&leaf, "os.execute('never-run')\n").unwrap();
        symlink(leaf.file_name().unwrap(), root.join("relative.lua")).unwrap();
        symlink(&leaf, root.join("absolute.lua")).unwrap();
        symlink(&root, f.0.join("alias")).unwrap();
        let scope = Scope::select(&f.0.join("alias/.")).unwrap();
        assert_eq!(scope.identity().links.len(), 1);
        for path in [
            b"nested/../relative.lua".to_vec(),
            b"absolute.lua".to_vec(),
            leaf.as_os_str().as_bytes().to_vec(),
        ] {
            let captured = scope.capture(&path).unwrap();
            assert_eq!(captured.content, "os.execute('never-run')\n");
            assert_eq!(
                captured.identity.resolved_path.as_deref(),
                Some(leaf.as_os_str().as_bytes())
            );
            if !path.starts_with(b"/") {
                assert_eq!(captured.identity.links.len(), 1);
            }
        }
        assert!(Scope::select(&root.join("nested/../")).is_ok());
    }
    #[test]
    fn outside_prefix_symlink_escape_cycles_and_nonregular_leaves_are_not_captured() {
        let f = Fixture::new();
        let root = f.0.join("approved");
        fs::create_dir(&root).unwrap();
        fs::create_dir(f.0.join("approved-sibling")).unwrap();
        fs::write(f.0.join("outside.lua"), "outside").unwrap();
        fs::write(f.0.join("approved-sibling/file.lua"), "sibling").unwrap();
        symlink("../outside.lua", root.join("escape.lua")).unwrap();
        symlink(f.0.join("outside.lua"), root.join("absolute-escape.lua")).unwrap();
        symlink("cycle.lua", root.join("cycle.lua")).unwrap();
        let pipe = CString::new(root.join("fifo").as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(pipe.as_ptr(), 0o600) }, 0);
        let scope = Scope::select(&root).unwrap();
        for path in [
            b"../outside.lua".to_vec(),
            b"escape.lua".to_vec(),
            b"absolute-escape.lua".to_vec(),
            f.0.join("approved-sibling/file.lua")
                .as_os_str()
                .as_bytes()
                .to_vec(),
            b"cycle.lua".to_vec(),
            b"fifo".to_vec(),
            b"fifo/".to_vec(),
            b".".to_vec(),
            b"".to_vec(),
            b"null\0name".to_vec(),
        ] {
            assert!(scope.capture(&path).is_err(), "{path:?}");
        }
    }
    #[test]
    fn retained_folder_survives_rename_without_following_its_replacement() {
        let f = Fixture::new();
        let root = f.0.join("approved");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("config.lua"), "selected folder").unwrap();
        let scope = Scope::select(&root).unwrap();
        fs::rename(&root, f.0.join("moved")).unwrap();
        fs::create_dir(&root).unwrap();
        fs::write(root.join("config.lua"), "replacement folder").unwrap();
        let captured = scope.capture(b"config.lua").unwrap();
        assert_eq!(captured.content, "selected folder");
        assert!(scope
            .capture(root.join("config.lua").as_os_str().as_bytes())
            .unwrap_err()
            .contains("alias changed"));
        assert_eq!(
            captured.identity.resolved_path.as_deref(),
            Some(f.0.join("moved/config.lua").as_os_str().as_bytes())
        );
        fs::write(f.0.join("moved/added.lua"), "new child").unwrap();
        assert_eq!(scope.capture(b"added.lua").unwrap().content, "new child");
    }
    #[test]
    fn forged_identity_wrong_descriptor_and_changed_permissions_require_reapproval() {
        let f = Fixture::new();
        let scope = Scope::select(&f.0).unwrap();
        let mut forged = scope.identity().clone();
        forged.inode ^= 1;
        assert!(Scope::from_descriptor(scope.node().try_clone().unwrap(), forged).is_err());
        let mut forged = scope.identity().clone();
        forged.mount_id ^= 1;
        assert!(Scope::from_descriptor(scope.node().try_clone().unwrap(), forged).is_err());
        assert!(
            Scope::from_descriptor(File::open(&f.0).unwrap(), scope.identity().clone()).is_err()
        );
        fs::set_permissions(&f.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(scope.validate().unwrap_err().contains("changed"));
        fs::set_permissions(&f.0, fs::Permissions::from_mode(0o700)).unwrap();
        scope.validate().unwrap();
    }
    #[test]
    fn metadata_probe_reports_file_directory_fifo_and_byte_paths_without_read_access() {
        let f = Fixture::new();
        let name = std::ffi::OsString::from_vec(b"private-\xff.lua".to_vec());
        fs::write(f.0.join(&name), [0xff, 0, 0xfe]).unwrap();
        fs::set_permissions(f.0.join(&name), fs::Permissions::from_mode(0o0)).unwrap();
        fs::create_dir(f.0.join("directory")).unwrap();
        let pipe = CString::new(f.0.join("fifo").as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(pipe.as_ptr(), 0o600) }, 0);
        let scope = Scope::select(&f.0).unwrap();
        for (path, kind) in [
            (name.as_os_str().as_bytes(), libc::S_IFREG),
            (b"directory".as_slice(), libc::S_IFDIR),
            (b".".as_slice(), libc::S_IFDIR),
            (f.0.as_os_str().as_bytes(), libc::S_IFDIR),
            (b"fifo".as_slice(), libc::S_IFIFO),
        ] {
            let observed = scope.probe(path).unwrap();
            observed.validate().unwrap();
            let Status::Present { node } = observed.status else {
                panic!("Expected metadata")
            };
            assert_eq!(node.mode & libc::S_IFMT, kind);
        }
    }
    #[test]
    fn missing_and_not_directory_are_distinct_from_denials_escape_and_budget() {
        let f = Fixture::new();
        fs::write(f.0.join("file"), "inert").unwrap();
        fs::create_dir(f.0.join("locked")).unwrap();
        symlink("../outside-missing", f.0.join("escape")).unwrap();
        symlink("cycle", f.0.join("cycle")).unwrap();
        symlink("absent", f.0.join("broken")).unwrap();
        let scope = Scope::select(&f.0).unwrap();
        for (path, errno) in [
            (b"absent".as_slice(), libc::ENOENT),
            (b"absent/child", libc::ENOENT),
            (b"file/child", libc::ENOTDIR),
            (b"file/", libc::ENOTDIR),
            (b"broken", libc::ENOENT),
        ] {
            let observation = scope.probe(path).unwrap();
            assert!(matches!(observation.status,Status::Absent{errno:actual,..} if actual==errno));
            if path == b"broken" {
                assert_eq!(observation.links.len(), 1);
            }
        }
        for path in [b"../absent".as_slice(), b"escape"] {
            assert_eq!(scope.probe(path).unwrap_err().kind, FailureKind::NeedsGrant);
        }
        assert_eq!(scope.probe(b"cycle").unwrap_err().kind, FailureKind::Budget);
        assert_eq!(
            scope.probe(b"").unwrap_err().kind,
            FailureKind::InvalidInput
        );
        if unsafe { libc::geteuid() } != 0 {
            fs::set_permissions(f.0.join("locked"), fs::Permissions::from_mode(0o0)).unwrap();
            let denied = scope.probe(b"locked/absent");
            fs::set_permissions(f.0.join("locked"), fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(denied.unwrap_err().kind, FailureKind::PermissionDenied);
        }
    }
    #[test]
    fn intermediate_links_require_manual_evidence_and_implicit_following_is_refused() {
        let f = Fixture::new();
        fs::create_dir(f.0.join("dir")).unwrap();
        fs::write(f.0.join("dir/file"), "inert").unwrap();
        symlink("dir", f.0.join("alias")).unwrap();
        let scope = Scope::select(&f.0).unwrap();
        let error = probe::open_node(scope.node(), b"alias/file").unwrap_err();
        assert_eq!(error.errno, Some(libc::ELOOP));
        let observed = scope.probe(b"alias/file").unwrap();
        assert_eq!(observed.links.len(), 1);
        assert!(matches!(observed.status,Status::Present{node} if node.is_regular()));
    }
    #[test]
    fn changed_link_or_scope_invalidates_a_pending_negative_observation() {
        let f = Fixture::new();
        symlink("absent", f.0.join("link")).unwrap();
        let scope = Scope::select(&f.0).unwrap();
        let mut chain = probe::Chain::default();
        let result = scope.walk(b"link", &mut chain);
        fs::remove_file(f.0.join("link")).unwrap();
        symlink("other", f.0.join("link")).unwrap();
        assert_eq!(
            scope.finish_probe(b"link", chain, result).unwrap_err().kind,
            FailureKind::LinkChanged
        );
        let mut chain = probe::Chain::default();
        let result = scope.walk(b"absent", &mut chain);
        fs::set_permissions(&f.0, fs::Permissions::from_mode(0o755)).unwrap();
        let invalidated = scope.finish_probe(b"absent", chain, result);
        fs::set_permissions(&f.0, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(invalidated.unwrap_err().kind, FailureKind::AuthorityChanged);
    }
    #[test]
    fn renamed_absolute_alias_is_not_a_missing_file_and_records_grant_identity_only() {
        let f = Fixture::new();
        fs::create_dir(f.0.join("root")).unwrap();
        let scope = Scope::select(&f.0.join("root")).unwrap();
        fs::rename(f.0.join("root"), f.0.join("moved")).unwrap();
        fs::create_dir(f.0.join("root")).unwrap();
        assert_eq!(
            scope
                .probe(f.0.join("root/absent").as_os_str().as_bytes())
                .unwrap_err()
                .kind,
            FailureKind::AuthorityChanged
        );
        assert!(matches!(
            scope.probe(b"absent").unwrap().status,
            Status::Absent { .. }
        ));
        let mut record = scope.probe(b"absent").unwrap();
        record.status = Status::Absent {
            errno: libc::EACCES,
            at: b"absent".to_vec(),
        };
        assert!(record.validate().is_err());
    }
}
