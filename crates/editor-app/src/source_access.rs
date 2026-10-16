//! Temporary authorized snapshots and model analysis. No draft reducer or writer.
use super::{directory_worker, label, monitors::compact, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    source::{display_path_bytes, CapturedSource},
    source_scope::Identity,
    workspace::{MAX_SOURCE_DOCUMENTS, MAX_WORKSPACE_BYTES},
    MAX_DOCUMENT_BYTES,
};
use std::{
    cell::RefCell,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    rc::{Rc, Weak},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};
const MAX_FOLDERS: usize = 8;
struct Folder {
    grant: Arc<directory_worker::Grant>,
    evidence: Arc<Identity>,
    cost: usize,
}
struct Capture {
    source: Arc<CapturedSource>,
    folder: Arc<Identity>,
    cost: usize,
}
fn encoded_len(value: &impl serde::Serialize) -> Result<usize, String> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > MAX_WORKSPACE_BYTES {
                return Err(std::io::Error::other(
                    "Dependency review retained-data limit exceeded",
                ));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut size = Counter(0);
    serde_json::to_writer(&mut size, value).map_err(|e| e.to_string())?;
    Ok(size.0)
}
impl Folder {
    fn prepare(grant: directory_worker::Grant) -> Result<Self, String> {
        let cost = encoded_len(grant.identity())?.saturating_mul(2);
        Ok(Self {
            evidence: Arc::new(grant.identity().clone()),
            grant: Arc::new(grant),
            cost,
        })
    }
}
impl Capture {
    fn prepare(source: CapturedSource, folder: Arc<Identity>) -> Result<Self, String> {
        source.identity.validate(&source.content)?;
        let cost = encoded_len(&(&source, folder.as_ref()))?;
        Ok(Self {
            source: Arc::new(source),
            folder,
            cost,
        })
    }
}
struct Session {
    version: String,
    root_hash: String,
    generation: u64,
    folders: Vec<Folder>,
    captures: Vec<Capture>,
    pending: bool,
    alive: bool,
    epoch: u64,
    analysis: super::source_analysis::Model,
    analysis_cancel: Option<Arc<AtomicBool>>,
    context: super::source_context::Options,
    context_cost: usize,
    #[cfg(test)]
    finished_epoch: u64,
}
impl Session {
    fn current(&self, state: &Shared) -> bool {
        let s = state.borrow();
        self.alive
            && s.generation == self.generation
            && s.draft.version == self.version
            && s.sources
                .get(&self.version)
                .is_some_and(|s| s.report.source_sha256 == self.root_hash)
    }
    fn cost(&self) -> usize {
        self.folders
            .iter()
            .map(|f| f.cost)
            .chain(self.captures.iter().map(|f| f.cost))
            .sum::<usize>()
            + self.context_cost
            + self
                .analysis
                .borrow()
                .as_ref()
                .map_or(0, |a| a.retained_bytes)
    }
    fn add_folder(&mut self, folder: Folder) -> Result<usize, String> {
        let prior = self
            .folders
            .iter()
            .position(|f| f.evidence.selected_path == folder.evidence.selected_path);
        let prior_cost = prior.map_or(0, |i| self.folders[i].cost);
        if (prior.is_none() && self.folders.len() >= MAX_FOLDERS)
            || self
                .cost()
                .saturating_sub(prior_cost)
                .saturating_add(folder.cost)
                > MAX_WORKSPACE_BYTES
        {
            return Err("Dependency review reached its folder or retained-data limit. Forget a folder or remove captures first.".into());
        }
        if let Some(index) = prior {
            self.folders[index] = folder;
            Ok(index)
        } else {
            self.folders.push(folder);
            Ok(self.folders.len() - 1)
        }
    }
    fn add_capture(&mut self, capture: Capture) -> Result<usize, String> {
        if let Some(index) = self.captures.iter().position(|c| {
            c.source.identity == capture.source.identity && c.folder == capture.folder
        }) {
            return Ok(index);
        }
        let bytes = self
            .captures
            .iter()
            .map(|c| c.source.content.len())
            .sum::<usize>();
        if self.captures.len() >= MAX_SOURCE_DOCUMENTS
            || bytes.saturating_add(capture.source.content.len()) > 4 * MAX_DOCUMENT_BYTES
            || self.cost().saturating_add(capture.cost) > MAX_WORKSPACE_BYTES
        {
            return Err("Dependency review reached its capture limit. Remove a capture before reading another file.".into());
        }
        self.captures.push(capture);
        Ok(self.captures.len() - 1)
    }
    fn clear(&mut self) {
        self.epoch += 1;
        self.pending = false;
        self.folders.clear();
        self.captures.clear();
        self.context = super::source_context::Options::default();
        self.context_cost = 0;
        self.clear_analysis();
    }
    fn clear_analysis(&mut self) {
        if let Some(cancel) = self.analysis_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.analysis.borrow_mut().take();
    }
}
#[derive(Clone)]
struct Controls {
    session: Weak<RefCell<Session>>,
    state: Weak<RefCell<super::State>>,
    window: gtk::glib::WeakRef<adw::Window>,
    folders: gtk::glib::WeakRef<gtk::DropDown>,
    captures: gtk::glib::WeakRef<gtk::DropDown>,
    add: gtk::glib::WeakRef<gtk::Button>,
    forget: gtk::glib::WeakRef<gtk::Button>,
    read: gtk::glib::WeakRef<gtk::Button>,
    remove: gtk::glib::WeakRef<gtk::Button>,
    feedback: gtk::glib::WeakRef<gtk::Label>,
    folder_details: gtk::glib::WeakRef<gtk::TextView>,
    capture_details: gtk::glib::WeakRef<gtk::TextView>,
    preview: gtk::glib::WeakRef<gtk::TextView>,
    analyze: gtk::glib::WeakRef<gtk::Button>,
    context: gtk::glib::WeakRef<gtk::Button>,
    cancel: gtk::glib::WeakRef<gtk::Button>,
    pages: gtk::glib::WeakRef<gtk::Stack>,
    analysis_view: super::source_analysis::View,
}
impl Controls {
    fn current(&self) -> bool {
        self.session
            .upgrade()
            .zip(self.state.upgrade())
            .is_some_and(|(s, state)| s.borrow().current(&state))
    }
    fn feedback(&self, text: &str) {
        if let Some(label) = self.feedback.upgrade() {
            label.set_text(text);
        }
    }
    fn refresh(&self, folder: Option<usize>, capture: Option<usize>) {
        let (Some(session), Some(folders), Some(captures)) = (
            self.session.upgrade(),
            self.folders.upgrade(),
            self.captures.upgrade(),
        ) else {
            return;
        };
        let s = session.borrow();
        let names = s
            .folders
            .iter()
            .map(|f| display_path_bytes(&f.evidence.selected_path))
            .collect::<Vec<_>>();
        let old = folders.selected();
        folders.set_model(Some(&gtk::StringList::new(
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
        folders.set_selected(
            folder
                .map_or(old, |i| i as u32)
                .min(names.len().saturating_sub(1) as u32),
        );
        let names = s
            .captures
            .iter()
            .map(|c| display_path_bytes(&c.source.identity.selected_path))
            .collect::<Vec<_>>();
        let old = captures.selected();
        captures.set_model(Some(&gtk::StringList::new(
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
        captures.set_selected(
            capture
                .map_or(old, |i| i as u32)
                .min(names.len().saturating_sub(1) as u32),
        );
        drop(s);
        self.details();
    }
    fn details(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let s = session.borrow();
        let valid = self.current();
        let folder = self
            .folders
            .upgrade()
            .and_then(|p| s.folders.get(p.selected() as usize));
        let capture = self
            .captures
            .upgrade()
            .and_then(|p| s.captures.get(p.selected() as usize));
        for (weak, available) in [
            (&self.add, valid && s.folders.len() < MAX_FOLDERS),
            (&self.forget, folder.is_some()),
            (&self.read, valid && folder.is_some()),
            (&self.remove, capture.is_some()),
            (&self.analyze, valid && capture.is_some()),
            (&self.context, valid),
        ] {
            if let Some(button) = weak.upgrade() {
                button.set_sensitive(!s.pending && available);
            }
        }
        if let Some(button) = self.cancel.upgrade() {
            button.set_visible(
                s.pending
                    && s.analysis_cancel.is_some()
                    && self
                        .pages
                        .upgrade()
                        .is_some_and(|p| p.visible_child_name().as_deref() == Some("analysis")),
            );
            button.set_sensitive(
                s.pending
                    && s.analysis_cancel
                        .as_ref()
                        .is_some_and(|c| !c.load(Ordering::Relaxed)),
            );
        }
        if let Some(details) = self.folder_details.upgrade() {
            let text=folder.map_or_else(||"No folder allowed. Choose one to capture files for review.".into(),|f|format!("Selected: {}\nResolved at approval: {}\nDevice/inode: {}/{} · mount {}\nOwner/group: {}/{} · mode {:o}\nThis live folder handle is temporary. Captured paths do not restore its access after closure.",display_path_bytes(&f.evidence.selected_path),display_path_bytes(&f.evidence.resolved_path),f.evidence.device,f.evidence.inode,f.evidence.mount_id,f.evidence.owner,f.evidence.group,f.evidence.mode & 0o7777));
            details.buffer().set_text(&text);
        }
        if let Some(details) = self.capture_details.upgrade() {
            let text=capture.map_or_else(||"No file captured.".into(),|c|format!("Selected: {}\nResolved: {}\nSHA-256: {}\n{} bytes · {} symlinks\nApproved folder: {}\nAnalyze File treats this snapshot as the main configuration. Capturing and analysis do not change the draft or desktop.",display_path_bytes(&c.source.identity.selected_path),c.source.identity.resolved_path.as_deref().map(display_path_bytes).unwrap_or_default(),c.source.identity.sha256,c.source.content.len(),c.source.identity.links.len(),display_path_bytes(&c.folder.selected_path)));
            details.buffer().set_text(&text);
        }
        if let Some(preview) = self.preview.upgrade() {
            preview.buffer().set_text(&capture.map_or_else(
                || "Choose a file to review its captured source.".into(),
                |c| excerpt(&c.source.content),
            ));
        }
        if !valid {
            self.feedback(
                "The draft or imported source changed. Reopen dependency review for the current snapshot.",
            );
        }
    }
    fn begin(&self) -> Option<u64> {
        if !self.current() {
            self.details();
            return None;
        }
        let session = self.session.upgrade()?;
        let mut s = session.borrow_mut();
        if s.pending {
            return None;
        }
        s.pending = true;
        s.clear_analysis();
        s.epoch += 1;
        let epoch = s.epoch;
        drop(s);
        self.analysis_view.reset("No current analysis.");
        self.details();
        Some(epoch)
    }
    fn finish(&self, epoch: u64) -> Option<Rc<RefCell<Session>>> {
        let session = self.session.upgrade()?;
        #[cfg(test)]
        {
            let prior = session.borrow().finished_epoch;
            session.borrow_mut().finished_epoch = prior.max(epoch);
        }
        if !session.borrow().alive || session.borrow().epoch != epoch {
            return None;
        }
        session.borrow_mut().pending = false;
        if !self.current() {
            session.borrow_mut().clear();
            self.refresh(None, None);
            return None;
        }
        Some(session)
    }
    fn approve(&self, path: PathBuf) {
        let Some(epoch) = self.begin() else {
            return;
        };
        self.feedback("Checking the selected folder…");
        let ui = self.clone();
        workflow::background(
            move || directory_worker::approve(&path).and_then(Folder::prepare),
            move |result| {
                let Some(session) = ui.finish(epoch) else {
                    return;
                };
                let result = result.and_then(|grant| session.borrow_mut().add_folder(grant));
                match result {
                    Ok(index) => {
                        ui.refresh(Some(index), None);
                        ui.feedback("Folder allowed. Choose a file within it to capture a temporary snapshot.");
                    }
                    Err(e) => {
                        ui.feedback(&e);
                        ui.details();
                    }
                }
            },
        );
    }
    fn capture(&self, path: PathBuf) {
        let (Some(session), Some(pick)) = (self.session.upgrade(), self.folders.upgrade()) else {
            return;
        };
        let Some((grant, evidence)) = session
            .borrow()
            .folders
            .get(pick.selected() as usize)
            .map(|f| (f.grant.clone(), f.evidence.clone()))
        else {
            return;
        };
        let Some(epoch) = self.begin() else {
            return;
        };
        self.feedback("Capturing the selected file…");
        let ui = self.clone();
        workflow::background(
            move || {
                directory_worker::read(&grant, path.as_os_str().as_bytes())
                    .and_then(|source| Capture::prepare(source, evidence))
            },
            move |result| {
                let Some(session) = ui.finish(epoch) else {
                    return;
                };
                let result = result.and_then(|capture| session.borrow_mut().add_capture(capture));
                match result {
                    Ok(index) => {
                        ui.refresh(None, Some(index));
                        ui.feedback(
                            "File captured for review. It has not changed the draft or desktop.",
                        );
                    }
                    Err(e) => {
                        ui.feedback(&e);
                        ui.details();
                    }
                }
            },
        );
    }
    fn analyze(&self) {
        let (Some(session), Some(pick)) = (self.session.upgrade(), self.captures.upgrade()) else {
            return;
        };
        let Some(root) = session
            .borrow()
            .captures
            .get(pick.selected() as usize)
            .map(|c| c.source.clone())
        else {
            return;
        };
        let Some(epoch) = self.begin() else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let (version, grants, budget, options) = {
            let mut s = session.borrow_mut();
            s.analysis_cancel = Some(cancel.clone());
            (
                s.version.clone(),
                s.folders
                    .iter()
                    .map(|f| f.grant.clone())
                    .collect::<Vec<_>>(),
                MAX_WORKSPACE_BYTES.saturating_sub(s.cost()),
                s.context.clone(),
            )
        };
        if let Some(pages) = self.pages.upgrade() {
            pages.set_visible_child_name("analysis");
        }
        self.analysis_view
            .reset("Reading referenced files from allowed folders…");
        self.feedback("Analyzing the selected root snapshot…");
        self.details();
        let context = hyprland_editor_core::module_path::Context {
            main_file: root.identity.selected_path.clone(),
            home: options.home,
            inherited_package_path: options.inherited_package_path,
        };
        let ui = self.clone();
        workflow::background(
            move || {
                super::source_graph::analyze_with_budget(
                    root,
                    context,
                    grants,
                    version,
                    options.working_directory,
                    cancel,
                    budget,
                )
            },
            move |result| {
                let Some(session) = ui.finish(epoch) else {
                    ui.analysis_view
                        .reset("The draft or source changed. Reopen dependency review.");
                    return;
                };
                session.borrow_mut().analysis_cancel = None;
                match result {
                    Ok(analysis) => {
                        if session
                            .borrow()
                            .cost()
                            .saturating_add(analysis.retained_bytes)
                            > MAX_WORKSPACE_BYTES
                        {
                            ui.analysis_view
                                .reset("Remove captures before retaining a larger analysis.");
                        } else {
                            *session.borrow().analysis.borrow_mut() = Some(Arc::new(analysis));
                            ui.analysis_view.render();
                            ui.feedback(
                                "Analysis is ready. Draft and desktop settings are unchanged.",
                            );
                        }
                    }
                    Err(error) => {
                        ui.analysis_view
                            .reset(&display_path_bytes(error.as_bytes()));
                        ui.feedback(&error);
                    }
                }
                ui.details();
            },
        );
    }
    fn set_context(&self, options: super::source_context::Options) -> Result<(), String> {
        options.validate()?;
        if !self.current() {
            return Err("The draft or source changed. Reopen dependency review.".into());
        }
        let session = self.session.upgrade().ok_or("Dependency review closed")?;
        let mut s = session.borrow_mut();
        if s.pending {
            return Err("Wait for the current operation before changing context.".into());
        }
        let cost = encoded_len(&options)?;
        let analysis_cost = s.analysis.borrow().as_ref().map_or(0, |a| a.retained_bytes);
        if s.cost()
            .saturating_sub(s.context_cost)
            .saturating_sub(analysis_cost)
            .saturating_add(cost)
            > MAX_WORKSPACE_BYTES
        {
            return Err("Remove captures before retaining this larger context.".into());
        }
        s.clear_analysis();
        s.context = options;
        s.context_cost = cost;
        s.epoch += 1;
        drop(s);
        self.analysis_view
            .reset("Context changed. Analyze the selected snapshot again.");
        self.feedback("Analysis context updated for this review. Folder access and draft settings are unchanged.");
        self.details();
        Ok(())
    }
    fn edit_context(&self) -> Option<(adw::Window, super::source_context::Editor)> {
        if !self.current() {
            self.details();
            return None;
        }
        let parent = self.window.upgrade()?;
        let session = self.session.upgrade()?;
        if session.borrow().pending {
            return None;
        }
        let options = session.borrow().context.clone();
        let u = self.clone();
        Some(super::source_context::show(
            &parent,
            options,
            move |options| u.set_context(options),
        ))
    }
}
pub(super) fn excerpt(source: &str) -> String {
    let mut end = source.len().min(16 * 1024);
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    let mut result = source[..end]
        .split('\n')
        .map(|line| display_path_bytes(line.as_bytes()))
        .collect::<Vec<_>>()
        .join("\n");
    if end < source.len() {
        result.push_str("\n… Showing the first 16 KiB. The complete captured snapshot remains in this review session.");
    }
    result
}
fn text_view(name: &str, height: i32, code: bool) -> (gtk::ScrolledWindow, gtk::TextView) {
    let view = gtk::TextView::builder()
        .editable(false)
        .monospace(code)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(8)
        .right_margin(8)
        .top_margin(8)
        .bottom_margin(8)
        .build();
    view.set_widget_name(name);
    (
        gtk::ScrolledWindow::builder()
            .child(&view)
            .min_content_height(height)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
        view,
    )
}
fn show(
    parent: &adw::Window,
    state: &Shared,
    version: String,
    root_hash: String,
) -> (adw::Window, Controls) {
    let win = adw::Window::builder()
        .title("Dependency files")
        .transient_for(parent)
        .default_width(700)
        .default_height(660)
        .build();
    win.set_size_request(350, 420);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    root.append(&header);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_start(16);
    body.set_margin_end(16);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    let introduction=label("Allow a folder, then choose files to capture for review. Closing this window discards these temporary captures. Multi-file editing and saving are still in development.");
    introduction.set_selectable(false);
    body.append(&introduction);
    let add = gtk::Button::with_label("Allow Folder…");
    add.set_widget_name("dependency-allow-folder");
    body.append(&add);
    let folders = gtk::DropDown::from_strings(&[]);
    compact(&folders);
    folders.set_widget_name("dependency-folder");
    folders.update_property(&[gtk::accessible::Property::Label(
        "Allowed dependency folder",
    )]);
    body.append(&folders);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let read = gtk::Button::with_label("Choose File…");
    read.set_widget_name("dependency-choose-file");
    read.set_hexpand(true);
    row.append(&read);
    let forget = gtk::Button::with_label("Forget Folder");
    forget.set_widget_name("dependency-forget-folder");
    forget.set_tooltip_text(Some("Stop using this folder for future captures"));
    row.append(&forget);
    body.append(&row);
    let (area, folder_details) = text_view("dependency-folder-details", 100, false);
    body.append(
        &gtk::Expander::builder()
            .label("Folder details")
            .child(&area)
            .build(),
    );
    body.append(&label("Captured files"));
    let captures = gtk::DropDown::from_strings(&[]);
    compact(&captures);
    captures.set_widget_name("dependency-capture");
    captures.update_property(&[gtk::accessible::Property::Label(
        "Captured dependency snapshot",
    )]);
    body.append(&captures);
    let (area, preview) = text_view("dependency-source-preview", 160, true);
    body.append(&area);
    let remove = gtk::Button::with_label("Remove Capture");
    remove.set_widget_name("dependency-remove-capture");
    let analyze = gtk::Button::with_label("Analyze File…");
    analyze.set_widget_name("dependency-analyze-root");
    analyze.set_tooltip_text(Some(
        "Use this snapshot as the main file and read dependencies only from allowed folders",
    ));
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    analyze.set_hexpand(true);
    actions.append(&analyze);
    actions.append(&remove);
    body.append(&actions);
    let context = gtk::Button::with_label("Analysis Context…");
    context.set_widget_name("dependency-analysis-context");
    context.set_tooltip_text(Some(
        "Supply home, working directory and inherited Lua search path for this snapshot",
    ));
    body.append(&context);
    let (area, capture_details) = text_view("dependency-capture-details", 100, false);
    body.append(
        &gtk::Expander::builder()
            .label("Capture details")
            .child(&area)
            .build(),
    );
    let feedback = label("No files read. Folder and file selection are required.");
    feedback.set_widget_name("dependency-feedback");
    body.append(&feedback);
    let pages = gtk::Stack::new();
    pages.set_vexpand(true);
    pages.add_named(
        &gtk::ScrolledWindow::builder()
            .child(&body)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
        Some("files"),
    );
    let analysis_model = Rc::new(RefCell::new(None));
    let (analysis_body, analysis_view) = super::source_analysis::build(&analysis_model);
    for set in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        set(analysis_body.upcast_ref(), 16);
    }
    let back = gtk::Button::from_icon_name("go-previous-symbolic");
    back.set_tooltip_text(Some("Back to files"));
    back.update_property(&[gtk::accessible::Property::Label("Back to files")]);
    back.set_visible(false);
    back.set_widget_name("dependency-analysis-back");
    let cancel = gtk::Button::with_label("Stop");
    cancel.set_widget_name("dependency-analysis-stop");
    header.pack_start(&back);
    header.pack_end(&cancel);
    pages.add_named(
        &gtk::ScrolledWindow::builder()
            .child(&analysis_body)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
        Some("analysis"),
    );
    root.append(&pages);
    win.set_content(Some(&root));
    let session = Rc::new(RefCell::new(Session {
        version,
        root_hash,
        generation: state.borrow().generation,
        folders: vec![],
        captures: vec![],
        pending: false,
        alive: true,
        epoch: 0,
        analysis: analysis_model,
        analysis_cancel: None,
        context: super::source_context::Options::default(),
        context_cost: 0,
        #[cfg(test)]
        finished_epoch: 0,
    }));
    let ui = Controls {
        session: Rc::downgrade(&session),
        state: Rc::downgrade(state),
        window: win.downgrade(),
        folders: folders.downgrade(),
        captures: captures.downgrade(),
        add: add.downgrade(),
        forget: forget.downgrade(),
        read: read.downgrade(),
        remove: remove.downgrade(),
        feedback: feedback.downgrade(),
        folder_details: folder_details.downgrade(),
        capture_details: capture_details.downgrade(),
        preview: preview.downgrade(),
        analyze: analyze.downgrade(),
        context: context.downgrade(),
        cancel: cancel.downgrade(),
        pages: pages.downgrade(),
        analysis_view,
    };
    win.connect_close_request(move |_| {
        let mut s = session.borrow_mut();
        s.alive = false;
        s.clear();
        gtk::glib::Propagation::Proceed
    });
    let u = ui.clone();
    analyze.connect_clicked(move |_| u.analyze());
    let u = ui.clone();
    context.connect_clicked(move |_| {
        u.edit_context();
    });
    let b = back.downgrade();
    let w = win.downgrade();
    let u = ui.clone();
    pages.connect_visible_child_name_notify(move |pages| {
        let analyzing = pages.visible_child_name().as_deref() == Some("analysis");
        if let Some(b) = b.upgrade() {
            b.set_visible(analyzing);
        }
        if let Some(w) = w.upgrade() {
            w.set_title(Some(if analyzing {
                "Source analysis"
            } else {
                "Dependency files"
            }));
        }
        u.details();
    });
    let p = pages.downgrade();
    back.connect_clicked(move |_| {
        if let Some(p) = p.upgrade() {
            p.set_visible_child_name("files");
        }
    });
    let u = ui.clone();
    cancel.connect_clicked(move |_| {
        if let Some(s) = u.session.upgrade() {
            if let Some(cancel) = s.borrow().analysis_cancel.as_ref() {
                cancel.store(true, Ordering::Relaxed);
            }
        }
        u.feedback("Stopping analysis…");
        u.details();
    });
    let u = ui.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        let Some(session) = u.session.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        if !session.borrow().alive {
            return gtk::glib::ControlFlow::Break;
        }
        if !u.current() {
            session.borrow_mut().clear();
            u.analysis_view
                .reset("The draft or source changed. Reopen dependency review.");
            u.refresh(None, None);
            return gtk::glib::ControlFlow::Break;
        }
        gtk::glib::ControlFlow::Continue
    });
    let u = ui.clone();
    folders.connect_selected_notify(move |_| u.details());
    let u = ui.clone();
    captures.connect_selected_notify(move |_| u.details());
    let u = ui.clone();
    add.connect_clicked(move |_| {
        let Some(win) = u.window.upgrade() else {
            return;
        };
        if !u.current() {
            u.details();
            return;
        }
        let u = u.clone();
        gtk::FileDialog::builder()
            .title("Allow a dependency folder")
            .build()
            .select_folder(Some(&win), None::<&gtk::gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        u.approve(path);
                    }
                }
            });
    });
    let u = ui.clone();
    read.connect_clicked(move |_| {
        let Some(win) = u.window.upgrade() else {
            return;
        };
        if !u.current() {
            u.details();
            return;
        }
        let dialog = gtk::FileDialog::builder()
            .title("Capture a file within the allowed folder")
            .build();
        if let (Some(session), Some(pick)) = (u.session.upgrade(), u.folders.upgrade()) {
            if let Some(folder) = session.borrow().folders.get(pick.selected() as usize) {
                // A live user selection is a chooser hint; only the descriptor grants a read.
                let path =
                    PathBuf::from(std::ffi::OsStr::from_bytes(&folder.evidence.selected_path));
                dialog.set_initial_folder(Some(&gtk::gio::File::for_path(path)));
            }
        }
        let u = u.clone();
        dialog.open(Some(&win), None::<&gtk::gio::Cancellable>, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    u.capture(path);
                }
            }
        });
    });
    let u = ui.clone();
    forget.connect_clicked(move|_|{
        let (Some(session),Some(pick))=(u.session.upgrade(),u.folders.upgrade())else{return;};
        let mut s=session.borrow_mut();if s.pending{return;}let index=pick.selected() as usize;if index<s.folders.len(){s.folders.remove(index);s.epoch+=1;}drop(s);u.refresh(None,None);u.feedback("Folder forgotten. Existing captured snapshots remain available until removed or this window closes.");
    });
    let u = ui.clone();
    remove.connect_clicked(move |_| {
        let (Some(session), Some(pick)) = (u.session.upgrade(), u.captures.upgrade()) else {
            return;
        };
        let mut s = session.borrow_mut();
        if s.pending {
            return;
        }
        let index = pick.selected() as usize;
        if index < s.captures.len() {
            s.captures.remove(index);
            s.clear_analysis();
        }
        drop(s);
        u.refresh(None, None);
        u.analysis_view
            .reset("Analysis cleared with its source capture.");
        u.feedback("Capture removed.");
    });
    ui.refresh(None, None);
    win.present();
    (win, ui)
}
pub(super) fn connect(
    button: &gtk::Button,
    parent: &adw::Window,
    state: &Shared,
    version: String,
    root_hash: String,
) {
    let parent = parent.downgrade();
    let state = Rc::downgrade(state);
    button.connect_clicked(move |_| {
        if let (Some(parent), Some(state)) = (parent.upgrade(), state.upgrade()) {
            show(&parent, &state, version.clone(), root_hash.clone());
        }
    });
}

#[cfg(test)]
pub(super) fn test_review(parent: &adw::Window, state: &Shared) {
    fn wait(test: impl Fn() -> bool) {
        let context = gtk::glib::MainContext::default();
        let start = std::time::Instant::now();
        loop {
            while context.pending() {
                context.iteration(false);
            }
            if test() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(6),
                "Dependency workflow did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    let root = std::env::temp_dir().join(format!(
        "he-dependency-native-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let marker = root.join("must-not-execute");
    let path = root.join("theme.lua");
    let source = format!(
        "-- captured theme\nos.execute('touch {}')\nhl.config{{general={{border_size=3}}}}\n",
        marker.display()
    );
    std::fs::write(&path, &source).unwrap();
    let (version, hash, generation) = {
        let s = state.borrow();
        (
            s.draft.version.clone(),
            s.sources[&s.draft.version].report.source_sha256.clone(),
            s.generation,
        )
    };
    let (win, ui) = show(parent, state, version.clone(), hash.clone());
    assert!(!ui.read.upgrade().unwrap().is_sensitive());
    ui.approve(root.clone());
    wait(|| ui.session.upgrade().is_some_and(|s| !s.borrow().pending));
    let session = ui.session.upgrade().unwrap();
    assert_eq!(
        session.borrow().folders.len(),
        1,
        "{}",
        ui.feedback.upgrade().unwrap().text()
    );
    let weak_grant = Arc::downgrade(&session.borrow().folders[0].grant);
    ui.capture(path.clone());
    wait(|| !session.borrow().pending);
    assert_eq!(
        session.borrow().captures.len(),
        1,
        "{}",
        ui.feedback.upgrade().unwrap().text()
    );
    let weak_source = Arc::downgrade(&session.borrow().captures[0].source);
    let preview = ui.preview.upgrade().unwrap();
    let buffer = preview.buffer();
    assert!(buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .contains("os.execute"));
    assert!(!marker.exists());
    assert_eq!(state.borrow().generation, generation);
    // A capture is immutable, not a watcher of the source filename.
    std::fs::write(&path, "-- newer bytes\n").unwrap();
    assert_eq!(session.borrow().captures[0].source.content, source);
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o750)).unwrap();
    ui.approve(root.clone());
    wait(|| !session.borrow().pending);
    assert_eq!(session.borrow().folders.len(), 1);
    assert_eq!(session.borrow().folders[0].evidence.mode & 0o777, 0o750);
    assert!(
        weak_grant.upgrade().is_none(),
        "Reapproval retained the previous folder grant"
    );
    let weak_grant = Arc::downgrade(&session.borrow().folders[0].grant);
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        wait(|| win.width() > 0 && win.height() > 0);
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        // Model replacement invalidates the paintable until the next frame. Wait for an
        // actual render node, rather than treating an allocated window as already painted.
        let node = RefCell::new(None);
        wait(|| {
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
            *node.borrow_mut() = snapshot.to_node();
            node.borrow().is_some()
        });
        win.renderer()
            .unwrap()
            .render_texture(node.into_inner().unwrap(), None)
            .save_to_png(format!("{path}.dependency-files.png"))
            .unwrap();
    }
    let (minimum, _, _, _) = win.measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Dependency review requires {minimum}px");
    ui.forget.upgrade().unwrap().emit_clicked();
    assert!(session.borrow().folders.is_empty());
    assert!(
        weak_grant.upgrade().is_none(),
        "Forgetting a folder retained read authority"
    );
    assert!(
        weak_source.upgrade().is_some(),
        "Forgetting access must preserve already captured bytes"
    );
    ui.remove.upgrade().unwrap().emit_clicked();
    assert!(session.borrow().captures.is_empty());
    assert!(weak_source.upgrade().is_none());
    // Actual captured resolution now feeds a focused, read-only graphical model.
    let main = root.join("main.lua");
    std::fs::write(
        &main,
        "local part=require('part');hl.config{general={border_size=part}}",
    )
    .unwrap();
    std::fs::write(root.join("part.lua"), "return 4").unwrap();
    ui.approve(root.clone());
    wait(|| !session.borrow().pending);
    ui.capture(main.clone());
    wait(|| !session.borrow().pending);
    ui.analyze.upgrade().unwrap().emit_clicked();
    wait(|| !session.borrow().pending);
    let model = session.borrow().analysis.clone();
    let weak_analysis = {
        let m = model.borrow();
        let a = m.as_ref().expect("No graphical analysis");
        assert_eq!(a.captures.len(), 2);
        assert!(a.report.halt.is_none());
        assert!(!a.report.runtime_verified);
        Arc::downgrade(a)
    };
    let weak_dependency = Arc::downgrade(&model.borrow().as_ref().unwrap().captures[1].source);
    assert_eq!(
        ui.pages.upgrade().unwrap().visible_child_name().as_deref(),
        Some("analysis")
    );
    let text = ui.analysis_view.test_call(1);
    assert!(text.contains("border_size: 4"), "{text}");
    let sources = ui.analysis_view.test_sources();
    assert!(
        sources.contains("Argument 1.general.border_size"),
        "{sources}"
    );
    assert!(
        sources.contains("part.lua") && sources.contains("Literal"),
        "{sources}"
    );
    assert_eq!(state.borrow().generation, generation);
    let (minimum, _, _, _) = win.measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Analysis requires {minimum}px");
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        // A render node can precede the tab's final CSS transition frame.
        let transition = std::time::Instant::now();
        wait(|| transition.elapsed() >= std::time::Duration::from_millis(300));
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let node = RefCell::new(None);
        wait(|| {
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
            *node.borrow_mut() = snapshot.to_node();
            node.borrow().is_some()
        });
        win.renderer()
            .unwrap()
            .render_texture(node.into_inner().unwrap(), None)
            .save_to_png(format!("{path}.dependency-analysis.png"))
            .unwrap();
    }
    ui.remove.upgrade().unwrap().emit_clicked();
    assert!(model.borrow().is_none());
    assert!(weak_analysis.upgrade().is_none());
    assert!(weak_dependency.upgrade().is_none());
    // Missing context remains partial until supplied through the actual editor.
    let cwd = root.join("working");
    let home = root.join("home");
    let library = root.join("library");
    for folder in [&cwd, &home, &library] {
        std::fs::create_dir(folder).unwrap();
    }
    std::fs::write(cwd.join("relative.lua"), "return 1").unwrap();
    std::fs::write(home.join("home.lua"), "return 2").unwrap();
    std::fs::write(library.join("named.lua"), "return 3").unwrap();
    std::fs::write(&main, "local a=dofile('relative.lua');local b=require('~/home');local c=require('named');hl.config{general={border_size=a+b+c}}").unwrap();
    ui.capture(main.clone());
    wait(|| !session.borrow().pending);
    ui.analyze();
    wait(|| !session.borrow().pending);
    assert!(session
        .borrow()
        .analysis
        .borrow()
        .as_ref()
        .unwrap()
        .blocked
        .as_deref()
        .is_some_and(|s| s.contains("working directory")));
    let old = Arc::downgrade(session.borrow().analysis.borrow().as_ref().unwrap());
    let options = super::source_context::Options {
        home: Some(home.as_os_str().as_bytes().to_vec()),
        working_directory: Some(cwd.as_os_str().as_bytes().to_vec()),
        inherited_package_path: Some([library.as_os_str().as_bytes(), b"/?.lua"].concat()),
    };
    let grants = session
        .borrow()
        .folders
        .iter()
        .map(|f| Arc::downgrade(&f.grant))
        .collect::<Vec<_>>();
    let (dialog, editor) = ui.edit_context().unwrap();
    editor.test_set(options.clone());
    let (minimum, _, _, _) = dialog.measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Context requires {minimum}px");
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&dialog));
        let node = RefCell::new(None);
        wait(|| {
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, dialog.width() as f64, dialog.height() as f64);
            *node.borrow_mut() = snapshot.to_node();
            node.borrow().is_some()
        });
        dialog
            .renderer()
            .unwrap()
            .render_texture(node.into_inner().unwrap(), None)
            .save_to_png(format!("{path}.dependency-context.png"))
            .unwrap();
    }
    editor.save.upgrade().unwrap().emit_clicked();
    assert!(!dialog.is_visible());
    drop(editor);
    drop(dialog);
    assert!(old.upgrade().is_none());
    assert_eq!(session.borrow().context, options);
    assert_eq!(session.borrow().folders.len(), grants.len());
    assert!(grants.iter().all(|g| g.upgrade().is_some()));
    ui.analyze();
    assert!(ui.edit_context().is_none());
    assert!(ui
        .set_context(super::source_context::Options::default())
        .is_err());
    wait(|| !session.borrow().pending);
    {
        let model = session.borrow().analysis.clone();
        let analysis = model.borrow();
        let analysis = analysis.as_ref().unwrap();
        assert!(analysis.blocked.is_none(), "{:?}", analysis.blocked);
        assert!(analysis.report.halt.is_none(), "{:?}", analysis.report.halt);
        assert_eq!(analysis.captures.len(), 4);
        assert_eq!(analysis.context.home, options.home);
        assert_eq!(
            analysis.program.working_directory,
            options.working_directory
        );
    }
    assert!(ui.analysis_view.test_call(3).contains("border_size: 6"));
    assert_eq!(state.borrow().generation, generation);
    // Canceling edits changes no retained options or completed analysis.
    let old = Arc::downgrade(session.borrow().analysis.borrow().as_ref().unwrap());
    let (dialog, editor) = ui.edit_context().unwrap();
    editor.test_set(super::source_context::Options::default());
    dialog.close();
    drop(editor);
    drop(dialog);
    assert_eq!(session.borrow().context, options);
    assert!(old.upgrade().is_some());
    let mut unknown = options.clone();
    unknown.inherited_package_path = None;
    ui.set_context(unknown).unwrap();
    assert!(old.upgrade().is_none());
    ui.analyze();
    wait(|| !session.borrow().pending);
    assert!(session
        .borrow()
        .analysis
        .borrow()
        .as_ref()
        .unwrap()
        .blocked
        .as_deref()
        .is_some_and(|s| s.contains("remaining search path is unknown")));
    let mut empty = options;
    empty.inherited_package_path = Some(vec![]);
    let (dialog, editor) = ui.edit_context().unwrap();
    editor.test_set(empty.clone());
    editor.save.upgrade().unwrap().emit_clicked();
    drop(editor);
    drop(dialog);
    assert_eq!(
        session.borrow().context.inherited_package_path,
        Some(vec![])
    );
    ui.analyze();
    wait(|| !session.borrow().pending);
    assert!(session
        .borrow()
        .analysis
        .borrow()
        .as_ref()
        .unwrap()
        .blocked
        .as_deref()
        .is_some_and(|s| s.contains("native/custom searchers")));
    // A result for an old imported snapshot must not attach a new folder.
    ui.approve(root.clone());
    let pending_epoch = session.borrow().epoch;
    state.borrow_mut().generation += 1;
    // Invalidating a session clears UI pending state before a worker finishes.
    // Wait for this exact completion, not a timing-dependent pending=false flag.
    wait(|| session.borrow().finished_epoch >= pending_epoch);
    assert!(session.borrow().folders.is_empty());
    assert!(!ui.read.upgrade().unwrap().is_sensitive());
    assert!(ui
        .feedback
        .upgrade()
        .unwrap()
        .text()
        .contains("imported source changed"));
    win.close();
    drop(win);
    drop(session);
    state.borrow_mut().generation = generation;
    state.borrow_mut().draft.version = version.clone();
    let (win, ui) = show(parent, state, version, hash);
    ui.approve(root.clone());
    wait(|| ui.session.upgrade().is_some_and(|s| !s.borrow().pending));
    let session = ui.session.upgrade().unwrap();
    let weak_grant = Arc::downgrade(&session.borrow().folders[0].grant);
    let weak_session = Rc::downgrade(&session);
    ui.capture(path);
    win.close();
    assert!(session.borrow().folders.is_empty());
    assert!(session.borrow().captures.is_empty());
    drop(session);
    drop(win);
    wait(|| weak_grant.upgrade().is_none() && weak_session.upgrade().is_none());
    assert_eq!(state.borrow().generation, generation);
    // Completed results also become invalid when the draft changes while idle.
    let (win, ui) = show(
        parent,
        state,
        state.borrow().draft.version.clone(),
        state.borrow().sources[&state.borrow().draft.version]
            .report
            .source_sha256
            .clone(),
    );
    ui.approve(root.clone());
    wait(|| ui.session.upgrade().is_some_and(|s| !s.borrow().pending));
    ui.capture(main.clone());
    wait(|| ui.session.upgrade().is_some_and(|s| !s.borrow().pending));
    ui.analyze();
    wait(|| ui.session.upgrade().is_some_and(|s| !s.borrow().pending));
    let session = ui.session.upgrade().unwrap();
    let weak_analysis = Arc::downgrade(session.borrow().analysis.borrow().as_ref().unwrap());
    let (dialog, editor) = ui.edit_context().unwrap();
    let attempted = super::source_context::Options {
        inherited_package_path: Some(vec![]),
        ..Default::default()
    };
    editor.test_set(attempted);
    state.borrow_mut().generation += 1;
    editor.save.upgrade().unwrap().emit_clicked();
    assert!(
        dialog.is_visible(),
        "Stale context must be refused, with the editor left open"
    );
    assert_eq!(
        session.borrow().context,
        super::source_context::Options::default()
    );
    wait(|| session.borrow().analysis.borrow().is_none());
    assert!(weak_analysis.upgrade().is_none());
    assert!(!ui.analyze.upgrade().unwrap().is_sensitive());
    dialog.close();
    drop(editor);
    drop(dialog);
    win.close();
    drop(session);
    drop(win);
    state.borrow_mut().generation = generation;
    // Closing during analysis cancels future steps and releases captured results.
    let (win, ui) = show(
        parent,
        state,
        state.borrow().draft.version.clone(),
        state.borrow().sources[&state.borrow().draft.version]
            .report
            .source_sha256
            .clone(),
    );
    ui.approve(root.clone());
    wait(|| ui.session.upgrade().is_some_and(|s| !s.borrow().pending));
    ui.capture(main);
    wait(|| ui.session.upgrade().is_some_and(|s| !s.borrow().pending));
    let session = ui.session.upgrade().unwrap();
    let weak_session = Rc::downgrade(&session);
    let weak_grant = Arc::downgrade(&session.borrow().folders[0].grant);
    ui.analyze();
    let cancel = session.borrow().analysis_cancel.clone().unwrap();
    win.close();
    assert!(cancel.load(Ordering::Relaxed));
    drop(session);
    drop(win);
    wait(|| weak_session.upgrade().is_none() && weak_grant.upgrade().is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoded_retention_budget_refuses_atomically_and_preview_bounds_unicode_and_controls() {
        let root =
            std::env::temp_dir().join(format!("he-dependency-budget-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let scope = hyprland_editor_core::source_scope::Scope::select(&root).unwrap();
        let folder = Arc::new(scope.identity().clone());
        let content = "\u{1}".repeat(MAX_DOCUMENT_BYTES);
        let mut session = Session {
            version: hyprland_editor_core::BASELINE.into(),
            root_hash: String::new(),
            generation: 0,
            folders: vec![],
            captures: vec![],
            pending: false,
            alive: true,
            epoch: 0,
            analysis: Rc::new(RefCell::new(None)),
            analysis_cancel: None,
            context: crate::source_context::Options::default(),
            context_cost: 0,
            finished_epoch: 0,
        };
        for name in ["first.lua", "second.lua"] {
            std::fs::write(root.join(name), &content).unwrap();
        }
        let capture = |name: &str| {
            Capture::prepare(
                hyprland_editor_core::source_path::read_selected(&root.join(name)).unwrap(),
                folder.clone(),
            )
            .unwrap()
        };
        session.add_capture(capture("first.lua")).unwrap();
        let cost = session.cost();
        assert!(cost > 6 * MAX_DOCUMENT_BYTES);
        assert!(session.add_capture(capture("second.lua")).is_err());
        assert_eq!(session.captures.len(), 1);
        assert_eq!(session.cost(), cost);
        assert_eq!(session.add_capture(capture("first.lua")).unwrap(), 0);
        let preview = excerpt(&format!("one\ntwo\u{202e}{}", "é".repeat(9000)));
        assert!(preview.starts_with("one\ntwo"));
        assert!(!preview.contains('\u{202e}'));
        assert!(preview.contains("16 KiB"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
