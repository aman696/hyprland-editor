//! Exact native values complement guided controls without imposing Lua descriptor bounds.
use super::*;
fn number_entry(
    panel: &gtk::Box,
    title: &str,
    initial: &str,
    name: &str,
    integer: bool,
) -> gtk::Entry {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.append(&label(title));
    let input = gtk::Entry::builder()
        .text(initial)
        .max_length(128)
        .hexpand(true)
        .build();
    input.set_widget_name(name);
    input.update_property(&[gtk::accessible::Property::Label(title)]);
    row.append(&input);
    panel.append(&row);
    let steps = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    for (text, step) in [("−1", -1i64), ("+1", 1)] {
        let b = gtk::Button::with_label(text);
        steps.append(&b);
        let weak = input.downgrade();
        b.connect_clicked(move |_| {
            let Some(e) = weak.upgrade() else {
                return;
            };
            if integer {
                if let Ok(v) = e.text().parse::<i64>() {
                    if let Some(v) = v.checked_add(step) {
                        e.set_text(&v.to_string());
                    }
                }
            } else if let Ok(v) = e.text().parse::<f64>() {
                let v = v + step as f64;
                if v.is_finite() {
                    e.set_text(&v.to_string());
                }
            }
        });
    }
    panel.append(&steps);
    input
}
fn exact(
    panel: &gtk::Box,
    setting: &Setting,
    initial: Option<Value>,
) -> Box<dyn Fn() -> Result<Value, String>> {
    let original = initial.or_else(|| setting.known_default());
    match setting.kind.as_str() {
        "Bool" | "Int" => {
            let shown = match &original {
                Some(Value::Int(n)) => n.to_string(),
                Some(Value::Bool(b)) => u8::from(*b).to_string(),
                _ => "0".into(),
            };
            let e = number_entry(
                panel,
                "Native integer",
                &shown,
                "device-native-number",
                true,
            );
            Box::new(move || {
                if e.text() == shown {
                    if let Some(v) = &original {
                        return Ok(v.clone());
                    }
                }
                Ok(Value::Int(
                    e.text()
                        .parse()
                        .map_err(|_| "Enter a signed whole number")?,
                ))
            })
        }
        "Float" => {
            let shown = match &original {
                Some(Value::Float(n)) => (*n as f32).to_string(),
                _ => "0".into(),
            };
            let e = number_entry(
                panel,
                "Native number",
                &shown,
                "device-native-number",
                false,
            );
            Box::new(move || {
                if e.text() == shown {
                    if let Some(v) = &original {
                        return Ok(v.clone());
                    }
                }
                Ok(Value::Float(
                    e.text()
                        .parse()
                        .map_err(|_| "Enter a finite native number")?,
                ))
            })
        }
        "Vec2" => {
            let original = if let Some(Value::Vec2(v)) = original {
                v
            } else {
                [0., 0.]
            };
            let shown = original.map(|n| (n as f32).to_string());
            let x = number_entry(panel, "X coordinate", &shown[0], "device-native-x", false);
            let y = number_entry(panel, "Y coordinate", &shown[1], "device-native-y", false);
            Box::new(move || {
                let component = |e: &gtk::Entry, i: usize| {
                    if e.text() == shown[i] {
                        Ok(original[i])
                    } else {
                        e.text()
                            .parse::<f64>()
                            .map_err(|_| "Enter two finite coordinates".to_string())
                    }
                };
                Ok(Value::Vec2([component(&x, 0)?, component(&y, 1)?]))
            })
        }
        _ => {
            let text = if let Some(Value::String(s)) = &original {
                s.clone()
            } else {
                String::new()
            };
            let e = gtk::Entry::builder()
                .text(text)
                .max_length(16384)
                .hexpand(true)
                .build();
            e.set_widget_name("device-native-text");
            e.update_property(&[gtk::accessible::Property::Label("Native text value")]);
            panel.append(&e);
            Box::new(move || Ok(Value::String(e.text().into())))
        }
    }
}
pub(super) fn form(
    setting: &Setting,
    spec: &Spec,
    panel: &gtk::Box,
    window: &adw::Window,
    registry: Option<Arc<Registry>>,
) -> Getter {
    let key = setting.lua_key.clone();
    let mut probe = spec.clone();
    probe.legacy = false;
    probe.fields.retain(|k, _| {
        *k == key
            || ["kb_layout", "kb_variant"].contains(&key.as_str())
                && ["kb_layout", "kb_variant"].contains(&k.as_str())
    });
    let compatible = probe.validate(&setting.version).is_ok();
    let mut guided_spec = spec.clone();
    if !compatible {
        guided_spec.fields.remove(&key);
        if ["kb_layout", "kb_variant"].contains(&key.as_str()) {
            guided_spec.fields.remove("kb_layout");
            guided_spec.fields.remove("kb_variant");
        }
        if setting.kind == "Bool" {
            if let Some(Value::Int(n)) = spec.fields.get(&key) {
                guided_spec.fields.insert(key.clone(), Value::Bool(*n != 0));
            }
        }
    }
    let native = gtk::CheckButton::with_label("Edit native value");
    native.set_widget_name("device-native-value-mode");
    native.set_active(!compatible);
    panel.append(&native);
    if !compatible {
        panel.append(&label("This value is outside the guided controls. Keep its native value, or choose a guided replacement."));
    }
    let guided_body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let exact_body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.append(&guided_body);
    panel.append(&exact_body);
    let guided = super::guided(setting, &guided_spec, &guided_body, window, registry);
    let baseline = guided().ok().and_then(|v| serde_json::to_vec(&v).ok());
    let get_exact = exact(&exact_body, setting, spec.fields.get(&key).cloned());
    guided_body.set_visible(!native.is_active());
    exact_body.set_visible(native.is_active());
    let g = guided_body.downgrade();
    let e = exact_body.downgrade();
    native.connect_toggled(move |n| {
        if let Some(g) = g.upgrade() {
            g.set_visible(!n.is_active());
        }
        if let Some(e) = e.upgrade() {
            e.set_visible(n.is_active());
        }
    });
    let old = spec.fields.clone();
    let setting = setting.clone();
    Box::new(move || {
        if native.is_active() {
            let value = get_exact()?;
            hyprland_editor_core::legacy_device::validate_value(&setting, &value)?;
            return Ok(value_map(&key, value));
        }
        let mut values = guided()?;
        if compatible && baseline.is_some() && serde_json::to_vec(&values).ok() == baseline {
            values.retain(|name, _| name == &key || old.contains_key(name));
            for (name, value) in &mut values {
                if let Some(previous) = old.get(name) {
                    *value = previous.clone();
                }
            }
        }
        Ok(values)
    })
}
