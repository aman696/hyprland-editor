//! Pending resolution is captured data, never saved acceptance or folder authority.
use super::*;
use hyprland_editor_core::{
    lua,
    workspace::{SourceLanguage, SourceSnapshot, MAX_SOURCE_DOCUMENTS},
    workspace_graph::{self, Bundle, View},
};

pub(crate) struct Pending {
    bundle: Bundle,
    program: Program,
    report: replay::Report,
    fingerprint: String,
}
impl Pending {
    pub(crate) fn bundle(&self) -> &Bundle {
        &self.bundle
    }
    pub(crate) fn program(&self) -> &Program {
        &self.program
    }
    pub(crate) fn report(&self) -> &replay::Report {
        &self.report
    }
    pub(crate) fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub(crate) fn request(&self) -> Option<&replay::Request> {
        match &self.report.halt {
            Some(Halt::NeedsResolution { request }) => Some(request),
            _ => None,
        }
    }
    /// Pure read-only materialization, including inactive archived data. This
    /// does not activate it in the candidate or grant any inherited binding.
    pub(crate) fn preserved_program(&self) -> Result<Program, String> {
        let original = self.bundle.graph.program(
            &self.bundle.sources,
            Some(&self.bundle.graph.root),
            &self.program.version,
        )?;
        let mut inputs = self.bundle.graph.private.clone();
        inputs.archived.clear();
        inputs.pending_program(&original, &self.bundle.graph.revisions)
    }
}

pub(crate) struct CapturedInput {
    pending: String,
    request: replay::Request,
    source: Arc<CapturedSource>,
    load_name: Vec<u8>,
    loader_data: Vec<u8>,
    probes: Vec<ProbeEvidence>,
}
impl CapturedInput {
    pub(crate) fn source(&self) -> &CapturedSource {
        &self.source
    }
    pub(crate) fn load_name(&self) -> &[u8] {
        &self.load_name
    }
    pub(crate) fn probe_count(&self) -> usize {
        self.probes.len()
    }
}
pub(crate) enum InputChoice {
    Capture(Arc<CapturedInput>),
    /// Explicitly use the current private view, including its private pieces.
    /// Neither its historical path nor byte equality proves current existence.
    Preserved(String),
    /// An explicit modeling assumption, never an automatic absence inference.
    Missing,
}

pub(crate) fn inspect_pending(
    bundle: &Bundle,
    version: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<Pending, String> {
    let Original {
        program: original,
        mut budget,
        program_cost,
        ..
    } = super::original(bundle, version, cancelled)?;
    let expected = bundle
        .graph
        .private
        .pending_program(&original, &bundle.graph.revisions)?;
    let worker::Response::SourceRevisionProgram(program) = worker::run_cancellable(
        worker::Request::InspectPrivateRequests {
            original,
            revisions: bundle.graph.revisions.clone(),
            inputs: bundle.graph.private.clone(),
        },
        &budget.cancelled,
    )?
    else {
        return Err("Unexpected pending source materialization response".into());
    };
    let program_hash = workspace_graph::program_fingerprint(&expected)?;
    if workspace_graph::program_fingerprint(&program)? != program_hash {
        return Err("Pending source worker changed the requested program".into());
    }
    budget.retained = budget.retained.saturating_sub(program_cost);
    budget.retain(&program)?;
    budget.check()?;
    let worker::Response::Replayed(report) = worker::run_cancellable(
        worker::Request::ReplayModules(program.clone()),
        &budget.cancelled,
    )?
    else {
        return Err("Unexpected pending source replay response".into());
    };
    check_report(&program, &report, &bundle.graph.compiler)?;
    budget.retain(report.as_ref())?;
    budget.check()?;
    let request = match &report.halt {
        Some(Halt::NeedsResolution { request }) => Some(request),
        _ => None,
    };
    let pin = serde_json::to_string(&(
        "pending-private-input-1",
        bundle.fingerprint()?,
        program_hash,
        request,
    ))
    .map_err(|e| e.to_string())?;
    Ok(Pending {
        bundle: bundle.clone(),
        program,
        report: *report,
        fingerprint: lua::source_hash(&pin),
    })
}
fn fresh(pending: &Pending, cancelled: Arc<AtomicBool>) -> Result<Pending, String> {
    let current = inspect_pending(&pending.bundle, &pending.program.version, cancelled)?;
    if current.fingerprint != pending.fingerprint || current.request() != pending.request() {
        return Err("Pending request changed; inspect it again".into());
    }
    if current.request().is_none() {
        return Err("No observed request is waiting for a resolution input".into());
    }
    Ok(current)
}
pub(crate) fn capture_input(
    pending: &Pending,
    grants: Vec<Arc<directory_worker::Grant>>,
    cancelled: Arc<AtomicBool>,
) -> Result<CapturedInput, String> {
    if grants.len() > 8 {
        return Err("At most eight temporary folders may resolve private inputs".into());
    }
    let current = fresh(pending, cancelled.clone())?;
    let request = current.request().unwrap().clone();
    let mut resolver = Resolver {
        grants: &grants,
        context: &current.bundle.graph.context,
        version: &current.program.version,
        budget: Budget {
            start: Instant::now(),
            probes: 0,
            retained: 0,
            cancelled,
            limit: 8 * MAX_DOCUMENT_BYTES,
        },
    };
    let mut probes = vec![];
    let capture = resolver.resolve(&request, &mut probes)?;
    resolver.budget.retain(capture.source.as_ref())?;
    resolver.budget.check()?;
    Ok(CapturedInput {
        pending: current.fingerprint,
        request,
        source: capture.source,
        load_name: capture.load_name,
        loader_data: capture.loader_data,
        probes,
    })
}
pub(crate) fn accept_input(
    pending: &Pending,
    choice: InputChoice,
    cancelled: Arc<AtomicBool>,
) -> Result<Pending, String> {
    let current = fresh(pending, cancelled.clone())?;
    let request = current.request().unwrap().clone();
    let mut bundle = current.bundle.clone();
    if bundle.graph.private.bindings.len() >= 128 {
        return Err("Private resolution input limit reached".into());
    }
    let resolution = match choice {
        InputChoice::Missing => Resolution::Missing,
        InputChoice::Preserved(document) => {
            let view = current
                .bundle
                .graph
                .views
                .iter()
                .chain(current.bundle.graph.private.views.iter())
                .find(|d| d.document == document)
                .ok_or("Preserved source view missing")?;
            bundle.graph.private.archived.remove(&document);
            Resolution::Loaded {
                document,
                loader_data: if matches!(request.loader, Loader::Dofile | Loader::Loadfile) {
                    vec![]
                } else {
                    view.load_name.clone()
                },
            }
        }
        InputChoice::Capture(capture) => {
            if capture.pending != current.fingerprint || capture.request != request {
                return Err("Captured input belongs to a different pending request".into());
            }
            capture.source.identity.validate(&capture.source.content)?;
            // Never substitute an edited private view for a fresh capture,
            // even when its resulting text happens to equal the captured bytes.
            let existing = bundle
                .sources
                .iter()
                .chain(bundle.graph.private.sources.iter())
                .find(|s| {
                    s.origin == capture.source.identity
                        && s.content == capture.source.content
                        && !bundle.graph.revisions.iter().any(|r| r.document == s.id)
                        && bundle
                            .graph
                            .views
                            .iter()
                            .chain(bundle.graph.private.views.iter())
                            .any(|v| v.document == s.id && v.load_name == capture.load_name)
                })
                .map(|s| s.id.clone());
            let document = if let Some(id) = existing {
                bundle.graph.private.archived.remove(&id);
                id
            } else {
                if bundle.sources.len() + bundle.graph.private.sources.len() >= MAX_SOURCE_DOCUMENTS
                {
                    return Err("Private capture exceeds the complete graph document budget".into());
                }
                let used = bundle
                    .sources
                    .iter()
                    .chain(bundle.graph.private.sources.iter())
                    .map(|s| s.id.as_str())
                    .collect::<std::collections::BTreeSet<_>>();
                let id = (1..=MAX_SOURCE_DOCUMENTS)
                    .map(|n| format!("private-{n}"))
                    .find(|id| !used.contains(id.as_str()))
                    .ok_or("No private source ID available")?;
                Arc::make_mut(&mut bundle.graph.private.sources).push(SourceSnapshot {
                    id: id.clone(),
                    language: SourceLanguage::Lua,
                    content: capture.source.content.clone(),
                    origin: capture.source.identity.clone(),
                });
                bundle.graph.private.views.push(View {
                    document: id.clone(),
                    load_name: capture.load_name.clone(),
                });
                id
            };
            Resolution::Loaded {
                document,
                loader_data: capture.loader_data.clone(),
            }
        }
    };
    if bundle
        .graph
        .private
        .bindings
        .iter()
        .any(|b| b.request == request)
    {
        return Err("This request already has a private resolution input".into());
    }
    bundle.graph.private.bindings.push(Binding {
        request,
        resolution,
    });
    bundle.fingerprint()?;
    inspect_pending(&bundle, &current.program.version, cancelled)
}

#[cfg(test)]
mod tests;
