//! Combine explicit graphical choices with immutable, multi-file source edits.
//! This module has no filesystem or compositor access.
use super::{edits, replay, Binding, Document, Program};
use crate::{lua, workspace::Workspace, Setting};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ops::Range};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Keep,
    Append,
    ReplaceReached,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Settings,
    Monitors,
    Devices,
    Bindings,
    Workspaces,
    Rules,
    Animations,
    Gestures,
}
impl Family {
    pub const ALL: [Self; 8] = [
        Self::Settings,
        Self::Monitors,
        Self::Devices,
        Self::Bindings,
        Self::Workspaces,
        Self::Rules,
        Self::Animations,
        Self::Gestures,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Monitors => "Monitors",
            Self::Devices => "Devices",
            Self::Bindings => "Bindings and submaps",
            Self::Workspaces => "Workspace rules",
            Self::Rules => "Window and layer rules",
            Self::Animations => "Curves and animations",
            Self::Gestures => "Gestures",
        }
    }
    fn of(api: &str) -> Option<Self> {
        Some(match api {
            "hl.config" => Self::Settings,
            "hl.monitor" => Self::Monitors,
            "hl.device" => Self::Devices,
            "hl.bind" | "hl.define_submap" => Self::Bindings,
            "hl.workspace_rule" => Self::Workspaces,
            "hl.window_rule" | "hl.layer_rule" => Self::Rules,
            "hl.curve" | "hl.animation" => Self::Animations,
            "hl.gesture" => Self::Gestures,
            _ => return None,
        })
    }
    fn render(self, w: &Workspace, catalogue: &[Setting]) -> Result<String, String> {
        let version = &w.draft.version;
        match self {
            Self::Settings if w.draft.values.is_empty() => Ok(String::new()),
            Self::Settings => {
                let source = w.draft.export_lua(catalogue)?;
                let tree = lua::syntax_tree(&source)?;
                let calls = lua::named(tree.root_node())
                    .into_iter()
                    .filter(|n| n.kind() != "comment")
                    .collect::<Vec<_>>();
                if calls.len() != 1 || calls[0].kind() != "function_call" {
                    return Err("Scalar serializer produced an unexpected statement".into());
                }
                Ok(lua::text(calls[0], &source).into())
            }
            Self::Monitors => crate::monitor::export_lua(&w.monitors, version),
            Self::Devices => crate::device::export_lua(&w.devices, version),
            Self::Bindings => w.bindings.export_lua(version),
            Self::Workspaces => crate::workspace_rule::export_lua(&w.workspace_rules, version),
            Self::Rules => crate::rule::export_lua(&w.rules, version),
            Self::Animations => w.animations.export_lua(version),
            Self::Gestures => w.gestures.export_lua(version),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub document: String,
    pub load_name: Vec<u8>,
    pub content: String,
    pub sha256: String,
    pub changed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepared {
    pub root: String,
    pub modes: [Mode; 8],
    pub files: Vec<File>,
    pub removed_sites: usize,
    pub inserted_sites: usize,
    pub findings: Vec<edits::DifferenceKind>,
    pub before_complete: bool,
    pub after_complete: bool,
}

fn contains(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && outer.end >= inner.end
}
fn patch(doc: &Document, span: Range<usize>, replacement: String) -> edits::SourceEdit {
    edits::SourceEdit {
        document: doc.id.clone(),
        base_sha256: doc.sha256.clone(),
        old_bytes_sha256: lua::source_hash(&doc.source[span.clone()]),
        span,
        replacement,
        semantic_targets: vec!["combined-project-copy".into()],
    }
}

pub fn prepare_workspace(
    workspace: &Workspace,
    modes: [Mode; 8],
    catalogue: &[Setting],
) -> Result<Prepared, String> {
    workspace.validate(catalogue)?;
    let graph = workspace
        .source_graph
        .as_ref()
        .ok_or("A captured source graph is required")?;
    let original = graph.program(
        &workspace.sources,
        workspace.root_source.as_deref(),
        &workspace.draft.version,
    )?;
    let program = graph.private.program(&original, &graph.revisions)?;
    let report = replay(&program)?;
    graph.private.check_reached(&program, &report)?;
    prepare(&program, workspace, modes, catalogue)
}

/// Every family choice is explicit. Keep retains current private source bytes;
/// append places drafts at the root's final statement boundary; replace removes
/// reached standalone calls first. Unreached code is retained, never guessed.
pub fn prepare(
    program: &Program,
    workspace: &Workspace,
    modes: [Mode; 8],
    catalogue: &[Setting],
) -> Result<Prepared, String> {
    workspace.validate(catalogue)?;
    if program.version != workspace.draft.version {
        return Err("Source and graphical drafts target different versions".into());
    }
    let before = replay(program)?;
    let mode = |api: &str| {
        Family::of(api).map(|f| modes[Family::ALL.iter().position(|v| *v == f).unwrap()])
    };
    if before.halt.is_some() && modes.contains(&Mode::ReplaceReached) {
        return Err("Replace needs a complete captured replay. Resolve missing inputs first, or keep/append this group.".into());
    }
    let mut patches = vec![];
    let mut intentions = vec![];
    for doc in &program.documents {
        let tree = lua::syntax_tree(&doc.source)?;
        let mut sites = before
            .events
            .iter()
            .filter(|e| e.document == doc.id && mode(&e.api) == Some(Mode::ReplaceReached))
            .map(|e| (e.span.start, e.span.end, e.api.clone()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        sites.sort_by_key(|(start, end, _)| (*start, std::cmp::Reverse(*end)));
        let mut covered: Option<Range<usize>> = None;
        for (start, end, api) in sites {
            let span = start..end;
            intentions.push(edits::StructuralCall::RemoveSite {
                document: doc.id.clone(),
                span: span.clone(),
                api,
            });
            if covered.as_ref().is_some_and(|outer| contains(outer, &span)) {
                continue;
            }
            let node = tree
                .root_node()
                .descendant_for_byte_range(start, end)
                .filter(|n| n.byte_range() == span)
                .ok_or("Captured declaration no longer has an exact syntax site")?;
            if node.kind() != "function_call"
                || !node
                    .parent()
                    .is_some_and(|p| matches!(p.kind(), "chunk" | "block"))
            {
                return Err("A replacement group contains a consumed call. Edit that declaration separately, or keep/append this group.".into());
            }
            if before.events.iter().any(|e| {
                e.document == doc.id
                    && contains(&span, &e.span)
                    && mode(&e.api) != Some(Mode::ReplaceReached)
            }) || before
                .loads
                .iter()
                .any(|l| l.request.document == doc.id && contains(&span, &l.request.span))
            {
                return Err("A replacement would remove another group or a module load inside a declaration. Edit it separately, or keep/append this group.".into());
            }
            patches.push(patch(
                doc,
                span.clone(),
                super::scalar_edit::layout::erase_tokens(node, &doc.source)?,
            ));
            covered = Some(span);
        }
    }
    let removed_sites = intentions.len();
    let mut rendered = String::new();
    for (family, mode) in Family::ALL.into_iter().zip(modes) {
        if mode != Mode::Keep {
            let source = family.render(workspace, catalogue)?;
            if !source.trim().is_empty() {
                rendered.push_str(&format!("-- {}\n{source}\n", family.label()));
            }
        }
    }
    let mut prototype = program.clone();
    prototype.root = "project-draft".into();
    prototype.documents = vec![Document {
        id: prototype.root.clone(),
        source: rendered.clone(),
        sha256: lua::source_hash(&rendered),
        load_name: b"<project-draft>".to_vec(),
    }];
    prototype.bindings.clear();
    prototype.initial_package_path = None;
    prototype.initial_package_path_unknown_tail = false;
    prototype.working_directory = None;
    let intended = replay(&prototype)?;
    if intended.halt.is_some()
        || !intended.loads.is_empty()
        || intended
            .events
            .iter()
            .any(|e| mode(&e.api).is_none_or(|m| m == Mode::Keep))
    {
        return Err("Graphical serialization produced an unsupported operation".into());
    }
    let inserted_sites = intended.events.len();
    let mut insertion = None;
    if !rendered.trim().is_empty() {
        let root = program
            .documents
            .iter()
            .find(|d| d.id == program.root)
            .ok_or("Root source missing")?;
        let tree = lua::syntax_tree(&root.source)?;
        let at = lua::named(tree.root_node())
            .into_iter()
            .find(|n| n.kind() == "return_statement")
            .map_or(root.source.len(), |n| n.start_byte());
        let prefix = "\n;do\n-- Graphical drafts: reviewed combined copy\n";
        let payload = format!("{prefix}{rendered}\nend;\n");
        // Adjacent insertions share a patch with the preceding removed site.
        if let Some(edit) = patches
            .iter_mut()
            .find(|p| p.document == root.id && p.span.end == at)
        {
            let offset = edit.replacement.len() + prefix.len();
            edit.replacement.push_str(&payload);
            insertion = Some((edit.span.clone(), offset));
        } else {
            patches.push(patch(root, at..at, payload));
            insertion = Some((at..at, prefix.len()));
        }
    }
    let mut findings = vec![];
    let mut after_complete = before.halt.is_none();
    if !patches.is_empty() {
        let provisional = edits::prepare(program, patches.clone(), vec![])?;
        let mut proposed = program.clone();
        for doc in &mut proposed.documents {
            for edit in provisional
                .edits
                .iter()
                .filter(|e| e.document == doc.id)
                .rev()
            {
                doc.source
                    .replace_range(edit.span.clone(), &edit.replacement);
            }
            doc.sha256 = lua::source_hash(&doc.source);
        }
        proposed.bindings = program
            .bindings
            .iter()
            .map(|b| {
                Ok(Binding {
                    request: edits::rebase_request(&proposed, &provisional, &b.request)?,
                    resolution: b.resolution.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let after = replay(&proposed)?;
        if let Some((span, offset)) = insertion {
            let delta: i64 = provisional
                .edits
                .iter()
                .filter(|p| p.document == program.root && p.span.start < span.start)
                .map(|p| p.replacement.len() as i64 - p.span.len() as i64)
                .sum();
            for event in intended.events {
                let call_span = offset + event.span.start..offset + event.span.end;
                let actual_start = usize::try_from(span.start as i64 + delta)
                    .map_err(|_| "Invalid inserted offset")?;
                let actual = after.events.iter().find(|e| e.document == program.root && e.api == event.api && e.span == (actual_start + call_span.start..actual_start + call_span.end))
                    .ok_or("A graphical declaration was not reached in the proposed copy. Resolve source inputs or root shadowing first.")?;
                intentions.push(edits::StructuralCall::Insert {
                    document: program.root.clone(),
                    edit_span: span.clone(),
                    call_span,
                    api: event.api,
                    arguments: actual.arguments.clone(),
                });
            }
        }
        let plan = edits::prepare_structural(program, patches, intentions)?;
        let review = edits::review_with_before(program, &plan, before.clone())?;
        if review.structural_calls_matched != plan.structural_calls.len() {
            return Err(
                "Some requested declarations did not match the combined source review".into(),
            );
        }
        findings = review.differences.into_iter().map(|d| d.kind).collect();
        after_complete = review.after_model_complete;
        return Ok(Prepared {
            root: program.root.clone(),
            modes,
            files: proposed
                .documents
                .into_iter()
                .map(|d| {
                    let changed = program
                        .documents
                        .iter()
                        .any(|old| old.id == d.id && old.source != d.source);
                    File {
                        document: d.id,
                        load_name: d.load_name,
                        content: d.source,
                        sha256: d.sha256,
                        changed,
                    }
                })
                .collect(),
            removed_sites,
            inserted_sites,
            findings,
            before_complete: before.halt.is_none(),
            after_complete,
        });
    }
    Ok(Prepared {
        root: program.root.clone(),
        modes,
        files: program
            .documents
            .iter()
            .map(|d| File {
                document: d.id.clone(),
                load_name: d.load_name.clone(),
                content: d.source.clone(),
                sha256: d.sha256.clone(),
                changed: false,
            })
            .collect(),
        removed_sites,
        inserted_sites,
        findings,
        before_complete: before.halt.is_none(),
        after_complete,
    })
}
