//! A short-lived, resource-limited parser with no filesystem, network or process-creation syscalls.
use hyprland_editor_core::{
    catalogue,
    lua::{self, LuaDocument, LuaReport},
    Draft, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const MAX_ENVELOPE: usize = 4 * MAX_DOCUMENT_BYTES;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Request {
    Parse {
        source: String,
        version: String,
    },
    Copy {
        source: String,
        report: Box<LuaReport>,
        draft: Draft,
    },
    InspectDependencies {
        source: String,
        version: String,
    },
    ReplayModules(lua::replay::Program),
    ReviewSourceEdits {
        program: lua::replay::Program,
        plan: lua::replay::edits::EditPlan,
    },
    DecodePrimitives(Vec<lua::scalar_codec::Case>),
    InspectConfiguration {
        events: Vec<lua::replay::Event>,
        version: String,
    },
    PlanModules {
        context: hyprland_editor_core::module_path::Context,
        #[serde(default)]
        mode: hyprland_editor_core::module_path::RequireMode,
        modules: Vec<Vec<u8>>,
        version: String,
    },
    ParseRegistry {
        xml: String,
    },
    CheckRegex(hyprland_editor_core::regex::Sample),
    Probe,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Response {
    Parsed(Box<LuaReport>),
    Dependencies(lua::DependencyInventory),
    Replayed(Box<lua::replay::Report>),
    SourceEditReview(Box<lua::replay::edits::EditReview>),
    Primitives {
        profile: lua::replay::CompilerProfile,
        numeric_locale: String,
        outcomes: Vec<lua::scalar_codec::Outcome>,
    },
    Configuration(lua::config_inspection::Inspection),
    ModulePlans(Vec<hyprland_editor_core::module_path::Plan>),
    Registry(hyprland_editor_core::xkb::Registry),
    Copy(String),
    RegexMatch(bool),
    Probe {
        file_denied: bool,
        network_denied: bool,
    },
    Error(String),
}
fn isolate() -> Result<(), String> {
    // All restrictions apply only to this child, after dynamic library startup.
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
            return Err("Cannot install parser resource limits".into());
        }
    }
    if unsafe { libc::syscall(libc::SYS_close_range, 3_u32, u32::MAX, 0_u32) } != 0 {
        return Err("Cannot close inherited parser capabilities".into());
    }
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err("Cannot disable parser privilege escalation".into());
    }
    super::process_policy::parser()?;
    Ok(())
}
pub fn entry() {
    let response = (|| {
        isolate()?;
        let mut bytes = vec![];
        std::io::stdin()
            .take(MAX_ENVELOPE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read parser request")?;
        if bytes.len() > MAX_ENVELOPE {
            return Err("Parser request exceeds 4 MiB".into());
        }
        let request: Request =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid parser request")?;
        // Conversion/replay-only requests do not need another parsed catalogue.
        let c = std::sync::LazyLock::new(catalogue);
        match request {
            Request::InspectConfiguration { events, version } => {
                lua::config_inspection::inspect(&events, &version).map(Response::Configuration)
            }
            Request::DecodePrimitives(cases) => {
                lua::scalar_codec::decode_batch(&cases).map(|outcomes| Response::Primitives {
                    profile: lua::replay::compiler_profile(),
                    numeric_locale: "C".into(),
                    outcomes,
                })
            }
            Request::ReplayModules(program) => {
                lua::replay::replay(&program).map(|report| Response::Replayed(Box::new(report)))
            }
            Request::ReviewSourceEdits { program, plan } => {
                lua::replay::edits::review(&program, &plan)
                    .map(|report| Response::SourceEditReview(Box::new(report)))
            }
            Request::InspectDependencies { source, version } => {
                lua::inspect_dependencies(&source, &version).map(Response::Dependencies)
            }
            Request::PlanModules {
                context,
                mode,
                modules,
                version,
            } => hyprland_editor_core::module_path::plan_requests_for(
                &context, &modules, &version, mode,
            )
            .map(Response::ModulePlans),
            Request::ParseRegistry { xml } => {
                hyprland_editor_core::xkb::parse(&xml).map(Response::Registry)
            }
            Request::Parse { source, version } => {
                lua::parse(&source, &version, &c).map(|report| Response::Parsed(Box::new(report)))
            }
            Request::Copy {
                source,
                report,
                draft,
            } => LuaDocument::from_report(source, *report, &c)?
                .edited_copy(&draft, &c)
                .map(Response::Copy),
            Request::CheckRegex(sample) => {
                hyprland_editor_core::regex::check(&sample).map(Response::RegexMatch)
            }
            Request::Probe => {
                let file = unsafe { libc::open(c"/etc/passwd".as_ptr(), libc::O_RDONLY) };
                let file_denied =
                    file < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
                if file >= 0 {
                    unsafe { libc::close(file) };
                }
                let socket = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
                let network_denied = socket < 0
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
                if socket >= 0 {
                    unsafe { libc::close(socket) };
                }
                Ok(Response::Probe {
                    file_denied,
                    network_denied,
                })
            }
        }
    })()
    .unwrap_or_else(Response::Error);
    let bytes = serde_json::to_vec(&response)
        .unwrap_or_else(|_| b"{\"Error\":\"Cannot encode parser result\"}".to_vec());
    let _ = std::io::stdout().write_all(&bytes);
}
pub fn run(request: Request) -> Result<Response, String> {
    let bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_ENVELOPE {
        return Err("Parser request exceeds 4 MiB".into());
    }
    #[cfg(not(test))]
    let exe = "/proc/self/exe";
    // Rust's test harness owns main(), so it cannot accept the worker CLI switch.
    // Native tests execute the normal Cargo-built application through a held FD.
    // This adapter exists only in test builds; production always pins self above.
    #[cfg(test)]
    let held_test_executable = {
        use std::os::unix::fs::OpenOptionsExt;
        let path = std::env::current_exe().map_err(|e| e.to_string())?;
        let target = path
            .parent()
            .and_then(std::path::Path::parent)
            .ok_or("Test executable is outside the Cargo target layout")?;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(target.join("hyprland-editor"))
            .map_err(|e| format!("Build the application before native worker tests: {e}"))?
    };
    #[cfg(test)]
    let exe = {
        use std::os::fd::AsRawFd;
        format!("/proc/self/fd/{}", held_test_executable.as_raw_fd())
    };
    let mut child = Command::new(exe)
        .arg("--lua-source-worker")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(&bytes));
    let reader = std::thread::spawn(move || {
        let mut bytes = vec![];
        output
            .take(MAX_ENVELOPE as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Ok(s),
            Ok(None) if start.elapsed() < Duration::from_secs(3) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err("Parser exceeded its wall-clock budget".into());
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(e.to_string());
            }
        }
    };
    let _ = writer.join();
    let bytes = reader
        .join()
        .map_err(|_| "Parser result reader failed")?
        .map_err(|e| e.to_string())?;
    if !status?.success() {
        return Err("Isolated parser exited unsuccessfully".into());
    }
    if bytes.len() > MAX_ENVELOPE {
        return Err("Parser result exceeds 4 MiB".into());
    }
    let response = serde_json::from_slice(&bytes).map_err(|_| "Invalid parser result")?;
    match response {
        Response::Error(e) => Err(e),
        other => Ok(other),
    }
}
