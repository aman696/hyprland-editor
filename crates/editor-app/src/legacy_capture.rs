//! User-selected file captures for native include references.
use super::{capture_worker, workflow, Shared, State};
use adw::prelude::*;
use hyprland_editor_core::{
    legacy_includes::captures::{self, LinkedRevision, Outcome, References},
    source::{self, CapturedSource},
    BASELINE,
};
use std::{
    cell::{Cell, RefCell},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

pub(super) struct Snapshot {
    pub files: Vec<CapturedSource>,
    pub revisions: Vec<LinkedRevision>,
    pub fingerprint: String,
    pub retained_cost: usize,
}

impl Snapshot {
    pub fn new(
        files: Vec<CapturedSource>,
        revisions: Vec<LinkedRevision>,
    ) -> Result<Option<Arc<Self>>, String> {
        captures::validate_revisions(&files, &revisions)?;
        if files.is_empty() {
            return Ok(None);
        }
        captures::validate(&files)?;
        let identities: Vec<_> = files.iter().map(|file| &file.identity).collect();
        let revisions_by_path: Vec<_> = revisions
            .iter()
            .map(|r| {
                (
                    &r.path,
                    hyprland_editor_core::lua::source_hash(&r.revision.content),
                )
            })
            .collect();
        let identity_text =
            serde_json::to_string(&(identities, revisions_by_path)).map_err(|e| e.to_string())?;
        let mut retained_cost = 0usize;
        for file in &files {
            let identity = serde_json::to_vec(&file.identity).map_err(|e| e.to_string())?;
            retained_cost = retained_cost.saturating_add(
                std::mem::size_of::<CapturedSource>()
                    + file.content.capacity()
                    + identity.len() * 4,
            );
        }
        for revision in &revisions {
            retained_cost = retained_cost.saturating_add(
                std::mem::size_of::<LinkedRevision>()
                    + revision.path.capacity()
                    + revision.revision.content.capacity()
                    + 192,
            );
        }
        Ok(Some(Arc::new(Self {
            files,
            revisions,
            fingerprint: hyprland_editor_core::lua::source_hash(&identity_text),
            retained_cost,
        })))
    }
}

fn replace(
    state: &mut State,
    files: Vec<CapturedSource>,
    revisions: Vec<LinkedRevision>,
) -> Result<(), String> {
    let proposed = Snapshot::new(files, revisions)?;
    let fingerprint = |snapshot: &Arc<Snapshot>| snapshot.fingerprint.clone();
    if proposed.as_ref().map(fingerprint) == state.legacy_include_files.as_ref().map(fingerprint) {
        return Ok(());
    }
    let mut workspace = workflow::captured_workspace(state)?;
    workspace.legacy_include_files = proposed.as_ref().map_or_else(Vec::new, |s| s.files.clone());
    workspace.legacy_include_revisions = proposed
        .as_ref()
        .map_or_else(Vec::new, |s| s.revisions.clone());
    workspace.to_json(&state.catalogue)?;
    state
        .undo
        .legacy_include_files(state.legacy_include_files.clone())?;
    state.legacy_include_files = proposed;
    state.generation += 1;
    Ok(())
}

fn inspect(state: &State) -> Result<References, String> {
    let version = &state.draft.version;
    let doc = state
        .legacy_sources
        .get(version)
        .ok_or("Open a native source project first.")?;
    let origin = state
        .source_origins
        .get(version)
        .ok_or("The project has no captured root file path.")?;
    let files = state
        .legacy_include_files
        .as_ref()
        .map_or(&[][..], |s| s.files.as_slice());
    let revisions = state
        .legacy_include_files
        .as_ref()
        .map_or(&[][..], |s| s.revisions.as_slice());
    captures::inspect(&origin.selected_path, &doc.content, files, revisions)
}

#[derive(Clone, Debug)]
struct Row {
    label: String,
    expected: Option<PathBuf>,
    captured: Option<usize>,
}

fn rows(state: &State) -> Result<(Vec<Row>, String), String> {
    let report = inspect(state)?;
    let files = state
        .legacy_include_files
        .as_ref()
        .map_or(&[][..], |s| s.files.as_slice());
    let root = state.source_origins[&state.draft.version].display_path();
    let mut rows = Vec::new();
    for reference in report.rows {
        let location = reference
            .parent
            .map_or_else(|| root.clone(), |i| files[i].identity.display_path());
        let (status, captured, selectable) = match reference.outcome {
            Outcome::Captured(i) => ("Captured", Some(i), true),
            Outcome::NeedsSelection => ("Choose file", None, true),
            Outcome::RootCycle | Outcome::Cycle(_) => ("Cycle", None, false),
            Outcome::UnresolvedTemplate => ("Unresolved path", None, false),
            Outcome::DepthLimit => ("Depth limit", None, false),
        };
        rows.push(Row {
            label: format!(
                "{status}  ·  {}  ·  {location}:{}",
                reference.template, reference.line
            ),
            expected: reference.expected.filter(|_| selectable),
            captured,
        });
    }
    for (index, file) in files.iter().enumerate() {
        if !report.reached.contains(&index) {
            rows.push(Row {
                label: format!("Retained snapshot  ·  {}", file.identity.display_path()),
                expected: None,
                captured: Some(index),
            });
        }
    }
    let mut details = format!(
        "{} of {} snapshots linked to this root.",
        report.reached.len(),
        files.len()
    );
    if report.opaque_directives != 0 {
        details.push_str(&format!(
            " {} directives have unsupported context or syntax.",
            report.opaque_directives
        ));
    }
    if report.truncated {
        details.push_str(" The reference list reached its display limit.");
    }
    Ok((rows, details))
}

pub(super) fn open(
    parent: &adw::Window,
    state: &Shared,
    status: &gtk::Label,
) -> Result<adw::Window, String> {
    let (initial, count) = rows(&state.borrow())?;
    let window = adw::Window::builder()
        .title("Linked File Snapshots")
        .transient_for(parent)
        .modal(true)
        .default_width(660)
        .default_height(600)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for set in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        set(body.upcast_ref(), 16);
    }
    body.append(&super::label(
        "Choose a linked file to keep a private snapshot in this project.",
    ));
    let count_label = super::label(&count);
    body.append(&count_label);
    let model = gtk::StringList::new(&[]);
    let selected = gtk::DropDown::builder().model(&model).hexpand(true).build();
    super::monitors::compact(&selected);
    selected.set_widget_name("include-capture-picker");
    body.append(&selected);
    let expected_label = super::label("");
    expected_label.set_widget_name("include-capture-path");
    body.append(&expected_label);
    let actions = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(2)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    let capture = gtk::Button::with_label("Choose Linked File…");
    capture.add_css_class("suggested-action");
    let forget = gtk::Button::with_label("Remove Snapshot");
    let edit = gtk::Button::with_label("Edit Linked File…");
    let export = gtk::Button::with_label("Export Linked File Copy…");
    let restore = gtk::Button::with_label("Restore Captured Version");
    actions.insert(&capture, -1);
    actions.insert(&edit, -1);
    actions.insert(&export, -1);
    actions.insert(&restore, -1);
    actions.insert(&forget, -1);
    body.append(&actions);
    let export_tree = gtk::Button::with_label("Review Entire Configuration Folder…");
    let trace = gtk::Button::with_label("Trace Source Settings…");
    let project_actions = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(2)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    project_actions.insert(&trace, -1);
    project_actions.insert(&export_tree, -1);
    body.append(&project_actions);
    let feedback = super::label("");
    feedback.set_widget_name("include-capture-feedback");
    body.append(&feedback);
    super::legacy_tree_export::connect(&export_tree, &window, state, &feedback);
    {
        let (parent, state, status) = (window.downgrade(), state.clone(), status.clone());
        trace.connect_clicked(move |_| {
            if let Some(parent) = parent.upgrade() {
                super::legacy_trace::open(&parent, &state, &status, None);
            }
        });
    }
    let preview = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::Char)
        .build();
    preview.set_widget_name("include-capture-preview");
    body.append(
        &gtk::ScrolledWindow::builder()
            .child(&preview)
            .vexpand(true)
            .build(),
    );
    root.append(&body);
    window.set_content(Some(&root));

    let catalogue = Rc::new(RefCell::new(initial));
    let row_generation = Rc::new(Cell::new(state.borrow().generation));
    let update_selection = {
        let (catalogue, state, selected, expected_label, preview, capture, forget) = (
            catalogue.clone(),
            state.clone(),
            selected.downgrade(),
            expected_label,
            preview,
            capture.downgrade(),
            forget.downgrade(),
        );
        let (edit, export, restore) = (edit.downgrade(), export.downgrade(), restore.downgrade());
        move || {
            let Some(selected) = selected.upgrade() else {
                return;
            };
            let catalogue = catalogue.borrow();
            let row = catalogue.get(selected.selected() as usize);
            let state = state.borrow();
            let file = row
                .and_then(|r| r.captured)
                .and_then(|i| state.legacy_include_files.as_ref()?.files.get(i));
            let text = file.map_or("", |f| {
                captures::effective_text(f, &state.legacy_include_files.as_ref().unwrap().revisions)
            });
            preview.buffer().set_text(text);
            let expected = row.and_then(|r| r.expected.as_ref());
            expected_label.set_text(&expected.map_or_else(
                || {
                    if row.is_some() {
                        "This entry has no selectable literal path.".into()
                    } else {
                        "No include directives in the kept project source.".into()
                    }
                },
                |p| source::display_path_bytes(p.as_os_str().as_bytes()),
            ));
            if let Some(capture) = capture.upgrade() {
                capture.set_sensitive(expected.is_some());
            }
            if let Some(edit) = edit.upgrade() {
                edit.set_sensitive(file.is_some());
            }
            if let Some(export) = export.upgrade() {
                export.set_sensitive(file.is_some());
            }
            if let Some(restore) = restore.upgrade() {
                restore.set_sensitive(file.is_some_and(|file| {
                    state
                        .legacy_include_files
                        .as_ref()
                        .unwrap()
                        .revisions
                        .iter()
                        .any(|r| r.path == file.identity.selected_path)
                }));
            }
            if let Some(forget) = forget.upgrade() {
                forget.set_sensitive(file.is_some());
            }
        }
    };
    {
        let update = update_selection.clone();
        selected.connect_selected_notify(move |_| update());
    }
    let refresh = Rc::new({
        let row_generation = row_generation.clone();
        let (catalogue, state, selected, count_label) = (
            catalogue.clone(),
            state.clone(),
            selected.downgrade(),
            count_label.clone(),
        );
        move || {
            let Some(selected) = selected.upgrade() else {
                return;
            };
            if let Ok((next, count)) = rows(&state.borrow()) {
                row_generation.set(state.borrow().generation);
                *catalogue.borrow_mut() = next;
                let index = selected.selected();
                let list = catalogue
                    .borrow()
                    .iter()
                    .map(|row| row.label.clone())
                    .collect::<Vec<_>>();
                let labels = list.iter().map(String::as_str).collect::<Vec<_>>();
                model.splice(0, model.n_items(), &labels);
                selected.set_selected(if labels.is_empty() {
                    gtk::INVALID_LIST_POSITION
                } else if index == gtk::INVALID_LIST_POSITION {
                    0
                } else {
                    index.min(labels.len() as u32 - 1)
                });
                count_label.set_text(&count);
            }
            update_selection();
        }
    });

    {
        let (window, catalogue, state, status, selected, refresh, feedback, body) = (
            window.downgrade(),
            catalogue.clone(),
            state.clone(),
            status.clone(),
            selected.downgrade(),
            refresh.clone(),
            feedback.clone(),
            body.downgrade(),
        );
        let row_generation = row_generation.clone();
        capture.connect_clicked(move |_| {
            let (Some(window), Some(selected)) = (window.upgrade(), selected.upgrade()) else {
                return;
            };
            let Some(expected) = catalogue
                .borrow()
                .get(selected.selected() as usize)
                .and_then(|row| row.expected.clone())
            else {
                return;
            };
            let generation = state.borrow().generation;
            if generation != row_generation.get() {
                refresh();
                feedback.set_text("The project changed. Choose an include again.");
                return;
            }
            let chooser = gtk::FileDialog::builder()
                .title("Choose the linked file named in the include")
                .build();
            let (window_ref, state, status, refresh, feedback, body) = (
                window.downgrade(),
                state.clone(),
                status.clone(),
                refresh.clone(),
                feedback.clone(),
                body.clone(),
            );
            chooser.open(
                Some(&window),
                None::<&gtk::gio::Cancellable>,
                move |result| {
                    let Ok(file) = result else { return };
                    let Some(path) = file
                        .path()
                        .filter(|p| captures::matches_selection(&expected, p))
                    else {
                        feedback.set_text("Choose the local file whose path matches this include.");
                        return;
                    };
                    if state.borrow().generation != generation {
                        feedback.set_text("The project changed. Reopen Linked File Snapshots.");
                        return;
                    }
                    let Some(body) = body.upgrade() else { return };
                    body.set_sensitive(false);
                    feedback.set_text("Capturing the selected file…");
                    workflow::background(
                        move || capture_worker::read_selected(&path),
                        move |result| {
                            body.set_sensitive(true);
                            if window_ref.upgrade().is_none_or(|w| !w.is_visible()) {
                                return;
                            }
                            if state.borrow().generation != generation {
                                feedback.set_text(
                                    "The project changed during capture. Choose the file again.",
                                );
                                return;
                            }
                            match result
                                .and_then(|file| accept(&mut state.borrow_mut(), &expected, file))
                            {
                                Ok(()) => {
                                    feedback.set_text("Snapshot kept in the private draft.");
                                    super::summary(&state, &status);
                                    refresh();
                                }
                                Err(error) => feedback.set_text(&error),
                            }
                        },
                    );
                },
            );
        });
    }
    {
        let (catalogue, state, status, selected, refresh, feedback) = (
            catalogue.clone(),
            state.clone(),
            status.clone(),
            selected.downgrade(),
            refresh.clone(),
            feedback.clone(),
        );
        let row_generation = row_generation.clone();
        forget.connect_clicked(move |_| {
            if state.borrow().generation != row_generation.get() {
                refresh();
                feedback.set_text("The project changed. Choose a snapshot again.");
                return;
            }
            let Some(selected) = selected.upgrade() else {
                return;
            };
            let Some(index) = catalogue
                .borrow()
                .get(selected.selected() as usize)
                .and_then(|r| r.captured)
            else {
                return;
            };
            let mut files = current_files(&state.borrow());
            if index >= files.len() {
                return;
            }
            let revisions = current_revisions(&state.borrow());
            if revisions
                .iter()
                .any(|r| r.path == files[index].identity.selected_path)
            {
                feedback.set_text("Restore the captured version before removing this snapshot.");
                return;
            }
            files.remove(index);
            let result = replace(&mut state.borrow_mut(), files, revisions);
            match result {
                Ok(()) => {
                    feedback.set_text("Snapshot removed from the private draft.");
                    super::summary(&state, &status);
                    refresh();
                }
                Err(error) => feedback.set_text(&error),
            }
        });
    }
    let selected_file = {
        let (selected, catalogue, state, generation) = (
            selected.downgrade(),
            catalogue,
            state.clone(),
            row_generation,
        );
        move || -> Option<(CapturedSource, String)> {
            let selected = selected.upgrade()?;
            let state = state.borrow();
            if generation.get() != state.generation {
                return None;
            }
            let index = catalogue
                .borrow()
                .get(selected.selected() as usize)?
                .captured?;
            let snapshots = state.legacy_include_files.as_ref()?;
            let file = snapshots.files.get(index)?;
            Some((
                file.clone(),
                captures::effective_text(file, &snapshots.revisions).to_owned(),
            ))
        }
    };
    {
        let (selected_file, window, state, status, refresh, feedback) = (
            selected_file.clone(),
            window.downgrade(),
            state.clone(),
            status.clone(),
            refresh.clone(),
            feedback.clone(),
        );
        edit.connect_clicked(move |button| {
            let Some((file, source)) = selected_file() else {
                return;
            };
            let (generation, settings) = {
                let state = state.borrow();
                (state.generation, state.catalogue.clone())
            };
            button.set_sensitive(false);
            feedback.set_text("Preparing the linked file draft…");
            let (button, window, state, status, refresh, feedback) = (
                button.downgrade(),
                window.clone(),
                state.clone(),
                status.clone(),
                refresh.clone(),
                feedback.clone(),
            );
            workflow::background(
                move || super::legacy_file_editor::load(file, source, &settings),
                move |result| {
                    if let Some(button) = button.upgrade() {
                        button.set_sensitive(true);
                    }
                    let Some(window) = window.upgrade().filter(gtk::prelude::WidgetExt::is_visible)
                    else {
                        return;
                    };
                    if generation != state.borrow().generation {
                        refresh();
                        feedback.set_text("The project changed. Select the file again.");
                        return;
                    }
                    if let Err(error) = result.and_then(|prepared| {
                        super::legacy_file_editor::open(
                            &window, &state, &status, prepared, refresh, None,
                        )
                    }) {
                        feedback.set_text(&error);
                    } else {
                        feedback.set_text("");
                    }
                },
            );
        });
    }
    {
        let (selected_file, window) = (selected_file.clone(), window.downgrade());
        export.connect_clicked(move |_| {
            if let (Some(window), Some((_, source))) = (window.upgrade(), selected_file()) {
                super::text_export::review_linked_file(&window, source);
            }
        });
    }
    {
        let (state, status, refresh, feedback) = (
            state.clone(),
            status.clone(),
            refresh.clone(),
            feedback.clone(),
        );
        restore.connect_clicked(move |_| {
            let Some((file, _)) = selected_file() else {
                return;
            };
            let result = keep_revision(
                &mut state.borrow_mut(),
                &file.identity.selected_path,
                file.content,
            );
            if let Err(error) = result {
                feedback.set_text(&error);
            } else {
                feedback.set_text("Captured version restored. Undo can recover your edits.");
                super::summary(&state, &status);
                refresh();
            }
        });
    }
    refresh();
    window.present();
    Ok(window)
}

fn current_files(state: &State) -> Vec<CapturedSource> {
    state
        .legacy_include_files
        .as_ref()
        .map_or_else(Vec::new, |s| s.files.clone())
}

pub(super) fn accept(
    state: &mut State,
    expected: &std::path::Path,
    file: CapturedSource,
) -> Result<(), String> {
    use std::{ffi::OsStr, path::Path};
    let selected = Path::new(OsStr::from_bytes(&file.identity.selected_path));
    if !captures::matches_selection(expected, selected) {
        return Err("The captured file does not match the include path.".into());
    }
    if state.draft.version != BASELINE
        || !inspect(state)?.rows.iter().any(|r| {
            r.expected.as_deref() == Some(expected)
                && matches!(r.outcome, Outcome::NeedsSelection | Outcome::Captured(_))
        })
    {
        return Err("The include no longer refers to this file.".into());
    }
    let mut files = current_files(state);
    if let Some(index) = files.iter().position(|existing| {
        captures::matches_selection(
            expected,
            Path::new(OsStr::from_bytes(&existing.identity.selected_path)),
        )
    }) {
        if files[index].identity.sha256 == file.identity.sha256 {
            return Ok(());
        }
        if current_revisions(state)
            .iter()
            .any(|r| r.path == files[index].identity.selected_path)
        {
            return Err(
                "Restore the captured version before replacing the edited snapshot.".into(),
            );
        }
        files[index] = file;
    } else {
        files.push(file);
    }
    let revisions = current_revisions(state);
    replace(state, files, revisions)
}

fn current_revisions(state: &State) -> Vec<LinkedRevision> {
    state
        .legacy_include_files
        .as_ref()
        .map_or_else(Vec::new, |s| s.revisions.clone())
}

pub(super) fn keep_revision(state: &mut State, path: &[u8], source: String) -> Result<(), String> {
    if state.draft.version != BASELINE {
        return Err("Open a native source project first.".into());
    }
    let files = current_files(state);
    let file = files
        .iter()
        .find(|f| f.identity.selected_path == path)
        .ok_or("The linked file snapshot has been removed.")?;
    let mut revisions = current_revisions(state);
    revisions.retain(|entry| entry.path != path);
    if file.content != source {
        revisions.push(LinkedRevision {
            path: path.to_vec(),
            revision: hyprland_editor_core::legacy_variables::Revision::new(&file.content, source)?,
        });
    }
    revisions.sort_by(|a, b| a.path.cmp(&b.path));
    replace(state, files, revisions)
}
