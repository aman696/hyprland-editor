//! Non-executing dispatcher constructor descriptions and argument values.
use crate::{quote_lua, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Deserialize)]
pub struct Field {
    pub name: String,
    pub kind: String,
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub choices: Vec<String>,
}
impl Field {
    pub fn initial(&self) -> Value {
        match self.kind.as_str() {
            "Bool" => Value::Bool(
                self.default
                    .as_ref()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            ),
            "Number" => Value::Float(self.default.as_ref().and_then(|v| v.as_f64()).unwrap_or(0.)),
            "DirectionOrBool" => Value::Bool(true),
            _ => Value::String(
                self.default
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
            ),
        }
    }
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        let valid = match (self.kind.as_str(), value) {
            ("Bool", Value::Bool(_)) => true,
            ("Number", Value::Float(n)) => n.is_finite(),
            ("Number", Value::Int(_)) => true,
            ("Selector", Value::Int(_)) => true,
            ("Selector", Value::Float(n)) => n.is_finite(),
            ("Selector" | "String" | "DirectionOrBool", Value::String(s)) => {
                s.len() <= 16384
                    && !s.contains('\0')
                    && (self.choices.is_empty() || self.choices.contains(s))
            }
            ("DirectionOrBool", Value::Bool(_)) => true,
            _ => false,
        };
        if !valid {
            return Err(format!("Invalid {} argument ({})", self.name, self.kind));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize)]
pub struct Variant {
    pub id: String,
    pub label: String,
    pub shape: String,
    pub table_required: bool,
    pub fields: Vec<Field>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Definition {
    pub name: String,
    pub version: String,
    pub effect: String,
    pub source: String,
    pub source_sha256: String,
    pub variants: Vec<Variant>,
}
fn all() -> &'static [Definition] {
    static ALL: std::sync::OnceLock<Vec<Definition>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        serde_json::from_str(include_str!("../../../data/actions.json"))
            .expect("checked dispatcher schemas")
    })
}
pub fn catalogue(version: &str) -> Vec<Definition> {
    all()
        .iter()
        .filter(|a| a.version == version)
        .cloned()
        .collect()
}
pub fn find(name: &str, version: &str) -> Option<&'static Definition> {
    all()
        .iter()
        .find(|d| d.name == name && d.version == version)
}
pub fn schema_hash() -> String {
    crate::lua::source_hash(include_str!("../../../data/actions.json"))
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub constructor: String,
    pub variant: String,
    #[serde(default)]
    pub at_load: bool,
    #[serde(default, deserialize_with = "crate::unique_values")]
    pub fields: BTreeMap<String, Value>,
}
impl Action {
    pub fn new(def: &Definition) -> Self {
        let variant = &def.variants[0];
        Self {
            constructor: def.name.clone(),
            variant: variant.id.clone(),
            at_load: false,
            fields: variant
                .fields
                .iter()
                .filter(|f| f.required)
                .map(|f| (f.name.clone(), f.initial()))
                .collect(),
        }
    }
    pub fn definition(&self, version: &str) -> Result<&'static Definition, String> {
        all()
            .iter()
            .find(|a| a.name == self.constructor && a.version == version)
            .ok_or_else(|| "Dispatcher is outside the selected snapshot".into())
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        let definition = self.definition(version)?;
        let variant = definition
            .variants
            .iter()
            .find(|v| v.id == self.variant)
            .ok_or("Unknown dispatcher variant")?;
        for f in &variant.fields {
            if f.required && !self.fields.contains_key(&f.name) {
                return Err(format!("{} needs {}", definition.name, f.name));
            }
        }
        for (key, value) in &self.fields {
            let field = variant
                .fields
                .iter()
                .find(|f| f.name == *key)
                .ok_or_else(|| format!("Unknown argument {key} for this dispatcher variant"))?;
            field.validate(value)?;
        }
        if self.constructor == "workspace.change_id" {
            let number = match self.fields.get("id") {
                Some(Value::Int(n)) => *n as f64,
                Some(Value::Float(n)) => *n,
                _ => 0.,
            };
            if number <= 0. || number >= f64::from(u32::MAX) {
                return Err("Workspace ID must be between 1 and 4294967294".into());
            }
        }
        if ["focus", "window.swap"].contains(&self.constructor.as_str())
            && self.fields.iter().any(|(k, v)| {
                ["next", "prev", "last", "urgent_or_last"].contains(&k.as_str())
                    && *v == Value::Bool(false)
            })
        {
            return Err("The selected dispatcher branch requires its condition to be true".into());
        }
        if self.constructor == "window.move" && self.variant == "out_of_group" {
            if let Some(v) = self.fields.get("out_of_group") {
                match v {
                    Value::Bool(true) => {}
                    Value::String(s)
                        if ["left", "right", "up", "down", "l", "r", "u", "t", "d", "b"]
                            .contains(&s.as_str()) => {}
                    _ => return Err("Out-of-group needs true or a supported direction".into()),
                }
            }
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let def = self.definition(version)?;
        let variant = def.variants.iter().find(|v| v.id == self.variant).unwrap();
        let args = match variant.shape.as_str() {
            "none" => String::new(),
            "position" => self
                .fields
                .get("argument")
                .map(Value::lua)
                .unwrap_or_default(),
            "table" if !variant.table_required && self.fields.is_empty() => String::new(),
            "table" => format!(
                "{{ {} }}",
                self.fields
                    .iter()
                    .map(|(k, v)| format!("[ {} ] = {}", quote_lua(k), v.lua()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => return Err("Invalid dispatcher argument shape".into()),
        };
        Ok(format!("hl.dsp.{}({args})", self.constructor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{binding, lua, AUDITED_MAIN, BASELINE};
    #[test]
    fn every_constructor_variant_exports_and_imports_its_typed_arguments() {
        for version in [BASELINE, AUDITED_MAIN] {
            let defs = catalogue(version);
            assert_eq!(defs.len(), if version == BASELINE { 51 } else { 52 });
            for def in defs {
                for variant in &def.variants {
                    let action = Action {
                        constructor: def.name.clone(),
                        variant: variant.id.clone(),
                        at_load: false,
                        fields: variant
                            .fields
                            .iter()
                            .filter(|f| f.required)
                            .map(|f| (f.name.clone(), f.initial()))
                            .collect(),
                    };
                    action
                        .validate(version)
                        .unwrap_or_else(|e| panic!("{} {}: {e}", def.name, variant.id));
                    let binding = binding::Bind {
                        id: "test".into(),
                        chord: binding::Chord::parse("SUPER + Q"),
                        actions: vec![action.clone()],
                        flags: Default::default(),
                        description: None,
                        description_alias: false,
                        device: None,
                        submap: None,
                    };
                    let text = binding.export_lua(version).unwrap();
                    let report = lua::parse(&text, version, &crate::catalogue()).unwrap();
                    assert!(
                        !report.opaque_effects,
                        "{} {} {:?}",
                        def.name, text, report.diagnostics
                    );
                    assert_eq!(report.bindings.binds[0].actions[0], action);
                }
            }
        }
    }
    #[test]
    fn arguments_reject_unknown_fields_missing_values_nonfinite_and_wrong_version() {
        let def = catalogue(BASELINE)
            .into_iter()
            .find(|d| d.name == "cursor.move")
            .unwrap();
        let mut a = Action::new(&def);
        a.fields.remove("x");
        assert!(a.validate(BASELINE).is_err());
        a.fields.insert("x".into(), Value::Float(f64::NAN));
        assert!(a.validate(BASELINE).is_err());
        a.fields.insert("x".into(), Value::Float(2.));
        a.fields.insert("unknown".into(), Value::Bool(true));
        assert!(a.validate(BASELINE).is_err());
        a.constructor = "evil(); os.execute('x')".into();
        assert!(a.export_lua(BASELINE).is_err());
        let def = catalogue(AUDITED_MAIN)
            .into_iter()
            .find(|d| d.name == "reload_config")
            .unwrap();
        assert!(Action::new(&def).validate(BASELINE).is_err());
    }
}
