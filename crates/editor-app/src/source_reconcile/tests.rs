use super::*;
#[cfg(test)]
pub(crate) fn test_review(window: &adw::Window, state: &Shared, expected_partial: bool) {
    fn widgets(root: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut all = vec![root.clone()];
        let mut child = root.first_child();
        while let Some(c) = child {
            all.extend(widgets(&c));
            child = c.next_sibling();
        }
        all
    }
    fn find<T: gtk::glib::prelude::IsA<gtk::Widget> + gtk::glib::object::IsClass>(
        w: &adw::Window,
        name: &str,
    ) -> T {
        widgets(w.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .downcast::<T>()
            .ok()
            .unwrap()
    }
    fn wait(check: impl Fn() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if check() {
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(20));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    let version = state.borrow().draft.version.clone();
    let before = state.borrow().source_graphs[&version].fingerprint.clone();
    let old: gtk::DropDown = find(window, "merge-old-file");
    let dest: gtk::DropDown = find(window, "merge-destination");
    let context: gtk::CheckButton = find(window, "merge-context");
    let next: gtk::Button = find(window, "merge-review-files");
    let inspect: gtk::Button = find(window, "merge-inspect");
    let files: gtk::DropDown = find(window, "merge-text-file");
    let conflicts: gtk::DropDown = find(window, "merge-conflict");
    let choice: gtk::DropDown = find(window, "merge-choice");
    let use_file: gtk::Button = find(window, "merge-use-file");
    let build: gtk::Button = find(window, "merge-build");
    let keep: gtk::Button = find(window, "merge-keep");
    let status: gtk::Label = find(window, "merge-status");
    eprintln!("merge GUI: wait correspondence");
    wait(|| old.model().is_some_and(|m| m.n_items() == 3));
    let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Merge review needs {minimum}px");
    // The private X11 fixture includes a 10px client-side shadow.
    window.set_default_size(360, 660);
    wait(|| (350..=360).contains(&window.width()));
    assert!(!dest.is_sensitive());
    old.set_selected(1);
    dest.set_selected(4);
    old.set_selected(2);
    dest.set_selected(1);
    context.set_active(true);
    assert!(next.is_sensitive());
    next.emit_clicked();
    for index in 0..2 {
        eprintln!("merge GUI: file {index}");
        files.set_selected(index);
        inspect.emit_clicked();
        wait(|| use_file.is_sensitive());
        if index == 1 {
            assert!(conflicts.model().unwrap().n_items() > 1);
            use_file.emit_clicked();
            wait(|| use_file.is_sensitive());
            assert!(status.text().contains("conflict"), "{}", status.text());
            conflicts.set_selected(1);
            choice.set_selected(1);
        }
        use_file.emit_clicked();
        wait(|| use_file.is_sensitive());
    }
    assert_eq!(state.borrow().source_graphs[&version].fingerprint, before);
    assert!(build.is_sensitive());
    conflicts.set_selected(0);
    conflicts.set_selected(1);
    assert_eq!(choice.selected(), 1);
    assert!(build.is_sensitive());
    old.set_selected(1);
    old.set_selected(2);
    old.set_selected(1);
    assert!(
        build.is_sensitive(),
        "Reflecting unchanged mappings must retain decisions"
    );
    eprintln!("merge GUI: rebuild");
    build.emit_clicked();
    let partial: gtk::CheckButton = find(window, "merge-partial");
    wait(|| keep.is_sensitive() || partial.is_visible());
    assert_eq!(partial.is_visible(), expected_partial);
    if expected_partial {
        assert!(!keep.is_sensitive());
        keep.emit_clicked();
        assert_eq!(state.borrow().source_graphs[&version].fingerprint, before);
        partial.set_active(true);
        assert!(keep.is_sensitive());
    }
    assert_eq!(state.borrow().source_graphs[&version].fingerprint, before);
    let compare: gtk::Button = find(window, "source-analysis-compare");
    assert!(!compare.is_visible());
    if let Some(path) = std::env::var_os("HYPRLAND_EDITOR_TEST_CAPTURE") {
        window.set_default_size(700, 660);
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_millis(80) {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
        }
        let paintable = gtk::WidgetPaintable::new(Some(window));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
        if let (Some(node), Some(surface)) = (snapshot.to_node(), window.surface()) {
            if let Some(renderer) = gtk::gsk::Renderer::for_surface(&surface) {
                let texture = renderer.render_texture(&node, None);
                texture
                    .save_to_png(std::path::PathBuf::from(format!(
                        "{}.external-merge{}.png",
                        path.to_string_lossy(),
                        if expected_partial { "-partial" } else { "" }
                    )))
                    .unwrap();
                renderer.unrealize();
            }
        }
    }
    eprintln!("merge GUI: keep");
    keep.emit_clicked();
    assert!(!window.is_visible());
}

#[test]
fn source_pages_cover_every_byte_once_without_splitting_unicode() {
    for text in [
        String::new(),
        "abc".into(),
        "a".repeat(16 * 1024),
        format!(
            "{}{}{}",
            "a".repeat(16 * 1024 - 1),
            "😀é".repeat(6000),
            "tail"
        ),
    ] {
        let (_, count) = page_span(&text, 1);
        let mut restored = String::new();
        let mut previous = 0;
        for n in 1..=count {
            let (span, total) = page_span(&text, n);
            assert_eq!(total, count);
            assert_eq!(span.start, previous);
            assert!(span.len() <= 16 * 1024 + 3);
            previous = span.end;
            restored.push_str(&text[span]);
        }
        assert_eq!(restored, text);
        assert_eq!(page_span(&text, usize::MAX), page_span(&text, count));
        assert_eq!(page_span(&text, 0), page_span(&text, 1));
    }
}

#[cfg(test)]
pub(crate) fn test_stale(
    parent: &adw::ApplicationWindow,
    state: &Shared,
    incoming: Arc<source_graph::Saved>,
) {
    fn wait(check: impl Fn() -> bool) {
        let start = std::time::Instant::now();
        loop {
            for _ in 0..128 {
                if !gtk::glib::MainContext::default().pending() {
                    break;
                }
                gtk::glib::MainContext::default().iteration(false);
            }
            if check() {
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    struct Release;
    impl Drop for Release {
        fn drop(&mut self) {
            HOLD.store(false, Ordering::Release);
        }
    }
    for close in [false, true] {
        let (window, ui) = show(parent, state, incoming.clone()).unwrap();
        let session = ui.session.upgrade().unwrap();
        wait(|| !session.borrow().pending);
        {
            let mut s = session.borrow_mut();
            let r = s.review.as_ref().unwrap();
            let pairs = r
                .rows
                .iter()
                .map(|row| {
                    (
                        row.old.clone(),
                        if row.root_pair {
                            Some(s.incoming.bundle.graph.root.clone())
                        } else if r
                            .old_views
                            .iter()
                            .any(|v| v.document == row.old && v.archived)
                        {
                            None
                        } else {
                            Some(s.incoming.bundle.graph.views[1].document.clone())
                        },
                    )
                })
                .collect();
            let archives = r
                .old_views
                .iter()
                .filter(|v| v.archived)
                .map(|v| v.document.clone())
                .collect();
            s.selection.pairs = pairs;
            s.selection.archive_private = archives;
            s.selection.accept_context_change = true;
            merge::validate_selection(s.review.as_ref().unwrap(), &s.selection).unwrap();
        }
        ui.files();
        ENTERED.store(false, Ordering::Release);
        HOLD.store(true, Ordering::Release);
        let _release = Release;
        ui.inspect();
        wait(|| ENTERED.load(Ordering::Acquire));
        assert!(crate::source_changes::Permit::running());
        let version = state.borrow().draft.version.clone();
        let fingerprint = state.borrow().source_graphs[&version].fingerprint.clone();
        if close {
            window.close();
        } else {
            state.borrow_mut().generation += 1;
        }
        wait(|| session.borrow().closed && !crate::source_changes::Permit::running());
        assert!(session.borrow().file.is_none());
        assert!(session.borrow().candidate.is_none());
        ui.keep();
        assert_eq!(
            state.borrow().source_graphs[&version].fingerprint,
            fingerprint
        );
        if !close {
            window.close();
        }
    }
}
