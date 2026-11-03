//! Structural family edits are explicit source-site removal/insertion intentions.
pub use super::super::monitor_edit::declarations::Anchor;
use super::super::{Binding, Document, Report};
use super::*;
use std::{collections::BTreeSet, ops::Range};

fn edit(
    document: &Document,
    span: Range<usize>,
    replacement: String,
    api: &str,
) -> Result<edits::SourceEdit, String> {
    Ok(edits::SourceEdit {
        document: document.id.clone(),
        base_sha256: document.sha256.clone(),
        old_bytes_sha256: lua::source_hash(
            document
                .source
                .get(span.clone())
                .ok_or("Invalid declaration range")?,
        ),
        span,
        replacement,
        semantic_targets: vec![format!("{api}.declarations")],
    })
}
fn review(
    program: &Program,
    before: Report,
    document: &str,
    range: Range<usize>,
    rendered: &str,
    api: &str,
) -> Result<scalar_edit::Comparison, String> {
    let doc = program
        .documents
        .iter()
        .find(|d| d.id == document)
        .ok_or("Selected source file is unavailable")?;
    let newline = if doc.source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let prefix = format!("{newline};");
    let inserted = if rendered.is_empty() {
        String::new()
    } else {
        format!("{prefix}{rendered}{newline}")
    };
    let mut trivia = String::new();
    let mut intentions = vec![];
    if !range.is_empty() {
        let tree = syntax_tree(&doc.source)?;
        let node = tree
            .root_node()
            .descendant_for_byte_range(range.start, range.end)
            .filter(|n| n.byte_range() == range)
            .ok_or("Selected declaration is no longer an exact source node")?;
        trivia = scalar_edit::layout::erase_tokens(node, &doc.source)?;
        let mut sites = BTreeSet::new();
        for event in before.events.iter().filter(|e| {
            e.document == document && e.span.start >= range.start && e.span.end <= range.end
        }) {
            if !same_family(api, &event.api) {
                return Err("This container also registers another feature family. Edit its declarations separately to retain them.".into());
            }
            if sites.insert((event.span.start, event.span.end, event.api.clone())) {
                intentions.push(edits::StructuralCall::RemoveSite {
                    document: document.into(),
                    span: event.span.clone(),
                    api: event.api.clone(),
                });
            }
        }
    }
    if range.is_empty() && inserted.is_empty() {
        return Err("Add a declaration in the graphical editor before comparing".into());
    }
    let call_offset = trivia.len() + prefix.len();
    let patches = vec![edit(
        doc,
        range.clone(),
        format!("{trivia}{inserted}"),
        api,
    )?];
    let provisional = edits::prepare(program, patches.clone(), vec![])?;
    let mut proposed = program.clone();
    let target = proposed
        .documents
        .iter_mut()
        .find(|d| d.id == document)
        .unwrap();
    // Apply in the same sorted/reversed order as the source transaction engine.
    for patch in provisional.edits.iter().rev() {
        target
            .source
            .replace_range(patch.span.clone(), &patch.replacement);
    }
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
    if !inserted.is_empty() {
        let mut prototype = program.clone();
        prototype.root = "declaration-draft".into();
        prototype.documents = vec![Document {
            id: prototype.root.clone(),
            source: rendered.into(),
            sha256: lua::source_hash(rendered),
            load_name: b"<declaration-draft>".to_vec(),
        }];
        prototype.bindings.clear();
        prototype.initial_package_path = None;
        prototype.initial_package_path_unknown_tail = false;
        prototype.working_directory = None;
        let intended = replay(&prototype)?;
        if intended.halt.is_some() || !intended.loads.is_empty() {
            return Err(
                "The graphical declarations contain an unsupported registration operation".into(),
            );
        }
        for event in intended.events {
            if !same_family(api, &event.api) {
                return Err("The serializer produced another declaration family".into());
            }
            let call_span = call_offset + event.span.start..call_offset + event.span.end;
            let actual_span = range.start + call_span.start..range.start + call_span.end;
            let arguments = after
                .events
                .iter()
                .find(|e| e.document == document && e.span == actual_span && e.api == event.api)
                .map_or(event.arguments, |e| e.arguments.clone());
            intentions.push(edits::StructuralCall::Insert {
                document: document.into(),
                edit_span: range.clone(),
                call_span,
                api: event.api,
                arguments,
            });
        }
    }
    let plan = edits::prepare_structural(program, patches, intentions)?;
    let review = edits::review_with_before(program, &plan, before)?;
    Ok(scalar_edit::Comparison{plan,review,proposed_native_value:lua::scalar_codec::Outcome::Unresolved{reason:"Graphical declarations were added, removed or rebuilt in captured source. Review ordering and any additional findings; runtime behavior remains unverified.".into()},table_layout_preserved:false})
}
pub(super) fn replace(
    program: &Program,
    before: Report,
    event: &Event,
    rendered: &str,
) -> Result<scalar_edit::Comparison, String> {
    review(
        program,
        before,
        &event.document,
        event.span.clone(),
        rendered,
        &event.api,
    )
}
pub fn insert(
    program: &Program,
    document: &str,
    anchor: &Anchor,
    api: &str,
    target: &Workspace,
    catalogue: &[Setting],
) -> Result<scalar_edit::Comparison, String> {
    validate_target(program, target, catalogue)?;
    if !supports(api) {
        return Err("Choose a supported declaration family".into());
    }
    let rendered = canonical(target, api)?;
    validate_rendered(&rendered, target, api, catalogue)?;
    let doc = program
        .documents
        .iter()
        .find(|d| d.id == document)
        .ok_or("Selected source file is unavailable")?;
    let tree = syntax_tree(&doc.source)?;
    let at = match anchor {
        Anchor::Beginning => named(tree.root_node())
            .first()
            .map_or(doc.source.len(), |n| n.start_byte()),
        Anchor::End => named(tree.root_node())
            .last()
            .filter(|n| n.kind() == "return_statement")
            .map_or(doc.source.len(), |n| n.start_byte()),
        Anchor::Before { span } | Anchor::After { span } => {
            let node = tree
                .root_node()
                .descendant_for_byte_range(span.start, span.end)
                .filter(|n| {
                    n.byte_range() == *span
                        && n.parent()
                            .is_some_and(|p| matches!(p.kind(), "chunk" | "block"))
                        && n.kind() != "comment"
                })
                .ok_or("Choose an exact statement boundary")?;
            if matches!(anchor, Anchor::After { .. }) {
                if node.kind() == "return_statement" {
                    return Err("Insert before the module return statement".into());
                }
                node.end_byte()
            } else {
                node.start_byte()
            }
        }
    };
    review(program, replay(program)?, document, at..at, &rendered, api)
}
