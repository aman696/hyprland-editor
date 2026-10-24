//! Ordered workspace declarations. This model never creates workspaces or runs empty hooks.
pub mod source_state;
use crate::{quote_lua, Setting, Value, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_RULES: usize = 1024;
pub fn catalogue(version: &str) -> Vec<Setting> {
    static ALL: std::sync::OnceLock<Vec<Setting>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        serde_json::from_str(include_str!("../../../data/workspaces.json"))
            .expect("checked workspace fields")
    })
    .iter()
    .filter(|s| s.version == version)
    .cloned()
    .collect()
}
pub fn schema_hash() -> String {
    crate::lua::source_hash(concat!(
        include_str!("../../../data/workspaces.json"),
        include_str!("../../../data/workspace-semantics.json")
    ))
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub selector: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default, deserialize_with = "crate::unique_values")]
    pub fields: BTreeMap<String, Value>,
    #[serde(default, deserialize_with = "crate::unique_values")]
    pub layout_options: BTreeMap<String, Value>,
}
impl Rule {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            selector: "1".into(),
            enabled: None,
            fields: BTreeMap::new(),
            layout_options: BTreeMap::new(),
        }
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Unknown workspace snapshot".into());
        }
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("Invalid workspace draft identity".into());
        }
        if self.selector.len() > 16384 || self.selector.contains('\0') {
            return Err("Invalid workspace selector".into());
        }
        if self.fields.len() > 15 || self.layout_options.len() > 128 {
            return Err("Too many workspace fields or layout options".into());
        }
        let fields = catalogue(version);
        for (key, value) in &self.fields {
            let setting = fields
                .iter()
                .find(|s| s.lua_key == *key)
                .ok_or_else(|| format!("Unknown workspace field {key}"))?;
            setting.validate(value)?;
        }
        for (key, value) in &self.layout_options {
            if key.is_empty() || key.len() > 256 || key.chars().any(char::is_control) {
                return Err("Invalid layout option name".into());
            }
            match value {
                Value::Bool(_) | Value::Int(_) => {}
                Value::Float(n) if n.is_finite() => {}
                Value::String(s) if s.len() <= 16384 && !s.contains('\0') => {}
                _ => {
                    return Err(
                        "Layout options need a literal string, bool or finite number".into(),
                    )
                }
            }
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut text = format!(
            "hl.workspace_rule({{ workspace = {}",
            quote_lua(&self.selector)
        );
        if let Some(enabled) = self.enabled {
            text.push_str(&format!(", enabled = {enabled}"));
        }
        for (key, value) in &self.fields {
            text.push_str(&format!(", [ {} ] = {}", quote_lua(key), value.lua()));
        }
        if !self.layout_options.is_empty() {
            text.push_str(", layout_opts = { ");
            for (i, (key, value)) in self.layout_options.iter().enumerate() {
                if i > 0 {
                    text.push_str(", ");
                }
                text.push_str(&format!("[ {} ] = {}", quote_lua(key), value.lua()));
            }
            text.push_str(" }");
        }
        text.push_str(" })\n");
        Ok(text)
    }
}
pub fn validate(rules: &[Rule], version: &str) -> Result<(), String> {
    if rules.len() > MAX_RULES {
        return Err("Workspace draft exceeds 1024 declarations".into());
    }
    let mut ids = BTreeSet::new();
    for rule in rules {
        rule.validate(version)?;
        if !ids.insert(&rule.id) {
            return Err("Duplicate workspace draft identity".into());
        }
    }
    if serde_json::to_vec(rules).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT_BYTES {
        return Err("Workspace declarations exceed 1 MiB".into());
    }
    Ok(())
}
pub fn export_lua(rules: &[Rule], version: &str) -> Result<String, String> {
    validate(rules, version)?;
    let mut text = String::new();
    for rule in rules {
        text.push_str(&rule.export_lua(version)?);
    }
    Ok(text)
}
pub fn same_declarations(a: &[Rule], b: &[Rule]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.selector == b.selector
                && a.enabled == b.enabled
                && a.fields == b.fields
                && a.layout_options == b.layout_options
        })
}
/// Literal declaration consolidation follows replaceOrAdd/mergeLeft. It does
/// not match a runtime workspace, stringify layout values or resolve monitors.
pub fn consolidate(rules: &[Rule]) -> Vec<Rule> {
    let mut result: Vec<Rule> = vec![];
    for rule in rules {
        if let Some(existing) = result
            .iter_mut()
            .find(|r| r.is_enabled() && r.selector == rule.selector)
        {
            for (key, value) in &rule.fields {
                if key == "monitor" && *value == Value::String(String::new()) {
                    continue;
                }
                existing.fields.insert(key.clone(), value.clone());
            }
            existing.layout_options.extend(rule.layout_options.clone());
            // Source mergeLeft deliberately does not copy enabled.
        } else {
            result.push(rule.clone());
        }
    }
    result
}
pub fn findings(rules: &[Rule]) -> Vec<String> {
    let mut active = BTreeSet::new();
    let mut messages = vec![];
    for rule in rules {
        if active.contains(&rule.selector) {
            messages.push(if !rule.is_enabled(){format!("{}: disabled declaration merges into an earlier enabled rule with this selector; it does not disable that rule.",rule.id)}else{format!("{}: later fields/options merge into an earlier enabled declaration of this selector.",rule.id)});
        }
        if rule.is_enabled() {
            active.insert(rule.selector.clone());
        }
        if rule.fields.get("persistent") == Some(&Value::Bool(true)) && rule.selector.contains('[')
        {
            messages.push(format!("{}: a filter matches existing workspaces; persistent creation needs a concrete workspace address.",rule.id));
        }
        if rule
            .fields
            .get("on_created_empty")
            .is_some_and(|v| matches!(v,Value::String(s)if !s.is_empty()))
        {
            messages.push(format!("{}: the empty-workspace hook is a command; editing never executes it and ordinary preview must omit it.",rule.id));
        }
        if messages.len() >= 256 {
            break;
        }
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue as scalar_catalogue, lua};
    #[test]
    fn every_audited_workspace_field_round_trips_as_an_explicit_override() {
        for version in [BASELINE, AUDITED_MAIN] {
            let fields = catalogue(version);
            assert_eq!(fields.len(), 15);
            for field in fields {
                let mut rule = Rule::new("test");
                rule.enabled = Some(false);
                rule.fields
                    .insert(field.lua_key.clone(), field.known_default().unwrap());
                rule.layout_options
                    .insert("orientation".into(), Value::String("center".into()));
                rule.layout_options
                    .insert("custom_bool".into(), Value::Bool(false));
                rule.layout_options
                    .insert("custom_number".into(), Value::Float(1.25));
                let source = rule.export_lua(version).unwrap();
                let report = lua::parse(&source, version, &scalar_catalogue()).unwrap();
                assert!(
                    !report.opaque_effects,
                    "{} {:?}",
                    field.id, report.diagnostics
                );
                assert_eq!(report.workspace_rules.len(), 1);
                let expected = &report.workspace_rules[0];
                assert_eq!(expected.fields, rule.fields);
                assert_eq!(expected.layout_options, rule.layout_options);
                assert_eq!(expected.enabled, rule.enabled);
            }
        }
    }
    #[test]
    fn repeated_selectors_follow_merge_left_and_disabled_declarations_do_not_disable_prior_rule() {
        let mut a = Rule::new("a");
        a.fields
            .insert("monitor".into(), Value::String("DP-1".into()));
        a.fields.insert("persistent".into(), Value::Bool(true));
        a.layout_options.insert("one".into(), Value::Int(1));
        let mut b = Rule::new("b");
        b.enabled = Some(false);
        b.fields
            .insert("monitor".into(), Value::String(String::new()));
        b.fields.insert("persistent".into(), Value::Bool(false));
        b.layout_options.insert("two".into(), Value::Bool(false));
        let merged = consolidate(&[a.clone(), b.clone()]);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].is_enabled());
        assert_eq!(merged[0].fields["monitor"], Value::String("DP-1".into()));
        assert_eq!(merged[0].fields["persistent"], Value::Bool(false));
        assert_eq!(merged[0].layout_options.len(), 2);
        assert!(findings(&[a.clone(), b.clone()])[0].contains("does not disable"));
        let mut disabled = a.clone();
        disabled.enabled = Some(false);
        assert_eq!(consolidate(&[disabled, b, a]).len(), 3);
    }
    #[test]
    fn opaque_workspace_arguments_are_inert_and_invalidate_later_load_time_api_trust() {
        let report=lua::parse("hl.workspace_rule{workspace='1',layout_opts={bad=untrusted()}};hl.config{general={border_size=7}}",BASELINE,&scalar_catalogue()).unwrap();
        assert!(report.opaque_effects);
        assert!(report.workspace_rules.is_empty());
        assert!(report.draft.values.is_empty());
        let report = lua::parse(
            "hl.workspace_rule{workspace='1',on_created_empty='never execute'}",
            BASELINE,
            &scalar_catalogue(),
        )
        .unwrap();
        assert!(!report.opaque_effects);
        assert!(findings(&report.workspace_rules)[0].contains("never executes"));
    }
    #[test]
    fn invalid_fields_nonfinite_options_duplicate_ids_and_oversize_fail_before_export() {
        let mut a = Rule::new("a");
        a.layout_options
            .insert("x".into(), Value::Float(f64::INFINITY));
        assert!(a.export_lua(BASELINE).is_err());
        a.layout_options.clear();
        a.fields.insert("unknown".into(), Value::Bool(true));
        assert!(a.validate(BASELINE).is_err());
        a.fields.clear();
        assert!(validate(&[a.clone(), a.clone()], BASELINE).is_err());
        a.selector = "x\0bad".into();
        assert!(a.validate(BASELINE).is_err());
    }
}
