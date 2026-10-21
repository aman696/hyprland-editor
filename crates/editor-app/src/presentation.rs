//! Navigation and presentation use native adaptive widgets and the GNOME HIG.
use super::{label, Shared};
use adw::prelude::*;
use hyprland_editor_core::{Setting, Value, BASELINE};
pub fn title_words(text: &str) -> String {
    text.split(['_', '.', '-', ' '])
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut c = word.chars();
            c.next()
                .map(|n| n.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn setting_name(id: &str) -> String {
    let leaf = id.rsplit(':').next().unwrap_or(id);
    if let Some(key) = leaf.strip_prefix("kb_") {
        return format!("Keyboard {}", title_words(key));
    }
    if let Some(color) = leaf.strip_prefix("col.") {
        format!("{} Color", title_words(color))
    } else {
        title_words(leaf)
    }
}
pub fn category_name(key: &str) -> String {
    match key {
        "decoration" => "Appearance".into(),
        "input" => "Input Devices".into(),
        "misc" => "Desktop".into(),
        "binds" => "Keybindings".into(),
        "group" => "Window Groups".into(),
        "debug" => "Debugging".into(),
        "render" => "Rendering".into(),
        "dwindle" => "Dwindle Layout".into(),
        "master" => "Master Layout".into(),
        "scrolling" => "Scrolling Layout".into(),
        "xwayland" => "XWayland".into(),
        _ => title_words(key),
    }
}
pub fn context(id: &str) -> String {
    let parts: Vec<_> = id.split(':').collect();
    let mut words = vec![category_name(parts[0])];
    for part in &parts[1..parts.len().saturating_sub(1)] {
        words.push(title_words(part));
    }
    words.join(" · ")
}
pub fn value_text(value: &Value) -> String {
    match value {
        Value::Bool(v) => if *v { "On" } else { "Off" }.into(),
        Value::String(v) => {
            if v.is_empty() {
                "Empty".into()
            } else {
                hyprland_editor_core::source::display_path_bytes(v.as_bytes())
                    .chars()
                    .take(128)
                    .collect()
            }
        }
        Value::Int(v) => v.to_string(),
        Value::Float(v) => v.to_string(),
        Value::Color(c) => format!("#{c:08X}"),
        Value::FontWeight(v) => v.to_string(),
        Value::Vec2([x, y]) => format!("X {x} · Y {y}"),
        Value::CssGap([t, r, b, l]) => format!("Top {t} · Right {r} · Bottom {b} · Left {l}"),
        Value::Gradient { colors, angle } => format!("{} colors · {angle}°", colors.len()),
    }
}
pub fn help(setting: &Setting) -> &'static str {
    match setting.id.as_str() {
        "general:border_size" => "Width of window borders.",
        "general:gaps_in" => "Space between tiled windows.",
        "general:gaps_out" => "Space between windows and the screen edges.",
        "decoration:rounding" => "Corner radius of windows.",
        "decoration:active_opacity" => "Opacity of the focused window.",
        "decoration:inactive_opacity" => "Opacity of unfocused windows.",
        "decoration:blur:enabled" => "Blur transparent areas behind windows.",
        "decoration:shadow:enabled" => "Show shadows around windows.",
        "animations:enabled" => "Enable window and workspace animations.",
        "general:layout" => "Tiling layout used for windows.",
        "input:kb_layout" => "Keyboard layout codes, such as us or gb.",
        _ => "Choose a value for this setting in the draft.",
    }
}
pub fn empty(panel: &gtk::Box, title: &str, description: &str) {
    while let Some(w) = panel.first_child() {
        panel.remove(&w);
    }
    let page = adw::StatusPage::builder()
        .icon_name("preferences-system-symbolic")
        .title(title)
        .description(description)
        .build();
    panel.append(&page);
}
pub fn bridge(window: &adw::ApplicationWindow, name: &str, button: &gtk::Button) {
    let action = gtk::gio::SimpleAction::new(name, None);
    let b = button.clone();
    action.connect_activate(move |_, _| b.emit_clicked());
    action.set_enabled(button.is_sensitive());
    let a = action.downgrade();
    button.connect_sensitive_notify(move |b| {
        if let Some(a) = a.upgrade() {
            a.set_enabled(b.is_sensitive());
        }
    });
    window.add_action(&action);
}
pub fn menu_model() -> gtk::gio::Menu {
    let menu = gtk::gio::Menu::new();
    let files = gtk::gio::Menu::new();
    files.append(Some("Import Configuration…"), Some("win.import"));
    files.append(Some("Save Draft"), Some("win.save-draft"));
    files.append(Some("Undo"), Some("win.undo"));
    files.append(Some("Edit Monitors…"), Some("win.monitors"));
    files.append(Some("Edit Devices…"), Some("win.devices"));
    files.append(Some("Edit Bindings…"), Some("win.bindings"));
    files.append(Some("Edit Workspaces…"), Some("win.workspaces"));
    files.append(Some("Edit Rules…"), Some("win.rules"));
    files.append(Some("Edit Animations…"), Some("win.animations"));
    files.append(Some("Edit Gestures…"), Some("win.gestures"));
    menu.append_section(None, &files);
    let inspect = gtk::gio::Menu::new();
    inspect.append(Some("Refresh Desktop Values"), Some("win.read-live"));
    inspect.append(Some("Review Source Copy…"), Some("win.source-copy"));
    inspect.append(Some("Verify Source File…"), Some("win.reattach-source"));
    inspect.append(
        Some("Compare Changed Source…"),
        Some("win.reconcile-source"),
    );
    inspect.append(Some("Import Diagnostics"), Some("win.diagnostics"));
    inspect.append(Some("Inspect Source Modules…"), Some("win.source-modules"));
    menu.append_section(None, &inspect);
    let tools = gtk::gio::Menu::new();
    tools.append(Some("Prepare Preview Draft…"), Some("win.prepare-preview"));
    tools.append(Some("Hyprland Version…"), Some("win.compatibility"));
    tools.append(Some("Feature Coverage"), Some("win.coverage"));
    tools.append(Some("About Draft Mode"), Some("win.draft-mode"));
    menu.append_section(None, &tools);
    menu
}
pub fn menu_button(model: &gtk::gio::Menu) -> gtk::MenuButton {
    gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Main Menu")
        .menu_model(model)
        .build()
}
pub fn connect_compatibility(window: &adw::ApplicationWindow, version: &gtk::DropDown) {
    let win = window.downgrade();
    // The hidden version model belongs to the window action.
    let master = version.clone();
    let action = gtk::gio::SimpleAction::new("compatibility", None);
    action.connect_activate(move |_, _| {
        let Some(win) = win.upgrade() else {
            return;
        };
        let dialog = adw::Window::builder()
            .title("Hyprland Version")
            .transient_for(&win)
            .modal(true)
            .default_width(460)
            .default_height(240)
            .build();
        let b = gtk::Box::new(gtk::Orientation::Vertical, 16);
        for set in [
            gtk::Widget::set_margin_top,
            gtk::Widget::set_margin_bottom,
            gtk::Widget::set_margin_start,
            gtk::Widget::set_margin_end,
        ] {
            set(b.upcast_ref(), 24);
        }
        b.append(&label(
            "Choose the version for this draft. Each version keeps its own values.",
        ));
        let picker =
            gtk::DropDown::from_strings(&["Hyprland 0.56.2", "Development Snapshot · 6a1b5f0"]);
        picker.set_widget_name("version-picker");
        picker.set_selected(master.selected());
        let m = master.downgrade();
        picker.connect_selected_notify(move |p| {
            if let Some(m) = m.upgrade() {
                m.set_selected(p.selected());
            }
        });
        b.append(&picker);
        let done = gtk::Button::with_label("Done");
        done.set_halign(gtk::Align::End);
        let d = dialog.downgrade();
        done.connect_clicked(move |_| {
            if let Some(d) = d.upgrade() {
                d.close();
            }
        });
        b.append(&done);
        dialog.set_content(Some(&b));
        dialog.present();
    });
    window.add_action(&action);
}
pub fn bind_feedback(
    status: &gtk::Label,
    state: &Shared,
    undo: &gtk::Button,
    review: &gtk::Button,
    scope: &gtk::Label,
) {
    let s = state.clone();
    let u = undo.downgrade();
    let r = review.downgrade();
    let scope = scope.downgrade();
    status.connect_label_notify(move |status| {
        status.set_tooltip_text(Some(&status.text()));
        let s = s.borrow();
        if let Some(u) = u.upgrade() {
            u.set_sensitive(!s.undo.is_empty());
        }
        if let Some(r) = r.upgrade() {
            r.set_sensitive(
                !s.draft.values.is_empty()
                    || s.gestures
                        .get(&s.draft.version)
                        .is_some_and(|g| !g.is_empty())
                    || s.animations
                        .get(&s.draft.version)
                        .is_some_and(|a| !a.is_empty())
                    || s.rules.get(&s.draft.version).is_some_and(|r| !r.is_empty())
                    || s.workspace_rules
                        .get(&s.draft.version)
                        .is_some_and(|r| !r.is_empty())
                    || s.monitors
                        .get(&s.draft.version)
                        .is_some_and(|m| !m.is_empty())
                    || s.bindings
                        .get(&s.draft.version)
                        .is_some_and(|b| !b.is_empty())
                    || s.devices
                        .get(&s.draft.version)
                        .is_some_and(|d| !d.is_empty()),
            );
        }
        if let Some(scope) = scope.upgrade() {
            scope.set_text(if s.draft.version == BASELINE {
                "Hyprland 0.56.2 · Draft Mode"
            } else {
                "Development Snapshot · Draft Mode"
            });
        }
    });
}
