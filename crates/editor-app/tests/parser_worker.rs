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
fn empty_statements_are_inert_in_the_actual_restricted_parser() {
    let source =
        ";local n=2;;hl.config{general={border_size=n}};;hl.config{decoration={rounding=n+1}};";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], false);
    assert_eq!(parsed["Parsed"]["diagnostics"], json!([]));
    assert_eq!(
        parsed["Parsed"]["draft"]["values"]["general:border_size"]["value"],
        2
    );
    assert_eq!(
        parsed["Parsed"]["draft"]["values"]["decoration:rounding"]["value"],
        3
    );
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
fn calls_in_ignored_fields_are_inert_and_invalidate_later_api_trust() {
    let marker =
        std::env::temp_dir().join(format!("he-ignored-must-not-create-{}", std::process::id()));
    let source=format!("hl.config{{unknown=os.execute('touch {}'),general={{border_size=2}}}}\nhl.config{{general={{border_size=7}}}}",marker.display());
    let result = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(result["Parsed"]["opaque_effects"], true);
    assert_eq!(
        result["Parsed"]["draft"]["values"]["general:border_size"]["value"],
        2
    );
    assert!(!marker.exists());
}
#[test]
fn invalid_and_oversized_source_fail_without_partial_import() {
    let result = request(json!({"Parse":{"source":"hl.config({","version":"0.56.2"}}));
    assert!(result["Error"].as_str().unwrap().contains("syntax"));
    let result = request(json!({"Parse":{"source":"x".repeat(1024*1024+1),"version":"0.56.2"}}));
    assert!(result["Error"].as_str().unwrap().contains("1 MiB"));
}

#[test]
fn actual_restricted_worker_projects_monitors_and_keeps_unknown_calls_inert() {
    let source="hl.monitor{output='DP-1',mode='2560x1440@144',scale='1.25',position='-2048x0',reserved={top=-5}};hl.config{general={border_size=2}}";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["monitors"][0]["output"]["name"], "DP-1");
    assert_eq!(
        parsed["Parsed"]["monitors"][0]["fields"]["reserved"]["value"],
        json!([-5, 0, 0, 0])
    );
    assert_eq!(parsed["Parsed"]["opaque_effects"], false);
    let marker = std::env::temp_dir().join(format!("monitor-inert-{}", std::process::id()));
    let source = format!(
        "hl.monitor{{output='DP-1',mode=os.execute('touch {}')}};hl.monitor{{output='DP-2'}}",
        marker.display()
    );
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], true);
    assert_eq!(parsed["Parsed"]["monitors"], json!([]));
    assert!(!marker.exists());
}
