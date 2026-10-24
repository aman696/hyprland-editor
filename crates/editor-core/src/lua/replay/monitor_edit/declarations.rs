//! Whole-declaration source intentions. All inputs are immutable captured data;
//! syntax sites are coordinates, never file ownership or live-write authority.
use super::*;
use crate::lua::{named, syntax_tree, text};
use std::{collections::BTreeMap, ops::Range};
use tree_sitter::Node;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub span: Range<usize>,
    pub line: usize,
    pub kind: String,
    pub observed: usize,
    pub standalone: bool,
    pub output_hint: Option<Vec<u8>>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub program_sha256: String,
    pub document: String,
    pub document_sha256: String,
    pub version: String,
    pub compiler: super::super::CompilerProfile,
    pub statements: Vec<Site>,
    pub declarations: Vec<Site>,
    pub runtime_verified: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "position", rename_all = "snake_case", deny_unknown_fields)]
pub enum Anchor {
    Beginning,
    End,
    Before { span: Range<usize> },
    After { span: Range<usize> },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    Insert {
        document: String,
        anchor: Anchor,
        rule: monitor::Rule,
    },
    Remove {
        document: String,
        span: Range<usize>,
    },
}
fn standalone(node: Node<'_>) -> bool {
    node.parent()
        .is_some_and(|p| matches!(p.kind(), "chunk" | "block"))
}
fn spelled_monitor(node: Node<'_>, source: &str) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    if !matches!(
        name.kind(),
        "dot_index_expression" | "bracket_index_expression"
    ) {
        return false;
    }
    let Some(table) = name.child_by_field_name("table") else {
        return false;
    };
    let Some(field) = name.child_by_field_name("field") else {
        return false;
    };
    table.kind() == "identifier"
        && text(table, source) == "hl"
        && if name.kind() == "dot_index_expression" {
            text(field, source) == "monitor"
        } else {
            matches!(crate::lua::evaluate(field,source,&BTreeMap::new(),0),Some(crate::lua::Expr::Text(value)) if value=="monitor")
        }
}
fn exact<'a>(tree: &'a tree_sitter::Tree, span: Range<usize>) -> Result<Node<'a>, String> {
    if span.is_empty() || span.end > tree.root_node().end_byte() {
        return Err("Declaration source range is outside this captured document".into());
    }
    tree.root_node()
        .descendant_for_byte_range(span.start, span.end)
        .filter(|n| n.byte_range() == span)
        .ok_or_else(|| "The selected declaration site is not an exact syntax node".into())
}
pub fn inspect(program: &Program, document: &str) -> Result<Inventory, String> {
    super::super::validate_program(program)?;
    let doc = program
        .documents
        .iter()
        .find(|d| d.id == document)
        .ok_or("Unknown captured declaration document")?;
    let before = replay(program)?;
    let tree = syntax_tree(&doc.source)?;
    if tree.root_node().has_error() {
        return Err("Declaration source does not parse".into());
    }
    let mut result = Inventory {
        program_sha256: edits::fingerprint(program)?,
        document: document.into(),
        document_sha256: doc.sha256.clone(),
        version: program.version.clone(),
        compiler: before.compiler_profile,
        statements: vec![],
        declarations: vec![],
        runtime_verified: false,
    };
    type Observations = BTreeMap<(usize, usize), (usize, Option<Vec<u8>>)>;
    let mut observations = Observations::new();
    for event in before
        .events
        .iter()
        .filter(|e| e.document == document && e.api == "hl.monitor")
    {
        let hint = event.arguments.first().and_then(|arg| {
            let Observed::Table(rows) = arg else {
                return None;
            };
            rows.iter().find_map(|(k, v)| match (k, v) {
                (Observed::Bytes(k), Observed::Bytes(v)) if k == b"output" => {
                    Some(v[..v.len().min(128)].to_vec())
                }
                _ => None,
            })
        });
        let entry = observations
            .entry((event.span.start, event.span.end))
            .or_insert((0, hint.clone()));
        entry.0 += 1;
        if entry.1 != hint {
            entry.1 = None;
        }
    }
    let mut pending = vec![tree.root_node()];
    let mut visited = 0;
    while let Some(node) = pending.pop() {
        visited += 1;
        if visited > 100_000 || result.statements.len() + result.declarations.len() > 4096 {
            return Err("Declaration site inventory exceeds its node/site budget".into());
        }
        let (observed, output_hint) = observations
            .get(&(node.start_byte(), node.end_byte()))
            .cloned()
            .unwrap_or_default();
        let site = || Site {
            span: node.byte_range(),
            line: node.start_position().row + 1,
            kind: node.kind().into(),
            observed,
            standalone: standalone(node),
            output_hint: output_hint.clone(),
        };
        if standalone(node) && node.kind() != "comment" {
            result.statements.push(site());
        }
        if node.kind() == "function_call" && (spelled_monitor(node, &doc.source) || observed > 0) {
            result.declarations.push(site());
        }
        pending.extend(named(node).into_iter().rev());
    }
    if result.statements.len() + result.declarations.len() > 4096 {
        return Err("Declaration site inventory exceeds its site budget".into());
    }
    edits::fingerprint(&result)?;
    Ok(result)
}
fn expected_table(version: &str, rule: &monitor::Rule) -> Result<Observed, String> {
    rule.validate(version)?;
    let mut rows = BTreeMap::from([(
        "output".to_string(),
        proposed(version, "output", &Value::String(rule.output.text()))?.0,
    )]);
    for (field, value) in &rule.fields {
        rows.insert(field.clone(), proposed(version, field, value)?.0);
    }
    Ok(Observed::Table(
        rows.into_iter()
            .map(|(k, v)| (Observed::Bytes(k.into_bytes()), v))
            .collect(),
    ))
}
pub fn compare(program: &Program, change: &Change) -> Result<Comparison, String> {
    let before = replay(program)?;
    let document = match change {
        Change::Insert { document, .. } | Change::Remove { document, .. } => document,
    };
    let doc = program
        .documents
        .iter()
        .find(|d| d.id == *document)
        .ok_or("Unknown captured declaration document")?;
    let tree = syntax_tree(&doc.source)?;
    if tree.root_node().has_error() {
        return Err("Declaration source does not parse".into());
    }
    let (span, replacement, structural, reason) = match change {
        Change::Insert { anchor, rule, .. } => {
            let at = match anchor {
                Anchor::Beginning => named(tree.root_node())
                    .first()
                    .map_or(doc.source.len(), |n| n.start_byte()),
                Anchor::End => named(tree.root_node())
                    .last()
                    .filter(|n| n.kind() == "return_statement")
                    .map_or(doc.source.len(), |n| n.start_byte()),
                Anchor::Before { span } | Anchor::After { span } => {
                    let node = exact(&tree, span.clone())?;
                    if !standalone(node) || node.kind() == "comment" {
                        return Err(
                            "Choose an exact statement boundary for declaration insertion".into(),
                        );
                    }
                    if matches!(anchor, Anchor::After { .. }) {
                        if node.kind() == "return_statement" {
                            return Err("Insert before the return; Lua requires it to be the last statement".into());
                        }
                        node.end_byte()
                    } else {
                        node.start_byte()
                    }
                }
            };
            let table = expected_table(&program.version, rule)?;
            let call = format!("hl.monitor({})", scalar_edit::print(&table)?);
            let newline = if doc.source.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            // A semicolon separates a preceding call/parenthesized expression;
            // line breaks keep existing line comments from consuming new code.
            let prefix = format!("{newline};");
            let call_span = prefix.len()..prefix.len() + call.len();
            let replacement = format!("{prefix}{call};{newline}");
            let structural = vec![edits::StructuralCall::Insert {
                document: document.clone(),
                edit_span: at..at,
                call_span,
                api: "hl.monitor".into(),
                arguments: vec![table],
            }];
            (at..at,replacement,structural,"The declaration is inserted in captured source. Its execution is modeled only on reached paths; native monitor state remains unverified.")
        }
        Change::Remove { span, .. } => {
            let node = exact(&tree, span.clone())?;
            let events = before
                .events
                .iter()
                .filter(|e| e.document == *document && e.span == *span && e.api == "hl.monitor")
                .collect::<Vec<_>>();
            if node.kind() != "function_call"
                || !spelled_monitor(node, &doc.source) && events.is_empty()
            {
                return Err("Select a spelled or observed monitor declaration call".into());
            }
            let trivia = scalar_edit::layout::erase_tokens(node, &doc.source)?;
            // Embedded calls must continue returning zero values. A nil literal
            // would change varargs/return-list arity even when no effect remains.
            let replacement = if standalone(node) {
                trivia
            } else {
                format!("(function() end)(){trivia}")
            };
            let structural = vec![edits::StructuralCall::RemoveSite {
                document: document.clone(),
                span: span.clone(),
                api: "hl.monitor".into(),
            }];
            (span.clone(),replacement,structural,"The source declaration is removed. Argument/callee effects and other reached declarations are compared separately; inherited monitor values remain unresolved.")
        }
    };
    let old = doc
        .source
        .get(span.clone())
        .ok_or("Invalid declaration source range")?;
    let edit = edits::SourceEdit {
        document: document.clone(),
        base_sha256: doc.sha256.clone(),
        span,
        old_bytes_sha256: crate::lua::source_hash(old),
        replacement,
        semantic_targets: vec!["monitor.declaration".into()],
    };
    let plan = edits::prepare_structural(program, vec![edit], structural)?;
    let review = edits::review_with_before(program, &plan, before)?;
    let result = Comparison {
        plan,
        review,
        proposed_native_value: Outcome::Unresolved {
            reason: reason.into(),
        },
        table_layout_preserved: true,
    };
    edits::fingerprint(&result)?;
    Ok(result)
}
