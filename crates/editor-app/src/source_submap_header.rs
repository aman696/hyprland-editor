//! Focused name/reset controls preserve complex submap callbacks verbatim.
use super::{label, source_analysis::Model, source_changes, worker, workflow, Shared};
use adw::prelude::*;
use std::{cell::RefCell, rc::Rc};
pub(super) fn open(
    parent: &gtk::Window,
    model: &Model,
    event_order: usize,
    state: &Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    let analysis = model.borrow().clone().expect("Selected submap analysis");
    let weak_analysis = std::sync::Arc::downgrade(&analysis);
    let window_slot = Rc::new(RefCell::new(None::<gtk::glib::WeakRef<adw::Window>>));
    let slot = window_slot.clone();
    let window = source_changes::open_custom(
        parent,
        model,
        state,
        refresh,
        Box::new(move |panel, changed, publish| {
            panel.append(&label("Change the name or automatic transition while retaining the complete callback, module loads and nested declarations. Other references to the old name are preserved; update related shortcuts separately. Compare lists affected submap contexts."));
            let name = gtk::Entry::new();
            name.set_widget_name("source-submap-name");
            name.set_max_length(4096);
            name.set_sensitive(false);
            name.update_property(&[gtk::accessible::Property::Label("Submap name")]);
            panel.append(&label("Submap name"));
            panel.append(&name);
            let has_reset =
                gtk::CheckButton::with_label("Set automatic transition after a binding");
            has_reset.set_widget_name("source-submap-has-reset");
            has_reset.set_sensitive(false);
            if let Some(label) = has_reset.child().and_downcast::<gtk::Label>() {
                label.set_wrap(true);
            }
            panel.append(&has_reset);
            let reset = gtk::Entry::builder()
                .placeholder_text("reset or another submap name")
                .max_length(4096)
                .sensitive(false)
                .build();
            reset.set_widget_name("source-submap-reset");
            reset.update_property(&[gtk::accessible::Property::Label(
                "Automatic transition target",
            )]);
            panel.append(&reset);
            let feedback = label("Loading captured submap values…");
            feedback.set_widget_name("source-submap-status");
            panel.append(&feedback);
            let changed_name = changed.clone();
            name.connect_changed(move |_| changed_name());
            let changed_reset = changed.clone();
            reset.connect_changed(move |_| changed_reset());
            let reset_weak = reset.downgrade();
            has_reset.connect_toggled(move |check| {
                if let Some(reset) = reset_weak.upgrade() {
                    reset.set_sensitive(check.is_active());
                }
                changed();
            });
            let Some(analysis) = weak_analysis.upgrade() else {
                return;
            };
            let program = analysis.program.clone();
            let getter_program = program.clone();
            let getter_name = name.downgrade();
            let getter_reset = reset.downgrade();
            let getter_has_reset = has_reset.downgrade();
            let getter: source_changes::RequestGetter = Rc::new(move || {
                let name = getter_name.upgrade().ok_or("The name editor is closed")?;
                let reset = getter_reset
                    .upgrade()
                    .ok_or("The transition editor is closed")?;
                let has_reset = getter_has_reset
                    .upgrade()
                    .ok_or("The transition editor is closed")?;
                Ok(worker::Request::CompareSubmapHeader {
                    program: getter_program.clone(),
                    event_order,
                    name: name.text().into(),
                    reset: has_reset.is_active().then(|| reset.text().into()),
                })
            });
            let name = name.downgrade();
            let reset = reset.downgrade();
            let has_reset = has_reset.downgrade();
            let feedback = feedback.downgrade();
            workflow::background(
                move || {
                    worker::run(worker::Request::InspectSubmapHeader {
                        program,
                        event_order,
                    })
                },
                move |result| {
                    if weak_analysis.upgrade().is_none()
                        || slot
                            .borrow()
                            .as_ref()
                            .and_then(|w| w.upgrade())
                            .is_none_or(|w| !w.is_visible())
                    {
                        return;
                    }
                    let (Some(name), Some(reset), Some(has_reset), Some(feedback)) = (
                        name.upgrade(),
                        reset.upgrade(),
                        has_reset.upgrade(),
                        feedback.upgrade(),
                    ) else {
                        return;
                    };
                    match result {
                        Ok(worker::Response::SubmapHeader(header)) => {
                            name.set_text(&header.name);
                            reset.set_text(header.reset.as_deref().unwrap_or("reset"));
                            has_reset.set_active(header.reset.is_some());
                            name.set_sensitive(true);
                            has_reset.set_sensitive(true);
                            reset.set_sensitive(header.reset.is_some());
                            feedback.set_text(&if header.occurrences>1{format!("This source line runs {} times. Changes affect every reached instance; the callback stays unchanged.",header.occurrences)}else{"The callback stays unchanged, including captured module loads.".into()});
                            publish(
                                Some(getter),
                                "Edit the submap name or transition, then Compare.",
                            );
                        }
                        Ok(_) => {
                            feedback.set_text("The parser did not return submap values.");
                            publish(None, "Submap values unavailable");
                        }
                        Err(error) => {
                            feedback.set_text(&error);
                            publish(None, &error);
                        }
                    }
                },
            );
        }),
    );
    window.set_title(Some("Edit submap name and reset"));
    *window_slot.borrow_mut() = Some(window.downgrade());
    window
}

#[cfg(test)]
pub(super) fn test_workflow(parent: &adw::ApplicationWindow, state: &Shared) {
    fn named<T: gtk::glib::object::IsA<gtk::Widget> + gtk::glib::object::IsClass>(
        root: &gtk::Widget,
        name: &str,
    ) -> T {
        fn find(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
            if root.widget_name() == name {
                return Some(root.clone());
            }
            let mut child = root.first_child();
            while let Some(widget) = child {
                if let Some(found) = find(&widget, name) {
                    return Some(found);
                }
                child = widget.next_sibling();
            }
            None
        }
        find(root, name)
            .unwrap_or_else(|| panic!("Missing {name}"))
            .downcast()
            .ok()
            .unwrap()
    }
    fn wait(test: impl Fn() -> bool) {
        let context = gtk::glib::MainContext::default();
        let start = std::time::Instant::now();
        while !test() {
            while context.pending() {
                context.iteration(false);
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(12));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    let version = state.borrow().draft.version.clone();
    let original = state.borrow().source_graphs[&version].fingerprint.clone();
    for iteration in 0..3 {
        let analysis =
            super::source_graph::restore(&state.borrow().source_graphs[&version].bundle, &version)
                .unwrap();
        let event=analysis.report.events.iter().find(|e|e.api=="hl.define_submap"&&matches!(e.arguments.first(),Some(hyprland_editor_core::lua::replay::Observed::Bytes(bytes))if bytes==b"demo"||bytes==b"renamed-demo")).unwrap().clone();
        let before = hyprland_editor_core::lua::replay::declaration_edit::submap::inspect(
            &analysis.program,
            event.order,
        )
        .unwrap();
        let source = analysis
            .program
            .documents
            .iter()
            .find(|d| d.id == event.document)
            .unwrap()
            .source
            .clone();
        let retained_tail = source[before.callback_span.start..].to_owned();
        let other_sources = analysis
            .program
            .documents
            .iter()
            .filter(|d| d.id != event.document)
            .map(|d| (d.id.clone(), d.source.clone()))
            .collect::<Vec<_>>();
        let model: Model = Rc::new(RefCell::new(Some(std::sync::Arc::new(analysis))));
        let (body, view) = super::source_analysis::build(&model, state, None);
        let inspector = adw::Window::builder()
            .transient_for(parent)
            .content(&body)
            .build();
        inspector.present();
        view.render();
        view.test_call(event.order as u32);
        let window = view.test_submap_header();
        let name: gtk::Entry = named(window.upcast_ref(), "source-submap-name");
        wait(|| name.is_sensitive());
        let has_reset: gtk::CheckButton = named(window.upcast_ref(), "source-submap-has-reset");
        let reset: gtk::Entry = named(window.upcast_ref(), "source-submap-reset");
        let generation = state.borrow().generation;
        name.set_text("renamed-demo");
        has_reset.set_active(iteration != 2);
        if iteration != 2 {
            reset.set_text("reset");
        }
        assert_eq!(reset.is_sensitive(), iteration != 2);
        assert_eq!(state.borrow().generation, generation);
        let compare: gtk::Button = named(window.upcast_ref(), "source-change-compare");
        let keep: gtk::Button = named(window.upcast_ref(), "source-change-keep");
        compare.emit_clicked();
        wait(|| keep.is_sensitive());
        let output: gtk::TextView = named(window.upcast_ref(), "source-change-comparison");
        let buffer = output.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        assert!(
            text.contains("callback expression") && text.contains("renamed-demo"),
            "{text}"
        );
        let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
        assert!(minimum <= 350, "Submap header needs {minimum}px");
        if iteration == 0 {
            if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
                let paintable = gtk::WidgetPaintable::new(Some(&window));
                let node = RefCell::new(None);
                wait(|| {
                    let snapshot = gtk::Snapshot::new();
                    paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
                    *node.borrow_mut() = snapshot.to_node();
                    node.borrow().is_some()
                });
                window
                    .renderer()
                    .unwrap()
                    .render_texture(node.into_inner().unwrap(), None)
                    .save_to_png(format!("{path}.submap-header.png"))
                    .unwrap();
            }
        }
        keep.emit_clicked();
        wait(|| state.borrow().generation != generation);
        window.close();
        inspector.close();
        let restored =
            super::source_graph::restore(&state.borrow().source_graphs[&version].bundle, &version)
                .unwrap();
        let next=restored.report.events.iter().find(|e|e.api=="hl.define_submap"&&matches!(e.arguments.first(),Some(hyprland_editor_core::lua::replay::Observed::Bytes(bytes))if bytes==b"renamed-demo")).unwrap();
        let header = hyprland_editor_core::lua::replay::declaration_edit::submap::inspect(
            &restored.program,
            next.order,
        )
        .unwrap();
        assert_eq!(
            header.reset.as_deref(),
            if iteration == 2 { None } else { Some("reset") }
        );
        let updated = restored
            .program
            .documents
            .iter()
            .find(|d| d.id == event.document)
            .unwrap();
        assert_eq!(&updated.source[header.callback_span.start..], retained_tail);
        for (id, source) in &other_sources {
            assert_eq!(
                &restored
                    .program
                    .documents
                    .iter()
                    .find(|d| d.id == *id)
                    .unwrap()
                    .source,
                source
            );
        }
        assert!(state
            .borrow()
            .bindings
            .get(&version)
            .is_none_or(|draft| draft.is_empty()));
        if iteration == 0 {
            state.borrow_mut().undo_last();
            assert_eq!(state.borrow().source_graphs[&version].fingerprint, original);
        }
    }
}
