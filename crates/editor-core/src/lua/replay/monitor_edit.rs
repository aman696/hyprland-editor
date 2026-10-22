//! Monitor value comparisons against fresh immutable source replay. Primary
//! parsing and argument routing do not certify helper/application/hardware state.
use super::{edits, replay, scalar_edit, ExpressionKind, Observed, Program};
use crate::{
    lua::{
        monitor_inspection,
        scalar_codec::{self, Outcome},
    },
    monitor, Value,
};
use serde::{Deserialize, Serialize};

pub type Comparison = scalar_edit::Comparison;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Selection {
    SelectedExpression,
    OriginalLiteral,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub event_order: usize,
    pub row: usize,
    pub field: String,
    pub selection: Selection,
    pub value: Value,
}
pub fn compare(program: &Program, change: &Change) -> Result<Comparison, String> {
    if change.field.len() > 64 || change.row >= 4096 {
        return Err("Invalid monitor field coordinates".into());
    }
    let before = replay(program)?;
    let event = before
        .events
        .iter()
        .find(|e| e.order == change.event_order && e.api == "hl.monitor")
        .ok_or("Selected monitor call is unavailable")?;
    let inspection = monitor_inspection::inspect(std::slice::from_ref(event), &program.version)?;
    let batch = inspection
        .batches
        .first()
        .ok_or("Monitor argument inspection is incomplete")?;
    if inspection.stopped.is_some() || !batch.models_submission_and_refresh {
        return Err("Monitor output identity is unresolved".into());
    }
    let Observed::Table(rows) = event.arguments.first().ok_or("Missing monitor table")? else {
        return Err("Missing monitor table".into());
    };
    let (key, _) = rows.get(change.row).ok_or("Missing monitor field row")?;
    let (kind, codec, expression) = if change.field == "output" {
        if !matches!(key, Observed::Bytes(raw) if raw == b"output") {
            return Err("Output row does not match the exact required field".into());
        }
        let Value::String(text) = &change.value else {
            return Err("Output needs a text selector".into());
        };
        let output = if text.is_empty() {
            monitor::Output::Fallback
        } else if let Some(description) = text.strip_prefix("desc:") {
            monitor::Output::Description(description.into())
        } else {
            monitor::Output::Connector(text.clone())
        };
        output.validate()?;
        (
            "String",
            scalar_codec::Codec::String { validator: None },
            event
                .argument_sources
                .first()
                .and_then(|s| s.rows.get(change.row))
                .and_then(|s| s.expression),
        )
    } else {
        let candidate = batch
            .candidates
            .iter()
            .find(|c| c.row == change.row && c.field == change.field)
            .ok_or("Selected monitor field is not recognized")?;
        if candidate.ambiguous {
            return Err(
                "Monitor aliases address the same property; resolve the conflict first".into(),
            );
        }
        let metadata = monitor_inspection::fields(&program.version)?
            .iter()
            .find(|f| f.key == change.field)
            .ok_or("Missing monitor factory metadata")?;
        // Product input constraints are separate from native primitive parsing.
        let mut rule = monitor::Rule::new("comparison");
        rule.fields
            .insert(change.field.clone(), change.value.clone());
        rule.validate(&program.version)?;
        (
            metadata.kind.as_str(),
            monitor_inspection::codec(metadata)?,
            candidate.expression,
        )
    };
    let proposed = scalar_edit::input(&change.value, kind)?;
    let outcome = scalar_codec::decode_batch(&[scalar_codec::Case {
        codec,
        input: proposed.clone(),
    }])?
    .remove(0);
    if !matches!(outcome, Outcome::Accepted { .. }) {
        return Err("The pinned primary parser rejects this value or cannot resolve it".into());
    }
    let mut id = expression.ok_or("No exact expression is recorded for this monitor value")?;
    if matches!(change.selection, Selection::OriginalLiteral) {
        for _ in 0..8192 {
            let node = before
                .expression_sources
                .get(id as usize)
                .ok_or("Missing expression evidence")?;
            if node.kind == ExpressionKind::Literal {
                break;
            }
            id = node.selected.ok_or("This monitor value has no single original literal; compare its selected expression instead")?;
        }
        if before.expression_sources[id as usize].kind != ExpressionKind::Literal {
            return Err("Monitor expression traversal exceeds its limit".into());
        }
    }
    let expression = before
        .expression_sources
        .get(id as usize)
        .ok_or("Missing expression evidence")?;
    let document = program
        .documents
        .get(expression.document)
        .ok_or("Missing monitor source document")?;
    let old = document
        .source
        .get(expression.span.clone())
        .ok_or("Invalid monitor expression span")?;
    let edit = edits::SourceEdit {
        document: document.id.clone(),
        base_sha256: document.sha256.clone(),
        span: expression.span.clone(),
        old_bytes_sha256: crate::lua::source_hash(old),
        replacement: scalar_edit::print(&proposed)?,
        semantic_targets: vec![format!("monitor.{}", change.field)],
    };
    let mut after_arguments = event.arguments.clone();
    let Observed::Table(rows) = &mut after_arguments[0] else {
        unreachable!()
    };
    rows[change.row].1 = proposed;
    let plan = edits::prepare(
        program,
        vec![edit],
        vec![edits::ExpectedCall {
            order: event.order,
            before_arguments: event.arguments.clone(),
            after_arguments,
        }],
    )?;
    let review = edits::review_with_before(program, &plan, before)?;
    let result = Comparison {
        plan,
        review,
        proposed_native_value: outcome,
        table_layout_preserved: false,
    };
    edits::fingerprint(&result)?;
    Ok(result)
}
