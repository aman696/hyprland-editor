//! Reviewed private native source revisions and graphical shared definitions.
use super::{legacy_source::Document, Shared, State};
use hyprland_editor_core::{binding, workspace::Workspace, BASELINE};
use std::sync::Arc;
pub(super) struct Snapshot {
    pub document: Arc<Document>,
    pub workspace: Workspace,
}
pub(super) fn restore(state: &mut State, snapshot: Snapshot) {
    let Snapshot {
        document,
        workspace: w,
    } = snapshot;
    let v = w.draft.version.clone();
    state.draft = w.draft;
    state.monitors.insert(v.clone(), w.monitors);
    state.devices.insert(v.clone(), w.devices);
    state.bindings.insert(v.clone(), w.bindings);
    state.workspace_rules.insert(v.clone(), w.workspace_rules);
    state.rules.insert(v.clone(), w.rules);
    state.animations.insert(v.clone(), w.animations);
    state.gestures.insert(v.clone(), w.gestures);
    state.session.insert(v.clone(), w.session);
    state.legacy_sources.insert(v, document);
}
fn rebuilt(previous: &Workspace, doc: &Document) -> Result<Workspace, String> {
    let monitors = if hyprland_editor_core::legacy_monitor::export(&previous.monitors)?
        == hyprland_editor_core::legacy_monitor::export(&doc.monitors.rules)?
    {
        previous.monitors.clone()
    } else {
        doc.monitors.rules.clone()
    };
    let devices =
        if hyprland_editor_core::device::same_declarations(&previous.devices, &doc.devices.devices)
        {
            previous.devices.clone()
        } else {
            doc.devices.devices.clone()
        };
    let workspaces = if hyprland_editor_core::workspace_rule::same_declarations(
        &previous.workspace_rules,
        &doc.workspaces.rules,
    ) {
        previous.workspace_rules.clone()
    } else {
        doc.workspaces.rules.clone()
    };
    let rules = if hyprland_editor_core::rule::same_declarations(&previous.rules, &doc.rules.rules)
    {
        previous.rules.clone()
    } else {
        doc.rules.rules.clone()
    };
    let animations = if hyprland_editor_core::legacy_animation::export(&previous.animations)?
        == hyprland_editor_core::legacy_animation::export(&doc.animations.draft)?
    {
        previous.animations.clone()
    } else {
        doc.animations.draft.clone()
    };
    let gestures = if hyprland_editor_core::legacy_gesture::export(&previous.gestures)?
        == hyprland_editor_core::legacy_gesture::export(&doc.gestures.draft)?
    {
        previous.gestures.clone()
    } else {
        doc.gestures.draft.clone()
    };
    let entries = |d: &hyprland_editor_core::session::Draft| {
        d.entries
            .iter()
            .map(hyprland_editor_core::legacy_session::encode)
            .collect::<Result<Vec<_>, String>>()
    };
    let session = if entries(&previous.session)? == entries(&doc.session.draft)? {
        previous.session.clone()
    } else {
        doc.session.draft.clone()
    };
    let mut w = previous.clone();
    w.draft = doc.report.draft.clone();
    w = w
        .with_monitors(monitors)
        .with_devices(devices)
        .with_bindings(binding::Draft {
            legacy: Some(doc.bindings.draft.clone()),
            ..Default::default()
        })
        .with_workspace_rules(workspaces)
        .with_rules(rules)
        .with_animations(animations)
        .with_gestures(gestures)
        .with_session(session);
    let root = w.root().ok_or("Captured native root missing")?;
    w.legacy_revision = if root.content == doc.content {
        None
    } else {
        Some(hyprland_editor_core::legacy_variables::Revision::new(
            &root.content,
            doc.content.clone(),
        )?)
    };
    let hash = doc.session.source_sha256.clone();
    for marker in [
        &mut w.legacy_binding_origin_sha256,
        &mut w.legacy_session_origin_sha256,
        &mut w.legacy_monitor_origin_sha256,
        &mut w.legacy_workspace_origin_sha256,
        &mut w.legacy_animation_origin_sha256,
        &mut w.legacy_gesture_origin_sha256,
        &mut w.legacy_device_origin_sha256,
        &mut w.legacy_rule_origin_sha256,
    ] {
        *marker = Some(hash.clone());
    }
    w.legacy_binding_schema_sha256 = Some(hyprland_editor_core::legacy_binding::draft_schema_hash(
        &doc.bindings.draft,
    ));
    Ok(w)
}
pub(super) fn keep(state: &mut State, doc: Arc<Document>) -> Result<(), String> {
    if state.draft.version != BASELINE || state.source_graphs.contains_key(BASELINE) {
        return Err("Native source revision belongs to a single legacy source project".into());
    }
    let old = state
        .legacy_sources
        .get(BASELINE)
        .cloned()
        .ok_or("Native source missing")?;
    if old.content == doc.content {
        return Ok(());
    }
    let previous = super::workflow::captured_workspace(state)?;
    let next = rebuilt(&previous, &doc)?;
    let base = &previous.root().ok_or("Captured source missing")?.content;
    if doc
        .original
        .as_ref()
        .map_or(doc.content.as_str(), |d| d.content.as_str())
        != base
    {
        return Err("Source proposal belongs to a different source capture".into());
    }
    let next_bytes = next.to_json(&state.catalogue)?;
    drop(next_bytes);
    // Conservatively charge both private models and source projections against
    // the shared undo budget before changing any draft or source identity.
    let cost = previous
        .to_json(&state.catalogue)?
        .len()
        .saturating_mul(4)
        .saturating_add(document_cost(&old)?);
    state.undo.legacy_revision(
        Snapshot {
            document: old,
            workspace: previous,
        },
        cost,
    )?;
    restore(
        state,
        Snapshot {
            document: doc,
            workspace: next,
        },
    );
    state.generation += 1;
    Ok(())
}
fn document_cost(doc: &Document) -> Result<usize, String> {
    let model = serde_json::to_vec(&(
        &doc.report,
        &doc.session,
        &doc.monitors,
        &doc.bindings,
        &doc.workspaces,
        &doc.animations,
        &doc.gestures,
        &doc.devices,
        &doc.rules,
        &doc.variables,
    ))
    .map_err(|e| e.to_string())?
    .len()
    .saturating_mul(4);
    Ok(doc
        .content
        .capacity()
        .saturating_add(model)
        .saturating_add(doc.original.as_ref().map_or(Ok(0), |d| document_cost(d))?))
}
// The controller below owns no folder access or live configuration authority.
use adw::prelude::*;
use hyprland_editor_core::legacy_variables::{self as codec, Definition};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Get = Rc<dyn Fn() -> Result<(), String>>;
fn text_view(text: &str) -> gtk::ScrolledWindow {
    let t = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    t.buffer().set_text(text);
    gtk::ScrolledWindow::builder()
        .child(&t)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build()
}
fn value_form(panel: &gtk::Box, raw: &str) -> Box<dyn Fn() -> Result<String, String>> {
    let color = if raw.starts_with("0x") || raw.starts_with("0X") {
        hyprland_editor_core::parse_color(raw).ok()
    } else if let Some(v) = raw
        .strip_prefix("rgba(")
        .and_then(|s| s.strip_suffix(')'))
        .filter(|s| s.len() == 8)
    {
        u32::from_str_radix(v, 16).ok().map(|n| n.rotate_right(8))
    } else {
        raw.strip_prefix("rgb(")
            .and_then(|s| s.strip_suffix(')'))
            .filter(|s| s.len() == 6)
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .map(|n| 0xff000000 | n)
    };
    const MODS: [&str; 8] = [
        "SUPER", "SHIFT", "CTRL", "ALT", "CAPS", "MOD2", "MOD3", "MOD5",
    ];
    let tokens = raw.split_whitespace().collect::<Vec<_>>();
    let modifiers = !tokens.is_empty() && tokens.iter().all(|t| MODS.contains(t));
    let initial = if color.is_some() {
        1
    } else if modifiers {
        3
    } else if ["true", "false", "yes", "no", "on", "off"].contains(&raw) {
        4
    } else if raw.len() <= 128 && raw.parse::<f64>().is_ok_and(f64::is_finite) {
        2
    } else {
        0
    };
    let mode = super::monitors::choices(
        panel,
        &[
            "Text or reference",
            "Color",
            "Number",
            "Modifier keys",
            "Switch",
        ],
        initial,
    );
    mode.set_widget_name("variable-value-helper");
    let stack = gtk::Stack::new();
    stack.set_vhomogeneous(false);
    stack.set_hhomogeneous(false);
    panel.append(&stack);
    let literal = gtk::TextView::builder()
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .height_request(96)
        .build();
    literal.buffer().set_text(raw);
    literal.set_widget_name("variable-template");
    literal.update_property(&[gtk::accessible::Property::Label("Variable source template")]);
    let text_body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    text_body.append(
        &gtk::ScrolledWindow::builder()
            .child(&literal)
            .min_content_height(96)
            .max_content_height(180)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    let limit = super::label("");
    text_body.append(&limit);
    let insertion_refused = Rc::new(Cell::new(false));
    let action_depth = Rc::new(Cell::new(0u32));
    let before_action = Rc::new(RefCell::new(None::<String>));
    {
        let depth = action_depth.clone();
        let before = before_action.clone();
        let refused = insertion_refused.clone();
        literal.buffer().connect_begin_user_action(move |b| {
            if depth.get() == 0 {
                *before.borrow_mut() = Some(b.text(&b.start_iter(), &b.end_iter(), true).into());
                refused.set(false);
            }
            depth.set(depth.get().saturating_add(1));
        });
    }
    {
        let refused = insertion_refused.clone();
        let feedback = limit.clone();
        literal.buffer().connect_insert_text(move |b, _, text| {
            let length = b.text(&b.start_iter(), &b.end_iter(), true).len();
            if length.saturating_add(text.len()) > 65536 {
                b.stop_signal_emission_by_name("insert-text");
                refused.set(true);
                feedback.set_text("Template limit is 64 KiB; this insertion was refused in full.");
            } else if !refused.get() {
                feedback.set_text("");
            }
        });
    }
    {
        let depth = action_depth;
        let before = before_action;
        let refused = insertion_refused;
        literal.buffer().connect_end_user_action(move |b| {
            depth.set(depth.get().saturating_sub(1));
            if depth.get() == 0 {
                let previous = before.borrow_mut().take();
                if refused.replace(false) {
                    if let Some(previous) = previous {
                        b.delete(&mut b.start_iter(), &mut b.end_iter());
                        b.insert(&mut b.start_iter(), &previous);
                        limit.set_text("Template limit is 64 KiB; the previous text was restored.");
                    }
                }
            }
        });
    }
    stack.add_named(&text_body, Some("0"));
    let color_button = super::controls::color_button(color.unwrap_or(0xffeeeeee));
    color_button.set_widget_name("variable-color");
    color_button.set_halign(gtk::Align::Start);
    color_button.set_valign(gtk::Align::Start);
    color_button.set_size_request(96, 40);
    color_button.update_property(&[gtk::accessible::Property::Label("Variable color")]);
    stack.add_named(&color_button, Some("1"));
    let numbers = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let number = gtk::Entry::builder()
        .text(if initial == 2 { raw } else { "0" })
        .max_length(128)
        .hexpand(true)
        .build();
    number.set_widget_name("variable-number");
    number.update_property(&[gtk::accessible::Property::Label("Variable number")]);
    numbers.append(&number);
    let steps = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    for (label, step) in [("−1", -1.), ("+1", 1.)] {
        let b = gtk::Button::with_label(label);
        steps.append(&b);
        let e = number.downgrade();
        b.connect_clicked(move |_| {
            if let Some(e) = e.upgrade() {
                if let Ok(n) = e.text().parse::<f64>() {
                    let n = n + step;
                    if n.is_finite() {
                        e.set_text(&n.to_string());
                    }
                }
            }
        });
    }
    numbers.append(&steps);
    stack.add_named(&numbers, Some("2"));
    let mod_box = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(2)
        .min_children_per_line(1)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    let mut toggles = vec![];
    for m in MODS {
        let b = gtk::CheckButton::with_label(m);
        b.set_active(tokens.contains(&m));
        mod_box.insert(&b, -1);
        toggles.push((m, b));
    }
    stack.add_named(&mod_box, Some("3"));
    let switch = gtk::Switch::builder()
        .active(["1", "true", "yes", "on"].contains(&raw))
        .halign(gtk::Align::Start)
        .build();
    switch.set_widget_name("variable-switch");
    stack.add_named(&switch, Some("4"));
    stack.set_visible_child_name(&initial.to_string());
    let weak = stack.downgrade();
    mode.connect_selected_notify(move |m| {
        if let Some(s) = weak.upgrade() {
            s.set_visible_child_name(&m.selected().to_string());
        }
    });
    let original = raw.to_owned();
    let baseline_color = super::controls::color_value(&color_button);
    let baseline_number = number.text();
    let baseline_mods = toggles
        .iter()
        .filter(|(_, b)| b.is_active())
        .map(|(m, _)| *m)
        .collect::<Vec<_>>()
        .join(" ");
    let baseline_switch = switch.is_active();
    Box::new(move || {
        let unchanged = mode.selected() == initial
            && match initial {
                0 => {
                    literal
                        .buffer()
                        .text(
                            &literal.buffer().start_iter(),
                            &literal.buffer().end_iter(),
                            true,
                        )
                        .as_str()
                        == original
                }
                1 => super::controls::color_value(&color_button) == baseline_color,
                2 => number.text() == baseline_number,
                3 => {
                    toggles
                        .iter()
                        .filter(|(_, b)| b.is_active())
                        .map(|(m, _)| *m)
                        .collect::<Vec<_>>()
                        .join(" ")
                        == baseline_mods
                }
                _ => switch.is_active() == baseline_switch,
            };
        if unchanged {
            return Ok(original.clone());
        }
        match mode.selected() {
            0 => Ok(literal
                .buffer()
                .text(
                    &literal.buffer().start_iter(),
                    &literal.buffer().end_iter(),
                    true,
                )
                .into()),
            1 => Ok(format!(
                "0x{:08x}",
                super::controls::color_value(&color_button)
            )),
            2 => {
                if !number.text().parse::<f64>().is_ok_and(f64::is_finite) {
                    return Err("Enter a finite number, or use Text or reference to retain a different template".into());
                }
                Ok(number.text().into())
            }
            3 => Ok(toggles
                .iter()
                .filter(|(_, b)| b.is_active())
                .map(|(m, _)| *m)
                .collect::<Vec<_>>()
                .join(" ")),
            _ => Ok(if switch.is_active() { "true" } else { "false" }.into()),
        }
    })
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
        if let Some(p) = weak.upgrade() {
            show(&p, &state, &status);
        }
    });
}
pub(super) fn show(
    parent: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) -> Option<adw::Window> {
    let Some(source) = state
        .borrow()
        .legacy_sources
        .get(&state.borrow().draft.version)
        .cloned()
    else {
        super::notice(parent,"Native configuration required","Import a .conf project to edit its source-defined variables. Lua local-definition editing remains separate work.");
        return None;
    };
    let win = adw::Window::builder()
        .title("Shared Variables")
        .transient_for(parent)
        .modal(true)
        .default_width(560)
        .default_height(720)
        .build();
    win.set_size_request(350, 480);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for f in [
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
    ] {
        f(content.upcast_ref(), 16);
    }
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    win.set_content(Some(&root));
    let introduction=super::label("Edit shared values, then review their source changes. Your original configuration stays intact.");
    introduction.set_selectable(false);
    content.append(&introduction);
    let initial = source.variables.definitions.clone();
    let draft = Rc::new(RefCell::new(initial.clone()));
    let model = gtk::StringList::new(&[]);
    let pick = gtk::DropDown::builder().model(&model).hexpand(true).build();
    super::monitors::compact(&pick);
    pick.set_widget_name("variable-declaration");
    content.append(&pick);
    let manage = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let expander = gtk::Expander::builder()
        .label("Add, reorder or remove definitions")
        .child(&manage)
        .build();
    expander.set_widget_name("variable-management");
    content.append(&expander);
    let actions = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(3)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    manage.append(&actions);
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.append(&panel);
    let feedback = super::label("");
    feedback.set_widget_name("variable-feedback");
    content.append(&feedback);
    let review = gtk::Button::with_label("Review Variable Changes");
    review.add_css_class("suggested-action");
    content.append(&review);
    content.append(&super::label("Only captured source definitions are edited. Includes, environment values and expressions are not evaluated. Renaming a definition leaves its references unchanged."));
    let active = Rc::new(Cell::new(gtk::INVALID_LIST_POSITION));
    let busy = Rc::new(Cell::new(false));
    let getter = Rc::new(RefCell::new(
        None::<Box<dyn Fn() -> Result<Definition, String>>>,
    ));
    let flush: Get = Rc::new({
        let active = active.clone();
        let draft = draft.clone();
        let getter = getter.clone();
        move || {
            if let Some(get) = getter.borrow().as_ref() {
                let d = get()?;
                let mut all = draft.borrow().clone();
                let slot = all
                    .get_mut(active.get() as usize)
                    .ok_or("Variable definition was removed")?;
                *slot = d;
                codec::validate(&all)?;
                *draft.borrow_mut() = all;
            }
            Ok(())
        }
    });
    let render: Rc<dyn Fn(usize)> = Rc::new({
        let draft = draft.clone();
        let active = active.clone();
        let getter = getter.clone();
        let panel = panel.clone();
        let model = model.clone();
        let pick = pick.clone();
        let busy = busy.clone();
        let source = source.clone();
        move |index| {
            busy.set(true);
            while let Some(c) = panel.first_child() {
                panel.remove(&c);
            }
            let defs = draft.borrow();
            let names = defs
                .iter()
                .enumerate()
                .map(|(i, d)| format!("{}. ${}", i + 1, d.name))
                .collect::<Vec<_>>();
            model.splice(
                0,
                model.n_items(),
                &names.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            let index = index.min(defs.len().saturating_sub(1));
            active.set(if defs.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            pick.set_selected(active.get());
            *getter.borrow_mut() = None;
            if let Some(d) = defs.get(index) {
                panel.append(&super::label("Variable name (without $)"));
                let name = gtk::Entry::builder()
                    .text(&d.name)
                    .max_length(256)
                    .hexpand(true)
                    .build();
                name.set_widget_name("variable-name");
                name.update_property(&[gtk::accessible::Property::Label(
                    "Variable name without dollar sign",
                )]);
                panel.append(&name);
                if defs.iter().filter(|other| other.name == d.name).count() > 1 {
                    panel.append(&super::label("This name is assigned more than once. Order matters; removing a definition can expose an earlier value."));
                }
                let references = codec::references(&source.content, &d.name).map_or(0, |v| v.len());
                panel.append(&super::label(&format!(
                    "References in this file: {references}."
                )));
                let value = value_form(&panel, &d.value);
                let id = d.id.clone();
                *getter.borrow_mut() = Some(Box::new(move || {
                    let d = Definition {
                        id: id.clone(),
                        name: name.text().into(),
                        value: value()?,
                    };
                    codec::validate(std::slice::from_ref(&d))?;
                    Ok(d)
                }));
            } else {
                panel.append(&super::label("Add a definition to begin."));
            }
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&render);
    let b = busy.clone();
    let f = flush.clone();
    let a = active.clone();
    let fb = feedback.clone();
    pick.connect_selected_notify(move |p| {
        if b.get() {
            return;
        }
        let next = p.selected();
        if let Err(e) = f() {
            b.set(true);
            p.set_selected(a.get());
            b.set(false);
            fb.set_text(&e);
            return;
        }
        if let Some(r) = weak.upgrade() {
            r(next as usize);
        }
    });
    for (label, op) in [
        ("Add", 0),
        ("Duplicate", 1),
        ("Remove", 2),
        ("↑", 3),
        ("↓", 4),
    ] {
        let button = gtk::Button::with_label(label);
        actions.insert(&button, -1);
        let f = flush.clone();
        let draft = draft.clone();
        let active = active.clone();
        let weak = Rc::downgrade(&render);
        let fb = feedback.clone();
        button.connect_clicked(move |_| {
            if op != 2 {
                if let Err(e) = f() {
                    fb.set_text(&e);
                    return;
                }
            }
            let mut all = draft.borrow().clone();
            let i = active.get() as usize;
            let next = match op {
                0 => {
                    let name = (0..)
                        .map(|n| {
                            if n == 0 {
                                "new_value".into()
                            } else {
                                format!("new_value{n}")
                            }
                        })
                        .find(|n| all.iter().all(|d| d.name != *n))
                        .unwrap();
                    let id = unique(&all);
                    all.push(Definition {
                        id,
                        name,
                        value: String::new(),
                    });
                    all.len() - 1
                }
                1 if i < all.len() => {
                    let mut d = all[i].clone();
                    d.id = unique(&all);
                    let base = d.name.clone();
                    d.name = (1..)
                        .map(|n| format!("{base}_copy{n}"))
                        .find(|n| all.iter().all(|d| d.name != *n))
                        .unwrap();
                    all.push(d);
                    all.len() - 1
                }
                2 if i < all.len() => {
                    all.remove(i);
                    i.min(all.len().saturating_sub(1))
                }
                3 if i > 0 && i < all.len() => {
                    all.swap(i, i - 1);
                    i - 1
                }
                4 if i + 1 < all.len() => {
                    all.swap(i, i + 1);
                    i + 1
                }
                _ => return,
            };
            if let Err(e) = codec::validate(&all) {
                fb.set_text(&e);
                return;
            }
            *draft.borrow_mut() = all;
            if let Some(r) = weak.upgrade() {
                r(next);
            }
        });
    }
    let weak = win.downgrade();
    let state_c = state.clone();
    let status_c = status.clone();
    let feedback_c = feedback.clone();
    let draft_c = draft.clone();
    let source_c = source.clone();
    let f = flush.clone();
    review.connect_clicked(move |button| {
        if let Err(e) = f() {
            feedback_c.set_text(&e);
            return;
        }
        let Some(parent) = weak.upgrade() else {
            return;
        };
        let snapshot = match super::workflow::captured_workspace(&state_c.borrow()) {
            Ok(w) => w,
            Err(e) => {
                feedback_c.set_text(&e);
                return;
            }
        };
        if snapshot.legacy_content() != Some(source_c.content.as_str()) {
            feedback_c
                .set_text("The source changed. Reopen Shared Variables before reviewing again.");
            return;
        }
        let epoch = state_c.borrow().generation;
        let catalogue = state_c.borrow().catalogue.clone();
        let definitions = draft_c.borrow().clone();
        let source = source_c.clone();
        let mode = source.bindings.draft.local_variables;
        button.set_sensitive(false);
        let weak_button = button.downgrade();
        let weak_parent = parent.downgrade();
        let state = state_c.clone();
        let status = status_c.clone();
        let fb = feedback_c.clone();
        super::workflow::background(
            move || {
                let output =
                    match super::worker::run(super::worker::Request::EditLegacyVariables {
                        workspace: Box::new(snapshot),
                        definitions,
                    })? {
                        super::worker::Response::Copy(s) => s,
                        _ => return Err("Unexpected variable proposal response".into()),
                    };
                let mut doc = Document::new_with_binding_mode(output, &catalogue, mode)?;
                let original = source.original.clone().unwrap_or_else(|| source.clone());
                if doc.content != original.content {
                    doc.original = Some(original);
                }
                Ok(Arc::new(doc))
            },
            move |result| {
                if let Some(b) = weak_button.upgrade() {
                    b.set_sensitive(true);
                }
                let Some(parent) = weak_parent.upgrade() else {
                    return;
                };
                if !parent.is_visible() {
                    return;
                }
                if state.borrow().generation != epoch {
                    fb.set_text("The draft changed. Review again before keeping variable changes.");
                    return;
                }
                match result {
                    Ok(doc) => review_proposal(&parent, &state, &status, doc, epoch, None),
                    Err(e) => fb.set_text(&e),
                }
            },
        );
    });
    let retain_render = render.clone();
    let f = flush.clone();
    let draft_c = draft.clone();
    let weak = win.downgrade();
    win.connect_close_request(move |_| {
        let _ = &retain_render;
        let valid = f().is_ok();
        if valid && *draft_c.borrow() == initial {
            return gtk::glib::Propagation::Proceed;
        }
        let Some(w) = weak.upgrade() else {
            return gtk::glib::Propagation::Proceed;
        };
        let dialog = adw::MessageDialog::builder()
            .transient_for(&w)
            .heading("Discard pending variable edits?")
            .body("These form edits have not been kept in your private project.")
            .build();
        dialog.add_responses(&[
            ("keep", "Keep Editing"),
            ("discard", "Discard Form Changes"),
        ]);
        dialog.set_default_response(Some("keep"));
        let weak = w.downgrade();
        dialog.connect_response(None, move |_, r| {
            if r == "discard" {
                if let Some(w) = weak.upgrade() {
                    w.destroy();
                }
            }
        });
        dialog.present();
        gtk::glib::Propagation::Stop
    });
    render(0);
    // The signal closures retain the real renderer; trigger its first selection.
    if !draft.borrow().is_empty() {
        pick.set_selected(0);
    }
    win.present();
    pick.grab_focus();
    Some(win)
}
fn unique(defs: &[Definition]) -> String {
    (0..)
        .map(|n| format!("new-variable-{n}"))
        .find(|id| defs.iter().all(|d| d.id != *id))
        .unwrap()
}
pub(super) fn review_proposal(
    parent: &adw::Window,
    state: &Shared,
    status: &gtk::Label,
    doc: Arc<Document>,
    epoch: u64,
    include_impact: Option<String>,
) {
    let Some(before) = state.borrow().legacy_sources.get(BASELINE).cloned() else {
        return;
    };
    let review = adw::Window::builder()
        .title(if include_impact.is_some() {
            "Review Include Changes"
        } else {
            "Review Variable Changes"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(680)
        .default_height(740)
        .build();
    review.set_widget_name("variable-review");
    review.set_size_request(350, 480);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for f in [
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
    ] {
        f(body.upcast_ref(), 16);
    }
    root.append(&body);
    review.set_content(Some(&root));
    body.append(&super::label("Review the resulting source before keeping it. Other staged native edits are included; your original capture stays unchanged. Save the project afterwards to retain this revision."));
    let mut impact = include_impact.unwrap_or_else(|| variable_impact(&before, &doc));
    impact.push_str("\nResulting supported shortcuts (literal source projection; not executed):\n");
    for entry in &doc.bindings.draft.entries {
        if impact.len() > 512 * 1024 {
            impact.push_str(
                "Further shortcut details omitted here; inspect the full proposed source.\n",
            );
            break;
        }
        if let Ok(s) = hyprland_editor_core::legacy_binding::encode(entry) {
            impact.push_str(&s);
            impact.push('\n');
        }
    }
    let tabs = gtk::Notebook::new();
    tabs.set_vexpand(true);
    for (name, text) in [
        ("Impact", impact.as_str()),
        ("Before", before.content.as_str()),
        ("Proposed", doc.content.as_str()),
    ] {
        tabs.append_page(&text_view(text), Some(&gtk::Label::new(Some(name))));
    }
    body.append(&tabs);
    let feedback = super::label("");
    feedback.set_widget_name("variable-review-feedback");
    body.append(&feedback);
    let row = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(2)
        .column_spacing(8)
        .row_spacing(8)
        .build();
    let cancel = gtk::Button::with_label("Keep Editing");
    let accept = gtk::Button::with_label("Keep Private Changes");
    accept.add_css_class("suggested-action");
    row.insert(&cancel, -1);
    row.insert(&accept, -1);
    body.append(&row);
    let weak = review.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(r) = weak.upgrade() {
            r.destroy();
        }
    });
    let weak = review.downgrade();
    let parent = parent.downgrade();
    let state = state.clone();
    let status = status.clone();
    accept.connect_clicked(move |button| {
        if state.borrow().generation != epoch {
            feedback.set_text("The draft changed. Close this proposal and review again.");
            button.set_sensitive(false);
            return;
        }
        let result = keep(&mut state.borrow_mut(), doc.clone());
        match result {
            Ok(()) => {
                super::summary(&state, &status);
                if let Some(r) = weak.upgrade() {
                    r.destroy();
                }
                if let Some(p) = parent.upgrade() {
                    p.destroy();
                }
            }
            Err(e) => feedback.set_text(&e),
        }
    });
    review.present();
    cancel.grab_focus();
}

fn variable_impact(before: &Document, doc: &Document) -> String {
    let mut impact=format!("Definitions: {} → {}\nSupported shortcut operations: {} → {}\nUnresolved scalar/source lines: {} → {}\n\nThese are captured-source projections, not effective compositor observations. Includes, ambient variables and expressions remain unresolved. Renaming changes the definition only; inspect its remaining references.\n\n",before.variables.definitions.len(),doc.variables.definitions.len(),before.bindings.draft.entries.len(),doc.bindings.draft.entries.len(),before.report.skipped.len(),doc.report.skipped.len());
    let short = |s: &str| {
        let mut out = s.chars().take(160).collect::<String>();
        if s.chars().count() > 160 {
            out.push('…');
        }
        out
    };
    for i in 0..before
        .variables
        .definitions
        .len()
        .max(doc.variables.definitions.len())
    {
        let a = before.variables.definitions.get(i);
        let b = doc.variables.definitions.get(i);
        if a.map(|d| (&d.name, &d.value)) != b.map(|d| (&d.name, &d.value)) {
            impact.push_str(&format!(
                "{} → {}\n",
                a.map_or("(added)".into(), |d| format!(
                    "${} = {}",
                    d.name,
                    short(&d.value)
                )),
                b.map_or("(removed)".into(), |d| format!(
                    "${} = {}",
                    d.name,
                    short(&d.value)
                ))
            ));
        }
    }
    impact
}

#[cfg(test)]
pub(super) fn test_template_boundaries() {
    fn descendants(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut v = vec![w.clone()];
        let mut c = w.first_child();
        while let Some(w) = c {
            v.extend(descendants(&w));
            c = w.next_sibling();
        }
        v
    }
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let original = "0".repeat(65536);
    let get = value_form(&panel, &original);
    assert_eq!(get().unwrap(), original);
    let mode = descendants(panel.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "variable-value-helper")
        .unwrap()
        .downcast::<gtk::DropDown>()
        .unwrap();
    assert_eq!(
        mode.selected(),
        0,
        "Long numeric templates must remain fully visible in the text helper"
    );
    let text = descendants(panel.upcast_ref())
        .into_iter()
        .find(|w| w.widget_name() == "variable-template")
        .unwrap()
        .downcast::<gtk::TextView>()
        .unwrap();
    let buffer = text.buffer();
    buffer.insert(&mut buffer.end_iter(), "extra");
    assert_eq!(
        get().unwrap(),
        original,
        "Oversized insertion must be refused in full"
    );
    let unicode = "🌕".repeat(16384);
    buffer.set_text(&unicode);
    assert_eq!(get().unwrap(), unicode);
    buffer.insert(&mut buffer.end_iter(), "x");
    assert_eq!(get().unwrap(), unicode);
    buffer.begin_user_action();
    buffer.delete(&mut buffer.start_iter(), &mut buffer.end_iter());
    buffer.insert(&mut buffer.start_iter(), &"x".repeat(65537));
    buffer.end_user_action();
    assert_eq!(
        get().unwrap(),
        unicode,
        "Oversized replacement must restore the selected original text"
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_native_revision_inverse_refuses_before_source_or_draft_changes() {
        use hyprland_editor_core::source::{RecordedOwnership, SourceIdentity};
        let base = format!("$name = original\n#{}\n", "x".repeat(900 * 1024));
        let catalogue = hyprland_editor_core::catalogue();
        let original = Arc::new(Document::new(base.clone(), &catalogue).unwrap());
        let mut current =
            Document::new(base.replacen("original", "current", 1), &catalogue).unwrap();
        current.original = Some(original.clone());
        let current = Arc::new(current);
        let mut incoming =
            Document::new(base.replacen("original", "incoming", 1), &catalogue).unwrap();
        incoming.original = Some(original);
        let incoming = Arc::new(incoming);
        let state = super::super::initial_state();
        {
            let mut s = state.borrow_mut();
            s.legacy_sources.insert(BASELINE.into(), current.clone());
            s.source_origins.insert(
                BASELINE.into(),
                SourceIdentity {
                    selected_path: b"/never-open/large.conf".to_vec(),
                    resolved_path: None,
                    links: vec![],
                    device: 1,
                    inode: 2,
                    owner: 1000,
                    mode: 0o100600,
                    size: base.len() as u64,
                    modified_seconds: 0,
                    modified_nanoseconds: 0,
                    changed_seconds: 0,
                    changed_nanoseconds: 0,
                    sha256: hyprland_editor_core::lua::source_hash(&base),
                    recorded_ownership: RecordedOwnership::UserOwned,
                },
            );
        }
        let before = super::super::workflow::captured_workspace(&state.borrow())
            .unwrap()
            .to_json(&catalogue)
            .unwrap();
        let generation = state.borrow().generation;
        let error = keep(&mut state.borrow_mut(), incoming).unwrap_err();
        assert!(error.contains("undo memory budget"), "{error}");
        assert_eq!(state.borrow().generation, generation);
        assert!(Arc::ptr_eq(
            &state.borrow().legacy_sources[BASELINE],
            &current
        ));
        assert_eq!(
            super::super::workflow::captured_workspace(&state.borrow())
                .unwrap()
                .to_json(&catalogue)
                .unwrap(),
            before
        );
        assert!(state.borrow_mut().undo.pop().is_none());
    }
}
