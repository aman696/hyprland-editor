//! Ordered native gesture source. Dispatcher text is never executed.
use crate::{
    gesture::{self, Direction, Draft, GestureAction, Spec},
    legacy_session::Site,
    legacy_syntax::{self, csv, literal, unescape},
    lua::source_hash,
    Value, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
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
fn text(s: &str) -> Result<(), String> {
    if s.len() > 16384 || s.contains('\0') {
        Err("Native gesture text exceeds its limit".into())
    } else {
        Ok(())
    }
}
fn number(v: &Value) -> Result<f32, String> {
    let raw = crate::animation::number(v, false)?;
    let n = raw as f32;
    if !n.is_finite() || n.is_subnormal() || n == 0. && raw != 0. {
        return Err("Native gesture scale exceeds the normal float32 domain".into());
    }
    Ok(n)
}
fn string(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => {
            text(s)?;
            Ok(s.clone())
        }
        Value::Int(n) => Ok(n.to_string()),
        Value::Float(n) if n.is_finite() => Ok(n.to_string()),
        _ => Err("Native gesture parameter needs literal text or number".into()),
    }
}
pub fn validate_spec(s: &Spec, version: &str) -> Result<(), String> {
    if !s.legacy || version != BASELINE {
        return Err("Native gestures require the baseline and native provenance".into());
    }
    if !matches!(s.fingers, Value::Int(2..=9)) {
        return Err("Gesture fingers must be 2..9".into());
    }
    Direction::parse(&s.direction)?;
    text(&s.direction)?;
    gesture::modifier_mask(s.mods.as_ref())?;
    if let Some(v) = &s.mods {
        let Value::String(v) = v else {
            return Err("Native modifiers need literal names".into());
        };
        text(v)?;
    }
    if let Some(v) = &s.scale {
        number(v)?;
    }
    if let Some(v) = &s.disable_inhibit {
        crate::animation::enabled(v)?;
    }
    for v in [&s.mode, &s.zoom_level, &s.workspace_name]
        .into_iter()
        .flatten()
    {
        string(v)?;
    }
    match &s.action {
        GestureAction::Builtin(name) if gesture::BUILTINS.contains(&name.as_str()) => {}
        GestureAction::NativeDispatcher { name, argument } => {
            text(name)?;
            text(argument)?;
        }
        _ => {
            return Err(
                "Native gesture source cannot implicitly convert Lua callback workflows".into(),
            )
        }
    }
    Ok(())
}
fn decode(key: &str, value: &str, index: usize) -> Result<Spec, String> {
    let flags = key
        .strip_prefix("gesture")
        .ok_or("Unknown gesture keyword")?;
    if !flags.chars().all(|c| c == 'p') {
        return Err("Unsupported gesture flag".into());
    }
    let data = csv(&unescape(value)?);
    let get = |at: usize| data.get(at).map(String::as_str).unwrap_or("");
    let mut s = Spec::new(&format!("legacy-gesture-{index}"));
    s.legacy = true;
    s.fingers = Value::Int(get(0).parse().map_err(|_| "Invalid native fingers")?);
    s.direction = get(1).into();
    s.disable_inhibit = (!flags.is_empty()).then_some(Value::Bool(true));
    let mut at = 2;
    loop {
        if let Some(raw) = get(at).strip_prefix("mod:") {
            s.mods = Some(Value::String(raw.into()));
            at += 1;
        } else if let Some(raw) = get(at).strip_prefix("scale:") {
            let n: f32 = raw
                .parse()
                .map_err(|_| "Unresolved native scale spelling")?;
            if !n.is_finite()
                || n.is_subnormal()
                || n == 0.
                    && raw
                        .split(['e', 'E'])
                        .next()
                        .unwrap_or("")
                        .bytes()
                        .any(|b| (b'1'..=b'9').contains(&b))
            {
                return Err("Unresolved native scale range".into());
            }
            s.scale = Some(Value::Float(f64::from(n)));
            at += 1;
        } else {
            break;
        }
    }
    let action = get(at);
    let count = match action {
        "dispatcher" => {
            s.action = GestureAction::NativeDispatcher {
                name: get(at + 1).into(),
                argument: data
                    .iter()
                    .skip(at + 2)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(","),
            };
            data.len()
        }
        "workspace" | "resize" | "move" | "close" | "unset" => {
            s.action = GestureAction::Builtin(action.into());
            at + 1
        }
        "special" => {
            s.action = GestureAction::Builtin(action.into());
            if data.len() > at + 1 {
                s.workspace_name = Some(Value::String(get(at + 1).into()));
            }
            at + 2
        }
        "float" | "fullscreen" => {
            s.action = GestureAction::Builtin(action.into());
            if data.len() > at + 1 {
                s.mode = Some(Value::String(get(at + 1).into()));
            }
            at + 2
        }
        "cursorZoom" => {
            s.action = GestureAction::Builtin(action.into());
            if data.len() > at + 1 {
                s.zoom_level = Some(Value::String(get(at + 1).into()));
            }
            if data.len() > at + 2 {
                s.mode = Some(Value::String(get(at + 2).into()));
            }
            at + 3
        }
        "scrollMove" => {
            s.action = GestureAction::Builtin("scroll_move".into());
            at + 1
        }
        _ => return Err("Unknown native gesture action".into()),
    };
    if data.iter().skip(count).any(|s| !s.is_empty()) {
        return Err("Unrepresented native action arguments".into());
    }
    s.validate(BASELINE)?;
    Ok(s)
}
pub fn inspect(content: &str) -> Result<Report, String> {
    let mut r = Report {
        schema_sha256: schema_hash(),
        source_sha256: source_hash(content),
        draft: Draft::default(),
        sites: vec![],
        unresolved: vec![],
        unresolved_count: 0,
    };
    let mut bytes = 32usize;
    legacy_syntax::walk_with_category(content, Some("gestures"), |line| {
        if !line.key.starts_with("gesture") || line.key.contains(':') {
            return;
        }
        let mut d = (line.editable && r.draft.declarations.len() < gesture::MAX_GESTURES)
            .then(|| decode(line.key, line.value, r.draft.declarations.len()).ok())
            .flatten();
        if let Some(s) = &d {
            let size = serde_json::to_vec(s).map_or(MAX_DOCUMENT_BYTES + 1, |v| v.len() + 1);
            if bytes.saturating_add(size) > MAX_DOCUMENT_BYTES {
                d = None;
            } else {
                bytes += size;
            }
        }
        if let Some(d) = d {
            r.draft.declarations.push(d);
            r.sites.push(Site {
                line: line.line,
                statement: line.statement,
            });
        } else {
            r.unresolved_count += 1;
            if r.unresolved.len() < 4096 {
                r.unresolved.push(line.line);
            }
        }
    })?;
    r.draft.validate(BASELINE)?;
    Ok(r)
}
fn arg(v: &str) -> Result<String, String> {
    if v.contains("\\,") {
        return Err("Backslash-comma gesture arguments require explicit conversion".into());
    }
    let mut v = v.replace(',', "\\,");
    if v.ends_with('\\') {
        v.push('\\');
    }
    literal(&v)
}
pub fn encode(s: &Spec) -> Result<String, String> {
    s.validate(BASELINE)?;
    if !s.legacy {
        return Err("Separate Lua gestures require explicit native conversion".into());
    }
    let mut data = vec![
        match &s.fingers {
            Value::Int(n) => n.to_string(),
            _ => unreachable!(),
        },
        s.direction.clone(),
    ];
    if let Some(Value::String(m)) = &s.mods {
        data.push(format!("mod:{m}"));
    }
    if let Some(n) = &s.scale {
        data.push(format!("scale:{}", number(n)?));
    }
    let (action, extra) = match &s.action {
        GestureAction::NativeDispatcher { name, argument } => {
            data.push("dispatcher".into());
            data.push(name.clone());
            let mut prefix = format!(
                "gesture{} = {}",
                if s.disable_inhibit
                    .as_ref()
                    .map(crate::animation::enabled)
                    .transpose()?
                    .unwrap_or(false)
                {
                    "p"
                } else {
                    ""
                },
                data.iter()
                    .map(|v| arg(v))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(",")
            );
            if !argument.is_empty() {
                prefix.push(',');
                prefix.push_str(&arg(argument)?);
            }
            return Ok(prefix);
        }
        GestureAction::Builtin(name) => {
            let mut args = vec![];
            let value = |v: &Option<Value>| {
                v.as_ref()
                    .map(string)
                    .transpose()
                    .map(|s| s.unwrap_or_default())
            };
            match name.as_str() {
                "special" => {
                    if s.workspace_name.is_some() {
                        args.push(value(&s.workspace_name)?);
                    }
                }
                "float" | "fullscreen" => {
                    if s.mode.is_some() {
                        args.push(value(&s.mode)?);
                    }
                }
                "cursorZoom" | "cursor_zoom" => {
                    if s.zoom_level.is_some() || s.mode.is_some() {
                        args.push(value(&s.zoom_level)?);
                    }
                    if s.mode.is_some() {
                        args.push(value(&s.mode)?);
                    }
                }
                _ => {}
            }
            (
                match name.as_str() {
                    "cursor_zoom" => "cursorZoom",
                    "scroll_move" => "scrollMove",
                    n => n,
                }
                .to_owned(),
                args,
            )
        }
        _ => return Err("Native gestures cannot contain Lua callback blocks".into()),
    };
    data.push(action);
    data.extend(extra);
    Ok(format!(
        "gesture{} = {}",
        if s.disable_inhibit
            .as_ref()
            .map(crate::animation::enabled)
            .transpose()?
            .unwrap_or(false)
        {
            "p"
        } else {
            ""
        },
        data.iter()
            .map(|v| arg(v))
            .collect::<Result<Vec<_>, _>>()?
            .join(",")
    ))
}
pub fn export(d: &Draft) -> Result<String, String> {
    d.validate(BASELINE)?;
    let mut out = String::new();
    for s in &d.declarations {
        out.push_str(&encode(s)?);
        out.push('\n');
        if out.len() > MAX_DOCUMENT_BYTES {
            return Err("Native gesture export exceeds 1 MiB".into());
        }
    }
    Ok(out)
}
pub fn edited_copy(content: &str, target: &Draft) -> Result<String, String> {
    target.validate(BASELINE)?;
    let before = inspect(content)?;
    if before.draft.same_declarations(target, BASELINE) {
        return Ok(content.into());
    }
    let mut out = content.to_owned();
    for (i, site) in before.sites.iter().enumerate().rev() {
        let next = target.declarations.get(i);
        if next.is_some_and(
            |d| matches!((encode(d),encode(&before.draft.declarations[i])),(Ok(a),Ok(b)) if a==b),
        ) {
            continue;
        }
        let mut text = next.map(encode).transpose()?.unwrap_or_default();
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
    for s in target.declarations.iter().skip(before.sites.len()) {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(nl);
        }
        out.push_str(&encode(s)?);
        out.push_str(nl);
    }
    if out.len() > MAX_DOCUMENT_BYTES {
        return Err("Native source copy exceeds 1 MiB".into());
    }
    let after = inspect(&out)?;
    if export(&after.draft)? != export(target)? || after.unresolved_count != before.unresolved_count
    {
        return Err("Native gesture copy did not reproduce the draft".into());
    }
    Ok(out)
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-gestures.json"))
}
