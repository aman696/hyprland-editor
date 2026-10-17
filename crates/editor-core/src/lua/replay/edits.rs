//! Pure, source-bound patch review. A matched model is never edit or live-write authority.
use super::{
    compiler_profile, replay, validate_program, CompilerProfile, Document, Event, LoadOutcome,
    Observed, Program, Report, Request,
};
use crate::{
    lua::{source_hash, syntax_tree},
    workspace_graph, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io::Write, ops::Range};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEdit {
    pub document: String,
    pub base_sha256: String,
    pub span: Range<usize>,
    pub old_bytes_sha256: String,
    pub replacement: String,
    /// Review labels only. Supplying a label cannot authorize its feature or file.
    pub semantic_targets: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedCall {
    pub order: usize,
    pub before_arguments: Vec<Observed>,
    pub after_arguments: Vec<Observed>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditPlan {
    pub format: String,
    pub schema_sha256: String,
    pub compiler: CompilerProfile,
    pub base_program_sha256: String,
    pub edits: Vec<SourceEdit>,
    pub expected_calls: Vec<ExpectedCall>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub document: String,
    pub base_sha256: String,
    pub proposed_sha256: String,
    pub content: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifferenceKind {
    CallSiteOrApi,
    Arguments,
    MissingCall,
    AddedCall,
    Loader,
    PathHistory,
    ReturnValues,
    LoggedModuleErrors,
    IncompleteModel,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Difference {
    pub kind: DifferenceKind,
    pub order: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_arguments: Option<Vec<Observed>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_arguments: Option<Vec<Observed>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditReview {
    pub schema_sha256: String,
    pub compiler: CompilerProfile,
    pub base_program_sha256: String,
    pub plan_sha256: String,
    pub revisions: Vec<Revision>,
    pub differences: Vec<Difference>,
    pub expected_calls_matched: usize,
    pub before_model_complete: bool,
    pub after_model_complete: bool,
    pub before_halt: Option<super::Halt>,
    pub after_halt: Option<super::Halt>,
    /// Only the modeled executed observations, not all possible program effects.
    pub modeled_observations_match: bool,
    pub ownership_verified: bool,
    pub runtime_verified: bool,
}
fn fingerprint(value: &impl Serialize) -> Result<String, String> {
    struct Hash {
        hash: Sha256,
        bytes: usize,
    }
    impl Write for Hash {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes = self.bytes.saturating_add(bytes.len());
            if self.bytes > 4 * MAX_DOCUMENT_BYTES {
                return Err(std::io::Error::other("patch envelope limit"));
            }
            self.hash.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Hash {
        hash: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut writer, value)
        .map_err(|_| "Patch input/output exceeds its 4 MiB envelope".to_string())?;
    Ok(format!("{:x}", writer.hash.finalize()))
}
pub fn prepare(
    program: &Program,
    mut edits: Vec<SourceEdit>,
    mut expected_calls: Vec<ExpectedCall>,
) -> Result<EditPlan, String> {
    validate_program(program)?;
    edits.sort_by(|a, b| {
        (&a.document, a.span.start, a.span.end).cmp(&(&b.document, b.span.start, b.span.end))
    });
    expected_calls.sort_by_key(|e| e.order);
    let plan = EditPlan {
        format: "hye-source-edit-plan-1".into(),
        schema_sha256: workspace_graph::schema_sha256(),
        compiler: compiler_profile(),
        base_program_sha256: fingerprint(program)?,
        edits,
        expected_calls,
    };
    validate(program, &plan)?;
    Ok(plan)
}
fn document<'a>(program: &'a Program, id: &str) -> Result<&'a Document, String> {
    program
        .documents
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| "Edit references an unknown logical document".into())
}
fn validate(program: &Program, plan: &EditPlan) -> Result<(), String> {
    validate_program(program)?;
    if plan.format != "hye-source-edit-plan-1"
        || plan.schema_sha256 != workspace_graph::schema_sha256()
        || plan.compiler != compiler_profile()
        || plan.base_program_sha256 != fingerprint(program)?
        || plan.edits.is_empty()
        || plan.edits.len() > 512
        || plan.expected_calls.len() > 512
    {
        return Err("Stale, incompatible or oversized source edit plan".into());
    }
    let mut replaced = 0usize;
    for (i, edit) in plan.edits.iter().enumerate() {
        let doc = document(program, &edit.document)?;
        let old = doc
            .source
            .get(edit.span.clone())
            .ok_or("Edit range is invalid or splits a UTF-8 character")?;
        replaced = replaced.saturating_add(edit.replacement.len());
        if edit.base_sha256 != doc.sha256
            || edit.old_bytes_sha256 != source_hash(old)
            || edit.replacement.as_bytes().contains(&0)
            || replaced > MAX_DOCUMENT_BYTES
            || edit.semantic_targets.is_empty()
            || edit.semantic_targets.len() > 16
            || edit
                .semantic_targets
                .iter()
                .any(|s| s.is_empty() || s.len() > 128)
            || old == edit.replacement
        {
            return Err("Edit bytes, targets or revision do not match the captured source".into());
        }
        if i > 0 {
            let previous = &plan.edits[i - 1];
            if (&previous.document, previous.span.start, previous.span.end)
                >= (&edit.document, edit.span.start, edit.span.end)
                || (previous.document == edit.document
                    && (previous.span.end > edit.span.start
                        || previous.span.start == edit.span.start
                        || (previous.span.end == edit.span.start
                            && (previous.span.is_empty() || edit.span.is_empty()))))
            {
                return Err(
                    "Edit ranges must be sorted, non-overlapping and unambiguous at insertions"
                        .into(),
                );
            }
        }
    }
    let mut nodes = 0usize;
    for expected in &plan.expected_calls {
        for value in expected
            .before_arguments
            .iter()
            .chain(&expected.after_arguments)
        {
            validate_value(value, 0, &mut nodes)?;
        }
    }
    if plan
        .expected_calls
        .windows(2)
        .any(|w| w[0].order >= w[1].order)
        || plan
            .expected_calls
            .iter()
            .any(|e| same_arguments(&e.before_arguments, &e.after_arguments))
    {
        return Err("Expected calls are duplicated, unsorted or unchanged".into());
    }
    fingerprint(plan)?;
    Ok(())
}
fn validate_value(value: &Observed, depth: usize, nodes: &mut usize) -> Result<(), String> {
    *nodes = nodes.saturating_add(1);
    if depth > 64 || *nodes > 32768 {
        return Err("Expected argument trees exceed their depth/node budget".into());
    }
    if let Observed::Table(rows) = value {
        for (key, value) in rows {
            validate_value(key, depth + 1, nodes)?;
            validate_value(value, depth + 1, nodes)?;
        }
    }
    Ok(())
}
fn edits_for<'a>(plan: &'a EditPlan, doc: &'a str) -> impl Iterator<Item = &'a SourceEdit> {
    plan.edits.iter().filter(move |e| e.document == doc)
}
fn shift(position: usize, delta: i64) -> Result<usize, String> {
    position
        .checked_add_signed(delta as isize)
        .ok_or_else(|| "Edit offset overflow".into())
}
fn map_span(
    plan: &EditPlan,
    doc: &str,
    span: &Range<usize>,
    immutable: bool,
) -> Result<Range<usize>, String> {
    let mut before = 0i64;
    let mut inside = 0i64;
    for edit in edits_for(plan, doc) {
        let delta = edit.replacement.len() as i64 - edit.span.len() as i64;
        if edit.span.end <= span.start {
            before += delta;
        } else if edit.span.start >= span.end {
            continue;
        } else if !immutable && edit.span.start >= span.start && edit.span.end <= span.end {
            inside += delta;
        } else {
            return Err("A source patch changes a loader request or destroys an observation boundary; recapture/reanalysis is required".into());
        }
    }
    Ok(shift(span.start, before)?..shift(span.end, before + inside)?)
}
fn rebase_request(
    program: &Program,
    plan: &EditPlan,
    request: &Request,
) -> Result<Request, String> {
    let mut next = request.clone();
    next.span = map_span(plan, &request.document, &request.span, true)?;
    next.sha256 = document(program, &request.document)?.sha256.clone();
    Ok(next)
}
fn same_value(a: &Observed, b: &Observed) -> bool {
    match (a, b) {
        (Observed::Float(a), Observed::Float(b)) => a.to_bits() == b.to_bits(),
        (Observed::Table(a), Observed::Table(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ak, av), (bk, bv))| same_value(ak, bk) && same_value(av, bv))
        }
        _ => a == b,
    }
}
fn same_arguments(a: &[Observed], b: &[Observed]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_value(a, b))
}
fn same_load(a: &LoadOutcome, b: &LoadOutcome) -> bool {
    match (a, b) {
        (LoadOutcome::Cached, LoadOutcome::Cached)
        | (LoadOutcome::Preload, LoadOutcome::Preload)
        | (LoadOutcome::SuppliedMissing, LoadOutcome::SuppliedMissing)
        | (LoadOutcome::NeedsResolution, LoadOutcome::NeedsResolution) => true,
        (
            LoadOutcome::SuppliedSnapshot {
                document: a,
                loader_data: ad,
            },
            LoadOutcome::SuppliedSnapshot {
                document: b,
                loader_data: bd,
            },
        ) => a == b && ad == bd,
        _ => false,
    }
}
fn same_site(plan: &EditPlan, before: &Event, after: &Event) -> bool {
    before.document == after.document
        && before.api == after.api
        && map_span(plan, &before.document, &before.span, false).is_ok_and(|s| s == after.span)
}
fn compare(
    plan: &EditPlan,
    proposed: &Program,
    before: &Report,
    after: &Report,
) -> Result<(Vec<Difference>, usize), String> {
    let mut differences = vec![];
    let mut add = |kind, order| {
        differences.push(Difference {
            kind,
            order,
            before_arguments: None,
            after_arguments: None,
        })
    };
    let mut matched = 0;
    for expected in &plan.expected_calls {
        let event = before
            .events
            .iter()
            .find(|e| e.order == expected.order)
            .ok_or("Expected call is absent from the original model")?;
        if !same_arguments(&event.arguments, &expected.before_arguments) {
            return Err("Expected call does not match the original modeled arguments".into());
        }
    }
    for event in &before.events {
        let Some(next) = after.events.iter().find(|e| e.order == event.order) else {
            add(DifferenceKind::MissingCall, Some(event.order));
            continue;
        };
        if !same_site(plan, event, next) {
            add(DifferenceKind::CallSiteOrApi, Some(event.order));
        }
        let expected = plan.expected_calls.iter().find(|e| e.order == event.order);
        if same_arguments(
            &next.arguments,
            expected.map_or(&event.arguments, |e| &e.after_arguments),
        ) {
            matched += usize::from(expected.is_some());
        } else {
            add(DifferenceKind::Arguments, Some(event.order));
        }
    }
    for event in &after.events {
        if !before.events.iter().any(|e| e.order == event.order) {
            add(DifferenceKind::AddedCall, Some(event.order));
        }
    }
    for load in &before.loads {
        let equal = after
            .loads
            .iter()
            .find(|l| l.order == load.order)
            .is_some_and(|next| {
                rebase_request(proposed, plan, &load.request).is_ok_and(|r| r == next.request)
                    && same_load(&load.outcome, &next.outcome)
            });
        if !equal {
            add(DifferenceKind::Loader, Some(load.order));
        }
    }
    for load in &after.loads {
        if !before.loads.iter().any(|l| l.order == load.order) {
            add(DifferenceKind::Loader, Some(load.order));
        }
    }
    let path_equal = before.path_decisions.len() == after.path_decisions.len()
        && before
            .path_decisions
            .iter()
            .zip(&after.path_decisions)
            .all(|(a, b)| {
                a.at_order == b.at_order
                    && a.kind == b.kind
                    && a.outcome == b.outcome
                    && proposed.documents.get(a.site.document).is_some_and(|doc| {
                        a.site.document == b.site.document
                            && map_span(plan, &doc.id, &a.site.span, false)
                                .is_ok_and(|s| s == b.site.span)
                    })
            });
    if !path_equal {
        add(DifferenceKind::PathHistory, None);
    }
    if !same_arguments(&before.returns, &after.returns) {
        add(DifferenceKind::ReturnValues, None);
    }
    if before.logged_module_errors != after.logged_module_errors {
        add(DifferenceKind::LoggedModuleErrors, None);
    }
    if before.halt.is_some() || after.halt.is_some() {
        add(DifferenceKind::IncompleteModel, None);
    }
    for difference in &mut differences {
        if difference.kind == DifferenceKind::ReturnValues {
            difference.before_arguments = Some(before.returns.clone());
            difference.after_arguments = Some(after.returns.clone());
        } else if matches!(
            difference.kind,
            DifferenceKind::Arguments
                | DifferenceKind::MissingCall
                | DifferenceKind::AddedCall
                | DifferenceKind::CallSiteOrApi
        ) {
            difference.before_arguments = before
                .events
                .iter()
                .find(|e| Some(e.order) == difference.order)
                .map(|e| e.arguments.clone());
            difference.after_arguments = after
                .events
                .iter()
                .find(|e| Some(e.order) == difference.order)
                .map(|e| e.arguments.clone());
        }
    }
    Ok((differences, matched))
}
/// Reparse and compare immutable proposed bytes only. This function has no path,
/// filesystem, compositor or write capability, and cannot certify ownership.
pub fn review(program: &Program, plan: &EditPlan) -> Result<EditReview, String> {
    validate(program, plan)?;
    let before = replay(program)?;
    if program
        .bindings
        .iter()
        .any(|b| !before.loads.iter().any(|l| l.request == b.request))
    {
        return Err("An original resolution input was not reached; reanalysis is required".into());
    }
    let mut proposed = program.clone();
    let changed = plan
        .edits
        .iter()
        .map(|e| e.document.as_str())
        .collect::<BTreeSet<_>>();
    for doc in &mut proposed.documents {
        if !changed.contains(doc.id.as_str()) {
            continue;
        }
        let old_len = doc.source.len();
        let growth = edits_for(plan, &doc.id)
            .map(|e| e.replacement.len() as i64 - e.span.len() as i64)
            .sum::<i64>();
        if shift(old_len, growth)? > MAX_DOCUMENT_BYTES {
            return Err("Proposed document exceeds 1 MiB".into());
        }
        for edit in edits_for(plan, &doc.id)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            doc.source
                .replace_range(edit.span.clone(), &edit.replacement);
        }
        doc.sha256 = source_hash(&doc.source);
        if doc.source.as_bytes().contains(&0)
            || super::compile(doc).is_err()
            || syntax_tree(&doc.source)?.root_node().has_error()
        {
            return Err("A changed document does not pass compilation and reparsing".into());
        }
    }
    for index in 0..proposed.bindings.len() {
        proposed.bindings[index].request =
            rebase_request(&proposed, plan, &program.bindings[index].request)?;
    }
    let after = replay(&proposed)?;
    let (differences, matched) = compare(plan, &proposed, &before, &after)?;
    let revisions = proposed
        .documents
        .into_iter()
        .filter(|d| changed.contains(d.id.as_str()))
        .map(|d| {
            let base_sha256 = document(program, &d.id)
                .expect("Validated logical document")
                .sha256
                .clone();
            Revision {
                document: d.id,
                base_sha256,
                proposed_sha256: d.sha256,
                content: d.source,
            }
        })
        .collect();
    let review = EditReview {
        schema_sha256: plan.schema_sha256.clone(),
        compiler: compiler_profile(),
        base_program_sha256: plan.base_program_sha256.clone(),
        plan_sha256: fingerprint(plan)?,
        revisions,
        expected_calls_matched: matched,
        before_model_complete: before.halt.is_none(),
        after_model_complete: after.halt.is_none(),
        modeled_observations_match: differences.is_empty() && matched == plan.expected_calls.len(),
        differences,
        ownership_verified: false,
        runtime_verified: false,
        before_halt: before.halt,
        after_halt: after.halt,
    };
    fingerprint(&review)?;
    Ok(review)
}
