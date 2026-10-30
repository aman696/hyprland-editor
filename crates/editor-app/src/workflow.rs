//! Background workflows keep IPC and parsing off the GTK main thread.
use super::{detail, notice, worker, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
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
    migration: Option<super::graph_migration::Summary>,
    graph: Option<std::sync::Arc<super::source_graph::Saved>>,
    draft: Draft,
    source: Option<LuaDocument>,
    legacy: Option<super::legacy_source::Document>,
    origin: Option<SourceIdentity>,
    message: String,
    saved_json: bool,
    monitors: Vec<hyprland_editor_core::monitor::Rule>,
    devices: Vec<hyprland_editor_core::device::Spec>,
    bindings: hyprland_editor_core::binding::Draft,
    workspace_rules: Vec<hyprland_editor_core::workspace_rule::Rule>,
    rules: Vec<hyprland_editor_core::rule::Rule>,
    animations: hyprland_editor_core::animation::Draft,
    gestures: hyprland_editor_core::gesture::Draft,
}
fn read_import(
    path: PathBuf,
    catalogue: Vec<Setting>,
    version: String,
) -> Result<Imported, String> {
    read_import_for_review(path, catalogue, version, false)
}
fn read_import_for_review(
    path: PathBuf,
    catalogue: Vec<Setting>,
    version: String,
    allow_migration: bool,
) -> Result<Imported, String> {
    if path.extension().is_some_and(|extension| extension == "hye") {
        let (text, _) = storage::read_bounded(&path, workspace::MAX_WORKSPACE_BYTES)?;
        let (workspace, migration, prepared_graph) = match Workspace::from_json(&text, &catalogue) {
            Ok(workspace) => (workspace, None, None),
            Err(error) if !allow_migration => return Err(error),
            Err(_) => {
                let prepared = super::graph_migration::prepare(&text, &catalogue)?;
                (
                    prepared.workspace,
                    Some(prepared.summary),
                    Some(prepared.graph),
                )
            }
        };
        let graph = if let Some(graph) = prepared_graph {
            Some(graph)
        } else if let Some(graph) = &workspace.source_graph {
            let bundle = hyprland_editor_core::workspace_graph::Bundle {
                graph: graph.clone(),
                sources: std::sync::Arc::new(workspace.sources.clone()),
            };
            super::source_graph::restore(&bundle, &workspace.draft.version)?;
            Some(std::sync::Arc::new(super::source_graph::Saved::new(
                bundle,
                &workspace.draft.version,
            )?))
        } else {
            None
        };
        if workspace.sources.len() > 1 && graph.is_none() {
            return Err("Multi-file workspace reopening needs the source-graph editor; this revision will not silently drop attached files".into());
        }
        let (source, legacy, origin) = if let Some(root) = workspace.root() {
            match root.language {
                SourceLanguage::Lua => {
                    let doc = parse_source(
                        root.content.clone(),
                        workspace.draft.version.clone(),
                        &catalogue,
                    )?;
                    (Some(doc), None, Some(root.origin.clone()))
                }
                SourceLanguage::Legacy => {
                    if workspace.draft.version != BASELINE || graph.is_some() {
                        return Err("Legacy source snapshots require the baseline pack and cannot be a Lua source graph.".into());
                    }
                    let doc =
                        super::legacy_source::Document::new(root.content.clone(), &catalogue)?;
                    (None, Some(doc), Some(root.origin.clone()))
                }
            }
        } else {
            (None, None, None)
        };
        let diagnostics = source
            .as_ref()
            .map_or(0, |doc| doc.report.diagnostics.len());
        let saved_json = migration.is_none();
        return Ok(Imported { migration,graph,draft:workspace.draft,source,legacy,origin,saved_json,monitors:workspace.monitors,devices:workspace.devices,bindings:workspace.bindings,workspace_rules:workspace.workspace_rules,rules:workspace.rules,animations:workspace.animations,gestures:workspace.gestures,
            message:format!("{} with {diagnostics} source diagnostics. Source snapshots were reparsed; recorded original paths were not opened. Desktop state is separate.", if saved_json { "Saved draft reopened" } else { "Rebuilt copy loaded; save it as a new private workspace" }) });
    }
    let captured = if path
        .extension()
        .is_some_and(|extension| extension == "lua" || extension == "conf")
    {
        super::capture_worker::read_selected(&path)?
    } else {
        storage::read_source(&path)?
    };
    match path.extension().and_then(|s|s.to_str()) {
        Some("json")=>Draft::from_json(&captured.content,&catalogue).map(|draft|Imported{migration:None,graph:None,draft,source:None,legacy:None,origin:None,monitors:vec![],devices:vec![],bindings:Default::default(),workspace_rules:vec![],rules:vec![],animations:Default::default(),gestures:Default::default(),message:"Validated value-only draft loaded".into(),saved_json:true}),
        Some("conf")=>super::legacy_source::Document::new(captured.content,&catalogue).map(|doc|Imported{migration:None,graph:None,message:format!("Retained legacy source and {} literal overrides; {} lines unresolved; {} repeated keys. Includes and variables are not evaluated. Review Source Copy exports scalar edits while preserving other directives.",doc.report.draft.values.len(),doc.report.skipped.len(),doc.report.duplicate_keys.len()),draft:doc.report.draft.clone(),source:None,legacy:Some(doc),origin:Some(captured.identity),monitors:vec![],devices:vec![],bindings:Default::default(),workspace_rules:vec![],rules:vec![],animations:Default::default(),gestures:Default::default(),saved_json:false}),
        Some("lua")=>{
            let doc=parse_source(captured.content,version,&catalogue)?;
            let mut message=format!("Imported {} scalar declarations, {} monitor rules and {} device rules; {} diagnostics. Effective values are not inferred by executing Lua.",doc.report.draft.values.len(),doc.report.monitors.len(),doc.report.devices.len(),doc.report.diagnostics.len());
            if doc.report.opaque_effects{message.push_str(" Opaque code remains in the source; this is a partial projection.");}
            Ok(Imported{migration:None,graph:None,draft:doc.report.draft.clone(),monitors:doc.report.monitors.clone(),devices:doc.report.devices.clone(),bindings:doc.report.bindings.clone(),workspace_rules:doc.report.workspace_rules.clone(),rules:doc.report.rules.clone(),animations:doc.report.animations.clone(),gestures:doc.report.gestures.clone(),source:Some(doc),legacy:None,origin:Some(captured.identity),message,saved_json:false})
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
    LuaDocument::from_report(source, *report, catalogue)
}
/// Headless recovery/inspection entry; uses the same restricted parser as the GUI.
pub(super) fn check_workspace(path: PathBuf) -> Result<serde_json::Value, String> {
    if path.extension().is_none_or(|extension| extension != "hye") {
        return Err("--check-workspace requires a saved .hye workspace".into());
    }
    let imported = read_import(path, hyprland_editor_core::catalogue(), BASELINE.into())?;
    Ok(
        serde_json::json!({"scope":"draft-and-static-source-only", "version":imported.draft.version,
        "draft_values":imported.draft.values.len(),"monitor_rules":imported.monitors.len(),"device_rules":imported.devices.len(),"binding_rules":imported.bindings.binds.len(),"workspace_rules":imported.workspace_rules.len(),"match_effect_rules":imported.rules.len(),"animation_declarations":imported.animations.declarations.len(),"gesture_declarations":imported.gestures.declarations.len(), "source_reparsed":imported.source.is_some(),
        "source_sha256":imported.source.as_ref().map(|s|s.report.source_sha256.as_str()),
        "opaque_effects":imported.source.as_ref().is_some_and(|s|s.report.opaque_effects),
        "source_diagnostics":imported.source.as_ref().map_or(0,|s|s.report.diagnostics.len()),
        "legacy_source_retained":imported.legacy.is_some(),
        "source_graph_replayed":imported.graph.is_some(),
        "source_graph_documents":imported.graph.as_ref().map_or(0,|g|g.bundle.sources.len()),
        "source_graph_bindings":imported.graph.as_ref().map_or(0,|g|g.bundle.graph.bindings.len()),
        "source_graph_revisions":imported.graph.as_ref().map_or(0,|g|g.bundle.graph.revisions.len()),
        "source_graph_private_documents":imported.graph.as_ref().map_or(0,|g|g.bundle.graph.private.sources.len()),
        "source_graph_private_bindings":imported.graph.as_ref().map_or(0,|g|g.bundle.graph.private.bindings.len()),
        "source_graph_archived_private_documents":imported.graph.as_ref().map_or(0,|g|g.bundle.graph.private.archived.len()),
        "folder_access_restored":false,
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
    if imported.graph.is_some() {
        return Err("Graph reconciliation must account for every saved file; single-root comparison is unavailable for this workspace.".into());
    }
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
pub(super) fn captured_workspace(s: &super::State) -> Result<Workspace, String> {
    let version = &s.draft.version;
    let mut snapshot = Workspace::new(s.draft.clone())
        .with_monitors(s.monitors.get(version).cloned().unwrap_or_default())
        .with_devices(s.devices.get(version).cloned().unwrap_or_default())
        .with_bindings(s.bindings.get(version).cloned().unwrap_or_default())
        .with_workspace_rules(s.workspace_rules.get(version).cloned().unwrap_or_default())
        .with_rules(s.rules.get(version).cloned().unwrap_or_default())
        .with_animations(s.animations.get(version).cloned().unwrap_or_default())
        .with_gestures(s.gestures.get(version).cloned().unwrap_or_default());
    if let Some(graph) = s.source_graphs.get(version) {
        snapshot = snapshot.with_graph(&graph.bundle);
    } else if let Some(doc) = s.sources.get(version) {
        let origin = s
            .source_origins
            .get(version)
            .ok_or("Reimport the original source before saving its context")?;
        snapshot = snapshot.with_root(doc.source.clone(), origin.clone());
    } else if let Some(doc) = s.legacy_sources.get(version) {
        let origin = s
            .source_origins
            .get(version)
            .ok_or("Legacy source identity missing")?;
        snapshot.root_source = Some("root".into());
        snapshot.sources.push(workspace::SourceSnapshot {
            id: "root".into(),
            language: SourceLanguage::Legacy,
            content: doc.content.clone(),
            origin: origin.clone(),
        });
    }
    Ok(snapshot)
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
    button.connect_clicked(move |button| {
        let Some(win) = win.upgrade() else {return;};
        let pending = match super::project_save::Snapshot::capture(&state.borrow()) {
            Ok(snapshot) => snapshot,
            Err(error) => {notice(&win,"Source identity missing",&error);return;}
        };
        let catalogue = state.borrow().catalogue.clone();
        let root = root.clone();
        button.set_sensitive(false);
        status.set_text("Saving a private draft and source snapshot…");
        let button = button.downgrade();
        let state = state.clone();
        let status = status.clone();
        let win = win.downgrade();
        background(move || {
            let root = root.ok_or("Cannot find a user state directory")?;
            if !root.is_absolute() {return Err("State directory must be absolute".into());}
            std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
            let path = workspace::save(&root,&pending.workspace,&catalogue)?;
            Ok((path,pending))
        },move |result| {
            if let Some(button) = button.upgrade() {button.set_sensitive(true);}
            let Some(win) = win.upgrade() else {return;};
            match result {
                Ok((path,pending)) => {
                    if pending.mark_saved(&state) {
                        status.set_text(&format!("Private draft and source snapshot saved: {}",path.display()));
                    } else {
                        status.set_text(&format!("An earlier draft revision was saved to {}. Current edits remain unsaved.",path.display()));
                    }
                }
                Err(error) => {super::summary(&state,&status);notice(&win,"Could not save draft",&error);}
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
            background(move||read_import_for_review(path,catalogue,selected,true),move|result|{
                if let Some(button)=button.upgrade(){button.set_sensitive(true);}let Some(win)=win.upgrade()else{return;};
                if state.borrow().generation!=generation{status.set_text("Draft changed while import was running; the result was discarded. Import again when ready.");return;}
                match result { Ok(imported) => {
                    let Some(version)=version.upgrade() else{return;};
                    review_import(imported,&win,&state,&status,&version,&selection,&panel,generation);
                },Err(e)=>notice(&win,"Import rejected",&e)}
            });
        });
    });
}
#[allow(clippy::too_many_arguments)]
fn confirm_import(
    imported: Imported,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
    version: &gtk::DropDown,
    selection: &gtk::SingleSelection,
    panel: &gtk::Box,
    generation: u64,
) {
    if state.borrow().generation != generation {
        return;
    }
    if !state.borrow().dirty_version(&imported.draft.version) {
        apply_import(imported, state, status, version, selection, panel);
        return;
    }
    let dialog = adw::MessageDialog::new(Some(window), Some("Replace unsaved draft?"), Some("The imported file would replace unsaved values for this snapshot. Save the existing draft first, or explicitly discard it."));
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("replace", "Discard and import");
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("replace", adw::ResponseAppearance::Destructive);
    let imported = std::cell::RefCell::new(Some(imported));
    let (state, status, version, selection, panel) = (
        state.clone(),
        status.clone(),
        version.downgrade(),
        selection.clone(),
        panel.clone(),
    );
    dialog.connect_response(None, move |_, response| {
        if response == "replace" && state.borrow().generation != generation {
            status.set_text("Draft changed during review; import was discarded. Open the saved file again when ready.");
            return;
        }
        if response == "replace" && state.borrow().generation == generation {
            if let (Some(imported), Some(version)) =
                (imported.borrow_mut().take(), version.upgrade())
            {
                apply_import(imported, &state, &status, &version, &selection, &panel);
            }
        }
    });
    dialog.present();
}
#[allow(clippy::too_many_arguments)]
fn review_import(
    imported: Imported,
    window: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
    version: &gtk::DropDown,
    selection: &gtk::SingleSelection,
    panel: &gtk::Box,
    generation: u64,
) {
    let Some(summary) = &imported.migration else {
        confirm_import(
            imported, window, state, status, version, selection, panel, generation,
        );
        return;
    };
    let mut body = format!("This workspace was saved by an earlier build. Its saved sources have been checked again.\n\nCaptured files: {}\nDependency inputs: {}\nPrivate source edits: {}\nAdditional captured files: {}\nAdditional dependency inputs: {}\nArchived files: {}\n\nYour source bytes, edits and settings are preserved. Original files were not opened. Folder access stays closed.\n\nKeep this copy, then save a new private workspace. The old saved file stays unchanged.",summary.captured_documents,summary.resolution_inputs,summary.private_revisions,summary.private_documents,summary.private_resolution_inputs,summary.archived_documents);
    if summary.compiler_observation_renewed {
        body.push_str("\n\nThe current compiler was checked again. This can change how numeric expressions are interpreted in source analysis.");
        if !summary.compiler_numeric_mode_known {
            body.push_str(" Its zero-subtraction behavior is unrecognized; affected expressions remain unresolved.");
        }
    }
    if summary.reanalysis_findings != 0 || summary.reanalysis_note.is_some() {
        body.push_str(
            "\n\nSome source behavior is unresolved. Review Source analysis after opening.",
        );
    }
    let dialog =
        adw::MessageDialog::new(Some(window), Some("Review rebuilt workspace"), Some(&body));
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("keep", "Use rebuilt copy");
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("keep", adw::ResponseAppearance::Suggested);
    let imported = std::cell::RefCell::new(Some(imported));
    let (window, state, status, version, selection, panel) = (
        window.downgrade(),
        state.clone(),
        status.clone(),
        version.downgrade(),
        selection.clone(),
        panel.clone(),
    );
    dialog.connect_response(None, move |_, response| {
        if response == "keep" && state.borrow().generation != generation {
            status.set_text("Draft changed during review; rebuilt copy was discarded. Open the saved file again when ready.");
            return;
        }
        if response == "keep" && state.borrow().generation == generation {
            if let (Some(imported), Some(window), Some(version)) = (
                imported.borrow_mut().take(),
                window.upgrade(),
                version.upgrade(),
            ) {
                confirm_import(
                    imported, &window, &state, &status, &version, &selection, &panel, generation,
                );
            }
        }
    });
    dialog.present();
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
    super::text_export::review(window,"Review edited Lua source copy","Scalar and structured draft edits are included. Changed declarations are rebuilt in their source slots; extra declarations follow the last slot for their family, or are added at the end. Review ordering before using this copy. Other statements and comments are preserved.","hyprland-edited.lua",text);
}
pub(super) fn connect_source_copy(
    button: &gtk::Button,
    window: &adw::ApplicationWindow,
    state: &Shared,
) {
    let win = window.downgrade();
    let state = state.clone();
    button.connect_clicked(move|button|{
        let Some(window)=win.upgrade()else{return;};let s=state.borrow();
        if s.source_graphs.contains_key(&s.draft.version) {drop(s);super::project_copy::show(&window,&state);return;}
        if let Some(doc)=s.legacy_sources.get(&s.draft.version).cloned(){
            let snapshot=match captured_workspace(&s){Ok(snapshot)=>snapshot,Err(error)=>{drop(s);notice(&window,"Cannot prepare legacy copy",&error);return;}};
            if !snapshot.monitors.is_empty() || !snapshot.devices.is_empty() || !snapshot.bindings.is_empty() || !snapshot.workspace_rules.is_empty() || !snapshot.rules.is_empty() || !snapshot.animations.declarations.is_empty() || !snapshot.gestures.declarations.is_empty() {
                drop(s);notice(&window,"Structured legacy export pending","This source copy includes scalar edits only. Save the project to keep all structured drafts; exporting those drafts into legacy syntax is not implemented yet.");return;
            }
            let catalogue=s.catalogue.clone();let draft=s.draft.clone();let generation=s.generation;drop(s);let state=state.clone();let win=window.downgrade();let button=button.downgrade();if let Some(button)=button.upgrade(){button.set_sensitive(false);}
            background(move||doc.edited_copy(&draft,&catalogue),move|result|{
                if let Some(button)=button.upgrade(){button.set_sensitive(true);}let Some(win)=win.upgrade()else{return;};if state.borrow().generation!=generation{notice(&win,"Draft changed","The draft changed while the copy was prepared. Review again to use the current edits.");return;}
                match result{Ok(source)=>{super::text_export::review_legacy(&win,source);},Err(error)=>notice(&win,"Cannot prepare legacy copy",&error)}
            });return;
        }
        if !s.sources.contains_key(&s.draft.version){drop(s);notice(&window,"Source required","Import a .lua or .conf source file first. Value-only JSON drafts have no original source to preserve.");return;}
        let workspace=match captured_workspace(&s){Ok(workspace)=>workspace,Err(error)=>{drop(s);notice(&window,"Cannot prepare source copy",&error);return;}};
        let generation=s.generation;drop(s);let state=state.clone();let win=window.downgrade();let button=button.downgrade();if let Some(button)=button.upgrade(){button.set_sensitive(false);}
        background(move||worker::run(worker::Request::CopyWorkspace{workspace:Box::new(workspace)}),move|result|{
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
    if imported.migration.is_some() {
        s.saved_source_graphs
            .insert(imported.draft.version.clone(), None);
    }
    let graph_fingerprint = imported.graph.as_ref().map(|g| g.fingerprint.clone());
    if let Some(graph) = imported.graph {
        s.source_graphs
            .insert(imported.draft.version.clone(), graph);
    } else {
        s.source_graphs.remove(&imported.draft.version);
    }
    if imported.saved_json {
        s.saved_source_graphs
            .insert(imported.draft.version.clone(), graph_fingerprint);
    }
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
    if let Some(doc) = imported.legacy {
        s.legacy_sources
            .insert(imported.draft.version.clone(), std::sync::Arc::new(doc));
    } else {
        s.legacy_sources.remove(&imported.draft.version);
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
    if imported.saved_json {
        s.saved_devices
            .insert(imported.draft.version.clone(), imported.devices.clone());
    }
    s.devices
        .insert(imported.draft.version.clone(), imported.devices);
    if imported.saved_json {
        s.saved_bindings
            .insert(imported.draft.version.clone(), imported.bindings.clone());
    }
    s.bindings
        .insert(imported.draft.version.clone(), imported.bindings);
    if imported.saved_json {
        s.saved_workspace_rules.insert(
            imported.draft.version.clone(),
            imported.workspace_rules.clone(),
        );
    }
    s.workspace_rules
        .insert(imported.draft.version.clone(), imported.workspace_rules);
    if imported.saved_json {
        s.saved_rules
            .insert(imported.draft.version.clone(), imported.rules.clone());
    }
    s.rules
        .insert(imported.draft.version.clone(), imported.rules);
    if imported.saved_json {
        s.saved_animations
            .insert(imported.draft.version.clone(), imported.animations.clone());
    }
    s.animations
        .insert(imported.draft.version.clone(), imported.animations);
    if imported.saved_json {
        s.saved_gestures
            .insert(imported.draft.version.clone(), imported.gestures.clone());
    }
    s.gestures
        .insert(imported.draft.version.clone(), imported.gestures);
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
            let s=state.borrow();if s.legacy_sources.contains_key(&s.draft.version){notice(&win,"Legacy source verification pending","Saved legacy projects retain their original bytes. Verifying or rebinding an external legacy file is not available yet; import a changed file as a separate project.");return;}let Some(origin)=s.source_origins.get(&s.draft.version).cloned()else {
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
                let captured=super::capture_worker::read_selected(&path)?;
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
        let Some(win)=win.upgrade()else{return;};let s=state.borrow();
        let (title,text)=if let Some(doc)=s.legacy_sources.get(&s.draft.version){
            let mut text=format!("{} recognized scalar values. Includes, variables and commands were not evaluated. Unresolved lines remain in the saved source.\n",doc.report.draft.values.len());
            for line in &doc.report.skipped {text.push_str(&format!("\nLine {line}: unresolved directive or value"));}
            for key in &doc.report.duplicate_keys {text.push_str(&format!("\nRepeated scalar: {key} (last recognized value shown)"));}
            ("Legacy import diagnostics",text)
        }else if let Some(doc)=s.sources.get(&s.draft.version){
            ("Static Lua import diagnostics",if doc.report.diagnostics.is_empty(){"No unresolved syntax was found in the supported static subset. This is not a runtime validation certificate.".into()}else{doc.report.diagnostics.iter().map(|d|format!("Line {}: {}",d.line,d.message)).collect::<Vec<_>>().join("\n")})
        }else{drop(s);notice(&win,"No import report","Import a .lua or .conf source file to inspect its line diagnostics.");return;};
        drop(s);let dialog=adw::Window::builder().title(title).transient_for(&win).default_width(760).default_height(500).build();let view=gtk::TextView::builder().editable(false).wrap_mode(gtk::WrapMode::WordChar).left_margin(16).right_margin(16).top_margin(16).bottom_margin(16).build();view.buffer().set_text(&text);dialog.set_content(Some(&gtk::ScrolledWindow::builder().child(&view).build()));dialog.present();
    });
}

#[cfg(test)]
pub(super) fn test_graph_migration(window: &adw::ApplicationWindow) {
    use hyprland_editor_core::{
        module_path::Context,
        workspace_graph::{self, Bundle, Graph, View},
    };
    use std::time::Instant;
    let temporary = super::test_support::TempDir::new("he-ui-migration");
    let original = temporary.path().join("main.lua");
    std::fs::write(&original, "hl.config{general={border_size=3}}\n").unwrap();
    let captured = storage::read_source(&original).unwrap();
    std::fs::remove_file(&original).unwrap();
    let bundle = Bundle {
        graph: Graph {
            private: Default::default(),
            revisions: vec![],
            format: "hye-workspace-graph-1".into(),
            schema_sha256: workspace_graph::schema_sha256(),
            compiler: hyprland_editor_core::lua::replay::compiler_profile(),
            root: "root".into(),
            context: Context {
                main_file: captured.identity.selected_path.clone(),
                home: None,
                inherited_package_path: None,
            },
            working_directory: None,
            views: vec![View {
                document: "root".into(),
                load_name: captured.identity.selected_path.clone(),
            }],
            bindings: vec![],
        },
        sources: std::sync::Arc::new(vec![hyprland_editor_core::workspace::SourceSnapshot {
            id: "root".into(),
            language: SourceLanguage::Lua,
            content: captured.content,
            origin: captured.identity,
        }]),
    };
    let workspace = Workspace::new(Draft::new(BASELINE)).with_graph(&bundle);
    let mut value = serde_json::to_value(workspace).unwrap();
    value["source_graph"]["compiler"]
        .as_object_mut()
        .unwrap()
        .remove("numeric_codegen");
    value["source_graph"]["schema_sha256"] = hyprland_editor_core::lua::source_hash(include_str!(
        "../../../data/migrations/graph-model-before-projection.json"
    ))
    .into();
    let bytes = serde_json::to_vec(&value).unwrap();
    let path = temporary.path().join("old.hye");
    std::fs::write(&path, &bytes).unwrap();
    let state = super::initial_state();
    let status = gtk::Label::new(None);
    let version = gtk::DropDown::from_strings(&[BASELINE, "audited-main"]);
    let selection = gtk::SingleSelection::new(Some(gtk::StringList::new(&[])));
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let catalogue = hyprland_editor_core::catalogue();
    let open =
        || read_import_for_review(path.clone(), catalogue.clone(), BASELINE.into(), true).unwrap();
    let settle = || {
        let context = gtk::glib::MainContext::default();
        for _ in 0..30 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    let dialog = |heading: &str| {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<adw::MessageDialog>().ok())
            .find(|w| w.heading().as_deref() == Some(heading))
            .expect("Visible migration/unsaved dialog")
    };
    let generation = state.borrow().generation;
    review_import(
        open(),
        window,
        &state,
        &status,
        &version,
        &selection,
        &panel,
        generation,
    );
    settle();
    let review = dialog("Review rebuilt workspace");
    assert_eq!(review.default_response().as_deref(), Some("cancel"));
    assert!(review.body().contains("current compiler was checked again"));
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&review));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, review.width() as f64, review.height() as f64);
        let node = snapshot.to_node().unwrap();
        review
            .renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(format!("{path}.migration.png"))
            .unwrap();
    }
    review.response("cancel");
    review.destroy();
    settle();
    assert_eq!(state.borrow().generation, generation);
    assert!(state.borrow().source_graphs.is_empty());
    review_import(
        open(),
        window,
        &state,
        &status,
        &version,
        &selection,
        &panel,
        generation,
    );
    settle();
    state.borrow_mut().generation += 1;
    let review = dialog("Review rebuilt workspace");
    review.response("keep");
    review.destroy();
    settle();
    assert!(state.borrow().source_graphs.is_empty());
    assert!(status.text().contains("discarded"));
    state.borrow_mut().draft.values.insert(
        "general:border_size".into(),
        hyprland_editor_core::Value::Int(9),
    );
    let generation = state.borrow().generation;
    review_import(
        open(),
        window,
        &state,
        &status,
        &version,
        &selection,
        &panel,
        generation,
    );
    settle();
    let review = dialog("Review rebuilt workspace");
    review.response("keep");
    review.destroy();
    settle();
    let replace = dialog("Replace unsaved draft?");
    replace.response("cancel");
    replace.destroy();
    settle();
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        hyprland_editor_core::Value::Int(9)
    );
    assert!(state.borrow().source_graphs.is_empty());
    review_import(
        open(),
        window,
        &state,
        &status,
        &version,
        &selection,
        &panel,
        generation,
    );
    settle();
    let review = dialog("Review rebuilt workspace");
    review.response("keep");
    review.destroy();
    settle();
    let replace = dialog("Replace unsaved draft?");
    replace.response("replace");
    replace.destroy();
    settle();
    assert!(state.borrow().source_graphs.contains_key(BASELINE));
    assert!(state.borrow().dirty());
    assert!(status.text().contains("save it as a new private workspace"));
    let save = gtk::Button::new();
    connect_save_at(
        &save,
        window,
        &state,
        &status,
        Some(temporary.path().into()),
    );
    save.emit_clicked();
    let start = Instant::now();
    while !save.is_sensitive() {
        settle();
        assert!(start.elapsed() < Duration::from_secs(5));
    }
    assert!(!state.borrow().dirty());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(!original.exists());
    let files: Vec<_> = std::fs::read_dir(temporary.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 2);
    let saved_files: Vec<_> = std::fs::read_dir(temporary.path().join("hyprland-editor"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(saved_files.len(), 1);
    let saved = workspace::read(&saved_files[0], &catalogue).unwrap();
    assert_eq!(
        saved.source_graph.unwrap().schema_sha256,
        workspace_graph::schema_sha256()
    );
}

#[cfg(test)]
pub(super) fn test_combined_source_copy(window: &adw::ApplicationWindow) {
    let root = super::test_support::TempDir::new("combined-source-copy");
    let path = root.path().join("original.lua");
    let original = "-- retained source\nhl.config{general={border_size=2,gaps_in=3}}\n";
    std::fs::write(&path, original).unwrap();
    let catalogue = hyprland_editor_core::catalogue();
    let imported = read_import(path.clone(), catalogue.clone(), BASELINE.into()).unwrap();
    std::fs::remove_file(&path).unwrap();
    let state = super::initial_state();
    let status = gtk::Label::new(None);
    let version = gtk::DropDown::from_strings(&[BASELINE, "audited-main"]);
    let selection = gtk::SingleSelection::new(Some(gtk::StringList::new(&[])));
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    apply_import(imported, &state, &status, &version, &selection, &panel);
    let desired = "hl.config{general={border_size=5},decoration={rounding=7}}\nhl.monitor{output='DP-1'}\nhl.device{name='mouse',sensitivity=0.2}\nhl.bind('Q',hl.dsp.no_op())\nhl.workspace_rule{workspace='1'}\nhl.window_rule{match={class='term'},float=true}\nhl.layer_rule{match={namespace='panel'},blur=true}\nhl.animation{leaf='windows',enabled=false}\nhl.gesture{fingers=3,direction='left',action={start=function()hl.dispatch(hl.dsp.no_op())end,finish=function()end}}";
    let target = parse_source(desired.into(), BASELINE.into(), &catalogue)
        .unwrap()
        .report;
    assert!(target.diagnostics.is_empty());
    {
        let mut s = state.borrow_mut();
        s.replace_monitors(target.monitors.clone()).unwrap();
        s.replace_devices(target.devices.clone()).unwrap();
        s.replace_bindings(target.bindings.clone()).unwrap();
        s.replace_workspaces(target.workspace_rules.clone())
            .unwrap();
        s.replace_rules(target.rules.clone()).unwrap();
        s.replace_animations(target.animations.clone()).unwrap();
        s.replace_gestures(target.gestures.clone()).unwrap();
        s.remember("general:gaps_in").unwrap();
        s.draft.values.remove("general:gaps_in");
        for (key, value) in &target.draft.values {
            s.remember(key).unwrap();
            s.draft.values.insert(key.clone(), value.clone());
        }
    }
    // Save/reopen before invoking the real graphical action and restricted worker.
    let saved = root.path().join("project.hye");
    std::fs::write(
        &saved,
        captured_workspace(&state.borrow())
            .unwrap()
            .to_json(&catalogue)
            .unwrap(),
    )
    .unwrap();
    let imported = read_import(saved, catalogue.clone(), BASELINE.into()).unwrap();
    apply_import(imported, &state, &status, &version, &selection, &panel);
    let button = gtk::Button::new();
    connect_source_copy(&button, window, &state);
    button.emit_clicked();
    let context = gtk::glib::MainContext::default();
    let started = std::time::Instant::now();
    while !button.is_sensitive() {
        while context.pending() {
            context.iteration(false);
        }
        assert!(started.elapsed() < Duration::from_secs(8));
        std::thread::sleep(Duration::from_millis(10));
    }
    let review = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Window>().ok())
        .find(|window| window.title().as_deref() == Some("Review edited Lua source copy"))
        .expect("Combined source review");
    fn find_text(widget: &gtk::Widget) -> Option<gtk::TextView> {
        if let Some(view) = widget.downcast_ref::<gtk::TextView>() {
            return Some(view.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            if let Some(view) = find_text(&widget) {
                return Some(view);
            }
            child = widget.next_sibling();
        }
        None
    }
    let buffer = find_text(review.upcast_ref()).unwrap().buffer();
    let copy = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
    assert!(copy.contains("-- retained source"));
    let actual = parse_source(copy.to_string(), BASELINE.into(), &catalogue)
        .unwrap()
        .report;
    assert_eq!(actual.draft.values, target.draft.values);
    assert_eq!(
        hyprland_editor_core::monitor::export_lua(&actual.monitors, BASELINE).unwrap(),
        hyprland_editor_core::monitor::export_lua(&target.monitors, BASELINE).unwrap()
    );
    assert!(hyprland_editor_core::device::same_declarations(
        &actual.devices,
        &target.devices
    ));
    assert!(actual
        .bindings
        .same_declarations(&target.bindings, BASELINE));
    assert!(hyprland_editor_core::workspace_rule::same_declarations(
        &actual.workspace_rules,
        &target.workspace_rules
    ));
    assert!(hyprland_editor_core::rule::same_declarations(
        &actual.rules,
        &target.rules
    ));
    assert!(actual.animations.same_declarations(&target.animations));
    assert!(actual
        .gestures
        .same_declarations(&target.gestures, BASELINE));
    assert_eq!(state.borrow().sources[BASELINE].source, original);
    assert!(!path.exists());
    let (minimum, _, _, _) = review
        .child()
        .unwrap()
        .measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Combined source review needs {minimum}px");
    review.destroy();
}

#[cfg(test)]
pub(super) fn test_legacy_persistence(window: &adw::ApplicationWindow) {
    use hyprland_editor_core::Value;
    let root = super::test_support::TempDir::new("legacy-project");
    let original = root.path().join("original.conf");
    let source = "# retained café\r\nsource = missing.conf\r\nexec = never-run\r\ngeneral {\r\n  border_size = 2 # keep\r\n}\r\n";
    std::fs::write(&original, source).unwrap();
    let catalogue = hyprland_editor_core::catalogue();
    let imported = read_import(original.clone(), catalogue.clone(), BASELINE.into()).unwrap();
    std::fs::remove_file(&original).unwrap();
    let state = super::initial_state();
    let status = gtk::Label::new(None);
    let version = gtk::DropDown::from_strings(&[BASELINE, "audited-main"]);
    let selection = gtk::SingleSelection::new(Some(gtk::StringList::new(&[])));
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    apply_import(imported, &state, &status, &version, &selection, &panel);
    assert!(state.borrow().sources.is_empty());
    assert_eq!(state.borrow().legacy_sources[BASELINE].content, source);
    {
        let mut s = state.borrow_mut();
        s.remember("general:border_size").unwrap();
        s.draft
            .values
            .insert("general:border_size".into(), Value::Int(6));
    }
    let save = gtk::Button::new();
    connect_save_at(&save, window, &state, &status, Some(root.path().into()));
    save.emit_clicked();
    let settle_until = |done: &dyn Fn() -> bool| {
        let context = gtk::glib::MainContext::default();
        let start = std::time::Instant::now();
        while !done() {
            while context.pending() {
                context.iteration(false);
            }
            assert!(
                start.elapsed() < Duration::from_secs(8),
                "Legacy workflow did not finish"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    settle_until(&|| save.is_sensitive());
    assert!(!state.borrow().dirty());
    let saved = std::fs::read_dir(root.path().join("hyprland-editor"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let bytes = std::fs::read(&saved).unwrap();
    let imported = read_import(saved.clone(), catalogue.clone(), BASELINE.into()).unwrap();
    assert!(imported.saved_json);
    assert_eq!(imported.legacy.as_ref().unwrap().content, source);
    apply_import(imported, &state, &status, &version, &selection, &panel);
    assert!(!state.borrow().dirty());
    let copy = gtk::Button::new();
    super::project_save::test_named_copy(window, &state);
    connect_source_copy(&copy, window, &state);
    copy.emit_clicked();
    settle_until(&|| copy.is_sensitive());
    let review = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Window>().ok())
        .find(|window| window.title().as_deref() == Some("Review edited legacy source copy"))
        .unwrap();
    fn descendants(widget: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut result = vec![widget.clone()];
        let mut child = widget.first_child();
        while let Some(widget) = child {
            result.extend(descendants(&widget));
            child = widget.next_sibling();
        }
        result
    }
    let children = descendants(review.upcast_ref());
    let text = children
        .iter()
        .find_map(|widget| widget.downcast_ref::<gtk::TextView>())
        .unwrap();
    let buffer = text.buffer();
    assert_eq!(
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .as_str(),
        source.replace("border_size = 2", "border_size = 6")
    );
    for label in ["Copy Config", "Save Config File…"] {
        assert!(children.iter().any(|widget| widget
            .downcast_ref::<gtk::Button>()
            .is_some_and(|button| button.label().as_deref() == Some(label))));
    }
    let (minimum, _, _, _) = review
        .child()
        .unwrap()
        .measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Legacy review needs {minimum}px");
    review.destroy();
    assert!(!original.exists());
    assert_eq!(std::fs::read(&saved).unwrap(), bytes);
    // A subsequent value-only import cannot keep an unrelated legacy source.
    let json = root.path().join("values.json");
    std::fs::write(&json, serde_json::to_vec(&Draft::new(BASELINE)).unwrap()).unwrap();
    let imported = read_import(json, catalogue, BASELINE.into()).unwrap();
    apply_import(imported, &state, &status, &version, &selection, &panel);
    assert!(state.borrow().legacy_sources.is_empty());
    assert!(state.borrow().source_origins.is_empty());
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
            migration: None,
            graph: None,
            draft,
            source: Some(doc),
            legacy: None,
            origin: Some(captured.identity.clone()),
            saved_json: false,
            monitors: vec![],
            devices: vec![],
            bindings: Default::default(),
            workspace_rules: vec![],
            rules: vec![],
            animations: Default::default(),
            gestures: Default::default(),
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
    let device = hyprland_editor_core::device::Spec::new("saved-device", "test keyboard");
    state
        .borrow_mut()
        .replace_devices(vec![device.clone()])
        .unwrap();
    let monitor = hyprland_editor_core::monitor::Rule::new("saved-monitor");
    state
        .borrow_mut()
        .replace_monitors(vec![monitor.clone()])
        .unwrap();
    let animations = hyprland_editor_core::lua::parse(
        "hl.animation{leaf='windows',enabled=false}",
        BASELINE,
        &catalogue,
    )
    .unwrap()
    .animations;
    state
        .borrow_mut()
        .replace_animations(animations.clone())
        .unwrap();
    let gestures = hyprland_editor_core::lua::parse(
        "hl.gesture{fingers=3,direction='left',action={start=function()hl.dispatch(hl.dsp.no_op())end,finish=function()end}}",
        BASELINE, &catalogue,
    ).unwrap().gestures;
    state
        .borrow_mut()
        .replace_gestures(gestures.clone())
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
    assert_eq!(saved.animations, animations);
    assert_eq!(saved.gestures, gestures);
    assert_eq!(saved.monitors, vec![monitor]);
    assert_eq!(saved.devices, vec![device]);
    assert_eq!(
        saved.draft.values["general:border_size"],
        hyprland_editor_core::Value::Int(5)
    );
    super::project_save::test_named_copy(window, &state);
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

#[cfg(test)]
pub(super) fn test_graph_persistence(
    window: &adw::ApplicationWindow,
    state: &Shared,
    root: &std::path::Path,
) {
    let version = state.borrow().draft.version.clone();
    let expected_sources = state.borrow().source_graphs[&version].bundle.sources.len();
    let saved_files = || {
        std::fs::read_dir(root.join("hyprland-editor"))
            .into_iter()
            .flatten()
            .map(|entry| entry.unwrap().path())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let before = saved_files();
    let button = gtk::Button::new();
    let status = gtk::Label::new(None);
    connect_save_at(&button, window, state, &status, Some(root.into()));
    let wait = || {
        let start = std::time::Instant::now();
        let context = gtk::glib::MainContext::default();
        while !button.is_sensitive() {
            while context.pending() {
                context.iteration(false);
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    button.emit_clicked();
    wait();
    assert!(!state.borrow().dirty(), "{}", status.text());
    // Multiple snapshots coexist. Directory enumeration order must never pick
    // an older snapshot when this helper verifies the newly saved revision.
    let new_files = saved_files()
        .difference(&before)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        new_files.len(),
        1,
        "A successful save must create one new private snapshot"
    );
    let path = new_files[0].clone();
    let imported = read_import(
        path.clone(),
        state.borrow().catalogue.clone(),
        version.clone(),
    )
    .unwrap();
    let document = std::sync::Arc::new(imported.source.unwrap());
    let origin = imported.origin.unwrap();
    let graph = imported.graph.unwrap();
    assert_eq!(graph.bundle.sources.len(), expected_sources);
    assert_eq!(
        graph.fingerprint,
        state.borrow().source_graphs[&version].fingerprint
    );
    assert!(
        source_workspace(path).is_err(),
        "Single-root reconciliation must not erase a graph"
    );
    // An older successful graph save cannot clear an intervening undo.
    let previous = state
        .borrow()
        .undo
        .previous_graph()
        .expect("Expected graph inverse before late save");
    button.emit_clicked();
    state.borrow_mut().undo_last();
    wait();
    assert_eq!(
        state
            .borrow()
            .source_graphs
            .get(&version)
            .map(|g| g.fingerprint.clone()),
        previous
    );
    assert!(state.borrow().dirty());
    let draft = state.borrow().draft.clone();
    let generation = state.borrow().generation;
    state
        .borrow_mut()
        .keep_graph_revision(graph, draft, document, origin, generation)
        .unwrap();
    assert!(!state.borrow().dirty());
    super::project_save::test_named_copy(window, state);
}
