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

#[test]
fn actual_worker_projects_devices_and_rejects_external_keyboard_entities() {
    let source="hl.device{name='USB keyboard',kb_layout='us,de',kb_variant='dvorak,',sensitivity=0.5};hl.monitor{output='DP-1'}";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["devices"][0]["name"], "USB keyboard");
    assert_eq!(
        parsed["Parsed"]["devices"][0]["fields"]["kb_variant"]["value"],
        "dvorak,"
    );
    assert_eq!(parsed["Parsed"]["opaque_effects"], false);
    let registry =
        request(json!({"ParseRegistry":{"xml":include_str!("fixtures/xkb-registry.xml")}}));
    assert_eq!(
        registry["Registry"]["layouts"][0]["variants"][0]["id"],
        "dvorak"
    );
    let xml="<!DOCTYPE xkbConfigRegistry [<!ENTITY leak SYSTEM 'file:///etc/passwd'>]><xkbConfigRegistry>&leak;</xkbConfigRegistry>";
    let result = request(json!({"ParseRegistry":{"xml":xml}}));
    assert!(result["Error"].is_string());
    let oversized = request(json!({"ParseRegistry":{"xml":" ".repeat(1024*1024+1)}}));
    assert!(oversized["Error"].is_string());
}
#[test]
fn installed_keyboard_registry_is_parsed_by_the_restricted_worker_when_available() {
    let path = std::path::Path::new("/usr/share/X11/xkb/rules/evdev.xml");
    if !path.is_file() {
        return;
    }
    let xml = hyprland_editor_core::storage::read_source(path)
        .unwrap()
        .content;
    let result = request(json!({"ParseRegistry":{"xml":xml}}));
    assert!(
        result["Registry"]["layouts"]
            .as_array()
            .is_some_and(|v| !v.is_empty()),
        "{result}"
    );
    assert!(result["Registry"]["groups"]
        .as_array()
        .is_some_and(|v| !v.is_empty()));
}

#[test]
fn binding_actions_remain_inert_in_actual_restricted_worker_and_preserve_timing() {
    let marker = std::env::temp_dir().join(format!("he-bind-never-execute-{}", std::process::id()));
    let source=format!("local action=hl.dsp.exec_cmd('touch {}');hl.define_submap('resize',function()hl.bind('R',function()hl.dispatch(action);hl.dispatch(hl.dsp.exit())end,{{release=true}})end)",marker.display());
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], false, "{parsed}");
    assert_eq!(
        parsed["Parsed"]["bindings"]["binds"][0]["actions"][0]["at_load"],
        true
    );
    assert_eq!(
        parsed["Parsed"]["bindings"]["binds"][0]["actions"][1]["at_load"],
        false
    );
    assert!(!marker.exists());
    let source=format!("hl.bind('Q',hl.dsp.no_op(),{{unknown=os.execute('touch {}')}});hl.bind('W',hl.dsp.no_op())",marker.display());
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], true);
    assert_eq!(parsed["Parsed"]["bindings"]["binds"], json!([]));
    assert!(!marker.exists());
}

#[test]
fn actual_restricted_worker_projects_workspace_rules_without_running_empty_hooks() {
    let marker =
        std::env::temp_dir().join(format!("he-workspace-never-run-{}", std::process::id()));
    let source=format!("hl.workspace_rule{{workspace='name:work',enabled=false,layout='master',layout_opts={{orientation='center',custom=false}},on_created_empty='touch {}'}}",marker.display());
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], false, "{parsed}");
    assert_eq!(parsed["Parsed"]["workspace_rules"][0]["enabled"], false);
    assert_eq!(
        parsed["Parsed"]["workspace_rules"][0]["layout_options"]["custom"]["value"],
        false
    );
    assert!(!marker.exists());
}

#[test]
fn bounded_re2_samples_run_inside_the_restricted_worker() {
    for (pattern, text, negated, expected) in [
        ("fire.*", "firefox", false, true),
        ("fire", "firefox", false, false),
        ("fire", "firefox", true, true),
        ("δοκιμή", "δοκιμή", false, true),
    ] {
        let result =
            request(json!({"CheckRegex":{"pattern":pattern,"text":text,"negated":negated}}));
        assert_eq!(result["RegexMatch"], expected, "{result}");
    }
    for pattern in ["[", "(?=x)", "(x)\\1"] {
        assert!(
            request(json!({"CheckRegex":{"pattern":pattern,"text":"x","negated":false}}))["Error"]
                .is_string()
        );
    }
    assert!(
        request(json!({"CheckRegex":{"pattern":"x".repeat(16385),"text":"","negated":false}}))
            ["Error"]
            .is_string()
    );
}

#[test]
fn actual_restricted_worker_projects_inert_rule_groups_and_preserves_copy() {
    let source="hl.window_rule{name='test',enabled=false,match={class='negative:foo.*'},move={'monitor_w - window_w',20}};hl.window_rule{name='test',center=true};hl.layer_rule{match={namespace='panel'},blur=false};hl.config{general={border_size=2}}";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    let report = &parsed["Parsed"];
    assert_eq!(report["opaque_effects"], false);
    assert_eq!(report["rules"].as_array().unwrap().len(), 3);
    assert_eq!(report["rules"][0]["enabled"], false);
    assert_eq!(report["rules"][1]["enabled"], Value::Null);
    let mut draft = report["draft"].clone();
    draft["values"]["general:border_size"]["value"] = json!(7);
    let copy = request(json!({"Copy":{"source":source,"report":report,"draft":draft}}));
    assert_eq!(
        copy["Copy"],
        source.replace("border_size=2", "border_size=7")
    );
    let tainted = request(
        json!({"Parse":{"source":"hl.window_rule{move={os.execute('never execute'),2}};hl.layer_rule{blur=true}","version":"0.56.2"}}),
    );
    assert_eq!(tainted["Parsed"]["opaque_effects"], true);
    assert_eq!(tainted["Parsed"]["rules"], json!([]));
}

#[test]
fn module_candidate_plans_use_only_supplied_bytes_and_reject_amplification() {
    let input = json!({"PlanModules":{
        "context":{"main_file":b"/synthetic/config/main.lua","home":null,"inherited_package_path":null},
        "modules":[b"./theme".to_vec(),b"named.theme".to_vec()],"version":"0.56.2"
    }});
    let response = request(input.clone());
    let plans = response["ModulePlans"].as_array().unwrap();
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0]["primary"]["kind"], "explicit_files");
    assert_eq!(
        plans[0]["primary"]["base"],
        json!(b"/synthetic/config/theme")
    );
    assert_eq!(plans[1]["inherited_search_unknown"], true);
    let mut denied = input;
    denied["PlanModules"]["context"]["inherited_package_path"] = json!(b"/x/?.lua;".repeat(500));
    denied["PlanModules"]["modules"] = json!(vec![vec![b'a'; 4096]]);
    assert!(request(denied)["Error"].as_str().unwrap().contains("1 MiB"));
}
