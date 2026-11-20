//! Source-spanned syntactic loader requests. Inspection never grants a file capability.
use super::{evaluate, named, source_hash, text, Expr};
use crate::{AUDITED_MAIN, BASELINE};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};
use tree_sitter::Node;

const MAX_SITES: usize = 512;
const MAX_BARRIERS: usize = 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Loader {
    Require,
    OriginalRequire,
    Dofile,
    Loadfile,
}
fn builtin(name: &str) -> Option<Loader> {
    match name {
        "require" => Some(Loader::Require),
        "__require" => Some(Loader::OriginalRequire),
        "dofile" => Some(Loader::Dofile),
        "loadfile" => Some(Loader::Loadfile),
        _ => None,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteScope {
    Linear,
    Conditional,
    FunctionBody,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Binding {
    EntryCandidate,
    CapturedCandidate,
    Shadowed,
    Uncertain,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarrierKind {
    UnknownCall,
    ControlFlow,
    EnvironmentMutation,
    NonlocalAssignment,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub loader: Loader,
    pub callee: String,
    pub module: Option<String>,
    pub scope: SiteScope,
    pub binding: Binding,
    pub span: Range<usize>,
    pub argument_span: Option<Range<usize>>,
    pub line: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Barrier {
    pub kind: BarrierKind,
    pub span: Range<usize>,
    pub scope: SiteScope,
    pub line: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub source_sha256: String,
    pub version: String,
    pub sites: Vec<Site>,
    pub barriers: Vec<Barrier>,
}
impl Inventory {
    pub fn validate(&self, source: &str, version: &str) -> Result<(), String> {
        if source.len() > crate::MAX_DOCUMENT_BYTES
            || self.version != version
            || ![BASELINE, AUDITED_MAIN].contains(&version)
            || self.source_sha256 != source_hash(source)
            || self.sites.len() > MAX_SITES
            || self.barriers.len() > MAX_BARRIERS
        {
            return Err("Invalid or stale module-request inventory".into());
        }
        let span_ok = |r: &Range<usize>| {
            r.start < r.end
                && r.end <= source.len()
                && source.is_char_boundary(r.start)
                && source.is_char_boundary(r.end)
        };
        for site in &self.sites {
            if !span_ok(&site.span)
                || site.callee.is_empty()
                || site.callee.len() > 16384
                || !source[site.span.clone()].starts_with(&site.callee)
                || site.module.as_ref().is_some_and(|s| s.len() > 16384)
                || site.argument_span.as_ref().is_some_and(|r| {
                    !span_ok(r) || r.start < site.span.start || r.end > site.span.end
                })
            {
                return Err("Invalid module-request spans or text".into());
            }
        }
        for b in &self.barriers {
            if !span_ok(&b.span) {
                return Err("Invalid module barrier span".into());
            }
        }
        // Count lines in one pass over disjoint source slices, not once per report row.
        let mut positions = self
            .sites
            .iter()
            .map(|s| s.span.start)
            .chain(self.barriers.iter().map(|b| b.span.start))
            .collect::<Vec<_>>();
        positions.sort_unstable();
        positions.dedup();
        let mut lines = BTreeMap::new();
        let mut previous = 0;
        let mut line = 1;
        for position in positions {
            line += source[previous..position]
                .bytes()
                .filter(|c| *c == b'\n')
                .count();
            lines.insert(position, line);
            previous = position;
        }
        if self.sites.iter().any(|s| lines[&s.span.start] != s.line)
            || self.barriers.iter().any(|b| lines[&b.span.start] != b.line)
        {
            return Err("Invalid module-request line evidence".into());
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > crate::MAX_DOCUMENT_BYTES {
            return Err("Module-request inventory exceeds 1 MiB".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
enum Name {
    Loader(Loader, bool),
    Other,
    Unknown,
}
#[derive(Clone)]
struct Frame {
    values: BTreeMap<String, Expr>,
    names: BTreeMap<String, Name>,
    locals: BTreeMap<String, usize>,
    writes: BTreeMap<usize, (Option<Expr>, Name)>,
    epoch: usize,
    uncertain: bool,
}
impl Frame {
    fn initial() -> Self {
        Self {
            values: BTreeMap::new(),
            names: ["require", "__require", "dofile", "loadfile"]
                .into_iter()
                .map(|n| (n.into(), Name::Loader(builtin(n).unwrap(), false)))
                .collect(),
            locals: BTreeMap::new(),
            writes: BTreeMap::new(),
            epoch: 0,
            uncertain: false,
        }
    }
    fn taint(&mut self) {
        self.uncertain = true;
        self.values.clear();
        self.epoch += 1;
        for value in self.writes.values_mut() {
            *value = (None, Name::Unknown);
        }
    }
    fn name(&self, n: Node<'_>, source: &str) -> Name {
        if n.kind() == "parenthesized_expression" {
            return named(n)
                .first()
                .map_or(Name::Unknown, |n| self.name(*n, source));
        }
        if n.kind() == "identifier" {
            return self
                .names
                .get(text(n, source))
                .copied()
                .unwrap_or(Name::Unknown);
        }
        if [
            "function_definition",
            "table_constructor",
            "number",
            "string",
            "nil",
            "true",
            "false",
        ]
        .contains(&n.kind())
        {
            Name::Other
        } else {
            Name::Unknown
        }
    }
}
struct Inspect<'a> {
    source: &'a str,
    output: Inventory,
    visited: usize,
    next_local: usize,
}
impl Inspect<'_> {
    fn barrier(
        &mut self,
        node: Node<'_>,
        scope: SiteScope,
        kind: BarrierKind,
    ) -> Result<(), String> {
        if self.output.barriers.len() >= MAX_BARRIERS {
            return Err("More than 256 unresolved module-state sites".into());
        }
        self.output.barriers.push(Barrier {
            kind,
            span: node.byte_range(),
            scope,
            line: node.start_position().row + 1,
        });
        Ok(())
    }
    fn bind_local(
        &mut self,
        frame: &mut Frame,
        name: &str,
        value: Option<Expr>,
        binding: Name,
        declaration: bool,
    ) -> Result<(), String> {
        if frame.locals.len() >= 1024 && !frame.locals.contains_key(name) {
            return Err("Too many dependency-inspection locals".into());
        }
        if declaration {
            self.next_local += 1;
            frame.locals.insert(name.into(), self.next_local);
        }
        let binding = match binding {
            Name::Loader(k, _) => Name::Loader(k, true),
            other => other,
        };
        if !declaration {
            let id = *frame
                .locals
                .get(name)
                .ok_or("Assignment has no lexical local")?;
            frame.writes.insert(id, (value.clone(), binding));
        }
        frame.values.remove(name);
        if let Some(value) = value {
            frame.values.insert(name.into(), value);
        }
        frame.names.insert(name.into(), binding);
        Ok(())
    }
    fn visit(
        &mut self,
        node: Node<'_>,
        scope: SiteScope,
        frame: &mut Frame,
        depth: usize,
    ) -> Result<(), String> {
        self.visited += 1;
        if self.visited > 100_000 || depth > 128 {
            return Err("Module-request syntax exceeds its inspection budget".into());
        }
        match node.kind() {
            "comment" | "empty_statement" | "hash_bang_line" => {}
            "function_definition" | "function_declaration" => {
                if let Some(name) = node.child_by_field_name("name") {
                    if name.kind() == "identifier" {
                        let name = text(name, self.source);
                        if text(node, self.source).trim_start().starts_with("local ") {
                            self.bind_local(frame, name, None, Name::Other, true)?;
                        } else {
                            self.barrier(node, scope, BarrierKind::EnvironmentMutation)?;
                            frame.taint();
                            frame.names.insert(name.into(), Name::Other);
                        }
                    } else {
                        self.barrier(node, scope, BarrierKind::EnvironmentMutation)?;
                        frame.taint();
                    }
                }
                let mut nested = frame.clone();
                nested.taint();
                if let Some(params) = node.child_by_field_name("parameters") {
                    for p in named(params) {
                        if p.kind() == "identifier" {
                            self.bind_local(
                                &mut nested,
                                text(p, self.source),
                                None,
                                Name::Unknown,
                                true,
                            )?;
                        }
                    }
                }
                if let Some(body) = node.child_by_field_name("body") {
                    self.visit(body, SiteScope::FunctionBody, &mut nested, depth + 1)?;
                }
            }
            "do_statement" => {
                let mut nested = frame.clone();
                if let Some(body) = node.child_by_field_name("body") {
                    self.visit(body, scope, &mut nested, depth + 1)?;
                }
                if nested.epoch != frame.epoch {
                    frame.taint();
                }
                // Propagate assignments by lexical identity, so an inner shadow does
                // not overwrite the outer value and an outer assignment is not lost.
                let locals = frame.locals.clone();
                for (name, id) in locals {
                    if let Some((value, binding)) = nested.writes.get(&id) {
                        frame.values.remove(&name);
                        if let Some(value) = value {
                            frame.values.insert(name.clone(), value.clone());
                        }
                        frame.names.insert(name, *binding);
                        frame.writes.insert(id, (value.clone(), *binding));
                    }
                }
            }
            "variable_declaration" | "assignment_statement" => {
                let children = named(node);
                let assignment = if node.kind() == "variable_declaration" {
                    children
                        .first()
                        .filter(|n| n.kind() == "assignment_statement")
                        .copied()
                        .unwrap_or(node)
                } else {
                    node
                };
                let parts = named(assignment);
                let vars = parts
                    .iter()
                    .find(|n| n.kind() == "variable_list")
                    .map(|n| named(*n))
                    .unwrap_or_default();
                let values = parts
                    .iter()
                    .find(|n| n.kind() == "expression_list")
                    .map(|n| named(*n))
                    .unwrap_or_default();
                let mut results = vec![];
                for value in &values {
                    let pure = evaluate(*value, self.source, &frame.values, 0);
                    let binding = if pure.is_some() {
                        Name::Other
                    } else {
                        frame.name(*value, self.source)
                    };
                    self.visit(*value, scope, frame, depth + 1)?;
                    results.push((pure, binding));
                }
                let dynamic_tail = values
                    .last()
                    .is_some_and(|v| ["function_call", "vararg_expression"].contains(&v.kind()));
                let local = node.kind() == "variable_declaration";
                let simple = vars.iter().all(|v| {
                    v.kind() == "identifier"
                        && (local || frame.locals.contains_key(text(*v, self.source)))
                });
                if simple {
                    for (i, var) in vars.into_iter().enumerate() {
                        let (value, binding) = results.get(i).cloned().unwrap_or({
                            if dynamic_tail {
                                (None, Name::Unknown)
                            } else {
                                (Some(Expr::Nil), Name::Other)
                            }
                        });
                        let name = text(var, self.source);
                        if ["_ENV", "_G"].contains(&name) {
                            self.barrier(node, scope, BarrierKind::EnvironmentMutation)?;
                            frame.taint();
                        }
                        self.bind_local(frame, name, value, binding, local)?;
                    }
                } else {
                    self.barrier(node, scope, BarrierKind::NonlocalAssignment)?;
                    frame.taint();
                }
            }
            "function_call" => {
                let Some(callee) = node.child_by_field_name("name") else {
                    return Ok(());
                };
                let raw = text(callee, self.source);
                let name = frame.name(callee, self.source);
                let candidate = match name {
                    Name::Loader(k, captured) => Some((
                        k,
                        if frame.uncertain {
                            Binding::Uncertain
                        } else if captured {
                            Binding::CapturedCandidate
                        } else {
                            Binding::EntryCandidate
                        },
                    )),
                    Name::Other => builtin(raw).map(|k| {
                        (
                            k,
                            if frame.uncertain {
                                Binding::Uncertain
                            } else {
                                Binding::Shadowed
                            },
                        )
                    }),
                    Name::Unknown => builtin(raw).map(|k| (k, Binding::Uncertain)),
                };
                let args = node
                    .child_by_field_name("arguments")
                    .map(named)
                    .unwrap_or_default();
                let module =
                    args.first()
                        .and_then(|n| match evaluate(*n, self.source, &frame.values, 0) {
                            Some(Expr::Text(s)) => Some(s),
                            _ => None,
                        });
                // Callee and argument expressions can themselves contain loader calls.
                self.visit(callee, scope, frame, depth + 1)?;
                for arg in &args {
                    self.visit(*arg, scope, frame, depth + 1)?;
                }
                if let Some((loader, binding)) = candidate {
                    if self.output.sites.len() >= MAX_SITES {
                        return Err("More than 512 module request sites".into());
                    }
                    self.output.sites.push(Site {
                        loader,
                        callee: raw.into(),
                        module,
                        scope,
                        binding,
                        span: node.byte_range(),
                        argument_span: args.first().map(Node::byte_range),
                        line: node.start_position().row + 1,
                    });
                } else {
                    self.barrier(node, scope, BarrierKind::UnknownCall)?;
                }
                frame.taint();
            }
            "if_statement" | "while_statement" | "repeat_statement" | "for_statement" => {
                self.barrier(node, scope, BarrierKind::ControlFlow)?;
                let mut nested = frame.clone();
                nested.taint();
                let nested_scope = if scope == SiteScope::FunctionBody {
                    scope
                } else {
                    SiteScope::Conditional
                };
                for child in named(node) {
                    self.visit(child, nested_scope, &mut nested, depth + 1)?;
                }
                frame.taint();
            }
            "return_statement" | "goto_statement" | "break_statement" => {
                for child in named(node) {
                    self.visit(child, scope, frame, depth + 1)?;
                }
                self.barrier(node, scope, BarrierKind::ControlFlow)?;
                frame.taint();
            }
            _ => {
                for child in named(node) {
                    self.visit(child, scope, frame, depth + 1)?;
                }
            }
        }
        Ok(())
    }
}
pub fn inspect(source: &str, version: &str) -> Result<Inventory, String> {
    if ![BASELINE, AUDITED_MAIN].contains(&version) {
        return Err("Dependency inspection needs an audited snapshot".into());
    }
    let tree = super::syntax_tree(source)?;
    let mut inspect = Inspect {
        source,
        output: Inventory {
            source_sha256: source_hash(source),
            version: version.into(),
            sites: vec![],
            barriers: vec![],
        },
        visited: 0,
        next_local: 0,
    };
    inspect.visit(
        tree.root_node(),
        SiteScope::Linear,
        &mut Frame::initial(),
        0,
    )?;
    inspect.output.sites.sort_by_key(|s| s.span.start);
    inspect.output.barriers.sort_by_key(|s| s.span.start);
    inspect.output.validate(source, version)?;
    Ok(inspect.output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literals_aliases_shadowing_and_source_spans_are_distinct_from_execution() {
        let source="local prefix='./';local path=prefix..'theme';local load=require;load(path);require('./later');do local require=3;require('./shadowed')end;require('./after')";
        let i = inspect(source, BASELINE).unwrap();
        assert_eq!(i.sites.len(), 4);
        assert_eq!(i.sites[0].module.as_deref(), Some("./theme"));
        assert_eq!(i.sites[0].binding, Binding::CapturedCandidate);
        assert_eq!(i.sites[1].binding, Binding::Uncertain);
        assert_eq!(i.sites[2].binding, Binding::Uncertain);
        for site in &i.sites {
            assert!(source[site.span.clone()].contains(&site.callee));
        }
        let i = inspect("do local require=3;require('./not-original')end", BASELINE).unwrap();
        assert_eq!(i.sites[0].binding, Binding::Shadowed);
    }
    #[test]
    fn function_bodies_control_flow_and_dynamic_arguments_stay_visible_and_uncertain() {
        let source="local p='old';local function callback(require)require(p)end;if flag then require('./maybe')end;local path=compute();dofile(path);loadfile('./later')";
        let i = inspect(source, BASELINE).unwrap();
        assert_eq!(i.sites.len(), 4);
        assert_eq!(i.sites[0].scope, SiteScope::FunctionBody);
        assert!(i.sites[0].module.is_none());
        assert_eq!(i.sites[1].scope, SiteScope::Conditional);
        assert_eq!(i.sites[1].module.as_deref(), Some("./maybe"));
        assert_eq!(i.sites[2].loader, Loader::Dofile);
        assert!(i.sites[2].module.is_none());
        assert_eq!(i.sites[3].loader, Loader::Loadfile);
        assert!(i
            .barriers
            .iter()
            .any(|b| b.kind == BarrierKind::UnknownCall));
    }
    #[test]
    fn stale_local_values_do_not_cross_opaque_initializers_or_function_creation() {
        let source="local p='old';local ignored,after=opaque(),p;require(after);local p='new';local f=function()require(p)end";
        let i = inspect(source, BASELINE).unwrap();
        assert_eq!(i.sites.len(), 2);
        assert!(i.sites.iter().all(|s| s.module.is_none()));
        let i = inspect(
            "local f=function() opaque() end;require('./literal')",
            BASELINE,
        )
        .unwrap();
        assert_eq!(i.sites[0].binding, Binding::EntryCandidate);
    }
    #[test]
    fn inventory_identity_bounds_and_syntax_are_checked_without_running_anything() {
        let source = "require('./theme')";
        let mut i = inspect(source, BASELINE).unwrap();
        i.sites[0].span.end += 1;
        assert!(i.validate(source, BASELINE).is_err());
        assert!(inspect("require('", BASELINE).is_err());
        assert!(inspect(source, "unknown").is_err());
        assert!(inspect(&"require('x');".repeat(MAX_SITES + 1), BASELINE).is_err());
    }
    #[test]
    fn scope_identity_preserves_outer_writes_and_inner_shadows() {
        for (source, expected) in [
            ("local p='old';do p='new' end;require(p)", "new"),
            (
                "local p='old';do local p='new';p='inner' end;require(p)",
                "old",
            ),
            (
                "local p='old';do p='new';local p='shadow' end;require(p)",
                "new",
            ),
            ("local p='old';do do p='new' end end;require(p)", "new"),
        ] {
            let i = inspect(source, BASELINE).unwrap();
            assert_eq!(i.sites[0].module.as_deref(), Some(expected), "{source}");
        }
        let i = inspect("local p='old';do p='new';opaque()end;require(p)", BASELINE).unwrap();
        assert!(i.sites[0].module.is_none());
        let i = inspect(
            "do function require()end end;require('./possible')",
            BASELINE,
        )
        .unwrap();
        assert_eq!(i.sites[0].binding, Binding::Uncertain);
    }
}
