//! Private test fixtures survive PID reuse and clean up during panic unwinding.
use std::{
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub(super) struct TempDir(PathBuf);
impl TempDir {
    pub(super) fn new(label: &str) -> Self {
        assert!(label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-'));
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        for attempt in 0..32 {
            let path = std::env::temp_dir()
                .join(format!("{label}-{}-{nonce}-{attempt}", std::process::id()));
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("Cannot create private test fixture: {e}"),
            }
        }
        panic!("Private fixture name collision limit")
    }
    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
