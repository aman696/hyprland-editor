mod animations;
mod bind_forms;
mod bindings;
mod capture_worker;
mod compiler_check;
#[allow(dead_code)] // Separate installed role; never selects a saved Lua 5.4 model.
mod compiler_client;
mod controls;
mod detected_devices;
mod detected_outputs;
mod device_forms;
mod devices;
mod directory_worker;
mod fd_channel;
mod gestures;
mod graph_migration;
#[cfg(test)]
mod graph_transaction_tests;
mod history;
mod legacy_source;
mod monitors;
mod native_build;
mod presentation;
mod preview_layers;
mod preview_preparation;
mod process_policy;
mod project_save;
mod projection_worker;
mod reconciliation;
mod rule_forms;
mod rules;
mod selector_forms;
mod source_access;
mod source_analysis;
mod source_changes;
mod source_compound_forms;
mod source_context;
mod source_declarations;
mod source_graph;
mod source_modules;
mod source_monitors;
mod source_overrides;
mod source_pending;
mod source_provenance;
mod source_reconcile;
#[cfg(test)]
mod test_support;
mod text_export;
mod worker;
mod workflow;
mod workspaces;
use adw::prelude::*;
use controls::form;
use hyprland_editor_core::{
    catalogue, source::SourceIdentity, Draft, Setting, Value, AUDITED_MAIN, BASELINE,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};

type Shared = Rc<RefCell<State>>;
struct State {
    main_status: gtk::glib::WeakRef<gtk::Label>,
    catalogue: Vec<Setting>,
    draft: Draft,
    undo: history::History,
    drafts: BTreeMap<String, Draft>,
    visible: Vec<Setting>,
    category: Option<String>,
    sources: BTreeMap<String, Arc<hyprland_editor_core::lua::LuaDocument>>,
    legacy_sources: BTreeMap<String, Arc<legacy_source::Document>>,
    source_origins: BTreeMap<String, SourceIdentity>,
    source_graphs: BTreeMap<String, Arc<source_graph::Saved>>,
    saved_source_graphs: BTreeMap<String, Option<String>>,
    saved_source_origins: BTreeMap<String, Option<SourceIdentity>>,
    active: Option<hyprland_editor_discovery::Snapshot>,
    generation: u64,
    saved: BTreeMap<String, BTreeMap<String, Value>>,
    monitors: BTreeMap<String, Vec<hyprland_editor_core::monitor::Rule>>,
    saved_monitors: BTreeMap<String, Vec<hyprland_editor_core::monitor::Rule>>,
    devices: BTreeMap<String, Vec<hyprland_editor_core::device::Spec>>,
    saved_devices: BTreeMap<String, Vec<hyprland_editor_core::device::Spec>>,
    bindings: BTreeMap<String, hyprland_editor_core::binding::Draft>,
    saved_bindings: BTreeMap<String, hyprland_editor_core::binding::Draft>,
    gestures: BTreeMap<String, hyprland_editor_core::gesture::Draft>,
    saved_gestures: BTreeMap<String, hyprland_editor_core::gesture::Draft>,
    animations: BTreeMap<String, hyprland_editor_core::animation::Draft>,
    saved_animations: BTreeMap<String, hyprland_editor_core::animation::Draft>,
    rules: BTreeMap<String, Vec<hyprland_editor_core::rule::Rule>>,
    saved_rules: BTreeMap<String, Vec<hyprland_editor_core::rule::Rule>>,
    workspace_rules: BTreeMap<String, Vec<hyprland_editor_core::workspace_rule::Rule>>,
    saved_workspace_rules: BTreeMap<String, Vec<hyprland_editor_core::workspace_rule::Rule>>,
}
impl State {
    fn dirty_version(&self, version: &str) -> bool {
        let draft = if self.draft.version == version {
            Some(&self.draft)
        } else {
            self.drafts.get(version)
        };
        let values_changed = draft.is_some_and(|d| {
            self.saved
                .get(version)
                .map_or(!d.values.is_empty(), |saved| saved != &d.values)
        });
        let source_changed = self.source_origins.get(version)
            != self
                .saved_source_origins
                .get(version)
                .and_then(Option::as_ref);
        let monitors = self.monitors.get(version).map(Vec::as_slice).unwrap_or(&[]);
        let saved_monitors = self
            .saved_monitors
            .get(version)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let empty = hyprland_editor_core::binding::Draft::default();
        let bindings_changed = self.bindings.get(version).unwrap_or(&empty)
            != self.saved_bindings.get(version).unwrap_or(&empty);
        self.rules.get(version).map(Vec::as_slice).unwrap_or(&[])
            != self
                .saved_rules
                .get(version)
                .map(Vec::as_slice)
                .unwrap_or(&[])
            || self
                .animations
                .get(version)
                .map(|a| &a.declarations)
                .map(Vec::as_slice)
                .unwrap_or(&[])
                != self
                    .saved_animations
                    .get(version)
                    .map(|a| &a.declarations)
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
            || self
                .gestures
                .get(version)
                .map(|g| g.declarations.as_slice())
                .unwrap_or(&[])
                != self
                    .saved_gestures
                    .get(version)
                    .map(|g| g.declarations.as_slice())
                    .unwrap_or(&[])
            || bindings_changed
            || self
                .workspace_rules
                .get(version)
                .map(Vec::as_slice)
                .unwrap_or(&[])
                != self
                    .saved_workspace_rules
                    .get(version)
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
            || values_changed
            || self
                .source_graphs
                .get(version)
                .map(|g| g.fingerprint.as_str())
                != self
                    .saved_source_graphs
                    .get(version)
                    .and_then(Option::as_deref)
            || source_changed
            || monitors != saved_monitors
            || self.devices.get(version).map(Vec::as_slice).unwrap_or(&[])
                != self
                    .saved_devices
                    .get(version)
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
    }
    fn dirty(&self) -> bool {
        self.dirty_version(&self.draft.version) || self.drafts.keys().any(|v| self.dirty_version(v))
    }
    fn remember(&mut self, id: &str) -> Result<(), String> {
        self.undo.field(id, self.draft.values.get(id).cloned())?;
        self.generation += 1;
        Ok(())
    }
    fn replace_source(
        &mut self,
        document: Arc<hyprland_editor_core::lua::LuaDocument>,
        origin: SourceIdentity,
        draft: Draft,
    ) -> Result<(), String> {
        if self.source_graphs.contains_key(&self.draft.version) {
            return Err("This workspace retains a source graph. Single-file replacement would invalidate its inputs; graph reconciliation is still required.".into());
        }
        draft.validate(&self.catalogue)?;
        origin.validate(&document.source)?;
        if draft.version != self.draft.version || document.report.draft.version != draft.version {
            return Err("Source revision belongs to a different snapshot".into());
        }
        let version = self.draft.version.clone();
        let old_doc = self
            .sources
            .get(&version)
            .cloned()
            .ok_or("Source snapshot missing")?;
        let old_origin = self
            .source_origins
            .get(&version)
            .cloned()
            .ok_or("Source identity missing")?;
        self.undo.source(&self.draft, &draft, old_doc, old_origin)?;
        self.sources.insert(version.clone(), document);
        self.source_origins.insert(version, origin);
        self.draft = draft;
        self.generation += 1;
        Ok(())
    }
    fn keep_graph(&mut self, graph: Arc<source_graph::Saved>) -> Result<(), String> {
        self.keep_graph_and_overrides(graph, self.draft.clone())
    }
    fn check_graph_root(&self, graph: &source_graph::Saved) -> Result<(), String> {
        let version = &self.draft.version;
        let root = graph
            .bundle
            .sources
            .iter()
            .find(|s| s.id == graph.bundle.graph.root)
            .ok_or("Graph root missing")?;
        if graph.version != *version
            || self.source_origins.get(version) != Some(&root.origin)
            || self
                .sources
                .get(version)
                .is_none_or(|d| d.source != root.content)
        {
            return Err("Analyze the imported root at its exact recorded revision before retaining the graph.".into());
        }
        Ok(())
    }
    fn keep_graph_and_monitors(
        &mut self,
        graph: Arc<source_graph::Saved>,
        monitors: Vec<hyprland_editor_core::monitor::Rule>,
    ) -> Result<(), String> {
        hyprland_editor_core::monitor::validate(&monitors, &self.draft.version)?;
        self.check_graph_root(&graph)?;
        let version = self.draft.version.clone();
        self.undo.graph_monitors(
            self.source_graphs.get(&version).cloned(),
            self.monitors.get(&version).cloned(),
        )?;
        self.source_graphs.insert(version.clone(), graph);
        self.monitors.insert(version, monitors);
        self.generation += 1;
        Ok(())
    }
    fn keep_graph_and_overrides(
        &mut self,
        graph: Arc<source_graph::Saved>,
        draft: Draft,
    ) -> Result<(), String> {
        self.check_graph_root(&graph)?;
        let version = &self.draft.version;
        let document = self
            .sources
            .get(version)
            .cloned()
            .ok_or("Source snapshot missing")?;
        let origin = self
            .source_origins
            .get(version)
            .cloned()
            .ok_or("Source identity missing")?;
        self.keep_graph_revision(graph, draft, document, origin, self.generation)
    }
    /// Commit a freshly rebuilt graph and its captured root under one inverse.
    /// Callers must separately reconstruct dependencies and review merge choices.
    fn keep_graph_revision(
        &mut self,
        graph: Arc<source_graph::Saved>,
        draft: Draft,
        document: Arc<hyprland_editor_core::lua::LuaDocument>,
        origin: SourceIdentity,
        expected_generation: u64,
    ) -> Result<(), String> {
        if self.generation != expected_generation {
            return Err("The draft changed while its graph revision was being reviewed".into());
        }
        draft.validate(&self.catalogue)?;
        origin.validate(&document.source)?;
        let root = graph
            .bundle
            .sources
            .iter()
            .find(|s| s.id == graph.bundle.graph.root)
            .ok_or("Graph root missing")?;
        if graph.version != self.draft.version
            || document.report.draft.version != self.draft.version
            || document.report.source_sha256
                != hyprland_editor_core::lua::source_hash(&document.source)
            || root.content != document.source
            || root.origin != origin
        {
            return Err(
                "The graph and parsed root must use the same captured bytes, identity and snapshot"
                    .into(),
            );
        }
        if draft.version != self.draft.version
            || draft.values.iter().any(|(id, value)| {
                self.draft
                    .values
                    .get(id)
                    .is_none_or(|old| match (old, value) {
                        (Value::Float(a), Value::Float(b)) => a.to_bits() != b.to_bits(),
                        _ => old != value,
                    })
            })
        {
            return Err(
                "Source reconciliation can only remove explicitly reviewed overrides".into(),
            );
        }
        let changes = self
            .draft
            .values
            .iter()
            .filter(|(id, _)| !draft.values.contains_key(*id))
            .map(|(id, v)| (id.clone(), Some(v.clone())))
            .collect::<Vec<_>>();
        let version = &self.draft.version;
        let old_document = self
            .sources
            .get(version)
            .cloned()
            .ok_or("Source snapshot missing")?;
        let old_origin = self
            .source_origins
            .get(version)
            .cloned()
            .ok_or("Source identity missing")?;
        let source_changed = old_document.source != document.source || old_origin != origin;
        if changes.is_empty()
            && !source_changed
            && self
                .source_graphs
                .get(version)
                .is_some_and(|g| g.fingerprint == graph.fingerprint)
        {
            return Ok(());
        }
        self.undo.graph(
            self.source_graphs.get(version).cloned(),
            changes,
            source_changed.then_some((old_document, old_origin)),
        )?;
        if source_changed {
            self.sources.insert(version.clone(), document);
            self.source_origins.insert(version.clone(), origin);
        }
        self.source_graphs.insert(version.clone(), graph);
        self.draft = draft;
        self.generation += 1;
        Ok(())
    }
    fn undo_last(&mut self) {
        let Some(operation) = self.undo.pop() else {
            return;
        };
        match operation {
            history::Undo::Graph {
                previous,
                source,
                changes,
                monitor_changed,
                previous_monitors,
                ..
            } => {
                if let Some(source) = source {
                    self.sources
                        .insert(self.draft.version.clone(), source.document);
                    self.source_origins
                        .insert(self.draft.version.clone(), source.origin);
                }
                if let Some(graph) = previous {
                    self.source_graphs.insert(self.draft.version.clone(), graph);
                } else {
                    self.source_graphs.remove(&self.draft.version);
                }
                for (id, previous) in changes {
                    if let Some(value) = previous {
                        self.draft.values.insert(id, value);
                    } else {
                        self.draft.values.remove(&id);
                    }
                }
                if monitor_changed {
                    if let Some(monitors) = previous_monitors {
                        self.monitors.insert(self.draft.version.clone(), monitors);
                    } else {
                        self.monitors.remove(&self.draft.version);
                    }
                }
            }
            history::Undo::Field { id, previous } => {
                if let Some(value) = previous {
                    self.draft.values.insert(id, value);
                } else {
                    self.draft.values.remove(&id);
                }
            }
            history::Undo::Source {
                changes,
                document,
                origin,
                ..
            } => {
                for (id, previous) in changes {
                    if let Some(value) = previous {
                        self.draft.values.insert(id, value);
                    } else {
                        self.draft.values.remove(&id);
                    }
                }
                let version = self.draft.version.clone();
                self.sources.insert(version.clone(), document);
                self.source_origins.insert(version, origin);
            }
            history::Undo::Gestures { previous, .. } => {
                self.gestures.insert(self.draft.version.clone(), previous);
            }
            history::Undo::Animations { previous, .. } => {
                self.animations.insert(self.draft.version.clone(), previous);
            }
            history::Undo::Rules { previous, .. } => {
                self.rules.insert(self.draft.version.clone(), previous);
            }
            history::Undo::Workspaces { previous, .. } => {
                self.workspace_rules
                    .insert(self.draft.version.clone(), previous);
            }
            history::Undo::Bindings { inverse, .. } => {
                inverse.restore(self.bindings.entry(self.draft.version.clone()).or_default());
            }
            history::Undo::Devices { previous, .. } => {
                self.devices.insert(self.draft.version.clone(), previous);
            }
            history::Undo::Monitors { previous, .. } => {
                self.monitors.insert(self.draft.version.clone(), previous);
            }
        }
        self.generation += 1;
    }
    fn export_lua(&self) -> Result<String, String> {
        if self
            .source_graphs
            .get(&self.draft.version)
            .is_some_and(|g| {
                !g.bundle.graph.revisions.is_empty() || !g.bundle.graph.private.is_empty()
            })
        {
            return Err("This draft includes private source revisions. Save the full workspace; override-only export would omit them. Full source export is not available yet.".into());
        }
        let mut text = self.draft.export_lua(&self.catalogue)?;
        if let Some(rules) = self.monitors.get(&self.draft.version) {
            if !rules.is_empty() {
                text.push_str(&hyprland_editor_core::monitor::export_lua(
                    rules,
                    &self.draft.version,
                )?);
            }
        }
        if let Some(specs) = self.devices.get(&self.draft.version) {
            text.push_str(&hyprland_editor_core::device::export_lua(
                specs,
                &self.draft.version,
            )?);
        }
        if let Some(rules) = self.workspace_rules.get(&self.draft.version) {
            text.push_str(&hyprland_editor_core::workspace_rule::export_lua(
                rules,
                &self.draft.version,
            )?);
        }
        if let Some(rules) = self.rules.get(&self.draft.version) {
            text.push_str(&hyprland_editor_core::rule::export_lua(
                rules,
                &self.draft.version,
            )?);
        }
        if let Some(bindings) = self.bindings.get(&self.draft.version) {
            text.push_str(&bindings.export_lua(&self.draft.version)?);
        }
        if let Some(animations) = self.animations.get(&self.draft.version) {
            text.push_str(&animations.export_lua(&self.draft.version)?);
        }
        if let Some(gestures) = self.gestures.get(&self.draft.version) {
            text.push_str(&gestures.export_lua(&self.draft.version)?);
        }
        Ok(text)
    }
    fn replace_gestures(
        &mut self,
        gestures: hyprland_editor_core::gesture::Draft,
    ) -> Result<(), String> {
        gestures.validate(&self.draft.version)?;
        let before = self
            .gestures
            .get(&self.draft.version)
            .cloned()
            .unwrap_or_default();
        if before == gestures {
            return Ok(());
        }
        self.undo.gestures(before)?;
        self.gestures.insert(self.draft.version.clone(), gestures);
        self.generation += 1;
        Ok(())
    }
    fn replace_animations(
        &mut self,
        animations: hyprland_editor_core::animation::Draft,
    ) -> Result<(), String> {
        animations.validate(&self.draft.version)?;
        let before = self
            .animations
            .get(&self.draft.version)
            .cloned()
            .unwrap_or_default();
        if before == animations {
            return Ok(());
        }
        self.undo.animations(before)?;
        self.animations
            .insert(self.draft.version.clone(), animations);
        self.generation += 1;
        Ok(())
    }
    fn replace_rules(
        &mut self,
        rules: Vec<hyprland_editor_core::rule::Rule>,
    ) -> Result<(), String> {
        hyprland_editor_core::rule::validate(&rules, &self.draft.version)?;
        let before = self
            .rules
            .get(&self.draft.version)
            .cloned()
            .unwrap_or_default();
        if before == rules {
            return Ok(());
        }
        self.undo.rules(before)?;
        self.rules.insert(self.draft.version.clone(), rules);
        self.generation += 1;
        Ok(())
    }
    fn replace_workspaces(
        &mut self,
        rules: Vec<hyprland_editor_core::workspace_rule::Rule>,
    ) -> Result<(), String> {
        hyprland_editor_core::workspace_rule::validate(&rules, &self.draft.version)?;
        let before = self
            .workspace_rules
            .get(&self.draft.version)
            .cloned()
            .unwrap_or_default();
        if before == rules {
            return Ok(());
        }
        self.undo.workspaces(before)?;
        self.workspace_rules
            .insert(self.draft.version.clone(), rules);
        self.generation += 1;
        Ok(())
    }
    fn replace_bindings(
        &mut self,
        bindings: hyprland_editor_core::binding::Draft,
    ) -> Result<(), String> {
        bindings.validate(&self.draft.version)?;
        let before = self
            .bindings
            .get(&self.draft.version)
            .cloned()
            .unwrap_or_default();
        if before == bindings {
            return Ok(());
        }
        self.undo.bindings(&before, &bindings)?;
        self.bindings.insert(self.draft.version.clone(), bindings);
        self.generation += 1;
        Ok(())
    }
    fn replace_devices(
        &mut self,
        specs: Vec<hyprland_editor_core::device::Spec>,
    ) -> Result<(), String> {
        hyprland_editor_core::device::validate(&specs, &self.draft.version)?;
        let before = self
            .devices
            .get(&self.draft.version)
            .cloned()
            .unwrap_or_default();
        if before == specs {
            return Ok(());
        }
        self.undo.devices(before)?;
        self.devices.insert(self.draft.version.clone(), specs);
        self.generation += 1;
        Ok(())
    }
    fn replace_monitors(
        &mut self,
        rules: Vec<hyprland_editor_core::monitor::Rule>,
    ) -> Result<(), String> {
        hyprland_editor_core::monitor::validate(&rules, &self.draft.version)?;
        let before = self
            .monitors
            .get(&self.draft.version)
            .cloned()
            .unwrap_or_default();
        if before == rules {
            return Ok(());
        }
        self.undo.monitors(before)?;
        self.monitors.insert(self.draft.version.clone(), rules);
        self.generation += 1;
        Ok(())
    }
}
fn human_name(id: &str) -> String {
    presentation::setting_name(id)
}

fn label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .xalign(0.)
        .selectable(true)
        .build()
}

fn summary(state: &Shared, status: &gtk::Label) {
    let s = state.borrow();
    status.set_text(&format!(
        "{} {} in draft{}{}{}{}{}{}{}{} · {}",
        s.draft.values.len(),
        if s.draft.values.len() == 1 {
            "setting"
        } else {
            "settings"
        },
        s.monitors
            .get(&s.draft.version)
            .filter(|m| !m.is_empty())
            .map_or(String::new(), |m| format!(" · {} monitor rules", m.len())),
        s.devices
            .get(&s.draft.version)
            .filter(|d| !d.is_empty())
            .map_or(String::new(), |d| format!(" · {} device rules", d.len())),
        s.bindings
            .get(&s.draft.version)
            .filter(|b| !b.is_empty())
            .map_or(String::new(), |b| format!(" · {} bindings", b.binds.len())),
        s.workspace_rules
            .get(&s.draft.version)
            .filter(|r| !r.is_empty())
            .map_or(String::new(), |r| format!(" · {} workspace rules", r.len())),
        s.rules
            .get(&s.draft.version)
            .filter(|r| !r.is_empty())
            .map_or(String::new(), |r| format!(
                " · {} window/layer declarations",
                r.len()
            )),
        s.animations
            .get(&s.draft.version)
            .filter(|a| !a.is_empty())
            .map_or(String::new(), |a| format!(
                " · {} animation declarations",
                a.declarations.len()
            )),
        s.gestures
            .get(&s.draft.version)
            .filter(|g| !g.is_empty())
            .map_or(String::new(), |g| format!(
                " · {} gesture declarations",
                g.declarations.len()
            )),
        s.source_graphs
            .get(&s.draft.version)
            .filter(|g| !g.bundle.graph.revisions.is_empty() || !g.bundle.graph.private.is_empty())
            .map_or(String::new(), |g| if !g.bundle.graph.private.is_empty() {
                " · Private source data".into()
            } else {
                format!(" · {} source revisions", g.bundle.graph.revisions.len())
            }),
        if s.dirty_version(&s.draft.version) {
            "Unsaved changes"
        } else {
            "Saved"
        }
    ));
}
fn notice(window: &adw::ApplicationWindow, title: &str, body: &str) {
    // Error text may contain untrusted JSON keys; bound it before constructing GTK labels.
    let bounded: String = body.chars().take(4096).collect();
    let dialog = adw::MessageDialog::new(Some(window), Some(title), Some(&bounded));
    dialog.add_response("close", "Close");
    dialog.set_close_response("close");
    dialog.present();
}

fn detail(state: &Shared, panel: &gtk::Box, setting: &Setting, status: &gtk::Label) {
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }
    let context = label(&presentation::context(&setting.id));
    context.add_css_class("dim-label");
    panel.append(&context);
    let title = label(&human_name(&setting.id));
    title.add_css_class("title-1");
    panel.append(&title);
    let current = state.borrow().draft.values.get(&setting.id).cloned();
    let observed = state
        .borrow()
        .active
        .as_ref()
        .filter(|a| a.version.as_deref() == Some(setting.version.as_str()))
        .and_then(|a| a.values.get(&setting.id))
        .cloned();
    let description = observed
        .as_ref()
        .map(|o| o.description.as_str())
        .filter(|d| !d.is_empty())
        .unwrap_or(presentation::help(setting));
    let description: String = description.chars().take(320).collect();
    let help = label(&description);
    help.add_css_class("dim-label");
    panel.append(&help);
    let group = adw::PreferencesGroup::new();
    group.set_title("Draft Value");
    let card = gtk::Box::new(gtk::Orientation::Vertical, 16);
    card.add_css_class("card");
    let fields = gtk::Box::new(gtk::Orientation::Vertical, 12);
    fields.set_margin_top(16);
    fields.set_margin_bottom(16);
    fields.set_margin_start(16);
    fields.set_margin_end(16);
    let initial = current
        .clone()
        .or_else(|| observed.as_ref().and_then(|o| o.current.clone()));
    let getter = form(setting, initial, &fields);
    card.append(&fields);
    group.add(&card);
    let origin = if current.is_some() {
        "This value is in the draft".into()
    } else if observed.as_ref().is_some_and(|o| o.current.is_some()) {
        "Starting from the desktop snapshot".into()
    } else if let Some(value) = setting.known_default() {
        format!("Default: {}", presentation::value_text(&value))
    } else {
        "Default unavailable. Choose a value explicitly.".into()
    };
    group.set_description(Some(&origin));
    panel.append(&group);
    let message = label("");
    message.set_visible(false);
    panel.append(&message);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    actions.set_halign(gtk::Align::End);
    let unset = gtk::Button::with_label("Remove Override");
    unset.add_css_class("flat");
    unset.set_visible(current.is_some());
    actions.append(&unset);
    let stage = gtk::Button::with_label(if current.is_some() {
        "Update Draft"
    } else {
        "Add to Draft"
    });
    stage.add_css_class("suggested-action");
    actions.append(&stage);
    panel.append(&actions);
    let advanced = gtk::Expander::builder()
        .label("Technical Details")
        .margin_top(8)
        .build();
    let details = gtk::Box::new(gtk::Orientation::Vertical, 8);
    details.set_margin_top(12);
    details.append(&label(&format!("Setting: {}", setting.id)));
    details.append(&label(&format!(
        "Type: {} · Version: {}",
        setting.kind, setting.version
    )));
    details.append(&label(&format!(
        "Default expression: {}",
        setting.default_expression
    )));
    if let Some(origin) = state.borrow().source_origins.get(&setting.version) {
        details.append(&label(&format!(
            "Recorded source: {}",
            origin.display_path()
        )));
        if let Some(resolved) = &origin.resolved_path {
            details.append(&label(&format!(
                "Resolved target: {}",
                hyprland_editor_core::source::display_path_bytes(resolved)
            )));
        }
        for link in &origin.links {
            details.append(&label(&format!(
                "Recorded link: {} → {}",
                hyprland_editor_core::source::display_path_bytes(&link.encountered_path),
                hyprland_editor_core::source::display_path_bytes(&link.target)
            )));
        }
        details.append(&label(&format!("Source revision: {}", origin.sha256)));
        details.append(&label(&format!(
            "Recorded ownership: {:?} (not write authorization)",
            origin.recorded_ownership
        )));
        details.append(&label("The editor uses a private source snapshot. Verify Source File checks an explicitly selected original or identical copy."));
    }
    if let Some(observation) = observed {
        let raw = serde_json::to_string(&observation.raw_current).unwrap_or_default();
        let raw: String = raw.chars().take(2048).collect();
        details.append(&label(&format!("Desktop snapshot: {raw}")));
        details.append(&label(
            "Snapshot values may be stale. Refresh them from the menu.",
        ));
    }
    let source = gtk::LinkButton::with_label(&setting.source, "View Audited Source");
    source.set_halign(gtk::Align::Start);
    details.append(&source);
    advanced.set_child(Some(&details));
    panel.append(&advanced);
    let s = state.clone();
    let spec = setting.clone();
    let st = status.clone();
    let reset = unset.downgrade();
    let group = group.downgrade();
    stage.connect_clicked(move |button| {
        match getter().and_then(|v| {
            spec.validate(&v)?;
            Ok(v)
        }) {
            Ok(v) => {
                let mut state = s.borrow_mut();
                if state.draft.values.get(&spec.id) != Some(&v) {
                    if let Err(error) = state.remember(&spec.id) {
                        message.add_css_class("error");
                        message.set_text(&error);
                        message.set_visible(true);
                        return;
                    }
                    state.draft.values.insert(spec.id.clone(), v);
                }
                drop(state);
                message.remove_css_class("error");
                message.set_text("Draft updated");
                if let Some(group) = group.upgrade() {
                    group.set_description(Some("This value is in the draft"));
                }
                message.set_visible(true);
                button.set_label("Update Draft");
                if let Some(reset) = reset.upgrade() {
                    reset.set_visible(true);
                }
                summary(&s, &st);
            }
            Err(e) => {
                message.add_css_class("error");
                message.set_text(&e);
                message.set_visible(true);
            }
        }
    });
    let s = state.clone();
    let spec = setting.clone();
    let p = panel.downgrade();
    let st = status.clone();
    unset.connect_clicked(move |_| {
        let Some(p) = p.upgrade() else {
            return;
        };
        let mut state = s.borrow_mut();
        if state.draft.values.contains_key(&spec.id) {
            if let Err(error) = state.remember(&spec.id) {
                drop(state);
                st.set_text(&error);
                return;
            }
            state.draft.values.remove(&spec.id);
        }
        drop(state);
        summary(&s, &st);
        detail(&s, &p, &spec, &st);
    });
}

fn refill(
    state: &Shared,
    model: &gtk::StringList,
    search: &gtk::SearchEntry,
    version: &gtk::DropDown,
) {
    let selected = if version.selected() == 0 {
        BASELINE
    } else {
        AUDITED_MAIN
    };
    let query = search.text().to_lowercase();
    let mut s = state.borrow_mut();
    let version_changed = s.visible.first().is_some_and(|x| x.version != selected);
    s.visible = s
        .catalogue
        .iter()
        .filter(|x| {
            x.version == selected
                && s.category
                    .as_ref()
                    .is_none_or(|cat| x.id.split(':').next() == Some(cat.as_str()))
                && (x.id.to_lowercase().contains(&query)
                    || human_name(&x.id).to_lowercase().contains(&query)
                    || presentation::context(&x.id).to_lowercase().contains(&query))
        })
        .cloned()
        .collect();
    let labels: Vec<String> = s.visible.iter().map(|x| x.id.clone()).collect();
    drop(s);
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    // SearchEntry can report the same query twice; keep the selected editor alive.
    if version_changed
        || model.n_items() as usize != labels.len()
        || labels
            .iter()
            .enumerate()
            .any(|(i, id)| model.string(i as u32).as_deref() != Some(*id))
    {
        model.splice(0, model.n_items(), &labels);
    }
}
fn initial_state() -> Shared {
    Rc::new(RefCell::new(State {
        main_status: gtk::glib::WeakRef::new(),
        catalogue: catalogue(),
        draft: Draft::new(BASELINE),
        undo: history::History::default(),
        drafts: BTreeMap::new(),
        visible: vec![],
        category: None,
        sources: BTreeMap::new(),
        legacy_sources: BTreeMap::new(),
        source_origins: BTreeMap::new(),
        source_graphs: BTreeMap::new(),
        saved_source_graphs: BTreeMap::new(),
        saved_source_origins: BTreeMap::new(),
        active: None,
        generation: 0,
        saved: BTreeMap::new(),
        monitors: BTreeMap::new(),
        saved_monitors: BTreeMap::new(),
        devices: BTreeMap::new(),
        saved_devices: BTreeMap::new(),
        bindings: BTreeMap::new(),
        saved_bindings: BTreeMap::new(),
        gestures: BTreeMap::new(),
        saved_gestures: BTreeMap::new(),
        animations: BTreeMap::new(),
        saved_animations: BTreeMap::new(),
        rules: BTreeMap::new(),
        saved_rules: BTreeMap::new(),
        workspace_rules: BTreeMap::new(),
        saved_workspace_rules: BTreeMap::new(),
    }))
}
fn build(app: &adw::Application) -> adw::ApplicationWindow {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Hyprland Editor · Draft workspace")
        .default_width(1100)
        .default_height(760)
        .build();
    let state = initial_state();
    let open = gtk::Button::builder()
        .icon_name("document-open-symbolic")
        .tooltip_text("Import Configuration (Ctrl+O)")
        .build();
    let save = gtk::Button::with_label("Save Draft");
    save.set_tooltip_text(Some(
        "Save a new backup in the editor's state folder (Ctrl+S)",
    ));
    let save_as = gtk::Button::with_label("Save Project As…");
    let review = gtk::Button::with_label("Review Draft");
    review.set_tooltip_text(Some("Review and Export Draft (Ctrl+Return)"));
    let review_child = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    review_child.append(&gtk::Image::from_icon_name("document-properties-symbolic"));
    let review_label = gtk::Label::new(Some("Review Draft"));
    review_child.append(&review_label);
    review.update_property(&[gtk::accessible::Property::Label("Review Draft")]);
    review.set_child(Some(&review_child));
    let undo = gtk::Button::with_label("Undo");
    let coverage = gtk::Button::with_label("Feature Coverage");
    undo.set_sensitive(false);
    review.set_sensitive(false);
    let version = gtk::DropDown::from_strings(&["Hyprland 0.56.2", "Development Snapshot"]);
    version.set_widget_name("compatibility-version");
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search settings")
        .build();
    search.set_widget_name("settings-search");
    let mut categories: Vec<String> = state
        .borrow()
        .catalogue
        .iter()
        .map(|s| s.id.split(':').next().unwrap().into())
        .collect();
    categories.sort_by_key(|c| presentation::category_name(c));
    categories.dedup();
    let mut names = vec!["All Settings".into()];
    names.extend(categories.iter().map(|c| presentation::category_name(c)));
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let category = gtk::DropDown::from_strings(&refs);
    category.set_widget_name("category-filter");
    category.set_tooltip_text(Some("Filter by Category"));
    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let sidebar_header = adw::HeaderBar::new();
    sidebar_header.pack_start(&open);
    sidebar_box.append(&sidebar_header);
    let filters = gtk::Box::new(gtk::Orientation::Vertical, 8);
    filters.set_margin_start(12);
    filters.set_margin_end(12);
    filters.set_margin_top(8);
    filters.set_margin_bottom(12);
    filters.append(&search);
    filters.append(&category);
    sidebar_box.append(&filters);
    let model = gtk::StringList::new(&[]);
    let selection = gtk::SingleSelection::new(Some(model.clone()));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
        row.set_margin_top(10);
        row.set_margin_bottom(10);
        row.set_margin_start(16);
        row.set_margin_end(16);
        let name = gtk::Label::builder()
            .xalign(0.)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let context = gtk::Label::builder()
            .xalign(0.)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        context.add_css_class("caption");
        context.add_css_class("dim-label");
        row.append(&name);
        row.append(&context);
        item.set_child(Some(&row));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let id = item
            .item()
            .and_downcast::<gtk::StringObject>()
            .unwrap()
            .string();
        let row = item.child().and_downcast::<gtk::Box>().unwrap();
        let first = row.first_child().and_downcast::<gtk::Label>().unwrap();
        first.set_text(&human_name(&id));
        first.set_tooltip_text(Some(&human_name(&id)));
        first
            .next_sibling()
            .and_downcast::<gtk::Label>()
            .unwrap()
            .set_text(&presentation::context(&id));
    });
    let list = gtk::ListView::new(Some(selection.clone()), Some(factory));
    list.add_css_class("navigation-sidebar");
    list.set_single_click_activate(true);
    let list_scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();
    let no_results = label("No matching settings. Try another search or category.");
    no_results.set_margin_start(16);
    no_results.set_margin_end(16);
    no_results.set_margin_top(16);
    no_results.add_css_class("dim-label");
    sidebar_box.append(&no_results);
    let empty_label = no_results.downgrade();
    model.connect_items_changed(move |model, _, _, _| {
        if let Some(label) = empty_label.upgrade() {
            label.set_visible(model.n_items() == 0);
        }
    });
    sidebar_box.append(&list_scroll);
    let scope = gtk::Label::builder()
        .label("Hyprland 0.56.2 · Draft Mode")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    scope.add_css_class("caption");
    scope.add_css_class("dim-label");
    sidebar_box.append(&scope);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    content.append(&header);
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 24);
    panel.set_margin_start(24);
    panel.set_margin_end(24);
    panel.set_margin_top(32);
    panel.set_margin_bottom(32);
    presentation::empty(
        &panel,
        "Choose a Setting",
        "Search or select a setting to start editing a draft.",
    );
    let clamp = adw::Clamp::builder()
        .child(&panel)
        .maximum_size(660)
        .tightening_threshold(500)
        .build();
    let detail_scroll = gtk::ScrolledWindow::builder()
        .child(&clamp)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();
    content.append(&detail_scroll);
    let status = gtk::Label::builder()
        .xalign(0.)
        .margin_start(24)
        .margin_end(24)
        .margin_top(10)
        .margin_bottom(10)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    status.add_css_class("caption");
    state.borrow_mut().main_status = status.downgrade();
    status.add_css_class("dim-label");
    content.append(&status);
    let sidebar_page = adw::NavigationPage::new(&sidebar_box, "Hyprland Editor");
    let content_page = adw::NavigationPage::new(&content, "Settings");
    let split = adw::NavigationSplitView::builder()
        .sidebar(&sidebar_page)
        .content(&content_page)
        .min_sidebar_width(250.)
        .max_sidebar_width(300.)
        .sidebar_width_fraction(0.28)
        .show_content(false)
        .build();
    let menu = presentation::menu_model();
    let menu_button = presentation::menu_button(&menu);
    header.pack_end(&menu_button);
    header.pack_end(&review);
    let sidebar_menu = presentation::menu_button(&menu);
    sidebar_menu.set_visible(false);
    sidebar_header.pack_end(&sidebar_menu);
    let breakpoint =
        adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 700sp").unwrap());
    breakpoint.add_setters(&[(&split, "collapsed", true)]);
    breakpoint.add_setters(&[(&sidebar_menu, "visible", true)]);
    breakpoint.add_setters(&[(&review_label, "visible", false)]);
    window.add_breakpoint(breakpoint);
    window.set_size_request(360, 480);
    window.set_content(Some(&split));
    presentation::bind_feedback(&status, &state, &undo, &review, &scope);
    presentation::connect_compatibility(&window, &version);
    refill(&state, &model, &search, &version);
    summary(&state, &status);
    let s = state.clone();
    let m = model.clone();
    let q = search.downgrade();
    let v = version.downgrade();
    category.connect_selected_notify(move |category| {
        s.borrow_mut().category = if category.selected() == 0 {
            None
        } else {
            categories.get(category.selected() as usize - 1).cloned()
        };
        if let (Some(q), Some(v)) = (q.upgrade(), v.upgrade()) {
            refill(&s, &m, &q, &v);
        }
    });
    let s = state.clone();
    let p = panel.clone();
    let st = status.clone();
    let nav = split.downgrade();
    selection.connect_selected_notify(move |selection| {
        let setting = s
            .borrow()
            .visible
            .get(selection.selected() as usize)
            .cloned();
        if let Some(setting) = setting {
            detail(&s, &p, &setting, &st);
            if let Some(nav) = nav.upgrade() {
                nav.set_show_content(true);
            }
        } else if s.borrow().visible.is_empty() {
            presentation::empty(
                &p,
                "No Matching Settings",
                "Try another search or category.",
            );
        } else {
            presentation::empty(
                &p,
                "Choose a Setting",
                "Search or select a setting to start editing a draft.",
            );
        }
    });
    // Reopening the selected row after Back must not depend on selection changing.
    let nav = split.downgrade();
    list.connect_activate(move |_, _| {
        if let Some(nav) = nav.upgrade() {
            nav.set_show_content(true);
        }
    });
    let s = state.clone();
    let m = model.clone();
    let v = version.downgrade();
    search.connect_search_changed(move |search| {
        if let Some(v) = v.upgrade() {
            refill(&s, &m, search, &v);
        }
    });
    let s = state.clone();
    let m = model.clone();
    let search_c = search.downgrade();
    let st = status.clone();
    let p = panel.clone();
    version.connect_selected_notify(move |v| {
        let name = if v.selected() == 0 {
            BASELINE
        } else {
            AUDITED_MAIN
        };
        if s.borrow().draft.version != name {
            let mut s = s.borrow_mut();
            let old = s.draft.clone();
            s.drafts.insert(old.version.clone(), old);
            s.draft = s
                .drafts
                .get(name)
                .cloned()
                .unwrap_or_else(|| Draft::new(name));
            s.undo.clear();
            s.generation += 1;
        }
        if let Some(search) = search_c.upgrade() {
            refill(&s, &m, &search, v);
        }
        presentation::empty(
            &p,
            "Choose a Setting",
            "Each Hyprland version keeps a separate draft.",
        );
        summary(&s, &st);
    });
    let s = state.clone();
    let sel = selection.clone();
    let p = panel.clone();
    let st = status.clone();
    undo.connect_clicked(move |_| {
        let mut state = s.borrow_mut();
        state.undo_last();
        let spec = state.visible.get(sel.selected() as usize).cloned();
        drop(state);
        if let Some(spec) = spec {
            detail(&s, &p, &spec, &st);
        }
        summary(&s, &st);
    });
    workflow::connect_save(&save, &window, &state, &status);
    project_save::connect(&save_as, &window, &state, &status);
    let s = state.clone();
    let win = window.downgrade();
    review.connect_clicked(move |_|{
        let Some(win) = win.upgrade() else { return; };
        let state=s.borrow();let result=state.export_lua();match result{Ok(lua)=>{
            text_export::review(&win,"Review draft overrides","Review the current draft overrides, then save a separate Lua copy or copy the text. This export contains the reviewed snapshot.","hyprland-overrides.lua",lua);
        },Err(e)=>notice(&win,"Invalid draft",&e)}
    });
    workflow::connect_import(
        &open, &window, &state, &status, &version, &selection, &panel,
    );
    let live = gtk::Button::with_label("Refresh Desktop Values");
    let source_copy = gtk::Button::with_label("Review Source Copy");
    let diagnostics = gtk::Button::with_label("Import Diagnostics");
    let source_modules = gtk::Button::with_label("Inspect Source Modules");
    let prepare_preview = gtk::Button::with_label("Prepare Preview Draft");
    preview_preparation::connect(&prepare_preview, &window, &state);
    source_modules::connect(&source_modules, &window, &state);
    let reattach = gtk::Button::with_label("Verify Source File");
    let reconcile = gtk::Button::with_label("Compare Changed Source");
    let binding_editor = gtk::Button::with_label("Edit Bindings");
    bindings::connect(&binding_editor, &window, &state, &status);
    let gesture_editor = gtk::Button::with_label("Gestures");
    gestures::connect(&gesture_editor, &window, &state, &status);
    let animation_editor = gtk::Button::with_label("Animations");
    animations::connect(&animation_editor, &window, &state, &status);
    let rule_editor = gtk::Button::with_label("Rules");
    rules::connect(&rule_editor, &window, &state, &status);
    let workspace_editor = gtk::Button::with_label("Workspaces");
    workspaces::connect(&workspace_editor, &window, &state, &status);
    let device_editor = gtk::Button::with_label("Edit Devices");
    devices::connect(&device_editor, &window, &state, &status);
    let monitor_editor = gtk::Button::with_label("Edit Monitors");
    workflow::connect_live(&live, &window, &state, &status, &selection, &panel);
    workflow::connect_source_copy(&source_copy, &window, &state);
    workflow::connect_diagnostics(&diagnostics, &window, &state);
    workflow::connect_reattach(&reattach, &window, &state, &status);
    reconciliation::connect(&reconcile, &window, &state, &status, &selection, &panel);
    monitors::connect(&monitor_editor, &window, &state, &status);
    for (name, button) in [
        ("import", &open),
        ("save-draft", &save),
        ("save-project-as", &save_as),
        ("review", &review),
        ("undo", &undo),
        ("coverage", &coverage),
        ("read-live", &live),
        ("source-copy", &source_copy),
        ("diagnostics", &diagnostics),
        ("source-modules", &source_modules),
        ("prepare-preview", &prepare_preview),
        ("reattach-source", &reattach),
        ("reconcile-source", &reconcile),
        ("monitors", &monitor_editor),
        ("devices", &device_editor),
        ("bindings", &binding_editor),
        ("workspaces", &workspace_editor),
        ("rules", &rule_editor),
        ("animations", &animation_editor),
        ("gestures", &gesture_editor),
    ] {
        presentation::bridge(&window, name, button);
    }
    let mode = gtk::gio::SimpleAction::new("draft-mode", None);
    let win = window.downgrade();
    mode.connect_activate(move|_,_|{if let Some(win)=win.upgrade(){notice(&win,"Draft Mode","Changes stay in editor drafts. Desktop inspection is read-only. Applying changes and compositor preview are not available in this release.");}});
    window.add_action(&mode);
    compiler_check::connect(&window, &state);
    app.set_accels_for_action("win.import", &["<Primary>o"]);
    app.set_accels_for_action("win.save-draft", &["<Primary>s"]);
    app.set_accels_for_action("win.save-project-as", &["<Primary><Shift>s"]);
    app.set_accels_for_action("win.undo", &["<Primary>z"]);
    app.set_accels_for_action("win.review", &["<Primary>Return"]);
    let focus = gtk::gio::SimpleAction::new("search", None);
    let search_focus = search.downgrade();
    let nav = split.downgrade();
    focus.connect_activate(move |_, _| {
        if let Some(nav) = nav.upgrade() {
            nav.set_show_content(false);
        }
        if let Some(search) = search_focus.upgrade() {
            search.grab_focus();
        }
    });
    window.add_action(&focus);
    app.set_accels_for_action("win.search", &["<Primary>f"]);
    let win = window.downgrade();
    coverage.connect_clicked(move |_|{
        let Some(win) = win.upgrade() else { return; };
        let dialog=adw::Window::builder().title("Audited feature coverage").transient_for(&win).default_width(850).default_height(600).build();let b=gtk::Box::new(gtk::Orientation::Vertical,12);b.set_margin_top(16);b.set_margin_start(16);b.set_margin_end(16);b.set_margin_bottom(16);b.append(&label("1,453 original audit contracts plus 5 source-discovered requirements. Draft editors cover parts of these families; full acceptance remains unproven."));
        // Loaded on demand. A single text buffer avoids constructing 1,453 widgets.
        let data:serde_json::Value=serde_json::from_str(include_str!("../../../data/features.json")).unwrap();let mut text=String::new();for feature in data.as_array().unwrap(){text.push_str(&format!("{}  ·  {}  ·  {}\n",feature["id"].as_str().unwrap_or("?"),feature["engine"].as_str().unwrap_or("?"),feature["version_membership"]));}
        let additions:serde_json::Value=serde_json::from_str(include_str!("../../../data/source-additions.json")).unwrap();text.push_str("\nAdditional source-discovered requirements (full acceptance unproven)\n");for requirement in additions["requirements"].as_array().unwrap(){text.push_str(&format!("{}  ·  {}  ·  {}\n",requirement["id"].as_str().unwrap(),requirement["engine"].as_str().unwrap(),requirement["versions"]));}
        let view=gtk::TextView::builder().editable(false).monospace(true).build();view.buffer().set_text(&text);b.append(&gtk::ScrolledWindow::builder().child(&view).vexpand(true).build());dialog.set_content(Some(&b));dialog.present();
    });
    workflow::connect_close(&window, &state);
    window.present();
    window
}
fn main() {
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--check-preview-projection-selected"))
    {
        let args: Vec<_> = std::env::args_os().skip(2).collect();
        let result = if args.len() == 3 {
            projection_worker::check_selected(
                args[0].clone().into(),
                args[1].clone().into(),
                Some(args[2].clone().into()),
            )
        } else {
            Err("Specify workspace, scenario and scalar selection files".into())
        };
        match result {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("{}", error.chars().take(1024).collect::<String>());
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--check-preview-layers"))
    {
        let args: Vec<_> = std::env::args_os().skip(2).collect();
        let result = if args.len() == 1 {
            projection_worker::check_layers(args[0].clone().into())
        } else {
            Err("Specify one saved workspace for read-only layer review".into())
        };
        match result {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("{}", error.chars().take(1024).collect::<String>());
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--projection-policy-probe"))
    {
        projection_worker::policy_probe();
        return;
    }
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--projection-worker")) {
        projection_worker::entry();
        return;
    }
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--check-preview-projection"))
    {
        let args: Vec<_> = std::env::args_os().skip(2).collect();
        let result = if args.len() == 2 {
            projection_worker::check(args[0].clone().into(), args[1].clone().into())
        } else {
            Err("Specify a saved workspace and private scenario file".into())
        };
        match result {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("{}", error.chars().take(1024).collect::<String>());
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--directory-source-worker"))
    {
        directory_worker::entry();
        return;
    }
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--check-source-graph")) {
        use std::os::unix::ffi::OsStrExt;
        let args: Vec<_> = std::env::args_os().skip(2).collect();
        let result = if (3..=4).contains(&args.len()) {
            args[2]
                .to_str()
                .ok_or_else(|| "Source graph needs an audited UTF-8 version identifier".into())
                .and_then(|version| {
                    source_graph::check(
                        std::path::Path::new(&args[0]),
                        args[1].as_bytes(),
                        version.to_owned(),
                        args.get(3).map(|cwd| cwd.as_bytes().to_vec()),
                    )
                })
        } else {
            Err("--check-source-graph requires an approved folder, root file, audited version and optional captured working directory".into())
        };
        match result {
            Ok(value) => println!("{value}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if matches!(std::env::args_os().nth(1).as_deref(),Some(flag) if flag==std::ffi::OsStr::new("--check-source-folder") || flag==std::ffi::OsStr::new("--probe-source-folder"))
    {
        use std::os::unix::ffi::OsStrExt;
        let args: Vec<_> = std::env::args_os().skip(2).collect();
        let result = if args.len() == 2 {
            if std::env::args_os().nth(1).as_deref()
                == Some(std::ffi::OsStr::new("--probe-source-folder"))
            {
                directory_worker::check_probe(std::path::Path::new(&args[0]), args[1].as_bytes())
            } else {
                directory_worker::check(std::path::Path::new(&args[0]), args[1].as_bytes())
            }
        } else {
            Err(
                "Folder diagnostics require an explicitly selected folder and a candidate path"
                    .into(),
            )
        };
        match result {
            Ok(value) => println!("{value}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--capture-source-worker"))
    {
        capture_worker::entry();
        return;
    }
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--lua-source-worker")) {
        worker::entry();
        return;
    }
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--check-workspace-migration"))
    {
        let args: Vec<_> = std::env::args_os().skip(2).collect();
        let result = if args.len() == 1 {
            graph_migration::check(args[0].clone().into())
        } else {
            Err("Specify one saved .hye workspace for read-only migration review".into())
        };
        match result {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("{}", error.chars().take(1024).collect::<String>());
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--check-workspace")) {
        let result = std::env::args_os()
            .nth(2)
            .ok_or_else(|| "Specify a saved .hye workspace".to_string())
            .and_then(|path| workflow::check_workspace(path.into()));
        match result {
            Ok(report) => println!("{}", report),
            Err(error) => {
                eprintln!("{}", error.chars().take(1024).collect::<String>());
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--check-source-comparison"))
    {
        let args: Vec<_> = std::env::args_os().skip(2).collect();
        let result = if args.len() == 2 {
            reconciliation::check(args[0].clone().into(), args[1].clone().into())
        } else {
            Err("Specify a saved .hye workspace and an explicitly selected Lua source".into())
        };
        match result {
            Ok(report) => println!("{}", report),
            Err(error) => {
                eprintln!("{}", error.chars().take(1024).collect::<String>());
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--check-preview-delegation"))
    {
        let result = (|| {
            use std::os::{fd::AsFd, unix::fs::OpenOptionsExt};
            let args = std::env::args_os().skip(2).collect::<Vec<_>>();
            if args.len() != 1 {
                return Err("Specify one explicitly selected delegation directory".to_string());
            }
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&args[0])
                .map_err(|e| e.to_string())?;
            let delegation =
                hyprland_editor_preview::cgroup::Delegation::from_directory(file.as_fd())
                    .map_err(|e| e.to_string())?;
            let observation = delegation.observe().map_err(|e| e.to_string())?;
            serde_json::to_string(&observation).map_err(|e| e.to_string())
        })();
        match result {
            Ok(value) => println!("{value}"),
            Err(error) => {
                eprintln!("{}", error.chars().take(1024).collect::<String>());
                std::process::exit(1);
            }
        }
        return;
    }
    let app = adw::Application::builder()
        .application_id("io.github.aman696.HyprlandEditor")
        .build();
    app.connect_activate(|app| {
        build(app);
    });
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    fn descendants(widget: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut out = vec![];
        let mut child = widget.first_child();
        while let Some(w) = child {
            out.push(w.clone());
            out.extend(descendants(&w));
            child = w.next_sibling();
        }
        out
    }
    fn click(window: &adw::ApplicationWindow, text: &str) {
        let button = descendants(window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(text))
            .unwrap();
        button.emit_clicked();
    }
    #[test]
    #[ignore = "requires a graphical display; runs no compositor commands"]
    fn native_forms_and_draft_workflow() {
        adw::init().unwrap();
        // Construct every version-scoped form, including types outside the first visible page.
        for setting in catalogue() {
            let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let value = setting.known_default();
            let getter = form(&setting, value.clone(), &panel);
            let actual = getter().unwrap();
            if let Some(expected) = value {
                assert_eq!(actual, expected, "{} {}", setting.version, setting.id);
            }
            setting
                .validate(&actual)
                .unwrap_or_else(|e| panic!("{}: {e}", setting.id));
        }
        let app = adw::Application::builder()
            .application_id("io.github.aman696.HyprlandEditor.Test")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let window = build(&app);
        if std::env::var("HYE_PENDING_INPUTS_ONLY").as_deref() == Ok("1") {
            source_pending::test_workflow(&window);
            window.destroy();
            return;
        }
        if std::env::var("HYE_EXTERNAL_MERGE_ONLY").as_deref() == Ok("1") {
            graph_transaction_tests::test_external_merge_ui(&window);
            window.destroy();
            return;
        }
        let widgets = descendants(window.upcast_ref());
        let list = widgets
            .iter()
            .filter_map(|w| w.clone().downcast::<gtk::ListView>().ok())
            .find(|list| list.model().is_some_and(|m| m.n_items() == 353))
            .unwrap();
        let selection = list.model().and_downcast::<gtk::SingleSelection>().unwrap();
        assert_eq!(selection.n_items(), 353);
        selection.set_selected(0);
        assert!(!window.lookup_action("review").unwrap().is_enabled());
        click(&window, "Add to Draft");
        assert!(window.lookup_action("review").unwrap().is_enabled());
        window
            .lookup_action("compatibility")
            .unwrap()
            .activate(None);
        let dialog = gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Window>().ok())
            .find(|w| w.title().as_deref() == Some("Hyprland Version"))
            .unwrap();
        let dropdown = descendants(dialog.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == "version-picker")
            .unwrap()
            .downcast::<gtk::DropDown>()
            .unwrap();
        dropdown.set_selected(1);
        assert_eq!(selection.n_items(), 402);
        assert!(!window.lookup_action("review").unwrap().is_enabled());
        dropdown.set_selected(0);
        dialog.destroy();
        selection.set_selected(0);
        click(&window, "Remove Override");
        window.lookup_action("undo").unwrap().activate(None);
        let labels: Vec<String> = descendants(window.upcast_ref())
            .iter()
            .filter_map(|w| w.clone().downcast::<gtk::Label>().ok())
            .map(|l| l.text().into())
            .collect();
        assert!(labels.iter().any(|s| s.contains("1 setting in draft")));
        let advanced = descendants(window.upcast_ref())
            .into_iter()
            .find_map(|w| w.downcast::<gtk::Expander>().ok())
            .unwrap();
        assert!(!advanced.is_expanded());
        let category = widgets
            .iter()
            .find(|w| w.widget_name() == "category-filter")
            .unwrap()
            .clone()
            .downcast::<gtk::DropDown>()
            .unwrap();
        category.set_selected(1);
        assert!(selection.n_items() > 0 && selection.n_items() < 353);
        category.set_selected(0);
        assert_eq!(selection.n_items(), 353);
        let search = widgets
            .iter()
            .find(|w| w.widget_name() == "settings-search")
            .unwrap()
            .clone()
            .downcast::<gtk::SearchEntry>()
            .unwrap();
        search.set_text("rounding");
        search.emit_by_name::<()>("search-changed", &[]);
        assert!(selection.n_items() > 0 && selection.n_items() < 353);
        search.set_text("no-such-setting-test");
        search.emit_by_name::<()>("search-changed", &[]);
        assert_eq!(selection.n_items(), 0);
        search.set_text("border size");
        search.emit_by_name::<()>("search-changed", &[]);
        assert_eq!(selection.n_items(), 1);
        selection.set_selected(0);
        search.set_text("");
        search.emit_by_name::<()>("search-changed", &[]);
        let category_model = category.model().unwrap();
        let general = (0..category_model.n_items())
            .find(|i| {
                category_model
                    .item(*i)
                    .and_downcast::<gtk::StringObject>()
                    .is_some_and(|s| s.string() == "General")
            })
            .unwrap();
        category.set_selected(general);
        let border = (0..selection.n_items())
            .find(|i| {
                selection
                    .item(*i)
                    .and_downcast::<gtk::StringObject>()
                    .is_some_and(|s| s.string() == "general:border_size")
            })
            .unwrap();
        selection.set_selected(border);
        let split = window
            .content()
            .and_downcast::<adw::NavigationSplitView>()
            .unwrap();
        let context = gtk::glib::MainContext::default();
        let settle = || {
            for _ in 0..50 {
                while context.pending() {
                    context.iteration(false);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };
        settle();
        let capture = |path: &str| {
            let paintable = gtk::WidgetPaintable::new(Some(&window));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
            let node = snapshot.to_node().expect("rendered window");
            window
                .renderer()
                .unwrap()
                .render_texture(&node, None)
                .save_to_png(path)
                .unwrap();
        };
        if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
            capture(&path);
        }
        // Exercise adaptive navigation even when a tiling compositor controls the width.
        split.set_collapsed(true);
        settle();
        assert!(split.shows_content());
        window.lookup_action("search").unwrap().activate(None);
        settle();
        assert!(!split.shows_content());
        list.emit_by_name::<()>("activate", &[&selection.selected()]);
        settle();
        assert!(split.shows_content(), "selected row must reopen after Back");
        split.set_collapsed(false);
        // Xvfb permits exact resizing, whereas a tiled surface may ignore the request.
        if std::env::var_os("HYPRLAND_EDITOR_TEST_EXACT_RESIZE").is_some() {
            window.set_default_size(360, 720);
            settle();
            assert!(
                split.is_collapsed(),
                "narrow windows must use one-page navigation"
            );
            assert!(
                window.width() <= 360,
                "controls must not force a wider window"
            );
            if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
                capture(&format!("{path}.narrow.png"));
            }
        }
        if std::env::var_os("HYPRLAND_EDITOR_TEST_EXACT_RESIZE").is_some() {
            category.set_selected(0);
            let mut kinds = std::collections::BTreeSet::new();
            for setting in catalogue().into_iter().filter(|s| s.version == BASELINE) {
                if !kinds.insert(setting.kind.clone()) {
                    continue;
                }
                let index = (0..selection.n_items())
                    .find(|i| {
                        selection
                            .item(*i)
                            .and_downcast::<gtk::StringObject>()
                            .is_some_and(|s| s.string() == setting.id)
                    })
                    .unwrap();
                selection.set_selected(index);
                settle();
                let (minimum, _, _, _) = split.measure(gtk::Orientation::Horizontal, -1);
                assert!(
                    minimum <= split.width(),
                    "{} form needs {minimum}px, only {} available",
                    setting.kind,
                    split.width()
                );
            }
        }
        window.lookup_action("review").unwrap().activate(None);
        let review = gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Window>().ok())
            .find(|w| w.title().as_deref() == Some("Review draft overrides"))
            .unwrap();
        assert!(descendants(review.upcast_ref())
            .iter()
            .any(|w| w.is::<gtk::TextView>()));
        for label in ["Copy Lua", "Save Lua File…"] {
            assert!(
                descendants(review.upcast_ref()).iter().any(|widget| {
                    widget
                        .downcast_ref::<gtk::Button>()
                        .is_some_and(|button| button.label().as_deref() == Some(label))
                }),
                "Missing export action: {label}"
            );
        }
        let (minimum, _, _, _) = review
            .child()
            .unwrap()
            .measure(gtk::Orientation::Horizontal, -1);
        assert!(minimum <= 350, "Lua export review needs {minimum}px");
        review.destroy();
        workflow::test_source_persistence(&window);
        workflow::test_combined_source_copy(&window);
        workflow::test_legacy_persistence(&window);
        workflow::test_graph_migration(&window);
        compiler_check::test_ui(&window);
        preview_preparation::test_preparation(&window);
        reconciliation::test_review(&window);
        monitors::test_editor(&window);
        monitors::test_detected_outputs(&window);
        devices::test_editor(&window);
        devices::test_detected_devices(&window);
        bindings::test_editor(&window);
        workspaces::test_editor(&window);
        rules::test_editor(&window);
        animations::test_editor(&window);
        gestures::test_editor(&window);
        source_modules::test_view(&window);
        source_access::test_retained_graph(&window);
        source_access::test_declaration_families(&window);
        graph_transaction_tests::test_private_graph_ui(&window);
        source_pending::test_workflow(&window);
        graph_transaction_tests::test_external_merge_ui(&window);
        source_compound_forms::test_workflow(&window);
        source_changes::test_monitor_workflow(&window);
        source_monitors::test_workflow(&window);
        source_declarations::test_workflow(&window);
        rule_forms::test_forms();
        window.close();
        assert!(
            window.is_visible(),
            "unsaved drafts must prevent immediate closure"
        );
        window.destroy();
    }
}
