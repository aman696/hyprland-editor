//! Native string values with optional guided replacements; unchanged spelling is retained.
use super::*;
fn spelling(value: EffectValue) -> Result<String, String> {
    let scalar = |v: Value| -> Result<String, String> {
        match v {
            Value::String(s) => Ok(s),
            Value::Bool(v) => Ok(u8::from(v).to_string()),
            Value::Int(n) => Ok(n.to_string()),
            Value::Float(n) if n.is_finite() => Ok(n.to_string()),
            _ => Err("Native value needs literal text".into()),
        }
    };
    match value {
        EffectValue::Literal(v) => scalar(v),
        EffectValue::BorderText(s) | EffectValue::Geometry(Geometry::Text(s)) => Ok(s),
        EffectValue::Geometry(Geometry::Axes([x, y])) => {
            Ok(format!("{} {}", scalar(x)?, scalar(y)?))
        }
    }
}
fn typed(setting: &Setting, raw: &str) -> Option<EffectValue> {
    Some(match setting.kind.as_str() {
        "Bool" => EffectValue::Literal(Value::Bool(match raw {
            "true" | "yes" | "on" | "1" => true,
            "false" | "no" | "off" | "0" => false,
            _ => return None,
        })),
        "Int" => EffectValue::Literal(Value::Int(raw.parse().ok()?)),
        "Float" => EffectValue::Literal(Value::Float(raw.parse().ok()?)),
        "ExpressionVec2" => EffectValue::Geometry(Geometry::Text(raw.into())),
        _ if setting.lua_key == "border_color" => EffectValue::BorderText(raw.into()),
        _ => EffectValue::Literal(Value::String(raw.into())),
    })
}
pub(super) fn effect(
    panel: &gtk::Box,
    setting: &Setting,
    value: Option<EffectValue>,
    version: &str,
) -> ValueGetter {
    let raw = match value {
        Some(EffectValue::Literal(Value::String(s))) => Some(s),
        _ => None,
    };
    let converted = raw.as_deref().and_then(|s| typed(setting, s));
    let compatible = converted.as_ref().is_none_or(|v| {
        Effect {
            key: setting.lua_key.clone(),
            value: v.clone(),
        }
        .validate(Kind::Window, version)
        .is_ok()
            || Effect {
                key: setting.lua_key.clone(),
                value: v.clone(),
            }
            .validate(Kind::Layer, version)
            .is_ok()
    }) && (raw.is_none() || converted.is_some());
    let native = gtk::CheckButton::with_label("Edit Native Text");
    native.set_widget_name("rule-native-text-mode");
    native.set_active(!compatible);
    panel.append(&native);
    let guided = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let exact = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.append(&guided);
    panel.append(&exact);
    let getter = super::effect_form(
        &guided,
        setting,
        if compatible { converted } else { None },
        version,
    );
    let baseline = getter().ok();
    let input = text(&exact, "Native effect value", raw.as_deref().unwrap_or(""));
    input.set_widget_name("rule-native-text");
    exact.append(&label(
        "Stored as native text; runtime effect acceptance and matching are unverified.",
    ));
    guided.set_visible(!native.is_active());
    exact.set_visible(native.is_active());
    let g = guided.downgrade();
    let e = exact.downgrade();
    native.connect_toggled(move |n| {
        if let Some(g) = g.upgrade() {
            g.set_visible(!n.is_active());
        }
        if let Some(e) = e.upgrade() {
            e.set_visible(n.is_active());
        }
    });
    Box::new(move || {
        let out = if native.is_active() {
            input.text().to_string()
        } else {
            let v = getter()?;
            if Some(&v) == baseline.as_ref() {
                if let Some(raw) = &raw {
                    return Ok(EffectValue::Literal(Value::String(raw.clone())));
                }
            }
            spelling(v)?
        };
        Ok(EffectValue::Literal(Value::String(out)))
    })
}
pub(super) fn predicate(
    panel: &gtk::Box,
    kind: Kind,
    key: &str,
    value: Option<Value>,
) -> ValueGetter {
    if rule::match_kind(kind, key).is_none() {
        panel.append(&label("Registered native condition; this rule kind does not consume it in the modeled matcher."));
        let raw = if let Some(Value::String(s)) = value {
            s
        } else {
            String::new()
        };
        let input = text(panel, "Native condition value", &raw);
        return Box::new(move || Ok(EffectValue::Literal(Value::String(input.text().into()))));
    }
    let old = value.clone();
    let converted = match (rule::match_kind(kind, key), &value) {
        (Some(MatchKind::Bool), Some(Value::String(s))) => match s.as_str() {
            "1" | "true" | "yes" | "on" => Some(Value::Bool(true)),
            "0" | "false" | "no" | "off" => Some(Value::Bool(false)),
            _ => value,
        },
        _ => value,
    };
    let get = super::predicate_form(panel, kind, key, converted);
    let baseline = get().ok();
    Box::new(move || {
        let v = get()?;
        if Some(&v) == baseline.as_ref() {
            if let Some(Value::String(s)) = &old {
                return Ok(EffectValue::Literal(Value::String(s.clone())));
            }
        }
        Ok(EffectValue::Literal(Value::String(spelling(
            EffectValue::Literal(v),
        )?)))
    })
}
