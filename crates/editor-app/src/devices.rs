//! Focused, versioned device drafts. Selecting a device category is an author hint, not detection.
#[cfg(not(test))]
use super::worker;
use super::{
    device_forms, label,
    monitors::{choices, compact},
    workflow, Shared,
};
use adw::prelude::*;
use hyprland_editor_core::{
    device::{self, Kind, Spec},
    xkb::Registry,
    Setting,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn read_registry() -> Result<Registry, String> {
    let xml = hyprland_editor_core::storage::read_source(std::path::Path::new(
        "/usr/share/X11/xkb/rules/evdev.xml",
    ))?
    .content;
    #[cfg(not(test))]
    {
        match worker::run(worker::Request::ParseRegistry { xml })? {
            worker::Response::Registry(r) => {
                r.validate()?;
                Ok(r)
            }
            _ => Err("Unexpected keyboard-registry parser response".into()),
        }
    }
    // The unit-test executable is a test harness, not the application's CLI.
    // Real worker isolation/protocol is exercised by tests/parser_worker.rs.
    #[cfg(test)]
    {
        hyprland_editor_core::xkb::parse(&xml)
    }
}
fn current(state: &Shared) -> Vec<Spec> {
    let s = state.borrow();
    s.devices.get(&s.draft.version).cloned().unwrap_or_default()
}
fn commit(state: &Shared, specs: Vec<Spec>, labels: (&gtk::Label, &gtk::Label)) -> bool {
    let result = state.borrow_mut().replace_devices(specs);
    match result {
        Ok(()) => {
            super::summary(state, labels.0);
            labels
                .1
                .set_text("Device draft updated; hardware was not changed.");
            true
        }
        Err(e) => {
            labels.1.set_text(&e);
            false
        }
    }
}
struct FieldContext<'a> {
    window: &'a adw::Window,
    state: &'a Shared,
    status: &'a gtk::Label,
    feedback: &'a gtk::Label,
    registry: Option<Arc<Registry>>,
}
fn field(panel: &gtk::Box, index: usize, setting: &Setting, ctx: FieldContext<'_>) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }
    let specs = current(ctx.state);
    let Some(spec) = specs.get(index) else {
        panel.append(&label("Add a device draft to begin."));
        return;
    };
    let relevant = device::relevance(&setting.lua_key);
    if spec.kind != Kind::Unknown && !relevant.contains(&spec.kind) {
        panel.append(&label("This field usually belongs to another device type. It remains available; actual hardware capabilities are unknown."));
    }
    let enabled = gtk::CheckButton::with_label("Set This Field");
    enabled.set_widget_name("device-field-enabled");
    enabled.set_active(spec.fields.contains_key(&setting.lua_key));
    panel.append(&enabled);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_sensitive(enabled.is_active());
    let get = device_forms::form(setting, spec, &body, ctx.window, ctx.registry);
    panel.append(&body);
    enabled.connect_toggled(move |w| body.set_sensitive(w.is_active()));
    let findings = label(&spec.findings().join("\n"));
    panel.append(&findings);
    let apply = gtk::Button::with_label("Update Device Draft");
    apply.add_css_class("suggested-action");
    panel.append(&apply);
    let id = spec.id.clone();
    let key = setting.lua_key.clone();
    let state = ctx.state.clone();
    let status = ctx.status.clone();
    let feedback = ctx.feedback.clone();
    apply.connect_clicked(move |_| {
        let mut specs = current(&state);
        let Some(spec) = specs.iter_mut().find(|s| s.id == id) else {
            return;
        };
        if enabled.is_active() {
            match get() {
                Ok(values) => spec.fields.extend(values),
                Err(e) => {
                    feedback.set_text(&e);
                    return;
                }
            }
        } else {
            spec.fields.remove(&key);
        }
        let messages = spec.findings().join("\n");
        if commit(&state, specs, (&status, &feedback)) {
            findings.set_text(&messages);
        }
    });
    let details = gtk::Expander::builder().label("Field Details").build();
    details.set_child(Some(&label(&format!("Field: {}\nAudited default: {}\nVersion: {}\nSource: {}\nUnset fields inherit global/device defaults. Device categories are draft hints; they do not certify capability.",setting.lua_key,setting.default_expression,setting.version,setting.source))));
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
            show(&win, &state, &status, None);
        }
    });
}
fn show(
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
    registry: Option<Arc<Registry>>,
) -> adw::Window {
    let win = adw::Window::builder()
        .title("Device Drafts")
        .transient_for(window)
        .modal(true)
        .default_width(720)
        .default_height(760)
        .build();
    win.set_size_request(350, 480);
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
    content.append(&label("Edit one override at a time. These are device declarations; connected devices, effective defaults and hardware behavior are not inferred."));
    let model = gtk::StringList::new(&[]);
    let selected = gtk::DropDown::builder().model(&model).build();
    compact(&selected);
    selected.set_widget_name("device-rule");
    content.append(&selected);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add");
    let duplicate = gtk::Button::with_label("Duplicate");
    let remove = gtk::Button::with_label("Remove");
    for b in [&add, &duplicate, &remove] {
        actions.append(b);
    }
    content.append(&actions);
    let order = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let up = gtk::Button::with_label("↑");
    let down = gtk::Button::with_label("↓");
    let undo = gtk::Button::with_label("Undo");
    up.update_property(&[gtk::accessible::Property::Label("Move device rule up")]);
    down.update_property(&[gtk::accessible::Property::Label("Move device rule down")]);
    for b in [&up, &down, &undo] {
        order.append(b);
    }
    content.append(&order);
    let match_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let name = gtk::Entry::new();
    name.set_widget_name("device-name");
    name.set_placeholder_text(Some("Hyprland device name"));
    match_panel.append(&name);
    let kind = choices(
        &match_panel,
        &Kind::ALL.iter().map(|k| k.label()).collect::<Vec<_>>(),
        0,
    );
    kind.set_widget_name("device-kind");
    match_panel.append(&label("The category guides field relevance and is not a hardware observation. Hyprland converts spaces in a name to hyphens."));
    let target = gtk::Button::with_label("Update Device Match");
    match_panel.append(&target);
    let expander = gtk::Expander::builder()
        .label("Device Name and Category")
        .build();
    expander.set_child(Some(&match_panel));
    content.append(&expander);
    let load = gtk::Button::with_label("Load Keyboard Choices");
    content.append(&load);
    let registry = Rc::new(RefCell::new(registry));
    let fields = device::catalogue(&state.borrow().draft.version);
    let labels: Vec<_> = fields
        .iter()
        .map(|s| super::presentation::setting_name(&s.id))
        .collect();
    let picker = choices(
        &content,
        &labels.iter().map(String::as_str).collect::<Vec<_>>(),
        0,
    );
    picker.set_widget_name("device-field");
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(&panel);
    let feedback = label("");
    feedback.set_widget_name("device-feedback");
    content.append(&feedback);
    let review = gtk::Button::with_label("Review Device Lua");
    content.append(&review);
    let save = gtk::Button::with_label("Save Draft");
    content.append(&save);
    let parent = window.downgrade();
    save.connect_clicked(move |_| {
        if let Some(parent) = parent.upgrade() {
            if let Some(a) = parent.lookup_action("save-draft") {
                a.activate(None);
            }
        }
    });
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    win.set_content(Some(&root));
    let busy = Rc::new(Cell::new(false));
    let refresh: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let panel = panel.clone();
        let selected = selected.clone();
        let model = model.clone();
        let picker = picker.clone();
        let fields = fields.clone();
        let name = name.clone();
        let kind = kind.clone();
        let win = win.downgrade();
        let registry = registry.clone();
        let busy = busy.clone();
        move || {
            if busy.replace(true) {
                return;
            }
            let specs = current(&state);
            let index = (selected.selected() as usize).min(specs.len().saturating_sub(1));
            let names: Vec<String> = specs
                .iter()
                .enumerate()
                .map(|(n, s)| {
                    format!(
                        "{}. {}",
                        n + 1,
                        hyprland_editor_core::source::display_path_bytes(s.name.as_bytes())
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
            selected.set_selected(if specs.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            if let Some(s) = specs.get(index) {
                name.set_text(&s.name);
                kind.set_selected(Kind::ALL.iter().position(|k| *k == s.kind).unwrap_or(0) as u32);
            }
            if let (Some(win), Some(setting)) =
                (win.upgrade(), fields.get(picker.selected() as usize))
            {
                field(
                    &panel,
                    index,
                    setting,
                    FieldContext {
                        window: &win,
                        state: &state,
                        status: &status,
                        feedback: &feedback,
                        registry: registry.borrow().clone(),
                    },
                );
            }
            busy.set(false);
        }
    });
    for p in [&selected, &picker] {
        let weak = Rc::downgrade(&refresh);
        p.connect_selected_notify(move |_| {
            if let Some(refresh) = weak.upgrade() {
                refresh();
            }
        });
    }
    for (b, op) in [
        (&add, 0),
        (&duplicate, 1),
        (&remove, 2),
        (&up, 3),
        (&down, 4),
    ] {
        let state = state.clone();
        let selected = selected.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let refresh = refresh.clone();
        b.connect_clicked(move |_| {
            let mut specs = current(&state);
            let index = selected.selected() as usize;
            match op {
                0 | 1 => {
                    if specs.len() >= device::MAX_DEVICES {
                        feedback.set_text("The device draft already has 128 declarations.");
                        return;
                    }
                    let id = (0..)
                        .map(|n| format!("device-{n}"))
                        .find(|id| !specs.iter().any(|s| s.id == *id))
                        .unwrap();
                    let mut spec = if op == 1 {
                        let Some(s) = specs.get(index) else {
                            return;
                        };
                        s.clone()
                    } else {
                        Spec::new(&id, "new-device")
                    };
                    spec.id = id;
                    specs.push(spec);
                }
                2 if index < specs.len() => {
                    specs.remove(index);
                }
                3 if index > 0 && index < specs.len() => specs.swap(index, index - 1),
                4 if index + 1 < specs.len() => specs.swap(index, index + 1),
                _ => return,
            }
            if commit(&state, specs, (&status, &feedback)) {
                refresh();
                selected.set_selected(match op {
                    0 | 1 => current(&state).len().saturating_sub(1) as u32,
                    3 => (index - 1) as u32,
                    4 => (index + 1) as u32,
                    _ => selected.selected(),
                });
            }
        });
    }
    let state_c = state.clone();
    let selected_c = selected.clone();
    let refresh_c = refresh.clone();
    let status_c = status.clone();
    let feedback_c = feedback.clone();
    target.connect_clicked(move |_| {
        let mut specs = current(&state_c);
        let Some(s) = specs.get_mut(selected_c.selected() as usize) else {
            return;
        };
        s.name = name.text().into();
        s.kind = Kind::ALL[kind.selected() as usize];
        if commit(&state_c, specs, (&status_c, &feedback_c)) {
            refresh_c();
        }
    });
    let state_c = state.clone();
    let status_c = status.clone();
    let refresh_c = refresh.clone();
    undo.connect_clicked(move |_| {
        state_c.borrow_mut().undo_last();
        super::summary(&state_c, &status_c);
        refresh_c();
    });
    let panel_c = panel.clone();
    let feedback_c = feedback.clone();
    let win_c = win.downgrade();
    let refresh_c = refresh.clone();
    load.connect_clicked(move|b|{
        b.set_sensitive(false);panel_c.set_sensitive(false);feedback_c.set_text("Reading installed evdev metadata; controls will rebuild from the staged draft…");let button=b.downgrade();let panel=panel_c.downgrade();let win=win_c.clone();let feedback=feedback_c.clone();let registry=registry.clone();let refresh=refresh_c.clone();
        workflow::background(read_registry,move|result:Result<Registry,String>|{if win.upgrade().is_none(){return;}
if let Some(b)=button.upgrade(){b.set_sensitive(true);}
if let Some(p)=panel.upgrade(){p.set_sensitive(true);}match result{Ok(r)=>{feedback.set_text(&format!("Loaded {} layouts and {} option groups from installed evdev metadata. Hardware layout was not changed.",r.layouts.len(),r.groups.len()));*registry.borrow_mut()=Some(Arc::new(r));refresh();},Err(e)=>feedback.set_text(&format!("Keyboard choices unavailable: {e}. Existing identifiers remain editable.")),}});
    });
    let state_c = state.clone();
    let win_c = win.downgrade();
    review.connect_clicked(move |_|{let s=state_c.borrow();let specs=s.devices.get(&s.draft.version).map(Vec::as_slice).unwrap_or(&[]);let text=match device::export_lua(specs,&s.draft.version){Ok(t)=>t,Err(_)=>return};let Some(parent)=win_c.upgrade()else{return;};let review=adw::Window::builder().title("Review Device Lua").transient_for(&parent).modal(true).default_width(680).default_height(500).build();let b=gtk::Box::new(gtk::Orientation::Vertical,12);b.append(&label("Ordered device draft declarations. Copying does not apply them. This fragment must not replace a full configuration."));let view=gtk::TextView::builder().editable(false).monospace(true).wrap_mode(gtk::WrapMode::WordChar).build();view.buffer().set_text(&text);b.append(&gtk::ScrolledWindow::builder().child(&view).vexpand(true).build());let copy=gtk::Button::with_label("Copy Reviewed Device Lua");b.append(&copy);let weak=review.downgrade();copy.connect_clicked(move |_|{if let Some(win)=weak.upgrade(){win.clipboard().set_text(&text);}});review.set_content(Some(&b));review.present();});
    refresh();
    win.present();
    win
}

#[cfg(test)]
pub(super) fn test_editor(window: &adw::ApplicationWindow) {
    fn descendants(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut all = vec![];
        let mut child = w.first_child();
        while let Some(w) = child {
            all.push(w.clone());
            all.extend(descendants(&w));
            child = w.next_sibling();
        }
        all
    }
    fn click(win: &adw::Window, text: &str) {
        descendants(win.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(text))
            .unwrap_or_else(|| panic!("button {text}"))
            .emit_clicked();
    }
    fn pick(win: &adw::Window, name: &str) -> gtk::DropDown {
        descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap()
            .downcast()
            .unwrap()
    }
    let xml = include_str!("../tests/fixtures/xkb-registry.xml");
    let registry = Arc::new(hyprland_editor_core::xkb::parse(xml).unwrap());
    let setting = device::catalogue(hyprland_editor_core::BASELINE)
        .into_iter()
        .find(|s| s.lua_key == "kb_rules")
        .unwrap();
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let value = hyprland_editor_core::Value::String("x".repeat(16384));
    let get = super::controls::form(&setting, Some(value.clone()), &panel);
    assert_eq!(get().unwrap(), value);
    for (key, value) in [
        (
            "tags",
            std::iter::repeat_n("tag", 129)
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "kb_layout",
            std::iter::repeat_n("us", 17).collect::<Vec<_>>().join(","),
        ),
    ] {
        let mut spec = Spec::new("oversized", "keyboard");
        spec.fields
            .insert(key.into(), hyprland_editor_core::Value::String(value));
        let setting = device::catalogue(hyprland_editor_core::BASELINE)
            .into_iter()
            .find(|s| s.lua_key == key)
            .unwrap();
        let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let parent = adw::Window::new();
        let get = device_forms::form(&setting, &spec, &body, &parent, Some(registry.clone()));
        assert!(get().is_err());
        parent.destroy();
    }
    let state = super::initial_state();
    let status = label("");
    let win = show(window, &state, &status, Some(registry.clone()));
    click(&win, "Add");
    assert_eq!(current(&state).len(), 1);
    assert!(state.borrow().dirty());
    let target = descendants(win.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Expander>().ok())
        .find(|e| e.label().as_deref() == Some("Device Name and Category"))
        .unwrap();
    target.set_expanded(true);
    let name = descendants(win.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "device-name")
        .unwrap()
        .downcast::<gtk::Entry>()
        .unwrap();
    name.set_text("Example USB keyboard");
    pick(&win, "device-kind").set_selected(1);
    click(&win, "Update Device Match");
    target.set_expanded(false);
    assert_eq!(current(&state)[0].normalized_name(), "Example-USB-keyboard");
    let fields = device::catalogue(&state.borrow().draft.version);
    assert_eq!(fields.len(), 47);
    let property = pick(&win, "device-field");
    for (index, setting) in fields.iter().enumerate() {
        property.set_selected(index as u32);
        let enabled = descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == "device-field-enabled")
            .unwrap()
            .downcast::<gtk::CheckButton>()
            .unwrap();
        enabled.set_active(true);
        click(&win, "Update Device Draft");
        assert!(
            current(&state)[0].fields.contains_key(&setting.lua_key),
            "{}: {}",
            setting.id,
            descendants(win.upcast_ref())
                .into_iter()
                .find(|w| w.widget_name() == "device-feedback")
                .unwrap()
                .downcast::<gtk::Label>()
                .unwrap()
                .text()
        );
    }
    // Newly discovered tablet-tool controls use the same draft/undo path.
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "eraser_button_mode")
            .unwrap() as u32,
    );
    pick(&win, "device-eraser-mode").set_selected(2);
    let mode = descendants(win.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "device-eraser-other-mode")
        .unwrap()
        .downcast::<gtk::SpinButton>()
        .unwrap();
    mode.set_value(5.0);
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["eraser_button_mode"],
        hyprland_editor_core::Value::Int(5)
    );
    state.borrow_mut().undo_last();
    assert_eq!(
        current(&state)[0].fields["eraser_button_mode"],
        hyprland_editor_core::Value::Int(0)
    );
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "eraser_button_override")
            .unwrap() as u32,
    );
    pick(&win, "device-eraser-button").set_selected(1);
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["eraser_button_override"],
        hyprland_editor_core::Value::Int(331)
    );
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "pressure_range_min")
            .unwrap() as u32,
    );
    let automatic = descendants(win.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "device-pressure-default")
        .unwrap()
        .downcast::<gtk::CheckButton>()
        .unwrap();
    automatic.set_active(false);
    let pressure = descendants(win.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "device-pressure-value")
        .unwrap()
        .downcast::<gtk::SpinButton>()
        .unwrap();
    pressure.set_value(0.25);
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["pressure_range_min"],
        hyprland_editor_core::Value::Float(0.25)
    );
    automatic.set_active(true);
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["pressure_range_min"],
        hyprland_editor_core::Value::Float(-1.0)
    );
    // Opening a preserved negative sentinel must not normalize it on its own.
    let setting = fields
        .iter()
        .find(|s| s.lua_key == "pressure_range_max")
        .unwrap();
    let mut sentinel = Spec::new("sentinel", "Pen");
    sentinel.fields.insert(
        "pressure_range_max".into(),
        hyprland_editor_core::Value::Float(-0.5),
    );
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let parent = adw::Window::new();
    let get = device_forms::form(setting, &sentinel, &body, &parent, None);
    assert_eq!(
        get().unwrap()["pressure_range_max"],
        hyprland_editor_core::Value::Float(-0.5)
    );
    parent.destroy();
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "accel_profile")
            .unwrap() as u32,
    );
    pick(&win, "device-acceleration-profile").set_selected(3);
    click(&win, "Add Curve Point");
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["accel_profile"],
        hyprland_editor_core::Value::String("custom 1 0 1 1".into())
    );
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "scroll_points")
            .unwrap() as u32,
    );
    descendants(win.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "device-scroll-points")
        .unwrap()
        .downcast::<gtk::CheckButton>()
        .unwrap()
        .set_active(true);
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["scroll_points"],
        hyprland_editor_core::Value::String("1 0 1".into())
    );
    property.set_selected(fields.iter().position(|s| s.lua_key == "tags").unwrap() as u32);
    click(&win, "Add Tag");
    click(&win, "Add Tag");
    let tags: Vec<_> = descendants(win.upcast_ref())
        .into_iter()
        .filter(|w| w.widget_name() == "device-tag")
        .map(|w| w.downcast::<gtk::Entry>().unwrap())
        .collect();
    tags[0].set_text("work");
    tags[1].set_text("test,tag");
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["tags"],
        hyprland_editor_core::Value::String("work,test\\,tag".into())
    );
    // Linked layout builder stages both fields and preserves positional variants.
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "kb_layout")
            .unwrap() as u32,
    );
    click(&win, "Add Keyboard Layout");
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["kb_layout"],
        hyprland_editor_core::Value::String("us,us".into())
    );
    assert_eq!(
        current(&state)[0].fields["kb_variant"],
        hyprland_editor_core::Value::String(",".into())
    );
    let variant_index = fields
        .iter()
        .position(|s| s.lua_key == "kb_variant")
        .unwrap() as u32;
    property.set_selected(variant_index);
    descendants(win.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "device-field-enabled")
        .unwrap()
        .downcast::<gtk::CheckButton>()
        .unwrap()
        .set_active(false);
    click(&win, "Update Device Draft");
    assert!(!current(&state)[0].fields.contains_key("kb_variant"));
    click(&win, "Undo");
    assert_eq!(
        current(&state)[0].fields["kb_variant"],
        hyprland_editor_core::Value::String(",".into())
    );
    // Single-selection XKB groups clear the earlier option when the next is selected.
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "kb_options")
            .unwrap() as u32,
    );
    let group = descendants(win.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Expander>().ok())
        .find(|e| e.label().as_deref() == Some("Caps Behavior"))
        .unwrap();
    group.set_expanded(true);
    let options: Vec<_> = descendants(group.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::CheckButton>().ok())
        .collect();
    options[0].set_active(true);
    options[1].set_active(true);
    assert!(!options[0].is_active());
    drop(options);
    group.set_expanded(false);
    assert!(group.child().is_none());
    group.set_expanded(true);
    let reopened: Vec<_> = descendants(group.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::CheckButton>().ok())
        .collect();
    assert!(!reopened[0].is_active());
    assert!(reopened[1].is_active());
    click(&win, "Update Device Draft");
    assert_eq!(
        current(&state)[0].fields["kb_options"],
        hyprland_editor_core::Value::String("caps:ctrl".into())
    );
    click(&win, "Duplicate");
    assert_eq!(current(&state).len(), 2);
    click(&win, "↑");
    assert_eq!(pick(&win, "device-rule").selected(), 0);
    let before = current(&state);
    click(&win, "Remove");
    assert_eq!(current(&state).len(), 1);
    click(&win, "Undo");
    assert_eq!(current(&state), before);
    click(&win, "Review Device Lua");
    let review = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Window>().ok())
        .find(|w| w.title().as_deref() == Some("Review Device Lua"))
        .unwrap();
    let text = descendants(review.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::TextView>().ok())
        .unwrap();
    let buffer = text.buffer();
    let lua = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
    assert_eq!(
        hyprland_editor_core::lua::parse(
            &lua,
            &state.borrow().draft.version,
            &state.borrow().catalogue
        )
        .unwrap()
        .devices
        .len(),
        2
    );
    review.destroy();
    let context = gtk::glib::MainContext::default();
    let settle = || {
        for _ in 0..25 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    // The actual installed registry path is metadata only and is parsed in the worker.
    if std::path::Path::new("/usr/share/X11/xkb/rules/evdev.xml").is_file() {
        let load = descendants(win.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some("Load Keyboard Choices"))
            .unwrap();
        load.emit_clicked();
        let start = std::time::Instant::now();
        while !load.is_sensitive() {
            while context.pending() {
                context.iteration(false);
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(5));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let feedback = descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == "device-feedback")
            .unwrap()
            .downcast::<gtk::Label>()
            .unwrap();
        assert!(
            feedback.text().starts_with("Loaded "),
            "{}",
            feedback.text()
        );
    }
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
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "kb_layout")
            .unwrap() as u32,
    );
    win.set_default_size(720, 760);
    settle();
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
        let node = snapshot.to_node().unwrap();
        win.renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(format!("{path}.devices.png"))
            .unwrap();
    }
    property.set_selected(
        fields
            .iter()
            .position(|s| s.lua_key == "pressure_range_max")
            .unwrap() as u32,
    );
    settle();
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
        let node = snapshot.to_node().unwrap();
        win.renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(format!("{path}.tablet-tool.png"))
            .unwrap();
    }
    win.destroy();
}
