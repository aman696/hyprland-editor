//! Reuse graphical family forms for private source edits and insertions.
use super::{label, source_analysis::Model, source_changes, worker, workflow, Shared};
use adw::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Loader = Rc<dyn Fn(Option<String>)>;
type LoaderSlot = Rc<RefCell<Option<std::rc::Weak<dyn Fn(Option<String>)>>>>;

#[derive(Clone)]
enum Place {
    Existing(usize),
    Insert(String),
}
const TEMPLATES: &[(&str, &str, &str)] = &[
    ("Device", "hl.device", "hl.device{name='new-device'}"),
    ("Key binding", "hl.bind", "hl.bind('Q',hl.dsp.no_op())"),
    (
        "Submap",
        "hl.define_submap",
        "hl.define_submap('new-map',function()end)",
    ),
    (
        "Workspace rule",
        "hl.workspace_rule",
        "hl.workspace_rule{workspace='1'}",
    ),
    (
        "Window rule",
        "hl.window_rule",
        "hl.window_rule{match={class='example'},float=true}",
    ),
    (
        "Layer rule",
        "hl.layer_rule",
        "hl.layer_rule{match={namespace='example'},blur=true}",
    ),
    (
        "Animation curve",
        "hl.curve",
        "hl.curve('new-curve',{type='bezier',points={{0.1,0.2},{0.8,0.9}}})",
    ),
    (
        "Animation",
        "hl.animation",
        "hl.animation{leaf='windows',enabled=false}",
    ),
    (
        "Gesture",
        "hl.gesture",
        "hl.gesture{fingers=3,direction='left',action='workspace'}",
    ),
];
pub(super) fn open(
    parent: &gtk::Window,
    model: &Model,
    order: usize,
    state: &Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    editor(parent, model, Place::Existing(order), state, refresh)
}
pub(super) fn add(
    parent: &gtk::Window,
    model: &Model,
    document: String,
    state: &Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    editor(parent, model, Place::Insert(document), state, refresh)
}
fn editor(
    parent: &gtk::Window,
    model: &Model,
    place: Place,
    state: &Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    let analysis = model.borrow().clone().expect("Selected source analysis");
    let initial_api = match &place {
        Place::Existing(order) => Some(
            analysis
                .report
                .events
                .iter()
                .find(|e| e.order == *order)
                .unwrap()
                .api
                .clone(),
        ),
        Place::Insert(_) => None,
    };
    let insertion = matches!(place, Place::Insert(_));
    let weak_analysis = std::sync::Arc::downgrade(&analysis);
    let window_slot = Rc::new(RefCell::new(None::<gtk::glib::WeakRef<adw::Window>>));
    let slot = window_slot.clone();
    let window = source_changes::open_custom(
        parent,
        model,
        state,
        refresh,
        Box::new(move |panel, changed, publish| {
            panel.append(&label(if insertion {"Choose a feature, edit its graphical controls, then Compare and Keep. New declarations are placed at the selected file's end, before its module return. Separate family drafts stay separate."} else {"Edit, add, duplicate or remove declarations in the graphical editor, then Compare and Keep. Arguments are rebuilt from the captured snapshot, so computed links become explicit values. Comments are retained. Separate family drafts stay separate."}));
            let pick = if insertion {
                let mut names = vec!["Choose a feature…"];
                names.extend(TEMPLATES.iter().map(|(name, _, _)| *name));
                let pick = gtk::DropDown::from_strings(&names);
                super::monitors::compact(&pick);
                pick.set_widget_name("source-family-editor-kind");
                pick.set_selected(0);
                panel.append(&pick);
                Some(pick)
            } else {
                None
            };
            let feedback = label(if insertion {
                "Choose a feature to start."
            } else {
                "Loading graphical fields…"
            });
            feedback.set_widget_name("source-family-editor-status");
            panel.append(&feedback);
            let edit = gtk::Button::with_label("Edit declaration fields…");
            edit.set_widget_name("source-family-editor-open");
            edit.set_sensitive(false);
            panel.append(&edit);
            let sandbox = super::initial_state();
            let loaded = Rc::new(Cell::new(false));
            let epoch = Rc::new(Cell::new(0u64));
            let active_api = Rc::new(RefCell::new(None::<String>));
            let pending = Rc::new(Cell::new(false));
            let loader_slot: LoaderSlot = Rc::new(RefCell::new(None));
            let load: Loader = Rc::new({
                let sandbox = sandbox.clone();
                let loaded = loaded.clone();
                let epoch = epoch.clone();
                let active_api = active_api.clone();
                let edit = edit.downgrade();
                let feedback = feedback.downgrade();
                let slot = slot.clone();
                let place = place.clone();
                let changed = changed.clone();
                let publish = publish.clone();
                let weak_analysis = weak_analysis.clone();
                let pending = pending.clone();
                let loader_slot = loader_slot.clone();
                move |api| {
                    changed();
                    loaded.set(false);
                    *active_api.borrow_mut() = api.clone();
                    let current = epoch.get().wrapping_add(1);
                    epoch.set(current);
                    if let Some(edit) = edit.upgrade() {
                        edit.set_sensitive(false);
                    }
                    publish(None, "Choose a feature and load its graphical fields.");
                    if pending.get() {
                        return;
                    }
                    let Some(api) = api else {
                        return;
                    };
                    let Some(analysis) = weak_analysis.upgrade() else {
                        return;
                    };
                    let request = match &place {
                        Place::Existing(order) => worker::Request::InspectStructuredDeclaration {
                            program: analysis.program.clone(),
                            event_order: *order,
                        },
                        Place::Insert(_) => {
                            let Some((_, _, source)) =
                                TEMPLATES.iter().find(|(_, name, _)| *name == api)
                            else {
                                return;
                            };
                            worker::Request::Parse {
                                source: (*source).into(),
                                version: analysis.program.version.clone(),
                            }
                        }
                    };
                    let getter: source_changes::RequestGetter = Rc::new({
                        let sandbox = sandbox.clone();
                        let loaded = loaded.clone();
                        let active_api = active_api.clone();
                        let place = place.clone();
                        let program = analysis.program.clone();
                        move || {
                            if !loaded.get() {
                                return Err("The graphical fields are unavailable".into());
                            }
                            let target = Box::new(workflow::captured_workspace(&sandbox.borrow())?);
                            Ok(match &place {
                                Place::Existing(order) => {
                                    worker::Request::CompareStructuredDeclaration {
                                        program: program.clone(),
                                        event_order: *order,
                                        target,
                                    }
                                }
                                Place::Insert(document) => {
                                    worker::Request::InsertStructuredDeclarations {
                                        program: program.clone(),
                                        document: document.clone(),
                                        api: active_api
                                            .borrow()
                                            .clone()
                                            .ok_or("Choose a feature")?,
                                        target,
                                    }
                                }
                            })
                        }
                    });
                    pending.set(true);
                    let pending = pending.clone();
                    let loader_slot = loader_slot.clone();
                    let active_api = active_api.clone();
                    let sandbox = sandbox.clone();
                    let loaded = loaded.clone();
                    let epoch = epoch.clone();
                    let edit = edit.clone();
                    let feedback = feedback.clone();
                    let slot = slot.clone();
                    let publish = publish.clone();
                    let weak_analysis = weak_analysis.clone();
                    workflow::background(
                        move || worker::run(request),
                        move |result| {
                            pending.set(false);
                            if weak_analysis.upgrade().is_none()
                                || slot
                                    .borrow()
                                    .as_ref()
                                    .and_then(|w| w.upgrade())
                                    .is_none_or(|w| !w.is_visible())
                            {
                                return;
                            }
                            if epoch.get() != current {
                                let next = active_api.borrow().clone();
                                if let Some(load) = loader_slot
                                    .borrow()
                                    .as_ref()
                                    .and_then(std::rc::Weak::upgrade)
                                {
                                    load(next);
                                }
                                return;
                            }
                            let Some(feedback) = feedback.upgrade() else {
                                return;
                            };
                            match result {
                                Ok(worker::Response::Parsed(report)) => {
                                    let mut s = sandbox.borrow_mut();
                                    let version = report.draft.version.clone();
                                    s.draft = report.draft;
                                    s.undo = Default::default();
                                    s.devices.clear();
                                    s.bindings.clear();
                                    s.workspace_rules.clear();
                                    s.rules.clear();
                                    s.animations.clear();
                                    s.gestures.clear();
                                    s.devices.insert(version.clone(), report.devices);
                                    s.bindings.insert(version.clone(), report.bindings);
                                    s.workspace_rules
                                        .insert(version.clone(), report.workspace_rules);
                                    s.rules.insert(version.clone(), report.rules);
                                    s.animations.insert(version.clone(), report.animations);
                                    s.gestures.insert(version, report.gestures);
                                    drop(s);
                                    loaded.set(true);
                                    if let Some(edit) = edit.upgrade() {
                                        edit.set_sensitive(true);
                                    }
                                    feedback.set_text("Ready. Edit the graphical fields, close the editor, then Compare.");
                                    publish(Some(getter),"Edit declarations, then compare the proposed source change.");
                                }
                                Ok(_) => {
                                    feedback
                                        .set_text("The parser did not return graphical fields.");
                                    publish(None, "Graphical declaration unavailable");
                                }
                                Err(error) => {
                                    feedback.set_text(&error);
                                    publish(None, &error);
                                }
                            }
                        },
                    );
                }
            });
            *loader_slot.borrow_mut() = Some(Rc::downgrade(&load));
            if let Some(pick) = pick {
                let load = load.clone();
                pick.connect_selected_notify(move |pick| {
                    load(
                        pick.selected()
                            .checked_sub(1)
                            .and_then(|index| TEMPLATES.get(index as usize))
                            .map(|(_, api, _)| (*api).into()),
                    )
                });
            } else {
                load(initial_api);
            }
            let slot = slot.clone();
            edit.connect_clicked(move |_| {
                if !loaded.get() {
                    return;
                }
                let Some(parent) = slot.borrow().as_ref().and_then(|w| w.upgrade()) else {
                    return;
                };
                changed();
                let child = match active_api.borrow().as_deref() {
                    Some("hl.device") => super::devices::show(&parent, &sandbox, &feedback, None),
                    Some("hl.bind" | "hl.define_submap") => {
                        super::bindings::show(&parent, &sandbox, &feedback)
                    }
                    Some("hl.workspace_rule") => {
                        super::workspaces::show(&parent, &sandbox, &feedback)
                    }
                    Some("hl.window_rule" | "hl.layer_rule") => {
                        super::rules::show(&parent, &sandbox, &feedback)
                    }
                    Some("hl.curve" | "hl.animation") => {
                        super::animations::show(&parent, &sandbox, &feedback)
                    }
                    Some("hl.gesture") => super::gestures::show(&parent, &sandbox, &feedback),
                    _ => return,
                };
                child.set_title(Some("Edit captured declarations"));
                child.set_destroy_with_parent(true);
            });
        }),
    );
    window.set_title(Some(if insertion {
        "Add configuration declarations"
    } else {
        "Edit captured declaration"
    }));
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
    fn click(window: &adw::Window, text: &str) {
        for e in descendants(window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Expander>().ok())
        {
            e.set_expanded(true);
        }
        descendants(window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|w| w.label().as_deref() == Some(text))
            .unwrap_or_else(|| panic!("Missing button {text}"))
            .emit_clicked();
    }
    fn child(window: &adw::Window) -> adw::Window {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<adw::Window>().ok())
            .find(|w| w.is_visible() && w.transient_for().as_ref() == Some(window.upcast_ref()))
            .unwrap()
    }
    fn current(state: &Shared, version: &str) -> Model {
        Rc::new(RefCell::new(Some(std::sync::Arc::new(
            super::source_graph::restore(&state.borrow().source_graphs[version].bundle, version)
                .unwrap(),
        ))))
    }
    fn keep(window: &adw::Window, state: &Shared) {
        let generation = state.borrow().generation;
        let compare: gtk::Button = named(window.upcast_ref(), "source-change-compare");
        let keep: gtk::Button = named(window.upcast_ref(), "source-change-keep");
        compare.emit_clicked();
        wait(|| keep.is_sensitive());
        keep.emit_clicked();
        wait(|| state.borrow().generation != generation);
    }
    // The Files action starts with an explicit prompt. Rapid template changes
    // cannot publish an older load over the final selection.
    for (kind, expected) in [(1, "new-device"), (3, "editor-map")] {
        let model = current(state, &version);
        let (body, view) = super::source_analysis::build(&model, state, None);
        let inspector = adw::Window::builder()
            .transient_for(parent)
            .content(&body)
            .build();
        inspector.present();
        view.render();
        let window = view.test_add_family();
        let pick: gtk::DropDown = named(window.upcast_ref(), "source-family-editor-kind");
        let edit: gtk::Button = named(window.upcast_ref(), "source-family-editor-open");
        let compare: gtk::Button = named(window.upcast_ref(), "source-change-compare");
        assert_eq!(pick.selected(), 0);
        assert!(!edit.is_sensitive() && !compare.is_sensitive());
        pick.set_selected(if kind == 1 { 3 } else { 1 });
        pick.set_selected(kind);
        wait(|| edit.is_sensitive());
        if kind == 3 {
            edit.emit_clicked();
            let child = child(&window);
            let name: gtk::Entry = named(child.upcast_ref(), "submap-name");
            name.set_text(expected);
            click(&child, "Update Submap Draft");
            child.close();
        }
        let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
        assert!(minimum <= 350, "Add declaration needs {minimum}px");
        keep(&window, state);
        window.close();
        inspector.close();
        let restored = current(state, &version);
        assert!(restored
            .borrow()
            .as_ref()
            .unwrap()
            .program
            .documents
            .iter()
            .any(|d| d.source.contains(expected)));
    }
    // Reuse the same source form for duplication and full declaration removal.
    // Both changes are private source transactions with ordinary project undo.
    for operation in ["Duplicate", "Remove"] {
        let original = state.borrow().source_graphs[&version].fingerprint.clone();
        let model = current(state, &version);
        let analysis = model.borrow().clone().unwrap();
        let before = analysis
            .report
            .events
            .iter()
            .filter(|e| e.api == "hl.device")
            .count();
        let order = analysis
            .report
            .events
            .iter()
            .find(|e| e.api == "hl.device")
            .unwrap()
            .order;
        let window = open(parent.upcast_ref(), &model, order, state, Rc::new(|| {}));
        let edit: gtk::Button = named(window.upcast_ref(), "source-family-editor-open");
        wait(|| edit.is_sensitive());
        edit.emit_clicked();
        let child = child(&window);
        click(&child, operation);
        child.close();
        keep(&window, state);
        window.close();
        let restored = current(state, &version);
        let after = restored
            .borrow()
            .as_ref()
            .unwrap()
            .report
            .events
            .iter()
            .filter(|e| e.api == "hl.device")
            .count();
        assert_eq!(
            after,
            if operation == "Duplicate" {
                before + 1
            } else {
                before - 1
            }
        );
        state.borrow_mut().undo_last();
        assert_eq!(state.borrow().source_graphs[&version].fingerprint, original);
    }
}
