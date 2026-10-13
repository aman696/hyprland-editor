mod worker;
mod workflow;
use adw::prelude::*;
use hyprland_editor_core::{catalogue, storage, Draft, Setting, Value, AUDITED_MAIN, BASELINE};
use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, rc::Rc};

type Shared = Rc<RefCell<State>>;
struct State {
    catalogue: Vec<Setting>,
    draft: Draft,
    undo: Vec<(String, Option<Value>)>,
    drafts: BTreeMap<String, Draft>,
    visible: Vec<Setting>,
    sources: BTreeMap<String, hyprland_editor_core::lua::LuaDocument>,
    active: Option<hyprland_editor_discovery::Snapshot>,
    generation: u64,
    saved: BTreeMap<String, BTreeMap<String, Value>>,
}
impl State {
    fn dirty_version(&self, version: &str) -> bool {
        let draft = if self.draft.version == version {
            Some(&self.draft)
        } else {
            self.drafts.get(version)
        };
        draft.is_some_and(|d| {
            self.saved
                .get(version)
                .map_or(!d.values.is_empty(), |saved| saved != &d.values)
        })
    }
    fn dirty(&self) -> bool {
        self.dirty_version(&self.draft.version) || self.drafts.keys().any(|v| self.dirty_version(v))
    }
    fn remember(&mut self, id: &str) {
        self.generation += 1;
        if self.undo.len() == 32 {
            self.undo.remove(0);
        }
        self.undo
            .push((id.into(), self.draft.values.get(id).cloned()));
    }
}
fn human_name(id: &str) -> String {
    let text = id.rsplit(':').next().unwrap_or(id).replace(['_', '.'], " ");
    let mut chars = text.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}
fn label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .xalign(0.)
        .selectable(true)
        .build()
}
fn spin(value: f64, min: f64, max: f64, digits: u32) -> gtk::SpinButton {
    let w = gtk::SpinButton::with_range(min, max, if digits == 0 { 1. } else { 0.1 });
    w.set_digits(digits);
    w.set_value(value);
    w
}
fn color_button(color: u32) -> gtk::ColorDialogButton {
    let dialog = gtk::ColorDialog::builder().with_alpha(true).build();
    let b = gtk::ColorDialogButton::new(Some(dialog));
    b.set_rgba(&gtk::gdk::RGBA::new(
        ((color >> 16) & 255) as f32 / 255.,
        ((color >> 8) & 255) as f32 / 255.,
        (color & 255) as f32 / 255.,
        (color >> 24) as f32 / 255.,
    ));
    b
}
fn color_value(b: &gtk::ColorDialogButton) -> u32 {
    let c = b.rgba();
    let byte = |x: f32| (x.clamp(0., 1.) * 255.).round() as u32;
    (byte(c.alpha()) << 24) | (byte(c.red()) << 16) | (byte(c.green()) << 8) | byte(c.blue())
}
fn summary(state: &Shared, status: &gtk::Label) {
    let s = state.borrow();
    status.set_text(&format!(
        "{} · {} draft values · {} undo steps",
        s.draft.version,
        s.draft.values.len(),
        s.undo.len()
    ));
}
fn notice(window: &adw::ApplicationWindow, title: &str, body: &str) {
    let dialog = adw::MessageDialog::new(Some(window), Some(title), Some(body));
    dialog.add_response("close", "Close");
    dialog.set_close_response("close");
    dialog.present();
}

fn form(
    setting: &Setting,
    value: Option<Value>,
    box_: &gtk::Box,
) -> Box<dyn Fn() -> Result<Value, String>> {
    let value = value.or_else(|| setting.known_default());
    match setting.kind.as_str() {
        "Bool" => {
            let w = gtk::Switch::builder()
                .halign(gtk::Align::Start)
                .active(matches!(value, Some(Value::Bool(true))))
                .build();
            box_.append(&w);
            Box::new(move || Ok(Value::Bool(w.is_active())))
        }
        "Int" => {
            let v = if let Some(Value::Int(v)) = value {
                v as f64
            } else {
                0.
            };
            let w = spin(
                v,
                setting.bounds.min.unwrap_or(-2_147_483_648.),
                setting.bounds.max.unwrap_or(2_147_483_647.),
                0,
            );
            box_.append(&w);
            Box::new(move || {
                w.update();
                Ok(Value::Int(w.value() as i64))
            })
        }
        "Float" => {
            let v = if let Some(Value::Float(v)) = value {
                v
            } else {
                0.
            };
            let w = spin(
                v,
                setting.bounds.min.unwrap_or(-1_000_000.),
                setting.bounds.max.unwrap_or(1_000_000.),
                6,
            );
            box_.append(&w);
            Box::new(move || {
                w.update();
                Ok(Value::Float(w.value()))
            })
        }
        "String" => {
            let v = if let Some(Value::String(v)) = value {
                v
            } else {
                String::new()
            };
            let w = gtk::Entry::builder().text(v).max_length(4096).build();
            box_.append(&w);
            Box::new(move || Ok(Value::String(w.text().into())))
        }
        "Color" => {
            let v = if let Some(Value::Color(v)) = value {
                v
            } else {
                0xff000000
            };
            let w = color_button(v);
            box_.append(&w);
            Box::new(move || Ok(Value::Color(color_value(&w))))
        }
        "FontWeight" => {
            let v = if let Some(Value::FontWeight(v)) = value {
                v
            } else {
                400
            };
            let w = spin(v.into(), 0., 65535., 0);
            box_.append(&w);
            Box::new(move || {
                w.update();
                Ok(Value::FontWeight(w.value() as u16))
            })
        }
        "Vec2" => {
            let v = if let Some(Value::Vec2(v)) = value {
                v
            } else {
                [0.; 2]
            };
            let mut fields = vec![];
            for (i, n) in v.into_iter().enumerate() {
                box_.append(&label(if i == 0 { "X" } else { "Y" }));
                let w = spin(n, -1_000_000., 1_000_000., 6);
                box_.append(&w);
                fields.push(w);
            }
            Box::new(move || {
                for f in &fields {
                    f.update();
                }
                Ok(Value::Vec2([fields[0].value(), fields[1].value()]))
            })
        }
        "CssGap" => {
            let v = if let Some(Value::CssGap(v)) = value {
                v
            } else {
                [0; 4]
            };
            let mut fields = vec![];
            for (name, n) in ["Top", "Right", "Bottom", "Left"].into_iter().zip(v) {
                box_.append(&label(name));
                let w = spin(n as f64, 0., 100_000., 0);
                box_.append(&w);
                fields.push(w);
            }
            Box::new(move || {
                for f in &fields {
                    f.update();
                }
                Ok(Value::CssGap(std::array::from_fn(|i| {
                    fields[i].value() as i64
                })))
            })
        }
        "Gradient" => {
            let (colors, angle) = if let Some(Value::Gradient { colors, angle }) = value {
                (colors, angle)
            } else {
                (vec![0xff000000], 0.)
            };
            let stops = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            stops.set_halign(gtk::Align::Start);
            let fields = Rc::new(RefCell::new(Vec::new()));
            for color in colors {
                let w = color_button(color);
                stops.append(&w);
                fields.borrow_mut().push(w);
            }
            box_.append(&label("Colors (left to right)"));
            box_.append(&stops);
            let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let add = gtk::Button::with_label("Add color");
            let remove = gtk::Button::with_label("Remove last color");
            actions.append(&add);
            actions.append(&remove);
            box_.append(&actions);
            let f = fields.clone();
            let b = stops.clone();
            add.connect_clicked(move |_| {
                if f.borrow().len() < 32 {
                    let w = color_button(0xffffffff);
                    b.append(&w);
                    f.borrow_mut().push(w);
                }
            });
            let f = fields.clone();
            remove.connect_clicked(move |_| {
                if f.borrow().len() > 1 {
                    let w = f.borrow_mut().pop().unwrap();
                    stops.remove(&w);
                }
            });
            box_.append(&label("Angle in degrees"));
            let angle = spin(angle, -36000., 36000., 2);
            box_.append(&angle);
            Box::new(move || {
                angle.update();
                Ok(Value::Gradient {
                    colors: fields.borrow().iter().map(color_value).collect(),
                    angle: angle.value(),
                })
            })
        }
        _ => Box::new(|| Err("No editor for this type".into())),
    }
}
fn detail(state: &Shared, panel: &gtk::Box, setting: &Setting, status: &gtk::Label) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }
    let title = label(&human_name(&setting.id));
    title.add_css_class("title-2");
    panel.append(&title);
    panel.append(&label(&setting.id));
    panel.append(&label(&format!(
        "{} · snapshot {}",
        setting.kind, setting.version
    )));
    panel.append(&label(&format!(
        "Upstream default expression: {}",
        setting.default_expression
    )));
    panel.append(&gtk::LinkButton::with_label(
        &setting.source,
        "View audited source",
    ));
    let current = state.borrow().draft.values.get(&setting.id).cloned();
    let observed = state
        .borrow()
        .active
        .as_ref()
        .filter(|a| a.version.as_deref() == Some(setting.version.as_str()))
        .and_then(|a| a.values.get(&setting.id))
        .cloned();
    if let Some(observation) = &observed {
        panel.append(&label(&observation.description));
        let raw = serde_json::to_string(&observation.raw_current).unwrap_or_default();
        let preview: String = raw.chars().take(2048).collect();
        panel.append(&label(&format!(
            "Observed value: {preview} · point-in-time snapshot; refresh to update"
        )));
        if observation.current.is_none() {
            panel.append(&label(
                "This live value has no certified normalization yet; the controls do not guess it.",
            ));
        }
    }
    if current.is_none() {
        panel.append(&label("No draft override. Controls start from a normalized snapshot when available, then a decoded default. Choose a value explicitly when neither is known."));
    }
    let current = current.or_else(|| observed.and_then(|o| o.current));
    let getter = form(setting, current, panel);
    let error = label("");
    error.add_css_class("error");
    panel.append(&error);
    let stage = gtk::Button::with_label("Stage value in draft");
    stage.add_css_class("suggested-action");
    panel.append(&stage);
    let unset = gtk::Button::with_label("Remove draft override");
    panel.append(&unset);
    let s = state.clone();
    let spec = setting.clone();
    let st = status.clone();
    stage.connect_clicked(move |_| {
        match getter().and_then(|v| {
            spec.validate(&v)?;
            Ok(v)
        }) {
            Ok(v) => {
                let mut state = s.borrow_mut();
                if state.draft.values.get(&spec.id) != Some(&v) {
                    state.remember(&spec.id);
                    state.draft.values.insert(spec.id.clone(), v);
                }
                drop(state);
                error.set_text("Value staged. Your running desktop has not been changed.");
                summary(&s, &st);
            }
            Err(e) => error.set_text(&e),
        }
    });
    let s = state.clone();
    let spec = setting.clone();
    let p = panel.downgrade();
    let st = status.clone();
    unset.connect_clicked(move |_| {
        let Some(p) = p.upgrade() else {
            return;
        };
        let mut state = s.borrow_mut();
        if state.draft.values.contains_key(&spec.id) {
            state.remember(&spec.id);
            state.draft.values.remove(&spec.id);
        }
        drop(state);
        summary(&s, &st);
        detail(&s, &p, &spec, &st);
    });
}
fn refill(
    state: &Shared,
    model: &gtk::StringList,
    search: &gtk::SearchEntry,
    version: &gtk::DropDown,
) {
    let selected = if version.selected() == 0 {
        BASELINE
    } else {
        AUDITED_MAIN
    };
    let query = search.text().to_lowercase();
    let mut s = state.borrow_mut();
    s.visible = s
        .catalogue
        .iter()
        .filter(|x| x.version == selected && x.id.to_lowercase().contains(&query))
        .cloned()
        .collect();
    let labels: Vec<String> = s
        .visible
        .iter()
        .map(|x| {
            format!(
                "{} · {}",
                human_name(&x.id),
                x.id.split(':').next().unwrap_or("General")
            )
        })
        .collect();
    drop(s);
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    model.splice(0, model.n_items(), &labels);
}
fn build(app: &adw::Application) -> adw::ApplicationWindow {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Hyprland Editor · Draft workspace")
        .default_width(1100)
        .default_height(760)
        .build();
    let state = Rc::new(RefCell::new(State {
        catalogue: catalogue(),
        draft: Draft::new(BASELINE),
        undo: vec![],
        drafts: BTreeMap::new(),
        visible: vec![],
        sources: BTreeMap::new(),
        active: None,
        generation: 0,
        saved: BTreeMap::new(),
    }));
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    content.append(&header);
    let open = gtk::Button::with_label("Import");
    let save = gtk::Button::with_label("Save draft");
    let review = gtk::Button::with_label("Review / copy Lua");
    let undo = gtk::Button::with_label("Undo");
    let coverage = gtk::Button::with_label("Feature coverage");
    header.pack_start(&open);
    header.pack_start(&undo);
    header.pack_end(&review);
    header.pack_end(&save);
    header.pack_end(&coverage);
    let banner=label("Draft workspace · Live inspection is read-only. Live apply and compositor preview remain unavailable.");
    banner.set_margin_start(16);
    banner.set_margin_end(16);
    banner.set_margin_bottom(8);
    content.append(&banner);
    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    toolbar.set_margin_start(16);
    toolbar.set_margin_end(16);
    toolbar.set_margin_bottom(8);
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search every scalar setting…")
        .hexpand(true)
        .build();
    let version =
        gtk::DropDown::from_strings(&["Hyprland 0.56.2 (353)", "Audited main commit (402)"]);
    toolbar.append(&search);
    toolbar.append(&version);
    content.append(&toolbar);
    let model = gtk::StringList::new(&[]);
    let selection = gtk::SingleSelection::new(Some(model.clone()));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = label("");
        row.set_selectable(false);
        row.set_margin_top(8);
        row.set_margin_bottom(8);
        row.set_margin_start(12);
        row.set_margin_end(12);
        item.set_child(Some(&row));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let text = item.item().and_downcast::<gtk::StringObject>().unwrap();
        item.child()
            .and_downcast::<gtk::Label>()
            .unwrap()
            .set_text(&text.string());
    });
    let list = gtk::ListView::new(Some(selection.clone()), Some(factory));
    let sidebar = gtk::ScrolledWindow::builder()
        .child(&list)
        .min_content_width(340)
        .build();
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    panel.set_margin_start(24);
    panel.set_margin_end(24);
    panel.set_margin_top(20);
    panel.set_margin_bottom(20);
    panel.append(&label("Choose a setting to edit a draft override."));
    let detail_scroll = gtk::ScrolledWindow::builder()
        .child(&panel)
        .min_content_width(420)
        .hexpand(true)
        .build();
    let split = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&sidebar)
        .end_child(&detail_scroll)
        .position(340)
        .resize_start_child(false)
        .shrink_start_child(false)
        .shrink_end_child(false)
        .vexpand(true)
        .build();
    content.append(&split);
    let status = label("");
    status.set_margin_start(16);
    status.set_margin_top(8);
    status.set_margin_bottom(8);
    content.append(&status);
    window.set_content(Some(&content));
    refill(&state, &model, &search, &version);
    summary(&state, &status);
    let s = state.clone();
    let p = panel.clone();
    let st = status.clone();
    selection.connect_selected_notify(move |selection| {
        let setting = s
            .borrow()
            .visible
            .get(selection.selected() as usize)
            .cloned();
        if let Some(setting) = setting {
            detail(&s, &p, &setting, &st);
        }
    });
    let s = state.clone();
    let m = model.clone();
    let v = version.downgrade();
    search.connect_search_changed(move |search| {
        if let Some(v) = v.upgrade() {
            refill(&s, &m, search, &v);
        }
    });
    let s = state.clone();
    let m = model.clone();
    let search_c = search.downgrade();
    let st = status.clone();
    let p = panel.clone();
    version.connect_selected_notify(move |v| {
        let name = if v.selected() == 0 {
            BASELINE
        } else {
            AUDITED_MAIN
        };
        if s.borrow().draft.version != name {
            let mut s = s.borrow_mut();
            let old = s.draft.clone();
            s.drafts.insert(old.version.clone(), old);
            s.draft = s
                .drafts
                .get(name)
                .cloned()
                .unwrap_or_else(|| Draft::new(name));
            s.undo.clear();
            s.generation += 1;
        }
        if let Some(search) = search_c.upgrade() {
            refill(&s, &m, &search, v);
        }
        while let Some(child) = p.first_child() {
            p.remove(&child);
        }
        p.append(&label(
            "Choose a setting. Drafts for each snapshot are kept separately in this workspace.",
        ));
        summary(&s, &st);
    });
    let s = state.clone();
    let sel = selection.clone();
    let p = panel.clone();
    let st = status.clone();
    undo.connect_clicked(move |_| {
        let mut state = s.borrow_mut();
        if let Some((id, previous)) = state.undo.pop() {
            if let Some(value) = previous {
                state.draft.values.insert(id, value);
            } else {
                state.draft.values.remove(&id);
            }
            state.generation += 1;
        }
        let spec = state.visible.get(sel.selected() as usize).cloned();
        drop(state);
        if let Some(spec) = spec {
            detail(&s, &p, &spec, &st);
        }
        summary(&s, &st);
    });
    let s = state.clone();
    let win = window.downgrade();
    let st = status.clone();
    save.connect_clicked(move |_| {
        let Some(win) = win.upgrade() else {
            return;
        };
        let root = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")));
        let result = root
            .ok_or_else(|| "Cannot find a user state directory".into())
            .and_then(|root| {
                if !root.is_absolute() {
                    return Err("State directory must be absolute".into());
                }
                std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
                let state = s.borrow();
                storage::save_draft(&root, &state.draft, &state.catalogue)
            });
        match result {
            Ok(path) => {
                let mut state = s.borrow_mut();
                let version = state.draft.version.clone();
                let values = state.draft.values.clone();
                state.saved.insert(version, values);
                drop(state);
                st.set_text(&format!("Private draft saved: {}", path.display()));
            }
            Err(e) => notice(&win, "Could not save draft", &e),
        }
    });
    let s = state.clone();
    let win = window.downgrade();
    review.connect_clicked(move |_|{
        let Some(win) = win.upgrade() else { return; };
        let state=s.borrow();match state.draft.export_lua(&state.catalogue){Ok(lua)=>{
            let dialog=adw::Window::builder().title("Review draft overrides").transient_for(&win).modal(true).default_width(720).default_height(560).build();let b=gtk::Box::new(gtk::Orientation::Vertical,12);b.set_margin_top(16);b.set_margin_bottom(16);b.set_margin_start(16);b.set_margin_end(16);b.append(&label("These are draft overrides, not a diff against the running compositor. Copying does not apply them."));let text=gtk::TextView::builder().editable(false).monospace(true).wrap_mode(gtk::WrapMode::WordChar).build();text.buffer().set_text(&lua);b.append(&gtk::ScrolledWindow::builder().child(&text).vexpand(true).build());let copy=gtk::Button::with_label("Copy reviewed Lua to clipboard");b.append(&copy);let w=dialog.downgrade();copy.connect_clicked(move |_|{if let Some(w)=w.upgrade(){w.clipboard().set_text(&lua);}});dialog.set_content(Some(&b));dialog.present();
        },Err(e)=>notice(&win,"Invalid draft",&e)}
    });
    workflow::connect_import(
        &open, &window, &state, &status, &version, &selection, &panel,
    );
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    actions.set_margin_start(16);
    actions.set_margin_end(16);
    actions.set_margin_bottom(8);
    let live = gtk::Button::with_label("Refresh live snapshot");
    let source_copy = gtk::Button::with_label("Review edited source copy");
    actions.append(&live);
    actions.append(&source_copy);
    let diagnostics = gtk::Button::with_label("Import diagnostics");
    actions.append(&diagnostics);
    content.insert_child_after(&actions, Some(&banner));
    workflow::connect_live(&live, &window, &state, &status, &selection, &panel);
    workflow::connect_source_copy(&source_copy, &window, &state);
    workflow::connect_diagnostics(&diagnostics, &window, &state);
    let win = window.downgrade();
    coverage.connect_clicked(move |_|{
        let Some(win) = win.upgrade() else { return; };
        let dialog=adw::Window::builder().title("Audited feature coverage").transient_for(&win).default_width(850).default_height(600).build();let b=gtk::Box::new(gtk::Orientation::Vertical,12);b.set_margin_top(16);b.set_margin_start(16);b.set_margin_end(16);b.set_margin_bottom(16);b.append(&label("1,453 audited feature contracts. Scalar draft controls are implemented; other feature families below are engineering requirements, not completed controls."));
        // Loaded on demand. A single text buffer avoids constructing 1,453 widgets.
        let data:serde_json::Value=serde_json::from_str(include_str!("../../../data/features.json")).unwrap();let mut text=String::new();for feature in data.as_array().unwrap(){text.push_str(&format!("{}  ·  {}  ·  {}\n",feature["id"].as_str().unwrap_or("?"),feature["engine"].as_str().unwrap_or("?"),feature["version_membership"]));}
        let view=gtk::TextView::builder().editable(false).monospace(true).build();view.buffer().set_text(&text);b.append(&gtk::ScrolledWindow::builder().child(&view).vexpand(true).build());dialog.set_content(Some(&b));dialog.present();
    });
    workflow::connect_close(&window, &state);
    window.present();
    window
}
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--lua-source-worker") {
        worker::entry();
        return;
    }
    let app = adw::Application::builder()
        .application_id("io.github.aman696.HyprlandEditor")
        .build();
    app.connect_activate(|app| {
        build(app);
    });
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    fn descendants(widget: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut out = vec![];
        let mut child = widget.first_child();
        while let Some(w) = child {
            out.push(w.clone());
            out.extend(descendants(&w));
            child = w.next_sibling();
        }
        out
    }
    fn click(window: &adw::ApplicationWindow, text: &str) {
        let button = descendants(window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(text))
            .unwrap();
        button.emit_clicked();
    }
    #[test]
    #[ignore = "requires a graphical display; runs no compositor commands"]
    fn native_forms_and_draft_workflow() {
        adw::init().unwrap();
        // Construct every version-scoped form, including types outside the first visible page.
        for setting in catalogue() {
            let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let value = setting.known_default();
            let getter = form(&setting, value.clone(), &panel);
            let actual = getter().unwrap();
            if let Some(expected) = value {
                assert_eq!(actual, expected, "{} {}", setting.version, setting.id);
            }
            setting
                .validate(&actual)
                .unwrap_or_else(|e| panic!("{}: {e}", setting.id));
        }
        let app = adw::Application::builder()
            .application_id("io.github.aman696.HyprlandEditor.Test")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let window = build(&app);
        let widgets = descendants(window.upcast_ref());
        let list = widgets
            .iter()
            .filter_map(|w| w.clone().downcast::<gtk::ListView>().ok())
            .find(|list| list.model().is_some_and(|m| m.n_items() == 353))
            .unwrap();
        let selection = list.model().and_downcast::<gtk::SingleSelection>().unwrap();
        assert_eq!(selection.n_items(), 353);
        selection.set_selected(0);
        click(&window, "Stage value in draft");
        let dropdown = widgets
            .iter()
            .find_map(|w| w.clone().downcast::<gtk::DropDown>().ok())
            .unwrap();
        dropdown.set_selected(1);
        assert_eq!(selection.n_items(), 402);
        dropdown.set_selected(0);
        selection.set_selected(0);
        click(&window, "Remove draft override");
        click(&window, "Undo");
        let labels: Vec<String> = descendants(window.upcast_ref())
            .iter()
            .filter_map(|w| w.clone().downcast::<gtk::Label>().ok())
            .map(|l| l.text().into())
            .collect();
        assert!(labels.iter().any(|s| s.contains("1 draft values")));
        let context = gtk::glib::MainContext::default();
        for _ in 0..50 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
            let paintable = gtk::WidgetPaintable::new(Some(&window));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
            let node = snapshot.to_node().expect("rendered window");
            let texture = window.renderer().unwrap().render_texture(&node, None);
            texture.save_to_png(path).unwrap();
        }
        window.close();
        assert!(
            window.is_visible(),
            "unsaved drafts must prevent immediate closure"
        );
        window.destroy();
    }
}
