//! Opt-in kernel checks with an explicitly supplied private delegation.
//! No host node is auto-selected and no parent controller is enabled here.
use super::*;
use std::{
    io::{Read, Write},
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Child, Command, Stdio},
};
const TEST: &str =
    "cgroup::kernel_tests::delegated_kernel_limits_enforce_and_teardown_whole_subtrees";
const ROOT: &str = "HYE_CGROUP_TEST_ROOT";
const MODE: &str = "HYE_CGROUP_TEST_WORKER";
const TOKEN: &str = "HYE_CGROUP_TEST_TOKEN_FD";
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn spawn(domain: &Domain, mode: &str) -> Process {
    let join = domain
        .join
        .as_ref()
        .expect("Prepared membership control")
        .as_raw_fd();
    let token = dup(domain.fd.as_fd()).unwrap();
    let token_fd = token.as_raw_fd();
    let mut command = Command::new("/proc/self/exe");
    command
        .args([
            "--ignored",
            "--exact",
            TEST,
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env("LC_ALL", "C")
        .env(MODE, mode)
        .env(TOKEN, token_fd.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // SAFETY: after fork use one kernel write and errno construction only. The
    // trusted test executable cannot start if self-placement fails. This is
    // test-only placement, not a production compositor launch certificate.
    unsafe {
        command.pre_exec(move || {
            let count = libc::write(join, b"0".as_ptr().cast(), 1);
            if count == 1 {
                if libc::fcntl(token_fd, libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            } else if count < 0 {
                Err(io::Error::last_os_error())
            } else {
                Err(io::Error::from_raw_os_error(libc::EIO))
            }
        });
    }
    let mut child = Process(command.spawn().unwrap());
    drop(token);
    let pid = child.0.id().to_string();
    assert!(
        read(domain.fd.as_fd(), c"cgroup.procs")
            .unwrap()
            .lines()
            .any(|line| line == pid),
        "Trusted worker did not join its intended leaf"
    );
    child.0.stdin.take().unwrap().write_all(b"G").unwrap();
    child
}
fn exited(child: &mut Process) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            if let Some(stderr) = child.0.stderr.take() {
                let mut bytes = Vec::new();
                stderr.take(8193).read_to_end(&mut bytes).unwrap();
                assert!(
                    bytes.len() <= 8192,
                    "Private worker diagnostic exceeds its bound"
                );
                if !bytes.is_empty() {
                    eprintln!(
                        "Private worker diagnostic: {}",
                        String::from_utf8_lossy(&bytes)
                    );
                }
            }
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "Private resource worker exceeded its deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn counter(domain: &Domain, file: &CStr, key: &str) -> u64 {
    let text = read(domain.fd.as_fd(), file).unwrap();
    let values = text
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(name, _)| *name == key)
        .map(|(_, value)| value.parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        values.len(),
        1,
        "Missing or duplicate kernel resource counter"
    );
    values[0]
}
fn marker(child: &mut Process, expected: &[u8]) {
    let stdout = child.0.stdout.as_mut().unwrap();
    // SAFETY: scalar flags of this private pipe; reads stay nonblocking/bounded.
    let flags = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_GETFL) };
    assert!(flags >= 0);
    assert_eq!(
        unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) },
        0
    );
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut output = Vec::new();
    loop {
        let mut bytes = [0; 512];
        match stdout.read(&mut bytes) {
            Ok(0) => panic!("Private PID worker exited before observing its limit"),
            Ok(count) => {
                output.extend_from_slice(&bytes[..count]);
                assert!(output.len() <= 8192);
                if output.windows(expected.len()).any(|v| v == expected) {
                    return;
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(e) => panic!("Cannot read private worker result: {e}"),
        }
        assert!(
            Instant::now() < deadline,
            "Private PID worker did not observe its limit"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn worker(mode: &str) {
    let raw = std::env::var(TOKEN)
        .expect("Explicit inherited resource token required")
        .parse::<i32>()
        .unwrap();
    assert!(raw >= 0);
    // SAFETY: fcntl safely validates the supplied number and creates fresh owned
    // authority; environment text alone cannot become an OwnedFd capability.
    let copied = unsafe { libc::fcntl(raw, libc::F_DUPFD_CLOEXEC, 3) };
    assert!(copied >= 0);
    // SAFETY: successful duplication returned a new descriptor owned here.
    let token = unsafe { OwnedFd::from_raw_fd(copied) };
    validate_root(token.as_fd()).unwrap();
    for (name, expected) in [
        (c"memory.max", "536870912"),
        (c"memory.swap.max", "0"),
        (c"memory.oom.group", "1"),
        (c"pids.max", "64"),
        (c"cpu.max", "100000 100000"),
    ] {
        assert_eq!(
            read(token.as_fd(), name).unwrap().trim(),
            expected,
            "Worker lacks its actual resource profile"
        );
    }
    let procs = read(token.as_fd(), c"cgroup.procs").unwrap();
    assert_eq!(
        procs.lines().collect::<Vec<_>>(),
        [std::process::id().to_string().as_str()],
        "Resource worker must be the sole process in its own leaf"
    );
    let mut go = [0; 1];
    std::io::stdin().read_exact(&mut go).unwrap();
    assert_eq!(go, [b'G']);
    match mode {
        "memory" => {
            let mut held = Vec::new();
            for _ in 0..128 {
                let mut chunk = vec![1u8; 8 * 1024 * 1024];
                std::hint::black_box(&mut chunk);
                held.push(chunk);
            }
            std::hint::black_box(held);
            panic!("Private memory worker exceeded twice its limit without kernel termination");
        }
        "pids" => {
            let mut children = Vec::new();
            for _ in 0..128 {
                // SAFETY: fork children execute only pause, allocate no Rust
                // state and are killed through the dedicated cgroup subtree.
                let pid = unsafe { libc::fork() };
                if pid == 0 {
                    loop {
                        unsafe {
                            libc::pause();
                        }
                    }
                }
                if pid < 0 {
                    assert_eq!(
                        io::Error::last_os_error().raw_os_error(),
                        Some(libc::EAGAIN)
                    );
                    println!("HYE_PIDS_LIMITED");
                    std::io::stdout().flush().unwrap();
                    loop {
                        unsafe {
                            libc::pause();
                        }
                    }
                }
                children.push(pid);
            }
            panic!("Private task limit did not refuse excess children");
        }
        "cpu" => {
            let until = Instant::now() + Duration::from_secs(3);
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    scope.spawn(move || {
                        let mut value = 1u64;
                        while Instant::now() < until {
                            value = value.wrapping_mul(6364136223846793005).wrapping_add(1);
                            std::hint::black_box(value);
                        }
                    });
                }
            });
        }
        _ => panic!("Unknown private worker mode"),
    }
}
#[test]
#[ignore = "requires an explicitly supplied dedicated cgroup v2 provider; never auto-select a host node"]
fn delegated_kernel_limits_enforce_and_teardown_whole_subtrees() {
    // SAFETY: fixed credential/syscall queries, no pointer state is retained.
    assert!(unsafe { libc::getuid() } > 0);
    assert_eq!(unsafe { libc::getuid() }, unsafe { libc::geteuid() });
    assert_eq!(unsafe { libc::getgid() }, unsafe { libc::getegid() });
    assert_eq!(unsafe { libc::getgroups(0, std::ptr::null_mut()) }, 0);
    assert_eq!(
        unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) },
        1
    );
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    for field in ["CapPrm:", "CapEff:", "CapBnd:", "CapAmb:"] {
        let value = status
            .lines()
            .find_map(|line| line.strip_prefix(field))
            .expect("Missing kernel privilege field")
            .trim();
        assert_eq!(
            u64::from_str_radix(value, 16).unwrap(),
            0,
            "Resource fixture retained privilege"
        );
    }
    if let Ok(mode) = std::env::var(MODE) {
        worker(&mode);
        return;
    }
    let path = std::env::var_os(ROOT).expect("Dedicated CI/test delegation must be explicit");
    use std::os::unix::fs::OpenOptionsExt;
    let directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .unwrap();
    let delegation = Delegation::from_directory(directory.as_fd()).unwrap();
    assert!(delegation.observe().unwrap().missing.is_empty());
    assert!(
        std::thread::available_parallelism().unwrap().get() >= 2,
        "The default one-core CPU fixture requires at least two available CPUs"
    );
    for mode in ["cpu", "memory", "pids"] {
        let domain = delegation.create(Limits::default()).unwrap();
        let name = domain.name.clone();
        let started = Instant::now();
        let mut child = spawn(&domain, mode);
        let counters = match mode {
            "cpu" => {
                assert!(exited(&mut child).success());
                assert!(counter(&domain, c"cpu.stat", "usage_usec") > 0);
                assert!(
                    counter(&domain, c"cpu.stat", "nr_throttled") > 0,
                    "Default CPU quota was not observed throttling four workers"
                );
                serde_json::json!({"usage_usec":counter(&domain,c"cpu.stat","usage_usec"),"nr_throttled":counter(&domain,c"cpu.stat","nr_throttled")})
            }
            "memory" => {
                assert_eq!(exited(&mut child).signal(), Some(libc::SIGKILL));
                assert!(
                    counter(&domain, c"memory.events", "oom") > 0,
                    "Leaf memory limit did not invoke its own OOM handling"
                );
                assert!(
                    counter(&domain, c"memory.events", "oom_kill") > 0,
                    "Memory limit did not produce a kernel OOM kill"
                );
                serde_json::json!({"oom":counter(&domain,c"memory.events","oom"),"oom_kill":counter(&domain,c"memory.events","oom_kill")})
            }
            _ => {
                marker(&mut child, b"HYE_PIDS_LIMITED");
                assert!(counter(&domain, c"pids.events", "max") > 0);
                let current = read(domain.fd.as_fd(), c"pids.current")
                    .unwrap()
                    .trim()
                    .parse::<u64>()
                    .unwrap();
                assert!(current > 1 && current <= 64);
                assert!(
                    read(domain.fd.as_fd(), c"cgroup.procs")
                        .unwrap()
                        .lines()
                        .count()
                        > 1
                );
                serde_json::json!({"max_events":counter(&domain,c"pids.events","max"),"current_tasks":current,"processes":read(domain.fd.as_fd(),c"cgroup.procs").unwrap().lines().count()})
            }
        };
        domain.cleanup(Duration::from_secs(5)).unwrap();
        if mode == "pids" {
            assert_eq!(exited(&mut child).signal(), Some(libc::SIGKILL));
        }
        let error = open(directory.as_fd(), &name, libc::O_RDONLY | libc::O_DIRECTORY).unwrap_err();
        assert!(
            matches!(error,Error::Io {error,..} if error.raw_os_error()==Some(libc::ENOENT)),
            "Expected explicit kernel absence after cleanup"
        );
        println!(
            "HYE_KERNEL_RESULT {}",
            serde_json::json!({"mode":mode,"profile":{"memory_max":536870912u64,"swap_max":0,"pids_max":64,"cpu_max":"100000 100000"},"counters":counters,"elapsed_ms":started.elapsed().as_millis() as u64,"empty_and_removed":true,"compositor_verified":false})
        );
    }
}
