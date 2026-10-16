//! Bounded detail text for expression evidence; no mutation or ownership inference.
use super::source_graph::Analysis;
use hyprland_editor_core::{
    lua::replay::{Event, Observed, ValueSource},
    source::display_path_bytes,
};
use std::fmt::Write;

struct Text<'a> {
    analysis: &'a Analysis,
    out: String,
    leaves: usize,
    nodes: usize,
    truncated: bool,
}
impl Text<'_> {
    fn node(&mut self, id: u32, depth: usize) {
        if depth > 8 || self.nodes >= 96 || self.out.len() > 12 * 1024 {
            self.truncated = true;
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
    }
    fn value(&mut self, value: &Observed, source: &ValueSource, path: &str, depth: usize) {
        if self.leaves >= 32 || depth > 8 || self.out.len() > 12 * 1024 {
            self.truncated = true;
            return;
        }
        if let Observed::Table(rows) = value {
            if !rows.is_empty() {
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
                    self.value(value, source, &format!("{path}{key}"), depth + 1);
                    if self.truncated {
                        break;
                    }
                }
                if rows.len() != source.rows.len() {
                    self.truncated = true;
                }
                return;
            }
        }
        self.leaves += 1;
        writeln!(self.out, "\n{path}").unwrap();
        if let Some(id) = source.expression {
            self.node(id, 0);
        } else {
            self.out.push_str(
                "  No source expression is recorded for this intrinsic or generated value.\n",
            );
        }
    }
}

pub(super) fn describe(analysis: &Analysis, event: &Event) -> String {
    let mut text=Text{analysis,out:"Expression evidence only. Complete dependencies, literal ownership and editing rights are not verified. Table-key origins are not recorded.\n".into(),leaves:0,nodes:0,truncated:false};
    for (i, (value, source)) in event
        .arguments
        .iter()
        .zip(&event.argument_sources)
        .enumerate()
    {
        text.value(value, source, &format!("Argument {}", i + 1), 0);
        if text.truncated {
            break;
        }
    }
    if event.arguments.len() != event.argument_sources.len() {
        text.truncated = true;
    }
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
            context: Context {
                main_file: name,
                home: None,
                inherited_package_path: None,
            },
            program,
            report,
            config_batches: vec![],
            captures: vec![],
            edges: vec![],
            blocked: None,
            retained_bytes: 0,
        }
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
