//! Linux descriptor-relative storage; no config paths are writable through this API.
use crate::{Draft, Setting, MAX_DOCUMENT_BYTES};
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
    let p = c_path(path)?;
    // NONBLOCK prevents a malicious FIFO/device import from hanging the UI.
    let fd = unsafe {
        libc::open(
            p.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(io_error());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > MAX_DOCUMENT_BYTES as u64 {
        return Err("Select a regular file smaller than 1 MiB".into());
    }
    let mut bytes = vec![];
    file.take(MAX_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err("File grew beyond 1 MiB while reading".into());
    }
    String::from_utf8(bytes).map_err(|_| "Configuration must be UTF-8".into())
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
    let filename = format!("draft-{stamp}-{}.json", std::process::id());
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
        file.write_all(&json).map_err(|e| e.to_string())?;
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
        dir.sync_all().map_err(|e| e.to_string())?;
        Ok(state_root.join("hyprland-editor").join(filename))
    })();
    unsafe { libc::unlinkat(dir.as_raw_fd(), temp.as_ptr(), 0) };
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
