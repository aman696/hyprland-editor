//! Bounded legacy session projection and source copies. No variable expansion,
//! environment lookup, includes, command execution or policy installation.
use crate::legacy_syntax::{self, csv, literal, trim, unescape};
use crate::{
    lua::source_hash,
    session::{Draft, Entry, PermissionMode, PermissionType, Trigger},
    MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::ops::Range;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub line: usize,
    pub statement: Range<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_sha256: String,
    pub source_sha256: String,
    pub draft: Draft,
    pub sites: Vec<Site>,
    pub unresolved: Vec<usize>,
    pub unresolved_count: usize,
}
fn candidate(key: &str) -> bool {
    key.starts_with("env") || key.starts_with("exec") || key == "permission"
}
fn decode(key: &str, value: &str) -> Result<Entry, String> {
    let value = unescape(value)?;
    let value = value.as_str();
    let entry = match key {
        "env" | "envd" => {
            let (name, value) = value.split_once(',').unwrap_or((value, ""));
            Entry::Environment {
                name: trim(name).into(),
                value: trim(value).into(),
                dbus: (key == "envd").then_some(true),
            }
        }
        "exec" | "execr" | "exec-once" | "execr-once" | "exec-shutdown" => Entry::LegacyCommand {
            command: value.into(),
            raw: key.starts_with("execr"),
            trigger: match key {
                "exec" | "execr" => Trigger::ConfigLoad,
                "exec-once" | "execr-once" => Trigger::Start,
                _ => Trigger::Shutdown,
            },
        },
        "permission" => {
            let parts = csv(value);
            if parts.len() != 3 {
                return Err("Permission needs three independent arguments".into());
            }
            crate::regex::check(&crate::regex::Sample {
                pattern: parts[0].clone(),
                text: String::new(),
                negated: false,
            })?;
            Entry::Permission {
                binary: parts[0].clone(),
                permission: PermissionType::parse(&parts[1]).ok_or("Unknown permission type")?,
                mode: PermissionMode::parse(&parts[2]).ok_or("Unknown permission mode")?,
            }
        }
        _ => return Err("Unknown session directive suffix".into()),
    };
    entry.validate()?;
    Ok(entry)
}
/// Untrusted calls must use the application's restricted source worker.
pub fn inspect(content: &str) -> Result<Report, String> {
    if content.len() > MAX_DOCUMENT_BYTES || content.contains('\0') {
        return Err("Legacy source exceeds its size policy or contains NUL".into());
    }
    let mut report = Report {
        schema_sha256: schema_hash(),
        source_sha256: source_hash(content),
        draft: Draft::default(),
        sites: vec![],
        unresolved: vec![],
        unresolved_count: 0,
    };
    let mut retained = 0usize;
    legacy_syntax::walk(content, |line| {
        if !candidate(line.key) {
            return;
        }
        let parsed = if line.editable && report.draft.entries.len() < 256 {
            decode(line.key, line.value).ok()
        } else {
            None
        };
        let parsed = parsed.filter(|entry| {
            let size = if matches!(entry, Entry::LegacyCommand { .. }) {
                serde_json::to_vec(entry).ok().map(|v| v.len())
            } else {
                entry.export_lua().ok().map(|v| v.len())
            };
            if let Some(size) = size {
                if retained.saturating_add(size + 1) <= MAX_DOCUMENT_BYTES {
                    retained += size + 1;
                    return true;
                }
            }
            false
        });
        if let Some(entry) = parsed {
            report.sites.push(Site {
                line: line.line,
                statement: line.statement,
            });
            report.draft.entries.push(entry);
        } else {
            report.unresolved_count += 1;
            if report.unresolved.len() < 4096 {
                report.unresolved.push(line.line);
            }
        }
    })?;
    Ok(report)
}
pub fn encode(entry: &Entry) -> Result<String, String> {
    entry.validate()?;
    Ok(match entry {
        Entry::Environment{name,value,dbus}=>format!("{} = {},{}",if *dbus==Some(true){"envd"}else{"env"},literal(name)?,literal(value)?),
        Entry::LegacyCommand{command,trigger,raw}=>format!("{} = {}",match(trigger,raw){(Trigger::ConfigLoad,false)=>"exec",(Trigger::ConfigLoad,true)=>"execr",(Trigger::Start,false)=>"exec-once",(Trigger::Start,true)=>"execr-once",(Trigger::Shutdown,false)=>"exec-shutdown",_=>return Err("This launch mode has no legacy directive".into())},literal(command)?),
        Entry::Command{..}=>return Err("Choose an explicit legacy command mode in the session editor before exporting Lua command intent into .conf".into()),
        Entry::Permission{binary,permission,mode}=>{if binary.contains("\\,"){return Err("Backslash-comma patterns need an explicit legacy pattern conversion".into());}
            let mut argument=binary.replace(',',"\\,");if argument.ends_with('\\'){argument.push('\\');}
            format!("permission = {},{},{}",literal(&argument)?,permission.text(),mode.text())}
    })
}
/// Rewrite known slots only; unresolved directives and all other bytes survive.
pub fn edited_copy(content: &str, target: &Draft) -> Result<String, String> {
    target.validate(crate::BASELINE)?;
    let before = inspect(content)?;
    if before.draft == *target {
        return Ok(content.into());
    }
    let mut edits = vec![];
    for (index, site) in before.sites.iter().enumerate() {
        let replacement = if let Some(entry) = target.entries.get(index) {
            if before.draft.entries[index] == *entry {
                continue;
            }
            format!("{} ", encode(entry)?)
        } else {
            String::new()
        };
        edits.push((site.statement.clone(), replacement));
    }
    let mut output = content.to_owned();
    for (span, replacement) in edits.into_iter().rev() {
        output.replace_range(span, &replacement);
    }
    if target.entries.len() > before.sites.len() {
        let newline = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        for entry in &target.entries[before.sites.len()..] {
            output.push_str(&encode(entry)?);
            output.push_str(newline);
        }
    }
    if output.len() > MAX_DOCUMENT_BYTES {
        return Err("Legacy source copy exceeds 1 MiB".into());
    }
    let after = inspect(&output)?;
    let wanted = target
        .entries
        .iter()
        .map(encode)
        .collect::<Result<Vec<_>, _>>()?;
    let actual = after
        .draft
        .entries
        .iter()
        .map(encode)
        .collect::<Result<Vec<_>, _>>()?;
    if actual != wanted || after.unresolved_count != before.unresolved_count {
        return Err("The legacy source copy did not reproduce the requested session draft; nothing was published".into());
    }
    Ok(output)
}

pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-session.json"))
}
