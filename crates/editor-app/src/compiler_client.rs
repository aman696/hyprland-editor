//! On-demand installed compiler: fixed root-owned registry, immutable static
//! image and bounded parent exchange. No saved path or reply grants execution.
#[path = "../../editor-lua55/src/protocol.rs"]
pub(crate) mod wire;
use hyprland_editor_core::lua::source_hash;
use hyprland_editor_preview::image::{FileSpec, Image, Manifest};
use serde::Deserialize;
use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd},
        unix::{
            fs::{FileExt, MetadataExt},
            process::CommandExt,
        },
    },
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
const DEADLINE: Duration = Duration::from_secs(3);
const STDERR_BOUND: usize = 65536;
const SEALS: i32 = libc::F_SEAL_WRITE
    | libc::F_SEAL_GROW
    | libc::F_SEAL_SHRINK
    | libc::F_SEAL_SEAL
    | libc::F_SEAL_EXEC;
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    format: String,
    release: String,
    image_bytes: u64,
    image_sha256: String,
}
pub(crate) struct Compiler {
    image: File,
    release: String,
    image_sha256: String,
}
fn error(reason: &'static str) -> String {
    reason.into()
}
fn owned_root(file: &File, directory: bool) -> Result<(), String> {
    let m = file.metadata().map_err(|e| e.to_string())?;
    if m.uid() != 0
        || m.mode() & 0o022 != 0
        || if directory {
            !m.is_dir()
        } else {
            !m.is_file() || m.nlink() != 1
        }
    {
        return Err(error("Installed compiler metadata must be a root-owned regular package with no group/other writes"));
    }
    Ok(())
}
fn open_at(directory: BorrowedFd<'_>, name: &std::ffi::CStr, folder: bool) -> Result<File, String> {
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if folder { libc::O_DIRECTORY } else { 0 };
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(error(
            "The packaged Lua 5.5 compiler is unavailable or cannot be opened safely",
        ));
    }
    let file = unsafe { File::from_raw_fd(fd) };
    owned_root(&file, folder)?;
    Ok(file)
}
impl Compiler {
    pub(crate) fn image_sha256(&self) -> &str {
        &self.image_sha256
    }
    #[cfg(test)]
    pub(crate) fn fixture(file: &File, sha256: &str, release: &str) -> Result<Self, String> {
        Self::capture(
            &Registration {
                format: "hye-installed-compiler55-1".into(),
                release: release.into(),
                image_bytes: file.metadata().map_err(|e| e.to_string())?.len(),
                image_sha256: sha256.into(),
            },
            file,
        )
    }
    /// Fixed system package location, never a setting, environment variable or
    /// user config path. Local/AppImage trust and package signatures need their
    /// own future admission paths; they cannot silently bypass this one.
    pub(crate) fn installed() -> Result<Self, String> {
        let fd = unsafe {
            libc::open(
                c"/".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(error("Cannot inspect compiler package root"));
        }
        let mut directory = unsafe { File::from_raw_fd(fd) };
        owned_root(&directory, true)?;
        for name in [c"usr", c"libexec", c"hyprland-editor", c"lua55"] {
            directory = open_at(directory.as_fd(), name, true)?;
        }
        Self::from_directory(&directory)
    }
    #[cfg(test)]
    pub(crate) fn fixture_installed(directory: &File) -> Result<Self, String> {
        Self::from_directory(directory)
    }
    fn from_directory(directory: &File) -> Result<Self, String> {
        owned_root(directory, true)?;
        let registry = open_at(directory.as_fd(), c"manifest.json", false)?;
        if registry.metadata().map_err(|e| e.to_string())?.len() > 16384 {
            return Err(error("Compiler package registry exceeds its bound"));
        }
        let mut bytes = Vec::new();
        registry
            .take(16385)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 16384 {
            return Err(error("Compiler package registry exceeds its bound"));
        }
        let registration: Registration = serde_json::from_slice(&bytes)
            .map_err(|_| error("Invalid compiler package registry"))?;
        if registration.format != "hye-installed-compiler55-1" {
            return Err(error("Unknown compiler package registry"));
        }
        let file = open_at(directory.as_fd(), c"compiler", false)?;
        Self::capture(&registration, &file)
    }
    fn capture(registration: &Registration, file: &File) -> Result<Self, String> {
        if !matches!(registration.release.as_str(), "5.5.0" | "5.5.1")
            || registration.image_bytes == 0
            || registration.image_bytes > 64 * 1024 * 1024
        {
            return Err(error("Unsupported compiler package release or size"));
        }
        // Image capture independently checks exact bytes/hash, original identity
        // before/after copying and seals the snapshot. It restores no path access.
        let captured = Image::capture(
            Manifest {
                format: 1,
                entry: "/app/compiler".into(),
                files: vec![FileSpec {
                    destination: "/app/compiler".into(),
                    bytes: registration.image_bytes,
                    sha256: registration.image_sha256.clone(),
                    executable: true,
                }],
            },
            &[file.as_fd()],
        )
        .map_err(|e| e.to_string())?;
        let (_, source) = captured.files().next().ok_or("Compiler image is empty")?;
        let source = File::from(source.try_clone_to_owned().map_err(|e| e.to_string())?);
        static_elf(&source)?;
        // Promote only this approved static snapshot to an executable copy.
        // No interpreter or DT_NEEDED dependency is followed through the host.
        let fd = unsafe {
            libc::memfd_create(
                c"hye-approved-compiler".as_ptr(),
                libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING | libc::MFD_EXEC,
            )
        };
        if fd < 0 {
            return Err(error(
                "Executable compiler snapshots are unsupported on this kernel",
            ));
        }
        let mut image = unsafe { File::from_raw_fd(fd) };
        let mut buffer = [0u8; 16384];
        let mut offset = 0;
        while offset < registration.image_bytes {
            let size = (registration.image_bytes - offset).min(buffer.len() as u64) as usize;
            source
                .read_exact_at(&mut buffer[..size], offset)
                .map_err(|e| e.to_string())?;
            image
                .write_all(&buffer[..size])
                .map_err(|e| e.to_string())?;
            offset += size as u64;
        }
        if unsafe { libc::fchmod(fd, 0o500) } != 0
            || unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, SEALS) } != 0
        {
            return Err(error("Cannot seal approved compiler executable"));
        }
        Ok(Self {
            image,
            release: registration.release.clone(),
            image_sha256: registration.image_sha256.clone(),
        })
    }
    pub(crate) fn run(
        &self,
        operation: wire::Operation,
        cancelled: &AtomicBool,
    ) -> Result<wire::Reply, String> {
        cancelled_check(cancelled)?;
        unprivileged()?;
        validate_operation(&operation)?;
        let mut nonce = [0u8; 32];
        let mut offset = 0;
        for _ in 0..8 {
            if offset == nonce.len() {
                break;
            }
            let count = unsafe {
                libc::getrandom(
                    nonce[offset..].as_mut_ptr().cast(),
                    nonce.len() - offset,
                    libc::GRND_NONBLOCK,
                )
            };
            if count > 0 {
                offset += count as usize;
            } else if count < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            } else {
                break;
            }
        }
        if offset != nonce.len() {
            return Err(error("Compiler request entropy unavailable"));
        }
        let request = wire::Request {
            protocol: wire::PROTOCOL.into(),
            nonce,
            expected_release: self.release.clone(),
            operation,
        };
        let bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        if bytes.len() > wire::MAX_WIRE {
            return Err(error("Compiler request exceeds 4 MiB"));
        }
        let mut command = Command::new(format!("/proc/self/fd/{}", self.image.as_raw_fd()));
        command.arg("--compiler-worker").env_clear();
        let reply = exchange(command, &bytes, cancelled, DEADLINE, |_| {})?;
        validate_reply(&request, &bytes, &reply)?;
        Ok(reply)
    }
}
/// Strict bounded ELF64 little-endian native static image. A package digest is
/// checked separately; accepting ELF shape never establishes publisher trust.
fn static_elf(file: &File) -> Result<(), String> {
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    let mut h = [0u8; 64];
    file.read_exact_at(&mut h, 0)
        .map_err(|_| error("Truncated compiler ELF header"))?;
    let u16at = |n| u16::from_le_bytes([h[n], h[n + 1]]);
    let machine = if cfg!(target_arch = "x86_64") {
        62
    } else if cfg!(target_arch = "aarch64") {
        183
    } else {
        return Err(error("Compiler image architecture is unsupported"));
    };
    if &h[..7] != b"\x7fELF\x02\x01\x01"
        || u32::from_le_bytes(h[20..24].try_into().unwrap()) != 1
        || !matches!(u16at(16), 2 | 3)
        || u16at(18) != machine
        || u16at(52) != 64
        || u16at(54) != 56
    {
        return Err(error(
            "Compiler image is not a supported native ELF64 executable",
        ));
    }
    let start = u64::from_le_bytes(h[32..40].try_into().unwrap());
    let count = u16at(56) as u64;
    if start < 64
        || count == 0
        || count > 128
        || start.checked_add(count * 56).is_none_or(|end| end > size)
    {
        return Err(error("Invalid compiler ELF program table"));
    }
    let entry = u64::from_le_bytes(h[24..32].try_into().unwrap());
    let mut executable_entry = false;
    for index in 0..count {
        let mut p = [0u8; 56];
        file.read_exact_at(&mut p, start + index * 56)
            .map_err(|e| e.to_string())?;
        let kind = u32::from_le_bytes(p[..4].try_into().unwrap());
        let flags = u32::from_le_bytes(p[4..8].try_into().unwrap());
        let offset = u64::from_le_bytes(p[8..16].try_into().unwrap());
        let length = u64::from_le_bytes(p[32..40].try_into().unwrap());
        if kind == 1 {
            let address = u64::from_le_bytes(p[16..24].try_into().unwrap());
            let memory = u64::from_le_bytes(p[40..48].try_into().unwrap());
            let end = address
                .checked_add(memory)
                .ok_or("Compiler load segment overflows its address space")?;
            if memory < length {
                return Err(error("Compiler load segment is shorter than its file data"));
            }
            executable_entry |= flags & 1 != 0 && address <= entry && entry < end;
        }
        if offset.checked_add(length).is_none_or(|end| end > size)
            || kind == 3
            || (kind == 1 && flags & 3 == 3)
            || (kind == 0x6474e551 && flags & 1 != 0)
        {
            return Err(error(
                "Compiler ELF has a loader, writable executable segment or invalid file extent",
            ));
        }
        if kind == 2 {
            if length > 1024 * 1024 || length % 16 != 0 {
                return Err(error("Invalid compiler ELF dynamic table"));
            }
            let mut terminated = false;
            for n in 0..length / 16 {
                let mut d = [0u8; 16];
                file.read_exact_at(&mut d, offset + n * 16)
                    .map_err(|e| e.to_string())?;
                match u64::from_le_bytes(d[..8].try_into().unwrap()) {
                    0 => {
                        terminated = true;
                        break;
                    }
                    1 | 15 | 29 | 0x7fffffff | 0x7ffffffd => {
                        return Err(error(
                            "Compiler executable has external library or search-path dependencies",
                        ))
                    }
                    _ => {}
                }
            }
            if !terminated {
                return Err(error("Unterminated compiler ELF dynamic table"));
            }
        }
    }
    if !executable_entry {
        return Err(error(
            "Compiler entry is outside its executable load segments",
        ));
    }
    Ok(())
}
fn unprivileged() -> Result<(), String> {
    #[repr(C)]
    struct Header {
        version: u32,
        pid: i32,
    }
    let mut header = Header {
        version: 0x20080522,
        pid: 0,
    };
    let mut data = [0u32; 6];
    if unsafe {
        libc::geteuid() == 0
            || libc::getuid() != libc::geteuid()
            || libc::getgid() != libc::getegid()
            || libc::syscall(libc::SYS_capget, &mut header, data.as_mut_ptr()) != 0
    } || data.iter().any(|v| *v != 0)
    {
        return Err(error(
            "Compiler launch requires an ordinary user with no retained capabilities",
        ));
    }
    Ok(())
}
fn validate_operation(operation: &wire::Operation) -> Result<(), String> {
    match operation {
        wire::Operation::Compile { source, load_name }
            if source.len() > wire::MAX_SOURCE
                || load_name.len() > 4096
                || load_name.contains(&0) =>
        {
            Err(error(
                "Compiler source/name exceeds its shape or byte bound",
            ))
        }
        wire::Operation::Convert { values }
            if values.len() > 256
                || values
                    .iter()
                    .any(|v| matches!(v,wire::Primitive::Bytes(b) if b.len()>65536)) =>
        {
            Err(error("Compiler primitive input exceeds its bounds"))
        }
        _ => Ok(()),
    }
}
fn validate_reply(
    request: &wire::Request,
    bytes: &[u8],
    reply: &wire::Reply,
) -> Result<(), String> {
    if reply.protocol != wire::PROTOCOL
        || reply.nonce != Some(request.nonce)
        || reply.request_sha256
            != source_hash(std::str::from_utf8(bytes).map_err(|e| e.to_string())?)
    {
        return Err(error("Compiler reply correlation differs from the request"));
    }
    let p = reply
        .profile
        .as_ref()
        .ok_or("Compiler reply has no observed profile")?;
    if p.release != request.expected_release
        || p.family != 505
        || p.library_identity.len() > 512
        || !p.library_identity.contains(&format!("Lua {} ", p.release))
        || p.numeric_locale != "C"
        || p.private_seed != "kernel_random"
        || p.imported_code_executed
        || p.complete_language_verified
        || p.selected_compositor_verified
    {
        return Err(error(
            "Compiler reply profile differs from the approved role",
        ));
    }
    match (&request.operation, &reply.outcome) {
        (_, wire::Outcome::Error { reason }) if !reason.is_empty() && reason.len() <= 4096 => {
            Err(format!("Compiler refused the operation: {reason}"))
        }
        (wire::Operation::Profile {}, wire::Outcome::Profile {}) => Ok(()),
        (wire::Operation::Compile { .. }, wire::Outcome::Compiled { valid, diagnostic })
            if diagnostic.len() <= 4096 && (!valid || diagnostic.is_empty()) =>
        {
            Ok(())
        }
        (wire::Operation::Convert { values: input }, wire::Outcome::Converted { values })
            if values.len() == input.len()
                && values
                    .iter()
                    .all(|v| v.flags & !15 == 0 && v.text.len() <= 65536)
                && values
                    .iter()
                    .map(|v| v.text.len() + std::mem::size_of::<wire::Conversion>())
                    .sum::<usize>()
                    <= 512 * 1024 =>
        {
            Ok(())
        }
        (
            wire::Operation::Probe {},
            wire::Outcome::Probe {
                file_denied: true,
                network_denied: true,
                process_denied: true,
                inherited_descriptor_closed: true,
            },
        ) => Ok(()),
        _ => Err(error("Compiler reply operation or output bounds differ")),
    }
}
fn cancelled_check(cancelled: &AtomicBool) -> Result<(), String> {
    if cancelled.load(Ordering::Acquire) {
        Err(error("Compiler operation was cancelled"))
    } else {
        Ok(())
    }
}
struct OwnedChild {
    child: Child,
    reaped: bool,
}
impl OwnedChild {
    fn poll(&mut self) -> Result<Option<ExitStatus>, String> {
        let status = self.child.try_wait().map_err(|e| e.to_string())?;
        self.reaped = status.is_some();
        Ok(status)
    }
    fn stop(&mut self) -> Result<(), String> {
        if !self.reaped {
            let _ = self.child.kill();
            self.child.wait().map_err(|e| e.to_string())?;
            self.reaped = true;
        }
        Ok(())
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
fn nonblocking(fd: i32) -> Result<(), String> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        Err(error("Cannot prepare compiler parent pipes"))
    } else {
        Ok(())
    }
}
fn read_pipe(pipe: &mut impl Read, bytes: &mut Vec<u8>, bound: usize) -> Result<bool, String> {
    let mut buffer = [0u8; 16384];
    match pipe.read(&mut buffer) {
        Ok(0) => Ok(true),
        Ok(n) => {
            if bytes.len().saturating_add(n) > bound {
                return Err(error("Compiler output exceeds its byte bound"));
            }
            bytes.extend_from_slice(&buffer[..n]);
            Ok(false)
        }
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) =>
        {
            Ok(false)
        }
        Err(e) => Err(e.to_string()),
    }
}
fn exchange(
    mut command: Command,
    bytes: &[u8],
    cancelled: &AtomicBool,
    bound: Duration,
    on_spawn: impl FnOnce(u32),
) -> Result<wire::Reply, String> {
    cancelled_check(cancelled)?;
    let start = Instant::now();
    let parent = unsafe { libc::getpid() };
    // Only async-signal-safe syscalls run between fork and exec. The approved
    // static image starts with no privilege escalation or unrelated descriptors.
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(
                libc::PR_SET_PDEATHSIG,
                libc::SIGKILL as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
            ) != 0
            {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(io::Error::from_raw_os_error(libc::ECHILD));
            }
            if libc::prctl(
                libc::PR_SET_NO_NEW_PRIVS,
                1 as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
            ) != 0
                || libc::syscall(
                    libc::SYS_close_range,
                    3u32,
                    u32::MAX,
                    libc::CLOSE_RANGE_CLOEXEC,
                ) != 0
            {
                return Err(io::Error::last_os_error());
            }
            for (resource, max) in [
                (libc::RLIMIT_AS, 128 * 1024 * 1024),
                (libc::RLIMIT_CPU, 2),
                (libc::RLIMIT_STACK, 8 * 1024 * 1024),
                (libc::RLIMIT_CORE, 0),
                (libc::RLIMIT_NOFILE, 16),
            ] {
                let limit = libc::rlimit {
                    rlim_cur: max,
                    rlim_max: max,
                };
                if libc::setrlimit(resource, &limit) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut owned = OwnedChild {
        child,
        reaped: false,
    };
    on_spawn(owned.child.id());
    let result = (|| {
        let mut input = owned.child.stdin.take();
        let mut out = owned
            .child
            .stdout
            .take()
            .ok_or("Compiler output pipe missing")?;
        let mut err = owned
            .child
            .stderr
            .take()
            .ok_or("Compiler diagnostic pipe missing")?;
        nonblocking(
            input
                .as_ref()
                .ok_or("Compiler input pipe missing")?
                .as_raw_fd(),
        )?;
        nonblocking(out.as_raw_fd())?;
        nonblocking(err.as_raw_fd())?;
        let (mut written, mut output, mut diagnostic, mut out_eof, mut err_eof, mut status) =
            (0, Vec::new(), Vec::new(), false, false, None);
        loop {
            cancelled_check(cancelled)?;
            if start.elapsed() >= bound {
                return Err(error("Compiler operation exceeded its parent deadline"));
            }
            if let Some(pipe) = input.as_mut() {
                if written < bytes.len() {
                    let end = (written + 16384).min(bytes.len());
                    match pipe.write(&bytes[written..end]) {
                        Ok(0) => return Err(error("Compiler input pipe closed early")),
                        Ok(n) => written += n,
                        Err(e)
                            if matches!(
                                e.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                            ) => {}
                        Err(e) => return Err(e.to_string()),
                    }
                }
                if written == bytes.len() {
                    input.take();
                }
            }
            if !out_eof {
                out_eof = read_pipe(&mut out, &mut output, wire::MAX_WIRE)?;
            }
            if !err_eof {
                err_eof = read_pipe(&mut err, &mut diagnostic, STDERR_BOUND)?;
            }
            if status.is_none() {
                status = owned.poll()?;
            }
            if let Some(status) = status {
                if out_eof && err_eof {
                    if !status.success() || written != bytes.len() {
                        return Err(error(
                            "Compiler exited unsuccessfully or before reading its request",
                        ));
                    }
                    break;
                }
            }
            let mut polls = [
                libc::pollfd {
                    fd: input.as_ref().map_or(-1, AsRawFd::as_raw_fd),
                    events: libc::POLLOUT,
                    revents: 0,
                },
                libc::pollfd {
                    fd: if out_eof { -1 } else { out.as_raw_fd() },
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: if err_eof { -1 } else { err.as_raw_fd() },
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            if unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as _, 10) } < 0
                && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted
            {
                return Err(error("Compiler parent pipe wait failed"));
            }
        }
        cancelled_check(cancelled)?;
        serde_json::from_slice(&output).map_err(|_| error("Invalid compiler reply schema"))
    })();
    owned.stop()?;
    result
}

#[cfg(test)]
pub(crate) fn fixture_exchange(
    command: Command,
    bytes: &[u8],
    cancelled: &AtomicBool,
    bound: Duration,
    on_spawn: impl FnOnce(u32),
) -> Result<wire::Reply, String> {
    exchange(command, bytes, cancelled, bound, on_spawn)
}
#[cfg(test)]
pub(crate) fn fixture_elf(file: &File) -> Result<(), String> {
    static_elf(file)
}
#[cfg(test)]
pub(crate) fn fixture_reply(
    request: &wire::Request,
    bytes: &[u8],
    reply: &wire::Reply,
) -> Result<(), String> {
    validate_reply(request, bytes, reply)
}
