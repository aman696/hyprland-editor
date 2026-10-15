//! Explicit file capture has read authority for one held regular inode. It never parses Lua.
//! A timed-out child retains the single-flight lease until its I/O and reaping finish.
use hyprland_editor_core::{source::CapturedSource, source_path, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    os::{fd::AsRawFd, unix::ffi::OsStrExt},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const MAX_REQUEST: usize = 32 * 1024;
const MAX_RESPONSE: usize = 8 * MAX_DOCUMENT_BYTES;
const DEADLINE: Duration = Duration::from_secs(3);
static BUSY: AtomicBool = AtomicBool::new(false);
struct Lease;
impl Lease {
    fn acquire() -> Result<Self, String> {
        BUSY.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| {
                "A source capture is still running or being cleaned up; retry later".into()
            })
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Request {
    Read {
        path: Vec<u8>,
    },
    // Only the debug application used by the private integration fixtures exposes probes.
    #[cfg(debug_assertions)]
    Probe {
        path: Vec<u8>,
        other: Vec<u8>,
    },
    #[cfg(debug_assertions)]
    Stop,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Response {
    Captured {
        source: CapturedSource,
        abi: u32,
    },
    #[cfg(debug_assertions)]
    Probe {
        read_allowed: bool,
        other_denied: bool,
        write_denied: bool,
        network_denied: bool,
        process_denied: bool,
    },
    Error(String),
}

fn io_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}
fn prepare() -> Result<(), String> {
    let expected = std::env::args_os()
        .nth(2)
        .and_then(|s| s.to_str().and_then(|s| s.parse::<libc::pid_t>().ok()))
        .filter(|p| *p > 1)
        .ok_or("Missing capture parent identity")?;
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) } != 0
        || unsafe { libc::getppid() } != expected
    {
        return Err("Capture parent exited or changed".into());
    }
    for (resource, max) in [
        (libc::RLIMIT_AS, 512 * 1024 * 1024),
        (libc::RLIMIT_CPU, 2),
        (libc::RLIMIT_CORE, 0),
        (libc::RLIMIT_STACK, 8 * 1024 * 1024),
        (libc::RLIMIT_NOFILE, 64),
        (libc::RLIMIT_FSIZE, 0),
    ] {
        let limit = libc::rlimit {
            rlim_cur: max,
            rlim_max: max,
        };
        if unsafe { libc::setrlimit(resource, &limit) } != 0 {
            return Err("Cannot install source capture resource limits".into());
        }
    }
    if unsafe { libc::syscall(libc::SYS_close_range, 3_u32, u32::MAX, 0_u32) } != 0 {
        return Err("Cannot close inherited capture capabilities".into());
    }
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err("Cannot disable capture privilege escalation".into());
    }
    Ok(())
}

/// Handle every filesystem right defined through ABI 3, allowing only READ_FILE on the leaf.
/// O_PATH resolution before this point handles metadata only; no configuration bytes are read.
fn restrict_to(node: &File) -> Result<u32, String> {
    #[repr(C)]
    struct Ruleset {
        handled_access_fs: u64,
    }
    #[repr(C, packed)]
    struct PathBeneath {
        allowed_access: u64,
        parent_fd: i32,
    }
    let abi = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<Ruleset>(),
            0,
            1,
        )
    };
    if abi < 3 {
        return Err(
            "Source capture requires enabled Landlock filesystem isolation (ABI 3 or newer)".into(),
        );
    }
    let attr = Ruleset {
        handled_access_fs: (1 << 15) - 1,
    };
    let fd = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            &attr,
            std::mem::size_of::<Ruleset>(),
            0,
        )
    };
    if fd < 0 {
        return Err(io_error("Cannot create capture filesystem policy"));
    }
    use std::os::fd::FromRawFd;
    let ruleset = unsafe { File::from_raw_fd(fd as i32) };
    let path = PathBeneath {
        allowed_access: 1 << 2,
        parent_fd: node.as_raw_fd(),
    };
    if unsafe {
        libc::syscall(
            libc::SYS_landlock_add_rule,
            ruleset.as_raw_fd(),
            1,
            &path,
            0,
        )
    } != 0
    {
        return Err(io_error("Cannot restrict capture to the selected inode"));
    }
    if unsafe { libc::syscall(libc::SYS_landlock_restrict_self, ruleset.as_raw_fd(), 0) } != 0 {
        return Err(io_error("Cannot enforce capture filesystem isolation"));
    }
    Ok(abi as u32)
}

fn selected(path: &[u8]) -> Result<source_path::SelectedSource, String> {
    if path.first() != Some(&b'/') || path.len() > 4096 || path.contains(&0) {
        return Err("Invalid explicit source path".into());
    }
    source_path::resolve_selected(Path::new(std::ffi::OsStr::from_bytes(path)))
}

fn execute(request: Request) -> Result<Response, String> {
    match request {
        Request::Read { path } => {
            let held = selected(&path)?;
            let abi = restrict_to(held.node())?;
            super::process_policy::capture()?;
            held.capture()
                .map(|source| Response::Captured { source, abi })
        }
        #[cfg(debug_assertions)]
        Request::Probe { path, other } => {
            let held = selected(&path)?;
            restrict_to(held.node())?;
            super::process_policy::capture()?;
            let attempt = |path: &[u8], flags, expected| {
                let path = std::ffi::CString::new(path).map_err(|_| "Invalid probe path")?;
                let fd = unsafe {
                    libc::openat(
                        libc::AT_FDCWD,
                        path.as_ptr(),
                        flags | libc::O_CLOEXEC | libc::O_NONBLOCK,
                    )
                };
                let denied =
                    fd < 0 && std::io::Error::last_os_error().raw_os_error() == Some(expected);
                if fd >= 0 {
                    unsafe { libc::close(fd) };
                }
                Ok::<_, String>(denied)
            };
            let other_denied = attempt(&other, libc::O_RDONLY, libc::EACCES)?;
            // No O_TRUNC and no write: a failed policy must not damage even a private fixture.
            let write_denied = attempt(&path, libc::O_WRONLY, libc::EACCES)?;
            let socket = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
            let network_denied =
                socket < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
            if socket >= 0 {
                unsafe { libc::close(socket) };
            }
            // fork is rejected before creating a child. Failure of the policy would fail the test.
            let forked = unsafe { libc::syscall(libc::SYS_clone, libc::SIGCHLD, 0, 0, 0, 0) };
            if forked == 0 {
                unsafe { libc::_exit(91) };
            }
            let process_denied =
                forked < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
            Ok(Response::Probe {
                read_allowed: held.capture().is_ok(),
                other_denied,
                write_denied,
                network_denied,
                process_denied,
            })
        }
        #[cfg(debug_assertions)]
        Request::Stop => {
            unsafe { libc::raise(libc::SIGSTOP) };
            Err("Stopped capture was resumed".into())
        }
    }
}
pub(super) fn entry() {
    let result = (|| {
        prepare()?;
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_REQUEST as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > MAX_REQUEST {
            return Err("Source capture request exceeds 32 KiB".into());
        }
        let request =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid source capture request")?;
        execute(request)
    })()
    .unwrap_or_else(Response::Error);
    if let Ok(bytes) = serde_json::to_vec(&result) {
        if bytes.len() <= MAX_RESPONSE {
            let _ = std::io::stdout().write_all(&bytes);
        }
    }
}

/// The reaper owns the lease; an uninterruptible kernel I/O wait cannot accumulate more children.
/// Killing is best effort. No GUI thread waits for this cleanup or joins an unfinished pipe reader.
fn quarantine(mut child: Child, reader: JoinHandle<()>, writer: JoinHandle<()>, lease: Lease) {
    let _ = child.kill();
    std::thread::spawn(move || {
        let _lease = lease;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                _ => std::thread::sleep(Duration::from_millis(100)),
            }
        }
        let _ = writer.join();
        let _ = reader.join();
    });
}
fn run(request: Request, budget: Duration) -> Result<Response, String> {
    let lease = Lease::acquire()?;
    let bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_REQUEST {
        return Err("Source capture request exceeds 32 KiB".into());
    }
    #[cfg(not(test))]
    let exe = "/proc/self/exe";
    #[cfg(test)]
    let held_test_executable = {
        use std::os::unix::fs::OpenOptionsExt;
        let path = std::env::current_exe().map_err(|e| e.to_string())?;
        let target = path
            .parent()
            .and_then(Path::parent)
            .ok_or("Test executable is outside the Cargo target layout")?;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(target.join("hyprland-editor"))
            .map_err(|e| format!("Build the application before capture tests: {e}"))?
    };
    #[cfg(test)]
    let exe = format!("/proc/self/fd/{}", held_test_executable.as_raw_fd());
    let mut child = Command::new(exe)
        .arg("--capture-source-worker")
        .arg(std::process::id().to_string())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let (writer_tx, writer_rx) = mpsc::channel();
    let writer = std::thread::spawn(move || {
        let _ = writer_tx.send(input.write_all(&bytes));
    });
    let (reader_tx, reader_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .take(MAX_RESPONSE as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = reader_tx.send(result);
    });
    let start = Instant::now();
    let mut output = None;
    let mut written = None;
    let mut status = None;
    loop {
        if written.is_none() {
            written = writer_rx.try_recv().ok();
        }
        if output.is_none() {
            output = reader_rx.try_recv().ok();
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(result) => status = result,
                Err(e) => {
                    quarantine(child, reader, writer, lease);
                    return Err(format!("Cannot inspect capture process: {e}"));
                }
            }
        }
        if let (Some(s), Some(_), Some(_)) = (&status, &written, &output) {
            if !s.success() {
                quarantine(child, reader, writer, lease);
                return Err("Isolated source capture exited unsuccessfully".into());
            }
            break;
        }
        if start.elapsed() >= budget {
            quarantine(child, reader, writer, lease);
            return Err(
                "Source capture exceeded its wall-clock budget; cleanup continues separately"
                    .into(),
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // Both threads sent their results, so joins cannot wait on source I/O.
    let _ = writer.join();
    let _ = reader.join();
    written.unwrap().map_err(|e| e.to_string())?;
    let bytes = output.unwrap().map_err(|e| e.to_string())?;
    if bytes.len() > MAX_RESPONSE {
        return Err("Source capture response exceeds 8 MiB".into());
    }
    let response = serde_json::from_slice(&bytes).map_err(|_| "Invalid source capture response")?;
    match response {
        Response::Error(e) => Err(e),
        other => Ok(other),
    }
}
pub(super) fn read_selected(path: &Path) -> Result<CapturedSource, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let path = absolute.as_os_str().as_bytes().to_vec();
    if path.len() > 4096 || path.contains(&0) {
        return Err("Invalid explicit source path".into());
    }
    let Response::Captured { source, abi } = run(Request::Read { path: path.clone() }, DEADLINE)?
    else {
        return Err("Unexpected source capture response".into());
    };
    if abi < 3 || source.identity.selected_path != path {
        return Err("Source capture does not match the selected file or required isolation".into());
    }
    source.identity.validate(&source.content)?;
    Ok(source)
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;
    #[test]
    fn stopped_capture_returns_and_eventually_releases_its_single_flight_lease() {
        let start = Instant::now();
        let error = run(Request::Stop, Duration::from_millis(150))
            .err()
            .unwrap();
        assert!(error.contains("wall-clock"), "{error}");
        assert!(start.elapsed() < Duration::from_secs(2));
        let limit = Instant::now() + Duration::from_secs(2);
        while BUSY.load(Ordering::Acquire) && Instant::now() < limit {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !BUSY.load(Ordering::Acquire),
            "stopped private child was not reaped"
        );
        let lease = Lease::acquire().unwrap();
        assert!(Lease::acquire().is_err());
        drop(lease);
        let directory = std::env::temp_dir().join(format!(
            "he-capture-parent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("selected.lua");
        let content = "hl.config{general={border_size=2}}";
        std::fs::write(&path, content).unwrap();
        let result = read_selected(&path);
        std::fs::remove_dir_all(&directory).unwrap();
        let captured = result.unwrap();
        assert_eq!(captured.content, content);
        captured.identity.validate(content).unwrap();
    }
}
