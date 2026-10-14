//! Graphical authoring for encoded rule values. Untouched imported literals stay exact.
use super::{label, monitors::choices};
use adw::prelude::*;
use hyprland_editor_core::{
    rule::EffectValue,
    rule_codec::{self, Alpha, Border, Events, FullscreenState, Gradient, Group, Opacity},
    Value, BASELINE,
};
use std::{cell::RefCell, rc::Rc};
type StringGet = Box<dyn Fn() -> Result<String, String>>;
type EffectGet = Box<dyn Fn() -> Result<EffectValue, String>>;
fn entry(panel: &gtk::Box, title: &str, value: &str) -> gtk::Entry {
    panel.append(&label(title));
    let e = gtk::Entry::builder().text(value).max_length(64).build();
    e.update_property(&[gtk::accessible::Property::Label(title)]);
    panel.append(&e);
    e
}
/// The emitted shape only changes after controls change, or the user explicitly replaces an unknown literal.
fn retain(
    parent: &gtk::Box,
    builder: &gtk::Box,
    original: Option<String>,
    understood: bool,
    get: StringGet,
) -> StringGet {
    let shown = get().ok();
    let replace = if let Some(raw) = original.as_ref() {
        let original = label(raw);
        parent.append(
            &gtk::Expander::builder()
                .label("Original Value")
                .child(&original)
                .build(),
        );
        if !understood {
            let set = gtk::CheckButton::with_label("Replace Original With Controls");
            parent.append(&set);
            parent.append(&label(
                "This spelling is retained. Enable the controls to replace it with a new value.",
            ));
            let weak = builder.downgrade();
            set.connect_toggled(move |b| {
                if let Some(panel) = weak.upgrade() {
                    panel.set_visible(b.is_active());
                }
            });
            Some(set)
        } else {
            None
        }
    } else {
        None
    };
    builder.set_visible(replace.as_ref().is_none_or(gtk::CheckButton::is_active));
    parent.append(builder);
    Box::new(move || {
        if replace.as_ref().is_some_and(|b| !b.is_active()) {
            return Ok(original.clone().unwrap());
        }
        let value = get()?;
        if understood && shown.as_ref() == Some(&value) {
            if let Some(original) = &original {
                return Ok(original.clone());
            }
        }
        Ok(value)
    })
}
fn alpha(panel: &gtk::Box, title: &str, value: Alpha) -> Box<dyn Fn() -> Result<Alpha, String>> {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
    row.append(&label(title));
    let slider = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 1., 0.01);
    slider.set_value(f64::from(value.amount).clamp(0., 1.));
    slider.set_draw_value(false);
    slider.set_hexpand(true);
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    line.append(&slider);
    let e = gtk::Entry::builder()
        .text(value.amount.to_string())
        .max_length(64)
        .width_chars(10)
        .max_width_chars(10)
        .build();
    e.update_property(&[gtk::accessible::Property::Label(&format!(
        "{title} opacity amount"
    ))]);
    line.append(&e);
    let over = gtk::CheckButton::with_label("Override");
    over.set_active(value.override_value);
    line.append(&over);
    row.append(&line);
    panel.append(&row);
    let weak = e.downgrade();
    slider.connect_value_changed(move |s| {
        if let Some(e) = weak.upgrade() {
            e.set_text(&(s.value() as f32).to_string());
        }
    });
    Box::new(move || {
        let raw = e.text();
        let n = raw
            .parse::<f32>()
            .map_err(|_| "Enter a finite opacity amount")?;
        if !n.is_finite()
            || (n == 0.
                && raw
                    .split(['e', 'E'])
                    .next()
                    .unwrap_or("")
                    .bytes()
                    .any(|b| (b'1'..=b'9').contains(&b)))
        {
            return Err("Opacity is outside the finite float32 range".into());
        }
        Ok(Alpha {
            amount: n,
            override_value: over.is_active(),
        })
    })
}
fn opacity(parent: &gtk::Box, raw: Option<String>) -> StringGet {
    let parsed = raw.as_deref().map(Opacity::decode);
    let understood = parsed.as_ref().is_none_or(Result::is_ok);
    let value = parsed.and_then(Result::ok).unwrap_or_default();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&label("Set active, inactive and fullscreen opacity separately. Override replaces the inherited amount; otherwise it multiplies it."));
    let active = alpha(&body, "Active", value.active);
    let inactive = alpha(&body, "Inactive", value.inactive);
    let full = alpha(&body, "Fullscreen", value.fullscreen);
    retain(
        parent,
        &body,
        raw,
        understood,
        Box::new(move || {
            Opacity {
                active: active()?,
                inactive: inactive()?,
                fullscreen: full()?,
            }
            .encode()
        }),
    )
}
fn fullscreen(parent: &gtk::Box, raw: Option<String>) -> StringGet {
    let parsed = raw.as_deref().map(FullscreenState::decode);
    let understood = parsed.as_ref().is_none_or(Result::is_ok);
    let value = parsed.and_then(Result::ok).unwrap_or(FullscreenState {
        internal: 0,
        client: None,
    });
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let internal = entry(
        &body,
        "Internal Fullscreen State",
        &value.internal.to_string(),
    );
    let set = gtk::CheckButton::with_label("Set Client Fullscreen State");
    set.set_active(value.client.is_some());
    body.append(&set);
    let client = entry(
        &body,
        "Client Fullscreen State",
        &value.client.unwrap_or(0).to_string(),
    );
    client.set_sensitive(set.is_active());
    let weak = client.downgrade();
    set.connect_toggled(move |b| {
        if let Some(e) = weak.upgrade() {
            e.set_sensitive(b.is_active());
        }
    });
    retain(
        parent,
        &body,
        raw,
        understood,
        Box::new(move || {
            let internal = internal
                .text()
                .parse::<i32>()
                .map_err(|_| "Internal state needs a 32-bit whole number")?;
            let client = if set.is_active() {
                Some(
                    client
                        .text()
                        .parse::<i32>()
                        .map_err(|_| "Client state needs a 32-bit whole number")?,
                )
            } else {
                None
            };
            Ok(FullscreenState { internal, client }.encode())
        }),
    )
}
fn enum_value(parent: &gtk::Box, key: &str, raw: Option<String>, values: &[&str]) -> StringGet {
    let initial = raw.as_deref().unwrap_or(values[0]);
    let alias = match key {
        "content" => match initial {
            "0" => "none",
            "1" => "photo",
            "2" => "video",
            "3" => "game",
            n => n,
        },
        "tonemap" => match initial {
            "0" => "off",
            "1" => "on",
            n => n,
        },
        _ => initial,
    };
    let index = values.iter().position(|n| *n == alias);
    let understood = raw.is_none() || index.is_some();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let picker = choices(&body, values, index.unwrap_or(0) as u32);
    let values = values.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    retain(
        parent,
        &body,
        raw,
        understood,
        Box::new(move || {
            values
                .get(picker.selected() as usize)
                .cloned()
                .ok_or("Choose a supported mode".into())
        }),
    )
}
fn events(parent: &gtk::Box, raw: Option<String>) -> StringGet {
    let parsed = raw.as_deref().map(Events::decode);
    let understood = parsed.as_ref().is_none_or(Result::is_ok);
    let value = parsed
        .and_then(Result::ok)
        .unwrap_or(Events { items: vec![] });
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let mut controls = vec![];
    for (key, name) in rule_codec::SUPPRESS_EVENTS.iter().zip([
        "Fullscreen Requests",
        "Maximize Requests",
        "Activation Requests",
        "Focus-only Activation",
        "Fullscreen Output Requests",
        "X11 Configure Requests",
    ]) {
        let c = gtk::CheckButton::with_label(name);
        c.set_active(value.items.iter().any(|s| s == key));
        body.append(&c);
        controls.push((key.to_string(), c));
    }
    retain(
        parent,
        &body,
        raw,
        understood,
        Box::new(move || {
            Events {
                items: controls
                    .iter()
                    .filter(|(_, c)| c.is_active())
                    .map(|(name, _)| name.clone())
                    .collect(),
            }
            .encode()
        }),
    )
}
struct GroupRow {
    widget: gtk::Box,
    picker: gtk::DropDown,
    actions: Vec<String>,
}
fn group_row(parent: &gtk::Box, rows: &Rc<RefCell<Vec<GroupRow>>>, initial: &str) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let mut actions = [
        "set", "new", "lock", "invade", "barred", "deny", "override", "unset", "always", "group",
        "",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();
    let mut captions = [
        "Open / Join Group",
        "Open New Group",
        "Lock",
        "Enter Locked Group",
        "Prevent Automatic Join",
        "Deny Grouping",
        "Override Previous Rules",
        "Clear and Stop",
        "Always",
        "Legacy Group Marker",
        "Empty Separator Token",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();
    if !actions.iter().any(|s| s == initial) {
        actions.push(initial.into());
        captions.push(format!("Retained: {initial}"));
    }
    let picker = choices(
        &row,
        &captions.iter().map(String::as_str).collect::<Vec<_>>(),
        actions.iter().position(|s| s == initial).unwrap() as u32,
    );
    picker.set_hexpand(true);
    let up = gtk::Button::with_label("↑");
    let down = gtk::Button::with_label("↓");
    row.append(&up);
    row.append(&down);
    parent.append(&row);
    for (button, direction) in [(&up, -1), (&down, 1)] {
        let rows = Rc::downgrade(rows);
        let parent = parent.downgrade();
        let widget = row.downgrade();
        button.connect_clicked(move |_| {
            if let (Some(rows), Some(parent), Some(widget)) =
                (rows.upgrade(), parent.upgrade(), widget.upgrade())
            {
                let mut rows = rows.borrow_mut();
                if let Some(i) = rows.iter().position(|r| r.widget == widget) {
                    let next = i as isize + direction;
                    if next >= 0 && (next as usize) < rows.len() {
                        rows.swap(i, next as usize);
                        while let Some(child) = parent.first_child() {
                            parent.remove(&child);
                        }
                        for r in rows.iter() {
                            parent.append(&r.widget);
                        }
                    }
                }
            }
        });
    }
    rows.borrow_mut().push(GroupRow {
        widget: row,
        picker,
        actions,
    });
}
fn grouping(parent: &gtk::Box, raw: Option<String>) -> StringGet {
    let parsed = raw.as_deref().map(Group::decode);
    let understood = parsed.as_ref().is_none_or(Result::is_ok);
    let value = parsed
        .and_then(Result::ok)
        .unwrap_or(Group { actions: vec![] });
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.append(&label("Grouping operations are ordered. Always follows Set or Lock. Override resets earlier flags; Clear and Stop ignores following operations."));
    let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.append(
        &gtk::ScrolledWindow::builder()
            .child(&list)
            .max_content_height(220)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    let rows = Rc::new(RefCell::new(vec![]));
    for action in value.actions {
        group_row(&list, &rows, &action);
    }
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add Operation");
    let remove = gtk::Button::with_label("Remove Last Operation");
    buttons.append(&add);
    buttons.append(&remove);
    body.append(&buttons);
    let data = rows.clone();
    let box_ = list.clone();
    add.connect_clicked(move |_| {
        if data.borrow().len() < 32 {
            group_row(&box_, &data, "set");
        }
    });
    let data = rows.clone();
    remove.connect_clicked(move |_| {
        if let Some(row) = data.borrow_mut().pop() {
            list.remove(&row.widget);
        }
    });
    retain(
        parent,
        &body,
        raw,
        understood,
        Box::new(move || {
            let actions = rows
                .borrow()
                .iter()
                .map(|r| {
                    r.actions
                        .get(r.picker.selected() as usize)
                        .cloned()
                        .ok_or("Choose an operation".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            Group { actions }.encode()
        }),
    )
}
fn gradient(parent: &gtk::Box, value: Gradient) -> Box<dyn Fn() -> Result<Gradient, String>> {
    let colors = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let buttons = Rc::new(RefCell::new(vec![]));
    for color in value.colors {
        let button = super::controls::color_button(color);
        colors.append(&button);
        buttons.borrow_mut().push(button);
    }
    parent.append(
        &gtk::ScrolledWindow::builder()
            .child(&colors)
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .build(),
    );
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add Color");
    let remove = gtk::Button::with_label("Remove Last Color");
    actions.append(&add);
    actions.append(&remove);
    parent.append(&actions);
    let data = buttons.clone();
    let box_ = colors.clone();
    add.connect_clicked(move |_| {
        if data.borrow().len() < 10 {
            let button = super::controls::color_button(0xffffffff);
            box_.append(&button);
            data.borrow_mut().push(button);
        }
    });
    let data = buttons.clone();
    remove.connect_clicked(move |_| {
        if data.borrow().len() > 1 {
            let button = data.borrow_mut().pop().unwrap();
            colors.remove(&button);
        }
    });
    parent.append(&label("Direction in Whole Degrees"));
    let angle = gtk::SpinButton::with_range(f64::from(i32::MIN), f64::from(i32::MAX), 1.);
    angle.set_digits(0);
    angle.set_value(f64::from(value.degrees));
    angle.update_property(&[gtk::accessible::Property::Label(
        "Border direction in degrees",
    )]);
    parent.append(&angle);
    Box::new(move || {
        angle.update();
        let value = Gradient {
            colors: buttons
                .borrow()
                .iter()
                .map(super::controls::color_value)
                .collect(),
            degrees: angle.value() as i32,
        };
        value.validate()?;
        Ok(value)
    })
}
pub(super) fn border(parent: &gtk::Box, original: Option<EffectValue>, version: &str) -> EffectGet {
    let decoded = original.as_ref().and_then(|v| match v {
        EffectValue::BorderText(s) => Border::decode(s, version).ok(),
        EffectValue::Literal(Value::Gradient { colors, angle })
            if angle.is_finite()
                && angle.fract() == 0.
                && *angle >= f64::from(i32::MIN)
                && *angle <= f64::from(i32::MAX) =>
        {
            Some(Border {
                active: Some(Gradient {
                    colors: colors.clone(),
                    degrees: *angle as i32,
                }),
                inactive: None,
            })
        }
        _ => None,
    });
    let decoded = decoded.filter(|b| b.validate(version).is_ok());
    let understood = decoded.is_some() || original.is_none();
    let border = decoded.unwrap_or(Border {
        active: Some(Gradient {
            colors: vec![0xff000000],
            degrees: 0,
        }),
        inactive: None,
    });
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let active_set = gtk::CheckButton::with_label("Set Active Border");
    active_set.set_active(border.active.is_some());
    active_set.set_sensitive(version != BASELINE);
    body.append(&active_set);
    let active_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let active = gradient(
        &active_box,
        border.active.unwrap_or(Gradient {
            colors: vec![0xff000000],
            degrees: 0,
        }),
    );
    active_box.set_visible(active_set.is_active());
    body.append(&active_box);
    let weak = active_box.downgrade();
    active_set.connect_toggled(move |s| {
        if let Some(b) = weak.upgrade() {
            b.set_visible(s.is_active());
        }
    });
    let inactive_set = gtk::CheckButton::with_label("Set Inactive Border");
    inactive_set.set_active(border.inactive.is_some());
    body.append(&inactive_set);
    let inactive_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let inactive = gradient(
        &inactive_box,
        border.inactive.unwrap_or(Gradient {
            colors: vec![0xff000000],
            degrees: 0,
        }),
    );
    inactive_box.set_visible(inactive_set.is_active());
    body.append(&inactive_box);
    let weak = inactive_box.downgrade();
    inactive_set.connect_toggled(move |s| {
        if let Some(b) = weak.upgrade() {
            b.set_visible(s.is_active());
        }
    });
    let version = version.to_string();
    let get = move || {
        Border {
            active: if active_set.is_active() {
                Some(active()?)
            } else {
                None
            },
            inactive: if inactive_set.is_active() {
                Some(inactive()?)
            } else {
                None
            },
        }
        .encode(&version)
    };
    let shown = get().ok();
    let replace = if let Some(original) = original.as_ref() {
        parent.append(
            &gtk::Expander::builder()
                .label("Original Value")
                .child(&label(&original.lua()))
                .build(),
        );
        if !understood {
            let set = gtk::CheckButton::with_label("Replace Original With Gradient Controls");
            parent.append(&set);
            let weak = body.downgrade();
            set.connect_toggled(move |s| {
                if let Some(b) = weak.upgrade() {
                    b.set_visible(s.is_active());
                }
            });
            Some(set)
        } else {
            None
        }
    } else {
        None
    };
    body.set_visible(replace.as_ref().is_none_or(gtk::CheckButton::is_active));
    parent.append(&body);
    Box::new(move || {
        if replace.as_ref().is_some_and(|b| !b.is_active()) {
            return Ok(original.clone().unwrap());
        }
        let raw = get()?;
        if understood && shown.as_ref() == Some(&raw) {
            if let Some(original) = &original {
                return Ok(original.clone());
            }
        }
        Ok(EffectValue::BorderText(raw))
    })
}
pub(super) fn string(parent: &gtk::Box, key: &str, raw: Option<String>) -> Option<EffectGet> {
    let get = match key {
        "opacity" => opacity(parent, raw),
        "fullscreen_state" => fullscreen(parent, raw),
        "idle_inhibit" => enum_value(parent, key, raw, rule_codec::IDLE_MODES),
        "content" => enum_value(parent, key, raw, rule_codec::CONTENT_MODES),
        "tonemap" => enum_value(parent, key, raw, rule_codec::TONEMAP_MODES),
        "suppress_event" => events(parent, raw),
        "group" => grouping(parent, raw),
        _ => return None,
    };
    Some(Box::new(move || {
        get().map(|s| EffectValue::Literal(Value::String(s)))
    }))
}

#[cfg(test)]
pub(super) fn test_forms() {
    fn descendants(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut out = vec![];
        let mut child = w.first_child();
        while let Some(w) = child {
            out.push(w.clone());
            out.extend(descendants(&w));
            child = w.next_sibling();
        }
        out
    }
    for (key, raw) in [
        ("opacity", "opacity 0.5 override"),
        ("opacity", "unmodeled value"),
        ("fullscreen_state", "0x2 0x3 ignored"),
        ("fullscreen_state", "2  3"),
        ("idle_inhibit", "unknown-mode"),
        ("content", "3"),
        ("tonemap", "1"),
        ("suppress_event", "fullscreen fullscreen maximize"),
        ("group", "set always override lock always"),
    ] {
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let get = string(&panel, key, Some(raw.into())).unwrap();
        assert_eq!(
            get().unwrap(),
            EffectValue::Literal(Value::String(raw.into())),
            "{key}"
        );
        let weak = panel.downgrade();
        drop(get);
        drop(panel);
        assert!(weak.upgrade().is_none(), "{key} form leaked");
    }
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let get = string(&panel, "opacity", Some("0.5 override".into())).unwrap();
    let sliders = descendants(panel.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Scale>().ok())
        .collect::<Vec<_>>();
    assert_eq!(sliders.len(), 3);
    sliders[1].set_value(0.25);
    let EffectValue::Literal(Value::String(encoded)) = get().unwrap() else {
        panic!()
    };
    let decoded = Opacity::decode(&encoded).unwrap();
    assert_eq!(decoded.inactive.amount, 0.25);
    assert_eq!(decoded.active.amount, 0.5);
    assert!(decoded.inactive.override_value);
    drop(sliders);
    drop(get);
    drop(panel);
    for (version, raw) in [
        (BASELINE, "0xFF000000 45deg #fff 90deg"),
        (hyprland_editor_core::AUDITED_MAIN, "0deg 0xFFFFFFFF 90deg"),
    ] {
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let original = EffectValue::BorderText(raw.into());
        let get = border(&panel, Some(original.clone()), version);
        assert_eq!(get().unwrap(), original);
        let checks = descendants(panel.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::CheckButton>().ok())
            .collect::<Vec<_>>();
        let active = checks
            .iter()
            .find(|c| c.label().as_deref() == Some("Set Active Border"))
            .unwrap();
        assert_eq!(active.is_sensitive(), version != BASELINE);
        let angles = descendants(panel.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::SpinButton>().ok())
            .collect::<Vec<_>>();
        angles[1].set_value(120.);
        let EffectValue::BorderText(encoded) = get().unwrap() else {
            panic!()
        };
        assert_eq!(
            Border::decode(&encoded, version)
                .unwrap()
                .inactive
                .unwrap()
                .degrees,
            120
        );
        drop(angles);
        drop(checks);
        let weak = panel.downgrade();
        drop(get);
        drop(panel);
        assert!(weak.upgrade().is_none(), "Border form leaked");
    }
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let get = string(&panel, "group", Some("set lock".into())).unwrap();
    let buttons = descendants(panel.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .filter(|b| b.label().as_deref() == Some("↓"))
        .collect::<Vec<_>>();
    buttons[0].emit_clicked();
    assert_eq!(
        get().unwrap(),
        EffectValue::Literal(Value::String("lock set".into()))
    );
    drop(buttons);
    let weak = panel.downgrade();
    drop(get);
    drop(panel);
    assert!(weak.upgrade().is_none(), "Group form leaked");
}
