//! Real helper transport/containment through the public diagnostic path, private files only.
use std::{
    fs,
    os::unix::{
        ffi::OsStrExt,
        fs::{symlink, PermissionsExt},
        process::CommandExt,
    },
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "he-folder-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.join("approved")).unwrap();
        Self(root)
    }
    fn command(&self, candidate: &std::ffi::OsStr) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"));
        command
            .arg("--check-source-folder")
            .arg(self.0.join("approved"))
            .arg(candidate)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn run(mut command: Command) -> std::process::Output {
    use std::io::Read;
    let mut child = command.spawn().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(7) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Private folder capture exceeded its deadline");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    std::process::Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}
#[test]
fn actual_folder_capture_keeps_source_inert_and_returns_hash_and_byte_safe_path_only() {
    let f = Fixture::new();
    let marker = f.0.join("never-executed");
    let source = format!(
        "os.execute('touch {}')\nhl.config{{general={{border_size=2}}}}",
        marker.display()
    );
    let name = std::ffi::OsStr::from_bytes(b"theme-\xff.lua");
    fs::write(f.0.join("approved").join(name), &source).unwrap();
    symlink(name, f.0.join("approved/relative.lua")).unwrap();
    symlink(
        f.0.join("approved").join(name),
        f.0.join("approved/absolute.lua"),
    )
    .unwrap();
    for candidate in [
        name,
        std::ffi::OsStr::new("relative.lua"),
        std::ffi::OsStr::new("absolute.lua"),
    ] {
        let result = run(f.command(candidate));
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(
            value["source_identity"]["sha256"],
            hyprland_editor_core::lua::source_hash(&source)
        );
        assert_eq!(value["files_written"], false);
        assert_eq!(value["lua_executed"], false);
        assert_eq!(value["runtime_verified"], false);
        assert!(value.get("content").is_none());
    }
    assert!(!marker.exists());
}
#[test]
fn actual_helper_refuses_escapes_prefix_confusion_cycles_and_fifo_without_reading_them() {
    let f = Fixture::new();
    fs::write(f.0.join("outside.lua"), "private outside").unwrap();
    fs::create_dir(f.0.join("approved-sibling")).unwrap();
    fs::write(f.0.join("approved-sibling/outside.lua"), "private sibling").unwrap();
    symlink("../outside.lua", f.0.join("approved/escape.lua")).unwrap();
    symlink(
        f.0.join("outside.lua"),
        f.0.join("approved/absolute-escape.lua"),
    )
    .unwrap();
    symlink("loop.lua", f.0.join("approved/loop.lua")).unwrap();
    let pipe = std::ffi::CString::new(f.0.join("approved/fifo").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(pipe.as_ptr(), 0o600) }, 0);
    for candidate in [
        PathBuf::from("../outside.lua"),
        PathBuf::from("escape.lua"),
        PathBuf::from("absolute-escape.lua"),
        f.0.join("approved-sibling/outside.lua"),
        PathBuf::from("loop.lua"),
        PathBuf::from("fifo"),
    ] {
        let result = run(f.command(candidate.as_os_str()));
        assert!(!result.status.success(), "{candidate:?}");
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
}
#[test]
fn unavailable_containment_or_landlock_refuses_folder_capture_without_fallback() {
    let f = Fixture::new();
    fs::write(f.0.join("approved/config.lua"), "-- source\n").unwrap();
    for syscall in [libc::SYS_openat2, libc::SYS_landlock_create_ruleset] {
        let mut command = f.command(std::ffi::OsStr::new("config.lua"));
        // Only the fixture parent and its helpers inherit this denial, never the test runner.
        unsafe {
            command.pre_exec(move || {
                let mut filter = [
                    libc::sock_filter {
                        code: 0x20,
                        jt: 0,
                        jf: 0,
                        k: 0,
                    },
                    libc::sock_filter {
                        code: 0x15,
                        jt: 0,
                        jf: 1,
                        k: syscall as u32,
                    },
                    libc::sock_filter {
                        code: 0x06,
                        jt: 0,
                        jf: 0,
                        k: 0x00050000 | libc::ENOSYS as u32,
                    },
                    libc::sock_filter {
                        code: 0x06,
                        jt: 0,
                        jf: 0,
                        k: 0x7fff0000,
                    },
                ];
                let program = libc::sock_fprog {
                    len: filter.len() as u16,
                    filter: filter.as_mut_ptr(),
                };
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
                    || libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &program) != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let result = run(command);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(
            error.contains("openat2") || error.contains("Landlock"),
            "{error}"
        );
    }
}
