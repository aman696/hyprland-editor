//! Read-only view of scalar assignments across a native draft and its linked files.
use super::{label, legacy_file_editor, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    legacy_includes::{
        captures,
        effective::{self, IssueKind, Report, ValueKind},
    },
    source::display_path_bytes,
    BASELINE,
};
use std::{cell::RefCell, fmt::Write, rc::Rc};

struct Snapshot {
    report: Report,
    paths: Vec<String>,
    generation: u64,
}

impl Snapshot {
    fn location(&self, file: Option<usize>, line: usize) -> String {
        format!("{}:{line}", self.paths[file.map_or(0, |index| index + 1)])
    }
}

fn issue_text(kind: IssueKind) -> &'static str {
    match kind {
        IssueKind::MissingInclude => "Include file has no snapshot",
        IssueKind::UnresolvedInclude => "Include path or context is unresolved",
        IssueKind::IncludeCycle => "Include cycle",
        IssueKind::IncludeDepth => "Include depth limit reached",
        IssueKind::Conditional => "Parser condition needs evaluation",
        IssueKind::ComputedKey => "Setting or directive name needs expansion",
        IssueKind::Syntax => "Scope or syntax could not be verified",
        IssueKind::Limit => "Trace limit reached",
    }
}

pub(super) fn open(
    parent: &impl IsA<gtk::Window>,
    state: &Shared,
    project_status: &gtk::Label,
    initial: Option<&str>,
) {
    let window = adw::Window::builder()
        .title("Source Setting Trace")
        .transient_for(parent)
        .modal(true)
        .default_width(660)
        .default_height(560)
        .build();
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    body.set_margin_start(16);
    body.set_margin_end(16);
    body.set_margin_top(12);
    body.set_margin_bottom(12);
    frame.append(&body);

    let search = gtk::SearchEntry::builder()
        .placeholder_text("Find a setting")
        .build();
    search.set_widget_name("source-trace-search");
    body.append(&search);
    let model = gtk::StringList::new(&[]);
    let settings = gtk::DropDown::builder().model(&model).hexpand(true).build();
    super::monitors::compact(&settings);
    settings.update_property(&[gtk::accessible::Property::Label("Setting to trace")]);
    body.append(&settings);
    let result = label("");
    result.set_widget_name("source-trace-result");
    body.append(&result);
    let history = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::Char)
        .build();
    history.set_widget_name("source-trace-history");
    body.append(
        &gtk::ScrolledWindow::builder()
            .child(&history)
            .vexpand(true)
            .min_content_height(150)
            .build(),
    );

    let footer = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(2)
        .column_spacing(6)
        .row_spacing(6)
        .build();
    let refresh = gtk::Button::with_label("Refresh Trace");
    let edit = gtk::Button::with_label("Edit in Linked File…");
    let feedback = label("");
    feedback.set_widget_name("source-trace-status");
    footer.insert(&refresh, -1);
    footer.insert(&edit, -1);
    body.append(&feedback);
    body.append(&footer);

    let mut options: Vec<_> = state
        .borrow()
        .catalogue
        .iter()
        .filter(|setting| setting.version == BASELINE)
        .map(|setting| (setting.id.clone(), super::human_name(&setting.id)))
        .collect();
    options.sort_by(|a, b| a.0.cmp(&b.0));
    let options = Rc::new(options);
    let snapshot = Rc::new(RefCell::new(None::<Snapshot>));
    let filtered = Rc::new(RefCell::new(Vec::<usize>::new()));
    let render = {
        let (snapshot, options, filtered, edit) = (
            snapshot.clone(),
            options.clone(),
            filtered.clone(),
            edit.downgrade(),
        );
        move |selection: u32| {
            let Some(edit) = edit.upgrade() else { return };
            edit.set_sensitive(false);
            let snapshot = snapshot.borrow();
            let Some(snapshot) = snapshot.as_ref() else {
                result.set_text("Reading captured configuration…");
                history.buffer().set_text("");
                return;
            };
            let filtered = filtered.borrow();
            let Some(index) = filtered.get(selection as usize) else {
                result.set_text("No matching settings.");
                history.buffer().set_text("");
                return;
            };
            let id = &options[*index].0;
            let trace = snapshot.report.settings.get(id);
            let last = trace.and_then(|trace| trace.assignments.back());
            let title = match (snapshot.report.issue_count, last.map(|a| a.kind)) {
                (0, Some(ValueKind::Literal)) => "Last captured literal",
                (0, Some(ValueKind::Expansion)) => "Last assignment needs expansion",
                (0, Some(ValueKind::Unverified)) => "Last assignment needs syntax review",
                (0, None) => "No captured assignment",
                (_, Some(_)) => "Last visible assignment · Trace is incomplete",
                (_, None) => "No visible assignment · Trace is incomplete",
            };
            result.set_text(&format!("{title} · {id}"));
            edit.set_sensitive(last.is_some_and(|assignment| assignment.file.is_some()));
            show_history(snapshot, trace, &history);
        }
    };
    {
        let render = render.clone();
        settings.connect_selected_notify(move |picker| render(picker.selected()));
    }
    if let Some(id) = initial {
        search.set_text(id);
    }
    {
        let (picker, options, filtered) = (settings.downgrade(), options.clone(), filtered.clone());
        let filter = move |query: &str| {
            let Some(picker) = picker.upgrade() else {
                return;
            };
            let query = query.trim().to_lowercase();
            let visible: Vec<_> = options
                .iter()
                .enumerate()
                .filter(|(_, (id, name))| {
                    id.contains(&query) || name.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect();
            let labels = visible
                .iter()
                .map(|index| options[*index].0.as_str())
                .collect::<Vec<_>>();
            *filtered.borrow_mut() = visible.clone();
            model.splice(0, model.n_items(), &labels);
            picker.set_selected(
                visible
                    .iter()
                    .position(|index| options[*index].0 == query)
                    .map(|index| index as u32)
                    .unwrap_or_else(|| {
                        if visible.is_empty() {
                            gtk::INVALID_LIST_POSITION
                        } else {
                            0
                        }
                    }),
            );
            render(picker.selected());
        };
        filter(search.text().as_str());
        search.connect_search_changed(move |entry| filter(entry.text().as_str()));
    }
    {
        let (window, state, snapshot, settings, feedback) = (
            window.downgrade(),
            state.clone(),
            snapshot.clone(),
            settings.downgrade(),
            feedback.clone(),
        );
        refresh.connect_clicked(move |button| {
            let Some((draft, document, path, captures, catalogue, generation)) = ({
                let state = state.borrow();
                let version = &state.draft.version;
                state
                    .legacy_sources
                    .get(version)
                    .zip(state.source_origins.get(version))
                    .map(|(document, path)| {
                        (
                            workflow::draft_workspace(&state),
                            document.clone(),
                            path.selected_path.clone(),
                            state.legacy_include_files.clone(),
                            state.catalogue.clone(),
                            state.generation,
                        )
                    })
            }) else {
                feedback.set_text("Open a native .conf project first.");
                return;
            };
            button.set_sensitive(false);
            feedback.set_text("Reading the staged root and captured includes…");
            let (button, window, state, snapshot, feedback, settings) = (
                button.downgrade(),
                window.clone(),
                state.clone(),
                snapshot.clone(),
                feedback.clone(),
                settings.clone(),
            );
            workflow::background(
                move || {
                    let root = document.edited_copy(&draft)?;
                    let (files, revisions) =
                        captures.as_ref().map_or((&[][..], &[][..]), |snapshot| {
                            (&snapshot.files[..], &snapshot.revisions[..])
                        });
                    let report = effective::inspect(&path, &root, files, revisions, &catalogue)?;
                    let paths = std::iter::once(display_path_bytes(&path))
                        .chain(files.iter().map(|file| file.identity.display_path()))
                        .collect();
                    Ok(Snapshot {
                        report,
                        paths,
                        generation,
                    })
                },
                move |output| {
                    if let Some(button) = button.upgrade() {
                        button.set_sensitive(true);
                    }
                    if window.upgrade().is_none_or(|window| !window.is_visible()) {
                        return;
                    }
                    match output {
                        Ok(current) if state.borrow().generation == generation => {
                            *snapshot.borrow_mut() = Some(current);
                            feedback.set_text("");
                        }
                        Ok(_) => {
                            *snapshot.borrow_mut() = None;
                            feedback.set_text(
                                "The draft changed during the trace. Refresh to try again.",
                            );
                        }
                        Err(error) => {
                            *snapshot.borrow_mut() = None;
                            feedback.set_text(&error);
                        }
                    }
                    if let Some(settings) = settings.upgrade() {
                        settings.notify("selected");
                    }
                },
            );
        });
    }
    {
        let (window, state, settings, refresh, project_status) = (
            window.downgrade(),
            state.clone(),
            settings.downgrade(),
            refresh.downgrade(),
            project_status.clone(),
        );
        edit.connect_clicked(move |button| {
            let Some(picker) = settings.upgrade() else {
                return;
            };
            let filtered = filtered.borrow();
            let Some(index) = filtered.get(picker.selected() as usize) else {
                return;
            };
            let id = &options[*index].0;
            let snapshot = snapshot.borrow();
            let Some(snapshot) = snapshot.as_ref() else {
                return;
            };
            let generation = snapshot.generation;
            let child = snapshot
                .report
                .settings
                .get(id)
                .and_then(|trace| trace.assignments.back())
                .and_then(|entry| entry.file);
            let (file, source, catalogue) = {
                let state = state.borrow();
                if state.generation != generation {
                    feedback.set_text("Refresh the trace before opening an assignment.");
                    return;
                }
                let Some((file, captures)) = state
                    .legacy_include_files
                    .as_ref()
                    .and_then(|s| s.files.get(child?).map(|file| (file, s)))
                else {
                    return;
                };
                (
                    file.clone(),
                    captures::effective_text(file, &captures.revisions).to_owned(),
                    state.catalogue.clone(),
                )
            };
            button.set_sensitive(false);
            let (window, id, state, button, refresh, feedback, project_status) = (
                window.clone(),
                id.clone(),
                state.clone(),
                button.downgrade(),
                refresh.clone(),
                feedback.clone(),
                project_status.clone(),
            );
            workflow::background(
                move || legacy_file_editor::load(file, source, &catalogue),
                move |output| {
                    if let Some(button) = button.upgrade() {
                        button.set_sensitive(true);
                    }
                    let Some(window) = window.upgrade().filter(gtk::prelude::WidgetExt::is_visible)
                    else {
                        return;
                    };
                    if state.borrow().generation != generation {
                        feedback.set_text("Refresh the trace before opening an assignment.");
                        return;
                    }
                    let refresh = Rc::new(move || {
                        if let Some(button) = refresh.upgrade() {
                            button.emit_clicked();
                        }
                    });
                    if let Err(error) = output.and_then(|prepared| {
                        legacy_file_editor::open(
                            &window,
                            &state,
                            &project_status,
                            prepared,
                            refresh,
                            Some(&id),
                        )
                    }) {
                        feedback.set_text(&error);
                    }
                },
            );
        });
    }
    window.set_content(Some(&frame));
    window.present();
    refresh.emit_clicked();
}

fn show_history(snapshot: &Snapshot, trace: Option<&effective::History>, view: &gtk::TextView) {
    let mut text = String::new();
    if let Some(trace) = trace {
        for assignment in trace.assignments.iter().rev() {
            let location = snapshot.location(assignment.file, assignment.line);
            let value = display_path_bytes(assignment.value.as_bytes());
            let note = match assignment.kind {
                ValueKind::Literal => "",
                ValueKind::Expansion => "  (needs expansion)",
                ValueKind::Unverified => "  (needs syntax review)",
            };
            let shown = if value.is_empty() { "(empty)" } else { &value };
            let _ = writeln!(text, "{shown}{note}\n{location}\n");
        }
        let omitted = trace.total - trace.assignments.len();
        if omitted != 0 {
            let _ = writeln!(text, "{omitted} older assignments omitted\n");
        }
    }
    if snapshot.report.issue_count != 0 {
        let _ = writeln!(text, "Unresolved source ({})", snapshot.report.issue_count);
        for issue in &snapshot.report.issues {
            let reason = issue_text(issue.kind);
            if let Some(line) = issue.line {
                let _ = writeln!(text, "{reason} · {}", snapshot.location(issue.file, line));
            } else {
                let _ = writeln!(text, "{reason}");
            }
        }
    }
    view.buffer().set_text(&text);
}
