//! Explicit private-import inputs; temporary grants never escape this review.
use super::{directory_worker, label, monitors::compact, source_graph, workflow};
use adw::prelude::*;
use hyprland_editor_core::{source::display_path_bytes, workspace_graph::Bundle};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
type Guard = Rc<dyn Fn() -> bool>;
type Return = Rc<dyn Fn(Bundle)>;
struct Preview {
    pin: String,
    text: Arc<str>,
    detail: String,
}
struct Session {
    data: Option<Arc<source_graph::Pending>>,
    previous: Option<Arc<source_graph::Pending>>,
    capture: Option<Arc<source_graph::CapturedInput>>,
    preview: Option<Preview>,
    grants: Vec<Arc<directory_worker::Grant>>,
    guard: Guard,
    epoch: u64,
    busy: bool,
    closed: bool,
    cancelled: Arc<AtomicBool>,
}
impl Session {
    fn current(&self) -> bool {
        !self.closed && (self.guard)()
    }
    fn clear(&mut self) {
        self.closed = true;
        self.epoch += 1;
        self.cancelled.store(true, Ordering::Release);
        self.grants.clear();
        self.capture = None;
        self.preview = None;
        self.data = None;
        self.previous = None;
    }
}
#[derive(Clone)]
struct Controls {
    session: Weak<RefCell<Session>>,
    window: gtk::glib::WeakRef<adw::Window>,
    updating: Rc<Cell<bool>>,
    callback: Return,
    summary: gtk::glib::WeakRef<gtk::Label>,
    status: gtk::glib::WeakRef<gtk::Label>,
    mode: gtk::glib::WeakRef<gtk::DropDown>,
    pages: gtk::glib::WeakRef<gtk::Stack>,
    folders: gtk::glib::WeakRef<gtk::DropDown>,
    preserved: gtk::glib::WeakRef<gtk::DropDown>,
    approve: gtk::glib::WeakRef<gtk::Button>,
    forget: gtk::glib::WeakRef<gtk::Button>,
    capture: gtk::glib::WeakRef<gtk::Button>,
    use_input: gtk::glib::WeakRef<gtk::Button>,
    undo: gtk::glib::WeakRef<gtk::Button>,
    finish: gtk::glib::WeakRef<gtk::Button>,
    stop: gtk::glib::WeakRef<gtk::Button>,
    missing: gtk::glib::WeakRef<gtk::CheckButton>,
    detail: gtk::glib::WeakRef<gtk::Label>,
    text: gtk::glib::WeakRef<gtk::TextView>,
    page: gtk::glib::WeakRef<gtk::SpinButton>,
}
fn model(pick: &gtk::DropDown, names: &[String]) {
    let unchanged = pick.model().is_some_and(|m| {
        m.n_items() == names.len() as u32
            && names.iter().enumerate().all(|(i, s)| {
                m.item(i as u32)
                    .and_downcast::<gtk::StringObject>()
                    .is_some_and(|v| v.string().as_str() == s.as_str())
            })
    });
    if !unchanged {
        pick.set_model(Some(&gtk::StringList::new(
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
    }
}
impl Controls {
    fn feedback(&self, text: &str) {
        if let Some(w) = self.status.upgrade() {
            w.set_text(&display_path_bytes(text.as_bytes()));
        }
    }
    fn begin(&self) -> Option<(u64, Arc<AtomicBool>, super::source_changes::Permit)> {
        let session = self.session.upgrade()?;
        let mut s = session.borrow_mut();
        if s.busy || !s.current() || self.window.upgrade().is_none_or(|w| !w.is_visible()) {
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
        s.busy = true;
        s.epoch += 1;
        s.cancelled = Arc::new(AtomicBool::new(false));
        let result = (s.epoch, s.cancelled.clone(), permit);
        drop(s);
        self.render();
        Some(result)
    }
    fn end(&self, epoch: u64) -> Option<Rc<RefCell<Session>>> {
        let session = self.session.upgrade()?;
        {
            let mut s = session.borrow_mut();
            if s.epoch != epoch
                || !s.current()
                || self.window.upgrade().is_none_or(|w| !w.is_visible())
            {
                return None;
            }
            s.busy = false;
        }
        self.render();
        Some(session)
    }
    fn render(&self) {
        if self.updating.get() {
            return;
        }
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let (data, capture, folders, available, busy, undo, cancelled) = {
            let s = session.borrow();
            (
                s.data.clone(),
                s.capture.clone(),
                s.grants
                    .iter()
                    .map(|g| display_path_bytes(&g.identity().selected_path))
                    .collect::<Vec<_>>(),
                s.current() && !s.busy,
                s.busy,
                s.previous.is_some(),
                s.cancelled.load(Ordering::Acquire),
            )
        };
        let Some(data) = data else {
            return;
        };
        let request = data.request();
        self.updating.set(true);
        if let Some(pick) = self.folders.upgrade() {
            model(&pick, &folders);
        }
        let preserved = data
            .bundle()
            .graph
            .views
            .iter()
            .chain(data.bundle().graph.private.views.iter())
            .collect::<Vec<_>>();
        let mut names = vec!["Choose a preserved view".into()];
        names.extend(preserved.iter().map(|d| {
            format!(
                "{} · {}{}",
                display_path_bytes(&d.load_name),
                d.document,
                if data.bundle().graph.private.archived.contains(&d.document) {
                    " · archived"
                } else {
                    ""
                }
            )
        }));
        if let Some(pick) = self.preserved.upgrade() {
            model(&pick, &names);
        }
        self.updating.set(false);
        if let Some(w) = self.summary.upgrade() {
            w.set_text(&request.map_or_else(||match &data.report().halt {
            None=>"No pending import remains. Verify these inputs before returning to the merge review.".into(),
            Some(_)=>"File resolution reached another analysis stopping point. The final result will remain partial.".into(),
        },|r|format!("{} · {}\nRequested by {} · bytes {}–{}\n{} explicit private inputs supplied",match r.loader {
            hyprland_editor_core::lua::replay::Loader::SafeRequire=>"require",
            hyprland_editor_core::lua::replay::Loader::OriginalRequire=>"__require",
            hyprland_editor_core::lua::replay::Loader::Dofile=>"dofile",
            hyprland_editor_core::lua::replay::Loader::Loadfile=>"loadfile",
        },display_path_bytes(&r.module),r.document,r.span.start,r.span.end,data.bundle().graph.private.bindings.len())));
        }
        let mode = self.mode.upgrade().map_or(0, |p| p.selected());
        if let Some(pages) = self.pages.upgrade() {
            pages.set_visible_child_name(match mode {
                1 => "preserved",
                2 => "missing",
                _ => "capture",
            });
        }
        let selected = self
            .preserved
            .upgrade()
            .and_then(|p| p.selected().checked_sub(1))
            .and_then(|i| preserved.get(i as usize));
        let supplied = match mode {
            1 => selected.is_some(),
            2 => self.missing.upgrade().is_some_and(|w| w.is_active()),
            _ => capture.is_some(),
        };
        for (weak, enabled) in [
            (
                &self.approve,
                available && request.is_some() && folders.len() < 8,
            ),
            (&self.forget, available && !folders.is_empty()),
            (
                &self.capture,
                available && request.is_some() && !folders.is_empty(),
            ),
            (&self.use_input, available && request.is_some() && supplied),
            (&self.undo, available && undo),
            (&self.finish, available && request.is_none()),
        ] {
            if let Some(b) = weak.upgrade() {
                b.set_sensitive(enabled);
            }
        }
        if let Some(w) = self.use_input.upgrade() {
            w.set_visible(request.is_some());
        }
        if let Some(w) = self.finish.upgrade() {
            w.set_visible(request.is_none());
        }
        if let Some(w) = self.undo.upgrade() {
            w.set_visible(undo);
        }
        for weak in [&self.mode, &self.folders, &self.preserved] {
            if let Some(p) = weak.upgrade() {
                p.set_sensitive(available && request.is_some());
            }
        }
        if let Some(w) = self.missing.upgrade() {
            w.set_sensitive(available && request.is_some());
        }
        if let Some(w) = self.stop.upgrade() {
            w.set_visible(busy);
            w.set_sensitive(busy && !cancelled);
        }
        let key = match mode {
            1 => selected.map(|d| format!("{}:preserved:{}", data.fingerprint(), d.document)),
            2 => None,
            _ => capture
                .as_ref()
                .map(|c| format!("{}:capture:{:p}", data.fingerprint(), Arc::as_ptr(c))),
        };
        let changed = {
            let s = session.borrow();
            s.preview.as_ref().map(|p| p.pin.as_str()) != key.as_deref()
        };
        if changed {
            let preview=match mode {
                1=>if let Some(selected)=selected {
                    let all=match data.preserved_program(){Ok(p)=>p,Err(e)=>{self.feedback(&e);return;}};
                    all.documents.iter().find(|d|d.id==selected.document).map(|d|Preview {pin:key.clone().unwrap(),text:Arc::from(d.source.as_str()),detail:format!("Preserved private view · {}\nSHA-256: {}\nHistorical path and current filesystem existence remain unverified. Private source pieces are included.",display_path_bytes(&d.load_name),d.sha256)})
                }else{None},
                2=>None,
                _=>capture.as_ref().map(|c|Preview {pin:key.clone().unwrap(),text:Arc::from(c.source().content.as_str()),detail:format!("Captured: {}\nLoad name: {}\nSHA-256: {} · {} bytes · {} metadata observations\nThis is the captured snapshot; later filesystem changes do not replace these bytes.",display_path_bytes(&c.source().identity.selected_path),display_path_bytes(c.load_name()),c.source().identity.sha256,c.source().content.len(),c.probe_count())}),
            };
            session.borrow_mut().preview = preview;
            if let Some(page) = self.page.upgrade() {
                self.updating.set(true);
                page.set_value(1.0);
                self.updating.set(false);
            }
        }
        self.preview();
    }
    fn preview(&self) {
        let (Some(session), Some(text), Some(page)) = (
            self.session.upgrade(),
            self.text.upgrade(),
            self.page.upgrade(),
        ) else {
            return;
        };
        let s = session.borrow();
        let Some(preview) = &s.preview else {
            text.buffer()
                .set_text("Choose a captured or preserved source to inspect its bytes.");
            if let Some(w) = self.detail.upgrade() {
                w.set_text("");
            }
            return;
        };
        let (span, count) =
            super::source_reconcile::page_span(&preview.text, page.value_as_int().max(1) as usize);
        self.updating.set(true);
        page.set_range(1.0, count as f64);
        text.buffer().set_text(&format!(
            "Page {} of {} · bytes {}–{} of {}\n\n{}",
            page.value_as_int(),
            count,
            span.start,
            span.end,
            preview.text.len(),
            super::source_access::excerpt(&preview.text[span.clone()])
        ));
        if let Some(w) = self.detail.upgrade() {
            w.set_text(&preview.detail);
        }
        self.updating.set(false);
    }
    fn approve(&self, path: PathBuf) {
        let Some((epoch, _, permit)) = self.begin() else {
            return;
        };
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                directory_worker::approve(&path).map(Arc::new)
            },
            move |result| {
                let Some(session) = ui.end(epoch) else {
                    return;
                };
                match result {
                    Ok(grant) => {
                        let mut s = session.borrow_mut();
                        let prior = s.grants.iter().position(|g| {
                            g.identity().selected_path == grant.identity().selected_path
                        });
                        if let Some(i) = prior {
                            s.grants[i] = grant;
                        } else if s.grants.len() < 8 {
                            s.grants.push(grant);
                        } else {
                            drop(s);
                            ui.feedback("Temporary folder limit reached");
                            return;
                        }
                        drop(s);
                        ui.feedback("Folder allowed temporarily. Capture follows the current request's search order.");
                    }
                    Err(e) => ui.feedback(&e),
                }
                ui.render();
            },
        );
    }
    fn capture(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let (data, grants) = {
            let s = session.borrow();
            let Some(data) = s.data.clone() else {
                return;
            };
            (data, s.grants.clone())
        };
        let Some((epoch, cancelled, permit)) = self.begin() else {
            return;
        };
        {
            let mut s = session.borrow_mut();
            s.capture = None;
            s.preview = None;
        }
        self.render();
        self.feedback("Capturing the requested file under the allowed folders…");
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                source_graph::capture_input(&data, grants, cancelled).map(Arc::new)
            },
            move |result| {
                let Some(session) = ui.end(epoch) else {
                    return;
                };
                match result {
                    Ok(capture) => {
                        session.borrow_mut().capture = Some(capture);
                        ui.feedback("Inspect the captured bytes, then explicitly use this input.");
                    }
                    Err(e) => ui.feedback(&e),
                }
                ui.render();
            },
        );
    }
    fn use_input(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let Some(mode) = self.mode.upgrade().map(|m| m.selected()) else {
            return;
        };
        let (data, choice) = {
            let s = session.borrow();
            if s.busy || !s.current() {
                return;
            }
            let Some(data) = s.data.clone() else {
                return;
            };
            let choice = match mode {
                1 => {
                    let Some(index) = self
                        .preserved
                        .upgrade()
                        .and_then(|p| p.selected().checked_sub(1))
                    else {
                        return;
                    };
                    let Some(document) = data
                        .bundle()
                        .graph
                        .views
                        .iter()
                        .chain(data.bundle().graph.private.views.iter())
                        .nth(index as usize)
                        .map(|d| d.document.clone())
                    else {
                        return;
                    };
                    source_graph::InputChoice::Preserved(document)
                }
                2 => {
                    if !self.missing.upgrade().is_some_and(|w| w.is_active()) {
                        return;
                    }
                    source_graph::InputChoice::Missing
                }
                _ => {
                    let Some(capture) = s.capture.clone() else {
                        return;
                    };
                    source_graph::InputChoice::Capture(capture)
                }
            };
            (data, choice)
        };
        let Some((epoch, cancelled, permit)) = self.begin() else {
            return;
        };
        self.feedback("Rechecking the exact request and replaying its selected input…");
        let ui = self.clone();
        let previous = data.clone();
        workflow::background(
            move || {
                let _permit = permit;
                source_graph::accept_input(&data, choice, cancelled).map(Arc::new)
            },
            move |result| {
                let Some(session) = ui.end(epoch) else {
                    return;
                };
                match result {
                    Ok(data) => {
                        let mut s = session.borrow_mut();
                        s.previous = Some(previous);
                        s.data = Some(data);
                        s.capture = None;
                        s.preview = None;
                        drop(s);
                        ui.updating.set(true);
                        if let Some(m) = ui.mode.upgrade() {
                            m.set_selected(0);
                        }
                        if let Some(m) = ui.preserved.upgrade() {
                            m.set_selected(0);
                        }
                        if let Some(c) = ui.missing.upgrade() {
                            c.set_active(false);
                        }
                        ui.updating.set(false);
                        ui.feedback("Input retained in this proposal. Continue with the next request, or verify and return to the result review.");
                    }
                    Err(e) => ui.feedback(&e),
                }
                ui.render();
            },
        );
    }
    fn finish(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let data = {
            let s = session.borrow();
            let Some(data) = s.data.clone() else {
                return;
            };
            if data.request().is_some() {
                return;
            }
            data
        };
        let Some((epoch, cancelled, permit)) = self.begin() else {
            return;
        };
        let ui = self.clone();
        workflow::background(
            move || {
                let _permit = permit;
                source_graph::restore_cancellable(
                    data.bundle(),
                    &data.program().version,
                    cancelled,
                )?;
                Ok(data.bundle().clone())
            },
            move |result| {
                let Some(_session) = ui.end(epoch) else {
                    return;
                };
                match result {
                    Ok(bundle) => {
                        let callback = ui.callback.clone();
                        if let Some(window) = ui.window.upgrade() {
                            window.close();
                        }
                        callback(bundle);
                    }
                    Err(e) => ui.feedback(&e),
                }
            },
        );
    }
}
fn pick(name: &str, labels: &[&str]) -> gtk::DropDown {
    let w = gtk::DropDown::from_strings(labels);
    w.set_widget_name(name);
    compact(&w);
    let description = match name {
        "pending-mode" => "Import input source",
        "pending-folders" => "Temporarily allowed folders",
        "pending-preserved" => "Preserved source view",
        _ => name,
    };
    w.update_property(&[gtk::accessible::Property::Label(description)]);
    w
}
fn button(name: &str, title: &str) -> gtk::Button {
    let w = gtk::Button::with_label(title);
    w.set_widget_name(name);
    w
}
pub(super) fn open(
    parent: &adw::Window,
    data: Arc<source_graph::Pending>,
    guard: Guard,
    callback: Return,
) -> adw::Window {
    build(parent, data, guard, callback).0
}
fn build(
    parent: &adw::Window,
    data: Arc<source_graph::Pending>,
    guard: Guard,
    callback: Return,
) -> (adw::Window, Controls) {
    let window = adw::Window::builder()
        .title("Resolve Imports")
        .transient_for(parent)
        .destroy_with_parent(true)
        .modal(true)
        .default_width(700)
        .default_height(700)
        .build();
    window.set_size_request(350, 420);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    let title = label("Imports");
    title.set_selectable(false);
    header.set_title_widget(Some(&title));
    let stop = button("pending-stop", "Stop");
    header.pack_end(&stop);
    root.append(&header);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for f in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        f(body.upcast_ref(), 16);
    }
    let summary = label("");
    summary.set_widget_name("pending-summary");
    body.append(&summary);
    let mode = pick(
        "pending-mode",
        &[
            "Capture from allowed folders",
            "Use a preserved view",
            "Assume this import is missing",
        ],
    );
    body.append(&mode);
    let pages = gtk::Stack::new();
    pages.set_hhomogeneous(false);
    pages.set_vhomogeneous(false);
    let capture_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let approve = button("pending-approve", "Allow Folder…");
    capture_page.append(&approve);
    let folders = pick("pending-folders", &[]);
    capture_page.append(&folders);
    let forget = button("pending-forget", "Forget Selected Folder");
    capture_page.append(&forget);
    let capture = button("pending-capture", "Capture Requested File");
    capture_page.append(&capture);
    pages.add_named(&capture_page, Some("capture"));
    let preserved_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    preserved_page.append(&label("Use the preserved private bytes, including their edits. This does not verify that their historical path exists now."));
    let preserved = pick("pending-preserved", &[]);
    preserved.set_enable_search(true);
    preserved_page.append(&preserved);
    pages.add_named(&preserved_page, Some("preserved"));
    let missing_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    missing_page.append(&label("No filesystem absence has been verified. This supplies a missing-module assumption to the static model."));
    let missing = gtk::CheckButton::new();
    let ack = label("Use a missing-input assumption for this exact request");
    ack.set_selectable(false);
    missing.set_child(Some(&ack));
    missing.set_widget_name("pending-missing-ack");
    missing.update_property(&[gtk::accessible::Property::Label(
        "Use a missing-input assumption for this exact request",
    )]);
    missing_page.append(&missing);
    pages.add_named(&missing_page, Some("missing"));
    body.append(&pages);
    let detail = label("");
    detail.set_widget_name("pending-detail");
    let disclosure = gtk::Expander::builder()
        .label("Source details")
        .child(&detail)
        .build();
    body.append(&disclosure);
    let page = gtk::SpinButton::with_range(1.0, 1.0, 1.0);
    page.set_widget_name("pending-text-page");
    page.update_property(&[gtk::accessible::Property::Label("Input source page")]);
    let page_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    page_row.append(&label("Source page"));
    page.set_hexpand(true);
    page_row.append(&page);
    body.append(&page_row);
    let text = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    text.set_widget_name("pending-text");
    let area = gtk::ScrolledWindow::builder()
        .child(&text)
        .min_content_height(180)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    body.append(&area);
    let use_input = button("pending-use", "Use This Input");
    use_input.add_css_class("suggested-action");
    let undo = button("pending-undo", "Undo Last Input");
    let finish = button("pending-finish", "Verify Inputs and Return");
    finish.add_css_class("suggested-action");
    let scroll = gtk::ScrolledWindow::builder()
        .child(&body)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    root.append(&scroll);
    let actions = gtk::Box::new(gtk::Orientation::Vertical, 8);
    actions.set_margin_start(16);
    actions.set_margin_end(16);
    actions.set_margin_top(8);
    actions.set_margin_bottom(8);
    actions.append(&use_input);
    actions.append(&finish);
    actions.append(&undo);
    root.append(&actions);
    let status = label(
        "No new folder access has been granted. This window changes only the pending proposal.",
    );
    status.set_margin_start(16);
    status.set_margin_end(16);
    status.set_margin_bottom(12);
    status.set_widget_name("pending-status");
    root.append(&status);
    window.set_content(Some(&root));
    let session = Rc::new(RefCell::new(Session {
        data: Some(data),
        previous: None,
        capture: None,
        preview: None,
        grants: vec![],
        guard,
        epoch: 0,
        busy: false,
        closed: false,
        cancelled: Arc::new(AtomicBool::new(false)),
    }));
    let ui = Controls {
        session: Rc::downgrade(&session),
        window: window.downgrade(),
        updating: Rc::new(Cell::new(false)),
        callback,
        summary: summary.downgrade(),
        status: status.downgrade(),
        mode: mode.downgrade(),
        pages: pages.downgrade(),
        folders: folders.downgrade(),
        preserved: preserved.downgrade(),
        approve: approve.downgrade(),
        forget: forget.downgrade(),
        capture: capture.downgrade(),
        use_input: use_input.downgrade(),
        undo: undo.downgrade(),
        finish: finish.downgrade(),
        stop: stop.downgrade(),
        missing: missing.downgrade(),
        detail: detail.downgrade(),
        text: text.downgrade(),
        page: page.downgrade(),
    };
    let close = session.clone();
    window.connect_close_request(move |_| {
        close.borrow_mut().clear();
        gtk::glib::Propagation::Proceed
    });
    for control in [&mode, &preserved] {
        let u = ui.clone();
        control.connect_selected_notify(move |_| u.render());
    }
    let u = ui.clone();
    missing.connect_toggled(move |_| u.render());
    let u = ui.clone();
    page.connect_value_changed(move |_| {
        if !u.updating.get() {
            u.preview();
        }
    });
    let u = ui.clone();
    capture.connect_clicked(move |_| u.capture());
    let u = ui.clone();
    use_input.connect_clicked(move |_| u.use_input());
    let u = ui.clone();
    finish.connect_clicked(move |_| u.finish());
    let u = ui.clone();
    undo.connect_clicked(move |_| {
        if let Some(s) = u.session.upgrade() {
            let mut s = s.borrow_mut();
            if s.busy || !s.current() {
                return;
            }
            if let Some(previous) = s.previous.take() {
                s.data = Some(previous);
                s.capture = None;
                s.preview = None;
                s.epoch += 1;
            }
        }
        u.render();
    });
    let u = ui.clone();
    forget.connect_clicked(move |_|{if let Some(s)=u.session.upgrade(){let mut s=s.borrow_mut();if s.busy||!s.current(){return;}let index=u.folders.upgrade().map_or(usize::MAX,|p|p.selected()as usize);if index<s.grants.len(){s.grants.remove(index);}}u.feedback("Folder forgotten. Already captured bytes remain data; future reads need a live allowed folder.");u.render();});
    let u = ui.clone();
    stop.connect_clicked(move |_|{if let Some(s)=u.session.upgrade(){s.borrow().cancelled.store(true,Ordering::Release);}u.feedback("Stopping the owned parser; an in-progress folder observation finishes within its existing bounds.");u.render();});
    let u = ui.clone();
    approve.connect_clicked(move |_| {
        let Some(window) = u.window.upgrade() else {
            return;
        };
        let valid = u.session.upgrade().is_some_and(|s| {
            let s = s.borrow();
            s.current() && !s.busy
        });
        if !valid {
            return;
        }
        let u = u.clone();
        gtk::FileDialog::builder()
            .title("Allow a folder for this import review")
            .build()
            .select_folder(
                Some(&window),
                None::<&gtk::gio::Cancellable>,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            u.approve(path);
                        }
                    }
                },
            );
    });
    let u = ui.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
        let Some(s) = u.session.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        let current = s.borrow().current() && u.window.upgrade().is_some_and(|w| w.is_visible());
        if !current {
            s.borrow_mut().clear();
            if let Some(w) = u.window.upgrade() {
                w.close();
            }
            return gtk::glib::ControlFlow::Break;
        }
        gtk::glib::ControlFlow::Continue
    });
    ui.render();
    window.present();
    (window, ui)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(super) use tests::test_workflow;
