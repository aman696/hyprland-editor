//! Focused read-only presentation of the bounded captured-source model.
use super::{label, monitors::compact, source_graph::Analysis};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::{
        config_table::{Batch, FindingKind},
        replay::{Halt, LoadOutcome, Loader, Observed},
        scalar_codec::{NativeValue, Outcome, Rejection, Representation},
    },
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
    state: Weak<RefCell<super::State>>,
    after_keep: Option<Rc<dyn Fn()>>,
    model: Weak<RefCell<Option<Arc<Analysis>>>>,
    summary: gtk::glib::WeakRef<gtk::Label>,
    status: gtk::glib::WeakRef<gtk::Label>,
    files: gtk::glib::WeakRef<gtk::DropDown>,
    calls: gtk::glib::WeakRef<gtk::DropDown>,
    source: gtk::glib::WeakRef<gtk::TextView>,
    detail: gtk::glib::WeakRef<gtk::TextView>,
    provenance: gtk::glib::WeakRef<gtk::TextView>,
    raw: gtk::glib::WeakRef<gtk::TextView>,
    raw_expander: gtk::glib::WeakRef<gtk::Expander>,
    order: Rc<RefCell<Vec<Call>>>,
    #[cfg(test)]
    tabs: gtk::glib::WeakRef<gtk::Stack>,
    source_expander: gtk::glib::WeakRef<gtk::Expander>,
    compare: gtk::glib::WeakRef<gtk::Button>,
    overrides: gtk::glib::WeakRef<gtk::Button>,
    change_window: Rc<RefCell<Option<gtk::glib::WeakRef<adw::Window>>>>,
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
pub(super) fn build(
    model: &Model,
    state: &super::Shared,
    after_keep: Option<Rc<dyn Fn()>>,
) -> (gtk::Box, View) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let summary = label("No analysis yet.");
    summary.set_widget_name("dependency-analysis-summary");
    body.append(&summary);
    let status=label("Only files in allowed folders can be read. Source code is never executed, and drafts and desktop settings stay unchanged.");
    status.set_widget_name("dependency-analysis-status");
    body.append(&status);
    let overrides = gtk::Button::with_label("Review Value Overrides");
    overrides.set_widget_name("source-analysis-overrides");
    overrides.set_halign(gtk::Align::Start);
    overrides.set_sensitive(false);
    body.append(&overrides);
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
    let compare = gtk::Button::with_label("Compare a Source Change");
    compare.set_sensitive(false);
    compare.set_widget_name("source-analysis-compare");
    compare.set_halign(gtk::Align::Start);
    calls_page.append(&compare);
    let (area, detail) = text_area("dependency-analysis-call-details");
    calls_page.append(&area);
    let detail_area = area.downgrade();
    let (raw_area, raw) = text_area("dependency-raw-arguments");
    raw.update_property(&[gtk::accessible::Property::Label(
        "Raw modeled settings arguments",
    )]);
    raw_area.set_vexpand(false);
    let raw_expander = gtk::Expander::builder()
        .label("Raw arguments")
        .child(&raw_area)
        .build();
    calls_page.append(&raw_expander);
    let (area, provenance) = text_area("dependency-value-sources");
    provenance.update_property(&[gtk::accessible::Property::Label(
        "Value source expression evidence",
    )]);
    area.set_vexpand(false);
    let source_expander = gtk::Expander::builder()
        .label("Value sources")
        .child(&area)
        .build();
    calls_page.append(&source_expander);
    let reflow: Rc<dyn Fn()> = Rc::new({
        let raw = raw_expander.downgrade();
        let source = source_expander.downgrade();
        let detail = detail_area;
        move || {
            let (Some(raw), Some(source), Some(detail)) =
                (raw.upgrade(), source.upgrade(), detail.upgrade())
            else {
                return;
            };
            let expanded = raw.is_expanded() || source.is_expanded();
            detail.set_vexpand(!expanded);
            detail.set_min_content_height(if expanded { 100 } else { 180 });
            for expander in [raw, source] {
                expander.set_vexpand(expander.is_expanded());
                if let Some(child) = expander.child() {
                    child.set_vexpand(expander.is_expanded());
                }
            }
        }
    });
    let weak = source_expander.downgrade();
    let resize = reflow.clone();
    raw_expander.connect_expanded_notify(move |e| {
        if e.is_expanded() {
            if let Some(other) = weak.upgrade() {
                other.set_expanded(false);
            }
        }
        resize();
    });
    let weak = raw_expander.downgrade();
    let resize = reflow;
    source_expander.connect_expanded_notify(move |e| {
        if e.is_expanded() {
            if let Some(other) = weak.upgrade() {
                other.set_expanded(false);
            }
        }
        resize();
    });
    tabs.add_titled(&calls_page, Some("calls"), "Ordered calls");
    let switcher = gtk::StackSwitcher::builder().stack(&tabs).build();
    body.append(&switcher);
    body.append(&tabs);
    let view = View {
        state: Rc::downgrade(state),
        after_keep,
        model: Rc::downgrade(model),
        summary: summary.downgrade(),
        status: status.downgrade(),
        files: files.downgrade(),
        calls: calls.downgrade(),
        source: source.downgrade(),
        detail: detail.downgrade(),
        provenance: provenance.downgrade(),
        raw: raw.downgrade(),
        raw_expander: raw_expander.downgrade(),
        order: Rc::new(RefCell::new(vec![])),
        #[cfg(test)]
        tabs: tabs.downgrade(),
        source_expander: source_expander.downgrade(),
        compare: compare.downgrade(),
        overrides: overrides.downgrade(),
        change_window: Rc::new(RefCell::new(None)),
    };
    let v = view.clone();
    files.connect_selected_notify(move |_| v.file());
    let v = view.clone();
    calls.connect_selected_notify(move |_| v.call());
    let v = view.clone();
    compare.connect_clicked(move |_| v.compare());
    let v = view.clone();
    overrides.connect_clicked(move |_| v.overrides());
    (body, view)
}
impl View {
    fn overrides(&self) {
        let (Some(model), Some(state), Some(button)) = (
            self.model.upgrade(),
            self.state.upgrade(),
            self.overrides.upgrade(),
        ) else {
            return;
        };
        if model.borrow().is_none() {
            return;
        }
        let Some(parent) = button.root().and_downcast::<gtk::Window>() else {
            return;
        };
        self.close_change();
        let view = self.clone();
        let window = super::source_overrides::open(
            &parent,
            &model,
            &state,
            Rc::new(move || {
                if let Some(callback) = &view.after_keep {
                    callback();
                }
                view.render();
            }),
        );
        *self.change_window.borrow_mut() = Some(window.downgrade());
    }
    #[cfg(test)]
    pub(super) fn test_compare(&self) -> adw::Window {
        self.compare.upgrade().unwrap().emit_clicked();
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<adw::Window>().ok())
            .find(|w| w.title().as_deref() == Some("Compare a Source Change") && w.is_visible())
            .unwrap()
    }
    fn close_change(&self) {
        if let Some(window) = self
            .change_window
            .borrow_mut()
            .take()
            .and_then(|w| w.upgrade())
        {
            window.close();
        }
    }
    fn compare(&self) {
        if let Some(window) = self
            .change_window
            .borrow()
            .as_ref()
            .and_then(|w| w.upgrade())
        {
            if window.is_visible() {
                window.present();
                return;
            }
        }
        let (Some(model), Some(pick), Some(button)) = (
            self.model.upgrade(),
            self.calls.upgrade(),
            self.compare.upgrade(),
        ) else {
            return;
        };
        let Some(Call::Api(index)) = self.order.borrow().get(pick.selected() as usize).copied()
        else {
            return;
        };
        let Some(analysis) = model.borrow().clone() else {
            return;
        };
        let event = &analysis.report.events[index];
        if event.api != "hl.config"
            || !analysis
                .config_batches
                .iter()
                .any(|b| b.event_order == event.order && !b.candidates.is_empty())
        {
            return;
        }
        let Some(parent) = button.root().and_downcast::<gtk::Window>() else {
            return;
        };
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let view = self.clone();
        let window = super::source_changes::open(
            &parent,
            &model,
            event.order,
            &state,
            Rc::new(move || {
                if let Some(callback) = &view.after_keep {
                    callback();
                }
                view.render();
            }),
        );
        *self.change_window.borrow_mut() = Some(window.downgrade());
    }
    #[cfg(test)]
    pub(super) fn test_raw(&self) -> String {
        let expander = self.raw_expander.upgrade().unwrap();
        assert!(!expander.is_expanded());
        expander.set_expanded(true);
        let b = self.raw.upgrade().unwrap().buffer();
        let text = b.text(&b.start_iter(), &b.end_iter(), true).into();
        expander.set_expanded(false);
        text
    }
    #[cfg(test)]
    pub(super) fn test_collapse_sources(&self) {
        self.source_expander.upgrade().unwrap().set_expanded(false);
    }
    #[cfg(test)]
    pub(super) fn test_status(&self) -> String {
        self.status.upgrade().unwrap().text().into()
    }
    #[cfg(test)]
    pub(super) fn test_sources(&self) -> String {
        self.source_expander.upgrade().unwrap().set_expanded(true);
        let buffer = self.provenance.upgrade().unwrap().buffer();
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .into()
    }
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
        self.close_change();
        if let Some(button) = self.overrides.upgrade() {
            button.set_sensitive(false);
        }
        if let Some(button) = self.compare.upgrade() {
            button.set_sensitive(false);
        }
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
        for weak in [&self.raw_expander, &self.source_expander] {
            if let Some(e) = weak.upgrade() {
                e.set_expanded(false);
            }
        }
        for weak in [&self.source, &self.detail, &self.provenance, &self.raw] {
            if let Some(view) = weak.upgrade() {
                view.buffer().set_text("");
            }
        }
    }
    pub(super) fn render(&self) {
        self.close_change();
        let Some(analysis) = self.model.upgrade().and_then(|m| m.borrow().clone()) else {
            return;
        };
        if let Some(button) = self.overrides.upgrade() {
            button.set_sensitive(self.state.upgrade().is_some_and(|s| {
                let s = s.borrow();
                s.draft.version == analysis.program.version && !s.draft.values.is_empty()
            }));
        }
        let edited = analysis
            .frozen
            .as_ref()
            .is_some_and(|b| !b.graph.revisions.is_empty());
        let complete = analysis.blocked.is_none() && analysis.report.halt.is_none();
        if let Some(label) = self.summary.upgrade() {
            label.set_text(&format!(
                "{} · {} files · {} calls",
                if complete {
                    if edited {
                        "Private source revision analysis finished"
                    } else {
                        "Snapshot analysis finished"
                    }
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
            let message = reason.map_or_else(
                || {
                    let errors = if analysis.report.logged_module_errors == 0 {
                        String::new()
                    } else {
                        format!("{} module errors. ", analysis.report.logged_module_errors)
                    };
                    format!(
                        "{errors}Read-only model for {}. Live desktop values are unverified.",
                        analysis.program.version
                    )
                },
                |r| format!("Stopped: {}", display_path_bytes(r.as_bytes())),
            );
            label.set_text(&if edited {
                format!("Private source revisions modeled. Original captures and separate value overrides are preserved. Review Value Overrides can reconcile scalar layers. Full source export remains unfinished. {message}")
            }else if analysis.restored {
                format!("Saved resolution inputs replayed. Original files and folder access were not reopened. {message}")
            }else{message});
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
        if let Some(bundle) = &analysis.frozen {
            if bundle.graph.revisions.iter().any(|r| r.document == doc.id) {
                if let Some(original) = bundle.sources.iter().find(|s| s.id == doc.id) {
                    view.buffer().set_text(&format!("{}\nPrivate draft SHA-256: {}\nOriginal capture SHA-256: {}\n\nPrivate source revision\n{}\n\nOriginal captured bytes (unchanged)\n{}",display_path_bytes(&doc.load_name),doc.sha256,original.origin.sha256,super::source_access::excerpt(&doc.source),super::source_access::excerpt(&original.content)));
                }
            }
        }
    }
    fn call(&self) {
        self.close_change();
        let (Some(analysis), Some(pick), Some(view)) = (
            self.model.upgrade().and_then(|m| m.borrow().clone()),
            self.calls.upgrade(),
            self.detail.upgrade(),
        ) else {
            return;
        };
        let Some(row) = self.order.borrow().get(pick.selected() as usize).copied() else {
            if let Some(button) = self.compare.upgrade() {
                button.set_sensitive(false);
            }
            view.buffer()
                .set_text("No calls were observed before analysis stopped.");
            for weak in [&self.provenance, &self.raw] {
                if let Some(view) = weak.upgrade() {
                    view.buffer().set_text("");
                }
            }
            for weak in [&self.raw_expander, &self.source_expander] {
                if let Some(e) = weak.upgrade() {
                    e.set_expanded(false);
                    e.set_visible(false);
                }
            }
            return;
        };
        if let Some(button) = self.compare.upgrade() {
            button.set_sensitive(match row {
                Call::Api(i) => {
                    analysis.report.events[i].api == "hl.config"
                        && analysis.config_batches.iter().any(|b| {
                            b.event_order == analysis.report.events[i].order
                                && !b.candidates.is_empty()
                        })
                }
                _ => false,
            });
        }
        for weak in [&self.raw_expander, &self.source_expander] {
            if let Some(e) = weak.upgrade() {
                e.set_visible(matches!(row, Call::Api(_)));
            }
        }
        if let Some(view) = self.raw.upgrade() {
            let mut text = String::new();
            if let Call::Api(i) = row {
                for arg in &analysis.report.events[i].arguments {
                    value(&mut text, arg, 0);
                    text.push('\n');
                }
            }
            if text.contains('…') {
                text.push_str("\nLong argument values are shown as excerpts.\n");
            }
            view.buffer().set_text(&text);
        }
        if let Some(view) = self.provenance.upgrade() {
            view.buffer().set_text(&match row {
                Call::Api(i)=>super::source_provenance::describe(&analysis,&analysis.report.events[i]),
                Call::Load(_)=>"Select a settings declaration to inspect its argument sources. Loader resolution evidence remains separate.".into(),
            });
        }
        let (document, span, body) = match row {
            Call::Api(i) => {
                let e = &analysis.report.events[i];
                let mut body = String::new();
                if let Some(batch) = analysis
                    .config_batches
                    .iter()
                    .find(|b| b.event_order == e.order)
                {
                    body.push_str(&configuration_fields(batch));
                } else if e.api == "hl.config" {
                    body.push_str("Field inspection unavailable.\n\n");
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
fn configuration_fields(batch: &Batch) -> String {
    let mut out = format!(
        "Recognized settings: {} · findings: {}\n",
        batch.candidates.len(),
        batch.findings.len()
    );
    for candidate in batch.candidates.iter().take(32) {
        out.push_str(&format!(
            "{} · {}{}\n",
            short(candidate.lua_key.as_bytes()),
            match candidate.value_kind.as_str() {
                "Bool" => "Toggle",
                "Int" => "Whole number",
                "Float" => "Number",
                "String" => "Text",
                "Color" => "Color",
                "Gradient" => "Gradient",
                "Vec2" => "Coordinates",
                "CssGap" => "Four-sided gaps",
                "FontWeight" => "Font weight",
                _ => "Setting",
            },
            if candidate.ambiguous {
                " · ambiguous"
            } else {
                ""
            }
        ));
        if let Some(model) = &candidate.model_value {
            let text = match model {
                Outcome::Accepted {
                    native,
                    representation,
                } => {
                    let mut text = match native {
                        NativeValue::Bool(v) => {
                            if *v {
                                "On".into()
                            } else {
                                "Off".into()
                            }
                        }
                        NativeValue::Int(v) => v.to_string(),
                        NativeValue::FloatBits(bits) => f32::from_bits(*bits).to_string(),
                        NativeValue::Bytes(bytes) => {
                            format!("\"{}\"", short(bytes).replace('"', "\\\""))
                        }
                    };
                    if matches!(representation, Representation::Unavailable { .. }) {
                        text.push_str(" · cannot fit the current draft format");
                    }
                    format!("Parsed value: {text}")
                }
                Outcome::Rejected { error } => format!(
                    "Input rejected by selected model: {}",
                    match error {
                        Rejection::OutOfRange => "outside its bounds",
                        Rejection::BadValue => "not an allowed value",
                        Rejection::BadType => "wrong value type",
                    }
                ),
                Outcome::Unresolved { .. } => "Value parsing is not modeled yet.".into(),
            };
            out.push_str(&format!("  {text}\n"));
        }
    }
    if batch.candidates.len() > 32 {
        out.push_str("… More fields are retained in the analysis.\n");
    }
    for finding in batch.findings.iter().take(16) {
        let message = match finding.kind {
            FindingKind::InvalidArgument => "Settings argument must be a table",
            FindingKind::IgnoredNonstringKey => "Non-text key ignored",
            FindingKind::UnknownKey => "Not in selected core catalogue",
            FindingKind::AmbiguousTarget => "Conflicting keys",
            FindingKind::UnresolvedCycle => "Cyclic/shared table needs further analysis",
        };
        out.push_str(message);
        if !finding.api_key.is_empty() {
            out.push_str(": ");
            out.push_str(&short(&finding.api_key));
        }
        out.push('\n');
    }
    if batch.findings.len() > 16 {
        out.push_str("… More findings are retained in the analysis.\n");
    }
    if !batch.traversal_resolved {
        out.push_str("Table traversal is incomplete.\n");
    }
    if batch.candidates.iter().any(|c| c.ambiguous) {
        out.push_str("Several keys address the same setting; no winning value is inferred.\n");
    }

    out
}
pub(super) fn value(out: &mut String, input: &Observed, depth: usize) {
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

pub(super) fn show_saved(
    parent: &impl IsA<gtk::Window>,
    state: &super::Shared,
) -> Result<adw::Window, String> {
    let (saved, generation) = {
        let s = state.borrow();
        (
            s.source_graphs
                .get(&s.draft.version)
                .cloned()
                .ok_or("No saved source graph in this draft")?,
            s.generation,
        )
    };
    // Repeated inspection must not create another retained model or worker.
    let prefix = format!("saved-source-graph-{}-", saved.version);
    let tag = format!("{prefix}{}-{generation}", saved.fingerprint);
    for window in gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<adw::Window>().ok())
    {
        if window.is_visible() && window.widget_name().starts_with(&prefix) {
            if window.widget_name() == tag {
                window.present();
                return Ok(window);
            }
            window.close();
        }
    }
    let window = adw::Window::builder()
        .title("Draft source files")
        .transient_for(parent)
        .default_width(700)
        .default_height(660)
        .build();
    window.set_size_request(350, 420);
    window.set_widget_name(&tag);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let model: Model = Rc::new(RefCell::new(None));
    let changed = window.downgrade();
    let changed_state = Rc::downgrade(state);
    let (body, view) = build(
        &model,
        state,
        Some(Rc::new(move || {
            if let (Some(window), Some(state)) = (changed.upgrade(), changed_state.upgrade()) {
                let s = state.borrow();
                if let Some(graph) = s.source_graphs.get(&s.draft.version) {
                    window.set_widget_name(&format!(
                        "saved-source-graph-{}-{}-{}",
                        graph.version, graph.fingerprint, s.generation
                    ));
                }
            }
        })),
    );
    for set in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        set(body.upcast_ref(), 16);
    }
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&body)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    window.set_content(Some(&root));
    view.reset("Rebuilding analysis from saved source bytes…");
    let close_model = model.clone();
    window.connect_close_request(move |_| {
        close_model.borrow_mut().take();
        gtk::glib::Propagation::Proceed
    });
    let weak = window.downgrade();
    let shared = Rc::downgrade(state);
    let expected = saved.fingerprint.clone();
    let version = saved.version.clone();
    let weak_model = Rc::downgrade(&model);
    super::workflow::background(
        move || super::source_graph::restore(&saved.bundle, &saved.version),
        move |result| {
            let (Some(window), Some(state), Some(model)) =
                (weak.upgrade(), shared.upgrade(), weak_model.upgrade())
            else {
                return;
            };
            if !window.is_visible() {
                return;
            }
            let s = state.borrow();
            if s.generation != generation
                || s.source_graphs
                    .get(&version)
                    .is_none_or(|g| g.fingerprint != expected)
            {
                view.reset("Draft changed. Reopen the saved source view.");
                return;
            }
            drop(s);
            match result {
                Ok(analysis) => {
                    *model.borrow_mut() = Some(Arc::new(analysis));
                    view.render();
                }
                Err(e) => view.reset(&display_path_bytes(e.as_bytes())),
            }
        },
    );
    window.present();
    Ok(window)
}
