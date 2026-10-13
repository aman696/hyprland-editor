//! Focused monitor draft editor. Hardware and original configuration are never mutated.
use super::{controls, label, presentation, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    monitor::{self, Mode, Modeline, Output, Position, Rule},
    Setting, Value,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn spin(value: f64, min: f64, max: f64, digits: u32) -> gtk::SpinButton {
    let w = gtk::SpinButton::with_range(min, max, if digits == 0 { 1. } else { 0.01 });
    w.set_digits(digits);
    w.set_value(value);
    w.set_width_chars(8);
    w
}
fn number(
    panel: &gtk::Box,
    name: &str,
    value: f64,
    min: f64,
    max: f64,
    digits: u32,
) -> gtk::SpinButton {
    panel.append(&label(name));
    let widget = spin(value, min, max, digits);
    widget.update_property(&[gtk::accessible::Property::Label(name)]);
    panel.append(&widget);
    widget
}
fn compact(picker: &gtk::DropDown) {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        item.set_child(Some(
            &gtk::Label::builder()
                .xalign(0.)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .max_width_chars(24)
                .build(),
        ));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        if let (Some(text), Some(label)) = (
            item.item().and_downcast::<gtk::StringObject>(),
            item.child().and_downcast::<gtk::Label>(),
        ) {
            label.set_text(&text.string());
        }
    });
    picker.set_factory(Some(&factory));
}
fn choices(panel: &gtk::Box, names: &[&str], index: u32) -> gtk::DropDown {
    let picker = gtk::DropDown::from_strings(names);
    compact(&picker);
    picker.set_selected(index);
    panel.append(&picker);
    picker
}
fn string_choice(
    panel: &gtk::Box,
    values: &[&'static str],
    text: &str,
) -> Box<dyn Fn() -> Result<Value, String>> {
    let picker = choices(
        panel,
        values,
        values.iter().position(|v| *v == text).unwrap_or(0) as u32,
    );
    let values = values.to_vec();
    Box::new(move || Ok(Value::String(values[picker.selected() as usize].into())))
}
fn builder(
    setting: &Setting,
    value: Option<Value>,
    panel: &gtk::Box,
    window: &adw::Window,
    outputs: &[Output],
) -> Box<dyn Fn() -> Result<Value, String>> {
    let initial = value.clone().or_else(|| setting.known_default());
    let string = match &initial {
        Some(Value::String(s)) => s.as_str(),
        _ => "",
    };
    match setting.lua_key.as_str() {
        "mode" => {
            let mode = Mode::parse(string).unwrap_or(Mode::Automatic("preferred".into()));
            let index = match &mode {
                Mode::Automatic(s) => monitor::MODES.iter().position(|v| *v == s).unwrap_or(0),
                Mode::Dimensions { .. } => 4,
                Mode::Modeline(_) => 5,
            };
            let picker = choices(
                panel,
                &[
                    "Preferred",
                    "Highest Refresh",
                    "Highest Resolution",
                    "Maximum Width",
                    "Resolution and Refresh",
                    "Custom Modeline",
                ],
                index as u32,
            );
            let dimensions = gtk::Box::new(gtk::Orientation::Vertical, 8);
            let timing = gtk::Box::new(gtk::Orientation::Vertical, 8);
            panel.append(&dimensions);
            panel.append(&timing);
            let (w, h, rate) = match mode {
                Mode::Dimensions {
                    width,
                    height,
                    refresh,
                } => (f64::from(width), f64::from(height), refresh),
                _ => (1920., 1080., Some(60.)),
            };
            let width = number(&dimensions, "Width (pixels)", w, 1., f64::from(i32::MAX), 0);
            let height = number(
                &dimensions,
                "Height (pixels)",
                h,
                1.,
                f64::from(i32::MAX),
                0,
            );
            let use_rate = gtk::CheckButton::with_label("Set Refresh Rate");
            use_rate.set_active(rate.is_some());
            dimensions.append(&use_rate);
            let refresh = number(
                &dimensions,
                "Refresh Rate (Hz)",
                rate.unwrap_or(60.),
                0.001,
                10000.,
                3,
            );
            let m = if let Mode::Modeline(m) = mode {
                m
            } else {
                Modeline {
                    clock_mhz: 148.5,
                    horizontal: [1920, 2008, 2052, 2200],
                    vertical: [1080, 1084, 1089, 1125],
                    hsync: 1,
                    vsync: 1,
                }
            };
            let clock = number(
                &timing,
                "Pixel Clock (MHz)",
                m.clock_mhz,
                0.001,
                f64::from(u32::MAX) / 1000.,
                3,
            );
            let mut counters = vec![];
            for (name, value) in [
                "Horizontal Display",
                "Horizontal Sync Start",
                "Horizontal Sync End",
                "Horizontal Total",
                "Vertical Display",
                "Vertical Sync Start",
                "Vertical Sync End",
                "Vertical Total",
            ]
            .iter()
            .zip(m.horizontal.iter().chain(&m.vertical))
            {
                counters.push(number(&timing, name, f64::from(*value), 1., 65535., 0));
            }
            let hs = choices(
                &timing,
                &["HSync Unspecified", "HSync Positive", "HSync Negative"],
                if m.hsync == 1 {
                    1
                } else if m.hsync == -1 {
                    2
                } else {
                    0
                },
            );
            let vs = choices(
                &timing,
                &["VSync Unspecified", "VSync Positive", "VSync Negative"],
                if m.vsync == 1 {
                    1
                } else if m.vsync == -1 {
                    2
                } else {
                    0
                },
            );
            timing.append(&label("The audited parser does not recognize its Interlace flag. Pixel-clock rounding and hardware timing need runtime certification."));
            dimensions.set_visible(index == 4);
            timing.set_visible(index == 5);
            picker.connect_selected_notify(move |p| {
                dimensions.set_visible(p.selected() == 4);
                timing.set_visible(p.selected() == 5);
            });
            Box::new(move || {
                for field in [&width, &height, &refresh, &clock] {
                    field.update();
                }
                for field in &counters {
                    field.update();
                }
                let mode = match picker.selected() {
                    0..=3 => Mode::Automatic(monitor::MODES[picker.selected() as usize].into()),
                    4 => Mode::Dimensions {
                        width: width.value() as u32,
                        height: height.value() as u32,
                        refresh: use_rate.is_active().then(|| refresh.value()),
                    },
                    _ => {
                        let values: Vec<u16> = counters.iter().map(|w| w.value() as u16).collect();
                        let sign = |i| match i {
                            1 => 1,
                            2 => -1,
                            _ => 0,
                        };
                        Mode::Modeline(Modeline {
                            clock_mhz: clock.value(),
                            horizontal: values[..4].try_into().unwrap(),
                            vertical: values[4..].try_into().unwrap(),
                            hsync: sign(hs.selected()),
                            vsync: sign(vs.selected()),
                        })
                    }
                };
                mode.encode().map(Value::String)
            })
        }
        "position" => {
            let position = Position::parse(string).unwrap_or(Position::Auto("auto".into()));
            let index = match &position {
                Position::Auto(s) => monitor::POSITIONS.iter().position(|v| *v == s).unwrap_or(0),
                _ => 9,
            };
            let picker = choices(
                panel,
                &[
                    "Automatic",
                    "Right",
                    "Left",
                    "Above",
                    "Below",
                    "Centered Right",
                    "Centered Left",
                    "Centered Above",
                    "Centered Below",
                    "Coordinates",
                ],
                index as u32,
            );
            let coords = gtk::Box::new(gtk::Orientation::Vertical, 8);
            let (x, y) = match position {
                Position::Point(x, y) => (x, y),
                _ => (0, 0),
            };
            let x = number(
                &coords,
                "X (logical pixels)",
                f64::from(x),
                f64::from(i32::MIN),
                f64::from(i32::MAX),
                0,
            );
            let y = number(
                &coords,
                "Y (logical pixels)",
                f64::from(y),
                f64::from(i32::MIN),
                f64::from(i32::MAX),
                0,
            );
            coords.set_visible(index == 9);
            panel.append(&coords);
            picker.connect_selected_notify(move |p| coords.set_visible(p.selected() == 9));
            Box::new(move || {
                x.update();
                y.update();
                Ok(Value::String(if picker.selected() == 9 {
                    Position::Point(x.value() as i32, y.value() as i32).encode()
                } else {
                    monitor::POSITIONS[picker.selected() as usize].into()
                }))
            })
        }
        "scale" => {
            let automatic = gtk::CheckButton::with_label("Automatic Scale");
            automatic.set_active(string == "auto" || string.is_empty());
            panel.append(&automatic);
            let scale = number(
                panel,
                "Scale",
                string.parse().unwrap_or(1.),
                0.25,
                10000.,
                3,
            );
            Box::new(move || {
                scale.update();
                Ok(Value::String(if automatic.is_active() {
                    "auto".into()
                } else {
                    scale.value().to_string()
                }))
            })
        }
        "mirror" => {
            let mut targets = vec![String::new()];
            for output in outputs {
                let name = output.text();
                if !name.is_empty() && !targets.contains(&name) {
                    targets.push(name);
                }
            }
            if !targets.iter().any(|s| s == string) {
                targets.push(string.into());
            }
            let mut labels = vec!["No Mirroring".to_string()];
            labels.extend(
                targets[1..]
                    .iter()
                    .map(|s| hyprland_editor_core::source::display_path_bytes(s.as_bytes())),
            );
            labels.push("Another Output".into());
            let refs: Vec<_> = labels.iter().map(String::as_str).collect();
            let index = targets.iter().position(|s| s == string).unwrap_or(0) as u32;
            let picker = choices(panel, &refs, index);
            let entry = gtk::Entry::new();
            entry.set_placeholder_text(Some("Connector name or desc:description"));
            entry.set_sensitive(false);
            panel.append(&entry);
            let last = targets.len() as u32;
            let weak = entry.downgrade();
            picker.connect_selected_notify(move |p| {
                if let Some(entry) = weak.upgrade() {
                    entry.set_sensitive(p.selected() == last);
                }
            });
            Box::new(move || {
                Ok(Value::String(if picker.selected() == last {
                    entry.text().into()
                } else {
                    targets[picker.selected() as usize].clone()
                }))
            })
        }
        "bitdepth" => {
            let value = if let Some(Value::Int(n)) = initial {
                n
            } else {
                8
            };
            let picker = choices(
                panel,
                &[
                    "8 bit",
                    "10 bit",
                    "Other Integer (interpreted as 8 bit upstream)",
                ],
                match value {
                    8 => 0,
                    10 => 1,
                    _ => 2,
                },
            );
            let custom = number(
                panel,
                "Custom Bit Depth",
                value as f64,
                f64::from(i32::MIN),
                f64::from(i32::MAX),
                0,
            );
            custom.set_sensitive(picker.selected() == 2);
            let weak = custom.downgrade();
            picker.connect_selected_notify(move |p| {
                if let Some(custom) = weak.upgrade() {
                    custom.set_sensitive(p.selected() == 2);
                }
            });
            Box::new(move || {
                custom.update();
                Ok(Value::Int(match picker.selected() {
                    0 => 8,
                    1 => 10,
                    _ => custom.value() as i64,
                }))
            })
        }
        "cm" => string_choice(panel, monitor::COLORS, string),
        "sdr_eotf" => string_choice(panel, monitor::TRANSFER, string),
        "transform" => {
            let index = if let Some(Value::Int(n)) = initial {
                n as u32
            } else {
                0
            };
            let picker = choices(
                panel,
                &[
                    "Normal",
                    "90°",
                    "180°",
                    "270°",
                    "Flipped",
                    "Flipped 90°",
                    "Flipped 180°",
                    "Flipped 270°",
                ],
                index,
            );
            Box::new(move || Ok(Value::Int(i64::from(picker.selected()))))
        }
        "vrr" => {
            let index = if let Some(Value::Int(n)) = initial {
                (n + 1) as u32
            } else {
                0
            };
            let picker = choices(
                panel,
                &[
                    "Inherit",
                    "Disabled",
                    "Enabled",
                    "Fullscreen Only",
                    "Fullscreen and Video",
                ],
                index,
            );
            Box::new(move || Ok(Value::Int(i64::from(picker.selected()) - 1)))
        }
        "supports_hdr" | "supports_wide_color" => {
            let index = if let Some(Value::Int(n)) = initial {
                (n + 1) as u32
            } else {
                1
            };
            let picker = choices(
                panel,
                &["Force Off", "Automatic Detection", "Force On"],
                index,
            );
            Box::new(move || Ok(Value::Int(i64::from(picker.selected()) - 1)))
        }
        "reserved" | "reserved_area" => {
            let values = if let Some(Value::CssGap(v)) = initial {
                v
            } else {
                [0; 4]
            };
            let fields: Vec<_> = ["Top", "Right", "Bottom", "Left"]
                .iter()
                .zip(values)
                .map(|(name, v)| {
                    number(
                        panel,
                        name,
                        v as f64,
                        f64::from(i32::MIN),
                        f64::from(i32::MAX),
                        0,
                    )
                })
                .collect();
            Box::new(move || {
                for field in &fields {
                    field.update();
                }
                Ok(Value::CssGap(
                    fields
                        .iter()
                        .map(|w| w.value() as i64)
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                ))
            })
        }
        "icc" => {
            let entry = gtk::Entry::new();
            entry.set_text(string);
            panel.append(&entry);
            let choose = gtk::Button::with_label("Choose ICC Profile…");
            panel.append(&choose);
            let win = window.downgrade();
            let output = entry.downgrade();
            choose.connect_clicked(move |_| {
                let Some(win) = win.upgrade() else {
                    return;
                };
                let output = output.clone();
                gtk::FileDialog::builder()
                    .title("Select an ICC profile path")
                    .build()
                    .open(Some(&win), None::<&gtk::gio::Cancellable>, move |result| {
                        if let (Ok(file), Some(output)) = (result, output.upgrade()) {
                            if let Some(path) = file.path() {
                                if let Some(path) = path.to_str() {
                                    output.set_text(path);
                                }
                            }
                        }
                    });
            });
            panel.append(&label("The selected profile is a path reference. Profile contents and hardware color accuracy remain unverified."));
            Box::new(move || Ok(Value::String(entry.text().into())))
        }
        _ => controls::form(setting, value, panel),
    }
}
fn current(state: &Shared) -> Vec<Rule> {
    let s = state.borrow();
    s.monitors
        .get(&s.draft.version)
        .cloned()
        .unwrap_or_default()
}
fn commit(state: &Shared, rules: Vec<Rule>, status: &gtk::Label, feedback: &gtk::Label) -> bool {
    let result = state.borrow_mut().replace_monitors(rules);
    match result {
        Ok(()) => {
            super::summary(state, status);
            feedback.set_text("Monitor draft updated. Save Draft keeps it; original files and desktop are unchanged.");
            true
        }
        Err(error) => {
            feedback.set_text(&error);
            false
        }
    }
}
fn edit_field(
    panel: &gtk::Box,
    window: &adw::Window,
    state: &Shared,
    selected: usize,
    setting: &Setting,
    labels: (&gtk::Label, &gtk::Label),
    canvas: &gtk::DrawingArea,
) {
    let (status, feedback) = labels;
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }
    let rules = current(state);
    let Some(rule) = rules.get(selected) else {
        panel.append(&label("Add a monitor rule to begin."));
        return;
    };
    let name = label(&presentation::setting_name(&setting.id));
    name.add_css_class("title-2");
    panel.append(&name);
    let enabled = gtk::CheckButton::with_label("Set This Field");
    enabled.set_widget_name("monitor-field-enabled");
    enabled.set_active(rule.fields.contains_key(&setting.lua_key));
    panel.append(&enabled);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_sensitive(enabled.is_active());
    let getter = builder(
        setting,
        rule.fields.get(&setting.lua_key).cloned(),
        &body,
        window,
        &rules.iter().map(|r| r.output.clone()).collect::<Vec<_>>(),
    );
    panel.append(&body);
    enabled.connect_toggled(move |w| body.set_sensitive(w.is_active()));
    let apply = gtk::Button::with_label("Update Monitor Draft");
    apply.add_css_class("suggested-action");
    panel.append(&apply);
    let id = rule.id.clone();
    let key = setting.lua_key.clone();
    let state = state.clone();
    let status = status.clone();
    let feedback = feedback.clone();
    let canvas = canvas.downgrade();
    apply.connect_clicked(move |_| {
        let mut rules = current(&state);
        let Some(rule) = rules.iter_mut().find(|r| r.id == id) else {
            return;
        };
        if enabled.is_active() {
            match getter() {
                Ok(value) => {
                    rule.fields.insert(key.clone(), value);
                }
                Err(error) => {
                    feedback.set_text(&error);
                    return;
                }
            }
        } else {
            rule.fields.remove(&key);
        }
        if commit(&state, rules, &status, &feedback) {
            if let Some(canvas) = canvas.upgrade() {
                canvas.queue_draw();
            }
        }
    });
    let details = gtk::Expander::builder().label("Field Details").build();
    details.set_child(Some(&label(&format!("Field: {}\nAudited default: {}\nVersion: {}\nSource: {}\nUnset fields inherit compositor rule state. This is a draft declaration, not a hardware observation.",setting.lua_key,setting.default_expression,setting.version,setting.source))));
    panel.append(&details);
}
pub(super) fn connect(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) {
    let win = window.downgrade();
    let state = state.clone();
    let status = status.clone();
    button.connect_clicked(move |_| {
        if let Some(win) = win.upgrade() {
            show(&win, &state, &status);
        }
    });
}
fn show(window: &adw::ApplicationWindow, state: &Shared, status: &gtk::Label) -> adw::Window {
    let dialog = adw::Window::builder()
        .title("Monitor Drafts")
        .transient_for(window)
        .modal(true)
        .default_width(820)
        .default_height(760)
        .build();
    dialog.set_size_request(350, 480);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for setter in [
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
    ] {
        setter(content.upcast_ref(), 16);
    }
    content.append(&label("Arrange outputs and edit one field at a time. This layout is a draft; automatic modes and scaling are schematic until their dimensions are known."));
    let canvas = gtk::DrawingArea::builder()
        .content_height(180)
        .hexpand(true)
        .build();
    canvas.set_widget_name("monitor-map");
    content.append(&canvas);
    let rule_model = gtk::StringList::new(&[]);
    let selected = gtk::DropDown::builder().model(&rule_model).build();
    compact(&selected);
    selected.set_widget_name("monitor-rule");
    content.append(&selected);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add");
    let duplicate = gtk::Button::with_label("Duplicate");
    let remove = gtk::Button::with_label("Remove");
    let up = gtk::Button::with_label("↑");
    let down = gtk::Button::with_label("↓");
    for button in [&add, &duplicate, &remove] {
        actions.append(button);
    }
    content.append(&actions);
    let target = gtk::Box::new(gtk::Orientation::Vertical, 8);
    target.append(&label("Output Match"));
    let target_type = choices(
        &target,
        &[
            "Fallback (unspecified outputs)",
            "Connector Name",
            "Description",
        ],
        0,
    );
    target_type.set_widget_name("monitor-target-kind");
    let target_name = gtk::Entry::new();
    target_name.set_widget_name("monitor-target");
    target.append(&target_name);
    let target_save = gtk::Button::with_label("Update Output Match");
    target.append(&target_save);
    let target_details = gtk::Expander::builder().label("Output Match").build();
    target_details.set_child(Some(&target));
    content.append(&target_details);
    let fields = monitor::catalogue(&state.borrow().draft.version);
    let names: Vec<_> = fields
        .iter()
        .map(|s| presentation::setting_name(&s.id))
        .collect();
    let refs: Vec<_> = names.iter().map(String::as_str).collect();
    let property = gtk::DropDown::from_strings(&refs);
    compact(&property);
    property.set_widget_name("monitor-field");
    content.append(&property);
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(&panel);
    let feedback = label("");
    feedback.set_widget_name("monitor-feedback");
    content.append(&feedback);
    let review = gtk::Button::with_label("Review Monitor Lua");
    content.append(&review);
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    dialog.set_content(Some(&root));
    let busy = Rc::new(Cell::new(false));
    let refresh: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let status = status.clone();
        let panel = panel.clone();
        let selected = selected.clone();
        let property = property.clone();
        let fields = fields.clone();
        let model = rule_model.clone();
        let target_type = target_type.clone();
        let target_name = target_name.clone();
        let feedback = feedback.clone();
        let canvas = canvas.clone();
        let win = dialog.downgrade();
        let busy = busy.clone();
        move || {
            if busy.replace(true) {
                return;
            }
            let rules = current(&state);
            let index = (selected.selected() as usize).min(rules.len().saturating_sub(1));
            let names: Vec<String> = rules
                .iter()
                .enumerate()
                .map(|(n, r)| {
                    format!(
                        "{}. {}",
                        n + 1,
                        if r.output == Output::Fallback {
                            "Fallback".into()
                        } else {
                            hyprland_editor_core::source::display_path_bytes(
                                r.output.text().as_bytes(),
                            )
                        }
                    )
                })
                .collect();
            let changed = model.n_items() as usize != names.len()
                || names
                    .iter()
                    .enumerate()
                    .any(|(n, s)| model.string(n as u32).as_deref() != Some(s.as_str()));
            if changed {
                model.splice(
                    0,
                    model.n_items(),
                    &names.iter().map(String::as_str).collect::<Vec<_>>(),
                );
            }
            selected.set_selected(if rules.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            if let Some(rule) = rules.get(index) {
                match &rule.output {
                    Output::Fallback => {
                        target_type.set_selected(0);
                        target_name.set_text("");
                    }
                    Output::Connector(name) => {
                        target_type.set_selected(1);
                        target_name.set_text(name);
                    }
                    Output::Description(name) => {
                        target_type.set_selected(2);
                        target_name.set_text(name);
                    }
                }
            }
            target_name.set_sensitive(target_type.selected() != 0);
            if let (Some(win), Some(setting)) =
                (win.upgrade(), fields.get(property.selected() as usize))
            {
                edit_field(
                    &panel,
                    &win,
                    &state,
                    index,
                    setting,
                    (&status, &feedback),
                    &canvas,
                );
            }
            canvas.queue_draw();
            busy.set(false);
        }
    });
    let callback = Rc::downgrade(&refresh);
    selected.connect_selected_notify(move |_| {
        if let Some(refresh) = callback.upgrade() {
            refresh();
        }
    });
    let callback = Rc::downgrade(&refresh);
    property.connect_selected_notify(move |_| {
        if let Some(refresh) = callback.upgrade() {
            refresh();
        }
    });
    let entry = target_name.downgrade();
    target_type.connect_selected_notify(move |picker| {
        if let Some(entry) = entry.upgrade() {
            entry.set_sensitive(picker.selected() != 0);
        }
    });
    for (button, operation) in [
        (&add, 0),
        (&duplicate, 1),
        (&remove, 2),
        (&up, 3),
        (&down, 4),
    ] {
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let selected = selected.clone();
        let refresh = refresh.clone();
        button.connect_clicked(move |_| {
            let mut rules = current(&state);
            let index = selected.selected() as usize;
            match operation {
                0 | 1 => {
                    if rules.len() >= monitor::MAX_RULES {
                        feedback.set_text("The monitor draft already has 64 rules.");
                        return;
                    }
                    let mut serial = 0;
                    let id = loop {
                        let id = format!("monitor-{serial}");
                        if !rules.iter().any(|r| r.id == id) {
                            break id;
                        }
                        serial += 1;
                    };
                    let mut rule = if operation == 1 {
                        let Some(rule) = rules.get(index) else {
                            return;
                        };
                        rule.clone()
                    } else {
                        Rule::new(&id)
                    };
                    rule.id = id;
                    rules.push(rule);
                }
                2 if index < rules.len() => {
                    rules.remove(index);
                }
                3 if index > 0 && index < rules.len() => rules.swap(index, index - 1),
                4 if index + 1 < rules.len() => rules.swap(index, index + 1),
                _ => return,
            }
            if commit(&state, rules, &status, &feedback) {
                refresh();
                if operation <= 1 {
                    selected.set_selected(current(&state).len().saturating_sub(1) as u32);
                } else if operation == 3 {
                    selected.set_selected((index - 1) as u32);
                } else if operation == 4 {
                    selected.set_selected((index + 1) as u32);
                }
            }
        });
    }
    let state_c = state.clone();
    let status_c = status.clone();
    let feedback_c = feedback.clone();
    let selected_c = selected.clone();
    let refresh_c = refresh.clone();
    target_save.connect_clicked(move |_| {
        let mut rules = current(&state_c);
        let Some(rule) = rules.get_mut(selected_c.selected() as usize) else {
            return;
        };
        rule.output = match target_type.selected() {
            1 => Output::Connector(target_name.text().into()),
            2 => Output::Description(target_name.text().into()),
            _ => Output::Fallback,
        };
        if commit(&state_c, rules, &status_c, &feedback_c) {
            refresh_c();
        }
    });
    let undo = gtk::Button::with_label("Undo");
    let ordering = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    for b in [&up, &down, &undo] {
        ordering.append(b);
    }
    content.insert_child_after(&ordering, Some(&actions));
    up.update_property(&[gtk::accessible::Property::Label("Move rule up")]);
    down.update_property(&[gtk::accessible::Property::Label("Move rule down")]);
    let state_c = state.clone();
    let status_c = status.clone();
    let refresh_c = refresh.clone();
    undo.connect_clicked(move |_| {
        state_c.borrow_mut().undo_last();
        super::summary(&state_c, &status_c);
        refresh_c();
    });
    let parent = window.downgrade();
    let save = gtk::Button::with_label("Save Draft");
    content.append(&save);
    save.connect_clicked(move |_| {
        if let Some(parent) = parent.upgrade() {
            if let Some(action) = parent.lookup_action("save-draft") {
                action.activate(None);
            }
        }
    });
    let state_c = state.clone();
    let win = dialog.downgrade();
    review.connect_clicked(move|_|{
        let s=state_c.borrow();let rules=s.monitors.get(&s.draft.version).map(Vec::as_slice).unwrap_or(&[]);
        let text=match monitor::export_lua(rules,&s.draft.version){Ok(text)=>text,Err(error)=>{if let Some(win)=win.upgrade(){let d=adw::MessageDialog::new(Some(&win),Some("Invalid monitor draft"),Some(&error));d.add_response("close","Close");d.present();}return;}};
        let Some(win)=win.upgrade()else{return;};let review=adw::Window::builder().title("Review Monitor Lua").transient_for(&win).modal(true).default_width(680).default_height(500).build();
        let box_=gtk::Box::new(gtk::Orientation::Vertical,12);box_.append(&label("These are ordered draft monitor declarations. Copying does not apply them. They must not replace a full configuration."));
        let view=gtk::TextView::builder().editable(false).monospace(true).wrap_mode(gtk::WrapMode::WordChar).build();view.buffer().set_text(&text);box_.append(&gtk::ScrolledWindow::builder().child(&view).vexpand(true).build());
        let copy=gtk::Button::with_label("Copy Reviewed Monitor Lua");box_.append(&copy);let target=review.downgrade();copy.connect_clicked(move|_|{if let Some(win)=target.upgrade(){win.clipboard().set_text(&text);}});review.set_content(Some(&box_));review.present();
    });
    let geometry = Rc::new(RefCell::new(Vec::<(String, f64, f64, f64, f64, f64)>::new()));
    let state_c = state.clone();
    let geometry_c = geometry.clone();
    canvas.set_draw_func(move |_, context, width, height| {
        let rules = current(&state_c);
        geometry_c.borrow_mut().clear();
        if rules.is_empty() {
            context.set_source_rgb(0.6, 0.6, 0.6);
            context.move_to(16., 30.);
            let _ = context.show_text("Add a monitor rule to see its layout.");
            return;
        }
        let mut boxes = vec![];
        let mut auto_x = 0.;
        for rule in &rules {
            let (w, h) = rule.logical_size().unwrap_or((800., 600.));
            let (x, y) = rule
                .coordinates()
                .map(|(x, y)| (f64::from(x), f64::from(y)))
                .unwrap_or_else(|| {
                    let x = auto_x;
                    auto_x += w + 100.;
                    (x, 0.)
                });
            boxes.push((rule, x, y, w, h));
        }
        let min_x = boxes
            .iter()
            .map(|(_, x, _, _, _)| *x)
            .fold(f64::INFINITY, f64::min);
        let min_y = boxes
            .iter()
            .map(|(_, _, y, _, _)| *y)
            .fold(f64::INFINITY, f64::min);
        let max_x = boxes
            .iter()
            .map(|(_, x, _, w, _)| x + w)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = boxes
            .iter()
            .map(|(_, _, y, _, h)| y + h)
            .fold(f64::NEG_INFINITY, f64::max);
        let scale = ((f64::from(width) - 32.) / (max_x - min_x).max(1.))
            .min((f64::from(height) - 32.) / (max_y - min_y).max(1.))
            .max(0.00000001);
        for (rule, x, y, w, h) in boxes {
            let (px, py, pw, ph) = (
                16. + (x - min_x) * scale,
                16. + (y - min_y) * scale,
                (w * scale).max(8.),
                (h * scale).max(8.),
            );
            context.set_source_rgba(
                0.35,
                0.45,
                0.65,
                if matches!(rule.fields.get("disabled"), Some(Value::Bool(true))) {
                    0.25
                } else {
                    0.7
                },
            );
            context.rectangle(px, py, pw, ph);
            let _ = context.fill();
            context.set_source_rgb(0.9, 0.9, 0.9);
            context.move_to(px + 6., py + 20.);
            context.set_font_size(12.);
            let name: String = if rule.output == Output::Fallback {
                "Fallback".into()
            } else {
                hyprland_editor_core::source::display_path_bytes(rule.output.text().as_bytes())
                    .chars()
                    .take(24)
                    .collect()
            };
            let _ = context.show_text(&name);
            if rule.logical_size().is_some() && rule.coordinates().is_some() {
                geometry_c
                    .borrow_mut()
                    .push((rule.id.clone(), px, py, pw, ph, scale));
            }
        }
    });
    let drag = gtk::GestureDrag::new();
    let dragging = Rc::new(RefCell::new(None::<(String, f64, i32, i32)>));
    let g = geometry.clone();
    let state_c = state.clone();
    let d = dragging.clone();
    drag.connect_drag_begin(move |_, x, y| {
        *d.borrow_mut() = g
            .borrow()
            .iter()
            .rev()
            .find(|(_, px, py, w, h, _)| x >= *px && x <= px + w && y >= *py && y <= py + h)
            .map(|(id, _, _, _, _, scale)| {
                let coords = current(&state_c)
                    .iter()
                    .find(|r| r.id == *id)
                    .and_then(Rule::coordinates)
                    .unwrap_or((0, 0));
                (id.clone(), *scale, coords.0, coords.1)
            });
    });
    let state_c = state.clone();
    let status_c = status.clone();
    let feedback_c = feedback.clone();
    let refresh_c = refresh.clone();
    drag.connect_drag_end(move |_, dx, dy| {
        let Some((id, scale, x, y)) = dragging.borrow_mut().take() else {
            return;
        };
        let mut rules = current(&state_c);
        let Some(rule) = rules.iter_mut().find(|r| r.id == id) else {
            return;
        };
        let x = (f64::from(x) + dx / scale).round();
        let y = (f64::from(y) + dy / scale).round();
        if x < f64::from(i32::MIN)
            || x > f64::from(i32::MAX)
            || y < f64::from(i32::MIN)
            || y > f64::from(i32::MAX)
        {
            feedback_c.set_text("Dragged coordinates exceed the supported range.");
            return;
        }
        rule.fields.insert(
            "position".into(),
            Value::String(Position::Point(x as i32, y as i32).encode()),
        );
        if commit(&state_c, rules, &status_c, &feedback_c) {
            refresh_c();
        }
    });
    canvas.add_controller(drag);
    refresh();
    dialog.present();
    dialog
}

#[cfg(test)]
pub(super) fn test_editor(window: &adw::ApplicationWindow) {
    fn descendants(widget: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut all = vec![];
        let mut child = widget.first_child();
        while let Some(w) = child {
            all.push(w.clone());
            all.extend(descendants(&w));
            child = w.next_sibling();
        }
        all
    }
    fn click(win: &adw::Window, name: &str) {
        descendants(win.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(name))
            .unwrap_or_else(|| panic!("button {name}"))
            .emit_clicked();
    }
    fn picker(win: &adw::Window, name: &str) -> gtk::DropDown {
        descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap()
            .downcast()
            .unwrap()
    }
    let state = super::initial_state();
    let status = label("");
    let win = show(window, &state, &status);
    click(&win, "Add");
    assert_eq!(current(&state).len(), 1);
    assert!(state.borrow().dirty());
    let target_details = descendants(win.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Expander>().ok())
        .find(|e| e.label().as_deref() == Some("Output Match"))
        .unwrap();
    target_details.set_expanded(true);
    picker(&win, "monitor-target-kind").set_selected(1);
    let target = descendants(win.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "monitor-target")
        .unwrap()
        .downcast::<gtk::Entry>()
        .unwrap();
    target.set_text("DP-1");
    click(&win, "Update Output Match");
    assert_eq!(current(&state)[0].output, Output::Connector("DP-1".into()));
    target_details.set_expanded(false);
    let fields = monitor::catalogue(&state.borrow().draft.version);
    let property = picker(&win, "monitor-field");
    let context = gtk::glib::MainContext::default();
    let settle = || {
        for _ in 0..30 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    for (index, setting) in fields.iter().enumerate() {
        property.set_selected(index as u32);
        let enabled = descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == "monitor-field-enabled")
            .unwrap()
            .downcast::<gtk::CheckButton>()
            .unwrap();
        if setting.lua_key == "icc" {
            continue;
        }
        if setting.lua_key == "reserved_area" {
            let mut rules = current(&state);
            rules[0].fields.remove("reserved");
            state.borrow_mut().replace_monitors(rules).unwrap();
        }
        enabled.set_active(true);
        click(&win, "Update Monitor Draft");
        assert!(
            current(&state)[0].fields.contains_key(&setting.lua_key),
            "{}",
            setting.lua_key
        );
    }
    click(&win, "Duplicate");
    assert_eq!(current(&state).len(), 2);
    click(&win, "↑");
    assert_eq!(picker(&win, "monitor-rule").selected(), 0);
    let after = current(&state);
    click(&win, "Remove");
    assert_eq!(current(&state).len(), 1);
    click(&win, "Undo");
    assert_eq!(current(&state), after);
    click(&win, "Review Monitor Lua");
    let review = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Window>().ok())
        .find(|w| w.title().as_deref() == Some("Review Monitor Lua"))
        .unwrap();
    let text = descendants(review.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::TextView>().ok())
        .unwrap();
    let buffer = text.buffer();
    let source = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
    assert_eq!(
        hyprland_editor_core::lua::parse(
            &source,
            &state.borrow().draft.version,
            &state.borrow().catalogue
        )
        .unwrap()
        .monitors
        .len(),
        2
    );
    review.destroy();
    if std::env::var_os("HYPRLAND_EDITOR_TEST_EXACT_RESIZE").is_some() {
        win.set_default_size(360, 760);
        settle();
        for (index, setting) in fields.iter().enumerate() {
            property.set_selected(index as u32);
            settle();
            assert!(
                win.width() <= 360,
                "{} forced {}px",
                setting.id,
                win.width()
            );
            let (min, _, _, _) = win
                .content()
                .unwrap()
                .measure(gtk::Orientation::Horizontal, -1);
            assert!(min <= 360, "{} needs {min}px", setting.id);
        }
    }
    // Show explicit geometry in the captured fixture; it is not a hardware preview.
    let mut rules = current(&state);
    rules[0]
        .fields
        .insert("mode".into(), Value::String("2560x1440@144".into()));
    rules[0]
        .fields
        .insert("scale".into(), Value::String("1.25".into()));
    rules[0]
        .fields
        .insert("position".into(), Value::String("0x0".into()));
    rules[1].output = Output::Connector("HDMI-A-1".into());
    rules[1]
        .fields
        .insert("mode".into(), Value::String("1920x1080@60".into()));
    rules[1]
        .fields
        .insert("scale".into(), Value::String("1".into()));
    rules[1]
        .fields
        .insert("position".into(), Value::String("2048x0".into()));
    state.borrow_mut().replace_monitors(rules).unwrap();
    picker(&win, "monitor-rule").set_selected(0);
    property.set_selected(fields.iter().position(|s| s.lua_key == "mode").unwrap() as u32);
    win.set_default_size(820, 760);
    settle();
    let canvas = descendants(win.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "monitor-map")
        .unwrap()
        .downcast::<gtk::DrawingArea>()
        .unwrap();
    let controllers = canvas.observe_controllers();
    let drag = (0..controllers.n_items())
        .find_map(|i| controllers.item(i).and_downcast::<gtk::GestureDrag>())
        .unwrap();
    let before_drag = current(&state);
    drag.emit_by_name::<()>("drag-begin", &[&30_f64, &30_f64]);
    drag.emit_by_name::<()>("drag-end", &[&20_f64, &10_f64]);
    assert_ne!(
        current(&state)[0].coordinates(),
        before_drag[0].coordinates()
    );
    click(&win, "Undo");
    assert_eq!(current(&state), before_drag);
    settle();
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
        let node = snapshot.to_node().unwrap();
        win.renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(format!("{path}.monitors.png"))
            .unwrap();
    }
    win.destroy();
}
