//! Bounded legacy session projection and source copies. No variable expansion,
//! environment lookup, includes, command execution or policy installation.
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
fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_ascii_whitespace())
}
// Match the pinned hyprlang hash scan, including its skipped byte after ##.
// Quotes do not protect # in this grammar. Offsets still refer to input bytes.
fn code(line: &str) -> (String, usize) {
    let bytes = line.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            if bytes.get(i + 1) == Some(&b'#') {
                result.push(b'#');
                i += 2;
                if i < bytes.len() {
                    result.push(bytes[i]);
                    i += 1;
                }
                continue;
            }
            break;
        }
        result.push(bytes[i]);
        i += 1;
    }
    (
        String::from_utf8(result).expect("Hash removal preserves UTF-8"),
        i,
    )
}
fn candidate(key: &str) -> bool {
    key.starts_with("env") || key.starts_with("exec") || key == "permission"
}
fn has_expansion(value: &str) -> bool {
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '$'
            && chars
                .clone()
                .next()
                .is_some_and(|next| next.is_alphanumeric() || next == '_' || next == '{')
        {
            return true;
        }
    }
    false
}
fn unescape(value: &str) -> Result<String, String> {
    if value.len() > 64 * 1024 || has_expansion(value) {
        return Err("Variable expansion remains unresolved".into());
    }
    let bytes = value.as_bytes();
    for (at, pair) in bytes.windows(2).enumerate() {
        if pair == b"{{" {
            let count = bytes[..at]
                .iter()
                .rev()
                .take_while(|b| **b == b'\\')
                .count();
            if count % 2 == 0 {
                return Err("Legacy expressions remain unresolved".into());
            }
        }
    }
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && bytes
                .get(i + 1)
                .is_some_and(|b| matches!(b, b'\\' | b'{' | b'}'))
        {
            i += 1;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).map_err(|_| "Invalid legacy string".into())
}
fn csv(value: &str) -> Vec<String> {
    let mut fields = vec![];
    let (mut begin, mut escaped) = (0, vec![]);
    let bytes = value.as_bytes();
    let finish = |end: usize, begin: usize, escaped: &[usize]| {
        let mut skip = escaped.iter().copied().peekable();
        let part = bytes[begin..end]
            .iter()
            .enumerate()
            .filter_map(|(at, b)| {
                if skip.peek() == Some(&at) {
                    skip.next();
                    None
                } else {
                    Some(*b)
                }
            })
            .collect::<Vec<_>>();
        trim(std::str::from_utf8(&part).expect("Delimiter removal preserves UTF-8")).to_owned()
    };
    for (at, b) in bytes.iter().enumerate() {
        if *b != b',' {
            continue;
        }
        if at > begin && bytes[at - 1] == b'\\' {
            escaped.push(at - begin - 1);
            if at - begin == 1 || bytes[at - 2] != b'\\' {
                continue;
            }
        }
        fields.push(finish(at, begin, &escaped));
        begin = at + 1;
        escaped.clear();
    }
    if begin < bytes.len() {
        fields.push(finish(bytes.len(), begin, &escaped));
    }
    fields
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
    let (mut offset, mut depth, mut continued, mut flags, mut retained) =
        (0usize, 0usize, false, false, 0usize);
    for (index, line) in content.split_inclusive('\n').enumerate() {
        let physical = line.trim_end_matches(['\r', '\n']);
        let previous = continued;
        continued = physical.ends_with('\\');
        let raw = trim(physical);
        if raw.starts_with('#') {
            if trim(raw.trim_start_matches('#')).starts_with("hyprlang") {
                flags = true;
            }
            offset += line.len();
            continue;
        }
        let (decoded, end) = code(physical);
        let value = trim(&decoded);
        if let Some((key, rhs)) = value.split_once('=') {
            let key = trim(key);
            if candidate(key) {
                let parsed = if depth == 0
                    && !flags
                    && !previous
                    && !continued
                    && report.draft.entries.len() < 256
                {
                    decode(key, trim(rhs)).ok()
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
                    let rawcode = &physical[..end];
                    let statement = offset + rawcode.len()
                        - rawcode
                            .trim_start_matches(|c: char| c.is_ascii_whitespace())
                            .len()
                        ..offset
                            + rawcode
                                .trim_end_matches(|c: char| c.is_ascii_whitespace())
                                .len();
                    report.sites.push(Site {
                        line: index + 1,
                        statement,
                    });
                    report.draft.entries.push(entry);
                } else {
                    report.unresolved_count += 1;
                    if report.unresolved.len() < 4096 {
                        report.unresolved.push(index + 1);
                    }
                }
            }
        } else if value == "}" {
            depth = depth.saturating_sub(1);
        } else if value.ends_with('{') {
            depth = depth
                .checked_add(1)
                .ok_or("Legacy category depth overflow")?;
            if depth > 16 {
                return Err("Legacy category nesting exceeds 16".into());
            }
        }
        offset += line.len();
    }
    Ok(report)
}
fn literal(value: &str) -> Result<String, String> {
    if trim(value) != value || value.chars().any(|c| c.is_control()) || has_expansion(value) {
        return Err("This value needs legacy expansion, escaping or multiline syntax. Its draft can be saved, but no incomplete source copy was produced.".into());
    }
    // Encoding runs this way reproduces the pinned lexer's ## scan exactly.
    let mut result = String::new();
    let mut hashes = 0usize;
    for ch in value.chars().chain(std::iter::once('\0')) {
        if ch == '#' {
            hashes += 1;
            continue;
        }
        if hashes > 0 {
            result.push_str(&"#".repeat(hashes + hashes.div_ceil(2)));
            hashes = 0;
        }
        if ch != '\0' {
            if matches!(ch, '\\' | '{' | '}') {
                result.push('\\');
            }
            result.push(ch);
        }
    }
    Ok(result)
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
