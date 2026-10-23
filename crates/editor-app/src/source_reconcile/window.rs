//! Focused file, conflict and result presentation.
use super::*;
pub(super) fn show(
    parent: &impl IsA<gtk::Window>,
    state: &Shared,
    incoming: Arc<source_graph::Saved>,
) -> Result<(adw::Window, Controls), String> {
    let (old, generation, root_hash) = {
        let s = state.borrow();
        let old =
            s.source_graphs.get(&s.draft.version).cloned().ok_or(
                "Retain the current source graph before reviewing an incoming configuration",
            )?;
        if incoming.version != s.draft.version {
            return Err("Incoming configuration belongs to a different snapshot".into());
        }
        (
            old,
            s.generation,
            s.sources
                .get(&s.draft.version)
                .ok_or("Current root missing")?
                .report
                .source_sha256
                .clone(),
        )
    };
    let win = adw::Window::builder()
        .title("Merge Configuration")
        .transient_for(parent)
        .default_width(760)
        .default_height(700)
        .build();
    win.set_size_request(350, 420);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    let title = label("Merge");
    title.set_selectable(false);
    header.set_title_widget(Some(&title));
    let stop = button("merge-stop", "Stop");
    header.pack_end(&stop);
    root.append(&header);
    let pages = adw::ViewStack::new();
    pages.set_hhomogeneous(false);
    pages.set_vhomogeneous(false);
    pages.set_vexpand(true);
    let switch = adw::ViewSwitcher::builder()
        .policy(adw::ViewSwitcherPolicy::Narrow)
        .stack(&pages)
        .halign(gtk::Align::Center)
        .build();
    root.append(&switch);
    let matches = page();
    matches.append(&label("Match each saved file to an incoming capture, keep it as an inactive archive, or explicitly discard it. The draft changes only after the result review."));
    let old_file = pick("merge-old-file", &[]);
    old_file.set_enable_search(true);
    matches.append(&label("Saved file"));
    matches.append(&old_file);
    let destination = pick("merge-destination", &[]);
    destination.set_enable_search(true);
    matches.append(&label("Incoming counterpart or preservation choice"));
    matches.append(&destination);
    let path = label("");
    matches.append(
        &gtk::Expander::builder()
            .label("Captured file details")
            .child(&path)
            .build(),
    );
    let (context_area, context_text) = text_area("merge-context-details");
    context_area.set_vexpand(false);
    context_area.set_min_content_height(160);
    context_text.buffer().set_text(&format!(
        "Saved configuration\n{}\n\nIncoming configuration\n{}",
        context_description(&old.bundle, &old.version)?,
        context_description(&incoming.bundle, &incoming.version)?
    ));
    matches.append(
        &gtk::Expander::builder()
            .label("File and search context")
            .child(&context_area)
            .build(),
    );
    let context = check_button("I reviewed the changed file and search context");
    context.set_widget_name("merge-context");
    matches.append(&context);
    let next = button("merge-review-files", "Review Matched Files");
    matches.append(&next);
    add_page(&pages, &matches, "matches", "Files");
    let text_page = page();
    let text_file = pick("merge-text-file", &[]);
    text_file.set_enable_search(true);
    text_page.append(&label("Mapped file"));
    text_page.append(&text_file);
    let inspect = button("merge-inspect", "Compare This File");
    text_page.append(&inspect);
    let conflict = pick("merge-conflict", &[]);
    text_page.append(&conflict);
    let branch = pick(
        "merge-branch",
        &[
            "Original captured bytes",
            "Private draft bytes",
            "Incoming captured bytes",
        ],
    );
    text_page.append(&branch);
    let text_page_pick = gtk::SpinButton::with_range(1.0, 1.0, 1.0);
    text_page_pick.set_widget_name("merge-text-page");
    text_page_pick.update_property(&[gtk::accessible::Property::Label("Source text page")]);
    text_page.append(&label("Text page · 16 KiB per page"));
    text_page.append(&text_page_pick);
    let (area, text) = text_area("merge-text");
    text_page.append(&area);
    let choice = pick(
        "merge-choice",
        &[
            "Choose conflict result",
            "Use private draft",
            "Use incoming capture",
            "Use original bytes",
            "Private then incoming",
            "Incoming then private",
        ],
    );
    text_page.append(&choice);
    let use_file = button("merge-use-file", "Use This File's Choices");
    text_page.append(&use_file);
    let build = button("merge-build", "Rebuild and Review Result");
    text_page.append(&build);
    add_page(&pages, &text_page, "text", "Text");
    let result = page();
    let model = Rc::new(RefCell::new(None));
    let (analysis_body, analysis) = crate::source_analysis::build(&model, state, None);
    analysis.read_only();
    result.append(&analysis_body);
    let partial = check_button(
        "Keep this as a partial private draft; unresolved behavior remains unverified",
    );
    partial.set_widget_name("merge-partial");
    partial.set_visible(false);
    result.append(&partial);
    let keep = button("merge-keep", "Keep Merge in Draft");
    result.append(&keep);
    add_page(&pages, &result, "result", "Result");
    root.append(&pages);
    let status = label("Preparing captured file correspondence…");
    status.set_widget_name("merge-status");
    status.set_margin_start(16);
    status.set_margin_end(16);
    status.set_margin_bottom(12);
    root.append(&status);
    win.set_content(Some(&root));
    let session = Rc::new(RefCell::new(Session {
        old,
        incoming,
        generation,
        root_hash,
        review: None,
        selection: Default::default(),
        decisions: Default::default(),
        file: None,
        candidate: None,
        model,
        epoch: 0,
        pending: false,
        closed: false,
        cancel: Arc::new(AtomicBool::new(false)),
    }));
    let ui = Controls {
        session: Rc::downgrade(&session),
        state: Rc::downgrade(state),
        updating: Rc::new(Cell::new(false)),
        window: win.downgrade(),
        pages: pages.downgrade(),
        old_file: old_file.downgrade(),
        destination: destination.downgrade(),
        context: context.downgrade(),
        path: path.downgrade(),
        next: next.downgrade(),
        text_file: text_file.downgrade(),
        inspect: inspect.downgrade(),
        branch: branch.downgrade(),
        page: text_page_pick.downgrade(),
        text: text.downgrade(),
        conflict: conflict.downgrade(),
        choice: choice.downgrade(),
        use_file: use_file.downgrade(),
        build: build.downgrade(),
        keep: keep.downgrade(),
        stop: stop.downgrade(),
        partial: partial.downgrade(),
        status: status.downgrade(),
        analysis,
    };
    let close = session.clone();
    win.connect_close_request(move |_| {
        let mut s = close.borrow_mut();
        s.closed = true;
        s.cancel.store(true, Ordering::Release);
        s.invalidate();
        gtk::glib::Propagation::Proceed
    });
    let u = ui.clone();
    old_file.connect_selected_notify(move |_| {
        if !u.updating.get() {
            u.matching();
        }
    });
    let u = ui.clone();
    destination.connect_selected_notify(move |_| u.map());
    let u = ui.clone();
    context.connect_toggled(move |c| {
        if u.updating.get() {
            return;
        }
        if let Some(s) = u.session.upgrade() {
            let mut s = s.borrow_mut();
            if s.pending {
                return;
            }
            s.selection.accept_context_change = c.is_active();
            s.invalidate();
        }
        u.refresh();
    });
    let u = ui.clone();
    next.connect_clicked(move |_| u.files());
    let u = ui.clone();
    inspect.connect_clicked(move |_| u.inspect());
    let u = ui.clone();
    text_file.connect_selected_notify(move |_| {
        if !u.updating.get() {
            if let Some(s) = u.session.upgrade() {
                s.borrow_mut().file = None;
            }
            u.display_text();
        }
    });
    let u = ui.clone();
    conflict.connect_selected_notify(move |_| {
        if !u.updating.get() {
            u.display_text();
        }
    });
    let u = ui.clone();
    branch.connect_selected_notify(move |_| u.display_text());
    let u = ui.clone();
    text_page_pick.connect_value_changed(move |_| {
        if !u.updating.get() {
            u.display_text();
        }
    });
    let u = ui.clone();
    choice.connect_selected_notify(move |_| u.choose());
    let u = ui.clone();
    use_file.connect_clicked(move |_| u.use_file());
    let u = ui.clone();
    build.connect_clicked(move |_| u.build());
    let u = ui.clone();
    keep.connect_clicked(move |_| u.keep());
    let u = ui.clone();
    partial.connect_toggled(move |_| u.refresh());
    let u = ui.clone();
    stop.connect_clicked(move |_| {
        if let Some(s) = u.session.upgrade() {
            s.borrow().cancel.store(true, Ordering::Release);
        }
        u.feedback("Stopping after the bounded worker returns…");
        u.refresh();
    });
    let u = ui.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        let (Some(s), Some(state)) = (u.session.upgrade(), u.state.upgrade()) else {
            return gtk::glib::ControlFlow::Break;
        };
        let current = s.borrow().current(&state);
        if !current {
            let mut s = s.borrow_mut();
            s.cancel.store(true, Ordering::Release);
            s.closed = true;
            s.invalidate();
            drop(s);
            u.feedback("Draft or source changed. Close this review and start again.");
            u.analysis.reset("This result is stale.");
            u.refresh();
            return gtk::glib::ControlFlow::Break;
        }
        gtk::glib::ControlFlow::Continue
    });
    let data = {
        let s = session.borrow();
        (s.old.clone(), s.incoming.clone())
    };
    let (epoch, cancel, permit) = ui.begin().ok_or("A source operation is still finishing")?;
    let u = ui.clone();
    workflow::background(
        move || {
            let _permit = permit;
            check(&cancel)?;
            let r = merge::prepare(&data.0.bundle, &data.1.bundle, &data.0.version)?;
            check(&cancel)?;
            Ok(r)
        },
        move |result| {
            let Some(session) = u.finish(epoch) else {
                return;
            };
            match result {
                Ok(r) => {
                    let names = r.old_views.iter().map(view_name).collect::<Vec<_>>();
                    let mut s = session.borrow_mut();
                    let old_root = s.old.bundle.graph.root.clone();
                    let incoming_root = s.incoming.bundle.graph.root.clone();
                    s.selection.pairs.insert(old_root, Some(incoming_root));
                    let changed = r.context_changed;
                    s.review = Some(r);
                    drop(s);
                    u.updating.set(true);
                    old_file.set_model(Some(&gtk::StringList::new(
                        &names.iter().map(String::as_str).collect::<Vec<_>>(),
                    )));
                    old_file.set_selected(0);
                    context.set_visible(changed);
                    u.updating.set(false);
                    u.matching();
                    u.feedback("Choose each file's counterpart or preservation action. Your draft is unchanged.");
                }
                Err(e) => u.feedback(&e),
            }
            u.refresh();
        },
    );
    win.present();
    Ok((win, ui))
}

fn check_button(title: &str) -> gtk::CheckButton {
    let b = gtk::CheckButton::new();
    let text = label(title);
    text.set_selectable(false);
    b.set_child(Some(&text));
    b.update_property(&[gtk::accessible::Property::Label(title)]);
    b
}
