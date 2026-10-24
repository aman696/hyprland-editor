//! One request per process. This image never contains GTK or the Lua 5.4 image.
mod native;
mod protocol;
// Reuse the reviewed closed policies; the other roles are not callable through
// this executable's protocol. Legacy parser/capture policies are unchanged.
#[allow(dead_code)]
#[path = "../../editor-app/src/process_policy.rs"]
mod process_policy;
use protocol::*;
use sha2::{Digest, Sha256};
use std::{
    io,
    time::{Duration, Instant},
};
const DEADLINE: Duration = Duration::from_secs(3);

fn isolate() -> Result<(), String> {
    for (resource, max) in [
        (libc::RLIMIT_AS, 128 * 1024 * 1024),
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
            return Err("Cannot set compiler resource limits".into());
        }
    }
    if unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) } != 0 {
        return Err("Cannot close inherited compiler descriptors".into());
    }
    for fd in [0, 1] {
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err("Cannot prepare bounded compiler pipes".into());
        }
    }
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err("Cannot disable compiler privilege escalation".into());
    }
    process_policy::compiler()
}
fn wait(fd: i32, events: i16, start: Instant) -> Result<(), String> {
    loop {
        let left = DEADLINE
            .checked_sub(start.elapsed())
            .ok_or("Compiler pipe deadline exceeded")?;
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
            return Err("Compiler pipe wait failed".into());
        }
    }
}
fn read_request(start: Instant) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16384];
    loop {
        if start.elapsed() >= DEADLINE {
            return Err("Compiler input deadline exceeded".into());
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
                _ => return Err("Cannot read compiler input".into()),
            }
        }
        if bytes.len().saturating_add(count as usize) > MAX_WIRE {
            return Err("Compiler input exceeds 4 MiB".into());
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
        outcome: Outcome::error("Invalid compiler request"),
    };
    let Ok(request) = serde_json::from_slice::<Request>(bytes) else {
        return reply;
    };
    reply.nonce = Some(request.nonce);
    if request.protocol != PROTOCOL {
        reply.outcome = Outcome::error("Unknown compiler protocol");
        return reply;
    }
    let profile = match native::profile() {
        Ok(profile) => profile,
        Err(reason) => {
            reply.outcome = Outcome::error(reason);
            return reply;
        }
    };
    if request.expected_release != profile.release {
        reply.profile = Some(profile);
        reply.outcome = Outcome::error("Compiler release differs from the explicit request");
        return reply;
    }
    reply.profile = Some(profile);
    let result = match request.operation {
        Operation::Profile => Ok(Outcome::Profile),
        Operation::Compile { source, load_name } => native::compile(&source, &load_name)
            .map(|(valid, diagnostic)| Outcome::Compiled { valid, diagnostic }),
        Operation::Convert { values } => {
            native::convert(&values).map(|values| Outcome::Converted { values })
        }
        Operation::Probe => Ok(probe()),
    };
    reply.outcome = result.unwrap_or_else(Outcome::error);
    reply
}
struct Bounded(Vec<u8>);
impl io::Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_WIRE {
            return Err(io::Error::other("Compiler output exceeds 4 MiB"));
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
        reply.outcome = Outcome::error("Compiler result exceeds its output budget");
        serde_json::to_writer(&mut output, &reply).map_err(|_| "Cannot encode compiler refusal")?;
    }
    let mut offset = 0;
    while offset < output.0.len() {
        if start.elapsed() >= DEADLINE {
            return Err("Compiler output deadline exceeded".into());
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
                _ => return Err("Compiler reply pipe closed".into()),
            }
        }
        offset += count as usize;
    }
    Ok(())
}
fn entry() -> Result<(), String> {
    if std::env::args_os().skip(1).collect::<Vec<_>>()
        != [std::ffi::OsStr::new("--compiler-worker")]
        || std::env::vars_os().next().is_some()
    {
        return Err(
            "The compiler worker requires its exact role argument and an empty environment".into(),
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
