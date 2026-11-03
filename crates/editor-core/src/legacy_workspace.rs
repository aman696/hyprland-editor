//! Literal ordered workspace declarations. Hooks remain inert source text.
use crate::{
    legacy_session::Site,
    legacy_syntax::{self, csv, literal, unescape},
    lua::source_hash,
    workspace_rule::{self, Rule},
    Value, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
// Order matters: the native handler searches substrings in this order.
const FIELDS: &[(&str, &str)] = &[
    ("gapsin:", "gaps_in"),
    ("gapsout:", "gaps_out"),
    ("bordersize:", "border_size"),
    ("border:", "no_border"),
    ("shadow:", "no_shadow"),
    ("rounding:", "no_rounding"),
    ("decorate:", "decorate"),
    ("monitor:", "monitor"),
    ("default:", "default"),
    ("persistent:", "persistent"),
    ("defaultName:", "default_name"),
    ("on-created-empty:", "on_created_empty"),
    ("layoutopt:", "layout_options"),
    ("layout:", "layout"),
    ("animation:", "animation"),
];
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_sha256: String,
    pub source_sha256: String,
    pub rules: Vec<Rule>,
    pub sites: Vec<Site>,
    pub unresolved: Vec<usize>,
    pub unresolved_count: usize,
}
fn boolean(value: &str) -> Result<bool, String> {
    match value {
        "true" | "yes" | "on" => Ok(true),
        "false" | "no" | "off" => Ok(false),
        _ => value
            .parse::<i64>()
            .map(|n| n != 0)
            .map_err(|_| "Unsupported workspace boolean".into()),
    }
}
fn gap(value: &str) -> Result<Value, String> {
    let v = value
        .split_ascii_whitespace()
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Invalid workspace gap")?;
    let sides = match v.as_slice() {
        [a] => [*a; 4],
        [a, b] => [*a, *b, *a, *b],
        [a, b, c] => [*a, *b, *c, *b],
        [a, b, c, d] => [*a, *b, *c, *d],
        _ => return Err("Workspace gaps need one to four sides".into()),
    };
    Ok(Value::CssGap(sides))
}
fn decode(value: &str, index: usize) -> Result<Rule, String> {
    let value = unescape(value)?;
    // The selector uses find_first_of(',') before the escaped argument splitter.
    let (selector, options) = value
        .split_once(',')
        .ok_or("Workspace delimiter required")?;
    let mut rule = Rule::new(&format!("legacy-workspace-{index}"));
    rule.legacy = true;
    rule.selector = selector.trim().into();
    if rule.selector.is_empty() {
        return Err("Workspace selector required".into());
    }
    for token in csv(options) {
        if token.is_empty() {
            continue;
        }
        let (prefix, key) = FIELDS
            .iter()
            .find(|(prefix, _)| token.contains(prefix))
            .ok_or("Unknown workspace option")?;
        let raw = token
            .strip_prefix(prefix)
            .ok_or("Ambiguous workspace option prefix")?;
        if *key == "layout_options" {
            let (key, value) = raw
                .split_once(':')
                .ok_or("Layout option needs a name and value")?;
            rule.layout_options
                .insert(key.into(), Value::String(value.into()));
        } else {
            let value = match *key {
                "gaps_in" | "gaps_out" => gap(raw)?,
                "border_size" => Value::Int(
                    raw.parse::<i32>()
                        .map_err(|_| "Invalid border size")?
                        .into(),
                ),
                "no_border" | "no_shadow" | "no_rounding" => Value::Bool(!boolean(raw)?),
                "decorate" | "default" | "persistent" => Value::Bool(boolean(raw)?),
                "default_name" => Value::String(raw.trim().into()),
                _ => Value::String(raw.into()),
            };
            rule.fields.insert((*key).into(), value);
        }
    }
    rule.validate(BASELINE)?;
    Ok(rule)
}
pub fn inspect(content: &str) -> Result<Report, String> {
    let mut report = Report {
        schema_sha256: schema_hash(),
        source_sha256: source_hash(content),
        rules: vec![],
        sites: vec![],
        unresolved: vec![],
        unresolved_count: 0,
    };
    let mut retained = 2usize;
    legacy_syntax::walk(content, |line| {
        if line.key != "workspace" {
            return;
        }
        let mut rule = (line.editable && report.rules.len() < workspace_rule::MAX_RULES)
            .then(|| decode(line.value, report.rules.len()).ok())
            .flatten();
        if let Some(value) = &rule {
            let size = serde_json::to_vec(value).map_or(MAX_DOCUMENT_BYTES + 1, |v| v.len());
            if retained.saturating_add(size + 1) > MAX_DOCUMENT_BYTES {
                rule = None;
            } else {
                retained += size + 1;
            }
        }
        if let Some(rule) = rule {
            report.rules.push(rule);
            report.sites.push(Site {
                line: line.line,
                statement: line.statement,
            });
        } else {
            report.unresolved_count += 1;
            if report.unresolved.len() < 4096 {
                report.unresolved.push(line.line);
            }
        }
    })?;
    workspace_rule::validate(&report.rules, BASELINE)?;
    Ok(report)
}
fn argument(value: &str) -> Result<String, String> {
    if value.contains("\\,") {
        return Err("Backslash-comma needs an explicit workspace conversion".into());
    }
    let mut escaped = value.replace(',', "\\,");
    if escaped.ends_with('\\') {
        escaped.push('\\');
    }
    literal(&escaped)
}
pub fn encode(rule: &Rule) -> Result<String, String> {
    rule.validate(BASELINE)?;
    if !rule.legacy {
        return Err("Separate Lua workspace drafts need an explicit legacy conversion".into());
    }
    if rule.selector.is_empty() || rule.selector.contains(',') {
        return Err("Choose a literal workspace selector without commas".into());
    }
    let mut args = vec![];
    for (prefix, key) in FIELDS {
        if *key == "layout_options" {
            for (key, value) in &rule.layout_options {
                let Value::String(value) = value else {
                    return Err("Native layout options require strings".into());
                };
                if key.contains(':') {
                    return Err("Native layout option names cannot contain a colon".into());
                }
                args.push(format!("{prefix}{key}:{value}"));
            }
        } else if let Some(value) = rule.fields.get(*key) {
            let value = match value {
                Value::Bool(v) => (if matches!(*key, "no_border" | "no_shadow" | "no_rounding") {
                    !v
                } else {
                    *v
                })
                .to_string(),
                Value::Int(v) => i32::try_from(*v)
                    .map_err(|_| "Workspace integer exceeds native range")?
                    .to_string(),
                Value::String(v) => v.clone(),
                Value::CssGap(v) => v.iter().map(i64::to_string).collect::<Vec<_>>().join(" "),
                _ => return Err("Unsupported native workspace value".into()),
            };
            args.push(format!("{prefix}{value}"));
        }
    }
    let out = format!(
        "workspace = {},{}",
        literal(&rule.selector)?,
        args.iter()
            .map(|v| argument(v))
            .collect::<Result<Vec<_>, _>>()?
            .join(",")
    );
    let (decoded, _) = legacy_syntax::code(out.strip_prefix("workspace = ").unwrap());
    let reparsed = decode(&decoded, 0)?;
    if !workspace_rule::same_declarations(std::slice::from_ref(rule), &[reparsed]) {
        return Err("Native workspace spelling would change a field; no copy produced".into());
    }
    Ok(out)
}
pub fn export(rules: &[Rule]) -> Result<String, String> {
    workspace_rule::validate(rules, BASELINE)?;
    let mut out = String::new();
    for rule in rules {
        out.push_str(&encode(rule)?);
        out.push('\n');
        if out.len() > MAX_DOCUMENT_BYTES {
            return Err("Workspace export exceeds 1 MiB".into());
        }
    }
    Ok(out)
}
pub fn edited_copy(content: &str, target: &[Rule]) -> Result<String, String> {
    workspace_rule::validate(target, BASELINE)?;
    let before = inspect(content)?;
    // IDs identify private rows, never source semantics.
    if workspace_rule::same_declarations(&before.rules, target) {
        return Ok(content.into());
    }
    let mut output = content.to_owned();
    for (index, site) in before.sites.iter().enumerate().rev() {
        if target.get(index).is_some_and(|r| {
            workspace_rule::same_declarations(
                &before.rules[index..index + 1],
                std::slice::from_ref(r),
            )
        }) {
            continue;
        }
        let mut text = target
            .get(index)
            .map(encode)
            .transpose()?
            .unwrap_or_default();
        if content.as_bytes().get(site.statement.end) == Some(&b'#') && !text.is_empty() {
            text.push(' ');
        }
        output.replace_range(site.statement.clone(), &text);
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    for rule in target.iter().skip(before.sites.len()) {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(&encode(rule)?);
        output.push_str(newline);
    }
    if output.len() > MAX_DOCUMENT_BYTES {
        return Err("Legacy copy exceeds 1 MiB".into());
    }
    let after = inspect(&output)?;
    if !workspace_rule::same_declarations(&after.rules, target)
        || before.unresolved_count != after.unresolved_count
    {
        return Err("Workspace source copy does not reproduce the draft".into());
    }
    Ok(output)
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-workspaces.json"))
}
