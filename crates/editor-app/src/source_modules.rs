//! Read-only inspection of loader sites in an immutable source snapshot.
use super::{label, monitors::compact, notice, worker, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::{DependencyBinding, DependencyInventory, DependencyLoader, DependencyScope, LuaDocument},
    source::display_path_bytes,
};
use std::{rc::Rc, sync::Arc};

fn text_area(text: &str, code: bool, height: i32) -> gtk::ScrolledWindow {
    let view = gtk::TextView::builder()
        .editable(false)
        .monospace(code)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(8)
        .right_margin(8)
        .top_margin(8)
        .bottom_margin(8)
        .build();
    view.buffer().set_text(text);
    gtk::ScrolledWindow::builder()
        .child(&view)
        .min_content_height(height)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build()
}
fn scope(scope: DependencyScope) -> &'static str {
    match scope {
        DependencyScope::Linear => "Top-level or plain block",
        DependencyScope::Conditional => "Conditional or loop",
        DependencyScope::FunctionBody => "Inside a function; invocation timing is unverified",
    }
}
fn binding(binding: DependencyBinding) -> &'static str {
    match binding {
        DependencyBinding::EntryCandidate => {
            "The name matches an initial loader. This does not prove the file is loaded."
        }
        DependencyBinding::CapturedCandidate => {
            "A local alias captures a loader name. Its execution is unverified."
        }
        DependencyBinding::Shadowed => "This loader name is shadowed by another local value.",
        DependencyBinding::Uncertain => "The loader or its surrounding state is unresolved.",
    }
}
fn title(site: &hyprland_editor_core::lua::DependencySite) -> String {
    let kind = match site.loader {
        DependencyLoader::Require => "Module",
        DependencyLoader::OriginalRequire => "Original loader",
        DependencyLoader::Dofile => "Script execution call",
        DependencyLoader::Loadfile => "File compilation call",
    };
    let path = site.module.as_ref().map_or_else(
        || "Unresolved path".into(),
        |m| display_path_bytes(m.as_bytes()),
    );
    let short = path.chars().take(160).collect::<String>();
    format!(
        "Line {} · {kind} · {short}{}",
        site.line,
        if path.chars().count() > 160 {
            "…"
        } else {
            ""
        }
    )
}
fn report(body: &gtk::Box, source: Arc<LuaDocument>, inventory: DependencyInventory) {
    body.append(&label(&format!(
        "{} loader calls · {} other unresolved operations",
        inventory.sites.len(),
        inventory.barriers.len()
    )));
    let titles = inventory.sites.iter().map(title).collect::<Vec<_>>();
    let pick = gtk::DropDown::from_strings(&titles.iter().map(String::as_str).collect::<Vec<_>>());
    compact(&pick);
    pick.set_enable_search(true);
    pick.set_widget_name("source-module-site");
    body.append(&pick);
    let detail = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.append(&detail);
    let inventory = Rc::new(inventory);
    let render: Rc<dyn Fn(u32)> = Rc::new({
        let detail = detail.clone();
        let inventory = inventory.clone();
        move |index| {
            while let Some(child) = detail.first_child() {
                detail.remove(&child);
            }
            let Some(site) = inventory.sites.get(index as usize) else {
                detail.append(&label("No recognized loader sites were found. Unresolved calls may still hide dependencies."));
                return;
            };
            detail.append(&label(scope(site.scope)));
            let status = label(binding(site.binding));
            status.set_widget_name("source-module-binding");
            detail.append(&status);
            detail.append(&label("Module or file argument"));
            let path = site.module.as_ref().map_or_else(
                || "Computed or unresolved value".into(),
                |m| display_path_bytes(m.as_bytes()),
            );
            let area = text_area(&path, false, 70);
            area.set_widget_name("source-module-argument");
            detail.append(&area);
            detail.append(
                &gtk::Expander::builder()
                    .label("Source expression")
                    .child(&text_area(
                        &display_path_bytes(source.source[site.span.clone()].as_bytes()),
                        true,
                        120,
                    ))
                    .build(),
            );
        }
    });
    let r = render.clone();
    pick.connect_selected_notify(move |pick| r(pick.selected()));
    render(pick.selected());
    let barriers = inventory
        .barriers
        .iter()
        .map(|barrier| {
            let reason = match barrier.kind {
                hyprland_editor_core::lua::DependencyBarrierKind::UnknownCall => {
                    "Call may change loading state"
                }
                hyprland_editor_core::lua::DependencyBarrierKind::ControlFlow => {
                    "Execution order needs further analysis"
                }
                hyprland_editor_core::lua::DependencyBarrierKind::EnvironmentMutation => {
                    "Loader environment may change"
                }
                hyprland_editor_core::lua::DependencyBarrierKind::NonlocalAssignment => {
                    "Global or shared value may change"
                }
            };
            format!("Line {} · {reason}", barrier.line)
        })
        .collect::<Vec<_>>()
        .join("\n");
    body.append(&gtk::Expander::builder().label("Unresolved effects").child(&text_area(if barriers.is_empty(){"None recorded by this inspection. This is not proof that the dependency graph is complete."}else{&barriers},false,120)).build());
}
fn show(parent: &adw::ApplicationWindow, state: &Shared) -> Result<adw::Window, String> {
    let (source, version, origin) = {
        let s = state.borrow();
        let version = s.draft.version.clone();
        let source = s
            .sources
            .get(&version)
            .cloned()
            .ok_or("Import Lua source before inspecting its module requests")?;
        (
            source,
            version.clone(),
            s.source_origins.get(&version).cloned(),
        )
    };
    let win = adw::Window::builder()
        .title("Source modules")
        .transient_for(parent)
        .default_width(700)
        .default_height(660)
        .build();
    win.set_size_request(350, 420);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for setter in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        setter(body.upcast_ref(), 16);
    }
    if state.borrow().source_graphs.contains_key(&version) {
        let saved = gtk::Button::with_label("View Retained Source Files");
        saved.set_widget_name("view-saved-source-graph");
        body.append(&saved);
        let shared = Rc::downgrade(state);
        let parent = win.downgrade();
        saved.connect_clicked(move |_| {
            if let (Some(state), Some(parent)) = (shared.upgrade(), parent.upgrade()) {
                if let Err(error) = super::source_analysis::show_saved(&parent, &state) {
                    let dialog = adw::MessageDialog::new(
                        Some(&parent),
                        Some("Cannot inspect saved graph"),
                        Some(&error),
                    );
                    dialog.add_response("close", "Close");
                    dialog.present();
                }
            }
        });
    }
    body.append(&label("Inspect module and file-loader calls in the imported snapshot. Referenced files are not opened by this view. Retained files can be reopened from a private workspace; source graph editing is still in development."));
    let location = origin.as_ref().map_or_else(
        || "Location unavailable".into(),
        |o| display_path_bytes(&o.selected_path),
    );
    body.append(&gtk::Expander::builder().label("Snapshot details").child(&text_area(&format!("Imported location: {location}\nSnapshot: {version}\nSHA-256: {}\nThis view uses saved source bytes; recorded locations are not reopened.",source.report.source_sha256),false,100)).build());
    let feedback = label("Inspecting source…");
    feedback.set_widget_name("source-module-feedback");
    body.append(&feedback);
    let results = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&results);
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&body)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    win.set_content(Some(&root));
    win.present();
    let source_copy = source.source.clone();
    let version_copy = version.clone();
    let expected_hash = source.report.source_sha256.clone();
    let state = Rc::downgrade(state);
    let weak = win.downgrade();
    let feedback = feedback.downgrade();
    let results = results.downgrade();
    workflow::background(
        move || {
            let worker::Response::Dependencies(inventory) =
                worker::run(worker::Request::InspectDependencies {
                    source: source_copy.clone(),
                    version: version_copy.clone(),
                })?
            else {
                return Err("Unexpected module-inspection response".into());
            };
            inventory.validate(&source_copy, &version_copy)?;
            Ok(inventory)
        },
        move |result| {
            let (Some(win), Some(feedback), Some(results), Some(state)) = (
                weak.upgrade(),
                feedback.upgrade(),
                results.upgrade(),
                state.upgrade(),
            ) else {
                return;
            };
            let s = state.borrow();
            if s.draft.version != version
                || s.sources
                    .get(&version)
                    .is_none_or(|doc| doc.report.source_sha256 != expected_hash)
            {
                feedback
                    .set_text("The imported snapshot changed. Open this view again to inspect it.");
                return;
            }
            drop(s);
            match result {
                Ok(inventory) => {
                    feedback.set_text("Snapshot inspection complete. These are possible dependencies, not an effective file graph.");
                    let access = gtk::Button::with_label("Review Dependency Files…");
                    access.set_widget_name("source-module-open-files");
                    super::source_access::connect(
                        &access,
                        &win,
                        &state,
                        version.clone(),
                        expected_hash.clone(),
                    );
                    results.append(&access);
                    if state.borrow().source_graphs.contains_key(&version) {
                        let incoming = gtk::Button::with_label("Merge Incoming Configuration…");
                        incoming.set_widget_name("source-module-incoming");
                        super::source_access::connect_incoming(
                            &incoming,
                            &win,
                            &state,
                            version.clone(),
                            expected_hash.clone(),
                        );
                        results.append(&incoming);
                    }
                    report(&results, source, inventory);
                }
                Err(error) => feedback.set_text(&error),
            }
        },
    );
    Ok(win)
}
pub(super) fn connect(button: &gtk::Button, parent: &adw::ApplicationWindow, state: &Shared) {
    let parent = parent.downgrade();
    let state = state.clone();
    button.connect_clicked(move |_| {
        if let Some(parent) = parent.upgrade() {
            if let Err(error) = show(&parent, &state) {
                notice(&parent, "Source modules unavailable", &error);
            }
        }
    });
}
#[cfg(test)]
pub(super) fn test_view(parent: &adw::ApplicationWindow) {
    fn descendants(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut out = vec![];
        let mut child = w.first_child();
        while let Some(w) = child {
            out.push(w.clone());
            out.extend(descendants(&w));
            child = w.next_sibling();
        }
        out
    }
    fn widget<T: IsA<gtk::Widget> + gtk::glib::object::ObjectType>(
        win: &adw::Window,
        name: &str,
    ) -> T {
        descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap_or_else(|| panic!("Missing source module widget {name}"))
            .downcast()
            .unwrap()
    }
    let state = super::initial_state();
    let source =
        "local r=require;r('./theme');local f=function()loadfile('/unopened/private.lua')end";
    let c = state.borrow().catalogue.clone();
    let parsed =
        hyprland_editor_core::lua::parse(source, hyprland_editor_core::BASELINE, &c).unwrap();
    state.borrow_mut().sources.insert(
        hyprland_editor_core::BASELINE.into(),
        Arc::new(LuaDocument::from_report(source.into(), parsed, &c).unwrap()),
    );
    let generation = state.borrow().generation;
    let win = show(parent, &state).unwrap();
    let context = gtk::glib::MainContext::default();
    let start = std::time::Instant::now();
    while !widget::<gtk::Label>(&win, "source-module-feedback")
        .text()
        .starts_with("Snapshot inspection complete")
    {
        while context.pending() {
            context.iteration(false);
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "Source inspection did not complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    widget::<gtk::DropDown>(&win, "source-module-site").set_selected(1);
    assert!(widget::<gtk::Label>(&win, "source-module-binding")
        .text()
        .contains("unresolved"));
    assert_eq!(
        state.borrow().generation,
        generation,
        "Read-only inspection must not edit drafts"
    );
    super::source_access::test_review(&win, &state);
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        for _ in 0..10 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
        win.renderer()
            .unwrap()
            .render_texture(snapshot.to_node().unwrap(), None)
            .save_to_png(format!("{path}.source-modules.png"))
            .unwrap();
    }
    let (minimum, _, _, _) = win.measure(gtk::Orientation::Horizontal, -1);
    assert!(
        minimum <= 350,
        "Module inspector requires {minimum}px at minimum"
    );
    win.close();
    drop(win);
    let stale = show(parent, &state).unwrap();
    state.borrow_mut().draft.version = hyprland_editor_core::AUDITED_MAIN.into();
    let start = std::time::Instant::now();
    while !widget::<gtk::Label>(&stale, "source-module-feedback")
        .text()
        .starts_with("The imported snapshot changed")
    {
        while context.pending() {
            context.iteration(false);
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "Stale inspection did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!descendants(stale.upcast_ref())
        .iter()
        .any(|w| w.widget_name() == "source-module-site"));
    stale.close();
    drop(stale);
    state.borrow_mut().draft.version = hyprland_editor_core::BASELINE.into();
    let win = show(parent, &state).unwrap();
    let weak = Rc::downgrade(&state);
    let source_weak = Arc::downgrade(&state.borrow().sources[hyprland_editor_core::BASELINE]);
    win.close();
    drop(win);
    drop(state);
    while context.pending() {
        context.iteration(false);
    }
    assert!(
        weak.upgrade().is_none(),
        "Closed module inspector retained draft state"
    );
    let start = std::time::Instant::now();
    while source_weak.upgrade().is_some() {
        while context.pending() {
            context.iteration(false);
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "Closed inspector retained source after worker completion"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
