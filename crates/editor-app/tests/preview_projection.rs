//! Exercise the real restricted staging child through the normal app binary.
use hyprland_editor_core::{
    action, binding, catalogue, device, monitor, storage, workspace::Workspace, Draft, Value,
    AUDITED_MAIN, BASELINE,
};
use hyprland_editor_projection::{
    wire::{Artifact, Expected},
    Scenario,
};
use std::{
    fs,
    io::Write,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        for _ in 0..64 {
            let path = std::env::temp_dir().join(format!(
                "hye-projection-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("{e}"),
            }
        }
        panic!("No private fixture directory");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn actual_staging_role_denies_file_network_process_execution_and_closes_inherited_fds() {
    use std::os::{fd::AsRawFd, unix::process::CommandExt};
    let root = Fixture::new();
    let path = root.0.join("capability");
    fs::write(&path, b"private").unwrap();
    let file = fs::File::open(&path).unwrap();
    let descriptor = file.as_raw_fd();
    let mut command = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"));
    command.arg("--projection-policy-probe").env_clear();
    // The child deliberately receives one fixture capability without CLOEXEC.
    // Its own role must close it before reading any request or probe operation.
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(descriptor, 60) != 60 || libc::fcntl(60, libc::F_SETFD, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let output = command.output().unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    for key in [
        "file_denied",
        "network_denied",
        "process_denied",
        "execution_denied",
        "inherited_fd_closed",
    ] {
        assert_eq!(result[key], true, "{key}: {result}");
    }
    assert_eq!(fs::read(path).unwrap(), b"private");
}
#[test]
fn actual_staging_preserves_inputs_maps_private_hardware_and_keeps_commands_inert() {
    for version in [BASELINE, AUDITED_MAIN] {
        let root = Fixture::new();
        let effect = root.0.join("must-not-exist");
        let source = root.0.join("original.lua");
        fs::write(
            &source,
            format!(
                "os.execute('touch {}') -- HYE_OPAQUE_SOURCE",
                effect.display()
            ),
        )
        .unwrap();
        let captured = storage::read_source(&source).unwrap();
        fs::remove_file(&source).unwrap();
        let mut action = action::Action::new(action::find("exec_cmd", version).unwrap());
        action.fields.insert(
            "argument".into(),
            Value::String(format!("touch {} # HYE_COMMAND", effect.display())),
        );
        let mut workspace = Workspace::new(Draft::new(version))
            .with_root(captured.content, captured.identity)
            .with_monitors(vec![monitor::Rule::new("monitor")])
            .with_devices(vec![device::Spec::new("device", "HYE_HOST_DEVICE")])
            .with_bindings(binding::Draft {
                binds: vec![binding::Bind {
                    id: "command".into(),
                    chord: binding::Chord::parse("SUPER+Q"),
                    actions: vec![action],
                    flags: Default::default(),
                    description: None,
                    description_alias: false,
                    device: None,
                    submap: None,
                }],
                submaps: vec![],
                order: vec!["command".into()],
            });
        workspace
            .draft
            .values
            .insert("general:border_size".into(), Value::Int(3));
        let mut scenario = Scenario::default();
        scenario
            .monitor_targets
            .insert("monitor".into(), "HEADLESS-1".into());
        scenario
            .device_targets
            .insert("device".into(), "hye-lab-pointer".into());
        let original = workspace.to_json(&catalogue()).unwrap();
        let input = root.0.join("workspace.hye");
        let layout = root.0.join("scenario.json");
        fs::write(&input, &original).unwrap();
        fs::write(&layout, serde_json::to_vec(&scenario).unwrap()).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
            .arg("--check-preview-projection")
            .arg(&input)
            .arg(&layout)
            .env_clear()
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        for key in [
            "runtime_verified",
            "compositor_started",
            "saved_file_written",
        ] {
            assert_eq!(response[key], false);
        }
        let artifact: Artifact = serde_json::from_value(response["artifact"].clone()).unwrap();
        artifact
            .verify(&Expected::new(&workspace, &scenario).unwrap())
            .unwrap();
        assert!(artifact.lua().contains("HEADLESS-1"));
        assert!(artifact.lua().contains("hye-lab-pointer"));
        assert!(artifact.lua().contains("no_op"));
        for marker in [
            "HYE_OPAQUE_SOURCE",
            "HYE_COMMAND",
            "HYE_HOST_DEVICE",
            effect.to_str().unwrap(),
        ] {
            assert!(!String::from_utf8_lossy(&output.stdout).contains(marker));
        }
        assert!(!effect.exists());
        assert!(!source.exists());
        assert_eq!(fs::read(&input).unwrap(), original);
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 2);
    }
}
#[test]
fn staging_refuses_unknown_scenarios_and_nonregular_selected_files_without_writes() {
    let root = Fixture::new();
    let workspace = root.0.join("workspace.hye");
    let scenario = root.0.join("scenario.json");
    fs::write(
        &workspace,
        Workspace::new(Draft::new(BASELINE))
            .to_json(&catalogue())
            .unwrap(),
    )
    .unwrap();
    for input in [
        r#"{"outputs":[],"monitor_targets":{},"device_targets":{},"hidden":true}"#,
        r#"{"outputs":[],"monitor_targets":{},"device_targets":{}}"#,
    ] {
        fs::write(&scenario, input).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
            .arg("--check-preview-projection")
            .arg(&workspace)
            .arg(&scenario)
            .env_clear()
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read_to_string(&scenario).unwrap(), input);
    }
    fs::remove_file(&scenario).unwrap();
    std::os::unix::fs::symlink(&workspace, &scenario).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
        .arg("--check-preview-projection")
        .arg(&workspace)
        .arg(&scenario)
        .env_clear()
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
#[test]
fn direct_staging_role_refuses_unknown_protocol_payload_and_request_overflow() {
    for bytes in [
        b"{\"hidden\":true}".to_vec(),
        vec![b' '; 10 * 1024 * 1024 + 1],
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
            .arg("--projection-worker")
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&bytes).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(response["Error"].is_string());
        assert!(response.get("Staged").is_none());
    }
}
