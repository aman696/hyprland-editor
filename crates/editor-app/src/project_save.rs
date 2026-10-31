//! A named, complete private workspace copy chosen through the native chooser.
use super::{notice, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::workspace::Workspace;

pub(super) struct Snapshot {
    generation: u64,
    graph: Option<String>,
    pub workspace: Workspace,
}

impl Snapshot {
    pub fn capture(state: &super::State) -> Result<Self, String> {
        Ok(Self {
            generation: state.generation,
            graph: state
                .source_graphs
                .get(&state.draft.version)
                .map(|graph| graph.fingerprint.clone()),
            workspace: workflow::captured_workspace(state)?,
        })
    }

    // A completed older copy cannot clear edits made while choosing or writing.
    pub fn mark_saved(self, state: &Shared) -> bool {
        let mut state = state.borrow_mut();
        if state.generation != self.generation {
            return false;
        }
        let origin = self.workspace.root().map(|root| root.origin.clone());
        let workspace = self.workspace;
        let version = workspace.draft.version;
        state.saved.insert(version.clone(), workspace.draft.values);
        state
            .saved_monitors
            .insert(version.clone(), workspace.monitors);
        state
            .saved_devices
            .insert(version.clone(), workspace.devices);
        state
            .saved_bindings
            .insert(version.clone(), workspace.bindings);
        state
            .saved_workspace_rules
            .insert(version.clone(), workspace.workspace_rules);
        state.saved_rules.insert(version.clone(), workspace.rules);
        state
            .saved_animations
            .insert(version.clone(), workspace.animations);
        state
            .saved_gestures
            .insert(version.clone(), workspace.gestures);
        state
            .saved_session
            .insert(version.clone(), workspace.session);
        state.saved_source_origins.insert(version.clone(), origin);
        state.saved_source_graphs.insert(version, self.graph);
        true
    }
}

pub(super) fn connect(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) {
    let window = window.downgrade();
    let state = state.clone();
    let status = status.clone();
    button.connect_clicked(move |button| {
        let Some(window) = window.upgrade() else {
            return;
        };
        let pending = match Snapshot::capture(&state.borrow()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                notice(&window, "Cannot save project", &error);
                return;
            }
        };
        let chooser = gtk::FileDialog::builder()
            .title("Save Project As")
            .initial_name("hyprland-project.hye")
            .accept_label("Save Project")
            .build();
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Hyprland Editor project (*.hye)"));
        filter.add_pattern("*.hye");
        let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        chooser.set_filters(Some(&filters));
        chooser.set_default_filter(Some(&filter));
        button.set_sensitive(false);
        let button = button.downgrade();
        let parent = window.downgrade();
        let state = state.clone();
        let status = status.clone();
        chooser.save(
            Some(&window),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                let Some(button) = button.upgrade() else {
                    return;
                };
                let Some(window) = parent.upgrade().filter(|window| window.is_visible()) else {
                    button.set_sensitive(true);
                    return;
                };
                match result {
                    Ok(file) => write_to_file(file, pending, &window, &button, &state, &status),
                    Err(error) => {
                        button.set_sensitive(true);
                        if !error.matches(gtk::DialogError::Dismissed)
                            && !error.matches(gtk::DialogError::Cancelled)
                        {
                            notice(&window, "Could not choose project file", error.message());
                        }
                    }
                }
            },
        );
    });
}

fn write_to_file(
    file: gtk::gio::File,
    pending: Snapshot,
    window: &adw::ApplicationWindow,
    button: &gtk::Button,
    state: &Shared,
    status: &gtk::Label,
) {
    let catalogue = state.borrow().catalogue.clone();
    let state = state.clone();
    let window = window.downgrade();
    button.set_sensitive(false);
    let button = button.downgrade();
    let status = status.clone();
    status.set_text("Saving the complete project copy…");
    workflow::background(
        move || {
            let bytes = pending.workspace.to_json(&catalogue)?;
            Ok((pending, bytes))
        },
        move |result| {
            let Some(window) = window.upgrade().filter(|window| window.is_visible()) else {
                if let Some(button) = button.upgrade() {
                    button.set_sensitive(true);
                }
                return;
            };
            let (pending, bytes) = match result {
                Ok(result) => result,
                Err(error) => {
                    if let Some(button) = button.upgrade() {
                        button.set_sensitive(true);
                    }
                    super::summary(&state, &status);
                    notice(&window, "Could not save project", &error);
                    return;
                }
            };
            gtk::glib::MainContext::default().spawn_local(async move {
                let result = super::text_export::save_project_new(&file, bytes).await;
                if let Some(button) = button.upgrade() {
                    button.set_sensitive(true);
                }
                match result {
                    Ok(()) => {
                        let current = pending.mark_saved(&state);
                        status.set_text(&format!(
                            "{} {}",
                            if current {
                                "Project copy saved:"
                            } else {
                                "Earlier project revision saved; current edits remain unsaved:"
                            },
                            file.parse_name()
                        ));
                    }
                    Err(error) => {
                        super::summary(&state, &status);
                        if window.is_visible() {
                            notice(&window, "Could not save project", &error);
                        }
                    }
                }
            });
        },
    );
}

#[cfg(test)]
pub(super) fn test_named_copy(window: &adw::ApplicationWindow, state: &Shared) {
    let temporary = super::test_support::TempDir::new("named-project");
    let path = temporary.path().join("my-rice.hye");
    let snapshot = Snapshot::capture(&state.borrow()).unwrap();
    let expected = snapshot
        .workspace
        .to_json(&state.borrow().catalogue)
        .unwrap();
    let button = gtk::Button::new();
    let status = gtk::Label::new(None);
    write_to_file(
        gtk::gio::File::for_path(&path),
        snapshot,
        window,
        &button,
        state,
        &status,
    );
    let context = gtk::glib::MainContext::default();
    let start = std::time::Instant::now();
    while !button.is_sensitive() {
        while context.pending() {
            context.iteration(false);
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(8),
            "Named project save did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        status.text().starts_with("Project copy saved:"),
        "{}",
        status.text()
    );
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    let check = workflow::check_workspace(path).unwrap();
    assert_eq!(check["original_paths_opened"], false);
    assert_eq!(
        check["source_graph_replayed"],
        !state.borrow().source_graphs.is_empty()
    );
    assert!(!state.borrow().dirty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyprland_editor_core::{Value, BASELINE};

    #[test]
    fn saved_revision_does_not_clear_edits_made_during_a_project_save() {
        let state = super::super::initial_state();
        {
            let mut state = state.borrow_mut();
            state.remember("general:border_size").unwrap();
            state
                .draft
                .values
                .insert("general:border_size".into(), Value::Int(3));
        }
        let snapshot = Snapshot::capture(&state.borrow()).unwrap();
        assert!(snapshot.mark_saved(&state));
        assert!(!state.borrow().dirty());
        let snapshot = Snapshot::capture(&state.borrow()).unwrap();
        {
            let mut state = state.borrow_mut();
            state.remember("general:border_size").unwrap();
            state
                .draft
                .values
                .insert("general:border_size".into(), Value::Int(7));
        }
        assert!(!snapshot.mark_saved(&state));
        assert!(state.borrow().dirty());
        assert_eq!(
            state.borrow().saved[BASELINE]["general:border_size"],
            Value::Int(3)
        );
    }
}
