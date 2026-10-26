//! A short-lived, resource-limited parser with no filesystem, network or process-creation syscalls.
use hyprland_editor_core::{
    catalogue,
    lua::{self, LuaDocument, LuaReport},
    Draft, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
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
    CopyWorkspace {
        workspace: Box<hyprland_editor_core::workspace::Workspace>,
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
    ReviewSourceMerge {
        base: String,
        private: String,
        incoming: String,
    },
    ResolveSourceMerge {
        base: String,
        private: String,
        incoming: String,
        review: hyprland_editor_core::source_merge::Review,
        choices: std::collections::BTreeMap<usize, hyprland_editor_core::source_merge::Choice>,
    },
    InspectStructuredDeclaration {
        program: lua::replay::Program,
        event_order: usize,
    },
    InsertStructuredDeclarations {
        program: lua::replay::Program,
        document: String,
        api: String,
        target: Box<hyprland_editor_core::workspace::Workspace>,
    },
    CompareStructuredDeclaration {
        program: lua::replay::Program,
        event_order: usize,
        target: Box<hyprland_editor_core::workspace::Workspace>,
    },
    CompareScalarSource {
        program: lua::replay::Program,
        change: lua::replay::scalar_edit::Change,
    },
    CompareMonitorSource {
        program: lua::replay::Program,
        change: lua::replay::monitor_edit::Change,
    },
    CompareMonitorFieldSource {
        program: lua::replay::Program,
        change: lua::replay::monitor_edit::FieldChange,
    },
    InspectMonitorDeclarations {
        program: lua::replay::Program,
        document: String,
    },
    CompareMonitorDeclaration {
        program: lua::replay::Program,
        change: lua::replay::monitor_edit::declarations::Change,
    },
    ReviewSourceOverrides {
        program: lua::replay::Program,
        draft: Draft,
    },
    ReviewSourceMonitors {
        program: lua::replay::Program,
        monitors: Vec<hyprland_editor_core::monitor::Rule>,
    },
    ResolveSourceMonitors {
        program: lua::replay::Program,
        monitors: Vec<hyprland_editor_core::monitor::Rule>,
        review: lua::replay::monitor_review::Review,
        choice: lua::replay::monitor_review::Choice,
    },
    ResolveSourceOverrides {
        program: lua::replay::Program,
        draft: Draft,
        review: lua::replay::override_review::Review,
        choices: std::collections::BTreeMap<String, lua::replay::override_review::Choice>,
    },
    MaterializeSourceRevisions {
        base: lua::replay::Program,
        revisions: Vec<hyprland_editor_core::workspace_graph::revisions::Revision>,
    },
    MaterializePrivateInputs {
        original: lua::replay::Program,
        revisions: Vec<hyprland_editor_core::workspace_graph::revisions::Revision>,
        inputs: hyprland_editor_core::workspace_graph::private::Inputs,
    },
    InspectPrivateRequests {
        original: lua::replay::Program,
        revisions: Vec<hyprland_editor_core::workspace_graph::revisions::Revision>,
        inputs: hyprland_editor_core::workspace_graph::private::Inputs,
    },
    ApplySourceRevision {
        base: lua::replay::Program,
        revisions: Vec<hyprland_editor_core::workspace_graph::revisions::Revision>,
        plan: lua::replay::edits::EditPlan,
    },
    DecodePrimitives(Vec<lua::scalar_codec::Case>),
    InspectConfiguration {
        events: Vec<lua::replay::Event>,
        version: String,
    },
    InspectMonitors {
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
    SourceMergeReview(Box<hyprland_editor_core::source_merge::Review>),
    SourceMergeResolution(Box<hyprland_editor_core::source_merge::Resolution>),
    ScalarSourceComparison(Box<lua::replay::scalar_edit::Comparison>),
    SourceOverrideReview(Box<lua::replay::override_review::Review>),
    SourceMonitorReview(Box<lua::replay::monitor_review::Review>),
    SourceMonitorResolution(Box<lua::replay::monitor_review::Resolution>),
    SourceOverrideResolution(Box<lua::replay::override_review::Resolution>),
    SourceRevisionProgram(lua::replay::Program),
    SourceRevisions(Vec<hyprland_editor_core::workspace_graph::revisions::Revision>),
    Primitives {
        profile: lua::replay::CompilerProfile,
        numeric_locale: String,
        outcomes: Vec<lua::scalar_codec::Outcome>,
    },
    Configuration(lua::config_inspection::Inspection),
    Monitors(lua::monitor_inspection::Inspection),
    MonitorDeclarationSites(lua::replay::monitor_edit::declarations::Inventory),
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
            Request::InsertStructuredDeclarations {
                program,
                document,
                api,
                target,
            } => lua::replay::declaration_edit::insert(
                &program,
                &document,
                &lua::replay::declaration_edit::Anchor::End,
                &api,
                &target,
                &c,
            )
            .map(|result| Response::ScalarSourceComparison(Box::new(result))),
            Request::InspectStructuredDeclaration {
                program,
                event_order,
            } => lua::replay::declaration_edit::inspect(&program, event_order, &c)
                .map(|report| Response::Parsed(Box::new(report))),
            Request::CompareStructuredDeclaration {
                program,
                event_order,
                target,
            } => lua::replay::declaration_edit::compare(&program, event_order, &target, &c)
                .map(|result| Response::ScalarSourceComparison(Box::new(result))),
            Request::InspectMonitorDeclarations { program, document } => {
                lua::replay::monitor_edit::declarations::inspect(&program, &document)
                    .map(Response::MonitorDeclarationSites)
            }
            Request::CompareMonitorDeclaration { program, change } => {
                lua::replay::monitor_edit::declarations::compare(&program, &change)
                    .map(|result| Response::ScalarSourceComparison(Box::new(result)))
            }
            Request::InspectMonitors { events, version } => {
                lua::monitor_inspection::inspect(&events, &version).map(Response::Monitors)
            }
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
            Request::MaterializeSourceRevisions { base, revisions } => {
                let program =
                    hyprland_editor_core::workspace_graph::revisions::program(base, &revisions)?;
                for revision in &revisions {
                    let doc = program
                        .documents
                        .iter()
                        .find(|d| d.id == revision.document)
                        .ok_or("Unknown private source revision")?;
                    lua::replay::validate_document(doc)?;
                }
                Ok(Response::SourceRevisionProgram(program))
            }
            Request::MaterializePrivateInputs {
                original,
                revisions,
                inputs,
            } => {
                let program = inputs.program(&original, &revisions)?;
                for revision in &revisions {
                    if let Some(doc) = program.documents.iter().find(|d| d.id == revision.document)
                    {
                        lua::replay::validate_document(doc)?;
                    }
                }
                Ok(Response::SourceRevisionProgram(program))
            }
            Request::InspectPrivateRequests {
                original,
                revisions,
                inputs,
            } => {
                let program = inputs.pending_program(&original, &revisions)?;
                for revision in &revisions {
                    if let Some(doc) = program.documents.iter().find(|d| d.id == revision.document)
                    {
                        lua::replay::validate_document(doc)?;
                    }
                }
                Ok(Response::SourceRevisionProgram(program))
            }
            Request::ApplySourceRevision {
                base,
                revisions,
                plan,
            } => hyprland_editor_core::workspace_graph::revisions::apply(&base, &revisions, &plan)
                .map(Response::SourceRevisions),
            Request::CompareScalarSource { program, change } => {
                lua::replay::scalar_edit::compare(&program, &change)
                    .map(|result| Response::ScalarSourceComparison(Box::new(result)))
            }
            Request::CompareMonitorSource { program, change } => {
                lua::replay::monitor_edit::compare(&program, &change)
                    .map(|result| Response::ScalarSourceComparison(Box::new(result)))
            }
            Request::CompareMonitorFieldSource { program, change } => {
                lua::replay::monitor_edit::compare_field(&program, &change)
                    .map(|result| Response::ScalarSourceComparison(Box::new(result)))
            }
            Request::ReviewSourceOverrides { program, draft } => {
                lua::replay::override_review::prepare(&program, &draft)
                    .map(|result| Response::SourceOverrideReview(Box::new(result)))
            }
            Request::ReviewSourceMonitors { program, monitors } => {
                lua::replay::monitor_review::prepare(&program, &monitors)
                    .map(|result| Response::SourceMonitorReview(Box::new(result)))
            }
            Request::ResolveSourceMonitors {
                program,
                monitors,
                review,
                choice,
            } => lua::replay::monitor_review::resolve(&program, &monitors, &review, &choice)
                .map(|result| Response::SourceMonitorResolution(Box::new(result))),
            Request::ResolveSourceOverrides {
                program,
                draft,
                review,
                choices,
            } => lua::replay::override_review::resolve(&program, &draft, &review, &choices)
                .map(|result| Response::SourceOverrideResolution(Box::new(result))),
            Request::ReviewSourceEdits { program, plan } => {
                lua::replay::edits::review(&program, &plan)
                    .map(|report| Response::SourceEditReview(Box::new(report)))
            }
            Request::ReviewSourceMerge {
                base,
                private,
                incoming,
            } => hyprland_editor_core::source_merge::prepare(&base, &private, &incoming)
                .map(|result| Response::SourceMergeReview(Box::new(result))),
            Request::ResolveSourceMerge {
                base,
                private,
                incoming,
                review,
                choices,
            } => hyprland_editor_core::source_merge::resolve(
                &base, &private, &incoming, &review, &choices,
            )
            .map(|result| Response::SourceMergeResolution(Box::new(result))),
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
            Request::CopyWorkspace { workspace } => {
                lua::workspace_copy::prepare(&workspace, &c).map(Response::Copy)
            }
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
    run_cancellable(request, &AtomicBool::new(false))
}
pub fn run_cancellable(request: Request, cancelled: &AtomicBool) -> Result<Response, String> {
    cancelled_check(cancelled)?;
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
    let mut command = Command::new(exe);
    command.arg("--lua-source-worker").env_clear();
    execute(command, bytes, cancelled, |_| {})
}
fn cancelled_check(cancelled: &AtomicBool) -> Result<(), String> {
    if cancelled.load(Ordering::Acquire) {
        Err("Parser operation was cancelled".into())
    } else {
        Ok(())
    }
}
struct OwnedChild {
    child: Child,
    reaped: bool,
}
impl OwnedChild {
    fn stop(&mut self) -> Result<(), String> {
        if self.reaped {
            return Ok(());
        }
        // The Child value is the sole process authority. No saved PID or path
        // is used to find or signal another process.
        let _ = self.child.kill();
        self.child
            .wait()
            .map_err(|e| format!("Could not reap parser child: {e}"))?;
        self.reaped = true;
        Ok(())
    }
    fn poll(&mut self) -> Result<Option<ExitStatus>, String> {
        let status = self.child.try_wait().map_err(|e| e.to_string())?;
        if status.is_some() {
            self.reaped = true;
        }
        Ok(status)
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
fn execute(
    mut command: Command,
    bytes: Vec<u8>,
    cancelled: &AtomicBool,
    on_spawn: impl FnOnce(u32),
) -> Result<Response, String> {
    cancelled_check(cancelled)?;
    let child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut child = OwnedChild {
        child,
        reaped: false,
    };
    on_spawn(child.child.id());
    let mut input = child
        .child
        .stdin
        .take()
        .ok_or("Parser input pipe missing")?;
    let output = child
        .child
        .stdout
        .take()
        .ok_or("Parser output pipe missing")?;
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
        if let Err(reason) = cancelled_check(cancelled) {
            break child.stop().and(Err(reason));
        }
        match child.poll() {
            Ok(Some(s)) => break Ok(s),
            Ok(None) if start.elapsed() < Duration::from_secs(3) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Ok(None) => {
                break child
                    .stop()
                    .and(Err("Parser exceeded its wall-clock budget".into()));
            }
            Err(e) => {
                break child.stop().and(Err(e));
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
    cancelled_check(cancelled)?;
    if bytes.len() > MAX_ENVELOPE {
        return Err("Parser result exceeds 4 MiB".into());
    }
    let response = serde_json::from_slice(&bytes).map_err(|_| "Invalid parser result")?;
    match response {
        Response::Error(e) => Err(e),
        other => Ok(other),
    }
}

#[cfg(test)]
mod cancellation_tests;
