//! Bounded compound controls with exact numeric text and one gradient swatch.
use super::{
    controls, label,
    source_changes::{input_form, Getter},
};
use adw::prelude::*;
use hyprland_editor_core::Value;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn component(body: &gtk::Box, name: &str, value: Value, changed: Rc<dyn Fn()>) -> Getter {
    let field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    field.set_widget_name(&format!("source-compound-{name}"));
    field.update_property(&[gtk::accessible::Property::Label(name)]);
    field.append(&label(name));
    body.append(&field);
    input_form(&value, None, &field, changed)
}
#[derive(Clone)]
struct Gradient {
    colors: Rc<RefCell<Vec<u32>>>,
    rendering: Rc<Cell<bool>>,
    changed: Rc<dyn Fn()>,
    selected: gtk::glib::WeakRef<gtk::DropDown>,
    swatch: gtk::glib::WeakRef<gtk::ColorDialogButton>,
    add: gtk::glib::WeakRef<gtk::Button>,
    remove: gtk::glib::WeakRef<gtk::Button>,
    up: gtk::glib::WeakRef<gtk::Button>,
    down: gtk::glib::WeakRef<gtk::Button>,
}
impl Gradient {
    fn refresh(&self, index: usize) {
        if self.rendering.get() {
            return;
        }
        let Some(selected) = self.selected.upgrade() else {
            return;
        };
        let colors = self.colors.borrow();
        let index = index.min(colors.len().saturating_sub(1));
        if colors.get(index).is_none() {
            return;
        }
        let names = colors
            .iter()
            .enumerate()
            .map(|(i, c)| format!("Color {} · 0x{c:08X}", i + 1))
            .collect::<Vec<_>>();
        drop(colors);
        self.rendering.set(true);
        selected.set_model(Some(&gtk::StringList::new(
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
        selected.set_selected(index as u32);
        self.rendering.set(false);
        self.show(index);
    }
    fn show(&self, index: usize) {
        if self.rendering.get() {
            return;
        }
        let Some(swatch) = self.swatch.upgrade() else {
            return;
        };
        let colors = self.colors.borrow();
        let Some(color) = colors.get(index).copied() else {
            return;
        };
        let count = colors.len();
        drop(colors);
        self.rendering.set(true);
        let rgba = gtk::gdk::RGBA::new(
            ((color >> 16) & 255) as f32 / 255.,
            ((color >> 8) & 255) as f32 / 255.,
            (color & 255) as f32 / 255.,
            (color >> 24) as f32 / 255.,
        );
        if swatch.rgba() != rgba {
            swatch.set_rgba(&rgba);
        }
        for (button, enabled) in [
            (&self.add, count < 32),
            (&self.remove, count > 1),
            (&self.up, index > 0),
            (&self.down, index + 1 < count),
        ] {
            if let Some(button) = button.upgrade() {
                button.set_sensitive(enabled);
            }
        }
        self.rendering.set(false);
    }
    fn index(&self) -> Option<usize> {
        self.selected
            .upgrade()
            .map(|p| p.selected() as usize)
            .filter(|i| *i < self.colors.borrow().len())
    }
}
pub(super) fn form(initial: &Value, body: &gtk::Box, changed: Rc<dyn Fn()>) -> Getter {
    match initial {
        Value::Color(value) => {
            let swatch = controls::color_button(*value);
            swatch.set_widget_name("source-change-color");
            swatch.update_property(&[gtk::accessible::Property::Label("Proposed color")]);
            body.append(&swatch);
            swatch.connect_rgba_notify(move |_| changed());
            Rc::new(move || Ok(Value::Color(controls::color_value(&swatch))))
        }
        Value::Vec2(values) => {
            let x = component(body, "X", Value::Float(values[0]), changed.clone());
            let y = component(body, "Y", Value::Float(values[1]), changed);
            Rc::new(move || {
                let Value::Float(x) = x()? else {
                    return Err("Missing X coordinate".into());
                };
                let Value::Float(y) = y()? else {
                    return Err("Missing Y coordinate".into());
                };
                Ok(Value::Vec2([x, y]))
            })
        }
        Value::CssGap(values) => {
            let fields = ["Top", "Right", "Bottom", "Left"]
                .into_iter()
                .zip(values)
                .map(|(name, v)| component(body, name, Value::Int(*v), changed.clone()))
                .collect::<Vec<_>>();
            Rc::new(move || {
                let mut values = [0; 4];
                for (i, get) in fields.iter().enumerate() {
                    let Value::Int(v) = get()? else {
                        return Err("Missing gap component".into());
                    };
                    values[i] = v;
                }
                Ok(Value::CssGap(values))
            })
        }
        Value::Gradient { colors, angle } => {
            let selected = gtk::DropDown::from_strings(&[]);
            super::monitors::compact(&selected);
            selected.set_enable_search(true);
            selected.set_widget_name("source-gradient-stop");
            selected.update_property(&[gtk::accessible::Property::Label("Gradient color stop")]);
            body.append(&selected);
            let swatch = controls::color_button(colors.first().copied().unwrap_or(0xff000000));
            swatch.set_widget_name("source-gradient-color");
            swatch.update_property(&[gtk::accessible::Property::Label("Selected gradient color")]);
            body.append(&swatch);
            let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let add = gtk::Button::with_label("Add");
            let remove = gtk::Button::with_label("Remove");
            let up = gtk::Button::from_icon_name("go-up-symbolic");
            let down = gtk::Button::from_icon_name("go-down-symbolic");
            for (button, name) in [
                (&add, "Add a color stop"),
                (&remove, "Remove selected stop"),
                (&up, "Move stop earlier"),
                (&down, "Move stop later"),
            ] {
                button.update_property(&[gtk::accessible::Property::Label(name)]);
                button.set_tooltip_text(Some(name));
                actions.append(button);
            }
            body.append(&actions);
            add.set_widget_name("source-gradient-add");
            remove.set_widget_name("source-gradient-remove");
            up.set_widget_name("source-gradient-up");
            down.set_widget_name("source-gradient-down");
            let colors = Rc::new(RefCell::new(colors.clone()));
            let ui = Gradient {
                colors: colors.clone(),
                rendering: Rc::new(Cell::new(false)),
                changed: changed.clone(),
                selected: selected.downgrade(),
                swatch: swatch.downgrade(),
                add: add.downgrade(),
                remove: remove.downgrade(),
                up: up.downgrade(),
                down: down.downgrade(),
            };
            let u = ui.clone();
            selected.connect_selected_notify(move |p| {
                if !u.rendering.get() {
                    u.show(p.selected() as usize);
                }
            });
            let u = ui.clone();
            swatch.connect_rgba_notify(move |button| {
                if u.rendering.get() {
                    return;
                }
                if let Some(i) = u.index() {
                    let color = controls::color_value(button);
                    if u.colors.borrow()[i] == color {
                        return;
                    }
                    u.colors.borrow_mut()[i] = color;
                    u.refresh(i);
                    (u.changed)();
                }
            });
            let u = ui.clone();
            add.connect_clicked(move |_| {
                if u.colors.borrow().len() >= 32 {
                    return;
                }
                u.colors.borrow_mut().push(0xffffffff);
                let i = u.colors.borrow().len() - 1;
                u.refresh(i);
                (u.changed)();
            });
            let u = ui.clone();
            remove.connect_clicked(move |_| {
                if u.colors.borrow().len() <= 1 {
                    return;
                }
                if let Some(i) = u.index() {
                    u.colors.borrow_mut().remove(i);
                    u.refresh(i);
                    (u.changed)();
                }
            });
            for (button, delta) in [(up, -1), (down, 1)] {
                let u = ui.clone();
                button.connect_clicked(move |_| {
                    if let Some(i) = u.index() {
                        let next = i as i64 + delta;
                        if next < 0 || next as usize >= u.colors.borrow().len() {
                            return;
                        }
                        u.colors.borrow_mut().swap(i, next as usize);
                        u.refresh(next as usize);
                        (u.changed)();
                    }
                });
            }
            ui.refresh(0);
            let angle = component(body, "Angle in degrees", Value::Float(*angle), changed);
            Rc::new(move || {
                let Value::Float(angle) = angle()? else {
                    return Err("Missing gradient angle".into());
                };
                Ok(Value::Gradient {
                    colors: colors.borrow().clone(),
                    angle,
                })
            })
        }
        _ => Rc::new(|| Err("This value has no compound control".into())),
    }
}

#[cfg(test)]
pub(super) fn test_workflow(parent: &adw::ApplicationWindow) {
    use super::source_changes;
    use hyprland_editor_core::{workspace::Workspace, workspace_graph::Bundle, BASELINE};
    use std::sync::{atomic::AtomicBool, Arc};
    fn named(root: &gtk::Widget, name: &str) -> gtk::Widget {
        fn find(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
            if root.widget_name() == name {
                return Some(root.clone());
            }
            let mut child = root.first_child();
            while let Some(w) = child {
                if let Some(w) = find(&w, name) {
                    return Some(w);
                }
                child = w.next_sibling();
            }
            None
        }
        find(root, name).unwrap_or_else(|| panic!("Missing {name}"))
    }
    #[track_caller]
    fn wait(mut ready: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if ready() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(12),
                "Compound GUI operation timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    // Controls preserve exact data before any native comparison takes place.
    for value in [
        Value::Vec2([-0.0, 1e35]),
        Value::CssGap([i64::MIN, 0, i64::MAX, -3]),
        Value::FontWeight(9007199254740993),
        Value::Color(0x80123456),
        Value::Gradient {
            colors: vec![0xff123456, 0x80abcdef],
            angle: -0.0,
        },
    ] {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let changes = Rc::new(Cell::new(0));
        let changed = changes.clone();
        let getter = input_form(
            &value,
            None,
            &body,
            Rc::new(move || changed.set(changed.get() + 1)),
        );
        assert_eq!(
            serde_json::to_string(&getter().unwrap()).unwrap(),
            serde_json::to_string(&value).unwrap()
        );
        if matches!(value, Value::Gradient { .. }) {
            named(body.upcast_ref(), "source-gradient-stop")
                .downcast::<gtk::DropDown>()
                .unwrap()
                .set_selected(1);
            assert_eq!(
                changes.get(),
                0,
                "Selecting a stop must not alter gradient data"
            );
            assert_eq!(getter().unwrap(), value);
        }
    }
    let fixture = super::test_support::TempDir::new("he-compound-source-gui");
    let path = fixture.path().join("main.lua");
    let original="-- preserve outer comments\nhl.config{['misc.background_color']='#123456'}\nhl.config{['general.col.active_border']={colors={'#123', -- stop memo\n '#456'},angle=45, note='gradient-extra'}}\nhl.config{['decoration.shadow.offset']={0, -- vector memo\n 0, note='vector-extra'}}\nhl.config{['general.gaps_out']={top=1 --[[gap memo]],right=2,bottom=3,left=4,note='gap-extra'}}\nhl.config{['group.groupbar.font_weight_active']=400}\n";
    std::fs::write(&path, original).unwrap();
    let captured = Arc::new(super::capture_worker::read_selected(&path).unwrap());
    let state = super::initial_state();
    let document = super::workflow::parse_source(
        captured.content.clone(),
        BASELINE.into(),
        &state.borrow().catalogue,
    )
    .unwrap();
    state
        .borrow_mut()
        .sources
        .insert(BASELINE.into(), Arc::new(document));
    state
        .borrow_mut()
        .source_origins
        .insert(BASELINE.into(), captured.identity.clone());
    let analysis = super::source_graph::analyze(
        captured.clone(),
        hyprland_editor_core::module_path::Context {
            main_file: captured.identity.selected_path.clone(),
            home: None,
            inherited_package_path: None,
        },
        vec![],
        BASELINE.into(),
        None,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let base = Arc::new(
        super::source_graph::Saved::new(super::source_graph::freeze(&analysis).unwrap(), BASELINE)
            .unwrap(),
    );
    state.borrow_mut().keep_graph(base.clone()).unwrap();
    let model: super::source_analysis::Model = Rc::new(RefCell::new(Some(Arc::new(analysis))));
    std::fs::remove_file(&path).unwrap();
    for index in 0..5 {
        let close = Rc::new(RefCell::new(None::<gtk::glib::WeakRef<adw::Window>>));
        let target = close.clone();
        let event = model.borrow().as_ref().unwrap().report.events[index].order;
        let window = source_changes::open(
            parent.upcast_ref(),
            &model,
            event,
            &state,
            Rc::new(move || {
                if let Some(window) = target.borrow().as_ref().and_then(|w| w.upgrade()) {
                    window.close();
                }
            }),
        );
        *close.borrow_mut() = Some(window.downgrade());
        let scope = named(window.upcast_ref(), "source-change-scope")
            .downcast::<gtk::DropDown>()
            .unwrap();
        assert_eq!(
            scope.selected(),
            if (1..=3).contains(&index) { 2 } else { 0 }
        );
        match index {
            0 => named(window.upcast_ref(), "source-change-color")
                .downcast::<gtk::ColorDialogButton>()
                .unwrap()
                .set_rgba(&gtk::gdk::RGBA::new(1., 0., 0., 0.5)),
            1 => {
                named(window.upcast_ref(), "source-gradient-add")
                    .downcast::<gtk::Button>()
                    .unwrap()
                    .emit_clicked();
                named(window.upcast_ref(), "source-gradient-up")
                    .downcast::<gtk::Button>()
                    .unwrap()
                    .emit_clicked();
            }
            2 => named(window.upcast_ref(), "source-change-value")
                .downcast::<gtk::Entry>()
                .unwrap()
                .set_text("-0.0"),
            3 => named(window.upcast_ref(), "source-change-value")
                .downcast::<gtk::Entry>()
                .unwrap()
                .set_text("-3"),
            _ => named(window.upcast_ref(), "source-change-value")
                .downcast::<gtk::Entry>()
                .unwrap()
                .set_text("9007199254740992"),
        }
        let compare = named(window.upcast_ref(), "source-change-compare")
            .downcast::<gtk::Button>()
            .unwrap();
        let keep = named(window.upcast_ref(), "source-change-keep")
            .downcast::<gtk::Button>()
            .unwrap();
        compare.emit_clicked();
        wait(|| compare.is_sensitive());
        let output = named(window.upcast_ref(), "source-change-comparison")
            .downcast::<gtk::TextView>()
            .unwrap();
        let b = output.buffer();
        let text = b.text(&b.start_iter(), &b.end_iter(), true);
        assert!(keep.is_sensitive(), "{index}: {text}");
        assert!(text.contains("Modeled parsed value:"));
        if (1..=3).contains(&index) {
            assert!(
                text.contains("Existing table comments, spacing and unrelated fields are retained")
            );
        }
        assert!(
            !text.contains("Additional modeled findings"),
            "{index}: {text}"
        );
        let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
        assert!(minimum <= 350, "Compound {index} needs {minimum}px");
        if index == 1 {
            if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
                let start = std::time::Instant::now();
                wait(|| start.elapsed() >= std::time::Duration::from_millis(300));
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
                    .save_to_png(format!("{path}.compound-source.png"))
                    .unwrap();
            }
        }
        let generation = state.borrow().generation;
        keep.emit_clicked();
        wait(|| !window.is_visible());
        assert_eq!(state.borrow().generation, generation + 1);
        assert!(state.borrow().draft.values.is_empty());
        assert!(Arc::ptr_eq(
            &base.bundle.sources,
            &state.borrow().source_graphs[BASELINE].bundle.sources
        ));
    }
    let after = state.borrow().source_graphs[BASELINE].clone();
    assert!(state.borrow().export_lua().is_err());
    assert_eq!(after.bundle.sources[0].content, original);
    let workspace = Workspace::new(state.borrow().draft.clone()).with_graph(&after.bundle);
    let saved = hyprland_editor_core::workspace::save(
        fixture.path(),
        &workspace,
        &state.borrow().catalogue,
    )
    .unwrap();
    let reopened =
        hyprland_editor_core::workspace::read(&saved, &state.borrow().catalogue).unwrap();
    let bundle = Bundle {
        graph: reopened.source_graph.unwrap(),
        sources: Arc::new(reopened.sources),
    };
    assert_eq!(bundle.fingerprint().unwrap(), after.fingerprint);
    let restored = super::source_graph::restore(&bundle, BASELINE).unwrap();
    assert!(restored.blocked.is_none());
    for retained in [
        "-- stop memo",
        "-- vector memo",
        "--[[gap memo]]",
        "note='gradient-extra'",
        "note='vector-extra'",
        "note='gap-extra'",
    ] {
        assert_eq!(
            restored.program.documents[0]
                .source
                .matches(retained)
                .count(),
            1,
            "{retained}"
        );
    }
    assert_eq!(restored.config_batches.len(), 5);
    assert!(restored
        .config_batches
        .iter()
        .flat_map(|b| &b.candidates)
        .all(|c| matches!(
            c.model_value,
            Some(hyprland_editor_core::lua::scalar_codec::Outcome::Accepted { .. })
        )));
    for _ in 0..5 {
        state.borrow_mut().undo_last();
    }
    assert_eq!(
        state.borrow().source_graphs[BASELINE].fingerprint,
        base.fingerprint
    );
}
