//! Exercises reopening through the actual application and its restricted child parser.
use hyprland_editor_core::{
    catalogue, lua, storage,
    workspace::{self, Workspace},
    Draft, Value, BASELINE,
};
use std::{
    fs,
    io::Read,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn folder() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "he-reopen-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    root
}
fn run(path: &Path) -> (bool, String, String) {
    run_args(&["--check-workspace".into(), path.as_os_str().to_os_string()])
}
fn run_args(args: &[std::ffi::OsString]) -> (bool, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let output = std::thread::spawn(move || {
        let mut bytes = vec![];
        (&mut stdout).take(16385).read_to_end(&mut bytes).unwrap();
        bytes
    });
    let errors = std::thread::spawn(move || {
        let mut bytes = vec![];
        (&mut stderr).take(16385).read_to_end(&mut bytes).unwrap();
        bytes
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(6) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Saved workspace validation did not terminate");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    (
        status.success(),
        String::from_utf8(output.join().unwrap()).unwrap(),
        String::from_utf8(errors.join().unwrap()).unwrap(),
    )
}
fn fixture(root: &Path, source: &str) -> Workspace {
    let path = root.join("original.lua");
    fs::write(&path, source).unwrap();
    let captured = storage::read_source(&path).unwrap();
    let mut draft = Draft::new(BASELINE);
    draft
        .values
        .insert("general:border_size".into(), Value::Int(5));
    Workspace::new(draft).with_root(captured.content, captured.identity)
}
#[test]
fn application_reparses_snapshot_and_preserves_edits_without_opening_recorded_paths() {
    let root = folder();
    let source = "-- retain\nhl.config{general={border_size=2}}\n";
    let mut snapshot = fixture(&root, source);
    // An untrusted recorded path is evidence, not an input capability.
    let pipe = root.join("must-not-open");
    let c = std::ffi::CString::new(pipe.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    snapshot.sources[0].origin.selected_path = pipe.as_os_str().as_bytes().to_vec();
    let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
    fs::remove_file(root.join("original.lua")).unwrap();
    let (success, output, error) = run(&path);
    assert!(success, "{error}");
    let report: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(report["source_reparsed"], true);
    assert_eq!(report["draft_values"], 1);
    assert_eq!(report["original_paths_opened"], false);
    assert_eq!(report["runtime_verified"], false);
    assert_eq!(report["source_sha256"], lua::source_hash(source));
    let restored = workspace::read(&path, &catalogue()).unwrap();
    assert_eq!(restored.draft.values["general:border_size"], Value::Int(5));
    // The draft remains edited; reopening did not replace it with source value 2.
    assert_eq!(
        fs::read(&path).unwrap(),
        snapshot.to_json(&catalogue()).unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn saved_opaque_code_remains_inert_and_explicitly_unverified() {
    let root = folder();
    let marker = root.join("must-not-create");
    let source = format!(
        "os.execute('touch {}')\nhl.config{{general={{border_size=2}}}}",
        marker.display()
    );
    let snapshot = fixture(&root, &source);
    let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
    let (success, output, error) = run(&path);
    assert!(success, "{error}");
    let report: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(report["opaque_effects"], true);
    assert!(report["source_diagnostics"].as_u64().unwrap() > 0);
    assert!(!marker.exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn application_rejects_bad_hash_forged_reports_and_syntax_in_saved_source() {
    let root = folder();
    let snapshot = fixture(&root, "hl.config{general={border_size=2}}");
    let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
    let original = fs::read_to_string(&path).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&original).unwrap();
    value["sources"][0]["content"] = serde_json::json!("-- hash changed");
    fs::write(&path, value.to_string()).unwrap();
    let (success, _, error) = run(&path);
    assert!(!success);
    assert!(error.contains("hash"));
    value = serde_json::from_str(&original).unwrap();
    value["sources"][0]["parser_report"] = serde_json::json!({"opaque_effects":false});
    fs::write(&path, value.to_string()).unwrap();
    assert!(!run(&path).0);
    let malformed = fixture(&root, "hl.config({");
    let malformed = workspace::save(&root, &malformed, &catalogue()).unwrap();
    let (success, _, error) = run(&malformed);
    assert!(!success);
    assert!(error.contains("syntax"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn application_compares_changed_and_opaque_sources_with_restricted_parser_and_no_writes() {
    let root = folder();
    let old = "hl.config{general={border_size=2},decoration={rounding=3}}";
    let mut snapshot = fixture(&root, old);
    snapshot
        .draft
        .values
        .insert("decoration:rounding".into(), Value::Int(3));
    // A recorded path is never used as the comparison input.
    snapshot.sources[0].origin.selected_path = b"/definitely-not-an-input.lua".to_vec();
    let saved = workspace::save(&root, &snapshot, &catalogue()).unwrap();
    fs::remove_file(root.join("original.lua")).unwrap();
    let selected = root.join("changed.lua");
    let marker = root.join("must-not-create");
    let run_comparison = || {
        let (success, output, error) = run_args(&[
            "--check-source-comparison".into(),
            saved.as_os_str().to_os_string(),
            selected.as_os_str().to_os_string(),
        ]);
        assert!(success, "{error}");
        serde_json::from_str::<serde_json::Value>(&output).unwrap()
    };
    let changed = "hl.config{general={border_size=7},decoration={rounding=4}}";
    fs::write(&selected, changed).unwrap();
    let report = run_comparison();
    assert_eq!(report["partial_projection"], false);
    let rows = report["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|r| r["setting"] == "general:border_size" && r["requires_choice"] == true));
    assert!(rows
        .iter()
        .any(|r| r["setting"] == "decoration:rounding" && r["automatic"] == "use_selected_file"));
    let opaque = format!(
        "os.execute('touch {}'); hl.config{{general={{border_size=7}}}}",
        marker.display()
    );
    fs::write(&selected, &opaque).unwrap();
    let report = run_comparison();
    assert_eq!(report["partial_projection"], true);
    assert!(!marker.exists());
    assert!(report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["requires_choice"] == true && r["selected_value_available"] == false));
    assert_eq!(report["original_paths_opened"], false);
    assert_eq!(report["files_written"], false);
    assert_eq!(report["runtime_verified"], false);
    assert_eq!(fs::read_to_string(&selected).unwrap(), opaque);
    assert_eq!(
        fs::read(&saved).unwrap(),
        snapshot.to_json(&catalogue()).unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn monitor_drafts_survive_actual_application_reopen_without_replacing_them_with_source() {
    use hyprland_editor_core::monitor::{Output, Rule};
    let root = folder();
    let source = "hl.monitor{output='DP-1',mode='1920x1080@60'}";
    let mut rule = Rule::new("edited-monitor");
    rule.output = Output::Connector("DP-1".into());
    rule.fields.insert(
        "mode".into(),
        hyprland_editor_core::Value::String("2560x1440@144".into()),
    );
    let snapshot = fixture(&root, source).with_monitors(vec![rule.clone()]);
    let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
    fs::remove_file(root.join("original.lua")).unwrap();
    let (success, output, error) = run(&path);
    assert!(success, "{error}");
    let report: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(report["monitor_rules"], 1);
    assert_eq!(report["source_reparsed"], true);
    assert_eq!(report["runtime_verified"], false);
    assert_eq!(
        workspace::read(&path, &catalogue()).unwrap().monitors,
        vec![rule]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn device_drafts_survive_application_reopen_and_schema_drift_is_rejected() {
    use hyprland_editor_core::device::Spec;
    let root = folder();
    let source = "hl.device{name='keyboard',kb_layout='us'}";
    let mut spec = Spec::new("edited", "keyboard");
    spec.fields
        .insert("kb_layout".into(), Value::String("de".into()));
    let snapshot = fixture(&root, source).with_devices(vec![spec.clone()]);
    let path = workspace::save(&root, &snapshot, &catalogue()).unwrap();
    fs::remove_file(root.join("original.lua")).unwrap();
    let (success, output, error) = run(&path);
    assert!(success, "{error}");
    let result: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(result["device_rules"], 1);
    assert_eq!(result["source_reparsed"], true);
    assert_eq!(
        workspace::read(&path, &catalogue()).unwrap().devices,
        vec![spec]
    );
    let mut forged: serde_json::Value =
        serde_json::from_slice(&snapshot.to_json(&catalogue()).unwrap()).unwrap();
    forged["device_schema_sha256"] = serde_json::json!("0".repeat(64));
    fs::write(&path, forged.to_string()).unwrap();
    assert!(!run(&path).0);
    fs::remove_dir_all(root).unwrap();
}
