//! Native actions stay plain text; this form never constructs Lua callbacks.
use super::*;
fn dispatcher(parent: &gtk::Box, name: &str, argument: &str) -> ActionGet {
    let chosen = entry(parent, "Dispatcher", name, "gesture-native-dispatcher");
    let mut names = vec!["Custom dispatcher".to_string()];
    names.extend(
        hyprland_editor_core::legacy_binding::dispatchers()
            .iter()
            .cloned(),
    );
    let captions = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if i == 0 {
                n.clone()
            } else {
                format!(
                    "{} ({n})",
                    super::super::legacy_binding_editor::action_name(n)
                )
            }
        })
        .collect::<Vec<_>>();
    let pick =
        gtk::DropDown::from_strings(&captions.iter().map(String::as_str).collect::<Vec<_>>());
    compact(&pick);
    pick.set_widget_name("gesture-native-dispatcher-choice");
    pick.set_selected(names.iter().position(|s| s == name).unwrap_or(0) as u32);
    parent.append(&pick);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    parent.append(&body);
    let get = Rc::new(RefCell::new(None::<Box<dyn Fn() -> String>>));
    let render: Rc<dyn Fn()> = Rc::new({
        let body = body.clone();
        let chosen = chosen.clone();
        let get = get.clone();
        let initial = argument.to_string();
        move || {
            let current = get
                .borrow()
                .as_ref()
                .map_or_else(|| initial.clone(), |g| g());
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            let getter =
                super::super::legacy_binding_editor::argument(&body, &chosen.text(), &current);
            *get.borrow_mut() = Some(getter);
        }
    });
    let weak = Rc::downgrade(&render);
    chosen.connect_changed(move |_| {
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    let weak = pick.downgrade();
    let known = names.clone();
    chosen.connect_changed(move |e| {
        if let Some(p) = weak.upgrade() {
            let index = known.iter().position(|n| *n == e.text()).unwrap_or(0) as u32;
            if p.selected() != index {
                p.set_selected(index);
            }
        }
    });
    let weak = chosen.downgrade();
    pick.connect_selected_notify(move |p| {
        if p.selected() == 0 {
            return;
        }
        if let (Some(chosen), Some(name)) = (weak.upgrade(), names.get(p.selected() as usize)) {
            if chosen.text() != *name {
                chosen.set_text(name);
            }
        }
    });
    parent.append(&label("The dispatcher and its argument are stored without running. In Hyprland this dispatcher is invoked when the gesture finishes."));
    render();
    Rc::new(move || {
        let _ = &render;
        Ok(GestureAction::NativeDispatcher {
            name: chosen.text().into(),
            argument: get.borrow().as_ref().map(|g| g()).unwrap_or_default(),
        })
    })
}
pub(super) fn form(parent: &gtk::Box, original: &GestureAction) -> ActionGet {
    let mut names = gesture::BUILTINS
        .iter()
        .map(|s| super::super::human_name(s))
        .collect::<Vec<_>>();
    names.push("Run Native Dispatcher".into());
    let pick = gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
    compact(&pick);
    pick.set_widget_name("gesture-native-action");
    parent.append(&pick);
    let initial = match original {
        GestureAction::Builtin(name) => gesture::BUILTINS
            .iter()
            .position(|s| *s == name)
            .unwrap_or(0),
        _ => gesture::BUILTINS.len(),
    };
    pick.set_selected(initial as u32);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    parent.append(&body);
    let saved = Rc::new(RefCell::new(match original {
        GestureAction::NativeDispatcher { .. } => original.clone(),
        _ => GestureAction::NativeDispatcher {
            name: "exec".into(),
            argument: String::new(),
        },
    }));
    let get = Rc::new(RefCell::new(None::<ActionGet>));
    let render: Rc<dyn Fn()> = Rc::new({
        let pick = pick.clone();
        let body = body.clone();
        let saved = saved.clone();
        let get = get.clone();
        move || {
            if let Some(g) = get.borrow().as_ref() {
                if let Ok(action @ GestureAction::NativeDispatcher { .. }) = g() {
                    *saved.borrow_mut() = action;
                }
            }
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            let next: ActionGet =
                if let Some(name) = gesture::BUILTINS.get(pick.selected() as usize) {
                    let name = (*name).to_string();
                    Rc::new(move || Ok(GestureAction::Builtin(name.clone())))
                } else {
                    let saved = saved.borrow();
                    let GestureAction::NativeDispatcher { name, argument } = &*saved else {
                        unreachable!()
                    };
                    dispatcher(&body, name, argument)
                };
            *get.borrow_mut() = Some(next);
        }
    });
    let weak = Rc::downgrade(&render);
    pick.connect_selected_notify(move |_| {
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    render();
    Rc::new(move || {
        let _ = &render;
        get.borrow()
            .as_ref()
            .ok_or("Choose a native gesture action")?()
    })
}
