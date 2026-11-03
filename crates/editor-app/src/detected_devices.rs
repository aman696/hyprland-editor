//! Read-only device observations fill existing match controls only by selection.
use super::{label, monitors::compact, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::device::Kind;
use hyprland_editor_discovery::{Device, DeviceCategory, DeviceSnapshot};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub(super) type Provider = Arc<dyn Fn() -> Result<DeviceSnapshot, String> + Send + Sync>;
const PROMPT: &str = "Choose a device…";
fn category(category: DeviceCategory) -> &'static str {
    match category {
        DeviceCategory::Pointer => "Pointer (may include touchpads)",
        DeviceCategory::Keyboard => "Keyboard",
        DeviceCategory::Tablet => "Tablet",
        DeviceCategory::Touch => "Touch",
        DeviceCategory::Switch => "Switch (observation only)",
    }
}
fn visible(text: &str) -> String {
    hyprland_editor_core::source::display_path_bytes(text.as_bytes())
}
fn row(device: &Device) -> String {
    format!(
        "{} · {}",
        category(device.category),
        visible(
            device
                .name
                .as_deref()
                .filter(|name| !name.is_empty())
                .unwrap_or("Name not reported")
        )
    )
}

pub(super) fn append(
    parent: &gtk::Box,
    window: &adw::Window,
    state: &Shared,
    target: (&gtk::Entry, &gtk::DropDown),
    provider: Provider,
) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.append(&label("Refresh to read device names. Choose a named device to fill the form, then use Update Device Match. Pointer observations do not distinguish touchpads; choose that category manually when appropriate."));
    let refresh = gtk::Button::with_label("Refresh Devices");
    refresh.set_widget_name("device-detect-refresh");
    body.append(&refresh);
    let model = gtk::StringList::new(&[PROMPT]);
    let picker = gtk::DropDown::builder()
        .model(&model)
        .enable_search(true)
        .build();
    picker.set_widget_name("device-detected-picker");
    picker.update_property(&[gtk::accessible::Property::Label("Detected device")]);
    compact(&picker);
    picker.set_sensitive(false);
    body.append(&picker);
    let details = gtk::Label::builder()
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .xalign(0.)
        .max_width_chars(36)
        .build();
    details.set_widget_name("device-detected-details");
    body.append(&details);
    let status = label("No device snapshot. Refresh when needed; observations may become stale.");
    status.set_widget_name("device-detected-status");
    body.append(&status);
    let section = gtk::Expander::builder()
        .label("Detected Devices")
        .child(&body)
        .build();
    section.set_widget_name("device-detected-section");
    parent.append(&section);
    let records = Rc::new(RefCell::new(None::<DeviceSnapshot>));
    let updating = Rc::new(Cell::new(false));
    let observed = records.clone();
    let changing = updating.clone();
    let target_name = target.0.downgrade();
    let target_kind = target.1.downgrade();
    let feedback = status.clone();
    let description = details.clone();
    picker.connect_selected_notify(move |picker| {
        if changing.get() { return; }
        let observed = observed.borrow();
        let record = picker.selected().checked_sub(1).and_then(|index| observed.as_ref()?.devices.get(index as usize));
        let Some(record) = record else {description.set_text(""); return;};
        let mut text = category(record.category).to_owned();
        for (heading, value) in [("Role", &record.role), ("Parent name", &record.owner_name), ("Reported layout", &record.layout), ("Active keymap", &record.active_keymap)] {
            if let Some(value) = value {text.push_str(&format!("\n{heading}: {}", visible(value)));}
        }
        description.set_text(&text);
        let Some(name) = record.configurable_name() else {
            feedback.set_text("Observation only: this record has no confirmed device-override name. The match form was not changed.");return;
        };
        if let (Some(entry), Some(kind)) = (target_name.upgrade(), target_kind.upgrade()) {
            let selected_kind = match record.category {
                DeviceCategory::Pointer => Kind::Pointer,
                DeviceCategory::Keyboard => Kind::Keyboard,
                DeviceCategory::Tablet => Kind::Tablet,
                DeviceCategory::Touch => Kind::Touch,
                DeviceCategory::Switch => return,
            };
            entry.set_text(name);
            kind.set_selected(Kind::ALL.iter().position(|kind| *kind == selected_kind).unwrap_or(0) as u32);
            feedback.set_text("Name and reported category filled in the form. Use Update Device Match to keep them in the draft. Hardware settings were not changed.");
        }
    });

    let state = state.clone();
    let window = window.downgrade();
    refresh.connect_clicked(move |button| {
        let Some(window) = window.upgrade() else {return;};
        let generation = state.borrow().generation;
        let version = state.borrow().draft.version.clone();
        updating.set(true);
        records.borrow_mut().take();
        model.splice(0, model.n_items(), &[PROMPT]);
        picker.set_selected(0);
        picker.set_sensitive(false);
        details.set_text("");
        updating.set(false);
        button.set_sensitive(false);
        status.set_text("Reading the selected desktop's device names…");
        let provider = provider.clone();
        let button = button.downgrade();
        let window = window.downgrade();
        let state = state.clone();
        let records = records.clone();
        let model = model.clone();
        let picker = picker.clone();
        let status = status.clone();
        let updating = updating.clone();
        workflow::background(move || provider(), move |result| {
            if let Some(button) = button.upgrade() {button.set_sensitive(true);}
            if window.upgrade().is_none_or(|window| !window.is_visible()) {return;}
            if state.borrow().generation != generation || state.borrow().draft.version != version {
                status.set_text("The draft changed during discovery. Refresh again for this revision.");return;
            }
            match result {
                Ok(snapshot) => {
                    let rows = snapshot.devices.iter().map(row).collect::<Vec<_>>();
                    updating.set(true);
                    model.splice(1, 0, &rows.iter().map(String::as_str).collect::<Vec<_>>());
                    picker.set_selected(0);
                    picker.set_sensitive(!rows.is_empty());
                    status.set_text(&format!("{} records read from PID {} in {} ms; {} categories were not reported. Select a device; no draft values were copied.{}", rows.len(), snapshot.identity.pid, snapshot.duration_ms, snapshot.unreported_categories.len(), if snapshot.version.is_none(){" This build has no audited pack; these are read-only observations."}else{""}));
                    *records.borrow_mut() = Some(snapshot);
                    updating.set(false);
                }
                Err(error) => status.set_text(&format!("Device discovery unavailable: {error}")),
            }
        });
    });
}
