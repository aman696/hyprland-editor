//! Flat literal monitorv2 blocks and comment-preserving field patches.
use super::*;
use crate::legacy_syntax::{code, trim};
use std::{collections::BTreeMap, ops::Range};
#[derive(Clone)]
pub(super) struct Field {
    key: String,
    value: String,
    statement: Range<usize>,
    value_span: Range<usize>,
}
pub(super) struct Block {
    pub site: Site,
    pub footer: usize,
    fields: Vec<Field>,
    editable: bool,
    depth: usize,
}
pub(super) fn scan(content: &str, mut visit: impl FnMut(Block)) -> Result<(), String> {
    if content.len() > MAX_DOCUMENT_BYTES || content.contains('\0') {
        return Err("Legacy source exceeds its bounds".into());
    }
    let (mut offset, mut depth, mut flags, mut continued) = (0usize, 0usize, false, false);
    let mut current: Option<Block> = None;
    for (index, line) in content.split_inclusive('\n').enumerate() {
        let physical = line.trim_end_matches(['\r', '\n']);
        let previous = continued;
        continued = physical.ends_with('\\');
        let (decoded, end) = code(physical);
        let value = trim(&decoded);
        let raw = &physical[..end];
        let statement = offset + raw.len()
            - raw
                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                .len()
            ..offset
                + raw
                    .trim_end_matches(|c: char| c.is_ascii_whitespace())
                    .len();
        if trim(physical).starts_with('#')
            && trim(trim(physical).trim_start_matches('#')).starts_with("hyprlang")
        {
            flags = true;
        }
        if let Some(block) = current.as_mut() {
            block.editable &= !flags && !previous && !continued;
            if value == "}" && depth == block.depth {
                let mut block = current.take().unwrap();
                block.site.statement.end = statement.end;
                block.footer = offset;
                visit(block);
            } else if !value.is_empty() {
                if depth != 1 || value.ends_with('{') || block.fields.len() >= 128 {
                    block.editable = false;
                } else if let Some((key, rhs)) = value.split_once('=') {
                    if let Some(eq) = raw.find('=') {
                        let raw_rhs = &raw[eq + 1..];
                        let start = offset + eq + 1 + raw_rhs.len()
                            - raw_rhs
                                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                                .len();
                        block.fields.push(Field {
                            key: trim(key).into(),
                            value: trim(rhs).into(),
                            statement: statement.clone(),
                            value_span: start
                                ..offset
                                    + raw
                                        .trim_end_matches(|c: char| c.is_ascii_whitespace())
                                        .len(),
                        });
                    } else {
                        block.editable = false;
                    }
                } else {
                    block.editable = false;
                }
            }
        } else if value
            .strip_suffix('{')
            .is_some_and(|name| trim(name) == "monitorv2")
            && !value.contains('=')
        {
            current = Some(Block {
                site: Site {
                    line: index + 1,
                    statement: statement.clone(),
                },
                footer: offset,
                fields: vec![],
                depth: depth + 1,
                editable: depth == 0 && !flags && !previous && !continued,
            });
        }
        if value == "}" {
            depth = depth.saturating_sub(1);
        } else if value.ends_with('{') && !value.contains('=') {
            depth += 1;
            if depth > 16 {
                return Err("Legacy category nesting exceeds 16".into());
            }
        }
        offset += line.len();
    }
    if let Some(mut block) = current {
        block.editable = false;
        block.site.statement.end = content.len();
        visit(block);
    }
    Ok(())
}
fn integer(value: &str) -> Result<i64, String> {
    match value {
        "true" | "yes" | "on" => Ok(1),
        "false" | "no" | "off" => Ok(0),
        _ => value.parse().map_err(|_| "Invalid monitor integer".into()),
    }
}
pub(super) fn parse(block: &Block, index: usize) -> Result<Rule, String> {
    if !block.editable
        || block
            .fields
            .first()
            .is_none_or(|field| field.key != "output")
    {
        return Err("Monitor block needs a first literal output and flat known fields".into());
    }
    let fields = monitor::catalogue(BASELINE);
    let mut rule = Rule::new(&format!("legacy-monitor-{index}"));
    rule.legacy_operation = Some(Op::Block);
    let mut output_seen = false;
    for field in &block.fields {
        let value = unescape(&field.value)?;
        if field.key == "output" {
            if output_seen || value.is_empty() {
                return Err("Monitor block needs one explicit output".into());
            }
            rule.output = if let Some(name) = value.strip_prefix("desc:") {
                Output::Description(name.into())
            } else {
                Output::Connector(value)
            };
            output_seen = true;
            continue;
        }
        let key = if field.key == "addreserved" {
            "reserved"
        } else {
            &field.key
        };
        // Only addreserved is a native block spelling; Lua aliases are GUI choices.
        if matches!(field.key.as_str(), "reserved" | "reserved_area") {
            return Err("Unsupported monitor block spelling".into());
        }
        let setting = fields
            .iter()
            .find(|s| s.lua_key == key)
            .ok_or("Unknown monitor block field")?;
        let typed = match setting.kind.as_str() {
            "String" => Value::String(value),
            "Bool" => Value::Bool(integer(&value)? != 0),
            "Int" => Value::Int(integer(&value)?),
            "Float" => Value::Float(value.parse().map_err(|_| "Invalid monitor number")?),
            "CssGap" => {
                let sides = value
                    .split(',')
                    .map(|s| trim(s).parse::<i64>().map_err(|_| "Invalid reserved side"))
                    .collect::<Result<Vec<_>, _>>()?;
                if sides.len() != 4 {
                    return Err("Reserved space needs four sides".into());
                }
                Value::CssGap([sides[0], sides[3], sides[1], sides[2]])
            }
            _ => return Err("Unsupported monitor block type".into()),
        };
        rule.fields.insert(key.into(), typed);
    }
    rule.validate(BASELINE)?;
    Ok(rule)
}
fn key(name: &str) -> &str {
    if matches!(name, "reserved" | "reserved_area") {
        "addreserved"
    } else {
        name
    }
}
fn value(value: &Value) -> Result<String, String> {
    literal(&match value {
        Value::String(s) => s.clone(),
        Value::Bool(v) => i32::from(*v).to_string(),
        Value::Int(v) => v.to_string(),
        Value::Float(v) => v.to_string(),
        Value::CssGap([top, right, bottom, left]) => format!("{top},{bottom},{left},{right}"),
        _ => return Err("Unsupported monitor block value".into()),
    })
}
fn fields(rule: &Rule) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::from([("output".into(), literal(&rule.output.text())?)]);
    for (name, typed) in &rule.fields {
        result.insert(key(name).into(), value(typed)?);
    }
    Ok(result)
}
pub(super) fn encode(rule: &Rule) -> Result<String, String> {
    rule.validate(BASELINE)?;
    let mut data = fields(rule)?;
    let output = data.remove("output").unwrap();
    let mut text = format!("monitorv2 {{\n    output = {output}\n");
    for (key, value) in data {
        text.push_str(&format!("    {key} = {value}\n"));
    }
    text.push('}');
    Ok(text)
}
/// Return source-relative patches; unchanged field spellings and comments stay exact.
pub(super) fn patches(
    content: &str,
    block: &Block,
    before: &Rule,
    target: &Rule,
) -> Result<Vec<(Range<usize>, String)>, String> {
    let (old, new) = (fields(before)?, fields(target)?);
    let mut edits = vec![];
    for field in &block.fields {
        if old.get(&field.key) == new.get(&field.key) {
            continue;
        }
        if let Some(value) = new.get(&field.key) {
            if block
                .fields
                .iter()
                .rev()
                .find(|f| f.key == field.key)
                .is_some_and(|last| last.statement == field.statement)
            {
                let suffix = if content.as_bytes().get(field.value_span.end) == Some(&b'#') {
                    " "
                } else {
                    ""
                };
                edits.push((field.value_span.clone(), format!("{value}{suffix}")));
            }
        } else {
            edits.push((field.statement.clone(), String::new()));
        }
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut added = String::new();
    for (key, value) in &new {
        if !block.fields.iter().any(|field| &field.key == key) {
            added.push_str(&format!("    {key} = {value}{newline}"));
        }
    }
    if !added.is_empty() {
        edits.push((block.footer..block.footer, added));
    }
    Ok(edits)
}
/// Preserve the block's comments if its operation is explicitly changed or removed.
pub(super) fn comments(content: &str, span: Range<usize>) -> String {
    let mut result = String::new();
    for line in content[span].split_inclusive('\n') {
        let physical = line.trim_end_matches(['\r', '\n']);
        let (decoded, end) = code(physical);
        if trim(&decoded).is_empty() {
            result.push_str(line);
        } else {
            result.push_str(&line[end..]);
        }
    }
    result
}
