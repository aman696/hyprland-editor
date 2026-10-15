//! Inert gesture declarations, ordered registration simulation and finite callback workflows.
use crate::{
    action::Action, animation, quote_lua, Value, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const MIN_SCALE: f64 = 0.1_f32 as f64;
pub const MAX_GESTURES: usize = 1024;
pub const BUILTINS: &[&str] = &[
    "workspace",
    "resize",
    "move",
    "special",
    "close",
    "float",
    "fullscreen",
    "cursor_zoom",
    "cursorZoom",
    "scroll_move",
    "unset",
];
pub const DIRECTIONS: &[&str] = &[
    "swipe",
    "left",
    "right",
    "up",
    "down",
    "horizontal",
    "vertical",
    "pinch",
    "pinchin",
    "pinchout",
];
pub const MODIFIERS: &[(&str, u8)] = &[
    ("SHIFT", 1),
    ("CAPS", 2),
    ("CTRL", 4),
    ("ALT", 8),
    ("MOD2", 16),
    ("MOD3", 32),
    ("SUPER", 64),
    ("MOD5", 128),
];
pub fn schema_hash() -> String {
    crate::lua::source_hash(&format!(
        "{}{}",
        include_str!("../../../data/gesture-semantics.json"),
        crate::action::schema_hash()
    ))
}
fn text(s: &str) -> Result<(), String> {
    if s.len() > 16384 || s.contains('\0') {
        Err("Gesture text exceeds 16 KiB or contains NUL".into())
    } else {
        Ok(())
    }
}
fn string_field(v: &Value) -> Result<(), String> {
    match v {
        Value::String(s) => text(s),
        Value::Int(_) => Ok(()),
        Value::Float(n) if n.is_finite() => Ok(()),
        _ => Err("Gesture string field requires literal text or a finite number".into()),
    }
}
fn literal(v: &Value) -> String {
    let raw = v.lua();
    if matches!(v, Value::Float(_)) && !raw.contains(['.', 'e', 'E']) {
        format!("{raw}.0")
    } else {
        raw
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Swipe,
    Left,
    Right,
    Up,
    Down,
    Horizontal,
    Vertical,
    Pinch,
    PinchIn,
    PinchOut,
}
impl Direction {
    pub fn parse(s: &str) -> Result<Self, String> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "swipe" => Self::Swipe,
            "left" | "l" => Self::Left,
            "right" | "r" => Self::Right,
            "up" | "u" | "top" | "t" => Self::Up,
            "down" | "d" | "bottom" | "b" => Self::Down,
            "horizontal" | "horiz" => Self::Horizontal,
            "vertical" | "vert" => Self::Vertical,
            "pinch" => Self::Pinch,
            "pinchin" | "zoomin" => Self::PinchIn,
            "pinchout" | "zoomout" => Self::PinchOut,
            _ => return Err("Gesture direction is outside the pinned aliases".into()),
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Swipe => "swipe",
            Self::Left => "left",
            Self::Right => "right",
            Self::Up => "up",
            Self::Down => "down",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Pinch => "pinch",
            Self::PinchIn => "pinchin",
            Self::PinchOut => "pinchout",
        }
    }
    pub fn event_name(self) -> String {
        match self {
            Self::PinchIn => "PINCH_IN".into(),
            Self::PinchOut => "PINCH_OUT".into(),
            d => d.name().to_ascii_uppercase(),
        }
    }
    fn axis(self) -> Self {
        match self {
            Self::Left | Self::Right | Self::Horizontal => Self::Horizontal,
            Self::Up | Self::Down | Self::Vertical => Self::Vertical,
            Self::Pinch | Self::PinchIn | Self::PinchOut => Self::Pinch,
            _ => Self::Swipe,
        }
    }
    pub fn shadows(self, next: Self) -> bool {
        self == next
            || self == next.axis()
            || (self == Self::Swipe && matches!(next.axis(), Self::Horizontal | Self::Vertical))
    }
}
pub fn modifier_mask(value: Option<&Value>) -> Result<u8, String> {
    let Some(value) = value else {
        return Ok(0);
    };
    string_field(value)?;
    let Value::String(raw) = value else {
        return Ok(0);
    };
    if !raw.is_ascii() {
        return Err("Non-ASCII modifier normalization is unresolved".into());
    }
    let raw = raw.to_ascii_uppercase();
    let mut mask = 0;
    for (names, bit) in [
        (&["SHIFT"][..], 1),
        (&["CAPS"][..], 2),
        (&["CTRL", "CONTROL"][..], 4),
        (&["ALT", "MOD1"][..], 8),
        (&["MOD2"][..], 16),
        (&["MOD3"][..], 32),
        (&["SUPER", "WIN", "LOGO", "MOD4", "META"][..], 64),
        (&["MOD5"][..], 128),
    ] {
        if names.iter().any(|n| raw.contains(n)) {
            mask |= bit;
        }
    }
    Ok(mask)
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    pub actions: Vec<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_parameter: Option<String>,
}
impl Workflow {
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if let Some(p) = &self.event_parameter {
            if p.is_empty()
                || p.len() > 64
                || p == "hl"
                || !p.bytes().enumerate().all(|(i, c)| {
                    c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                })
                || [
                    "and", "break", "do", "else", "elseif", "end", "false", "for", "function",
                    "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then",
                    "true", "until", "while",
                ]
                .contains(&p.as_str())
            {
                return Err("Callback parameter must be a non-reserved Lua identifier".into());
            }
        }
        if self.actions.len() > 64 {
            return Err("Gesture phase supports at most 64 action blocks".into());
        }
        for a in &self.actions {
            a.validate(version)?;
        }
        Ok(())
    }
    fn function(&self, version: &str, names: &[Option<String>]) -> Result<String, String> {
        self.validate(version)?;
        let statements = self
            .actions
            .iter()
            .zip(names)
            .map(|(a, name)| {
                Ok(format!(
                    "hl.dispatch({})",
                    if let Some(name) = name {
                        name.clone()
                    } else {
                        a.export_lua(version)?
                    }
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(format!(
            "function({}) {} end",
            self.event_parameter.as_deref().unwrap_or(""),
            statements.join("; ")
        ))
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Callbacks {
    pub start: Option<Workflow>,
    pub update: Option<Workflow>,
    pub finish: Option<Workflow>,
}
impl Callbacks {
    pub fn phases(&self) -> [Option<&Workflow>; 3] {
        [
            self.start.as_ref(),
            self.update.as_ref(),
            self.finish.as_ref(),
        ]
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub enum GestureAction {
    Builtin(String),
    Callbacks(Callbacks),
    LegacyFinish(Workflow),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub id: String,
    pub fingers: Value,
    pub direction: String,
    pub action: GestureAction,
    pub mods: Option<Value>,
    pub scale: Option<Value>,
    pub disable_inhibit: Option<Value>,
    pub mode: Option<Value>,
    pub zoom_level: Option<Value>,
    pub workspace_name: Option<Value>,
}
impl Spec {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            fingers: Value::Int(3),
            direction: "horizontal".into(),
            action: GestureAction::Builtin("workspace".into()),
            mods: None,
            scale: None,
            disable_inhibit: None,
            mode: None,
            zoom_level: None,
            workspace_name: None,
        }
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Gesture needs an audited snapshot".into());
        }
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err("Invalid gesture declaration identity".into());
        }
        let Value::Int(fingers) = self.fingers else {
            return Err(
                "Finger count requires a Lua integer, not a float or numeric string".into(),
            );
        };
        if !(2..=9).contains(&fingers) {
            return Err("Finger count must be 2..9".into());
        }
        text(&self.direction)?;
        Direction::parse(&self.direction)?;
        modifier_mask(self.mods.as_ref())?;
        if let Some(scale) = &self.scale {
            let n = animation::number(scale, true)?;
            let valid = if version == BASELINE {
                (MIN_SCALE..=10.).contains(&n)
            } else {
                (-10. ..=10.).contains(&n) && (n as f32).abs() >= 0.1_f32
            };
            if !valid {
                return Err(if version == BASELINE {
                    format!("Gesture scale must lie in {MIN_SCALE}..10")
                } else {
                    "Gesture scale must round to a float32 magnitude of at least 0.1 and lie in -10..10".into()
                });
            }
        }
        if let Some(v) = &self.disable_inhibit {
            animation::enabled(v)?;
        }
        for v in [&self.mode, &self.zoom_level, &self.workspace_name]
            .into_iter()
            .flatten()
        {
            string_field(v)?;
        }
        match &self.action {
            GestureAction::Builtin(name) if BUILTINS.contains(&name.as_str()) => {}
            GestureAction::Builtin(_) => {
                return Err("Gesture action is outside the pinned built-ins".into())
            }
            GestureAction::LegacyFinish(w) => w.validate(version)?,
            GestureAction::Callbacks(c) => {
                if c.phases().iter().all(Option::is_none) {
                    return Err("Enable at least one callback phase".into());
                }
                for w in c.phases().into_iter().flatten() {
                    w.validate(version)?;
                }
            }
        }
        Ok(())
    }
    pub fn identity(&self, version: &str) -> Result<Identity, String> {
        self.validate(version)?;
        let Value::Int(fingers) = self.fingers else {
            unreachable!()
        };
        Ok(Identity {
            fingers: fingers as u8,
            direction: Direction::parse(&self.direction)?,
            modifiers: modifier_mask(self.mods.as_ref())?,
            scale_bits: (self
                .scale
                .as_ref()
                .map(|v| animation::number(v, true))
                .transpose()?
                .unwrap_or(1.) as f32)
                .to_bits(),
            disable_inhibit: self
                .disable_inhibit
                .as_ref()
                .map(animation::enabled)
                .transpose()?
                .unwrap_or(false),
        })
    }
    pub fn unset(&self) -> Self {
        let mut s = self.clone();
        s.action = GestureAction::Builtin("unset".into());
        s
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut before = String::new();
        let mut index = 0;
        let mut callback = |w: &Workflow| -> Result<String, String> {
            let mut names = vec![];
            for a in &w.actions {
                let name = if a.at_load {
                    let mut name = format!("_hye_gesture_action_{index}");
                    while w.event_parameter.as_deref() == Some(name.as_str()) {
                        index += 1;
                        name = format!("_hye_gesture_action_{index}");
                    }
                    before.push_str(&format!("local {name} = {}\n", a.export_lua(version)?));
                    Some(name)
                } else {
                    None
                };
                index += 1;
                names.push(name);
            }
            w.function(version, &names)
        };
        let action = match &self.action {
            GestureAction::Builtin(s) => quote_lua(s),
            GestureAction::LegacyFinish(w) => callback(w)?,
            GestureAction::Callbacks(c) => {
                let mut fields = vec![];
                for (phase, w) in ["start", "update", "finish"].into_iter().zip(c.phases()) {
                    if let Some(w) = w {
                        fields.push(format!("{phase} = {}", callback(w)?));
                    }
                }
                format!("{{ {} }}", fields.join(", "))
            }
        };
        let mut fields = vec![
            format!("fingers = {}", literal(&self.fingers)),
            format!("direction = {}", quote_lua(&self.direction)),
            format!("action = {action}"),
        ];
        for (key, value) in [
            ("mods", &self.mods),
            ("scale", &self.scale),
            ("disable_inhibit", &self.disable_inhibit),
            ("mode", &self.mode),
            ("zoom_level", &self.zoom_level),
            ("workspace_name", &self.workspace_name),
        ] {
            if let Some(v) = value {
                fields.push(format!("{key} = {}", literal(v)));
            }
        }
        let call = format!("hl.gesture({{ {} }})\n", fields.join(", "));
        if before.is_empty() {
            Ok(call)
        } else {
            Ok(format!("do\n{before}{call}end\n"))
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub fingers: u8,
    pub direction: Direction,
    pub modifiers: u8,
    pub scale_bits: u32,
    pub disable_inhibit: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub declarations: Vec<Spec>,
}
impl Draft {
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if self.declarations.len() > MAX_GESTURES || ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err(
                "Gesture draft needs an audited snapshot and at most 1024 declarations".into(),
            );
        }
        let mut ids = BTreeSet::new();
        for d in &self.declarations {
            d.validate(version)?;
            if !ids.insert(&d.id) {
                return Err("Duplicate gesture declaration identity".into());
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT_BYTES {
            return Err("Gesture draft exceeds 1 MiB".into());
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        self.declarations
            .iter()
            .map(|d| d.export_lua(version))
            .collect()
    }
    pub fn same_declarations(&self, other: &Self, version: &str) -> bool {
        match (self.export_lua(version), other.export_lua(version)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }
    /// Starts from an empty registry; opaque includes, plugins and existing runtime state are excluded.
    pub fn registrations(&self, version: &str) -> Result<Vec<Registration>, String> {
        self.validate(version)?;
        let mut active = Vec::<(usize, Identity)>::new();
        let mut outcomes = vec![];
        for (index, s) in self.declarations.iter().enumerate() {
            let identity = s.identity(version)?;
            let unset = matches!(&s.action,GestureAction::Builtin(n) if n=="unset");
            let result = if unset {
                if let Some(i) = active.iter().position(|(_, id)| *id == identity) {
                    let (removed, _) = active.remove(i);
                    Ok(Some(removed))
                } else {
                    Err("No preceding gesture has this full removal identity".into())
                }
            } else if let Some((prior, _)) = active.iter().find(|(_, id)| {
                id.fingers == identity.fingers
                    && id.modifiers == identity.modifiers
                    && id.direction.shadows(identity.direction)
            }) {
                Err(format!(
                    "Overshadowed by declaration {}",
                    self.declarations[*prior].id
                ))
            } else {
                active.push((index, identity));
                Ok(None)
            };
            outcomes.push(Registration {
                id: s.id.clone(),
                result,
            });
        }
        Ok(outcomes)
    }
    pub fn findings(&self, version: &str) -> Result<Vec<String>, String> {
        let mut messages = self
            .registrations(version)?
            .into_iter()
            .filter_map(|r| r.result.err().map(|e| format!("{}: {e}", r.id)))
            .take(128)
            .collect::<Vec<_>>();
        for d in &self.declarations {
            if matches!(&d.action, GestureAction::LegacyFinish(_)) {
                messages.push(format!(
                    "{}: legacy callback runs at finish with no event payload",
                    d.id
                ));
            }
            if messages.len() >= 128 {
                break;
            }
        }
        Ok(messages)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Registration {
    pub id: String,
    pub result: Result<Option<usize>, String>,
}
/// Synthetic callback payload only; never injects a pointer or executes an action.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventSample {
    pub phase: Phase,
    pub kind: EventKind,
    pub direction: Direction,
    pub time_ms: u32,
    pub fingers: u8,
    pub delta: [f64; 2],
    pub scale: f64,
    pub rotation: f64,
    pub cancelled: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Start,
    Update,
    Finish,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Swipe,
    Pinch,
}
impl EventSample {
    pub fn payload(&self) -> Result<serde_json::Value, String> {
        if !self
            .delta
            .iter()
            .chain([&self.scale, &self.rotation])
            .all(|n| n.is_finite())
            || !(2..=9).contains(&self.fingers)
        {
            return Err("Synthetic event needs finite coordinates and 2..9 fingers".into());
        }
        let mut payload = serde_json::json!({"phase":match self.phase{Phase::Start=>"start",Phase::Update=>"update",Phase::Finish=>"end"},"direction":self.direction.event_name(),"type":match self.kind{EventKind::Swipe=>"swipe",EventKind::Pinch=>"pinch"},"time_ms":self.time_ms});
        if self.phase == Phase::Finish {
            payload["cancelled"] = self.cancelled.into();
        } else {
            payload["fingers"] = self.fingers.into();
            payload["delta"] = serde_json::json!({"x":self.delta[0],"y":self.delta[1]});
            if self.kind == EventKind::Pinch {
                payload["scale"] = self.scale.into();
                payload["rotation"] = self.rotation.into();
            }
        }
        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue, lua, workspace::Workspace, Draft as ScalarDraft};
    fn parse(source: &str, version: &str) -> lua::LuaReport {
        lua::parse(source, version, &catalogue()).unwrap()
    }
    #[test]
    fn source_registration_and_callback_payload_observations_match_the_models() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/gesture-source-fixtures.json"))
                .unwrap();
        for case in fixtures["cases"].as_array().unwrap() {
            let version = if case["version"] == "baseline" {
                BASELINE
            } else {
                AUDITED_MAIN
            };
            let name = case["case"].as_str().unwrap();
            if name.starts_with("payloads:") {
                let kind = if name.ends_with("pinchin") {
                    EventKind::Pinch
                } else {
                    EventKind::Swipe
                };
                let direction = if kind == EventKind::Pinch {
                    Direction::PinchIn
                } else {
                    Direction::Left
                };
                for (i, phase) in [Phase::Start, Phase::Update, Phase::Finish]
                    .into_iter()
                    .enumerate()
                {
                    let sample = EventSample {
                        phase,
                        kind,
                        direction,
                        time_ms: if phase == Phase::Finish { 124 } else { 123 },
                        fingers: 3,
                        delta: [-12., 5.],
                        scale: 0.8,
                        rotation: 10.,
                        cancelled: true,
                    };
                    assert_eq!(
                        sample.payload().unwrap(),
                        case["trace"][i],
                        "{version}: {name}/{phase:?}"
                    );
                }
                continue;
            }
            if name == "legacy-end-has-no-payload" {
                assert_eq!(case["trace"], serde_json::json!(["nil-event"]));
                continue;
            }
            let report = parse(case["source"].as_str().unwrap(), version);
            let draft = report.gestures;
            let outcomes = draft.registrations(version).unwrap();
            let mut active = Vec::<usize>::new();
            let known_error = outcomes.iter().any(|o| o.result.is_err());
            assert_eq!(
                case["accepted"].as_bool().unwrap(),
                !report.opaque_effects && !known_error,
                "{version}: {name}: {:?}",
                report.diagnostics
            );
            for (i, outcome) in outcomes.into_iter().enumerate() {
                match outcome.result {
                    Ok(Some(removed)) => active.retain(|i| *i != removed),
                    Ok(None) => active.push(i),
                    Err(_) => {}
                }
            }
            let registered = case["registered"].as_array().unwrap();
            assert_eq!(active.len(), registered.len(), "{version}: {name}");
            for (i, row) in active.into_iter().zip(registered) {
                let spec = &draft.declarations[i];
                let identity = spec.identity(version).unwrap();
                assert_eq!(identity.fingers as u64, row["fingers"].as_u64().unwrap());
                assert_eq!(
                    identity.direction.event_name(),
                    row["direction"].as_str().unwrap()
                );
                assert_eq!(
                    identity.modifiers as u64,
                    row["modifiers"].as_u64().unwrap()
                );
                assert_eq!(
                    f64::from(f32::from_bits(identity.scale_bits)),
                    row["scale"].as_f64().unwrap()
                );
                assert_eq!(
                    identity.disable_inhibit,
                    row["disable_inhibit"].as_bool().unwrap()
                );
            }
            if name == "captured-and-late-preparation" {
                assert_eq!(
                    case["trace"],
                    serde_json::json!(["prepare", "dispatch", "prepare", "dispatch"])
                );
            }
        }
    }
    #[test]
    fn strict_finger_and_float32_scale_bounds_preserve_literal_types() {
        for version in [BASELINE, AUDITED_MAIN] {
            for v in ["3.0", "'3'", "true", "1", "10"] {
                let r = parse(
                    &format!("hl.gesture{{fingers={v},direction='horizontal',action='workspace'}}"),
                    version,
                );
                assert!(r.opaque_effects, "{v}");
                assert!(r.gestures.is_empty());
            }
            for v in ["false", "10.0000001", "1e-50"] {
                let r=parse(&format!("hl.gesture{{fingers=3,direction='horizontal',action='workspace',scale={v}}}"),version);
                assert!(r.opaque_effects, "{v}");
            }
            for v in ["0.1", "-0.1", "-10", "0.099999999", "-0.099999999"] {
                let r = parse(
                    &format!(
                        "hl.gesture{{fingers=3,direction='left',action='workspace',scale={v}}}"
                    ),
                    version,
                );
                assert_eq!(!r.opaque_effects, version == AUDITED_MAIN, "{version}: {v}");
            }
            for v in ["0.10000000149011612", "1", "true", "'1'", "10"] {
                let r=parse(&format!("hl.gesture{{fingers=3,direction='horizontal',action='workspace',scale={v},disable_inhibit='1'}}"),version);
                assert!(!r.opaque_effects, "{v}: {:?}", r.diagnostics);
                let out = r.gestures.export_lua(version).unwrap();
                assert!(parse(&out, version)
                    .gestures
                    .same_declarations(&r.gestures, version));
            }
        }
    }
    #[test]
    fn all_builtin_and_direction_aliases_round_trip_without_running_actions() {
        for version in [BASELINE, AUDITED_MAIN] {
            for action in BUILTINS {
                for direction in [
                    "swipe", "LEFT", "l", "r", "up", "top", "u", "t", "d", "bottom", "b", "horiz",
                    "vert", "pinch", "zoomin", "zoomout",
                ] {
                    let source=format!("hl.gesture{{fingers=3,direction='{direction}',action='{action}',mods='notWINd SHIFT',zoom_level=2.0,mode='toggle',workspace_name='scratch'}}");
                    let r = parse(&source, version);
                    assert!(
                        !r.opaque_effects,
                        "{action}/{direction}: {:?}",
                        r.diagnostics
                    );
                    assert!(parse(&r.gestures.export_lua(version).unwrap(), version)
                        .gestures
                        .same_declarations(&r.gestures, version));
                    assert_eq!(
                        r.gestures.declarations[0]
                            .identity(version)
                            .unwrap()
                            .modifiers,
                        65
                    );
                }
            }
        }
    }
    #[test]
    fn callbacks_preserve_phase_presence_parameters_preparation_and_legacy_shape() {
        for version in [BASELINE, AUDITED_MAIN] {
            let source="do local saved=hl.dsp.no_op();hl.gesture{fingers=4,direction='left',action={start=function(event) hl.dispatch(saved) end,update=function() end,finish=function(e) hl.dispatch(hl.dsp.no_op()) end}}end;hl.gesture{fingers=5,direction='vertical',action=function() hl.dispatch(hl.dsp.no_op()) end}";
            let r = parse(source, version);
            assert!(!r.opaque_effects, "{:?}", r.diagnostics);
            assert_eq!(r.gestures.declarations.len(), 2);
            let GestureAction::Callbacks(c) = &r.gestures.declarations[0].action else {
                panic!()
            };
            assert!(c.start.as_ref().unwrap().actions[0].at_load);
            assert_eq!(
                c.start.as_ref().unwrap().event_parameter.as_deref(),
                Some("event")
            );
            assert!(c.update.as_ref().unwrap().actions.is_empty());
            assert!(!c.finish.as_ref().unwrap().actions[0].at_load);
            assert!(matches!(
                &r.gestures.declarations[1].action,
                GestureAction::LegacyFinish(_)
            ));
            let out = r.gestures.export_lua(version).unwrap();
            let next = parse(&out, version);
            assert!(!next.opaque_effects, "{out} {:?}", next.diagnostics);
            assert!(next.gestures.same_declarations(&r.gestures, version));
            assert_eq!(
                next.gestures.declarations[0].action,
                r.gestures.declarations[0].action
            );
            let mut collision = r.gestures.clone();
            let GestureAction::Callbacks(c) = &mut collision.declarations[0].action else {
                panic!()
            };
            c.start.as_mut().unwrap().event_parameter = Some("_hye_gesture_action_0".into());
            let out = collision.export_lua(version).unwrap();
            let next = parse(&out, version);
            assert!(!next.opaque_effects, "{out} {:?}", next.diagnostics);
            assert_eq!(
                next.gestures.declarations[0].action,
                collision.declarations[0].action
            );
            for source in ["hl.gesture{fingers=3,direction='left',action={}}","hl.gesture{fingers=3,direction='left',action={['end']=function() end}}","hl.gesture{fingers=3,direction='left',action=function(hl) hl.dispatch(hl.dsp.no_op()) end}"] {assert!(parse(source,version).opaque_effects);}
            let r=parse("local e='captured';hl.gesture{fingers=3,direction='left',action=function(e) hl.dispatch(hl.dsp.exec_cmd(e)) end}",version);
            assert!(r.opaque_effects);
            assert!(r.gestures.is_empty());
        }
    }
    #[test]
    fn registry_checks_prior_broad_directions_and_all_five_unset_identity_parts() {
        let mut broad = Spec::new("broad");
        broad.direction = "swipe".into();
        let mut left = Spec::new("left");
        left.direction = "left".into();
        left.scale = Some(Value::Int(2));
        let draft = Draft {
            declarations: vec![broad.clone(), left.clone()],
        };
        assert!(draft.registrations(BASELINE).unwrap()[1].result.is_err());
        let draft = Draft {
            declarations: vec![left.clone(), broad.clone()],
        };
        assert!(draft.registrations(BASELINE).unwrap()[1].result.is_ok());
        broad.direction = "pinch".into();
        left.direction = "pinchin".into();
        assert!(Draft {
            declarations: vec![broad, left.clone()]
        }
        .registrations(BASELINE)
        .unwrap()[1]
            .result
            .is_err());
        let mut unset = left.unset();
        unset.id = "unset".into();
        let mut wrong = unset.clone();
        wrong.id = "wrong-scale".into();
        wrong.scale = None;
        let registry = Draft {
            declarations: vec![left.clone(), wrong, unset],
        }
        .registrations(BASELINE)
        .unwrap();
        assert!(registry[1].result.is_err());
        assert_eq!(registry[2].result, Ok(Some(0)));
        for (field, value) in [
            ("mods", Value::String("CTRL".into())),
            ("disable_inhibit", Value::Bool(true)),
            ("fingers", Value::Int(4)),
        ] {
            let mut wrong = left.unset();
            wrong.id = "wrong".into();
            match field {
                "mods" => wrong.mods = Some(value),
                "disable_inhibit" => wrong.disable_inhibit = Some(value),
                _ => wrong.fingers = value,
            };
            assert!(Draft {
                declarations: vec![left.clone(), wrong]
            }
            .registrations(BASELINE)
            .unwrap()[1]
                .result
                .is_err());
        }
    }
    #[test]
    fn opaque_callback_bodies_remain_inert_but_at_load_unknown_arguments_taint_following_calls() {
        let r=parse("hl.gesture{fingers=3,direction='left',action=function(e) os.execute('never run') end};hl.config{general={border_size=8}}",BASELINE);
        assert!(r.opaque_effects);
        assert!(r.gestures.is_empty());
        assert_eq!(r.draft.values["general:border_size"], Value::Int(8));
        let r=parse("hl.gesture{fingers=3,direction='left',action=unknown()};hl.config{general={border_size=8}}",BASELINE);
        assert!(r.opaque_effects);
        assert!(r.draft.values.is_empty());
    }
    #[test]
    fn synthetic_events_expose_finish_end_mapping_and_phase_specific_fields() {
        let mut sample = EventSample {
            phase: Phase::Start,
            kind: EventKind::Pinch,
            direction: Direction::PinchIn,
            time_ms: 123,
            fingers: 3,
            delta: [-12., 5.],
            scale: 0.8,
            rotation: 10.,
            cancelled: true,
        };
        let p = sample.payload().unwrap();
        assert_eq!(p["phase"], "start");
        assert_eq!(p["direction"], "PINCH_IN");
        assert_eq!(p["scale"], 0.8);
        assert!(p.get("cancelled").is_none());
        sample.phase = Phase::Finish;
        let p = sample.payload().unwrap();
        assert_eq!(p["phase"], "end");
        assert_eq!(p["cancelled"], true);
        assert!(p.get("delta").is_none());
        assert!(p.get("fingers").is_none());
        sample.rotation = f64::NAN;
        assert!(sample.payload().is_err());
    }
    #[test]
    fn schema_pinned_storage_and_declaration_caps_reject_forged_gestures() {
        let mut s = Spec::new("invalid");
        s.fingers = Value::Float(3.);
        assert!(s.validate(BASELINE).is_err());
        let draft = Draft {
            declarations: vec![Spec::new("a")],
        };
        let w = Workspace::new(ScalarDraft::new(BASELINE)).with_gestures(draft.clone());
        w.validate(&catalogue()).unwrap();
        let mut bad = w.clone();
        bad.gesture_schema_sha256 = Some("0".repeat(64));
        assert!(bad.validate(&catalogue()).is_err());
        let mut large = Draft::default();
        for i in 0..=MAX_GESTURES {
            let mut s = Spec::new(&format!("g-{i}"));
            s.fingers = Value::Int(2 + (i % 8) as i64);
            large.declarations.push(s);
        }
        assert!(large.validate(BASELINE).is_err());
    }
}
