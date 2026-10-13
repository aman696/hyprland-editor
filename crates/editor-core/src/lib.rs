//! Non-executing, version-scoped editing. This crate has no compositor IPC.
pub mod import;
pub mod lua;
pub mod reconcile;
pub mod source;
pub mod source_path;
pub mod storage;
pub mod workspace;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_DOCUMENT_BYTES: usize = 1_048_576;
pub const BASELINE: &str = "0.56.2";
pub const AUDITED_MAIN: &str = "audited-main";
pub const BASELINE_COMMIT: &str = "efb50993780079460b0cbed1363e2166a2de1d9f";
pub const AUDITED_MAIN_COMMIT: &str = "6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7";

#[derive(Clone, Debug, Deserialize)]
pub struct Bounds {
    pub min: Option<f64>,
    pub max: Option<f64>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Setting {
    pub id: String,
    pub version: String,
    pub kind: String,
    pub lua_key: String,
    pub default_expression: String,
    pub bounds: Bounds,
    pub source: String,
}

pub fn catalogue() -> Vec<Setting> {
    serde_json::from_str(include_str!("../../../data/scalars.json"))
        .expect("bundled catalogue must pass CI validation")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", deny_unknown_fields)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Color(u32),
    Gradient { colors: Vec<u32>, angle: f64 },
    Vec2([f64; 2]),
    CssGap([i64; 4]),
    FontWeight(u16),
}

impl Setting {
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        let (kind, number) = match value {
            Value::Bool(_) => ("Bool", None),
            Value::Int(v) => ("Int", Some(*v as f64)),
            Value::Float(v) => ("Float", Some(*v)),
            Value::String(s) => {
                if s.len() > 16_384 || s.contains('\0') {
                    return Err("Text is too long or contains a NUL character".into());
                }
                ("String", None)
            }
            Value::Color(_) => ("Color", None),
            Value::Gradient { colors, angle } => {
                if colors.is_empty() || colors.len() > 32 || !angle.is_finite() {
                    return Err("A gradient needs 1–32 colors and a finite angle".into());
                }
                ("Gradient", None)
            }
            Value::Vec2(v) => {
                if !v.iter().all(|n| n.is_finite()) {
                    return Err("Coordinates must be finite".into());
                }
                ("Vec2", None)
            }
            Value::CssGap(v) => {
                if v.iter().any(|n| *n < 0 || *n > 100_000) {
                    return Err("Each gap must be between 0 and 100000".into());
                }
                ("CssGap", None)
            }
            Value::FontWeight(v) => ("FontWeight", Some(f64::from(*v))),
        };
        if self.kind != kind {
            return Err(format!("Expected {}, received {kind}", self.kind));
        }
        if let Some(number) = number {
            if !number.is_finite() {
                return Err("Number must be finite".into());
            }
            if self.bounds.min.is_some_and(|min| number < min)
                || self.bounds.max.is_some_and(|max| number > max)
            {
                return Err(format!(
                    "Value is outside the audited bounds {:?}–{:?}",
                    self.bounds.min, self.bounds.max
                ));
            }
        }
        Ok(())
    }

    /// A small literal decoder, never an evaluator of the upstream C++ default.
    pub fn known_default(&self) -> Option<Value> {
        let raw = self.default_expression.trim();
        match self.kind.as_str() {
            "Bool" => raw.parse().ok().map(Value::Bool),
            "Int" => raw.parse().ok().map(Value::Int),
            "Float" => raw.trim_end_matches('f').parse().ok().map(Value::Float),
            "String" if raw == "STRVAL_EMPTY" => Some(Value::String(String::new())),
            "String" => serde_json::from_str::<String>(raw).ok().map(Value::String),
            "CssGap" => raw.parse::<i64>().ok().map(|n| Value::CssGap([n; 4])),
            "Vec2" if raw == "Config::VEC2{}" => Some(Value::Vec2([0., 0.])),
            "Color" => parse_color(raw).ok().map(Value::Color),
            "Gradient" => raw
                .strip_prefix("CHyprColor{")
                .and_then(|s| s.strip_suffix('}'))
                .and_then(|s| parse_color(s).ok())
                .map(|color| Value::Gradient {
                    colors: vec![color],
                    angle: 0.,
                }),
            "FontWeight" => raw
                .split_whitespace()
                .next()?
                .parse()
                .ok()
                .map(Value::FontWeight),
            _ => None,
        }
    }
}

pub fn parse_color(text: &str) -> Result<u32, String> {
    let raw = text.trim();
    // Hyprland uses -1 as the inherit sentinel on several color settings.
    if raw == "-1" {
        return Ok(u32::MAX);
    }
    let hex = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .ok_or("Use 0xAARRGGBB")?;
    if hex.len() != 8 {
        return Err("Use eight hexadecimal digits in 0xAARRGGBB order".into());
    }
    u32::from_str_radix(hex, 16).map_err(|_| "Invalid color".into())
}

fn quote_lua(text: &str) -> String {
    // Fixed-width decimal escapes prevent following digits joining an escape.
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if c.is_control() => {
                for b in c.to_string().bytes() {
                    out.push_str(&format!("\\{b:03}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
impl Value {
    pub fn lua(&self) -> String {
        match self {
            Self::Bool(v) => v.to_string(),
            Self::Int(v) => v.to_string(),
            Self::Float(v) => v.to_string(),
            Self::String(v) => quote_lua(v),
            Self::Color(v) => format!("\"0x{v:08X}\""),
            Self::FontWeight(v) => v.to_string(),
            Self::Vec2([x, y]) => format!("{{ {x}, {y} }}"),
            Self::CssGap([top, right, bottom, left]) => {
                format!("{{ top = {top}, right = {right}, bottom = {bottom}, left = {left} }}")
            }
            Self::Gradient { colors, angle } => format!(
                "{{ colors = {{ {} }}, angle = {angle} }}",
                colors
                    .iter()
                    .map(|c| format!("\"0x{c:08X}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub format: u32,
    pub version: String,
    #[serde(deserialize_with = "unique_values")]
    pub values: BTreeMap<String, Value>,
}
fn unique_values<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<BTreeMap<String, Value>, D::Error> {
    struct Values;
    impl<'de> serde::de::Visitor<'de> for Values {
        type Value = BTreeMap<String, Value>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a bounded map of unique draft setting IDs")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            use serde::de::Error;
            let mut values = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, Value>()? {
                if key.len() > 256 || values.len() >= 512 {
                    return Err(M::Error::custom("draft setting map exceeds its limits"));
                }
                if values.insert(key, value).is_some() {
                    return Err(M::Error::custom("duplicate draft setting ID"));
                }
            }
            Ok(values)
        }
    }
    decoder.deserialize_map(Values)
}
impl Draft {
    pub fn new(version: &str) -> Self {
        Self {
            format: 1,
            version: version.into(),
            values: BTreeMap::new(),
        }
    }
    pub fn validate(&self, catalogue: &[Setting]) -> Result<(), String> {
        if self.format != 1 || ![BASELINE, AUDITED_MAIN].contains(&self.version.as_str()) {
            return Err("Unsupported draft format or Hyprland snapshot".into());
        }
        if self.values.len() > 512 {
            return Err("Too many draft settings".into());
        }
        for (key, value) in &self.values {
            let setting = catalogue
                .iter()
                .find(|s| s.version == self.version && s.id == *key)
                .ok_or_else(|| format!("Unknown setting {key} for {}", self.version))?;
            setting.validate(value).map_err(|e| format!("{key}: {e}"))?;
        }
        Ok(())
    }
    pub fn from_json(text: &str, catalogue: &[Setting]) -> Result<Self, String> {
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err("Draft exceeds 1 MiB".into());
        }
        let draft: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        draft.validate(catalogue)?;
        Ok(draft)
    }
    pub fn export_lua(&self, catalogue: &[Setting]) -> Result<String, String> {
        self.validate(catalogue)?;
        let mut out = format!("-- Hyprland Editor draft; snapshot {}\n-- Review before manually integrating. This is not your full configuration.\nhl.config({{\n",self.version);
        for (key, value) in &self.values {
            let setting = catalogue
                .iter()
                .find(|s| s.version == self.version && s.id == *key)
                .unwrap();
            out.push_str(&format!(
                "    [{}] = {},\n",
                quote_lua(&setting.lua_key),
                value.lua()
            ));
        }
        out.push_str("})\n");
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_draft_keys_instead_of_silently_replacing_values() {
        let json = r#"{"format":1,"version":"0.56.2","values":{"general:border_size":{"type":"Int","value":2},"general:border_size":{"type":"Int","value":5}}}"#;
        assert!(Draft::from_json(json, &catalogue())
            .unwrap_err()
            .contains("duplicate"));
    }
    fn border() -> Setting {
        catalogue()
            .into_iter()
            .find(|s| s.version == BASELINE && s.id == "general:border_size")
            .unwrap()
    }
    #[test]
    fn pinned_coverage_and_uniqueness() {
        let c = catalogue();
        assert_eq!(c.iter().filter(|s| s.version == BASELINE).count(), 353);
        assert_eq!(c.iter().filter(|s| s.version == AUDITED_MAIN).count(), 402);
        let keys: std::collections::BTreeSet<_> = c.iter().map(|s| (&s.version, &s.id)).collect();
        assert_eq!(keys.len(), c.len());
    }
    #[test]
    fn rejects_wrong_types_bounds_and_nan() {
        let s = border();
        assert!(s.validate(&Value::Int(20)).is_ok());
        assert!(s.validate(&Value::Int(21)).is_err());
        assert!(s.validate(&Value::Bool(true)).is_err());
        let mut s = s;
        s.kind = "Float".into();
        assert!(s.validate(&Value::Float(f64::NAN)).is_err());
    }
    #[test]
    fn strings_cannot_inject_lua() {
        assert_eq!(
            Value::String("\"\n); os.execute('oops') --\\".into()).lua(),
            "\"\\\"\\010); os.execute('oops') --\\\\\""
        );
        assert_eq!(Value::String("\u{1}99".into()).lua(), "\"\\00199\"");
    }
    #[test]
    fn codecs_keep_lua_semantics() {
        assert_eq!(Value::Vec2([1., 2.]).lua(), "{ 1, 2 }");
        assert_eq!(Value::Color(0xff123456).lua(), "\"0xFF123456\"");
        assert!(Value::CssGap([1, 2, 3, 4]).lua().contains("bottom = 3"));
    }
    #[test]
    fn rejects_unknown_draft_and_version_mixing() {
        assert!(Draft::from_json(
            r#"{"format":1,"version":"latest","values":{}}"#,
            &catalogue()
        )
        .is_err());
        let mut d = Draft::new(BASELINE);
        d.values
            .insert("os.execute".into(), Value::String("x".into()));
        assert!(d.export_lua(&catalogue()).is_err());
        assert!(Draft::from_json(
            r#"{"format":1,"version":"0.56.2","values":{},"exec":"x"}"#,
            &catalogue()
        )
        .is_err());
    }
    #[test]
    fn validated_draft_round_trip() {
        let mut d = Draft::new(BASELINE);
        d.values.insert(border().id, Value::Int(3));
        let copy = Draft::from_json(&serde_json::to_string(&d).unwrap(), &catalogue()).unwrap();
        assert_eq!(copy.values, d.values);
        assert!(copy
            .export_lua(&catalogue())
            .unwrap()
            .contains("[\"general.border_size\"] = 3"));
    }
}
