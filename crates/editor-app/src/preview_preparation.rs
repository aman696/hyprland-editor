//! Explicit private scenario selection and lazy fidelity review. Never launches
//! a compositor, copies source code into a runtime or changes draft values.
use super::{
    projection_worker::{self, Cancellation},
    Shared,
};
use adw::prelude::*;
use hyprland_editor_projection::{Fidelity, Output, Scenario};
use std::{cell::RefCell, rc::Rc};

fn text(value: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(value));
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_xalign(0.0);
    label
}
pub(super) fn connect(button: &gtk::Button, parent: &adw::ApplicationWindow, state: &Shared) {
    let (parent, state) = (parent.downgrade(), state.clone());
    let current: Rc<RefCell<gtk::glib::WeakRef<adw::Window>>> =
        Rc::new(RefCell::new(gtk::glib::WeakRef::new()));
    button.connect_clicked(move |_| {
        if let Some(window) = current.borrow().upgrade() { if window.is_visible() { window.present(); return; } }
        let Some(parent) = parent.upgrade() else { return; };
        let (snapshot, generation) = {
            let state = state.borrow();
            (super::workflow::captured_workspace(&state), state.generation)
        };
        let snapshot = match snapshot { Ok(snapshot) => snapshot, Err(error) => { super::notice(&parent, "Preparation unavailable", &error); return; } };
        let window = adw::Window::builder().title("Prepare preview draft").transient_for(&parent).modal(true).default_width(620).default_height(620).width_request(350).build();
        current.borrow_mut().set(Some(&window));
        let cached = current.clone();
        window.connect_destroy(move |window| clear_cached(&cached, window));
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&adw::HeaderBar::new());
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.set_margin_top(16); content.set_margin_bottom(16); content.set_margin_start(16); content.set_margin_end(16);
        let scroll = gtk::ScrolledWindow::builder().child(&content).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
        root.append(&scroll);
        content.append(&text("Prepare a sample configuration and review what it can represent. This changes nothing on your desktop. Rendered preview is not available yet."));
        let layout = gtk::DropDown::from_strings(&["One display · 1280 × 720", "Two displays · 640 × 480 each"]);
        layout.update_property(&[gtk::accessible::Property::Label("Sample display layout")]);
        content.append(&text("Sample layout")); content.append(&layout);
        let mappings = gtk::StringList::new(&["Omit from sample", "Display 1"]);
        let mut monitors = vec![];
        for (index, monitor) in snapshot.monitors.iter().enumerate() {
            content.append(&text(&format!("Monitor {} · {}", index + 1, monitor.id)));
            let choice = gtk::DropDown::new(Some(mappings.clone()), None::<gtk::Expression>);
            choice.update_property(&[gtk::accessible::Property::Label(&format!("Monitor {} representation", index + 1))]);
            content.append(&choice); monitors.push((monitor.id.clone(), choice));
        }
        let mut devices = vec![];
        for (index, device) in snapshot.devices.iter().enumerate() {
            content.append(&text(&format!("Device {} · {}", index + 1, device.id)));
            let choice = gtk::DropDown::from_strings(&["Omit from sample", "Synthetic device"]);
            choice.update_property(&[gtk::accessible::Property::Label(&format!("Device {} representation", index + 1))]);
            content.append(&choice); devices.push((device.id.clone(), choice));
        }
        let choices: Vec<_> = monitors.iter().map(|(_, choice)| choice.clone()).collect();
        layout.connect_selected_notify(move |layout| {
            let names: &[&str] = if layout.selected() == 0 { &["Omit from sample", "Display 1"] } else { &["Omit from sample", "Display 1", "Display 2"] };
            mappings.splice(0, mappings.n_items(), names);
            for choice in &choices { choice.set_selected(0); }
        });
        let prepare = gtk::Button::with_label("Prepare and review"); prepare.add_css_class("suggested-action");
        content.append(&prepare);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        footer.set_margin_top(12); footer.set_margin_bottom(12); footer.set_margin_start(16); footer.set_margin_end(16);
        let close = gtk::Button::with_label("Close"); footer.append(&close); root.append(&footer);
        window.set_content(Some(&root));
        let cancellation = Cancellation::default();
        let stop = cancellation.clone(); window.connect_destroy(move |_| stop.cancel());
        let stop = cancellation.clone(); let cached = current.clone(); window.connect_close_request(move |window| { clear_cached(&cached, window); stop.cancel(); gtk::glib::Propagation::Proceed });
        let weak = window.downgrade(); close.connect_clicked(move |_| { if let Some(window) = weak.upgrade() { window.close(); } });
        let snapshot = Rc::new(RefCell::new(Some(std::sync::Arc::new(snapshot))));
        let selected_layers:Rc<RefCell<Option<hyprland_editor_projection::layers::Selection>>>=Rc::new(RefCell::new(None));
        let inputs=text("Using pending draft values only");
        let review_inputs=gtk::Button::with_label("Review setting values");
        content.insert_child_after(&inputs,Some(&layout));content.insert_child_after(&review_inputs,Some(&inputs));
        let (source_snapshot,source_layers,source_parent,source_state,source_label,source_prepare,source_button)=(snapshot.clone(),selected_layers.clone(),window.downgrade(),Rc::downgrade(&state),inputs.downgrade(),prepare.downgrade(),review_inputs.downgrade());
        review_inputs.connect_clicked(move |_| {
            let (Some(parent),Some(state))=(source_parent.upgrade(),source_state.upgrade())else{return;};
            if state.borrow().generation!=generation{if let Some(label)=source_label.upgrade(){label.set_text("Draft changed. Close preparation and open it again.");}return;}
            let Some(workspace)=source_snapshot.borrow().as_ref().cloned()else{return;};
            if let Some(prepare)=source_prepare.upgrade(){prepare.set_sensitive(false);}
            if let Some(button)=source_button.upgrade(){button.set_sensitive(false);}
            let (selected,label)=(source_layers.clone(),source_label.clone());let (prepare,button)=(source_prepare.clone(),source_button.clone());
            super::preview_layers::open(&parent,&state,generation,workspace,move|selection|{*selected.borrow_mut()=Some(selection);if let Some(label)=label.upgrade(){label.set_text("Using reviewed setting choices");}},move||{if let Some(prepare)=prepare.upgrade(){prepare.set_sensitive(true);}
                if let Some(button)=button.upgrade(){button.set_sensitive(true);}});
        });
        let (weak_window, weak_content, weak_close, weak_state) = (window.downgrade(), content.downgrade(), close.downgrade(), Rc::downgrade(&state));
        prepare.connect_clicked(move |prepare| {
            let Some(state) = weak_state.upgrade() else { return; };
            if state.borrow().generation != generation {
                if let Some(content) = weak_content.upgrade() { content.append(&text("Draft changed. Close this review and prepare again.")); }
                prepare.set_sensitive(false); return;
            }
            let Some(workspace) = snapshot.borrow_mut().take() else { return; };
            let workspace=std::sync::Arc::try_unwrap(workspace).unwrap_or_else(|workspace|workspace.as_ref().clone());
            let two = layout.selected() == 1;
            let mut scenario = Scenario::default();
            if two { scenario.outputs = (1..=2).map(|id| Output { name: format!("HEADLESS-{id}"), width: 640, height: 480 }).collect(); }
            for (id, choice) in &monitors { if choice.selected() != 0 { scenario.monitor_targets.insert(id.clone(), format!("HEADLESS-{}", choice.selected())); } }
            for (index, (id, choice)) in devices.iter().enumerate() { if choice.selected() == 1 { scenario.device_targets.insert(id.clone(), format!("hye-lab-device-{index}")); } }
            prepare.set_sensitive(false);
            if let Some(content) = weak_content.upgrade() { clear(&content); content.append(&text("Preparing the sample in a restricted background worker…")); }
            if let Some(close) = weak_close.upgrade() { close.set_label("Cancel preparation"); }
            let (weak_window, weak_content, weak_close, weak_state) = (weak_window.clone(), weak_content.clone(), weak_close.clone(), weak_state.clone());
            let stop = cancellation.clone();
            let layers=selected_layers.borrow().clone();
            super::workflow::background(move || if layers.is_some(){projection_worker::stage_selected(workspace, scenario, layers, stop)}else{projection_worker::stage(workspace,scenario,stop)}, move |result| {
                let (Some(window), Some(content), Some(state)) = (weak_window.upgrade(), weak_content.upgrade(), weak_state.upgrade()) else { return; };
                if !window.is_visible() { return; }
                clear(&content);
                if let Some(close) = weak_close.upgrade() { close.set_label("Close"); }
                if state.borrow().generation != generation { content.append(&text("Draft changed during preparation. Close this review and prepare again.")); return; }
                match result {
                    Err(error) => content.append(&text(&format!("Preparation stopped: {error}"))),
                    Ok(artifact) => {
                        content.append(&text(if artifact.report().layers.is_some(){"Sample prepared from your reviewed scalar inputs. Other source settings, effective defaults and runtime behavior remain unresolved."}else{"Sample prepared. Review the approximations and omissions below. Imported sources and default values are not included."}));
                        if let Some(layers)=&artifact.report().layers{content.append(&text(&format!("{} captured · {} pending · {} unset · {} source arguments preserved",layers.captured_values,layers.pending_values,layers.unset_values,layers.source_value_roundtrips)));}
                        let entries: Vec<String> = artifact.report().entries.iter().map(|entry| format!("{} · {}\n{}", fidelity(entry.fidelity), entry.subject, entry.reason)).collect();
                        content.append(&text(&format!("{} review items · {} virtual displays", entries.len(), artifact.report().scenario.outputs.len())));
                        let search = gtk::SearchEntry::new(); search.set_placeholder_text(Some("Find review items")); content.append(&search);
                        let strings = gtk::StringList::new(&entries.iter().map(String::as_str).collect::<Vec<_>>());
                        let filter = gtk::StringFilter::new(Some(gtk::PropertyExpression::new(gtk::StringObject::static_type(), None::<gtk::Expression>, "string")));
                        let search_filter = filter.clone(); search.connect_search_changed(move |search| search_filter.set_search(Some(&search.text())));
                        let filtered = gtk::FilterListModel::new(Some(strings), Some(filter));
                        let factory = gtk::SignalListItemFactory::new();
                        factory.connect_setup(|_, item| { let item = item.downcast_ref::<gtk::ListItem>().unwrap(); let label = text(""); label.set_margin_top(8); label.set_margin_bottom(8); item.set_child(Some(&label)); });
                        factory.connect_bind(|_, item| { let item = item.downcast_ref::<gtk::ListItem>().unwrap(); let value = item.item().unwrap().downcast::<gtk::StringObject>().unwrap(); item.child().unwrap().downcast::<gtk::Label>().unwrap().set_text(&value.string()); });
                        let list = gtk::ListView::new(Some(gtk::NoSelection::new(Some(filtered))), Some(factory));
                        // An inner viewport keeps GTK's list virtualization active;
                        // a large report does not create one widget per declaration.
                        content.append(&gtk::ScrolledWindow::builder().child(&list).min_content_height(240).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build());
                        search.grab_focus();
                    }
                }
            });
        });
        window.present();
    });
}
fn clear(content: &gtk::Box) {
    while let Some(child) = content.first_child() {
        content.remove(&child);
    }
}
fn clear_cached(current: &RefCell<gtk::glib::WeakRef<adw::Window>>, window: &adw::Window) {
    if current
        .borrow()
        .upgrade()
        .is_some_and(|active| active == *window)
    {
        current.borrow().set(None);
    }
}
fn fidelity(value: Fidelity) -> &'static str {
    match value {
        Fidelity::SampleApproximation => "Approximation",
        Fidelity::MockedEffect => "Inert action",
        Fidelity::OmittedUntrusted => "Omitted",
        Fidelity::HardwareUnverified => "Hardware unverified",
        Fidelity::Unsupported => "Unresolved",
    }
}

#[cfg(test)]
pub(super) fn test_preparation(parent: &adw::ApplicationWindow) {
    use hyprland_editor_core::{device, monitor, Value, BASELINE};
    use std::time::{Duration, Instant};
    fn descendants(root: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut widgets = vec![root.clone()];
        let mut child = root.first_child();
        while let Some(current) = child {
            widgets.extend(descendants(&current));
            child = current.next_sibling();
        }
        widgets
    }
    fn settle() {
        let context = gtk::glib::MainContext::default();
        for _ in 0..5 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn window() -> adw::Window {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<adw::Window>().ok())
            .find(|w| w.title().as_deref() == Some("Prepare preview draft"))
            .unwrap()
    }
    fn prepare(window: &adw::Window) {
        descendants(window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|w| w.label().as_deref() == Some("Prepare and review"))
            .unwrap()
            .emit_clicked();
    }
    fn wait_for(window: &adw::Window, expected: &str) {
        let start = Instant::now();
        loop {
            settle();
            let labels: Vec<_> = descendants(window.upcast_ref())
                .into_iter()
                .filter_map(|w| w.downcast::<gtk::Label>().ok())
                .map(|label| label.text().to_string())
                .collect();
            if labels.iter().any(|label| label.contains(expected)) {
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(6),
                "Missing {expected}: {labels:?}"
            );
        }
    }
    let state = super::initial_state();
    let temporary = super::test_support::TempDir::new("hye-preview-layer-ui");
    let original = temporary.path().join("source.lua");
    std::fs::write(&original, "hl.config{general={border_size=7}}\n").unwrap();
    let captured = hyprland_editor_core::storage::read_source(&original).unwrap();
    std::fs::remove_file(&original).unwrap();
    let catalogue = hyprland_editor_core::catalogue();
    let report = hyprland_editor_core::lua::parse(&captured.content, BASELINE, &catalogue).unwrap();
    let document =
        hyprland_editor_core::lua::LuaDocument::from_report(captured.content, report, &catalogue)
            .unwrap();
    state
        .borrow_mut()
        .sources
        .insert(BASELINE.into(), std::sync::Arc::new(document));
    state
        .borrow_mut()
        .source_origins
        .insert(BASELINE.into(), captured.identity);
    state
        .borrow_mut()
        .draft
        .values
        .insert("general:border_size".into(), Value::Int(3));
    state
        .borrow_mut()
        .monitors
        .insert(BASELINE.into(), vec![monitor::Rule::new("sample-monitor")]);
    state.borrow_mut().devices.insert(
        BASELINE.into(),
        vec![device::Spec::new("sample-device", "original-device")],
    );
    let before = super::workflow::captured_workspace(&state.borrow())
        .unwrap()
        .to_json(&hyprland_editor_core::catalogue())
        .unwrap();
    let button = gtk::Button::new();
    connect(&button, parent, &state);
    button.emit_clicked();
    settle();
    let review = window();
    button.emit_clicked();
    assert_eq!(window(), review);
    review.set_default_size(350, 620);
    settle();
    let choices: Vec<_> = descendants(review.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::DropDown>().ok())
        .collect();
    assert_eq!(choices.len(), 3);
    choices[0].set_selected(1);
    choices[1].set_selected(2);
    choices[2].set_selected(1);
    prepare(&review);
    wait_for(&review, "Sample prepared");
    assert_eq!(
        super::workflow::captured_workspace(&state.borrow())
            .unwrap()
            .to_json(&hyprland_editor_core::catalogue())
            .unwrap(),
        before
    );
    let search = descendants(review.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::SearchEntry>().ok())
        .unwrap();
    search.set_text("monitor");
    let list = descendants(review.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::ListView>().ok())
        .unwrap();
    let start = Instant::now();
    while list.model().unwrap().n_items() != 1 {
        settle();
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "Search did not filter the report"
        );
    }
    settle();
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&review));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, review.width() as f64, review.height() as f64);
        review
            .renderer()
            .unwrap()
            .render_texture(snapshot.to_node().unwrap(), None)
            .save_to_png(format!("{path}.projection.png"))
            .unwrap();
    }
    let close = descendants(review.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .find(|b| b.label().as_deref() == Some("Close"))
        .unwrap();
    assert!(close.is_visible());
    let bounds = close.compute_bounds(&review).unwrap();
    assert!(bounds.y() >= 0.0 && bounds.y() + bounds.height() <= review.height() as f32);
    assert!(review.width() <= 350);
    review.close();
    settle();
    button.emit_clicked();
    settle();
    let review = window();
    descendants(review.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .find(|b| b.label().as_deref() == Some("Review setting values"))
        .unwrap()
        .emit_clicked();
    let layers = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<adw::Window>().ok())
        .find(|w| w.title().as_deref() == Some("Review setting values"))
        .unwrap();
    layers.set_default_size(350, 660);
    wait_for(&layers, "Captured values were reconstructed");
    let search = descendants(layers.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::SearchEntry>().ok())
        .unwrap();
    search.set_text("general:border_size");
    let list = descendants(layers.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::ListView>().ok())
        .unwrap();
    let start = Instant::now();
    while list.model().unwrap().n_items() != 1 {
        settle();
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    settle();
    let choice = descendants(layers.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::DropDown>().ok())
        .unwrap();
    assert_eq!(choice.selected(), 1);
    choice.set_selected(2);
    settle();
    let keep = descendants(layers.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .find(|b| b.label().as_deref() == Some("Use these inputs"))
        .unwrap();
    let bounds = keep.compute_bounds(&layers).unwrap();
    assert!(bounds.y() >= 0.0 && bounds.y() + bounds.height() <= layers.height() as f32);
    assert!(layers.width() <= 350);
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&layers));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, layers.width() as f64, layers.height() as f64);
        layers
            .renderer()
            .unwrap()
            .render_texture(snapshot.to_node().unwrap(), None)
            .save_to_png(format!("{path}.projection-layers.png"))
            .unwrap();
    }
    keep.emit_clicked();
    settle();
    assert!(!layers.is_visible());
    prepare(&review);
    wait_for(&review, "1 captured");
    assert_eq!(
        super::workflow::captured_workspace(&state.borrow())
            .unwrap()
            .to_json(&catalogue)
            .unwrap(),
        before
    );
    assert!(!original.exists());
    review.close();
    settle();
    // A partial source can be sampled only after acknowledging its limitations.
    std::fs::write(&original, "hl.config{general={border_size=7}}; mystery()\n").unwrap();
    let captured = hyprland_editor_core::storage::read_source(&original).unwrap();
    std::fs::remove_file(&original).unwrap();
    let report = hyprland_editor_core::lua::parse(&captured.content, BASELINE, &catalogue).unwrap();
    state.borrow_mut().sources.insert(
        BASELINE.into(),
        std::sync::Arc::new(
            hyprland_editor_core::lua::LuaDocument::from_report(
                captured.content,
                report,
                &catalogue,
            )
            .unwrap(),
        ),
    );
    state
        .borrow_mut()
        .source_origins
        .insert(BASELINE.into(), captured.identity);
    state.borrow_mut().generation += 1;
    let partial_before = super::workflow::captured_workspace(&state.borrow())
        .unwrap()
        .to_json(&catalogue)
        .unwrap();
    button.emit_clicked();
    settle();
    let review = window();
    descendants(review.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .find(|b| b.label().as_deref() == Some("Review setting values"))
        .unwrap()
        .emit_clicked();
    let layers = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<adw::Window>().ok())
        .find(|w| w.title().as_deref() == Some("Review setting values"))
        .unwrap();
    wait_for(&layers, "Some source behavior is unresolved");
    descendants(layers.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .find(|b| b.label().as_deref() == Some("Use available captured values"))
        .unwrap()
        .emit_clicked();
    let keep = descendants(layers.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .find(|b| b.label().as_deref() == Some("Use these inputs"))
        .unwrap();
    keep.emit_clicked();
    wait_for(&layers, "Acknowledge incomplete analysis");
    assert!(layers.is_visible());
    descendants(layers.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::CheckButton>().ok())
        .unwrap()
        .set_active(true);
    keep.emit_clicked();
    settle();
    prepare(&review);
    wait_for(&review, "1 captured");
    assert_eq!(
        super::workflow::captured_workspace(&state.borrow())
            .unwrap()
            .to_json(&catalogue)
            .unwrap(),
        partial_before
    );
    assert!(!original.exists());
    review.close();
    settle();
    button.emit_clicked();
    settle();
    let review = window();
    prepare(&review);
    state.borrow_mut().generation += 1;
    wait_for(&review, "Draft changed during preparation");
    review.close();
    settle();
    button.emit_clicked();
    settle();
    let review = window();
    prepare(&review);
    review.close();
    settle();
}
