//! Parent transport fixtures are trusted literals. The ignored case requires an
//! explicit private static worker; no system package or compositor is modified.
#[allow(dead_code)]
#[path = "../src/compiler_client.rs"]
mod client;
use client::wire;
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{Read, Write},
    os::fd::FromRawFd,
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
fn memory(bytes: &[u8]) -> File {
    let fd = unsafe {
        libc::memfd_create(
            c"hye-client-fixture".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    assert!(fd >= 0);
    let mut file = unsafe { File::from_raw_fd(fd) };
    file.write_all(bytes).unwrap();
    file
}
fn python(body: &str) -> Command {
    let mut command = Command::new("/usr/bin/python3");
    command.arg("-I").arg("-S").arg("-c").arg(body).env_clear();
    command
}
fn response() -> Value {
    json!({"protocol":wire::PROTOCOL,"nonce":vec![9u8;32],"request_sha256":"pending","profile":{"release":"5.5.1","library_identity":"Lua 5.5.1 fixture ","family":505,"numeric_locale":"C","private_seed":"kernel_random","global_declaration_compiles":true,"global_identifier_compiles":true,"imported_code_executed":false,"complete_language_verified":false,"selected_compositor_verified":false},"outcome":{"kind":"profile"}})
}
fn reaped(pid: u32) {
    assert!(pid > 0);
    assert_eq!(
        unsafe { libc::waitpid(pid as _, std::ptr::null_mut(), libc::WNOHANG) },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
#[test]
fn strict_reply_correlates_exact_request_profile_operation_and_output_shape() {
    let request = wire::Request {
        protocol: wire::PROTOCOL.into(),
        nonce: [9; 32],
        expected_release: "5.5.1".into(),
        operation: wire::Operation::Profile {},
    };
    let bytes = serde_json::to_vec(&request).unwrap();
    let mut good = response();
    good["request_sha256"] =
        hyprland_editor_core::lua::source_hash(std::str::from_utf8(&bytes).unwrap()).into();
    let decode = |v: Value| serde_json::from_value::<wire::Reply>(v).unwrap();
    client::fixture_reply(&request, &bytes, &decode(good.clone())).unwrap();
    for case in 0..9 {
        let mut v = good.clone();
        match case {
            0 => v["nonce"][0] = 0.into(),
            1 => v["request_sha256"] = "0".repeat(64).into(),
            2 => v["protocol"] = "other".into(),
            3 => v["profile"]["release"] = "5.5.0".into(),
            4 => v["profile"]["private_seed"] = "fixed".into(),
            5 => v["profile"]["imported_code_executed"] = true.into(),
            6 => v["profile"]["complete_language_verified"] = true.into(),
            7 => v["profile"]["selected_compositor_verified"] = true.into(),
            _ => v["outcome"] = json!({"kind":"compiled","valid":true,"diagnostic":[]}),
        }
        assert!(
            client::fixture_reply(&request, &bytes, &decode(v)).is_err(),
            "case {case}"
        );
    }
    for field in ["extra", "profile", "outcome"] {
        let mut v = good.clone();
        if field == "extra" {
            v["extra"] = true.into();
        } else {
            v[field]["extra"] = true.into();
        }
        assert!(serde_json::from_value::<wire::Reply>(v).is_err());
    }
}
#[test]
fn duplex_parent_drains_output_before_request_is_consumed_and_requires_pipe_eof() {
    let output = serde_json::to_string(&response()).unwrap();
    let body=format!("import sys\nsys.stdout.buffer.write(b' '*262144);sys.stdout.flush()\nsys.stdin.buffer.read()\nsys.stdout.write({output:?})\n");
    let reply = client::fixture_exchange(
        python(&body),
        &vec![b' '; 1024 * 1024],
        &AtomicBool::new(false),
        Duration::from_secs(2),
        |_| {},
    )
    .unwrap();
    assert!(matches!(reply.outcome, wire::Outcome::Profile {}));
}
#[test]
fn parent_deadline_kills_and_reaps_a_child_that_never_reads_its_request() {
    let pid = AtomicU32::new(0);
    let start = Instant::now();
    let result = client::fixture_exchange(
        python("import time;time.sleep(20)"),
        &vec![b'x'; 1024 * 1024],
        &AtomicBool::new(false),
        Duration::from_millis(120),
        |p| pid.store(p, Ordering::Release),
    );
    assert!(result.unwrap_err().contains("deadline"));
    assert!(start.elapsed() < Duration::from_secs(2));
    reaped(pid.load(Ordering::Acquire));
}
#[test]
fn active_and_pre_spawn_cancellation_do_not_leave_a_child() {
    assert!(client::fixture_exchange(
        python("raise Exception('must not start')"),
        b"",
        &AtomicBool::new(true),
        Duration::from_secs(2),
        |_| panic!("spawned")
    )
    .unwrap_err()
    .contains("cancelled"));
    let cancelled = Arc::new(AtomicBool::new(false));
    let child_flag = cancelled.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        client::fixture_exchange(
            python("import time;time.sleep(20)"),
            b"",
            &child_flag,
            Duration::from_secs(2),
            |p| sender.send(p).unwrap(),
        )
    });
    let pid = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    cancelled.store(true, Ordering::Release);
    assert!(thread.join().unwrap().unwrap_err().contains("cancelled"));
    reaped(pid);
}
#[test]
fn unwind_and_output_floods_stop_and_reap_the_owned_process() {
    let pid = AtomicU32::new(0);
    let panic = std::panic::catch_unwind(|| {
        let _ = client::fixture_exchange(
            python("import time;time.sleep(20)"),
            b"",
            &AtomicBool::new(false),
            Duration::from_secs(2),
            |p| {
                pid.store(p, Ordering::Release);
                panic!("owned fixture unwind")
            },
        );
    });
    assert!(panic.is_err());
    reaped(pid.load(Ordering::Acquire));
    for fd in [1, 2] {
        let pid = AtomicU32::new(0);
        let body = format!("import os\nwhile True: os.write({fd},b'x'*16384)");
        assert!(client::fixture_exchange(
            python(&body),
            b"",
            &AtomicBool::new(false),
            Duration::from_secs(2),
            |p| pid.store(p, Ordering::Release)
        )
        .unwrap_err()
        .contains("byte bound"));
        reaped(pid.load(Ordering::Acquire));
    }
}
#[test]
fn malformed_nonzero_and_truncated_replies_are_not_accepted() {
    for body in ["print('{}')", "print('{')", "raise SystemExit(7)"] {
        let pid = AtomicU32::new(0);
        assert!(client::fixture_exchange(
            python(body),
            b"",
            &AtomicBool::new(false),
            Duration::from_secs(1),
            |p| pid.store(p, Ordering::Release)
        )
        .is_err());
        reaped(pid.load(Ordering::Acquire));
    }
}
fn elf() -> Vec<u8> {
    let mut b = vec![0; 512];
    b[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    b[16..18].copy_from_slice(&2u16.to_le_bytes());
    let machine: u16 = if cfg!(target_arch = "aarch64") {
        183
    } else {
        62
    };
    b[18..20].copy_from_slice(&machine.to_le_bytes());
    b[20..24].copy_from_slice(&1u32.to_le_bytes());
    b[24..32].copy_from_slice(&0x400100u64.to_le_bytes());
    b[32..40].copy_from_slice(&64u64.to_le_bytes());
    b[52..54].copy_from_slice(&64u16.to_le_bytes());
    b[54..56].copy_from_slice(&56u16.to_le_bytes());
    b[56..58].copy_from_slice(&1u16.to_le_bytes());
    b[64..68].copy_from_slice(&1u32.to_le_bytes());
    b[68..72].copy_from_slice(&5u32.to_le_bytes());
    b[80..88].copy_from_slice(&0x400000u64.to_le_bytes());
    b[96..104].copy_from_slice(&512u64.to_le_bytes());
    b[104..112].copy_from_slice(&512u64.to_le_bytes());
    b
}
#[test]
fn image_admission_refuses_loaders_dependencies_foreign_and_writable_code() {
    client::fixture_elf(&memory(&elf())).unwrap();
    for case in 0..6 {
        let mut b = elf();
        match case {
            0 => b[64..68].copy_from_slice(&3u32.to_le_bytes()),
            1 => b[68..72].copy_from_slice(&7u32.to_le_bytes()),
            2 => b[18..20].copy_from_slice(&0u16.to_le_bytes()),
            3 => b[32..40].copy_from_slice(&u64::MAX.to_le_bytes()),
            4 => {
                b[64..68].copy_from_slice(&2u32.to_le_bytes());
                b[72..80].copy_from_slice(&128u64.to_le_bytes());
                b[96..104].copy_from_slice(&16u64.to_le_bytes());
                b[128..136].copy_from_slice(&1u64.to_le_bytes());
            }
            _ => b.truncate(63),
        };
        assert!(client::fixture_elf(&memory(&b)).is_err(), "case {case}");
    }
    assert!(client::Compiler::fixture(&memory(&elf()), &"0".repeat(64), "5.5.1").is_err());
}
#[test]
#[ignore = "requires a temporary explicitly supplied static Lua compiler image"]
fn actual_static_worker_parent_preserves_identity_and_never_executes_source() {
    let path = std::env::var("HYE_COMPILER_CLIENT_IMAGE").expect("explicit private image");
    let sha = std::env::var("HYE_COMPILER_CLIENT_SHA256").unwrap();
    let release = std::env::var("HYE_COMPILER_CLIENT_RELEASE").unwrap();
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    let origin = memory(&bytes);
    let compiler = client::Compiler::fixture(&origin, &sha, &release).unwrap();
    assert_eq!(compiler.image_sha256(), sha);
    origin.set_len(0).unwrap();
    drop(origin);
    let cancelled = AtomicBool::new(false);
    compiler
        .run(wire::Operation::Profile {}, &cancelled)
        .unwrap();
    let valid = compiler
        .run(
            wire::Operation::Compile {
                source: b"global os; os.execute('must never run'); while true do end".to_vec(),
                load_name: b"/private/test.lua".to_vec(),
            },
            &cancelled,
        )
        .unwrap();
    assert!(matches!(
        valid.outcome,
        wire::Outcome::Compiled { valid: true, .. }
    ));
    let invalid = compiler
        .run(
            wire::Operation::Compile {
                source: b"for i=1,2 do i=3 end".to_vec(),
                load_name: b"/private/test.lua".to_vec(),
            },
            &cancelled,
        )
        .unwrap();
    assert!(matches!(
        invalid.outcome,
        wire::Outcome::Compiled { valid: false, .. }
    ));
    let converted = compiler
        .run(
            wire::Operation::Convert {
                values: vec![
                    wire::Primitive::FloatBits((-0.0f64).to_bits()),
                    wire::Primitive::Int(i64::MAX),
                    wire::Primitive::Bytes(b"a\0b".to_vec()),
                ],
            },
            &cancelled,
        )
        .unwrap();
    let wire::Outcome::Converted { values } = converted.outcome else {
        panic!("conversion")
    };
    assert_eq!(values[0].number_bits, (-0.0f64).to_bits());
    assert_eq!(values[1].integer, i64::MAX);
    assert_eq!(values[2].text, b"a");
    compiler.run(wire::Operation::Probe {}, &cancelled).unwrap();
    cancelled.store(true, Ordering::Release);
    assert!(compiler
        .run(wire::Operation::Profile {}, &cancelled)
        .unwrap_err()
        .contains("cancelled"));
}

#[test]
#[ignore = "requires root-owned disposable package fixtures and ordinary-user execution"]
fn actual_root_package_registry_admits_only_immutable_static_registered_worker() {
    use std::os::unix::fs::OpenOptionsExt;
    let root = std::path::PathBuf::from(
        std::env::var("HYE_COMPILER_REGISTRY_FIXTURE")
            .expect("explicit private root package fixture"),
    );
    for name in [
        "writable-directory",
        "writable-manifest",
        "foreign-image",
        "symlink-image",
        "symlink-manifest",
        "fifo-image",
        "hardlink-image",
        "wrong-hash",
        "unknown-manifest",
    ] {
        let folder = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(root.join(name))
            .unwrap();
        assert!(
            client::Compiler::fixture_installed(&folder).is_err(),
            "accepted {name}"
        );
    }
    let folder = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(root.join("good"))
        .unwrap();
    let compiler = client::Compiler::fixture_installed(&folder).unwrap();
    let cancelled = AtomicBool::new(false);
    compiler
        .run(wire::Operation::Profile {}, &cancelled)
        .unwrap();
    compiler.run(wire::Operation::Probe {}, &cancelled).unwrap();
}
