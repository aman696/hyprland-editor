//! Ordered environment, command and permission drafts; no host effects.
use super::{label, worker, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::session::{Draft, Entry, PermissionMode, PermissionType, Trigger};
fn current(state: &Shared) -> Draft {
    let s = state.borrow();
    s.session.get(&s.draft.version).cloned().unwrap_or_default()
}
fn field(panel: &gtk::Box, title: &str, value: &str, name: &str) -> gtk::Entry {
    panel.append(&label(title));
    let entry = gtk::Entry::builder().text(value).max_length(16384).build();
    entry.set_widget_name(name);
    entry.update_property(&[gtk::accessible::Property::Label(title)]);
    panel.append(&entry);
    entry
}
fn select(panel: &gtk::Box, title: &str, items: &[&str], index: u32, name: &str) -> gtk::DropDown {
    panel.append(&label(title));
    let choice = gtk::DropDown::from_strings(items);
    choice.set_selected(index);
    choice.set_widget_name(name);
    choice.update_property(&[gtk::accessible::Property::Label(title)]);
    panel.append(&choice);
    choice
}
// Recognize only patterns reproduced byte-for-byte by our literal builders.
// Other regular expressions stay editable as their original pattern.
fn literal_input(pattern: &str) -> Option<String> {
    let mut chars = pattern.chars();
    let mut text = String::new();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let next = chars.next()?;
            if !"\\.+*?()|[]{}^$".contains(next) {
                return None;
            }
            text.push(next);
        } else if "\\.+*?()|[]{}^$".contains(c) {
            return None;
        } else {
            text.push(c);
        }
    }
    Some(text)
}
fn permission_input(pattern: &str) -> (u32, String) {
    let shapes = [
        pattern.strip_prefix('^').and_then(|p| p.strip_suffix('$')),
        pattern.strip_prefix('^').and_then(|p| p.strip_suffix(".*")),
        pattern.strip_prefix(".*").and_then(|p| p.strip_suffix('$')),
        pattern
            .strip_prefix(".*")
            .and_then(|p| p.strip_suffix(".*")),
    ];
    for (index, shape) in shapes.into_iter().enumerate() {
        if let Some(text) = shape.and_then(literal_input) {
            return (index as u32, text);
        }
    }
    (4, pattern.into())
}
type GetEntry = Box<dyn Fn() -> Result<Entry, String>>;
fn form(panel: &gtk::Box, entry: &Entry) -> GetEntry {
    match entry {
        Entry::Environment { name, value, dbus } => {
            let name = field(panel, "Variable name", name, "session-env-name");
            panel.append(&label("Value (an empty value is preserved)"));
            let value_view = gtk::TextView::builder()
                .wrap_mode(gtk::WrapMode::WordChar)
                .height_request(80)
                .build();
            value_view.set_widget_name("session-env-value");
            value_view.buffer().set_text(value);
            value_view.update_property(&[gtk::accessible::Property::Label("Environment value")]);
            panel.append(
                &gtk::ScrolledWindow::builder()
                    .child(&value_view)
                    .min_content_height(80)
                    .build(),
            );
            let activation = select(
                panel,
                "Also update activation services",
                &["Unspecified", "No", "Yes"],
                match dbus {
                    None => 0,
                    Some(false) => 1,
                    Some(true) => 2,
                },
                "session-env-dbus",
            );
            panel.append(&label("This stages the declaration only. Existing applications may require a new session; activation-service propagation is an external effect."));
            Box::new(move || {
                let buffer = value_view.buffer();
                Ok(Entry::Environment {
                    name: name.text().into(),
                    value: buffer
                        .text(&buffer.start_iter(), &buffer.end_iter(), false)
                        .into(),
                    dbus: match activation.selected() {
                        1 => Some(false),
                        2 => Some(true),
                        _ => None,
                    },
                })
            })
        }
        Entry::Command { command, trigger } => {
            let command = field(panel, "Command", command, "session-command");
            let trigger = select(
                panel,
                "Run when",
                &Trigger::ALL.map(Trigger::label),
                Trigger::ALL.iter().position(|v| v == trigger).unwrap() as u32,
                "session-command-trigger",
            );
            panel.append(&label("Commands are stored and exported without running them. Each configuration load can include startup and reload; choose Session starts for a startup-only hook. An application already launched cannot be undone by restoring a configuration."));
            Box::new(move || {
                Ok(Entry::Command {
                    command: command.text().into(),
                    trigger: Trigger::ALL[trigger.selected() as usize],
                })
            })
        }
        Entry::Permission {
            binary,
            permission,
            mode,
        } => {
            let (match_mode, input) = permission_input(binary);
            let matching = select(
                panel,
                "Application matching",
                &[
                    "Exact path or text",
                    "Starts with",
                    "Ends with",
                    "Contains",
                    "Existing regular expression",
                ],
                match_mode,
                "session-permission-match",
            );
            let binary = field(
                panel,
                "Application path or pattern",
                &input,
                "session-permission-binary",
            );
            let permission = select(
                panel,
                "Capability",
                &[
                    "Screen capture",
                    "Cursor position",
                    "Native plugin",
                    "Keyboard input",
                    "Input capture",
                ],
                PermissionType::ALL
                    .iter()
                    .position(|v| v == permission)
                    .unwrap() as u32,
                "session-permission-type",
            );
            let mode = select(
                panel,
                "Decision",
                &["Ask", "Allow", "Deny"],
                PermissionMode::ALL.iter().position(|v| v == mode).unwrap() as u32,
                "session-permission-mode",
            );
            panel.append(&label("Permission rules take effect at session startup. Editing does not enable enforcement or change current permissions. Check the separate enforcement setting before relying on a rule."));
            Box::new(move || {
                let text = binary.text();
                let literal = hyprland_editor_core::regex::literal(&text);
                let binary = match matching.selected() {
                    0 => format!("^{literal}$"),
                    1 => format!("^{literal}.*"),
                    2 => format!(".*{literal}$"),
                    3 => format!(".*{literal}.*"),
                    _ => text.into(),
                };
                Ok(Entry::Permission {
                    binary,
                    permission: PermissionType::ALL[permission.selected() as usize],
                    mode: PermissionMode::ALL[mode.selected() as usize],
                })
            })
        }
    }
}
fn render(
    window: &adw::Window,
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
        panel.append(&label("Add an entry to begin."));
        return;
    };
    let getter = form(panel, entry);
    let update = gtk::Button::with_label("Update Entry");
    update.add_css_class("suggested-action");
    panel.append(&update);
    let state = state.clone();
    let win = window.downgrade();
    let selection = selection.downgrade();
    let panel = panel.downgrade();
    let status = status.downgrade();
    let feedback = feedback.downgrade();
    update.connect_clicked(move |button| {
        let proposed = match getter().and_then(|e| {
            e.validate()?;
            Ok(e)
        }) {
            Ok(entry) => entry,
            Err(error) => {
                if let Some(feedback) = feedback.upgrade() {
                    feedback.set_text(&error);
                }
                return;
            }
        };
        let source = proposed.export_lua().unwrap();
        let epoch = state.borrow().generation;
        let version = state.borrow().draft.version.clone();
        button.set_sensitive(false);
        if let Some(panel) = panel.upgrade() {
            panel.set_sensitive(false);
        }
        let win = win.clone();
        let state = state.clone();
        let selection = selection.clone();
        let panel = panel.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let button = button.downgrade();
        workflow::background(
            move || worker::run(worker::Request::Parse { source, version }),
            move |result| {
                if let Some(button) = button.upgrade() {
                    button.set_sensitive(true);
                }
                if let Some(panel) = panel.upgrade() {
                    panel.set_sensitive(true);
                }
                let (Some(window), Some(selection), Some(panel), Some(status), Some(feedback)) = (
                    win.upgrade(),
                    selection.upgrade(),
                    panel.upgrade(),
                    status.upgrade(),
                    feedback.upgrade(),
                ) else {
                    return;
                };
                if !window.is_visible() {
                    return;
                }
                if state.borrow().generation != epoch || selection.selected() as usize != index {
                    feedback.set_text("The draft or selection changed. Review this entry again.");
                    render(&window, &state, &selection, &panel, &status, &feedback);
                    return;
                }
                match result {
                    Ok(worker::Response::Parsed(report))
                        if !report.opaque_effects
                            && report.diagnostics.is_empty()
                            && report.session.entries == [proposed.clone()] =>
                    {
                        let mut draft = current(&state);
                        let Some(entry) = draft.entries.get_mut(index) else {
                            feedback
                                .set_text("This entry is no longer available. Select it again.");
                            refresh(
                                &window, &state, &selection, &panel, &status, &feedback, index,
                            );
                            return;
                        };
                        *entry = proposed;
                        match state.borrow_mut().replace_session(draft) {
                            Ok(()) => feedback.set_text("Entry updated in the draft."),
                            Err(error) => feedback.set_text(&error),
                        }
                        super::summary(&state, &status);
                        refresh(
                            &window, &state, &selection, &panel, &status, &feedback, index,
                        );
                    }
                    Ok(worker::Response::Parsed(report)) => {
                        let messages = report
                            .diagnostics
                            .iter()
                            .map(|d| d.message.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        feedback.set_text(if messages.is_empty() {
                            "Validation did not reproduce the requested entry."
                        } else {
                            &messages
                        });
                    }
                    Ok(_) => feedback.set_text("Unexpected validation response."),
                    Err(error) => feedback.set_text(&error),
                }
            },
        );
    });
}
fn refresh(
    window: &adw::Window,
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
        .map(Entry::label)
        .collect::<Vec<_>>();
    selection.set_model(Some(&gtk::StringList::new(
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
    if !names.is_empty() {
        selection.set_selected(index.min(names.len() - 1) as u32);
    }
    render(window, state, selection, panel, status, feedback);
}
pub(super) fn show(
    parent: &impl IsA<gtk::Window>,
    state: &Shared,
    status: &gtk::Label,
) -> adw::Window {
    let window = adw::Window::builder()
        .title("Environment and Startup")
        .transient_for(parent)
        .modal(true)
        .default_width(620)
        .default_height(700)
        .build();
    window.set_size_request(320, 420);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.append(&label("Stage environment, startup commands and permission rules. Their order is retained. Nothing runs or changes permissions while editing."));
    let selection = gtk::DropDown::from_strings(&[]);
    selection.set_widget_name("session-entries");
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        item.set_child(Some(
            &gtk::Label::builder()
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .max_width_chars(24)
                .xalign(0.0)
                .build(),
        ));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        item.child().and_downcast::<gtk::Label>().unwrap().set_text(
            &item
                .item()
                .and_downcast::<gtk::StringObject>()
                .unwrap()
                .string(),
        );
    });
    selection.set_factory(Some(&factory));
    content.append(&selection);
    let actions = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let kind = gtk::DropDown::from_strings(&[
        "Choose an entry type…",
        "Environment variable",
        "Startup command",
        "Permission rule",
    ]);
    actions.append(&kind);
    let add = gtk::Button::with_label("Add Entry");
    add.set_sensitive(false);
    actions.append(&add);
    let more = gtk::Grid::builder()
        .row_spacing(6)
        .column_spacing(6)
        .column_homogeneous(true)
        .build();
    let duplicate = gtk::Button::with_label("Duplicate");
    let remove = gtk::Button::with_label("Remove");
    let up = gtk::Button::with_label("Up");
    let down = gtk::Button::with_label("Down");
    for (index, button) in [&duplicate, &remove, &up, &down].into_iter().enumerate() {
        button.set_hexpand(true);
        more.attach(button, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
    actions.append(&more);
    content.append(&actions);
    let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.append(
        &gtk::ScrolledWindow::builder()
            .child(&fields)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    let feedback = label("");
    feedback.set_widget_name("session-editor-status");
    content.append(&feedback);
    window.set_content(Some(&content));
    let add_weak = add.downgrade();
    kind.connect_selected_notify(move |kind| {
        if let Some(add) = add_weak.upgrade() {
            add.set_sensitive(kind.selected() > 0);
        }
    });
    let win = window.downgrade();
    let state_c = state.clone();
    let fields_c = fields.clone();
    let status_c = status.clone();
    let feedback_c = feedback.clone();
    selection.connect_selected_notify(move |selection| {
        if let Some(window) = win.upgrade() {
            render(
                &window,
                &state_c,
                selection,
                &fields_c,
                &status_c,
                &feedback_c,
            );
        }
    });
    for (button, operation) in [(add, 0), (duplicate, 1), (remove, 2), (up, 3), (down, 4)] {
        let win = window.downgrade();
        let state = state.clone();
        let selection = selection.clone();
        let fields = fields.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let kind = kind.clone();
        button.connect_clicked(move |_| {
            let Some(window) = win.upgrade() else {
                return;
            };
            let mut draft = current(&state);
            let mut index = selection.selected() as usize;
            match operation {
                0 => {
                    let entry = match kind.selected() {
                        1 => Entry::Environment {
                            name: "NEW_VARIABLE".into(),
                            value: String::new(),
                            dbus: None,
                        },
                        2 => Entry::Command {
                            command: "application".into(),
                            trigger: Trigger::Start,
                        },
                        3 => Entry::Permission {
                            binary: "^/usr/bin/application$".into(),
                            permission: PermissionType::Screencopy,
                            mode: PermissionMode::Ask,
                        },
                        _ => return,
                    };
                    draft.entries.push(entry);
                    index = draft.entries.len() - 1;
                }
                1 if index < draft.entries.len() => {
                    let entry = draft.entries[index].clone();
                    draft.entries.insert(index + 1, entry);
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
                _ => return,
            }
            let result = state.borrow_mut().replace_session(draft);
            match result {
                Ok(()) => {
                    super::summary(&state, &status);
                    feedback.set_text("Session draft updated.");
                    refresh(
                        &window, &state, &selection, &fields, &status, &feedback, index,
                    );
                }
                Err(error) => feedback.set_text(&error),
            }
        });
    }
    refresh(&window, state, &selection, &fields, status, &feedback, 0);
    window.present();
    window
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

#[cfg(test)]
pub(super) fn test_editor(parent: &adw::ApplicationWindow) {
    fn widgets(root: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut out = vec![root.clone()];
        let mut child = root.first_child();
        while let Some(w) = child {
            out.extend(widgets(&w));
            child = w.next_sibling();
        }
        out
    }
    fn named<
        T: gtk::glib::object::IsA<gtk::Widget>
            + gtk::glib::object::IsA<gtk::glib::Object>
            + Clone
            + gtk::prelude::StaticType,
    >(
        window: &adw::Window,
        name: &str,
    ) -> T {
        widgets(window.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap()
            .downcast()
            .ok()
            .unwrap()
    }
    fn button(window: &adw::Window, name: &str) -> gtk::Button {
        widgets(window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(name))
            .unwrap()
    }
    fn wait(test: impl Fn() -> bool) {
        let start = std::time::Instant::now();
        while !test() {
            let context = gtk::glib::MainContext::default();
            while context.pending() {
                context.iteration(false);
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(12));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    let state = super::initial_state();
    let status = gtk::Label::new(None);
    let version = state.borrow().draft.version.clone();
    let root = super::test_support::TempDir::new("session-ui");
    let original = root.path().join("original.lua");
    let source="hl.env('GTK_THEME','Old',false);hl.on('hyprland.start',function()hl.exec_cmd('old command')end);hl.permission{binary='^/usr/bin/application$',type='screencopy',mode='ask'}";
    std::fs::write(&original, source).unwrap();
    let captured = hyprland_editor_core::storage::read_source(&original).unwrap();
    let document =
        workflow::parse_source(captured.content, version.clone(), &state.borrow().catalogue)
            .unwrap();
    assert!(document.report.diagnostics.is_empty());
    {
        let mut s = state.borrow_mut();
        s.session
            .insert(version.clone(), document.report.session.clone());
        s.sources
            .insert(version.clone(), std::sync::Arc::new(document));
        s.source_origins.insert(version.clone(), captured.identity);
    }
    std::fs::remove_file(&original).unwrap();
    let initial = current(&state);
    let window = show(parent, &state, &status);
    let selection: gtk::DropDown = named(&window, "session-entries");
    let name: gtk::Entry = named(&window, "session-env-name");
    name.set_text("GUI_THEME");
    let value: gtk::TextView = named(&window, "session-env-value");
    value.buffer().set_text("");
    let dbus: gtk::DropDown = named(&window, "session-env-dbus");
    dbus.set_selected(2);
    let update = button(&window, "Update Entry");
    let generation = state.borrow().generation;
    update.emit_clicked();
    wait(|| state.borrow().generation != generation);
    assert_eq!(
        current(&state).entries[0],
        Entry::Environment {
            name: "GUI_THEME".into(),
            value: String::new(),
            dbus: Some(true)
        }
    );
    assert!(state.borrow().dirty_version(&version));
    state.borrow_mut().undo_last();
    assert_eq!(current(&state), initial);
    selection.set_selected(1);
    let command: gtk::Entry = named(&window, "session-command");
    command.set_text("printf 'GUI_COMMAND_NEVER_RUN'");
    let trigger: gtk::DropDown = named(&window, "session-command-trigger");
    trigger.set_selected(3);
    let generation = state.borrow().generation;
    button(&window, "Update Entry").emit_clicked();
    wait(|| state.borrow().generation != generation);
    selection.set_selected(2);
    let matching: gtk::DropDown = named(&window, "session-permission-match");
    assert_eq!(
        matching.selected(),
        0,
        "Literal permission path must stay graphical"
    );
    matching.set_selected(4);
    let invalid: gtk::Entry = named(&window, "session-permission-binary");
    invalid.set_text("(");
    let bad_update = button(&window, "Update Entry");
    let before = state.borrow().generation;
    bad_update.emit_clicked();
    wait(|| bad_update.is_sensitive());
    assert_eq!(state.borrow().generation, before);
    assert_eq!(
        invalid.text(),
        "(",
        "Validation errors must retain entered text"
    );
    matching.set_selected(0);
    let binary: gtk::Entry = named(&window, "session-permission-binary");
    binary.set_text("/usr/bin/application+literal");
    let mode: gtk::DropDown = named(&window, "session-permission-mode");
    mode.set_selected(2);
    let generation = state.borrow().generation;
    button(&window, "Update Entry").emit_clicked();
    wait(|| state.borrow().generation != generation);
    assert!(
        matches!(&current(&state).entries[2],Entry::Permission{binary,mode:PermissionMode::Deny,..}if binary=="^/usr/bin/application\\+literal$")
    );
    let count = current(&state).entries.len();
    button(&window, "Duplicate").emit_clicked();
    assert_eq!(current(&state).entries.len(), count + 1);
    button(&window, "Up").emit_clicked();
    button(&window, "Down").emit_clicked();
    button(&window, "Remove").emit_clicked();
    assert_eq!(current(&state).entries.len(), count);
    let snapshot = super::project_save::Snapshot::capture(&state.borrow()).unwrap();
    let saved = root.path().join("session.hye");
    std::fs::write(
        &saved,
        snapshot
            .workspace
            .to_json(&state.borrow().catalogue)
            .unwrap(),
    )
    .unwrap();
    assert!(snapshot.mark_saved(&state));
    assert!(!state.borrow().dirty_version(&version));
    let checked = workflow::check_workspace(saved).unwrap();
    assert_eq!(checked["session_declarations"], 3);
    let workspace = workflow::captured_workspace(&state.borrow()).unwrap();
    let worker::Response::Copy(copy) = worker::run(worker::Request::CopyWorkspace {
        workspace: Box::new(workspace),
    })
    .unwrap() else {
        panic!("wrong copy response")
    };
    assert!(
        copy.contains("GUI_COMMAND_NEVER_RUN")
            && copy.contains("hyprland.shutdown")
            && copy.contains("deny")
    );
    let (minimum, _, _, _) = window
        .content()
        .unwrap()
        .measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Session editor needs {minimum}px");
    window.set_default_size(360, 760);
    window.present();
    wait(|| (350..=360).contains(&window.width()));
    for _ in 0..25 {
        let context = gtk::glib::MainContext::default();
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if let Some(path) = std::env::var_os("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&window));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
        window
            .renderer()
            .unwrap()
            .render_texture(snapshot.to_node().expect("session editor frame"), None)
            .save_to_png(format!(
                "{}.session.png",
                std::path::Path::new(&path).display()
            ))
            .unwrap();
    }
    window.destroy();
}
