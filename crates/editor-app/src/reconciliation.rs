//! Review and accept a captured source revision without opening recorded paths or writing config.
use super::{detail, label, notice, presentation, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::LuaDocument,
    reconcile::{self, Choice, Observation, Plan},
    source::SourceIdentity,
    Draft, Setting,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

struct Input {
    base: Arc<LuaDocument>,
    origin: SourceIdentity,
    draft: Draft,
    catalogue: Vec<Setting>,
    generation: u64,
}
struct Review {
    plan: Plan,
    incoming: Arc<LuaDocument>,
    origin: SourceIdentity,
    base: Arc<LuaDocument>,
    base_origin: SourceIdentity,
    generation: u64,
}
fn input(state: &Shared) -> Result<Input, String> {
    let s = state.borrow();
    let version = &s.draft.version;
    if s.source_graphs.contains_key(version) {
        return Err("Saved graph reconciliation must include every file. Single-root comparison would invalidate its resolution inputs.".into());
    }
    Ok(Input {
        base: s
            .sources
            .get(version)
            .cloned()
            .ok_or("Import a Lua source snapshot first")?,
        origin: s
            .source_origins
            .get(version)
            .cloned()
            .ok_or("Source identity is missing")?,
        draft: s.draft.clone(),
        catalogue: s.catalogue.clone(),
        generation: s.generation,
    })
}
fn compare(path: PathBuf, input: Input) -> Result<Review, String> {
    let captured = super::capture_worker::read_selected(&path)?;
    let incoming = Arc::new(workflow::parse_source(
        captured.content,
        input.draft.version.clone(),
        &input.catalogue,
    )?);
    let plan = reconcile::prepare(&input.base, &input.draft, &incoming, &input.catalogue)?;
    Ok(Review {
        plan,
        incoming,
        origin: captured.identity,
        base: input.base,
        base_origin: input.origin,
        generation: input.generation,
    })
}
pub(super) fn check(workspace: PathBuf, selected: PathBuf) -> Result<serde_json::Value, String> {
    let (draft, base, origin) = workflow::source_workspace(workspace)?;
    let review = compare(
        selected,
        Input {
            base: Arc::new(base),
            origin,
            draft,
            catalogue: hyprland_editor_core::catalogue(),
            generation: 0,
        },
    )?;
    let rows:Vec<_>=review.plan.rows().iter().map(|r|serde_json::json!({"setting":r.id,"requires_choice":r.automatic.is_none(),"selected_value_available":r.incoming.can_use(),"automatic":r.automatic.map(|c|match c{Choice::KeepDraft=>"keep_draft",Choice::UseSelectedFile=>"use_selected_file"})})).collect();
    Ok(
        serde_json::json!({"scope":"static-source-comparison-only","base_sha256":review.base.report.source_sha256,
        "selected_sha256":review.incoming.report.source_sha256,"rows":rows,"partial_projection":review.plan.partial(),
        "original_paths_opened":false,"files_written":false,"runtime_verified":false}),
    )
}
fn accept(
    state: &Shared,
    review: &Review,
    choices: &BTreeMap<String, Choice>,
    ack: bool,
) -> Result<(), String> {
    let mut s = state.borrow_mut();
    if s.generation != review.generation
        || s.source_origins.get(&s.draft.version) != Some(&review.base_origin)
    {
        return Err("The draft or source changed while this review was open. Compare the current revision again.".into());
    }
    let base = s
        .sources
        .get(&s.draft.version)
        .ok_or("Source snapshot is missing")?;
    let draft =
        review
            .plan
            .resolve(base, &s.draft, &review.incoming, choices, ack, &s.catalogue)?;
    s.replace_source(review.incoming.clone(), review.origin.clone(), draft)
}
pub(super) fn connect(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
    selection: &gtk::SingleSelection,
    panel: &gtk::Box,
) {
    let win = window.downgrade();
    let state = state.clone();
    let status = status.clone();
    let selection = selection.clone();
    let panel = panel.clone();
    button.connect_clicked(move |button| {
        let Some(win) = win.upgrade() else {
            return;
        };
        let context = match input(&state) {
            Ok(context) => context,
            Err(error) => {
                notice(&win, "Source snapshot required", &error);
                return;
            }
        };
        let picker = gtk::FileDialog::builder()
            .title("Select the changed Lua source to compare")
            .build();
        let button = button.downgrade();
        let state = state.clone();
        let status = status.clone();
        let selection = selection.clone();
        let panel = panel.clone();
        picker.open(
            Some(&win.clone()),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                let Ok(file) = result else {
                    return;
                };
                let Some(path) = file.path() else {
                    return;
                };
                let Some(button) = button.upgrade() else {
                    return;
                };
                button.set_sensitive(false);
                let button = button.downgrade();
                let win = win.downgrade();
                workflow::background(
                    move || compare(path, context),
                    move |result| {
                        if let Some(button) = button.upgrade() {
                            button.set_sensitive(true);
                        }
                        let Some(win) = win.upgrade() else {
                            return;
                        };
                        match result {
                            Ok(review) if review.generation == state.borrow().generation => {
                                show(&win, &state, &status, &selection, &panel, review);
                            }
                            Ok(_) => notice(
                                &win,
                                "Draft changed",
                                "Compare again to review your current edits.",
                            ),
                            Err(error) => notice(&win, "Comparison failed", &error),
                        }
                    },
                );
            },
        );
    });
}
fn observation(value: &Observation) -> String {
    match value {
        Observation::Absent => "Not declared".into(),
        Observation::Known(v) => presentation::value_text(v),
        Observation::Unresolved(Some(v)) => format!(
            "Literal: {} · projection incomplete",
            presentation::value_text(v)
        ),
        Observation::Unresolved(None) => "Unresolved · absence cannot be inferred".into(),
    }
}
fn ready(button: &gtk::Button, review: &Review, choices: &BTreeMap<String, Choice>, ack: bool) {
    button.set_sensitive(
        (!review.plan.partial() || ack)
            && review
                .plan
                .rows()
                .iter()
                .all(|row| row.automatic.is_some() || choices.contains_key(&row.id)),
    );
}
fn source_view(source: &str) -> gtk::ScrolledWindow {
    let text = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(12)
        .right_margin(12)
        .top_margin(12)
        .bottom_margin(12)
        .build();
    text.buffer().set_text(source);
    gtk::ScrolledWindow::builder()
        .child(&text)
        .vexpand(true)
        .hexpand(true)
        .build()
}
fn row_editor(
    panel: &gtk::Box,
    review: &Rc<Review>,
    index: usize,
    choices: &Rc<RefCell<BTreeMap<String, Choice>>>,
    apply: &gtk::Button,
    ack: &Rc<Cell<bool>>,
) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }
    let Some(row) = review.plan.rows().get(index) else {
        panel.append(&label("No scalar declarations to compare. Review both source texts before accepting this revision."));
        return;
    };
    let title = label(&presentation::setting_name(&row.id));
    title.add_css_class("title-2");
    panel.append(&title);
    panel.append(&label(&presentation::context(&row.id)));
    panel.append(&label(&format!(
        "Previous file: {}",
        observation(&row.base)
    )));
    panel.append(&label(&format!(
        "Your draft: {}",
        row.draft
            .as_ref()
            .map(presentation::value_text)
            .unwrap_or_else(|| "No override".into())
    )));
    panel.append(&label(&format!(
        "Selected file: {}",
        observation(&row.incoming)
    )));
    let explanation = if row.automatic.is_none() {
        "A choice is required. The setting changed on both sides, or a source projection is incomplete."
    } else {
        "The comparison found an independent change. You can keep your draft value or use the selected file."
    };
    panel.append(&label(explanation));
    let can_use = row.incoming.can_use();
    let automatic = match row.automatic {
        None => "Choose a result…",
        Some(Choice::KeepDraft) => "Automatic: Keep My Draft",
        Some(Choice::UseSelectedFile) if matches!(row.incoming, Observation::Absent) => {
            "Automatic: Remove Override"
        }
        Some(Choice::UseSelectedFile) => "Automatic: Use File Value",
    };
    let model = gtk::StringList::new(&[automatic, "Keep My Draft"]);
    if can_use {
        model.append(if matches!(row.incoming, Observation::Absent) {
            "Use File (remove override)"
        } else if matches!(row.incoming, Observation::Unresolved(Some(_))) {
            "Use Visible Literal"
        } else {
            "Use File Value"
        });
    }
    let picker = gtk::DropDown::builder().model(&model).build();
    picker.set_widget_name("reconcile-choice");
    let accessible = format!("Result for {}", presentation::setting_name(&row.id));
    picker.update_property(&[gtk::accessible::Property::Label(&accessible)]);
    let selected = choices.borrow().get(&row.id).copied().or(row.automatic);
    picker.set_selected(match selected {
        Some(Choice::KeepDraft) => 1,
        Some(Choice::UseSelectedFile) => 2,
        None => 0,
    });
    let id = row.id.clone();
    let review = review.clone();
    let choices = choices.clone();
    let apply = apply.downgrade();
    let ack = ack.clone();
    picker.connect_selected_notify(move |picker| {
        let choice = match picker.selected() {
            1 => Some(Choice::KeepDraft),
            2 if can_use => Some(Choice::UseSelectedFile),
            _ => None,
        };
        if let Some(choice) = choice {
            choices.borrow_mut().insert(id.clone(), choice);
        } else {
            choices.borrow_mut().remove(&id);
        }
        if let Some(apply) = apply.upgrade() {
            ready(&apply, &review, &choices.borrow(), ack.get());
        }
    });
    panel.append(&picker);
}
fn show(
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
    selection: &gtk::SingleSelection,
    panel: &gtk::Box,
    review: Review,
) -> adw::Window {
    let dialog = adw::Window::builder()
        .title("Compare Changed Source")
        .transient_for(window)
        .modal(true)
        .default_width(680)
        .default_height(700)
        .build();
    dialog.set_size_request(350, 480);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    let review = Rc::new(review);
    let choices = Rc::new(RefCell::new(BTreeMap::new()));
    let ack = Rc::new(Cell::new(false));
    let conflicts = review
        .plan
        .rows()
        .iter()
        .filter(|r| r.automatic.is_none())
        .count();
    content.append(&label(&format!(
        "{} settings compared · {conflicts} need a choice",
        review.plan.rows().len()
    )));
    content.append(&label("Accepting updates your private draft and its source snapshot. Review both files; later external changes need another comparison. Undo restores the earlier revision."));
    content.append(&label("This comparison merges scalar settings. Monitor, device, binding and workspace drafts stay unchanged; structured source reconciliation is not implemented yet."));
    let technical = gtk::Expander::builder().label("File Details").build();
    technical.set_widget_name("reconcile-details");
    let evidence=label(&format!("Previous: {}\nSelected: {}\nPrevious SHA-256: {}\nSelected SHA-256: {}\nSelected resolved target: {}\nSelected links: {}\nSelected UID: {} · mode {:o}\nSource diagnostics: {}",review.base_origin.display_path(),review.origin.display_path(),review.base.report.source_sha256,review.incoming.report.source_sha256,review.origin.resolved_path.as_deref().map(hyprland_editor_core::source::display_path_bytes).unwrap_or_else(||"Not recorded".into()),review.origin.links.len(),review.origin.owner,review.origin.mode & 0o777,review.incoming.report.diagnostics.len()));
    technical.set_child(Some(
        &gtk::ScrolledWindow::builder()
            .child(&evidence)
            .max_content_height(180)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    ));
    content.append(&technical);
    let stack = gtk::Stack::builder().vexpand(true).build();
    stack.set_widget_name("reconcile-pages");
    let switcher =
        gtk::DropDown::from_strings(&["Setting Changes", "Previous File", "Selected File"]);
    switcher.set_tooltip_text(Some("Choose the part of the comparison to review"));
    switcher.set_widget_name("reconcile-page");
    switcher.update_property(&[gtk::accessible::Property::Label("Review Page")]);
    let pages = stack.downgrade();
    switcher.connect_selected_notify(move |picker| {
        if let Some(stack) = pages.upgrade() {
            stack.set_visible_child_name(match picker.selected() {
                1 => "previous",
                2 => "incoming",
                _ => "changes",
            });
        }
    });
    content.append(&switcher);
    let changes = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let names: Vec<String> = review
        .plan
        .rows()
        .iter()
        .map(|r| {
            format!(
                "{}{} · {}",
                if r.automatic.is_none() { "• " } else { "" },
                presentation::context(&r.id),
                presentation::setting_name(&r.id)
            )
        })
        .collect();
    let model = gtk::StringList::new(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let selector = gtk::DropDown::builder()
        .model(&model)
        .enable_search(true)
        .build();
    selector.set_widget_name("reconcile-setting");
    selector.update_property(&[gtk::accessible::Property::Label("Setting to Compare")]);
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        item.set_child(Some(
            &gtk::Label::builder()
                .xalign(0.)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .max_width_chars(28)
                .build(),
        ));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let text = item.item().and_downcast::<gtk::StringObject>().unwrap();
        item.child()
            .and_downcast::<gtk::Label>()
            .unwrap()
            .set_text(&text.string());
    });
    selector.set_factory(Some(&factory));
    let first = review
        .plan
        .rows()
        .iter()
        .position(|r| r.automatic.is_none())
        .unwrap_or(0);
    selector.set_selected(first as u32);
    selector.set_visible(!review.plan.rows().is_empty());
    changes.append(&selector);
    let editor = gtk::Box::new(gtk::Orientation::Vertical, 12);
    changes.append(
        &gtk::ScrolledWindow::builder()
            .child(&editor)
            .vexpand(true)
            .build(),
    );
    stack.add_titled(&changes, Some("changes"), "Settings");
    stack.add_titled(
        &source_view(&review.base.source),
        Some("previous"),
        "Previous File",
    );
    stack.add_titled(
        &source_view(&review.incoming.source),
        Some("incoming"),
        "Selected File",
    );
    content.append(&stack);
    let acknowledge = gtk::CheckButton::new();
    acknowledge.set_child(Some(&label(
        "I reviewed the incomplete projection and source text",
    )));
    acknowledge.set_widget_name("reconcile-acknowledge");
    acknowledge.update_property(&[gtk::accessible::Property::Label(
        "I reviewed the incomplete projection and source text",
    )]);
    acknowledge.set_visible(review.plan.partial());
    if review.plan.partial() {
        content.append(&label("Opaque or unsupported syntax is retained. Missing projected values are unresolved. File literals are declarations, not verified desktop values."));
    }
    content.append(&acknowledge);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    actions.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let apply = gtk::Button::with_label("Use Reviewed Revision");
    apply.add_css_class("suggested-action");
    apply.set_widget_name("reconcile-accept");
    actions.append(&cancel);
    actions.append(&apply);
    content.append(&actions);
    root.append(&content);
    dialog.set_content(Some(&root));
    row_editor(&editor, &review, first, &choices, &apply, &ack);
    let r = review.clone();
    let c = choices.clone();
    let a = apply.downgrade();
    let acknowledged = ack.clone();
    selector.connect_selected_notify(move |selector| {
        if let Some(a) = a.upgrade() {
            row_editor(
                &editor,
                &r,
                selector.selected() as usize,
                &c,
                &a,
                &acknowledged,
            );
        }
    });
    let r = review.clone();
    let c = choices.clone();
    let a = apply.downgrade();
    let acknowledged = ack.clone();
    acknowledge.connect_toggled(move |checkbox| {
        acknowledged.set(checkbox.is_active());
        if let Some(a) = a.upgrade() {
            ready(&a, &r, &c.borrow(), acknowledged.get());
        }
    });
    ready(&apply, &review, &choices.borrow(), ack.get());
    let d = dialog.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(d) = d.upgrade() {
            d.close();
        }
    });
    let d = dialog.downgrade();
    let state = state.clone();
    let status = status.clone();
    let selection = selection.clone();
    let panel = panel.clone();
    let parent = window.downgrade();
    apply.connect_clicked(move|_|{
        let Some(parent)=parent.upgrade()else{return;};
        if let Err(error)=accept(&state,&review,&choices.borrow(),ack.get()){notice(&parent,"Revision not accepted",&error);return;}
        let setting=state.borrow().visible.get(selection.selected() as usize).cloned();
        if let Some(setting)=setting{detail(&state,&panel,&setting,&status);}
        super::summary(&state,&status);status.set_tooltip_text(Some("Reviewed source revision retained in memory. Save Draft persists it. No configuration file was written or desktop setting applied."));
        if let Some(dialog)=d.upgrade(){dialog.close();}
    });
    dialog.present();
    selector.grab_focus();
    dialog
}

#[cfg(test)]
pub(super) fn test_review(window: &adw::ApplicationWindow) {
    use hyprland_editor_core::{lua, source_path, Value, BASELINE};
    use std::time::{SystemTime, UNIX_EPOCH};
    fn find<
        T: gtk::glib::object::IsA<gtk::Widget> + gtk::glib::object::IsClass + Clone + 'static,
    >(
        dialog: &adw::Window,
        name: &str,
    ) -> T {
        fn walk(widget: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
            if widget.widget_name() == name {
                return Some(widget.clone());
            }
            let mut child = widget.first_child();
            while let Some(w) = child {
                if let Some(found) = walk(&w, name) {
                    return Some(found);
                }
                child = w.next_sibling();
            }
            None
        }
        walk(dialog.upcast_ref(), name)
            .unwrap()
            .downcast::<T>()
            .unwrap()
    }
    let root = std::env::temp_dir().join(format!(
        "he-merge-ui-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let old_path = root.join("old.lua");
    let new_path = root.join("selected.lua");
    let old = "-- previous\nhl.config{general={border_size=2},decoration={rounding=3}}\n";
    let new = "-- changed\nhl.config{general={border_size=7},decoration={rounding=4}}\n";
    std::fs::write(&old_path, old).unwrap();
    std::fs::write(&new_path, new).unwrap();
    let state = super::initial_state();
    let catalogue = state.borrow().catalogue.clone();
    let make_doc = |source: &str| {
        Arc::new(
            LuaDocument::from_report(
                source.into(),
                lua::parse(source, BASELINE, &catalogue).unwrap(),
                &catalogue,
            )
            .unwrap(),
        )
    };
    let original = source_path::read_selected(&old_path).unwrap().identity;
    {
        let mut s = state.borrow_mut();
        s.draft = make_doc(old).report.draft.clone();
        s.draft
            .values
            .insert("general:border_size".into(), Value::Int(5));
        s.sources.insert(BASELINE.into(), make_doc(old));
        s.source_origins.insert(BASELINE.into(), original.clone());
        let values = s.draft.values.clone();
        s.saved.insert(BASELINE.into(), values);
        s.saved_source_origins
            .insert(BASELINE.into(), Some(original.clone()));
    }
    let review_for = |source: &str| {
        let context = input(&state).unwrap();
        let incoming = make_doc(source);
        Review {
            plan: reconcile::prepare(&context.base, &context.draft, &incoming, &context.catalogue)
                .unwrap(),
            incoming,
            origin: source_path::read_selected(&new_path).unwrap().identity,
            base: context.base,
            base_origin: context.origin,
            generation: context.generation,
        }
    };
    let status = gtk::Label::new(None);
    let selection = gtk::SingleSelection::new(Some(gtk::StringList::new(&[])));
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let dialog = show(window, &state, &status, &selection, &panel, review_for(new));
    let apply: gtk::Button = find(&dialog, "reconcile-accept");
    assert!(!apply.is_sensitive());
    let picker: gtk::DropDown = find(&dialog, "reconcile-choice");
    picker.set_selected(1);
    assert!(apply.is_sensitive());
    if std::env::var_os("HYPRLAND_EDITOR_TEST_EXACT_RESIZE").is_some() {
        dialog.set_default_size(360, 720);
        let context = gtk::glib::MainContext::default();
        for _ in 0..25 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            dialog.width() <= 360,
            "reconciliation controls force {}px",
            dialog.width()
        );
        let evidence: gtk::Expander = find(&dialog, "reconcile-details");
        evidence.set_expanded(true);
        for _ in 0..25 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            dialog.width() <= 360 && dialog.height() <= 720,
            "file evidence must scroll within the narrow review"
        );
        evidence.set_expanded(false);
        let page: gtk::DropDown = find(&dialog, "reconcile-page");
        page.set_selected(2);
        let pages: gtk::Stack = find(&dialog, "reconcile-pages");
        assert_eq!(pages.visible_child_name().as_deref(), Some("incoming"));
        page.set_selected(0);
        for _ in 0..25 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
            let paintable = gtk::WidgetPaintable::new(Some(&dialog));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, dialog.width() as f64, dialog.height() as f64);
            dialog
                .renderer()
                .unwrap()
                .render_texture(snapshot.to_node().unwrap(), None)
                .save_to_png(format!("{path}.comparison.png"))
                .unwrap();
        }
    }
    apply.emit_clicked();
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        Value::Int(5)
    );
    assert_eq!(
        state.borrow().draft.values["decoration:rounding"],
        Value::Int(4)
    );
    assert!(state.borrow().dirty());
    assert_eq!(state.borrow().sources[BASELINE].source, new);
    assert_eq!(std::fs::read_to_string(&old_path).unwrap(), old);
    assert_eq!(std::fs::read_to_string(&new_path).unwrap(), new);
    {
        let mut s = state.borrow_mut();
        s.remember("general:border_size").unwrap();
        s.draft
            .values
            .insert("general:border_size".into(), Value::Int(9));
        s.undo_last();
        assert_eq!(s.draft.values["general:border_size"], Value::Int(5));
        s.undo_last();
        assert_eq!(s.draft.values["decoration:rounding"], Value::Int(3));
        assert_eq!(s.sources[BASELINE].source, old);
        assert_eq!(s.source_origins[BASELINE], original);
        assert!(!s.dirty());
    }
    dialog.destroy();
    // An accepted-looking old review cannot overwrite a newer field edit.
    let stale = review_for(new);
    {
        let mut s = state.borrow_mut();
        s.remember("general:border_size").unwrap();
        s.draft
            .values
            .insert("general:border_size".into(), Value::Int(8));
    }
    let generation = state.borrow().generation;
    assert!(accept(
        &state,
        &stale,
        &BTreeMap::from([("general:border_size".into(), Choice::UseSelectedFile)]),
        false
    )
    .is_err());
    assert_eq!(state.borrow().generation, generation);
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        Value::Int(8)
    );
    assert_eq!(state.borrow().sources[BASELINE].source, old);
    state.borrow_mut().undo_last();
    let opaque = "require('never-executed')\n";
    std::fs::write(&new_path, opaque).unwrap();
    let dialog = show(
        window,
        &state,
        &status,
        &selection,
        &panel,
        review_for(opaque),
    );
    if std::env::var_os("HYPRLAND_EDITOR_TEST_EXACT_RESIZE").is_some() {
        dialog.set_default_size(360, 720);
        let context = gtk::glib::MainContext::default();
        for _ in 0..25 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            dialog.width() <= 360,
            "partial-source controls force {}px",
            dialog.width()
        );
    }
    let apply: gtk::Button = find(&dialog, "reconcile-accept");
    let ack: gtk::CheckButton = find(&dialog, "reconcile-acknowledge");
    assert!(ack.is_visible());
    assert!(!apply.is_sensitive());
    let picker: gtk::DropDown = find(&dialog, "reconcile-choice");
    assert_eq!(picker.model().unwrap().n_items(), 2);
    picker.set_selected(1);
    let selector: gtk::DropDown = find(&dialog, "reconcile-setting");
    selector.set_selected(1);
    let picker: gtk::DropDown = find(&dialog, "reconcile-choice");
    picker.set_selected(1);
    assert!(!apply.is_sensitive());
    ack.set_active(true);
    assert!(apply.is_sensitive());
    apply.emit_clicked();
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        Value::Int(5)
    );
    assert_eq!(state.borrow().sources[BASELINE].source, opaque);
    state.borrow_mut().undo_last();
    assert_eq!(state.borrow().sources[BASELINE].source, old);
    dialog.destroy();
    std::fs::remove_dir_all(root).unwrap();
}
