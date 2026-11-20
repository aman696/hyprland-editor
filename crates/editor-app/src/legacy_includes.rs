//! Graphical editing of source directives through the private revision workflow.
use super::{legacy_source::Document, worker, Shared};
use adw::prelude::*;
use hyprland_editor_core::legacy_includes::{self as codec, Include};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub(super) fn connect(
    button: &gtk::Button,
    parent: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) {
    let (parent, state, status) = (parent.downgrade(), state.clone(), status.clone());
    button.connect_clicked(move |_| {
        if let Some(parent) = parent.upgrade() {
            if let Err(error) = open(&parent, &state, &status) {
                super::notice(&parent, "Cannot edit included files", &error);
            }
        }
    });
}

pub(super) fn open(
    parent: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) -> Result<adw::Window, String> {
    open_for_file(parent, state, status, true)
}

pub(super) fn open_for_file(
    parent: &impl IsA<gtk::Window>,
    state: &Shared,
    status: &gtk::Label,
    show_snapshots: bool,
) -> Result<adw::Window, String> {
    let source = {
        let state = state.borrow();
        state.legacy_sources.get(&state.draft.version).cloned()
    }
    .ok_or("Import a .conf project to edit its included-file directives.")?;
    let (initial, unresolved) = codec::inspect(&source.content)?;
    let win = adw::Window::builder()
        .title("Included Files")
        .transient_for(parent)
        .modal(true)
        .default_width(560)
        .default_height(520)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    panel.set_margin_top(16);
    panel.set_margin_bottom(16);
    panel.set_margin_start(16);
    panel.set_margin_end(16);
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&panel)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    win.set_content(Some(&root));
    panel.append(&super::label("Edit the paths and order of this file’s include directives. Review the resulting configuration before keeping changes."));
    if unresolved > 0 {
        panel.append(&super::label(&format!(
            "{unresolved} include directives use syntax that this editor preserves as source text."
        )));
    }
    let snapshots = gtk::Button::with_label("Linked File Snapshots…");
    panel.append(&snapshots);
    snapshots.set_visible(show_snapshots);
    {
        let (win, state, status) = (win.downgrade(), state.clone(), status.clone());
        snapshots.connect_clicked(move |_| {
            if let Some(win) = win.upgrade() {
                if let Err(error) = super::legacy_capture::open(&win, &state, &status) {
                    let dialog = adw::MessageDialog::builder()
                        .transient_for(&win)
                        .heading("Cannot show linked files")
                        .body(&error)
                        .build();
                    dialog.add_response("close", "Close");
                    dialog.present();
                }
            }
        });
    }
    let model = gtk::StringList::new(&[]);
    let picker = gtk::DropDown::builder().model(&model).hexpand(true).build();
    super::monitors::compact(&picker);
    picker.set_widget_name("include-picker");
    panel.append(&picker);
    let path = gtk::Entry::builder()
        .max_length(codec::MAX_PATH as i32)
        .build();
    path.set_widget_name("include-path");
    path.update_property(&[gtk::accessible::Property::Label("Include path or pattern")]);
    panel.append(&super::label("Path or pattern"));
    panel.append(&path);
    let buttons = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(5)
        .column_spacing(6)
        .row_spacing(6)
        .build();
    panel.append(&buttons);
    let feedback = super::label("");
    panel.append(&feedback);
    let review = gtk::Button::with_label("Review Include Changes");
    review.add_css_class("suggested-action");
    panel.append(&review);

    let rows = Rc::new(RefCell::new(initial.clone()));
    let active = Rc::new(Cell::new(gtk::INVALID_LIST_POSITION));
    let rendering = Rc::new(Cell::new(false));
    let flush = {
        let (rows, active, path) = (rows.clone(), active.clone(), path.clone());
        move || {
            if let Some(row) = rows.borrow_mut().get_mut(active.get() as usize) {
                let value = path.text();
                codec::validate_template(&value)?;
                row.template = value.to_string();
            }
            Ok::<_, String>(())
        }
    };
    let render = {
        let (rows, active, path, picker, rendering) = (
            rows.clone(),
            active.clone(),
            path.clone(),
            picker.downgrade(),
            rendering.clone(),
        );
        move |index: usize| {
            let Some(picker) = picker.upgrade() else {
                return;
            };
            rendering.set(true);
            let rows = rows.borrow();
            let labels: Vec<_> = rows.iter().map(|row| row.template.as_str()).collect();
            model.splice(0, model.n_items(), &labels);
            let index = index.min(rows.len().saturating_sub(1));
            let row = rows.get(index);
            active.set(row.map_or(gtk::INVALID_LIST_POSITION, |_| index as u32));
            picker.set_selected(active.get());
            path.set_sensitive(row.is_some());
            path.set_text(row.map_or("", |row| row.template.as_str()));
            rendering.set(false);
        }
    };
    {
        let (active, feedback, flush, render) = (
            active.clone(),
            feedback.clone(),
            flush.clone(),
            render.clone(),
        );
        picker.connect_selected_notify(move |picker| {
            if rendering.get() {
                return;
            }
            let next = picker.selected();
            match flush() {
                Ok(()) => {
                    feedback.set_text("");
                    render(next as usize);
                }
                Err(error) => {
                    rendering.set(true);
                    picker.set_selected(active.get());
                    rendering.set(false);
                    feedback.set_text(&error);
                }
            }
        });
    }
    enum Operation {
        Add,
        Duplicate,
        Remove,
        Up,
        Down,
    }
    for (label, operation) in [
        ("Add", Operation::Add),
        ("Duplicate", Operation::Duplicate),
        ("Remove", Operation::Remove),
        ("Move Up", Operation::Up),
        ("Move Down", Operation::Down),
    ] {
        let button = gtk::Button::with_label(label);
        buttons.insert(&button, -1);
        let (rows, active, path, feedback, flush, render) = (
            rows.clone(),
            active.clone(),
            path.clone(),
            feedback.clone(),
            flush.clone(),
            render.clone(),
        );
        button.connect_clicked(move |_| {
            if !matches!(operation, Operation::Remove) {
                if let Err(error) = flush() {
                    feedback.set_text(&error);
                    return;
                }
            }
            let mut rows = rows.borrow_mut();
            if matches!(operation, Operation::Add | Operation::Duplicate)
                && rows.len() >= codec::MAX_ROWS
            {
                feedback.set_text("This editor supports up to 1,024 include directives.");
                return;
            }
            let index = active.get() as usize;
            let select = match operation {
                Operation::Add => {
                    let index = rows.get(index).map_or(rows.len(), |_| index + 1);
                    rows.insert(
                        index,
                        Include {
                            original: None,
                            template: String::new(),
                        },
                    );
                    index
                }
                Operation::Duplicate if index < rows.len() => {
                    let template = rows[index].template.clone();
                    rows.insert(
                        index + 1,
                        Include {
                            original: None,
                            template,
                        },
                    );
                    index + 1
                }
                Operation::Remove if index < rows.len() => {
                    rows.remove(index);
                    index
                }
                Operation::Up if index > 0 && index < rows.len() => {
                    rows.swap(index - 1, index);
                    index - 1
                }
                Operation::Down if index < rows.len().saturating_sub(1) => {
                    rows.swap(index, index + 1);
                    index + 1
                }
                _ => return,
            };
            drop(rows);
            feedback.set_text("");
            render(select);
            if matches!(operation, Operation::Add | Operation::Duplicate) {
                path.grab_focus();
                path.select_region(0, -1);
            }
        });
    }
    render(0);
    let weak = win.downgrade();
    {
        let (state, status, rows, feedback, flush, weak) = (
            state.clone(),
            status.clone(),
            rows.clone(),
            feedback.clone(),
            flush.clone(),
            weak.clone(),
        );
        let panel = panel.downgrade();
        review.connect_clicked(move |_| {
            if let Err(error) = flush() { feedback.set_text(&error); return; }
            let pending = rows.borrow().clone();
            let (workspace, epoch, catalogue) = {
                let state = state.borrow();
                let workspace = match super::workflow::captured_workspace(&state) {
                    Ok(workspace) if workspace.legacy_content() == Some(&source.content) => workspace,
                    Ok(_) => {
                        feedback.set_text("The source changed. Reopen Included Files to review the current draft.");
                        return;
                    }
                    Err(error) => { feedback.set_text(&error); return; }
                };
                (workspace, state.generation, state.catalogue.clone())
            };
            let (source, state, status, feedback, weak) =
                (source.clone(), state.clone(), status.clone(), feedback.clone(), weak.clone());
            let Some(editor_panel) = panel.upgrade() else { return };
            editor_panel.set_sensitive(false);
            super::workflow::background(move || {
                let output = match worker::run(worker::Request::EditLegacyIncludes {
                    workspace: Box::new(workspace), includes: pending,
                })? {
                    worker::Response::Copy(output) => output,
                    _ => return Err("Unexpected response preparing included files.".into()),
                };
                let (before, _) = codec::inspect(&source.content)?;
                let (after, _) = codec::inspect(&output)?;
                let impact = format!("Include directives: {} → {}\n\nThe editor keeps these paths as source text. Linked file contents are unavailable in this review.\n\nSupported shortcut operations after rebuilding: ", before.len(), after.len());
                let mut doc = Document::new_with_binding_mode(output, &catalogue, source.bindings.draft.local_variables)?;
                let original = source.original.clone().unwrap_or(source);
                if doc.content != original.content {
                    doc.original = Some(original);
                }
                let impact = format!("{impact}{}\n", doc.bindings.draft.entries.len());
                Ok((Arc::new(doc), impact))
            }, move |result| {
                editor_panel.set_sensitive(true);
                let Some(parent) = weak.upgrade().filter(gtk::prelude::WidgetExt::is_visible) else { return };
                if state.borrow().generation != epoch {
                    feedback.set_text("The draft changed. Prepare the include review again.");
                    return;
                }
                match result {
                    Ok((doc, impact)) => super::legacy_variables::review_proposal(&parent, &state, &status, doc, epoch, Some(impact)),
                    Err(error) => feedback.set_text(&error),
                }
            });
        });
    }
    let weak = win.downgrade();
    win.connect_close_request(move |_| {
        if flush().is_ok() && *rows.borrow() == initial {
            return gtk::glib::Propagation::Proceed;
        }
        if let Some(parent) = weak.upgrade() {
            let dialog = adw::MessageDialog::builder()
                .transient_for(&parent)
                .heading("Discard pending include changes?")
                .build();
            dialog.add_responses(&[("keep", "Keep Editing"), ("discard", "Discard Changes")]);
            dialog.set_default_response(Some("keep"));
            dialog.set_close_response("keep");
            let weak = parent.downgrade();
            dialog.connect_response(None, move |_, response| {
                if response == "discard" {
                    if let Some(parent) = weak.upgrade() {
                        parent.destroy();
                    }
                }
            });
            dialog.present();
        }
        gtk::glib::Propagation::Stop
    });
    win.present();
    Ok(win)
}
