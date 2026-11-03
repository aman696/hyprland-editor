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
const EXE: &str = env!("CARGO_BIN_EXE_hyprland-editor-lua55");
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
    cmd.arg("--compiler-worker")
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
    // CI supplies an explicit private release. No paths or libraries come from
    // this variable; it is only the expected response version.
    std::env::var("HYE_LUA55_TEST_RELEASE")
        .unwrap_or_else(|_| env!("HYE_LUA55_PACKAGE_RELEASE").into())
}
fn request(operation: Value) -> Value {
    let value = json!({"protocol":"hye-lua55-compiler-1","nonce":vec![7u8;32],"expected_release":release(),"operation":operation});
    let wire = serde_json::to_vec(&value).unwrap();
    let digest = format!("{:x}", Sha256::digest(&wire));
    let response = bytes(wire);
    assert_eq!(response["protocol"], "hye-lua55-compiler-1");
    assert_eq!(response["nonce"], json!(vec![7u8; 32]));
    assert_eq!(response["request_sha256"], digest);
    assert_eq!(response["profile"]["release"], release());
    assert_eq!(response["profile"]["family"], 505);
    assert_eq!(response["profile"]["numeric_locale"], "C");
    assert_eq!(response["profile"]["private_seed"], "kernel_random");
    assert_eq!(response["profile"]["imported_code_executed"], false);
    assert_eq!(response["profile"]["complete_language_verified"], false);
    assert_eq!(response["profile"]["selected_compositor_verified"], false);
    response["outcome"].clone()
}
fn compile(source: &[u8], name: &[u8]) -> Value {
    request(json!({"kind":"compile","source":source,"load_name":name}))
}
#[test]
fn actual_separate_library_profile_and_compile_only_language_boundaries() {
    let profile = request(json!({"kind":"profile"}));
    assert_eq!(profile["kind"], "profile");
    let good = compile(b"global x; x=3; return x", b"/private/fixture.lua");
    assert_eq!(good["valid"], true);
    assert_eq!(good["diagnostic"], json!([]));
    let bad = compile(b"for i=1,2 do i=3 end", b"/private/fixture.lua");
    assert_eq!(bad["valid"], false);
    let diagnostic: Vec<u8> = serde_json::from_value(bad["diagnostic"].clone()).unwrap();
    assert!(String::from_utf8_lossy(&diagnostic).contains("const variable"));
    let binary = compile(&[27, b'L', b'u', b'a'], b"binary");
    assert_eq!(binary["valid"], false);
}
#[test]
fn imported_code_is_compiled_without_execution_or_filesystem_access() {
    let path =
        std::env::temp_dir().join(format!("hye-lua55-never-executed-{}", std::process::id()));
    assert!(!path.exists());
    let code = format!("os.execute('touch {}'); while true do end", path.display());
    assert_eq!(compile(code.as_bytes(), b"effects")["valid"], true);
    assert!(!path.exists());
    let probe = request(json!({"kind":"probe"}));
    for field in [
        "file_denied",
        "network_denied",
        "process_denied",
        "inherited_descriptor_closed",
    ] {
        assert_eq!(probe[field], true, "{probe}");
    }
}
#[test]
fn actual_primitive_conversions_preserve_integer_float_bits_and_c_string_rules() {
    let values = vec![
        json!({"type":"nil"}),
        json!({"type":"bool","value":false}),
        json!({"type":"int","value":i64::MAX}),
        json!({"type":"int","value":i64::MIN}),
        json!({"type":"float_bits","value":(-0.0f64).to_bits()}),
        json!({"type":"float_bits","value":3.0f64.to_bits()}),
        json!({"type":"float_bits","value":3.5f64.to_bits()}),
        json!({"type":"bytes","value":b" 0x1p0 ".to_vec()}),
        json!({"type":"bytes","value":b"a\0b".to_vec()}),
        json!({"type":"float_bits","value":0x7ff8_0000_0000_0042u64}),
    ];
    let response = request(json!({"kind":"convert","values":values}));
    assert_eq!(response["kind"], "converted");
    let v = response["values"].as_array().unwrap();
    assert_eq!(v[0]["flags"], 0);
    assert_eq!(v[1]["flags"], 8);
    assert_eq!(v[2]["integer"], i64::MAX);
    assert_eq!(v[2]["coerced_integer"], i64::MAX);
    assert_eq!(v[3]["integer"], i64::MIN);
    assert_eq!(v[3]["coerced_integer"], i64::MIN);
    assert_eq!(v[4]["number_bits"], (-0.0f64).to_bits());
    assert_eq!(v[5]["text"], json!(b"3.0".to_vec()));
    assert_eq!(v[6]["coerced_integer"], 0);
    assert_eq!(v[7]["flags"], 6);
    assert_eq!(v[7]["coerced_integer"], 1);
    assert_eq!(v[8]["text"], json!([b'a']));
    assert_eq!(v[9]["number_bits"], 0x7ff8_0000_0000_0042u64);
    let batch = (0..256)
        .map(|v| json!({"type":"int","value":v}))
        .collect::<Vec<_>>();
    let response = request(json!({"kind":"convert","values":batch}));
    for (i, row) in response["values"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row["integer"], i as i64);
    }
}
#[test]
fn release_protocol_unknown_fields_and_executable_operations_are_refused() {
    let mut value = json!({"protocol":"hye-lua55-compiler-1","nonce":vec![9u8;32],"expected_release":"5.4.9","operation":{"kind":"profile"}});
    let response = bytes(serde_json::to_vec(&value).unwrap());
    assert_eq!(response["outcome"]["kind"], "error");
    assert!(response["outcome"]["reason"]
        .as_str()
        .unwrap()
        .contains("release differs"));
    value["expected_release"] = release().into();
    value["protocol"] = "future".into();
    assert_eq!(
        bytes(serde_json::to_vec(&value).unwrap())["outcome"]["kind"],
        "error"
    );
    value["protocol"] = "hye-lua55-compiler-1".into();
    value["executable"] = "/no-such-worker".into();
    let refused = bytes(serde_json::to_vec(&value).unwrap());
    assert!(refused["profile"].is_null());
    value.as_object_mut().unwrap().remove("executable");
    for kind in ["profile", "probe"] {
        value["operation"] = json!({"kind":kind,"hidden":"must refuse"});
        let refused = bytes(serde_json::to_vec(&value).unwrap());
        assert!(refused["profile"].is_null());
        assert_eq!(refused["outcome"]["kind"], "error");
    }
    value["operation"] = json!({"kind":"execute","source":[]});
    assert_eq!(
        bytes(serde_json::to_vec(&value).unwrap())["outcome"]["kind"],
        "error"
    );
}
#[test]
fn source_name_primitive_and_aggregate_output_bounds_are_actual_refusals() {
    assert_eq!(compile(b"return 1", b"bad\0name")["kind"], "error");
    assert_eq!(compile(b"return 1", &vec![b'n'; 4097])["kind"], "error");
    assert_eq!(
        compile(&vec![0; 1024 * 1024 + 1], b"large")["kind"],
        "error"
    );
    assert_eq!(
        request(json!({"kind":"convert","values":vec![json!({"type":"nil"});257]}))["kind"],
        "error"
    );
    assert_eq!(
        request(json!({"kind":"convert","values":[{"type":"bytes","value":vec![0;65537]}]}))
            ["kind"],
        "error"
    );
    let large = json!({"type":"bytes","value":vec![b'x';65536]});
    let response = request(json!({"kind":"convert","values":vec![large;9]}));
    assert_eq!(response["kind"], "error");
    assert!(response["reason"].as_str().unwrap().contains("512 KiB"));
}
#[test]
fn oversized_wire_and_malformed_json_cannot_become_accepted_operations() {
    let malformed = bytes(b"{not a request}".to_vec());
    assert_eq!(malformed["outcome"]["kind"], "error");
    assert!(malformed["nonce"].is_null() && malformed["profile"].is_null());
    let (mut cmd, held) = command();
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    let mut input = child.0.stdin.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(&vec![b' '; 4 * 1024 * 1024 + 1]));
    let output = child.0.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        output.take(16384).read_to_end(&mut bytes).unwrap();
        bytes
    });
    assert!(!finish(&mut child.0, Duration::from_secs(6)).success());
    let _ = writer.join().unwrap();
    assert!(reader.join().unwrap().is_empty());
}
#[test]
fn native_allocator_refuses_large_valid_source_without_executing_a_prefix() {
    let literals = (0..95000)
        .map(|i| format!("'a{i}'"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("return {{{literals}}}");
    assert!(source.len() < 1024 * 1024);
    let response = compile(source.as_bytes(), b"allocator-bound");
    assert_eq!(response["kind"], "error");
    assert_eq!(response["reason"], "Compiler allocation budget exceeded");
    assert_eq!(
        compile(b"return 1", b"independent-next-request")["valid"],
        true
    );
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
    let wire=serde_json::to_vec(&json!({"protocol":"hye-lua55-compiler-1","nonce":vec![1u8;32],"expected_release":release(),"operation":{"kind":"probe"}})).unwrap();
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
fn unavailable_kernel_entropy_is_refused_without_a_fixed_seed_fallback() {
    let (mut cmd, held) = command();
    unsafe {
        cmd.pre_exec(|| {
            // Additional denial in this owned test child only. All other host
            // restrictions remain; no desktop or parent policy is changed.
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
                    k: libc::SYS_getrandom as u32,
                },
                libc::sock_filter {
                    code: 0x06,
                    jt: 0,
                    jf: 0,
                    k: 0x00050000 | libc::EPERM as u32,
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
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    let wire = serde_json::to_vec(&json!({"protocol":"hye-lua55-compiler-1","nonce":vec![3u8;32],"expected_release":release(),"operation":{"kind":"profile"}})).unwrap();
    child.0.stdin.take().unwrap().write_all(&wire).unwrap();
    let output = child.0.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        output.take(16384).read_to_end(&mut bytes).unwrap();
        bytes
    });
    assert!(finish(&mut child.0, Duration::from_secs(6)).success());
    let response: Value = serde_json::from_slice(&reader.join().unwrap()).unwrap();
    assert!(response["profile"].is_null());
    assert_eq!(response["outcome"]["kind"], "error");
    assert_eq!(
        response["outcome"]["reason"],
        "Private compiler seed is unavailable"
    );
}
#[test]
fn unfinished_input_and_blocked_output_have_worker_owned_wall_deadlines() {
    let (mut cmd, held) = command();
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    let start = Instant::now(); // Keep stdin open, so EOF cannot end the request.
    assert!(!finish(&mut child.0, Duration::from_secs(5)).success());
    assert!(start.elapsed() >= Duration::from_secs(2));
    let (mut cmd, held) = command();
    let mut child = Running(cmd.spawn().unwrap());
    drop(held);
    let payload = json!({"protocol":"hye-lua55-compiler-1","nonce":vec![1u8;32],"expected_release":release(),"operation":{"kind":"convert","values":[{"type":"bytes","value":vec![b'x';128*1024/2]}]}});
    let wire = serde_json::to_vec(&payload).unwrap();
    let mut input = child.0.stdin.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(&wire));
    // Do not drain stdout. The encoded result exceeds a normal pipe capacity.
    assert!(!finish(&mut child.0, Duration::from_secs(5)).success());
    let _ = writer.join().unwrap();
}
