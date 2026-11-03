//! These fixtures invoke the real capture executable against private temporary files only.
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    os::unix::{ffi::OsStrExt, fs::symlink},
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
            "he-capture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn bytes(&self, name: &str) -> Vec<u8> {
        self.0.join(name).as_os_str().as_bytes().to_vec()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn request(value: Value) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"));
    command
        .arg("--capture-source-worker")
        .arg(std::process::id().to_string())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    request_with(command, value)
}
fn request_with(mut command: Command, value: Value) -> Value {
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&value).unwrap())
        .unwrap();
    let output = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        output
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .unwrap();
        data
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(4) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Private capture fixture exceeded its deadline");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(status.success());
    let bytes = reader.join().unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("Invalid capture response: {e}; {bytes:?}"))
}
fn read(path: Vec<u8>) -> Value {
    request(json!({"Read":{"path":path}}))
}
#[test]
#[cfg(debug_assertions)]
fn actual_policy_allows_selected_read_and_denies_other_read_write_network_and_process_creation() {
    let f = Fixture::new();
    std::fs::write(
        f.0.join("selected.lua"),
        "hl.config{general={border_size=2}}\n",
    )
    .unwrap();
    std::fs::write(f.0.join("other.lua"), "private sibling").unwrap();
    let response =
        request(json!({"Probe":{"path":f.bytes("selected.lua"),"other":f.bytes("other.lua")}}));
    assert_eq!(
        response,
        json!({"Probe":{"read_allowed":true,"other_denied":true,"write_denied":true,"network_denied":true,"process_denied":true}})
    );
    assert_eq!(
        std::fs::read_to_string(f.0.join("other.lua")).unwrap(),
        "private sibling"
    );
    assert_eq!(
        std::fs::read_to_string(f.0.join("selected.lua")).unwrap(),
        "hl.config{general={border_size=2}}\n"
    );
}
#[test]
fn capture_preserves_non_utf8_filename_symlink_evidence_and_keeps_lua_inert() {
    let f = Fixture::new();
    let leaf = f.0.join(std::ffi::OsStr::from_bytes(b"config-\xff.lua"));
    let marker = f.0.join("never-created");
    let content = format!(
        "os.execute('touch {}'); hl.config{{general={{border_size=2}}}}",
        marker.display()
    );
    std::fs::write(&leaf, &content).unwrap();
    symlink(leaf.file_name().unwrap(), f.0.join("selected.lua")).unwrap();
    let response = read(f.bytes("selected.lua"));
    let result = &response["Captured"];
    assert!(
        result["abi"].as_u64().is_some_and(|abi| abi >= 3),
        "{response}"
    );
    let captured: hyprland_editor_core::source::CapturedSource =
        serde_json::from_value(result["source"].clone()).unwrap();
    captured.identity.validate(&captured.content).unwrap();
    assert_eq!(captured.content, content);
    assert_eq!(captured.identity.selected_path, f.bytes("selected.lua"));
    assert_eq!(
        captured.identity.resolved_path.as_deref(),
        Some(leaf.as_os_str().as_bytes())
    );
    assert_eq!(captured.identity.links.len(), 1);
    assert_eq!(captured.identity.links[0].target, b"config-\xff.lua");
    assert!(!marker.exists());
    let direct = read(leaf.as_os_str().as_bytes().to_vec());
    assert_eq!(direct["Captured"]["source"]["content"], content);
}
#[test]
fn nonregular_magiclink_invalid_path_and_oversized_sources_are_rejected() {
    let f = Fixture::new();
    let fifo = std::ffi::CString::new(f.bytes("pipe")).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    for path in [
        f.bytes("pipe"),
        f.0.as_os_str().as_bytes().to_vec(),
        b"/proc/self/exe".to_vec(),
        b"relative.lua".to_vec(),
        b"/invalid\0name".to_vec(),
    ] {
        let response = read(path);
        assert!(response["Error"].is_string(), "{response}");
    }
    std::fs::write(f.0.join("large.lua"), vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(read(f.bytes("large.lua"))["Error"].is_string());
    std::fs::write(f.0.join("not-utf8.lua"), [255]).unwrap();
    assert!(read(f.bytes("not-utf8.lua"))["Error"]
        .as_str()
        .unwrap()
        .contains("UTF-8"));
}
#[test]
fn response_can_carry_the_worst_case_valid_source_escaping_within_its_bound() {
    let f = Fixture::new();
    let content = "\u{1}".repeat(1024 * 1024);
    std::fs::write(f.0.join("escaped.lua"), &content).unwrap();
    let response = read(f.bytes("escaped.lua"));
    assert_eq!(response["Captured"]["source"]["content"], content);
}
#[test]
fn wrong_parent_identity_is_rejected_before_source_resolution() {
    let output = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
        .arg("--capture-source-worker")
        .arg("1")
        .env_clear()
        .output()
        .unwrap();
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(response["Error"]
        .as_str()
        .unwrap()
        .contains("parent identity"));
}

#[test]
fn unavailable_landlock_refuses_capture_without_an_unrestricted_fallback() {
    use std::os::unix::process::CommandExt;
    let f = Fixture::new();
    std::fs::write(f.0.join("selected.lua"), "-- selected private fixture\n").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"));
    command
        .arg("--capture-source-worker")
        .arg(std::process::id().to_string())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // This inherited filter simulates an unavailable kernel API in this child only.
    // All other syscalls, including exec and the real worker's later policy setup, stay available.
    unsafe {
        command.pre_exec(|| {
            let mut filters = [
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
                    k: libc::SYS_landlock_create_ruleset as u32,
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
                len: filters.len() as u16,
                filter: filters.as_mut_ptr(),
            };
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
                || libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &program) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let response = request_with(command, json!({"Read":{"path":f.bytes("selected.lua")}}));
    assert!(
        response["Error"]
            .as_str()
            .is_some_and(|s| s.contains("Landlock")),
        "{response}"
    );
    assert!(response.get("Captured").is_none());
}
