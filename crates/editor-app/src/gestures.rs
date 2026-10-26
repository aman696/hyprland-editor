//! Selected gesture forms; callback blocks and data samples stay inert.
use super::{label, monitors::compact, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    action,
    gesture::{
        self, Callbacks, Draft, EventKind, EventSample, GestureAction, Phase, Spec, Workflow,
    },
    Value, AUDITED_MAIN,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Submit = Rc<dyn Fn() -> Result<(), String>>;
type ActionGet = Rc<dyn Fn() -> Result<GestureAction, String>>;
fn current(state: &Shared) -> Draft {
    let s = state.borrow();
    s.gestures
        .get(&s.draft.version)
        .cloned()
        .unwrap_or_default()
}
fn title(spec: &Spec) -> String {
    let action = match &spec.action {
        GestureAction::Builtin(n) => super::human_name(n),
        GestureAction::Callbacks(_) => "Phase Action Blocks".into(),
        GestureAction::LegacyFinish(_) => "Legacy Finish Blocks".into(),
    };
    format!(
        "{} fingers · {} · {}",
        display(&spec.fingers),
        super::human_name(&spec.direction),
        action
    )
}
fn unique(d: &Draft) -> String {
    (0..)
        .map(|n| format!("gesture-{n}"))
        .find(|id| d.declarations.iter().all(|s| s.id != *id))
        .unwrap()
}
fn entry(parent: &gtk::Box, title: &str, initial: &str, name: &str) -> gtk::Entry {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = label(title);
    title.set_width_chars(12);
    row.append(&title);
    let e = gtk::Entry::builder()
        .text(initial)
        .max_length(16384)
        .hexpand(true)
        .build();
    e.set_widget_name(name);
    row.append(&e);
    parent.append(&row);
    e
}
fn display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        v => v.lua(),
    }
}
fn text_field(
    parent: &gtk::Box,
    title: &str,
    original: Option<Value>,
    name: &str,
) -> Box<dyn Fn() -> Option<Value>> {
    let set = gtk::CheckButton::with_label(&format!("Set {title}"));
    set.set_active(original.is_some());
    parent.append(&set);
    let shown = original.as_ref().map_or(String::new(), display);
    let e = entry(parent, title, &shown, name);
    e.set_sensitive(set.is_active());
    let weak = e.downgrade();
    set.connect_toggled(move |set| {
        if let Some(e) = weak.upgrade() {
            e.set_sensitive(set.is_active());
        }
    });
    Box::new(move || {
        if !set.is_active() {
            None
        } else if e.text().as_str() == shown {
            original
                .clone()
                .or_else(|| Some(Value::String(String::new())))
        } else {
            Some(Value::String(e.text().into()))
        }
    })
}
fn workflow_form(
    parent: &gtk::Box,
    original: &Workflow,
    version: &str,
    feedback: &gtk::Label,
) -> Rc<dyn Fn() -> Result<Workflow, String>> {
    let draft = Rc::new(RefCell::new(original.clone()));
    let version = version.to_string();
    let model = gtk::StringList::new(&[]);
    let pick = gtk::DropDown::builder().model(&model).build();
    compact(&pick);
    pick.set_widget_name("gesture-step");
    parent.append(&pick);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let buttons =
        ["Add Action Block", "Remove Block", "↑ Block", "↓ Block"].map(gtk::Button::with_label);
    for b in &buttons {
        row.append(b);
    }
    parent.append(&row);
    let defs = action::catalogue(&version);
    let names = defs.iter().map(|d| d.name.clone()).collect::<Vec<_>>();
    let titles = names
        .iter()
        .map(|n| super::human_name(n))
        .collect::<Vec<_>>();
    let constructor =
        gtk::DropDown::from_strings(&titles.iter().map(String::as_str).collect::<Vec<_>>());
    compact(&constructor);
    constructor.set_widget_name("gesture-constructor");
    parent.append(&constructor);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    parent.append(&body);
    let busy = Rc::new(Cell::new(false));
    let active = Rc::new(Cell::new(0usize));
    let get = Rc::new(RefCell::new(
        None::<Box<dyn Fn() -> Result<action::Action, String>>>,
    ));
    let flush: Submit = Rc::new({
        let draft = draft.clone();
        let active = active.clone();
        let get = get.clone();
        move || {
            if let Some(g) = get.borrow().as_ref() {
                let a = g()?;
                let mut d = draft.borrow_mut();
                if let Some(slot) = d.actions.get_mut(active.get()) {
                    *slot = a;
                }
            }
            Ok(())
        }
    });
    let render: Rc<dyn Fn()> = Rc::new({
        let model = model.clone();
        let pick = pick.clone();
        let constructor = constructor.clone();
        let body = body.clone();
        let busy = busy.clone();
        let active = active.clone();
        let draft = draft.clone();
        let get = get.clone();
        let names = names.clone();
        let version = version.clone();
        move || {
            busy.set(true);
            let d = draft.borrow();
            let index = active.get().min(d.actions.len().saturating_sub(1));
            active.set(index);
            let titles = d
                .actions
                .iter()
                .enumerate()
                .map(|(i, a)| format!("{} · {}", i + 1, super::human_name(&a.constructor)))
                .collect::<Vec<_>>();
            model.splice(
                0,
                model.n_items(),
                &titles.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            pick.set_selected(if d.actions.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            constructor.set_sensitive(!d.actions.is_empty());
            *get.borrow_mut() = d.actions.get(index).map(|a| {
                constructor
                    .set_selected(names.iter().position(|n| n == &a.constructor).unwrap() as u32);
                super::bind_forms::action(&body, a, &version)
            });
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&render);
    let f = flush.clone();
    let a = active.clone();
    let b = busy.clone();
    let message = feedback.clone();
    pick.connect_selected_notify(move |p| {
        if b.get() {
            return;
        }
        if let Err(e) = f() {
            b.set(true);
            p.set_selected(a.get() as u32);
            b.set(false);
            message.set_text(&e);
            return;
        }
        a.set(p.selected() as usize);
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    let weak = Rc::downgrade(&render);
    let d = draft.clone();
    let f = flush.clone();
    let a = active.clone();
    let b = busy.clone();
    let names_c = names.clone();
    let v = version.clone();
    let message = feedback.clone();
    constructor.connect_selected_notify(move |p| {
        if b.get() {
            return;
        }
        if let Err(e) = f() {
            b.set(true);
            if let Some(old) = d.borrow().actions.get(a.get()) {
                p.set_selected(names_c.iter().position(|n| n == &old.constructor).unwrap() as u32);
            }
            b.set(false);
            message.set_text(&e);
            return;
        }
        let Some(name) = names_c.get(p.selected() as usize) else {
            return;
        };
        let def = action::find(name, &v).unwrap();
        let mut draft = d.borrow_mut();
        if let Some(slot) = draft.actions.get_mut(a.get()) {
            let at_load = slot.at_load;
            *slot = action::Action::new(def);
            slot.at_load = at_load;
        }
        drop(draft);
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    for (op, button) in buttons.iter().enumerate() {
        let d = draft.clone();
        let a = active.clone();
        let f = flush.clone();
        let r = render.clone();
        let v = version.clone();
        let message = feedback.clone();
        button.connect_clicked(move |_| {
            if op != 1 {
                if let Err(e) = f() {
                    message.set_text(&e);
                    return;
                }
            }
            let mut d = d.borrow_mut();
            let index = a.get();
            match op {
                0 => {
                    if d.actions.len() >= 64 {
                        message.set_text("A phase supports at most 64 action blocks");
                        return;
                    }
                    let def = action::find("no_op", &v).unwrap();
                    d.actions.push(action::Action::new(def));
                    a.set(d.actions.len() - 1);
                }
                1 => {
                    if index < d.actions.len() {
                        d.actions.remove(index);
                    }
                }
                2 | 3 => {
                    let next = if op == 2 {
                        index.checked_sub(1)
                    } else {
                        index.checked_add(1)
                    };
                    if let Some(next) = next.filter(|n| *n < d.actions.len()) {
                        d.actions.swap(index, next);
                        a.set(next);
                    }
                }
                _ => {}
            }
            drop(d);
            r();
        });
    }
    render();
    let version = version.clone();
    Rc::new(move || {
        let _ = &render;
        flush()?;
        let d = draft.borrow().clone();
        d.validate(&version)?;
        Ok(d)
    })
}
fn callback_form(
    parent: &gtk::Box,
    original: &Callbacks,
    version: &str,
    feedback: &gtk::Label,
) -> ActionGet {
    parent.append(&label("Build action blocks for each phase using the values you set here. Editing actions that read gesture data is not available yet."));
    let phases = gtk::DropDown::from_strings(&["Start", "Update", "Finish"]);
    phases.set_widget_name("gesture-phase");
    parent.append(&phases);
    let enabled = gtk::CheckButton::with_label("Use this phase");
    enabled.set_widget_name("gesture-phase-enabled");
    parent.append(&enabled);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    parent.append(&body);
    let draft = Rc::new(RefCell::new(original.clone()));
    let active = Rc::new(Cell::new(0usize));
    let busy = Rc::new(Cell::new(false));
    type WorkflowGet = Rc<dyn Fn() -> Result<Workflow, String>>;
    let get = Rc::new(RefCell::new(None::<WorkflowGet>));
    let flush: Submit = Rc::new({
        let d = draft.clone();
        let a = active.clone();
        let get = get.clone();
        let enabled = enabled.clone();
        move || {
            let value = if enabled.is_active() {
                Some(get.borrow().as_ref().ok_or("Phase form missing")?()?)
            } else {
                None
            };
            let mut d = d.borrow_mut();
            match a.get() {
                0 => d.start = value,
                1 => d.update = value,
                _ => d.finish = value,
            };
            Ok(())
        }
    });
    let render: Rc<dyn Fn()> = Rc::new({
        let d = draft.clone();
        let a = active.clone();
        let b = busy.clone();
        let body = body.clone();
        let enabled = enabled.clone();
        let get = get.clone();
        let version = version.to_string();
        let feedback = feedback.clone();
        move || {
            b.set(true);
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            let d = d.borrow();
            let original = d.phases()[a.get()].cloned();
            enabled.set_active(original.is_some());
            body.set_sensitive(enabled.is_active());
            *get.borrow_mut() = Some(workflow_form(
                &body,
                &original.unwrap_or_default(),
                &version,
                &feedback,
            ));
            b.set(false);
        }
    });
    let weak = Rc::downgrade(&render);
    let f = flush.clone();
    let a = active.clone();
    let b = busy.clone();
    let message = feedback.clone();
    phases.connect_selected_notify(move |p| {
        if b.get() {
            return;
        }
        if let Err(e) = f() {
            b.set(true);
            p.set_selected(a.get() as u32);
            b.set(false);
            message.set_text(&e);
            return;
        }
        a.set(p.selected() as usize);
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    let weak = body.downgrade();
    enabled.connect_toggled(move |e| {
        if let Some(body) = weak.upgrade() {
            body.set_sensitive(e.is_active());
        }
    });
    render();
    Rc::new(move || {
        let _ = &render;
        flush()?;
        Ok(GestureAction::Callbacks(draft.borrow().clone()))
    })
}
fn action_form(
    parent: &gtk::Box,
    original: &GestureAction,
    version: &str,
    feedback: &gtk::Label,
) -> ActionGet {
    let kind = gtk::DropDown::from_strings(&[
        "Built-in Gesture",
        "Phase Action Blocks",
        "Legacy Finish Action Blocks",
    ]);
    kind.set_widget_name("gesture-action-kind");
    parent.append(&kind);
    let index = match original {
        GestureAction::Builtin(_) => 0,
        GestureAction::Callbacks(_) => 1,
        GestureAction::LegacyFinish(_) => 2,
    };
    kind.set_selected(index);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    parent.append(&body);
    let active = Rc::new(Cell::new(index));
    let busy = Rc::new(Cell::new(false));
    let draft = Rc::new(RefCell::new(original.clone()));
    let get = Rc::new(RefCell::new(None::<ActionGet>));
    let flush: Submit = Rc::new({
        let draft = draft.clone();
        let get = get.clone();
        move || {
            if let Some(g) = get.borrow().as_ref() {
                *draft.borrow_mut() = g()?;
            }
            Ok(())
        }
    });
    let render: Rc<dyn Fn()> = Rc::new({
        let body = body.clone();
        let draft = draft.clone();
        let get = get.clone();
        let version = version.to_string();
        let feedback = feedback.clone();
        move || {
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            let next: ActionGet = match &*draft.borrow() {
                GestureAction::Builtin(name) => {
                    let choices = gtk::DropDown::from_strings(&[
                        "Switch workspace",
                        "Resize window",
                        "Move window",
                        "Special workspace",
                        "Close window",
                        "Float or tile window",
                        "Fullscreen or maximize",
                        "Cursor zoom",
                        "Cursor zoom (alias)",
                        "Scroll movement",
                        "Remove registration",
                    ]);
                    compact(&choices);
                    choices.set_widget_name("gesture-builtin");
                    choices.set_selected(
                        gesture::BUILTINS
                            .iter()
                            .position(|s| *s == name)
                            .unwrap_or(0) as u32,
                    );
                    body.append(&choices);
                    Rc::new(move || {
                        Ok(GestureAction::Builtin(
                            gesture::BUILTINS
                                .get(choices.selected() as usize)
                                .ok_or("Choose a gesture action")?
                                .to_string(),
                        ))
                    })
                }
                GestureAction::Callbacks(c) => callback_form(&body, c, &version, &feedback),
                GestureAction::LegacyFinish(w) => {
                    body.append(&label("Legacy finish callbacks receive no event data."));
                    let g = workflow_form(&body, w, &version, &feedback);
                    Rc::new(move || Ok(GestureAction::LegacyFinish(g()?)))
                }
            };
            *get.borrow_mut() = Some(next);
        }
    });
    let weak = Rc::downgrade(&render);
    let f = flush.clone();
    let d = draft.clone();
    let a = active.clone();
    let b = busy.clone();
    let message = feedback.clone();
    kind.connect_selected_notify(move |p| {
        if b.get() {
            return;
        }
        if let Err(e) = f() {
            b.set(true);
            p.set_selected(a.get());
            b.set(false);
            message.set_text(&e);
            return;
        }
        a.set(p.selected());
        *d.borrow_mut() = match p.selected() {
            1 => GestureAction::Callbacks(Callbacks {
                start: Some(Workflow::default()),
                ..Default::default()
            }),
            2 => GestureAction::LegacyFinish(Workflow::default()),
            _ => GestureAction::Builtin("workspace".into()),
        };
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    render();
    Rc::new(move || {
        let _ = &render;
        flush()?;
        Ok(draft.borrow().clone())
    })
}
fn editor(
    parent: &gtk::Box,
    state: &Shared,
    original: &Spec,
    status: &gtk::Label,
    feedback: &gtk::Label,
    changed: Rc<dyn Fn()>,
) -> Submit {
    let version = state.borrow().draft.version.clone();
    let outer = parent;
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::None)
        .build();
    stack.set_widget_name("gesture-details");
    let matching = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let action_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    stack.add_titled(&matching, Some("matching"), "Matching & Motion");
    stack.add_titled(&action_panel, Some("action"), "Action");
    outer.append(
        &gtk::StackSwitcher::builder()
            .stack(&stack)
            .halign(gtk::Align::Center)
            .build(),
    );
    outer.append(&stack);
    let parent = &matching;
    let fingers = gtk::SpinButton::with_range(2., 9., 1.);
    fingers.set_value(if let Value::Int(n) = original.fingers {
        n as f64
    } else {
        3.
    });
    fingers.set_widget_name("gesture-fingers");
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.append(&label("Fingers"));
    row.append(&fingers);
    parent.append(&row);
    let direction = gtk::DropDown::from_strings(&[
        "Any swipe",
        "Left",
        "Right",
        "Up",
        "Down",
        "Horizontal",
        "Vertical",
        "Any pinch",
        "Pinch in",
        "Pinch out",
    ]);
    compact(&direction);
    direction.set_widget_name("gesture-direction");
    let original_direction = gesture::Direction::parse(&original.direction).unwrap();
    let direction_index = gesture::DIRECTIONS
        .iter()
        .position(|s| *s == original_direction.name())
        .unwrap() as u32;
    direction.set_selected(direction_index);
    parent.append(&direction);
    let modifiers = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(2)
        .max_children_per_line(4)
        .homogeneous(true)
        .build();
    let mask = gesture::modifier_mask(original.mods.as_ref()).unwrap();
    let checks = gesture::MODIFIERS
        .iter()
        .map(|(name, bit)| {
            let check = gtk::CheckButton::with_label(name);
            check.set_active(mask & bit != 0);
            modifiers.insert(&check, -1);
            (*name, *bit, check)
        })
        .collect::<Vec<_>>();
    parent.append(
        &gtk::Expander::builder()
            .label("Modifiers — exact match")
            .child(&modifiers)
            .build(),
    );
    let scale_set = gtk::CheckButton::with_label("Set motion multiplier");
    scale_set.set_widget_name("gesture-scale-set");
    scale_set.set_active(original.scale.is_some());
    parent.append(&scale_set);
    let scale_row = gtk::Box::new(gtk::Orientation::Vertical, 4);
    parent.append(&scale_row);
    let scale_shown = original.scale.as_ref().map_or("1".into(), display);
    let scale = entry(&scale_row, "Multiplier", &scale_shown, "gesture-scale");
    let n = original
        .scale
        .as_ref()
        .map(|v| hyprland_editor_core::animation::number(v, true).unwrap())
        .unwrap_or(1.);
    let invert = gtk::CheckButton::with_label("Invert motion");
    invert.set_sensitive(version == AUDITED_MAIN);
    invert.set_active(n < 0.);
    scale_row.append(&invert);
    let slider = gtk::Scale::with_range(
        gtk::Orientation::Horizontal,
        if version == AUDITED_MAIN {
            0.1
        } else {
            gesture::MIN_SCALE
        },
        10.,
        0.01,
    );
    slider.set_draw_value(false);
    slider.set_value(n.abs().clamp(gesture::MIN_SCALE, 10.));
    scale_row.append(&slider);
    let weak = scale.downgrade();
    let invert_c = invert.clone();
    slider.connect_value_changed(move |s| {
        if let Some(e) = weak.upgrade() {
            e.set_text(&(s.value() * if invert_c.is_active() { -1. } else { 1. }).to_string());
        }
    });
    let weak = scale.downgrade();
    invert.connect_toggled(move |invert| {
        if let Some(e) = weak.upgrade() {
            if let Ok(n) = e.text().parse::<f64>() {
                e.set_text(&(n.abs() * if invert.is_active() { -1. } else { 1. }).to_string());
            }
        }
    });
    scale_row.set_sensitive(scale_set.is_active());
    let weak = scale_row.downgrade();
    scale_set.connect_toggled(move |c| {
        if let Some(row) = weak.upgrade() {
            row.set_sensitive(c.is_active());
        }
    });
    let inhibit = gtk::CheckButton::with_label("Allow while shortcuts are inhibited");
    inhibit.set_widget_name("gesture-disable-inhibit");
    let initial_inhibit = original
        .disable_inhibit
        .as_ref()
        .map(hyprland_editor_core::animation::enabled)
        .transpose()
        .unwrap()
        .unwrap_or(false);
    inhibit.set_active(initial_inhibit);
    parent.append(&inhibit);
    let extra = gtk::Box::new(gtk::Orientation::Vertical, 4);
    parent.append(
        &gtk::Expander::builder()
            .label("Extra action parameters")
            .child(&extra)
            .build(),
    );
    let mode = text_field(&extra, "Mode", original.mode.clone(), "gesture-mode");
    let zoom = text_field(
        &extra,
        "Zoom Level",
        original.zoom_level.clone(),
        "gesture-zoom",
    );
    let workspace = text_field(
        &extra,
        "Workspace Name",
        original.workspace_name.clone(),
        "gesture-workspace",
    );
    extra.append(&label("Mode: float/tile/toggle; fullscreen/maximize; cursor zoom mult/live/toggle. Parameters are retained when a different action ignores them."));
    let action = action_form(&action_panel, &original.action, &version, feedback);
    let original = original.clone();
    let state = state.clone();
    let status = status.clone();
    let submit: Submit = Rc::new(move || {
        let mut spec = original.clone();
        fingers.update();
        spec.fingers = Value::Int(fingers.value() as i64);
        if direction.selected() != direction_index {
            spec.direction = gesture::DIRECTIONS
                .get(direction.selected() as usize)
                .ok_or("Choose a direction")?
                .to_string();
        }
        let new_mask = checks.iter().fold(
            0,
            |mask, (_, bit, c)| if c.is_active() { mask | bit } else { mask },
        );
        if new_mask != mask {
            let raw = checks
                .iter()
                .filter(|(_, _, c)| c.is_active())
                .map(|(name, _, _)| *name)
                .collect::<Vec<_>>()
                .join(" ");
            spec.mods = (!raw.is_empty()).then_some(Value::String(raw));
        }
        spec.scale = if !scale_set.is_active() {
            None
        } else if scale.text().as_str() == scale_shown {
            original.scale.clone().or(Some(Value::Int(1)))
        } else {
            Some(Value::Float(
                scale
                    .text()
                    .parse::<f64>()
                    .map_err(|_| "Enter a finite motion multiplier")?,
            ))
        };
        if inhibit.is_active() != initial_inhibit {
            spec.disable_inhibit = Some(Value::Bool(inhibit.is_active()));
        }
        spec.mode = mode();
        spec.zoom_level = zoom();
        spec.workspace_name = workspace();
        spec.action = action()?;
        let mut draft = current(&state);
        let slot = draft
            .declarations
            .iter_mut()
            .find(|s| s.id == spec.id)
            .ok_or("Gesture declaration removed")?;
        *slot = spec;
        state.borrow_mut().replace_gestures(draft)?;
        changed();
        super::summary(&state, &status);
        Ok(())
    });
    let update = gtk::Button::with_label("Update Gesture Draft");
    update.add_css_class("suggested-action");
    outer.append(&update);
    let weak = Rc::downgrade(&submit);
    let feedback = feedback.clone();
    update.connect_clicked(move |_| {
        if let Some(g) = weak.upgrade() {
            feedback.set_text(&match g() {
                Ok(()) => "Gesture draft updated".into(),
                Err(e) => e,
            });
        }
    });
    submit
}
pub(super) fn connect(
    button: &gtk::Button,
    parent: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) {
    let parent = parent.downgrade();
    let state = state.clone();
    let status = status.clone();
    button.connect_clicked(move |_| {
        if let Some(parent) = parent.upgrade() {
            show(&parent, &state, &status);
        }
    });
}
pub(super) fn show(
    parent: &impl IsA<gtk::Window>,
    state: &Shared,
    status: &gtk::Label,
) -> adw::Window {
    let win = adw::Window::builder()
        .title("Gestures")
        .transient_for(parent)
        .modal(true)
        .default_width(700)
        .default_height(760)
        .build();
    win.set_size_request(350, 480);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for setter in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        setter(content.upcast_ref(), 16);
    }
    content.append(&label("Create private gesture drafts and callback action blocks. Declaration order affects registration and matching."));
    let model = gtk::StringList::new(&[]);
    let pick = gtk::DropDown::builder().model(&model).build();
    compact(&pick);
    pick.set_widget_name("gesture-declaration");
    content.append(&pick);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let row2 = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let buttons = [
        "Add Gesture",
        "Duplicate",
        "Remove",
        "Create Unset",
        "↑",
        "↓",
        "Undo",
    ]
    .map(gtk::Button::with_label);
    for b in &buttons[..4] {
        row.append(b);
    }
    for b in &buttons[4..] {
        row2.append(b);
    }
    content.append(&row);
    content.append(&row2);
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.append(&panel);
    let feedback = label("");
    feedback.set_widget_name("gesture-feedback");
    content.append(&feedback);
    let check = gtk::Button::with_label("Check Gesture Relationships");
    content.append(&check);
    let sample_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let sample_kind = gtk::DropDown::from_strings(&["Swipe", "Pinch"]);
    let sample_phase = gtk::DropDown::from_strings(&["Start", "Update", "Finish"]);
    sample_box.append(&sample_kind);
    sample_box.append(&sample_phase);
    let sample = gtk::Button::with_label("Inspect Sample Callback Data");
    sample_box.append(&sample);
    sample_box.append(&label("Synthetic data only. This does not run callbacks, inject touchpad input or preview compositor effects."));
    let sample_text = label("");
    sample_box.append(&sample_text);
    content.append(
        &gtk::Expander::builder()
            .label("Callback Data Sample")
            .child(&sample_box)
            .build(),
    );
    sample.connect_clicked(move |_| {
        let kind = if sample_kind.selected() == 0 {
            EventKind::Swipe
        } else {
            EventKind::Pinch
        };
        let phase = match sample_phase.selected() {
            0 => Phase::Start,
            1 => Phase::Update,
            _ => Phase::Finish,
        };
        let event = EventSample {
            phase,
            kind,
            direction: if kind == EventKind::Swipe {
                gesture::Direction::Left
            } else {
                gesture::Direction::PinchIn
            },
            time_ms: 123,
            fingers: 3,
            delta: [-12., 5.],
            scale: 1.2,
            rotation: 10.,
            cancelled: false,
        };
        sample_text.set_text(&serde_json::to_string_pretty(&event.payload().unwrap()).unwrap());
    });
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    win.set_content(Some(&root));
    let busy = Rc::new(Cell::new(false));
    let active = Rc::new(Cell::new(0usize));
    let submit = Rc::new(RefCell::new(None::<Submit>));
    let flush: Submit = Rc::new({
        let submit = submit.clone();
        move || {
            if let Some(g) = submit.borrow().as_ref() {
                g()?;
            }
            Ok(())
        }
    });
    let render: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let model = model.clone();
        let pick = pick.clone();
        let panel = panel.clone();
        let submit = submit.clone();
        let busy = busy.clone();
        let active = active.clone();
        move || {
            busy.set(true);
            let draft = current(&state);
            let index = active.get().min(draft.declarations.len().saturating_sub(1));
            active.set(index);
            let titles = draft.declarations.iter().map(title).collect::<Vec<_>>();
            model.splice(
                0,
                model.n_items(),
                &titles.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            pick.set_selected(if draft.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            while let Some(child) = panel.first_child() {
                panel.remove(&child);
            }
            *submit.borrow_mut() = draft.declarations.get(index).map(|s| {
                let changed: Rc<dyn Fn()> = Rc::new({
                    let state = state.clone();
                    let model = model.clone();
                    let pick = pick.downgrade();
                    let busy = busy.clone();
                    move || {
                        let Some(pick) = pick.upgrade() else {
                            return;
                        };
                        let selected = pick.selected();
                        let titles = current(&state)
                            .declarations
                            .iter()
                            .map(title)
                            .collect::<Vec<_>>();
                        let was_busy = busy.replace(true);
                        model.splice(
                            0,
                            model.n_items(),
                            &titles.iter().map(String::as_str).collect::<Vec<_>>(),
                        );
                        pick.set_selected(selected);
                        busy.set(was_busy);
                    }
                });
                editor(&panel, &state, s, &status, &feedback, changed)
            });
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&render);
    let f = flush.clone();
    let a = active.clone();
    let b = busy.clone();
    let message = feedback.clone();
    pick.connect_selected_notify(move |p| {
        if b.get() {
            return;
        }
        if let Err(e) = f() {
            b.set(true);
            p.set_selected(a.get() as u32);
            b.set(false);
            message.set_text(&e);
            return;
        }
        a.set(p.selected() as usize);
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    for (op, button) in buttons.iter().enumerate() {
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let active = active.clone();
        let flush = flush.clone();
        let render = render.clone();
        button.connect_clicked(move |_| {
            if op != 2 && op != 6 {
                if let Err(e) = flush() {
                    feedback.set_text(&e);
                    return;
                }
            }
            let mut draft = current(&state);
            let index = active.get();
            let mut target = index;
            if op == 6 {
                state.borrow_mut().undo_last();
            } else {
                match op {
                    0 => {
                        let id = unique(&draft);
                        draft.declarations.push(Spec::new(&id));
                        target = draft.declarations.len() - 1;
                    }
                    1 | 3 => {
                        let Some(old) = draft.declarations.get(index) else {
                            return;
                        };
                        let mut spec = if op == 3 { old.unset() } else { old.clone() };
                        spec.id = unique(&draft);
                        target = index + 1;
                        draft.declarations.insert(target, spec);
                    }
                    2 => {
                        if index < draft.declarations.len() {
                            draft.declarations.remove(index);
                        }
                    }
                    4 | 5 => {
                        let next = if op == 4 {
                            index.checked_sub(1)
                        } else {
                            index.checked_add(1)
                        };
                        if let Some(next) = next.filter(|n| *n < draft.declarations.len()) {
                            draft.declarations.swap(index, next);
                            target = next;
                        }
                    }
                    _ => {}
                }
                if let Err(e) = state.borrow_mut().replace_gestures(draft) {
                    feedback.set_text(&e);
                    return;
                }
            }
            active.set(target);
            render();
            super::summary(&state, &status);
            feedback.set_text("Gesture draft changed. Check relationships before integration.");
        });
    }
    let f = flush.clone();
    let state_c = state.clone();
    let feedback_c = feedback.clone();
    check.connect_clicked(move |_| {
        if let Err(e) = f() {
            feedback_c.set_text(&e);
            return;
        }
        let s = state_c.borrow();
        let d = s
            .gestures
            .get(&s.draft.version)
            .cloned()
            .unwrap_or_default();
        let message = match d.findings(&s.draft.version) {
            Ok(messages) if messages.is_empty() => {
                "No registration conflicts found in this draft. Runtime state remains unverified."
                    .into()
            }
            Ok(messages) => messages.join("\n").chars().take(4096).collect(),
            Err(e) => e,
        };
        feedback_c.set_text(&message);
    });
    render();
    win.connect_close_request(move |_| {
        let _ = &render;
        if let Err(e) = flush() {
            feedback.set_text(&e);
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
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
    fn widget<T: IsA<gtk::Widget> + gtk::glib::object::ObjectType>(
        win: &adw::Window,
        name: &str,
    ) -> T {
        descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap_or_else(|| panic!("Missing gesture widget {name}"))
            .downcast()
            .unwrap()
    }
    fn click(win: &adw::Window, name: &str) {
        descendants(win.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(name))
            .unwrap_or_else(|| panic!("Missing button {name}"))
            .emit_clicked();
    }
    fn capture(win: &adw::Window, suffix: &str) {
        let context = gtk::glib::MainContext::default();
        for _ in 0..10 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
            let paintable = gtk::WidgetPaintable::new(Some(win));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
            win.renderer()
                .unwrap()
                .render_texture(snapshot.to_node().unwrap(), None)
                .save_to_png(format!("{path}.{suffix}.png"))
                .unwrap();
        }
    }
    let state = super::initial_state();
    let status = label("");
    let win = show(parent, &state, &status);
    click(&win, "Add Gesture");
    widget::<gtk::SpinButton>(&win, "gesture-fingers").set_value(4.);
    widget::<gtk::DropDown>(&win, "gesture-direction").set_selected(1);
    widget::<gtk::CheckButton>(&win, "gesture-scale-set").set_active(true);
    widget::<gtk::Entry>(&win, "gesture-scale").set_text("2");
    click(&win, "Update Gesture Draft");
    assert!(state.borrow().dirty());
    assert_eq!(current(&state).declarations[0].fingers, Value::Int(4));
    click(&win, "Duplicate");
    assert_eq!(current(&state).declarations.len(), 2);
    let generation = state.borrow().generation;
    widget::<gtk::Entry>(&win, "gesture-scale").set_text("-2");
    widget::<gtk::DropDown>(&win, "gesture-declaration").set_selected(0);
    assert_eq!(
        widget::<gtk::DropDown>(&win, "gesture-declaration").selected(),
        1
    );
    assert_eq!(state.borrow().generation, generation);
    win.close();
    assert!(win.is_visible(), "Invalid form must keep its window open");
    widget::<gtk::Entry>(&win, "gesture-scale").set_text("3");
    click(&win, "Update Gesture Draft");
    click(&win, "Undo");
    assert_eq!(
        current(&state).declarations[1].scale,
        Some(Value::Float(2.))
    );
    click(&win, "Remove");
    click(&win, "Create Unset");
    let registration = current(&state)
        .registrations(hyprland_editor_core::BASELINE)
        .unwrap();
    assert!(registration.iter().all(|r| r.result.is_ok()));
    click(&win, "↑");
    click(&win, "Check Gesture Relationships");
    assert!(!widget::<gtk::Label>(&win, "gesture-feedback")
        .text()
        .starts_with("No registration"));
    click(&win, "Undo");
    click(&win, "Remove");
    widget::<gtk::DropDown>(&win, "gesture-action-kind").set_selected(1);
    click(&win, "Add Action Block");
    click(&win, "Add Action Block");
    click(&win, "↑ Block");
    click(&win, "Remove Block");
    widget::<gtk::DropDown>(&win, "gesture-phase").set_selected(2);
    widget::<gtk::CheckButton>(&win, "gesture-phase-enabled").set_active(true);
    // An explicitly present empty finish function must survive GUI editing.
    click(&win, "Update Gesture Draft");
    let GestureAction::Callbacks(c) = &current(&state).declarations[0].action else {
        panic!()
    };
    assert_eq!(c.start.as_ref().unwrap().actions.len(), 1);
    assert!(c.finish.as_ref().unwrap().actions.is_empty());
    widget::<gtk::DropDown>(&win, "gesture-phase").set_selected(0);
    widget::<gtk::Stack>(&win, "gesture-details").set_visible_child_name("action");
    capture(&win, "gestures-callbacks");
    let export = state.borrow().export_lua().unwrap();
    let report = hyprland_editor_core::lua::parse(
        &export,
        hyprland_editor_core::BASELINE,
        &state.borrow().catalogue,
    )
    .unwrap();
    assert!(!report.opaque_effects);
    assert!(report
        .gestures
        .same_declarations(&current(&state), hyprland_editor_core::BASELINE));
    win.close();
    drop(win);
    // Imported aliases and coercible scalar spellings are kept until changed.
    let report = hyprland_editor_core::lua::parse("hl.gesture{fingers=3,direction='L',action='cursorZoom',mods='notWINd',scale='2',disable_inhibit='1',zoom_level=2.0}", hyprland_editor_core::BASELINE, &state.borrow().catalogue).unwrap();
    state
        .borrow_mut()
        .replace_gestures(report.gestures.clone())
        .unwrap();
    let win = show(parent, &state, &status);
    click(&win, "Update Gesture Draft");
    assert_eq!(current(&state), report.gestures);
    capture(&win, "gestures");
    win.close();
    drop(win);
    state.borrow_mut().draft.version = AUDITED_MAIN.into();
    let win = show(parent, &state, &status);
    click(&win, "Add Gesture");
    widget::<gtk::CheckButton>(&win, "gesture-scale-set").set_active(true);
    widget::<gtk::Entry>(&win, "gesture-scale").set_text("-0.1");
    click(&win, "Update Gesture Draft");
    assert_eq!(
        current(&state).declarations[0].scale,
        Some(Value::Float(-0.1))
    );
    let weak_state = Rc::downgrade(&state);
    win.close();
    drop(win);
    drop(state);
    let context = gtk::glib::MainContext::default();
    while context.pending() {
        context.iteration(false);
    }
    assert!(
        weak_state.upgrade().is_none(),
        "Closed gesture editor retained its draft state"
    );
}
