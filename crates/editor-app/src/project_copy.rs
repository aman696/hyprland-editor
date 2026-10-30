//! A focused, two-step review for combined graphical drafts and captured files.
use super::{label, worker, workflow, Shared};
use adw::prelude::*;
use hyprland_editor_core::lua::replay::project_copy::{Family, Mode, Prepared};
use std::{cell::RefCell, rc::Rc};

fn finding_label(kind: &hyprland_editor_core::lua::replay::edits::DifferenceKind) -> &'static str {
    use hyprland_editor_core::lua::replay::edits::DifferenceKind::*;
    match kind {
        CallSiteOrApi => "call site changed",
        Arguments => "other arguments changed",
        MissingCall => "other call removed",
        AddedCall => "other call added",
        Loader => "module loading changed",
        PathHistory => "path decisions changed",
        RegistrationScope => "submap context changed",
        ReturnValues => "module returns changed",
        LoggedModuleErrors => "module errors changed",
        IncompleteModel => "partial static analysis",
        RequestedDeclaration => "requested call mismatch",
    }
}

pub(super) fn show(parent: &adw::ApplicationWindow, state: &Shared) -> adw::Window {
    let window = adw::Window::builder()
        .title("Combined Project Copy")
        .transient_for(parent)
        .modal(true)
        .default_width(760)
        .default_height(660)
        .build();
    window.set_size_request(320, 420);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    let stack = gtk::Stack::new();
    stack.set_vexpand(true);
    let choices_page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    choices_page.append(&label("Choose which graphical drafts to include. Current private source edits are retained. Append adds drafts near the end of the main file; replace first removes reached calls of that group across captured files. Unreached code is retained."));
    let choices_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let mut choices = vec![];
    let counts = {
        let s = state.borrow();
        let v = &s.draft.version;
        [
            s.draft.values.len(),
            s.monitors.get(v).map_or(0, Vec::len),
            s.devices.get(v).map_or(0, Vec::len),
            s.bindings
                .get(v)
                .map_or(0, |d| d.binds.len() + d.submaps.len()),
            s.workspace_rules.get(v).map_or(0, Vec::len),
            s.rules.get(v).map_or(0, Vec::len),
            s.animations.get(v).map_or(0, |d| d.declarations.len()),
            s.gestures.get(v).map_or(0, |d| d.declarations.len()),
            s.session.get(v).map_or(0, |d| d.entries.len()),
        ]
    };
    for (index, family) in Family::ALL.into_iter().enumerate() {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
        row.append(&label(&format!(
            "{} · {} draft items",
            family.label(),
            counts[index]
        )));
        let choice = gtk::DropDown::from_strings(&[
            "Keep source edits",
            "Append graphical draft",
            "Replace reached calls",
        ]);
        choice.set_widget_name(&format!("project-copy-mode-{index}"));
        choice.update_property(&[gtk::accessible::Property::Label(family.label())]);
        row.append(&choice);
        choices_box.append(&row);
        choices.push(choice);
    }
    choices_page.append(
        &gtk::ScrolledWindow::builder()
            .child(&choices_box)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    let status = label("An empty draft removes reached calls only when you explicitly choose replace. Nothing is applied to your desktop.");
    status.set_widget_name("project-copy-status");
    choices_page.append(&status);
    let prepare = gtk::Button::with_label("Review Combined Copy");
    prepare.add_css_class("suggested-action");
    choices_page.append(&prepare);
    stack.add_named(&choices_page, Some("choices"));
    let review_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let summary = label("");
    summary.set_widget_name("project-copy-summary");
    review_page.append(&summary);
    let details = label("");
    review_page.append(
        &gtk::Expander::builder()
            .label("Choices and analysis details")
            .child(&details)
            .build(),
    );
    review_page.append(&label("Files keep their relative directory layout. Original path strings and uncaptured assets are unchanged; this copy is not a verified portable setup or a live preview."));
    let files = gtk::DropDown::from_strings(&[]);
    files.set_widget_name("project-copy-files");
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        item.set_child(Some(
            &gtk::Label::builder()
                .ellipsize(gtk::pango::EllipsizeMode::Middle)
                .max_width_chars(24)
                .xalign(0.0)
                .build(),
        ));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let text = item.item().and_downcast::<gtk::StringObject>().unwrap();
        item.child()
            .and_downcast::<gtk::Label>()
            .unwrap()
            .set_text(&text.string());
    });
    files.set_factory(Some(&factory));
    files.update_property(&[gtk::accessible::Property::Label("Captured file to review")]);
    review_page.append(&files);
    let text = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    review_page.append(
        &gtk::ScrolledWindow::builder()
            .child(&text)
            .vexpand(true)
            .build(),
    );
    let feedback = label("");
    feedback.set_widget_name("project-copy-export-status");
    review_page.append(&feedback);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let back = gtk::Button::with_label("Change Choices");
    let single = gtk::Button::with_label("Export This File…");
    let save = gtk::Button::with_label("Save All Files…");
    save.add_css_class("suggested-action");
    actions.append(&back);
    actions.append(&single);
    actions.append(&save);
    // Wrap actions on small windows using a vertical layout.
    actions.set_orientation(gtk::Orientation::Vertical);
    review_page.append(&actions);
    stack.add_named(&review_page, Some("review"));
    content.append(&stack);
    window.set_content(Some(&content));
    let prepared = Rc::new(RefCell::new(None::<Rc<Prepared>>));
    let reviewed_generation = Rc::new(std::cell::Cell::new(0u64));
    let snapshot = prepared.clone();
    let view = text.downgrade();
    files.connect_selected_notify(move |files| {
        if let (Some(copy), Some(view)) = (snapshot.borrow().as_ref(), view.upgrade()) {
            if let Some(file) = copy.files.get(files.selected() as usize) {
                view.buffer().set_text(&file.content);
            }
        }
    });
    let page = stack.downgrade();
    let snapshot = prepared.clone();
    let view = text.downgrade();
    let file_list = files.downgrade();
    back.connect_clicked(move |_| {
        if let Some(page) = page.upgrade() {
            page.set_visible_child_name("choices");
        }
        snapshot.borrow_mut().take();
        if let Some(view) = view.upgrade() {
            view.buffer().set_text("");
        }
        if let Some(files) = file_list.upgrade() {
            files.set_model(None::<&gtk::StringList>);
        }
    });
    let snapshot = prepared.clone();
    let chooser = files.downgrade();
    let win = window.downgrade();
    single.connect_clicked(move |_| {
        let (Some(window),Some(files))=(win.upgrade(),chooser.upgrade()) else {return;};
        let copied=snapshot.borrow(); let Some(copy)=copied.as_ref()else{return;};
        let Some(file)=copy.files.get(files.selected() as usize)else{return;};
        super::text_export::review(&window,"Review combined file","This file belongs to the reviewed combined copy. Save all files to preserve its captured directory layout.","hyprland-edited.lua",file.content.clone());
    });
    let win = window.downgrade();
    let page = stack.downgrade();
    let state_copy = state.clone();
    let output = prepared.clone();
    let list = files.downgrade();
    let heading = summary.downgrade();
    let detail_text = details.downgrade();
    let progress = status.downgrade();
    let generation = reviewed_generation.clone();
    let view = text.downgrade();
    prepare.connect_clicked(move|button| {
        if win.upgrade().is_none_or(|w|!w.is_visible()){return;}
        let current=state_copy.borrow();
        let workspace=match workflow::captured_workspace(&current){Ok(w)=>w,Err(e)=>{if let Some(progress)=progress.upgrade(){progress.set_text(&e);}return;}};
        let epoch=current.generation; drop(current);
        let modes=std::array::from_fn(|i| match choices[i].selected(){1=>Mode::Append,2=>Mode::ReplaceReached,_=>Mode::Keep});
        button.set_sensitive(false);
        if let Some(progress)=progress.upgrade(){progress.set_text("Combining captured files and selected drafts…");}
        let button=button.downgrade(); let win=win.clone(); let page=page.clone(); let output=output.clone();
        let list=list.clone(); let heading=heading.clone(); let detail_text=detail_text.clone(); let progress=progress.clone(); let generation=generation.clone();
        let state=state_copy.clone(); let view=view.clone();
        workflow::background(move||worker::run(worker::Request::CopyProject{workspace:Box::new(workspace),modes}),move|result|{
            if let Some(button)=button.upgrade(){button.set_sensitive(true);}
            if win.upgrade().is_none_or(|w|!w.is_visible()){return;}
            let result=if state.borrow().generation!=epoch {Err("The draft changed during preparation. Review again.".into())} else {result};
            match result {
                Ok(worker::Response::ProjectCopy(copy))=>{
                    let (Some(page),Some(list),Some(heading),Some(view))=(page.upgrade(),list.upgrade(),heading.upgrade(),view.upgrade())else{return;};
                    let copy=Rc::new(*copy);
                    let names=super::project_export::layout(&copy).map(|paths|paths.into_iter().zip(&copy.files).map(|(path,file)|format!("{}{}",path.to_string_lossy(),if file.changed{" — edited"}else{""})).collect::<Vec<_>>());
                    let (names,layout_note)=match names {Ok(names)=>(names,String::new()),Err(error)=>(copy.files.iter().map(|f|String::from_utf8_lossy(&f.load_name).into_owned()).collect(),format!("\nFolder export unavailable: {error}"))};
                    let selected=copy.files.iter().position(|f|f.document==copy.root).unwrap_or(0) as u32;
                    let choices=Family::ALL.into_iter().zip(modes).map(|(f,m)|format!("{}: {}",f.label(),match m{Mode::Keep=>"keep",Mode::Append=>"append",Mode::ReplaceReached=>"replace reached"})).collect::<Vec<_>>().join(" · ");
                    heading.set_text(&format!("{} files · {} removed call sites · {} inserted calls\n{} additional changes to review",copy.files.len(),copy.removed_sites,copy.inserted_sites,copy.findings.len()));
                    if let Some(details)=detail_text.upgrade(){
                        let findings=if copy.findings.is_empty(){"None".into()}else{copy.findings.iter().map(finding_label).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>().join(", ")};
                        details.set_text(&format!("{choices}\nStatic capture: {} → {}. Additional findings: {findings}.{layout_note}",if copy.before_complete{"complete"}else{"partial"},if copy.after_complete{"complete"}else{"partial"}));
                    }
                    *output.borrow_mut()=Some(copy.clone()); generation.set(epoch);
                    list.set_model(Some(&gtk::StringList::new(&names.iter().map(String::as_str).collect::<Vec<_>>())));
                    list.set_selected(selected); view.buffer().set_text(&copy.files[selected as usize].content);
                    page.set_visible_child_name("review");
                }
                Ok(_)=>{if let Some(progress)=progress.upgrade(){progress.set_text("The worker returned an unexpected response.");}}
                Err(error)=>{if let Some(progress)=progress.upgrade(){progress.set_text(&error);}}
            }
        });
    });
    let win = window.downgrade();
    let state = state.clone();
    let snapshot = prepared.clone();
    let generation = reviewed_generation.clone();
    save.connect_clicked(move |_| {
        let Some(parent) = win.upgrade() else {
            return;
        };
        let Some(copy) = snapshot.borrow().clone() else {
            return;
        };
        if state.borrow().generation != generation.get() {
            feedback
                .set_text("The draft changed. Change Choices and review again before exporting.");
            return;
        }
        if let Err(error) = super::project_export::layout(&copy) {
            feedback.set_text(&error);
            return;
        }
        export_dialog(&parent, copy, &feedback);
    });
    window.present();
    window
}

fn export_dialog(parent: &adw::Window, copy: Rc<Prepared>, feedback: &gtk::Label) {
    let dialog = adw::Window::builder()
        .title("Save Combined Copy")
        .transient_for(parent)
        .modal(true)
        .default_width(420)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.append(&label("Choose a parent folder. A new private folder will contain all reviewed files; existing folders are refused."));
    let name = gtk::Entry::builder()
        .text("hyprland-edited")
        .max_length(128)
        .build();
    name.update_property(&[gtk::accessible::Property::Label("New folder name")]);
    content.append(&name);
    let choose = gtk::Button::with_label("Choose Parent Folder…");
    choose.add_css_class("suggested-action");
    content.append(&choose);
    let status = label("");
    content.append(&status);
    dialog.set_content(Some(&content));
    let win = dialog.downgrade();
    let feedback = feedback.downgrade();
    choose.connect_clicked(move |button| {
        let Some(parent) = win.upgrade() else {
            return;
        };
        let name = name.text().to_string();
        let chooser = gtk::FileDialog::builder()
            .title("Choose Parent for New Config Folder")
            .accept_label("Create Copy Here")
            .build();
        let button = button.downgrade();
        let status = status.downgrade();
        let copy = copy.clone();
        let feedback = feedback.clone();
        gtk::glib::MainContext::default().spawn_local(async move {
            let selected = chooser.select_folder_future(Some(&parent)).await;
            if !parent.is_visible() {
                return;
            }
            let file = match selected {
                Ok(file) => file,
                Err(error)
                    if error.matches(gtk::DialogError::Dismissed)
                        || error.matches(gtk::DialogError::Cancelled) =>
                {
                    return
                }
                Err(error) => {
                    if let Some(status) = status.upgrade() {
                        status.set_text(&error.to_string());
                    }
                    return;
                }
            };
            let Some(path) = file.path() else {
                if let Some(status) = status.upgrade() {
                    status.set_text("Choose a local folder.");
                }
                return;
            };
            if let Some(button) = button.upgrade() {
                button.set_sensitive(false);
            }
            if let Some(status) = status.upgrade() {
                status.set_text("Saving the reviewed files…");
            }
            let copy = (*copy).clone();
            workflow::background(
                move || super::project_export::save(&path, &name, &copy),
                move |result| {
                    if let Some(button) = button.upgrade() {
                        button.set_sensitive(true);
                    }
                    match result {
                        Ok(path) => {
                            if let Some(feedback) = feedback.upgrade() {
                                feedback.set_text(&format!(
                                    "Combined copy saved to {}",
                                    path.display()
                                ));
                            }
                            parent.destroy();
                        }
                        Err(error) => {
                            if let Some(status) = status.upgrade() {
                                status.set_text(&error);
                            }
                        }
                    }
                },
            );
        });
    });
    dialog.present();
}

#[cfg(test)]
pub(super) fn test_workflow(parent: &adw::ApplicationWindow, source_state: &Shared) {
    fn widgets(root: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut result = vec![root.clone()];
        let mut child = root.first_child();
        while let Some(widget) = child {
            result.extend(widgets(&widget));
            child = widget.next_sibling();
        }
        result
    }
    fn button(window: &adw::Window, name: &str) -> gtk::Button {
        widgets(window.upcast_ref())
            .into_iter()
            .find_map(|w| {
                w.downcast::<gtk::Button>()
                    .ok()
                    .filter(|b| b.label().as_deref() == Some(name))
            })
            .unwrap()
    }
    fn wait(test: impl Fn() -> bool) {
        let started = std::time::Instant::now();
        while !test() {
            let context = gtk::glib::MainContext::default();
            while context.pending() {
                context.iteration(false);
            }
            assert!(started.elapsed() < std::time::Duration::from_secs(12));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    use hyprland_editor_core::BASELINE;
    let state = super::initial_state();
    state.borrow_mut().source_graphs = source_state.borrow().source_graphs.clone();
    let catalogue = state.borrow().catalogue.clone();
    let desired=hyprland_editor_core::lua::parse("hl.config{general={border_size=8}};hl.monitor{output='DP-3'};hl.device{name='combined mouse'};hl.bind('Z',hl.dsp.no_op());hl.workspace_rule{workspace='3'};hl.window_rule{match={class='combined'},float=true};hl.curve('combined',{type='bezier',points={{0.1,0.2},{0.8,0.9}}});hl.animation{leaf='windows',enabled=false};hl.gesture{fingers=4,direction='left',action='workspace'}",BASELINE,&catalogue).unwrap();
    assert!(desired.diagnostics.is_empty() && !desired.opaque_effects);
    {
        let mut s = state.borrow_mut();
        s.draft = desired.draft;
        s.replace_monitors(desired.monitors).unwrap();
        s.replace_devices(desired.devices).unwrap();
        s.replace_bindings(desired.bindings).unwrap();
        s.replace_workspaces(desired.workspace_rules).unwrap();
        s.replace_rules(desired.rules).unwrap();
        s.replace_animations(desired.animations).unwrap();
        s.replace_gestures(desired.gestures).unwrap();
    }
    let workspace = workflow::captured_workspace(&state.borrow()).unwrap();
    let epoch = state.borrow().generation;
    // Invoke the product action, after the source fixture's original directory has been removed.
    let launch = gtk::Button::new();
    workflow::connect_source_copy(&launch, parent, &state);
    launch.emit_clicked();
    let window = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<adw::Window>().ok())
        .find(|w| w.title().as_deref() == Some("Combined Project Copy"))
        .unwrap();
    let choices = widgets(window.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::DropDown>().ok())
        .filter(|w| w.widget_name().starts_with("project-copy-mode-"))
        .collect::<Vec<_>>();
    assert_eq!(choices.len(), 9);
    assert!(choices.iter().all(|c| c.selected() == 0));
    let review = button(&window, "Review Combined Copy");
    review.emit_clicked();
    wait(|| review.is_sensitive());
    let summary = widgets(window.upcast_ref())
        .into_iter()
        .find_map(|w| {
            w.downcast::<gtk::Label>()
                .ok()
                .filter(|w| w.widget_name() == "project-copy-summary")
        })
        .unwrap();
    assert!(
        summary
            .text()
            .contains("3 files · 0 removed call sites · 0 inserted calls"),
        "{}",
        summary.text()
    );
    button(&window, "Change Choices").emit_clicked();
    for choice in &choices {
        choice.set_selected(1);
    }
    review.emit_clicked();
    wait(|| review.is_sensitive());
    assert!(
        summary
            .text()
            .contains("3 files · 0 removed call sites · 9 inserted calls"),
        "{}",
        summary.text()
    );
    let text = widgets(window.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::TextView>().ok())
        .unwrap();
    let buffer = text.buffer();
    let shown = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
    assert!(
        shown.contains("combined mouse") && shown.contains("Graphical drafts"),
        "{shown}"
    );
    let worker::Response::ProjectCopy(copy) = worker::run(worker::Request::CopyProject {
        workspace: Box::new(workspace),
        modes: [Mode::Append; 9],
    })
    .unwrap() else {
        panic!("wrong response")
    };
    assert_eq!(
        shown.as_str(),
        copy.files
            .iter()
            .find(|f| f.document == copy.root)
            .unwrap()
            .content
    );
    let temp = super::test_support::TempDir::new("combined-ui-copy");
    let saved = super::project_export::save(temp.path(), "exported", &copy).unwrap();
    for (file, path) in copy
        .files
        .iter()
        .zip(super::project_export::layout(&copy).unwrap())
    {
        assert_eq!(
            std::fs::read(saved.join(path)).unwrap(),
            file.content.as_bytes()
        );
    }
    button(&window, "Save All Files…").emit_clicked();
    let save = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<adw::Window>().ok())
        .find(|w| w.title().as_deref() == Some("Save Combined Copy"))
        .unwrap();
    assert!(button(&save, "Choose Parent Folder…").is_sensitive());
    save.destroy();
    let (minimum, _, _, _) = window
        .content()
        .unwrap()
        .measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Combined review needs {minimum}px");
    if std::env::var_os("HYPRLAND_EDITOR_TEST_EXACT_RESIZE").is_some() {
        window.set_default_size(360, 760);
        window.present();
        wait(|| (350..=360).contains(&window.width()));
    }
    // Wait for an actual rendered frame, rather than accepting an empty snapshot.
    for _ in 0..25 {
        let context = gtk::glib::MainContext::default();
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if let Some(path) = std::env::var_os("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let path = std::path::PathBuf::from(format!(
            "{}.project-copy.png",
            std::path::Path::new(&path).display()
        ));
        let paintable = gtk::WidgetPaintable::new(Some(&window));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
        window
            .renderer()
            .expect("review renderer")
            .render_texture(snapshot.to_node().expect("rendered combined review"), None)
            .save_to_png(path)
            .unwrap();
    }
    assert_eq!(
        state.borrow().generation,
        epoch,
        "Review/export must not change drafts or saved sources"
    );
    window.destroy();
}
