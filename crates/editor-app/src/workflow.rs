//! Background workflows keep IPC and parsing off the GTK main thread.
use super::{detail, notice, worker, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    import,
    lua::LuaDocument,
    source::{Reattachment, SourceIdentity},
    storage,
    workspace::{self, SourceLanguage, Workspace},
    Draft, Setting, BASELINE,
};
use std::{path::PathBuf, sync::mpsc, time::Duration};
pub(super) fn background<T: Send + 'static>(
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
    origin: Option<SourceIdentity>,
    message: String,
    saved_json: bool,
    monitors: Vec<hyprland_editor_core::monitor::Rule>,
}
fn read_import(
    path: PathBuf,
    catalogue: Vec<Setting>,
    version: String,
) -> Result<Imported, String> {
    if path.extension().is_some_and(|extension| extension == "hye") {
        let workspace = workspace::read(&path, &catalogue)?;
        if workspace.sources.len() > 1 {
            return Err("Multi-file workspace reopening needs the source-graph editor; this revision will not silently drop attached files".into());
        }
        let (source, origin) = if let Some(root) = workspace.root() {
            if root.language != SourceLanguage::Lua {
                return Err("Legacy source snapshots need the lossless legacy editor; no source was rebound".into());
            }
            let doc = parse_source(
                root.content.clone(),
                workspace.draft.version.clone(),
                &catalogue,
            )?;
            (Some(doc), Some(root.origin.clone()))
        } else {
            (None, None)
        };
        let diagnostics = source
            .as_ref()
            .map_or(0, |doc| doc.report.diagnostics.len());
        return Ok(Imported { draft:workspace.draft,source,origin,saved_json:true,monitors:workspace.monitors,
            message:format!("Saved draft reopened with {diagnostics} source diagnostics. Source snapshots were reparsed; recorded original paths were not opened. Desktop state is separate.") });
    }
    let captured = if path
        .extension()
        .is_some_and(|extension| extension == "lua" || extension == "conf")
    {
        hyprland_editor_core::source_path::read_selected(&path)?
    } else {
        storage::read_source(&path)?
    };
    match path.extension().and_then(|s|s.to_str()) {
        Some("json")=>Draft::from_json(&captured.content,&catalogue).map(|draft|Imported{draft,source:None,origin:None,monitors:vec![],message:"Validated value-only draft loaded".into(),saved_json:true}),
        Some("conf")=>import::legacy(&captured.content,&catalogue).map(|r|Imported{message:format!("Imported {} literal overrides; {} lines skipped; {} repeated keys. Includes and variables remain unresolved. Active state is separate.",r.draft.values.len(),r.skipped.len(),r.duplicate_keys.len()),draft:r.draft,source:None,origin:None,monitors:vec![],saved_json:false}),
        Some("lua")=>{
            let doc=parse_source(captured.content,version,&catalogue)?;
            let mut message=format!("Imported {} scalar declarations and {} monitor rules; {} diagnostics. Effective values are not inferred by executing Lua.",doc.report.draft.values.len(),doc.report.monitors.len(),doc.report.diagnostics.len());
            if doc.report.opaque_effects{message.push_str(" Opaque code remains in the source; this is a partial projection.");}
            Ok(Imported{draft:doc.report.draft.clone(),monitors:doc.report.monitors.clone(),source:Some(doc),origin:Some(captured.identity),message,saved_json:false})
        },_=>Err("Select saved .hye workspace, draft JSON, legacy .conf or Lua .lua source".into()),
    }
}
pub(super) fn parse_source(
    source: String,
    version: String,
    catalogue: &[Setting],
) -> Result<LuaDocument, String> {
    let worker::Response::Parsed(report) = worker::run(worker::Request::Parse {
        source: source.clone(),
        version,
    })?
    else {
        return Err("Unexpected parser response".into());
    };
    LuaDocument::from_report(source, report, catalogue)
}
/// Headless recovery/inspection entry; uses the same restricted parser as the GUI.
pub(super) fn check_workspace(path: PathBuf) -> Result<serde_json::Value, String> {
    if path.extension().is_none_or(|extension| extension != "hye") {
        return Err("--check-workspace requires a saved .hye workspace".into());
    }
    let imported = read_import(path, hyprland_editor_core::catalogue(), BASELINE.into())?;
    Ok(
        serde_json::json!({"scope":"draft-and-static-source-only", "version":imported.draft.version,
        "draft_values":imported.draft.values.len(),"monitor_rules":imported.monitors.len(), "source_reparsed":imported.source.is_some(),
        "source_sha256":imported.source.as_ref().map(|s|s.report.source_sha256.as_str()),
        "opaque_effects":imported.source.as_ref().is_some_and(|s|s.report.opaque_effects),
        "source_diagnostics":imported.source.as_ref().map_or(0,|s|s.report.diagnostics.len()),
        "runtime_verified":false,"original_paths_opened":false}),
    )
}
pub(super) fn source_workspace(
    path: PathBuf,
) -> Result<(Draft, LuaDocument, SourceIdentity), String> {
    if path.extension().is_none_or(|extension| extension != "hye") {
        return Err("Comparison requires a saved .hye workspace".into());
    }
    let imported = read_import(path, hyprland_editor_core::catalogue(), BASELINE.into())?;
    Ok((
        imported.draft,
        imported
            .source
            .ok_or("Workspace has no Lua source snapshot")?,
        imported.origin.ok_or("Workspace has no source identity")?,
    ))
}

pub(super) fn connect_save(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) {
    let root = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")));
    connect_save_at(button, window, state, status, root);
}
fn connect_save_at(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
    root: Option<PathBuf>,
) {
    let win = window.downgrade();
    let state = state.clone();
    let status = status.clone();
    button.connect_clicked(move|button| {
        let Some(win)=win.upgrade() else{return;};
        let (generation,catalogue,workspace)={
            let s=state.borrow();let mut snapshot=Workspace::new(s.draft.clone()).with_monitors(s.monitors.get(&s.draft.version).cloned().unwrap_or_default());
            if let Some(doc)=s.sources.get(&s.draft.version) {
                let Some(origin)=s.source_origins.get(&s.draft.version) else {
                    notice(&win,"Source identity missing","Reimport the original source before saving its context.");return;
                };
                snapshot=snapshot.with_root(doc.source.clone(),origin.clone());
            }
            (s.generation,s.catalogue.clone(),snapshot)
        };
        let saved_monitors=workspace.monitors.clone();let saved_values=workspace.draft.values.clone();let saved_version=workspace.draft.version.clone();
        let saved_origin=workspace.root().map(|root|root.origin.clone());
        let root=root.clone();
        button.set_sensitive(false);status.set_text("Saving a private draft and source snapshot…");
        let button=button.downgrade();let state=state.clone();let status=status.clone();let win=win.downgrade();
        background(move||{
            let root=root.ok_or("Cannot find a user state directory")?;
            if !root.is_absolute(){return Err("State directory must be absolute".into());}
            std::fs::create_dir_all(&root).map_err(|e|e.to_string())?;
            workspace::save(&root,&workspace,&catalogue)
        },move|result|{
            if let Some(button)=button.upgrade(){button.set_sensitive(true);}
            let Some(win)=win.upgrade()else{return;};
            match result {
                Ok(path)=>{
                    let mut s=state.borrow_mut();
                    if s.generation==generation {
                        s.saved.insert(saved_version.clone(),saved_values);s.saved_monitors.insert(saved_version.clone(),saved_monitors);s.saved_source_origins.insert(saved_version,saved_origin);
                        drop(s);status.set_text(&format!("Private draft and source snapshot saved: {}",path.display()));
                    } else {drop(s);status.set_text(&format!("An earlier draft revision was saved to {}. Current edits remain unsaved.",path.display()));}
                },Err(e)=>{super::summary(&state,&status);notice(&win,"Could not save draft",&e);}
            }
        });
    });
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
        let dialog=gtk::FileDialog::builder().title("Open saved draft, JSON, legacy .conf or Lua source").build();let state=state.clone();let status=status.clone();let selection=selection.clone();let panel=panel.clone();let button=button.downgrade();
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
        if s.monitors.get(&s.draft.version).map(Vec::as_slice).unwrap_or(&[]) != doc.report.monitors.as_slice() { drop(s);notice(&window,"Monitor source edits pending","Monitor drafts can be saved and reviewed as Lua. Updating their original source spans is not implemented yet; a scalar source copy would omit those edits.");return; }
        let draft=s.draft.clone();let generation=s.generation;drop(s);let state=state.clone();let win=window.downgrade();let button=button.downgrade();if let Some(button)=button.upgrade(){button.set_sensitive(false);}
        background(move||worker::run(worker::Request::Copy{source:doc.source.clone(),report:doc.report.clone(),draft}),move|result|{
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
    if imported.saved_json {
        s.saved_source_origins
            .insert(imported.draft.version.clone(), imported.origin.clone());
    }
    if let Some(origin) = imported.origin {
        s.source_origins
            .insert(imported.draft.version.clone(), origin);
    } else {
        s.source_origins.remove(&imported.draft.version);
    }
    if let Some(doc) = imported.source {
        s.sources
            .insert(imported.draft.version.clone(), std::sync::Arc::new(doc));
    } else {
        s.sources.remove(&imported.draft.version);
    }
    if imported.saved_json {
        s.saved.insert(
            imported.draft.version.clone(),
            imported.draft.values.clone(),
        );
    }
    if imported.saved_json {
        s.saved_monitors
            .insert(imported.draft.version.clone(), imported.monitors.clone());
    }
    s.monitors
        .insert(imported.draft.version.clone(), imported.monitors);
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

pub(super) fn connect_reattach(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) {
    let win = window.downgrade();
    let state = state.clone();
    let status = status.clone();
    button.connect_clicked(move|button|{
        let Some(win)=win.upgrade()else{return;};
        let (origin,version,generation,catalogue)={
            let s=state.borrow();let Some(origin)=s.source_origins.get(&s.draft.version).cloned()else {
                notice(&win,"Source snapshot required","Open a saved source-backed draft or import Lua before verifying its source.");return;
            };(origin,s.draft.version.clone(),s.generation,s.catalogue.clone())
        };
        let dialog=gtk::FileDialog::builder().title("Select the original source or an identical copy").build();
        let button=button.downgrade();let state=state.clone();let status=status.clone();
        dialog.open(Some(&win.clone()),None::<&gtk::gio::Cancellable>,move|result|{
            let Ok(file)=result else{return;};let Some(path)=file.path()else{return;};
            let Some(button)=button.upgrade()else{return;};button.set_sensitive(false);
            let button=button.downgrade();let win=win.downgrade();
            background(move||{
                let captured=hyprland_editor_core::source_path::read_selected(&path)?;
                let comparison=origin.compare_selected(&captured)?;
                let doc=parse_source(captured.content,version,&catalogue)?;
                Ok((comparison,doc,captured.identity))
            },move|result|{
                if let Some(button)=button.upgrade(){button.set_sensitive(true);}
                let Some(win)=win.upgrade()else{return;};
                if state.borrow().generation!=generation {notice(&win,"Draft changed","Select the source again to verify the current draft.");return;}
                let (comparison,doc,origin)=match result {Ok(value)=>value,Err(error)=>{notice(&win,"Source not attached",&error);return;}};
                let old_path=state.borrow().source_origins.get(&doc.report.draft.version).map(SourceIdentity::display_path).unwrap_or_default();
                let resolved=origin.resolved_path.as_deref().map(hyprland_editor_core::source::display_path_bytes).unwrap_or_else(||"Not recorded".into());
                let review=format!("Saved path: {old_path}\nSelected path: {}\nResolved target: {resolved}\nRecorded symlinks: {}\nRevision SHA-256: {}\nSelected device/inode: {}/{}\nOwner UID: {} · mode {:o}\n\nThe bytes are identical, but source identity or metadata changed. Attaching updates recorded evidence only. Configuration files and desktop settings are not changed.",origin.display_path(),origin.links.len(),origin.sha256,origin.device,origin.inode,origin.owner,origin.mode & 0o777);
                let error_win=win.downgrade();
                let apply=move||{
                    let Some(win)=error_win.upgrade()else{return;};
                    if state.borrow().generation!=generation{notice(&win,"Draft changed","Select and review the current source again.");return;}
                    let mut s=state.borrow_mut();let draft=s.draft.clone();
                    if let Err(error)=s.replace_source(std::sync::Arc::new(doc),origin,draft) {drop(s);notice(&win,"Source not attached",&error);return;}
                    drop(s);super::summary(&state,&status);
                    status.set_tooltip_text(Some("Source bytes verified through an explicitly selected file. No configuration was written or applied."));
                };
                if comparison==Reattachment::SameRevision {apply();} else {
                    // Relocation changes identity evidence, even when bytes are identical.
                    let dialog=adw::MessageDialog::new(Some(&win),Some("Attach this identical source copy?"),Some(&review));
                    dialog.add_response("cancel","Cancel");dialog.add_response("attach","Attach Source");dialog.set_default_response(Some("cancel"));dialog.set_close_response("cancel");
                    let apply=std::cell::RefCell::new(Some(apply));
                    dialog.connect_response(None,move|_,response|{if response=="attach"{if let Some(apply)=apply.borrow_mut().take(){apply();}}});dialog.present();
                }
            });
        });
    });
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

#[cfg(test)]
pub(super) fn test_source_persistence(window: &adw::ApplicationWindow) {
    use std::time::{Instant, SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "he-ui-source-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let original = root.join("synthetic.lua");
    std::fs::write(
        &original,
        "-- private fixture\nhl.config{general={border_size=2}}\n",
    )
    .unwrap();
    let captured = storage::read_source(&original).unwrap();
    let catalogue = hyprland_editor_core::catalogue();
    // Fixture syntax is trusted; real-process reparsing is covered in tests/workspace.rs.
    let report = hyprland_editor_core::lua::parse(&captured.content, BASELINE, &catalogue).unwrap();
    let doc = LuaDocument::from_report(captured.content, report, &catalogue).unwrap();
    let mut draft = doc.report.draft.clone();
    draft.values.insert(
        "general:border_size".into(),
        hyprland_editor_core::Value::Int(5),
    );
    let state = super::initial_state();
    let status = gtk::Label::new(None);
    let button = gtk::Button::new();
    let version = gtk::DropDown::from_strings(&[BASELINE, "audited-main"]);
    let selection = gtk::SingleSelection::new(Some(gtk::StringList::new(&[])));
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    apply_import(
        Imported {
            draft,
            source: Some(doc),
            origin: Some(captured.identity.clone()),
            saved_json: false,
            monitors: vec![],
            message: "Fixture imported".into(),
        },
        &state,
        &status,
        &version,
        &selection,
        &panel,
    );
    assert!(state.borrow().dirty());
    connect_save_at(&button, window, &state, &status, Some(root.clone()));
    let wait = || {
        let start = Instant::now();
        let context = gtk::glib::MainContext::default();
        while !button.is_sensitive() {
            while context.pending() {
                context.iteration(false);
            }
            assert!(
                start.elapsed() < Duration::from_secs(4),
                "private save did not complete"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    let monitor = hyprland_editor_core::monitor::Rule::new("saved-monitor");
    state
        .borrow_mut()
        .replace_monitors(vec![monitor.clone()])
        .unwrap();
    button.emit_clicked();
    wait();
    assert!(!state.borrow().dirty());
    assert!(status
        .text()
        .starts_with("Private draft and source snapshot saved"));
    let files: Vec<_> = std::fs::read_dir(root.join("hyprland-editor"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    let saved = workspace::read(&files[0], &catalogue).unwrap();
    assert_eq!(saved.root().unwrap().origin, captured.identity);
    assert_eq!(saved.monitors, vec![monitor]);
    assert_eq!(
        saved.draft.values["general:border_size"],
        hyprland_editor_core::Value::Int(5)
    );
    {
        let mut s = state.borrow_mut();
        s.source_origins.get_mut(BASELINE).unwrap().inode += 1;
    }
    assert!(
        state.borrow().dirty(),
        "a different source identity must need saving"
    );
    {
        let mut s = state.borrow_mut();
        s.source_origins.insert(BASELINE.into(), captured.identity);
        s.remember("general:border_size").unwrap();
        s.draft.values.insert(
            "general:border_size".into(),
            hyprland_editor_core::Value::Int(7),
        );
    }
    button.emit_clicked();
    {
        let mut s = state.borrow_mut();
        s.remember("general:border_size").unwrap();
        s.draft.values.insert(
            "general:border_size".into(),
            hyprland_editor_core::Value::Int(9),
        );
    }
    wait();
    assert!(status
        .text()
        .starts_with("An earlier draft revision was saved"));
    assert!(
        state.borrow().dirty(),
        "saving an earlier generation cannot clear newer edits"
    );
    assert_eq!(
        state.borrow().saved[BASELINE]["general:border_size"],
        hyprland_editor_core::Value::Int(5)
    );
    std::fs::remove_dir_all(root).unwrap();
}
