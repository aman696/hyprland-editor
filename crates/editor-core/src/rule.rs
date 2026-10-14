//! Inert window/layer declarations. A call is an unordered effect group; calls are ordered.
//! Named calls reuse an existing rule, replace supplied predicates and append effects.
use crate::{quote_lua, Setting, Value, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RULES: usize = 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Window,
    Layer,
}
impl Kind {
    pub fn function(self) -> &'static str {
        match self {
            Self::Window => "window_rule",
            Self::Layer => "layer_rule",
        }
    }
}
pub fn catalogue(kind: Kind, version: &str) -> Vec<Setting> {
    static WINDOW: std::sync::OnceLock<Vec<Setting>> = std::sync::OnceLock::new();
    static LAYER: std::sync::OnceLock<Vec<Setting>> = std::sync::OnceLock::new();
    let all = match kind {
        Kind::Window => WINDOW.get_or_init(|| {
            serde_json::from_str(include_str!("../../../data/window-effects.json"))
                .expect("checked window effects")
        }),
        Kind::Layer => LAYER.get_or_init(|| {
            serde_json::from_str(include_str!("../../../data/layer-effects.json"))
                .expect("checked layer effects")
        }),
    };
    all.iter()
        .filter(|s| s.version == version)
        .cloned()
        .collect()
}
pub fn schema_hash() -> String {
    crate::lua::source_hash(concat!(
        include_str!("../../../data/window-effects.json"),
        include_str!("../../../data/layer-effects.json"),
        include_str!("../../../data/rule-semantics.json")
    ))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchKind {
    Regex,
    Bool,
    Int,
    Workspace,
    Tag,
}
pub fn match_kind(kind: Kind, property: &str) -> Option<MatchKind> {
    use MatchKind::*;
    match (kind, property) {
        (Kind::Layer, "namespace") => Some(Regex),
        (
            Kind::Window,
            "class" | "title" | "initial_class" | "initial_title" | "content" | "xdg_tag",
        ) => Some(Regex),
        (
            Kind::Window,
            "float" | "xwayland" | "fullscreen" | "pin" | "focus" | "group" | "modal",
        ) => Some(Bool),
        (Kind::Window, "fullscreen_state_internal" | "fullscreen_state_client") => Some(Int),
        (Kind::Window, "workspace") => Some(Workspace),
        (Kind::Window, "tag") => Some(Tag),
        _ => None,
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Predicate {
    pub property: String,
    pub value: Value,
}
impl Predicate {
    pub fn validate(&self, kind: Kind) -> Result<(), String> {
        match_kind(kind, &self.property)
            .ok_or("Match property is not consumed by this rule kind")?;
        match &self.value {
            Value::String(s) => text_policy(s),
            Value::Bool(_) => Ok(()),
            Value::Int(_) if kind == Kind::Window => Ok(()),
            _ => Err("Match needs a literal string, bool or window integer; fractional coercion is unresolved".into()),
        }
    }
    /// Hyprland's regex matcher consumes a string and recognizes this prefix.
    /// Checking a sample is isolated worker work, not part of model validation.
    pub fn regex_sample(&self, kind: Kind, text: &str) -> Result<crate::regex::Sample, String> {
        self.validate(kind)?;
        if match_kind(kind, &self.property) != Some(MatchKind::Regex) {
            return Err("This predicate is not a regex".into());
        }
        let value = match &self.value {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            _ => unreachable!(),
        };
        let (pattern, negated) = value
            .strip_prefix("negative:")
            .map_or((value.as_str(), false), |p| (p, true));
        Ok(crate::regex::Sample {
            pattern: pattern.into(),
            text: text.into(),
            negated,
        })
    }
}
fn text_policy(s: &str) -> Result<(), String> {
    if s.len() > 16384 || s.contains('\0') {
        Err("Rule text exceeds 16 KiB or contains NUL".into())
    } else {
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "form",
    content = "value",
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub enum Geometry {
    Text(String),
    Axes([Value; 2]),
}
impl Geometry {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Text(raw) => {
                text_policy(raw)?;
                let trimmed = raw.trim_matches(|c: char| c.is_ascii_whitespace());
                if trimmed.is_empty() {
                    return Ok(());
                } // upstream clearing sentinel
                let split = trimmed
                    .find(|c: char| c.is_ascii_whitespace())
                    .ok_or("Geometry needs two expressions")?;
                if trimmed[split..]
                    .trim_matches(|c: char| c.is_ascii_whitespace())
                    .is_empty()
                {
                    return Err("Geometry needs two expressions".into());
                }
                Ok(())
            }
            Self::Axes(axes) => {
                for axis in axes {
                    match axis {
                        Value::String(s) if !s.is_empty() => text_policy(s)?,
                        Value::Int(_) => {}
                        Value::Float(n) if n.is_finite() => {}
                        _ => {
                            return Err(
                                "Each geometry axis needs a nonempty expression or finite number"
                                    .into(),
                            )
                        }
                    }
                }
                Ok(())
            }
        }
    }
    fn lua(&self) -> String {
        match self {
            Self::Text(s) => quote_lua(s),
            Self::Axes([a, b]) => format!("{{ {}, {} }}", a.lua(), b.lua()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "form",
    content = "value",
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub enum EffectValue {
    Literal(Value),
    Geometry(Geometry),
    BorderText(String),
}
impl EffectValue {
    pub fn lua(&self) -> String {
        match self {
            Self::Literal(v) => v.lua(),
            Self::Geometry(v) => v.lua(),
            Self::BorderText(v) => quote_lua(v),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub key: String,
    pub value: EffectValue,
}
impl Effect {
    pub fn validate(&self, kind: Kind, version: &str) -> Result<(), String> {
        let setting = catalogue(kind, version)
            .into_iter()
            .find(|s| s.lua_key == self.key)
            .ok_or("Effect is outside the selected snapshot")?;
        match &self.value {
            EffectValue::Geometry(v) if setting.kind == "ExpressionVec2" => v.validate(),
            EffectValue::BorderText(s) if kind == Kind::Window && self.key == "border_color" => {
                text_policy(s)
            }
            EffectValue::Literal(v) => setting.validate(v),
            _ => Err("Effect form does not match its audited type".into()),
        }
    }
}
/// One public hl.*_rule call. Effects in the same call have unspecified lua_next order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub kind: Kind,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub predicates: Vec<Predicate>,
    pub effects: Vec<Effect>,
}
impl Rule {
    pub fn new(id: &str, kind: Kind) -> Self {
        Self {
            id: id.into(),
            kind,
            name: None,
            enabled: None,
            predicates: vec![],
            effects: vec![],
        }
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Unknown rule snapshot".into());
        }
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err("Invalid rule draft identity".into());
        }
        if let Some(name) = &self.name {
            text_policy(name)?;
        }
        if self.predicates.len() > 18 || self.effects.len() > 64 {
            return Err("Too many rule predicates or effects".into());
        }
        let mut keys = BTreeSet::new();
        for p in &self.predicates {
            p.validate(self.kind)?;
            if !keys.insert(&p.property) {
                return Err("Duplicate predicate in a Lua table".into());
            }
        }
        keys.clear();
        for e in &self.effects {
            e.validate(self.kind, version)?;
            if !keys.insert(&e.key) {
                return Err(
                    "Duplicate effect in a Lua table; use separate named declarations".into(),
                );
            }
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut fields = vec![];
        if let Some(name) = &self.name {
            fields.push(format!("name = {}", quote_lua(name)));
        }
        if let Some(enabled) = self.enabled {
            fields.push(format!("enabled = {enabled}"));
        }
        if !self.predicates.is_empty() {
            fields.push(format!(
                "match = {{ {} }}",
                self.predicates
                    .iter()
                    .map(|p| format!("[ {} ] = {}", quote_lua(&p.property), p.value.lua()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        fields.extend(
            self.effects
                .iter()
                .map(|e| format!("[ {} ] = {}", quote_lua(&e.key), e.value.lua())),
        );
        Ok(format!(
            "hl.{}({{ {} }})\n",
            self.kind.function(),
            fields.join(", ")
        ))
    }
}
pub fn validate(rules: &[Rule], version: &str) -> Result<(), String> {
    if rules.len() > MAX_RULES {
        return Err("Rule draft exceeds 1024 declarations".into());
    }
    let mut ids = BTreeSet::new();
    for rule in rules {
        rule.validate(version)?;
        if !ids.insert(&rule.id) {
            return Err("Duplicate rule draft identity".into());
        }
    }
    if serde_json::to_vec(rules).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT_BYTES {
        return Err("Rule draft exceeds 1 MiB".into());
    }
    Ok(())
}
pub fn export_lua(rules: &[Rule], version: &str) -> Result<String, String> {
    validate(rules, version)?;
    let mut out = String::new();
    for r in rules {
        out.push_str(&r.export_lua(version)?);
    }
    Ok(out)
}
/// Compare declarations independently of internal draft IDs and Lua table display order.
pub fn same_declarations(a: &[Rule], b: &[Rule]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.kind == b.kind
                && a.name == b.name
                && a.enabled == b.enabled
                && a.predicates
                    .iter()
                    .map(|p| (&p.property, &p.value))
                    .collect::<BTreeMap<_, _>>()
                    == b.predicates
                        .iter()
                        .map(|p| (&p.property, &p.value))
                        .collect::<BTreeMap<_, _>>()
                && a.effects
                    .iter()
                    .map(|e| (&e.key, &e.value))
                    .collect::<BTreeMap<_, _>>()
                    == b.effects
                        .iter()
                        .map(|e| (&e.key, &e.value))
                        .collect::<BTreeMap<_, _>>()
        })
}
#[derive(Clone, Debug, PartialEq)]
pub struct Consolidated {
    pub kind: Kind,
    pub name: Option<String>,
    pub enabled: bool,
    pub predicates: BTreeMap<String, Value>,
    /// Group boundaries are semantic; order inside a group is not runtime ordering.
    pub effect_groups: Vec<Vec<Effect>>,
}
/// Literal accumulation only, never a claim that a real window matches.
pub fn consolidate(rules: &[Rule]) -> Vec<Consolidated> {
    let mut result: Vec<Consolidated> = vec![];
    let mut names = BTreeMap::new();
    for r in rules {
        let identity = r
            .name
            .as_ref()
            .filter(|n| !n.is_empty())
            .map(|n| (r.kind, n.clone()));
        let existing = identity.as_ref().and_then(|n| names.get(n)).copied();
        let index = existing.unwrap_or_else(|| {
            let i = result.len();
            if let Some(n) = identity {
                names.insert(n, i);
            }
            result.push(Consolidated {
                kind: r.kind,
                name: r.name.clone(),
                enabled: true,
                predicates: BTreeMap::new(),
                effect_groups: vec![],
            });
            i
        });
        let rule = &mut result[index];
        rule.enabled = r.enabled.unwrap_or(true);
        rule.predicates.extend(
            r.predicates
                .iter()
                .map(|p| (p.property.clone(), p.value.clone())),
        );
        if !r.effects.is_empty() {
            rule.effect_groups.push(r.effects.clone());
        }
    }
    result
}
pub fn findings(rules: &[Rule]) -> Vec<String> {
    let mut messages = vec![];
    let mut names = BTreeSet::new();
    for r in rules {
        if let Some(name) = r.name.as_ref().filter(|n| !n.is_empty()) {
            if !names.insert((r.kind, name)) {
                messages.push(format!("{}: this named declaration reuses the earlier rule, appends effects and resets enabled to {}. Omitted predicates remain.",r.id,r.enabled.unwrap_or(true)));
            }
        }
        if r.effects.len() > 1 {
            messages.push(format!("{}: effects in one Lua table have unspecified order. For explicit effect order, choose a name and separate single-effect declarations.",r.id));
        }
        for e in &r.effects {
            if matches!(&e.value, EffectValue::Geometry(_)) {
                messages.push(format!("{}: {} geometry is retained without evaluating muParser expressions or certifying runtime dimensions.",r.id,e.key));
            }
            if matches!(&e.value, EffectValue::BorderText(_)) {
                messages.push(format!("{}: legacy active/inactive border text is retained; color parsing is not yet certified.",r.id));
            }
        }
        if messages.len() >= 256 {
            messages.truncate(256);
            break;
        }
    }
    for r in consolidate(rules) {
        if r.predicates.is_empty() && messages.len() < 256 {
            messages.push(format!(
                "{}: no match conditions; Hyprland does not treat this as a match-all rule.",
                r.name.unwrap_or_else(|| "Anonymous rule".into())
            ));
        }
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue as scalars, lua, workspace::Workspace, Draft};
    fn source_report(source: &str, version: &str) -> lua::LuaReport {
        lua::parse(source, version, &scalars()).unwrap()
    }
    fn literal(field: &Setting) -> EffectValue {
        match field.kind.as_str() {
            "ExpressionVec2" => EffectValue::Geometry(Geometry::Axes([
                Value::String("monitor_w - window_w".into()),
                Value::Float(20.5),
            ])),
            "Gradient" => EffectValue::Literal(Value::Gradient {
                colors: vec![0xff00ff00, 0xffff0000],
                angle: 45.,
            }),
            _ => EffectValue::Literal(field.known_default().unwrap()),
        }
    }
    #[test]
    fn all_audited_effects_and_predicates_round_trip_without_executing_expressions() {
        for version in [BASELINE, AUDITED_MAIN] {
            for kind in [Kind::Window, Kind::Layer] {
                let fields = catalogue(kind, version);
                assert_eq!(
                    fields.len(),
                    match (kind, version) {
                        (Kind::Window, BASELINE) => 57,
                        (Kind::Window, _) => 60,
                        (Kind::Layer, _) => 10,
                    }
                );
                for field in fields {
                    let mut rule = Rule::new("rule", kind);
                    rule.name = Some("quoted \"rule\" λ".into());
                    rule.enabled = Some(false);
                    rule.predicates.push(Predicate {
                        property: if kind == Kind::Window {
                            "class"
                        } else {
                            "namespace"
                        }
                        .into(),
                        value: Value::String("negative:foo.*".into()),
                    });
                    rule.effects.push(Effect {
                        key: field.lua_key.clone(),
                        value: literal(&field),
                    });
                    let parsed = source_report(&rule.export_lua(version).unwrap(), version);
                    assert!(!parsed.opaque_effects, "{:?}", parsed.diagnostics);
                    assert_eq!(parsed.rules.len(), 1);
                    let mut expected = rule;
                    expected.id = parsed.rules[0].id.clone();
                    assert_eq!(parsed.rules[0], expected);
                }
            }
            let source="hl.window_rule{match={class='x',title='x',initial_class='x',initial_title='x',float=false,tag='tag',xwayland=false,fullscreen=false,pin=false,focus=false,group=false,modal=false,fullscreen_state_internal=0,fullscreen_state_client=0,workspace='1',content='none',xdg_tag='tag'}};hl.layer_rule{match={namespace=true}}";
            let report = source_report(source, version);
            assert!(!report.opaque_effects);
            assert_eq!(report.rules[0].predicates.len(), 17);
            assert_eq!(report.rules[1].predicates.len(), 1);
        }
    }
    #[test]
    fn named_calls_reset_enabled_replace_supplied_matches_append_groups_and_keep_registration_order(
    ) {
        let report=source_report("hl.window_rule{name='same',enabled=false,match={class='old',title='keep'},center=true,float=true};hl.layer_rule{name='same',match={namespace='panel'},blur=true};hl.window_rule{match={class='anon'},pin=true};hl.window_rule{name='same',match={class='new'},center=false};hl.window_rule{name='',enabled=false};hl.window_rule{name=''}",BASELINE);
        assert!(!report.opaque_effects);
        let merged = consolidate(&report.rules);
        assert_eq!(merged.len(), 5);
        assert_eq!(merged[0].kind, Kind::Window);
        assert_eq!(merged[1].kind, Kind::Layer);
        assert!(merged[0].enabled);
        assert_eq!(merged[0].predicates["class"], Value::String("new".into()));
        assert_eq!(merged[0].predicates["title"], Value::String("keep".into()));
        assert_eq!(
            merged[0]
                .effect_groups
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(
            merged[0].effect_groups[1][0].value,
            EffectValue::Literal(Value::Bool(false))
        );
        assert!(findings(&report.rules)
            .iter()
            .any(|s| s.contains("unspecified order")));
        assert!(findings(&report.rules)
            .iter()
            .any(|s| s.contains("no match conditions")));
        assert_eq!(
            consolidate(
                &source_report(&export_lua(&report.rules, BASELINE).unwrap(), BASELINE).rules
            ),
            merged
        );
    }
    #[test]
    fn geometry_and_legacy_border_text_preserve_clearing_axes_and_input_types() {
        let report=source_report("local axes={'monitor_w - window_w', 9223372036854775807};hl.window_rule{match={class='x'},move=axes,min_size='',max_size='  100\t200 + 1  ',border_color='0xFF000000 45deg 0xFFFFFFFF 90deg'}",BASELINE);
        assert!(!report.opaque_effects, "{:?}", report.diagnostics);
        let saved = report.rules[0].export_lua(BASELINE).unwrap();
        assert!(saved.contains("9223372036854775807"));
        assert!(saved.contains("monitor_w - window_w"));
        assert_eq!(
            source_report(&saved, BASELINE).rules[0].effects,
            report.rules[0].effects
        );
        for source in [
            "hl.window_rule{move={1,2,3}}",
            "hl.window_rule{move={1,2,bad=3}}",
            "hl.window_rule{move={true,2}}",
            "hl.window_rule{move={'',2}}",
            "hl.window_rule{move='10'}",
            "hl.window_rule{move='10   '}",
            "hl.window_rule{border_color={colors={'0xFF000000'},extra=true}}",
            "hl.window_rule{border_color={colors={'0xFF000000',bad='0xFFFFFFFF'}}}",
        ] {
            let r = source_report(source, BASELINE);
            assert!(r.opaque_effects, "{source}");
            assert!(r.rules.is_empty());
        }
    }
    #[test]
    fn unknown_and_ambiguous_fields_stay_opaque_and_evaluated_unknown_calls_taint_following_api() {
        for source in [
            "hl.window_rule{center=true,center=false}",
            "hl.window_rule{match={class='x',class='y'}}",
            "hl.window_rule{match={namespace='x'}}",
            "hl.layer_rule{match={class='x'}}",
            "hl.window_rule{plugin_effect='x'}",
            "hl.window_rule{match={float=0.5}}",
            "hl.window_rule{enabled='false'}",
        ] {
            let r = source_report(source, BASELINE);
            assert!(r.opaque_effects, "{source}");
            assert!(r.rules.is_empty());
        }
        let r=source_report("hl.window_rule{move={untrusted(),2}};hl.window_rule{center=true};hl.config{general={border_size=7}}",BASELINE);
        assert!(r.rules.is_empty());
        assert!(r.draft.values.is_empty());
        assert!(r.opaque_effects);
        let r = source_report(
            "hl.window_rule{plugin_effect='x'};hl.window_rule{center=true}",
            BASELINE,
        );
        assert!(r.opaque_effects);
        assert_eq!(r.rules.len(), 1);
    }
    #[test]
    fn predicate_regex_samples_follow_full_match_prefix_and_report_invalid_negation() {
        let p = Predicate {
            property: "class".into(),
            value: Value::String("negative:foo.*".into()),
        };
        assert!(!crate::regex::check(&p.regex_sample(Kind::Window, "foobar").unwrap()).unwrap());
        assert!(crate::regex::check(&p.regex_sample(Kind::Window, "bar").unwrap()).unwrap());
        let p = Predicate {
            value: Value::String("negative:[".into()),
            ..p
        };
        assert!(crate::regex::check(&p.regex_sample(Kind::Window, "bar").unwrap()).is_err());
        assert!(Predicate {
            property: "float".into(),
            value: Value::Bool(false)
        }
        .regex_sample(Kind::Window, "x")
        .is_err());
    }
    #[test]
    fn bounded_private_rule_storage_requires_matching_schema_and_unique_identity() {
        let report=source_report("hl.window_rule{name='test',enabled=false,match={class='x'},move={'monitor_w - window_w',42}};hl.layer_rule{match={namespace='panel'},blur=false}",BASELINE);
        let workspace = Workspace::new(Draft::new(BASELINE)).with_rules(report.rules.clone());
        let encoded = workspace.to_json(&scalars()).unwrap();
        assert_eq!(
            Workspace::from_json(std::str::from_utf8(&encoded).unwrap(), &scalars())
                .unwrap()
                .rules,
            report.rules
        );
        let mut bad = workspace.clone();
        bad.rule_schema_sha256 = Some("0".repeat(64));
        assert!(bad.to_json(&scalars()).is_err());
        bad = workspace.clone();
        bad.rules.push(bad.rules[0].clone());
        assert!(bad.to_json(&scalars()).is_err());
        let mut rule = Rule::new("bounded", Kind::Window);
        rule.name = Some("x".repeat(16385));
        assert!(rule.validate(BASELINE).is_err());
        rule.name = None;
        rule.predicates.push(Predicate {
            property: "class".into(),
            value: Value::String("x\0y".into()),
        });
        assert!(rule.validate(BASELINE).is_err());
        assert!(serde_json::from_str::<Rule>(r#"{"id":"x","kind":"window","name":null,"enabled":null,"predicates":[],"effects":[],"unknown":1}"#).is_err());
        assert!(validate(&vec![Rule::new("x", Kind::Window); MAX_RULES + 1], BASELINE).is_err());
    }
}
