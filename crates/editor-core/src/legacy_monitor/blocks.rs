//! Flat literal monitorv2 blocks and comment-preserving field patches.
use super::*;
use crate::{legacy_blocks::Block, legacy_syntax::trim};
use std::{collections::BTreeMap, ops::Range};
pub(super) fn scan(content: &str, visit: impl FnMut(Block)) -> Result<(), String> {
    crate::legacy_blocks::scan(content, "monitorv2", visit)
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
pub(super) fn patches(
    content: &str,
    block: &Block,
    before: &Rule,
    target: &Rule,
) -> Result<Vec<(Range<usize>, String)>, String> {
    crate::legacy_blocks::patches(content, block, &fields(before)?, &fields(target)?)
}
pub(super) fn comments(content: &str, span: Range<usize>) -> String {
    crate::legacy_blocks::comments(content, span)
}
