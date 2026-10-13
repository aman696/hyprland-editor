use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn request(request: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
        .arg("--lua-source-worker")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let data = serde_json::to_vec(&request).unwrap();
    let writer = std::thread::spawn(move || input.write_all(&data));
    let output = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut data = vec![];
        output
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .unwrap();
        data
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Parser worker failed to respect its deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let _ = writer.join().unwrap();
    let data = reader.join().unwrap();
    assert!(status.success());
    serde_json::from_slice(&data).unwrap()
}
#[test]
fn filesystem_and_network_are_denied() {
    let result = request(json!("Probe"));
    assert_eq!(result["Probe"]["file_denied"], true);
    assert_eq!(result["Probe"]["network_denied"], true);
}
#[test]
fn real_worker_imports_and_prepares_a_preserved_copy() {
    let source = "-- untouched\nhl.config({ ['general.border_size'] = 2 })\n";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    let mut report = parsed["Parsed"].clone();
    assert_eq!(report["draft"]["values"]["general:border_size"]["value"], 2);
    let mut draft = report["draft"].clone();
    draft["values"]["general:border_size"]["value"] = json!(5);
    let copy = request(json!({"Copy":{"source":source,"report":report.take(),"draft":draft}}));
    assert_eq!(copy["Copy"], source.replace("= 2", "= 5"));
}
#[test]
fn opaque_commands_cannot_create_files() {
    let marker = std::env::temp_dir().join(format!(
        "hyprland-editor-must-not-create-{}",
        std::process::id()
    ));
    let source = format!(
        "os.execute('touch {}')\nhl.config({{['general.border_size']=3}})",
        marker.display()
    );
    let result = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(result["Parsed"]["opaque_effects"], true);
    assert!(!marker.exists());
    assert_eq!(result["Parsed"]["draft"]["values"], json!({}));
}
#[test]
fn invalid_and_oversized_source_fail_without_partial_import() {
    let result = request(json!({"Parse":{"source":"hl.config({","version":"0.56.2"}}));
    assert!(result["Error"].as_str().unwrap().contains("syntax"));
    let result = request(json!({"Parse":{"source":"x".repeat(1024*1024+1),"version":"0.56.2"}}));
    assert!(result["Error"].as_str().unwrap().contains("1 MiB"));
}
