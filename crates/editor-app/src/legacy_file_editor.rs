//! One native draft scoped to a captured include, using the regular editors.
use super::{legacy_source::Document, project_save, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::{source::CapturedSource, Setting, BASELINE};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(super) struct Prepared {
    file: CapturedSource,
    doc: Document,
}

pub(super) fn load(
    file: CapturedSource,
    source: String,
    catalogue: &[Setting],
) -> Result<Prepared, String> {
    let mut doc = Document::new(source, catalogue)?;
    if doc.content != file.content {
        doc.original = Some(Arc::new(Document::new(file.content.clone(), catalogue)?));
    }
    Ok(Prepared { file, doc })
}

fn draft(prepared: Prepared) -> Result<Shared, String> {
    let state = super::initial_state();
    {
        let mut child = state.borrow_mut();
        let doc = prepared.doc;
        child.draft = doc.report.draft.clone();
        child
            .source_origins
            .insert(BASELINE.into(), prepared.file.identity);
        child
            .monitors
            .insert(BASELINE.into(), doc.monitors.rules.clone());
        child
            .devices
            .insert(BASELINE.into(), doc.devices.devices.clone());
        child.bindings.insert(
            BASELINE.into(),
            hyprland_editor_core::binding::Draft {
                legacy: Some(doc.bindings.draft.clone()),
                ..Default::default()
            },
        );
        child
            .workspace_rules
            .insert(BASELINE.into(), doc.workspaces.rules.clone());
        child.rules.insert(BASELINE.into(), doc.rules.rules.clone());
        child
            .animations
            .insert(BASELINE.into(), doc.animations.draft.clone());
        child
            .gestures
            .insert(BASELINE.into(), doc.gestures.draft.clone());
        child
            .session
            .insert(BASELINE.into(), doc.session.draft.clone());
        child.legacy_sources.insert(BASELINE.into(), Arc::new(doc));
    }
    let saved = project_save::Snapshot::capture(&state.borrow())?;
    saved.mark_saved(&state);
    Ok(state)
}

pub(super) fn open(
    parent: &adw::Window,
    project: &Shared,
    status: &gtk::Label,
    prepared: Prepared,
    refresh: Rc<dyn Fn()>,
    initial_setting: Option<&str>,
) -> Result<adw::Window, String> {
    let original_path = prepared.file.identity.selected_path.clone();
    let file_name = prepared.file.identity.display_path();
    let entry_generation = project.borrow().generation;
    let before = prepared.doc.content.clone();
    let child = draft(prepared)?;
    let window = adw::Window::builder()
        .title("Edit Linked File")
        .transient_for(parent)
        .modal(true)
        .default_width(690)
        .default_height(680)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(16);
    body.set_margin_end(16);
    body.append(&super::label(&file_name));
    let local_status = super::label("Edits in this window belong to the selected file.");
    child.borrow_mut().main_status.set(Some(&local_status));
    body.append(&local_status);
    let groups = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(4)
        .column_spacing(6)
        .row_spacing(6)
        .build();
    body.append(&groups);
    connect_groups(&groups, &window, &child, &local_status);

    let search = gtk::SearchEntry::builder()
        .placeholder_text("Find a setting")
        .build();
    search.set_widget_name("linked-setting-search");
    body.append(&search);
    let model = gtk::StringList::new(&[]);
    let selected = gtk::DropDown::builder().model(&model).hexpand(true).build();
    super::monitors::compact(&selected);
    selected.set_widget_name("linked-setting-picker");
    body.append(&selected);
    let setting_panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    body.append(&setting_panel);
    let feedback = super::label("");
    body.append(&feedback);
    let undo = gtk::Button::with_label("Undo File Edit");
    body.append(&undo);
    let review = gtk::Button::with_label("Review File Changes");
    review.add_css_class("suggested-action");
    body.append(&review);
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&body)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    window.set_content(Some(&root));

    let available = Rc::new(
        child
            .borrow()
            .catalogue
            .iter()
            .filter(|s| s.version == BASELINE)
            .map(|s| (s.clone(), super::human_name(&s.id)))
            .collect::<Vec<_>>(),
    );
    let visible = Rc::new(RefCell::new(Vec::<usize>::new()));
    let show_setting = {
        let (visible, available, child, setting_panel, local_status) = (
            visible.clone(),
            available.clone(),
            child.clone(),
            setting_panel.clone(),
            local_status.clone(),
        );
        move |selected: u32| {
            if let Some(index) = visible.borrow().get(selected as usize) {
                super::detail(&child, &setting_panel, &available[*index].0, &local_status);
            } else {
                while let Some(w) = setting_panel.first_child() {
                    setting_panel.remove(&w);
                }
                setting_panel.append(&super::label("No matching settings."));
            }
        }
    };
    {
        let show_setting = show_setting.clone();
        selected.connect_selected_notify(move |picker| show_setting(picker.selected()));
    }
    let search_settings = {
        let (selected, show_setting, visible) =
            (selected.downgrade(), show_setting.clone(), visible.clone());
        move |query: &str| {
            let Some(selected) = selected.upgrade() else {
                return;
            };
            let query = query.to_lowercase();
            let matches: Vec<_> = available
                .iter()
                .enumerate()
                .filter(|(_, (setting, name))| {
                    setting.id.contains(&query) || name.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect();
            let labels: Vec<_> = matches.iter().map(|&i| available[i].1.as_str()).collect();
            *visible.borrow_mut() = matches.clone();
            model.splice(0, model.n_items(), &labels);
            selected.set_selected(if matches.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                0
            });
            show_setting(selected.selected());
        }
    };
    if let Some(id) = initial_setting {
        search.set_text(id);
    }
    search_settings(search.text().as_str());
    search.connect_search_changed(move |entry| search_settings(entry.text().as_str()));
    {
        let (child, local_status, selected) =
            (child.clone(), local_status.clone(), selected.downgrade());
        undo.connect_clicked(move |_| {
            child.borrow_mut().undo_last();
            super::summary(&child, &local_status);
            if let Some(selected) = selected.upgrade() {
                show_setting(selected.selected());
            }
        });
    }
    window.connect_close_request({
        let (child, window) = (child.clone(), window.downgrade());
        move |_| {
            if child.borrow().dirty() {
                if let Some(window) = window.upgrade() {
                    let dialog = adw::MessageDialog::new(
                        Some(&window),
                        Some("Discard changes to this file?"),
                        None,
                    );
                    dialog.add_responses(&[("cancel", "Continue Editing"), ("discard", "Discard")]);
                    dialog.set_close_response("cancel");
                    dialog.set_default_response(Some("cancel"));
                    let weak = window.downgrade();
                    dialog.connect_response(Some("discard"), move |_, _| {
                        if let Some(window) = weak.upgrade() {
                            window.destroy();
                        }
                    });
                    dialog.present();
                }
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        }
    });
    {
        let (window, body, project, status) = (
            window.downgrade(),
            body.downgrade(),
            project.clone(),
            status.clone(),
        );
        review.connect_clicked(move |_| {
            if entry_generation != project.borrow().generation {
                feedback
                    .set_text("The project changed. Reopen this file to edit its current version.");
                return;
            }
            let (doc, workspace, generation) = {
                let child = child.borrow();
                let workspace = match workflow::captured_workspace(&child) {
                    Ok(value) => value,
                    Err(error) => {
                        feedback.set_text(&error);
                        return;
                    }
                };
                (
                    child.legacy_sources[BASELINE].clone(),
                    workspace,
                    child.generation,
                )
            };
            let Some(body) = body.upgrade() else { return };
            body.set_sensitive(false);
            feedback.set_text("Preparing the reviewed copy…");
            let (window, feedback, project, status, refresh, before, path, child) = (
                window.clone(),
                feedback.clone(),
                project.clone(),
                status.clone(),
                refresh.clone(),
                before.clone(),
                original_path.clone(),
                child.clone(),
            );
            workflow::background(
                move || doc.edited_copy(&workspace),
                move |result| {
                    body.set_sensitive(true);
                    let Some(window) = window.upgrade().filter(gtk::prelude::WidgetExt::is_visible)
                    else {
                        return;
                    };
                    match result {
                        Ok(source) if source == before => {
                            feedback.set_text("No file changes to keep.")
                        }
                        Ok(source) => {
                            feedback.set_text("");
                            show_review(&window, &before, source, move |source| {
                                if project.borrow().generation != entry_generation
                                    || child.borrow().generation != generation
                                {
                                    return Err(
                                        "The draft changed. Review the current file again.".into(),
                                    );
                                }
                                super::legacy_capture::keep_revision(
                                    &mut project.borrow_mut(),
                                    &path,
                                    source.to_owned(),
                                )?;
                                super::summary(&project, &status);
                                refresh();
                                Ok(())
                            });
                        }
                        Err(error) => feedback.set_text(&error),
                    }
                },
            );
        });
    }
    window.present();
    Ok(window)
}

fn connect_groups(
    groups: &gtk::FlowBox,
    parent: &adw::Window,
    child: &Shared,
    status: &gtk::Label,
) {
    for (index, name) in [
        "Monitors",
        "Devices",
        "Bindings",
        "Workspaces",
        "Rules",
        "Animations",
        "Gestures",
        "Session",
        "Variables",
        "Included Files",
    ]
    .iter()
    .enumerate()
    {
        let button = gtk::Button::with_label(name);
        groups.insert(&button, -1);
        let (parent, child, status) = (parent.downgrade(), child.clone(), status.clone());
        button.connect_clicked(move |_| {
            let Some(parent) = parent.upgrade() else {
                return;
            };
            match index {
                0 => {
                    super::monitors::show(&parent, &child, &status);
                }
                1 => {
                    super::devices::show(&parent, &child, &status, None);
                }
                2 => {
                    super::bindings::show(&parent, &child, &status);
                }
                3 => {
                    super::workspaces::show(&parent, &child, &status);
                }
                4 => {
                    super::rules::show(&parent, &child, &status);
                }
                5 => {
                    super::animations::show(&parent, &child, &status);
                }
                6 => {
                    super::gestures::show(&parent, &child, &status);
                }
                7 => {
                    super::session_editor::show(&parent, &child, &status);
                }
                8 => {
                    super::legacy_variables::show(&parent, &child, &status);
                }
                _ => {
                    if let Err(error) =
                        super::legacy_includes::open_for_file(&parent, &child, &status, false)
                    {
                        super::notice(&parent, "Cannot edit include directives", &error);
                    }
                }
            }
        });
    }
}

fn show_review(
    parent: &adw::Window,
    before: &str,
    after: String,
    keep: impl Fn(&str) -> Result<(), String> + 'static,
) {
    let review = adw::Window::builder()
        .title("Review Linked File Changes")
        .transient_for(parent)
        .modal(true)
        .default_width(700)
        .default_height(600)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(16);
    body.set_margin_end(16);
    let tabs = gtk::Notebook::new();
    tabs.set_vexpand(true);
    for (title, text) in [("Before", before), ("Proposed", after.as_str())] {
        let view = gtk::TextView::builder()
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::Char)
            .build();
        view.buffer().set_text(text);
        let scroll = gtk::ScrolledWindow::builder().child(&view).build();
        tabs.append_page(&scroll, Some(&gtk::Label::new(Some(title))));
    }
    tabs.set_current_page(Some(1));
    body.append(&tabs);
    let feedback = super::label("");
    body.append(&feedback);
    let action = gtk::Button::with_label("Keep Linked File Changes");
    action.add_css_class("suggested-action");
    body.append(&action);
    let (review_ref, parent) = (review.downgrade(), parent.downgrade());
    action.connect_clicked(move |_| match keep(&after) {
        Ok(()) => {
            if let Some(review) = review_ref.upgrade() {
                review.destroy();
            }
            if let Some(parent) = parent.upgrade() {
                parent.destroy();
            }
        }
        Err(error) => feedback.set_text(&error),
    });
    root.append(&body);
    review.set_content(Some(&root));
    review.present();
}
