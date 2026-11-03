use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{fs::OpenOptionsExt, process::CommandExt},
    },
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
const EXE: &str = env!("CARGO_BIN_EXE_hyprland-editor-source55");
struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn command() -> (Command, File) {
    // Keep one exact Cargo-built inode; no request/saved path selects this role.
    let held = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(EXE)
        .unwrap();
    let mut cmd = Command::new(format!("/proc/self/fd/{}", held.as_raw_fd()));
    cmd.arg("--source-worker")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    (cmd, held)
}
fn finish(child: &mut Child, bound: Duration) -> std::process::ExitStatus {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(
            start.elapsed() < bound,
            "Compiler child did not meet its bound"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn bytes(bytes: Vec<u8>) -> Value {
    let (mut cmd, held) = command();
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    let mut input = child.0.stdin.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(&bytes));
    let output = child.0.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        output
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        bytes
    });
    let status = finish(&mut child.0, Duration::from_secs(6));
    let _ = writer.join().unwrap();
    let bytes = reader.join().unwrap();
    assert!(status.success(), "Compiler failed: {status}");
    assert!(bytes.len() <= 4 * 1024 * 1024);
    serde_json::from_slice(&bytes).unwrap()
}
fn release() -> String {
    hyprland_editor_core::lua::replay::compiler_profile().release
}
fn request(operation: Value) -> Value {
    let value = json!({"protocol":"hye-lua55-source-1","nonce":vec![7u8;32],"expected_release":release(),"operation":operation});
    let wire = serde_json::to_vec(&value).unwrap();
    let digest = format!("{:x}", Sha256::digest(&wire));
    let response = bytes(wire);
    assert_eq!(response["protocol"], "hye-lua55-source-1");
    if !response["nonce"].is_null() {
        assert_eq!(response["nonce"], json!(vec![7u8; 32]));
    }
    assert_eq!(response["request_sha256"], digest);
    if !response["profile"].is_null() {
        assert_eq!(response["profile"]["release"], release());
    }
    response
}
fn profile() -> Value {
    request(json!({"kind":"profile"}))["profile"].clone()
}
fn program(source: &str, version: &str) -> Value {
    json!({"version":version,"profile":"hyprland_lua55","root":"root","documents":[{"id":"root","source":source,"sha256":hyprland_editor_core::lua::source_hash(source),"load_name":list(b"/fixture.lua")}],"bindings":[],"initial_package_path":null,"initial_package_path_unknown_tail":false,"working_directory":null})
}
fn list(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}
fn analyze(p: Value) -> Value {
    request(json!({"kind":"analyze","compiler":profile(),"program":p}))["outcome"].clone()
}
#[test]
fn restricted_source_image_models_both_snapshot_config_calls_and_returns() {
    for version in [
        hyprland_editor_core::BASELINE,
        hyprland_editor_core::AUDITED_MAIN,
    ] {
        for code in [
            "local c={general={border_size=3}};hl.config(c);return 7",
            "local global=function(x)hl.config{general={border_size=x}}end;global(3);return 7",
        ] {
            let outcome = analyze(program(code, version));
            assert_eq!(outcome["kind"], "analyzed", "{outcome}");
            let report = &outcome["report"];
            assert!(report["halt"].is_null(), "{report}");
            assert_eq!(report["profile"], "hyprland_lua55");
            assert_eq!(report["returns"], json!([{"type":"int","value":7}]));
            assert_eq!(report["events"].as_array().unwrap().len(), 1);
            assert_eq!(report["compiler_profile"], profile());
            assert_eq!(report["runtime_verified"], false);
        }
    }
}
#[test]
fn source_worker_requires_exact_compiler_observation_and_explicit_family() {
    let p = program("return 1", hyprland_editor_core::BASELINE);
    for field in ["release", "schema_sha256", "library_identity"] {
        let mut compiler = profile();
        compiler[field] = "forged".into();
        assert_eq!(
            request(json!({"kind":"analyze","compiler":compiler,"program":p}))["outcome"]["kind"],
            "error"
        );
    }
    let mut compiler = profile();
    compiler["numeric_codegen"]["probe_sha256"] = "0".repeat(64).into();
    assert_eq!(
        request(json!({"kind":"analyze","compiler":compiler,"program":p}))["outcome"]["kind"],
        "error"
    );
    for family in ["hyprland_lua54", "synthetic_emit"] {
        let mut other = p.clone();
        other["profile"] = family.into();
        assert_eq!(analyze(other)["kind"], "error");
    }
    let mut other: hyprland_editor_core::lua::replay::Program = serde_json::from_value(p).unwrap();
    other.profile = hyprland_editor_core::lua::replay::Profile::HyprlandLua54;
    assert!(hyprland_editor_core::lua::replay::replay(&other).is_err());
}
#[test]
fn nil_error_is_converted_after_error_function_location_handling() {
    for code in [
        "return pcall(error,nil)",
        "return pcall(function() error(nil) end)",
    ] {
        let outcome = analyze(program(code, hyprland_editor_core::BASELINE));
        assert!(outcome["report"]["halt"].is_null(), "{outcome}");
        assert_eq!(
            outcome["report"]["returns"],
            json!([{"type":"bool","value":false},{"type":"bytes","value":list(b"<no error object>")}])
        );
    }
    let outcome = analyze(program(
        "return pcall(error,false)",
        hyprland_editor_core::BASELINE,
    ));
    assert_eq!(
        outcome["report"]["returns"],
        json!([{"type":"bool","value":false},{"type":"bool","value":false}])
    );
}
#[test]
fn native55_declarations_preserve_scopes_and_unmodeled_initializers_stay_unresolved() {
    for code in [
        "for i=1,2 do i=3 end",
        "for k,v in pairs({x=1}) do k=2 end",
        "global x; y=1",
    ] {
        let out = analyze(program(code, hyprland_editor_core::BASELINE));
        assert_eq!(out["report"]["halt"]["kind"], "lua_error", "{out}");
        assert!(out["report"]["events"].as_array().unwrap().is_empty());
    }
    for code in ["x=1;global x=2;return x", "global<const> x=7;return x"] {
        let out = analyze(program(code, hyprland_editor_core::BASELINE));
        assert_eq!(out["report"]["halt"]["kind"], "unresolved", "{out}");
        assert!(out["report"]["events"].as_array().unwrap().is_empty());
    }
    for (code, values) in [
        ("global x; x=3;return x", vec![3]),
        ("local global=1;return global,2,global", vec![1,2,1]),
        ("local global=function(x)return x+1 end;return global(3)", vec![4]),
        ("local global=2;local t={global,3};local function f(a,b)return a,b end;return f(global,t[2])", vec![2,3]),
        ("local x=9;do global x;x=3 end;return x,_G.x", vec![9, 3]),
        (
            "local x=1;local function f()return x end;global x,_G;x=3;return x,f(),_G.x",
            vec![3, 1, 3],
        ),
        ("global<const> x;_ENV.x=9;return x", vec![9]),
        ("local x=2;global *;return x", vec![2]),
    ] {
        let out = analyze(program(code, hyprland_editor_core::BASELINE));
        assert!(out["report"]["halt"].is_null(), "{code}: {out}");
        assert_eq!(
            out["report"]["returns"],
            json!(values
                .into_iter()
                .map(|value| json!({"type":"int","value":value}))
                .collect::<Vec<_>>())
        );
    }
    let out = analyze(program(
        "local global=1;for i=1,2 do local i=i+3;return global+i end",
        hyprland_editor_core::BASELINE,
    ));
    assert!(out["report"]["halt"].is_null(), "{out}");
    assert_eq!(out["report"]["returns"], json!([{"type":"int","value":5}]));
}
#[test]
fn private_module_requests_replay_with_exact_bound_documents() {
    let mut p = program(
        "local p=require('part');return p+2",
        hyprland_editor_core::BASELINE,
    );
    let out = analyze(p.clone());
    let request = out["report"]["halt"]["request"].clone();
    assert_eq!(out["report"]["halt"]["kind"], "needs_resolution", "{out}");
    p["documents"].as_array_mut().unwrap().push(json!({"id":"part","source":"return 3","sha256":hyprland_editor_core::lua::source_hash("return 3"),"load_name":list(b"/part.lua")}));
    p["bindings"] = json!([{"request":request,"resolution":{"kind":"loaded","document":"part","loader_data":list(b"/part.lua")}}]);
    let out = analyze(p.clone());
    assert!(out["report"]["halt"].is_null(), "{out}");
    assert_eq!(out["report"]["returns"], json!([{"type":"int","value":5}]));
    p["documents"][1]["source"] = "return 9".into();
    assert_eq!(analyze(p)["kind"], "error");
}
#[test]
fn strict_protocol_refuses_unknown_operations_fields_and_profiles() {
    for op in [
        json!({"kind":"profile","hidden":true}),
        json!({"kind":"probe","hidden":true}),
        json!({"kind":"execute","source":[]}),
        json!({"kind":"analyze","compiler":profile(),"program":program("return 1",hyprland_editor_core::BASELINE),"hidden":1}),
    ] {
        assert_eq!(request(op)["outcome"]["kind"], "error");
    }
    let mut p = program("return 1", hyprland_editor_core::BASELINE);
    p["documents"][0]["hidden"] = true.into();
    assert_eq!(analyze(p)["kind"], "error");
    let bad = bytes(b"{bad JSON}".to_vec());
    assert!(bad["profile"].is_null() && bad["nonce"].is_null());
}
#[test]
fn imported_effects_stop_and_kernel_boundary_denies_capabilities() {
    let out = analyze(program(
        "os.execute('touch /no-such-authority');return 1",
        hyprland_editor_core::BASELINE,
    ));
    assert_eq!(out["report"]["halt"]["kind"], "unresolved", "{out}");
    let out = request(json!({"kind":"probe"}));
    for field in [
        "file_denied",
        "network_denied",
        "process_denied",
        "inherited_descriptor_closed",
    ] {
        assert_eq!(out["outcome"][field], true, "{out}");
    }
}

#[test]
fn source_request_stall_and_blocked_output_have_owned_deadlines() {
    let (mut cmd, held) = command();
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    let start = Instant::now();
    assert!(!finish(&mut child.0, Duration::from_secs(5)).success());
    assert!(start.elapsed() >= Duration::from_secs(2));
    let source = format!("return '{}'", "x".repeat(20000));
    let payload = json!({"protocol":"hye-lua55-source-1","nonce":vec![7u8;32],"expected_release":release(),"operation":{"kind":"analyze","compiler":profile(),"program":program(&source,hyprland_editor_core::BASELINE)}});
    let wire = serde_json::to_vec(&payload).unwrap();
    let (mut cmd, held) = command();
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    let mut input = child.0.stdin.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(&wire));
    let start = Instant::now();
    assert!(!finish(&mut child.0, Duration::from_secs(5)).success());
    assert!(start.elapsed() >= Duration::from_secs(2));
    let _ = writer.join().unwrap();
}
#[test]
fn oversized_wire_is_refused_before_an_operation_can_be_accepted() {
    let (mut cmd, held) = command();
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    let mut input = child.0.stdin.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(&vec![b' '; 4 * 1024 * 1024 + 1]));
    assert!(!finish(&mut child.0, Duration::from_secs(5)).success());
    let _ = writer.join().unwrap();
}
#[test]
fn inherited_capability_is_closed_before_any_request_and_environment_is_refused() {
    let (mut cmd, held) = command();
    let file = File::open("/etc/passwd").unwrap();
    let fd = file.as_raw_fd();
    unsafe {
        cmd.pre_exec(move || {
            if libc::dup2(fd, 63) < 0 || libc::fcntl(63, libc::F_SETFD, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    drop(file);
    let wire=serde_json::to_vec(&json!({"protocol":"hye-lua55-source-1","nonce":vec![1u8;32],"expected_release":release(),"operation":{"kind":"probe"}})).unwrap();
    child.0.stdin.take().unwrap().write_all(&wire).unwrap();
    let mut output = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut output)
        .unwrap();
    assert!(finish(&mut child.0, Duration::from_secs(5)).success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output).unwrap()["outcome"]["inherited_descriptor_closed"],
        true
    );
    let (mut cmd, held) = command();
    cmd.env("HYE_UNEXPECTED_ENV", "1");
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    assert!(!finish(&mut child.0, Duration::from_secs(2)).success());
}

#[test]
fn language_corpus_distinguishes_modeled_compile_refused_and_unresolved_cases() {
    let corpus: Value =
        serde_json::from_str(include_str!("../../../data/lua55-language-fixtures.json")).unwrap();
    assert_eq!(corpus["cases"].as_array().unwrap().len(), 16);
    for case in corpus["cases"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let out = analyze(program(source, hyprland_editor_core::BASELINE));
        let report = &out["report"];
        if !case["observation"]["compiled"].as_bool().unwrap() {
            assert_eq!(report["halt"]["kind"], "lua_error", "{case}: {out}");
            assert!(report["events"].as_array().unwrap().is_empty());
            continue;
        }
        if [
            "global_constant_initialization",
            "global_initialization_does_not_replace",
            "generic_for_secondary_writable",
        ]
        .contains(&case["case"].as_str().unwrap())
        {
            assert_eq!(report["halt"]["kind"], "unresolved", "{case}: {out}");
            let reason = report["halt"]["reason"].as_str().unwrap();
            assert!(
                if case["case"] == "generic_for_secondary_writable" {
                    reason.contains("Generic iteration order")
                } else {
                    reason.contains("Global initialization")
                },
                "{case}: {out}"
            );
            continue;
        }
        assert!(report["halt"].is_null(), "{case}: {out}");
        let expected = case["observation"]["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                if v["type"] == "bytes" {
                    let hex = v["hex"].as_str().unwrap();
                    let value: Vec<u8> = (0..hex.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                        .collect();
                    json!({"type":"bytes","value":value})
                } else {
                    v.clone()
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(report["returns"], json!(expected), "{case}: {out}");
    }
}
