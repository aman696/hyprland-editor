//! Bounded detail text for expression evidence; no mutation or ownership inference.
use super::source_graph::Analysis;
use hyprland_editor_core::{
    lua::replay::{DecisionOutcome, Event, KeySource, Observed, ValueSource, WriteKind},
    source::display_path_bytes,
};
use std::{collections::BTreeSet, fmt::Write};

struct Text<'a> {
    analysis: &'a Analysis,
    out: String,
    leaves: usize,
    nodes: usize,
    truncated: bool,
    seen: BTreeSet<(u8, u32)>,
}
impl Text<'_> {
    fn path(&mut self, prefix: usize) {
        if prefix == 0 {
            return;
        }
        self.out.push_str("\nExecuted branch and loop history\nEarlier decisions may be unrelated to this value. This is a conservative history, not a minimal dependency proof.\n");
        let Some(history) = self.analysis.report.path_decisions.get(..prefix) else {
            self.out.push_str("Path evidence is unavailable.\n");
            return;
        };
        let start = prefix.saturating_sub(12);
        if start > 0 {
            writeln!(self.out, "… {} earlier decisions omitted.", start).unwrap();
            self.truncated = true;
        }
        for decision in &history[start..] {
            if self.nodes >= 96 || self.out.len() > 12 * 1024 {
                self.truncated = true;
                break;
            }
            self.nodes += 1;
            let outcome = match decision.outcome {
                DecisionOutcome::Truthiness { truthy } => {
                    if truthy {
                        "truthy".into()
                    } else {
                        "false or nil".into()
                    }
                }
                DecisionOutcome::NumericRange {
                    current,
                    end,
                    step,
                    entered,
                } => format!(
                    "{current} toward {end}, step {step} · {}",
                    if entered { "entered" } else { "finished" }
                ),
                DecisionOutcome::IntegerRangeOverflow { .. } => {
                    "finished at the integer limit".into()
                }
            };
            let site = &decision.site;
            let Some(document) = self.analysis.program.documents.get(site.document) else {
                continue;
            };
            let name = display_path_bytes(&document.load_name)
                .chars()
                .take(180)
                .collect::<String>();
            writeln!(
                self.out,
                "  {:?} · {outcome} · {name} · line {}",
                decision.kind, site.line
            )
            .unwrap();
            for &input in &decision.inputs {
                self.node(input, 1);
            }
        }
    }
    fn node(&mut self, id: u32, depth: usize) {
        if depth > 8 || self.nodes >= 96 || self.out.len() > 12 * 1024 {
            self.truncated = true;
            return;
        }
        if !self.seen.insert((0, id)) {
            return;
        }
        let Some(node) = self.analysis.report.expression_sources.get(id as usize) else {
            self.out.push_str("  Expression evidence is unavailable.\n");
            return;
        };
        let Some(doc) = self.analysis.program.documents.get(node.document) else {
            self.out
                .push_str("  The expression document is unavailable.\n");
            return;
        };
        self.nodes += 1;
        let name = display_path_bytes(&doc.load_name)
            .chars()
            .take(180)
            .collect::<String>();
        let raw = doc.source.get(node.span.clone()).unwrap_or_default();
        let snippet = raw.chars().take(120).collect::<String>();
        let mut snippet = display_path_bytes(snippet.as_bytes());
        if raw.chars().take(121).count() > 120 {
            snippet.push('…');
            self.truncated = true;
        }
        writeln!(
            self.out,
            "{}{:?} · {name} · line {}\n{}{}",
            "  ".repeat(depth + 1),
            node.kind,
            node.line,
            "  ".repeat(depth + 2),
            snippet
        )
        .unwrap();
        if let Some(previous) = node.selected {
            self.node(previous, depth + 1);
        } else if !node.evaluated.is_empty() {
            self.out.push_str(
                "  Evaluated expressions (includes selectors, arguments and control effects):\n",
            );
            for &input in node.evaluated.iter().take(4) {
                self.node(input, depth + 1);
            }
            if node.evaluated.len() > 4 {
                self.truncated = true;
            }
        }
        if let Some(write) = node.storage_write {
            self.write(write, depth + 1);
        }
    }
    fn write(&mut self, id: u32, depth: usize) {
        if depth > 8 || self.nodes >= 96 || self.out.len() > 12 * 1024 {
            self.truncated = true;
            return;
        }
        if !self.seen.insert((1, id)) {
            return;
        }
        let Some(write) = self.analysis.report.storage_writes.get(id as usize) else {
            self.out.push_str("  Assignment evidence is unavailable.\n");
            return;
        };
        self.nodes += 1;
        let kind = match write.kind {
            WriteKind::LocalDeclaration => "Local declaration",
            WriteKind::Assignment => "Assignment",
            WriteKind::FunctionDeclaration => "Function binding",
            WriteKind::TableConstructor => "Table field construction",
            WriteKind::Parameter => "Parameter binding at call",
            WriteKind::NumericFor => "Loop binding",
            WriteKind::Rawset => "Raw table assignment",
            WriteKind::ModeledIntrinsic => "Modeled loader/library assignment",
        };
        writeln!(
            self.out,
            "{}{}{}",
            "  ".repeat(depth + 1),
            kind,
            if write.deletes_slot {
                " · removes the slot"
            } else {
                ""
            }
        )
        .unwrap();
        if write.path_prefix > 0 {
            writeln!(
                self.out,
                "  {} earlier branch/loop decisions recorded at this write.",
                write.path_prefix
            )
            .unwrap();
        }
        if let Some(site) = &write.site {
            if let Some(document) = self.analysis.program.documents.get(site.document) {
                let name = display_path_bytes(&document.load_name)
                    .chars()
                    .take(180)
                    .collect::<String>();
                let raw = document.source.get(site.span.clone()).unwrap_or_default();
                let snippet = raw.chars().take(120).collect::<String>();
                writeln!(
                    self.out,
                    "{}{} · line {}\n{}{}",
                    "  ".repeat(depth + 2),
                    name,
                    site.line,
                    "  ".repeat(depth + 2),
                    display_path_bytes(snippet.as_bytes())
                )
                .unwrap();
            }
        } else {
            self.out
                .push_str("  No editable source location is claimed for this intrinsic write.\n");
        }
        if let Some(expression) = write.value_expression {
            self.node(expression, depth + 1);
        }
        if let Some(key) = write.key_expression {
            self.out.push_str("  Address key expression:\n");
            self.node(key, depth + 1);
        }
        if let Some(receiver) = write.receiver_expression {
            self.out.push_str("  Assigned through:\n");
            self.node(receiver, depth + 1);
        } else if let Some(receiver) = write.receiver_write {
            self.write(receiver, depth + 1);
        }
        if let Some(value) = write.value_write {
            self.out.push_str("  Value forwarded from:\n");
            self.write(value, depth + 1);
        }
        if let Some(previous) = write.previous {
            self.out.push_str("  Earlier write to the same storage:\n");
            self.write(previous, depth + 1);
        }
    }
    fn value(
        &mut self,
        value: &Observed,
        source: &ValueSource,
        path: &str,
        depth: usize,
        key: Option<&KeySource>,
    ) {
        if self.leaves >= 32 || depth > 8 || self.out.len() > 12 * 1024 {
            self.truncated = true;
            return;
        }
        if let Observed::Table(rows) = value {
            if !rows.is_empty() {
                if let Some(write) = source.storage_write {
                    writeln!(self.out, "\n{path} · Table binding").unwrap();
                    self.write(write, depth);
                }
                let source_keys = &source.keys;
                for (i, ((key, value), source)) in rows.iter().zip(&source.rows).enumerate() {
                    let key = match key {
                        Observed::Bytes(bytes)
                            if bytes
                                .first()
                                .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
                                && bytes
                                    .iter()
                                    .all(|b| b.is_ascii_alphanumeric() || *b == b'_') =>
                        {
                            format!(
                                ".{}",
                                std::str::from_utf8(&bytes[..bytes.len().min(80)]).unwrap()
                            )
                        }
                        Observed::Bytes(bytes) => {
                            let mut key = String::from("[bytes:");
                            for byte in bytes.iter().take(80) {
                                write!(key, "{byte:02x}").unwrap();
                            }
                            if bytes.len() > 80 {
                                key.push('…');
                            }
                            key.push(']');
                            key
                        }
                        Observed::Int(n) => format!("[{n}]"),
                        _ => format!("[row {}]", i + 1),
                    };
                    self.value(
                        value,
                        source,
                        &format!("{path}{key}"),
                        depth + 1,
                        source_keys.get(i),
                    );
                    if self.truncated {
                        break;
                    }
                }
                if rows.len() != source.rows.len() || rows.len() != source.keys.len() {
                    self.truncated = true;
                }
                return;
            }
        }
        self.leaves += 1;
        writeln!(self.out, "\n{path}").unwrap();
        if let Some(key) = key {
            if key.introduced_by != key.latest_write {
                self.out
                    .push_str("  The present key slot was introduced by an earlier write.\n");
            }
        }
        if let Some(id) = source.expression {
            self.node(id, 0);
        } else {
            self.out.push_str(
                "  No source expression is recorded for this intrinsic or generated value.\n",
            );
        }
        if let Some(write) = source.storage_write {
            self.write(write, 0);
        }
        if let Some(introduction) = key.and_then(|k| k.introduced_by) {
            self.write(introduction, 0);
        }
    }
}

pub(super) fn describe(analysis: &Analysis, event: &Event) -> String {
    let mut text=Text{analysis,out:"Value, key, assignment and executed-path evidence only. Complete control dependencies, future effects, literal ownership and editing rights are not verified.\n".into(),leaves:0,nodes:0,truncated:false,seen:BTreeSet::new()};
    for (i, (value, source)) in event
        .arguments
        .iter()
        .zip(&event.argument_sources)
        .enumerate()
    {
        text.value(value, source, &format!("Argument {}", i + 1), 0, None);
        if text.truncated {
            break;
        }
    }
    if event.arguments.len() != event.argument_sources.len() {
        text.truncated = true;
    }
    text.path(event.path_prefix);
    if text.truncated {
        text.out
            .push_str("\n… Showing a bounded excerpt of expression evidence.\n");
    }
    if text.out.len() > 16 * 1024 {
        let mut end = 16 * 1024;
        while !text.out.is_char_boundary(end) {
            end -= 1;
        }
        text.out.truncate(end);
        text.out
            .push_str("\n… Expression evidence excerpt truncated.\n");
    }
    text.out
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyprland_editor_core::{
        lua::{
            replay::{self, Document, Profile, Program},
            source_hash,
        },
        module_path::Context,
        BASELINE,
    };
    fn analysis(source: &str) -> Analysis {
        let name = b"/fixture/main.lua".to_vec();
        let program = Program {
            version: BASELINE.into(),
            profile: Profile::SyntheticEmit,
            root: "root".into(),
            documents: vec![Document {
                id: "root".into(),
                source: source.into(),
                sha256: source_hash(source),
                load_name: name.clone(),
            }],
            bindings: vec![],
            initial_package_path: None,
            initial_package_path_unknown_tail: false,
            working_directory: None,
        };
        let report = replay::replay(&program).unwrap();
        assert!(report.halt.is_none(), "{:?}", report.halt);
        Analysis {
            restored: false,
            context: Context {
                main_file: name,
                home: None,
                inherited_package_path: None,
            },
            program,
            report,
            config_batches: vec![],
            config_profile: None,
            captures: vec![],
            edges: vec![],
            blocked: None,
            retained_bytes: 0,
        }
    }
    #[test]
    fn path_details_show_only_the_observation_prefix_and_bound_earlier_history() {
        let a = analysis(
            "if false then emit('never')elseif 0 then emit(2)end;while false do end;emit(3)",
        );
        let first = describe(&a, &a.report.events[0]);
        assert!(first.contains("Executed branch and loop history"));
        assert!(first.contains("If · false or nil"));
        assert!(first.contains("ElseIf · truthy"));
        assert!(!first.contains("While ·"));
        let later = describe(&a, &a.report.events[1]);
        assert!(later.contains("While · false or nil"));
        assert!(later.contains("not a minimal dependency proof"));
        let a = analysis("for i=1,100 do end;emit(1)");
        let text = describe(&a, &a.report.events[0]);
        assert!(text.contains("89 earlier decisions omitted"));
        assert!(text.len() <= 16 * 1024 + 100);
    }
    #[test]
    fn assignment_details_preserve_current_value_key_and_prior_write_roles() {
        let a =
            analysis("local old='border_size';local now=old;local t={[old]=2};t[now]=4;emit(t)");
        let text = describe(&a, &a.report.events[0]);
        for item in [
            "Argument 1.border_size",
            "earlier write",
            "Address key expression",
            "Table field construction",
            "Assignment",
            "Literal",
            "old",
            "now",
        ] {
            assert!(text.contains(item), "Missing {item}: {text}");
        }
        assert!(text.contains("editing rights are not verified"));
        assert!(text.len() <= 16 * 1024 + 100);
    }
    #[test]
    fn evidence_paths_distinguish_dotted_keys_from_nested_values_and_bound_shared_expansion() {
        let a = analysis("emit({['a.b']=1,a={b=2},[3]='third'})");
        let text = describe(&a, &a.report.events[0]);
        for path in [
            "Argument 1[bytes:612e62]",
            "Argument 1.a.b",
            "Argument 1[3]",
        ] {
            assert!(text.contains(path), "{text}");
        }
        assert!(text.contains("editing rights are not verified"));
        let a = analysis("local t={};for i=1,200 do t[i]={a=1,b=2,c=3}end;emit(t)");
        let text = describe(&a, &a.report.events[0]);
        assert!(text.contains('…'));
        assert!(text.len() <= 16 * 1024 + 100);
    }
}
