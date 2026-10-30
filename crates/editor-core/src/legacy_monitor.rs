//! Ordered legacy monitor declarations. Source updates are inert and bounded.
use crate::{
    legacy_session::Site,
    legacy_syntax::{self, csv, literal, unescape},
    lua::source_hash,
    monitor::{self, LegacyOperation as Op, Output, Rule},
    Value, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
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
fn decode(value: &str, index: usize) -> Result<Rule, String> {
    let parts = csv(&unescape(value)?);
    let arg = |at: usize| parts.get(at).map(String::as_str).unwrap_or("");
    let mut rule = Rule::new(&format!("legacy-monitor-{index}"));
    rule.output = match arg(0) {
        "" => Output::Fallback,
        name if name.starts_with("desc:") => Output::Description(name[5..].into()),
        name => Output::Connector(name.into()),
    };
    let mut insert = |key: &str, value| {
        rule.fields.insert(key.into(), value);
    };
    let operation = match arg(1) {
        "disable" | "disabled" if parts.len() == 2 => {
            insert("disabled", Value::Bool(true));
            Op::Disable
        }
        "transform" if parts.len() == 3 => {
            insert(
                "transform",
                Value::Int(arg(2).parse().map_err(|_| "Invalid transform")?),
            );
            Op::Transform
        }
        "addreserved" if parts.len() == 6 => {
            let number = |at| arg(at).parse::<i64>().map_err(|_| "Invalid reserved space");
            insert(
                "reserved",
                Value::CssGap([number(2)?, number(5)?, number(3)?, number(4)?]),
            );
            Op::Reserved
        }
        "disable" | "disabled" | "transform" | "addreserved" => {
            return Err("Unsupported monitor operation arguments".into())
        }
        _ => {
            for (key, at, default) in [
                ("mode", 1, "preferred"),
                ("position", 2, "auto"),
                ("scale", 3, "auto"),
            ] {
                insert(
                    key,
                    Value::String(if arg(at).is_empty() { default } else { arg(at) }.into()),
                );
            }
            let mut at = 4;
            while at < parts.len() {
                let key = arg(at);
                if !Op::Replace.accepts(key)
                    || matches!(key, "mode" | "position" | "scale")
                    || at + 1 >= parts.len()
                {
                    return Err("Unsupported legacy monitor option".into());
                }
                let value = match key {
                    "transform" | "bitdepth" | "vrr" => {
                        Value::Int(arg(at + 1).parse().map_err(|_| "Invalid monitor integer")?)
                    }
                    "sdrbrightness" | "sdrsaturation" => {
                        Value::Float(arg(at + 1).parse().map_err(|_| "Invalid monitor number")?)
                    }
                    _ => Value::String(arg(at + 1).into()),
                };
                insert(key, value);
                at += 2;
            }
            Op::Replace
        }
    };
    rule.legacy_operation = Some(operation);
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
    legacy_syntax::walk(content, |line| {
        if line.key != "monitor" {
            return;
        }
        let parsed = if line.editable && report.rules.len() < monitor::MAX_RULES {
            decode(line.value, report.rules.len()).ok()
        } else {
            None
        };
        if let Some(rule) = parsed {
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
    Ok(report)
}
fn argument(value: &str) -> Result<String, String> {
    if value.contains("\\,") {
        return Err("Backslash-comma monitor arguments need an explicit legacy conversion".into());
    }
    let mut escaped = value.replace(',', "\\,");
    // The argument splitter consumes one escape before a delimiter.
    if escaped.ends_with('\\') {
        escaped.push('\\');
    }
    literal(&escaped)
}
pub fn encode(rule: &Rule) -> Result<String, String> {
    rule.validate(BASELINE)?;
    let operation = rule
        .legacy_operation
        .ok_or("Choose an explicit legacy monitor operation before source export")?;
    let mut args = vec![rule.output.text()];
    match operation {
        Op::Disable => args.push("disable".into()),
        Op::Transform => {
            args.push("transform".into());
            let Some(Value::Int(value)) = rule.fields.get("transform") else {
                return Err("Set a transform for this update".into());
            };
            args.push(value.to_string());
        }
        Op::Reserved => {
            args.push("addreserved".into());
            let Some(Value::CssGap([top, right, bottom, left])) = rule
                .fields
                .get("reserved")
                .or_else(|| rule.fields.get("reserved_area"))
            else {
                return Err("Set four reserved-space sides for this update".into());
            };
            for value in [top, bottom, left, right] {
                args.push(value.to_string());
            }
        }
        Op::Replace => {
            for (key, default) in [
                ("mode", "preferred"),
                ("position", "auto"),
                ("scale", "auto"),
            ] {
                args.push(match rule.fields.get(key) {
                    Some(Value::String(value)) => value.clone(),
                    _ => default.into(),
                });
            }
            for (key, value) in &rule.fields {
                if matches!(key.as_str(), "mode" | "position" | "scale") {
                    continue;
                }
                args.push(key.clone());
                args.push(match value {
                    Value::String(s) => s.clone(),
                    Value::Int(v) => v.to_string(),
                    Value::Float(v) => v.to_string(),
                    _ => return Err("Unrepresentable legacy monitor value".into()),
                });
            }
        }
    }
    Ok(format!(
        "monitor = {}",
        args.iter()
            .map(|s| argument(s))
            .collect::<Result<Vec<_>, _>>()?
            .join(",")
    ))
}
pub fn export(rules: &[Rule]) -> Result<String, String> {
    monitor::validate(rules, BASELINE)?;
    let mut out = String::new();
    for rule in rules {
        out.push_str(&encode(rule)?);
        out.push('\n');
    }
    if out.len() > MAX_DOCUMENT_BYTES {
        return Err("Legacy monitor draft exceeds 1 MiB".into());
    }
    Ok(out)
}
pub fn edited_copy(content: &str, target: &[Rule]) -> Result<String, String> {
    monitor::validate(target, BASELINE)?;
    let before = inspect(content)?;
    if before.rules == target {
        return Ok(content.into());
    }
    let mut output = content.to_owned();
    for (index, site) in before.sites.iter().enumerate().rev() {
        if target.get(index) == before.rules.get(index) {
            continue;
        }
        let text = target
            .get(index)
            .map(|rule| {
                encode(rule).map(|s| {
                    format!(
                        "{s}{}",
                        if content.as_bytes().get(site.statement.end) == Some(&b'#') {
                            " "
                        } else {
                            ""
                        }
                    )
                })
            })
            .transpose()?
            .unwrap_or_default();
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
        return Err("Legacy source copy exceeds 1 MiB".into());
    }
    let after = inspect(&output)?;
    let canonical = |rules: &[Rule]| rules.iter().map(encode).collect::<Result<Vec<_>, _>>();
    if canonical(&after.rules)? != canonical(target)?
        || before.unresolved_count != after.unresolved_count
    {
        return Err(
            "The legacy copy did not reproduce the monitor draft; no copy was published".into(),
        );
    }
    Ok(output)
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-monitors.json"))
}
