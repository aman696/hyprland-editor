use super::*;
use hyprland_editor_core::{
    lua::{self, replay},
    module_path::Context,
    storage,
    workspace::{SourceLanguage, SourceSnapshot},
    workspace_graph::{
        self,
        revisions::{Buffer, Piece, Revision},
        Graph, View,
    },
    AUDITED_MAIN, BASELINE,
};
use std::path::Path;

fn fixture(folder: &Path, version: &str) -> Arc<source_graph::Pending> {
    let root = folder.join("main.lua");
    let archive = folder.join("archive.lua");
    std::fs::write(&root, "hl.config{general={border_size=3}}\n").unwrap();
    std::fs::write(&archive, "return 4\n").unwrap();
    let main = storage::read_source(&root).unwrap();
    let old = storage::read_source(&archive).unwrap();
    let main_view = View {
        document: "root".into(),
        load_name: main.identity.selected_path.clone(),
    };
    let old_view = View {
        document: "archive".into(),
        load_name: old.identity.selected_path.clone(),
    };
    let revision = |id: &str, before: &str, after: &str| Revision {
        document: id.into(),
        base_sha256: lua::source_hash(before),
        sha256: lua::source_hash(after),
        buffers: vec![Arc::from(after)],
        pieces: vec![Piece {
            buffer: Buffer::Inserted { index: 0 },
            span: 0..after.len(),
        }],
    };
    let private = workspace_graph::private::Inputs {
        sources: Arc::new(vec![SourceSnapshot {
            id: "archive".into(),
            language: SourceLanguage::Lua,
            content: old.content.clone(),
            origin: old.identity,
        }]),
        views: vec![old_view],
        bindings: vec![],
        archived: ["archive".into()].into_iter().collect(),
    };
    let bundle = Bundle {
        graph:Graph {
            format:"hye-workspace-graph-1".into(),schema_sha256:workspace_graph::schema_sha256(),compiler:replay::compiler_profile(),root:"root".into(),
            context:Context{main_file:main.identity.selected_path.clone(),home:None,inherited_package_path:None},working_directory:None,views:vec![main_view],bindings:vec![],private,
            revisions:vec![revision("root",&main.content,"local a=require('fresh');local b=require('second');hl.config{general={border_size=a+b}}\n"),revision("archive",&old.content,"return 6\n")],
        },
        sources:Arc::new(vec![SourceSnapshot{id:"root".into(),language:SourceLanguage::Lua,content:main.content,origin:main.identity}]),
    };
    std::fs::remove_file(root).unwrap();
    std::fs::remove_file(archive).unwrap();
    Arc::new(
        source_graph::inspect_pending(&bundle, version, Arc::new(AtomicBool::new(false))).unwrap(),
    )
}
fn wait(check: impl Fn() -> bool) {
    let start = std::time::Instant::now();
    loop {
        while gtk::glib::MainContext::default().pending() {
            gtk::glib::MainContext::default().iteration(false);
        }
        if check() {
            return;
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(20),
            "Pending import GUI timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
fn idle(ui: &Controls) {
    wait(|| ui.session.upgrade().is_some_and(|s| !s.borrow().busy));
}
fn click(button: &gtk::glib::WeakRef<gtk::Button>) {
    button.upgrade().unwrap().emit_clicked();
}

pub(crate) fn test_workflow(parent: &adw::ApplicationWindow) {
    let review = adw::Window::builder()
        .transient_for(parent)
        .title("Private import fixture")
        .build();
    review.present();
    for version in [BASELINE, AUDITED_MAIN] {
        eprintln!("pending import GUI: {version}");
        let temp = super::super::test_support::TempDir::new("he-pending-native");
        let data = fixture(temp.path(), version);
        let original = data.bundle().fingerprint().unwrap();
        std::fs::write(temp.path().join("fresh.lua"), "return 5\n").unwrap();
        let returned = Rc::new(RefCell::new(None));
        let destination = returned.clone();
        let (window, ui) = build(
            &review,
            data.clone(),
            Rc::new(|| true),
            Rc::new(move |b| *destination.borrow_mut() = Some(b)),
        );
        wait(|| window.is_visible());
        let session = ui.session.upgrade().unwrap();
        assert!(!ui.capture.upgrade().unwrap().is_sensitive());
        assert!(!ui.finish.upgrade().unwrap().is_sensitive());
        // Even a programmatic request cannot manufacture filesystem absence.
        ui.capture();
        idle(&ui);
        assert!(session.borrow().capture.is_none());
        assert!(session
            .borrow()
            .data
            .as_ref()
            .unwrap()
            .bundle()
            .graph
            .private
            .bindings
            .is_empty());
        ui.mode.upgrade().unwrap().set_selected(2);
        assert!(!ui.use_input.upgrade().unwrap().is_sensitive());
        click(&ui.use_input);
        assert_eq!(
            session.borrow().data.as_ref().unwrap().fingerprint(),
            data.fingerprint()
        );
        ui.mode.upgrade().unwrap().set_selected(0);
        ui.approve(temp.path().into());
        idle(&ui);
        assert_eq!(session.borrow().grants.len(), 1);
        let grant = Arc::downgrade(&session.borrow().grants[0]);
        ui.capture();
        idle(&ui);
        assert_eq!(
            session.borrow().capture.as_ref().unwrap().source().content,
            "return 5\n"
        );
        assert!(!ui.text.upgrade().unwrap().is_editable());
        assert!(ui.detail.upgrade().unwrap().text().contains("SHA-256"));
        // Forget drops the live descriptor, while immutable captured bytes stay data.
        ui.folders.upgrade().unwrap().set_selected(0);
        click(&ui.forget);
        assert!(grant.upgrade().is_none());
        std::fs::remove_file(temp.path().join("fresh.lua")).unwrap();
        click(&ui.use_input);
        idle(&ui);
        assert_eq!(
            session
                .borrow()
                .data
                .as_ref()
                .unwrap()
                .request()
                .unwrap()
                .module,
            b"second"
        );
        assert_eq!(
            session
                .borrow()
                .data
                .as_ref()
                .unwrap()
                .bundle()
                .graph
                .private
                .bindings
                .len(),
            1
        );
        assert!(returned.borrow().is_none());
        // Archived source selection previews its private pieces without activating it.
        ui.mode.upgrade().unwrap().set_selected(1);
        ui.preserved.upgrade().unwrap().set_selected(2);
        assert!(ui
            .text
            .upgrade()
            .unwrap()
            .buffer()
            .text(
                &ui.text.upgrade().unwrap().buffer().start_iter(),
                &ui.text.upgrade().unwrap().buffer().end_iter(),
                false
            )
            .contains("return 6"));
        assert!(session
            .borrow()
            .data
            .as_ref()
            .unwrap()
            .bundle()
            .graph
            .private
            .archived
            .contains("archive"));
        if let Some(path) = std::env::var_os("HYPRLAND_EDITOR_TEST_CAPTURE") {
            let start = std::time::Instant::now();
            wait(|| start.elapsed() >= std::time::Duration::from_millis(120));
            let paintable = gtk::WidgetPaintable::new(Some(&window));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
            let node = snapshot.to_node().expect("Import review has a render node");
            let surface = window.surface().expect("Import review has a surface");
            let renderer =
                gtk::gsk::Renderer::for_surface(&surface).expect("Private renderer is available");
            renderer
                .render_texture(&node, None)
                .save_to_png(std::path::PathBuf::from(format!(
                    "{}.pending-input.png",
                    path.to_string_lossy()
                )))
                .unwrap();
            renderer.unrealize();
        }
        click(&ui.use_input);
        idle(&ui);
        assert!(session.borrow().data.as_ref().unwrap().request().is_none());
        assert!(!session
            .borrow()
            .data
            .as_ref()
            .unwrap()
            .bundle()
            .graph
            .private
            .archived
            .contains("archive"));
        click(&ui.undo);
        assert_eq!(
            session
                .borrow()
                .data
                .as_ref()
                .unwrap()
                .request()
                .unwrap()
                .module,
            b"second"
        );
        assert!(session
            .borrow()
            .data
            .as_ref()
            .unwrap()
            .bundle()
            .graph
            .private
            .archived
            .contains("archive"));
        ui.mode.upgrade().unwrap().set_selected(1);
        ui.preserved.upgrade().unwrap().set_selected(2);
        click(&ui.use_input);
        idle(&ui);
        let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
        assert!(minimum <= 350, "Import review needs {minimum}px");
        window.set_default_size(360, 700);
        wait(|| (350..=360).contains(&window.width()));
        click(&ui.finish);
        wait(|| returned.borrow().is_some());
        let bundle = returned.borrow_mut().take().unwrap();
        assert_eq!(bundle.graph.private.bindings.len(), 2);
        let restored = source_graph::restore(&bundle, version).unwrap();
        assert!(restored.report.halt.is_none());
        assert!(restored
            .program
            .documents
            .iter()
            .any(|d| d.source == "return 5\n"));
        assert!(restored
            .program
            .documents
            .iter()
            .any(|d| d.source == "return 6\n"));
        assert_eq!(data.bundle().fingerprint().unwrap(), original);
        assert!(session.borrow().closed);
        assert!(session.borrow().data.is_none() && session.borrow().grants.is_empty());

        // Missing is explicit, reversible, and stays a partial model.
        let (missing_window, missing) = build(
            &review,
            data.clone(),
            Rc::new(|| true),
            Rc::new(|_| panic!("Unexpected return")),
        );
        let held = missing.session.upgrade().unwrap();
        missing.mode.upgrade().unwrap().set_selected(2);
        missing.missing.upgrade().unwrap().set_active(true);
        click(&missing.use_input);
        idle(&missing);
        assert!(held.borrow().data.as_ref().unwrap().request().is_none());
        assert!(held.borrow().data.as_ref().unwrap().report().halt.is_some());
        assert!(!missing.missing.upgrade().unwrap().is_active());
        click(&missing.undo);
        assert_eq!(
            held.borrow().data.as_ref().unwrap().fingerprint(),
            data.fingerprint()
        );
        missing_window.close();
        assert!(held.borrow().closed);

        // Invalidation closes the resolver and drops new grants, including on destroy.
        let valid = Rc::new(Cell::new(true));
        let guard = valid.clone();
        let (stale_window, stale) = build(
            &review,
            data.clone(),
            Rc::new(move || guard.get()),
            Rc::new(|_| panic!("Stale result returned")),
        );
        let held = stale.session.upgrade().unwrap();
        stale.approve(temp.path().into());
        idle(&stale);
        let grant = Arc::downgrade(&held.borrow().grants[0]);
        valid.set(false);
        wait(|| held.borrow().closed);
        assert!(!stale_window.is_visible());
        assert!(grant.upgrade().is_none());
        assert!(stale.begin().is_none());
        let (destroyed, ui) = build(
            &review,
            data,
            Rc::new(|| true),
            Rc::new(|_| panic!("Destroyed result returned")),
        );
        let held = ui.session.upgrade().unwrap();
        let (epoch, cancelled, permit) = ui.begin().unwrap();
        destroyed.destroy();
        wait(|| held.borrow().closed);
        assert!(cancelled.load(Ordering::Acquire));
        assert!(ui.end(epoch).is_none());
        drop(permit);
    }
    review.close();
}
