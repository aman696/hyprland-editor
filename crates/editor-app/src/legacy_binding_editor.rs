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
fn action_name(name: &str) -> &str {
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
fn argument(panel: &gtk::Box, dispatcher: &str, initial: &str) -> Box<dyn Fn() -> String> {
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
    let index = selection.selected() as usize;
    let draft = current(state);
    let Some(entry) = draft.entries.get(index) else {
        panel.append(&label("Add a shortcut or submap switch to begin."));
        return;
    };
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
    let intro = label("Edit shortcuts in declaration order. Included files and variable-based entries remain unresolved.");
    intro.set_selectable(false);
    content.append(&intro);
    let selection = gtk::DropDown::from_strings(&[]);
    compact(&selection);
    selection.set_widget_name("legacy-bind-entries");
    content.append(&selection);
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
                    draft.entries.push(match kind.selected() {
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
                    let e = draft.entries[index].clone();
                    draft.entries.insert(index + 1, e);
                    index += 1;
                }
                2 if index < draft.entries.len() => {
                    draft.entries.remove(index);
                    index = index.saturating_sub(1);
                }
                3 if index > 0 && index < draft.entries.len() => {
                    draft.entries.swap(index, index - 1);
                    index -= 1;
                }
                4 if index < draft.entries.len().saturating_sub(1) => {
                    draft.entries.swap(index, index + 1);
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
