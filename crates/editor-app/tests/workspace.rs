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
    let mut child = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
        .arg("--check-workspace")
        .arg(path)
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
