//! Graphical whole-argument edits for one reached structured declaration.
//! Original bytes and other graph observations remain independently compared.
mod structure;
pub mod submap;
use super::{edits, replay, scalar_edit, Event, Program};
use crate::{
    lua::{self, named, syntax_tree, text, LuaReport},
    workspace::Workspace,
    Setting,
};
pub use structure::{insert, Anchor};

pub fn supports(api: &str) -> bool {
    matches!(
        api,
        "hl.device"
            | "hl.bind"
            | "hl.define_submap"
            | "hl.workspace_rule"
            | "hl.window_rule"
            | "hl.layer_rule"
            | "hl.curve"
            | "hl.animation"
            | "hl.gesture"
    )
}
fn canonical(workspace: &Workspace, api: &str) -> Result<String, String> {
    let version = &workspace.draft.version;
    match api {
        "hl.device" => crate::device::export_lua(&workspace.devices, version),
        "hl.bind" | "hl.define_submap" => workspace.bindings.export_lua(version),
        "hl.workspace_rule" => {
            crate::workspace_rule::export_lua(&workspace.workspace_rules, version)
        }
        "hl.window_rule" | "hl.layer_rule" => crate::rule::export_lua(&workspace.rules, version),
        "hl.curve" | "hl.animation" => workspace.animations.export_lua(version),
        "hl.gesture" => workspace.gestures.export_lua(version),
        _ => Err("This declaration uses another source editor".into()),
    }
}
fn same_family(api: &str, other: &str) -> bool {
    match api {
        "hl.bind" | "hl.define_submap" => matches!(other, "hl.bind" | "hl.define_submap"),
        "hl.window_rule" | "hl.layer_rule" => matches!(other, "hl.window_rule" | "hl.layer_rule"),
        "hl.curve" | "hl.animation" => matches!(other, "hl.curve" | "hl.animation"),
        _ => api == other,
    }
}
fn only_family(target: &Workspace, api: &str) -> Result<(), String> {
    let others = [
        (!target.devices.is_empty(), "hl.device"),
        (!target.bindings.is_empty(), "hl.bind"),
        (!target.workspace_rules.is_empty(), "hl.workspace_rule"),
        (!target.rules.is_empty(), "hl.window_rule"),
        (!target.animations.declarations.is_empty(), "hl.animation"),
        (!target.gestures.is_empty(), "hl.gesture"),
    ];
    if !target.draft.values.is_empty()
        || !target.monitors.is_empty()
        || others
            .iter()
            .any(|(present, other)| *present && !same_family(api, other))
    {
        return Err("This editor can change one feature family at a time. Other declarations must remain in their own source editor.".into());
    }
    Ok(())
}
fn validate_target(
    program: &Program,
    target: &Workspace,
    catalogue: &[Setting],
) -> Result<(), String> {
    target.validate(catalogue)?;
    if target.draft.version != program.version
        || target.source_graph.is_some()
        || !target.sources.is_empty()
    {
        return Err("Declaration edits need an isolated draft for this source version".into());
    }
    Ok(())
}
fn validate_rendered(
    rendered: &str,
    target: &Workspace,
    api: &str,
    catalogue: &[Setting],
) -> Result<(), String> {
    only_family(target, api)?;
    let tree = syntax_tree(rendered)?;
    if named(tree.root_node()).iter().any(|call| {
        call.kind() != "function_call"
            || call
                .child_by_field_name("name")
                .is_none_or(|name| !same_family(api, text(name, rendered)))
    }) {
        return Err("The serializer produced another declaration family".into());
    }
    let checked = lua::parse(rendered, &target.draft.version, catalogue)?;
    if checked.opaque_effects
        || !checked.diagnostics.is_empty()
        || canonical(&workspace(checked), api)? != rendered
    {
        return Err(
            "The graphical declarations do not round-trip through their source codec".into(),
        );
    }
    Ok(())
}
fn workspace(report: LuaReport) -> Workspace {
    Workspace::new(report.draft)
        .with_monitors(report.monitors)
        .with_devices(report.devices)
        .with_bindings(report.bindings)
        .with_workspace_rules(report.workspace_rules)
        .with_rules(report.rules)
        .with_animations(report.animations)
        .with_gestures(report.gestures)
}
fn event(report: &super::Report, order: usize) -> Result<&Event, String> {
    let event = report
        .events
        .iter()
        .find(|e| e.order == order && supports(&e.api))
        .ok_or("Select a supported structured declaration")?;
    if report
        .events
        .iter()
        .filter(|e| e.document == event.document && e.span == event.span)
        .count()
        != 1
    {
        return Err(
            "This source site runs repeatedly; edit its shared source explicitly first".into(),
        );
    }
    Ok(event)
}
fn literal(value: &super::Observed) -> Result<String, String> {
    use super::Observed;
    let text = match value {
        Observed::Dispatcher {
            constructor,
            arguments,
            ..
        } => format!(
            "hl.dsp.{constructor}({})",
            arguments
                .iter()
                .map(literal)
                .collect::<Result<Vec<_>, _>>()?
                .join(",")
        ),
        Observed::Table(rows) => format!(
            "{{{}}}",
            rows.iter()
                .map(|(key, value)| Ok(format!("[{}]={}", literal(key)?, literal(value)?)))
                .collect::<Result<Vec<_>, String>>()?
                .join(",")
        ),
        _ => scalar_edit::print(value)?,
    };
    if text.len() > crate::MAX_DOCUMENT_BYTES {
        return Err("Captured declaration arguments exceed the graphical form limit".into());
    }
    Ok(text)
}
fn selected(program: &Program, event: &Event, catalogue: &[Setting]) -> Result<LuaReport, String> {
    let doc = program
        .documents
        .iter()
        .find(|d| d.id == event.document)
        .ok_or("Declaration document missing")?;
    let tree = syntax_tree(&doc.source)?;
    let node = tree
        .root_node()
        .descendant_for_byte_range(event.span.start, event.span.end)
        .filter(|n| n.byte_range() == event.span && n.kind() == "function_call")
        .ok_or("Declaration is not an exact call node")?;
    if !node
        .parent()
        .is_some_and(|p| matches!(p.kind(), "chunk" | "block"))
    {
        return Err("This declaration's returned object is used by surrounding source".into());
    }
    let mut report = lua::parse(text(node, &doc.source), &program.version, catalogue)?;
    if report.opaque_effects || !report.diagnostics.is_empty() {
        // Known values from captured modules/expressions can populate a form.
        // Function bodies and cycles cannot be invented from an observation.
        let arguments = event
            .arguments
            .iter()
            .map(literal)
            .collect::<Result<Vec<_>, _>>()?;
        let source = format!("{}({})", event.api, arguments.join(","));
        report = lua::parse(&source, &program.version, catalogue)?;
    }
    if report.opaque_effects || !report.diagnostics.is_empty() || !report.draft.values.is_empty() {
        return Err("This declaration needs surrounding values or unsupported callback syntax. Its source stays retained; no partial graphical form was opened.".into());
    }
    let rendered = canonical(&workspace(report.clone()), &event.api)?;
    let tree = syntax_tree(&rendered)?;
    let calls = named(tree.root_node());
    if calls.is_empty()
        || calls.iter().any(|call| {
            call.child_by_field_name("name")
                .is_none_or(|name| !same_family(&event.api, text(name, &rendered)))
        })
    {
        return Err(
            "This source declaration cannot be represented by the graphical family editor".into(),
        );
    }
    let snapshot = workspace(report.clone());
    only_family(&snapshot, &event.api)?;
    Ok(report)
}
pub fn inspect(
    program: &Program,
    order: usize,
    catalogue: &[Setting],
) -> Result<LuaReport, String> {
    let report = replay(program)?;
    selected(program, event(&report, order)?, catalogue)
}
pub fn compare(
    program: &Program,
    order: usize,
    target: &Workspace,
    catalogue: &[Setting],
) -> Result<scalar_edit::Comparison, String> {
    validate_target(program, target, catalogue)?;
    let before = replay(program)?;
    let original = event(&before, order)?;
    let old = selected(program, original, catalogue)?;
    let rendered = canonical(target, &original.api)?;
    if rendered == canonical(&workspace(old), &original.api)? {
        return Err("Change a field in the graphical editor before comparing".into());
    }
    validate_rendered(&rendered, target, &original.api, catalogue)?;
    let rendered_tree = syntax_tree(&rendered)?;
    let calls = named(rendered_tree.root_node());
    if calls.len() != 1
        || calls[0]
            .child_by_field_name("name")
            .is_none_or(|name| text(name, &rendered) != original.api)
        || original.api == "hl.define_submap"
    {
        let original = original.clone();
        return structure::replace(program, before, &original, &rendered);
    }
    let doc = program
        .documents
        .iter()
        .find(|d| d.id == original.document)
        .ok_or("Declaration document missing")?;
    let tree = syntax_tree(&doc.source)?;
    let node = tree
        .root_node()
        .descendant_for_byte_range(original.span.start, original.span.end)
        .ok_or("Declaration node missing")?;
    let args = node
        .child_by_field_name("arguments")
        .ok_or("Declaration arguments missing")?;
    let new_args = calls[0]
        .child_by_field_name("arguments")
        .ok_or("Serialized declaration arguments missing")?;
    let mut replacement = text(new_args, &rendered).trim().to_owned();
    if !replacement.ends_with(')') {
        return Err("Declaration serializer must produce parenthesized arguments".into());
    }
    replacement.pop();
    replacement.push('\n');
    replacement.push_str(&scalar_edit::layout::erase_tokens(args, &doc.source)?);
    replacement.push_str("\n)");
    let edit = edits::SourceEdit {
        document: doc.id.clone(),
        base_sha256: doc.sha256.clone(),
        span: args.byte_range(),
        old_bytes_sha256: lua::source_hash(text(args, &doc.source)),
        replacement,
        semantic_targets: vec![format!("{}.declaration", original.api)],
    };
    let provisional = edits::prepare(program, vec![edit.clone()], vec![])?;
    // Reconstruct the actual graph context for function/constructor identities.
    // The requested payload itself comes only from the typed serializer above.
    let mut proposed = program.clone();
    let document = proposed
        .documents
        .iter_mut()
        .find(|d| d.id == doc.id)
        .unwrap();
    document
        .source
        .replace_range(edit.span.clone(), &edit.replacement);
    document.sha256 = lua::source_hash(&document.source);
    proposed.bindings = proposed
        .bindings
        .iter()
        .map(|binding| {
            Ok(super::Binding {
                request: edits::rebase_request(&proposed, &provisional, &binding.request)?,
                resolution: binding.resolution.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let after = replay(&proposed)?;
    let next = after
        .events
        .iter()
        .find(|e| {
            e.order == order
                && e.api == original.api
                && e.document == original.document
                && e.span.start == original.span.start
        })
        .ok_or("The edited declaration was not reached at its original position")?;
    let unchanged = original.arguments.len() == next.arguments.len()
        && original
            .arguments
            .iter()
            .zip(&next.arguments)
            .all(|(a, b)| edits::same_value(a, b));
    let expected = if unchanged {
        vec![]
    } else {
        vec![edits::ExpectedCall {
            order,
            before_arguments: original.arguments.clone(),
            after_arguments: next.arguments.clone(),
        }]
    };
    let plan = edits::prepare(program, vec![edit], expected)?;
    let review = edits::review_with_before(program, &plan, before)?;
    Ok(scalar_edit::Comparison { plan, review, proposed_native_value: lua::scalar_codec::Outcome::Unresolved { reason: "The requested typed declaration was reconstructed; native acceptance and runtime effects remain unverified.".into() }, table_layout_preserved: false })
}
