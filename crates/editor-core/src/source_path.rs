//! Descriptor-based resolution for a source explicitly selected by the user.
//! This reader records symlinks; it grants no write authority and never reads saved path hints.
use crate::{
    source::{CapturedSource, LinkIdentity, RecordedOwnership, SourceIdentity},
    storage, MAX_DOCUMENT_BYTES,
};
use std::{
    collections::VecDeque,
    ffi::CString,
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::Path,
};

fn error() -> String {
    std::io::Error::last_os_error().to_string()
}
fn open_node(parent: i32, name: &[u8], directory: bool) -> Result<File, String> {
    let name = CString::new(name).map_err(|_| "Invalid source path")?;
    // O_PATH does not open a device for I/O; no-follow lets us inspect links first.
    let fd = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_PATH
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | if directory { libc::O_DIRECTORY } else { 0 },
        )
    };
    if fd < 0 {
        return Err(error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn link_bytes(file: &File) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0; 4097];
    // The empty name reads the held link itself, avoiding substitution at its path.
    let n = unsafe {
        libc::readlinkat(
            file.as_raw_fd(),
            c"".as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
        )
    };
    if n < 0 {
        return Err(error());
    }
    if n == 0 || n > 4096 {
        return Err("Symlink target exceeds the source path limit".into());
    }
    bytes.truncate(n as usize);
    Ok(bytes)
}
fn components(bytes: &[u8]) -> VecDeque<Vec<u8>> {
    let mut parts: VecDeque<_> = bytes
        .split(|b| *b == b'/')
        .filter(|p| !p.is_empty())
        .map(Vec::from)
        .collect();
    // A trailing slash requires a directory. Do not normalize file/ into a file import.
    if bytes.last() == Some(&b'/') {
        parts.push_back(b".".to_vec());
    }
    parts
}
fn path_bytes(parts: &[Vec<u8>], leaf: Option<&[u8]>) -> Vec<u8> {
    let mut out = vec![b'/'];
    for part in parts {
        if out.len() > 1 {
            out.push(b'/');
        }
        out.extend(part);
    }
    if let Some(leaf) = leaf {
        if out.len() > 1 {
            out.push(b'/');
        }
        out.extend(leaf);
    }
    out
}
fn is_proc_link(file: &File) -> Result<bool, String> {
    let mut fs = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::fstatfs(file.as_raw_fd(), fs.as_mut_ptr()) } < 0 {
        return Err(error());
    }
    Ok(unsafe { fs.assume_init() }.f_type as u64 == 0x9fa0)
}

/// Only an explicit picker/CLI path should call this. Recorded workspace paths are never inputs.
pub fn read_selected(path: &Path) -> Result<CapturedSource, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let selected = absolute.as_os_str().as_bytes();
    if selected.len() > 4096 || selected.contains(&0) {
        return Err("Invalid source path".into());
    }
    let root = open_node(libc::AT_FDCWD, b"/", true)?;
    let mut directory = root.try_clone().map_err(|e| e.to_string())?;
    let mut pending = components(selected);
    let mut resolved = vec![];
    let mut links = vec![];
    let mut held_links = vec![];
    let mut traversed = 0;
    while let Some(component) = pending.pop_front() {
        traversed += 1;
        if traversed > 4096 {
            return Err("Source path resolution exceeded its step budget".into());
        }
        if component == b"." {
            if pending.is_empty() {
                return Err("Select a source file, not a directory".into());
            }
            continue;
        }
        if component == b".." {
            directory = open_node(directory.as_raw_fd(), b"..", true)?;
            resolved.pop();
            continue;
        }
        let node = open_node(directory.as_raw_fd(), &component, false)?;
        let metadata = node.metadata().map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            if links.len() >= 32 {
                return Err("Source symlink cycle or depth limit exceeded".into());
            }
            if is_proc_link(&node)? {
                return Err("Procfs magic links need a dedicated descriptor import; this source was not opened".into());
            }
            let target = link_bytes(&node)?;
            let encountered = path_bytes(&resolved, Some(&component));
            if encountered.len() > 4096 {
                return Err("Resolved source path exceeds its limit".into());
            }
            links.push(LinkIdentity {
                encountered_path: encountered,
                target: target.clone(),
                device: metadata.dev(),
                inode: metadata.ino(),
                owner: metadata.uid(),
                mode: metadata.mode(),
                changed_seconds: metadata.ctime(),
                changed_nanoseconds: metadata.ctime_nsec(),
            });
            let mut next = components(&target);
            if target.first() == Some(&b'/') {
                directory = root.try_clone().map_err(|e| e.to_string())?;
                resolved.clear();
            }
            next.append(&mut pending);
            pending = next;
            held_links.push(node);
            continue;
        }
        if !pending.is_empty() {
            if !metadata.is_dir() {
                return Err("A source path component is not a directory".into());
            }
            directory = node;
            resolved.push(component);
            continue;
        }
        if !metadata.is_file() || metadata.len() > MAX_DOCUMENT_BYTES as u64 {
            return Err("Select a regular source no larger than 1 MiB".into());
        }
        let (content, after) = storage::read_regular_node(&node, MAX_DOCUMENT_BYTES)?;
        // This name refers only to our own held inode, never a saved path hint.
        let own_descriptor = CString::new(format!("/proc/self/fd/{}", node.as_raw_fd())).unwrap();
        for (record, link) in links.iter().zip(&held_links) {
            let now = link.metadata().map_err(|e| e.to_string())?;
            if now.nlink() == 0
                || now.dev() != record.device
                || now.ino() != record.inode
                || now.uid() != record.owner
                || now.mode() != record.mode
                || now.ctime() != record.changed_seconds
                || now.ctime_nsec() != record.changed_nanoseconds
            {
                return Err(
                    "A selected source link changed during capture; retry the stable chain".into(),
                );
            }
        }
        let mut identity = SourceIdentity::captured(&absolute, &after, &content)?;
        // The held inode's kernel path is more accurate than a lexical hint if parents moved.
        let mut resolved_path = vec![0; 4097];
        let n = unsafe {
            libc::readlink(
                own_descriptor.as_ptr(),
                resolved_path.as_mut_ptr().cast(),
                resolved_path.len(),
            )
        };
        if n <= 0 || n > 4096 {
            return Err("Cannot record the selected source's bounded resolved path".into());
        }
        resolved_path.truncate(n as usize);
        if resolved_path.starts_with(b"/nix/store/") {
            identity.recorded_ownership = RecordedOwnership::StorePath;
        } else if !links.is_empty() {
            identity.recorded_ownership = RecordedOwnership::SymlinkManaged;
        } else if after.uid() != unsafe { libc::geteuid() } {
            identity.recorded_ownership = RecordedOwnership::OtherOwner;
        } else if after.mode() & 0o200 == 0 {
            identity.recorded_ownership = RecordedOwnership::ReadOnly;
        } else {
            identity.recorded_ownership = RecordedOwnership::UserOwned;
        }
        identity.resolved_path = Some(resolved_path);
        identity.links = links;
        identity.validate(&content)?;
        return Ok(CapturedSource { content, identity });
    }
    Err("Select a source file, not a directory".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::{ffi::OsStringExt, fs::symlink},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn folder() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "he-links-{}-{}",
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
    fn relative_absolute_and_parent_links_preserve_targets_and_chain() {
        let root = folder();
        fs::create_dir(root.join("actual")).unwrap();
        fs::write(root.join("actual/config.lua"), "-- source\n").unwrap();
        symlink("actual", root.join("directory")).unwrap();
        symlink("directory/config.lua", root.join("entry.lua")).unwrap();
        let source = read_selected(&root.join("entry.lua")).unwrap();
        assert_eq!(source.content, "-- source\n");
        assert_eq!(source.identity.links.len(), 2);
        assert_eq!(
            source.identity.recorded_ownership,
            RecordedOwnership::SymlinkManaged
        );
        assert_eq!(
            source.identity.resolved_path.as_deref(),
            Some(root.join("actual/config.lua").as_os_str().as_bytes())
        );
        symlink(root.join("actual/config.lua"), root.join("absolute.lua")).unwrap();
        assert_eq!(
            read_selected(&root.join("absolute.lua")).unwrap().content,
            source.content
        );
        symlink("../actual/config.lua", root.join("actual/parent.lua")).unwrap();
        assert_eq!(
            read_selected(&root.join("actual/parent.lua"))
                .unwrap()
                .content,
            source.content
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn cycles_broken_links_directories_and_linked_fifo_fail_without_reading() {
        let root = folder();
        symlink("b", root.join("a")).unwrap();
        symlink("a", root.join("b")).unwrap();
        assert!(read_selected(&root.join("a"))
            .unwrap_err()
            .contains("depth"));
        symlink("absent", root.join("missing")).unwrap();
        assert!(read_selected(&root.join("missing")).is_err());
        assert!(read_selected(&root).is_err());
        let pipe = root.join("pipe");
        let name = CString::new(pipe.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        symlink("pipe", root.join("pipe.lua")).unwrap();
        assert!(read_selected(&root.join("pipe.lua")).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn non_utf8_targets_survive_without_lossy_path_resolution() {
        let root = folder();
        let target = std::ffi::OsString::from_vec(b"target-\xff.lua".to_vec());
        fs::write(root.join(&target), "-- bytes\n").unwrap();
        symlink(&target, root.join("entry.lua")).unwrap();
        let source = read_selected(&root.join("entry.lua")).unwrap();
        assert_eq!(source.identity.links[0].target, target.as_bytes());
        assert_eq!(source.content, "-- bytes\n");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn proc_magic_links_are_not_followed_as_normal_source_links() {
        assert!(read_selected(Path::new("/proc/self/exe"))
            .unwrap_err()
            .contains("magic"));
    }
    #[test]
    fn trailing_slash_and_dot_do_not_turn_non_directory_paths_into_sources() {
        let root = folder();
        fs::write(root.join("file.lua"), "-- data\n").unwrap();
        assert!(read_selected(&root.join("file.lua/")).is_err());
        assert!(read_selected(&root.join("file.lua/.")).is_err());
        symlink("file.lua/", root.join("invalid.lua")).unwrap();
        assert!(read_selected(&root.join("invalid.lua")).is_err());
        assert!(read_selected(&root.join("./file.lua")).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn identical_bytes_through_a_replaced_link_change_recorded_identity() {
        let root = folder();
        fs::write(root.join("first.lua"), "-- same bytes\n").unwrap();
        fs::write(root.join("second.lua"), "-- same bytes\n").unwrap();
        let entry = root.join("entry.lua");
        symlink("first.lua", &entry).unwrap();
        let first = read_selected(&entry).unwrap();
        fs::remove_file(&entry).unwrap();
        symlink("second.lua", &entry).unwrap();
        let second = read_selected(&entry).unwrap();
        assert_eq!(first.content, second.content);
        assert_eq!(
            first.identity.compare_selected(&second).unwrap(),
            crate::source::Reattachment::RelocatedRevision
        );
        assert_ne!(
            first.identity.links[0].target,
            second.identity.links[0].target
        );
        fs::remove_dir_all(root).unwrap();
    }
}
