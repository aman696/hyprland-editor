//! Review a materialized native draft and its captured include files.
use super::{label, project_copy, project_export, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::legacy_includes::project_copy::{self as native, Prepared};
use std::sync::Arc;

pub(super) fn connect(
    button: &gtk::Button,
    parent: &adw::Window,
    state: &Shared,
    feedback: &gtk::Label,
) {
    let (parent, state, feedback) = (parent.downgrade(), state.clone(), feedback.downgrade());
    button.connect_clicked(move |button| {
        let Some(feedback) = feedback.upgrade() else {
            return;
        };
        let (document, origin, draft, captures, generation) = {
            let state = state.borrow();
            let version = &state.draft.version;
            let (Some(document), Some(origin)) = (
                state.legacy_sources.get(version),
                state.source_origins.get(version),
            ) else {
                feedback.set_text("Import a native configuration before exporting its files.");
                return;
            };
            (
                document.clone(),
                origin.selected_path.clone(),
                workflow::draft_workspace(&state),
                state.legacy_include_files.clone(),
                state.generation,
            )
        };
        feedback.set_text("Preparing the root and linked file copies…");
        button.set_sensitive(false);
        let (parent, button, state) = (parent.clone(), button.downgrade(), state.clone());
        workflow::background(
            move || {
                let root = document.edited_copy(&draft)?;
                let (files, revisions) = captures
                    .as_ref()
                    .map_or((&[][..], &[][..]), |s| (&s.files[..], &s.revisions[..]));
                native::prepare(&origin, &root, files, revisions)
            },
            move |result| {
                if let Some(button) = button.upgrade() {
                    button.set_sensitive(true);
                }
                let Some(parent) = parent.upgrade().filter(gtk::prelude::WidgetExt::is_visible)
                else {
                    return;
                };
                if state.borrow().generation != generation {
                    feedback.set_text("The draft changed. Review the current files again.");
                    return;
                }
                match result {
                    Ok(copy) => {
                        feedback.set_text("");
                        review(&parent, &state, generation, Arc::new(copy));
                    }
                    Err(error) => feedback.set_text(&error),
                }
            },
        );
    });
}

fn review(parent: &adw::Window, state: &Shared, reviewed_at: u64, copy: Arc<Prepared>) {
    let window = adw::Window::builder()
        .title("Review Native Configuration Folder")
        .transient_for(parent)
        .modal(true)
        .default_width(680)
        .default_height(570)
        .build();
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    body.set_margin_start(16);
    body.set_margin_end(16);
    body.set_margin_top(12);
    body.set_margin_bottom(12);
    frame.append(&body);
    let mut summary = format!(
        "{} files · {} include occurrences",
        copy.files.len(),
        copy.include_count,
    );
    if copy.detached_count != 0 {
        summary.push_str(&format!(
            " · {} detached snapshots omitted",
            copy.detached_count
        ));
    }
    body.append(&label(&summary));
    body.append(&label(
        "Start with hyprland.conf in the exported folder. Include paths point to these copies. Other paths and commands stay as written.",
    ));
    let names = copy
        .files
        .iter()
        .map(|file| file.name.as_str())
        .collect::<Vec<_>>();
    let selected = gtk::DropDown::from_strings(&names);
    super::monitors::compact(&selected);
    selected.set_widget_name("native-folder-copy-files");
    selected.update_property(&[gtk::accessible::Property::Label("Export filename")]);
    body.append(&selected);
    let origin = label("");
    origin.set_widget_name("native-folder-copy-origin");
    body.append(&origin);
    let view = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::Char)
        .build();
    view.set_widget_name("native-folder-copy-source");
    body.append(
        &gtk::ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .min_content_height(140)
            .build(),
    );
    let display = {
        let copy = copy.clone();
        move |selected: &gtk::DropDown| {
            if let Some(file) = copy.files.get(selected.selected() as usize) {
                origin.set_text(&format!("From {}", file.source_path));
                view.buffer().set_text(&file.content);
            }
        }
    };
    display(&selected);
    selected.connect_selected_notify(display);
    let feedback = label("");
    feedback.set_widget_name("native-folder-copy-status");
    body.append(&feedback);
    let save = gtk::Button::with_label("Save New Folder…");
    save.add_css_class("suggested-action");
    save.set_halign(gtk::Align::End);
    body.append(&save);
    let (win, state) = (window.downgrade(), state.clone());
    save.connect_clicked(move |_| {
        let Some(window) = win.upgrade() else {
            return;
        };
        if state.borrow().generation != reviewed_at {
            feedback.set_text("The draft changed. Close this review and prepare it again.");
            return;
        }
        let (state, copy) = (state.clone(), copy.clone());
        project_copy::export_dialog(
            &window,
            &feedback,
            move || state.borrow().generation == reviewed_at,
            move |path, name| project_export::save_native(path, name, &copy),
        );
    });
    window.set_content(Some(&frame));
    window.present();
}
