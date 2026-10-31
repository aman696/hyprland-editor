use hyprland_editor_preview::cgroup::{Delegation, Limits};
use std::{
    fs,
    os::fd::AsFd,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "hye-resource-authority-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("{e}"),
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn plausible_control_files_cannot_turn_a_regular_directory_into_resource_authority() {
    let fixture = Fixture::new();
    for (name, value) in [
        ("cgroup.type", "domain\n"),
        ("cgroup.controllers", "cpu memory pids\n"),
        ("cgroup.subtree_control", "cpu memory pids\n"),
        ("cgroup.procs", ""),
        ("cgroup.kill", "canary"),
    ] {
        fs::write(fixture.0.join(name), value).unwrap();
    }
    let file = fs::File::open(&fixture.0).unwrap();
    let error = Delegation::from_directory(file.as_fd()).unwrap_err();
    assert!(error.to_string().contains("cgroup v2"));
    assert_eq!(
        fs::read_to_string(fixture.0.join("cgroup.kill")).unwrap(),
        "canary"
    );
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 5);
}
#[test]
fn file_and_relocated_link_evidence_cannot_replace_a_live_delegation_directory() {
    let fixture = Fixture::new();
    let path = fixture.0.join("evidence.json");
    fs::write(&path,r#"{"domain_type":"domain","available":["cpu","memory","pids"],"enabled_for_children":["cpu","memory","pids"],"live_limits_verified":true}"#).unwrap();
    std::os::unix::fs::symlink(&path, fixture.0.join("alias")).unwrap();
    for path in [path, fixture.0.join("alias")] {
        let file = fs::File::open(path).unwrap();
        assert!(Delegation::from_directory(file.as_fd())
            .unwrap_err()
            .to_string()
            .contains("directory descriptor"));
    }
}
#[test]
fn initial_limits_preserve_the_explicit_memory_task_and_one_core_budget() {
    let defaults = Limits::default();
    assert_eq!(defaults.memory_bytes, 512 * 1024 * 1024);
    assert_eq!(defaults.pids, 64);
    assert_eq!(defaults.cpu_quota_us, defaults.cpu_period_us);
    assert!(defaults.validate().is_ok());
    for limits in [
        Limits {
            memory_bytes: 0,
            ..defaults
        },
        Limits {
            memory_bytes: 512 * 1024 * 1024 + 1,
            ..defaults
        },
        Limits {
            pids: 0,
            ..defaults
        },
        Limits {
            pids: 257,
            ..defaults
        },
        Limits {
            cpu_quota_us: 200000,
            ..defaults
        },
        Limits {
            cpu_period_us: 0,
            ..defaults
        },
    ] {
        assert!(limits.validate().is_err(), "{limits:?}");
    }
}
