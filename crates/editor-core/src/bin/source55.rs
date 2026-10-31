//! Separate native55 ordered-analysis image. Imported Lua is never invoked.
use hyprland_editor_core::lua::replay::{self, CompilerProfile, Program, Report};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io,
    time::{Duration, Instant},
};
#[allow(dead_code)]
#[path = "../../../editor-app/src/process_policy.rs"]
mod process_policy;
const PROTOCOL: &str = "hye-lua55-source-1";
const MAX_WIRE: usize = 4 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    nonce: [u8; 32],
    expected_release: String,
    operation: Operation,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Profile {},
    Analyze {
        compiler: Box<CompilerProfile>,
        program: Box<Program>,
    },
    Probe {},
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Outcome {
    Profile {},
    Analyzed {
        report: Box<Report>,
    },
    Probe {
        file_denied: bool,
        network_denied: bool,
        process_denied: bool,
        inherited_descriptor_closed: bool,
    },
    Error {
        reason: String,
    },
}
impl Outcome {
    fn error(reason: impl Into<String>) -> Self {
        Self::Error {
            reason: reason.into(),
        }
    }
}
#[derive(Serialize)]
struct Reply {
    protocol: &'static str,
    request_sha256: String,
    nonce: Option<[u8; 32]>,
    profile: Option<CompilerProfile>,
    outcome: Outcome,
}
const DEADLINE: Duration = Duration::from_secs(3);

fn isolate() -> Result<(), String> {
    #[repr(C)]
    struct CapHeader {
        version: u32,
        pid: i32,
    }
    let mut header = CapHeader {
        version: 0x20080522,
        pid: 0,
    };
    let mut capabilities = [0u32; 6];
    if unsafe {
        libc::geteuid() == 0
            || libc::getegid() == 0
            || libc::getuid() != libc::geteuid()
            || libc::getgid() != libc::getegid()
            || libc::syscall(libc::SYS_capget, &mut header, capabilities.as_mut_ptr()) != 0
    } || capabilities.iter().any(|v| *v != 0)
    {
        return Err(
            "Source analysis requires an ordinary user without retained capabilities".into(),
        );
    }
    for (resource, max) in [
        (libc::RLIMIT_AS, 256 * 1024 * 1024),
        (libc::RLIMIT_CPU, 2),
        (libc::RLIMIT_CORE, 0),
        (libc::RLIMIT_STACK, 8 * 1024 * 1024),
        (libc::RLIMIT_NOFILE, 16),
    ] {
        let limit = libc::rlimit {
            rlim_cur: max,
            rlim_max: max,
        };
        if unsafe { libc::setrlimit(resource, &limit) } != 0 {
            return Err("Cannot set source resource limits".into());
        }
    }
    if unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) } != 0 {
        return Err("Cannot close inherited source descriptors".into());
    }
    for fd in [0, 1] {
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err("Cannot prepare bounded source pipes".into());
        }
    }
    if unsafe {
        libc::prctl(
            libc::PR_SET_NO_NEW_PRIVS,
            1 as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
        )
    } != 0
    {
        return Err("Cannot disable source privilege escalation".into());
    }
    process_policy::compiler()
}
fn wait(fd: i32, events: i16, start: Instant) -> Result<(), String> {
    loop {
        let left = DEADLINE
            .checked_sub(start.elapsed())
            .ok_or("Source pipe deadline exceeded")?;
        let mut poll = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let timeout = libc::timespec {
            tv_sec: left.as_secs() as _,
            tv_nsec: left.subsec_nanos() as _,
        };
        let result = unsafe { libc::ppoll(&mut poll, 1, &timeout, std::ptr::null()) };
        if result > 0 {
            return Ok(());
        }
        if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            return Err("Source pipe wait failed".into());
        }
    }
}
fn read_request(start: Instant) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16384];
    loop {
        if start.elapsed() >= DEADLINE {
            return Err("Source input deadline exceeded".into());
        }
        let count = unsafe { libc::read(0, chunk.as_mut_ptr().cast(), chunk.len()) };
        if count == 0 {
            return Ok(bytes);
        }
        if count < 0 {
            match io::Error::last_os_error().kind() {
                io::ErrorKind::Interrupted => continue,
                io::ErrorKind::WouldBlock => {
                    wait(0, libc::POLLIN, start)?;
                    continue;
                }
                _ => return Err("Cannot read source input".into()),
            }
        }
        if bytes.len().saturating_add(count as usize) > MAX_WIRE {
            return Err("Source input exceeds 4 MiB".into());
        }
        bytes.extend_from_slice(&chunk[..count as usize]);
    }
}
fn probe() -> Outcome {
    let file = unsafe { libc::open(c"/etc/passwd".as_ptr(), libc::O_RDONLY) };
    let file_denied = file < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
    if file >= 0 {
        unsafe { libc::close(file) };
    }
    let socket = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    let network_denied =
        socket < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
    if socket >= 0 {
        unsafe { libc::close(socket) };
    }
    // Invalid null arguments cannot create a child even if a policy regresses.
    let process =
        unsafe { libc::syscall(libc::SYS_clone3, std::ptr::null::<libc::c_void>(), 0usize) };
    let process_denied =
        process < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
    // Resource setup has already closed all descriptors above stderr.
    let mut byte = [0u8];
    let inherited = unsafe { libc::read(63, byte.as_mut_ptr().cast(), 1) };
    let inherited_descriptor_closed =
        inherited < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EBADF);
    Outcome::Probe {
        file_denied,
        network_denied,
        process_denied,
        inherited_descriptor_closed,
    }
}
fn handle(bytes: &[u8]) -> Reply {
    let mut reply = Reply {
        protocol: PROTOCOL,
        request_sha256: format!("{:x}", Sha256::digest(bytes)),
        nonce: None,
        profile: None,
        outcome: Outcome::error("Invalid source-analysis request"),
    };
    let Ok(request) = serde_json::from_slice::<Request>(bytes) else {
        return reply;
    };
    reply.nonce = Some(request.nonce);
    if request.protocol != PROTOCOL {
        reply.outcome = Outcome::error("Unknown source-analysis protocol");
        return reply;
    }
    let profile = replay::compiler_profile();
    if !matches!(profile.release.as_str(), "5.5.0" | "5.5.1")
        || !profile.primitive_conversions_profiled
        || profile.error_call_line.is_none()
        || profile.numeric_codegen.is_none()
    {
        reply.outcome =
            Outcome::error("The source worker has no admitted native55 compiler profile");
        return reply;
    }
    reply.profile = Some(profile.clone());
    if request.expected_release != profile.release {
        reply.outcome = Outcome::error("Source compiler release differs from the explicit request");
        return reply;
    }
    reply.outcome = match request.operation {
        Operation::Profile {} => Outcome::Profile {},
        Operation::Probe {} => probe(),
        Operation::Analyze { compiler, program } => {
            if *compiler != profile {
                Outcome::error("Source compiler observation differs from the explicit request")
            } else if program.profile != replay::Profile::HyprlandLua55 {
                Outcome::error("This source image requires an explicit Hyprland Lua 5.5 program")
            } else {
                match replay::replay(&program) {
                    Ok(report) => Outcome::Analyzed {
                        report: Box::new(report),
                    },
                    Err(reason) => Outcome::error(reason),
                }
            }
        }
    };
    reply
}
struct Bounded(Vec<u8>);
impl io::Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_WIRE {
            return Err(io::Error::other("Source output exceeds 4 MiB"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn write_reply(mut reply: Reply, start: Instant) -> Result<(), String> {
    let mut output = Bounded(Vec::new());
    if serde_json::to_writer(&mut output, &reply).is_err() {
        output.0.clear();
        reply.outcome = Outcome::error("Source result exceeds its output budget");
        serde_json::to_writer(&mut output, &reply).map_err(|_| "Cannot encode source refusal")?;
    }
    let mut offset = 0;
    while offset < output.0.len() {
        if start.elapsed() >= DEADLINE {
            return Err("Source output deadline exceeded".into());
        }
        let count = unsafe {
            libc::write(
                1,
                output.0[offset..].as_ptr().cast(),
                output.0.len() - offset,
            )
        };
        if count <= 0 {
            match io::Error::last_os_error().kind() {
                io::ErrorKind::Interrupted => continue,
                io::ErrorKind::WouldBlock => {
                    wait(1, libc::POLLOUT, start)?;
                    continue;
                }
                _ => return Err("Source reply pipe closed".into()),
            }
        }
        offset += count as usize;
    }
    Ok(())
}
fn entry() -> Result<(), String> {
    if std::env::args_os().skip(1).collect::<Vec<_>>() != [std::ffi::OsStr::new("--source-worker")]
        || std::env::vars_os().next().is_some()
    {
        return Err(
            "The source worker requires its exact role argument and an empty environment".into(),
        );
    }
    isolate()?;
    let start = Instant::now();
    let bytes = read_request(start)?;
    let reply = handle(&bytes);
    write_reply(reply, start)
}
fn main() {
    if let Err(reason) = entry() {
        eprintln!("{reason}");
        std::process::exit(1);
    }
}
