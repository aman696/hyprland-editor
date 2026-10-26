//! Reuse graphical family forms for an isolated selected source declaration.
use super::{label, source_analysis::Model, source_changes, worker, workflow, Shared};
use adw::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub(super) fn open(
    parent: &gtk::Window,
    model: &Model,
    order: usize,
    state: &Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    let analysis = model.borrow().clone().expect("Selected source analysis");
    let api = analysis
        .report
        .events
        .iter()
        .find(|e| e.order == order)
        .unwrap()
        .api
        .clone();
    let weak_analysis = std::sync::Arc::downgrade(&analysis);
    let window_slot = Rc::new(RefCell::new(None::<gtk::glib::WeakRef<adw::Window>>));
    let slot = window_slot.clone();
    let window = source_changes::open_custom(
        parent,
        model,
        state,
        refresh,
        Box::new(move |panel, changed, publish| {
            panel.append(&label("Edit this declaration's fields, close the editor, then Compare and Keep. Its arguments are rebuilt from this captured snapshot, so computed links become explicit values. Comments are retained. Separate family drafts stay separate."));
            let feedback = label("Loading graphical fields…");
            feedback.set_widget_name("source-family-editor-status");
            panel.append(&feedback);
            let edit = gtk::Button::with_label("Edit declaration fields…");
            edit.set_widget_name("source-family-editor-open");
            edit.set_sensitive(false);
            panel.append(&edit);
            let Some(analysis) = weak_analysis.upgrade() else {
                return;
            };
            let program = analysis.program.clone();
            let sandbox = super::initial_state();
            let loaded = Rc::new(std::cell::Cell::new(false));
            let button = edit.downgrade();
            let feedback_weak = feedback.downgrade();
            let target_state = sandbox.clone();
            let target_loaded = loaded.clone();
            let expected_analysis = weak_analysis.clone();
            let target_slot = slot.clone();
            let getter_state = sandbox.clone();
            let getter_program = program.clone();
            let getter_loaded = loaded.clone();
            let getter: source_changes::RequestGetter = Rc::new(move || {
                if !getter_loaded.get() {
                    return Err("The declaration form is unavailable".into());
                }
                Ok(worker::Request::CompareStructuredDeclaration {
                    program: getter_program.clone(),
                    event_order: order,
                    target: Box::new(workflow::captured_workspace(&getter_state.borrow())?),
                })
            });
            let publish_loaded = publish.clone();
            workflow::background(
                move || {
                    worker::run(worker::Request::InspectStructuredDeclaration {
                        program,
                        event_order: order,
                    })
                },
                move |result| {
                    if expected_analysis.upgrade().is_none()
                        || target_slot
                            .borrow()
                            .as_ref()
                            .and_then(|w| w.upgrade())
                            .is_none_or(|w| !w.is_visible())
                    {
                        return;
                    }
                    let Some(feedback) = feedback_weak.upgrade() else {
                        return;
                    };
                    match result {
                        Ok(worker::Response::Parsed(report)) => {
                            let mut s = target_state.borrow_mut();
                            let version = report.draft.version.clone();
                            s.draft = report.draft;
                            s.devices.insert(version.clone(), report.devices);
                            s.bindings.insert(version.clone(), report.bindings);
                            s.workspace_rules
                                .insert(version.clone(), report.workspace_rules);
                            s.rules.insert(version.clone(), report.rules);
                            s.animations.insert(version.clone(), report.animations);
                            s.gestures.insert(version, report.gestures);
                            drop(s);
                            target_loaded.set(true);
                            if let Some(button) = button.upgrade() {
                                button.set_sensitive(true);
                            }
                            feedback.set_text(
                                "Ready. Edit the selected declaration's fields, then Compare.",
                            );
                            publish_loaded(
                                Some(getter),
                                "Edit fields, then compare the proposed source change.",
                            );
                        }
                        Ok(_) => {
                            feedback.set_text("The parser did not return graphical fields.");
                            publish_loaded(None, "Graphical declaration unavailable");
                        }
                        Err(error) => {
                            feedback.set_text(&error);
                            publish_loaded(None, &error);
                        }
                    }
                },
            );
            let slot = slot.clone();
            edit.connect_clicked(move |_| {
                if !loaded.get() {
                    return;
                }
                let Some(parent) = slot.borrow().as_ref().and_then(|w| w.upgrade()) else {
                    return;
                };
                changed();
                let child = match api.as_str() {
                    "hl.device" => super::devices::show(&parent, &sandbox, &feedback, None),
                    "hl.bind" => super::bindings::show(&parent, &sandbox, &feedback),
                    "hl.workspace_rule" => super::workspaces::show(&parent, &sandbox, &feedback),
                    "hl.window_rule" | "hl.layer_rule" => {
                        super::rules::show(&parent, &sandbox, &feedback)
                    }
                    "hl.curve" | "hl.animation" => {
                        super::animations::show(&parent, &sandbox, &feedback)
                    }
                    "hl.gesture" => super::gestures::show(&parent, &sandbox, &feedback),
                    _ => return,
                };
                child.set_title(Some("Edit captured declaration"));
                child.set_destroy_with_parent(true);
            });
        }),
    );
    window.set_title(Some("Edit captured declaration"));
    *window_slot.borrow_mut() = Some(window.downgrade());
    window
}

#[cfg(test)]
pub(super) fn test_workflow(parent: &adw::ApplicationWindow, state: &Shared) {
    fn descendants(root: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut result = vec![];
        let mut child = root.first_child();
        while let Some(widget) = child {
            result.push(widget.clone());
            result.extend(descendants(&widget));
            child = widget.next_sibling();
        }
        result
    }
    fn named<T: gtk::glib::object::IsA<gtk::Widget> + gtk::glib::object::IsClass>(
        root: &gtk::Widget,
        name: &str,
    ) -> T {
        descendants(root)
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap_or_else(|| panic!("Missing {name}"))
            .downcast()
            .ok()
            .unwrap()
    }
    fn wait(test: impl Fn() -> bool) {
        let start = std::time::Instant::now();
        let context = gtk::glib::MainContext::default();
        while !test() {
            while context.pending() {
                context.iteration(false);
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(12));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    let version = state.borrow().draft.version.clone();
    let fingerprint = state.borrow().source_graphs[&version].fingerprint.clone();
    for attempt in 0..2 {
        let analysis =
            super::source_graph::restore(&state.borrow().source_graphs[&version].bundle, &version)
                .unwrap();
        let order = analysis
            .report
            .events
            .iter()
            .find(|e| e.api == "hl.device")
            .unwrap()
            .order;
        let model: Model = Rc::new(RefCell::new(Some(std::sync::Arc::new(analysis))));
        let window = open(parent.upcast_ref(), &model, order, state, Rc::new(|| {}));
        let edit: gtk::Button = named(window.upcast_ref(), "source-family-editor-open");
        wait(|| edit.is_sensitive());
        let generation = state.borrow().generation;
        edit.emit_clicked();
        let child = gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<adw::Window>().ok())
            .find(|w| w.is_visible() && w.transient_for().as_ref() == Some(window.upcast_ref()))
            .unwrap();
        for expander in descendants(child.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Expander>().ok())
        {
            expander.set_expanded(true);
        }
        let entry: gtk::Entry = named(child.upcast_ref(), "device-name");
        entry.set_text("renamed mouse");
        descendants(child.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|w| w.label().as_deref() == Some("Update Device Match"))
            .unwrap()
            .emit_clicked();
        assert_eq!(
            state.borrow().generation,
            generation,
            "Editing the isolated form changed the project before Keep"
        );
        for button in descendants(child.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .filter(|w| w.label().as_deref() == Some("Save Draft"))
        {
            assert!(!button.is_visible());
        }
        child.close();
        let compare: gtk::Button = named(window.upcast_ref(), "source-change-compare");
        let keep: gtk::Button = named(window.upcast_ref(), "source-change-keep");
        compare.emit_clicked();
        wait(|| keep.is_sensitive());
        let output: gtk::TextView = named(window.upcast_ref(), "source-change-comparison");
        let buffer = output.buffer();
        assert!(buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .contains("renamed mouse"));
        let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
        assert!(minimum <= 350, "Source family editor needs {minimum}px");
        keep.emit_clicked();
        wait(|| state.borrow().generation != generation);
        assert_ne!(
            state.borrow().source_graphs[&version].fingerprint,
            fingerprint
        );
        assert!(
            state
                .borrow()
                .devices
                .get(&version)
                .is_none_or(Vec::is_empty),
            "A source edit silently changed the separate device draft"
        );
        let current =
            super::source_graph::restore(&state.borrow().source_graphs[&version].bundle, &version)
                .unwrap();
        assert!(current
            .program
            .documents
            .iter()
            .any(|d| d.source.contains("renamed mouse")));
        window.close();
        drop(window);
        drop(model);
        if attempt == 0 {
            state.borrow_mut().undo_last();
            assert_eq!(
                state.borrow().source_graphs[&version].fingerprint,
                fingerprint
            );
        }
    }
}
