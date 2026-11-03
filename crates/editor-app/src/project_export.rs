//! Save a reviewed bundle into a new directory, using held directory descriptors.
use hyprland_editor_core::{lua, lua::replay::project_copy::Prepared, MAX_DOCUMENT_BYTES};
use std::{
    collections::BTreeSet,
    ffi::CString,
    fs::File,
    io::Write,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    },
    path::{Component, Path, PathBuf},
};

pub(super) fn layout(copy: &Prepared) -> Result<Vec<PathBuf>, String> {
    if copy.files.is_empty()
        || copy.files.len() > 32
        || !copy.files.iter().any(|f| f.document == copy.root)
    {
        return Err("The reviewed copy has an invalid file list".into());
    }
    let mut paths = vec![];
    let mut total = 0usize;
    let mut ids = BTreeSet::new();
    for file in &copy.files {
        total = total.saturating_add(file.content.len());
        if file.content.len() > MAX_DOCUMENT_BYTES
            || total > 4 * MAX_DOCUMENT_BYTES
            || file.content.as_bytes().contains(&0)
            || lua::source_hash(&file.content) != file.sha256
            || !ids.insert(&file.document)
            || file.load_name.len() > 4096
            || file.load_name.contains(&0)
        {
            return Err("The reviewed copy failed its file, size or hash checks".into());
        }
        let path = Path::new(std::ffi::OsStr::from_bytes(&file.load_name));
        if !path.is_absolute()
            || path.file_name().is_none()
            || path
                .components()
                .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
        {
            return Err("A captured filename cannot be represented in the export folder. Export its file separately.".into());
        }
        paths.push(path.to_path_buf());
    }
    let mut common = paths[0]
        .parent()
        .ok_or("Missing source parent")?
        .to_path_buf();
    for path in &paths {
        while !path.starts_with(&common) {
            if !common.pop() {
                return Err("Source files have no shared directory layout".into());
            }
        }
    }
    let mut names = BTreeSet::new();
    let mut relative = vec![];
    for path in paths {
        let path = path
            .strip_prefix(&common)
            .map_err(|_| "Invalid relative source name")?
            .to_path_buf();
        if path.as_os_str().is_empty()
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || !names.insert(path.clone())
        {
            return Err("Two source views map to the same export filename".into());
        }
        relative.push(path);
    }
    if relative
        .iter()
        .any(|a| relative.iter().any(|b| a != b && b.starts_with(a)))
    {
        return Err("A source filename conflicts with another file's directory".into());
    }
    Ok(relative)
}
fn c_name(name: &std::ffi::OsStr) -> Result<CString, String> {
    CString::new(name.as_bytes()).map_err(|_| "Invalid export path component".into())
}
fn error() -> String {
    std::io::Error::last_os_error().to_string()
}
fn open_at(parent: &File, name: &CString, flags: i32) -> Result<File, String> {
    // SAFETY: held directory fd and a checked, NUL-terminated single component.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if fd < 0 {
        return Err(error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn mkdir_at(parent: &File, name: &CString) -> Result<(), String> {
    // EEXIST is an error: this operation creates only new nodes.
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        return Err(error());
    }
    Ok(())
}

pub(super) fn save(parent: &Path, folder: &str, copy: &Prepared) -> Result<PathBuf, String> {
    let paths = layout(copy)?;
    if folder.is_empty()
        || folder.len() > 128
        || folder == "."
        || folder == ".."
        || folder
            .bytes()
            .any(|b| b == b'/' || b == 0 || b.is_ascii_control())
    {
        return Err("Use a new folder name without slashes or control characters".into());
    }
    let parent_name = c_name(parent.as_os_str())?;
    // The parent is selected by the user; recorded source paths never reach open().
    let fd = unsafe {
        libc::open(
            parent_name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(format!("Cannot open the chosen parent folder: {}", error()));
    }
    let parent_fd = unsafe { File::from_raw_fd(fd) };
    let name = c_name(std::ffi::OsStr::new(folder))?;
    mkdir_at(&parent_fd, &name)
        .map_err(|e| format!("Cannot create a new folder: {e}. Choose an unused name."))?;
    let destination = parent.join(folder);
    let result = (|| {
        let root = open_at(&parent_fd, &name, libc::O_RDONLY | libc::O_DIRECTORY)?;
        let mut created = BTreeSet::new();
        for (source, path) in copy.files.iter().zip(paths) {
            let components = path.components().map(|c| c.as_os_str()).collect::<Vec<_>>();
            let mut dir = root.try_clone().map_err(|e| e.to_string())?;
            let mut walked = PathBuf::new();
            for component in &components[..components.len() - 1] {
                walked.push(component);
                let name = c_name(component)?;
                if created.insert(walked.clone()) {
                    mkdir_at(&dir, &name)?;
                }
                dir = open_at(&dir, &name, libc::O_RDONLY | libc::O_DIRECTORY)?;
            }
            let name = c_name(components.last().unwrap())?;
            let mut output = open_at(&dir, &name, libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL)?;
            output
                .write_all(source.content.as_bytes())
                .map_err(|e| e.to_string())?;
            output.sync_all().map_err(|e| e.to_string())?;
            dir.sync_all().map_err(|e| e.to_string())?;
        }
        root.sync_all().map_err(|e| e.to_string())?;
        parent_fd.sync_all().map_err(|e| e.to_string())?;
        Ok::<_, String>(destination.clone())
    })();
    result.map_err(|e| format!("Could not finish exporting: {e}. A partial copy may remain in {}. Choose a new folder name to retry.", destination.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyprland_editor_core::lua::replay::project_copy::{File as SourceFile, Mode};
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
    };
    fn copy() -> Prepared {
        Prepared {
            root: "root".into(),
            modes: [Mode::Keep; 9],
            files: [
                ("root", "/config/main.lua", "require('parts.appearance')"),
                (
                    "part",
                    "/config/parts/appearance.lua",
                    "-- café\r\nhl.config{}\r\n",
                ),
            ]
            .into_iter()
            .map(|(id, name, source)| SourceFile {
                document: id.into(),
                load_name: name.as_bytes().into(),
                content: source.into(),
                sha256: lua::source_hash(source),
                changed: false,
            })
            .collect(),
            removed_sites: 0,
            inserted_sites: 0,
            findings: vec![],
            before_complete: true,
            after_complete: true,
        }
    }
    #[test]
    fn folder_copy_preserves_tree_bytes_and_refuses_existing_targets() {
        let temp = super::super::test_support::TempDir::new("project-export-tree");
        let copy = copy();
        let result = save(temp.path(), "reviewed", &copy).unwrap();
        for (source, path) in copy.files.iter().zip(layout(&copy).unwrap()) {
            assert_eq!(
                fs::read(result.join(&path)).unwrap(),
                source.content.as_bytes()
            );
            assert_eq!(
                fs::metadata(result.join(path))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
        }
        assert_eq!(
            fs::metadata(&result).unwrap().permissions().mode() & 0o077,
            0
        );
        assert!(save(temp.path(), "reviewed", &copy).is_err());
        symlink(&result, temp.path().join("link")).unwrap();
        assert!(save(temp.path(), "link", &copy).is_err());
        for name in ["", ".", "..", "../escape", "line\nbreak"] {
            assert!(save(temp.path(), name, &copy).is_err());
        }
    }
    #[test]
    fn malformed_bundle_cannot_publish_any_files() {
        let temp = super::super::test_support::TempDir::new("project-export-invalid");
        for variant in 0..5 {
            let mut copy = copy();
            match variant {
                0 => copy.files[1].sha256 = "0".repeat(64),
                1 => copy.files[1].load_name = b"/config/../escape.lua".into(),
                2 => copy.files[1].load_name = copy.files[0].load_name.clone(),
                3 => copy.files[1].load_name = b"/config/main.lua/child.lua".into(),
                _ => copy.files[1].load_name = b"relative.lua".into(),
            }
            assert!(save(temp.path(), "refused", &copy).is_err());
            assert!(!temp.path().join("refused").exists());
        }
    }
}
