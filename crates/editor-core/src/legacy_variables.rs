//! Editable source definitions; raw templates remain distinct from effective values.
use crate::{
    legacy_session::Site,
    legacy_syntax::{self, code, trim},
    lua::source_hash,
    MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ops::Range};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub name: String,
    pub value: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_sha256: String,
    pub source_sha256: String,
    pub definitions: Vec<Definition>,
    pub sites: Vec<Site>,
    pub unresolved: Vec<usize>,
    pub unresolved_count: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub format: String,
    pub base_sha256: String,
    pub schema_sha256: String,
    pub content: String,
}
impl Revision {
    pub fn new(base: &str, content: String) -> Result<Self, String> {
        let r = Self {
            format: "hye-private-legacy-source-1".into(),
            base_sha256: source_hash(base),
            schema_sha256: schema_hash(),
            content,
        };
        r.validate(base)?;
        Ok(r)
    }
    pub fn validate(&self, base: &str) -> Result<(), String> {
        if self.format != "hye-private-legacy-source-1"
            || self.base_sha256 != source_hash(base)
            || self.schema_sha256 != schema_hash()
            || self.content.len() > MAX_DOCUMENT_BYTES
            || self.content.contains('\0')
        {
            return Err(
                "Private native revision does not match its captured base, codec or bounds".into(),
            );
        }
        Ok(())
    }
}
pub(crate) fn identifier(s: &str) -> bool {
    !s.is_empty() && s.len() <= 256 && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}
pub(crate) fn value(raw: &str) -> Result<String, String> {
    if raw.len() > 65536
        || raw != trim(raw)
        || raw.chars().any(char::is_control)
        || raw.ends_with('\\')
    {
        return Err("Variable template must be a single trimmed line within 64 KiB and cannot end in a continuation".into());
    }
    // Encode hashes for the actual native ## scan, while preserving template
    // backslashes, references and expression text exactly rather than evaluating them.
    let mut out = String::new();
    let mut hashes = 0usize;
    for ch in raw.chars().chain(std::iter::once('\0')) {
        if ch == '#' {
            hashes += 1;
            continue;
        }
        if hashes > 0 {
            out.push_str(&"#".repeat(hashes + hashes.div_ceil(2)));
            hashes = 0;
        }
        if ch != '\0' {
            out.push(ch);
        }
    }
    Ok(out)
}
pub fn validate(definitions: &[Definition]) -> Result<(), String> {
    if definitions.len() > 1024 {
        return Err("Variable draft exceeds 1024 definitions".into());
    }
    let mut ids = BTreeSet::new();
    for d in definitions {
        if !identifier(&d.name)
            || d.id.is_empty()
            || d.id.len() > 64
            || !d
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || !ids.insert(&d.id)
        {
            return Err("Invalid variable name or declaration identity".into());
        }
        value(&d.value)?;
    }
    if serde_json::to_vec(definitions)
        .map_err(|e| e.to_string())?
        .len()
        > MAX_DOCUMENT_BYTES
    {
        return Err("Variable draft exceeds 1 MiB".into());
    }
    Ok(())
}
pub fn inspect(content: &str) -> Result<Report, String> {
    let mut r = Report {
        schema_sha256: schema_hash(),
        source_sha256: source_hash(content),
        definitions: vec![],
        sites: vec![],
        unresolved: vec![],
        unresolved_count: 0,
    };
    let mut retained = 2usize;
    legacy_syntax::walk(content, |s| {
        let Some(name) = s.key.strip_prefix('$') else {
            return;
        };
        let d = Definition {
            id: format!("variable-{}", r.definitions.len()),
            name: name.into(),
            value: s.value.into(),
        };
        let n = serde_json::to_vec(&d).map_or(MAX_DOCUMENT_BYTES + 1, |s| s.len() + 1);
        if s.editable
            && r.definitions.len() < 1024
            && identifier(name)
            && value(s.value).is_ok()
            && retained.saturating_add(n) <= MAX_DOCUMENT_BYTES
        {
            retained += n;
            r.definitions.push(d);
            r.sites.push(Site {
                line: s.line,
                statement: s.statement,
            });
        } else {
            r.unresolved_count += 1;
            if r.unresolved.len() < 4096 {
                r.unresolved.push(s.line);
            }
        }
    })?;
    validate(&r.definitions)?;
    Ok(r)
}
fn same(a: &[Definition], b: &[Definition]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.name == b.name && a.value == b.value)
}
fn assignment(d: &Definition) -> Result<String, String> {
    Ok(format!("${} = {}", d.name, value(&d.value)?))
}
pub fn edited_copy(content: &str, target: &[Definition]) -> Result<String, String> {
    validate(target)?;
    let before = inspect(content)?;
    if same(&before.definitions, target) {
        return Ok(content.into());
    }
    let mut edits: Vec<(Range<usize>, String)> = vec![];
    for (i, site) in before.sites.iter().enumerate() {
        let raw = &content[site.statement.clone()];
        if let Some(next) = target.get(i) {
            if before.definitions[i].name == next.name {
                let eq = raw
                    .find('=')
                    .ok_or("Captured definition has no assignment")?;
                let rhs = &raw[eq + 1..];
                let start = site.statement.start + eq + 1 + rhs.len()
                    - rhs
                        .trim_start_matches(|c: char| c.is_ascii_whitespace())
                        .len();
                edits.push((start..site.statement.end, value(&next.value)?));
            } else {
                edits.push((site.statement.clone(), assignment(next)?));
            }
        } else {
            edits.push((site.statement.clone(), String::new()));
        }
    }
    let nl = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    if target.len() > before.sites.len() {
        let at = before.sites.last().map_or(0, |s| {
            content[s.statement.end..]
                .find('\n')
                .map_or(content.len(), |n| s.statement.end + n + 1)
        });
        let mut addition = String::new();
        if at > 0 && !content[..at].ends_with('\n') {
            addition.push_str(nl);
        }
        for d in target.iter().skip(before.sites.len()) {
            addition.push_str(&assignment(d)?);
            addition.push_str(nl);
        }
        edits.push((at..at, addition));
    }
    edits.sort_by_key(|(span, _)| span.start);
    let mut out = content.to_owned();
    for (s, text) in edits.into_iter().rev() {
        out.replace_range(s, &text);
    }
    if out.len() > MAX_DOCUMENT_BYTES {
        return Err("Variable source copy exceeds 1 MiB".into());
    }
    let after = inspect(&out)?;
    if !same(&after.definitions, target) || after.unresolved_count != before.unresolved_count {
        return Err("Variable source copy did not reproduce the requested definition order".into());
    }
    Ok(out)
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-variables.json"))
}
/// Count source reference occurrences only; this does not infer runtime uses or values.
pub fn references(content: &str, name: &str) -> Result<Vec<usize>, String> {
    if !identifier(name) || content.len() > MAX_DOCUMENT_BYTES {
        return Err("Invalid variable reference query".into());
    }
    let mut lines = vec![];
    for (i, line) in content.lines().enumerate() {
        let (text, _) = code(line);
        let mut at = 0;
        while let Some(found) = text[at..].find('$') {
            at += found + 1;
            let begin = at;
            while let Some(c) = text[at..].chars().next() {
                if !(c.is_alphanumeric() || c == '_') {
                    break;
                }
                at += c.len_utf8();
            }
            if &text[begin..at] == name
                && !(trim(&text).starts_with(&format!("${name}")) && text[..begin].trim() == "$")
                && lines.len() < 4096
            {
                lines.push(i + 1);
            }
        }
    }
    Ok(lines)
}
/// Materialize staged native edits before changing shared definitions. Reparse
/// afterwards so source-derived shortcuts follow new values while explicitly
/// edited literal fields keep their chosen values. No config is executed.
pub fn prepare(
    workspace: &crate::workspace::Workspace,
    target: &[Definition],
    catalogue: &[crate::Setting],
) -> Result<String, String> {
    let content = workspace
        .legacy_content()
        .ok_or("Import a native configuration before editing variables")?;
    let copy = materialize(workspace, catalogue)?;
    if !same(&inspect(content)?.definitions, &inspect(&copy)?.definitions) {
        return Err("Staged edits changed variable declarations; review the source again".into());
    }
    let output = edited_copy(&copy, target)?;
    crate::legacy::Document::new_with_binding_mode(
        output.clone(),
        catalogue,
        workspace
            .bindings
            .legacy
            .as_ref()
            .is_some_and(|draft| draft.local_variables),
    )?;
    Ok(output)
}

pub(crate) fn materialize(
    workspace: &crate::workspace::Workspace,
    catalogue: &[crate::Setting],
) -> Result<String, String> {
    workspace.validate(catalogue)?;
    let content = workspace
        .legacy_content()
        .ok_or("Import a native configuration before editing source directives")?;
    let bindings = workspace
        .bindings
        .legacy
        .as_ref()
        .ok_or("Native binding source view missing")?;
    if !workspace.bindings.binds.is_empty()
        || !workspace.bindings.submaps.is_empty()
        || !workspace.bindings.order.is_empty()
    {
        return Err(
            "Separate Lua drafts require explicit review before native source edits".into(),
        );
    }
    let original = crate::legacy::Document::new_with_binding_mode(
        content.into(),
        catalogue,
        bindings.local_variables,
    )?;
    original.edited_groups(
        crate::legacy::Edits {
            draft: &workspace.draft,
            session: &workspace.session,
            monitors: &workspace.monitors,
            bindings,
            workspaces: &workspace.workspace_rules,
            animations: &workspace.animations,
            gestures: &workspace.gestures,
            devices: &workspace.devices,
            rules: &workspace.rules,
        },
        catalogue,
    )
}
