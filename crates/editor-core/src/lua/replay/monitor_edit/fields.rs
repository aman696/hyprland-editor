//! Field presence edits from fresh expression/storage evidence. Never effective
//! defaults, exclusive ownership, permission to reopen a path, or live effects.
use super::super::{Event, Report, StorageKey, StoragePlace, WriteKind};
use super::*;
use crate::lua::{named, syntax_tree};
use std::ops::Range;
use tree_sitter::Node;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableSelection {
    SelectedConstructor,
    OriginalConstructor,
    LatestWrite,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldChange {
    Add {
        event_order: usize,
        field: String,
        value: Value,
        selection: TableSelection,
    },
    Remove {
        event_order: usize,
        row: usize,
        raw_key: Vec<u8>,
    },
}
fn node_for<'a>(tree: &'a tree_sitter::Tree, span: Range<usize>) -> Result<Node<'a>, String> {
    tree.root_node()
        .descendant_for_byte_range(span.start, span.end)
        .filter(|n| n.byte_range() == span)
        .ok_or_else(|| "Recorded monitor source span is not an exact syntax node".into())
}
fn edit(
    program: &Program,
    document: usize,
    span: Range<usize>,
    replacement: String,
    target: &str,
) -> Result<edits::SourceEdit, String> {
    let doc = program
        .documents
        .get(document)
        .ok_or("Recorded monitor source is unavailable")?;
    let old = doc
        .source
        .get(span.clone())
        .ok_or("Invalid recorded monitor source span")?;
    Ok(edits::SourceEdit {
        document: doc.id.clone(),
        base_sha256: doc.sha256.clone(),
        span,
        old_bytes_sha256: crate::lua::source_hash(old),
        replacement,
        semantic_targets: vec![target.into()],
    })
}
fn table_id(report: &Report, event: &Event) -> Result<usize, String> {
    event
        .argument_sources
        .first()
        .and_then(|s| s.expression)
        .and_then(|id| report.expression_sources.get(id as usize))
        .and_then(|s| s.table_identity)
        .ok_or_else(|| "The monitor argument has no recorded table identity".into())
}
fn replace_value(
    program: &Program,
    report: &Report,
    id: Option<u32>,
    replacement: &str,
    target: &str,
) -> Result<edits::SourceEdit, String> {
    let expression = id
        .and_then(|id| report.expression_sources.get(id as usize))
        .ok_or("The field write has no exact value expression")?;
    let doc = program
        .documents
        .get(expression.document)
        .ok_or("Missing field source")?;
    let tree = syntax_tree(&doc.source)?;
    let node = node_for(&tree, expression.span.clone())?;
    let trivia = scalar_edit::layout::erase_tokens(node, &doc.source)?;
    edit(
        program,
        expression.document,
        expression.span.clone(),
        format!("{replacement}{trivia}"),
        target,
    )
}
fn remove(
    program: &Program,
    before: &Report,
    event: &Event,
    row: usize,
    raw_key: &[u8],
) -> Result<Vec<edits::SourceEdit>, String> {
    if row >= 4096 || raw_key.len() > 65536 || raw_key == b"output" {
        return Err(
            "Cannot remove the required output selector or an invalid field coordinate".into(),
        );
    }
    let Observed::Table(rows) = &event.arguments[0] else {
        return Err("Monitor argument must be a table".into());
    };
    if !matches!(rows.get(row), Some((Observed::Bytes(key),_)) if key == raw_key) {
        return Err("The field row does not match its exact recorded key".into());
    }
    let id = table_id(before, event)?;
    let write = event
        .argument_sources
        .first()
        .and_then(|s| s.keys.get(row))
        .and_then(|s| s.latest_write)
        .and_then(|i| before.storage_writes.get(i as usize))
        .ok_or("The selected field has no recorded write")?;
    if !matches!(&write.place, StoragePlace::Table {identity, key: StorageKey::Bytes(key)} if *identity == id && key == raw_key)
    {
        return Err("Recorded field write does not address this monitor table".into());
    }
    if write.kind != WriteKind::TableConstructor {
        // Keep the assignment/rawset and its other targets. Explicit nil deletes
        // this slot. A shared/multiple-result expression may affect other values;
        // the independent before/after comparison reports those effects.
        return Ok(vec![replace_value(
            program,
            before,
            write.value_expression,
            "nil",
            "monitor.field.remove",
        )?]);
    }
    let site = write
        .site
        .as_ref()
        .ok_or("Missing constructor field site")?;
    let doc = program
        .documents
        .get(site.document)
        .ok_or("Missing constructor source")?;
    let tree = syntax_tree(&doc.source)?;
    let node = node_for(&tree, site.span.clone())?;
    if node.kind() != "field"
        || node
            .parent()
            .is_none_or(|p| p.kind() != "table_constructor")
    {
        return Err("Recorded monitor write is not a constructor field".into());
    }
    let mut result = vec![edit(
        program,
        site.document,
        site.span.clone(),
        scalar_edit::layout::erase_tokens(node, &doc.source)?,
        "monitor.field.remove",
    )?];
    let mut following = node.next_sibling();
    while following.is_some_and(|n| n.kind() == "comment") {
        following = following.and_then(|n| n.next_sibling());
    }
    let separator = following
        .filter(|n| matches!(n.kind(), "," | ";"))
        .or_else(|| {
            let mut previous = node.prev_sibling();
            while previous.is_some_and(|n| n.kind() == "comment") {
                previous = previous.and_then(|n| n.prev_sibling());
            }
            previous.filter(|n| matches!(n.kind(), "," | ";"))
        });
    if let Some(separator) = separator {
        result.push(edit(
            program,
            site.document,
            separator.byte_range(),
            String::new(),
            "monitor.field.remove",
        )?);
    }
    Ok(result)
}
fn add(
    program: &Program,
    before: &Report,
    event: &Event,
    field: &str,
    value: &Observed,
    selection: &TableSelection,
) -> Result<Vec<edits::SourceEdit>, String> {
    let source = event
        .argument_sources
        .first()
        .ok_or("Missing monitor argument source")?;
    let identity = table_id(before, event)?;
    if matches!(selection, TableSelection::LatestWrite) {
        let write=before.storage_writes.iter().rev().find(|w| w.at_order<=event.order && matches!(&w.place,StoragePlace::Table {identity:table,key:StorageKey::Bytes(key)} if *table==identity && key==field.as_bytes())).ok_or("This field has no recorded write; choose its constructor instead")?;
        if !write.deletes_slot {
            return Err("The recorded field write does not establish an absent slot".into());
        }
        return Ok(vec![replace_value(
            program,
            before,
            write.value_expression,
            &scalar_edit::print(value)?,
            &format!("monitor.{field}"),
        )?]);
    }
    let mut id = source
        .expression
        .ok_or("Missing monitor argument expression")?;
    if matches!(selection, TableSelection::OriginalConstructor) {
        for _ in 0..8192 {
            let expression = before
                .expression_sources
                .get(id as usize)
                .ok_or("Missing table expression")?;
            if expression.kind == ExpressionKind::Table {
                break;
            }
            id = expression
                .selected
                .ok_or("This monitor argument has no single original constructor")?;
        }
    }
    let expression = before
        .expression_sources
        .get(id as usize)
        .ok_or("Missing constructor expression")?;
    if expression.kind != ExpressionKind::Table || expression.table_identity != Some(identity) {
        return Err("Select an exact table constructor or trace its original constructor".into());
    }
    let doc = program
        .documents
        .get(expression.document)
        .ok_or("Missing monitor constructor source")?;
    let tree = syntax_tree(&doc.source)?;
    let node = node_for(&tree, expression.span.clone())?;
    if node.kind() != "table_constructor" || named(node).len() > 4096 {
        return Err("Invalid or oversized monitor constructor".into());
    }
    let target = format!("monitor.{field}");
    // Includes nil writes, which are absent from the observed table. Reuse a
    // recorded placeholder instead of introducing duplicate constructor keys.
    let writes = before.storage_writes.iter().filter(|w| w.kind == WriteKind::TableConstructor && matches!(&w.place, StoragePlace::Table {identity: table, key: StorageKey::Bytes(key)} if *table == identity && key == field.as_bytes()) && w.site.as_ref().is_some_and(|s| s.document == expression.document && s.span.start > expression.span.start && s.span.end < expression.span.end)).collect::<Vec<_>>();
    if writes.len() > 1 {
        return Err("Several constructor writes address this field".into());
    }
    if let Some(write) = writes.first() {
        return Ok(vec![replace_value(
            program,
            before,
            write.value_expression,
            &scalar_edit::print(value)?,
            &target,
        )?]);
    }
    // An insertion directly after '{' cannot be swallowed by an existing line
    // comment. Explicit string keys preserve all implicit array indices.
    let newline = if doc.source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let inserted = format!(
        "{newline}[{}] = {},{newline}",
        scalar_edit::print(&Observed::Bytes(field.as_bytes().to_vec()))?,
        scalar_edit::print(value)?
    );
    let at = expression.span.start + 1;
    Ok(vec![edit(
        program,
        expression.document,
        at..at,
        inserted,
        &target,
    )?])
}
pub fn compare_field(program: &Program, change: &FieldChange) -> Result<Comparison, String> {
    let order = match change {
        FieldChange::Add { event_order, .. } | FieldChange::Remove { event_order, .. } => {
            *event_order
        }
    };
    let before = replay(program)?;
    let event = before
        .events
        .iter()
        .find(|e| e.order == order && e.api == "hl.monitor")
        .ok_or("The selected monitor call is unavailable")?;
    let inspection = monitor_inspection::inspect(std::slice::from_ref(event), &program.version)?;
    if inspection.stopped.is_some() {
        return Err("Monitor field inspection is incomplete".into());
    }
    let batch = inspection
        .batches
        .first()
        .ok_or("Monitor field inspection is unavailable")?;
    let mut arguments = event.arguments.clone();
    let Some(Observed::Table(rows)) = arguments.first_mut() else {
        return Err("Monitor argument must be a table".into());
    };
    let (edits, proposed_native_value) = match change {
        FieldChange::Add {
            field,
            value,
            selection,
            ..
        } => {
            if field.is_empty() || field.len() > 64 {
                return Err("Invalid monitor field name".into());
            }
            let canonical = if field == "reserved_area" {
                "reserved"
            } else {
                field.as_str()
            };
            if rows
                .iter()
                .any(|(key, _)| matches!(key, Observed::Bytes(key) if key == field.as_bytes()))
                || batch.candidates.iter().any(|c| c.target == canonical)
            {
                return Err("This monitor field or its alias is already present".into());
            }
            let (value, outcome) = proposed(&program.version, field, value)?;
            let edits = add(program, &before, event, field, &value, selection)?;
            let key = field.as_bytes();
            let at = rows
                .iter()
                .position(|(k, _)| match k {
                    Observed::Bytes(v) => v.as_slice() > key,
                    Observed::Table(_) | Observed::Function(_) | Observed::Cycle(_) => true,
                    _ => false,
                })
                .unwrap_or(rows.len());
            rows.insert(at, (Observed::Bytes(key.to_vec()), value));
            (edits, outcome)
        }
        FieldChange::Remove { row, raw_key, .. } => {
            let edits = remove(program, &before, event, *row, raw_key)?;
            rows.remove(*row);
            (edits, Outcome::Unresolved {reason: "The explicit field is removed. Inherited and effective values remain unresolved.".into()})
        }
    };
    let plan = edits::prepare(
        program,
        edits,
        vec![edits::ExpectedCall {
            order,
            before_arguments: event.arguments.clone(),
            after_arguments: arguments,
        }],
    )?;
    let review = edits::review_with_before(program, &plan, before)?;
    let result = Comparison {
        plan,
        review,
        proposed_native_value,
        table_layout_preserved: true,
    };
    edits::fingerprint(&result)?;
    Ok(result)
}
