//! One-shot typed projection staging. No compositor or executable source is run.
use hyprland_editor_core::{storage, workspace::Workspace, MAX_DOCUMENT_BYTES};
use hyprland_editor_projection::{
    self as projection,
    wire::{Artifact, Expected},
    Scenario,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    os::fd::AsRawFd,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
const MAX_REQUEST: usize = 10 * MAX_DOCUMENT_BYTES;
const MAX_RESPONSE: usize = projection::wire::MAX_RESULT_BYTES;
const DEADLINE: Duration = Duration::from_secs(3);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    workspace: Workspace,
    scenario: Scenario,
}
#[derive(Serialize, Deserialize)]
enum Response {
    Staged(Box<Artifact>),
    Error(String),
}

/// A live process-local signal. Saved data cannot recreate cancellation authority.
#[derive(Clone, Default)]
pub(super) struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    fn check(&self) -> Result<(), String> {
        if self.0.load(Ordering::Acquire) {
            Err("Projection preparation cancelled".into())
        } else {
            Ok(())
        }
    }
}
fn isolate() -> Result<(), String> {
    for (resource, max) in [
        (libc::RLIMIT_AS, 512 * 1024 * 1024),
        (libc::RLIMIT_CPU, 2),
        (libc::RLIMIT_CORE, 0),
        (libc::RLIMIT_STACK, 8 * 1024 * 1024),
    ] {
        let limit = libc::rlimit {
            rlim_cur: max,
            rlim_max: max,
        };
        if unsafe { libc::setrlimit(resource, &limit) } != 0 {
            return Err("Cannot install projection resource limits".into());
        }
    }
    if unsafe { libc::syscall(libc::SYS_close_range, 3_u32, u32::MAX, 0_u32) } != 0 {
        return Err("Cannot close inherited projection capabilities".into());
    }
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err("Cannot disable projection privilege escalation".into());
    }
    super::process_policy::parser()
}
/// Fixed read-only diagnostic for this exact role. No filenames/syscall choices
/// come from the caller. Denial is established before any attempted operation.
pub(super) fn policy_probe() {
    let result = (|| -> Result<serde_json::Value, String> {
        isolate()?;
        let file = unsafe { libc::open(c"/etc/passwd".as_ptr(), libc::O_RDONLY) };
        let file_denied =
            file < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
        if file >= 0 {
            unsafe {
                libc::close(file);
            }
        }
        let network = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
        let network_denied =
            network < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
        if network >= 0 {
            unsafe {
                libc::close(network);
            }
        }
        let process = unsafe { libc::fork() };
        if process == 0 {
            unsafe {
                libc::_exit(0);
            }
        }
        let process_denied =
            process < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
        let arguments = [c"true".as_ptr(), std::ptr::null()];
        let environment = [std::ptr::null::<libc::c_char>()];
        let executed = unsafe {
            libc::execve(
                c"/bin/true".as_ptr(),
                arguments.as_ptr(),
                environment.as_ptr(),
            )
        };
        let execution_denied =
            executed < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
        let mut byte = 0_u8;
        let inherited = unsafe { libc::read(60, (&mut byte as *mut u8).cast(), 1) };
        let inherited_fd_closed =
            inherited < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EBADF);
        Ok(
            serde_json::json!({"file_denied":file_denied,"network_denied":network_denied,"process_denied":process_denied,"execution_denied":execution_denied,"inherited_fd_closed":inherited_fd_closed}),
        )
    })();
    let result = result.unwrap_or_else(|error| serde_json::json!({"error":error}));
    let _ = std::io::stdout().write_all(result.to_string().as_bytes());
}
pub(super) fn entry() {
    let response = (|| -> Result<Response, String> {
        isolate()?;
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_REQUEST as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read projection input")?;
        if bytes.len() > MAX_REQUEST {
            return Err("Projection request exceeds 10 MiB".into());
        }
        let request: Request =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid projection request")?;
        drop(bytes);
        projection::compile(&request.workspace, &request.scenario)
            .map(|result| Response::Staged(Box::new(Artifact::from(result))))
    })()
    .unwrap_or_else(|error| Response::Error(error.chars().take(1024).collect()));
    let bytes = serde_json::to_vec(&response)
        .unwrap_or_else(|_| b"{\"Error\":\"Cannot encode projection result\"}".to_vec());
    if bytes.len() > MAX_RESPONSE {
        let _ =
            std::io::stdout().write_all(b"{\"Error\":\"Projection response exceeds its limit\"}");
    } else {
        let _ = std::io::stdout().write_all(&bytes);
    }
}
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn nonblocking(fd: i32) -> Result<(), String> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        Err("Cannot make projection pipe nonblocking".into())
    } else {
        Ok(())
    }
}
fn executable() -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt;
    #[cfg(not(test))]
    let path = PathBuf::from("/proc/self/exe");
    #[cfg(test)]
    let path = {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        exe.parent()
            .and_then(std::path::Path::parent)
            .ok_or("Unexpected Cargo test layout")?
            .join("hyprland-editor")
    };
    // /proc/self/exe is the running inode; Cargo tests explicitly hold the built
    // normal app rather than asking the Rust harness to handle a role argument.
    let flags = if cfg!(test) { libc::O_NOFOLLOW } else { 0 };
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
        .map_err(|e| e.to_string())
}
fn exchange(
    bytes: &[u8],
    cancellation: &Cancellation,
    deadline: Duration,
    #[cfg(test)] started: Option<std::sync::mpsc::Sender<u32>>,
) -> Result<Vec<u8>, String> {
    cancellation.check()?;
    let executable = executable()?;
    let mut child = OwnedChild(
        Command::new(format!("/proc/self/fd/{}", executable.as_raw_fd()))
            .arg("--projection-worker")
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?,
    );
    #[cfg(test)]
    if let Some(started) = started {
        let _ = started.send(child.0.id());
    }
    let mut input = child.0.stdin.take();
    let mut output = child
        .0
        .stdout
        .take()
        .ok_or("Missing projection output pipe")?;
    nonblocking(
        input
            .as_ref()
            .ok_or("Missing projection input pipe")?
            .as_raw_fd(),
    )?;
    nonblocking(output.as_raw_fd())?;
    let start = Instant::now();
    let mut written = 0;
    let mut response = Vec::new();
    let mut eof = false;
    let mut status = None;
    loop {
        cancellation.check()?;
        if start.elapsed() >= deadline {
            return Err("Projection exceeded its wall-clock budget".into());
        }
        if let Some(pipe) = input.as_mut() {
            match pipe.write(&bytes[written..]) {
                Ok(0) if written != bytes.len() => {
                    return Err("Projection input pipe stopped accepting bytes".into())
                }
                Ok(count) => {
                    written += count;
                    if written == bytes.len() {
                        input.take();
                    }
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                    input.take();
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        // Bound each iteration so a busy sender cannot starve cancellation or
        // the deadline. One staging child cannot create descendants after setup.
        for _ in 0..16 {
            let mut buffer = [0_u8; 8192];
            match output.read(&mut buffer) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(count) => {
                    if response.len().saturating_add(count) > MAX_RESPONSE {
                        return Err("Projection response exceeds 12 MiB".into());
                    }
                    response.extend_from_slice(&buffer[..count]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        if status.is_none() {
            status = child.0.try_wait().map_err(|e| e.to_string())?;
        }
        if let Some(status) = status {
            if eof {
                if !status.success() {
                    return Err("Isolated projection worker exited unsuccessfully".into());
                }
                return Ok(response);
            }
        }
        let mut fds = [
            libc::pollfd {
                fd: if eof { -1 } else { output.as_raw_fd() },
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: input.as_ref().map_or(-1, AsRawFd::as_raw_fd),
                events: libc::POLLOUT,
                revents: 0,
            },
        ];
        if unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, 10) } < 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
        {
            return Err("Projection pipe polling failed".into());
        }
    }
}
pub(super) fn stage(
    workspace: Workspace,
    scenario: Scenario,
    cancellation: Cancellation,
) -> Result<Artifact, String> {
    cancellation.check()?;
    let expected = Expected::new(&workspace, &scenario)?;
    cancellation.check()?;
    let bytes = serde_json::to_vec(&Request {
        workspace,
        scenario,
    })
    .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_REQUEST {
        return Err("Projection request exceeds 10 MiB".into());
    }
    let response = exchange(
        &bytes,
        &cancellation,
        DEADLINE,
        #[cfg(test)]
        None,
    )?;
    cancellation.check()?;
    match serde_json::from_slice::<Response>(&response)
        .map_err(|_| "Invalid projection response")?
    {
        Response::Error(error) => Err(error.chars().take(1024).collect()),
        Response::Staged(artifact) => {
            artifact.verify(&expected)?;
            cancellation.check()?;
            Ok(*artifact)
        }
    }
}
pub(super) fn check(workspace: PathBuf, scenario: PathBuf) -> Result<serde_json::Value, String> {
    let (text, _) = storage::read_bounded(
        &workspace,
        hyprland_editor_core::workspace::MAX_WORKSPACE_BYTES,
    )?;
    let workspace = Workspace::from_json(&text, &hyprland_editor_core::catalogue())?;
    let (text, _) = storage::read_bounded(&scenario, 64 * 1024)?;
    let scenario = serde_json::from_str(&text).map_err(|_| "Invalid private scenario")?;
    let result = stage(workspace, scenario, Cancellation::default())?;
    Ok(
        serde_json::json!({"scope":"restricted-typed-draft-projection-only", "artifact":result, "runtime_verified":false, "compositor_started":false,"saved_file_written":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_staging_never_reaches_the_launcher() {
        let cancellation = Cancellation::default();
        cancellation.cancel();
        assert!(stage(
            Workspace::new(hyprland_editor_core::Draft::new(
                hyprland_editor_core::BASELINE
            )),
            Scenario::default(),
            cancellation
        )
        .unwrap_err()
        .contains("cancelled"));
    }
    #[test]
    fn active_staging_cancellation_and_timeout_reap_the_owned_child() {
        for cancel in [true, false] {
            let cancellation = Cancellation::default();
            let stop = cancellation.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            let task = std::thread::spawn(move || {
                exchange(
                    &vec![b' '; MAX_REQUEST],
                    &cancellation,
                    if cancel {
                        DEADLINE
                    } else {
                        Duration::from_millis(1)
                    },
                    Some(tx),
                )
            });
            let pid = rx.recv_timeout(Duration::from_secs(2)).unwrap();
            if cancel {
                stop.cancel();
            }
            let error = task.join().unwrap().unwrap_err();
            assert!(
                error.contains(if cancel { "cancelled" } else { "wall-clock" }),
                "{error}"
            );
            let mut status = 0;
            assert_eq!(
                unsafe { libc::waitpid(pid as _, &mut status, libc::WNOHANG) },
                -1
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
        }
    }
}
