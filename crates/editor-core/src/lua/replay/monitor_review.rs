//! Explicit reconciliation of ordered requested monitor declarations. Primary
//! parsing is not effective rule inheritance, native application or hardware.
use super::{edits, monitor_edit, replay, Event, Observed, Program};
use crate::{
    lua::{
        monitor_inspection,
        scalar_codec::{Outcome, Representation},
    },
    monitor::{self, Rule},
    workspace_graph::{self, revisions},
    Value,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub event_order: usize,
    pub document: String,
    pub span: std::ops::Range<usize>,
    pub rule: Option<Rule>,
    pub reason: Option<String>,
    pub repeated_site: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub schema_sha256: String,
    pub program_sha256: String,
    pub draft_sha256: String,
    pub version: String,
    pub compiler: super::CompilerProfile,
    pub model_complete: bool,
    pub sources: Vec<Source>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Choice {
    CopySource {
        event_order: usize,
        replace_id: Option<String>,
    },
    CopyAllSources,
    WriteDraft {
        draft_id: String,
        event_order: usize,
    },
    InsertDraft {
        draft_id: String,
        document: String,
        anchor: monitor_edit::declarations::Anchor,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub document: String,
    pub before: String,
    pub after: String,
    pub truncated: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    pub review_sha256: String,
    pub choice_sha256: String,
    pub program_sha256: String,
    pub monitors: Vec<Rule>,
    pub plans: Vec<edits::EditPlan>,
    pub previews: Vec<Preview>,
    pub model_complete: bool,
    pub ownership_verified: bool,
    pub runtime_verified: bool,
}
fn source_rule(event: &Event, version: &str, id: &str) -> Result<Rule, String> {
    let inspection = monitor_inspection::inspect(std::slice::from_ref(event), version)?;
    let batch = inspection
        .batches
        .first()
        .ok_or("Monitor inspection unavailable")?;
    if inspection.stopped.is_some()
        || !batch.models_submission_and_refresh
        || !batch.findings.is_empty()
    {
        return Err("This declaration has rejected, unknown, aliased or incomplete fields. Review its source values first.".into());
    }
    let output = batch
        .output_bytes
        .as_ref()
        .ok_or("The output selector is unresolved")?;
    let output = std::str::from_utf8(output)
        .map_err(|_| "The output selector cannot be represented as text")?;
    let mut rule = Rule::new(id);
    rule.output = if output.is_empty() {
        monitor::Output::Fallback
    } else if let Some(description) = output.strip_prefix("desc:") {
        monitor::Output::Description(description.into())
    } else {
        monitor::Output::Connector(output.into())
    };
    for candidate in &batch.candidates {
        if candidate.ambiguous || candidate.raw_key != candidate.field.as_bytes() {
            return Err(
                "This declaration needs explicit raw-key or alias cleanup before import".into(),
            );
        }
        let Outcome::Accepted {
            representation: Representation::Value { value },
            ..
        } = &candidate.primary_value
        else {
            return Err("A monitor field has no lossless typed representation".into());
        };
        rule.fields.insert(candidate.field.clone(), value.clone());
    }
    rule.validate(version)?;
    Ok(rule)
}
fn check_draft(program: &Program, monitors: &[Rule]) -> Result<(), String> {
    if program.profile != super::Profile::HyprlandLua54 {
        return Err("Monitor reconciliation requires the pinned Hyprland Lua model".into());
    }
    monitor::validate(monitors, &program.version)?;
    edits::fingerprint(&monitors)?;
    Ok(())
}
pub fn prepare(program: &Program, monitors: &[Rule]) -> Result<Review, String> {
    check_draft(program, monitors)?;
    let report = replay(program)?;
    let events = report
        .events
        .iter()
        .filter(|e| e.api == "hl.monitor")
        .collect::<Vec<_>>();
    if events.len() > 256 {
        return Err("Monitor review exceeds 256 reached declarations".into());
    }
    let mut counts = BTreeMap::new();
    for event in &events {
        *counts
            .entry((&event.document, event.span.start, event.span.end))
            .or_insert(0) += 1;
    }
    let mut sources = Vec::with_capacity(events.len());
    for (i, event) in events.iter().enumerate() {
        let (rule, reason) = match source_rule(
            event,
            &program.version,
            &format!("monitor-source-{}", i + 1),
        ) {
            Ok(rule) => (Some(rule), None),
            Err(reason) => (None, Some(reason)),
        };
        sources.push(Source {
            event_order: event.order,
            document: event.document.clone(),
            span: event.span.clone(),
            rule,
            reason,
            repeated_site: counts[&(&event.document, event.span.start, event.span.end)] > 1,
        });
    }
    let review = Review {
        schema_sha256: workspace_graph::schema_sha256(),
        program_sha256: workspace_graph::program_fingerprint(program)?,
        draft_sha256: edits::fingerprint(&monitors)?,
        version: program.version.clone(),
        compiler: report.compiler_profile,
        model_complete: report.halt.is_none(),
        sources,
    };
    edits::fingerprint(&review)?;
    Ok(review)
}
fn source(review: &Review, order: usize) -> Result<&Source, String> {
    review
        .sources
        .iter()
        .find(|s| s.event_order == order)
        .ok_or_else(|| "Choose a reached monitor declaration".into())
}
fn draft<'a>(monitors: &'a [Rule], id: &str) -> Result<&'a Rule, String> {
    monitors
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| "Choose a current monitor draft rule".into())
}
fn copied(monitors: &[Rule], review: &Review, choice: &Choice) -> Result<Vec<Rule>, String> {
    let mut next = monitors.to_vec();
    match choice {
        Choice::CopySource {
            event_order,
            replace_id,
        } => {
            let mut rule = source(review, *event_order)?
                .rule
                .clone()
                .ok_or("This source declaration cannot be imported without losing information")?;
            if let Some(id) = replace_id {
                let row = next
                    .iter_mut()
                    .find(|r| r.id == *id)
                    .ok_or("The chosen draft rule is missing")?;
                rule.id = row.id.clone();
                *row = rule;
            } else {
                let ids = next.iter().map(|r| r.id.as_str()).collect::<BTreeSet<_>>();
                rule.id = (1..=monitor::MAX_RULES + 1)
                    .map(|n| format!("monitor-source-{n}"))
                    .find(|id| !ids.contains(id.as_str()))
                    .ok_or("No available monitor draft identity")?;
                next.push(rule);
            }
        }
        Choice::CopyAllSources => {
            if !review.model_complete {
                return Err(
                    "Incomplete analysis cannot establish the complete source declaration sequence"
                        .into(),
                );
            }
            next=review.sources.iter().map(|s|s.rule.clone().ok_or_else(||"Every reached declaration must be representable before replacing the whole draft sequence".into())).collect::<Result<Vec<_>,String>>()?;
        }
        Choice::WriteDraft { draft_id, .. } | Choice::InsertDraft { draft_id, .. } => {
            draft(monitors, draft_id)?;
        }
    }
    monitor::validate(&next, &review.version)?;
    Ok(next)
}
fn event(program: &Program, order: usize) -> Result<Event, String> {
    replay(program)?
        .events
        .into_iter()
        .find(|e| e.order == order && e.api == "hl.monitor")
        .ok_or_else(|| "Selected monitor declaration disappeared".into())
}
fn rows(event: &Event) -> Result<&Vec<(Observed, Observed)>, String> {
    match event.arguments.first() {
        Some(Observed::Table(rows)) => Ok(rows),
        _ => Err("Monitor argument is not a modeled table".into()),
    }
}
fn native_equal(
    program: &Program,
    field: &str,
    value: &Observed,
    wanted: &Value,
) -> Result<bool, String> {
    let proposed = monitor_edit::proposed(&program.version, field, wanted)?.1;
    let codec = if field == "output" {
        crate::lua::scalar_codec::Codec::String { validator: None }
    } else {
        monitor_inspection::codec(
            monitor_inspection::fields(&program.version)?
                .iter()
                .find(|m| m.key == field)
                .ok_or("Unknown monitor field")?,
        )?
    };
    let parsed = crate::lua::scalar_codec::decode_batch(&[crate::lua::scalar_codec::Case {
        codec,
        input: value.clone(),
    }])?
    .remove(0);
    Ok(
        matches!((parsed,proposed),(Outcome::Accepted {native:a,..},Outcome::Accepted {native:b,..}) if a==b),
    )
}
fn retain(
    program: &Program,
    pieces: &mut Vec<revisions::Revision>,
    current: &mut Program,
    plans: &mut Vec<edits::EditPlan>,
    previews: &mut Vec<Preview>,
    comparison: monitor_edit::Comparison,
) -> Result<(), String> {
    if !comparison.review.modeled_observations_match {
        return Err("This change affects other modeled calls, loader inputs, paths or results, or crosses incomplete analysis. Review it in the source-change editor first.".into());
    }
    if plans.len() >= 25 {
        return Err("Monitor reconciliation exceeds its source-operation budget".into());
    }
    for edit in &comparison.plan.edits {
        let doc = current
            .documents
            .iter()
            .find(|d| d.id == edit.document)
            .ok_or("Missing changed source")?;
        let before = doc
            .source
            .get(edit.span.clone())
            .ok_or("Invalid source preview range")?;
        previews.push(Preview {
            document: doc.id.clone(),
            before: before.chars().take(1024).collect(),
            after: edit.replacement.chars().take(1024).collect(),
            truncated: before.chars().count() > 1024 || edit.replacement.chars().count() > 1024,
        });
    }
    *pieces = revisions::prepare_pieces(program, pieces, &comparison.plan)?;
    *current = revisions::program(program.clone(), pieces)?;
    plans.push(comparison.plan);
    edits::fingerprint(plans)?;
    Ok(())
}
pub fn resolve(
    program: &Program,
    monitors: &[Rule],
    review: &Review,
    choice: &Choice,
) -> Result<Resolution, String> {
    if edits::fingerprint(&prepare(program, monitors)?)? != edits::fingerprint(review)? {
        return Err("The monitor review changed; rebuild it before choosing".into());
    }
    let next = copied(monitors, review, choice)?;
    let (mut current, mut pieces, mut plans) = (program.clone(), vec![], vec![]);
    let mut previews = vec![];
    match choice {
        Choice::WriteDraft {
            draft_id,
            event_order,
        } => {
            let selected = source(review, *event_order)?;
            if !review.model_complete || selected.repeated_site {
                return Err("Writing one declaration needs complete analysis and a uniquely reached source site".into());
            }
            let rule = draft(monitors, draft_id)?;
            let before = event(&current, *event_order)?;
            let known = monitor_inspection::fields(&program.version)?;
            let mut remove = vec![];
            for (key, _) in rows(&before)? {
                let Observed::Bytes(key) = key else {
                    return Err(
                        "Review ignored non-string monitor fields before reconciling".into(),
                    );
                };
                let field = std::str::from_utf8(key)
                    .map_err(|_| "Review non-text monitor keys before reconciling")?;
                if field != "output" && !known.iter().any(|m| m.key == field) {
                    return Err("Review unknown or NUL monitor keys before reconciling".into());
                }
                if field != "output" && !rule.fields.contains_key(field) {
                    remove.push(key.clone());
                }
            }
            for key in remove {
                let now = event(&current, *event_order)?;
                let row = rows(&now)?
                    .iter()
                    .position(|(k, _)| matches!(k,Observed::Bytes(v) if *v==key))
                    .ok_or("Monitor field disappeared")?;
                let comparison = monitor_edit::compare_field(
                    &current,
                    &monitor_edit::FieldChange::Remove {
                        event_order: *event_order,
                        row,
                        raw_key: key,
                    },
                )?;
                retain(
                    program,
                    &mut pieces,
                    &mut current,
                    &mut plans,
                    &mut previews,
                    comparison,
                )?;
            }
            let desired =
                std::iter::once(("output".to_string(), Value::String(rule.output.text())))
                    .chain(rule.fields.clone());
            for (field, value) in desired {
                let now = event(&current, *event_order)?;
                let row = rows(&now)?
                    .iter()
                    .position(|(k, _)| matches!(k,Observed::Bytes(v) if v==field.as_bytes()));
                let comparison = if let Some(row) = row {
                    if native_equal(&current, &field, &rows(&now)?[row].1, &value)? {
                        continue;
                    }
                    let report = replay(&current)?;
                    let direct_table = now
                        .argument_sources
                        .first()
                        .and_then(|s| s.rows.get(row))
                        .and_then(|s| s.expression)
                        .and_then(|id| report.expression_sources.get(id as usize))
                        .is_some_and(|e| e.kind == super::ExpressionKind::Table);
                    monitor_edit::compare(
                        &current,
                        &monitor_edit::Change {
                            event_order: *event_order,
                            row,
                            field,
                            selection: if direct_table && matches!(value, Value::CssGap(_)) {
                                monitor_edit::Selection::PreserveTableLayout
                            } else {
                                monitor_edit::Selection::SelectedExpression
                            },
                            value,
                        },
                    )?
                } else {
                    // Use recorded deletion writes first; otherwise follow the
                    // original constructor. Each attempt is fresh and private.
                    let mut comparison = None;
                    let mut reason = "No owned insertion site".to_string();
                    for selection in [
                        monitor_edit::TableSelection::LatestWrite,
                        monitor_edit::TableSelection::OriginalConstructor,
                    ] {
                        match monitor_edit::compare_field(&current,&monitor_edit::FieldChange::Add {event_order:*event_order,field:field.clone(),value:value.clone(),selection}) {Ok(c) if c.review.modeled_observations_match=>{comparison=Some(c);break;},Ok(_)=>reason="The insertion changes other modeled effects or is masked by a later write".into(),Err(e)=>reason=e}
                    }
                    comparison.ok_or(reason)?
                };
                retain(
                    program,
                    &mut pieces,
                    &mut current,
                    &mut plans,
                    &mut previews,
                    comparison,
                )?;
            }
            let after = event(&current, *event_order)?;
            let after_rows = rows(&after)?;
            if after_rows.len() != rule.fields.len() + 1 {
                return Err("The proposed declaration has unexpected fields".into());
            }
            for (field, value) in
                std::iter::once(("output".to_string(), Value::String(rule.output.text())))
                    .chain(rule.fields.clone())
            {
                let observed = after_rows
                    .iter()
                    .find_map(|(k, v)| {
                        matches!(k,Observed::Bytes(b) if b==field.as_bytes()).then_some(v)
                    })
                    .ok_or("The proposed declaration is missing a requested field")?;
                if !native_equal(&current, &field, observed, &value)? {
                    return Err(
                        "The proposed declaration does not establish every requested primary value"
                            .into(),
                    );
                }
            }
        }
        Choice::InsertDraft {
            draft_id,
            document,
            anchor,
        } => {
            let rule = draft(monitors, draft_id)?.clone();
            let comparison = monitor_edit::declarations::compare(
                &current,
                &monitor_edit::declarations::Change::Insert {
                    document: document.clone(),
                    anchor: anchor.clone(),
                    rule,
                },
            )?;
            retain(
                program,
                &mut pieces,
                &mut current,
                &mut plans,
                &mut previews,
                comparison,
            )?;
        }
        _ => {}
    }
    let result = Resolution {
        review_sha256: edits::fingerprint(review)?,
        choice_sha256: edits::fingerprint(choice)?,
        program_sha256: workspace_graph::program_fingerprint(&current)?,
        monitors: next,
        plans,
        previews,
        model_complete: replay(&current)?.halt.is_none(),
        ownership_verified: false,
        runtime_verified: false,
    };
    validate_resolution(monitors, review, choice, &result)?;
    edits::fingerprint(&result)?;
    Ok(result)
}
/// Correlate the explicit layer choice before independent private-piece rebuild.
pub fn validate_resolution(
    monitors: &[Rule],
    review: &Review,
    choice: &Choice,
    result: &Resolution,
) -> Result<(), String> {
    if result.ownership_verified
        || result.runtime_verified
        || result.review_sha256 != edits::fingerprint(review)?
        || result.choice_sha256 != edits::fingerprint(choice)?
        || review.draft_sha256 != edits::fingerprint(&monitors)?
        || edits::fingerprint(&result.monitors)?
            != edits::fingerprint(&copied(monitors, review, choice)?)?
    {
        return Err("The monitor result changes layers beyond the reviewed choice".into());
    }
    if result.previews.len() != result.plans.iter().map(|p| p.edits.len()).sum::<usize>() {
        return Err("Monitor preview count differs from the planned edits".into());
    }
    for (preview, edit) in result
        .previews
        .iter()
        .zip(result.plans.iter().flat_map(|p| &p.edits))
    {
        if preview.document != edit.document
            || preview.before.len() > edit.span.len()
            || (preview.before.len() < edit.span.len() && preview.before.chars().count() != 1024)
            || preview.truncated
                != (preview.before.len() < edit.span.len()
                    || edit.replacement.chars().count() > 1024)
            || preview.after != edit.replacement.chars().take(1024).collect::<String>()
            || preview.before.chars().count() > 1024
            || (preview.before.len() == edit.span.len()
                && crate::lua::source_hash(&preview.before) != edit.old_bytes_sha256)
        {
            return Err("Monitor source excerpts differ from the planned edits".into());
        }
    }
    match choice {
        Choice::CopySource { .. } | Choice::CopyAllSources => {
            if !result.plans.is_empty()
                || result.program_sha256 != review.program_sha256
                || result.model_complete != review.model_complete
            {
                return Err("Copying source into monitor drafts cannot change source".into());
            }
        }
        Choice::WriteDraft { event_order, .. } => {
            if result.plans.len() > 25 || !result.model_complete {
                return Err("Invalid monitor write result".into());
            }
            for plan in &result.plans {
                if plan.expected_calls.len() != 1
                    || plan.expected_calls[0].order != *event_order
                    || !plan.structural_calls.is_empty()
                    || plan.edits.is_empty()
                    || plan.edits.iter().any(|e| {
                        e.semantic_targets.len() != 1
                            || !e.semantic_targets[0].starts_with("monitor.")
                    })
                {
                    return Err("The monitor write contains an unselected source operation".into());
                }
            }
        }
        Choice::InsertDraft {
            draft_id,
            document,
            anchor,
        } => {
            if result.plans.len() != 1 || !result.model_complete {
                return Err("Invalid monitor insertion result".into());
            }
            let plan = &result.plans[0];
            let rule = draft(monitors, draft_id)?;
            let mut values = BTreeMap::from([(
                "output".to_string(),
                Observed::Bytes(rule.output.text().into_bytes()),
            )]);
            for (field, value) in &rule.fields {
                let metadata = monitor_inspection::fields(&review.version)?
                    .iter()
                    .find(|m| m.key == *field)
                    .ok_or("Unknown monitor field")?;
                values.insert(
                    field.clone(),
                    super::scalar_edit::input(value, &metadata.kind)?,
                );
            }
            let argument = Observed::Table(
                values
                    .into_iter()
                    .map(|(key, value)| (Observed::Bytes(key.into_bytes()), value))
                    .collect(),
            );
            let call = format!("hl.monitor({})", super::scalar_edit::print(&argument)?);
            if !plan.expected_calls.is_empty()
                || plan.edits.len() != 1
                || plan.edits[0].document != *document
                || plan.structural_calls.len() != 1
                || !matches!(&plan.structural_calls[0],edits::StructuralCall::Insert {document:d,api,..} if d==document&&api=="hl.monitor")
            {
                return Err("The monitor insertion differs from the selected document/call".into());
            }
            let edit = &plan.edits[0];
            let newline = if edit.replacement.starts_with("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            if !edit.span.is_empty()
                || edit.old_bytes_sha256 != crate::lua::source_hash("")
                || edit.semantic_targets != ["monitor.declaration"]
                || edit.replacement != format!("{newline};{call};{newline}")
            {
                return Err(
                    "The monitor insertion is not the exact selected typed declaration".into(),
                );
            }
            if let edits::StructuralCall::Insert {
                edit_span,
                call_span,
                arguments,
                ..
            } = &plan.structural_calls[0]
            {
                if *edit_span != edit.span
                    || *call_span != (newline.len() + 1..newline.len() + 1 + call.len())
                    || arguments.len() != 1
                    || !edits::same_value(&arguments[0], &argument)
                {
                    return Err(
                        "The inserted observation differs from the requested monitor draft".into(),
                    );
                }
            }
            if match anchor {
                monitor_edit::declarations::Anchor::Before { span } => {
                    edit.span.start != span.start
                }
                monitor_edit::declarations::Anchor::After { span } => edit.span.start != span.end,
                _ => false,
            } {
                return Err(
                    "The monitor insertion moved from the selected statement boundary".into(),
                );
            }
        }
    }
    Ok(())
}
