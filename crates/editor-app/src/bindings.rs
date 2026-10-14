//! Focused bindings/submaps editor; actions are data and are never dispatched by the editor.
use super::{
    bind_forms, label,
    monitors::{choices, compact},
    Shared,
};
use adw::prelude::*;
use hyprland_editor_core::{
    action::{self, Action},
    binding::{self, Bind, DeviceFilter, Draft, Submap},
    AUDITED_MAIN,
};
fn wrap_check(check: &gtk::CheckButton) {
    if let Some(label) = check.child().and_downcast::<gtk::Label>() {
        label.set_wrap(true);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    }
}
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Redraw = Rc<RefCell<Option<std::rc::Weak<dyn Fn()>>>>;
fn redraw(redraw: &Redraw) {
    if let Some(r) = redraw.borrow().as_ref().and_then(std::rc::Weak::upgrade) {
        r();
    }
}
type Submit = Rc<dyn Fn() -> Result<(), String>>;
fn action_name(name: &str) -> String {
    match name {
        "exec_cmd" => "Run Command".into(),
        "exec_raw" => "Run Raw Command".into(),
        "no_op" => "Do Nothing".into(),
        "submap" => "Switch Submap".into(),
        _ => super::presentation::setting_name(name),
    }
}
fn current(state: &Shared) -> Draft {
    let s = state.borrow();
    s.bindings
        .get(&s.draft.version)
        .cloned()
        .unwrap_or_default()
}
fn commit(state: &Shared, draft: Draft, status: &gtk::Label, feedback: &gtk::Label) -> bool {
    let result = state.borrow_mut().replace_bindings(draft);
    match result {
        Ok(()) => {
            super::summary(state, status);
            feedback.set_text("Binding draft updated; no action was executed.");
            true
        }
        Err(e) => {
            feedback.set_text(&e);
            false
        }
    }
}
fn unique(draft: &Draft, prefix: &str) -> String {
    (0..)
        .map(|n| format!("{prefix}-{n}"))
        .find(|id| !draft.order.contains(id))
        .unwrap()
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
            show(&win, &state, &status);
        }
    });
}
fn show(parent: &adw::ApplicationWindow, state: &Shared, status: &gtk::Label) -> adw::Window {
    let win = adw::Window::builder()
        .title("Binding Drafts")
        .transient_for(parent)
        .modal(true)
        .default_width(780)
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
    content.append(&label("Build shortcuts, ordered actions and submaps in your draft. Actions and shortcuts are not applied to the desktop."));
    let model = gtk::StringList::new(&[]);
    let selected = gtk::DropDown::builder().model(&model).build();
    compact(&selected);
    selected.set_widget_name("binding-rule");
    content.append(&selected);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add Binding");
    let map = gtk::Button::with_label("Add Submap");
    let remove = gtk::Button::with_label("Remove");
    for b in [&add, &map] {
        row.append(b);
    }
    content.append(&row);
    let order = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let up = gtk::Button::with_label("↑");
    let down = gtk::Button::with_label("↓");
    let duplicate = gtk::Button::with_label("Duplicate");
    let undo = gtk::Button::with_label("Undo");
    for b in [&up, &down, &duplicate, &undo] {
        order.append(b);
    }
    let manage = gtk::Box::new(gtk::Orientation::Vertical, 6);
    manage.append(&order);
    manage.append(&remove);
    let expander = gtk::Expander::builder()
        .label("Manage Bindings and Submaps")
        .child(&manage)
        .build();
    content.append(&expander);
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(&panel);
    let feedback = label("");
    feedback.set_widget_name("binding-feedback");
    content.append(&feedback);
    let check = gtk::Button::with_label("Check Draft Relationships");
    content.append(&check);
    let review = gtk::Button::with_label("Review Binding Lua");
    content.append(&review);
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
    let redraw: Redraw = Rc::new(RefCell::new(None));
    let busy = Rc::new(Cell::new(false));
    let active = Rc::new(Cell::new(gtk::INVALID_LIST_POSITION));
    let submit = Rc::new(RefCell::new(None::<Submit>));
    let flush: Submit = Rc::new({
        let submit = submit.clone();
        move || {
            if let Some(submit) = submit.borrow().as_ref() {
                submit()?;
            }
            Ok(())
        }
    });
    let state_c = state.clone();
    let feedback_c = feedback.clone();
    let flush_c = flush.clone();
    check.connect_clicked(move |_| {
        if let Err(e)=flush_c(){feedback_c.set_text(&e);return;}
        let findings=current(&state_c).findings();
        feedback_c.set_text(&if findings.is_empty() {
            "No literal relationship warnings. Runtime collisions, device coverage and escape delivery are unverified.".into()
        } else { format!("Static draft findings (up to 256):\n{}\nThese do not simulate runtime delivery.",findings.join("\n")) });
    });
    let parent = parent.downgrade();
    let flush_c = flush.clone();
    let feedback_c = feedback.clone();
    save.connect_clicked(move |_| {
        if let Err(e) = flush_c() {
            feedback_c.set_text(&e);
            return;
        }
        if let Some(parent) = parent.upgrade() {
            if let Some(a) = parent.lookup_action("save-draft") {
                a.activate(None);
            }
        }
    });
    let refresh: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let panel = panel.clone();
        let model = model.clone();
        let selected = selected.clone();
        let win = win.downgrade();
        let busy = busy.clone();
        let submit = submit.clone();
        let active = active.clone();
        let redraw = redraw.clone();
        move || {
            if busy.replace(true) {
                return;
            }
            let draft = current(&state);
            let index = (selected.selected() as usize).min(draft.order.len().saturating_sub(1));
            let bind_index = std::collections::BTreeMap::<_, _>::from_iter(
                draft.binds.iter().map(|b| (b.id.as_str(), b)),
            );
            let map_index = std::collections::BTreeMap::<_, _>::from_iter(
                draft.submaps.iter().map(|m| (m.id.as_str(), m)),
            );
            let names: Vec<_> = draft
                .order
                .iter()
                .map(|id| {
                    if let Some(b) = bind_index.get(id.as_str()) {
                        format!(
                            "{} · {}",
                            b.chord.text(),
                            b.submap.as_deref().unwrap_or("Global")
                        )
                    } else {
                        format!(
                            "Submap: {}",
                            map_index
                                .get(id.as_str())
                                .map(|m| m.name.as_str())
                                .unwrap_or("Unknown")
                        )
                    }
                })
                .collect();
            if model.n_items() as usize != names.len()
                || names
                    .iter()
                    .enumerate()
                    .any(|(i, name)| model.string(i as u32).as_deref() != Some(name))
            {
                model.splice(
                    0,
                    model.n_items(),
                    &names.iter().map(String::as_str).collect::<Vec<_>>(),
                );
            }
            selected.set_selected(if draft.order.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            while let Some(child) = panel.first_child() {
                panel.remove(&child);
            }
            active.set(selected.selected());
            *submit.borrow_mut() = None;
            if let (Some(id), Some(win)) = (draft.order.get(index), win.upgrade()) {
                if let Some(bind) = draft.binds.iter().find(|b| b.id == *id) {
                    *submit.borrow_mut() = Some(edit_bind(
                        &panel,
                        &win,
                        &state,
                        bind,
                        (&status, &feedback),
                        &redraw,
                    ));
                } else if let Some(map) = draft.submaps.iter().find(|m| m.id == *id) {
                    *submit.borrow_mut() =
                        Some(edit_map(&panel, &state, map, (&status, &feedback), &redraw));
                }
            } else {
                panel.append(&label("Add a binding or submap to begin."));
            }
            busy.set(false);
        }
    });
    *redraw.borrow_mut() = Some(Rc::downgrade(&refresh));
    let weak = Rc::downgrade(&refresh);
    let flush_c = flush.clone();
    let busy_c = busy.clone();
    let active_c = active.clone();
    let feedback_c = feedback.clone();
    selected.connect_selected_notify(move |selected| {
        if busy_c.get() {
            return;
        }
        if let Err(e) = flush_c() {
            busy_c.set(true);
            selected.set_selected(active_c.get());
            busy_c.set(false);
            feedback_c.set_text(&format!(
                "Fix or remove the current draft before switching: {e}"
            ));
            return;
        }
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    for (button, op) in [
        (&add, 0),
        (&map, 1),
        (&remove, 2),
        (&up, 3),
        (&down, 4),
        (&duplicate, 5),
    ] {
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let selected = selected.clone();
        let refresh = refresh.clone();
        let flush_c = flush.clone();
        button.connect_clicked(move |_| {
            if op!=2 {if let Err(e)=flush_c(){feedback.set_text(&e);return;}}
            let mut draft=current(&state);
            let index=selected.selected() as usize;
            let chosen=draft.order.get(index).cloned();
            let mut selection=chosen.clone();
            match op {
                0=>{
                    let id=unique(&draft,"bind");
                    let action=Action::new(action::find("no_op",&state.borrow().draft.version).unwrap());
                    draft.binds.push(Bind{id:id.clone(),chord:binding::Chord::parse("SUPER + Q"),actions:vec![action],flags:Default::default(),description:None,description_alias:false,device:None,submap:None});
                    selection=Some(id.clone());draft.order.push(id);
                }
                1=>{
                    let id=unique(&draft,"map");
                    let name=(0..).map(|n|format!("submap-{n}")).find(|name|draft.submaps.iter().all(|m|m.name!=*name)).unwrap();
                    draft.submaps.push(Submap{id:id.clone(),name,parent:None,reset:None});
                    selection=Some(id.clone());draft.order.push(id);
                }
                2=>{
                    let Some(id)=chosen else{return;};
                    if draft.binds.iter().any(|b|b.submap.as_deref()==Some(&id))||draft.submaps.iter().any(|m|m.parent.as_deref()==Some(&id)){
                        feedback.set_text("Move or remove this submap's children first; they will not be discarded.");return;
                    }
                    draft.binds.retain(|b|b.id!=id);draft.submaps.retain(|m|m.id!=id);draft.order.retain(|i|*i!=id);selection=None;
                }
                3 if index>0=>draft.order.swap(index,index-1),
                4 if index+1<draft.order.len()=>draft.order.swap(index,index+1),
                5=>{
                    let Some(id)=chosen else{return;};
                    if let Some(b)=draft.binds.iter().find(|b|b.id==id){
                        let mut b=b.clone();b.id=unique(&draft,"bind");selection=Some(b.id.clone());draft.order.push(b.id.clone());draft.binds.push(b);
                    }else{
                        match draft.duplicate_submap(&id,&state.borrow().draft.version){Ok(id)=>selection=Some(id),Err(e)=>{feedback.set_text(&e);return;}}
                    }
                }
                _=>return,
            }
            let next=selection.and_then(|id|draft.order.iter().position(|i|*i==id)).unwrap_or_else(||index.min(draft.order.len().saturating_sub(1)));
            if commit(&state,draft,&status,&feedback){refresh();selected.set_selected(next as u32);}
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
    let state_c = state.clone();
    let win_c = win.downgrade();
    let flush_c = flush.clone();
    let feedback_c = feedback.clone();
    review.connect_clicked(move |_|{if let Err(e)=flush_c(){feedback_c.set_text(&e);return;}let text=match current(&state_c).export_lua(&state_c.borrow().draft.version){Ok(t)=>t,Err(_)=>return};let Some(parent)=win_c.upgrade()else{return;};let review=adw::Window::builder().title("Review Binding Lua").transient_for(&parent).modal(true).default_width(720).default_height(560).build();let box_=gtk::Box::new(gtk::Orientation::Vertical,12);box_.append(&label("This is a binding draft fragment. It does not apply shortcuts and must not replace a full configuration."));let view=gtk::TextView::builder().editable(false).monospace(true).wrap_mode(gtk::WrapMode::WordChar).build();view.buffer().set_text(&text);box_.append(&gtk::ScrolledWindow::builder().child(&view).vexpand(true).build());let copy=gtk::Button::with_label("Copy Reviewed Binding Lua");box_.append(&copy);let w=review.downgrade();copy.connect_clicked(move |_|{if let Some(w)=w.upgrade(){w.clipboard().set_text(&text);}});review.set_content(Some(&box_));review.present();});
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
            dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
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
fn scopes(
    panel: &gtk::Box,
    draft: &Draft,
    selected: Option<&str>,
) -> (gtk::DropDown, Vec<Option<String>>) {
    let mut ids = vec![None];
    let mut names = vec!["Global".to_string()];
    for m in &draft.submaps {
        ids.push(Some(m.id.clone()));
        names.push(m.name.clone());
    }
    let picker = choices(
        panel,
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
        ids.iter()
            .position(|id| id.as_deref() == selected)
            .unwrap_or(0) as u32,
    );
    (picker, ids)
}
fn edit_map(
    panel: &gtk::Box,
    state: &Shared,
    map: &Submap,
    labels: (&gtk::Label, &gtk::Label),
    redraw_handle: &Redraw,
) -> Submit {
    let name = gtk::Entry::builder().text(&map.name).build();
    name.set_widget_name("submap-name");
    panel.append(&label("Submap Name"));
    panel.append(&name);
    panel.append(&label(
        "Parent Scope (names are global; nesting does not prefix them)",
    ));
    let (parent, ids) = scopes(panel, &current(state), map.parent.as_deref());
    let draft = current(state);
    let mut destinations = vec!["reset".to_string()];
    for m in &draft.submaps {
        if !destinations.contains(&m.name) {
            destinations.push(m.name.clone());
        }
    }
    if let Some(r) = &map.reset {
        if !destinations.contains(r) {
            destinations.push(r.clone());
        }
    }
    let reset = choices(
        panel,
        &destinations.iter().map(String::as_str).collect::<Vec<_>>(),
        destinations
            .iter()
            .position(|r| Some(r) == map.reset.as_ref())
            .unwrap_or(0) as u32,
    );
    reset.set_widget_name("submap-reset");
    let has_reset = gtk::CheckButton::with_label("Set Automatic Transition After a Binding");
    wrap_check(&has_reset);
    has_reset.set_active(map.reset.is_some());
    panel.append(&has_reset);
    let escape = gtk::Button::with_label("Add Escape Binding");
    panel.append(&escape);
    let state_c = state.clone();
    let id_c = map.id.clone();
    let status = labels.0.clone();
    let feedback = labels.1.clone();
    let redraw_c = redraw_handle.clone();
    escape.connect_clicked(move |_| {
        let mut draft = current(&state_c);
        let id = unique(&draft, "bind");
        let mut action =
            Action::new(action::find("submap", &state_c.borrow().draft.version).unwrap());
        action.fields.insert(
            "argument".into(),
            hyprland_editor_core::Value::String("reset".into()),
        );
        draft.binds.push(Bind {
            id: id.clone(),
            chord: binding::Chord::parse("Escape"),
            actions: vec![action],
            flags: Default::default(),
            description: Some("Return to global shortcuts".into()),
            description_alias: false,
            device: None,
            submap: Some(id_c.clone()),
        });
        draft.order.push(id);
        if commit(&state_c, draft, &status, &feedback) {
            redraw(&redraw_c);
        }
    });
    let apply = gtk::Button::with_label("Update Submap Draft");
    panel.append(&apply);
    let old_name = map.name.clone();
    let id = map.id.clone();
    let state = state.clone();
    let status = labels.0.clone();
    let feedback = labels.1.clone();
    let submit: Submit = Rc::new(move || {
        let mut draft = current(&state);
        let new_name = name.text().to_string();
        draft.rename_submap(&id, &new_name)?;
        let m = draft.submaps.iter_mut().find(|m| m.id == id).unwrap();
        m.parent = ids[parent.selected() as usize].clone();
        m.reset = has_reset.is_active().then(|| {
            let selected = &destinations[reset.selected() as usize];
            if *selected == old_name {
                new_name.clone()
            } else {
                selected.clone()
            }
        });
        state.borrow_mut().replace_bindings(draft)?;
        super::summary(&state, &status);
        Ok(())
    });
    let submit_c = submit.clone();
    let redraw_c = redraw_handle.clone();
    apply.connect_clicked(move |_| match submit_c() {
        Ok(()) => {
            feedback
                .set_text("Submap draft updated, including exact literal transition references.");
            redraw(&redraw_c);
        }
        Err(e) => feedback.set_text(&e),
    });
    submit
}
fn edit_bind(
    panel: &gtk::Box,
    window: &adw::Window,
    state: &Shared,
    bind: &Bind,
    labels: (&gtk::Label, &gtk::Label),
    redraw_handle: &Redraw,
) -> Submit {
    let stack = gtk::Stack::new();
    stack.set_widget_name("binding-pages");
    stack.set_hhomogeneous(false);
    stack.set_vhomogeneous(false);
    let pages = choices(panel, &["Shortcut", "Options", "Actions"], 0);
    pages.set_widget_name("binding-page");
    let weak = stack.downgrade();
    pages.connect_selected_notify(move |p| {
        if let Some(stack) = weak.upgrade() {
            stack.set_visible_child_name(["shortcut", "options", "actions"][p.selected() as usize]);
        }
    });
    panel.append(&stack);
    let shortcut = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let options = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let actions = gtk::Box::new(gtk::Orientation::Vertical, 8);
    stack.add_titled(&shortcut, Some("shortcut"), "Shortcut");
    stack.add_titled(&options, Some("options"), "Options");
    stack.add_titled(&actions, Some("actions"), "Actions");
    let chord = bind_forms::chord(&shortcut, &bind.chord, &state.borrow().draft.version);
    shortcut.append(&label("Submap"));
    let (scope, scope_ids) = scopes(&shortcut, &current(state), bind.submap.as_deref());
    let flags = binding::FLAGS
        .iter()
        .filter(|flag| **flag != "mouse" || state.borrow().draft.version == AUDITED_MAIN)
        .map(|flag| {
            let check = gtk::CheckButton::with_label(&super::presentation::setting_name(flag));
            check.set_active(bind.flags.contains(*flag));
            options.append(&check);
            ((*flag).to_string(), check)
        })
        .collect::<Vec<_>>();
    let description = gtk::Entry::builder()
        .text(bind.description.as_deref().unwrap_or(""))
        .build();
    options.append(&label("Description"));
    options.append(&description);
    let describe = gtk::CheckButton::with_label("Set Description");
    describe.set_active(bind.description.is_some());
    options.append(&describe);
    let alias = gtk::CheckButton::with_label("Use desc Alias");
    alias.set_active(bind.description_alias);
    options.append(&alias);
    let target = gtk::CheckButton::with_label("Set Device / Tag Filter");
    target.set_active(bind.device.is_some());
    options.append(&target);
    let inclusive = choices(
        &options,
        &["Include Listed Names / Tags", "Exclude Listed Names / Tags"],
        if bind.device.as_ref().is_none_or(|d| d.inclusive) {
            0
        } else {
            1
        },
    );
    let targets = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    targets.buffer().set_text(
        &bind
            .device
            .as_ref()
            .map(|d| d.items.join("\n"))
            .unwrap_or_default(),
    );
    options.append(&label(
        "One device name or tag per line. Names/tags can overlap; live capabilities are unknown.",
    ));
    options.append(
        &gtk::ScrolledWindow::builder()
            .child(&targets)
            .min_content_height(80)
            .build(),
    );
    let action_model = gtk::StringList::new(
        &bind
            .actions
            .iter()
            .enumerate()
            .map(|(i, a)| format!("{} · {}", i + 1, action_name(&a.constructor)))
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let action_pick = gtk::DropDown::builder().model(&action_model).build();
    compact(&action_pick);
    action_pick.set_widget_name("binding-action");
    actions.append(&action_pick);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add Action");
    let remove = gtk::Button::with_label("Remove Action");
    let up = gtk::Button::with_label("↑");
    let down = gtk::Button::with_label("↓");
    for b in [&add, &remove] {
        controls.append(b);
    }

    let reorder = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    reorder.append(&up);
    reorder.append(&down);
    let manage = gtk::Box::new(gtk::Orientation::Vertical, 6);
    manage.append(&controls);
    manage.append(&reorder);
    actions.append(
        &gtk::Expander::builder()
            .label("Manage Actions")
            .child(&manage)
            .build(),
    );
    let defs = action::catalogue(&state.borrow().draft.version);
    actions.append(&label("Action Type"));
    let constructors = choices(
        &actions,
        &defs
            .iter()
            .map(|d| action_name(&d.name))
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        0,
    );
    constructors.set_widget_name("binding-constructor");
    let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
    actions.append(&fields);
    let local = Rc::new(RefCell::new(bind.actions.clone()));
    let getter = Rc::new(RefCell::new(
        None::<Box<dyn Fn() -> Result<Action, String>>>,
    ));
    let version = state.borrow().draft.version.clone();
    let busy = Rc::new(Cell::new(false));
    let active = Rc::new(Cell::new(gtk::INVALID_LIST_POSITION));
    let flush: Rc<dyn Fn() -> Result<(), String>> = Rc::new({
        let local = local.clone();
        let getter = getter.clone();
        let active = active.clone();
        move || {
            let value = getter.borrow().as_ref().map(|get| get()).transpose()?;
            if let Some(a) = value {
                if let Some(slot) = local.borrow_mut().get_mut(active.get() as usize) {
                    *slot = a;
                }
            }
            Ok(())
        }
    });
    let render: Rc<dyn Fn()> = Rc::new({
        let local = local.clone();
        let getter = getter.clone();
        let fields = fields.clone();
        let defs = defs.clone();
        let pick = action_pick.clone();
        let constructors = constructors.clone();
        let model = action_model.clone();
        let version = version.clone();
        let busy = busy.clone();
        let active = active.clone();
        move || {
            if busy.replace(true) {
                return;
            }
            let list = local.borrow();
            let index = (pick.selected() as usize).min(list.len().saturating_sub(1));
            model.splice(
                0,
                model.n_items(),
                &list
                    .iter()
                    .enumerate()
                    .map(|(i, a)| format!("{} · {}", i + 1, action_name(&a.constructor)))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            );
            pick.set_selected(if list.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            while let Some(child) = fields.first_child() {
                fields.remove(&child);
            }
            active.set(pick.selected());
            if let Some(a) = list.get(index) {
                let def = defs.iter().find(|d| d.name == a.constructor).unwrap();
                constructors.set_selected(defs.iter().position(|d|d.name==a.constructor).unwrap() as u32);
                fields.append(&label(match def.effect.as_str() {
                    "external-command" => "Runs a command when activated.",
                    "session-or-power" => "Can change session or power state when activated.",
                    "input-or-capture" => "Can send or route input when activated.",
                    "process-termination-or-signal" => {
                        "Can close a window or signal a process when activated."
                    }
                    "external-or-layout" => {
                        "Can call an external action or change layout state when activated."
                    }
                    "inert" => "Performs no operation.",
                    _ => "Changes window, workspace or cursor state when activated.",
                }));
                *getter.borrow_mut() = Some(bind_forms::action(&fields, a, &version));
            } else {
                *getter.borrow_mut() = None;
            }
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&render);
    let flush_c = flush.clone();
    let busy_c = busy.clone();
    let active_c = active.clone();
    let feedback = labels.1.clone();
    action_pick.connect_selected_notify(move |pick| {
        if busy_c.get() {
            return;
        }
        if let Err(e) = flush_c() {
            busy_c.set(true);
            pick.set_selected(active_c.get());
            busy_c.set(false);
            feedback.set_text(&format!("Fix the current action before switching: {e}"));
            return;
        }
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    let local_c = local.clone();
    let pick_c = action_pick.clone();
    let defs_c = defs.clone();
    let busy_c = busy.clone();
    let render_c = Rc::downgrade(&render);
    constructors.connect_selected_notify(move |p| {
        if busy_c.get() {
            return;
        }
        if let Some(a) = local_c.borrow_mut().get_mut(pick_c.selected() as usize) {
            *a = Action::new(&defs_c[p.selected() as usize]);
        }
        if let Some(r) = render_c.upgrade() {
            r();
        }
    });
    for (button, op) in [(&add, 0), (&remove, 1), (&up, 2), (&down, 3)] {
        let local = local.clone();
        let pick = action_pick.clone();
        let render = Rc::downgrade(&render);
        let defs = defs.clone();
        let flush = flush.clone();
        let feedback = labels.1.clone();
        let busy = busy.clone();
        button.connect_clicked(move |_| {
            if op != 1 {
                if let Err(e) = flush() {
                    feedback.set_text(&e);
                    return;
                }
            }
            let mut list = local.borrow_mut();
            let index = pick.selected() as usize;
            match op {
                0 if list.len() < 64 => list.push(Action::new(
                    defs.iter().find(|d| d.name == "no_op").unwrap(),
                )),
                1 if index < list.len() => {
                    list.remove(index);
                }
                2 if index > 0 && index < list.len() => list.swap(index, index - 1),
                3 if index + 1 < list.len() => list.swap(index, index + 1),
                _ => return,
            }
            let next = match op {
                0 => list.len() - 1,
                2 => index - 1,
                3 => index + 1,
                _ => index.min(list.len().saturating_sub(1)),
            };
            drop(list);
            busy.set(true);
            pick.set_selected(next as u32);
            busy.set(false);
            if let Some(r) = render.upgrade() {
                r();
            }
        });
    }
    render();
    let apply = gtk::Button::with_label("Update Binding Draft");
    apply.add_css_class("suggested-action");
    panel.append(&apply);
    let id = bind.id.clone();
    let state = state.clone();
    let status = labels.0.clone();
    let feedback = labels.1.clone();
    let submit: Submit = Rc::new(move || {
        let _ = &render;
        let mut draft = current(&state);
        let bind = draft
            .binds
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or("Binding was removed")?;
        let c = chord()?;
        bind.chord = c;
        bind.submap = scope_ids[scope.selected() as usize].clone();
        bind.flags = flags
            .iter()
            .filter(|(_, c)| c.is_active())
            .map(|(f, _)| f.clone())
            .collect();
        bind.description = describe.is_active().then(|| description.text().into());
        bind.description_alias = alias.is_active();
        let buffer = targets.buffer();
        bind.device = target.is_active().then(|| DeviceFilter {
            inclusive: inclusive.selected() == 0,
            items: buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), true)
                .lines()
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        });
        flush()?;
        bind.actions = local.borrow().clone();
        state.borrow_mut().replace_bindings(draft)?;
        super::summary(&state, &status);
        Ok(())
    });
    let submit_c = submit.clone();
    let redraw_c = redraw_handle.clone();
    apply.connect_clicked(move |_| match submit_c() {
        Ok(()) => {
            feedback.set_text("Binding draft updated; no action was executed.");
            redraw(&redraw_c);
        }
        Err(e) => feedback.set_text(&e),
    });
    let _ = window;
    submit
}

#[cfg(test)]
pub(super) fn test_editor(window: &adw::ApplicationWindow) {
    use hyprland_editor_core::{Value, BASELINE};
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
    fn entry(win: &adw::Window, name: &str) -> gtk::Entry {
        descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap()
            .downcast()
            .unwrap()
    }
    let state = super::initial_state();
    let status = label("");
    let win = show(window, &state, &status);
    click(&win, "Add Binding");
    assert_eq!(current(&state).binds.len(), 1);
    pick(&win, "bind-token-kind").set_selected(3);
    pick(&win, "bind-wheel").set_selected(2);
    click(&win, "Update Binding Draft");
    assert_eq!(current(&state).binds[0].chord.text(), "SUPER + mouse_left");
    let defs = action::catalogue(BASELINE);
    let exec = defs.iter().position(|d| d.name == "exec_cmd").unwrap() as u32;
    let stack = descendants(win.upcast_ref())
        .into_iter()
        .find_map(|w| {
            w.downcast::<gtk::Stack>()
                .ok()
                .filter(|s| s.widget_name() == "binding-pages")
        })
        .unwrap();
    pick(&win, "binding-page").set_selected(2);
    pick(&win, "binding-constructor").set_selected(exec);
    let command = descendants(&stack.visible_child().unwrap())
        .into_iter()
        .find_map(|w| {
            w.downcast::<gtk::Entry>()
                .ok()
                .filter(gtk::prelude::WidgetExt::is_visible)
        })
        .unwrap();
    command.set_text("printf 'draft only'");
    click(&win, "Add Action");
    pick(&win, "binding-action").set_selected(0);
    click(&win, "Update Binding Draft");
    assert_eq!(current(&state).binds[0].actions.len(), 2);
    assert!(current(&state)
        .export_lua(BASELINE)
        .unwrap()
        .contains("draft only"));
    click(&win, "Duplicate");
    assert_eq!(current(&state).binds.len(), 2);
    click(&win, "Undo");
    assert_eq!(current(&state).binds.len(), 1);
    click(&win, "Add Submap");
    entry(&win, "submap-name").set_text("resize");
    click(&win, "Update Submap Draft");
    assert_eq!(current(&state).submaps[0].name, "resize");
    click(&win, "Add Escape Binding");
    assert_eq!(current(&state).binds.len(), 2);
    assert!(current(&state)
        .findings()
        .iter()
        .all(|w| !w.contains("no visible literal transition")));
    click(&win, "Duplicate");
    assert_eq!(current(&state).submaps.len(), 2);
    assert_eq!(current(&state).binds.len(), 3);
    click(&win, "Undo");
    assert_eq!(current(&state).submaps.len(), 1);
    assert_eq!(current(&state).binds.len(), 2);
    pick(&win, "binding-rule").set_selected(1);
    let before = current(&state);
    click(&win, "Remove");
    assert_eq!(
        current(&state),
        before,
        "submap children must not be discarded"
    );
    pick(&win, "binding-rule").set_selected(2);
    click(&win, "Remove");
    assert_eq!(current(&state).binds.len(), 1);
    pick(&win, "binding-rule").set_selected(1);
    click(&win, "Remove");
    assert!(current(&state).submaps.is_empty());
    click(&win, "Undo");
    assert_eq!(current(&state).submaps.len(), 1);
    click(&win, "Undo");
    assert_eq!(current(&state).binds.len(), 2);
    let text = current(&state).export_lua(BASELINE).unwrap();
    let parsed =
        hyprland_editor_core::lua::parse(&text, BASELINE, &hyprland_editor_core::catalogue())
            .unwrap();
    assert!(!parsed.opaque_effects, "{:?}", parsed.diagnostics);
    let workspace = hyprland_editor_core::workspace::Workspace::new(state.borrow().draft.clone())
        .with_bindings(current(&state));
    let saved = workspace
        .to_json(&hyprland_editor_core::catalogue())
        .unwrap();
    let loaded = hyprland_editor_core::workspace::Workspace::from_json(
        std::str::from_utf8(&saved).unwrap(),
        &hyprland_editor_core::catalogue(),
    )
    .unwrap();
    assert_eq!(loaded.bindings, current(&state));
    // Exercise every reviewed constructor/variant form, including optional fields,
    // with the actual GTK controls; this does not execute or certify dispatchers.
    let forms = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let forms_win = adw::Window::builder()
        .default_width(360)
        .default_height(720)
        .build();
    forms_win.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .child(&forms)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    ));
    forms_win.present();
    let context = gtk::glib::MainContext::default();
    let settle = || {
        for _ in 0..4 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    for version in [BASELINE, AUDITED_MAIN] {
        for def in action::catalogue(version) {
            for variant in &def.variants {
                while let Some(child) = forms.first_child() {
                    forms.remove(&child);
                }
                let fields = variant
                    .fields
                    .iter()
                    .map(|f| {
                        let mut v = f.initial();
                        if !f.choices.is_empty() {
                            v = Value::String(f.choices[0].clone());
                        }
                        (f.name.clone(), v)
                    })
                    .collect();
                let initial = Action {
                    constructor: def.name.clone(),
                    variant: variant.id.clone(),
                    at_load: false,
                    fields,
                };
                let get = bind_forms::action(&forms, &initial, version);
                assert_eq!(
                    get().unwrap_or_else(|e| panic!("{} {}: {e}", def.name, variant.id)),
                    initial
                );
                settle();
                let (min, _, _, _) = forms.measure(gtk::Orientation::Horizontal, -1);
                assert!(min <= 328, "{} {} needs {min}px", def.name, variant.id);
            }
        }
    }
    // Preserve numeric-selector types, large integers and values above generic scalar limits.
    for v in [
        Value::Int(i64::MAX),
        Value::Float(1.5),
        Value::String("name:work".into()),
    ] {
        let field = action::find("focus", BASELINE)
            .unwrap()
            .variants
            .iter()
            .find(|v| v.id == "workspace")
            .unwrap()
            .fields[0]
            .clone();
        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let get = super::bind_forms::test_field(&box_, &field, v.clone());
        assert_eq!(get().unwrap(), v);
    }
    let number = action::find("cursor.move", BASELINE).unwrap().variants[0].fields[0].clone();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let get = super::bind_forms::test_field(&box_, &number, Value::Float(1e20));
    assert_eq!(get().unwrap(), Value::Float(1e20));
    let string = action::find("exec_cmd", BASELINE).unwrap().variants[0].fields[0].clone();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let v = Value::String("printf 'line\nquote\"\t'".into());
    let get = super::bind_forms::test_field(&body, &string, v.clone());
    assert_eq!(get().unwrap(), v);
    super::bind_forms::test_recording();
    forms_win.destroy();
    pick(&win, "binding-rule").set_selected(0);
    win.set_default_size(360, 760);
    settle();
    for name in ["shortcut", "options", "actions"] {
        pick(&win, "binding-page").set_selected(
            ["shortcut", "options", "actions"]
                .iter()
                .position(|n| *n == name)
                .unwrap() as u32,
        );
        settle();
        let (min, _, _, _) = win
            .content()
            .unwrap()
            .measure(gtk::Orientation::Horizontal, -1);
        assert!(min <= 350, "Binding {name} needs {min}px");
    }
    win.set_default_size(780, 760);
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
            .save_to_png(format!("{path}.bindings.png"))
            .unwrap();
    }
    let weak = pick(&win, "binding-constructor").downgrade();
    win.destroy();
    drop(win);
    settle();
    assert!(
        weak.upgrade().is_none(),
        "Binding editor retained action controls after closure"
    );
}
