//! Explicit, one-operation monitor layer reconciliation on private captured data.
use super::{source_analysis::Model, source_changes::Permit, source_graph, worker, workflow};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::replay::{
        monitor_edit::declarations::Anchor,
        monitor_review::{self, Choice, Resolution, Review},
    },
    monitor::Rule,
    workspace_graph,
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
};
#[cfg(test)]
static HOLD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn pin(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|b| {
            hyprland_editor_core::lua::source_hash(
                std::str::from_utf8(&b).expect("Serialized JSON"),
            )
        })
        .map_err(|e| e.to_string())
}
struct Session {
    monitors: Vec<Rule>,
    generation: u64,
    graph: Option<String>,
    review: Option<Box<Review>>,
    result: Option<(Choice, Box<Resolution>)>,
    epoch: u64,
    busy: bool,
    closed: bool,
}
#[derive(Clone)]
struct Ui {
    state: Weak<RefCell<super::State>>,
    model: Weak<RefCell<Option<Arc<source_graph::Analysis>>>>,
    analysis: std::sync::Weak<source_graph::Analysis>,
    session: Weak<RefCell<Session>>,
    refresh: Rc<dyn Fn()>,
    window: gtk::glib::WeakRef<adw::Window>,
    mode: gtk::glib::WeakRef<gtk::DropDown>,
    source: gtk::glib::WeakRef<gtk::DropDown>,
    draft: gtk::glib::WeakRef<gtk::DropDown>,
    file: gtk::glib::WeakRef<gtk::DropDown>,
    position: gtk::glib::WeakRef<gtk::DropDown>,
    details: gtk::glib::WeakRef<gtk::Label>,
    status: gtk::glib::WeakRef<gtk::Label>,
    output: gtk::glib::WeakRef<gtk::TextView>,
    compare: gtk::glib::WeakRef<gtk::Button>,
    keep: gtk::glib::WeakRef<gtk::Button>,
    source_group: gtk::glib::WeakRef<gtk::Box>,
    draft_group: gtk::glib::WeakRef<gtk::Box>,
    position_group: gtk::glib::WeakRef<gtk::Box>,
}
impl Ui {
    fn message(&self, text: &str) {
        if let Some(label) = self.status.upgrade() {
            label.set_text(&text.chars().take(1024).collect::<String>());
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
        let s = session.borrow();
        let state = state.borrow();
        !s.closed
            && state.generation == s.generation
            && state.draft.version == analysis.program.version
            && state
                .source_graphs
                .get(&state.draft.version)
                .map(|g| g.fingerprint.clone())
                == s.graph
            && model
                .borrow()
                .as_ref()
                .is_some_and(|m| Arc::ptr_eq(m, &analysis))
    }
    fn ready(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let s = session.borrow();
        let ready = !s.closed && !s.busy && s.review.is_some();
        for weak in [
            &self.mode,
            &self.source,
            &self.draft,
            &self.file,
            &self.position,
        ] {
            if let Some(w) = weak.upgrade() {
                w.set_sensitive(ready);
            }
        }
        if let Some(w) = self.compare.upgrade() {
            w.set_sensitive(
                !s.closed
                    && !s.busy
                    && (s.review.is_none()
                        || self.mode.upgrade().is_some_and(|m| m.selected() != 0)),
            );
            w.set_label(if s.review.is_some() {
                "Compare"
            } else {
                "Retry Review"
            });
        }
        if let Some(w) = self.keep.upgrade() {
            w.set_sensitive(ready && s.result.is_some());
        }
    }
    fn stale(&self) {
        if let Some(s) = self.session.upgrade() {
            let mut s = s.borrow_mut();
            s.closed = true;
            s.review = None;
            s.result = None;
        }
        if let Some(w) = self.output.upgrade() {
            w.buffer().set_text("");
        }
        self.message("The draft or source changed. Close this review and open it again.");
        self.ready();
    }
    fn changed(&self) {
        let (Some(session), Some(mode)) = (self.session.upgrade(), self.mode.upgrade()) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.closed {
            return;
        }
        s.epoch += 1;
        s.result = None;
        let selected = mode.selected();
        if let Some(w) = self.source_group.upgrade() {
            w.set_visible(matches!(selected, 1 | 2 | 4));
        }
        if let Some(w) = self.draft_group.upgrade() {
            w.set_visible(matches!(selected, 1 | 4 | 5));
        }
        if let Some(w) = self.position_group.upgrade() {
            w.set_visible(selected == 5);
        }
        let source = self.source.upgrade().and_then(|p| {
            s.review
                .as_ref()?
                .sources
                .get(p.selected() as usize)
                .cloned()
        });
        if let Some(w) = self.details.upgrade() {
            w.set_text(&source.map_or_else(||"Choose an explicit operation. Reached declarations describe requested values, not effective monitor state.".into(),|r|format!("Source call {} · {}{}\n{}",r.event_order,r.document,if r.repeated_site {" · repeated execution of one source site"} else {""},r.reason.unwrap_or_else(||r.rule.map_or_else(String::new,|r|format!("Output: {} · {} explicit fields. Unset fields remain inherited/unresolved.",r.output.text(),r.fields.len()))))));
        }
        drop(s);
        if let Some(w) = self.output.upgrade() {
            w.buffer().set_text("");
        }
        self.message("Compare the explicit choice before keeping it. Originals and the desktop stay unchanged.");
        self.ready();
    }
    fn choice(&self) -> Result<Choice, String> {
        let s = self.session.upgrade().ok_or("Review closed")?;
        let s = s.borrow();
        let mode = self.mode.upgrade().ok_or("Review closed")?.selected();
        let source = || {
            self.source
                .upgrade()
                .and_then(|p| {
                    s.review
                        .as_ref()?
                        .sources
                        .get(p.selected() as usize)
                        .map(|r| r.event_order)
                })
                .ok_or_else(|| "Choose a reached source declaration".to_string())
        };
        let draft = || {
            self.draft
                .upgrade()
                .and_then(|p| s.monitors.get(p.selected() as usize).map(|r| r.id.clone()))
                .ok_or_else(|| "Choose a monitor draft rule".to_string())
        };
        Ok(match mode {
            1 => Choice::CopySource {
                event_order: source()?,
                replace_id: Some(draft()?),
            },
            2 => Choice::CopySource {
                event_order: source()?,
                replace_id: None,
            },
            3 => Choice::CopyAllSources,
            4 => Choice::WriteDraft {
                draft_id: draft()?,
                event_order: source()?,
            },
            5 => Choice::InsertDraft {
                draft_id: draft()?,
                document: self
                    .file
                    .upgrade()
                    .and_then(|p| {
                        self.analysis
                            .upgrade()?
                            .program
                            .documents
                            .get(p.selected() as usize)
                            .map(|d| d.id.clone())
                    })
                    .ok_or("Choose a captured file")?,
                anchor: if self.position.upgrade().is_some_and(|p| p.selected() == 0) {
                    Anchor::Beginning
                } else {
                    Anchor::End
                },
            },
            _ => return Err("Choose how to reconcile these layers".into()),
        })
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
            Ok(p) => p,
            Err(e) => {
                self.message(&e);
                return;
            }
        };
        let program = analysis.program.clone();
        let monitors = session.borrow().monitors.clone();
        session.borrow_mut().busy = true;
        self.ready();
        self.message("Reviewing reached monitor declarations…");
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                match worker::run(worker::Request::ReviewSourceMonitors { program, monitors })? {
                    worker::Response::SourceMonitorReview(r) => Ok(r),
                    _ => Err("Unexpected monitor review response".into()),
                }
            },
            move |result| {
                if !ui.current() {
                    ui.stale();
                    return;
                }
                let (Some(analysis), Some(session)) = (ui.analysis.upgrade(), ui.session.upgrade())
                else {
                    return;
                };
                session.borrow_mut().busy = false;
                let result = result.and_then(|r| {
                    let s = session.borrow();
                    let events = analysis
                        .report
                        .events
                        .iter()
                        .filter(|e| e.api == "hl.monitor")
                        .collect::<Vec<_>>();
                    if r.schema_sha256 != workspace_graph::schema_sha256()
                        || r.version != analysis.program.version
                        || r.compiler != analysis.report.compiler_profile
                        || r.program_sha256
                            != workspace_graph::program_fingerprint(&analysis.program)?
                        || r.draft_sha256 != pin(&s.monitors)?
                        || r.model_complete != analysis.report.halt.is_none()
                        || r.sources.len() != events.len()
                        || r.sources.len() > 256
                        || r.sources.iter().zip(events).any(|(r, e)| {
                            r.event_order != e.order || r.document != e.document || r.span != e.span
                        })
                    {
                        return Err(
                            "Monitor review differs from the captured source or draft".into()
                        );
                    }
                    Ok(r)
                });
                match result {
                    Ok(r) => {
                        let names = r
                            .sources
                            .iter()
                            .enumerate()
                            .map(|(i, r)| {
                                format!(
                                    "{} · {}",
                                    i + 1,
                                    r.rule.as_ref().map_or_else(
                                        || "Needs source review".into(),
                                        |r| if r.output.text().is_empty() {
                                            "Fallback".into()
                                        } else {
                                            hyprland_editor_core::source::display_path_bytes(
                                                r.output.text().as_bytes(),
                                            )
                                            .chars()
                                            .take(96)
                                            .collect::<String>()
                                        }
                                    )
                                )
                            })
                            .collect::<Vec<String>>();
                        session.borrow_mut().review = Some(r);
                        if let Some(p) = ui.source.upgrade() {
                            p.set_model(Some(&gtk::StringList::new(
                                &names.iter().map(String::as_str).collect::<Vec<_>>(),
                            )));
                            p.set_selected(0);
                        }
                        ui.changed();
                    }
                    Err(e) => {
                        ui.message(&e);
                        ui.ready();
                    }
                }
            },
        );
    }
    fn compare(&self) {
        if self
            .session
            .upgrade()
            .is_some_and(|s| s.borrow().review.is_none())
        {
            self.load();
            return;
        }
        if !self.current() {
            self.stale();
            return;
        }
        let choice = match self.choice() {
            Ok(c) => c,
            Err(e) => {
                self.message(&e);
                return;
            }
        };
        let (Some(analysis), Some(session)) = (self.analysis.upgrade(), self.session.upgrade())
        else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.busy || s.closed {
            return;
        }
        let Some(review) = s.review.clone() else {
            return;
        };
        let permit = match Permit::acquire() {
            Ok(p) => p,
            Err(e) => {
                drop(s);
                self.message(&e);
                return;
            }
        };
        let epoch = s.epoch;
        let monitors = s.monitors.clone();
        let program = analysis.program.clone();
        s.busy = true;
        s.result = None;
        drop(s);
        self.ready();
        self.message("Comparing the selected monitor operation…");
        let ui = self.clone();
        let requested = choice.clone();
        workflow::background(
            move || {
                let _permit = permit;
                #[cfg(test)]
                while HOLD.load(std::sync::atomic::Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                match worker::run(worker::Request::ResolveSourceMonitors {
                    program,
                    monitors,
                    review: *review,
                    choice: requested,
                })? {
                    worker::Response::SourceMonitorResolution(r) => Ok(r),
                    _ => Err("Unexpected monitor comparison response".into()),
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
                let mut s = session.borrow_mut();
                s.busy = false;
                if s.epoch != epoch {
                    drop(s);
                    ui.message("The choice changed. Compare again before keeping it.");
                    ui.ready();
                    return;
                }
                let result = result.and_then(|r| {
                    monitor_review::validate_resolution(
                        &s.monitors,
                        s.review.as_ref().ok_or("Monitor review unavailable")?,
                        &choice,
                        &r,
                    )?;
                    Ok(r)
                });
                match result {
                    Ok(r) => {
                        let mut text = format!(
                            "Monitor drafts: {} → {} ordered rules\nSource change steps: {}\n\n",
                            s.monitors.len(),
                            r.monitors.len(),
                            r.plans.len()
                        );
                        for (name, rules) in
                            [("Draft before", &s.monitors), ("Draft after", &r.monitors)]
                        {
                            let printed = hyprland_editor_core::monitor::export_lua(
                                rules,
                                s.review.as_ref().map_or("", |r| r.version.as_str()),
                            )
                            .unwrap_or_else(|e| format!("Draft summary unavailable: {e}"));
                            text.push_str(&format!(
                                "{name}\n{}\n{}\n",
                                printed.chars().take(4096).collect::<String>(),
                                if printed.chars().count() > 4096 {
                                    "Draft excerpt shortened; all ordered rules are retained."
                                } else {
                                    ""
                                }
                            ));
                        }
                        for (i, p) in r.previews.iter().take(32).enumerate() {
                            text.push_str(&format!(
                                "Patch {} · {}\nBefore\n{}\n\nAfter\n{}\n{}\n",
                                i + 1,
                                p.document,
                                if p.before.is_empty() {
                                    "(insertion point)"
                                } else {
                                    &p.before
                                },
                                if p.after.trim().is_empty() {
                                    "(removed)"
                                } else {
                                    &p.after
                                },
                                if p.truncated {
                                    "Excerpt shortened; complete edit stays in the private plan."
                                } else {
                                    ""
                                }
                            ));
                        }
                        if r.previews.len() > 32 {
                            text.push_str("Further retained patches omitted from this view.\n");
                        }
                        text.push_str("Only the explicit draft/source operation is retained. Conditional behavior, native rule inheritance and hardware remain unverified.\n");
                        if let Some(w) = ui.output.upgrade() {
                            w.buffer().set_text(&text);
                        }
                        s.result = Some((choice, r));
                        drop(s);
                        ui.message("Comparison ready. Keep it as one private draft operation, or change the choice.");
                    }
                    Err(e) => {
                        drop(s);
                        ui.message(&e);
                    }
                }
                ui.ready();
            },
        );
    }
    fn keep(&self) {
        if !self.current() {
            self.stale();
            return;
        }
        let (Some(analysis), Some(session), Some(state)) = (
            self.analysis.upgrade(),
            self.session.upgrade(),
            self.state.upgrade(),
        ) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.busy || s.closed {
            return;
        }
        let Some((choice, result)) = s.result.clone() else {
            return;
        };
        let Some(review) = s.review.as_ref() else {
            return;
        };
        if let Err(e) = monitor_review::validate_resolution(&s.monitors, review, &choice, &result) {
            drop(s);
            self.message(&e);
            return;
        }
        let frozen = match source_graph::freeze(&analysis) {
            Ok(b) => b,
            Err(e) => {
                drop(s);
                self.message(&e);
                return;
            }
        };
        let bundle =
            if let Some(saved) = state.borrow().source_graphs.get(&analysis.program.version) {
                if frozen.fingerprint().ok().as_ref() != Some(&saved.fingerprint) {
                    drop(s);
                    self.message("Reopen the retained source view before reconciling it.");
                    return;
                }
                saved.bundle.clone()
            } else {
                frozen
            };
        let permit = match Permit::acquire() {
            Ok(p) => p,
            Err(e) => {
                drop(s);
                self.message(&e);
                return;
            }
        };
        let epoch = s.epoch;
        s.busy = true;
        drop(s);
        self.ready();
        self.message("Rebuilding the private source and monitor draft…");
        let version = analysis.program.version.clone();
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                let (saved, analysis) =
                    source_graph::apply_revisions(&bundle, &version, &result.plans)?;
                if workspace_graph::program_fingerprint(&analysis.program)? != result.program_sha256
                {
                    return Err(
                        "The rebuilt source differs from the reviewed monitor operation".into(),
                    );
                }
                Ok((saved, analysis, result.monitors))
            },
            move |result| {
                if !ui.current() {
                    ui.stale();
                    return;
                }
                let (Some(state), Some(model), Some(session)) =
                    (ui.state.upgrade(), ui.model.upgrade(), ui.session.upgrade())
                else {
                    return;
                };
                {
                    let mut s = session.borrow_mut();
                    s.busy = false;
                    if s.epoch != epoch {
                        s.result = None;
                        drop(s);
                        ui.message("The choice changed during the rebuild. Compare again.");
                        ui.ready();
                        return;
                    }
                }
                match result {
                    Ok((saved, analysis, monitors)) => {
                        let kept = state.borrow_mut().keep_graph_and_monitors(saved, monitors);
                        match kept {
                            Ok(()) => {
                                *model.borrow_mut() = Some(Arc::new(analysis));
                                if let Some(status) = state.borrow().main_status.upgrade() {
                                    super::summary(&state, &status);
                                }
                                (ui.refresh)();
                                if let Some(w) = ui.window.upgrade() {
                                    w.close();
                                }
                            }
                            Err(e) => ui.message(&e),
                        }
                    }
                    Err(e) => ui.message(&e),
                }
                ui.ready();
            },
        );
    }
}
pub(super) fn open(
    parent: &gtk::Window,
    model: &Model,
    state: &super::Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    let analysis = model.borrow().clone().expect("Selected source analysis");
    let version = analysis.program.version.clone();
    let session = Rc::new(RefCell::new(Session {
        monitors: state
            .borrow()
            .monitors
            .get(&version)
            .cloned()
            .unwrap_or_default(),
        generation: state.borrow().generation,
        graph: state
            .borrow()
            .source_graphs
            .get(&version)
            .map(|g| g.fingerprint.clone()),
        review: None,
        result: None,
        epoch: 0,
        busy: false,
        closed: false,
    }));
    let window = adw::Window::builder()
        .title("Review Monitor Drafts")
        .transient_for(parent)
        .modal(true)
        .default_width(620)
        .default_height(740)
        .width_request(350)
        .height_request(420)
        .build();
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let title = gtk::Label::builder()
        .label("Monitor drafts")
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    title.add_css_class("title");
    header.set_title_widget(Some(&title));
    toolbar.add_top_bar(&header);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for f in [
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
    ] {
        f(body.upcast_ref(), 16);
    }
    body.append(&super::label(
        "Reconcile monitor drafts with captured source. Originals and the desktop stay unchanged.",
    ));
    let form = gtk::Box::new(gtk::Orientation::Vertical, 8);
    fn pick(body: &gtk::Box, name: &str, label: &str, values: &[&str]) -> gtk::DropDown {
        body.append(&super::label(label));
        let p = gtk::DropDown::from_strings(values);
        super::monitors::compact(&p);
        p.set_enable_search(true);
        p.set_widget_name(name);
        p.update_property(&[gtk::accessible::Property::Label(label)]);
        body.append(&p);
        p
    }
    let mode = pick(
        &form,
        "monitor-review-operation",
        "Operation",
        &[
            "Keep separate",
            "Copy source into selected draft rule",
            "Append source as a new draft rule",
            "Replace draft sequence with reached source",
            "Write selected draft into source",
            "Insert selected draft into source",
        ],
    );
    let source_group = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let source = pick(
        &source_group,
        "monitor-review-source",
        "Reached source declaration",
        &[],
    );
    form.append(&source_group);
    let draft_group = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let names = session
        .borrow()
        .monitors
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{} · {}",
                i + 1,
                hyprland_editor_core::source::display_path_bytes(r.output.text().as_bytes())
                    .chars()
                    .take(96)
                    .collect::<String>()
            )
        })
        .collect::<Vec<_>>();
    let draft = pick(
        &draft_group,
        "monitor-review-draft",
        "Monitor draft rule",
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    form.append(&draft_group);
    let position_group = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let names = analysis
        .program
        .documents
        .iter()
        .map(|d| {
            hyprland_editor_core::source::display_path_bytes(&d.load_name)
                .chars()
                .take(128)
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let file = pick(
        &position_group,
        "monitor-review-file",
        "Captured file",
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let position = pick(
        &position_group,
        "monitor-review-position",
        "Insertion position",
        &["Beginning of file", "End (before final return)"],
    );
    position.set_selected(1);
    form.append(&position_group);
    let details = super::label("");
    details.set_widget_name("monitor-review-details");
    form.append(&details);
    details.set_lines(4);
    details.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let form_area = gtk::ScrolledWindow::builder()
        .child(&form)
        .min_content_height(140)
        .max_content_height(220)
        .propagate_natural_height(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    body.append(&form_area);
    let output = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    output.set_widget_name("monitor-review-output");
    let area = gtk::ScrolledWindow::builder()
        .child(&output)
        .min_content_height(140)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    body.append(&area);
    let status = super::label("");
    status.set_widget_name("monitor-review-status");
    status.set_lines(3);
    status.set_ellipsize(gtk::pango::EllipsizeMode::End);
    body.append(&status);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let compare = gtk::Button::with_label("Compare");
    compare.set_widget_name("monitor-review-compare");
    let keep = gtk::Button::with_label("Keep in Draft");
    keep.set_widget_name("monitor-review-keep");
    keep.add_css_class("suggested-action");
    compare.set_hexpand(true);
    keep.set_hexpand(true);
    actions.append(&compare);
    actions.append(&keep);
    body.append(&actions);
    let limits=gtk::Expander::builder().label("Scope and limits").child(&super::label("Only reached, representable requested values can be copied. Replacing the whole draft uses their observed order; it does not copy conditional code or resolve inherited monitor state.")).build();
    body.append(&limits);
    toolbar.set_content(Some(&body));
    window.set_content(Some(&toolbar));
    let ui = Ui {
        state: Rc::downgrade(state),
        model: Rc::downgrade(model),
        analysis: Arc::downgrade(&analysis),
        session: Rc::downgrade(&session),
        refresh,
        window: window.downgrade(),
        mode: mode.downgrade(),
        source: source.downgrade(),
        draft: draft.downgrade(),
        file: file.downgrade(),
        position: position.downgrade(),
        details: details.downgrade(),
        status: status.downgrade(),
        output: output.downgrade(),
        compare: compare.downgrade(),
        keep: keep.downgrade(),
        source_group: source_group.downgrade(),
        draft_group: draft_group.downgrade(),
        position_group: position_group.downgrade(),
    };
    for p in [mode, source, draft, file, position] {
        let ui = ui.clone();
        p.connect_selected_notify(move |_| ui.changed());
    }
    let u = ui.clone();
    compare.connect_clicked(move |_| u.compare());
    let u = ui.clone();
    keep.connect_clicked(move |_| u.keep());
    window.connect_destroy(move |_| {
        let mut s = session.borrow_mut();
        s.closed = true;
        s.review = None;
        s.result = None;
        s.monitors.clear();
    });
    ui.changed();
    ui.load();
    window.present();
    window
}

#[cfg(test)]
pub(super) fn test_workflow(parent: &adw::ApplicationWindow) {
    use hyprland_editor_core::{
        lua::LuaDocument, module_path::Context, monitor::Output, storage, workspace::Workspace,
        Value, AUDITED_MAIN, BASELINE,
    };
    fn find(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
        if root.widget_name() == name {
            return Some(root.clone());
        }
        let mut child = root.first_child();
        while let Some(w) = child {
            if let Some(found) = find(&w, name) {
                return Some(found);
            }
            child = w.next_sibling();
        }
        None
    }
    fn named(root: &gtk::Widget, name: &str) -> gtk::Widget {
        find(root, name).unwrap_or_else(|| panic!("Missing {name}"))
    }
    #[track_caller]
    fn wait(mut done: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if done() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(12),
                "Monitor review timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    fn dropdown(root: &gtk::Widget, name: &str) -> gtk::DropDown {
        named(root, name).downcast().unwrap()
    }
    fn button(root: &gtk::Widget, name: &str) -> gtk::Button {
        named(root, name).downcast().unwrap()
    }
    for version in [BASELINE, AUDITED_MAIN] {
        let dir = std::env::temp_dir().join(format!(
            "hye-monitor-review-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let original="-- monitor review header\nlocal t={output='DP-1',transform=1,reserved={top=-1 --[[keep gap]],right='2',bottom=3,left=4,note='stay'}};hl.monitor(t);hl.monitor{output='DP-3',transform=2};hl.config{general={border_size=1}}\n";
        let path = dir.join("main.lua");
        std::fs::write(&path, original).unwrap();
        let captured = Arc::new(storage::read_source(&path).unwrap());
        let analysis = source_graph::analyze(
            captured.clone(),
            Context {
                main_file: captured.identity.selected_path.clone(),
                home: None,
                inherited_package_path: None,
            },
            vec![],
            version.into(),
            None,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .unwrap();
        let model: Model = Rc::new(RefCell::new(Some(Arc::new(analysis))));
        let state = super::initial_state();
        let worker::Response::Parsed(report) = worker::run(worker::Request::Parse {
            source: original.into(),
            version: version.into(),
        })
        .unwrap() else {
            panic!()
        };
        let mut rule = Rule::new("draft");
        rule.output = Output::Connector("DP-2".into());
        rule.fields.insert("transform".into(), Value::Int(3));
        rule.fields
            .insert("reserved".into(), Value::CssGap([-8, 2, 3, 4]));
        {
            let mut s = state.borrow_mut();
            s.draft = hyprland_editor_core::Draft::new(version);
            s.draft
                .values
                .insert("general:border_size".into(), Value::Int(9));
            let doc = LuaDocument::from_report(original.into(), *report, &s.catalogue).unwrap();
            s.sources.insert(version.into(), Arc::new(doc));
            s.source_origins
                .insert(version.into(), captured.identity.clone());
            s.monitors.insert(version.into(), vec![rule.clone()]);
        }
        let (pane, view) = super::source_analysis::build(&model, &state, None);
        let holder = adw::Window::builder()
            .transient_for(parent)
            .content(&pane)
            .default_width(620)
            .default_height(740)
            .build();
        holder.present();
        view.render();
        let launch = button(pane.upcast_ref(), "source-analysis-monitor-review");
        assert!(launch.is_sensitive());
        launch.emit_clicked();
        let window = gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<adw::Window>().ok())
            .find(|w| w.title().as_deref() == Some("Review Monitor Drafts") && w.is_visible())
            .unwrap();
        let mode = dropdown(window.upcast_ref(), "monitor-review-operation");
        wait(|| mode.is_sensitive());
        assert_eq!(mode.selected(), 0);
        let compare = button(window.upcast_ref(), "monitor-review-compare");
        let keep = button(window.upcast_ref(), "monitor-review-keep");
        assert!(!compare.is_sensitive() && !keep.is_sensitive());
        mode.set_selected(4);
        HOLD.store(true, std::sync::atomic::Ordering::Release);
        compare.emit_clicked();
        mode.set_selected(2);
        HOLD.store(false, std::sync::atomic::Ordering::Release);
        wait(|| compare.is_sensitive());
        assert!(
            !keep.is_sensitive(),
            "A changed operation retained a stale monitor comparison"
        );
        mode.set_selected(4);
        compare.emit_clicked();
        wait(|| keep.is_sensitive());
        let output = named(window.upcast_ref(), "monitor-review-output")
            .downcast::<gtk::TextView>()
            .unwrap();
        let text = output.buffer().text(
            &output.buffer().start_iter(),
            &output.buffer().end_iter(),
            true,
        );
        assert!(
            text.contains("Before") && text.contains("After") && text.contains("-8"),
            "{text}"
        );
        let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
        assert!(minimum <= 350, "Monitor reconciliation needs {minimum}px");
        window.set_default_size(360, 740);
        wait(|| (350..=360).contains(&window.width()));
        let bounds = keep.compute_bounds(&window).unwrap();
        assert!(bounds.x() >= 0. && bounds.x() + bounds.width() <= window.width() as f32);
        assert!(bounds.y() >= 0. && bounds.y() + bounds.height() <= window.height() as f32);
        if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
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
                .save_to_png(format!("{path}.monitor-review.png"))
                .unwrap();
        }
        keep.emit_clicked();
        wait(|| state.borrow().source_graphs.contains_key(version));
        assert_eq!(state.borrow().monitors[version], vec![rule.clone()]);
        assert_eq!(
            state.borrow().draft.values["general:border_size"],
            Value::Int(9)
        );
        let first = state.borrow().source_graphs[version].clone();
        let source = source_graph::restore(&first.bundle, version).unwrap();
        assert!(source.program.documents[0]
            .source
            .contains("--[[keep gap]]"));
        assert!(source.program.documents[0].source.contains("note='stay'"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let window = open(parent.upcast_ref(), &model, &state, Rc::new(|| {}));
        let mode = dropdown(window.upcast_ref(), "monitor-review-operation");
        wait(|| mode.is_sensitive());
        mode.set_selected(1);
        dropdown(window.upcast_ref(), "monitor-review-source").set_selected(1);
        let compare = button(window.upcast_ref(), "monitor-review-compare");
        compare.emit_clicked();
        let keep = button(window.upcast_ref(), "monitor-review-keep");
        wait(|| keep.is_sensitive());
        keep.emit_clicked();
        wait(|| state.borrow().monitors[version][0].output == Output::Connector("DP-3".into()));
        assert_eq!(state.borrow().monitors[version][0].id, "draft");
        assert_eq!(
            state.borrow().source_graphs[version].fingerprint,
            first.fingerprint
        );
        let saved = Workspace::new(state.borrow().draft.clone())
            .with_monitors(state.borrow().monitors[version].clone())
            .with_graph(&state.borrow().source_graphs[version].bundle)
            .to_json(&state.borrow().catalogue)
            .unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let reopened = Workspace::from_json(
            std::str::from_utf8(&saved).unwrap(),
            &state.borrow().catalogue,
        )
        .unwrap();
        assert_eq!(reopened.monitors, state.borrow().monitors[version]);
        let bundle = workspace_graph::Bundle {
            graph: reopened.source_graph.unwrap(),
            sources: Arc::new(reopened.sources),
        };
        assert!(source_graph::restore(&bundle, version).is_ok());
        state.borrow_mut().undo_last();
        assert_eq!(state.borrow().monitors[version], vec![rule.clone()]);
        assert_eq!(
            state.borrow().draft.values["general:border_size"],
            Value::Int(9)
        );
        assert_eq!(
            state.borrow().source_graphs[version].fingerprint,
            first.fingerprint
        );
        state.borrow_mut().undo_last();
        assert!(!state.borrow().source_graphs.contains_key(version));
        assert_eq!(state.borrow().monitors[version], vec![rule]);
        holder.close();
    }
}
