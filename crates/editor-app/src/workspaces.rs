//! Focused private workspace rule editor. No workspace or empty hook is activated.
use super::{
    label,
    monitors::{choices, compact},
    selector_forms, Shared,
};
use adw::prelude::*;
use hyprland_editor_core::{
    workspace_rule::{self, Rule},
    Setting, Value,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Getter = Rc<dyn Fn() -> Result<(), String>>;
fn current(state: &Shared) -> Vec<Rule> {
    let s = state.borrow();
    s.workspace_rules
        .get(&s.draft.version)
        .cloned()
        .unwrap_or_default()
}
fn unique(rules: &[Rule]) -> String {
    (0..)
        .map(|n| format!("workspace-{n}"))
        .find(|id| rules.iter().all(|r| r.id != *id))
        .unwrap()
}
fn scalar(
    panel: &gtk::Box,
    setting: &Setting,
    value: Option<Value>,
) -> Box<dyn Fn() -> Result<Value, String>> {
    let initial = value.or_else(|| setting.known_default());
    if setting.kind == "Int" {
        let n = if let Some(Value::Int(n)) = initial {
            n
        } else {
            0
        };
        let e = gtk::Entry::builder()
            .text(n.to_string())
            .input_purpose(gtk::InputPurpose::Number)
            .build();
        panel.append(&e);
        return Box::new(move || {
            e.text()
                .parse::<i64>()
                .map(Value::Int)
                .map_err(|_| "Enter a whole number".into())
        });
    }
    if setting.lua_key == "layout" {
        let value = if let Some(Value::String(s)) = initial {
            s
        } else {
            String::new()
        };
        let mut names = vec![
            "".to_string(),
            "dwindle".into(),
            "master".into(),
            "scrolling".into(),
            "monocle".into(),
        ];
        if !names.contains(&value) {
            names.push(value.clone());
        }
        let captions = names
            .iter()
            .map(|s| {
                if s.is_empty() {
                    "Empty Layout Value".to_string()
                } else {
                    s.clone()
                }
            })
            .collect::<Vec<_>>();
        let picker = choices(
            panel,
            &captions.iter().map(String::as_str).collect::<Vec<_>>(),
            names.iter().position(|n| *n == value).unwrap() as u32,
        );
        return Box::new(move || Ok(Value::String(names[picker.selected() as usize].clone())));
    }
    super::controls::form(setting, initial, panel)
}
fn editor(
    panel: &gtk::Box,
    state: &Shared,
    rule: &Rule,
    status: &gtk::Label,
    feedback: &gtk::Label,
) -> Getter {
    let version = state.borrow().draft.version.clone();
    let selector_group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let selector = selector_forms::workspace(&selector_group, &rule.selector, &version);
    panel.append(
        &gtk::Expander::builder()
            .label("Workspace Selection")
            .child(&selector_group)
            .build(),
    );
    let enabled = choices(
        panel,
        &["Enabled (Default)", "Explicitly Enabled", "Disabled"],
        match rule.enabled {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        },
    );
    enabled.set_widget_name("workspace-enabled");
    let fields = workspace_rule::catalogue(&version);
    let field = choices(
        panel,
        &fields
            .iter()
            .map(|s| super::presentation::setting_name(&s.lua_key))
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        0,
    );
    field.set_widget_name("workspace-field");
    let set = gtk::CheckButton::with_label("Set This Field");
    set.set_widget_name("workspace-field-enabled");
    panel.append(&set);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.append(&body);
    let values = Rc::new(RefCell::new(rule.fields.clone()));
    let get = Rc::new(RefCell::new(None::<Box<dyn Fn() -> Result<Value, String>>>));
    let active = Rc::new(Cell::new(0usize));
    let busy = Rc::new(Cell::new(false));
    let flush: Getter = Rc::new({
        let values = values.clone();
        let get = get.clone();
        let active = active.clone();
        let fields = fields.clone();
        let set = set.clone();
        move || {
            let key = fields[active.get()].lua_key.clone();
            if set.is_active() {
                if let Some(get) = get.borrow().as_ref() {
                    let value = get()?;
                    fields[active.get()].validate(&value)?;
                    values.borrow_mut().insert(key, value);
                }
            } else {
                values.borrow_mut().remove(&key);
            }
            Ok(())
        }
    });
    let render: Rc<dyn Fn()> = Rc::new({
        let field = field.clone();
        let fields = fields.clone();
        let set = set.clone();
        let body = body.clone();
        let values = values.clone();
        let get = get.clone();
        let active = active.clone();
        let busy = busy.clone();
        move || {
            busy.set(true);
            let index = field.selected() as usize;
            active.set(index);
            let setting = &fields[index];
            set.set_active(values.borrow().contains_key(&setting.lua_key));
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            if setting.lua_key == "on_created_empty" {
                body.append(&label("This command is only stored as text. It is never run while editing or in ordinary preview."));
            }
            *get.borrow_mut() = Some(scalar(
                &body,
                setting,
                values.borrow().get(&setting.lua_key).cloned(),
            ));
            body.set_sensitive(set.is_active());
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&render);
    let flush_c = flush.clone();
    let busy_c = busy.clone();
    let active_c = active.clone();
    let feedback_c = feedback.clone();
    field.connect_selected_notify(move |field| {
        if busy_c.get() {
            return;
        }
        if let Err(e) = flush_c() {
            busy_c.set(true);
            field.set_selected(active_c.get() as u32);
            busy_c.set(false);
            feedback_c.set_text(&e);
            return;
        }
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    let weak = body.downgrade();
    set.connect_toggled(move |check| {
        if let Some(body) = weak.upgrade() {
            body.set_sensitive(check.is_active());
        }
    });
    render();
    let opts = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let orientation_set = gtk::CheckButton::with_label("Set Master Orientation");
    opts.append(&orientation_set);
    orientation_set.set_active(rule.layout_options.contains_key("orientation"));
    let mut orientations = vec![
        "left".to_string(),
        "top".into(),
        "right".into(),
        "bottom".into(),
        "center".into(),
        "".into(),
    ];
    let orientation_initial = rule.layout_options.get("orientation");
    if let Some(Value::String(s)) = orientation_initial {
        if !orientations.contains(s) {
            orientations.push(s.clone());
        }
    }
    let mut captions = orientations
        .iter()
        .map(|s| {
            if s.is_empty() {
                "Empty Literal".into()
            } else {
                s.clone()
            }
        })
        .collect::<Vec<String>>();
    let oi = if let Some(Value::String(s)) = orientation_initial {
        orientations.iter().position(|n| n == s).unwrap()
    } else if let Some(v) = orientation_initial {
        captions.push(format!("Retained {}", v.lua()));
        captions.len() - 1
    } else {
        0
    };
    let orientation = choices(
        &opts,
        &captions.iter().map(String::as_str).collect::<Vec<_>>(),
        oi as u32,
    );
    let direction_set = gtk::CheckButton::with_label("Set Scrolling Direction");
    opts.append(&direction_set);
    direction_set.set_active(rule.layout_options.contains_key("direction"));
    let mut directions = vec![
        "right".to_string(),
        "left".into(),
        "up".into(),
        "down".into(),
        "".into(),
    ];
    let direction_initial = rule.layout_options.get("direction");
    if let Some(Value::String(s)) = direction_initial {
        if !directions.contains(s) {
            directions.push(s.clone());
        }
    }
    let mut captions = directions
        .iter()
        .map(|s| {
            if s.is_empty() {
                "Empty Literal".into()
            } else {
                s.clone()
            }
        })
        .collect::<Vec<String>>();
    let di = if let Some(Value::String(s)) = direction_initial {
        directions.iter().position(|n| n == s).unwrap()
    } else if let Some(v) = direction_initial {
        captions.push(format!("Retained {}", v.lua()));
        captions.len() - 1
    } else {
        0
    };
    let direction = choices(
        &opts,
        &captions.iter().map(String::as_str).collect::<Vec<_>>(),
        di as u32,
    );
    opts.append(&label("Other imported layout options are retained unchanged. Custom-layout option authoring requires the later layout editor."));
    panel.append(
        &gtk::Expander::builder()
            .label("Layout Options")
            .child(&opts)
            .build(),
    );
    let original_opts = rule.layout_options.clone();
    let original_orientation = orientation.selected();
    let original_direction = direction.selected();
    let id = rule.id.clone();
    let state = state.clone();
    let status = status.clone();
    let feedback = feedback.clone();
    let submit: Getter = Rc::new(move || {
        let _ = &render;
        flush()?;
        let mut rules = current(&state);
        let r = rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Workspace declaration was removed")?;
        r.selector = selector()?;
        r.enabled = match enabled.selected() {
            0 => None,
            1 => Some(true),
            _ => Some(false),
        };
        r.fields = values.borrow().clone();
        r.layout_options = original_opts.clone();
        for (key, set, picker, initial, values) in [
            (
                "orientation",
                &orientation_set,
                &orientation,
                original_orientation,
                &orientations[..],
            ),
            (
                "direction",
                &direction_set,
                &direction,
                original_direction,
                &directions[..],
            ),
        ] {
            if !set.is_active() {
                r.layout_options.remove(key);
            } else if picker.selected() != initial || !r.layout_options.contains_key(key) {
                r.layout_options.insert(
                    key.into(),
                    Value::String(
                        values
                            .get(picker.selected() as usize)
                            .ok_or("Select a known option or retain the imported value")?
                            .clone(),
                    ),
                );
            }
        }
        state.borrow_mut().replace_workspaces(rules)?;
        super::summary(&state, &status);
        Ok(())
    });
    let update = gtk::Button::with_label("Update Workspace Draft");
    update.add_css_class("suggested-action");
    panel.append(&update);
    let submit_c = submit.clone();
    update.connect_clicked(move |_| match submit_c() {
        Ok(()) => {
            feedback.set_text("Workspace draft updated; no workspace or command was activated.")
        }
        Err(e) => feedback.set_text(&e),
    });
    submit
}
pub(super) fn connect(
    button: &gtk::Button,
    parent: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) {
    let weak = parent.downgrade();
    let state = state.clone();
    let status = status.clone();
    button.connect_clicked(move |_| {
        if let Some(parent) = weak.upgrade() {
            show(&parent, &state, &status);
        }
    });
}
fn show(parent: &adw::ApplicationWindow, state: &Shared, status: &gtk::Label) -> adw::Window {
    let win = adw::Window::builder()
        .title("Workspace Drafts")
        .transient_for(parent)
        .modal(true)
        .default_width(700)
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
    content.append(&label("Configure workspace declarations in your private draft. Existing-workspace filters and new workspace addresses have different meanings."));
    let model = gtk::StringList::new(&[]);
    let pick = gtk::DropDown::builder().model(&model).build();
    compact(&pick);
    pick.set_widget_name("workspace-rule");
    content.append(&pick);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add");
    let duplicate = gtk::Button::with_label("Duplicate");
    let remove = gtk::Button::with_label("Remove");
    for b in [&add, &duplicate, &remove] {
        row.append(b);
    }
    content.append(&row);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let up = gtk::Button::with_label("↑");
    let down = gtk::Button::with_label("↓");
    let undo = gtk::Button::with_label("Undo");
    for b in [&up, &down, &undo] {
        row.append(b);
    }
    content.append(&row);
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(&panel);
    let feedback = label("");
    feedback.set_widget_name("workspace-feedback");
    content.append(&feedback);
    let check = gtk::Button::with_label("Check Declaration Order");
    content.append(&check);
    let save = gtk::Button::with_label("Save Draft");
    content.append(&save);
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    win.set_content(Some(&root));
    let busy = Rc::new(Cell::new(false));
    let active = Rc::new(Cell::new(gtk::INVALID_LIST_POSITION));
    let submit = Rc::new(RefCell::new(None::<Getter>));
    let flush: Getter = Rc::new({
        let submit = submit.clone();
        move || {
            if let Some(submit) = submit.borrow().as_ref() {
                submit()?;
            }
            Ok(())
        }
    });
    let refresh: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let model = model.clone();
        let pick = pick.clone();
        let panel = panel.clone();
        let submit = submit.clone();
        let active = active.clone();
        let busy = busy.clone();
        move || {
            if busy.replace(true) {
                return;
            }
            let rules = current(&state);
            let index = (pick.selected() as usize).min(rules.len().saturating_sub(1));
            let names = rules
                .iter()
                .map(|r| {
                    format!(
                        "{}{}",
                        if r.is_enabled() { "" } else { "Disabled · " },
                        if r.selector.is_empty() {
                            "All Workspaces"
                        } else {
                            &r.selector
                        }
                    )
                })
                .collect::<Vec<_>>();
            if model.n_items() as usize != names.len()
                || names
                    .iter()
                    .enumerate()
                    .any(|(i, n)| model.string(i as u32).as_deref() != Some(n))
            {
                model.splice(
                    0,
                    model.n_items(),
                    &names.iter().map(String::as_str).collect::<Vec<_>>(),
                );
            }
            pick.set_selected(if rules.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            active.set(pick.selected());
            while let Some(child) = panel.first_child() {
                panel.remove(&child);
            }
            *submit.borrow_mut() = rules
                .get(index)
                .map(|r| editor(&panel, &state, r, &status, &feedback));
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&refresh);
    let flush_c = flush.clone();
    let busy_c = busy.clone();
    let active_c = active.clone();
    let feedback_c = feedback.clone();
    pick.connect_selected_notify(move |p| {
        if busy_c.get() {
            return;
        }
        if let Err(e) = flush_c() {
            busy_c.set(true);
            p.set_selected(active_c.get());
            busy_c.set(false);
            feedback_c.set_text(&e);
            return;
        }
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    for (button, op) in [
        (&add, 0),
        (&duplicate, 1),
        (&remove, 2),
        (&up, 3),
        (&down, 4),
    ] {
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let pick = pick.clone();
        let flush = flush.clone();
        let refresh = refresh.clone();
        button.connect_clicked(move |_| {
            if op != 2 {
                if let Err(e) = flush() {
                    feedback.set_text(&e);
                    return;
                }
            }
            let mut rules = current(&state);
            let index = pick.selected() as usize;
            let target = match op {
                0 => {
                    let mut r = Rule::new(&unique(&rules));
                    r.selector = (rules.len() + 1).to_string();
                    rules.push(r);
                    rules.len() - 1
                }
                1 if index < rules.len() => {
                    let mut r = rules[index].clone();
                    r.id = unique(&rules);
                    rules.push(r);
                    rules.len() - 1
                }
                2 if index < rules.len() => {
                    rules.remove(index);
                    index.min(rules.len().saturating_sub(1))
                }
                3 if index > 0 => {
                    rules.swap(index, index - 1);
                    index - 1
                }
                4 if index + 1 < rules.len() => {
                    rules.swap(index, index + 1);
                    index + 1
                }
                _ => return,
            };
            let result = state.borrow_mut().replace_workspaces(rules);
            match result {
                Ok(()) => {
                    super::summary(&state, &status);
                    refresh();
                    pick.set_selected(target as u32);
                }
                Err(e) => feedback.set_text(&e),
            }
        });
    }
    let state_c = state.clone();
    let status_c = status.clone();
    let refresh_c = refresh.clone();
    undo.connect_clicked(move |_| {
        state_c.borrow_mut().undo_last();
        super::summary(&state_c, &status_c);
        refresh_c();
    });
    let flush_c = flush.clone();
    let state_c = state.clone();
    let feedback_c = feedback.clone();
    check.connect_clicked(move |_| {
        if let Err(e) = flush_c() {
            feedback_c.set_text(&e);
            return;
        }
        let messages = workspace_rule::findings(&current(&state_c));
        feedback_c.set_text(&if messages.is_empty() {
            "No literal order warnings. Live matching and hardware behavior remain unverified."
                .into()
        } else {
            messages.join("\n")
        });
    });
    let flush_c = flush.clone();
    let feedback_c = feedback.clone();
    let weak = parent.downgrade();
    save.connect_clicked(move |_| {
        if let Err(e) = flush_c() {
            feedback_c.set_text(&e);
            return;
        }
        if let Some(parent) = weak.upgrade() {
            if let Some(a) = parent.lookup_action("save-draft") {
                a.activate(None);
            }
        }
    });
    let flush_c = flush.clone();
    win.connect_close_request(move |win| match flush_c() {
        Ok(()) => gtk::glib::Propagation::Proceed,
        Err(e) => {
            let dialog =
                adw::MessageDialog::new(Some(win), Some("Discard invalid form changes?"), Some(&e));
            dialog.add_response("keep", "Keep Editing");
            dialog.add_response("discard", "Discard Form Changes");
            dialog.set_default_response(Some("keep"));
            dialog.set_close_response("keep");
            let weak = win.downgrade();
            dialog.connect_response(None, move |_, response| {
                if response == "discard" {
                    if let Some(win) = weak.upgrade() {
                        win.destroy();
                    }
                }
            });
            dialog.present();
            gtk::glib::Propagation::Stop
        }
    });
    refresh();
    win.present();
    win
}

#[cfg(test)]
pub(super) fn test_editor(parent: &adw::ApplicationWindow) {
    fn descendants(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut out = vec![];
        let mut child = w.first_child();
        while let Some(w) = child {
            out.push(w.clone());
            out.extend(descendants(&w));
            child = w.next_sibling();
        }
        out
    }
    fn click(win: &adw::Window, text: &str) {
        for e in descendants(win.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Expander>().ok())
        {
            e.set_expanded(true);
        }
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
    let state = super::initial_state();
    let status = label("");
    let win = show(parent, &state, &status);
    click(&win, "Add");
    assert_eq!(current(&state).len(), 1);
    assert!(state.borrow().dirty());
    let fields = workspace_rule::catalogue(&state.borrow().draft.version);
    for (index, s) in fields.iter().enumerate() {
        pick(&win, "workspace-field").set_selected(index as u32);
        let set = descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == "workspace-field-enabled")
            .unwrap()
            .downcast::<gtk::CheckButton>()
            .unwrap();
        set.set_active(true);
        click(&win, "Update Workspace Draft");
        assert!(
            current(&state)[0].fields.contains_key(&s.lua_key),
            "{}",
            s.id
        );
    }
    pick(&win, "workspace-enabled").set_selected(2);
    click(&win, "Update Workspace Draft");
    assert_eq!(current(&state)[0].enabled, Some(false));
    click(&win, "Duplicate");
    assert_eq!(current(&state).len(), 2);
    click(&win, "Undo");
    assert_eq!(current(&state).len(), 1);
    click(&win, "Remove");
    assert!(current(&state).is_empty());
    click(&win, "Undo");
    assert_eq!(current(&state).len(), 1);
    let source = state.borrow().export_lua().unwrap();
    let parsed = hyprland_editor_core::lua::parse(
        &source,
        hyprland_editor_core::BASELINE,
        &hyprland_editor_core::catalogue(),
    )
    .unwrap();
    assert!(!parsed.opaque_effects, "{:?}", parsed.diagnostics);
    assert_eq!(parsed.workspace_rules.len(), 1);
    let stored = hyprland_editor_core::workspace::Workspace::new(state.borrow().draft.clone())
        .with_workspace_rules(current(&state))
        .to_json(&hyprland_editor_core::catalogue())
        .unwrap();
    assert_eq!(
        hyprland_editor_core::workspace::Workspace::from_json(
            std::str::from_utf8(&stored).unwrap(),
            &hyprland_editor_core::catalogue()
        )
        .unwrap()
        .workspace_rules,
        current(&state)
    );
    let mut retained = Rule::new("retained");
    retained.selector = "name:work".into();
    retained
        .layout_options
        .insert("orientation".into(), Value::Bool(false));
    retained
        .layout_options
        .insert("direction".into(), Value::Int(7));
    retained
        .layout_options
        .insert("custom".into(), Value::Float(1.25));
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
    state
        .borrow_mut()
        .replace_workspaces(vec![retained.clone()])
        .unwrap();
    let submit = editor(&box_, &state, &retained, &status, &label(""));
    submit().unwrap();
    assert_eq!(current(&state)[0], retained);
    state.borrow_mut().undo_last();
    let context = gtk::glib::MainContext::default();
    let settle = || {
        for _ in 0..5 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    win.set_default_size(360, 760);
    settle();
    for (i, s) in fields.iter().enumerate() {
        pick(&win, "workspace-field").set_selected(i as u32);
        settle();
        let (min, _, _, _) = win
            .content()
            .unwrap()
            .measure(gtk::Orientation::Horizontal, -1);
        assert!(min <= 350, "{} requires {min}px", s.id);
    }
    let samples = [
        "",
        "3",
        "name:work",
        "special",
        "special:notes",
        "r[1-9] s[0] n[1] n[s:work] n[e:dev] m[DP-1] w[fpgv1-3] f[-1]",
        "unknown[opaque]",
    ];
    for version in [
        hyprland_editor_core::BASELINE,
        hyprland_editor_core::AUDITED_MAIN,
    ] {
        for text in samples {
            let panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let get = selector_forms::workspace(&panel, text, version);
            assert_eq!(get().unwrap(), text);
            let (min, _, _, _) = panel.measure(gtk::Orientation::Horizontal, -1);
            assert!(min <= 318, "Selector {text} requires {min}px");
        }
    }
    win.set_default_size(700, 760);
    pick(&win, "workspace-field")
        .set_selected(fields.iter().position(|s| s.lua_key == "layout").unwrap() as u32);
    for e in descendants(win.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Expander>().ok())
    {
        e.set_expanded(false);
    }
    settle();
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
        let node = snapshot.to_node().unwrap();
        win.renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(format!("{path}.workspaces.png"))
            .unwrap();
    }
    let weak = pick(&win, "workspace-field").downgrade();
    win.destroy();
    drop(win);
    settle();
    assert!(
        weak.upgrade().is_none(),
        "Workspace controls leaked after closure"
    );
}
