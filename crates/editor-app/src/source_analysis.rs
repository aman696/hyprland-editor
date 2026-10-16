//! Focused read-only presentation of the bounded captured-source model.
use super::{label, monitors::compact, source_graph::Analysis};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::replay::{Halt, LoadOutcome, Loader, Observed},
    source::display_path_bytes,
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
};
pub(super) type Model = Rc<RefCell<Option<Arc<Analysis>>>>;
#[derive(Clone, Copy)]
enum Call {
    Api(usize),
    Load(usize),
}
#[derive(Clone)]
pub(super) struct View {
    model: Weak<RefCell<Option<Arc<Analysis>>>>,
    summary: gtk::glib::WeakRef<gtk::Label>,
    status: gtk::glib::WeakRef<gtk::Label>,
    files: gtk::glib::WeakRef<gtk::DropDown>,
    calls: gtk::glib::WeakRef<gtk::DropDown>,
    source: gtk::glib::WeakRef<gtk::TextView>,
    detail: gtk::glib::WeakRef<gtk::TextView>,
    order: Rc<RefCell<Vec<Call>>>,
    #[cfg(test)]
    tabs: gtk::glib::WeakRef<gtk::Stack>,
}
fn short(bytes: &[u8]) -> String {
    let text = display_path_bytes(&bytes[..bytes.len().min(4096)]);
    let mut value = text.chars().take(160).collect::<String>();
    if text.chars().count() > 160 || bytes.len() > 4096 {
        value.push('…');
    }
    value
}
fn text_area(name: &str) -> (gtk::ScrolledWindow, gtk::TextView) {
    let view = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
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
            .min_content_height(180)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
        view,
    )
}
pub(super) fn build(model: &Model) -> (gtk::Box, View) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let summary = label("No analysis yet.");
    summary.set_widget_name("dependency-analysis-summary");
    body.append(&summary);
    let status=label("Only files in allowed folders can be read. Source code is never executed, and drafts and desktop settings stay unchanged.");
    status.set_widget_name("dependency-analysis-status");
    body.append(&status);
    let tabs = gtk::Stack::new();
    let files_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let files = gtk::DropDown::from_strings(&[]);
    compact(&files);
    files.set_enable_search(true);
    files.set_widget_name("dependency-analysis-file");
    files.update_property(&[gtk::accessible::Property::Label("Analyzed source snapshot")]);
    files_page.append(&files);
    let (area, source) = text_area("dependency-analysis-source");
    files_page.append(&area);
    tabs.add_titled(&files_page, Some("files"), "Files");
    let calls_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let calls = gtk::DropDown::from_strings(&[]);
    compact(&calls);
    calls.set_enable_search(true);
    calls.set_widget_name("dependency-analysis-call");
    calls.update_property(&[gtk::accessible::Property::Label("Calls in modeled order")]);
    calls_page.append(&calls);
    let (area, detail) = text_area("dependency-analysis-call-details");
    calls_page.append(&area);
    tabs.add_titled(&calls_page, Some("calls"), "Ordered calls");
    let switcher = gtk::StackSwitcher::builder().stack(&tabs).build();
    body.append(&switcher);
    body.append(&tabs);
    let view = View {
        model: Rc::downgrade(model),
        summary: summary.downgrade(),
        status: status.downgrade(),
        files: files.downgrade(),
        calls: calls.downgrade(),
        source: source.downgrade(),
        detail: detail.downgrade(),
        order: Rc::new(RefCell::new(vec![])),
        #[cfg(test)]
        tabs: tabs.downgrade(),
    };
    let v = view.clone();
    files.connect_selected_notify(move |_| v.file());
    let v = view.clone();
    calls.connect_selected_notify(move |_| v.call());
    (body, view)
}
impl View {
    #[cfg(test)]
    pub(super) fn test_call(&self, index: u32) -> String {
        self.tabs.upgrade().unwrap().set_visible_child_name("calls");
        let tabs = self.tabs.upgrade().unwrap();
        let switcher = tabs
            .prev_sibling()
            .unwrap()
            .downcast::<gtk::StackSwitcher>()
            .unwrap();
        let first = switcher
            .first_child()
            .unwrap()
            .downcast::<gtk::ToggleButton>()
            .unwrap();
        let second = first
            .next_sibling()
            .unwrap()
            .downcast::<gtk::ToggleButton>()
            .unwrap();
        assert!(!first.is_active());
        assert!(second.is_active());
        self.calls.upgrade().unwrap().set_selected(index);
        let buffer = self.detail.upgrade().unwrap().buffer();
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .into()
    }
    pub(super) fn reset(&self, message: &str) {
        if let Some(model) = self.model.upgrade() {
            model.borrow_mut().take();
        }
        self.order.borrow_mut().clear();
        if let Some(label) = self.summary.upgrade() {
            label.set_text(message);
        }
        if let Some(label) = self.status.upgrade() {
            label.set_text("This analysis does not change the draft or desktop.");
        }
        for weak in [&self.files, &self.calls] {
            if let Some(pick) = weak.upgrade() {
                pick.set_model(Some(&gtk::StringList::new(&[])));
            }
        }
        for weak in [&self.source, &self.detail] {
            if let Some(view) = weak.upgrade() {
                view.buffer().set_text("");
            }
        }
    }
    pub(super) fn render(&self) {
        let Some(analysis) = self.model.upgrade().and_then(|m| m.borrow().clone()) else {
            return;
        };
        let complete = analysis.blocked.is_none() && analysis.report.halt.is_none();
        if let Some(label) = self.summary.upgrade() {
            label.set_text(&format!(
                "{} · {} files · {} calls",
                if complete {
                    "Snapshot analysis finished"
                } else {
                    "Partial analysis"
                },
                analysis.captures.len(),
                analysis.report.events.len() + analysis.report.loads.len()
            ));
        }
        let reason = analysis
            .blocked
            .clone()
            .or_else(|| match &analysis.report.halt {
                Some(Halt::Unresolved { reason, .. } | Halt::Budget { reason }) => {
                    Some(reason.clone())
                }
                Some(Halt::LuaError { value: error }) => {
                    let mut message = String::from("Source error: ");
                    value(&mut message, error, 0);
                    Some(message)
                }
                Some(Halt::NeedsResolution { .. }) => {
                    Some("More dependency information is needed.".into())
                }
                None => None,
            });
        if let Some(label) = self.status.upgrade() {
            label.set_text(&reason.map_or_else(||format!("{} module errors. These are modeled calls; setting acceptance and effective desktop values are not verified.",analysis.report.logged_module_errors),|r|format!("Stopped: {}",display_path_bytes(r.as_bytes()))));
        }
        if let Some(pick) = self.files.upgrade() {
            let names = analysis
                .program
                .documents
                .iter()
                .map(|d| short(&d.load_name))
                .collect::<Vec<_>>();
            pick.set_model(Some(&gtk::StringList::new(
                &names.iter().map(String::as_str).collect::<Vec<_>>(),
            )));
            pick.set_selected(0);
        }
        let mut rows = analysis
            .report
            .events
            .iter()
            .enumerate()
            .map(|(i, e)| {
                (
                    e.order,
                    Call::Api(i),
                    format!("{} · Settings declaration", e.order + 1),
                )
            })
            .chain(analysis.report.loads.iter().enumerate().map(|(i, e)| {
                (
                    e.order,
                    Call::Load(i),
                    format!("{} · {}", e.order + 1, short(&e.request.module)),
                )
            }))
            .collect::<Vec<_>>();
        rows.sort_by_key(|r| r.0);
        *self.order.borrow_mut() = rows.iter().map(|r| r.1).collect();
        if let Some(pick) = self.calls.upgrade() {
            pick.set_model(Some(&gtk::StringList::new(
                &rows.iter().map(|r| r.2.as_str()).collect::<Vec<_>>(),
            )));
            pick.set_selected(0);
        }
        self.file();
        self.call();
    }
    fn file(&self) {
        let (Some(analysis), Some(pick), Some(view)) = (
            self.model.upgrade().and_then(|m| m.borrow().clone()),
            self.files.upgrade(),
            self.source.upgrade(),
        ) else {
            return;
        };
        let Some(doc) = analysis.program.documents.get(pick.selected() as usize) else {
            view.buffer().set_text("");
            return;
        };
        view.buffer().set_text(&format!(
            "{}\nSHA-256: {}\n\n{}",
            display_path_bytes(&doc.load_name),
            doc.sha256,
            super::source_access::excerpt(&doc.source)
        ));
    }
    fn call(&self) {
        let (Some(analysis), Some(pick), Some(view)) = (
            self.model.upgrade().and_then(|m| m.borrow().clone()),
            self.calls.upgrade(),
            self.detail.upgrade(),
        ) else {
            return;
        };
        let Some(row) = self.order.borrow().get(pick.selected() as usize).copied() else {
            view.buffer()
                .set_text("No calls were observed before analysis stopped.");
            return;
        };
        let (document, span, body) = match row {
            Call::Api(i) => {
                let e = &analysis.report.events[i];
                let mut body = String::from(
                    "Settings declaration\nModeled arguments; acceptance is unverified.\n\n",
                );
                for arg in &e.arguments {
                    value(&mut body, arg, 0);
                    body.push('\n');
                }
                if body.contains('…') {
                    body.push_str("\nLong argument values are shown as excerpts.\n");
                }
                (e.document.as_str(), e.span.clone(), body)
            }
            Call::Load(i) => {
                let e = &analysis.report.loads[i];
                let kind = match e.request.loader {
                    Loader::SafeRequire | Loader::OriginalRequire => "Module load",
                    Loader::Dofile => "File call",
                    Loader::Loadfile => "Prepare a file function",
                };
                let outcome = match &e.outcome {
                    LoadOutcome::Cached => "Reused the cached module result.",
                    LoadOutcome::Preload => "Used the registered module function.",
                    LoadOutcome::SuppliedSnapshot { .. } => "Used a captured source snapshot.",
                    LoadOutcome::SuppliedMissing => "The supplied model records a missing file.",
                    LoadOutcome::NeedsResolution => "This dependency is unresolved.",
                };
                (
                    e.request.document.as_str(),
                    e.request.span.clone(),
                    format!(
                        "{kind}\n{}\n\n{outcome}",
                        display_path_bytes(&e.request.module)
                    ),
                )
            }
        };
        let doc = analysis.program.documents.iter().find(|d| d.id == document);
        let location = doc
            .map(|d| {
                format!(
                    "{} · line {}",
                    display_path_bytes(&d.load_name),
                    d.source
                        .get(..span.start)
                        .map_or(0, |s| s.bytes().filter(|b| *b == b'\n').count())
                        + 1
                )
            })
            .unwrap_or_else(|| document.into());
        view.buffer().set_text(&format!("{location}\n\n{body}"));
    }
}
fn value(out: &mut String, input: &Observed, depth: usize) {
    if out.len() > 16 * 1024 {
        out.push('…');
        return;
    }
    if depth > 8 {
        out.push('…');
        return;
    }
    match input {
        Observed::Nil => out.push_str("nil"),
        Observed::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Observed::Int(v) => out.push_str(&v.to_string()),
        Observed::Float(v) => out.push_str(&v.to_string()),
        Observed::Bytes(v) => {
            out.push('"');
            out.push_str(&display_path_bytes(&v[..v.len().min(1024)]).replace('"', "\\\""));
            if v.len() > 1024 {
                out.push('…');
            }
            out.push('"');
        }
        Observed::Table(rows) => {
            out.push_str("{\n");
            for (key, item) in rows.iter().take(128) {
                if out.len() > 16 * 1024 {
                    out.push('…');
                    break;
                }
                out.push_str(&"  ".repeat(depth + 1));
                if let Observed::Bytes(bytes) = key {
                    out.push_str(&display_path_bytes(&bytes[..bytes.len().min(256)]));
                    if bytes.len() > 256 {
                        out.push('…');
                    }
                } else {
                    value(out, key, depth + 1);
                }
                out.push_str(": ");
                value(out, item, depth + 1);
                out.push('\n');
            }
            out.push_str(&"  ".repeat(depth));
            out.push('}');
            if rows.len() > 128 {
                out.push_str(" …");
            }
        }
        Observed::Function(_) => out.push_str("function"),
        Observed::Cycle(_) => out.push_str("shared or cyclic table"),
    }
}
