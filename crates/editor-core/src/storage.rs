//! Linux descriptor-relative storage; no config paths are writable through this API.
use crate::{source::SourceIdentity, Draft, Setting, MAX_DOCUMENT_BYTES};
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn c_path(path: &Path) -> Result<CString, String> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| "Invalid path".into())
}
fn io_error() -> String {
    std::io::Error::last_os_error().to_string()
}
fn open_dir(path: &Path) -> Result<File, String> {
    let path = c_path(path)?;
    // SAFETY: NUL-terminated path; returned descriptor is exclusively owned by File.
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(io_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
pub fn read_document(path: &Path) -> Result<String, String> {
    read_source(path).map(|source| source.content)
}
/// Capture the identity and bytes from the same descriptor, not a separate path stat.
pub fn read_source(path: &Path) -> Result<crate::source::CapturedSource, String> {
    let (content, meta) = read_bounded(path, MAX_DOCUMENT_BYTES)?;
    let identity = SourceIdentity::captured(path, &meta, &content)?;
    Ok(crate::source::CapturedSource { content, identity })
}
pub(crate) fn read_bounded(
    path: &Path,
    limit: usize,
) -> Result<(String, std::fs::Metadata), String> {
    let p = c_path(path)?;
    // Inspect the node without opening a FIFO/device for I/O.
    let fd = unsafe {
        libc::open(
            p.as_ptr(),
            libc::O_PATH | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(io_error());
    }
    let node = unsafe { File::from_raw_fd(fd) };
    read_regular_node(&node, limit)
}
pub(crate) fn read_regular_node(
    node: &File,
    limit: usize,
) -> Result<(String, std::fs::Metadata), String> {
    let meta = node.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return Err(format!(
            "Select a regular file no larger than {limit} bytes"
        ));
    }
    // Only reopen our held regular inode; do not resolve the caller's path again.
    let descriptor = CString::new(format!("/proc/self/fd/{}", node.as_raw_fd())).unwrap();
    let fd = unsafe {
        libc::open(
            descriptor.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(io_error());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if opened.dev() != meta.dev() || opened.ino() != meta.ino() {
        return Err("File identity changed during descriptor reopen".into());
    }
    read_opened(file, limit)
}
fn read_opened(file: File, limit: usize) -> Result<(String, std::fs::Metadata), String> {
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return Err(format!(
            "Select a regular file no larger than {limit} bytes"
        ));
    }
    let mut bytes = vec![];
    (&file)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("File grew beyond the document limit while reading".into());
    }
    let after = file.metadata().map_err(|e| e.to_string())?;
    if after.nlink() == 0
        || meta.len() != after.len()
        || meta.mtime() != after.mtime()
        || meta.mtime_nsec() != after.mtime_nsec()
        || meta.ctime() != after.ctime()
        || meta.ctime_nsec() != after.ctime_nsec()
        || bytes.len() as u64 != after.len()
    {
        return Err("File changed while reading; retry from a stable revision".into());
    }
    let content =
        String::from_utf8(bytes).map_err(|_| "Configuration must be UTF-8".to_string())?;
    Ok((content, after))
}
pub fn save_draft(
    state_root: &Path,
    draft: &Draft,
    catalogue: &[Setting],
) -> Result<PathBuf, String> {
    draft.validate(catalogue)?;
    let json = serde_json::to_vec_pretty(draft).map_err(|e| e.to_string())?;
    if json.len() > MAX_DOCUMENT_BYTES {
        return Err("Draft exceeds 1 MiB".into());
    }
    save_private(state_root, &json, "draft", "json")
}
/// A fixed app directory and immutable, generated names; no caller-supplied target path.
pub(crate) fn save_private(
    state_root: &Path,
    bytes: &[u8],
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    if !state_root.is_absolute()
        || !["draft", "workspace"].contains(&prefix)
        || !["json", "hye"].contains(&extension)
    {
        return Err("Invalid private storage request".into());
    }
    let base = open_dir(state_root)?;
    let meta = base.metadata().map_err(|e| e.to_string())?;
    if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o022 != 0 {
        return Err("State root must be owned by you and not writable by other users".into());
    }
    let name = CString::new("hyprland-editor").unwrap();
    let ret = unsafe { libc::mkdirat(base.as_raw_fd(), name.as_ptr(), 0o700) };
    if ret < 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
        return Err(io_error());
    }
    let fd = unsafe {
        libc::openat(
            base.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io_error());
    }
    let dir = unsafe { File::from_raw_fd(fd) };
    let meta = dir.metadata().map_err(|e| e.to_string())?;
    if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
        return Err("Draft directory must be owned by you and private (0700)".into());
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let filename = format!("{prefix}-{stamp}-{}.{}", std::process::id(), extension);
    let final_name = CString::new(filename.clone()).unwrap();
    let temp = CString::new(format!(".{filename}.tmp")).unwrap();
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            temp.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io_error());
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let result = (|| {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        // A link publishes atomically and fails if the destination already exists.
        if unsafe {
            libc::linkat(
                dir.as_raw_fd(),
                temp.as_ptr(),
                dir.as_raw_fd(),
                final_name.as_ptr(),
                0,
            )
        } < 0
        {
            return Err(io_error());
        }
        // Remove the temporary name before syncing the directory, including its unlink.
        if unsafe { libc::unlinkat(dir.as_raw_fd(), temp.as_ptr(), 0) } < 0 {
            return Err(io_error());
        }
        dir.sync_all().map_err(|e| e.to_string())?;
        // A first save must also persist the app directory's entry in its parent.
        if ret == 0 {
            base.sync_all().map_err(|e| e.to_string())?;
        }
        Ok(state_root.join("hyprland-editor").join(filename))
    })();
    if result.is_err() {
        unsafe { libc::unlinkat(dir.as_raw_fd(), temp.as_ptr(), 0) };
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "he-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn storage_is_private_and_round_trips() {
        let p = root();
        let path = save_draft(&p, &Draft::new(crate::BASELINE), &crate::catalogue()).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(Draft::from_json(&read_document(&path).unwrap(), &crate::catalogue()).is_ok());
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn rejects_symlinked_directory_and_file() {
        let p = root();
        symlink(&p, p.join("hyprland-editor")).unwrap();
        assert!(save_draft(&p, &Draft::new(crate::BASELINE), &crate::catalogue()).is_err());
        let f = p.join("link");
        symlink("/etc/passwd", &f).unwrap();
        assert!(read_document(&f).is_err());
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn rejects_world_readable_draft_directory() {
        let p = root();
        std::fs::create_dir(p.join("hyprland-editor")).unwrap();
        std::fs::set_permissions(
            p.join("hyprland-editor"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(save_draft(&p, &Draft::new(crate::BASELINE), &crate::catalogue()).is_err());
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn fifo_is_rejected_without_blocking() {
        let p = root();
        let f = p.join("pipe");
        let c = c_path(&f).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
        assert!(read_document(&f).is_err());
        std::fs::remove_dir_all(p).unwrap();
    }
}
