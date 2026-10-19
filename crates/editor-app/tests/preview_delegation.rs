//! Read-only diagnostics must not block on user-named FIFOs or mint capabilities from JSON.
use std::{
    ffi::CString,
    fs,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "hye-preview-diagnostic-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn check(path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"));
    command
        .arg("--check-preview-delegation")
        .env("LC_ALL", "C")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = path {
        command.arg(path);
    }
    let mut child = command.spawn().unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if start.elapsed() > Duration::from_secs(3) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Preview directory diagnostics blocked on an invalid input")
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn diagnostic_rejects_blocking_non_directory_and_forged_control_inputs_without_writes() {
    let fixture = Fixture::new();
    let regular = fixture.0.join("record.json");
    fs::write(
        &regular,
        r#"{"live_limits_verified":true,"compositor_verified":true}"#,
    )
    .unwrap();
    let fifo = fixture.0.join("fifo");
    let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: one owned fixture path, no pointers retained, private FIFO only.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let directory = fixture.0.join("lookalike");
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("cgroup.type"), "domain\n").unwrap();
    fs::write(directory.join("cgroup.kill"), "untouched").unwrap();
    let alias = fixture.0.join("alias");
    std::os::unix::fs::symlink(&directory, &alias).unwrap();
    for path in [&regular, &fifo, &directory, &alias] {
        let output = check(Some(path));
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    assert!(!check(None).status.success());
    assert_eq!(
        fs::read_to_string(directory.join("cgroup.kill")).unwrap(),
        "untouched"
    );
    assert_eq!(
        fs::read_to_string(regular).unwrap(),
        r#"{"live_limits_verified":true,"compositor_verified":true}"#
    );
}
