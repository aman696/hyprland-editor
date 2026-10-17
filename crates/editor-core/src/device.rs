//! Device override declarations. Hardware observations and global input values remain separate.
use crate::{quote_lua, Setting, Value, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_DEVICES: usize = 128;
const BASE_FIELDS: &str = include_str!("../../../data/devices.json");
const ADDED_FIELDS: &str = include_str!("../../../data/device-additions.json");
const LEGACY_SCHEMA: &str = "f555d3d357acca4e74b4472f9a7fc441c6f347b8a2b47c8b6d466dbde45966b7";
pub const ADDED_KEYS: &[&str] = &[
    "eraser_button_mode",
    "eraser_button_override",
    "pressure_range_min",
    "pressure_range_max",
];
pub fn catalogue(version: &str) -> Vec<Setting> {
    static ALL: std::sync::OnceLock<Vec<Setting>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut fields: Vec<Setting> =
            serde_json::from_str(BASE_FIELDS).expect("checked device catalogue");
        fields.extend(
            serde_json::from_str::<Vec<Setting>>(ADDED_FIELDS).expect("checked device additions"),
        );
        fields
    })
    .iter()
    .filter(|s| s.version == version)
    .cloned()
    .collect()
}
pub fn schema_hash() -> String {
    static HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        crate::lua::source_hash(&format!(
            "hyprland-editor-device-fields-2\n{}\n{}",
            crate::lua::source_hash(BASE_FIELDS),
            crate::lua::source_hash(ADDED_FIELDS)
        ))
    });
    HASH.clone()
}
/// A known additive old schema remains valid for its original field subset.
/// No value/default is inserted and recorded paths are never consulted.
pub fn compatible_schema(pin: &str, specs: &[Spec]) -> bool {
    pin == schema_hash()
        || pin == LEGACY_SCHEMA
            && crate::lua::source_hash(BASE_FIELDS) == LEGACY_SCHEMA
            && specs.iter().all(|s| {
                s.fields
                    .keys()
                    .all(|key| !ADDED_KEYS.contains(&key.as_str()))
            })
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Unknown,
    Keyboard,
    Pointer,
    Touchpad,
    Tablet,
    Touch,
    Virtual,
}
impl Kind {
    pub const ALL: &'static [Self] = &[
        Self::Unknown,
        Self::Keyboard,
        Self::Pointer,
        Self::Touchpad,
        Self::Tablet,
        Self::Touch,
        Self::Virtual,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "All Fields / Unknown Device",
            Self::Keyboard => "Keyboard",
            Self::Pointer => "Pointer",
            Self::Touchpad => "Touchpad",
            Self::Tablet => "Tablet",
            Self::Touch => "Touchscreen",
            Self::Virtual => "Virtual Device",
        }
    }
}
pub fn relevance(key: &str) -> &'static [Kind] {
    use Kind::*;
    match key {
        "kb_file"
        | "kb_layout"
        | "kb_variant"
        | "kb_options"
        | "kb_rules"
        | "kb_model"
        | "repeat_rate"
        | "repeat_delay"
        | "numlock_by_default"
        | "resolve_binds_by_sym"
        | "share_states"
        | "release_pressed_on_close" => &[Keyboard, Virtual],
        "sensitivity" | "accel_profile" | "rotation" | "scroll_method" | "scroll_button"
        | "scroll_button_lock" | "scroll_points" | "scroll_factor" | "natural_scroll" => {
            &[Pointer, Touchpad]
        }
        "tap_button_map"
        | "disable_while_typing"
        | "clickfinger_behavior"
        | "middle_button_emulation"
        | "tap_to_click"
        | "tap_and_drag"
        | "drag_lock"
        | "drag_3fg"
        | "flip_x"
        | "flip_y" => &[Touchpad],
        "relative_input"
        | "region_position"
        | "region_size"
        | "absolute_region_position"
        | "active_area_position"
        | "active_area_size"
        | "eraser_button_mode"
        | "eraser_button_override"
        | "pressure_range_min"
        | "pressure_range_max" => &[Tablet],
        "transform" | "output" => &[Tablet, Touch],
        "left_handed" => &[Pointer, Touchpad, Tablet],
        _ => &[Keyboard, Pointer, Touchpad, Tablet, Touch, Virtual],
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: Kind,
    #[serde(default, deserialize_with = "crate::unique_values")]
    pub fields: BTreeMap<String, Value>,
}
impl Spec {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: Kind::Unknown,
            fields: BTreeMap::new(),
        }
    }
    pub fn normalized_name(&self) -> String {
        self.name.replace(' ', "-")
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Unknown device snapshot".into());
        }
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err("Invalid device draft identity".into());
        }
        if self.name.trim().is_empty()
            || self.name.len() > 4096
            || self.name.chars().any(char::is_control)
        {
            return Err("Choose a nonempty device name without control characters".into());
        }
        let c = catalogue(version);
        if self.fields.len() > c.len() {
            return Err("Too many device fields".into());
        }
        for (key, value) in &self.fields {
            let setting = c
                .iter()
                .find(|s| s.lua_key == *key)
                .ok_or_else(|| format!("Unknown device field {key}"))?;
            setting.validate(value)?;
            match (key.as_str(),value) {
                ("region_size"|"active_area_size",Value::Vec2(v)) if v.iter().any(|n|*n<0.) => return Err("Region sizes must be nonnegative; zero retains upstream automatic-area behavior".into()),
                ("accel_profile",Value::String(s)) => {Acceleration::parse(s)?;},
                ("scroll_points",Value::String(s)) if !s.is_empty()=>{Curve::parse(s)?;},
                ("scroll_method",Value::String(s)) if !["","no_scroll","2fg","edge","on_button_down"].contains(&s.as_str())=>return Err("Choose an audited scroll method".into()),
                ("tap_button_map",Value::String(s)) if !["","lrm","lmr"].contains(&s.as_str())=>return Err("Choose an audited tap button map".into()),
                _=>{},
            }
        }
        if let (Some(Value::String(layouts)), Some(Value::String(variants))) =
            (self.fields.get("kb_layout"), self.fields.get("kb_variant"))
        {
            if !variants.is_empty() && layouts.split(',').count() != variants.split(',').count() {
                return Err("Each keyboard layout needs a corresponding variant slot".into());
            }
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut text = format!("hl.device({{ name = {}", quote_lua(&self.name));
        for (key, value) in &self.fields {
            text.push_str(&format!(", [ {} ] = {}", quote_lua(key), value.lua()));
        }
        text.push_str(" })\n");
        Ok(text)
    }
    pub fn findings(&self) -> Vec<String> {
        let mut result = vec![];
        if self.normalized_name() != self.name {
            result.push(format!(
                "Hyprland matches this name as {}",
                self.normalized_name()
            ));
        }
        if self.fields.get("enabled") == Some(&Value::Bool(false)) {
            result.push("This declaration disables a matching device. A future apply operation requires a usable input/recovery plan.".into());
        }
        if matches!(self.fields.get("scroll_points"),Some(Value::String(s))if !s.is_empty())
            && !matches!(self.fields.get("accel_profile"),Some(Value::String(s))if s.starts_with("custom "))
        {
            result.push("Scroll points are only used with custom acceleration; inherited acceleration is currently unknown.".into());
        }
        if self
            .fields
            .get("kb_file")
            .is_some_and(|v| matches!(v,Value::String(s)if !s.is_empty()))
        {
            result.push("A keymap file takes precedence over layout/model/options. Its contents have not been compiled or inspected.".into());
        }
        if self
            .fields
            .get("transform")
            .is_some_and(|v| matches!(v,Value::Int(n)if !(-1..=7).contains(n)))
        {
            result.push("The input handler clamps this transform to -1..7. The imported declaration is preserved.".into());
        }
        result
    }
}
pub fn validate(specs: &[Spec], version: &str) -> Result<(), String> {
    if specs.len() > MAX_DEVICES {
        return Err("At most 128 ordered device declarations are supported".into());
    }
    let mut ids = BTreeSet::new();
    for s in specs {
        s.validate(version)?;
        if !ids.insert(&s.id) {
            return Err("Duplicate device draft identity".into());
        }
    }
    if serde_json::to_vec(specs).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT_BYTES {
        return Err("Device draft exceeds 1 MiB".into());
    }
    Ok(())
}
pub fn same_declarations(a: &[Spec], b: &[Spec]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.name == b.name && a.fields == b.fields)
}
pub fn export_lua(specs: &[Spec], version: &str) -> Result<String, String> {
    validate(specs, version)?;
    let mut text = String::new();
    for s in specs {
        text.push_str(&s.export_lua(version)?);
    }
    Ok(text)
}
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub step: f64,
    pub points: Vec<f64>,
}
impl Curve {
    pub fn validate(&self) -> Result<(), String> {
        if !self.step.is_finite()
            || self.step <= 0.
            || self.points.len() < 2
            || self.points.len() > 64
            || self.points.iter().any(|p| !p.is_finite() || *p < 0.)
        {
            return Err(
                "A custom curve needs a positive finite step and 2–64 nonnegative finite points"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut parts = text.split_whitespace();
        let step = parts
            .next()
            .ok_or("Missing curve step")?
            .parse()
            .map_err(|_| "Invalid curve step")?;
        let points = parts
            .map(|p| p.parse().map_err(|_| "Invalid curve point"))
            .collect::<Result<Vec<_>, _>>()?;
        let curve = Self { step, points };
        curve.validate()?;
        Ok(curve)
    }
    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        let mut text = self.step.to_string();
        for p in &self.points {
            text.push_str(&format!(" {p}"));
        }
        Ok(text)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Acceleration {
    Default,
    Adaptive,
    Flat,
    Custom(Curve),
}
impl Acceleration {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "" => Ok(Self::Default),
            "adaptive" => Ok(Self::Adaptive),
            "flat" => Ok(Self::Flat),
            _ => s
                .strip_prefix("custom ")
                .ok_or_else(|| "Choose an audited acceleration profile".into())
                .and_then(|s| Curve::parse(s).map(Self::Custom)),
        }
    }
    pub fn encode(&self) -> Result<String, String> {
        match self {
            Self::Default => Ok(String::new()),
            Self::Adaptive => Ok("adaptive".into()),
            Self::Flat => Ok("flat".into()),
            Self::Custom(c) => Ok(format!("custom {}", c.encode()?)),
        }
    }
}

/// Comma-separated CVarList2 spelling, with escaped commas/backslashes.
pub fn parse_tags(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }
    let chars: Vec<_> = text.chars().collect();
    let mut result = vec![];
    let mut token = String::new();
    let mut start = 0;
    for (i, ch) in chars.iter().copied().enumerate() {
        if ch == ',' {
            if i > start && chars[i - 1] == '\\' {
                token.pop();
                if i == start + 1 || chars[i - 2] != '\\' {
                    token.push(',');
                    continue;
                }
            }
            result.push(token.trim().to_string());
            token.clear();
            start = i + 1;
        } else {
            token.push(ch);
        }
    }
    if start < chars.len() {
        result.push(token.trim().to_string());
    }
    result
}
pub fn encode_tags(tags: &[String]) -> Result<String, String> {
    if tags.len() > 128
        || tags.iter().any(|t| {
            t.is_empty() || t.trim() != t || t.contains("\\,") || t.chars().any(char::is_control)
        })
    {
        return Err(
            "Use up to 128 nonempty tags without edge whitespace/control characters; a backslash directly before a comma cannot be represented by this parser".into(),
        );
    }
    let mut text = String::new();
    for (i, tag) in tags.iter().enumerate() {
        text.push_str(&tag.replace(',', "\\,"));
        if i + 1 < tags.len() {
            if tag.ends_with('\\') {
                text.push('\\');
            }
            text.push(',');
        }
    }
    if text.len() > 16384 {
        return Err("Tags exceed 16 KiB".into());
    }
    Ok(text)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lua, workspace::Workspace, Draft};
    #[test]
    fn every_device_field_round_trips_the_pinned_static_lua_codec() {
        for version in [BASELINE, AUDITED_MAIN] {
            let fields = catalogue(version);
            assert_eq!(fields.len(), 47);
            for setting in fields {
                let mut spec = Spec::new("round-trip", "Example USB keyboard");
                let value = setting
                    .known_default()
                    .unwrap_or_else(|| panic!("{} default", setting.id));
                spec.fields.insert(setting.lua_key.clone(), value.clone());
                let source = spec.export_lua(version).unwrap();
                let report = lua::parse(&source, version, &crate::catalogue()).unwrap();
                assert!(
                    !report.opaque_effects,
                    "{}: {:?}",
                    setting.id, report.diagnostics
                );
                assert_eq!(report.devices.len(), 1);
                assert_eq!(report.devices[0].fields[&setting.lua_key], value);
                assert_eq!(report.devices[0].normalized_name(), "Example-USB-keyboard");
            }
        }
    }
    #[test]
    fn tag_rows_keep_delimiter_escapes_and_reject_empty_entries() {
        let tags = vec!["work".into(), "a,b".into(), "path\\name".into()];
        assert_eq!(parse_tags(&encode_tags(&tags).unwrap()), tags);
        assert_eq!(parse_tags("a,,b,"), vec!["a", "", "b"]);
        assert_eq!(parse_tags("a\\\\,b"), vec!["a\\", "b"]);
        assert_eq!(parse_tags("a\\,b"), vec!["a,b"]);
        assert!(encode_tags(&["a\\,b".into()]).is_err());
        assert!(encode_tags(&["".into()]).is_err());
        assert!(encode_tags(&[" leading".into()]).is_err());
        assert!(encode_tags(&vec!["tag".into(); 129]).is_err());
    }
    #[test]
    fn numeric_curve_builder_checks_counts_ranges_and_nonfinite_values() {
        for text in ["1 0 1 4", "0.25 0 0.5 1.75"] {
            let c = Curve::parse(text).unwrap();
            assert_eq!(Curve::parse(&c.encode().unwrap()).unwrap(), c);
            let a = Acceleration::Custom(c);
            assert_eq!(Acceleration::parse(&a.encode().unwrap()).unwrap(), a);
        }
        for text in ["0 0 1", "-1 1 2", "NaN 0 1", "1 0", "1 0 -1", "1 0 inf"] {
            assert!(Curve::parse(text).is_err(), "{text}");
        }
        assert!(Curve {
            step: 1.,
            points: vec![1.; 65]
        }
        .validate()
        .is_err());
        let mut spec = Spec::new("curve", "pointer");
        spec.fields
            .insert("accel_profile".into(), Value::String("custom 1 0 1".into()));
        assert!(spec.validate(BASELINE).is_ok());
        spec.fields.insert(
            "accel_profile".into(),
            Value::String("execute anything".into()),
        );
        assert!(spec.validate(BASELINE).is_err());
    }
    #[test]
    fn invalid_names_fields_bounds_and_linked_layouts_are_rejected() {
        let mut spec = Spec::new("device", "mouse");
        spec.fields.insert("sensitivity".into(), Value::Float(1.1));
        assert!(spec.validate(BASELINE).is_err());
        spec.fields.clear();
        spec.fields
            .insert("region_size".into(), Value::Vec2([-1., 2.]));
        assert!(spec.validate(BASELINE).is_err());
        spec.fields.clear();
        spec.fields
            .insert("kb_layout".into(), Value::String("us,de".into()));
        spec.fields
            .insert("kb_variant".into(), Value::String("dvorak".into()));
        assert!(spec.validate(BASELINE).is_err());
        spec.fields
            .insert("kb_variant".into(), Value::String("dvorak,".into()));
        assert!(spec.validate(BASELINE).is_ok());
        spec.name = "\n".into();
        assert!(spec.validate(BASELINE).is_err());
        spec.name = "mouse".into();
        spec.fields.insert("invented".into(), Value::Bool(true));
        assert!(spec.validate(BASELINE).is_err());
        assert!(Spec::new("bad id", "name").validate(BASELINE).is_err());
        assert!(Spec::new("valid", "").validate(BASELINE).is_err());
    }
    #[test]
    fn source_copy_comparison_ignores_author_hints_but_detects_declaration_edits() {
        let a = Spec::new("source", "keyboard");
        let mut b = a.clone();
        b.kind = Kind::Keyboard;
        b.id = "draft".into();
        assert!(same_declarations(
            std::slice::from_ref(&a),
            std::slice::from_ref(&b)
        ));
        b.fields.insert("enabled".into(), Value::Bool(false));
        assert!(!same_declarations(&[a], &[b]));
    }
    #[test]
    fn declarations_preserve_order_repeated_names_and_private_schema_pin() {
        let mut a = Spec::new("a", "USB keyboard");
        a.kind = Kind::Keyboard;
        a.fields
            .insert("kb_layout".into(), Value::String("us".into()));
        let b = Spec::new("b", "USB-keyboard");
        let specs = vec![a, b];
        assert!(validate(&specs, BASELINE).is_ok());
        let c = crate::catalogue();
        let saved = Workspace::new(Draft::new(BASELINE)).with_devices(specs.clone());
        let bytes = saved.to_json(&c).unwrap();
        assert_eq!(
            Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &c)
                .unwrap()
                .devices,
            specs
        );
        let mut bad = saved;
        bad.device_schema_sha256 = None;
        assert!(bad.to_json(&c).is_err());
        assert!(validate(&vec![Spec::new("a", "name"); 129], BASELINE).is_err());
    }
    #[test]
    fn opaque_calls_do_not_execute_or_make_later_device_calls_trusted() {
        let c = crate::catalogue();
        for source in [
            "hl.device{name='a',kb_layout=calculate()};hl.device{name='b'}",
            "local hl={};hl.device{name='a'}",
            "hl.device{name='a',enabled=true,enabled=false}",
        ] {
            let r = lua::parse(source, BASELINE, &c).unwrap();
            assert!(r.opaque_effects);
            assert!(r.devices.is_empty());
        }
        let source="local n=25;hl.device{name='keyboard',repeat_rate=n};hl.monitor{output='DP-1'};hl.config{general={border_size=2}}";
        let report = lua::parse(source, BASELINE, &c).unwrap();
        assert_eq!(report.devices.len(), 1);
        assert_eq!(report.monitors.len(), 1);
        assert!(!report.opaque_effects);
        let json = r#"{"id":"a","name":"name","fields":{"enabled":{"type":"Bool","value":true},"enabled":{"type":"Bool","value":false}}}"#;
        assert!(serde_json::from_str::<Spec>(json).is_err());
    }
}
