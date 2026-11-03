//! Trusted static boundary probe only. Not a compositor/runtime launch API.
use super::*;
use std::{
    io::{Read, Seek, SeekFrom},
    os::{fd::AsFd, unix::process::CommandExt},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[cfg(target_arch = "x86_64")]
const AUDIT_ARCH: u32 = 0xc000003e;
#[cfg(target_arch = "aarch64")]
const AUDIT_ARCH: u32 = 0xc00000b7;

// Default-deny for the static probe, not a claimed Hyprland/Mesa syscall profile.
// The initial execve is necessary for bubblewrap; execveat is never allowed.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn filter() -> Vec<libc::sock_filter> {
    fn ins(code: u16, jt: u8, jf: u8, k: u32) -> libc::sock_filter {
        libc::sock_filter { code, jt, jf, k }
    }
    let load = (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16;
    let equal = (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16;
    let ret = (libc::BPF_RET | libc::BPF_K) as u16;
    let deny = libc::SECCOMP_RET_ERRNO | libc::EPERM as u32;
    let mut rules = vec![
        ins(load, 0, 0, 4),
        ins(equal, 1, 0, AUDIT_ARCH),
        ins(ret, 0, 0, libc::SECCOMP_RET_KILL_PROCESS),
        ins(load, 0, 0, 0),
    ];
    for number in [libc::SYS_socket, libc::SYS_socketpair] {
        rules.extend([
            ins(equal, 0, 4, number as u32),
            ins(load, 0, 0, 16),
            ins(equal, 0, 1, libc::AF_UNIX as u32),
            ins(ret, 0, 0, libc::SECCOMP_RET_ALLOW),
            ins(ret, 0, 0, deny),
        ]);
    }
    let mut calls = vec![
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_close,
        libc::SYS_openat,
        libc::SYS_newfstatat,
        libc::SYS_fstat,
        libc::SYS_statfs,
        libc::SYS_fstatfs,
        libc::SYS_lseek,
        libc::SYS_pread64,
        libc::SYS_mmap,
        libc::SYS_mprotect,
        libc::SYS_munmap,
        libc::SYS_brk,
        libc::SYS_set_tid_address,
        libc::SYS_set_robust_list,
        libc::SYS_rseq,
        libc::SYS_prlimit64,
        libc::SYS_getrandom,
        libc::SYS_rt_sigaction,
        libc::SYS_rt_sigprocmask,
        libc::SYS_rt_sigreturn,
        libc::SYS_sigaltstack,
        libc::SYS_futex,
        libc::SYS_exit,
        libc::SYS_exit_group,
        libc::SYS_execve,
        libc::SYS_wait4,
        libc::SYS_getuid,
        libc::SYS_geteuid,
        libc::SYS_getgid,
        libc::SYS_getegid,
        libc::SYS_getgroups,
        libc::SYS_getpid,
        libc::SYS_getppid,
        libc::SYS_gettid,
        libc::SYS_getdents64,
        libc::SYS_readlinkat,
        libc::SYS_fcntl,
        libc::SYS_uname,
        libc::SYS_getcwd,
        libc::SYS_clock_gettime,
        libc::SYS_connect,
        libc::SYS_unlinkat,
        libc::SYS_mkdirat,
    ];
    #[cfg(target_arch = "x86_64")]
    calls.extend([
        libc::SYS_arch_prctl,
        libc::SYS_stat,
        libc::SYS_readlink,
        libc::SYS_access,
        libc::SYS_mkdir,
        libc::SYS_unlink,
    ]);
    calls.sort_unstable();
    calls.dedup();
    for call in calls {
        rules.extend([
            ins(equal, 0, 1, call as u32),
            ins(ret, 0, 0, libc::SECCOMP_RET_ALLOW),
        ]);
    }
    rules.push(ins(ret, 0, 0, deny));
    rules
}

fn independent_read(fd: BorrowedFd<'_>) -> File {
    // Only our own live sealed file; new open description avoids sharing offsets.
    File::open(format!("/proc/self/fd/{}", fd.as_raw_fd())).unwrap()
}

#[test]
#[ignore = "requires explicit ephemeral CI provider, static fixture and user namespaces; no host fallback"]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn sealed_runtime_executes_only_inside_private_root_and_namespaces() {
    runtime_probe(false);
}

#[test]
#[ignore = "requires the explicit ephemeral CI provider, trusted constructor and delegated launch gate"]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn gated_runtime_image_keeps_resource_placement_and_private_boundaries_together() {
    runtime_probe(true);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn runtime_probe(gated: bool) {
    assert!(unsafe { libc::getuid() } > 0);
    assert_eq!(unsafe { libc::getuid() }, unsafe { libc::geteuid() });
    assert_eq!(unsafe { libc::getgroups(0, std::ptr::null_mut()) }, 0);
    assert_eq!(
        unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) },
        1
    );
    let binary = File::open(
        std::env::var_os("HYE_RUNTIME_PROBE_BIN").expect("Explicit trusted static probe required"),
    )
    .unwrap();
    let mut bytes = Vec::new();
    binary
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() as u64 <= MAX_FILE_BYTES);
    let binary = File::open(std::env::var_os("HYE_RUNTIME_PROBE_BIN").unwrap()).unwrap();
    let manifest = Manifest {
        format: 1,
        entry: "/app/hye-probe".into(),
        files: vec![
            FileSpec {
                destination: "/app/hye-probe".into(),
                bytes: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                executable: true,
            },
            FileSpec {
                destination: "/usr/share/hye/canary".into(),
                bytes: 16,
                sha256: format!("{:x}", Sha256::digest(b"reviewed-runtime")),
                executable: false,
            },
        ],
    };
    drop(bytes);
    let canary = super::tests::source(b"reviewed-runtime");
    let image = Image::capture(manifest, &[binary.as_fd(), canary.as_fd()]).unwrap();
    canary.write_all_at(b"modified-runtime", 0).unwrap();
    let mut snapshots: Vec<_> = image.files().map(|(_, fd)| independent_read(fd)).collect();
    // The native sock_filter representation is the Linux --seccomp FD ABI.
    let rules = filter();
    let policy = unsafe {
        std::slice::from_raw_parts(
            rules.as_ptr().cast::<u8>(),
            rules.len() * std::mem::size_of::<libc::sock_filter>(),
        )
    };
    let mut policy = if gated {
        eprintln!("HYE_GATED_POLICY_SHA256 {:x}", Sha256::digest(policy));
        frozen_input(policy, false)
    } else {
        super::tests::source(policy)
    };
    policy.seek(SeekFrom::Start(0)).unwrap();
    let inherited: Vec<_> = snapshots
        .iter()
        .map(AsRawFd::as_raw_fd)
        .chain([policy.as_raw_fd()])
        .collect();
    let fd_name = |raw: i32| {
        if gated {
            (3 + inherited
                .iter()
                .position(|fd| *fd == raw)
                .expect("Explicit runtime input"))
            .to_string()
        } else {
            raw.to_string()
        }
    };

    let abstract_name = format!("hye-probe-{}", std::process::id());
    use std::os::linux::net::SocketAddrExt;
    let address =
        std::os::unix::net::SocketAddr::from_abstract_name(abstract_name.as_bytes()).unwrap();
    let _host_listener = std::os::unix::net::UnixListener::bind_addr(&address).unwrap();
    let host_path =
        std::env::var("HYE_RUNTIME_PROBE_HOST_CANARY").expect("Explicit CI canary required");
    let _host_file = File::open(&host_path).unwrap();
    let mut command = Command::new(
        std::env::var_os("HYE_RUNTIME_BWRAP_BIN").expect("Explicit trusted constructor required"),
    );
    command
        .env_clear()
        .env("LC_ALL", "C")
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args([
            "--unshare-user",
            "--unshare-pid",
            "--unshare-net",
            "--unshare-ipc",
            "--unshare-uts",
            "--unshare-cgroup",
            "--disable-userns",
            "--assert-userns-disabled",
            "--uid",
            "1000",
            "--gid",
            "1000",
            "--hostname",
            "hye-lab",
            "--new-session",
            "--die-with-parent",
            "--cap-drop",
            "ALL",
            "--clearenv",
            "--size",
            "33554432",
            "--tmpfs",
            "/",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--size",
            "33554432",
            "--perms",
            "0700",
            "--tmpfs",
            "/tmp",
            "--size",
            "16777216",
            "--perms",
            "0700",
            "--tmpfs",
            "/run",
            "--size",
            "16777216",
            "--perms",
            "0700",
            "--tmpfs",
            "/home",
            "--size",
            "33554432",
            "--perms",
            "0700",
            "--tmpfs",
            "/state",
            "--dir",
            "/home/lab",
            "--dir",
            "/run/lab",
            "--dir",
            "/state/config",
            "--dir",
            "/state/data",
            "--dir",
            "/state/cache",
        ]);
    for (spec, fd) in image.manifest().files.iter().zip(&snapshots) {
        command
            .args([
                "--perms",
                if spec.executable { "0500" } else { "0400" },
                "--ro-bind-data",
            ])
            .arg(fd_name(fd.as_raw_fd()))
            .arg(&spec.destination);
    }
    command
        .args(["--seccomp"])
        .arg(fd_name(policy.as_raw_fd()))
        .args(["--remount-ro", "/", "--chdir", "/tmp"]);
    for (key, value) in [
        ("HOME", "/home/lab"),
        ("XDG_RUNTIME_DIR", "/run/lab"),
        ("XDG_CONFIG_HOME", "/state/config"),
        ("XDG_DATA_HOME", "/state/data"),
        ("XDG_CACHE_HOME", "/state/cache"),
        ("LC_ALL", "C"),
        ("TMPDIR", "/tmp"),
        ("PWD", "/tmp"),
    ] {
        command.args(["--setenv", key, value]);
    }
    command
        .arg("--")
        .arg(&image.manifest().entry)
        .arg(host_path)
        .arg(abstract_name);
    for namespace in ["user", "mnt", "pid", "net", "ipc", "uts", "cgroup"] {
        command.arg(
            std::fs::metadata(format!("/proc/self/ns/{namespace}"))
                .unwrap()
                .ino()
                .to_string(),
        );
    }
    let descriptors = inherited;
    // Only this child gets inherited data/policy descriptors. No shared parent FD
    // flag changes; all host/cgroup/process authority stays CLOEXEC in the parent.
    unsafe {
        command.pre_exec(move || {
            for raw in &descriptors {
                if libc::fcntl(*raw, libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let (status_code, success, out, err) = if gated {
        let passed: Vec<_> = snapshots
            .iter()
            .map(AsFd::as_fd)
            .chain([policy.as_fd()])
            .collect();
        gated_probe(&command, &passed)
    } else {
        ordinary_probe(command)
    };
    snapshots.clear();
    drop(policy);
    eprintln!("HYE_RUNTIME_IMAGE {}\n{out}{err}", image.identity());
    if std::env::var("HYE_RUNTIME_EXPECT_UNAVAILABLE").as_deref() == Ok("1") {
        // Separate compatibility refusal, never counted as positive execution.
        assert_eq!(status_code, Some(1));
        assert!(out.is_empty());
        assert_eq!(
            err.trim(),
            "bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted"
        );
        eprintln!("HYE_RUNTIME_UNAVAILABLE constructor refused before trusted probe execution");
        return;
    }
    assert!(success, "Private runtime probe failed: {status_code:?}");
    assert!(out.contains("HYE_RUNTIME_RESULT all boundary assertions passed"));
    if gated {
        eprintln!("HYE_GATED_RUNTIME_RESULT resource placement, private boundaries and explicit cleanup passed");
    }
}

fn ordinary_probe(mut command: Command) -> (Option<i32>, bool, String, String) {
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(12);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "Private runtime probe exceeded deadline; provider must kill its whole hierarchy"
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut out = String::new();
    let mut err = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .take(16385)
        .read_to_string(&mut out)
        .unwrap();
    child
        .stderr
        .take()
        .unwrap()
        .take(16385)
        .read_to_string(&mut err)
        .unwrap();
    assert!(out.len() <= 16384 && err.len() <= 16384);
    (status.code(), status.success(), out, err)
}

fn frozen_input(bytes: &[u8], executable: bool) -> File {
    use std::os::fd::FromRawFd;
    assert!(bytes.len() as u64 <= MAX_FILE_BYTES);
    let permission = if executable {
        libc::MFD_EXEC
    } else {
        libc::MFD_NOEXEC_SEAL
    };
    let raw = unsafe {
        libc::memfd_create(
            c"hye-ci-frozen-input".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING | permission,
        )
    };
    assert!(raw >= 0);
    let mut file = unsafe { File::from_raw_fd(raw) };
    file.write_all(bytes).unwrap();
    assert_eq!(
        unsafe { libc::fchmod(raw, if executable { 0o500 } else { 0o400 }) },
        0
    );
    assert_eq!(unsafe { libc::fcntl(raw, libc::F_ADD_SEALS, SEALS) }, 0);
    file.seek(SeekFrom::Start(0)).unwrap();
    file
}

#[test]
fn gated_policy_bytes_and_nonexecutable_mode_remain_sealed() {
    let mut policy = frozen_input(b"fixed policy bytes", false);
    let mut bytes = Vec::new();
    policy.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"fixed policy bytes");
    assert_eq!(policy.metadata().unwrap().mode() & 0o777, 0o400);
    assert_eq!(
        unsafe { libc::fcntl(policy.as_raw_fd(), libc::F_GET_SEALS) } & SEALS,
        SEALS
    );
    assert_eq!(
        policy.write(b"changed").unwrap_err().raw_os_error(),
        Some(libc::EPERM)
    );
    assert_eq!(unsafe { libc::fchmod(policy.as_raw_fd(), 0o500) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));
}

fn gated_probe(
    command: &Command,
    passed: &[BorrowedFd<'_>],
) -> (Option<i32>, bool, String, String) {
    use crate::cgroup::{launch, Delegation, Limits};
    use std::{ffi::CString, os::fd::FromRawFd};
    // Explicit fixture constructor only. Freeze its bytes before handing any
    // execution capability to Plan; no dependency discovery or ldd execution.
    let mut source = File::open(command.get_program()).unwrap();
    let mut bytes = Vec::new();
    Read::by_ref(&mut source)
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(!bytes.is_empty() && bytes.len() as u64 <= MAX_FILE_BYTES);
    let constructor = frozen_input(&bytes, true);
    eprintln!("HYE_GATED_CONSTRUCTOR_SHA256 {:x}", Sha256::digest(&bytes));
    drop((source, bytes));
    fn pipe() -> (File, File) {
        let mut pair = [-1; 2];
        assert_eq!(
            unsafe { libc::pipe2(pair.as_mut_ptr(), libc::O_CLOEXEC) },
            0
        );
        unsafe { (File::from_raw_fd(pair[0]), File::from_raw_fd(pair[1])) }
    }
    fn nonblocking(file: &File) {
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0);
        assert_eq!(
            unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) },
            0
        );
    }
    fn drain(file: &mut File, output: &mut Vec<u8>) -> io::Result<bool> {
        for _ in 0..8 {
            let mut buffer = [0; 4096];
            match file.read(&mut buffer) {
                Ok(0) => return Ok(true),
                Ok(count) => {
                    if output.len() + count > 16384 {
                        return Err(io::Error::other(
                            "Private runtime diagnostic exceeds its limit",
                        ));
                    }
                    output.extend_from_slice(&buffer[..count]);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(false),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(false)
    }
    let (mut stdout, output) = pipe();
    let (mut stderr, error) = pipe();
    nonblocking(&stdout);
    nonblocking(&stderr);
    let null = File::options()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let args: Vec<_> = std::iter::once(CString::new("hye-trusted-ci-bwrap").unwrap())
        .chain(
            command
                .get_args()
                .map(|arg| CString::new(arg.as_encoded_bytes()).unwrap()),
        )
        .collect();
    let plan = launch::Plan::new(
        constructor.as_fd(),
        &args,
        [null.as_fd(), output.as_fd(), error.as_fd()],
        passed,
    )
    .unwrap();
    let root = File::open(
        std::env::var_os("HYE_CGROUP_TEST_ROOT").expect("Explicit private delegation required"),
    )
    .unwrap();
    let delegation = Delegation::from_directory(root.as_fd()).unwrap();
    let domain = delegation.create(Limits::default()).unwrap();
    let mut session = launch::Session::spawn(domain, plan, Duration::from_secs(3)).unwrap();
    drop((output, error));
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let result = (|| -> io::Result<_> {
        session.start(Duration::from_secs(3))?;
        let end = Instant::now() + Duration::from_secs(12);
        loop {
            drain(&mut stdout, &mut out)?;
            drain(&mut stderr, &mut err)?;
            if let Some(outcome) = session.try_wait()? {
                return Ok(outcome);
            }
            if Instant::now() >= end {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Gated runtime fixture exceeded its deadline",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    })();
    // Even a diagnostic/timeout failure must stop the complete owned leaf.
    session.stop(Duration::from_secs(3)).unwrap();
    assert!(
        drain(&mut stdout, &mut out).unwrap() && drain(&mut stderr, &mut err).unwrap(),
        "Private runtime output retained a writer after cleanup"
    );
    let outcome = result.unwrap_or_else(|e| panic!("{e}; stdout={out:?}; stderr={err:?}"));
    eprintln!("HYE_GATED_RUNTIME_CLEANUP child_reaped=true leaf_removed=true diagnostic_eof=true");
    (
        (outcome.signal.is_none()).then_some(outcome.code),
        outcome.signal.is_none() && outcome.code == 0,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}
