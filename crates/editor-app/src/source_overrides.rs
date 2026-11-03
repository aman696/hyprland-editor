//! Explicit private source/override review. No live setter or file writer.
use super::source_changes::Permit;
use super::{source_analysis::Model, source_graph, worker, workflow};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::replay::{
        override_review::{self, Choice, Resolution, Review, SourceValue},
        Program,
    },
    workspace_graph::{self, revisions},
    Draft,
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::Arc,
};
fn caption(text: &str) -> gtk::Label {
    let label = super::label(text);
    label.set_selectable(false);
    label
}
struct Session {
    draft: Draft,
    generation: u64,
    graph: Option<String>,
    choices: BTreeMap<String, Choice>,
    review: Option<Box<Review>>,
    result: Option<Box<Resolution>>,
    epoch: u64,
    busy: bool,
    closed: bool,
    rendering: bool,
}
#[derive(Clone)]
struct Controls {
    state: Weak<RefCell<super::State>>,
    model: Weak<RefCell<Option<Arc<source_graph::Analysis>>>>,
    analysis: std::sync::Weak<source_graph::Analysis>,
    session: Weak<RefCell<Session>>,
    refresh: Rc<dyn Fn()>,
    window: gtk::glib::WeakRef<adw::Window>,
    targets: gtk::glib::WeakRef<gtk::DropDown>,
    choice: gtk::glib::WeakRef<gtk::DropDown>,
    details: gtk::glib::WeakRef<gtk::Label>,
    output: gtk::glib::WeakRef<gtk::TextView>,
    status: gtk::glib::WeakRef<gtk::Label>,
    compare: gtk::glib::WeakRef<gtk::Button>,
    keep: gtk::glib::WeakRef<gtk::Button>,
}
impl Controls {
    fn message(&self, text: &str) {
        if let Some(status) = self.status.upgrade() {
            status.set_text(&text.chars().take(1024).collect::<String>());
        }
    }
    fn current(&self) -> bool {
        let (Some(state), Some(model), Some(analysis), Some(session)) = (
            self.state.upgrade(),
            self.model.upgrade(),
            self.analysis.upgrade(),
            self.session.upgrade(),
        ) else {
            return false;
        };
        let state = state.borrow();
        let s = session.borrow();
        !s.closed
            && state.generation == s.generation
            && state.draft.version == s.draft.version
            && state
                .source_graphs
                .get(&s.draft.version)
                .map(|g| g.fingerprint.clone())
                == s.graph
            && model
                .borrow()
                .as_ref()
                .is_some_and(|m| Arc::ptr_eq(m, &analysis))
    }
    fn stale(&self) {
        if let Some(session) = self.session.upgrade() {
            let mut s = session.borrow_mut();
            s.closed = true;
            s.review = None;
            s.result = None;
            s.choices.clear();
        }
        if let Some(output) = self.output.upgrade() {
            output.buffer().set_text("");
        }
        if let Some(details) = self.details.upgrade() {
            details.set_text("");
        }
        self.message("The draft or source analysis changed. Close this review and open it again.");
        self.sensitive();
    }
    fn sensitive(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let s = session.borrow();
        let ready = !s.closed && !s.busy && s.review.is_some();
        for weak in [&self.targets, &self.choice] {
            if let Some(pick) = weak.upgrade() {
                pick.set_sensitive(ready);
            }
        }
        if let Some(b) = self.compare.upgrade() {
            b.set_sensitive(!s.closed && !s.busy);
            b.set_label(if s.review.is_some() {
                "Compare"
            } else {
                "Retry Review"
            });
        }
        if let Some(b) = self.keep.upgrade() {
            b.set_sensitive(
                ready
                    && s.result.is_some()
                    && s.choices.values().any(|c| *c != Choice::KeepSeparate),
            );
        }
    }
    fn select(&self) {
        let (Some(session), Some(targets), Some(choice), Some(details)) = (
            self.session.upgrade(),
            self.targets.upgrade(),
            self.choice.upgrade(),
            self.details.upgrade(),
        ) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.closed {
            return;
        }
        let Some(row) = s
            .review
            .as_ref()
            .and_then(|r| r.rows.get(targets.selected() as usize))
            .cloned()
        else {
            return;
        };
        let source=match row.source {
            SourceValue::Parsed{value,provisional,..}=>format!("Last modeled declaration: {}{}",super::source_access::excerpt(&value.lua()),if provisional{" (provisional)"}else{""}),
            SourceValue::Absent=>"No declaration found in the modeled configuration calls. No default is substituted.".into(),
            SourceValue::Unresolved{reason}=>format!("Source value unresolved: {reason}"),
        };
        details.set_text(&format!(
            "{source}\n\nOverride: {}\n{} reached declaration(s).{}",
            super::source_access::excerpt(&row.override_value.lua()),
            row.declaration_count,
            if row.can_write {
                ""
            } else {
                " Writing this override needs declaration insertion or an unfinished codec."
            }
        ));
        let options = if row.can_write {
            vec![
                "Keep separate",
                "Use source · remove override",
                "Write override into source",
            ]
        } else {
            vec!["Keep separate", "Use source · remove override"]
        };
        s.rendering = true;
        drop(s);
        choice.set_model(Some(&gtk::StringList::new(&options)));
        let selected = {
            let s = session.borrow();
            match s
                .choices
                .get(&row.setting_id)
                .copied()
                .unwrap_or(Choice::KeepSeparate)
            {
                Choice::KeepSeparate => 0,
                Choice::UseSource => 1,
                Choice::WriteOverride => 2,
            }
        };
        choice.set_selected(selected);
        session.borrow_mut().rendering = false;
    }
    fn changed(&self) {
        let (Some(session), Some(targets), Some(choice)) = (
            self.session.upgrade(),
            self.targets.upgrade(),
            self.choice.upgrade(),
        ) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.closed || s.rendering || s.busy {
            return;
        }
        let Some(row) = s
            .review
            .as_ref()
            .and_then(|r| r.rows.get(targets.selected() as usize))
            .cloned()
        else {
            return;
        };
        let selected = match choice.selected() {
            0 => Choice::KeepSeparate,
            1 => Choice::UseSource,
            2 if row.can_write => Choice::WriteOverride,
            _ => return,
        };
        s.choices.insert(row.setting_id, selected);
        s.epoch += 1;
        s.result = None;
        let changed = s
            .choices
            .values()
            .filter(|c| **c != Choice::KeepSeparate)
            .count();
        drop(s);
        if let Some(output) = self.output.upgrade() {
            output.buffer().set_text("");
        }
        self.message(&format!(
            "{changed} explicit choice(s). Compare before keeping them in the draft."
        ));
        self.sensitive();
    }
    fn load(&self) {
        if !self.current() {
            self.stale();
            return;
        }
        let (Some(analysis), Some(session)) = (self.analysis.upgrade(), self.session.upgrade())
        else {
            return;
        };
        let permit = match Permit::acquire() {
            Ok(v) => v,
            Err(e) => {
                self.message(&e);
                return;
            }
        };
        session.borrow_mut().busy = true;
        self.sensitive();
        self.message("Comparing source declarations with value overrides…");
        let program = analysis.program.clone();
        let draft = session.borrow().draft.clone();
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                match worker::run(worker::Request::ReviewSourceOverrides { program, draft })? {
                    worker::Response::SourceOverrideReview(r) => Ok(r),
                    _ => Err("Unexpected source/override response".into()),
                }
            },
            move |result| {
                if !ui.current() {
                    ui.stale();
                    return;
                }
                let Some(session) = ui.session.upgrade() else {
                    return;
                };
                session.borrow_mut().busy = false;
                match result {
                    Ok(review) => {
                        let Some(analysis) = ui.analysis.upgrade() else {
                            return;
                        };
                        let s = session.borrow();
                        let valid = review.program_sha256
                            == workspace_graph::program_fingerprint(&analysis.program)
                                .unwrap_or_default()
                            && review.version == s.draft.version
                            && review.compiler == analysis.report.compiler_profile
                            && review.schema_sha256 == workspace_graph::schema_sha256()
                            && review.rows.len() == s.draft.values.len()
                            && review.rows.iter().map(|r| r.setting_id.as_str()).eq(s
                                .draft
                                .values
                                .keys()
                                .map(String::as_str))
                            && review.rows.iter().all(|r| {
                                s.draft.values.get(&r.setting_id).is_some_and(|v| {
                                    serde_json::to_string(v).ok()
                                        == serde_json::to_string(&r.override_value).ok()
                                })
                            });
                        drop(s);
                        if !valid {
                            ui.message("The review does not match the current source and overrides. Close and retry.");
                            ui.sensitive();
                            return;
                        }
                        let names = review
                            .rows
                            .iter()
                            .map(|r| r.setting_id.as_str())
                            .collect::<Vec<_>>();
                        if let Some(targets) = ui.targets.upgrade() {
                            targets.set_model(Some(&gtk::StringList::new(&names)));
                        }
                        let complete = review.model_complete;
                        let empty = review.rows.is_empty();
                        session.borrow_mut().review = Some(review);
                        ui.select();
                        ui.message(if empty{"There are no scalar overrides to reconcile."}else if complete{"Choose which layer to keep for each setting. These are modeled source values; live desktop state is unverified."}else{"Source analysis is partial. Removing an override is an explicit choice; unknown source values stay unknown."});
                    }
                    Err(e) => ui.message(&e),
                }
                ui.sensitive();
                if session.borrow().review.is_some() {
                    if let (Some(window), Some(targets)) =
                        (ui.window.upgrade(), ui.targets.upgrade())
                    {
                        gtk::prelude::GtkWindowExt::set_focus(&window, Some(&targets));
                    }
                }
            },
        );
    }
    fn compare(&self) {
        if !self.current() {
            self.stale();
            return;
        }
        let (Some(session), Some(analysis)) = (self.session.upgrade(), self.analysis.upgrade())
        else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.busy || s.closed {
            return;
        }
        let Some(review) = s.review.clone() else {
            drop(s);
            self.load();
            return;
        };
        let permit = match Permit::acquire() {
            Ok(v) => v,
            Err(e) => {
                drop(s);
                self.message(&e);
                return;
            }
        };
        let draft = s.draft.clone();
        let choices = s.choices.clone();
        let epoch = s.epoch;
        s.busy = true;
        s.result = None;
        drop(s);
        self.sensitive();
        self.message("Preparing the proposed source and override changes…");
        let program = analysis.program.clone();
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                match worker::run(worker::Request::ResolveSourceOverrides {
                    program,
                    draft,
                    review: *review,
                    choices,
                })? {
                    worker::Response::SourceOverrideResolution(r) => Ok(r),
                    _ => Err("Unexpected source/override resolution".into()),
                }
            },
            move |result| {
                if !ui.current() {
                    ui.stale();
                    return;
                }
                let (Some(session), Some(analysis)) = (ui.session.upgrade(), ui.analysis.upgrade())
                else {
                    return;
                };
                let mut s = session.borrow_mut();
                s.busy = false;
                if s.epoch != epoch {
                    drop(s);
                    ui.sensitive();
                    return;
                }
                match result {
                    Ok(result) => {
                        let described = s
                            .review
                            .as_ref()
                            .ok_or_else(|| "Review is unavailable".to_string())
                            .and_then(|review| {
                                describe(&analysis.program, &s.draft, review, &s.choices, &result)
                            });
                        match described {
                            Ok(text) => {
                                if let Some(output) = ui.output.upgrade() {
                                    output.buffer().set_text(&text);
                                }
                                s.result = Some(result);
                                drop(s);
                                ui.message("Review the changes below, then keep them in the private draft. Original files and live settings remain untouched.");
                            }
                            Err(e) => {
                                drop(s);
                                ui.message(&e);
                            }
                        }
                    }
                    Err(e) => {
                        drop(s);
                        ui.message(&e);
                    }
                }
                ui.sensitive();
            },
        );
    }
    fn keep(&self) {
        if !self.current() {
            self.stale();
            return;
        }
        let (Some(session), Some(analysis), Some(state)) = (
            self.session.upgrade(),
            self.analysis.upgrade(),
            self.state.upgrade(),
        ) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.closed || s.busy {
            return;
        }
        let Some(result) = s.result.clone() else {
            return;
        };
        let Some(review) = &s.review else {
            return;
        };
        if let Err(e) = override_review::validate_resolution(&s.draft, review, &s.choices, &result)
        {
            drop(s);
            self.message(&e);
            return;
        }
        let bundle = match source_graph::freeze(&analysis) {
            Ok(b) => b,
            Err(e) => {
                drop(s);
                self.message(&e);
                return;
            }
        };
        let bundle = if let Some(saved) = state.borrow().source_graphs.get(&s.draft.version) {
            if bundle.fingerprint().ok().as_ref() != Some(&saved.fingerprint) {
                drop(s);
                self.message("The retained source revision differs from this analysis. Reopen its current source view.");
                return;
            }
            saved.bundle.clone()
        } else {
            bundle
        };
        let permit = match Permit::acquire() {
            Ok(v) => v,
            Err(e) => {
                drop(s);
                self.message(&e);
                return;
            }
        };
        s.busy = true;
        let version = s.draft.version.clone();
        drop(s);
        self.sensitive();
        self.message("Rebuilding the private source revision…");
        let ui = self.clone();
        let next_draft = result.draft.clone();
        let expected = result.program_sha256.clone();
        workflow::background(
            move || {
                let _permit = permit;
                let (saved, analysis) =
                    source_graph::apply_revisions(&bundle, &version, &result.plans)?;
                if workspace_graph::program_fingerprint(&analysis.program)? != expected {
                    return Err("The rebuilt source differs from the reviewed proposal".into());
                }
                Ok((saved, analysis, next_draft))
            },
            move |result| {
                if !ui.current() {
                    ui.stale();
                    return;
                }
                let (Some(state), Some(session), Some(model)) =
                    (ui.state.upgrade(), ui.session.upgrade(), ui.model.upgrade())
                else {
                    return;
                };
                session.borrow_mut().busy = false;
                match result {
                    Ok((saved, analysis, draft)) => {
                        let committed = state.borrow_mut().keep_graph_and_overrides(saved, draft);
                        match committed {
                            Ok(()) => {
                                *model.borrow_mut() = Some(Arc::new(analysis));
                                if let Some(status) = state.borrow().main_status.upgrade() {
                                    super::summary(&state, &status);
                                }
                                (ui.refresh)();
                                if let Some(window) = ui.window.upgrade() {
                                    window.close();
                                }
                            }
                            Err(e) => {
                                ui.message(&e);
                                ui.sensitive();
                            }
                        }
                    }
                    Err(e) => {
                        ui.message(&e);
                        ui.sensitive();
                    }
                }
            },
        );
    }
}
fn describe(
    program: &Program,
    draft: &Draft,
    review: &Review,
    choices: &BTreeMap<String, Choice>,
    result: &Resolution,
) -> Result<String, String> {
    use std::fmt::Write;
    override_review::validate_resolution(draft, review, choices, result)?;
    if review.program_sha256 != workspace_graph::program_fingerprint(program)? {
        return Err("The source review is stale".into());
    }
    let mut text = format!(
        "{} override(s) removed; {} remain separate.\n{} source expression(s) change.\n\n",
        draft.values.len() - result.remaining,
        result.remaining,
        result.plans.len()
    );
    for (id, choice) in choices.iter().filter(|(_, c)| **c != Choice::KeepSeparate) {
        let _ = writeln!(
            text,
            "{id}: {}",
            if *choice == Choice::UseSource {
                "use source; remove override"
            } else {
                "write override into source; remove override"
            }
        );
        if *choice == Choice::WriteOverride
            && !result
                .plans
                .iter()
                .flat_map(|p| &p.edits)
                .any(|e| e.semantic_targets.iter().any(|target| target == id))
        {
            let _=writeln!(text,"  The source already has the same modeled native stored value; its bytes stay unchanged.");
        }
    }
    let mut pieces = vec![];
    let mut current = program.clone();
    for plan in &result.plans {
        for edit in &plan.edits {
            let document = current
                .documents
                .iter()
                .find(|d| d.id == edit.document)
                .ok_or("Unknown proposed document")?;
            let old = document
                .source
                .get(edit.span.clone())
                .ok_or("Invalid proposed source range")?;
            let line = document.source[..edit.span.start]
                .bytes()
                .filter(|b| *b == b'\n')
                .count()
                + 1;
            let _ = write!(
                text,
                "\n{} · line {line}\nBefore: {}\nAfter: {}\n",
                hyprland_editor_core::source::display_path_bytes(&document.load_name),
                super::source_access::excerpt(old),
                super::source_access::excerpt(&edit.replacement)
            );
        }
        pieces = revisions::prepare_pieces(program, &pieces, plan)?;
        current = revisions::program(program.clone(), &pieces)?;
    }
    if workspace_graph::program_fingerprint(&current)? != result.program_sha256 {
        return Err("The proposed source fingerprint does not match its plans".into());
    }
    text.push_str(if result.model_complete{"\nModeled configuration calls are complete. Ownership and live behavior remain unverified."}else{"\nConfiguration analysis remains partial. Unknown source values and effects are unresolved."});
    Ok(text)
}
pub(super) fn open(
    parent: &gtk::Window,
    model: &Model,
    state: &super::Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    let analysis = model.borrow().clone().expect("Available source analysis");
    let session = Rc::new(RefCell::new({
        let s = state.borrow();
        Session {
            draft: s.draft.clone(),
            generation: s.generation,
            graph: s
                .source_graphs
                .get(&s.draft.version)
                .map(|g| g.fingerprint.clone()),
            choices: BTreeMap::new(),
            review: None,
            result: None,
            epoch: 0,
            busy: false,
            closed: false,
            rendering: false,
        }
    }));
    let window = adw::Window::builder()
        .title("Review Value Overrides")
        .transient_for(parent)
        .modal(true)
        .default_width(620)
        .default_height(760)
        .width_request(350)
        .height_request(420)
        .build();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    body.set_margin_start(16);
    body.set_margin_end(16);
    body.set_margin_top(10);
    body.set_margin_bottom(10);
    body.append(&caption("Choose how each value override relates to the private source draft. Every setting starts with Keep separate."));
    body.append(&caption("Setting"));
    let targets = gtk::DropDown::from_strings(&[]);
    super::monitors::compact(&targets);
    targets.set_enable_search(true);
    targets.set_widget_name("source-overrides-setting");
    targets.update_property(&[gtk::accessible::Property::Label("Override setting")]);
    body.append(&targets);
    let details = caption("");
    details.set_widget_name("source-overrides-details");
    body.append(&details);
    body.append(&caption("Resolution"));
    let choice = gtk::DropDown::from_strings(&["Keep separate"]);
    super::monitors::compact(&choice);
    choice.set_widget_name("source-overrides-choice");
    choice.update_property(&[gtk::accessible::Property::Label(
        "Source and override resolution",
    )]);
    body.append(&choice);
    let output = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(8)
        .right_margin(8)
        .top_margin(8)
        .bottom_margin(8)
        .build();
    output.set_widget_name("source-overrides-preview");
    output.update_property(&[gtk::accessible::Property::Label(
        "Proposed source and override changes",
    )]);
    body.append(
        &gtk::ScrolledWindow::builder()
            .child(&output)
            .min_content_height(160)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    outer.append(
        &gtk::ScrolledWindow::builder()
            .child(&body)
            .vexpand(true)
            .min_content_height(140)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    body.set_vexpand(true);
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 10);
    footer.set_margin_start(16);
    footer.set_margin_end(16);
    footer.set_margin_bottom(16);
    let status = caption("");
    status.set_widget_name("source-overrides-status");
    footer.append(&status);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let compare = gtk::Button::with_label("Compare");
    compare.set_widget_name("source-overrides-compare");
    compare.set_hexpand(true);
    actions.append(&compare);
    let keep = gtk::Button::with_label("Keep in Draft");
    keep.set_widget_name("source-overrides-keep");
    keep.set_hexpand(true);
    keep.add_css_class("suggested-action");
    actions.append(&keep);
    footer.append(&actions);
    footer.append(&caption("Keeping changes is private and can be undone together. Save Draft preserves them. Full source export and live activation remain unavailable."));
    outer.append(&footer);
    window.set_content(Some(&outer));
    let ui = Controls {
        state: Rc::downgrade(state),
        model: Rc::downgrade(model),
        analysis: Arc::downgrade(&analysis),
        session: Rc::downgrade(&session),
        refresh,
        window: window.downgrade(),
        targets: targets.downgrade(),
        choice: choice.downgrade(),
        details: details.downgrade(),
        output: output.downgrade(),
        status: status.downgrade(),
        compare: compare.downgrade(),
        keep: keep.downgrade(),
    };
    let u = ui.clone();
    targets.connect_selected_notify(move |_| u.select());
    let u = ui.clone();
    choice.connect_selected_notify(move |_| u.changed());
    let u = ui.clone();
    compare.connect_clicked(move |_| u.compare());
    let u = ui.clone();
    keep.connect_clicked(move |_| u.keep());
    window.connect_close_request(move |_| {
        let mut s = session.borrow_mut();
        s.closed = true;
        s.review = None;
        s.result = None;
        s.choices.clear();
        gtk::glib::Propagation::Proceed
    });
    let weak = window.downgrade();
    let u = ui.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
        if weak.upgrade().is_none_or(|w| !w.is_visible()) {
            return gtk::glib::ControlFlow::Break;
        }
        if !u.current() {
            u.stale();
            return gtk::glib::ControlFlow::Break;
        }
        gtk::glib::ControlFlow::Continue
    });
    ui.load();
    gtk::prelude::GtkWindowExt::set_focus(&window, Some(&targets));
    window.present();
    window
}

#[cfg(test)]
pub(super) fn test_review(parent: &adw::Window, state: &super::Shared) {
    use hyprland_editor_core::{Value, BASELINE};
    fn named(root: &gtk::Widget, name: &str) -> gtk::Widget {
        fn find(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
            if root.widget_name() == name {
                return Some(root.clone());
            }
            let mut next = root.first_child();
            while let Some(child) = next {
                if let Some(found) = find(&child, name) {
                    return Some(found);
                }
                next = child.next_sibling();
            }
            None
        }
        find(root, name).unwrap_or_else(|| panic!("Missing {name}"))
    }
    #[track_caller]
    fn wait(mut check: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if check() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "Override review timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    let before = state.borrow().source_graphs[BASELINE].clone();
    let generation = state.borrow().generation;
    let mut unreviewed = state.borrow().draft.clone();
    unreviewed
        .values
        .insert("general:border_size".into(), Value::Int(8));
    assert!(state
        .borrow_mut()
        .keep_graph_and_overrides(before.clone(), unreviewed)
        .is_err());
    assert_eq!(state.borrow().generation, generation);
    assert_eq!(
        state.borrow().source_graphs[BASELINE].fingerprint,
        before.fingerprint
    );
    // Exercise the actual inspector entry point before the focused reducer cases.
    let entry = named(parent.upcast_ref(), "source-analysis-overrides")
        .downcast::<gtk::Button>()
        .unwrap();
    assert!(entry.is_sensitive());
    entry.emit_clicked();
    let launched = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<adw::Window>().ok())
        .find(|w| w.is_visible() && w.title().as_deref() == Some("Review Value Overrides"))
        .unwrap();
    let ready = named(launched.upcast_ref(), "source-overrides-compare")
        .downcast::<gtk::Button>()
        .unwrap();
    wait(|| ready.is_sensitive());
    assert!(!named(launched.upcast_ref(), "source-overrides-keep")
        .downcast::<gtk::Button>()
        .unwrap()
        .is_sensitive());
    launched.close();
    wait(|| !Permit::running());
    let model: Model = Rc::new(RefCell::new(Some(Arc::new(
        source_graph::restore(&before.bundle, BASELINE).unwrap(),
    ))));
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        Value::Int(9)
    );
    let refreshed = Rc::new(RefCell::new(0));
    let create = || {
        let count = refreshed.clone();
        open(
            parent.upcast_ref(),
            &model,
            state,
            Rc::new(move || *count.borrow_mut() += 1),
        )
    };
    let window = create();
    let compare = named(window.upcast_ref(), "source-overrides-compare")
        .downcast::<gtk::Button>()
        .unwrap();
    wait(|| compare.is_sensitive());
    let choice = named(window.upcast_ref(), "source-overrides-choice")
        .downcast::<gtk::DropDown>()
        .unwrap();
    let details = named(window.upcast_ref(), "source-overrides-details")
        .downcast::<gtk::Label>()
        .unwrap();
    let keep = named(window.upcast_ref(), "source-overrides-keep")
        .downcast::<gtk::Button>()
        .unwrap();

    assert_eq!(choice.selected(), 0);
    assert!(!keep.is_sensitive());
    assert!(details.text().contains("Last modeled declaration: 6"));
    assert!(details.text().contains("Override: 9"));
    compare.emit_clicked();
    wait(|| compare.is_sensitive());
    assert!(!keep.is_sensitive());
    choice.set_selected(1);
    compare.emit_clicked();
    wait(|| keep.is_sensitive());
    let generation = state.borrow().generation;
    keep.emit_clicked();
    wait(|| !window.is_visible());
    assert_eq!(state.borrow().generation, generation + 1);
    assert!(state.borrow().draft.values.is_empty());
    assert_eq!(
        state.borrow().source_graphs[BASELINE].fingerprint,
        before.fingerprint
    );
    state.borrow_mut().undo_last();
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        Value::Int(9)
    );
    assert_eq!(
        state.borrow().source_graphs[BASELINE].fingerprint,
        before.fingerprint
    );
    let window = create();
    let compare = named(window.upcast_ref(), "source-overrides-compare")
        .downcast::<gtk::Button>()
        .unwrap();
    wait(|| compare.is_sensitive());
    let choice = named(window.upcast_ref(), "source-overrides-choice")
        .downcast::<gtk::DropDown>()
        .unwrap();
    let keep = named(window.upcast_ref(), "source-overrides-keep")
        .downcast::<gtk::Button>()
        .unwrap();
    choice.set_selected(2);
    compare.emit_clicked();
    wait(|| keep.is_sensitive());
    choice.set_selected(1);
    assert!(!keep.is_sensitive());
    choice.set_selected(2);
    compare.emit_clicked();
    wait(|| keep.is_sensitive());
    // An unrelated draft edit during rebuilding must reject both layer changes.
    keep.emit_clicked();
    {
        let mut s = state.borrow_mut();
        s.draft
            .values
            .insert("general:border_size".into(), Value::Int(8));
        s.generation += 1;
    }
    let status = named(window.upcast_ref(), "source-overrides-status")
        .downcast::<gtk::Label>()
        .unwrap();
    wait(|| status.text().contains("draft or source analysis changed"));
    wait(|| !Permit::running());
    assert_eq!(
        state.borrow().source_graphs[BASELINE].fingerprint,
        before.fingerprint
    );
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        Value::Int(8)
    );
    assert!(!keep.is_sensitive());
    window.close();
    {
        let mut s = state.borrow_mut();
        s.draft
            .values
            .insert("general:border_size".into(), Value::Int(9));
        s.generation += 1;
    }
    let window = create();
    let compare = named(window.upcast_ref(), "source-overrides-compare")
        .downcast::<gtk::Button>()
        .unwrap();
    wait(|| compare.is_sensitive());
    let choice = named(window.upcast_ref(), "source-overrides-choice")
        .downcast::<gtk::DropDown>()
        .unwrap();
    let keep = named(window.upcast_ref(), "source-overrides-keep")
        .downcast::<gtk::Button>()
        .unwrap();
    choice.set_selected(2);
    compare.emit_clicked();
    wait(|| keep.is_sensitive());
    let output = named(window.upcast_ref(), "source-overrides-preview")
        .downcast::<gtk::TextView>()
        .unwrap();
    let b = output.buffer();
    let text = b.text(&b.start_iter(), &b.end_iter(), true);
    assert!(text.contains("Before: 6\nAfter: 9"));
    assert!(text.contains("1 override(s) removed; 0 remain separate"));
    let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Override review needs {minimum}px");
    if std::env::var_os("HYPRLAND_EDITOR_TEST_EXACT_RESIZE").is_some() {
        window.set_default_size(360, 820);
        // GTK's client-side shadow may sit outside the widget allocation.
        wait(|| (350..=360).contains(&window.width()));
        let bounds = keep.compute_bounds(&window).unwrap();
        assert!(bounds.x() >= 0.0 && bounds.x() + bounds.width() <= window.width() as f32);
        assert!(bounds.y() >= 0.0 && bounds.y() + bounds.height() <= window.height() as f32);
        window.set_default_size(620, 760);
        wait(|| (610..=620).contains(&window.width()));
    }
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let start = std::time::Instant::now();
        wait(|| start.elapsed() >= std::time::Duration::from_millis(300));
        let paintable = gtk::WidgetPaintable::new(Some(&window));
        let node = RefCell::new(None);
        wait(|| {
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
            *node.borrow_mut() = snapshot.to_node();
            node.borrow().is_some()
        });
        window
            .renderer()
            .unwrap()
            .render_texture(node.into_inner().unwrap(), None)
            .save_to_png(format!("{path}.source-overrides.png"))
            .unwrap();
    }
    let generation = state.borrow().generation;
    keep.emit_clicked();
    wait(|| !window.is_visible());
    let after = state.borrow().source_graphs[BASELINE].clone();
    assert_eq!(state.borrow().generation, generation + 1);
    assert!(state.borrow().draft.values.is_empty());
    assert_eq!(*refreshed.borrow(), 2);
    assert!(Arc::ptr_eq(&before.bundle.sources, &after.bundle.sources));
    assert!(state.borrow().export_lua().is_err());
    assert!(model.borrow().as_ref().unwrap().program.documents[0]
        .source
        .contains("border_size=9"));
    let fixture = super::test_support::TempDir::new("he-override-review");
    let root = fixture.path().to_path_buf();
    let workspace = hyprland_editor_core::workspace::Workspace::new(state.borrow().draft.clone())
        .with_graph(&after.bundle);
    let path = hyprland_editor_core::workspace::save(&root, &workspace, &state.borrow().catalogue)
        .unwrap();
    let reopened = hyprland_editor_core::workspace::read(&path, &state.borrow().catalogue).unwrap();
    assert!(reopened.draft.values.is_empty());
    let bundle = workspace_graph::Bundle {
        graph: reopened.source_graph.unwrap(),
        sources: Arc::new(reopened.sources),
    };
    assert_eq!(bundle.fingerprint().unwrap(), after.fingerprint);
    assert!(source_graph::restore(&bundle, BASELINE)
        .unwrap()
        .program
        .documents[0]
        .source
        .contains("border_size=9"));
    std::fs::remove_dir_all(root).unwrap();
    state.borrow_mut().undo_last();
    assert_eq!(
        state.borrow().source_graphs[BASELINE].fingerprint,
        before.fingerprint
    );
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        Value::Int(9)
    );
    // Closing/replacing a model while inspection runs releases reports now;
    // only its bounded Program input is retained by the background work.
    let weak = Arc::downgrade(model.borrow().as_ref().unwrap());
    let window = create();
    model.borrow_mut().take();
    assert!(weak.upgrade().is_none());
    let status = named(window.upcast_ref(), "source-overrides-status")
        .downcast::<gtk::Label>()
        .unwrap();
    wait(|| status.text().contains("draft or source analysis changed"));
    window.close();
    wait(|| !Permit::running());
    drop((output, keep));
}
