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
    let mut policy = super::tests::source(policy);
    policy.seek(SeekFrom::Start(0)).unwrap();

    let abstract_name = format!("hye-probe-{}", std::process::id());
    use std::os::linux::net::SocketAddrExt;
    let address =
        std::os::unix::net::SocketAddr::from_abstract_name(abstract_name.as_bytes()).unwrap();
    let _host_listener = std::os::unix::net::UnixListener::bind_addr(&address).unwrap();
    let host_path =
        std::env::var("HYE_RUNTIME_PROBE_HOST_CANARY").expect("Explicit CI canary required");
    let _host_file = File::open(&host_path).unwrap();
    let mut command = Command::new("/usr/bin/bwrap");
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
            .arg(fd.as_raw_fd().to_string())
            .arg(&spec.destination);
    }
    command
        .args(["--seccomp"])
        .arg(policy.as_raw_fd().to_string())
        .args(["--remount-ro", "/", "--chdir", "/tmp"]);
    for (key, value) in [
        ("HOME", "/home/lab"),
        ("XDG_RUNTIME_DIR", "/run/lab"),
        ("XDG_CONFIG_HOME", "/state/config"),
        ("XDG_DATA_HOME", "/state/data"),
        ("XDG_CACHE_HOME", "/state/cache"),
        ("LC_ALL", "C"),
        ("TMPDIR", "/tmp"),
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
    let descriptors: Vec<_> = snapshots
        .iter()
        .map(AsRawFd::as_raw_fd)
        .chain([policy.as_raw_fd()])
        .collect();
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
    let mut child = command.spawn().unwrap();
    snapshots.clear();
    drop(policy);
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
    eprintln!("HYE_RUNTIME_IMAGE {}\n{out}{err}", image.identity());
    assert!(status.success(), "Private runtime probe failed: {status}");
    assert!(out.contains("HYE_RUNTIME_RESULT all boundary assertions passed"));
}
