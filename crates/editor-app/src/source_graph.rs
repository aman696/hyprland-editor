//! Sequential authorized captures feeding the no-filesystem evaluator.
//! Never an atomic filesystem snapshot or a claim of effective compositor state.
use super::{directory_worker, worker};
use hyprland_editor_core::{
    lua::{
        config_inspection, config_table,
        replay::{self, Binding, Document, Halt, Loader, Program, Resolution},
    },
    module_path::{self, Context, Primary, RequireMode},
    source::{CapturedSource, SourceIdentity},
    source_scope::{Identity, Observation, Status},
    MAX_DOCUMENT_BYTES,
};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[derive(Serialize)]
enum ProbeOutcome {
    Observed(Observation),
    Rejected(hyprland_editor_core::source_scope::Failure),
    Unavailable(String),
}
#[derive(Serialize)]
pub(super) struct ProbeEvidence {
    folder: Identity,
    candidate: Vec<u8>,
    outcome: ProbeOutcome,
}
#[derive(Serialize)]
pub(super) struct EdgeEvidence {
    request: replay::Request,
    probes: Vec<ProbeEvidence>,
    document: Option<String>,
    blocked: Option<String>,
}
pub(super) struct CapturedDocument {
    pub(super) id: String,
    pub(super) source: Arc<CapturedSource>,
}
pub(super) struct Analysis {
    pub(super) restored: bool,
    pub(super) context: Context,
    pub(super) program: Program,
    pub(super) report: replay::Report,
    pub(super) config_batches: Vec<config_table::Batch>,
    pub(super) config_profile: Option<config_inspection::ModelContext>,
    pub(super) captures: Vec<CapturedDocument>,
    pub(super) edges: Vec<EdgeEvidence>,
    pub(super) blocked: Option<String>,
    pub(super) retained_bytes: usize,
}
struct Budget {
    start: Instant,
    probes: usize,
    retained: usize,
    cancelled: Arc<AtomicBool>,
    limit: usize,
}
impl Budget {
    fn check(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err("Dependency analysis was cancelled".into());
        }
        if self.start.elapsed() > Duration::from_secs(10) {
            return Err("Dependency analysis reached its time budget".into());
        }
        Ok(())
    }
    fn retain(&mut self, value: &impl Serialize) -> Result<(), String> {
        struct Count(usize, usize);
        impl std::io::Write for Count {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 = self.0.saturating_add(bytes.len());
                if self.0 > self.1 {
                    return Err(std::io::Error::other(
                        "Dependency retained-data budget exceeded",
                    ));
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut count = Count(self.retained, self.limit);
        serde_json::to_writer(&mut count, value).map_err(|e| e.to_string())?;
        self.retained = count.0;
        Ok(())
    }
}
struct Resolver<'a> {
    grants: &'a [Arc<directory_worker::Grant>],
    context: &'a Context,
    version: &'a str,
    budget: Budget,
}
struct LoadedCapture {
    source: Arc<CapturedSource>,
    load_name: Vec<u8>,
    loader_data: Vec<u8>,
}
impl Resolver<'_> {
    fn grant(&self, path: &[u8]) -> Result<&directory_worker::Grant, String> {
        if !path.starts_with(b"/") {
            return Err("A relative dependency needs its captured working directory before folder selection".into());
        }
        let mut selected: Option<(&directory_worker::Grant, usize)> = None;
        let mut ambiguous = false;
        for grant in self.grants {
            if grant.identity().candidate_path(path).is_err() {
                continue;
            }
            let score = [
                &grant.identity().selected_path,
                &grant.identity().resolved_path,
            ]
            .iter()
            .filter(|root| {
                path.starts_with(root.as_slice())
                    && (path.len() == root.len()
                        || root.ends_with(b"/")
                        || path.get(root.len()) == Some(&b'/'))
            })
            .map(|root| root.len())
            .max()
            .unwrap_or(0);
            if let Some((prior, best)) = selected {
                if score == best && prior.identity() != grant.identity() {
                    ambiguous = true;
                }
                if score <= best {
                    continue;
                }
            }
            selected = Some((grant.as_ref(), score));
            ambiguous = false;
        }
        if ambiguous {
            return Err("Overlapping folder aliases need an unambiguous selection".into());
        }
        selected
            .map(|(g, _)| g)
            .ok_or("A dependency needs a separately approved folder".into())
    }
    fn probe(
        &mut self,
        path: &[u8],
        evidence: &mut Vec<ProbeEvidence>,
    ) -> Result<Observation, String> {
        self.budget.check()?;
        if self.budget.probes >= 256 {
            return Err("Dependency metadata observation limit exceeded".into());
        }
        self.budget.probes += 1;
        let grant = self.grant(path)?;
        let result = directory_worker::probe(grant, path);
        let outcome = match &result {
            Ok(Ok(o)) => ProbeOutcome::Observed(o.clone()),
            Ok(Err(e)) => ProbeOutcome::Rejected(e.clone()),
            Err(e) => ProbeOutcome::Unavailable(e.clone()),
        };
        let record = ProbeEvidence {
            folder: grant.identity().clone(),
            candidate: path.to_vec(),
            outcome,
        };
        self.budget.retain(&record)?;
        evidence.push(record);
        result?.map_err(|e| format!("Dependency observation blocked: {:?}: {}", e.kind, e.detail))
    }
    fn absolute(&self, path: &[u8], cwd: Option<&[u8]>) -> Result<Vec<u8>, String> {
        if path.starts_with(b"/") {
            return Ok(path.to_vec());
        }
        let cwd = cwd
            .filter(|p| p.starts_with(b"/") && !p.contains(&0))
            .ok_or("A relative dependency needs its captured working directory")?;
        let joined = [cwd, if cwd.ends_with(b"/") { b"" } else { b"/" }, path].concat();
        if joined.len() > 4096 {
            return Err("Dependency pathname exceeds 4096 bytes".into());
        }
        Ok(joined)
    }
    fn capture(
        &mut self,
        path: &[u8],
        evidence: &mut Vec<ProbeEvidence>,
    ) -> Result<Arc<CapturedSource>, String> {
        let before = self.probe(path, evidence)?;
        let Status::Present { node } = &before.status else {
            return Err(
                "The selected loader file is missing; its actual loading diagnostic is unresolved"
                    .into(),
            );
        };
        if !node.is_regular() {
            return Err("A nonregular loader target needs its actual loading semantics; no content was read".into());
        }
        self.budget.check()?;
        let source = directory_worker::read(self.grant(path)?, path)?;
        let after = self.probe(path, evidence)?;
        if before != after
            || !matches_source(node, &source.identity)
            || source.identity.links != before.links
        {
            return Err("A dependency changed during byte capture; retry a stable snapshot".into());
        }
        Ok(Arc::new(source))
    }
    fn resolve(
        &mut self,
        request: &replay::Request,
        evidence: &mut Vec<ProbeEvidence>,
    ) -> Result<LoadedCapture, String> {
        if matches!(request.loader, Loader::Dofile | Loader::Loadfile) {
            let absolute = self.absolute(&request.module, request.working_directory.as_deref())?;
            return Ok(LoadedCapture {
                source: self.capture(&absolute, evidence)?,
                load_name: request.module.clone(),
                loader_data: vec![],
            });
        }
        let plan = self.context.plan_current(
            &request.module,
            self.version,
            if request.loader == Loader::SafeRequire {
                RequireMode::Safe
            } else {
                RequireMode::Original
            },
            request.package_path.as_deref(),
            request.package_path_unknown_tail,
        )?;
        match plan.primary {
            Primary::ExplicitWildcard { .. } => {
                return Err("Wildcard loading needs an ordered directory snapshot".into())
            }
            Primary::ExplicitFiles { candidates, .. } => {
                for candidate in candidates {
                    let status = self.probe(&candidate, evidence)?;
                    if matches!(status.status,Status::Present{node} if node.is_regular()) {
                        // Upstream checks the raw spelling, then loads its lexical
                        // normalization. These may identify different physical files.
                        let selected = module_path::normalized_load_path(&candidate)?;
                        return Ok(LoadedCapture {
                            source: self.capture(&selected, evidence)?,
                            load_name: selected.clone(),
                            loader_data: selected,
                        });
                    }
                }
            }
            Primary::Named => {}
        }
        for candidate in plan.lua_file_candidates {
            if candidate.is_empty() {
                continue;
            } // fopen("") cannot identify a file.
            let absolute = self.absolute(&candidate, request.working_directory.as_deref())?;
            let status = self.probe(&absolute, evidence)?;
            if matches!(status.status, Status::Absent { .. }) {
                continue;
            }
            // searchpath uses readability, not is_regular_file. Never skip a
            // directory/FIFO or denied read as though the candidate were absent.
            return Ok(LoadedCapture {
                source: self.capture(&absolute, evidence)?,
                load_name: candidate.clone(),
                loader_data: candidate,
            });
        }
        Err(if plan.search_unknown{"The known package.path prefix is exhausted; its remaining search path is unknown"}else{"Lua file candidates are exhausted; native/custom searchers and actual miss diagnostics remain unresolved"}.into())
    }
}
fn matches_source(
    node: &hyprland_editor_core::source_scope::NodeEvidence,
    source: &SourceIdentity,
) -> bool {
    node.device == source.device
        && node.inode == source.inode
        && node.owner == source.owner
        && node.mode == source.mode
        && node.size == source.size
        && node.changed_seconds == source.changed_seconds
        && node.changed_nanoseconds == source.changed_nanoseconds
        && node.modified_seconds == source.modified_seconds
        && node.modified_nanoseconds == source.modified_nanoseconds
}
pub(super) fn analyze(
    root: Arc<CapturedSource>,
    context: Context,
    grants: Vec<Arc<directory_worker::Grant>>,
    version: String,
    cwd: Option<Vec<u8>>,
    cancelled: Arc<AtomicBool>,
) -> Result<Analysis, String> {
    analyze_with_budget(
        root,
        context,
        grants,
        version,
        cwd,
        cancelled,
        8 * MAX_DOCUMENT_BYTES,
    )
}
pub(super) fn analyze_with_budget(
    root: Arc<CapturedSource>,
    context: Context,
    grants: Vec<Arc<directory_worker::Grant>>,
    version: String,
    cwd: Option<Vec<u8>>,
    cancelled: Arc<AtomicBool>,
    retained_budget: usize,
) -> Result<Analysis, String> {
    if retained_budget > 8 * MAX_DOCUMENT_BYTES {
        return Err("Invalid dependency retained-data budget".into());
    }
    if grants.len() > 8 {
        return Err("Dependency analysis allows at most eight live approved folders".into());
    }
    root.identity.validate(&root.content)?;
    if root.identity.selected_path != context.main_file {
        return Err("The selected root and module context do not match".into());
    }
    let (path, unknown) = context.initial_path(&version)?;
    let mut program = Program {
        version,
        profile: replay::Profile::HyprlandLua54,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            source: root.content.clone(),
            sha256: root.identity.sha256.clone(),
            load_name: context.main_file.clone(),
        }],
        bindings: vec![],
        initial_package_path: Some(path),
        initial_package_path_unknown_tail: unknown,
        working_directory: cwd,
    };
    let version = program.version.clone();
    let mut resolver = Resolver {
        grants: &grants,
        context: &context,
        version: &version,
        budget: Budget {
            start: Instant::now(),
            probes: 0,
            retained: 0,
            cancelled,
            limit: retained_budget,
        },
    };
    resolver.budget.retain(&program)?;
    resolver.budget.retain(&context)?;
    resolver.budget.retain(root.as_ref())?;
    let mut captures = vec![CapturedDocument {
        id: "root".into(),
        source: root,
    }];
    let mut edges = vec![];
    let mut blocked = None;
    let report = loop {
        resolver.budget.check()?;
        let worker::Response::Replayed(report) =
            worker::run(worker::Request::ReplayModules(program.clone()))?
        else {
            return Err("Unexpected dependency evaluator response".into());
        };
        resolver.budget.check()?;
        let request = match &report.halt {
            Some(Halt::NeedsResolution { request }) => request.clone(),
            _ => break *report,
        };
        let mut evidence = vec![];
        let result: Result<String, String> = (|| {
            if program.bindings.len() >= 128 {
                return Err("Dependency resolution limit exceeded".into());
            }
            let LoadedCapture {
                source,
                load_name,
                loader_data,
            } = resolver.resolve(&request, &mut evidence)?;
            let same = captures.iter().find(|c| {
                c.source.identity == source.identity
                    && program
                        .documents
                        .iter()
                        .any(|d| d.id == c.id && d.load_name == load_name)
            });
            let document = if let Some(same) = same {
                same.id.clone()
            } else {
                if program.documents.len() >= 32
                    || program
                        .documents
                        .iter()
                        .map(|d| d.source.len())
                        .sum::<usize>()
                        + source.content.len()
                        > 4 * MAX_DOCUMENT_BYTES
                {
                    return Err("Dependency source snapshot budget exceeded".into());
                }
                if captures.iter().any(|c| {
                    c.source.identity.selected_path == source.identity.selected_path
                        && c.source.identity != source.identity
                }) {
                    return Err("A source path changed revisions during dependency capture".into());
                }
                let id = format!("module-{}", program.documents.len());
                let doc = Document {
                    id: id.clone(),
                    source: source.content.clone(),
                    sha256: source.identity.sha256.clone(),
                    load_name,
                };
                resolver.budget.retain(&doc)?;
                resolver.budget.retain(source.as_ref())?;
                program.documents.push(doc);
                captures.push(CapturedDocument {
                    id: id.clone(),
                    source,
                });
                id
            };
            let binding = Binding {
                request: request.clone(),
                resolution: Resolution::Loaded {
                    document: document.clone(),
                    loader_data,
                },
            };
            resolver.budget.retain(&binding)?;
            program.bindings.push(binding);
            Ok(document)
        })();
        resolver.budget.retain(&request)?;
        match result {
            Ok(document) => edges.push(EdgeEvidence {
                request,
                probes: evidence,
                document: Some(document),
                blocked: None,
            }),
            Err(error) => {
                edges.push(EdgeEvidence {
                    request,
                    probes: evidence,
                    document: None,
                    blocked: Some(error.clone()),
                });
                blocked = Some(error);
                break *report;
            }
        }
    };
    resolver.budget.retain(&report)?;
    let mut budget = resolver.budget;
    finish(
        Analysis {
            restored: false,
            context,
            program,
            report,
            captures,
            edges,
            blocked,
            config_batches: vec![],
            config_profile: None,
            retained_bytes: 0,
        },
        &mut budget,
    )
}
fn finish(mut analysis: Analysis, budget: &mut Budget) -> Result<Analysis, String> {
    let program = &analysis.program;
    let report = &analysis.report;
    let mut blocked = analysis.blocked.take();
    let mut config_batches = Vec::new();
    let mut config_profile = None;
    let events = report
        .events
        .iter()
        .filter(|e| e.api == "hl.config")
        .cloned()
        .collect::<Vec<_>>();
    if !events.is_empty() {
        let result = (|| {
            budget.check()?;
            let expected_orders = events.iter().map(|e| e.order).collect::<Vec<_>>();
            let worker::Response::Configuration(inspection) =
                worker::run(worker::Request::InspectConfiguration {
                    events,
                    version: program.version.clone(),
                })?
            else {
                return Err("Unexpected configuration model response".to_string());
            };
            if inspection.context.version != program.version
                || inspection.context.schema_sha256
                    != hyprland_editor_core::lua::registration::schema_sha256()
                || inspection.context.profile.as_ref() != Some(&report.compiler_profile)
                || inspection.context.numeric_locale.as_deref() != Some("C")
                || inspection.context.runtime_verified
            {
                return Err("Configuration model context does not match captured analysis".into());
            }
            if inspection.batches.len() > expected_orders.len()
                || inspection.stopped.is_none() && inspection.batches.len() != expected_orders.len()
                || inspection
                    .batches
                    .iter()
                    .zip(&expected_orders)
                    .any(|(batch, order)| {
                        batch.event_order != *order
                            || batch.native_values_verified
                            || batch.order_within_call_known
                            || batch.candidates.iter().any(|c| c.event_order != *order)
                    })
            {
                return Err(
                    "Configuration model observations do not match captured call order".into(),
                );
            }
            budget.check()?;
            Ok(inspection)
        })();
        match result {
            Ok(inspection) => {
                let retain_context = budget
                    .retain(&inspection.context)
                    .and_then(|()| budget.retain(&inspection.stopped));
                if let Err(reason) = retain_context {
                    blocked.get_or_insert(reason);
                } else {
                    config_profile = Some(inspection.context);
                    if let Some(reason) = inspection.stopped {
                        blocked.get_or_insert(reason);
                    }
                    for batch in inspection.batches {
                        if let Err(reason) = budget.check().and_then(|()| budget.retain(&batch)) {
                            blocked.get_or_insert(reason);
                            break;
                        }
                        config_batches.push(batch);
                    }
                }
            }
            Err(error) => {
                blocked.get_or_insert(error);
            }
        }
    }
    analysis.config_batches = config_batches;
    analysis.config_profile = config_profile;
    analysis.blocked = blocked;
    analysis.retained_bytes = budget.retained;
    Ok(analysis)
}
pub(super) struct Saved {
    pub(super) bundle: hyprland_editor_core::workspace_graph::Bundle,
    pub(super) fingerprint: String,
    pub(super) version: String,
    pub(super) cost: usize,
}
impl Saved {
    pub(super) fn new(
        bundle: hyprland_editor_core::workspace_graph::Bundle,
        version: &str,
    ) -> Result<Self, String> {
        hyprland_editor_core::workspace::Workspace::new(hyprland_editor_core::Draft::new(version))
            .with_graph(&bundle)
            .validate(&hyprland_editor_core::catalogue())?;
        let fingerprint = bundle.fingerprint()?;
        let cost = serde_json::to_vec(&bundle)
            .map_err(|e| e.to_string())?
            .len()
            .saturating_mul(2);
        if cost > 8 * MAX_DOCUMENT_BYTES {
            return Err("Retained source graph exceeds its data budget".into());
        }
        Ok(Self {
            bundle,
            fingerprint,
            version: version.into(),
            cost,
        })
    }
}
pub(super) fn freeze(
    analysis: &Analysis,
) -> Result<hyprland_editor_core::workspace_graph::Bundle, String> {
    use hyprland_editor_core::{
        workspace::{SourceLanguage, SourceSnapshot},
        workspace_graph::{Bundle, Graph, View},
    };
    let bundle = Bundle {
        graph: Graph {
            format: "hye-workspace-graph-1".into(),
            schema_sha256: hyprland_editor_core::workspace_graph::schema_sha256(),
            compiler: analysis.report.compiler_profile.clone(),
            root: analysis.program.root.clone(),
            context: analysis.context.clone(),
            working_directory: analysis.program.working_directory.clone(),
            views: analysis
                .program
                .documents
                .iter()
                .map(|d| View {
                    document: d.id.clone(),
                    load_name: d.load_name.clone(),
                })
                .collect(),
            bindings: analysis.program.bindings.clone(),
        },
        sources: analysis
            .captures
            .iter()
            .map(|c| SourceSnapshot {
                id: c.id.clone(),
                language: SourceLanguage::Lua,
                content: c.source.content.clone(),
                origin: c.source.identity.clone(),
            })
            .collect(),
    };
    hyprland_editor_core::workspace::Workspace::new(hyprland_editor_core::Draft::new(
        &analysis.program.version,
    ))
    .with_graph(&bundle)
    .validate(&hyprland_editor_core::catalogue())?;
    bundle.fingerprint()?;
    Ok(bundle)
}
pub(super) fn restore(
    bundle: &hyprland_editor_core::workspace_graph::Bundle,
    version: &str,
) -> Result<Analysis, String> {
    let program = bundle
        .graph
        .program(&bundle.sources, Some(&bundle.graph.root), version)?;
    let mut budget = Budget {
        start: Instant::now(),
        probes: 0,
        retained: 0,
        cancelled: Arc::new(AtomicBool::new(false)),
        limit: 8 * MAX_DOCUMENT_BYTES,
    };
    budget.retain(bundle)?;
    budget.retain(&program)?;
    let worker::Response::Replayed(report) =
        worker::run(worker::Request::ReplayModules(program.clone()))?
    else {
        return Err("Unexpected saved-graph evaluator response".into());
    };
    if report.compiler_profile != bundle.graph.compiler || report.runtime_verified {
        return Err("Saved graph compiler profile differs; explicit reanalysis is required".into());
    }
    // A saved location/span is not proof of a reached loader. Rebuild and check it.
    for binding in &program.bindings {
        if !report.loads.iter().any(|load| {
            load.request == binding.request
                && match (&load.outcome, &binding.resolution) {
                    (
                        replay::LoadOutcome::SuppliedSnapshot {
                            document: a,
                            loader_data: x,
                        },
                        Resolution::Loaded {
                            document: b,
                            loader_data: y,
                        },
                    ) => a == b && x == y,
                    (replay::LoadOutcome::SuppliedMissing, Resolution::Missing) => true,
                    _ => false,
                }
        }) {
            return Err(
                "Saved graph contains an unreachable or inconsistent resolution input".into(),
            );
        }
    }
    for document in &program.documents {
        if document.id != program.root
            && !report.loads.iter().any(|load| {
                matches!(&load.outcome,
            replay::LoadOutcome::SuppliedSnapshot {document:target,..} if target==&document.id)
            })
        {
            return Err("Saved graph contains a source view that no loader reaches".into());
        }
    }
    budget.check()?;
    budget.retain(report.as_ref())?;
    let captures = bundle
        .sources
        .iter()
        .map(|source| CapturedDocument {
            id: source.id.clone(),
            source: Arc::new(CapturedSource {
                content: source.content.clone(),
                identity: source.origin.clone(),
            }),
        })
        .collect();
    finish(
        Analysis {
            restored: true,
            context: bundle.graph.context.clone(),
            program,
            report: *report,
            captures,
            edges: vec![],
            blocked: None,
            config_batches: vec![],
            config_profile: None,
            retained_bytes: 0,
        },
        &mut budget,
    )
}
pub(super) fn check(
    folder: &std::path::Path,
    root_path: &[u8],
    version: String,
    cwd: Option<Vec<u8>>,
) -> Result<serde_json::Value, String> {
    if ![
        hyprland_editor_core::BASELINE,
        hyprland_editor_core::AUDITED_MAIN,
    ]
    .contains(&version.as_str())
    {
        return Err("Dependency analysis requires an audited version".into());
    }
    let grant = Arc::new(directory_worker::approve(folder)?);
    let root = Arc::new(directory_worker::read(&grant, root_path)?);
    let context = Context {
        main_file: root.identity.selected_path.clone(),
        home: None,
        inherited_package_path: None,
    };
    let analysis = analyze(
        root,
        context,
        vec![grant],
        version,
        cwd,
        Arc::new(AtomicBool::new(false)),
    )?;
    Ok(serde_json::json!({
        "scope":"authorized-sequential-snapshot-analysis",
        "supplied_context":analysis.context,"context_runtime_verified":false,
        "model_completed":analysis.report.halt.is_none()&&analysis.blocked.is_none(),
        "documents":analysis.captures.iter().map(|c|serde_json::json!({"id":c.id,"identity":c.source.identity,"load_name":analysis.program.documents.iter().find(|d|d.id==c.id).map(|d|&d.load_name)})).collect::<Vec<_>>(),
        "resolution_evidence":analysis.edges,"report":analysis.report,"blocked":analysis.blocked,
        "configuration_tables":analysis.config_batches,"configuration_model_context":analysis.config_profile,
        "bindings":analysis.program.bindings.len(),"files_written":false,"lua_executed":false,
        "runtime_verified":false,"atomic_filesystem_snapshot":false
    }))
}
