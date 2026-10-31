//! Retained legacy source and lossless edits to its supported scalar projection.
//! No includes, variable expansion or commands are evaluated here.
use crate::{
    import, legacy_monitor, legacy_session, session, Draft, Setting, Value, BASELINE,
    MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ops::Range};

#[derive(Debug)]
pub struct Document {
    pub content: String,
    pub report: import::ImportReport,
    pub session: legacy_session::Report,
    pub monitors: legacy_monitor::Report,
    pub bindings: crate::legacy_binding::Report,
    pub workspaces: crate::legacy_workspace::Report,
    pub animations: crate::legacy_animation::Report,
}

pub struct Edits<'a> {
    pub draft: &'a Draft,
    pub session: &'a session::Draft,
    pub monitors: &'a [crate::monitor::Rule],
    pub bindings: &'a crate::legacy_binding::Draft,
    pub workspaces: &'a [crate::workspace_rule::Rule],
    pub animations: &'a crate::animation::Draft,
}

struct Assignment {
    key: String,
    value: Range<usize>,
    statement: Range<usize>,
    ambiguous: bool,
}

// Match the conservative importer's comment boundary, preserving original bytes.
fn code(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if ch == '"' && !escaped {
            quoted = !quoted;
        }
        if ch == '#' && !quoted {
            return &line[..index];
        }
        escaped = ch == '\\' && !escaped;
    }
    line
}

fn assignments(content: &str, keys: &BTreeSet<&String>) -> Vec<Assignment> {
    let mut blocks = Vec::<String>::new();
    let mut result = vec![];
    let mut offset = 0;
    let mut continued = false;
    for line in content.split_inclusive('\n') {
        let previous_continues = continued;
        continued = line.trim_end_matches(['\r', '\n']).ends_with('\\');
        let code = code(line);
        let trimmed = code.trim();
        let start = offset + code.len() - code.trim_start().len();
        if trimmed == "}" {
            blocks.pop();
        } else if let Some(name) = trimmed.strip_suffix('{').filter(|_| !trimmed.contains('=')) {
            blocks.push(name.trim().into());
        } else if let Some((key, raw)) = trimmed.split_once('=') {
            let key = key.trim();
            let full = if blocks.is_empty() {
                key.to_owned()
            } else {
                format!("{}:{key}", blocks.join(":"))
            };
            if !keys.contains(&full) {
                offset += line.len();
                continue;
            }
            let raw_start = start + trimmed.len() - raw.len();
            let value_start = raw_start + raw.len() - raw.trim_start().len();
            result.push(Assignment {
                key: full,
                value: value_start..value_start + raw.trim().len(),
                statement: start..start + trimmed.len(),
                // The existing projection does not model escaped hashes or line
                // continuations. Keep these bytes, but never rewrite their line.
                ambiguous: previous_continues || line.contains("##") || line.contains('\\'),
            });
        }
        offset += line.len();
    }
    result
}

fn same(before: Option<&Value>, after: Option<&Value>) -> bool {
    match (before, after) {
        (Some(Value::Float(a)), Some(Value::Float(b))) => a.to_bits() == b.to_bits(),
        (Some(Value::Vec2(a)), Some(Value::Vec2(b))) => {
            a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
        }
        (
            Some(Value::Gradient {
                colors: a,
                angle: aa,
            }),
            Some(Value::Gradient {
                colors: b,
                angle: ba,
            }),
        ) => a == b && aa.to_bits() == ba.to_bits(),
        _ => before == after,
    }
}

fn encode(value: &Value) -> Result<String, String> {
    Ok(match value {
        Value::Bool(value) => if *value { "true" } else { "false" }.into(),
        Value::Int(value) | Value::FontWeight(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Color(value) => format!("rgba({:08x})", value.rotate_left(8)),
        Value::Gradient { colors, angle } => {
            let mut parts = colors
                .iter()
                .map(|value| format!("rgba({:08x})", value.rotate_left(8)))
                .collect::<Vec<_>>();
            parts.push(format!("{angle}deg"));
            parts.join(" ")
        }
        Value::Vec2(values) => format!("{} {}", values[0], values[1]),
        Value::CssGap(values) => values
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(" "),
        Value::String(value) => {
            if value.trim() != value
                || value
                    .chars()
                    .any(|ch| ch.is_control() || "$#\\\"{}".contains(ch))
            {
                return Err("This text needs legacy escaping or expansion that the editor cannot preserve yet. Save the project to keep the draft; no source copy was produced.".into());
            }
            value.clone()
        }
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub source_sha256: String,
    pub scalar: import::ImportReport,
    pub session: legacy_session::Report,
    pub monitors: legacy_monitor::Report,
    pub bindings: crate::legacy_binding::Report,
    pub workspaces: crate::legacy_workspace::Report,
    pub animations: crate::legacy_animation::Report,
}
impl Document {
    pub fn into_report(self) -> Report {
        Report {
            source_sha256: crate::lua::source_hash(&self.content),
            scalar: self.report,
            session: self.session,
            monitors: self.monitors,
            bindings: self.bindings,
            workspaces: self.workspaces,
            animations: self.animations,
        }
    }
    pub fn edited_with_session(
        &self,
        draft: &Draft,
        session: &session::Draft,
        catalogue: &[Setting],
    ) -> Result<String, String> {
        self.edited_with_monitors(draft, session, &self.monitors.rules, catalogue)
    }
    pub fn edited_with_monitors(
        &self,
        draft: &Draft,
        session: &session::Draft,
        monitors: &[crate::monitor::Rule],
        catalogue: &[Setting],
    ) -> Result<String, String> {
        self.edited_with_bindings(draft, session, monitors, &self.bindings.draft, catalogue)
    }
    pub fn edited_with_bindings(
        &self,
        draft: &Draft,
        session: &session::Draft,
        monitors: &[crate::monitor::Rule],
        bindings: &crate::legacy_binding::Draft,
        catalogue: &[Setting],
    ) -> Result<String, String> {
        self.edited_with_workspaces(
            draft,
            session,
            monitors,
            bindings,
            &self.workspaces.rules,
            catalogue,
        )
    }
    pub fn edited_with_workspaces(
        &self,
        draft: &Draft,
        session: &session::Draft,
        monitors: &[crate::monitor::Rule],
        bindings: &crate::legacy_binding::Draft,
        workspaces: &[crate::workspace_rule::Rule],
        catalogue: &[Setting],
    ) -> Result<String, String> {
        self.edited_with_animations(
            Edits {
                draft,
                session,
                monitors,
                bindings,
                workspaces,
                animations: &self.animations.draft,
            },
            catalogue,
        )
    }
    pub fn edited_with_animations(
        &self,
        edits: Edits<'_>,
        catalogue: &[Setting],
    ) -> Result<String, String> {
        let Edits {
            draft,
            session,
            monitors,
            bindings,
            workspaces,
            animations,
        } = edits;
        let scalars = self.edited_copy(draft, catalogue)?;
        let sessions = legacy_session::edited_copy(&scalars, session)?;
        let output = legacy_monitor::edited_copy(&sessions, monitors)?;
        let output = crate::legacy_binding::edited_copy(&output, bindings)?;
        let output = crate::legacy_workspace::edited_copy(&output, workspaces)?;
        let output = crate::legacy_animation::edited_copy(&output, animations)?;
        // env with unspecified versus explicitly false activation both encode
        // as the same native directive. Compare the native projection, while
        // private projects retain that presence distinction.
        let session_projection = |draft: &session::Draft| {
            draft
                .entries
                .iter()
                .map(legacy_session::encode)
                .collect::<Result<Vec<_>, String>>()
        };
        if session_projection(&legacy_session::inspect(&output)?.draft)?
            != session_projection(session)?
            || legacy_monitor::export(&legacy_monitor::inspect(&output)?.rules)?
                != legacy_monitor::export(monitors)?
            || crate::legacy_workspace::export(&crate::legacy_workspace::inspect(&output)?.rules)?
                != crate::legacy_workspace::export(workspaces)?
            || crate::legacy_animation::export(&crate::legacy_animation::inspect(&output)?.draft)?
                != crate::legacy_animation::export(animations)?
            || crate::legacy_binding::export(
                &crate::legacy_binding::inspect_mode(&output, bindings.local_variables)?.draft,
            )? != crate::legacy_binding::export(bindings)?
        {
            return Err(
                "Binding edits changed another source projection; no copy was produced".into(),
            );
        }
        let final_report = import::legacy(&output, catalogue)?;
        if final_report.draft.values.len() != draft.values.len()
            || draft
                .values
                .iter()
                .any(|(key, value)| !same(final_report.draft.values.get(key), Some(value)))
        {
            return Err("Session edits changed the scalar projection; no copy was produced".into());
        }
        Ok(output)
    }

    pub fn new(content: String, catalogue: &[Setting]) -> Result<Self, String> {
        Self::new_with_binding_mode(content, catalogue, true)
    }
    pub fn new_with_binding_mode(
        content: String,
        catalogue: &[Setting],
        local_variables: bool,
    ) -> Result<Self, String> {
        let report = import::legacy(&content, catalogue)?;
        let session = legacy_session::inspect(&content)?;
        let monitors = legacy_monitor::inspect(&content)?;
        let bindings = crate::legacy_binding::inspect_mode(&content, local_variables)?;
        let workspaces = crate::legacy_workspace::inspect(&content)?;
        let animations = crate::legacy_animation::inspect(&content)?;
        Ok(Self {
            content,
            report,
            session,
            monitors,
            bindings,
            workspaces,
            animations,
        })
    }

    pub fn edited_copy(&self, draft: &Draft, catalogue: &[Setting]) -> Result<String, String> {
        draft.validate(catalogue)?;
        if draft.version != BASELINE {
            return Err("Legacy source copies require the baseline settings pack.".into());
        }
        let keys: BTreeSet<_> = self
            .report
            .draft
            .values
            .keys()
            .chain(draft.values.keys())
            .filter(|key| !same(self.report.draft.values.get(*key), draft.values.get(*key)))
            .collect();
        let declarations = assignments(&self.content, &keys);
        let mut edits = vec![];
        let mut appended = String::new();
        let newline = if self.content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        for key in keys {
            let before = self.report.draft.values.get(key);
            let after = draft.values.get(key);
            if same(before, after) {
                continue;
            }
            let occurrences = declarations
                .iter()
                .filter(|item| item.key == *key)
                .collect::<Vec<_>>();
            if occurrences.iter().any(|item| item.ambiguous) {
                return Err(format!("{key} uses an escaped hash or line continuation. Its source cannot be safely rewritten yet; save the project to retain your draft."));
            }
            if let Some(value) = after {
                let encoded = encode(value).map_err(|error| format!("{key}: {error}"))?;
                if let Some(last) = occurrences.last() {
                    edits.push((last.value.clone(), encoded));
                } else {
                    appended.push_str(&format!("{key} = {encoded}{newline}"));
                }
            } else {
                // Removing only the last duplicate would reveal an older value.
                // Remove all declarations of this key, retaining inline comments.
                for item in occurrences {
                    edits.push((item.statement.clone(), String::new()));
                }
            }
        }
        edits.sort_by_key(|(range, _)| range.start);
        let mut output = self.content.clone();
        for (range, replacement) in edits.into_iter().rev() {
            output.replace_range(range, &replacement);
        }
        if !appended.is_empty() {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push_str(newline);
            }
            output.push_str(&appended);
        }
        if output.len() > MAX_DOCUMENT_BYTES {
            return Err("The edited legacy source exceeds 1 MiB.".into());
        }
        let reparsed = import::legacy(&output, catalogue)?;
        if reparsed.draft.values.len() != draft.values.len()
            || draft
                .values
                .iter()
                .any(|(key, value)| !same(reparsed.draft.values.get(key), Some(value)))
        {
            return Err("The edited legacy source does not match the reviewed scalar draft. No copy was produced.".into());
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_nested_duplicates_inserts_and_removes_without_losing_other_bytes() {
        let catalogue = crate::catalogue();
        let source = "# café\r\nsource = other.conf\r\nexec = never-run\r\ngeneral {\r\n  border_size = 2 # earlier\r\n  border_size = 3  # kept\r\n  gaps_out = 4 # remove\r\n}\r\n";
        let doc = Document::new(source.into(), &catalogue).unwrap();
        assert_eq!(
            doc.edited_copy(&doc.report.draft, &catalogue).unwrap(),
            source
        );
        let mut draft = doc.report.draft.clone();
        draft
            .values
            .insert("general:border_size".into(), Value::Int(5));
        draft
            .values
            .insert("general:gaps_in".into(), Value::CssGap([7; 4]));
        draft.values.remove("general:gaps_out");
        let output = doc.edited_copy(&draft, &catalogue).unwrap();
        assert_eq!(
            output,
            source
                .replace("border_size = 3", "border_size = 5")
                .replace("gaps_out = 4", "")
                + "general:gaps_in = 7 7 7 7\r\n"
        );
        draft.values.remove("general:border_size");
        let output = doc.edited_copy(&draft, &catalogue).unwrap();
        assert!(!output.contains("border_size"));
        assert!(output.contains("# earlier\r\n"));
        assert!(output.contains("# kept\r\n"));
    }

    #[test]
    fn every_scalar_kind_roundtrips_and_dynamic_target_is_explicitly_replaced() {
        let catalogue = crate::catalogue();
        let doc = Document::new("general:border_size = $size".into(), &catalogue).unwrap();
        let mut draft = doc.report.draft.clone();
        draft
            .values
            .insert("general:border_size".into(), Value::Int(4));
        assert_eq!(
            doc.edited_copy(&draft, &catalogue).unwrap(),
            "general:border_size = 4"
        );
        let values = [
            Value::Bool(true),
            Value::Int(3),
            Value::Float(0.75),
            Value::String("Noto Sans".into()),
            Value::Color(0x7f123456),
            Value::Gradient {
                colors: vec![0xff123456, 0x7fabcdef],
                angle: 45.,
            },
            Value::Vec2([1.25, 2.5]),
            Value::CssGap([1, 2, 3, 4]),
            Value::FontWeight(500),
        ];
        for value in values {
            let setting = catalogue
                .iter()
                .find(|setting| setting.version == BASELINE && setting.validate(&value).is_ok())
                .unwrap();
            let mut draft = Draft::new(BASELINE);
            draft.values.insert(setting.id.clone(), value);
            Document::new(String::new(), &catalogue)
                .unwrap()
                .edited_copy(&draft, &catalogue)
                .unwrap();
        }
        let setting = catalogue
            .iter()
            .find(|setting| {
                setting.version == BASELINE && setting.validate(&Value::Float(-0.0)).is_ok()
            })
            .unwrap();
        let doc = Document::new(format!("{} = -0.0\n", setting.id), &catalogue).unwrap();
        let mut draft = doc.report.draft.clone();
        draft.values.insert(setting.id.clone(), Value::Float(0.0));
        let output = doc.edited_copy(&draft, &catalogue).unwrap();
        let parsed = import::legacy(&output, &catalogue).unwrap();
        let Value::Float(number) = parsed.draft.values[&setting.id] else {
            panic!("Float expected")
        };
        assert_eq!(number.to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn refuses_unmodeled_escaping_without_altering_unchanged_lines() {
        let catalogue = crate::catalogue();
        for source in [
            "general:border_size = 3 ## escaped",
            "general:border_size = 3 # \\\n",
            "exec = never \\\ngeneral:border_size = 3\n",
        ] {
            let doc = Document::new(source.into(), &catalogue).unwrap();
            assert_eq!(
                doc.edited_copy(&doc.report.draft, &catalogue).unwrap(),
                source
            );
            let mut draft = doc.report.draft.clone();
            draft
                .values
                .insert("general:border_size".into(), Value::Int(4));
            assert!(doc.edited_copy(&draft, &catalogue).is_err());
        }
        for value in ["a\nb", "$variable", "hash#value", "a\\b", " leading"] {
            assert!(encode(&Value::String(value.into())).is_err());
        }
    }
}
