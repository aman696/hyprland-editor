//! Conservative legacy literal import. Includes, variables and commands are never followed.
use crate::{parse_color, Draft, Setting, Value, BASELINE, MAX_DOCUMENT_BYTES};

#[derive(Debug)]
pub struct ImportReport {
    pub draft: Draft,
    pub skipped: Vec<usize>,
    pub duplicate_keys: Vec<String>,
}

fn without_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        if c == '"' && !escaped {
            quoted = !quoted;
        }
        if c == '#' && !quoted {
            return &line[..i];
        }
        escaped = c == '\\' && !escaped;
    }
    line
}
fn color(s: &str) -> Result<u32, String> {
    if let Some(hex) = s.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
        if hex.len() != 8 {
            return Err("Invalid RGBA color".into());
        }
        let rgba = u32::from_str_radix(hex, 16).map_err(|_| "Invalid RGBA color")?;
        return Ok(rgba.rotate_right(8));
    }
    if let Some(hex) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        if hex.len() != 6 {
            return Err("Invalid RGB color".into());
        }
        return u32::from_str_radix(hex, 16)
            .map(|c| c | 0xff000000)
            .map_err(|_| "Invalid RGB color".into());
    }
    parse_color(s)
}
fn literal(s: &Setting, raw: &str) -> Option<Value> {
    if raw.contains('$') || raw.contains('\0') {
        return None;
    }
    match s.kind.as_str() {
        "Bool" => match raw {
            "true" | "yes" | "on" | "1" => Some(Value::Bool(true)),
            "false" | "no" | "off" | "0" => Some(Value::Bool(false)),
            _ => None,
        },
        "Int" => raw.parse().ok().map(Value::Int),
        "Float" => raw.parse().ok().map(Value::Float),
        "FontWeight" => raw.parse().ok().map(Value::FontWeight),
        "String" => Some(Value::String(raw.to_string())),
        "Color" => color(raw).ok().map(Value::Color),
        "Gradient" => {
            let mut angle = 0.;
            let mut colors = vec![];
            for token in raw.split_whitespace() {
                if let Some(deg) = token.strip_suffix("deg") {
                    angle = deg.parse().ok()?;
                } else {
                    colors.push(color(token).ok()?);
                }
            }
            Some(Value::Gradient { colors, angle })
        }
        "Vec2" => {
            let v: Vec<f64> = raw
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()
                .ok()?;
            Some(Value::Vec2(v.try_into().ok()?))
        }
        "CssGap" => {
            let v: Vec<i64> = raw
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()
                .ok()?;
            match v.as_slice() {
                [a] => Some(Value::CssGap([*a; 4])),
                [a, b] => Some(Value::CssGap([*a, *b, *a, *b])),
                [a, b, c, d] => Some(Value::CssGap([*a, *b, *c, *d])),
                _ => None,
            }
        }
        _ => None,
    }
}
pub fn legacy(text: &str, catalogue: &[Setting]) -> Result<ImportReport, String> {
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err("Configuration exceeds 1 MiB".into());
    }
    let mut report = ImportReport {
        draft: Draft::new(BASELINE),
        skipped: vec![],
        duplicate_keys: vec![],
    };
    let mut blocks: Vec<String> = vec![];
    for (index, line) in text.lines().enumerate() {
        let line = without_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if line == "}" {
            if blocks.pop().is_none() {
                return Err(format!("Unmatched closing brace at line {}", index + 1));
            }
            continue;
        }
        if let Some(name) = line.strip_suffix('{').map(str::trim) {
            if blocks.len() >= 16
                || name.is_empty()
                || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(format!("Unsupported block at line {}", index + 1));
            }
            blocks.push(name.into());
            continue;
        }
        let Some((key, raw)) = line.split_once('=') else {
            report.skipped.push(index + 1);
            continue;
        };
        let key = key.trim();
        let full = if blocks.is_empty() {
            key.into()
        } else {
            format!("{}:{key}", blocks.join(":"))
        };
        let Some(setting) = catalogue
            .iter()
            .find(|s| s.version == BASELINE && s.id == full)
        else {
            report.skipped.push(index + 1);
            continue;
        };
        let Some(value) = literal(setting, raw.trim()) else {
            report.draft.values.remove(&full);
            report.skipped.push(index + 1);
            continue;
        };
        if setting.validate(&value).is_err() {
            report.draft.values.remove(&full);
            report.skipped.push(index + 1);
            continue;
        }
        if report.draft.values.insert(full.clone(), value).is_some() {
            report.duplicate_keys.push(full);
        }
    }
    if !blocks.is_empty() {
        return Err("Unclosed configuration block".into());
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_never_follows_sources_or_executes() {
        let r=legacy("source = /etc/shadow\nexec = touch /tmp/never\ngeneral {\n border_size = 4\n gaps_in = $variable\n}\n",&crate::catalogue()).unwrap();
        assert_eq!(r.draft.values.len(), 1);
        assert_eq!(r.skipped, vec![1, 2, 5]);
        assert_eq!(r.draft.values["general:border_size"], Value::Int(4));
    }
    #[test]
    fn imports_last_literal_and_reports_overrides() {
        let r = legacy(
            "general:border_size = 2\ngeneral:border_size = 3 # comment",
            &crate::catalogue(),
        )
        .unwrap();
        assert_eq!(r.draft.values["general:border_size"], Value::Int(3));
        assert_eq!(r.duplicate_keys.len(), 1);
    }
    #[test]
    fn nested_sections_and_color_order() {
        let r = legacy(
            "decoration {\n shadow {\n color = rgba(123456FF)\n }\n}",
            &crate::catalogue(),
        )
        .unwrap();
        assert_eq!(
            r.draft.values["decoration:shadow:color"],
            Value::Gradient {
                colors: vec![0xff123456],
                angle: 0.
            }
        );
    }
    #[test]
    fn later_unknown_assignment_invalidates_earlier_literal() {
        let r = legacy(
            "general:border_size = 2\ngeneral:border_size = $size",
            &crate::catalogue(),
        )
        .unwrap();
        assert!(r.draft.values.is_empty());
        assert_eq!(r.skipped, vec![2]);
    }
    #[test]
    fn malformed_and_oversized_fail_closed() {
        assert!(legacy("}", &crate::catalogue()).is_err());
        assert!(legacy("general {", &crate::catalogue()).is_err());
        assert!(legacy(&"x".repeat(MAX_DOCUMENT_BYTES + 1), &[]).is_err());
    }
}
