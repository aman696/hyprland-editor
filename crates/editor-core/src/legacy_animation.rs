//! Inert source projection for native animation/Bézier declarations.
use crate::{
    animation::{self, Curve, CurveKind, Declaration, Draft, Node, Operation},
    legacy_session::Site,
    legacy_syntax::{self, literal, trim, unescape},
    lua::source_hash,
    Value, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
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
fn text(v: &str) -> Result<(), String> {
    if v.len() > 16384 || v.contains('\0') {
        Err("Animation text exceeds its limit".into())
    } else {
        Ok(())
    }
}
fn finite(value: &Value) -> Result<f32, String> {
    let n = animation::number(value, false)? as f32;
    if !n.is_finite() || n.is_subnormal() || (n == 0. && animation::number(value, false)? != 0.) {
        return Err("Native animation number must be finite float32".into());
    }
    Ok(n)
}
pub fn validate_operation(op: &Operation) -> Result<(), String> {
    match op {
        Operation::Curve(c) => {
            text(&c.name)?;
            let CurveKind::Bezier { points } = &c.shape else {
                return Err("Native source has no spring declaration codec".into());
            };
            for v in points.iter().flatten() {
                finite(v)?;
            }
        }
        Operation::Node(n) => {
            if !animation::catalogue(BASELINE)
                .iter()
                .any(|s| s.name == n.leaf)
            {
                return Err("Unknown native animation branch".into());
            }
            if n.spring.is_some() {
                return Err("Native animation requires a Bézier reference".into());
            }
            for s in [&n.bezier, &n.style].into_iter().flatten() {
                text(s)?;
            }
            if animation::enabled(&n.enabled)? {
                let speed = finite(n.speed.as_ref().ok_or("Set native animation speed")?)?;
                if speed <= 0. {
                    return Err("Native animation speed must be positive".into());
                }
                if n.bezier.is_none() {
                    return Err("Choose a native Bézier".into());
                }
                if let Some(style) = &n.style {
                    animation::validate_style(&n.leaf, style)?;
                }
            } else if let Some(v) = &n.speed {
                match v {
                    Value::String(s) => text(s)?,
                    Value::Bool(_) | Value::Int(_) => {}
                    Value::Float(n) if n.is_finite() => {}
                    _ => return Err("Invalid retained disabled speed".into()),
                }
            }
        }
    }
    Ok(())
}
fn parse_number(s: &str) -> Result<Value, String> {
    if s.is_empty() || !s.as_bytes().last().is_some_and(u8::is_ascii_digit) {
        return Err("Unresolved native numeric spelling".into());
    }
    let mut decimal = false;
    for (at, b) in s.bytes().enumerate() {
        if b.is_ascii_digit() || at == 0 && b == b'-' {
            continue;
        }
        if b == b'.' && at > 0 && !decimal {
            decimal = true;
            continue;
        }
        return Err("Unresolved native numeric spelling".into());
    }
    let v: f32 = s.parse().map_err(|_| "Invalid native number")?;
    if !v.is_finite()
        || v.is_subnormal()
        || v == 0. && s.bytes().any(|b| (b'1'..=b'9').contains(&b))
    {
        return Err("Native number exceeds its normal float32 domain".into());
    }
    Ok(Value::Float(f64::from(v)))
}
fn decode(key: &str, value: &str, index: usize) -> Result<Declaration, String> {
    let value = unescape(value)?;
    let mut args: Vec<_> = value.split(',').map(trim).collect();
    while args.len() > 5 && args.last() == Some(&"") {
        args.pop();
    }
    if args.len() > 5 {
        return Err("Unrepresented native animation arguments".into());
    }
    let operation = if key == "bezier" {
        if args.len() != 5 {
            return Err("Bézier needs a name and four coordinates".into());
        }
        Operation::Curve(Curve {
            name: args[0].into(),
            shape: CurveKind::Bezier {
                points: [
                    [parse_number(args[1])?, parse_number(args[2])?],
                    [parse_number(args[3])?, parse_number(args[4])?],
                ],
            },
        })
    } else {
        if args.len() < 2 {
            return Err("Animation needs branch and enabled state".into());
        }
        let on = match args[1] {
            "true" | "yes" | "on" => true,
            "false" | "no" | "off" => false,
            s => s
                .parse::<i64>()
                .map(|n| n == 1)
                .map_err(|_| "Unresolved native enabled state")?,
        };
        let speed = args
            .get(2)
            .map(|s| {
                if on {
                    parse_number(s)
                } else {
                    Ok(Value::String((*s).into()))
                }
            })
            .transpose()?;
        Operation::Node(Node {
            leaf: args[0].into(),
            enabled: Value::Bool(on),
            speed,
            bezier: args.get(3).map(|s| (*s).into()),
            spring: None,
            style: args.get(4).map(|s| (*s).into()),
        })
    };
    validate_operation(&operation)?;
    Ok(Declaration {
        id: format!("legacy-animation-{index}"),
        legacy: true,
        operation,
    })
}
pub fn inspect(content: &str) -> Result<Report, String> {
    let mut report = Report {
        schema_sha256: schema_hash(),
        source_sha256: source_hash(content),
        draft: Draft::default(),
        sites: vec![],
        unresolved: vec![],
        unresolved_count: 0,
    };
    let mut curves: BTreeSet<String> = ["default".into(), "linear".into()].into();
    let mut retained = 32usize;
    legacy_syntax::walk_with_category(content, Some("animations"), |line| {
        if !["bezier", "animation"].contains(&line.key) {
            return;
        }
        let mut parsed = (line.editable
            && report.draft.declarations.len() < animation::MAX_OPERATIONS)
            .then(|| decode(line.key, line.value, report.draft.declarations.len()).ok())
            .flatten();
        if let Some(d) = &parsed {
            if let Operation::Node(n) = &d.operation {
                if animation::enabled(&n.enabled).unwrap_or(false)
                    && !n.bezier.as_ref().is_some_and(|s| curves.contains(s))
                {
                    parsed = None;
                }
            }
        }
        if let Some(d) = &parsed {
            let size = serde_json::to_vec(d).map_or(MAX_DOCUMENT_BYTES + 1, |v| v.len() + 1);
            if retained.saturating_add(size) > MAX_DOCUMENT_BYTES {
                parsed = None;
            } else {
                retained += size;
            }
        }
        if let Some(d) = parsed {
            if let Operation::Curve(c) = &d.operation {
                curves.insert(c.name.clone());
            }
            report.draft.declarations.push(d);
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
    report.draft.validate(BASELINE)?;
    Ok(report)
}
fn arg(v: &str) -> Result<String, String> {
    if v.contains(',') {
        return Err("Native animation arguments cannot contain commas".into());
    }
    literal(v)
}
pub fn encode(d: &Declaration) -> Result<String, String> {
    if !d.legacy {
        return Err("Separate Lua animations require explicit native conversion".into());
    }
    validate_operation(&d.operation)?;
    let (key, mut args) = match &d.operation {
        Operation::Curve(c) => {
            let CurveKind::Bezier { points } = &c.shape else {
                unreachable!()
            };
            let mut args = vec![c.name.clone()];
            for v in points.iter().flatten() {
                args.push(finite(v)?.to_string());
            }
            ("bezier", args)
        }
        Operation::Node(n) => {
            let on = animation::enabled(&n.enabled)?;
            let mut args = vec![n.leaf.clone(), u8::from(on).to_string()];
            if n.speed.is_some() || n.bezier.is_some() || n.style.is_some() {
                args.push(match &n.speed {
                    Some(v) if on => finite(v)?.to_string(),
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => v.lua(),
                    None => String::new(),
                });
            }
            if n.bezier.is_some() || n.style.is_some() {
                args.push(n.bezier.clone().unwrap_or_default());
            }
            if let Some(style) = &n.style {
                args.push(style.clone());
            }
            ("animation", args)
        }
    };
    for a in &mut args {
        *a = arg(a)?;
    }
    Ok(format!("{key} = {}", args.join(",")))
}
pub fn export(draft: &Draft) -> Result<String, String> {
    draft.validate(BASELINE)?;
    let mut out = String::new();
    for d in &draft.declarations {
        out.push_str(&encode(d)?);
        out.push('\n');
        if out.len() > MAX_DOCUMENT_BYTES {
            return Err("Animation export exceeds 1 MiB".into());
        }
    }
    Ok(out)
}
pub fn edited_copy(content: &str, target: &Draft) -> Result<String, String> {
    target.validate(BASELINE)?;
    let before = inspect(content)?;
    if before.draft.same_declarations(target) {
        return Ok(content.into());
    }
    let mut out = content.to_owned();
    for (i, site) in before.sites.iter().enumerate().rev() {
        let d = target.declarations.get(i);
        if d.is_some_and(
            |d| matches!((encode(d),encode(&before.draft.declarations[i])), (Ok(a),Ok(b)) if a==b),
        ) {
            continue;
        }
        let mut text = d.map(encode).transpose()?.unwrap_or_default();
        if !text.is_empty() && content.as_bytes().get(site.statement.end) == Some(&b'#') {
            text.push(' ');
        }
        out.replace_range(site.statement.clone(), &text);
    }
    let nl = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    for d in target.declarations.iter().skip(before.sites.len()) {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(nl);
        }
        out.push_str(&encode(d)?);
        out.push_str(nl);
    }
    if out.len() > MAX_DOCUMENT_BYTES {
        return Err("Legacy copy exceeds 1 MiB".into());
    }
    let after = inspect(&out)?;
    if export(&after.draft)? != export(target)? || after.unresolved_count != before.unresolved_count
    {
        return Err("Animation copy did not reproduce the native draft".into());
    }
    Ok(out)
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-animations.json"))
}
