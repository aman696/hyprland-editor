//! Typed monitor drafts and builders. No runtime commands or hardware changes.
use crate::{quote_lua, Setting, Value, AUDITED_MAIN, BASELINE};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_RULES: usize = 64;
pub const COLORS: &[&str] = &[
    "auto", "srgb", "wide", "edid", "hdr", "hdredid", "dcip3", "dp3", "adobe",
];
pub const TRANSFER: &[&str] = &[
    "default",
    "auto",
    "srgb",
    "gamma22",
    "gamma22force",
    "0",
    "1",
    "2",
    "3",
];
pub const POSITIONS: &[&str] = &[
    "auto",
    "auto-right",
    "auto-left",
    "auto-up",
    "auto-down",
    "auto-center-right",
    "auto-center-left",
    "auto-center-up",
    "auto-center-down",
];
pub const MODES: &[&str] = &["preferred", "highrr", "highres", "maxwidth"];
pub fn catalogue(version: &str) -> Vec<Setting> {
    let all: Vec<Setting> = serde_json::from_str(include_str!("../../../data/monitors.json"))
        .expect("checked monitor catalogue");
    all.into_iter().filter(|s| s.version == version).collect()
}
pub fn schema_hash() -> String {
    crate::lua::source_hash(include_str!("../../../data/monitors.json"))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "name",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Output {
    Fallback,
    Connector(String),
    Description(String),
}
impl Output {
    pub fn text(&self) -> String {
        match self {
            Self::Fallback => String::new(),
            Self::Connector(s) => s.clone(),
            Self::Description(s) => format!("desc:{s}"),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Fallback => Ok(()),
            Self::Connector(s) if s.starts_with("desc:") => {
                Err("Use Description matching for a desc: selector".into())
            }
            Self::Connector(s) | Self::Description(s)
                if !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control) =>
            {
                Ok(())
            }
            _ => Err("Choose an output name or description without control characters".into()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub output: Output,
    #[serde(default, deserialize_with = "crate::unique_values")]
    pub fields: BTreeMap<String, Value>,
}
impl Rule {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            output: Output::Fallback,
            fields: BTreeMap::new(),
        }
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Unknown monitor snapshot".into());
        }
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("Invalid monitor draft identity".into());
        }
        self.output.validate()?;
        let fields = catalogue(version);
        if self.fields.len() > 22 {
            return Err("Too many monitor fields".into());
        }
        if self.fields.contains_key("reserved") && self.fields.contains_key("reserved_area") {
            return Err("Reserved and reserved_area are aliases; choose one spelling".into());
        }
        for (key, value) in &self.fields {
            let setting = fields
                .iter()
                .find(|s| s.lua_key == *key)
                .ok_or_else(|| format!("Unknown monitor field {key}"))?;
            if setting.kind == "CssGap" {
                if !matches!(value, Value::CssGap(_)) {
                    return Err("Reserved space needs four integer sides".into());
                }
            } else {
                setting.validate(value)?;
            }
            if let Value::String(text) = value {
                match key.as_str() {
                    "mode" => {
                        Mode::parse(text)?.encode()?;
                    }
                    "position" => {
                        Position::parse(text)?;
                    }
                    "scale" => {
                        if text != "auto" {
                            let value = text.parse::<f64>().map_err(|_| "Invalid monitor scale")?;
                            if !value.is_finite() || value < 0.25 {
                                return Err(
                                    "Scale must be automatic or a finite number of at least 0.25"
                                        .into(),
                                );
                            }
                        }
                    }
                    "cm" if !COLORS.contains(&text.as_str()) => {
                        return Err("Color mode is outside the audited choices".into())
                    }
                    "sdr_eotf" if !TRANSFER.contains(&text.as_str()) => {
                        return Err("Transfer function is outside the audited choices".into())
                    }
                    "icc" if text.is_empty() => {
                        return Err("Choose an ICC path or remove the ICC field".into())
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut text = format!("hl.monitor({{ output = {}", quote_lua(&self.output.text()));
        for (key, value) in &self.fields {
            text.push_str(&format!(", [ {} ] = {}", quote_lua(key), value.lua()));
        }
        text.push_str(" })\n");
        Ok(text)
    }
    pub fn coordinates(&self) -> Option<(i32, i32)> {
        match self.fields.get("position") {
            Some(Value::String(s)) => match Position::parse(s).ok()? {
                Position::Point(x, y) => Some((x, y)),
                _ => None,
            },
            _ => None,
        }
    }
    pub fn logical_size(&self) -> Option<(f64, f64)> {
        let (mut w, mut h) = match self.fields.get("mode") {
            Some(Value::String(s)) => match Mode::parse(s).ok()? {
                Mode::Dimensions { width, height, .. } => (f64::from(width), f64::from(height)),
                Mode::Modeline(ref m) => (f64::from(m.horizontal[0]), f64::from(m.vertical[0])),
                _ => return None,
            },
            _ => return None,
        };
        if matches!(
            self.fields.get("transform"),
            Some(Value::Int(1 | 3 | 5 | 7))
        ) {
            std::mem::swap(&mut w, &mut h);
        }
        let scale = match self.fields.get("scale") {
            Some(Value::String(s)) if s != "auto" => s.parse::<f64>().ok()?,
            _ => return None,
        };
        if !scale.is_finite() || scale < 0.25 {
            return None;
        }
        Some((w / scale, h / scale))
    }
}
pub fn validate(rules: &[Rule], version: &str) -> Result<(), String> {
    if rules.len() > MAX_RULES {
        return Err("A monitor draft supports at most 64 ordered rules".into());
    }
    let mut ids = BTreeSet::new();
    let mut mirrors = BTreeMap::new();
    for rule in rules {
        rule.validate(version)?;
        if !ids.insert(&rule.id) {
            return Err("Duplicate monitor identity".into());
        }
        if let Some(Value::String(target)) = rule.fields.get("mirror") {
            if !target.is_empty() {
                mirrors.insert(rule.output.text(), target.clone());
            } else {
                mirrors.remove(&rule.output.text());
            }
        }
    }
    for origin in mirrors.keys() {
        let mut seen = BTreeSet::new();
        let mut next = origin;
        while let Some(target) = mirrors.get(next) {
            if !seen.insert(next) {
                return Err("Monitor mirror cycle".into());
            }
            next = target;
        }
    }
    if serde_json::to_vec(rules).map_err(|e| e.to_string())?.len() > crate::MAX_DOCUMENT_BYTES {
        return Err("Monitor draft exceeds 1 MiB".into());
    }
    Ok(())
}
pub fn export_lua(rules: &[Rule], version: &str) -> Result<String, String> {
    validate(rules, version)?;
    let mut text = "-- Monitor draft declarations; hardware behavior unverified.\n".to_string();
    for rule in rules {
        text.push_str(&rule.export_lua(version)?);
    }
    Ok(text)
}
#[derive(Clone, Debug, PartialEq)]
pub enum Position {
    Auto(String),
    Point(i32, i32),
}
impl Position {
    pub fn parse(text: &str) -> Result<Self, String> {
        if POSITIONS.contains(&text) {
            return Ok(Self::Auto(text.into()));
        }
        let (x, y) = text.split_once('x').ok_or("Invalid monitor position")?;
        Ok(Self::Point(
            x.parse().map_err(|_| "Invalid X position")?,
            y.parse().map_err(|_| "Invalid Y position")?,
        ))
    }
    pub fn encode(&self) -> String {
        match self {
            Self::Auto(s) => s.clone(),
            Self::Point(x, y) => format!("{x}x{y}"),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Automatic(String),
    Dimensions {
        width: u32,
        height: u32,
        refresh: Option<f64>,
    },
    Modeline(Modeline),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Modeline {
    pub clock_mhz: f64,
    pub horizontal: [u16; 4],
    pub vertical: [u16; 4],
    pub hsync: i8,
    pub vsync: i8,
}
impl Mode {
    pub fn parse(text: &str) -> Result<Self, String> {
        if MODES.contains(&text) {
            return Ok(Self::Automatic(text.into()));
        }
        if let Some(rest) = text.strip_prefix("modeline ") {
            let parts: Vec<_> = rest.split_whitespace().collect();
            if parts.len() < 9 {
                return Err("A modeline needs clock and eight timings".into());
            }
            let clock = parts[0].parse().map_err(|_| "Invalid modeline clock")?;
            let mut timings = [0_u16; 8];
            for (index, part) in parts[1..9].iter().enumerate() {
                timings[index] = part.parse().map_err(|_| "Invalid modeline timing")?;
            }
            let (mut hsync, mut vsync) = (0, 0);
            for flag in &parts[9..] {
                match *flag {
                    "+hsync" => hsync = 1,
                    "-hsync" => hsync = -1,
                    "+vsync" => vsync = 1,
                    "-vsync" => vsync = -1,
                    _ => return Err("Unsupported modeline flag in the audited parser".into()),
                }
            }
            return Ok(Self::Modeline(Modeline {
                clock_mhz: clock,
                horizontal: timings[..4].try_into().unwrap(),
                vertical: timings[4..].try_into().unwrap(),
                hsync,
                vsync,
            }));
        }
        let (resolution, refresh) = text
            .split_once('@')
            .map_or((text, None), |(a, b)| (a, Some(b)));
        let (w, h) = resolution
            .split_once('x')
            .ok_or("Choose a mode using the graphical mode builder")?;
        Ok(Self::Dimensions {
            width: w.parse().map_err(|_| "Invalid mode width")?,
            height: h.parse().map_err(|_| "Invalid mode height")?,
            refresh: refresh
                .map(|s| s.parse().map_err(|_| "Invalid refresh rate"))
                .transpose()?,
        })
    }
    pub fn encode(&self) -> Result<String, String> {
        match self {
            Self::Automatic(s) if MODES.contains(&s.as_str()) => Ok(s.clone()),
            Self::Dimensions {
                width,
                height,
                refresh,
            } => {
                if *width == 0
                    || *height == 0
                    || *width > i32::MAX as u32
                    || *height > i32::MAX as u32
                    || refresh.is_some_and(|r| !r.is_finite() || r <= 0.)
                {
                    return Err("Mode dimensions and refresh must be positive and finite".into());
                }
                Ok(format!(
                    "{width}x{height}{}",
                    refresh.map_or(String::new(), |r| format!("@{r}"))
                ))
            }
            Self::Modeline(m) => {
                if !m.clock_mhz.is_finite()
                    || m.clock_mhz <= 0.
                    || m.clock_mhz > f64::from(u32::MAX) / 1000.
                    || m.horizontal[0] == 0
                    || m.vertical[0] == 0
                    || !m.horizontal.windows(2).all(|p| p[0] <= p[1])
                    || !m.vertical.windows(2).all(|p| p[0] <= p[1])
                    || !(-1..=1).contains(&m.hsync)
                    || !(-1..=1).contains(&m.vsync)
                {
                    return Err("Invalid modeline timing order or clock".into());
                }
                let mut text = format!("modeline {}", m.clock_mhz);
                for n in m.horizontal.iter().chain(&m.vertical) {
                    text.push_str(&format!(" {n}"));
                }
                for (axis, sign) in [("hsync", m.hsync), ("vsync", m.vsync)] {
                    if sign != 0 {
                        text.push_str(&format!(" {}{axis}", if sign > 0 { '+' } else { '-' }));
                    }
                }
                Ok(text)
            }
            _ => Err("Unknown monitor mode".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue as scalar_catalogue, lua, workspace::Workspace, Draft};
    #[test]
    fn every_audited_monitor_field_has_a_typed_export_and_static_import() {
        for version in [BASELINE, AUDITED_MAIN] {
            let fields = catalogue(version);
            assert_eq!(fields.len(), 22);
            for setting in fields {
                let mut rule = Rule::new("field-test");
                rule.output = Output::Description("Test display \"quoted\"".into());
                let value = if setting.lua_key == "icc" {
                    Value::String("/tmp/reference profile.icc".into())
                } else {
                    setting
                        .known_default()
                        .unwrap_or_else(|| panic!("{}", setting.id))
                };
                rule.fields.insert(setting.lua_key.clone(), value.clone());
                let source = rule.export_lua(version).unwrap();
                let report = lua::parse(&source, version, &scalar_catalogue()).unwrap();
                assert!(
                    !report.opaque_effects,
                    "{}: {:?}",
                    setting.id, report.diagnostics
                );
                assert_eq!(report.monitors.len(), 1);
                assert_eq!(
                    report.monitors[0].fields.get(&setting.lua_key),
                    Some(&value)
                );
                assert_eq!(report.monitors[0].output, rule.output);
            }
        }
    }
    #[test]
    fn mode_and_position_builders_round_trip_and_reject_bad_timings() {
        for mode in [
            "preferred",
            "highrr",
            "highres",
            "maxwidth",
            "2560x1440@144.5",
            "1920x1080",
            "modeline 148.5 1920 2008 2052 2200 1080 1084 1089 1125 +hsync -vsync",
        ] {
            let parsed = Mode::parse(mode).unwrap();
            assert_eq!(Mode::parse(&parsed.encode().unwrap()).unwrap(), parsed);
        }
        for mode in [
            "0x1080",
            "1920x1080@NaN",
            "1920x1080@0",
            "modeline 0 1920 2008 2052 2200 1080 1084 1089 1125",
            "modeline 100 1920 1000 2052 2200 1080 1084 1089 1125",
        ] {
            assert!(
                Mode::parse(mode).and_then(|m| m.encode()).is_err(),
                "{mode}"
            );
        }
        for pos in POSITIONS
            .iter()
            .copied()
            .chain(["-1920x-10", "2147483647x-2147483648"])
        {
            let value = Position::parse(pos).unwrap();
            assert_eq!(value.encode(), pos);
        }
        assert!(Position::parse("auto-invalid").is_err());
        assert!(Position::parse("2147483648x0").is_err());
    }
    #[test]
    fn rejects_invalid_fields_aliases_identity_scale_and_mirror_cycles() {
        let mut a = Rule::new("a");
        a.output = Output::Connector("DP-1".into());
        for scale in ["NaN", "inf", "0.24", "-1"] {
            a.fields.insert("scale".into(), Value::String(scale.into()));
            assert!(a.validate(BASELINE).is_err());
        }
        a.fields.clear();
        a.fields
            .insert("reserved".into(), Value::CssGap([-4, 0, 5, 10]));
        assert!(a.validate(BASELINE).is_ok());
        a.fields
            .insert("reserved_area".into(), Value::CssGap([0; 4]));
        assert!(a.validate(BASELINE).is_err());
        a.fields.clear();
        a.fields.insert("unknown".into(), Value::Int(1));
        assert!(a.validate(BASELINE).is_err());
        a.fields.clear();
        a.fields
            .insert("mirror".into(), Value::String("DP-2".into()));
        let mut b = Rule::new("b");
        b.output = Output::Connector("DP-2".into());
        b.fields
            .insert("mirror".into(), Value::String("DP-1".into()));
        assert!(validate(&[a.clone(), b.clone()], BASELINE).is_err());
        let mut clear = Rule::new("clear");
        clear.output = a.output.clone();
        clear
            .fields
            .insert("mirror".into(), Value::String(String::new()));
        assert!(validate(&[a.clone(), b.clone(), clear], BASELINE).is_ok());
        assert!(validate(&[a.clone(), a.clone()], BASELINE).is_err());
        assert!(Output::Connector("desc:display".into()).validate().is_err());
        assert!(Output::Description("\n".into()).validate().is_err());
        assert!(validate(&vec![a; MAX_RULES + 1], BASELINE).is_err());
    }
    #[test]
    fn logical_geometry_uses_explicit_scale_and_rotation_only() {
        let mut rule = Rule::new("geometry");
        rule.fields
            .insert("mode".into(), Value::String("2560x1440@144".into()));
        assert!(rule.logical_size().is_none());
        rule.fields
            .insert("scale".into(), Value::String("2".into()));
        assert_eq!(rule.logical_size(), Some((1280., 720.)));
        rule.fields.insert("transform".into(), Value::Int(1));
        assert_eq!(rule.logical_size(), Some((720., 1280.)));
        rule.fields
            .insert("position".into(), Value::String("-720x10".into()));
        assert_eq!(rule.coordinates(), Some((-720, 10)));
    }
    #[test]
    fn monitor_workspace_round_trip_preserves_order_and_rejects_schema_drift() {
        let a = Rule::new("a");
        let mut b = Rule::new("b");
        b.output = Output::Connector("HDMI-A-1".into());
        let rules = vec![b, a];
        let c = scalar_catalogue();
        let saved = Workspace::new(Draft::new(BASELINE)).with_monitors(rules.clone());
        let bytes = saved.to_json(&c).unwrap();
        let restored = Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &c).unwrap();
        assert_eq!(restored.monitors, rules);
        let mut forged = saved.clone();
        forged.monitor_schema_sha256 = Some("0".repeat(64));
        assert!(forged.to_json(&c).is_err());
        forged.monitor_schema_sha256 = None;
        assert!(forged.to_json(&c).is_err());
        let old = Workspace::new(Draft::new(BASELINE)).to_json(&c).unwrap();
        assert!(!std::str::from_utf8(&old).unwrap().contains("monitor"));
        assert!(Workspace::from_json(std::str::from_utf8(&old).unwrap(), &c)
            .unwrap()
            .monitors
            .is_empty());
    }
    #[test]
    fn opaque_monitor_expressions_are_not_executed_and_aliases_are_rejected() {
        for source in ["hl.monitor{output='DP-1',scale=os.execute('touch /tmp/do-not-create')};hl.monitor{output='DP-2'}", "local hl={};hl.monitor{output='DP-1'}"] {
            let report=lua::parse(source,BASELINE,&scalar_catalogue()).unwrap();assert!(report.opaque_effects);assert!(report.monitors.is_empty());
        }
        for fields in [
            "scale='1',scale='2'",
            "reserved=0,reserved_area=0",
            "unknown=true",
            "reserved={top=1,top=2}",
            "output='DP-2'",
        ] {
            let report = lua::parse(
                &format!("hl.monitor{{output='DP-1',{fields}}}"),
                BASELINE,
                &scalar_catalogue(),
            )
            .unwrap();
            assert!(report.monitors.is_empty(), "{fields}");
            assert!(!report.diagnostics.is_empty());
        }
        let report=lua::parse("local n=2;hl.monitor{output='DP-1',scale='1',reserved={top=-4}};hl.config{general={border_size=n}}",BASELINE,&scalar_catalogue()).unwrap();
        assert_eq!(report.monitors.len(), 1);
        assert!(!report.opaque_effects);
        assert_eq!(
            report.draft.values.get("general:border_size"),
            Some(&Value::Int(2))
        );
    }
}
