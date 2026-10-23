//! Explicit external graph choices. No live configuration writer or restored grants.
use super::{label, monitors::compact, source_graph, worker, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::LuaDocument,
    source::{display_path_bytes, SourceIdentity},
    source_merge::{self, Choice},
    workspace_graph::merge,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

struct Candidate {
    saved: Arc<source_graph::Saved>,
    document: Arc<LuaDocument>,
    origin: SourceIdentity,
    analysis: Arc<source_graph::Analysis>,
}
struct File {
    inputs: merge::FileInputs,
    decision: merge::Decision,
}
struct Session {
    old: Arc<source_graph::Saved>,
    incoming: Arc<source_graph::Saved>,
    generation: u64,
    root_hash: String,
    review: Option<merge::Review>,
    selection: merge::Selection,
    decisions: BTreeMap<String, merge::Decision>,
    file: Option<File>,
    candidate: Option<Candidate>,
    model: super::source_analysis::Model,
    epoch: u64,
    pending: bool,
    closed: bool,
    cancel: Arc<AtomicBool>,
}
impl Session {
    fn current(&self, state: &Shared) -> bool {
        let s = state.borrow();
        !self.closed
            && s.generation == self.generation
            && s.draft.version == self.old.version
            && s.source_graphs
                .get(&self.old.version)
                .is_some_and(|g| g.fingerprint == self.old.fingerprint)
            && s.sources
                .get(&self.old.version)
                .is_some_and(|d| d.report.source_sha256 == self.root_hash)
    }
    fn invalidate(&mut self) {
        self.epoch += 1;
        self.file = None;
        self.decisions.clear();
        self.candidate = None;
        self.model.borrow_mut().take();
    }
}
#[derive(Clone)]
struct Controls {
    session: Weak<RefCell<Session>>,
    state: Weak<RefCell<super::State>>,
    updating: Rc<Cell<bool>>,
    window: gtk::glib::WeakRef<adw::Window>,
    pages: gtk::glib::WeakRef<adw::ViewStack>,
    old_file: gtk::glib::WeakRef<gtk::DropDown>,
    destination: gtk::glib::WeakRef<gtk::DropDown>,
    context: gtk::glib::WeakRef<gtk::CheckButton>,
    path: gtk::glib::WeakRef<gtk::Label>,
    next: gtk::glib::WeakRef<gtk::Button>,
    text_file: gtk::glib::WeakRef<gtk::DropDown>,
    inspect: gtk::glib::WeakRef<gtk::Button>,
    branch: gtk::glib::WeakRef<gtk::DropDown>,
    page: gtk::glib::WeakRef<gtk::SpinButton>,
    text: gtk::glib::WeakRef<gtk::TextView>,
    conflict: gtk::glib::WeakRef<gtk::DropDown>,
    choice: gtk::glib::WeakRef<gtk::DropDown>,
    use_file: gtk::glib::WeakRef<gtk::Button>,
    build: gtk::glib::WeakRef<gtk::Button>,
    keep: gtk::glib::WeakRef<gtk::Button>,
    stop: gtk::glib::WeakRef<gtk::Button>,
    partial: gtk::glib::WeakRef<gtk::CheckButton>,
    status: gtk::glib::WeakRef<gtk::Label>,
    analysis: super::source_analysis::View,
}
fn text_area(name: &str) -> (gtk::ScrolledWindow, gtk::TextView) {
    let text = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    text.set_widget_name(name);
    text.update_property(&[gtk::accessible::Property::Label(
        "Selected source or conflict bytes",
    )]);
    let area = gtk::ScrolledWindow::builder()
        .child(&text)
        .min_content_height(180)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    (area, text)
}
fn pick(name: &str, labels: &[&str]) -> gtk::DropDown {
    let p = gtk::DropDown::from_strings(labels);
    p.set_widget_name(name);
    compact(&p);
    p.update_property(&[gtk::accessible::Property::Label(name)]);
    p
}
fn button(name: &str, text: &str) -> gtk::Button {
    let b = gtk::Button::with_label(text);
    b.set_widget_name(name);
    b
}
fn page() -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for f in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        f(b.upcast_ref(), 16);
    }
    b
}
fn add_page(stack: &adw::ViewStack, body: &gtk::Box, id: &str, title: &str) {
    let area = gtk::ScrolledWindow::builder()
        .child(body)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();
    stack.add_titled_with_icon(
        &area,
        Some(id),
        title,
        match id {
            "matches" => "folder-symbolic",
            "text" => "document-edit-symbolic",
            _ => "emblem-ok-symbolic",
        },
    );
}
fn view_name(view: &merge::View) -> String {
    format!(
        "{} · {}{}",
        display_path_bytes(&view.selected_path),
        view.document,
        if view.archived {
            " · archived"
        } else if view.private_input {
            " · private capture"
        } else {
            ""
        }
    )
}
fn context_description(
    bundle: &hyprland_editor_core::workspace_graph::Bundle,
    version: &str,
) -> Result<String, String> {
    fn optional(value: Option<&[u8]>) -> String {
        value
            .map(|v| {
                if v.is_empty() {
                    "Known empty".into()
                } else {
                    display_path_bytes(v)
                }
            })
            .unwrap_or_else(|| "Unknown".into())
    }
    let context = &bundle.graph.context;
    let (path, unknown) = context.initial_path(version)?;
    Ok(format!("Main file: {}\nHome: {}\nWorking folder: {}\nInherited search path: {}\nInitial search path ({}): {}",
        display_path_bytes(&context.main_file),optional(context.home.as_deref()),optional(bundle.graph.working_directory.as_deref()),optional(context.inherited_package_path.as_deref()),
        if unknown{"unknown tail remains"}else{"complete captured assumption"},display_path_bytes(&path)))
}
fn page_span(text: &str, page: usize) -> (std::ops::Range<usize>, usize) {
    let count = text.len().div_ceil(16 * 1024).max(1);
    let page = page.clamp(1, count) - 1;
    let mut start = (page * 16 * 1024).min(text.len());
    let mut end = ((page + 1) * 16 * 1024).min(text.len());
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (start..end, count)
}
#[cfg(test)]
static HOLD: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static ENTERED: AtomicBool = AtomicBool::new(false);
fn check(cancel: &AtomicBool) -> Result<(), String> {
    #[cfg(test)]
    while HOLD.load(Ordering::Acquire) {
        ENTERED.store(true, Ordering::Release);
        if cancel.load(Ordering::Acquire) {
            return Err("Merge review stopped".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    if cancel.load(Ordering::Acquire) {
        Err("Merge review stopped".into())
    } else {
        Ok(())
    }
}
impl Controls {
    fn feedback(&self, text: &str) {
        if let Some(s) = self.status.upgrade() {
            s.set_text(&display_path_bytes(text.as_bytes()));
        }
    }
    fn begin(&self) -> Option<(u64, Arc<AtomicBool>, super::source_changes::Permit)> {
        let (session, state) = (self.session.upgrade()?, self.state.upgrade()?);
        let mut s = session.borrow_mut();
        if s.pending || !s.current(&state) {
            return None;
        }
        let permit = match super::source_changes::Permit::acquire() {
            Ok(p) => p,
            Err(e) => {
                drop(s);
                self.feedback(&e);
                return None;
            }
        };
        s.pending = true;
        s.epoch += 1;
        s.cancel = Arc::new(AtomicBool::new(false));
        let out = (s.epoch, s.cancel.clone(), permit);
        drop(s);
        self.refresh();
        Some(out)
    }
    fn finish(&self, epoch: u64) -> Option<Rc<RefCell<Session>>> {
        let (session, state) = (self.session.upgrade()?, self.state.upgrade()?);
        {
            let mut s = session.borrow_mut();
            if s.epoch != epoch || !s.current(&state) {
                return None;
            }
            s.pending = false;
        }
        self.refresh();
        Some(session)
    }
    fn refresh(&self) {
        let (Some(session), Some(state)) = (self.session.upgrade(), self.state.upgrade()) else {
            return;
        };
        let s = session.borrow();
        let available = s.current(&state) && !s.pending;
        for weak in [
            &self.old_file,
            &self.destination,
            &self.text_file,
            &self.branch,
            &self.conflict,
            &self.choice,
        ] {
            if let Some(w) = weak.upgrade() {
                w.set_sensitive(available);
            }
        }
        if let (Some(pick), Some(destination)) =
            (self.old_file.upgrade(), self.destination.upgrade())
        {
            if s.review
                .as_ref()
                .and_then(|r| r.old_views.get(pick.selected() as usize))
                .is_some_and(|v| v.document == s.old.bundle.graph.root)
            {
                destination.set_sensitive(false);
            }
        }
        if let Some(w) = self.context.upgrade() {
            w.set_sensitive(available);
        }
        if let Some(b) = self.next.upgrade() {
            b.set_sensitive(
                available
                    && s.review
                        .as_ref()
                        .is_some_and(|r| merge::validate_selection(r, &s.selection).is_ok()),
            );
        }
        if let Some(b) = self.inspect.upgrade() {
            b.set_sensitive(available && s.review.is_some());
        }
        if let Some(b) = self.use_file.upgrade() {
            b.set_sensitive(available && s.file.is_some());
        }
        if let Some(b) = self.build.upgrade() {
            b.set_sensitive(
                available
                    && s.review
                        .as_ref()
                        .is_some_and(|r| merge::validate_selection(r, &s.selection).is_ok())
                    && s.selection.pairs.values().filter(|x| x.is_some()).count()
                        == s.decisions.len(),
            );
        }
        if let Some(b) = self.keep.upgrade() {
            b.set_sensitive(
                available
                    && s.candidate.as_ref().is_some_and(|c| {
                        c.analysis.blocked.is_none() && c.analysis.report.halt.is_none()
                            || self.partial.upgrade().is_some_and(|p| p.is_active())
                    }),
            );
        }
        if let Some(b) = self.stop.upgrade() {
            b.set_visible(s.pending);
            b.set_sensitive(!s.cancel.load(Ordering::Acquire));
        }
    }
    fn matching(&self) {
        let (Some(session), Some(old), Some(destination), Some(info)) = (
            self.session.upgrade(),
            self.old_file.upgrade(),
            self.destination.upgrade(),
            self.path.upgrade(),
        ) else {
            return;
        };
        let s = session.borrow();
        let Some(r) = &s.review else {
            return;
        };
        let Some(v) = r.old_views.get(old.selected() as usize) else {
            return;
        };
        let mut names = vec![
            "Choose what happens to this file".into(),
            "Keep as an inactive archive".into(),
            "Discard from this draft".into(),
        ];
        names.extend(r.incoming_views.iter().map(view_name));
        let selected = match s.selection.pairs.get(&v.document) {
            Some(Some(id)) => r
                .incoming_views
                .iter()
                .position(|v| &v.document == id)
                .map_or(0, |n| n as u32 + 3),
            Some(None) => {
                if s.selection.archive_private.contains(&v.document) {
                    1
                } else {
                    2
                }
            }
            None => 0,
        };
        let root = v.document == s.old.bundle.graph.root;
        let text=format!("{}\nLoad name: {}\nCaptured: {}\nPrivate: {}\n{}\n{} of {} file choices made. Selecting a path pairs captured data; it grants no file access.",
            view_name(v),display_path_bytes(&v.load_name),v.captured_sha256,v.private_sha256,
            if v.private_changed {"Contains private source edits, including retained piece provenance."}else{"No private piece revision."},s.selection.pairs.len(),r.rows.len());
        drop(s);
        self.updating.set(true);
        // Incoming views are immutable during this review. Replacing the
        // model from its own selected callback can produce nested GTK model
        // transitions; install it once and only reflect the chosen position.
        if destination
            .model()
            .is_none_or(|m| m.n_items() != names.len() as u32)
        {
            destination.set_model(Some(&gtk::StringList::new(
                &names.iter().map(String::as_str).collect::<Vec<_>>(),
            )));
        }
        destination.set_selected(selected);
        self.updating.set(false);
        info.set_text(&text);
        self.refresh();
        if root {
            destination.set_sensitive(false);
        }
    }
    fn map(&self) {
        if self.updating.get() {
            return;
        }
        let (Some(session), Some(state), Some(old), Some(pick)) = (
            self.session.upgrade(),
            self.state.upgrade(),
            self.old_file.upgrade(),
            self.destination.upgrade(),
        ) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.pending || !s.current(&state) {
            return;
        }
        let Some(r) = &s.review else {
            return;
        };
        let Some(v) = r.old_views.get(old.selected() as usize).cloned() else {
            return;
        };
        if pick.selected() == gtk::INVALID_LIST_POSITION {
            return;
        }
        if v.document == s.old.bundle.graph.root {
            return;
        }
        let target = r
            .incoming_views
            .get(pick.selected().saturating_sub(3) as usize)
            .map(|v| v.document.clone());
        // GtkDropDown may deliver a coalesced selected notification after the
        // model-update guard is released. Reflecting the same choice must not
        // rebuild its model, invalidate decisions or start a notification loop.
        let unchanged = match pick.selected() {
            0 => !s.selection.pairs.contains_key(&v.document),
            1 => {
                s.selection.pairs.get(&v.document) == Some(&None)
                    && s.selection.archive_private.contains(&v.document)
            }
            2 => {
                s.selection.pairs.get(&v.document) == Some(&None)
                    && !s.selection.archive_private.contains(&v.document)
            }
            _ => target
                .as_ref()
                .is_some_and(|id| s.selection.pairs.get(&v.document) == Some(&Some(id.clone()))),
        };
        if unchanged {
            return;
        }
        s.selection.pairs.remove(&v.document);
        s.selection.archive_private.remove(&v.document);
        s.selection.discard_private.remove(&v.document);
        match pick.selected() {
            0 => {}
            1 => {
                s.selection.pairs.insert(v.document.clone(), None);
                s.selection.archive_private.insert(v.document);
            }
            2 => {
                s.selection.pairs.insert(v.document.clone(), None);
                if v.private_changed || v.private_input {
                    s.selection.discard_private.insert(v.document);
                }
            }
            _ => {
                if let Some(id) = target {
                    s.selection.pairs.insert(v.document, Some(id));
                }
            }
        }
        s.invalidate();
        drop(s);
        self.analysis
            .reset("File choices changed. Rebuild the merge result.");
        self.matching();
    }
    fn files(&self) {
        let (Some(session), Some(pick), Some(pages)) = (
            self.session.upgrade(),
            self.text_file.upgrade(),
            self.pages.upgrade(),
        ) else {
            return;
        };
        let s = session.borrow();
        let Some(r) = &s.review else {
            return;
        };
        if let Err(e) = merge::validate_selection(r, &s.selection) {
            drop(s);
            self.feedback(&e);
            return;
        }
        let names = r
            .old_views
            .iter()
            .filter(|v| {
                s.selection
                    .pairs
                    .get(&v.document)
                    .is_some_and(Option::is_some)
            })
            .map(view_name)
            .collect::<Vec<_>>();
        drop(s);
        self.updating.set(true);
        pick.set_model(Some(&gtk::StringList::new(
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
        pick.set_selected(0);
        self.updating.set(false);
        pages.set_visible_child_name("text");
        self.feedback(
            "Review every mapped file. Conflict choices are required; archives stay inactive.",
        );
        self.refresh();
    }
    fn inspect(&self) {
        let (Some(session), Some(pick)) = (self.session.upgrade(), self.text_file.upgrade()) else {
            return;
        };
        let data = {
            let s = session.borrow();
            let Some(r) = &s.review else {
                return;
            };
            let Some(v) = r
                .old_views
                .iter()
                .filter(|v| {
                    s.selection
                        .pairs
                        .get(&v.document)
                        .is_some_and(Option::is_some)
                })
                .nth(pick.selected() as usize)
            else {
                return;
            };
            (
                s.old.clone(),
                s.incoming.clone(),
                r.clone(),
                s.selection.clone(),
                v.document.clone(),
                s.decisions.get(&v.document).cloned(),
            )
        };
        let Some((epoch, cancel, permit)) = self.begin() else {
            return;
        };
        self.feedback("Comparing original, private and incoming bytes…");
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                check(&cancel)?;
                let input =
                    merge::file_inputs(&data.0.bundle, &data.1.bundle, &data.2, &data.3, &data.4)?;
                let worker::Response::SourceMergeReview(review) =
                    worker::run(worker::Request::ReviewSourceMerge {
                        base: input.base.clone(),
                        private: input.private.clone(),
                        incoming: input.incoming.clone(),
                    })?
                else {
                    return Err("Unexpected text review response".into());
                };
                // Bind the independent worker review to these exact immutable inputs.
                if *review != input.text_review()? {
                    return Err("Text review differs from its input bytes".into());
                }
                check(&cancel)?;
                let decision = data
                    .5
                    .filter(|d| d.review == *review)
                    .unwrap_or(merge::Decision {
                        review: *review,
                        choices: Default::default(),
                    });
                Ok(File {
                    inputs: input,
                    decision,
                })
            },
            move |result| {
                let Some(session) = ui.finish(epoch) else {
                    return;
                };
                match result {
                    Ok(file) => {
                        session.borrow_mut().file = Some(file);
                        ui.show_file();
                        ui.feedback("Choose each conflict, then use this file's choices.");
                    }
                    Err(e) => ui.feedback(&e),
                }
                ui.refresh();
            },
        );
    }
    fn show_file(&self) {
        let (Some(session), Some(conflicts)) = (self.session.upgrade(), self.conflict.upgrade())
        else {
            return;
        };
        let s = session.borrow();
        let Some(f) = &s.file else {
            return;
        };
        let names = std::iter::once("Whole file".into())
            .chain(
                f.decision
                    .review
                    .regions
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.conflict.is_some())
                    .map(|(i, r)| {
                        format!(
                            "Region {} · original bytes {}–{} · {:?}",
                            i + 1,
                            r.base.start,
                            r.base.end,
                            r.conflict.unwrap()
                        )
                    }),
            )
            .collect::<Vec<_>>();
        drop(s);
        self.updating.set(true);
        conflicts.set_model(Some(&gtk::StringList::new(
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
        conflicts.set_selected(0);
        self.updating.set(false);
        self.display_text();
    }
    fn display_text(&self) {
        let (Some(session), Some(conflict), Some(branch), Some(text), Some(choice)) = (
            self.session.upgrade(),
            self.conflict.upgrade(),
            self.branch.upgrade(),
            self.text.upgrade(),
            self.choice.upgrade(),
        ) else {
            return;
        };
        let s = session.borrow();
        let Some(f) = &s.file else {
            text.buffer()
                .set_text("Select a mapped file and compare it.");
            return;
        };
        let index = conflict.selected().checked_sub(1).and_then(|n| {
            f.decision
                .review
                .regions
                .iter()
                .enumerate()
                .filter(|(_, r)| r.conflict.is_some())
                .nth(n as usize)
                .map(|(i, _)| i)
        });
        let input = match branch.selected() {
            0 => &f.inputs.base,
            1 => &f.inputs.private,
            _ => &f.inputs.incoming,
        };
        let value = if let Some(i) = index {
            let r = &f.decision.review.regions[i];
            if branch.selected() == 0 {
                f.inputs.base[r.base.clone()].to_string()
            } else {
                let fragments = if branch.selected() == 1 {
                    &r.private
                } else {
                    &r.incoming
                };
                fragments
                    .iter()
                    .map(|p| {
                        match p.input {
                            source_merge::Input::Base => &f.inputs.base,
                            source_merge::Input::Private => &f.inputs.private,
                            source_merge::Input::Incoming => &f.inputs.incoming,
                        }
                        .get(p.span.clone())
                        .unwrap_or("")
                    })
                    .collect::<String>()
            }
        } else {
            input.clone()
        };
        let selected = index
            .and_then(|i| f.decision.choices.get(&i))
            .map_or(0, |c| match c {
                Choice::Private => 1,
                Choice::Incoming => 2,
                Choice::Base => 3,
                Choice::PrivateThenIncoming => 4,
                Choice::IncomingThenPrivate => 5,
            });
        drop(s);
        let Some(page) = self.page.upgrade() else {
            return;
        };
        let (span, count) = page_span(&value, page.value_as_int().max(1) as usize);
        self.updating.set(true);
        page.set_range(1.0, count as f64);
        text.buffer().set_text(&format!(
            "Page {} of {} · bytes {}–{} of {}\n\n{}",
            page.value_as_int(),
            count,
            span.start,
            span.end,
            value.len(),
            super::source_access::excerpt(&value[span.clone()])
        ));
        choice.set_selected(selected);
        choice.set_sensitive(index.is_some());
        self.updating.set(false);
    }
    fn choose(&self) {
        if self.updating.get() {
            return;
        }
        let (Some(session), Some(state), Some(conflict), Some(pick)) = (
            self.session.upgrade(),
            self.state.upgrade(),
            self.conflict.upgrade(),
            self.choice.upgrade(),
        ) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.pending || !s.current(&state) {
            return;
        }
        let Some(f) = s.file.as_mut() else {
            return;
        };
        let Some(i) = conflict.selected().checked_sub(1).and_then(|n| {
            f.decision
                .review
                .regions
                .iter()
                .enumerate()
                .filter(|(_, r)| r.conflict.is_some())
                .nth(n as usize)
                .map(|(i, _)| i)
        }) else {
            return;
        };
        let chosen = [
            Choice::Private,
            Choice::Incoming,
            Choice::Base,
            Choice::PrivateThenIncoming,
            Choice::IncomingThenPrivate,
        ]
        .get(pick.selected().wrapping_sub(1) as usize)
        .copied();
        if f.decision.choices.get(&i).copied() == chosen {
            return;
        }
        f.decision.choices.remove(&i);
        if let Some(c) = chosen {
            f.decision.choices.insert(i, c);
        }
        let id = f.inputs.old.clone();
        s.decisions.remove(&id);
        s.candidate = None;
        s.model.borrow_mut().take();
        drop(s);
        self.analysis
            .reset("Text choices changed. Confirm this file again and rebuild the result.");
        self.refresh();
    }
    fn use_file(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let data = {
            let s = session.borrow();
            let Some(f) = &s.file else {
                return;
            };
            (f.inputs.clone(), f.decision.clone())
        };
        let Some((epoch, cancel, permit)) = self.begin() else {
            return;
        };
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                check(&cancel)?;
                let worker::Response::SourceMergeResolution(_) =
                    worker::run(worker::Request::ResolveSourceMerge {
                        base: data.0.base,
                        private: data.0.private,
                        incoming: data.0.incoming,
                        review: data.1.review.clone(),
                        choices: data.1.choices.clone(),
                    })?
                else {
                    return Err("Unexpected text decision response".into());
                };
                check(&cancel)?;
                Ok((data.0.old, data.1))
            },
            move |result| {
                let Some(session) = ui.finish(epoch) else {
                    return;
                };
                match result {
                    Ok((id, d)) => {
                        let mut s = session.borrow_mut();
                        let prior = s.decisions.insert(id.clone(), d);
                        if let Err(e) = super::source_access::encoded_len(&s.decisions) {
                            s.decisions.remove(&id);
                            if let Some(prior) = prior {
                                s.decisions.insert(id, prior);
                            }
                            drop(s);
                            ui.feedback(&e);
                        } else {
                            let count = s.decisions.len();
                            s.candidate = None;
                            s.model.borrow_mut().take();
                            drop(s);
                            ui.feedback(&format!("{count} file decisions retained for this proposal. Rebuild the result after reviewing every mapped file."));
                        }
                    }
                    Err(e) => ui.feedback(&e),
                }
                ui.refresh();
            },
        );
    }
    fn build(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let data = {
            let s = session.borrow();
            let Some(r) = &s.review else {
                return;
            };
            (
                s.old.clone(),
                s.incoming.clone(),
                r.clone(),
                s.selection.clone(),
                s.decisions.clone(),
            )
        };
        let Some((epoch, cancel, permit)) = self.begin() else {
            return;
        };
        {
            let mut s = session.borrow_mut();
            s.candidate = None;
            s.model.borrow_mut().take();
        }
        self.analysis
            .reset("Rebuilding the candidate from the reviewed inputs…");
        self.feedback("Rebuilding original, incoming and merged graphs…");
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                check(&cancel)?;
                source_graph::restore(&data.0.bundle, &data.0.version)?;
                check(&cancel)?;
                source_graph::restore(&data.1.bundle, &data.0.version)?;
                check(&cancel)?;
                let composed =
                    merge::compose(&data.0.bundle, &data.1.bundle, &data.2, &data.3, &data.4)?;
                if let Some(reason) = composed.dependency_renewal {
                    return Err(format!(
                        "The merged import needs fresh resolution before it can be kept. {reason}"
                    ));
                }
                let analysis = Arc::new(source_graph::restore(&composed.bundle, &data.0.version)?);
                check(&cancel)?;
                let root = composed
                    .bundle
                    .sources
                    .iter()
                    .find(|s| s.id == composed.bundle.graph.root)
                    .ok_or("Merged root missing")?;
                let worker::Response::Parsed(report) = worker::run(worker::Request::Parse {
                    source: root.content.clone(),
                    version: data.0.version.clone(),
                })?
                else {
                    return Err("Unexpected root parser response".into());
                };
                let document = Arc::new(LuaDocument::from_report(
                    root.content.clone(),
                    *report,
                    &hyprland_editor_core::catalogue(),
                )?);
                let origin = root.origin.clone();
                let saved = Arc::new(source_graph::Saved::new(composed.bundle, &data.0.version)?);
                check(&cancel)?;
                Ok(Candidate {
                    saved,
                    document,
                    origin,
                    analysis,
                })
            },
            move |result| {
                let Some(session) = ui.finish(epoch) else {
                    return;
                };
                match result {
                    Ok(candidate) => {
                        let partial = candidate.analysis.report.halt.is_some()
                            || candidate.analysis.blocked.is_some();
                        let mut s = session.borrow_mut();
                        *s.model.borrow_mut() = Some(candidate.analysis.clone());
                        s.candidate = Some(candidate);
                        drop(s);
                        if let Some(p) = ui.partial.upgrade() {
                            p.set_active(false);
                            p.set_visible(partial);
                        }
                        ui.analysis.render();
                        if let Some(pages) = ui.pages.upgrade() {
                            pages.set_visible_child_name("result");
                        }
                        ui.feedback(if partial{"Analysis is partial. Inspect its stopping point before choosing whether to keep this private draft."}else{"Merged inputs rebuilt. Review the files and calls, then keep the merge in this draft."});
                    }
                    Err(e) => ui.feedback(&e),
                }
                ui.refresh();
            },
        );
    }
    fn keep(&self) {
        let (Some(session), Some(state)) = (self.session.upgrade(), self.state.upgrade()) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.pending || !s.current(&state) {
            drop(s);
            self.feedback("The draft changed. Start a new merge review.");
            return;
        }
        let Some(c) = &s.candidate else {
            return;
        };
        if (c.analysis.report.halt.is_some() || c.analysis.blocked.is_some())
            && !self.partial.upgrade().is_some_and(|p| p.is_active())
        {
            return;
        }
        let draft = state.borrow().draft.clone();
        let result = state.borrow_mut().keep_graph_revision(
            c.saved.clone(),
            draft,
            c.document.clone(),
            c.origin.clone(),
            s.generation,
        );
        match result {
            Ok(()) => {
                s.closed = true;
                s.cancel.store(true, Ordering::Release);
                drop(s);
                self.feedback("Merge kept in the private draft. Save Draft to persist it; Undo restores the previous graph and root.");
                if let Some(w) = self.window.upgrade() {
                    w.close();
                }
            }
            Err(e) => {
                drop(s);
                self.feedback(&e);
            }
        }
    }
}
pub(super) fn open(
    parent: &adw::Window,
    state: &Shared,
    incoming: Arc<source_graph::Saved>,
) -> Result<adw::Window, String> {
    show(parent, state, incoming).map(|(window, _)| window)
}
mod window;
use window::show;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(super) use tests::{test_review, test_stale};
