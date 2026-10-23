//! In-memory graph/root transactions. All filesystem fixtures are disposable;
//! transaction and undo run after the original paths have been removed.
use super::*;
use hyprland_editor_core::{
    lua::{
        self,
        replay::{self, *},
        LuaDocument,
    },
    module_path::Context,
    storage,
    workspace::{SourceLanguage, SourceSnapshot, Workspace},
    workspace_graph::{self, Bundle, Graph, View},
};
use std::path::Path;

struct Fixture {
    graph: Arc<source_graph::Saved>,
    document: Arc<LuaDocument>,
    origin: SourceIdentity,
}
fn fixture(folder: &Path, name: &str, version: &str, value: i32) -> Fixture {
    let main = folder.join(format!("{name}.lua"));
    let part = folder.join(format!("{name}-part.lua"));
    let content =
        format!("local part=require('part');hl.config{{general={{border_size=part}}}} -- {name}\n");
    std::fs::write(&main, &content).unwrap();
    std::fs::write(&part, format!("return {value}\n")).unwrap();
    let captured = storage::read_source(&main).unwrap();
    let dependency = storage::read_source(&part).unwrap();
    let context = Context {
        main_file: captured.identity.selected_path.clone(),
        home: None,
        inherited_package_path: None,
    };
    let (path, unknown) = context.initial_path(version).unwrap();
    let mut program = Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: name.into(),
        documents: vec![Document {
            id: name.into(),
            source: content.clone(),
            sha256: lua::source_hash(&content),
            load_name: context.main_file.clone(),
        }],
        bindings: vec![],
        initial_package_path: Some(path),
        initial_package_path_unknown_tail: unknown,
        working_directory: None,
    };
    let report = replay::replay(&program).unwrap();
    let Some(Halt::NeedsResolution { request }) = report.halt else {
        panic!("Reached module input")
    };
    let id = format!("{name}-part");
    program.documents.push(Document {
        id: id.clone(),
        source: dependency.content.clone(),
        sha256: dependency.identity.sha256.clone(),
        load_name: dependency.identity.selected_path.clone(),
    });
    program.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: id.clone(),
            loader_data: dependency.identity.selected_path.clone(),
        },
    });
    let report = replay::replay(&program).unwrap();
    assert!(report.halt.is_none());
    let graph = Graph {
        private: Default::default(),
        revisions: vec![],
        format: "hye-workspace-graph-1".into(),
        schema_sha256: workspace_graph::schema_sha256(),
        compiler: report.compiler_profile,
        root: name.into(),
        context,
        working_directory: None,
        views: program
            .documents
            .iter()
            .map(|d| View {
                document: d.id.clone(),
                load_name: d.load_name.clone(),
            })
            .collect(),
        bindings: program.bindings,
    };
    let bundle = Bundle {
        graph,
        sources: Arc::new(vec![
            SourceSnapshot {
                id: name.into(),
                language: SourceLanguage::Lua,
                content: captured.content,
                origin: captured.identity.clone(),
            },
            SourceSnapshot {
                id,
                language: SourceLanguage::Lua,
                content: dependency.content,
                origin: dependency.identity,
            },
        ]),
    };
    Fixture {
        graph: Arc::new(source_graph::Saved::new(bundle, version).unwrap()),
        document: Arc::new(
            LuaDocument::from_report(
                content.clone(),
                lua::parse(&content, version, &catalogue()).unwrap(),
                &catalogue(),
            )
            .unwrap(),
        ),
        origin: captured.identity,
    }
}
fn attach(state: &mut State, f: &Fixture, version: &str) {
    state.draft = Draft::new(version);
    state.sources.insert(version.into(), f.document.clone());
    state
        .source_origins
        .insert(version.into(), f.origin.clone());
    state.source_graphs.insert(version.into(), f.graph.clone());
    state
        .saved_source_graphs
        .insert(version.into(), Some(f.graph.fingerprint.clone()));
    state
        .saved_source_origins
        .insert(version.into(), Some(f.origin.clone()));
}
#[test]
fn root_origin_graph_and_override_removal_commit_and_undo_as_one_operation() {
    for version in [BASELINE, AUDITED_MAIN] {
        let temp = test_support::TempDir::new("he-root-transaction");
        let before = fixture(temp.path(), "old-root", version, 3);
        let after = fixture(temp.path(), "new-root", version, 5);
        let state = initial_state();
        let mut s = state.borrow_mut();
        attach(&mut s, &before, version);
        s.draft
            .values
            .insert("general:border_size".into(), Value::Int(8));
        let saved_values = s.draft.values.clone();
        s.saved.insert(version.into(), saved_values);
        let monitor = hyprland_editor_core::monitor::Rule::new("separate-monitor");
        s.monitors.insert(version.into(), vec![monitor.clone()]);
        s.saved_monitors
            .insert(version.into(), vec![monitor.clone()]);
        let device = hyprland_editor_core::device::Spec::new("separate-device", "keyboard");
        s.devices.insert(version.into(), vec![device.clone()]);
        s.saved_devices.insert(version.into(), vec![device.clone()]);
        let binding = hyprland_editor_core::binding::Draft::default();
        s.bindings.insert(version.into(), binding.clone());
        assert!(!s.dirty_version(version));
        std::fs::remove_dir_all(temp.path()).unwrap();
        let generation = s.generation;
        // An identical graph/root refresh must not consume history or invalidate
        // other reviews.
        let same_draft = s.draft.clone();
        s.keep_graph_revision(
            before.graph.clone(),
            same_draft,
            before.document.clone(),
            before.origin.clone(),
            generation,
        )
        .unwrap();
        assert_eq!(s.generation, generation);
        s.keep_graph_revision(
            after.graph.clone(),
            Draft::new(version),
            after.document.clone(),
            after.origin.clone(),
            generation,
        )
        .unwrap();
        assert_eq!(s.generation, generation + 1);
        assert!(s.dirty_version(version));
        assert!(Arc::ptr_eq(&s.sources[version], &after.document));
        assert_eq!(s.source_origins[version], after.origin);
        assert_eq!(
            s.source_graphs[version].fingerprint,
            after.graph.fingerprint
        );
        assert!(s.draft.values.is_empty());
        assert_eq!(s.monitors[version], vec![monitor.clone()]);
        assert_eq!(s.devices[version], vec![device.clone()]);
        assert_eq!(s.bindings[version], binding);
        let saved = Workspace::new(s.draft.clone()).with_graph(&s.source_graphs[version].bundle);
        let bytes = saved.to_json(&s.catalogue).unwrap();
        let reopened =
            Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &s.catalogue).unwrap();
        assert_eq!(reopened.root().unwrap().origin, after.origin);
        assert_eq!(reopened.source_graph.unwrap().root, "new-root");
        s.undo_last();
        assert_eq!(s.generation, generation + 2);
        assert!(Arc::ptr_eq(&s.sources[version], &before.document));
        assert_eq!(s.source_origins[version], before.origin);
        assert_eq!(
            s.source_graphs[version].fingerprint,
            before.graph.fingerprint
        );
        assert_eq!(s.draft.values["general:border_size"], Value::Int(8));
        assert_eq!(s.monitors[version], vec![monitor]);
        assert_eq!(s.devices[version], vec![device]);
        assert!(s.undo.is_empty());
        assert!(!s.dirty_version(version));
    }
}
#[test]
fn graph_transaction_stale_inputs_mismatches_and_budget_failures_leave_state_and_history_intact() {
    for version in [BASELINE, AUDITED_MAIN] {
        let temp = test_support::TempDir::new("he-root-transaction-refusal");
        let mut before = fixture(temp.path(), "before", version, 3);
        let after = fixture(temp.path(), "after", version, 5);
        Arc::get_mut(&mut before.document)
            .unwrap()
            .source
            .reserve(9 * 1024 * 1024);
        let state = initial_state();
        let mut s = state.borrow_mut();
        attach(&mut s, &before, version);
        s.undo.field("prior", Some(Value::Int(7))).unwrap();
        let generation = s.generation;
        for (doc, origin, expected, message) in [
            (
                after.document.clone(),
                after.origin.clone(),
                generation + 1,
                "draft changed",
            ),
            (
                after.document.clone(),
                before.origin.clone(),
                generation,
                "identity",
            ),
            (
                before.document.clone(),
                before.origin.clone(),
                generation,
                "same captured bytes",
            ),
            (
                after.document.clone(),
                after.origin.clone(),
                generation,
                "undo memory budget",
            ),
        ] {
            let error = s
                .keep_graph_revision(
                    after.graph.clone(),
                    Draft::new(version),
                    doc,
                    origin,
                    expected,
                )
                .unwrap_err();
            assert!(error.contains(message), "{error}");
            assert_eq!(s.generation, generation);
            assert!(Arc::ptr_eq(&s.sources[version], &before.document));
            assert_eq!(s.source_origins[version], before.origin);
            assert_eq!(
                s.source_graphs[version].fingerprint,
                before.graph.fingerprint
            );
        }
        assert!(matches!(s.undo.pop(),Some(history::Undo::Field {id,..}) if id=="prior"));
        assert!(s.undo.is_empty());
    }
}

#[test]
fn first_graph_revision_undo_restores_source_and_graph_absence() {
    for version in [BASELINE, AUDITED_MAIN] {
        let temp = test_support::TempDir::new("he-first-root-transaction");
        let before = fixture(temp.path(), "before", version, 3);
        let after = fixture(temp.path(), "after", version, 5);
        let state = initial_state();
        let mut s = state.borrow_mut();
        attach(&mut s, &before, version);
        s.source_graphs.remove(version);
        let generation = s.generation;
        let mut changed_override = Draft::new(version);
        changed_override
            .values
            .insert("general:border_size".into(), Value::Int(9));
        assert!(s
            .keep_graph_revision(
                after.graph.clone(),
                changed_override,
                after.document.clone(),
                after.origin.clone(),
                generation
            )
            .is_err());
        assert!(s.undo.is_empty());
        assert_eq!(s.generation, generation);
        s.keep_graph_revision(
            after.graph,
            Draft::new(version),
            after.document,
            after.origin,
            generation,
        )
        .unwrap();
        s.undo_last();
        assert!(!s.source_graphs.contains_key(version));
        assert!(Arc::ptr_eq(&s.sources[version], &before.document));
        assert_eq!(s.source_origins[version], before.origin);
        assert!(s.undo.is_empty());
    }
}

#[test]
fn repeated_graph_inverses_share_large_archived_private_capture_allocations() {
    let temp = test_support::TempDir::new("he-private-history");
    let f = fixture(temp.path(), "root", BASELINE, 3);
    let mut bundle = f.graph.bundle.clone();
    let mut sources = vec![];
    for n in 0..3 {
        let path = temp.path().join(format!("archive-{n}.lua"));
        std::fs::write(&path, " ".repeat(1024 * 1024)).unwrap();
        let capture = storage::read_source(&path).unwrap();
        let id = format!("archive-{n}");
        bundle.graph.private.views.push(View {
            document: id.clone(),
            load_name: capture.identity.selected_path.clone(),
        });
        bundle.graph.private.archived.insert(id.clone());
        sources.push(SourceSnapshot {
            id,
            language: SourceLanguage::Lua,
            content: capture.content,
            origin: capture.identity,
        });
    }
    bundle.graph.private.sources = Arc::new(sources);
    let graph = Arc::new(source_graph::Saved::new(bundle, BASELINE).unwrap());
    assert!(graph.private_shared_cost > 6 * 1024 * 1024);
    let mut history = history::History::default();
    for _ in 0..3 {
        history.graph(Some(graph.clone()), vec![], None).unwrap();
    }
    for _ in 0..3 {
        let Some(history::Undo::Graph {
            previous: Some(previous),
            ..
        }) = history.pop()
        else {
            panic!("shared private inputs evicted an inverse")
        };
        assert!(Arc::ptr_eq(&previous, &graph));
    }
    assert!(history.is_empty());
}

pub(super) fn test_private_graph_ui(parent: &adw::ApplicationWindow) {
    use hyprland_editor_core::workspace_graph::{
        private::Inputs,
        revisions::{Buffer, Piece, Revision},
    };
    fn named(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
        if root.widget_name() == name {
            return Some(root.clone());
        }
        let mut child = root.first_child();
        while let Some(w) = child {
            if let Some(v) = named(&w, name) {
                return Some(v);
            }
            child = w.next_sibling();
        }
        None
    }
    fn wait(mut check: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if check() {
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    for version in [BASELINE, AUDITED_MAIN] {
        let temp = test_support::TempDir::new("he-private-graph-ui");
        let old = fixture(temp.path(), "old-root", version, 4);
        let mut bundle = old.graph.bundle.clone();
        let original = bundle
            .graph
            .program(&bundle.sources, Some("old-root"), version)
            .unwrap();
        let doc = &original.documents[0];
        let at = doc.source.find("'part'").unwrap() + 1;
        let content = format!("{}new{}", &doc.source[..at], &doc.source[at + 4..]);
        bundle.graph.revisions = vec![Revision {
            document: doc.id.clone(),
            base_sha256: doc.sha256.clone(),
            sha256: lua::source_hash(&content),
            buffers: vec![Arc::from("new")],
            pieces: vec![
                Piece {
                    buffer: Buffer::Original,
                    span: 0..at,
                },
                Piece {
                    buffer: Buffer::Inserted { index: 0 },
                    span: 0..3,
                },
                Piece {
                    buffer: Buffer::Original,
                    span: at + 4..doc.source.len(),
                },
            ],
        }];
        let path = temp.path().join("new.lua");
        std::fs::write(&path, "return 4").unwrap();
        let capture = storage::read_source(&path).unwrap();
        let source = SourceSnapshot {
            id: "private-new".into(),
            language: SourceLanguage::Lua,
            content: capture.content,
            origin: capture.identity,
        };
        let mut inputs = Inputs {
            sources: Arc::new(vec![source.clone()]),
            views: vec![View {
                document: source.id.clone(),
                load_name: source.origin.selected_path.clone(),
            }],
            ..Inputs::default()
        };
        let pending = inputs.program(&original, &bundle.graph.revisions).unwrap();
        let Some(Halt::NeedsResolution { request }) = replay::replay(&pending).unwrap().halt else {
            panic!("new private request")
        };
        inputs.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: source.id.clone(),
                loader_data: source.origin.selected_path.clone(),
            },
        });
        bundle.graph.private = inputs;
        for path in [
            temp.path().join("old-root.lua"),
            temp.path().join("old-root-part.lua"),
            path,
        ] {
            std::fs::remove_file(path).unwrap();
        }
        let before = Arc::new(source_graph::Saved::new(bundle, version).unwrap());
        let state = initial_state();
        {
            let mut s = state.borrow_mut();
            attach(&mut s, &old, version);
            s.source_graphs.insert(version.into(), before.clone());
            s.saved_source_graphs
                .insert(version.into(), Some(before.fingerprint.clone()));
        }
        let window = source_analysis::show_saved(parent, &state).unwrap();
        wait(|| {
            named(window.upcast_ref(), "dependency-analysis-file")
                .and_then(|w| w.downcast::<gtk::DropDown>().ok())
                .and_then(|p| p.model())
                .is_some_and(|m| m.n_items() == 3)
        });
        source_changes::test_keep_in_draft(&window, &state);
        let after = state.borrow().source_graphs[version].clone();
        assert!(Arc::ptr_eq(
            &before.bundle.graph.private.sources,
            &after.bundle.graph.private.sources
        ));
        assert_eq!(after.bundle.graph.private.bindings.len(), 1);
        assert_eq!(after.bundle.graph.private.sources[0].content, "return 4");
        workflow::test_graph_persistence(parent, &state, temp.path());
        state.borrow_mut().undo_last();
        assert_eq!(
            state.borrow().source_graphs[version].fingerprint,
            before.fingerprint
        );
        window.close();
    }
}
