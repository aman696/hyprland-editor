//! Fresh typed-draft projection. Never copies executable source, reads paths or
//! talks to a compositor. A projection and its content identity authorize no run.
use editor_core::{action, gesture, monitor, workspace::Workspace, Value, MAX_DOCUMENT_BYTES};
use hyprland_editor_core as editor_core;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_ENTRIES: usize = 16384;
pub const MAX_PIXELS: u64 = 1280 * 720;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub name: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Requested private names; the future launcher must establish/verify them.
    #[serde(deserialize_with = "wire::outputs")]
    pub outputs: Vec<Output>,
    /// Explicit draft ID -> scenario output. No host-pattern inference.
    #[serde(deserialize_with = "wire::targets")]
    pub monitor_targets: BTreeMap<String, String>,
    /// Explicit draft ID -> private synthetic device name.
    #[serde(deserialize_with = "wire::targets")]
    pub device_targets: BTreeMap<String, String>,
}
impl Default for Scenario {
    fn default() -> Self {
        Self {
            outputs: vec![Output {
                name: "HEADLESS-1".into(),
                width: 1280,
                height: 720,
            }],
            monitor_targets: BTreeMap::new(),
            device_targets: BTreeMap::new(),
        }
    }
}
impl Scenario {
    fn validate(&self, workspace: &Workspace) -> Result<(), String> {
        if self.outputs.is_empty() || self.outputs.len() > 4 {
            return Err("Preview scenario needs one to four private outputs".into());
        }
        let mut names = BTreeSet::new();
        let mut pixels = 0;
        for output in &self.outputs {
            if !output.name.strip_prefix("HEADLESS-").is_some_and(|n| {
                !n.is_empty() && n.len() <= 3 && n.bytes().all(|b| b.is_ascii_digit())
            }) || !names.insert(output.name.as_str())
                || output.width < 64
                || output.height < 64
                || output.width > 1280
                || output.height > 1280
            {
                return Err("Invalid or duplicate private output geometry/name".into());
            }
            pixels += u64::from(output.width) * u64::from(output.height);
        }
        if pixels > MAX_PIXELS {
            return Err("Preview scenario exceeds the aggregate 720p pixel budget".into());
        }
        for (id, target) in &self.monitor_targets {
            if !workspace.monitors.iter().any(|m| &m.id == id) || !names.contains(target.as_str()) {
                return Err("Stale monitor mapping or unknown private output".into());
            }
        }
        for (id, target) in &self.device_targets {
            if !workspace.devices.iter().any(|d| &d.id == id)
                || target.len() > 64
                || !target.strip_prefix("hye-lab-").is_some_and(|s| {
                    !s.is_empty()
                        && s.bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                })
            {
                return Err("Stale device mapping or invalid synthetic device name".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fidelity {
    SampleApproximation,
    MockedEffect,
    OmittedUntrusted,
    HardwareUnverified,
    Unsupported,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub subject: String,
    pub fidelity: Fidelity,
    pub reason: String,
    /// Constructor/builtin label only; never its raw command or hook arguments.
    pub mocked_intent: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub version: String,
    pub upstream_commit: String,
    /// Binds omitted and mocked inputs without disclosing their raw payloads.
    pub workspace_sha256: String,
    pub model_sha256: String,
    pub compiler: editor_core::lua::replay::CompilerProfile,
    #[serde(deserialize_with = "wire::pins")]
    pub schema_pins: BTreeMap<String, String>,
    pub scenario: Scenario,
    #[serde(deserialize_with = "wire::entries")]
    pub entries: Vec<Entry>,
    #[serde(deserialize_with = "wire::limitations")]
    pub limitations: Vec<String>,
}
#[derive(Debug)]
pub struct Projection {
    lua: String,
    report: Report,
    identity: String,
}
impl Projection {
    pub fn lua(&self) -> &str {
        &self.lua
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
}
struct Builder {
    lua: String,
    entries: Vec<Entry>,
}
impl Builder {
    fn record(
        &mut self,
        subject: String,
        fidelity: Fidelity,
        reason: &'static str,
    ) -> Result<(), String> {
        if self.entries.len() >= MAX_ENTRIES {
            return Err("Preview report exceeds its entry budget".into());
        }
        self.entries.push(Entry {
            subject,
            fidelity,
            reason: reason.into(),
            mocked_intent: None,
        });
        Ok(())
    }
    fn append(&mut self, lua: &str) -> Result<(), String> {
        if self.lua.len().saturating_add(lua.len()) > MAX_DOCUMENT_BYTES {
            return Err("Preview configuration exceeds 1 MiB".into());
        }
        self.lua.push_str(lua);
        Ok(())
    }
    fn mock(
        &mut self,
        action: &mut action::Action,
        version: &str,
        subject: String,
    ) -> Result<(), String> {
        self.record(subject, Fidelity::MockedEffect, "Action replaced by an inert no-op; no command, session, input or process effect is executed; mock telemetry remains unavailable")?;
        self.entries.last_mut().unwrap().mocked_intent = Some(action.constructor.clone());
        let at_load = action.at_load;
        *action = action::Action::new(
            action::find("no_op", version).ok_or("Missing pinned inert action")?,
        );
        action.at_load = at_load;
        Ok(())
    }
    fn workflow(
        &mut self,
        workflow: &mut gesture::Workflow,
        version: &str,
        subject: &str,
    ) -> Result<(), String> {
        // Source-chosen local parameter names are unnecessary for no-op actions.
        workflow.event_parameter = None;
        for (index, action) in workflow.actions.iter_mut().enumerate() {
            self.mock(action, version, format!("{subject}/action/{index}"))?;
        }
        Ok(())
    }
}

/// Staging-worker API: validates typed inputs, preserves declaration order and
/// returns inert bytes/report only. It does not resolve imported source layers,
/// create outputs/devices, establish a sandbox or certify runtime semantics.
pub fn compile(workspace: &Workspace, scenario: &Scenario) -> Result<Projection, String> {
    let catalogue = editor_core::catalogue();
    let workspace_sha256 = {
        let bytes = workspace.to_json(&catalogue)?;
        editor_core::lua::source_hash(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
    };
    scenario.validate(workspace)?;
    let version = workspace.draft.version.as_str();
    let mut builder = Builder {
        lua: "-- Hyprland Editor private typed-draft projection; execution is not authorized.\n"
            .into(),
        entries: vec![],
    };
    builder.record("source-layers".into(), Fidelity::Unsupported, "Only explicit typed draft values are projected; source/override resolution and effective defaults remain separate required gates")?;
    for source in &workspace.sources {
        builder.record(
            format!("source/{}", source.id),
            Fidelity::OmittedUntrusted,
            "Executable source bytes, modules, hooks and plugins are never copied",
        )?;
    }
    if workspace.source_graph.is_some() {
        builder.record("source-graph".into(), Fidelity::OmittedUntrusted, "Captured graph and private revisions are not executed; complete layer projection remains required")?;
    }
    // Baseline topology comes only from the bounded scenario, not hardware modes.
    let mut x = 0;
    for output in &scenario.outputs {
        let mut monitor = monitor::Rule::new("hye-scenario");
        monitor.output = monitor::Output::Connector(output.name.clone());
        monitor.fields.insert(
            "mode".into(),
            Value::String(format!("{}x{}@60", output.width, output.height)),
        );
        monitor
            .fields
            .insert("position".into(), Value::String(format!("{x}x0")));
        monitor
            .fields
            .insert("scale".into(), Value::String("1".into()));
        builder.append(&monitor.export_lua(version)?)?;
        x += output.width;
    }
    let mut scalars = editor_core::Draft::new(version);
    for (id, value) in &workspace.draft.values {
        let fidelity = if matches!(id.as_str(), "decoration:screen_shader" | "input:kb_file") {
            Fidelity::OmittedUntrusted
        } else if matches!(
            id.as_str(),
            "input:touchdevice:output" | "input:tablet:output" | "cursor:default_monitor"
        ) {
            Fidelity::Unsupported
        } else {
            Fidelity::SampleApproximation
        };
        builder.record(format!("scalar/{id}"), fidelity, match fidelity {
            Fidelity::OmittedUntrusted => "Opaque shader/keymap assets are excluded; reviewed copied/generated assets remain required",
            Fidelity::Unsupported => "Host output reference requires explicit scenario identity mapping",
            _ => "Typed scalar emitted; native constraints, dependencies and actual effects require runtime validation",
        })?;
        if fidelity == Fidelity::SampleApproximation {
            scalars.values.insert(id.clone(), value.clone());
        }
    }
    builder.append("hl.config({\n")?;
    for (id, value) in &scalars.values {
        let setting = catalogue
            .iter()
            .find(|s| s.version == version && &s.id == id)
            .ok_or("Unknown projection scalar")?;
        builder.append(&format!(
            "    [{}] = {},\n",
            Value::String(setting.lua_key.clone()).lua(),
            value.lua()
        ))?;
    }
    builder.append("})\n")?;
    for original in &workspace.monitors {
        let Some(target) = scenario.monitor_targets.get(&original.id) else {
            builder.record(
                format!("monitor/{}", original.id),
                Fidelity::Unsupported,
                "Select a private scenario output before projecting this monitor declaration",
            )?;
            continue;
        };
        let mut projected = original.clone();
        projected.output = monitor::Output::Connector(target.clone());
        for key in [
            "mode",
            "modeline",
            "position",
            "scale",
            "transform",
            "mirror",
            "icc",
            "disabled",
        ] {
            if projected.fields.remove(key).is_some() {
                builder.record(format!("monitor/{}/{key}", original.id), Fidelity::HardwareUnverified, "Physical mode/topology or external asset omitted in favor of bounded scenario topology")?;
            }
        }
        builder.record(format!("monitor/{}", original.id), Fidelity::HardwareUnverified, "Output explicitly remapped; remaining typed fields are requests without hardware/render certification")?;
        builder.append(&projected.export_lua(version)?)?;
    }
    for original in &workspace.devices {
        let Some(target) = scenario.device_targets.get(&original.id) else {
            builder.record(
                format!("device/{}", original.id),
                Fidelity::Unsupported,
                "Select a synthetic scenario device before projecting this declaration",
            )?;
            continue;
        };
        let mut projected = original.clone();
        projected.name = target.clone();
        for key in ["kb_file", "output"] {
            if projected.fields.remove(key).is_some() {
                builder.record(format!("device/{}/{key}", original.id), Fidelity::OmittedUntrusted, "External keymap or host output reference requires a reviewed private asset/mapping")?;
            }
        }
        builder.record(format!("device/{}", original.id), Fidelity::HardwareUnverified, "Device explicitly remapped; actual synthetic device creation and hardware capabilities remain unverified")?;
        builder.append(&projected.export_lua(version)?)?;
    }
    for original in &workspace.workspace_rules {
        let mut projected = original.clone();
        for key in ["on_created_empty", "monitor"] {
            if projected.fields.remove(key).is_some() {
                builder.record(
                    format!("workspace/{}/{key}", original.id),
                    Fidelity::OmittedUntrusted,
                    "Empty-workspace command hook or host output assignment omitted",
                )?;
            }
        }
        builder.record(format!("workspace/{}", original.id), Fidelity::SampleApproximation, "Ordered workspace declaration emitted; scenario matching/layout behavior requires validation")?;
        builder.append(&projected.export_lua(version)?)?;
    }
    for original in &workspace.rules {
        let mut projected = original.clone();
        projected.effects.retain(|effect| effect.key != "monitor");
        if projected.effects.len() != original.effects.len() {
            builder.record(
                format!("rule/{}/monitor", original.id),
                Fidelity::Unsupported,
                "Host monitor effect needs explicit scenario mapping",
            )?;
        }
        builder.record(format!("rule/{}", original.id), Fidelity::SampleApproximation, "Ordered predicates/named effects emitted for private synthetic clients; match/effect fidelity remains unverified")?;
        builder.append(&projected.export_lua(version)?)?;
    }
    builder.append(&workspace.animations.export_lua(version)?)?;
    for declaration in &workspace.animations.declarations {
        builder.record(
            format!("animation/{}", declaration.id),
            Fidelity::SampleApproximation,
            "Typed curve/node declaration emitted in order; renderer timing remains unverified",
        )?;
    }
    let mut bindings = workspace.bindings.clone();
    for submap in &bindings.submaps {
        builder.record(
            format!("submap/{}", submap.id),
            Fidelity::SampleApproximation,
            "Private submap declaration/order retained; transitions are inert action mocks",
        )?;
    }
    for bind in &mut bindings.binds {
        builder.record(
            format!("binding/{}", bind.id),
            Fidelity::SampleApproximation,
            "Chord/flags/order retained; action effects are separately reported inert mocks",
        )?;
        for (index, action) in bind.actions.iter_mut().enumerate() {
            builder.mock(
                action,
                version,
                format!("binding/{}/action/{index}", bind.id),
            )?;
        }
        if bind.device.take().is_some() {
            builder.record(
                format!("binding/{}/device", bind.id),
                Fidelity::Unsupported,
                "Physical device/tag targeting omitted in the private sample",
            )?;
        }
    }
    builder.append(&bindings.export_lua(version)?)?;
    let mut gestures = workspace.gestures.clone();
    for spec in &mut gestures.declarations {
        let subject = format!("gesture/{}", spec.id);
        builder.record(subject.clone(), Fidelity::SampleApproximation, "Gesture registration/identity emitted; synthetic input routing and native effects remain unverified")?;
        match &mut spec.action {
            gesture::GestureAction::Builtin(name) if name == "unset" => {}
            gesture::GestureAction::Builtin(name) => {
                let intent = format!("gesture.{name}");
                let mut workflow = gesture::Workflow {
                    actions: vec![action::Action::new(
                        action::find("no_op", version).ok_or("Missing pinned inert action")?,
                    )],
                    event_parameter: None,
                };
                builder.workflow(&mut workflow, version, &subject)?;
                builder.entries.last_mut().unwrap().mocked_intent = Some(intent);
                spec.action = gesture::GestureAction::LegacyFinish(workflow);
            }
            gesture::GestureAction::LegacyFinish(workflow) => {
                builder.workflow(workflow, version, &subject)?
            }
            gesture::GestureAction::Callbacks(callbacks) => {
                for (phase, workflow) in [
                    ("start", &mut callbacks.start),
                    ("update", &mut callbacks.update),
                    ("finish", &mut callbacks.finish),
                ] {
                    if let Some(workflow) = workflow {
                        builder.workflow(workflow, version, &format!("{subject}/{phase}"))?;
                    }
                }
            }
        }
    }
    builder.append(&gestures.export_lua(version)?)?;
    // Compile and independently reparse only newly generated bytes; never the
    // imported program. This checks syntax, not actual native effects.
    editor_core::lua::replay::validate_document(&editor_core::lua::replay::Document {
        id: "preview-projection".into(),
        source: builder.lua.clone(),
        sha256: editor_core::lua::source_hash(&builder.lua),
        load_name: b"@hye-preview-projection".to_vec(),
    })?;
    let report = Report { version: version.into(), upstream_commit: workspace.schema.upstream_commit.clone(), workspace_sha256,
        model_sha256: model_sha256(),
        compiler: editor_core::lua::replay::compiler_profile(),
        schema_pins: schema_pins(workspace),
        scenario: scenario.clone(), entries: builder.entries,
        limitations: vec!["No execution authority or ready-to-run certification", "Source layers/defaults are unresolved; this projects explicit typed drafts only", "Private outputs/devices must be created and verified by a future launcher", "All binding and gesture actions are inert mocks without telemetry", "Assets are not copied; opaque shaders, keymaps, ICC and hooks remain omitted", "Renderer, actual capture/input, resource/isolation/lifecycle and all full audit gates remain unverified"].into_iter().map(String::from).collect() };
    let manifest = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    if manifest.len() > 4 * MAX_DOCUMENT_BYTES {
        return Err("Preview report exceeds 4 MiB".into());
    }
    let identity = editor_core::lua::source_hash(&format!(
        "HYE-PROJECTION-1\0{}\0{}",
        builder.lua,
        String::from_utf8(manifest).map_err(|e| e.to_string())?
    ));
    Ok(Projection {
        lua: builder.lua,
        report,
        identity,
    })
}

/// Content/model pins only: these do not authenticate a package or authorize execution.
pub fn model_sha256() -> String {
    editor_core::lua::source_hash(concat!(
        include_str!("lib.rs"),
        include_str!("wire.rs"),
        include_str!("../../editor-core/src/lib.rs"),
        include_str!("../../editor-core/src/action.rs"),
        include_str!("../../editor-core/src/animation.rs"),
        include_str!("../../editor-core/src/binding.rs"),
        include_str!("../../editor-core/src/device.rs"),
        include_str!("../../editor-core/src/gesture.rs"),
        include_str!("../../editor-core/src/monitor.rs"),
        include_str!("../../editor-core/src/rule.rs"),
        include_str!("../../editor-core/src/workspace_rule.rs")
    ))
}
fn schema_pins(workspace: &Workspace) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("scalar", workspace.schema.catalogue_sha256.clone()),
        ("monitor", monitor::schema_hash()),
        ("device", editor_core::device::schema_hash()),
        ("action", action::schema_hash()),
        ("binding", editor_core::binding::schema_hash()),
        ("workspace", editor_core::workspace_rule::schema_hash()),
        ("rule", editor_core::rule::schema_hash()),
        ("animation", editor_core::animation::schema_hash()),
        ("gesture", gesture::schema_hash()),
    ])
    .into_iter()
    .map(|(key, value)| (key.into(), value))
    .collect()
}

pub mod wire;

#[cfg(test)]
mod tests;
