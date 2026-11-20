//! Graphical chord and dispatcher forms. Recording is focused and never opens /dev/input.
use super::{controls, label, monitors::choices};
use adw::prelude::*;
use hyprland_editor_core::{
    action::{Action, Field},
    binding::{Chord, MODIFIERS, SIDED},
    Bounds, Setting, Value, AUDITED_MAIN,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub(super) type ChordGetter = Box<dyn Fn() -> Result<Chord, String>>;
const KEYS: &[&str] = &[
    "Return",
    "Escape",
    "Tab",
    "space",
    "BackSpace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "Page_Up",
    "Page_Down",
    "Left",
    "Right",
    "Up",
    "Down",
    "plus",
    "minus",
    "equal",
    "comma",
    "period",
    "slash",
    "backslash",
    "semicolon",
    "apostrophe",
    "bracketleft",
    "bracketright",
    "grave",
    "XF86AudioMute",
    "XF86AudioLowerVolume",
    "XF86AudioRaiseVolume",
    "XF86AudioPlay",
    "XF86AudioPause",
    "XF86AudioStop",
    "XF86AudioNext",
    "XF86AudioPrev",
    "XF86MonBrightnessUp",
    "XF86MonBrightnessDown",
    "Print",
    "Pause",
    "Scroll_Lock",
    "Caps_Lock",
    "Num_Lock",
];
fn modifier(token: &str) -> Option<&'static str> {
    match token.to_ascii_uppercase().as_str() {
        "SHIFT" => Some("SHIFT"),
        "CAPS" => Some("CAPS"),
        "CTRL" | "CONTROL" => Some("CTRL"),
        "ALT" | "MOD1" => Some("ALT"),
        "MOD2" => Some("MOD2"),
        "MOD3" => Some("MOD3"),
        "SUPER" | "WIN" | "LOGO" | "MOD4" | "META" => Some("SUPER"),
        "MOD5" => Some("MOD5"),
        _ => None,
    }
}
struct TokenRow {
    root: gtk::Box,
    _update: Rc<dyn Fn()>,
    kind: gtk::DropDown,
    pick: gtk::DropDown,
    names: Vec<String>,
    wheel: gtk::DropDown,
    sided: gtk::DropDown,
    switch_mode: gtk::DropDown,
    switch_name: gtk::Entry,
    custom: gtk::Entry,
    code: gtk::SpinButton,
}
const WHEELS: &[&str] = &["mouse_down", "mouse_up", "mouse_left", "mouse_right"];
fn token(row: &TokenRow) -> Result<String, String> {
    Ok(match row.kind.selected() {
        0 => row
            .names
            .get(row.pick.selected() as usize)
            .cloned()
            .unwrap_or_else(|| row.custom.text().into()),
        1 | 2 => {
            row.code.update();
            format!(
                "{}:{}",
                if row.kind.selected() == 1 {
                    "code"
                } else {
                    "mouse"
                },
                row.code.value() as u32
            )
        }
        3 => WHEELS[row.wheel.selected() as usize].into(),
        4 => format!(
            "switch:{}{}",
            ["", "on:", "off:"][row.switch_mode.selected() as usize],
            row.switch_name.text()
        ),
        5 => SIDED[row.sided.selected() as usize].into(),
        _ => row.custom.text().into(),
    })
}
pub(super) fn chord(panel: &gtk::Box, initial: &Chord, version: &str) -> ChordGetter {
    chord_inner(panel, initial, version, false)
}
pub(super) fn legacy_chord(panel: &gtk::Box, initial: &Chord) -> ChordGetter {
    chord_inner(panel, initial, hyprland_editor_core::BASELINE, true)
}
fn chord_inner(
    panel: &gtk::Box,
    initial: &Chord,
    version: &str,
    native_catchall: bool,
) -> ChordGetter {
    let manual = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.append(&manual);
    let mods = Rc::new(
        MODIFIERS
            .iter()
            .map(|name| {
                let check = gtk::CheckButton::with_label(name);
                check.set_active(initial.tokens.iter().any(|t| modifier(t) == Some(*name)));
                manual.append(&check);
                (*name, check)
            })
            .collect::<Vec<_>>(),
    );
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 8);
    manual.append(&rows);
    let entries = Rc::new(RefCell::new(Vec::<TokenRow>::new()));
    let current = version == AUDITED_MAIN;
    let add: Rc<dyn Fn(&str)> = Rc::new({
        let rows = rows.downgrade();
        let entries = entries.clone();
        move |value| {
            let Some(rows) = rows.upgrade() else {
                return;
            };
            if entries.borrow().len() >= 32 {
                return;
            }
            let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let mouse_number = value
                .strip_prefix("mouse:")
                .and_then(|v| v.parse::<u32>().ok());
            let upper = value.to_ascii_uppercase();
            let kind = choices(
                &root,
                &[
                    "Key Symbol",
                    "XKB Keycode",
                    "Mouse Button",
                    "Wheel Direction",
                    "Switch Event",
                    "Sided Modifier (Audited Main)",
                    "Exact Event Token",
                ],
                if value.starts_with("code:") {
                    1
                } else if mouse_number.is_some_and(|n| value == format!("mouse:{n}")) {
                    2
                } else if WHEELS.contains(&value) {
                    3
                } else if value.starts_with("switch:") {
                    4
                } else if current && SIDED.contains(&upper.as_str()) {
                    5
                } else if value.starts_with("mouse:") {
                    6
                } else {
                    0
                },
            );
            kind.set_widget_name("bind-token-kind");
            let mut names = (b'A'..=b'Z')
                .map(|c| char::from(c).to_string())
                .chain((0..=9).map(|n| n.to_string()))
                .chain((1..=35).map(|n| format!("F{n}")))
                .chain(KEYS.iter().map(|s| s.to_string()))
                .collect::<Vec<_>>();
            if !names.iter().any(|n| n == value) && kind.selected() == 0 {
                names.push(value.into());
            }
            let mut captions = names.clone();
            captions.push("Another Key Symbol".into());
            let pick = choices(
                &root,
                &captions.iter().map(String::as_str).collect::<Vec<_>>(),
                names.iter().position(|n| n == value).unwrap_or(0) as u32,
            );
            pick.set_widget_name("bind-key-symbol");
            let wheel = choices(
                &root,
                WHEELS,
                WHEELS.iter().position(|n| *n == value).unwrap_or(0) as u32,
            );
            wheel.set_widget_name("bind-wheel");
            let sided = choices(
                &root,
                SIDED,
                SIDED.iter().position(|n| *n == upper).unwrap_or(0) as u32,
            );
            sided.set_sensitive(current);
            let switch = value.strip_prefix("switch:").unwrap_or("lid");
            let (mode, name) = if let Some(n) = switch.strip_prefix("on:") {
                (1, n)
            } else if let Some(n) = switch.strip_prefix("off:") {
                (2, n)
            } else {
                (0, switch)
            };
            let switch_mode = choices(&root, &["Either State", "On", "Off"], mode);
            let switch_name = gtk::Entry::builder()
                .text(name)
                .placeholder_text("Switch name, for example lid")
                .build();
            root.append(&switch_name);
            let custom = gtk::Entry::builder()
                .text(value)
                .placeholder_text("Key symbol or exact event token")
                .build();
            root.append(&custom);
            let code = gtk::SpinButton::with_range(0., f64::from(u32::MAX), 1.);
            code.set_digits(0);
            code.set_width_chars(10);
            code.set_value(
                value
                    .split_once(':')
                    .and_then(|(_, n)| n.parse::<f64>().ok())
                    .unwrap_or(30.),
            );
            root.append(&code);
            let update: Rc<dyn Fn()> = Rc::new({
                let kind = kind.clone();
                let pick = pick.clone();
                let wheel = wheel.clone();
                let sided = sided.clone();
                let switch_mode = switch_mode.clone();
                let switch_name = switch_name.clone();
                let custom = custom.clone();
                let code = code.clone();
                let count = names.len();
                move || {
                    let k = kind.selected();
                    pick.set_visible(k == 0);
                    wheel.set_visible(k == 3);
                    sided.set_visible(k == 5);
                    switch_mode.set_visible(k == 4);
                    switch_name.set_visible(k == 4);
                    custom.set_visible(k == 6 || (k == 0 && pick.selected() as usize >= count));
                    code.set_visible(k == 1 || k == 2);
                }
            });
            update();
            let u = Rc::downgrade(&update);
            kind.connect_selected_notify(move |_| {
                if let Some(u) = u.upgrade() {
                    u();
                }
            });
            let u = Rc::downgrade(&update);
            pick.connect_selected_notify(move |_| {
                if let Some(u) = u.upgrade() {
                    u();
                }
            });
            let remove = gtk::Button::with_label("Remove Key");
            root.append(&remove);
            rows.append(&root);
            entries.borrow_mut().push(TokenRow {
                root: root.clone(),
                _update: update,
                kind,
                pick,
                names,
                wheel,
                sided,
                switch_mode,
                switch_name,
                custom,
                code,
            });
            let row = root.downgrade();
            let rows = rows.downgrade();
            let entries = Rc::downgrade(&entries);
            remove.connect_clicked(move |_| {
                if let (Some(row), Some(rows), Some(entries)) =
                    (row.upgrade(), rows.upgrade(), entries.upgrade())
                {
                    entries.borrow_mut().retain(|e| e.root != row);
                    rows.remove(&row);
                }
            });
        }
    });
    for t in &initial.tokens {
        if modifier(t).is_none() && t != "catchall" {
            add(t);
        }
    }
    let append = gtk::Button::with_label("Add Key");
    manual.append(&append);
    let add_c = add.clone();
    append.connect_clicked(move |_| add_c("Return"));
    let catchall = gtk::CheckButton::with_label("Catch All (Submap Only)");
    catchall.set_active(if native_catchall {
        initial.tokens.iter().any(|t| t == "catchall")
    } else {
        initial.tokens == ["catchall"]
    });
    panel.append(&catchall);
    let mode = choices(
        panel,
        &["Manually Selected Chord", "Focused Keyboard Recording"],
        0,
    );
    let recorded = Rc::new(RefCell::new(None::<Chord>));
    let recording = Rc::new(Cell::new(false));
    let timer = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));
    let feedback=label("Recording captures key symbols only. Choose XKB keycodes manually; physical-device/keycode conversion is unverified.");
    panel.append(&feedback);
    let record = gtk::Button::with_label("Record Shortcut");
    record.set_widget_name("bind-record");
    panel.append(&record);
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let stop: Rc<dyn Fn()> = Rc::new({
        let recording = recording.clone();
        let timer = timer.clone();
        let record = record.downgrade();
        move || {
            recording.set(false);
            if let Some(id) = timer.borrow_mut().take() {
                id.remove();
            }
            if let Some(record) = record.upgrade() {
                record.set_label("Record Shortcut");
            }
        }
    });
    let start = recording.clone();
    let timer_c = timer.clone();
    let stop_c = stop.clone();
    let feedback_c = feedback.clone();
    let mode_c = mode.clone();
    let manual_c = manual.downgrade();
    let recorded_c = recorded.clone();
    record.connect_clicked(move|button|{stop_c();*recorded_c.borrow_mut()=None;start.set(true);button.set_label("Press Keys Here…");button.grab_focus();mode_c.set_selected(1);if let Some(m)=manual_c.upgrade(){m.set_sensitive(false);}feedback_c.set_text("Focused recording active. Escape cancels; keyboard events stop on blur or after 10 seconds.");let recording=start.clone();let timer=timer_c.clone();let weak=button.downgrade();let feedback=feedback_c.clone();let id=gtk::glib::timeout_add_local_once(std::time::Duration::from_secs(10),move||{recording.set(false);timer.borrow_mut().take();if let Some(b)=weak.upgrade(){b.set_label("Record Shortcut");feedback.set_text("Recording timed out. Select keys manually or record again.");}});*timer_c.borrow_mut()=Some(id);});
    let active = recording.clone();
    let result = recorded.clone();
    let feedback_c = feedback.clone();
    let stop_c = stop.clone();
    controller.connect_key_pressed(move |_, key, _, state| {
        if !active.get() {
            return gtk::glib::Propagation::Proceed;
        }
        let name = key.name().map(|s| s.to_string()).unwrap_or_default();
        if name == "Escape" {
            stop_c();
            feedback_c.set_text("Recording cancelled.");
            return gtk::glib::Propagation::Stop;
        }
        if [
            "Shift_L",
            "Shift_R",
            "Control_L",
            "Control_R",
            "Alt_L",
            "Alt_R",
            "Super_L",
            "Super_R",
            "Meta_L",
            "Meta_R",
            "Caps_Lock",
            "Num_Lock",
        ]
        .contains(&name.as_str())
        {
            return gtk::glib::Propagation::Stop;
        }
        let mut tokens = vec![];
        for (mask, name) in [
            (gtk::gdk::ModifierType::SHIFT_MASK, "SHIFT"),
            (gtk::gdk::ModifierType::CONTROL_MASK, "CTRL"),
            (gtk::gdk::ModifierType::ALT_MASK, "ALT"),
            (gtk::gdk::ModifierType::SUPER_MASK, "SUPER"),
            (gtk::gdk::ModifierType::META_MASK, "SUPER"),
        ] {
            if state.contains(mask) && !tokens.iter().any(|s| s == name) {
                tokens.push(name.into());
            }
        }
        tokens.push(name);
        let chord = Chord { tokens };
        feedback_c.set_text(&format!("Captured {}", chord.text()));
        *result.borrow_mut() = Some(chord);
        stop_c();
        gtk::glib::Propagation::Stop
    });
    record.add_controller(controller);
    let focus = gtk::EventControllerFocus::new();
    let stop_c = stop.clone();
    focus.connect_leave(move |_| stop_c());
    record.add_controller(focus);
    let stop_c = stop.clone();
    record.connect_unmap(move |_| stop_c());
    let m = manual.downgrade();
    let stop_c = stop.clone();
    mode.connect_selected_notify(move |p| {
        if p.selected() == 0 {
            stop_c();
        }
        if let Some(m) = m.upgrade() {
            m.set_sensitive(p.selected() == 0);
        }
    });
    let catchall_mods = mods.clone();
    let manual_get: Rc<dyn Fn() -> Result<Chord, String>> = Rc::new(move || {
        let mut tokens = mods
            .iter()
            .filter(|(_, w)| w.is_active())
            .map(|(name, _)| (*name).into())
            .collect::<Vec<_>>();
        tokens.extend(
            entries
                .borrow()
                .iter()
                .map(token)
                .collect::<Result<Vec<_>, _>>()?,
        );
        Ok(Chord { tokens })
    });
    let canonical_initial = manual_get().ok();
    let original = initial.clone();
    Box::new(move || {
        if catchall.is_active() {
            let mut tokens = if native_catchall {
                catchall_mods
                    .iter()
                    .filter(|(_, w)| w.is_active())
                    .map(|(name, _)| (*name).to_owned())
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            tokens.push("catchall".into());
            return Ok(Chord { tokens });
        }
        if mode.selected() == 1 {
            return recorded
                .borrow()
                .clone()
                .ok_or_else(|| "Record a chord or select manual mode".into());
        }
        let chord = manual_get()?;
        Ok(
            if original.tokens != ["catchall"] && canonical_initial.as_ref() == Some(&chord) {
                original.clone()
            } else {
                chord
            },
        )
    })
}

fn field_value(
    panel: &gtk::Box,
    field: &Field,
    initial: Option<Value>,
) -> Box<dyn Fn() -> Result<Value, String>> {
    if !field.choices.is_empty() {
        let text = match initial.as_ref() {
            Some(Value::String(s)) => s.as_str(),
            _ => "",
        };
        let picker = choices(
            panel,
            &field.choices.iter().map(String::as_str).collect::<Vec<_>>(),
            field.choices.iter().position(|s| s == text).unwrap_or(0) as u32,
        );
        let values = field.choices.clone();
        return Box::new(move || Ok(Value::String(values[picker.selected() as usize].clone())));
    }
    if field.kind == "DirectionOrBool" {
        let directions = ["left", "right", "up", "down", "l", "r", "u", "t", "d", "b"];
        let mut labels = vec!["No Direction (true)"];
        labels.extend(directions);
        let picker = choices(
            panel,
            &labels,
            match initial {
                Some(Value::String(ref s)) => {
                    directions.iter().position(|n| *n == s).unwrap_or(0) as u32 + 1
                }
                _ => 0,
            },
        );
        return Box::new(move || {
            Ok(if picker.selected() == 0 {
                Value::Bool(true)
            } else {
                Value::String(directions[picker.selected() as usize - 1].into())
            })
        });
    }
    // Preserve numeric type and exact finite values; generic bounded scalar spinners
    // would clamp dispatcher values and numeric selectors into a different action.
    if field.kind == "Number" || field.kind == "Selector" {
        let initial = initial.unwrap_or_else(|| field.initial());
        let type_picker = (field.kind == "Selector").then(|| {
            choices(
                panel,
                &["Selector Text", "Integer Selector", "Numeric Selector"],
                match initial {
                    Value::Int(_) => 1,
                    Value::Float(_) => 2,
                    _ => 0,
                },
            )
        });
        let text = match &initial {
            Value::Int(n) => n.to_string(),
            Value::Float(n) => n.to_string(),
            Value::String(s) => s.clone(),
            _ => String::new(),
        };
        let entry = gtk::Entry::builder().text(text).max_length(16384).build();
        entry.set_widget_name("action-number-or-selector");
        panel.append(&entry);
        return Box::new(move || {
            let text = entry.text();
            if type_picker.as_ref().is_some_and(|p| p.selected() == 0) {
                return Ok(Value::String(text.into()));
            }
            if type_picker.as_ref().is_some_and(|p| p.selected() == 1) {
                return text
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| "Enter a whole-number selector".into());
            }
            let n = text.parse::<f64>().map_err(|_| "Enter a finite number")?;
            if !n.is_finite() {
                return Err("Enter a finite number".into());
            }
            Ok(Value::Float(n))
        });
    }
    let kind = if field.kind == "Bool" {
        "Bool"
    } else {
        "String"
    };
    let setting = Setting {
        id: field.name.clone(),
        lua_key: field.name.clone(),
        version: String::new(),
        kind: kind.into(),
        default_expression: String::new(),
        bounds: Bounds {
            min: None,
            max: None,
        },
        source: String::new(),
    };
    controls::form(&setting, initial.or_else(|| Some(field.initial())), panel)
}
pub(super) fn action(
    panel: &gtk::Box,
    initial: &Action,
    version: &str,
) -> Box<dyn Fn() -> Result<Action, String>> {
    let definition = initial.definition(version).expect("validated action");
    let variants = definition.variants.clone();
    let variant = choices(
        panel,
        &variants
            .iter()
            .map(|v| v.label.as_str())
            .collect::<Vec<_>>(),
        variants
            .iter()
            .position(|v| v.id == initial.variant)
            .unwrap_or(0) as u32,
    );
    if variants.len() == 1 {
        variant.set_visible(false);
    }
    let advanced = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let at_load = gtk::CheckButton::with_label("Prepare Action When Configuration Loads");
    if let Some(label) = at_load.child().and_downcast::<gtk::Label>() {
        label.set_wrap(true);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    }
    at_load.set_active(initial.at_load);
    advanced.append(&at_load);
    advanced.append(&label("Otherwise the action is prepared when this block runs. Original timing is retained; editing never runs it."));
    panel.append(
        &gtk::Expander::builder()
            .label("Action Preparation")
            .child(&advanced)
            .build(),
    );
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.append(&body);
    let fields = Rc::new(RefCell::new(Vec::<(
        String,
        gtk::CheckButton,
        Box<dyn Fn() -> Result<Value, String>>,
    )>::new()));
    let refresh: Rc<dyn Fn()> = Rc::new({
        let body = body.clone();
        let fields = fields.clone();
        let variants = variants.clone();
        let variant = variant.clone();
        let initial = initial.clone();
        move || {
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            fields.borrow_mut().clear();
            let v = &variants[variant.selected() as usize];
            for field in &v.fields {
                let enabled = gtk::CheckButton::with_label(&format!(
                    "Set {}",
                    super::presentation::setting_name(&field.name)
                ));
                enabled.set_active(field.required || initial.fields.contains_key(&field.name));
                enabled.set_sensitive(!field.required);
                if field.required {
                    body.append(&label(&if field.name == "argument"
                        && ["exec_cmd", "exec_raw"].contains(&initial.constructor.as_str())
                    {
                        "Command".into()
                    } else {
                        super::presentation::setting_name(&field.name)
                    }));
                } else {
                    body.append(&enabled);
                }
                let form = gtk::Box::new(gtk::Orientation::Vertical, 6);
                form.set_sensitive(enabled.is_active());
                let get = field_value(&form, field, initial.fields.get(&field.name).cloned());
                body.append(&form);
                enabled.connect_toggled(move |w| form.set_sensitive(w.is_active()));
                fields.borrow_mut().push((field.name.clone(), enabled, get));
            }
        }
    });
    let r = Rc::downgrade(&refresh);
    variant.connect_selected_notify(move |_| {
        if let Some(r) = r.upgrade() {
            r();
        }
    });
    refresh();
    let key = initial.constructor.clone();
    let version = version.to_string();
    // Anchor refresh with the getter; callbacks retain only weak references.
    Box::new(move || {
        let _ = &refresh;
        let mut values = std::collections::BTreeMap::new();
        for (name, enabled, get) in fields.borrow().iter() {
            if enabled.is_active() {
                values.insert(name.clone(), get()?);
            }
        }
        let a = Action {
            constructor: key.clone(),
            variant: variants[variant.selected() as usize].id.clone(),
            at_load: at_load.is_active(),
            fields: values,
        };
        a.validate(&version)?;
        Ok(a)
    })
}

#[cfg(test)]
pub(super) fn test_field(
    panel: &gtk::Box,
    field: &Field,
    value: Value,
) -> Box<dyn Fn() -> Result<Value, String>> {
    field_value(panel, field, Some(value))
}

#[cfg(test)]
pub(super) fn test_recording() {
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
    for (version, text) in [
        (hyprland_editor_core::BASELINE, "CONTROL + code:030"),
        (hyprland_editor_core::BASELINE, "mouse:0272"),
        (hyprland_editor_core::AUDITED_MAIN, "shift_l + mouse_down"),
        (hyprland_editor_core::BASELINE, "switch:off:lid"),
    ] {
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let initial = Chord::parse(text);
        let get = chord(&panel, &initial, version);
        assert_eq!(get().unwrap(), initial);
        let weak = descendants(panel.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == "bind-token-kind")
            .unwrap()
            .downgrade();
        drop(get);
        drop(panel);
        assert!(
            weak.upgrade().is_none(),
            "Chord control callback cycle retained a removed form"
        );
    }
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let get = chord(
        &panel,
        &Chord::parse("SUPER + Q"),
        hyprland_editor_core::BASELINE,
    );
    let record = descendants(panel.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "bind-record")
        .unwrap()
        .downcast::<gtk::Button>()
        .unwrap();
    let controllers = record.observe_controllers();
    let key = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerKey>()
        })
        .unwrap();
    let focus = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerFocus>()
        })
        .unwrap();
    record.emit_clicked();
    assert!(get().is_err());
    key.emit_by_name::<bool>(
        "key-pressed",
        &[
            &gtk::gdk::Key::q,
            &30u32,
            &gtk::gdk::ModifierType::CONTROL_MASK,
        ],
    );
    assert_eq!(get().unwrap().text(), "CTRL + q");
    record.emit_clicked();
    assert!(
        get().is_err(),
        "Starting a new recording must discard the previous captured chord"
    );
    key.emit_by_name::<bool>(
        "key-pressed",
        &[
            &gtk::gdk::Key::Escape,
            &9u32,
            &gtk::gdk::ModifierType::empty(),
        ],
    );
    assert!(get().is_err());
    assert_eq!(record.label().as_deref(), Some("Record Shortcut"));
    record.emit_clicked();
    focus.emit_by_name::<()>("leave", &[]);
    key.emit_by_name::<bool>(
        "key-pressed",
        &[&gtk::gdk::Key::a, &38u32, &gtk::gdk::ModifierType::empty()],
    );
    assert!(get().is_err(), "Blur must disarm keyboard capture");
    record.emit_clicked();
    record.emit_by_name::<()>("unmap", &[]);
    assert_eq!(record.label().as_deref(), Some("Record Shortcut"));
}
