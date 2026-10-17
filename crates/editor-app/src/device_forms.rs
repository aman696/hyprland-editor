//! Specialized device builders: identifier choices, linked XKB groups and numeric curves.
use super::{
    controls, label,
    monitors::{choices, compact, number},
};
use adw::prelude::*;
use hyprland_editor_core::{
    device::{Acceleration, Curve, Spec},
    xkb::{Choice, Registry},
    Setting, Value,
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
};
pub(super) type Getter = Box<dyn Fn() -> Result<BTreeMap<String, Value>, String>>;
fn selected_text(spec: &Spec, key: &str, default: &str) -> String {
    match spec.fields.get(key) {
        Some(Value::String(s)) => s.clone(),
        _ => default.into(),
    }
}
fn value_map(key: &str, value: Value) -> BTreeMap<String, Value> {
    BTreeMap::from([(key.into(), value)])
}
fn strings(panel: &gtk::Box, items: &[(&str, &str)], value: &str) -> Box<dyn Fn() -> String> {
    let mut ids: Vec<String> = items.iter().map(|(id, _)| (*id).into()).collect();
    let mut labels: Vec<String> = items.iter().map(|(_, s)| (*s).into()).collect();
    if !ids.iter().any(|id| id == value) {
        ids.push(value.into());
        labels.push(format!(
            "Existing: {}",
            hyprland_editor_core::source::display_path_bytes(value.as_bytes())
        ));
    }
    let refs: Vec<_> = labels.iter().map(String::as_str).collect();
    let pick = choices(
        panel,
        &refs,
        ids.iter().position(|id| id == value).unwrap_or(0) as u32,
    );
    Box::new(move || ids[pick.selected() as usize].clone())
}
fn safe_text(text: &str) -> String {
    hyprland_editor_core::source::display_path_bytes(text.as_bytes())
}
fn registry_choice(panel: &gtk::Box, items: &[Choice], value: &str) -> Box<dyn Fn() -> String> {
    let mut choices_ = vec![(String::new(), "Default".to_string())];
    choices_.extend(
        items
            .iter()
            .map(|c| (c.id.clone(), format!("{} · {}", safe_text(&c.label), c.id))),
    );
    let refs: Vec<_> = choices_
        .iter()
        .map(|(id, text)| (id.as_str(), text.as_str()))
        .collect();
    strings(panel, &refs, value)
}
fn curve(panel: &gtk::Box, initial: Curve) -> Box<dyn Fn() -> Result<Curve, String>> {
    panel.append(&label("Points have equally spaced input speeds. The step is device units per millisecond; point values are pointer speeds. Hardware response is unverified."));
    let step = number(
        panel,
        "Input Speed Step",
        initial.step,
        f64::MIN_POSITIVE,
        f64::MAX,
        6,
    );
    step.set_max_width_chars(8);
    step.connect_output(|w| {
        w.set_text(&w.value().to_string());
        gtk::glib::Propagation::Stop
    });
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
    panel.append(&rows);
    let entries = Rc::new(RefCell::new(Vec::<(gtk::Box, gtk::SpinButton)>::new()));
    let add_row: Rc<dyn Fn(f64)> = Rc::new({
        let rows = rows.downgrade();
        let entries = entries.clone();
        move |value| {
            let Some(rows) = rows.upgrade() else {
                return;
            };
            if entries.borrow().len() >= 64 {
                return;
            }
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let input = gtk::SpinButton::with_range(0., f64::MAX, 0.01);
            input.set_digits(6);
            input.set_value(value);
            input.set_width_chars(8);
            input.set_max_width_chars(8);
            input.connect_output(|w| {
                w.set_text(&w.value().to_string());
                gtk::glib::Propagation::Stop
            });
            input.set_hexpand(true);
            input.update_property(&[gtk::accessible::Property::Label("Curve point speed")]);
            let remove = gtk::Button::with_label("Remove");
            row.append(&input);
            row.append(&remove);
            rows.append(&row);
            entries.borrow_mut().push((row.clone(), input));
            let entries = Rc::downgrade(&entries);
            let row = row.downgrade();
            let rows = rows.downgrade();
            remove.connect_clicked(move |_| {
                if let (Some(entries), Some(row), Some(rows)) =
                    (entries.upgrade(), row.upgrade(), rows.upgrade())
                {
                    entries.borrow_mut().retain(|(r, _)| *r != row);
                    rows.remove(&row);
                }
            });
        }
    });
    for value in initial.points {
        add_row(value);
    }
    let add = gtk::Button::with_label("Add Curve Point");
    panel.append(&add);
    let append = add_row.clone();
    add.connect_clicked(move |_| append(1.));
    Box::new(move || {
        step.update();
        let points = entries
            .borrow()
            .iter()
            .map(|(_, p)| {
                p.update();
                p.value()
            })
            .collect();
        let c = Curve {
            step: step.value(),
            points,
        };
        c.validate()?;
        Ok(c)
    })
}
type AppendLayout = Rc<dyn Fn(&str, &str)>;
struct KeyboardRow {
    widget: gtk::Box,
    layout: gtk::DropDown,
    layout_ids: Vec<String>,
    variant: gtk::DropDown,
    variant_ids: Rc<RefCell<Vec<String>>>,
}
fn layouts(panel: &gtk::Box, spec: &Spec, registry: Arc<Registry>) -> Getter {
    if selected_text(spec, "kb_layout", "us").split(',').count() > 16 {
        panel.append(&label("This declaration has more than 16 layout slots. Its saved values are preserved; this builder cannot stage a truncated list."));
        return Box::new(|| {
            Err("Layout list exceeds this builder's 16-slot limit; no values were staged".into())
        });
    }
    panel.append(&label("Choose layouts in the order you want to switch between them, with a variant for each. Updating saves both choices in your draft."));
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 10);
    panel.append(&rows);
    let entries = Rc::new(RefCell::new(Vec::<KeyboardRow>::new()));
    let add_row: AppendLayout = Rc::new({
        let rows = rows.downgrade();
        let entries = entries.clone();
        move |layout, variant| {
            let Some(rows) = rows.upgrade() else {
                return;
            };
            if entries.borrow().len() >= 16 {
                return;
            }
            let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let mut ids = vec![String::new()];
            ids.extend(registry.layouts.iter().map(|l| l.choice.id.clone()));
            let mut labels = vec!["XKB Default".to_string()];
            labels.extend(
                registry
                    .layouts
                    .iter()
                    .map(|l| format!("{} · {}", safe_text(&l.choice.label), l.choice.id)),
            );
            if !ids.iter().any(|s| s == layout) {
                ids.push(layout.into());
                labels.push(format!("Existing: {}", safe_text(layout)));
            }
            let layout_pick = choices(
                &row,
                &labels.iter().map(String::as_str).collect::<Vec<_>>(),
                ids.iter().position(|s| s == layout).unwrap_or(0) as u32,
            );
            layout_pick.update_property(&[gtk::accessible::Property::Label("Keyboard layout")]);
            let model = gtk::StringList::new(&[]);
            let variant_pick = gtk::DropDown::builder().model(&model).build();
            compact(&variant_pick);
            variant_pick.update_property(&[gtk::accessible::Property::Label("Keyboard variant")]);
            row.append(&variant_pick);
            let variant_ids = Rc::new(RefCell::new(vec![]));
            let populate = Rc::new({
                let registry = registry.clone();
                let model = model.clone();
                let variant_ids = variant_ids.clone();
                let variant_pick = variant_pick.downgrade();
                move |layout: &str, current: &str| {
                    let mut ids = vec![String::new()];
                    let mut names = vec!["Default Variant".to_string()];
                    if let Some(l) = registry.layouts.iter().find(|l| l.choice.id == layout) {
                        ids.extend(l.variants.iter().map(|v| v.id.clone()));
                        names.extend(
                            l.variants
                                .iter()
                                .map(|v| format!("{} · {}", safe_text(&v.label), v.id)),
                        );
                    }
                    if !ids.iter().any(|s| s == current) {
                        ids.push(current.into());
                        names.push(format!("Existing: {}", safe_text(current)));
                    }
                    let index = ids.iter().position(|s| s == current).unwrap_or(0);
                    *variant_ids.borrow_mut() = ids;
                    model.splice(
                        0,
                        model.n_items(),
                        &names.iter().map(String::as_str).collect::<Vec<_>>(),
                    );
                    if let Some(p) = variant_pick.upgrade() {
                        p.set_selected(index as u32);
                    }
                }
            });
            populate(layout, variant);
            let change = populate.clone();
            let choice_ids = ids.clone();
            layout_pick
                .connect_selected_notify(move |p| change(&choice_ids[p.selected() as usize], ""));
            let remove = gtk::Button::with_label("Remove Layout");
            row.append(&remove);
            rows.append(&row);
            entries.borrow_mut().push(KeyboardRow {
                widget: row.clone(),
                layout: layout_pick,
                layout_ids: ids,
                variant: variant_pick,
                variant_ids,
            });
            let entries = Rc::downgrade(&entries);
            let row = row.downgrade();
            let rows = rows.downgrade();
            remove.connect_clicked(move |_| {
                if let (Some(entries), Some(row), Some(rows)) =
                    (entries.upgrade(), row.upgrade(), rows.upgrade())
                {
                    entries.borrow_mut().retain(|r| r.widget != row);
                    rows.remove(&row);
                }
            });
        }
    });
    let layouts = selected_text(spec, "kb_layout", "us");
    let variants = selected_text(spec, "kb_variant", "");
    let variant_list: Vec<_> = variants.split(',').collect();
    for (index, l) in layouts.split(',').enumerate() {
        add_row(l, variant_list.get(index).copied().unwrap_or(""));
    }
    let add = gtk::Button::with_label("Add Keyboard Layout");
    panel.append(&add);
    let append = add_row.clone();
    add.connect_clicked(move |_| append("us", ""));
    Box::new(move || {
        let rows = entries.borrow();
        if rows.is_empty() {
            return Err("Keep at least one layout slot, or unset the field".into());
        }
        let layouts = rows
            .iter()
            .map(|r| r.layout_ids[r.layout.selected() as usize].clone())
            .collect::<Vec<_>>()
            .join(",");
        let variants = rows
            .iter()
            .map(|r| r.variant_ids.borrow()[r.variant.selected() as usize].clone())
            .collect::<Vec<_>>()
            .join(",");
        Ok(BTreeMap::from([
            ("kb_layout".into(), Value::String(layouts)),
            ("kb_variant".into(), Value::String(variants)),
        ]))
    })
}
fn options(panel: &gtk::Box, text: &str, registry: Arc<Registry>) -> Getter {
    let original = text
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let selected = Rc::new(RefCell::new(
        text.split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect::<BTreeSet<_>>(),
    ));
    let known: BTreeSet<_> = registry
        .groups
        .iter()
        .flat_map(|g| g.options.iter().map(|o| o.id.clone()))
        .collect();
    for id in selected
        .borrow()
        .difference(&known)
        .cloned()
        .collect::<Vec<_>>()
    {
        let row =
            gtk::CheckButton::with_label(&format!("Keep existing option: {}", safe_text(&id)));
        row.set_active(true);
        panel.append(&row);
        let s = selected.clone();
        row.connect_toggled(move |w| {
            if w.is_active() {
                s.borrow_mut().insert(id.clone());
            } else {
                s.borrow_mut().remove(&id);
            }
        });
    }
    for group in &registry.groups {
        let expander = gtk::Expander::builder().label(&group.choice.label).build();
        let title = label(&hyprland_editor_core::source::display_path_bytes(
            group.choice.label.as_bytes(),
        ));
        title.set_selectable(false);
        expander.set_label_widget(Some(&title));
        panel.append(&expander);
        let group = group.clone();
        let selected = selected.clone();
        let built = Cell::new(false);
        expander.connect_expanded_notify(move |e| {
            if !e.is_expanded() {
                e.set_child(None::<&gtk::Widget>);
                built.set(false);
                return;
            }
            if built.replace(true) {
                return;
            }
            let rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let widgets = Rc::new(RefCell::new(Vec::<(
                String,
                gtk::glib::WeakRef<gtk::CheckButton>,
            )>::new()));
            let busy = Rc::new(Cell::new(false));
            for option in &group.options {
                let check = gtk::CheckButton::new();
                let title = label(&format!("{} · {}", safe_text(&option.label), option.id));
                title.set_selectable(false);
                check.set_child(Some(&title));
                check.set_active(selected.borrow().contains(&option.id));
                rows.append(&check);
                widgets
                    .borrow_mut()
                    .push((option.id.clone(), check.downgrade()));
                let selected = selected.clone();
                let id = option.id.clone();
                let widgets = widgets.clone();
                let busy = busy.clone();
                let multiple = group.multiple;
                check.connect_toggled(move |w| {
                    if busy.replace(true) {
                        return;
                    }
                    if w.is_active() {
                        if !multiple {
                            for (other, weak) in widgets.borrow().iter() {
                                if *other != id {
                                    selected.borrow_mut().remove(other);
                                    if let Some(other) = weak.upgrade() {
                                        other.set_active(false);
                                    }
                                }
                            }
                        }
                        selected.borrow_mut().insert(id.clone());
                    } else {
                        selected.borrow_mut().remove(&id);
                    }
                    busy.set(false);
                });
            }
            e.set_child(Some(&rows));
        });
    }
    Box::new(move || {
        let selected = selected.borrow();
        let mut ids = original
            .iter()
            .filter(|id| selected.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        ids.extend(selected.iter().filter(|id| !original.contains(id)).cloned());
        Ok(value_map("kb_options", Value::String(ids.join(","))))
    })
}
pub(super) fn form(
    setting: &Setting,
    spec: &Spec,
    panel: &gtk::Box,
    window: &adw::Window,
    registry: Option<Arc<Registry>>,
) -> Getter {
    let key = setting.lua_key.clone();
    let initial = spec
        .fields
        .get(&key)
        .cloned()
        .or_else(|| setting.known_default());
    let text = match &initial {
        Some(Value::String(s)) => s.as_str(),
        _ => "",
    };
    if let Some(registry) = registry {
        match key.as_str() {
            "kb_layout" | "kb_variant" => return layouts(panel, spec, registry),
            "kb_options" => return options(panel, text, registry),
            "kb_model" => {
                let get = registry_choice(panel, &registry.models, text);
                return Box::new(move || Ok(value_map(&key, Value::String(get()))));
            }
            _ => {}
        }
    }
    match key.as_str() {
        "pressure_range_min" | "pressure_range_max" => {
            let value = match initial {
                Some(Value::Float(v)) => v,
                _ => -1.0,
            };
            let automatic = gtk::CheckButton::with_label("Use hardware default");
            automatic.set_widget_name("device-pressure-default");
            automatic.set_active(value < 0.0);
            panel.append(&automatic);
            let input = number(
                panel,
                "Pressure (0–1)",
                if value >= 0.0 {
                    value
                } else if key == "pressure_range_max" {
                    1.0
                } else {
                    0.0
                },
                0.0,
                1.0,
                6,
            );
            input.set_widget_name("device-pressure-value");
            input.set_sensitive(!automatic.is_active());
            panel.append(&label("Negative source values use the tool's hardware default. The device's measured pressure range is not detected here."));
            let changed = Rc::new(Cell::new(false));
            let flag = changed.clone();
            let weak = input.downgrade();
            automatic.connect_toggled(move |w| {
                flag.set(true);
                if let Some(input) = weak.upgrade() {
                    input.set_sensitive(!w.is_active());
                }
            });
            Box::new(move || {
                let v = if automatic.is_active() {
                    if value < 0.0 && !changed.get() {
                        value
                    } else {
                        -1.0
                    }
                } else {
                    input.update();
                    input.value()
                };
                Ok(value_map(&key, Value::Float(v)))
            })
        }
        "eraser_button_mode" => {
            let value = match initial {
                Some(Value::Int(v)) => v,
                _ => 0,
            };
            let pick = choices(
                panel,
                &[
                    "Hardware behavior (0)",
                    "Send a button event (1)",
                    "Other mode number",
                ],
                if value == 0 {
                    0
                } else if value == 1 {
                    1
                } else {
                    2
                },
            );
            pick.set_widget_name("device-eraser-mode");
            let other = gtk::Box::new(gtk::Orientation::Vertical, 6);
            panel.append(&other);
            let input = number(&other, "Mode number", value.max(2) as f64, 2.0, 6.0, 0);
            input.set_widget_name("device-eraser-other-mode");
            other.set_visible(pick.selected() == 2);
            let weak = other.downgrade();
            pick.connect_selected_notify(move |w| {
                if let Some(input) = weak.upgrade() {
                    input.set_visible(w.selected() == 2);
                }
            });
            panel.append(&label("The source describes modes 0 and 1. Other declared mode numbers remain available without assuming their hardware behavior."));
            Box::new(move || {
                Ok(value_map(
                    &key,
                    Value::Int(match pick.selected() {
                        0 => 0,
                        1 => 1,
                        _ => {
                            input.update();
                            input.value() as i64
                        }
                    }),
                ))
            })
        }
        "eraser_button_override" => {
            // Stable Linux input UAPI button identifiers; no event device is read.
            const BUTTONS: &[(&str, i64)] = &[
                ("Default (0)", 0),
                ("Stylus button (BTN_STYLUS)", 0x14b),
                ("Second stylus button (BTN_STYLUS2)", 0x14c),
                ("Third stylus button (BTN_STYLUS3)", 0x149),
                ("Touch contact (BTN_TOUCH)", 0x14a),
                ("Left button (BTN_LEFT)", 0x110),
                ("Right button (BTN_RIGHT)", 0x111),
                ("Middle button (BTN_MIDDLE)", 0x112),
                ("Side button (BTN_SIDE)", 0x113),
                ("Extra button (BTN_EXTRA)", 0x114),
            ];
            let value = match initial {
                Some(Value::Int(v)) => v,
                _ => 0,
            };
            let index = BUTTONS
                .iter()
                .position(|(_, code)| *code == value)
                .unwrap_or(BUTTONS.len());
            let mut labels = BUTTONS.iter().map(|(name, _)| *name).collect::<Vec<_>>();
            labels.push("Other numeric button code");
            let pick = choices(panel, &labels, index as u32);
            pick.set_widget_name("device-eraser-button");
            let other = gtk::Box::new(gtk::Orientation::Vertical, 6);
            panel.append(&other);
            let get = controls::form(setting, Some(Value::Int(value)), &other);
            other.set_visible(index == BUTTONS.len());
            let weak = other.downgrade();
            pick.connect_selected_notify(move |w| {
                if let Some(other) = weak.upgrade() {
                    other.set_visible(w.selected() as usize == BUTTONS.len());
                }
            });
            panel.append(&label("Used with eraser mode 1. Choose a real button code; keyboard keys and fake BTN_TOOL identifiers are not button overrides. This stores a draft only."));
            Box::new(move || {
                Ok(value_map(
                    &key,
                    if let Some((_, code)) = BUTTONS.get(pick.selected() as usize) {
                        Value::Int(*code)
                    } else {
                        get()?
                    },
                ))
            })
        }
        "accel_profile" => {
            let value = Acceleration::parse(text).unwrap_or(Acceleration::Default);
            let index = match value {
                Acceleration::Default => 0,
                Acceleration::Adaptive => 1,
                Acceleration::Flat => 2,
                Acceleration::Custom(_) => 3,
            };
            let picker = choices(
                panel,
                &["Device Default", "Adaptive", "Flat", "Custom Curve"],
                index,
            );
            picker.set_widget_name("device-acceleration-profile");
            let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
            panel.append(&body);
            body.set_visible(index == 3);
            let get = curve(
                &body,
                match value {
                    Acceleration::Custom(c) => c,
                    _ => Curve {
                        step: 1.,
                        points: vec![0., 1.],
                    },
                },
            );
            let weak = body.downgrade();
            picker.connect_selected_notify(move |p| {
                if let Some(b) = weak.upgrade() {
                    b.set_visible(p.selected() == 3);
                }
            });
            Box::new(move || {
                let a = match picker.selected() {
                    1 => Acceleration::Adaptive,
                    2 => Acceleration::Flat,
                    3 => Acceleration::Custom(get()?),
                    _ => Acceleration::Default,
                };
                Ok(value_map(&key, Value::String(a.encode()?)))
            })
        }
        "scroll_points" => {
            let custom = gtk::CheckButton::with_label("Use Custom Scroll Points");
            custom.set_widget_name("device-scroll-points");
            custom.set_active(!text.is_empty());
            panel.append(&custom);
            let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
            body.set_sensitive(custom.is_active());
            panel.append(&body);
            let get = curve(
                &body,
                Curve::parse(text).unwrap_or(Curve {
                    step: 1.,
                    points: vec![0., 1.],
                }),
            );
            custom.connect_toggled(move |w| body.set_sensitive(w.is_active()));
            Box::new(move || {
                Ok(value_map(
                    &key,
                    Value::String(if custom.is_active() {
                        get()?.encode()?
                    } else {
                        String::new()
                    }),
                ))
            })
        }
        "scroll_method" | "tap_button_map" => {
            let get = strings(
                panel,
                if key == "scroll_method" {
                    &[
                        ("", "Device Default"),
                        ("no_scroll", "Disabled"),
                        ("2fg", "Two Fingers"),
                        ("edge", "Edge"),
                        ("on_button_down", "While Button Is Held"),
                    ]
                } else {
                    &[
                        ("", "Device Default"),
                        ("lrm", "Left / Right / Middle"),
                        ("lmr", "Left / Middle / Right"),
                    ]
                },
                text,
            );
            Box::new(move || Ok(value_map(&key, Value::String(get()))))
        }
        "drag_lock" | "drag_3fg" | "share_states" => {
            let n = match initial {
                Some(Value::Int(n)) => n,
                _ => 0,
            };
            let names = match key.as_str() {
                "drag_lock" => ["Disabled", "Timeout Lock", "Sticky Lock"],
                "drag_3fg" => ["Disabled", "Three-Finger Drag", "Four-Finger Drag"],
                _ => [
                    "State Sharing Mode 0",
                    "State Sharing Mode 1",
                    "State Sharing Mode 2",
                ],
            };
            let picker = choices(panel, &names, n as u32);
            // Registry integer meanings differ by field/library version; show the exact value.
            panel.append(&label("Values 0, 1 and 2 retain the pinned field semantics. Hardware/libinput applicability remains unverified."));
            Box::new(move || Ok(value_map(&key, Value::Int(i64::from(picker.selected())))))
        }
        "transform" => {
            let n = match initial {
                Some(Value::Int(n)) => n,
                _ => -1,
            };
            if !(-1..=7).contains(&n) {
                panel.append(&label("Existing transform is outside the usual range; use the numeric control to preserve it."));
                let get = controls::form(setting, Some(Value::Int(n)), panel);
                return Box::new(move || Ok(value_map(&key, get()?)));
            }
            let picker = choices(
                panel,
                &[
                    "Unset / Inherit",
                    "Normal",
                    "90°",
                    "180°",
                    "270°",
                    "Flipped",
                    "Flipped 90°",
                    "Flipped 180°",
                    "Flipped 270°",
                ],
                (n + 1) as u32,
            );
            Box::new(move || {
                Ok(value_map(
                    &key,
                    Value::Int(i64::from(picker.selected()) - 1),
                ))
            })
        }
        "tags" => {
            if hyprland_editor_core::device::parse_tags(text).len() > 128 {
                panel.append(&label("This declaration has more than 128 tags. Its saved value is preserved; this builder cannot stage a truncated list."));
                return Box::new(|| {
                    Err(
                        "Tag list exceeds this builder's 128-row limit; no values were staged"
                            .into(),
                    )
                });
            }

            panel.append(&label(
                "Add one tag per row. Binding filters are not recalculated yet.",
            ));
            let rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
            panel.append(&rows);
            let entries = Rc::new(RefCell::new(Vec::<(gtk::Box, gtk::Entry)>::new()));
            let append: Rc<dyn Fn(&str)> = Rc::new({
                let rows = rows.downgrade();
                let entries = entries.clone();
                move |text| {
                    let Some(rows) = rows.upgrade() else {
                        return;
                    };
                    if entries.borrow().len() >= 128 {
                        return;
                    }
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                    let entry = gtk::Entry::builder().text(text).hexpand(true).build();
                    entry.set_widget_name("device-tag");
                    entry.update_property(&[gtk::accessible::Property::Label("Device tag")]);
                    let remove = gtk::Button::with_label("Remove");
                    row.append(&entry);
                    row.append(&remove);
                    rows.append(&row);
                    entries.borrow_mut().push((row.clone(), entry));
                    let rows = rows.downgrade();
                    let row = row.downgrade();
                    let entries = Rc::downgrade(&entries);
                    remove.connect_clicked(move |_| {
                        if let (Some(rows), Some(row), Some(entries)) =
                            (rows.upgrade(), row.upgrade(), entries.upgrade())
                        {
                            entries.borrow_mut().retain(|(r, _)| *r != row);
                            rows.remove(&row);
                        }
                    });
                }
            });
            for tag in hyprland_editor_core::device::parse_tags(text) {
                append(&tag);
            }
            let add = gtk::Button::with_label("Add Tag");
            panel.append(&add);
            let add_row = append.clone();
            add.connect_clicked(move |_| add_row(""));
            Box::new(move || {
                let tags = entries
                    .borrow()
                    .iter()
                    .map(|(_, e)| e.text().to_string())
                    .collect::<Vec<_>>();
                let text = hyprland_editor_core::device::encode_tags(&tags)?;
                Ok(value_map(&key, Value::String(text)))
            })
        }
        "kb_file" => {
            let entry = gtk::Entry::new();
            entry.set_text(text);
            panel.append(&entry);
            let choose = gtk::Button::with_label("Choose Keymap File…");
            panel.append(&choose);
            let window = window.downgrade();
            let output = entry.downgrade();
            choose.connect_clicked(move |_| {
                let Some(win) = window.upgrade() else {
                    return;
                };
                let output = output.clone();
                gtk::FileDialog::builder()
                    .title("Select a keymap path reference")
                    .build()
                    .open(Some(&win), None::<&gtk::gio::Cancellable>, move |result| {
                        if let (Ok(file), Some(entry)) = (result, output.upgrade()) {
                            if let Some(path) =
                                file.path().and_then(|p| p.to_str().map(str::to_string))
                            {
                                entry.set_text(&path);
                            }
                        }
                    });
            });
            panel.append(&label(
                "This stores a path reference only. The file is not read, executed or compiled.",
            ));
            Box::new(move || Ok(value_map(&key, Value::String(entry.text().into()))))
        }
        _ => {
            if key.starts_with("kb_") {
                panel.append(&label("Load Keyboard Choices for installed layout/variant/model/option pickers. Existing identifiers remain editable and preserved."));
            }
            let get = controls::form(setting, initial, panel);
            Box::new(move || Ok(value_map(&key, get()?)))
        }
    }
}
