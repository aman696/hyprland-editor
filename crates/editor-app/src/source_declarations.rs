//! Lazy declaration choices feeding the shared source comparison/keep lifecycle.
use super::{label, source_analysis::Model, source_changes, worker, workflow};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::replay::monitor_edit::declarations::{Anchor, Change, Inventory},
    monitor::{Output, Rule},
    source::display_path_bytes,
};
use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};

pub(super) fn open(
    parent: &gtk::Window,
    model: &Model,
    document: String,
    preferred: Option<Range<usize>>,
    state: &super::Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    let analysis = model
        .borrow()
        .clone()
        .expect("Selected captured source analysis");
    let weak = Arc::downgrade(&analysis);
    let state_weak = Rc::downgrade(state);
    let version = analysis.program.version.clone();
    let file_name = analysis
        .program
        .documents
        .iter()
        .find(|d| d.id == document)
        .map(|d| {
            display_path_bytes(
                d.load_name
                    .rsplit(|b| *b == b'/')
                    .next()
                    .unwrap_or(&d.load_name),
            )
        })
        .unwrap_or_else(|| document.clone());
    let names = {
        let state = state.borrow();
        let mut names = vec!["New empty declaration".into()];
        names.extend(state.monitors.get(&version).into_iter().flatten().map(|r| {
            let text = r.output.text();
            let bytes = text.as_bytes();
            format!(
                "Draft · {} · {} fields",
                if text.is_empty() {
                    "Fallback".into()
                } else {
                    display_path_bytes(&bytes[..bytes.len().min(96)])
                },
                r.fields.len()
            )
        }));
        names
    };
    source_changes::open_custom(
        parent,
        model,
        state,
        refresh,
        Box::new(move |body, changed, publish| {
            body.append(&label("Choose where to add a monitor declaration, or remove one from this captured file. End placement keeps a final return last. Monitor drafts remain separate."));
            body.append(&label(&format!("Captured file: {file_name}")));
            let mode = gtk::DropDown::from_strings(&["Add declaration", "Remove declaration"]);
            mode.set_widget_name("source-declaration-operation");
            mode.update_property(&[gtk::accessible::Property::Label("Declaration operation")]);
            super::monitors::compact(&mode);
            body.append(&mode);
            let templates_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
            body.append(&templates_box);
            templates_box.append(&label("Starting declaration"));
            let template =
                gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
            template.set_widget_name("source-declaration-template");
            template.update_property(&[gtk::accessible::Property::Label(
                "Starting monitor declaration",
            )]);
            super::monitors::compact(&template);
            templates_box.append(&template);
            let fields=label("No optional fields. Add fields using the source inspector after keeping this declaration.");
            templates_box.append(&fields);
            templates_box.append(&label("Output match"));
            let output =
                gtk::DropDown::from_strings(&["Fallback", "Connector name", "Description"]);
            output.set_widget_name("source-declaration-output");
            output.update_property(&[gtk::accessible::Property::Label("Monitor output matching")]);
            super::monitors::compact(&output);
            templates_box.append(&output);
            let name = gtk::Entry::builder()
                .max_length(4096)
                .placeholder_text("Connector name or description")
                .sensitive(false)
                .build();
            name.set_widget_name("source-declaration-name");
            name.update_property(&[gtk::accessible::Property::Label(
                "Output name or description",
            )]);
            templates_box.append(&name);
            let selected_rule = Rc::new(RefCell::new(Rule::new("source-declaration")));
            let selected = selected_rule.clone();
            let state = state_weak.clone();
            let template_version = version.clone();
            let out = output.downgrade();
            let entry = name.downgrade();
            let summary = fields.downgrade();
            let changed_template = changed.clone();
            template.connect_selected_notify(move |pick| {
                let rule = if pick.selected() == 0 {
                    Some(Rule::new("source-declaration"))
                } else {
                    state.upgrade().and_then(|s| {
                        s.borrow().monitors.get(&template_version)
                            .and_then(|rules| rules.get(pick.selected() as usize - 1)).cloned()
                    })
                };
                if let Some(rule) = rule {
                    if let Some(output) = out.upgrade() {
                        output.set_selected(match &rule.output {
                            Output::Fallback => 0,
                            Output::Connector(_) => 1,
                            Output::Description(_) => 2,
                        });
                    }
                    if let Some(entry) = entry.upgrade() {
                        entry.set_text(match &rule.output {
                            Output::Fallback => "",
                            Output::Connector(v) | Output::Description(v) => v,
                        });
                    }
                    if let Some(summary) = summary.upgrade() {
                        summary.set_text(&format!("{} optional fields copied when selected. Change fields later using the source inspector.", rule.fields.len()));
                    }
                    *selected.borrow_mut() = rule;
                }
                changed_template();
            });
            let entry = name.downgrade();
            let changed_output = changed.clone();
            output.connect_selected_notify(move |pick| {
                if let Some(entry) = entry.upgrade() {
                    entry.set_sensitive(pick.selected() != 0);
                }
                changed_output();
            });
            let changed_name = changed.clone();
            name.connect_changed(move |_| changed_name());
            body.append(&label("Source position or declaration"));
            let positions = gtk::DropDown::from_strings(&[]);
            super::monitors::compact(&positions);
            positions.set_enable_search(true);
            positions.set_widget_name("source-declaration-position");
            positions.update_property(&[gtk::accessible::Property::Label(
                "Source statement or insertion position",
            )]);
            body.append(&positions);
            let changed_position = changed.clone();
            positions.connect_selected_notify(move |_| changed_position());
            let inventory: Rc<RefCell<Option<Inventory>>> = Rc::new(RefCell::new(None));
            let anchors: Rc<RefCell<Vec<Anchor>>> = Rc::new(RefCell::new(vec![]));
            let inv = inventory.clone();
            let choices = anchors.clone();
            let mode_weak = mode.downgrade();
            let pos = positions.downgrade();
            let group = templates_box.downgrade();
            let preferred = preferred.clone();
            let requested_remove = preferred.is_some();
            let changed_mode = changed.clone();
            let rebuild: Rc<dyn Fn()> = Rc::new(move || {
                let (Some(mode), Some(positions), Some(group)) =
                    (mode_weak.upgrade(), pos.upgrade(), group.upgrade())
                else {
                    return;
                };
                let inv = inv.borrow();
                let Some(inv) = inv.as_ref() else {
                    return;
                };
                let removing = mode.selected() == 1;
                group.set_visible(!removing);
                let mut names = vec![];
                let mut next = vec![];
                if removing {
                    names.extend(inv.declarations.iter().map(|s| {
                        format!(
                            "Line {} · {} · {}",
                            s.line,
                            s.output_hint.as_ref().map_or_else(
                                || if s.standalone {
                                    "Declaration".into()
                                } else {
                                    "Embedded call".into()
                                },
                                |v| if v.is_empty() {
                                    "Fallback source selector".into()
                                } else {
                                    display_path_bytes(v)
                                }
                            ),
                            if s.observed == 0 {
                                "not observed".into()
                            } else {
                                format!("{} observed calls", s.observed)
                            }
                        )
                    }));
                } else {
                    names.extend([
                        "Beginning of file".to_string(),
                        "End of file (before final return)".into(),
                    ]);
                    next.extend([Anchor::Beginning, Anchor::End]);
                    for site in &inv.statements {
                        let kind = site.kind.replace('_', " ");
                        names.push(format!("Before line {} · {kind}", site.line));
                        next.push(Anchor::Before {
                            span: site.span.clone(),
                        });
                        if site.kind != "return_statement" {
                            names.push(format!("After line {} · {kind}", site.line));
                            next.push(Anchor::After {
                                span: site.span.clone(),
                            });
                        }
                    }
                }
                *choices.borrow_mut() = next;
                positions.set_model(Some(&gtk::StringList::new(
                    &names.iter().map(String::as_str).collect::<Vec<_>>(),
                )));
                positions.set_selected(if removing {
                    preferred
                        .as_ref()
                        .and_then(|p| inv.declarations.iter().position(|s| s.span == *p))
                        .unwrap_or(0) as u32
                } else {
                    1
                });
                changed_mode();
            });
            let update = rebuild.clone();
            mode.connect_selected_notify(move |_| update());
            publish(None, "Collecting source declaration sites…");
            let Some(analysis) = weak.upgrade() else {
                publish(None, "The source analysis changed.");
                return;
            };
            let program = analysis.program.clone();
            let expected_program =
                hyprland_editor_core::workspace_graph::program_fingerprint(&program);
            let expected_doc = program
                .documents
                .iter()
                .find(|d| d.id == document)
                .map(|d| d.sha256.clone());
            let compiler = analysis.report.compiler_profile.clone();
            drop(analysis);
            let permit = match source_changes::Permit::acquire() {
                Ok(p) => p,
                Err(e) => {
                    publish(None, &e);
                    return;
                }
            };
            let requested_doc = document.clone();
            let mode_weak = mode.downgrade();
            workflow::background(
                move || {
                    let _permit = permit;
                    match worker::run(worker::Request::InspectMonitorDeclarations {
                        program,
                        document: requested_doc,
                    })? {
                        worker::Response::MonitorDeclarationSites(inventory) => Ok(inventory),
                        _ => Err("Unexpected declaration inventory response".into()),
                    }
                },
                move |result| {
                    let result = result.and_then(|result| {
                        if result.runtime_verified
                            || result.version != version
                            || result.compiler != compiler
                            || expected_program.as_ref().ok() != Some(&result.program_sha256)
                            || expected_doc.as_ref() != Some(&result.document_sha256)
                            || result.document != document
                            || result.statements.len() + result.declarations.len() > 4096
                        {
                            return Err(
                                "Declaration inventory differs from this captured source/model"
                                    .into(),
                            );
                        }
                        Ok(result)
                    });
                    let result = match result {
                        Ok(result) => result,
                        Err(e) => {
                            publish(
                                None,
                                &display_path_bytes(&e.as_bytes()[..e.len().min(1024)]),
                            );
                            return;
                        }
                    };
                    let Some(current_mode) = mode_weak.upgrade() else {
                        return;
                    };
                    let has_declarations = !result.declarations.is_empty();
                    *inventory.borrow_mut() = Some(result);
                    current_mode.set_model(Some(&gtk::StringList::new(if has_declarations {
                        &["Add declaration", "Remove declaration"]
                    } else {
                        &["Add declaration"]
                    })));
                    current_mode.set_selected(u32::from(has_declarations && requested_remove));
                    rebuild();
                    let getter: source_changes::RequestGetter = Rc::new(move || {
                        let analysis = weak.upgrade().ok_or("The source analysis changed")?;
                        let inv = inventory.borrow();
                        let inv = inv.as_ref().ok_or("Declaration sites are not available")?;
                        let change = if mode.selected() == 1 {
                            let site = inv
                                .declarations
                                .get(positions.selected() as usize)
                                .ok_or("Choose a monitor declaration")?;
                            Change::Remove {
                                document: document.clone(),
                                span: site.span.clone(),
                            }
                        } else {
                            let anchor = anchors
                                .borrow()
                                .get(positions.selected() as usize)
                                .cloned()
                                .ok_or("Choose a source position")?;
                            let mut rule = selected_rule.borrow().clone();
                            rule.output = match output.selected() {
                                0 => Output::Fallback,
                                1 => Output::Connector(name.text().into()),
                                _ => Output::Description(name.text().into()),
                            };
                            Change::Insert {
                                document: document.clone(),
                                anchor,
                                rule,
                            }
                        };
                        Ok(worker::Request::CompareMonitorDeclaration {
                            program: analysis.program.clone(),
                            change,
                        })
                    });
                    publish(
                        Some(getter),
                        "Choose a declaration or position, then compare the private source change.",
                    );
                },
            );
        }),
    )
}
#[cfg(test)]
pub(super) fn test_workflow(parent: &adw::ApplicationWindow) {
    use hyprland_editor_core::{
        lua::LuaDocument, module_path::Context, storage, workspace::Workspace, AUDITED_MAIN,
        BASELINE,
    };
    fn named(root: &gtk::Widget, name: &str) -> gtk::Widget {
        fn find(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
            if root.widget_name() == name {
                return Some(root.clone());
            }
            let mut child = root.first_child();
            while let Some(widget) = child {
                if let Some(found) = find(&widget, name) {
                    return Some(found);
                }
                child = widget.next_sibling();
            }
            None
        }
        find(root, name).unwrap_or_else(|| panic!("Missing {name}"))
    }
    #[track_caller]
    fn wait(mut condition: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if condition() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(12),
                "Declaration workflow timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    fn dialog() -> adw::Window {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<adw::Window>().ok())
            .find(|w| w.is_visible() && w.title().as_deref() == Some("Manage Monitor Declarations"))
            .unwrap()
    }
    fn buttons(window: &adw::Window) -> (gtk::Button, gtk::Button) {
        (
            named(window.upcast_ref(), "source-change-compare")
                .downcast()
                .unwrap(),
            named(window.upcast_ref(), "source-change-keep")
                .downcast()
                .unwrap(),
        )
    }
    for version in [BASELINE, AUDITED_MAIN] {
        wait(|| !source_changes::Permit::running());
        let directory = std::env::temp_dir().join(format!(
            "hye-declarations-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let source = "-- retained source notes\n";
        let path = directory.join("main.lua");
        std::fs::write(&path, source).unwrap();
        let captured = Arc::new(storage::read_source(&path).unwrap());
        let analysis = super::source_graph::analyze(
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
        assert!(analysis.report.events.is_empty());
        let model: Model = Rc::new(RefCell::new(Some(Arc::new(analysis))));
        let state = super::initial_state();
        let worker::Response::Parsed(report) = worker::run(worker::Request::Parse {
            source: source.into(),
            version: version.into(),
        })
        .unwrap() else {
            panic!()
        };
        {
            let mut s = state.borrow_mut();
            s.draft = hyprland_editor_core::Draft::new(version);
            let document = LuaDocument::from_report(source.into(), *report, &s.catalogue).unwrap();
            s.sources.insert(version.into(), Arc::new(document));
            s.source_origins
                .insert(version.into(), captured.identity.clone());
        }
        let changes = Rc::new(std::cell::Cell::new(0));
        let done = changes.clone();
        let (body, view) = super::source_analysis::build(
            &model,
            &state,
            Some(Rc::new(move || done.set(done.get() + 1))),
        );
        let viewer = adw::Window::builder()
            .title("Declaration source fixture")
            .transient_for(parent)
            .modal(true)
            .default_width(620)
            .default_height(740)
            .content(&body)
            .build();
        viewer.present();
        view.render();
        let manage = named(viewer.upcast_ref(), "source-analysis-declarations")
            .downcast::<gtk::Button>()
            .unwrap();
        assert!(manage.is_sensitive());
        manage.emit_clicked();
        let window = dialog();
        let (compare, keep) = buttons(&window);
        wait(|| compare.is_sensitive());
        named(window.upcast_ref(), "source-declaration-output")
            .downcast::<gtk::DropDown>()
            .unwrap()
            .set_selected(1);
        let name = named(window.upcast_ref(), "source-declaration-name")
            .downcast::<gtk::Entry>()
            .unwrap();
        name.set_text("DP-1");
        let held = source_changes::hold_comparisons();
        compare.emit_clicked();
        name.set_text("DP-2");
        drop(held);
        wait(|| compare.is_sensitive());
        assert!(
            !keep.is_sensitive(),
            "Changed declaration input kept a stale comparison"
        );
        name.set_text("DP-1");
        compare.emit_clicked();
        wait(|| keep.is_sensitive());
        keep.emit_clicked();
        wait(|| changes.get() == 1);
        assert_eq!(model.borrow().as_ref().unwrap().report.events.len(), 1);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        // Template selection copies fields without changing the separate draft.
        let mut rule = Rule::new("template");
        rule.output = Output::Connector("DP-2".into());
        rule.fields
            .insert("transform".into(), hyprland_editor_core::Value::Int(3));
        state
            .borrow_mut()
            .monitors
            .insert(version.into(), vec![rule.clone()]);
        manage.emit_clicked();
        let window = dialog();
        let (compare, keep) = buttons(&window);
        wait(|| compare.is_sensitive());
        named(window.upcast_ref(), "source-declaration-template")
            .downcast::<gtk::DropDown>()
            .unwrap()
            .set_selected(1);
        compare.emit_clicked();
        wait(|| keep.is_sensitive());
        keep.emit_clicked();
        wait(|| changes.get() == 2);
        assert_eq!(model.borrow().as_ref().unwrap().report.events.len(), 2);
        assert_eq!(state.borrow().monitors[version], [rule]);
        manage.emit_clicked();
        let window = dialog();
        let (compare, keep) = buttons(&window);
        wait(|| compare.is_sensitive());
        named(window.upcast_ref(), "source-declaration-operation")
            .downcast::<gtk::DropDown>()
            .unwrap()
            .set_selected(1);
        named(window.upcast_ref(), "source-declaration-position")
            .downcast::<gtk::DropDown>()
            .unwrap()
            .set_selected(1);
        compare.emit_clicked();
        wait(|| keep.is_sensitive());
        window.set_default_size(360, 740);
        wait(|| (350..=360).contains(&window.width()));
        let bounds = keep.compute_bounds(&window).unwrap();
        assert!(
            bounds.x() >= 0.
                && bounds.x() + bounds.width() <= window.width() as f32
                && bounds.y() + bounds.height() <= window.height() as f32
        );
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
                .save_to_png(format!("{path}.monitor-declarations.png"))
                .unwrap();
        }
        keep.emit_clicked();
        wait(|| changes.get() == 3);
        assert_eq!(model.borrow().as_ref().unwrap().report.events.len(), 1);
        let bundle = state.borrow().source_graphs[version].bundle.clone();
        let data = Workspace::new(state.borrow().draft.clone())
            .with_graph(&bundle)
            .to_json(&state.borrow().catalogue)
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        std::fs::remove_dir_all(&directory).unwrap();
        let saved = Workspace::from_json(
            std::str::from_utf8(&data).unwrap(),
            &state.borrow().catalogue,
        )
        .unwrap();
        let bundle = hyprland_editor_core::workspace_graph::Bundle {
            graph: saved.source_graph.unwrap(),
            sources: Arc::new(saved.sources),
        };
        let restored = super::source_graph::restore(&bundle, version).unwrap();
        assert_eq!(restored.report.events.len(), 1);
        assert_eq!(bundle.sources[0].content, source);
        for _ in 0..3 {
            state.borrow_mut().undo_last();
        }
        assert!(!state.borrow().source_graphs.contains_key(version));
        viewer.close();
    }
}
