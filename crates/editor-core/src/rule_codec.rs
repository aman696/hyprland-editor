//! Structured authoring for rule string effects. These codecs do not run Lua or expressions.
//! Unsupported spellings remain literal source values; callers preserve them until explicitly edited.
use crate::{AUDITED_MAIN, BASELINE};
use serde::{Deserialize, Serialize};
fn policy(raw: &str) -> Result<(), String> {
    if raw.len() > 16384 || raw.contains('\0') {
        Err("Rule text exceeds 16 KiB or contains NUL".into())
    } else {
        Ok(())
    }
}
fn trim(raw: &str) -> &str {
    raw.trim_matches(|c: char| c.is_ascii_whitespace())
}
/// Mirrors non-escaped VarList2 splitting/empty tokens for inspected canonical forms.
/// Escaped input remains retained text until the full escaped-list codec is certified.
fn fields(raw: &str, spaces: bool) -> Result<Vec<&str>, String> {
    policy(raw)?;
    if raw.contains('\\') {
        return Err("Escaped list remains an original literal".into());
    }
    if raw.is_empty() {
        return Ok(vec![]);
    }
    let mut items = raw
        .split(|c: char| {
            if spaces {
                c.is_ascii_whitespace()
            } else {
                c == ' '
            }
        })
        .map(trim)
        .collect::<Vec<_>>();
    if raw.ends_with(|c: char| {
        if spaces {
            c.is_ascii_whitespace()
        } else {
            c == ' '
        }
    }) {
        items.pop();
    }
    if items.len() > 128 {
        return Err("List exceeds the structured editor policy".into());
    }
    Ok(items)
}
fn int(raw: &str) -> Result<i32, String> {
    if raw.is_empty() || raw.starts_with('+') {
        return Err("Expected a 32-bit integer".into());
    }
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        i32::from_str_radix(hex, 16).map_err(|_| "Expected a 32-bit integer".into())
    } else {
        raw.parse().map_err(|_| "Expected a 32-bit integer".into())
    }
}
fn number(raw: &str) -> Result<f32, String> {
    if raw.is_empty() || raw.starts_with('+') {
        return Err("Expected a finite opacity number".into());
    }
    let n = raw
        .parse::<f32>()
        .map_err(|_| "Expected a finite opacity number")?;
    let mantissa = raw.split(['e', 'E']).next().unwrap_or(raw);
    if n == 0. && mantissa.bytes().any(|b| (b'1'..=b'9').contains(&b)) {
        return Err("Opacity underflows float32".into());
    }
    if !n.is_finite() {
        return Err("Opacity must be finite".into());
    }
    Ok(n)
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alpha {
    pub amount: f32,
    pub override_value: bool,
}
impl Default for Alpha {
    fn default() -> Self {
        Self {
            amount: 1.,
            override_value: false,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opacity {
    pub active: Alpha,
    pub inactive: Alpha,
    pub fullscreen: Alpha,
}
impl Opacity {
    pub fn decode(raw: &str) -> Result<Self, String> {
        let mut values = [Alpha::default(); 3];
        let mut count = 0;
        for token in fields(raw, false)? {
            match token {
                "opacity" => {}
                "override" => {
                    if count > 0 {
                        values[count - 1].override_value = true;
                    }
                }
                _ => {
                    if count >= 3 {
                        return Err("Opacity has more than three amounts".into());
                    }
                    values[count].amount = number(token)?;
                    count += 1;
                }
            }
        }
        if count == 1 {
            values[1] = values[0];
            values[2] = values[0];
        }
        Ok(Self {
            active: values[0],
            inactive: values[1],
            fullscreen: values[2],
        })
    }
    /// Three explicit channels avoid inheriting the special one/two-channel defaults.
    pub fn encode(&self) -> Result<String, String> {
        let values = [self.active, self.inactive, self.fullscreen];
        if values.iter().any(|v| !v.amount.is_finite()) {
            return Err("Opacity must be finite".into());
        }
        Ok(values
            .iter()
            .map(|a| {
                format!(
                    "{}{}",
                    a.amount,
                    if a.override_value { " override" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join(" "))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullscreenState {
    pub internal: i32,
    pub client: Option<i32>,
}
impl FullscreenState {
    pub fn decode(raw: &str) -> Result<Self, String> {
        let v = fields(raw, true)?;
        let internal = int(v.first().copied().unwrap_or(""))?;
        let client = match v.get(1).copied() {
            None | Some("") => None,
            Some(n) => Some(int(n)?),
        };
        Ok(Self { internal, client })
    }
    pub fn encode(self) -> String {
        format!(
            "{}{}",
            self.internal,
            self.client.map_or(String::new(), |n| format!(" {n}"))
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gradient {
    pub colors: Vec<u32>,
    pub degrees: i32,
}
impl Gradient {
    pub fn validate(&self) -> Result<(), String> {
        if self.colors.is_empty() || self.colors.len() > 10 {
            Err("A rule border gradient needs 1–10 colors".into())
        } else {
            Ok(())
        }
    }
    fn encoded(&self) -> String {
        format!(
            "{} {}deg",
            self.colors
                .iter()
                .map(|c| format!("0x{c:08X}"))
                .collect::<Vec<_>>()
                .join(" "),
            self.degrees
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Border {
    pub active: Option<Gradient>,
    pub inactive: Option<Gradient>,
}
impl Border {
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Unknown border snapshot".into());
        }
        if self.active.is_none() && version == BASELINE {
            return Err("The baseline snapshot requires active border colors".into());
        }
        if self.active.is_none() && self.inactive.is_none() {
            return Err("Select active or inactive border colors, or unset this field".into());
        }
        for g in [self.active.as_ref(), self.inactive.as_ref()]
            .into_iter()
            .flatten()
        {
            g.validate()?;
        }
        Ok(())
    }
    pub fn encode(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let active = self
            .active
            .as_ref()
            .map_or("0deg".into(), Gradient::encoded);
        Ok(if let Some(inactive) = &self.inactive {
            format!("{active} {}", inactive.encoded())
        } else {
            active
        })
    }
    pub fn decode(raw: &str, version: &str) -> Result<Self, String> {
        policy(raw)?;
        // The public Lua descriptor accepts one color directly before legacy fallback.
        if let Ok(color) = color(raw) {
            return Ok(Self {
                active: Some(Gradient {
                    colors: vec![color],
                    degrees: 0,
                }),
                inactive: None,
            });
        }
        Self::decode_legacy(raw, version)
    }
    pub fn decode_legacy(raw: &str, version: &str) -> Result<Self, String> {
        policy(raw)?;
        let tokens = raw.split_ascii_whitespace().collect::<Vec<_>>();
        if tokens.len() > 128 {
            return Err("Border exceeds the structured editor policy".into());
        }
        if tokens.len() == 2 && !tokens[1].contains("deg") {
            let border = Self {
                active: Some(Gradient {
                    colors: vec![color(tokens[0])?],
                    degrees: 0,
                }),
                inactive: Some(Gradient {
                    colors: vec![color(tokens[1])?],
                    degrees: 0,
                }),
            };
            border.validate(version)?;
            return Ok(border);
        }
        let mut active = Gradient {
            colors: vec![],
            degrees: 0,
        };
        let mut inactive = active.clone();
        let mut second = false;
        for token in tokens {
            if let Some(degrees) = token.strip_suffix("deg") {
                if !second {
                    active.degrees = int(degrees)?;
                    second = true;
                } else {
                    inactive.degrees = int(degrees)?;
                }
            } else {
                let gradient = if second { &mut inactive } else { &mut active };
                gradient.colors.push(color(token)?);
            }
        }
        let border = Self {
            active: (!active.colors.is_empty()).then_some(active),
            inactive: (!inactive.colors.is_empty()).then_some(inactive),
        };
        border.validate(version)?;
        Ok(border)
    }
}
/// Color forms suitable for deterministic 8-bit color controls. Exotic numeric
/// literals and out-of-range/non-finite rgba amounts remain original text.
pub fn color(raw: &str) -> Result<u32, String> {
    policy(raw)?;
    if let Some(hex) = raw.strip_prefix("0x") {
        return u32::from_str_radix(hex, 16).map_err(|_| "Invalid 32-bit color".into());
    }
    if let Some(hex) = raw.strip_prefix('#') {
        return match hex.len() {
            3 => {
                let n = u16::from_str_radix(hex, 16).map_err(|_| "Invalid color")?;
                Ok(0xff000000
                    | (u32::from((n >> 8) & 15) * 0x110000)
                    | (u32::from((n >> 4) & 15) * 0x1100)
                    | (u32::from(n & 15) * 0x11))
            }
            6 => u32::from_str_radix(hex, 16)
                .map(|n| n | 0xff000000)
                .map_err(|_| "Invalid color".into()),
            8 => u32::from_str_radix(hex, 16)
                .map(|n| n.rotate_right(8))
                .map_err(|_| "Invalid color".into()),
            _ => Err("Unsupported color spelling".into()),
        };
    }
    let (raw, alpha) = if let Some(s) = raw.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')'))
    {
        (trim(s), true)
    } else if let Some(s) = raw.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        (trim(s), false)
    } else {
        return Err("Unsupported color spelling".into());
    };
    if !raw.contains(',') {
        if raw.len() != if alpha { 8 } else { 6 } {
            return Err("Wrong hexadecimal color length".into());
        }
        let n = u32::from_str_radix(raw, 16).map_err(|_| "Invalid color")?;
        return Ok(if alpha {
            n.rotate_right(8)
        } else {
            n | 0xff000000
        });
    }
    let channels = raw.split(',').map(trim).collect::<Vec<_>>();
    if channels.len() != if alpha { 4 } else { 3 } {
        return Err("Wrong color channel count".into());
    }
    let byte = |i: usize| {
        let raw = channels[i];
        if raw.starts_with('+') {
            return Err("Invalid RGB channel".into());
        }
        if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
            u8::from_str_radix(hex, 16).map_err(|_| "Invalid RGB channel".to_string())
        } else {
            raw.parse::<u8>()
                .map_err(|_| "Invalid RGB channel".to_string())
        }
    };
    let a = if alpha {
        let n = number(channels[3])?;
        if !(0. ..=1.).contains(&n) {
            return Err("Alpha outside color chooser range".into());
        }
        (n * 255.).floor() as u32
    } else {
        255
    };
    Ok(a << 24 | u32::from(byte(0)?) << 16 | u32::from(byte(1)?) << 8 | u32::from(byte(2)?))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub actions: Vec<String>,
}
impl Group {
    pub fn decode(raw: &str) -> Result<Self, String> {
        let actions = fields(raw, true)?.into_iter().map(str::to_string).collect();
        let value = Self { actions };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.actions.len() > 32 {
            return Err("Grouping has more than 32 operations".into());
        }
        for a in &self.actions {
            policy(a)?;
        }
        let mut previous = "";
        for a in &self.actions {
            if ![
                "set", "new", "lock", "invade", "barred", "deny", "override", "unset", "always",
                "group", "",
            ]
            .contains(&a.as_str())
            {
                return Err("Unknown grouping operation remains literal".into());
            }
            if a == "always" && !["set", "lock", "group"].contains(&previous) {
                return Err("Always must immediately follow set, lock or group".into());
            }
            if a == "unset" {
                break;
            }
            if a != "group" {
                previous = a;
            }
        }
        Ok(())
    }
    /// Static bit accumulation only; this is not a prediction that a real window matches.
    pub fn flags(&self, initial: u16) -> Result<u16, String> {
        self.validate()?;
        if initial & 64 != 0 || self.actions.join(" ").trim() == "group" {
            return Ok(initial);
        }
        let mut flags = initial;
        let mut previous = "";
        for action in &self.actions {
            match action.as_str() {
                "group" => continue,
                "set" => flags |= 1,
                "new" => flags |= 5,
                "lock" => flags |= 8,
                "invade" => flags |= 32,
                "barred" => flags |= 4,
                "deny" => flags |= 128,
                "override" => flags = 64,
                "unset" => {
                    flags = 64;
                    break;
                }
                "always" => match previous {
                    "set" | "group" => flags |= 2,
                    "lock" => flags |= 16,
                    _ => {}
                },
                _ => {}
            }
            previous = action;
        }
        Ok(flags)
    }
    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        let raw = self.actions.join(" ");
        policy(&raw)?;
        Ok(raw)
    }
}
pub const SUPPRESS_EVENTS: &[&str] = &[
    "fullscreen",
    "maximize",
    "activate",
    "activatefocus",
    "fullscreenoutput",
    "x11configurerequest",
];
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Events {
    pub items: Vec<String>,
}
impl Events {
    pub fn decode(raw: &str) -> Result<Self, String> {
        let value = Self {
            items: fields(raw, true)?.into_iter().map(str::to_string).collect(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.items.len() > 128
            || self
                .items
                .iter()
                .any(|s| !SUPPRESS_EVENTS.contains(&s.as_str()))
        {
            Err("Unknown event list remains an original literal".into())
        } else {
            Ok(())
        }
    }
    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        Ok(self.items.join(" "))
    }
}
pub const IDLE_MODES: &[&str] = &["none", "always", "focus", "fullscreen"];
pub const CONTENT_MODES: &[&str] = &["none", "photo", "video", "game"];
pub const TONEMAP_MODES: &[&str] = &["off", "on", "clamp", "limited"];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opacity_defaults_replication_override_and_empty_tokens_are_distinct() {
        assert_eq!(Opacity::decode("").unwrap(), Opacity::default());
        let o = Opacity::decode("override opacity 0.5 override").unwrap();
        assert_eq!(o.active, o.inactive);
        assert_eq!(o.active, o.fullscreen);
        assert!(o.active.override_value);
        let o = Opacity::decode("0.5 override 0.25").unwrap();
        assert_eq!(o.fullscreen, Alpha::default());
        assert!(o.active.override_value);
        assert!(!o.inactive.override_value);
        assert_eq!(Opacity::decode(&o.encode().unwrap()).unwrap(), o);
        for raw in [
            "0.5  0.25",
            "0.5 0.25 1 1",
            "inf",
            "nan",
            "+0.5",
            "0.5\\ override",
        ] {
            assert!(Opacity::decode(raw).is_err(), "{raw}");
        }
        assert!(Opacity {
            active: Alpha {
                amount: f32::INFINITY,
                ..Alpha::default()
            },
            ..Opacity::default()
        }
        .encode()
        .is_err());
    }
    #[test]
    fn fullscreen_client_option_hex_and_ignored_tail_follow_the_parser() {
        assert_eq!(
            FullscreenState::decode("2").unwrap(),
            FullscreenState {
                internal: 2,
                client: None
            }
        );
        assert_eq!(
            FullscreenState::decode("0x2 0x3 ignored").unwrap(),
            FullscreenState {
                internal: 2,
                client: Some(3)
            }
        );
        assert_eq!(FullscreenState::decode("2  3").unwrap().client, None);
        assert!(FullscreenState::decode(" 2 3").is_err());
        assert!(FullscreenState::decode("2147483648").is_err());
    }
    #[test]
    fn border_pairs_and_inactive_only_are_versioned_and_color_forms_keep_argb() {
        for raw in [
            "0xFF000000 0xFFFFFFFF",
            "0xFF000000 45deg #fff 90deg",
            "#ff000080",
            "rgba(ff000080)",
            "rgba(255, 0, 0, 0.5)",
        ] {
            let b = Border::decode(raw, BASELINE).unwrap();
            assert_eq!(
                Border::decode(&b.encode(BASELINE).unwrap(), BASELINE).unwrap(),
                b
            );
        }
        let raw = "0deg 0xFFFFFFFF 90deg";
        assert!(Border::decode(raw, BASELINE).is_err());
        let b = Border::decode(raw, AUDITED_MAIN).unwrap();
        assert!(b.active.is_none());
        assert_eq!(b.encode(AUDITED_MAIN).unwrap(), raw);
        assert_eq!(color("rgba(255, 0, 0, 0.5)").unwrap(), 0x7fff0000);
        assert_eq!(color("#ff000080").unwrap(), 0x80ff0000);
        let many = Gradient {
            colors: vec![0xff000000; 11],
            degrees: 0,
        };
        assert!(many.validate().is_err());
        assert!(Border::decode("0deg", AUDITED_MAIN).is_err());
    }
    #[test]
    fn group_order_is_preserved_and_invalid_always_is_retained_as_unknown() {
        let raw = "set always override lock always unset deny";
        let g = Group::decode(raw).unwrap();
        assert_eq!(g.encode().unwrap(), raw);
        assert!(Group::decode("new always").is_err());
        assert!(Group::decode("unknown").is_err());
    }
}

#[cfg(test)]
mod source_fixtures {
    use super::*;
    #[test]
    fn typed_codecs_agree_with_pinned_primitive_parser_observations() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/rule-codec-fixtures.json")).unwrap();
        for case in fixtures["cases"].as_array().unwrap() {
            let raw = case["raw"].as_str().unwrap();
            let version = if case["version"] == "baseline" {
                BASELINE
            } else {
                AUDITED_MAIN
            };
            match case["kind"].as_str().unwrap() {
                "opacity" => match Opacity::decode(raw) {
                    Ok(value) => assert_eq!(
                        value,
                        serde_json::from_value::<Opacity>(case["parsed"].clone()).unwrap(),
                        "{raw:?} {version}"
                    ),
                    Err(_) => assert!(case["parsed"].is_null(), "{raw:?} {version}"),
                },
                "fullscreen" => match FullscreenState::decode(raw) {
                    Ok(value) => assert_eq!(
                        value,
                        serde_json::from_value::<FullscreenState>(case["parsed"].clone()).unwrap(),
                        "{raw:?} {version}"
                    ),
                    Err(_) => assert!(case["parsed"].is_null(), "{raw:?} {version}"),
                },
                "border" | "border_public_string" => {
                    let value = if case["kind"] == "border" {
                        Border::decode_legacy(raw, version)
                    } else {
                        Border::decode(raw, version)
                    };
                    match value {
                        Ok(value) => assert_eq!(
                            value,
                            serde_json::from_value::<Border>(case["parsed"].clone()).unwrap(),
                            "{raw:?} {version}"
                        ),
                        Err(_) => assert!(case["parsed"].is_null(), "{raw:?} {version}"),
                    }
                }
                kind if kind.starts_with("group:") => {
                    let initial = kind[6..].parse::<u16>().unwrap();
                    if let Ok(group) = Group::decode(raw) {
                        assert_eq!(
                            group.flags(initial).unwrap(),
                            case["parsed"].as_u64().unwrap() as u16,
                            "{raw:?} {version}"
                        );
                    } else {
                        assert_eq!(raw, "set  always", "Unexpected unsupported group fixture");
                    }
                }
                _ => panic!("Unknown fixture kind"),
            }
        }
    }
}
