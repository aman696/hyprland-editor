//! On-demand read-only output snapshots, explicitly selected into draft controls.
use super::{label, monitors::compact, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_discovery::{Monitor, MonitorSnapshot};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub(super) type Provider = Arc<dyn Fn() -> Result<MonitorSnapshot, String> + Send + Sync>;
const OUTPUT_PROMPT: &str = "Choose an output…";
const MODE_PROMPT: &str = "Choose a reported mode…";

fn chosen(index: u32) -> Option<usize> {
    index.checked_sub(1).map(|index| index as usize)
}

fn modes(monitor: &Monitor) -> Vec<String> {
    let mut result = vec![];
    for raw in monitor.available_modes.as_deref().unwrap_or(&[]) {
        let mode = raw.strip_suffix("Hz").unwrap_or(raw);
        if matches!(
            hyprland_editor_core::monitor::Mode::parse(mode),
            Ok(hyprland_editor_core::monitor::Mode::Dimensions {
                refresh: Some(_),
                ..
            })
        ) && !result.iter().any(|existing| existing == mode)
        {
            result.push(mode.to_owned());
        }
    }
    result
}

pub(super) fn append(
    parent: &gtk::Box,
    window: &adw::Window,
    state: &Shared,
    target: (&gtk::DropDown, &gtk::Entry),
    provider: Provider,
    use_mode: impl Fn(&str, &str) -> Result<(), String> + 'static,
) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.append(&label("Refresh to read active and inactive outputs from the selected desktop. Choosing a name fills the match above; update the match before using a reported mode."));
    let refresh = gtk::Button::with_label("Refresh Outputs");
    refresh.set_widget_name("monitor-detect-refresh");
    body.append(&refresh);
    let output_model = gtk::StringList::new(&[OUTPUT_PROMPT]);
    let output = gtk::DropDown::builder()
        .model(&output_model)
        .enable_search(true)
        .build();
    output.set_widget_name("monitor-detected-output");
    output.set_sensitive(false);
    output.update_property(&[gtk::accessible::Property::Label("Detected output")]);
    compact(&output);
    body.append(&output);
    let details = gtk::Label::builder()
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .xalign(0.)
        .max_width_chars(36)
        .build();
    details.set_widget_name("monitor-detected-details");
    body.append(&details);
    let mode_model = gtk::StringList::new(&[MODE_PROMPT]);
    let mode = gtk::DropDown::builder()
        .model(&mode_model)
        .enable_search(true)
        .build();
    mode.set_widget_name("monitor-detected-mode");
    mode.set_sensitive(false);
    mode.update_property(&[gtk::accessible::Property::Label("Reported mode")]);
    compact(&mode);
    body.append(&mode);
    let use_selected = gtk::Button::with_label("Use Mode in Draft");
    use_selected.set_widget_name("monitor-use-detected-mode");
    use_selected.set_sensitive(false);
    body.append(&use_selected);
    let status = label("No output snapshot. Refresh when needed; observations may become stale.");
    status.set_widget_name("monitor-detected-status");
    body.append(&status);
    let section = gtk::Expander::builder()
        .label("Detected Outputs")
        .child(&body)
        .build();
    section.set_widget_name("monitor-detected-section");
    parent.append(&section);
    let snapshot = Rc::new(RefCell::new(None::<MonitorSnapshot>));
    let mode_values = Rc::new(RefCell::new(Vec::<String>::new()));
    let updating = Rc::new(Cell::new(false));
    output.set_selected(0);
    mode.set_selected(0);

    let target_kind = target.0.downgrade();
    let target_name = target.1.downgrade();
    let records = snapshot.clone();
    let values = mode_values.clone();
    let changing = updating.clone();
    let mode_picker = mode.clone();
    let selected_button = use_selected.clone();
    let description = details.clone();
    let mode_list = mode_model.clone();
    output.connect_selected_notify(move |picker| {
        if changing.get() {return;}
        values.borrow_mut().clear();
        mode_list.splice(0,mode_list.n_items(),&[MODE_PROMPT]);
        mode_picker.set_selected(0);
        mode_picker.set_sensitive(false);
        selected_button.set_sensitive(false);
        let records = records.borrow();
        let Some(record) = chosen(picker.selected()).and_then(|index|records.as_ref()?.monitors.get(index)) else {description.set_text("");return;};
        if let (Some(kind),Some(name)) = (target_kind.upgrade(),target_name.upgrade()) {
            kind.set_selected(1);
            name.set_text(&record.name);
        }
        let current = match (record.width,record.height,record.refresh_rate) {
            (Some(w),Some(h),Some(rate)) => format!("{w}×{h} at {rate:.2} Hz"),
            _ => "Current mode not reported".into(),
        };
        let availability = match record.disabled {Some(true)=>"Inactive",Some(false)=>"Active",None=>"State not reported"};
        let available = modes(record);
        description.set_text(&format!("{availability} · {current}\n{} selectable modes from {} reported entries. This is an observation, not a hardware validation.",available.len(),record.available_modes.as_ref().map_or(0,Vec::len)));
        mode_picker.set_sensitive(!available.is_empty());
        mode_list.splice(1,0,&available.iter().map(String::as_str).collect::<Vec<_>>());
        mode_picker.set_selected(0);
        *values.borrow_mut() = available;
    });
    let values = mode_values.clone();
    let selected_button = use_selected.clone();
    mode.connect_selected_notify(move |picker| {
        selected_button.set_sensitive(
            chosen(picker.selected()).is_some_and(|index| values.borrow().get(index).is_some()),
        );
    });
    let records = snapshot.clone();
    let values = mode_values.clone();
    let output_picker = output.downgrade();
    let mode_picker = mode.downgrade();
    let feedback = status.clone();
    use_selected.connect_clicked(move |_| {
        let (Some(output_picker), Some(mode_picker)) =
            (output_picker.upgrade(), mode_picker.upgrade())
        else {
            return;
        };
        let records = records.borrow();
        let values = values.borrow();
        if let (Some(record), Some(mode)) = (
            records
                .as_ref()
                .and_then(|snapshot| snapshot.monitors.get(chosen(output_picker.selected())?)),
            chosen(mode_picker.selected()).and_then(|index| values.get(index)),
        ) {
            match use_mode(&record.name, mode) {
                Ok(()) => feedback.set_text(
                    "Reported mode kept in the draft. The running display was not changed.",
                ),
                Err(error) => feedback.set_text(&error),
            }
        }
    });

    let window = window.downgrade();
    let state = state.clone();
    refresh.connect_clicked(move |button| {
        let Some(window) = window.upgrade() else {return;};
        let generation = state.borrow().generation;
        let version = state.borrow().draft.version.clone();
        updating.set(true);
        snapshot.borrow_mut().take();
        mode_values.borrow_mut().clear();
        output_model.splice(0,output_model.n_items(),&[OUTPUT_PROMPT]);
        mode_model.splice(0,mode_model.n_items(),&[MODE_PROMPT]);
        output.set_selected(0);
        mode.set_selected(0);
        output.set_sensitive(false);
        mode.set_sensitive(false);
        use_selected.set_sensitive(false);
        details.set_text("");
        updating.set(false);
        button.set_sensitive(false);
        status.set_text("Reading the selected desktop's outputs…");
        let provider = provider.clone();
        let button = button.downgrade();
        let window = window.downgrade();
        let state = state.clone();
        let records = snapshot.clone();
        let output_model = output_model.clone();
        let output = output.clone();
        let status = status.clone();
        let updating = updating.clone();
        workflow::background(move ||provider(),move |result| {
            if let Some(button) = button.upgrade() {button.set_sensitive(true);}
            if window.upgrade().is_none_or(|window|!window.is_visible()) {return;}
            if state.borrow().generation != generation || state.borrow().draft.version != version {
                status.set_text("The draft changed during discovery. Refresh again for this revision.");return;
            }
            match result {
                Ok(observed) => {
                    let names = observed.monitors.iter().map(|monitor| {
                        let name = hyprland_editor_core::source::display_path_bytes(monitor.name.as_bytes());
                        match monitor.description.as_deref().filter(|description|!description.is_empty()) {
                            Some(description) => format!("{name} · {}",hyprland_editor_core::source::display_path_bytes(description.as_bytes())),
                            None => name,
                        }
                    }).collect::<Vec<_>>();
                    updating.set(true);
                    output_model.splice(1,0,&names.iter().map(String::as_str).collect::<Vec<_>>());
                    output.set_selected(0);
                    output.set_sensitive(!names.is_empty());
                    status.set_text(&format!("{} outputs read from PID {} in {} ms. Select an output; no draft values were copied.{}",observed.monitors.len(),observed.identity.pid,observed.duration_ms,if observed.version.is_none(){" This build has no audited pack; these are read-only observations."}else{""}));
                    *records.borrow_mut() = Some(observed);
                    updating.set(false);
                }
                Err(error) => status.set_text(&format!("Output discovery unavailable: {error}")),
            }
        });
    });
}
