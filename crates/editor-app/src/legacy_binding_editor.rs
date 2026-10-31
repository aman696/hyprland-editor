//! Ordered native legacy shortcut editing. All operations change private drafts only.
use super::{label, monitors::compact, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    binding,
    legacy_binding::{self, Chord, Draft, Entry},
};
use std::{cell::RefCell, rc::Rc};
type GetEntry = Box<dyn Fn() -> Result<Entry, String>>;
fn field(panel: &gtk::Box, title: &str, value: &str, name: &str) -> gtk::Entry {
    panel.append(&label(title));
    let input = gtk::Entry::builder().text(value).max_length(16384).build();
    input.set_widget_name(name);
    input.update_property(&[gtk::accessible::Property::Label(title)]);
    panel.append(&input);
    input
}
fn select(panel: &gtk::Box, title: &str, names: &[&str], index: u32, name: &str) -> gtk::DropDown {
    panel.append(&label(title));
    let input = gtk::DropDown::from_strings(names);
    compact(&input);
    input.set_enable_search(true);
    input.set_selected(index);
    input.set_widget_name(name);
    input.update_property(&[gtk::accessible::Property::Label(title)]);
    panel.append(&input);
    input
}
fn section(panel: &gtk::Box, title: &str) -> gtk::Box {
    let fields = gtk::Box::new(gtk::Orientation::Vertical, 6);
    panel.append(&gtk::Expander::builder().label(title).child(&fields).build());
    fields
}
fn tokens(
    panel: &gtk::Box,
    title: &'static str,
    values: &[String],
    limit: usize,
) -> Box<dyn Fn() -> Vec<String>> {
    panel.append(&label(title));
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
    panel.append(&rows);
    let inputs = Rc::new(RefCell::new(Vec::<gtk::Entry>::new()));
    let add: Rc<dyn Fn(&str)> = Rc::new({
        let rows = rows.downgrade();
        let inputs = inputs.clone();
        move |value| {
            let Some(rows) = rows.upgrade() else {
                return;
            };
            if inputs.borrow().len() >= limit {
                return;
            }
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let input = gtk::Entry::builder()
                .text(value)
                .hexpand(true)
                .max_length(4096)
                .build();
            input.update_property(&[gtk::accessible::Property::Label(title)]);
            let remove = gtk::Button::from_icon_name("list-remove-symbolic");
            remove.set_tooltip_text(Some("Remove item"));
            row.append(&input);
            row.append(&remove);
            rows.append(&row);
            inputs.borrow_mut().push(input.clone());
            let inputs = Rc::downgrade(&inputs);
            let rows = rows.downgrade();
            let row = row.downgrade();
            let input = input.downgrade();
            remove.connect_clicked(move |_| {
                if let (Some(inputs), Some(rows), Some(row), Some(input)) = (
                    inputs.upgrade(),
                    rows.upgrade(),
                    row.upgrade(),
                    input.upgrade(),
                ) {
                    inputs.borrow_mut().retain(|old| *old != input);
                    rows.remove(&row);
                }
            });
        }
    });
    for value in values {
        add(value);
    }
    let button = gtk::Button::with_label("Add Item");
    panel.append(&button);
    button.connect_clicked(move |_| add(""));
    Box::new(move || {
        inputs
            .borrow()
            .iter()
            .map(|input| input.text().to_string())
            .collect()
    })
}
fn chord(
    panel: &gtk::Box,
    initial: &Chord,
    multi_allowed: bool,
) -> Box<dyn Fn() -> Result<Chord, String>> {
    let mode = select(
        panel,
        "Shortcut matching",
        if multi_allowed {
            &["Modifier masks and one key", "Multiple key symbols"]
        } else {
            &["Modifier masks and one key"]
        },
        u32::from(initial.multi),
        "legacy-bind-chord-mode",
    );
    let normal = gtk::Box::new(gtk::Orientation::Vertical, 6);
    panel.append(&normal);
    let base = binding::Chord {
        tokens: if initial.multi {
            vec!["Return".into()]
        } else {
            initial
                .modifiers
                .iter()
                .chain(&initial.keys)
                .cloned()
                .collect()
        },
    };
    let get_normal = super::bind_forms::legacy_chord(&normal, &base);
    let multi = gtk::Box::new(gtk::Orientation::Vertical, 6);
    panel.append(&multi);
    multi.append(&label("One XKB symbol per item, such as Control_L or Return. Up to eight modifiers and eight keys; no separator syntax is needed."));
    let get_mods = tokens(
        &multi,
        "Modifier symbols",
        if initial.multi {
            &initial.modifiers
        } else {
            &[]
        },
        8,
    );
    let get_keys = tokens(
        &multi,
        "Key symbols",
        if initial.multi { &initial.keys } else { &[] },
        8,
    );
    normal.set_visible(!initial.multi);
    multi.set_visible(initial.multi);
    let n = normal.downgrade();
    let m = multi.downgrade();
    mode.connect_selected_notify(move |choice| {
        if let Some(n) = n.upgrade() {
            n.set_visible(choice.selected() == 0);
        }
        if let Some(m) = m.upgrade() {
            m.set_visible(choice.selected() == 1);
        }
    });
    Box::new(move || {
        let result = if mode.selected() == 1 {
            Chord {
                modifiers: get_mods(),
                keys: get_keys(),
                multi: true,
            }
        } else {
            let native = get_normal()?;
            let (modifiers, keys) = native
                .tokens
                .into_iter()
                .partition(|v| binding::MODIFIERS.contains(&v.as_str()));
            Chord {
                modifiers,
                keys,
                multi: false,
            }
        };
        result.validate()?;
        Ok(result)
    })
}
pub(super) fn action_name(name: &str) -> &str {
    match name {
        "exec" => "Run command",
        "execr" => "Run raw command",
        "killactive" => "Close active window",
        "forcekillactive" => "Force close active window",
        "closewindow" => "Close matching window",
        "killwindow" => "Kill matching window",
        "signal" => "Signal active window",
        "signalwindow" => "Signal matching window",
        "togglefloating" => "Toggle floating",
        "setfloating" => "Make floating",
        "settiled" => "Make tiled",
        "workspace" => "Switch workspace",
        "renameworkspace" => "Rename workspace",
        "fullscreen" => "Toggle fullscreen",
        "fullscreenstate" => "Set fullscreen state",
        "movetoworkspace" => "Move to workspace",
        "movetoworkspacesilent" => "Move to workspace silently",
        "pseudo" => "Toggle pseudotiling",
        "movefocus" => "Focus by direction",
        "movewindow" => "Move window by direction",
        "swapwindow" => "Swap windows",
        "centerwindow" => "Center window",
        "togglegroup" => "Toggle window group",
        "changegroupactive" => "Change active group window",
        "movegroupwindow" => "Reorder group window",
        "focusmonitor" => "Focus monitor",
        "movecursortocorner" => "Move cursor to corner",
        "movecursor" => "Move cursor",
        "workspaceopt" => "Change workspace option",
        "exit" => "Exit session",
        "movecurrentworkspacetomonitor" => "Move current workspace to monitor",
        "focusworkspaceoncurrentmonitor" => "Focus workspace on current monitor",
        "moveworkspacetomonitor" => "Move workspace to monitor",
        "togglespecialworkspace" => "Toggle special workspace",
        "forcerendererreload" => "Reload renderer",
        "resizeactive" => "Resize active window",
        "moveactive" => "Move active window",
        "cyclenext" => "Cycle window focus",
        "focuswindowbyclass" => "Focus window by class",
        "focuswindow" => "Focus matching window",
        "tagwindow" => "Change window tags",
        "toggleswallow" => "Toggle swallowing",
        "submap" => "Switch shortcut submap",
        "pass" => "Pass key to window",
        "sendshortcut" => "Send shortcut",
        "sendkeystate" => "Send key state",
        "layoutmsg" => "Send layout message",
        "dpms" => "Change display power",
        "movewindowpixel" => "Move matching window",
        "resizewindowpixel" => "Resize matching window",
        "swapnext" => "Swap with next window",
        "swapactiveworkspaces" => "Swap active workspaces",
        "pin" => "Toggle pinned window",
        "mouse" => "Mouse window operation",
        "bringactivetotop" => "Raise active window",
        "alterzorder" => "Change window stacking",
        "focusurgentorlast" => "Focus urgent or last window",
        "focuscurrentorlast" => "Focus current or last window",
        "lockgroups" => "Lock window groups",
        "lockactivegroup" => "Lock active group",
        "moveintogroup" => "Move into a group",
        "moveoutofgroup" => "Move out of a group",
        "movewindoworgroup" => "Move window or group",
        "moveintoorcreategroup" => "Move into or create group",
        "setignoregrouplock" => "Change group lock handling",
        "denywindowfromgroup" => "Change group admission",
        "event" => "Send custom event",
        "global" => "Activate global shortcut",
        "setprop" => "Set window property",
        "forceidle" => "Force idle state",
        "releaseinputcapture" => "Release input capture",
        _ => name,
    }
}
pub(super) fn argument(
    panel: &gtk::Box,
    dispatcher: &str,
    initial: &str,
) -> Box<dyn Fn() -> String> {
    // Preserve exact existing arguments; choosing a preset is always explicit.
    let presets: &[(&str, &str)] = match dispatcher {
        "movefocus"
        | "movewindow"
        | "swapwindow"
        | "moveintogroup"
        | "movewindoworgroup"
        | "moveintoorcreategroup" => &[("Left", "l"), ("Right", "r"), ("Up", "u"), ("Down", "d")],
        "dpms" => &[("Turn on", "on"), ("Turn off", "off"), ("Toggle", "toggle")],
        "mouse" => &[
            ("Move window", "movewindow"),
            ("Resize window", "resizewindow"),
        ],
        "submap" => &[("Global shortcuts", "reset")],
        _ => &[],
    };
    let exact = field(
        panel,
        match dispatcher {
            "exec" | "execr" => "Command (stored without running)",
            "submap" => "Target submap",
            "workspace" | "movetoworkspace" | "movetoworkspacesilent" => "Workspace selector",
            _ => "Action argument",
        },
        initial,
        "legacy-bind-argument",
    );
    if !presets.is_empty() {
        let mut names = vec!["Choose a value…"];
        names.extend(presets.iter().map(|(label, _)| *label));
        let pick = select(
            panel,
            "Quick choices",
            &names,
            0,
            "legacy-bind-argument-preset",
        );
        let exact = exact.downgrade();
        pick.connect_selected_notify(move |pick| {
            if let Some((_, value)) = presets.get(pick.selected().wrapping_sub(1) as usize) {
                if let Some(exact) = exact.upgrade() {
                    exact.set_text(value);
                }
            }
        });
    }
    Box::new(move || exact.text().into())
}
fn form(panel: &gtk::Box, entry: &Entry) -> GetEntry {
    match entry {
        Entry::Submap { name, reset } => {
            let global = gtk::CheckButton::with_label("Global shortcuts / reset");
            global.set_active(name.is_empty());
            panel.append(&global);
            let name = field(panel, "Submap name", name, "legacy-submap-name");
            name.set_sensitive(!global.is_active());
            let weak = name.downgrade();
            global.connect_toggled(move |b| {
                if let Some(name) = weak.upgrade() {
                    name.set_sensitive(!b.is_active());
                }
            });
            let has = gtk::CheckButton::with_label("Specify automatic reset target");
            has.set_active(reset.is_some());
            panel.append(&has);
            let reset = field(
                panel,
                "Reset target",
                reset.as_deref().unwrap_or(""),
                "legacy-submap-reset",
            );
            panel.append(&label("This switches the scope of following bindings. It does not create a nested callback. Reset returns following declarations to global shortcuts."));
            Box::new(move || {
                Ok(Entry::Submap {
                    name: if global.is_active() {
                        String::new()
                    } else {
                        name.text().into()
                    },
                    reset: has.is_active().then(|| reset.text().into()),
                })
            })
        }
        Entry::Unbind {
            all,
            chord: initial,
        } => {
            let all_check =
                gtk::CheckButton::with_label("Remove all previously registered shortcuts");
            all_check.set_active(*all);
            panel.append(&all_check);
            let fields = section(panel, "Shortcut to remove");
            let default_chord = Chord {
                keys: vec!["Return".into()],
                ..Default::default()
            };
            let get = chord(&fields, if *all { &default_chord } else { initial }, false);
            panel.append(&label("A specific unbind matches across all submaps. This only stages a declaration; shortcuts on your desktop are unchanged."));
            Box::new(move || {
                Ok(Entry::Unbind {
                    all: all_check.is_active(),
                    chord: if all_check.is_active() {
                        Chord::default()
                    } else {
                        get()?
                    },
                })
            })
        }
        Entry::Bind {
            chord: initial,
            flags,
            description,
            device,
            dispatcher,
            argument: initial_arg,
        } => {
            let shortcut = section(panel, &format!("Shortcut · {}", initial.text()));
            let get_chord = chord(&shortcut, initial, true);
            let options = section(panel, "Behavior and device filters");
            let flags = legacy_binding::FLAGS
                .iter()
                .map(|(flag, title)| {
                    let check = gtk::CheckButton::with_label(title);
                    check.set_active(flags.contains(flag));
                    options.append(&check);
                    (*flag, check)
                })
                .collect::<Vec<_>>();
            let has_description = gtk::CheckButton::with_label("Include description");
            has_description.set_active(description.is_some());
            options.append(&has_description);
            let description = field(
                &options,
                "Description",
                description.as_deref().unwrap_or(""),
                "legacy-bind-description",
            );
            let has_device = gtk::CheckButton::with_label("Filter devices");
            has_device.set_active(device.is_some());
            options.append(&has_device);
            let device_mode = select(
                &options,
                "Device matching",
                &["Only listed devices", "Except listed devices"],
                u32::from(device.as_ref().is_some_and(|d| !d.inclusive)),
                "legacy-bind-device-mode",
            );
            let device_items = tokens(
                &options,
                "Device names or tags",
                device.as_ref().map_or(&[], |d| d.items.as_slice()),
                128,
            );
            let names = legacy_binding::dispatchers();
            let dispatch = select(
                panel,
                "Action",
                &names
                    .iter()
                    .map(|name| format!("{} ({name})", action_name(name)))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                names.iter().position(|n| n == dispatcher).unwrap_or(0) as u32,
                "legacy-bind-dispatcher",
            );
            let args = gtk::Box::new(gtk::Orientation::Vertical, 6);
            panel.append(&args);
            let getter = Rc::new(RefCell::new(argument(&args, dispatcher, initial_arg)));
            let weak = args.downgrade();
            let getter_c = getter.clone();
            dispatch.connect_selected_notify(move |choice| {
                if let Some(args) = weak.upgrade() {
                    while let Some(child) = args.first_child() {
                        args.remove(&child);
                    }
                    *getter_c.borrow_mut() = argument(
                        &args,
                        &legacy_binding::dispatchers()[choice.selected() as usize],
                        "",
                    );
                }
            });
            Box::new(move || {
                Ok(Entry::Bind {
                    chord: get_chord()?,
                    flags: flags
                        .iter()
                        .filter(|(_, c)| c.is_active())
                        .map(|(f, _)| *f)
                        .collect(),
                    description: has_description
                        .is_active()
                        .then(|| description.text().into()),
                    device: has_device.is_active().then(|| binding::DeviceFilter {
                        inclusive: device_mode.selected() == 0,
                        items: device_items(),
                    }),
                    dispatcher: legacy_binding::dispatchers()[dispatch.selected() as usize].clone(),
                    argument: (getter.borrow())(),
                })
            })
        }
    }
}
fn current(state: &Shared) -> Draft {
    let s = state.borrow();
    s.bindings
        .get(&s.draft.version)
        .and_then(|d| d.legacy.clone())
        .unwrap_or_default()
}
fn commit(state: &Shared, draft: Draft, status: &gtk::Label, feedback: &gtk::Label) -> bool {
    let mut s = state.borrow_mut();
    let mut combined = s
        .bindings
        .get(&s.draft.version)
        .cloned()
        .unwrap_or_default();
    combined.legacy = Some(draft);
    let result = s.replace_bindings(combined);
    drop(s);
    match result {
        Ok(()) => {
            super::summary(state, status);
            feedback.set_text("Shortcut draft updated; no action was executed.");
            true
        }
        Err(e) => {
            feedback.set_text(&e);
            false
        }
    }
}
fn find_named(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
    if root.widget_name() == name {
        return Some(root.clone());
    }
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Some(found) = find_named(&widget, name) {
            return Some(found);
        }
        child = widget.next_sibling();
    }
    None
}
fn mode_controls(selection: &gtk::DropDown, state: &Shared) {
    if let Some(window) = selection.root().and_downcast::<adw::Window>() {
        let draft = current(state);
        if let Some(button) = find_named(window.upcast_ref(), "legacy-bind-upgrade") {
            button.set_visible(!draft.local_variables);
        }
        if let Some(button) = find_named(window.upcast_ref(), "legacy-bind-materialize") {
            button.set_sensitive(
                draft.local_variables
                    && draft
                        .source_links
                        .get(selection.selected() as usize)
                        .is_some_and(Option::is_some),
            );
        }
    }
}
fn review_upgrade(
    parent: &adw::Window,
    state: &Shared,
    status: &gtk::Label,
    selection: &gtk::DropDown,
    panel: &gtk::Box,
    feedback: &gtk::Label,
) {
    let literal = current(state);
    if literal.local_variables {
        return;
    }
    let s = state.borrow();
    let Some(doc) = s.legacy_sources.get(&s.draft.version) else {
        return;
    };
    let source = doc.content.clone();
    let epoch = s.generation;
    drop(s);
    let busy = find_named(parent.upcast_ref(), "legacy-bind-upgrade").map(|widget| {
        widget.set_sensitive(false);
        widget.downgrade()
    });
    let parent = parent.downgrade();
    let state = state.clone();
    let status = status.downgrade();
    let selection = selection.downgrade();
    let panel = panel.downgrade();
    let feedback = feedback.downgrade();
    super::workflow::background(
        move || {
            super::worker::run(super::worker::Request::ExpandLegacyBindings {
                source,
                draft: literal,
            })
        },
        move |result| {
            if let Some(button) = busy.as_ref().and_then(gtk::glib::WeakRef::upgrade) {
                button.set_sensitive(true);
            }
            let (Some(parent), Some(status), Some(selection), Some(panel), Some(feedback)) = (
                parent.upgrade(),
                status.upgrade(),
                selection.upgrade(),
                panel.upgrade(),
                feedback.upgrade(),
            ) else {
                return;
            };
            if !parent.is_visible() {
                return;
            }
            if state.borrow().generation != epoch {
                feedback.set_text("The draft changed. Review the variable-aware view again.");
                return;
            }
            let proposal = match result {
                Ok(super::worker::Response::LegacyBindings(draft)) => draft,
                Ok(_) => {
                    feedback.set_text("Unexpected variable view response.");
                    return;
                }
                Err(e) => {
                    feedback.set_text(&e);
                    return;
                }
            };
            if !proposal.local_variables
                || proposal.validate(&state.borrow().draft.version).is_err()
            {
                feedback.set_text("Invalid variable-aware proposal.");
                return;
            }
            let review = adw::Window::builder()
                .title("Review Variable-Based Shortcuts")
                .transient_for(&parent)
                .modal(true)
                .default_width(580)
                .default_height(650)
                .build();
            let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
            for set in [
                gtk::Widget::set_margin_start,
                gtk::Widget::set_margin_end,
                gtk::Widget::set_margin_top,
                gtk::Widget::set_margin_bottom,
            ] {
                set(content.upcast_ref(), 16);
            }
            content.append(&label(&format!("Review {} proposed entries. Your literal-only edits and removals are included; newly recognized local-variable entries are added in source order. No definitions or desktop shortcuts are changed. Undo can restore the previous view.",proposal.entries.len())));
            let choices = gtk::DropDown::from_strings(
                &proposal
                    .entries
                    .iter()
                    .map(Entry::label)
                    .collect::<Vec<_>>()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            );
            compact(&choices);
            content.append(&choices);
            let fields = gtk::Box::new(gtk::Orientation::Vertical, 6);
            content.append(
                &gtk::ScrolledWindow::builder()
                    .child(&fields)
                    .vexpand(true)
                    .hscrollbar_policy(gtk::PolicyType::Never)
                    .build(),
            );
            let proposal = Rc::new(proposal);
            let redraw = Rc::new({
                let proposal = proposal.clone();
                let fields = fields.downgrade();
                move |index: u32| {
                    if let Some(fields) = fields.upgrade() {
                        while let Some(child) = fields.first_child() {
                            fields.remove(&child);
                        }
                        if let Some(entry) = proposal.entries.get(index as usize) {
                            let _get = form(&fields, entry);
                            let mut child = fields.first_child();
                            while let Some(widget) = child {
                                if let Some(expander) = widget.downcast_ref::<gtk::Expander>() {
                                    expander.set_expanded(true);
                                }
                                child = widget.next_sibling();
                            }
                            fields.set_sensitive(false);
                        }
                    }
                }
            });
            redraw(choices.selected());
            let redraw_c = redraw.clone();
            choices.connect_selected_notify(move |choice| redraw_c(choice.selected()));
            let error = label("");
            content.append(&error);
            let accept = gtk::Button::with_label("Use Variable-Aware View");
            accept.set_widget_name("legacy-bind-use-expanded");
            accept.add_css_class("suggested-action");
            content.append(&accept);
            let close = gtk::Button::with_label("Keep Literal-Only View");
            content.append(&close);
            let weak = review.downgrade();
            close.connect_clicked(move |_| {
                if let Some(window) = weak.upgrade() {
                    window.destroy();
                }
            });
            let weak = review.downgrade();
            accept.connect_clicked(move |_| {
                if state.borrow().generation != epoch {
                    error.set_text("The draft changed. Close this review and prepare it again.");
                    return;
                }
                if commit(&state,(*proposal).clone(),&status,&error) {
                    refresh(&state,&selection,&panel,&status,&feedback,0);
                    feedback.set_text("Variable-aware view retained in the draft. Unchanged fields preserve their references.");
                    if let Some(window)=weak.upgrade() {window.destroy();}
                }
            });
            review.set_content(Some(&content));
            review.present();
        },
    );
}
fn render(
    state: &Shared,
    selection: &gtk::DropDown,
    panel: &gtk::Box,
    status: &gtk::Label,
    feedback: &gtk::Label,
) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }
    mode_controls(selection, state);
    let index = selection.selected() as usize;
    let draft = current(state);
    let Some(entry) = draft.entries.get(index) else {
        panel.append(&label("Add a shortcut or submap switch to begin."));
        return;
    };
    if draft.local_variables
        && draft
            .source_links
            .get(index)
            .copied()
            .flatten()
            .is_some_and(|line| {
                let s = state.borrow();
                s.legacy_sources
                    .get(&s.draft.version)
                    .is_some_and(|doc| doc.bindings.derived_sites.binary_search(&line).is_ok())
            })
    {
        panel.append(&label("This source row uses local definitions. Unchanged fields retain their variable references."));
    }
    let getter = form(panel, entry);
    let update = gtk::Button::with_label("Update Entry");
    update.set_widget_name("legacy-bind-update");
    update.add_css_class("suggested-action");
    panel.append(&update);
    let state = state.clone();
    let selection = selection.downgrade();
    let panel = panel.downgrade();
    let status = status.downgrade();
    let feedback = feedback.downgrade();
    let generation = state.borrow().generation;
    update.connect_clicked(move |_| {
        let (Some(selection), Some(panel), Some(status), Some(feedback)) = (
            selection.upgrade(),
            panel.upgrade(),
            status.upgrade(),
            feedback.upgrade(),
        ) else {
            return;
        };
        if state.borrow().generation != generation || selection.selected() as usize != index {
            feedback.set_text("The draft changed. Select this entry again before updating.");
            return;
        }
        let proposed = match getter().and_then(|e| {
            e.validate()?;
            legacy_binding::encode(&e)?;
            Ok(e)
        }) {
            Ok(e) => e,
            Err(e) => {
                feedback.set_text(&e);
                return;
            }
        };
        let mut draft = current(&state);
        draft.entries[index] = proposed;
        if commit(&state, draft, &status, &feedback) {
            refresh(&state, &selection, &panel, &status, &feedback, index);
        }
    });
}
fn refresh(
    state: &Shared,
    selection: &gtk::DropDown,
    panel: &gtk::Box,
    status: &gtk::Label,
    feedback: &gtk::Label,
    index: usize,
) {
    let names = current(state)
        .entries
        .iter()
        .map(|entry| match entry {
            Entry::Bind {
                chord, dispatcher, ..
            } => format!("{} · {}", chord.text(), action_name(dispatcher)),
            _ => entry.label(),
        })
        .collect::<Vec<_>>();
    selection.set_model(Some(&gtk::StringList::new(
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
    if !names.is_empty() {
        selection.set_selected(index.min(names.len() - 1) as u32);
    }
    render(state, selection, panel, status, feedback);
}
pub(super) fn show(
    parent: &impl IsA<gtk::Window>,
    state: &Shared,
    status: &gtk::Label,
) -> adw::Window {
    let window = adw::Window::builder()
        .title("Legacy Shortcuts and Submaps")
        .transient_for(parent)
        .modal(true)
        .default_width(620)
        .default_height(700)
        .build();
    window.set_size_request(320, 420);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for set in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        set(content.upcast_ref(), 16);
    }
    root.append(&content);
    let intro = label("Edit shortcuts in declaration order. Preceding local definitions can supply values; includes and external variables remain unresolved.");
    intro.set_selectable(false);
    content.append(&intro);
    let selection = gtk::DropDown::from_strings(&[]);
    compact(&selection);
    selection.set_widget_name("legacy-bind-entries");
    content.append(&selection);
    let upgrade = gtk::Button::with_label("Review Variable-Based Shortcuts…");
    upgrade.set_widget_name("legacy-bind-upgrade");
    upgrade.set_visible(!current(state).local_variables);
    content.append(&upgrade);
    let kind = gtk::DropDown::from_strings(&["Shortcut", "Submap switch", "Unbind"]);
    compact(&kind);
    kind.set_widget_name("legacy-bind-kind");
    let manage = gtk::Box::new(gtk::Orientation::Vertical, 6);
    manage.append(&kind);
    let grid = gtk::Grid::builder()
        .column_spacing(6)
        .row_spacing(6)
        .column_homogeneous(true)
        .build();
    let buttons =
        ["Add Entry", "Duplicate", "Remove", "Up", "Down", "Undo"].map(gtk::Button::with_label);
    for (i, b) in buttons.iter().enumerate() {
        b.set_hexpand(true);
        grid.attach(b, (i % 2) as i32, (i / 2) as i32, 1, 1);
    }
    manage.append(&grid);
    let materialize = gtk::Button::with_label("Use Literal Values for This Entry");
    materialize.set_widget_name("legacy-bind-materialize");
    materialize.set_tooltip_text(Some("Explicitly stop preserving this entry's source references. Other entries and variable definitions remain unchanged."));
    materialize.set_sensitive(current(state).local_variables);
    manage.append(&materialize);
    let management = gtk::Expander::builder()
        .label("Add, reorder or remove entries")
        .child(&manage)
        .expanded(current(state).is_empty())
        .build();
    management.set_widget_name("legacy-bind-management");
    content.append(&management);
    let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.append(
        &gtk::ScrolledWindow::builder()
            .child(&fields)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    let feedback = label("");
    feedback.set_widget_name("legacy-bind-feedback");
    content.append(&feedback);
    let review = gtk::Button::with_label("Review Binding Config");
    content.append(&review);
    let state_c = state.clone();
    let weak = window.downgrade();
    review.connect_clicked(move |_| {
        if let Some(w) = weak.upgrade() {
            match legacy_binding::export(&current(&state_c)) {
                Ok(text) => {
                    super::text_export::review_legacy_bindings(&w, text);
                }
                Err(e) => {
                    let dialog = adw::AlertDialog::builder()
                        .heading("Cannot prepare binding config")
                        .body(&e)
                        .build();
                    dialog.add_response("close", "Close");
                    dialog.present(Some(&w));
                }
            }
        }
    });
    let separate = gtk::Button::with_label("Separate Lua Binding Drafts…");
    separate.set_visible({
        let s = state.borrow();
        s.bindings
            .get(&s.draft.version)
            .is_some_and(|d| !d.order.is_empty())
    });
    content.append(&separate);
    let weak = window.downgrade();
    let s = state.clone();
    let t = status.clone();
    separate.connect_clicked(move |_| {
        if let Some(w) = weak.upgrade() {
            super::bindings::show_lua(&w, &s, &t);
        }
    });
    let save = gtk::Button::with_label("Save Updated Draft");
    save.set_visible(
        parent
            .upcast_ref::<gtk::Window>()
            .is::<adw::ApplicationWindow>(),
    );
    content.append(&save);
    let parent_weak = parent.downgrade();
    save.connect_clicked(move |_| {
        if let Some(parent) = parent_weak.upgrade() {
            if let Some(action) = parent
                .upcast_ref::<gtk::Window>()
                .downcast_ref::<adw::ApplicationWindow>()
                .and_then(|window| window.lookup_action("save-draft"))
            {
                action.activate(None);
            }
        }
    });
    window.set_content(Some(&root));
    let s = state.clone();
    let choice = selection.clone();
    management.connect_expanded_notify(move |_| mode_controls(&choice, &s));
    let weak = window.downgrade();
    let s = state.clone();
    let t = status.clone();
    let choice = selection.clone();
    let p = fields.clone();
    let f = feedback.clone();
    upgrade.connect_clicked(move |_| {
        if let Some(window) = weak.upgrade() {
            review_upgrade(&window, &s, &t, &choice, &p, &f);
        }
    });
    let s = state.clone();
    let t = status.clone();
    let choice = selection.clone();
    let p = fields.clone();
    let f = feedback.clone();
    materialize.connect_clicked(move |_|{let index=choice.selected()as usize;let mut draft=current(&s);if draft.local_variables&&index<draft.entries.len(){draft.materialize(index);if commit(&s,draft,&t,&f){refresh(&s,&choice,&p,&t,&f,index);f.set_text("This entry will export explicit literal values. Undo restores its source references.");}}});

    let s = state.clone();
    let p = fields.downgrade();
    let t = status.downgrade();
    let f = feedback.downgrade();
    selection.connect_selected_notify(move |selection| {
        if let (Some(p), Some(t), Some(f)) = (p.upgrade(), t.upgrade(), f.upgrade()) {
            render(&s, selection, &p, &t, &f);
        }
    });
    for (operation, button) in buttons.into_iter().enumerate() {
        let s = state.clone();
        let selection = selection.clone();
        let p = fields.clone();
        let t = status.clone();
        let f = feedback.clone();
        let kind = kind.clone();
        button.connect_clicked(move |_| {
            let mut draft = current(&s);
            let mut index = selection.selected() as usize;
            match operation {
                0 => {
                    draft.push(match kind.selected() {
                        1 => Entry::Submap {
                            name: "new-submap".into(),
                            reset: None,
                        },
                        2 => Entry::Unbind {
                            all: false,
                            chord: Chord {
                                keys: vec!["Return".into()],
                                ..Default::default()
                            },
                        },
                        _ => Entry::Bind {
                            chord: Chord {
                                modifiers: vec!["SUPER".into()],
                                keys: vec!["Return".into()],
                                multi: false,
                            },
                            flags: Default::default(),
                            description: None,
                            device: None,
                            dispatcher: "exec".into(),
                            argument: "application".into(),
                        },
                    });
                    index = draft.entries.len() - 1;
                }
                1 if index < draft.entries.len() => {
                    draft.duplicate(index);
                    index += 1;
                }
                2 if index < draft.entries.len() => {
                    draft.remove(index);
                    index = index.saturating_sub(1);
                }
                3 if index > 0 && index < draft.entries.len() => {
                    draft.swap(index, index - 1);
                    index -= 1;
                }
                4 if index < draft.entries.len().saturating_sub(1) => {
                    draft.swap(index, index + 1);
                    index += 1;
                }
                5 => {
                    s.borrow_mut().undo_last();
                    super::summary(&s, &t);
                    refresh(&s, &selection, &p, &t, &f, index);
                    return;
                }
                _ => return,
            }
            if commit(&s, draft, &t, &f) {
                refresh(&s, &selection, &p, &t, &f, index);
            }
        });
    }
    refresh(state, &selection, &fields, status, &feedback, 0);
    window.present();
    window
}
