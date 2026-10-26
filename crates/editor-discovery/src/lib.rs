//! Session-pinned, bounded, read-only Unix IPC. No arbitrary request string is exposed.
use hyprland_editor_core::{Setting, Value, AUDITED_MAIN, BASELINE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    mem,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{FileTypeExt, MetadataExt},
            net::UnixStream,
        },
    },
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const MAX_REPLY: usize = 2 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);
const BASE_COMMIT: &str = "efb50993780079460b0cbed1363e2166a2de1d9f";
const MAIN_COMMIT: &str = "6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7";
#[derive(Clone, Debug, Serialize)]
pub struct SessionIdentity {
    pub signature: String,
    pub pid: u32,
    pub uid: u32,
    pub start_ticks: u64,
    pub boot_id: String,
    pub binary_sha256: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Observation {
    pub current: Option<Value>,
    pub raw_current: serde_json::Value,
    pub default: Option<Value>,
    pub description: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub identity: SessionIdentity,
    pub commit: String,
    pub version: Option<String>,
    pub sampled_unix_ms: u128,
    pub duration_ms: u128,
    pub values: BTreeMap<String, Observation>,
    pub unknown_settings: usize,
}
/// Read-only output observations. Missing fields stay unknown rather than defaults.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Monitor {
    pub name: String,
    pub description: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(rename = "refreshRate")]
    pub refresh_rate: Option<f64>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub scale: Option<f64>,
    pub disabled: Option<bool>,
    #[serde(rename = "availableModes")]
    pub available_modes: Option<Vec<String>>,
}
#[derive(Clone, Debug, Serialize)]
pub struct MonitorSnapshot {
    pub identity: SessionIdentity,
    pub commit: String,
    pub version: Option<String>,
    pub sampled_unix_ms: u128,
    pub duration_ms: u128,
    pub monitors: Vec<Monitor>,
}
#[derive(Deserialize)]
struct VersionReply {
    commit: String,
}
#[derive(Deserialize)]
struct DescriptionReply {
    name: String,
    #[serde(default)]
    description: String,
    current: serde_json::Value,
    default: serde_json::Value,
}
#[derive(Clone, Copy)]
enum Query {
    Version,
    Descriptions,
    Monitors,
}
impl Query {
    fn wire(self) -> &'static [u8] {
        match self {
            Self::Version => b"j/version",
            Self::Descriptions => b"j/descriptions",
            Self::Monitors => b"j/monitors all",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
struct SocketId {
    dev: u64,
    ino: u64,
}
struct ReadSession {
    path: PathBuf,
    socket: SocketId,
    // Keep the filesystem inode allocated even if the socket is unlinked.
    // A bare dev/inode tuple can be reused immediately on some filesystems.
    _socket_pin: fs::File,
    process_pin: fs::File,
    identity: SessionIdentity,
    executable: (u64, u64, u64, i64, i64),
}
fn error() -> String {
    std::io::Error::last_os_error().to_string()
}
fn owned_dir(path: &Path) -> Result<(), String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
        return Err(
            "Runtime directory must be owned by you, without symlinks or other-user writes".into(),
        );
    }
    Ok(())
}
fn socket_id(path: &Path) -> Result<SocketId, String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.file_type().is_socket() || m.uid() != unsafe { libc::geteuid() } {
        return Err("Not an owned compositor socket".into());
    }
    Ok(SocketId {
        dev: m.dev(),
        ino: m.ino(),
    })
}
fn pin_socket(path: &Path) -> Result<(fs::File, SocketId), String> {
    let path =
        std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| "Invalid socket path")?;
    // O_PATH holds the filesystem node without connecting to the compositor.
    // SAFETY: path is NUL terminated and File exclusively owns a valid returned fd.
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(error());
    }
    let file = unsafe { fs::File::from_raw_fd(fd) };
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
        return Err("Not an owned compositor socket".into());
    }
    let identity = SocketId {
        dev: meta.dev(),
        ino: meta.ino(),
    };
    Ok((file, identity))
}
fn start_ticks(pid: u32) -> Result<u64, String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|_| "Compositor process is not visible in this environment")?;
    let tail = stat.rsplit_once(')').ok_or("Invalid process identity")?.1;
    tail.split_whitespace()
        .nth(19)
        .ok_or("Missing process start time")?
        .parse()
        .map_err(|_| "Invalid process start time".into())
}
fn peer(stream: &UnixStream) -> Result<libc::ucred, String> {
    let mut cred: libc::ucred = unsafe { mem::zeroed() };
    let mut size = mem::size_of::<libc::ucred>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut size,
        )
    };
    if rc < 0 || size as usize != mem::size_of::<libc::ucred>() {
        return Err("Cannot verify Unix peer credentials".into());
    }
    Ok(cred)
}
fn ready(stream: &UnixStream, event: i16, deadline: Instant) -> Result<(), String> {
    loop {
        let left = deadline
            .checked_duration_since(Instant::now())
            .ok_or("Read-only request timed out")?;
        let mut fd = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: event,
            revents: 0,
        };
        let rc = unsafe {
            libc::poll(
                &mut fd,
                1,
                left.as_millis().max(1).min(i32::MAX as u128) as i32,
            )
        };
        if rc < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error());
        }
        if rc == 0 {
            return Err("Read-only request timed out".into());
        }
        if fd.revents & libc::POLLNVAL != 0 {
            return Err("Compositor socket closed".into());
        }
        return Ok(());
    }
}
fn connect(path: &Path, deadline: Instant) -> Result<UnixStream, String> {
    let bytes = path.as_os_str().as_bytes();
    let mut address: libc::sockaddr_un = unsafe { mem::zeroed() };
    if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
        return Err("Compositor socket path is too long or invalid".into());
    }
    address.sun_family = libc::AF_UNIX as _;
    for (i, b) in bytes.iter().enumerate() {
        address.sun_path[i] = *b as _;
    }
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(error());
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    let rc = unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_un).cast(),
            mem::size_of::<libc::sockaddr_un>() as _,
        )
    };
    if rc < 0 {
        let errno = std::io::Error::last_os_error().raw_os_error();
        if ![Some(libc::EINPROGRESS), Some(libc::EAGAIN)].contains(&errno) {
            return Err(error());
        }
        ready(&stream, libc::POLLOUT, deadline)?;
        let mut value: libc::c_int = 0;
        let mut size = mem::size_of_val(&value) as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                (&mut value as *mut libc::c_int).cast(),
                &mut size,
            )
        } < 0
        {
            return Err(error());
        }
        if value != 0 {
            return Err(std::io::Error::from_raw_os_error(value).to_string());
        }
    }
    Ok(stream)
}
impl ReadSession {
    fn attach(runtime: &Path, signature: &str) -> Result<Self, String> {
        if !runtime.is_absolute()
            || signature.is_empty()
            || signature.len() > 160
            || !signature
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err("Invalid compositor session selector".into());
        }
        for dir in [
            runtime.to_path_buf(),
            runtime.join("hypr"),
            runtime.join("hypr").join(signature),
        ] {
            owned_dir(&dir)?;
        }
        let path = runtime.join("hypr").join(signature).join(".socket.sock");
        let (socket_pin, socket) = pin_socket(&path)?;
        // Hash the selected process before connecting: Hyprland handles short IPC
        // requests in its event loop, so an idle accepted connection could stall it.
        let lock_path = path.parent().unwrap().join("hyprland.lock");
        let lock_meta = fs::symlink_metadata(&lock_path).map_err(|e| e.to_string())?;
        if !lock_meta.is_file()
            || lock_meta.uid() != unsafe { libc::geteuid() }
            || lock_meta.mode() & 0o022 != 0
        {
            return Err(
                "Compositor lock must be owned by you and protected from other-user writes".into(),
            );
        }
        let lock = hyprland_editor_core::storage::read_document(
            &path.parent().unwrap().join("hyprland.lock"),
        )?;
        if lock.len() > 1024 {
            return Err("Invalid compositor lock record".into());
        }
        let pid: u32 = lock
            .lines()
            .next()
            .ok_or("Missing compositor PID")?
            .parse()
            .map_err(|_| "Invalid compositor PID")?;
        if pid == 0 {
            return Err("Invalid compositor PID".into());
        }
        // A pidfd keeps process lifetime distinct from a recycled numeric PID.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0_u32) };
        if fd < 0 {
            return Err("Cannot pin the compositor process lifetime".into());
        }
        let process_pin = unsafe { fs::File::from_raw_fd(fd as i32) };
        let uid = unsafe { libc::geteuid() };
        if fs::metadata(format!("/proc/{pid}"))
            .map_err(|_| "Compositor process is not visible in this environment")?
            .uid()
            != uid
        {
            return Err("Compositor process belongs to another user".into());
        }
        let ticks = start_ticks(pid)?;
        let exe_path = PathBuf::from(format!("/proc/{pid}/exe"));
        let target = fs::read_link(&exe_path).map_err(|e| e.to_string())?;
        #[cfg(not(test))]
        if !target
            .file_name()
            .is_some_and(|n| n == "Hyprland" || n == "hyprland")
        {
            return Err("Socket peer is not a recognized Hyprland executable".into());
        }
        #[cfg(test)]
        let _ = target;
        let mut file = fs::File::open(exe_path).map_err(|e| e.to_string())?;
        let meta = file.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > 128 * 1024 * 1024 {
            return Err(
                "Compositor executable cannot be fingerprinted within the size budget".into(),
            );
        }
        let mut digest = Sha256::new();
        let mut buffer = [0; 65536];
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        let identity = SessionIdentity {
            signature: signature.into(),
            pid,
            uid,
            start_ticks: ticks,
            boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .map_err(|e| e.to_string())?
                .trim()
                .into(),
            binary_sha256: format!("{:x}", digest.finalize()),
        };
        let executable = (
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mtime(),
            meta.mtime_nsec(),
        );
        let session = Self {
            path,
            socket,
            _socket_pin: socket_pin,
            process_pin,
            identity,
            executable,
        };
        session.recheck()?;
        Ok(session)
    }
    fn recheck(&self) -> Result<(), String> {
        let mut process = libc::pollfd {
            fd: self.process_pin.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut process, 1, 0) };
        if ready != 0 {
            return Err("Pinned compositor process exited or became unverifiable".into());
        }
        let meta = fs::metadata(format!("/proc/{}/exe", self.identity.pid))
            .map_err(|_| "Compositor executable changed")?;
        if (
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mtime(),
            meta.mtime_nsec(),
        ) != self.executable
        {
            return Err("Compositor executable changed; discard the snapshot".into());
        }
        if socket_id(&self.path)? != self.socket
            || start_ticks(self.identity.pid)? != self.identity.start_ticks
        {
            return Err("Compositor session changed; discard this snapshot and reconnect".into());
        }
        Ok(())
    }
    fn request(&self, query: Query) -> Result<Vec<u8>, String> {
        self.recheck()?;
        let deadline = Instant::now() + TIMEOUT;
        let mut stream = connect(&self.path, deadline)?;
        let cred = peer(&stream)?;
        if cred.uid != self.identity.uid || cred.pid as u32 != self.identity.pid {
            return Err("Compositor peer changed".into());
        }
        let mut wire = query.wire();
        while !wire.is_empty() {
            ready(&stream, libc::POLLOUT, deadline)?;
            match stream.write(wire) {
                Ok(0) => return Err("Socket refused the read-only request".into()),
                Ok(n) => wire = &wire[n..],
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    continue
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        let mut out = vec![];
        let mut bytes = [0; 16384];
        loop {
            ready(&stream, libc::POLLIN, deadline)?;
            match stream.read(&mut bytes) {
                Ok(0) => break,
                Ok(n) => {
                    if out.len() + n > MAX_REPLY {
                        return Err("Compositor response exceeds 2 MiB".into());
                    }
                    out.extend_from_slice(&bytes[..n]);
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    continue
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        drop(stream);
        self.recheck()?;
        Ok(out)
    }
}

fn normalize(setting: &Setting, raw: &serde_json::Value) -> Option<Value> {
    let value = match setting.kind.as_str() {
        "Bool" => Value::Bool(raw.as_bool()?),
        "Int" => Value::Int(raw.as_i64()?),
        "Float" => Value::Float(raw.as_f64()?),
        "String" => Value::String(raw.as_str()?.into()),
        "Color" => {
            let s = raw.as_str()?.strip_prefix("0x").unwrap_or(raw.as_str()?);
            if s.is_empty() || s.len() > 8 {
                return None;
            }
            Value::Color(u32::from_str_radix(s, 16).ok()?)
        }
        "FontWeight" => Value::FontWeight(raw.as_str()?.parse().ok()?),
        "Vec2" => {
            let a = raw.as_array()?;
            if a.len() != 2 {
                return None;
            }
            Value::Vec2([a[0].as_f64()?, a[1].as_f64()?])
        }
        "CssGap" => {
            let a: Vec<i64> = raw
                .as_str()?
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()
                .ok()?;
            let gap = match a.as_slice() {
                [n] => [*n; 4],
                [a, b] => [*a, *b, *a, *b],
                [a, b, c, d] => [*a, *b, *c, *d],
                _ => return None,
            };
            Value::CssGap(gap)
        }
        // Complex grammar stays raw until its live codec is source-certified.
        "Gradient" => return None,
        _ => return None,
    };
    setting.validate(&value).ok()?;
    Some(value)
}
pub fn snapshot(
    runtime: &Path,
    signature: &str,
    catalogue: &[Setting],
) -> Result<Snapshot, String> {
    let start = Instant::now();
    let session = ReadSession::attach(runtime, signature)?;
    let v: VersionReply = serde_json::from_slice(&session.request(Query::Version)?)
        .map_err(|_| "Compositor returned malformed version JSON")?;
    let version = match v.commit.as_str() {
        BASE_COMMIT => Some(BASELINE.into()),
        MAIN_COMMIT => Some(AUDITED_MAIN.into()),
        _ => None,
    };
    let records: Vec<DescriptionReply> =
        serde_json::from_slice(&session.request(Query::Descriptions)?).map_err(|_| {
            "Compositor returned malformed descriptions JSON; values remain unknown"
        })?;
    if records.len() > 2048 {
        return Err("Too many compositor description records".into());
    }
    let mut names = BTreeSet::new();
    let mut values = BTreeMap::new();
    let mut unknown = 0;
    for record in records {
        if record.name.len() > 256
            || record.description.len() > 16384
            || !names.insert(record.name.clone())
        {
            return Err("Invalid or duplicate compositor description record".into());
        }
        let setting = version.as_ref().and_then(|ver| {
            catalogue
                .iter()
                .find(|s| s.version == *ver && (s.id == record.name || s.lua_key == record.name))
        });
        if let Some(setting) = setting {
            values.insert(
                setting.id.clone(),
                Observation {
                    current: normalize(setting, &record.current),
                    default: normalize(setting, &record.default),
                    raw_current: record.current,
                    description: record.description,
                },
            );
        } else {
            unknown += 1;
        }
    }
    session.recheck()?;
    Ok(Snapshot {
        identity: session.identity,
        commit: v.commit,
        version,
        sampled_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
        duration_ms: start.elapsed().as_millis(),
        values,
        unknown_settings: unknown,
    })
}
pub fn current_session_snapshot(catalogue: &[Setting]) -> Result<Snapshot, String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("No user runtime directory; start the editor inside your Hyprland session")?;
    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .map_err(|_| "No selected Hyprland session in this environment")?;
    snapshot(&runtime, &signature, catalogue)
}

fn parse_monitors(bytes: &[u8]) -> Result<Vec<Monitor>, String> {
    if bytes.len() > MAX_REPLY {
        return Err("Monitor response exceeds 2 MiB".into());
    }
    let monitors: Vec<Monitor> =
        serde_json::from_slice(bytes).map_err(|_| "Compositor returned malformed monitor JSON")?;
    if monitors.len() > 256 {
        return Err("More than 256 output observations were returned".into());
    }
    let mut names = BTreeSet::new();
    for monitor in &monitors {
        if monitor.name.is_empty()
            || monitor.name.len() > 4096
            || monitor.name.chars().any(char::is_control)
            || !names.insert(monitor.name.as_str())
            || monitor
                .description
                .as_ref()
                .is_some_and(|text| text.len() > 4096)
            || monitor
                .refresh_rate
                .is_some_and(|value| !value.is_finite() || value < 0.)
            || monitor
                .scale
                .is_some_and(|value| !value.is_finite() || value < 0.)
            || monitor.available_modes.as_ref().is_some_and(|modes| {
                modes.len() > 512
                    || modes
                        .iter()
                        .any(|mode| mode.len() > 128 || mode.chars().any(char::is_control))
            })
        {
            return Err("Invalid, duplicate or oversized output observation".into());
        }
    }
    Ok(monitors)
}

pub fn monitor_snapshot(runtime: &Path, signature: &str) -> Result<MonitorSnapshot, String> {
    let start = Instant::now();
    let session = ReadSession::attach(runtime, signature)?;
    let version: VersionReply = serde_json::from_slice(&session.request(Query::Version)?)
        .map_err(|_| "Compositor returned malformed version JSON")?;
    let pack = match version.commit.as_str() {
        BASE_COMMIT => Some(BASELINE.into()),
        MAIN_COMMIT => Some(AUDITED_MAIN.into()),
        _ => None,
    };
    let monitors = parse_monitors(&session.request(Query::Monitors)?)?;
    session.recheck()?;
    Ok(MonitorSnapshot {
        identity: session.identity,
        commit: version.commit,
        version: pack,
        sampled_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_millis(),
        duration_ms: start.elapsed().as_millis(),
        monitors,
    })
}

pub fn current_session_monitors() -> Result<MonitorSnapshot, String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("No user runtime directory; start the editor inside your Hyprland session")?;
    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .map_err(|_| "No selected Hyprland session in this environment")?;
    monitor_snapshot(&runtime, &signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::net::UnixListener,
        sync::{Arc, Mutex},
        thread,
    };
    fn fixture() -> (PathBuf, UnixListener) {
        use std::{
            os::unix::fs::DirBuilderExt,
            sync::atomic::{AtomicU64, Ordering},
        };
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = loop {
            let candidate = std::env::temp_dir().join(format!(
                "he-ipc-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::DirBuilder::new().mode(0o700).create(&candidate) {
                Ok(()) => break candidate,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("Cannot create private IPC fixture: {e}"),
            }
        };
        fs::create_dir_all(root.join("hypr/test")).unwrap();
        fs::write(
            root.join("hypr/test/hyprland.lock"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        let listener = UnixListener::bind(root.join("hypr/test/.socket.sock")).unwrap();
        (root, listener)
    }
    #[test]
    fn selected_session_only_uses_fixed_read_queries() {
        let (root, listener) = fixture();
        let requests = Arc::new(Mutex::new(vec![]));
        let read = requests.clone();
        let server = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = [0; 64];
                let n = stream.read(&mut bytes).unwrap();
                let request = std::str::from_utf8(&bytes[..n]).unwrap();
                if request.is_empty() {
                    continue;
                }
                read.lock().unwrap().push(request.to_string());
                let reply = if request == "j/version" {
                    format!("{{\"commit\":\"{BASE_COMMIT}\"}}")
                } else {
                    r#"[{"name":"general:border_size","description":"Border width","current":3,"default":1},{"name":"general:col.active_border","current":"complex","default":"complex"}]"#.into()
                };
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        let snap = snapshot(&root, "test", &hyprland_editor_core::catalogue()).unwrap();
        server.join().unwrap();
        assert_eq!(
            *requests.lock().unwrap(),
            vec!["j/version", "j/descriptions"]
        );
        assert_eq!(
            snap.values["general:border_size"].current,
            Some(Value::Int(3))
        );
        assert_eq!(snap.values["general:col.active_border"].current, None);
        assert_eq!(snap.identity.pid, std::process::id());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn output_snapshot_uses_only_fixed_queries_and_keeps_missing_values_unknown() {
        let (root, listener) = fixture();
        let requests = Arc::new(Mutex::new(vec![]));
        let observed = requests.clone();
        let server = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = [0; 64];
                let count = stream.read(&mut bytes).unwrap();
                let request = String::from_utf8(bytes[..count].to_vec()).unwrap();
                observed.lock().unwrap().push(request.clone());
                let reply = match request.as_str() {
                    "j/version" => br#"{"commit":"unmapped-build"}"#.as_slice(),
                    "j/monitors all" => br#"[{"name":"DP-1","width":2560,"height":1440,"refreshRate":143.97,"x":-2560,"y":0,"scale":1.25,"disabled":false,"availableModes":["2560x1440@143.97Hz"],"unmappedField":true},{"name":"HDMI-A-1","disabled":true}]"#.as_slice(),
                    _ => panic!("Unexpected request: {request}"),
                };
                stream.write_all(reply).unwrap();
            }
        });
        let snapshot = monitor_snapshot(&root, "test").unwrap();
        server.join().unwrap();
        assert_eq!(
            *requests.lock().unwrap(),
            vec!["j/version", "j/monitors all"]
        );
        assert!(snapshot.version.is_none());
        assert_eq!(snapshot.monitors[0].x, Some(-2560));
        assert_eq!(snapshot.monitors[0].disabled, Some(false));
        assert_eq!(snapshot.monitors[1].disabled, Some(true));
        assert!(snapshot.monitors[1].width.is_none());
        assert!(snapshot.monitors[1].available_modes.is_none());
        assert_eq!(snapshot.identity.pid, std::process::id());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn output_payload_rejects_ambiguous_invalid_and_oversized_records() {
        for bytes in [
            b"null".as_slice(),
            br#"[{"name":"DP-1"},{"name":"DP-1"}]"#,
            br#"[{"name":"DP-1","name":"DP-2"}]"#,
            br#"[{"name":"bad\nname"}]"#,
            br#"[{"name":"DP-1","width":-1}]"#,
            br#"[{"name":"DP-1","scale":-1}]"#,
        ] {
            assert!(parse_monitors(bytes).is_err(), "{bytes:?}");
        }
        let too_many = serde_json::to_vec(
            &(0..257)
                .map(|index| serde_json::json!({"name":format!("HEADLESS-{index}")}))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert!(parse_monitors(&too_many).is_err());
        let many_modes = serde_json::to_vec(
            &serde_json::json!([{"name":"DP-1","availableModes":vec!["1x1@60Hz";513]}]),
        )
        .unwrap();
        assert!(parse_monitors(&many_modes).is_err());
        assert!(parse_monitors(&vec![b' '; MAX_REPLY + 1]).is_err());
    }
    #[test]
    fn rejects_path_injection_symlinks_and_replaced_session() {
        let (root, listener) = fixture();
        assert!(snapshot(&root, "../bad", &[]).is_err());
        let session = ReadSession::attach(&root, "test").unwrap();
        drop(listener);
        fs::remove_file(&session.path).unwrap();
        let _replacement = UnixListener::bind(&session.path).unwrap();
        assert!(session.recheck().is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn malformed_json_and_limits_fail_closed() {
        let (root, listener) = fixture();
        let server = thread::spawn(move || {
            for _ in 0..1 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut b = [0; 64];
                let n = stream.read(&mut b).unwrap();
                if n > 0 {
                    stream.write_all(b"not JSON").unwrap();
                }
            }
        });
        assert!(snapshot(&root, "test", &[]).is_err());
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stalled_peer_has_absolute_timeout() {
        let (root, listener) = fixture();
        let server = thread::spawn(move || {
            for _ in 0..1 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut b = [0; 64];
                let n = stream.read(&mut b).unwrap();
                if n > 0 {
                    thread::sleep(Duration::from_millis(2200));
                }
            }
        });
        let session = ReadSession::attach(&root, "test").unwrap();
        let start = Instant::now();
        assert!(session.request(Query::Version).is_err());
        assert!(start.elapsed() < Duration::from_millis(2500));
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
