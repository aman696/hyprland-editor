//! Background workflows keep IPC and parsing off the GTK main thread.
use super::{detail, notice, worker, Shared};
use adw::prelude::*;
use hyprland_editor_core::{import, lua::LuaDocument, storage, Draft, Setting, BASELINE};
use std::{path::PathBuf, sync::mpsc, time::Duration};
fn background<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
    finish: impl FnOnce(Result<T, String>) + 'static,
) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    let mut finish = Some(finish);
    gtk::glib::timeout_add_local(Duration::from_millis(40), move || match rx.try_recv() {
        Ok(result) => {
            finish.take().unwrap()(result);
            gtk::glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            finish.take().unwrap()(Err("Background operation exited unexpectedly".into()));
            gtk::glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
    });
}
struct Imported {
    draft: Draft,
    source: Option<LuaDocument>,
    message: String,
    saved_json: bool,
}
fn read_import(
    path: PathBuf,
    catalogue: Vec<Setting>,
    version: String,
) -> Result<Imported, String> {
    let source = storage::read_document(&path)?;
    match path.extension().and_then(|s|s.to_str()){
        Some("json")=>Draft::from_json(&source,&catalogue).map(|draft|Imported{draft,source:None,message:"Validated draft loaded".into(),saved_json:true}),
        Some("conf")=>import::legacy(&source,&catalogue).map(|r|Imported{message:format!("Imported {} literal overrides; {} lines skipped; {} repeated keys. Includes and variables remain unresolved. Active state is separate.",r.draft.values.len(),r.skipped.len(),r.duplicate_keys.len()),draft:r.draft,source:None,saved_json:false}),
        Some("lua")=>{
            let worker::Response::Parsed(report)=worker::run(worker::Request::Parse{source:source.clone(),version})? else{return Err("Unexpected parser response".into());};
            let doc=LuaDocument::from_report(source,report,&catalogue)?;
            let mut message=format!("Imported {} static declarations; {} diagnostics. Effective values are not inferred by executing Lua.",doc.report.draft.values.len(),doc.report.diagnostics.len());
            if doc.report.opaque_effects{message.push_str(" Opaque code remains in the source; this is a partial projection.");}
            Ok(Imported{draft:doc.report.draft.clone(),source:Some(doc),message,saved_json:false})
        },_=>Err("Select draft JSON, legacy .conf or Lua .lua source".into()),
    }
}
pub(super) fn connect_import(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
    version: &gtk::DropDown,
    selection: &gtk::SingleSelection,
    panel: &gtk::Box,
) {
    let win = window.downgrade();
    let state = state.clone();
    let status = status.clone();
    let version = version.downgrade();
    let selection = selection.clone();
    let panel = panel.clone();
    button.connect_clicked(move|button|{
        let Some(win)=win.upgrade()else{return;};let Some(version)=version.upgrade()else{return;};
        let dialog=gtk::FileDialog::builder().title("Import draft JSON, legacy .conf or Lua source (never executed)").build();let state=state.clone();let status=status.clone();let selection=selection.clone();let panel=panel.clone();let button=button.downgrade();
        dialog.open(Some(&win.clone()),None::<&gtk::gio::Cancellable>,move|result|{
            let Ok(file)=result else{return;};let Some(path)=file.path()else{notice(&win,"Local file required","Only local regular files can be imported.");return;};
            let Some(button)=button.upgrade()else{return;};button.set_sensitive(false);status.set_text("Reading source in a bounded background operation…");
            let(generation,catalogue,selected)={let s=state.borrow();(s.generation,s.catalogue.clone(),s.draft.version.clone())};let win=win.downgrade();let button=button.downgrade();let version=version.downgrade();
            background(move||read_import(path,catalogue,selected),move|result|{
                if let Some(button)=button.upgrade(){button.set_sensitive(true);}let Some(win)=win.upgrade()else{return;};
                if state.borrow().generation!=generation{status.set_text("Draft changed while import was running; the result was discarded. Import again when ready.");return;}
                match result { Ok(imported) => {
                    let Some(version)=version.upgrade() else{return;};
                    if state.borrow().dirty_version(&imported.draft.version) {
                        let dialog=adw::MessageDialog::new(Some(&win),Some("Replace unsaved draft?"),Some("The imported file would replace unsaved values for this snapshot. Save the existing draft first, or explicitly discard it."));
                        dialog.add_response("cancel","Cancel");dialog.add_response("replace","Discard and import");dialog.set_default_response(Some("cancel"));dialog.set_close_response("cancel");dialog.set_response_appearance("replace",adw::ResponseAppearance::Destructive);
                        let imported=std::cell::RefCell::new(Some(imported));
                        dialog.connect_response(None,move|_,response|{if response=="replace"{if let Some(imported)=imported.borrow_mut().take(){if state.borrow().generation==generation{apply_import(imported,&state,&status,&version,&selection,&panel);}}}});dialog.present();
                    }else{apply_import(imported,&state,&status,&version,&selection,&panel);}
                },Err(e)=>notice(&win,"Import rejected",&e)}
            });
        });
    });
}
pub(super) fn connect_live(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
    selection: &gtk::SingleSelection,
    panel: &gtk::Box,
) {
    let win = window.downgrade();
    let state = state.clone();
    let status = status.clone();
    let selection = selection.clone();
    let panel = panel.clone();
    button.connect_clicked(move|button|{
        let catalogue=state.borrow().catalogue.clone();button.set_sensitive(false);status.set_text("Reading the selected compositor's version and descriptions…");let button=button.downgrade();let win=win.clone();let state=state.clone();let status=status.clone();let selection=selection.clone();let panel=panel.clone();
        background(move||hyprland_editor_discovery::current_session_snapshot(&catalogue),move|result|{
            if let Some(button)=button.upgrade(){button.set_sensitive(true);}let Some(win)=win.upgrade()else{return;};
            match result{Ok(snapshot)=>{
                let normalized=snapshot.values.values().filter(|o|o.current.is_some()).count();let message=format!("Read {} matched settings ({normalized} normalized) from PID {} in {} ms. This snapshot may become stale; refresh to update. {}",snapshot.values.len(),snapshot.identity.pid,snapshot.duration_ms,if snapshot.version.is_none(){"The compositor commit has no audited pack; values remain unmapped."}else{"Draft values were not changed."});
                let mut s=state.borrow_mut();s.active=Some(snapshot);let spec=s.visible.get(selection.selected()as usize).cloned();drop(s);if let Some(spec)=spec{detail(&state,&panel,&spec,&status);}status.set_text(&message);
            },Err(e)=>{state.borrow_mut().active=None;let spec=state.borrow().visible.get(selection.selected()as usize).cloned();if let Some(spec)=spec{detail(&state,&panel,&spec,&status);}status.set_text("Live observation unavailable; active values remain unknown.");notice(&win,"Cannot read this compositor session",&e);}}
        });
    });
}
fn review_copy(window: &adw::ApplicationWindow, text: String) {
    let dialog = adw::Window::builder()
        .title("Review edited Lua source copy")
        .transient_for(window)
        .modal(true)
        .default_width(800)
        .default_height(620)
        .build();
    let b = gtk::Box::new(gtk::Orientation::Vertical, 12);
    b.set_margin_start(16);
    b.set_margin_end(16);
    b.set_margin_top(16);
    b.set_margin_bottom(16);
    b.append(&super::label("Only changed literal expressions were replaced. Other statements and comments are preserved as text and have not been executed. Copying does not publish or apply this configuration."));
    let view = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    view.buffer().set_text(&text);
    b.append(
        &gtk::ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .build(),
    );
    let copy = gtk::Button::with_label("Copy source to clipboard");
    b.append(&copy);
    let win = dialog.downgrade();
    copy.connect_clicked(move |_| {
        if let Some(w) = win.upgrade() {
            w.clipboard().set_text(&text);
        }
    });
    dialog.set_content(Some(&b));
    dialog.present();
}
pub(super) fn connect_source_copy(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
) {
    let win = window.downgrade();
    let state = state.clone();
    button.connect_clicked(move|button|{
        let Some(window)=win.upgrade()else{return;};let s=state.borrow();let Some(doc)=s.sources.get(&s.draft.version).cloned()else{drop(s);notice(&window,"Lua source required","Import a Lua source file first. JSON drafts and legacy imports have no Lua source spans to preserve.");return;};
        let draft=s.draft.clone();let generation=s.generation;drop(s);let state=state.clone();let win=window.downgrade();let button=button.downgrade();if let Some(button)=button.upgrade(){button.set_sensitive(false);}
        background(move||worker::run(worker::Request::Copy{source:doc.source,report:doc.report,draft}),move|result|{
            if let Some(button)=button.upgrade(){button.set_sensitive(true);}let Some(win)=win.upgrade()else{return;};if state.borrow().generation!=generation{notice(&win,"Draft changed","The draft changed while the source copy was prepared. Review again to use the current edits.");return;}
            match result{Ok(worker::Response::Copy(source))=>review_copy(&win,source),Ok(_)=>notice(&win,"Invalid parser response","The worker did not return a source copy."),Err(e)=>notice(&win,"Cannot prepare source copy",&e)}
        });
    });
}

fn apply_import(
    imported: Imported,
    state: &Shared,
    status: &gtk::Label,
    version: &gtk::DropDown,
    selection: &gtk::SingleSelection,
    panel: &gtk::Box,
) {
    version.set_selected(if imported.draft.version == BASELINE {
        0
    } else {
        1
    });
    let mut s = state.borrow_mut();
    if let Some(doc) = imported.source {
        s.sources.insert(imported.draft.version.clone(), doc);
    } else {
        s.sources.remove(&imported.draft.version);
    }
    if imported.saved_json {
        s.saved.insert(
            imported.draft.version.clone(),
            imported.draft.values.clone(),
        );
    }
    s.draft = imported.draft;
    s.undo.clear();
    s.generation += 1;
    let spec = s.visible.get(selection.selected() as usize).cloned();
    drop(s);
    if let Some(spec) = spec {
        detail(state, panel, &spec, status);
    }
    status.set_text(&imported.message);
}
pub(super) fn connect_close(window: &adw::ApplicationWindow, state: &Shared) {
    let state = state.clone();
    window.connect_close_request(move|window|{
        if !state.borrow().dirty(){return gtk::glib::Propagation::Proceed;}
        let dialog=adw::MessageDialog::new(Some(window),Some("Close with unsaved drafts?"),Some("Save the changed drafts before closing to keep your edits. Closing does not modify the running desktop or your original configuration files."));
        dialog.add_response("cancel","Keep editing");dialog.add_response("discard","Discard unsaved drafts");dialog.set_default_response(Some("cancel"));dialog.set_close_response("cancel");dialog.set_response_appearance("discard",adw::ResponseAppearance::Destructive);
        let win=window.downgrade();dialog.connect_response(None,move|_,response|{if response=="discard"{if let Some(win)=win.upgrade(){win.destroy();}}});dialog.present();gtk::glib::Propagation::Stop
    });
}

pub(super) fn connect_diagnostics(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
) {
    let win = window.downgrade();
    let state = state.clone();
    button.connect_clicked(move|_|{
        let Some(win)=win.upgrade()else{return;};let s=state.borrow();let Some(doc)=s.sources.get(&s.draft.version)else{drop(s);notice(&win,"No Lua import report","Import Lua source first to inspect its line diagnostics.");return;};
        let text=if doc.report.diagnostics.is_empty(){"No unresolved syntax was found in the supported static subset. This is not a runtime validation certificate.".into()}else{doc.report.diagnostics.iter().map(|d|format!("Line {}: {}",d.line,d.message)).collect::<Vec<_>>().join("\n")};
        drop(s);let dialog=adw::Window::builder().title("Static Lua import diagnostics").transient_for(&win).default_width(760).default_height(500).build();let view=gtk::TextView::builder().editable(false).wrap_mode(gtk::WrapMode::WordChar).left_margin(16).right_margin(16).top_margin(16).bottom_margin(16).build();view.buffer().set_text(&text);dialog.set_content(Some(&gtk::ScrolledWindow::builder().child(&view).build()));dialog.present();
    });
}
