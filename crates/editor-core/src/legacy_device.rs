//! Native device categories remain ordered source declarations, never hardware state.
use crate::{
    device::{self, Spec},
    legacy_blocks::{self, Block},
    legacy_session::Site,
    legacy_syntax::{literal, unescape},
    lua::source_hash,
    Setting, Value, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_sha256: String,
    pub source_sha256: String,
    pub devices: Vec<Spec>,
    pub sites: Vec<Site>,
    pub unresolved: Vec<usize>,
    pub unresolved_count: usize,
}
pub fn native_key(key: &str) -> &str {
    match key {
        "tap_to_click" => "tap-to-click",
        "tap_and_drag" => "tap-and-drag",
        key => key,
    }
}
fn gui_key(key: &str) -> &str {
    match key {
        "tap-to-click" => "tap_to_click",
        "tap-and-drag" => "tap_and_drag",
        key => key,
    }
}
fn number(n: f64) -> Result<f32, String> {
    let v = n as f32;
    if !v.is_finite() || v.is_subnormal() || v == 0. && n != 0. {
        return Err("Native device number exceeds its normal float32 domain".into());
    }
    Ok(v)
}
pub fn validate_value(setting: &Setting, value: &Value) -> Result<(), String> {
    match (setting.kind.as_str(), value) {
        ("Bool", Value::Int(_)) => Ok(()),
        ("Bool", Value::Bool(_)) | ("Int", Value::Int(_)) => Ok(()),
        ("Float", Value::Float(n)) => number(*n).map(|_| ()),
        ("Vec2", Value::Vec2(v)) => {
            for n in v {
                number(*n)?;
            }
            Ok(())
        }
        ("String", Value::String(s)) if s.len() <= 16384 && !s.contains('\0') => Ok(()),
        _ => Err("Native device value does not match its registered type".into()),
    }
}
pub fn validate_spec(s: &Spec, version: &str) -> Result<(), String> {
    if !s.legacy || version != BASELINE {
        return Err("Native devices require the baseline and native provenance".into());
    }
    let fields = device::catalogue(BASELINE);
    if s.fields.len() > fields.len() {
        return Err("Too many device fields".into());
    }
    for (key, value) in &s.fields {
        let setting = fields
            .iter()
            .find(|f| f.lua_key == *key)
            .ok_or("Unknown native device field")?;
        validate_value(setting, value)?;
    }
    Ok(())
}
fn integer(raw: &str) -> Result<i64, String> {
    match raw {
        "true" | "yes" | "on" => Ok(1),
        "false" | "no" | "off" => Ok(0),
        s => s
            .parse()
            .map_err(|_| "Unresolved native integer spelling".into()),
    }
}
fn floating(raw: &str) -> Result<f64, String> {
    let v: f32 = raw
        .parse()
        .map_err(|_| "Unresolved native number spelling")?;
    if !v.is_finite()
        || v.is_subnormal()
        || v == 0.
            && raw
                .split(['e', 'E'])
                .next()
                .unwrap_or("")
                .bytes()
                .any(|b| (b'1'..=b'9').contains(&b))
    {
        return Err("Native number is outside its normal float32 range".into());
    }
    Ok(f64::from(v))
}
fn parse(block: &Block, index: usize) -> Result<Spec, String> {
    if !block.editable || block.fields.first().is_none_or(|f| f.key != "name") {
        return Err("Device block needs a first literal name and flat fields".into());
    }
    let mut s = Spec::new(&format!("legacy-device-{index}"), "");
    s.legacy = true;
    let fields = device::catalogue(BASELINE);
    let mut named = false;
    for field in &block.fields {
        let raw = unescape(&field.value)?;
        if field.key == "name" {
            if named {
                return Err("Device block needs one name".into());
            }
            s.name = raw;
            named = true;
            continue;
        }
        let key = gui_key(&field.key);
        let setting = fields
            .iter()
            .find(|s| s.lua_key == key && native_key(key) == field.key)
            .ok_or("Unknown native device field spelling")?;
        let value = match setting.kind.as_str() {
            "Bool" => match integer(&raw)? {
                0 => Value::Bool(false),
                1 => Value::Bool(true),
                n => Value::Int(n),
            },
            "Int" => Value::Int(integer(&raw)?),
            "Float" => Value::Float(floating(&raw)?),
            "String" => Value::String(raw),
            "Vec2" => {
                let (a, b) = raw
                    .split_once(' ')
                    .ok_or("Native vector needs two space-separated components")?;
                if b.contains(' ') {
                    return Err("Native vector has extra components".into());
                }
                Value::Vec2([floating(a)?, floating(b)?])
            }
            _ => return Err("Unknown native device field type".into()),
        };
        s.fields.insert(key.into(), value);
    }
    s.validate(BASELINE)?;
    Ok(s)
}
pub fn inspect(content: &str) -> Result<Report, String> {
    let mut r = Report {
        schema_sha256: schema_hash(),
        source_sha256: source_hash(content),
        devices: vec![],
        sites: vec![],
        unresolved: vec![],
        unresolved_count: 0,
    };
    let mut retained = 2usize;
    legacy_blocks::scan(content, "device", |block| {
        let mut spec = (r.devices.len() < device::MAX_DEVICES)
            .then(|| parse(&block, r.devices.len()).ok())
            .flatten();
        if let Some(s) = &spec {
            let n = serde_json::to_vec(s).map_or(MAX_DOCUMENT_BYTES + 1, |v| v.len() + 1);
            if retained.saturating_add(n) > MAX_DOCUMENT_BYTES {
                spec = None;
            } else {
                retained += n;
            }
        }
        if let Some(s) = spec {
            r.devices.push(s);
            r.sites.push(block.site);
        } else {
            r.unresolved_count += 1;
            if r.unresolved.len() < 4096 {
                r.unresolved.push(block.site.line);
            }
        }
    })?;
    device::validate(&r.devices, BASELINE)?;
    Ok(r)
}
fn value(v: &Value) -> Result<String, String> {
    literal(&match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => u8::from(*b).to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(n) => number(*n)?.to_string(),
        Value::Vec2(v) => format!("{} {}", number(v[0])?, number(v[1])?),
        _ => return Err("Unsupported native device value".into()),
    })
}
fn fields(s: &Spec) -> Result<BTreeMap<String, String>, String> {
    s.validate(BASELINE)?;
    if !s.legacy {
        return Err("Separate Lua device drafts require explicit native conversion".into());
    }
    let mut values = BTreeMap::from([("name".into(), literal(&s.name)?)]);
    for (key, v) in &s.fields {
        values.insert(native_key(key).into(), value(v)?);
    }
    Ok(values)
}
pub fn encode(s: &Spec) -> Result<String, String> {
    let mut data = fields(s)?;
    let name = data.remove("name").unwrap();
    let mut text = format!("device {{\n    name = {name}\n");
    for (key, value) in data {
        text.push_str(&format!("    {key} = {value}\n"));
    }
    text.push('}');
    Ok(text)
}
pub fn export(specs: &[Spec]) -> Result<String, String> {
    device::validate(specs, BASELINE)?;
    let mut out = String::new();
    for s in specs {
        out.push_str(&encode(s)?);
        out.push('\n');
        if out.len() > MAX_DOCUMENT_BYTES {
            return Err("Native device export exceeds 1 MiB".into());
        }
    }
    Ok(out)
}
pub fn edited_copy(content: &str, target: &[Spec]) -> Result<String, String> {
    device::validate(target, BASELINE)?;
    let before = inspect(content)?;
    if device::same_declarations(&before.devices, target) {
        return Ok(content.into());
    }
    let mut blocks = BTreeMap::new();
    legacy_blocks::scan(content, "device", |b| {
        if before
            .sites
            .binary_search_by_key(&b.site.statement.start, |s| s.statement.start)
            .is_ok()
        {
            blocks.insert(b.site.statement.start, b);
        }
    })?;
    let mut edits = vec![];
    for (i, site) in before.sites.iter().enumerate() {
        let block = blocks
            .get(&site.statement.start)
            .ok_or("Missing captured device block")?;
        if let Some(next) = target.get(i) {
            edits.extend(legacy_blocks::patches(
                content,
                block,
                &fields(&before.devices[i])?,
                &fields(next)?,
            )?);
        } else {
            edits.push((
                site.statement.clone(),
                legacy_blocks::comments(content, site.statement.clone()),
            ));
        }
    }
    edits.sort_by_key(|(span, _)| span.start);
    let mut out = content.to_owned();
    for (span, text) in edits.into_iter().rev() {
        out.replace_range(span, &text);
    }
    let nl = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    for s in target.iter().skip(before.sites.len()) {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(nl);
        }
        out.push_str(&encode(s)?);
        out.push_str(nl);
    }
    if out.len() > MAX_DOCUMENT_BYTES {
        return Err("Legacy device copy exceeds 1 MiB".into());
    }
    let after = inspect(&out)?;
    if export(&after.devices)? != export(target)?
        || before.unresolved_count != after.unresolved_count
    {
        return Err("Native device copy did not reproduce the draft".into());
    }
    Ok(out)
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-devices.json"))
}
