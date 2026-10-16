//! Folder grants retain live O_PATH capabilities across helper invocations; paths are evidence.
use super::{capture_worker, fd_channel, process_policy};
use hyprland_editor_core::{
    source::CapturedSource,
    source_scope::{Identity, Scope},
    MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::os::fd::AsRawFd;
use std::{
    fs::File,
    os::{
        fd::FromRawFd,
        unix::{ffi::OsStrExt, net::UnixStream},
    },
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
const MAX_REQUEST: usize = 2 * MAX_DOCUMENT_BYTES;
const MAX_RESPONSE: usize = 8 * MAX_DOCUMENT_BYTES;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Request {
    Approve {
        path: Vec<u8>,
    },
    Read {
        identity: Identity,
        candidate: Vec<u8>,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Response {
    Approved { identity: Identity, abi: u32 },
    Captured { source: CapturedSource, abi: u32 },
    Error(String),
}
pub(super) struct Grant {
    node: File,
    identity: Identity,
}
impl Grant {
    pub(super) fn identity(&self) -> &Identity {
        &self.identity
    }
}
pub(super) fn entry() {
    let mut channel = unsafe { UnixStream::from_raw_fd(0) };
    let result = (|| {
        capture_worker::prepare()?;
        let message = fd_channel::receive(&mut channel, MAX_REQUEST)?;
        let request: Request =
            serde_json::from_slice(&message.bytes).map_err(|_| "Invalid folder capture request")?;
        match request {
            Request::Approve { path } => {
                if message.descriptor.is_some()
                    || path.first() != Some(&b'/')
                    || path.len() > 4096
                    || path.contains(&0)
                {
                    return Err("Invalid explicit folder approval".into());
                }
                let scope = Scope::select(Path::new(std::ffi::OsStr::from_bytes(&path)))?;
                // No source bytes are read. After this point even filesystem opens are denied.
                let abi = capture_worker::restrict_to(scope.node())?;
                process_policy::capability_reply(false)?;
                let (descriptor, identity) = scope.into_parts();
                Ok((Response::Approved { identity, abi }, Some(descriptor)))
            }
            Request::Read {
                identity,
                candidate,
            } => {
                let node = message
                    .descriptor
                    .ok_or("Folder capture requires a live approved descriptor")?;
                let scope = Scope::from_descriptor(node, identity)?;
                let held = scope.resolve(&candidate)?;
                // The child reads only this contained leaf, not the whole granted folder.
                let abi = capture_worker::restrict_to(held.node())?;
                process_policy::capability_reply(true)?;
                let source = held.capture()?;
                scope.validate()?;
                Ok((Response::Captured { source, abi }, None))
            }
        }
    })()
    .unwrap_or_else(|error| (Response::Error(error), None));
    if let Ok(bytes) = serde_json::to_vec(&result.0) {
        if bytes.len() <= MAX_RESPONSE {
            let _ = fd_channel::send(&mut channel, &bytes, result.1.as_ref());
        }
    }
}
fn run(request: Request, descriptor: Option<File>) -> Result<(Response, Option<File>), String> {
    let lease = capture_worker::Lease::acquire()?;
    let bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_REQUEST {
        return Err("Folder request exceeds its byte budget".into());
    }
    let (mut channel, child_channel) = UnixStream::pair().map_err(|e| e.to_string())?;
    let mut input = channel.try_clone().map_err(|e| e.to_string())?;
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
            .map_err(|e| format!("Build the application before folder capture tests: {e}"))?
    };
    #[cfg(test)]
    let exe = format!("/proc/self/fd/{}", held_test_executable.as_raw_fd());
    let mut child = Command::new(exe)
        .arg("--directory-source-worker")
        .arg(std::process::id().to_string())
        .env_clear()
        .stdin(Stdio::from(std::os::fd::OwnedFd::from(child_channel)))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let (input_tx, input_rx) = mpsc::channel();
    let writer = std::thread::spawn(move || {
        let _ = input_tx.send(fd_channel::send(&mut input, &bytes, descriptor.as_ref()));
    });
    let (output_tx, output_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let _ = output_tx.send(fd_channel::receive(&mut channel, MAX_RESPONSE));
    });
    let start = Instant::now();
    let mut status = None;
    let mut sent = None;
    let mut received = None;
    loop {
        if sent.is_none() {
            sent = input_rx.try_recv().ok();
        }
        if received.is_none() {
            received = output_rx.try_recv().ok();
        }
        if let Some(Err(error)) = &sent {
            let error = error.clone();
            capture_worker::quarantine(child, reader, writer, lease);
            return Err(format!("Folder request transport failed: {error}"));
        }
        if let Some(Err(error)) = &received {
            let error = error.clone();
            capture_worker::quarantine(child, reader, writer, lease);
            return Err(format!("Folder response transport failed: {error}"));
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(result) => status = result,
                Err(e) => {
                    capture_worker::quarantine(child, reader, writer, lease);
                    return Err(e.to_string());
                }
            }
        }
        if status.is_some() && sent.is_some() && received.is_some() {
            break;
        }
        if start.elapsed() >= Duration::from_secs(3) {
            capture_worker::quarantine(child, reader, writer, lease);
            return Err(
                "Folder capture exceeded its response budget; cleanup continues separately".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let _ = writer.join();
    let _ = reader.join();
    if !status.unwrap().success() {
        return Err("Isolated folder capture exited unsuccessfully".into());
    }
    sent.unwrap()?;
    let message = received.unwrap()?;
    let response =
        serde_json::from_slice(&message.bytes).map_err(|_| "Invalid folder capture response")?;
    match response {
        Response::Error(e) => Err(e),
        other => Ok((other, message.descriptor)),
    }
}
pub(super) fn approve(path: &Path) -> Result<Grant, String> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let path = path.as_os_str().as_bytes().to_vec();
    if path.len() > 4096 || path.contains(&0) {
        return Err("Invalid explicit folder selection".into());
    }
    let (Response::Approved { identity, abi }, Some(node)) =
        run(Request::Approve { path: path.clone() }, None)?
    else {
        return Err("Folder approval did not return a live capability".into());
    };
    if abi < 3 || identity.selected_path != path {
        return Err(
            "Folder approval does not match the selected path or required isolation".into(),
        );
    }
    // The trusted helper checked descriptor type/identity. Keep filesystem metadata operations
    // in that bounded process; every read reconstructs and verifies its Scope there again.
    identity.validate()?;
    Ok(Grant { node, identity })
}
pub(super) fn read(grant: &Grant, candidate: &[u8]) -> Result<CapturedSource, String> {
    let expected_path = grant.identity.candidate_path(candidate)?;
    if candidate.is_empty() || candidate.len() > 4096 || candidate.contains(&0) {
        return Err("Invalid dependency candidate".into());
    }
    let node = grant.node.try_clone().map_err(|e| e.to_string())?;
    let (Response::Captured { source, abi }, None) = run(
        Request::Read {
            identity: grant.identity().clone(),
            candidate: candidate.to_vec(),
        },
        Some(node),
    )?
    else {
        return Err("Unexpected folder capture response".into());
    };
    if abi < 3 || source.identity.selected_path != expected_path {
        return Err("Folder capture lacks required isolation".into());
    }
    source.identity.validate(&source.content)?;
    Ok(source)
}
/// Explicit headless fixture/diagnostic entry. It never grants authority from saved source paths.
pub(super) fn check(folder: &Path, candidate: &[u8]) -> Result<serde_json::Value, String> {
    let grant = approve(folder)?;
    let source = read(&grant, candidate)?;
    Ok(
        serde_json::json!({"scope":"approved-folder-capture-only","folder":grant.identity(),"source_identity":source.identity,"files_written":false,"lua_executed":false,"runtime_verified":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_capability_transport_retains_approved_folder_and_refuses_external_targets() {
        let _test = capture_worker::TEST_LOCK.lock().unwrap();
        let path = std::env::temp_dir().join(format!("he-folder-parent-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        let root = path.join("approved");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("config.lua"), "-- approved inode\n").unwrap();
        std::fs::write(path.join("outside.lua"), "-- outside\n").unwrap();
        std::os::unix::fs::symlink("../outside.lua", root.join("outside.lua")).unwrap();
        let grant = approve(&root).unwrap();
        let fd = grant.node.as_raw_fd();
        assert_ne!(
            unsafe { libc::fcntl(fd, libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
        assert_eq!(
            read(&grant, b"config.lua").unwrap().content,
            "-- approved inode\n"
        );
        assert!(read(&grant, b"outside.lua")
            .unwrap_err()
            .contains("escapes"));
        std::fs::rename(&root, path.join("moved")).unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("config.lua"), "-- replacement\n").unwrap();
        assert_eq!(
            read(&grant, b"config.lua").unwrap().content,
            "-- approved inode\n"
        );
        use std::os::unix::fs::PermissionsExt;
        let moved = path.join("moved");
        std::fs::set_permissions(&moved, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(read(&grant, b"config.lua").unwrap_err().contains("changed"));
        std::fs::set_permissions(
            &moved,
            std::fs::Permissions::from_mode(grant.identity.mode & 0o7777),
        )
        .unwrap();
        let identity = grant.identity().clone();
        let root_identity = (identity.device, identity.inode);
        let result = run(
            Request::Read {
                identity,
                candidate: b"config.lua".to_vec(),
            },
            None,
        );
        assert!(result.err().unwrap().contains("live approved descriptor"));
        drop(grant);
        // Concurrent tests may reuse a just-closed FD number. It must no longer own this root.
        let mut now = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe { libc::fstat(fd, now.as_mut_ptr()) } == 0 {
            let now = unsafe { now.assume_init() };
            assert_ne!((now.st_dev, now.st_ino), root_identity);
        }
        std::fs::remove_dir_all(path).unwrap();
    }
}
