//! Held folder authority. Serialized identity/path evidence alone cannot construct a scope.
//! Application callers use this inside the capture process, never in a parser worker.
use crate::{
    source::{CapturedSource, LinkIdentity},
    source_path::{self, SelectedSource},
    MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
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
        self.validate()?;
        if candidate.is_empty() || candidate.len() > 4096 || candidate.contains(&0) {
            return Err("Invalid dependency path".into());
        }
        let relative = self.relative(candidate)?;
        self.verify_absolute_alias(candidate)?;
        let mut pending = source_path::components(relative);
        let mut resolved: Vec<Vec<u8>> = Vec::new();
        let mut links = Vec::new();
        let mut held_links = Vec::new();
        let mut steps = 0;
        while let Some(part) = pending.pop_front() {
            steps += 1;
            if steps > 4096 {
                return Err("Dependency path resolution exceeded its step budget".into());
            }
            if part == b"." {
                continue;
            }
            if part == b".." {
                if resolved.pop().is_none() {
                    return Err(
                        "Dependency escapes its approved folder; select its target separately"
                            .into(),
                    );
                }
                continue;
            }
            let joined = join_relative(&resolved, &part);
            let node = open_beneath(&self.node, &joined)?;
            let meta = node.metadata().map_err(|e| e.to_string())?;
            if meta.file_type().is_symlink() {
                if links.len() >= 32 {
                    return Err("Dependency symlink cycle or depth limit exceeded".into());
                }
                if source_path::is_proc_link(&node)? {
                    return Err("Dependency magic links are not authorized".into());
                }
                let target = source_path::link_bytes(&node)?;
                links.push(LinkIdentity {
                    encountered_path: join_absolute(&self.identity.resolved_path, &joined)?,
                    target: target.clone(),
                    device: meta.dev(),
                    inode: meta.ino(),
                    owner: meta.uid(),
                    mode: meta.mode(),
                    changed_seconds: meta.ctime(),
                    changed_nanoseconds: meta.ctime_nsec(),
                });
                let next = if target.starts_with(b"/") {
                    self.verify_absolute_alias(&target)?;
                    resolved.clear();
                    source_path::components(self.relative(&target)?)
                } else {
                    source_path::components(&target)
                };
                let mut next = next;
                next.append(&mut pending);
                pending = next;
                held_links.push(node);
                continue;
            }
            if !pending.is_empty() {
                if !meta.is_dir() {
                    return Err("A dependency path component is not a directory".into());
                }
                resolved.push(part);
                continue;
            }
            if !meta.is_file() || meta.len() > MAX_DOCUMENT_BYTES as u64 {
                return Err("Select a regular dependency no larger than 1 MiB".into());
            }
            let absolute = join_absolute(&self.identity.selected_path, relative)?;
            return Ok(SelectedSource {
                node,
                absolute: PathBuf::from(std::ffi::OsStr::from_bytes(&absolute)),
                links,
                held_links,
            });
        }
        Err("Select a dependency file, not a directory".into())
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
        let prefix_len = candidate.len() - tail.len();
        let root_path = &candidate[..prefix_len];
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
    #[repr(C)]
    struct OpenHow {
        flags: u64,
        mode: u64,
        resolve: u64,
    }
    let path = CString::new(path).map_err(|_| "Invalid dependency path")?;
    let how = OpenHow {
        flags: (libc::O_PATH | libc::O_CLOEXEC | libc::O_NOFOLLOW) as u64,
        mode: 0,
        resolve: 0x08 | 0x02 | 0x01,
    }; // BENEATH | NO_MAGICLINKS | NO_XDEV
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            root.as_raw_fd(),
            path.as_ptr(),
            &how,
            std::mem::size_of::<OpenHow>(),
        )
    };
    if fd < 0 {
        let e = std::io::Error::last_os_error();
        return Err(match e.raw_os_error() {
            Some(libc::EXDEV) => {
                "Dependency crosses its approved folder or mount; select its target separately"
                    .into()
            }
            Some(libc::ENOSYS | libc::EINVAL) => {
                "Dependency capture requires openat2 containment support".into()
            }
            _ => e.to_string(),
        });
    }
    Ok(unsafe { File::from_raw_fd(fd as i32) })
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
}
