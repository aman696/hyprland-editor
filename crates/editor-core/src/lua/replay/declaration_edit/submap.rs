//! Submap header edits retain the complete callback expression and its source.
use super::super::{Binding, Observed, Report};
use super::*;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub name: String,
    pub reset: Option<String>,
    pub occurrences: usize,
    pub callback_span: Range<usize>,
}
fn strings(event: &Event) -> Result<(String, Option<String>), String> {
    fn string(value: &Observed) -> Result<String, String> {
        match value {
            Observed::Bytes(bytes) => String::from_utf8(bytes.clone())
                .map_err(|_| "The submap text cannot be represented in this graphical form".into()),
            _ => Err("The submap name or reset target is not known text".into()),
        }
    }
    match event.arguments.as_slice() {
        [name, Observed::Function(_)] => Ok((string(name)?, None)),
        [name, reset, Observed::Function(_)] => Ok((string(name)?, Some(string(reset)?))),
        _ => {
            Err("This submap needs a known name, optional reset target and modeled callback".into())
        }
    }
}
fn validate(version: &str, name: &str, reset: &Option<String>) -> Result<(), String> {
    crate::binding::Draft {
        binds: vec![],
        submaps: vec![crate::binding::Submap {
            id: "header".into(),
            name: name.into(),
            parent: None,
            reset: reset.clone(),
        }],
        order: vec!["header".into()],
    }
    .validate(version)
}
fn selected<'a>(
    program: &Program,
    report: &'a Report,
    order: usize,
) -> Result<(&'a Event, Header, Range<usize>), String> {
    let event = report
        .events
        .iter()
        .find(|e| e.order == order && e.api == "hl.define_submap")
        .ok_or("Select a reached submap declaration")?;
    let (name, reset) = strings(event)?;
    if name.len() > 4096
        || name.chars().any(char::is_control)
        || reset
            .as_ref()
            .is_some_and(|text| text.len() > 4096 || text.chars().any(char::is_control))
    {
        return Err("The captured submap text exceeds the graphical form's limits".into());
    }
    let document = program
        .documents
        .iter()
        .find(|d| d.id == event.document)
        .ok_or("The submap source is unavailable")?;
    let tree = syntax_tree(&document.source)?;
    let call = tree
        .root_node()
        .descendant_for_byte_range(event.span.start, event.span.end)
        .filter(|n| n.byte_range() == event.span && n.kind() == "function_call")
        .ok_or("The submap no longer has an exact source call")?;
    let arguments = call
        .child_by_field_name("arguments")
        .ok_or("Submap argument source is unavailable")?;
    let values = named(arguments);
    if values.len() != event.arguments.len()
        || !matches!(values.len(), 2 | 3)
        || !text(arguments, &document.source).starts_with('(')
    {
        return Err("Submap argument expansion needs a different source editor".into());
    }
    let callback = values.last().unwrap().byte_range();
    let occurrences = report
        .events
        .iter()
        .filter(|e| e.document == event.document && e.span == event.span && e.api == event.api)
        .count();
    Ok((
        event,
        Header {
            name,
            reset,
            occurrences,
            callback_span: callback.clone(),
        },
        arguments.start_byte()..callback.start,
    ))
}
pub fn inspect(program: &Program, order: usize) -> Result<Header, String> {
    let report = replay(program)?;
    Ok(selected(program, &report, order)?.1)
}
pub fn compare(
    program: &Program,
    order: usize,
    name: String,
    reset: Option<String>,
) -> Result<scalar_edit::Comparison, String> {
    validate(&program.version, &name, &reset)?;
    let before = replay(program)?;
    let (original, header, prefix) = selected(program, &before, order)?;
    if name == header.name && reset == header.reset {
        return Err("Change the submap name or reset target before comparing".into());
    }
    let document = program
        .documents
        .iter()
        .find(|d| d.id == original.document)
        .unwrap();
    let old = document
        .source
        .get(prefix.clone())
        .ok_or("The submap header range is invalid")?;
    // Parse a complete synthetic call to recover only header trivia. The actual
    // callback is never copied into this template or reconstructed from a report.
    let template = format!("hl.define_submap{old}function()end)");
    let tree = syntax_tree(&template)?;
    let call = named(tree.root_node())
        .into_iter()
        .find(|n| n.kind() == "function_call")
        .ok_or("The submap header could not be parsed independently")?;
    let args = call
        .child_by_field_name("arguments")
        .ok_or("The submap header has no argument list")?;
    let trivia = scalar_edit::layout::erase_tokens(args, &template)?;
    let newline = if document.source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let reset_source = reset
        .as_ref()
        .map(|value| format!("{},", crate::quote_lua(value)))
        .unwrap_or_default();
    let replacement = format!(
        "({},{}{newline}{trivia}{newline}",
        crate::quote_lua(&name),
        reset_source
    );
    let edit = edits::SourceEdit {
        document: document.id.clone(),
        base_sha256: document.sha256.clone(),
        span: prefix,
        old_bytes_sha256: lua::source_hash(old),
        replacement,
        semantic_targets: vec!["binding.submap.header".into()],
    };
    let provisional = edits::prepare(program, vec![edit.clone()], vec![])?;
    let mut proposed = program.clone();
    let target = proposed
        .documents
        .iter_mut()
        .find(|d| d.id == document.id)
        .unwrap();
    target
        .source
        .replace_range(edit.span.clone(), &edit.replacement);
    target.sha256 = lua::source_hash(&target.source);
    proposed.bindings = proposed
        .bindings
        .iter()
        .map(|binding| {
            Ok(Binding {
                request: edits::rebase_request(&proposed, &provisional, &binding.request)?,
                resolution: binding.resolution.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let after = replay(&proposed)?;
    let mut expected = vec![];
    for event in before.events.iter().filter(|e| {
        e.document == original.document && e.span == original.span && e.api == original.api
    }) {
        let next = after
            .events
            .iter()
            .find(|e| {
                e.order == event.order
                    && e.document == event.document
                    && e.api == event.api
                    && e.span.start == event.span.start
            })
            .ok_or("The edited submap was not reached at its original position")?;
        let observed = strings(next)?;
        if observed != (name.clone(), reset.clone()) {
            return Err(
                "The edited submap does not reproduce the requested name and reset target".into(),
            );
        }
        if event.arguments.len() != next.arguments.len()
            || event
                .arguments
                .iter()
                .zip(&next.arguments)
                .any(|(a, b)| !edits::same_value(a, b))
        {
            expected.push(edits::ExpectedCall {
                order: event.order,
                before_arguments: event.arguments.clone(),
                after_arguments: next.arguments.clone(),
            });
        }
    }
    let plan = edits::prepare(program, vec![edit], expected)?;
    let review = edits::review_with_before(program, &plan, before)?;
    Ok(scalar_edit::Comparison{plan,review,table_layout_preserved:false,proposed_native_value:lua::scalar_codec::Outcome::Unresolved{reason:"Only the name/reset prefix is rebuilt. The callback expression and remaining source are retained verbatim. Enclosed registrations change submap context; references elsewhere are preserved and may need separate editing. Runtime behavior remains unverified.".into()}})
}
