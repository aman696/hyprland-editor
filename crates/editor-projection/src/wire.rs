//! Bounded staging transport. Decoding checks correlation/integrity, never
//! authenticity or execution safety. A decoded result authorizes no launch.
use super::*;
pub const MAX_RESULT_BYTES: usize = 12 * MAX_DOCUMENT_BYTES;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    lua: String,
    report: Report,
    identity: String,
}
impl From<Projection> for Artifact {
    fn from(value: Projection) -> Self {
        Self {
            lua: value.lua,
            report: value.report,
            identity: value.identity,
        }
    }
}
impl Artifact {
    pub fn lua(&self) -> &str {
        &self.lua
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn verify(&self, expected: &Expected) -> Result<(), String> {
        if self.report.entries.iter().any(|e| {
            e.subject.len() > 512
                || e.reason.len() > 1024
                || e.mocked_intent.as_ref().is_some_and(|s| s.len() > 128)
        }) || self.report.limitations.len() > 16
            || self.report.limitations.iter().any(|s| s.len() > 1024)
        {
            return Err("Staged report exceeds its field limits".into());
        }
        let report = serde_json::to_vec(&self.report).map_err(|e| e.to_string())?;
        if self.lua.len() > MAX_DOCUMENT_BYTES
            || self.report.entries.len() > MAX_ENTRIES
            || report.len() > 4 * MAX_DOCUMENT_BYTES
            || self.report.version != expected.version
            || self.report.upstream_commit != expected.upstream_commit
            || self.report.workspace_sha256 != expected.workspace_sha256
            || self.report.model_sha256 != model_sha256()
            || self.report.compiler != editor_core::lua::replay::compiler_profile()
            || self.report.schema_pins != expected.schema_pins
            || self.report.scenario != expected.scenario
        {
            return Err("Staged projection does not match its bounded input/model/profile".into());
        }
        let identity = editor_core::lua::source_hash(&format!(
            "HYE-PROJECTION-1\0{}\0{}",
            self.lua,
            std::str::from_utf8(&report).map_err(|e| e.to_string())?
        ));
        if identity != self.identity {
            return Err("Staged projection content identity changed".into());
        }
        Ok(())
    }
}

fn bounded_vec<'de, D, T, const LIMIT: usize>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Visitor<T, const N: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const N: usize> serde::de::Visitor<'de> for Visitor<T, N> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "at most {N} items")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            while let Some(value) = seq.next_element()? {
                if values.len() == N {
                    return Err(serde::de::Error::custom(
                        "Staged collection exceeds its limit",
                    ));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Visitor::<T, LIMIT>(std::marker::PhantomData))
}
fn bounded_map<'de, D, const LIMIT: usize>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor<const N: usize>;
    impl<'de, const N: usize> serde::de::Visitor<'de> for Visitor<N> {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "at most {N} unique keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if values.len() == N || values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "Duplicate or excessive staged mapping",
                    ));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(Visitor::<LIMIT>)
}
pub(super) fn outputs<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Output>, D::Error> {
    bounded_vec::<D, Output, 4>(d)
}
pub(super) fn targets<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    bounded_map::<D, 256>(d)
}
pub(super) fn pins<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    bounded_map::<D, 16>(d)
}
pub(super) fn entries<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Entry>, D::Error> {
    bounded_vec::<D, Entry, MAX_ENTRIES>(d)
}
pub(super) fn limitations<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    bounded_vec::<D, String, 16>(d)
}

/// Locally constructed correlation evidence, never deserialized from a worker.
pub struct Expected {
    version: String,
    upstream_commit: String,
    workspace_sha256: String,
    schema_pins: BTreeMap<String, String>,
    scenario: Scenario,
}
impl Expected {
    pub fn new(workspace: &Workspace, scenario: &Scenario) -> Result<Self, String> {
        let bytes = workspace.to_json(&editor_core::catalogue())?;
        scenario.validate(workspace)?;
        Ok(Self {
            version: workspace.draft.version.clone(),
            upstream_commit: workspace.schema.upstream_commit.clone(),
            workspace_sha256: editor_core::lua::source_hash(
                std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
            ),
            schema_pins: schema_pins(workspace),
            scenario: scenario.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_collections_refuse_excessive_counts_duplicates_and_hidden_fields() {
        let workspace = Workspace::new(editor_core::Draft::new(editor_core::BASELINE));
        let scenario = Scenario::default();
        let result =
            serde_json::to_value(Artifact::from(compile(&workspace, &scenario).unwrap())).unwrap();
        for mutation in 0..4 {
            let mut value = result.clone();
            match mutation {
                0 => {
                    value["report"]["entries"] =
                        vec![value["report"]["entries"][0].clone(); MAX_ENTRIES + 1].into()
                }
                1 => value["report"]["limitations"] = vec!["x"; 17].into(),
                2 => {
                    value["report"]["scenario"]["outputs"] =
                        vec![value["report"]["scenario"]["outputs"][0].clone(); 5].into()
                }
                _ => value["report"]["hidden"] = true.into(),
            }
            assert!(serde_json::from_value::<Artifact>(value).is_err());
        }
        assert!(serde_json::from_str::<Scenario>(r#"{"outputs":[],"monitor_targets":{"same":"HEADLESS-1","same":"HEADLESS-1"},"device_targets":{}}"#).is_err());
    }
    #[test]
    fn staged_correlation_refuses_other_inputs_scenarios_profiles_and_changed_content() {
        let workspace = Workspace::new(editor_core::Draft::new(editor_core::BASELINE));
        let scenario = Scenario::default();
        let expected = Expected::new(&workspace, &scenario).unwrap();
        let bytes =
            serde_json::to_vec(&Artifact::from(compile(&workspace, &scenario).unwrap())).unwrap();
        let artifact: Artifact = serde_json::from_slice(&bytes).unwrap();
        artifact.verify(&expected).unwrap();
        for field in [
            "version",
            "upstream_commit",
            "workspace_sha256",
            "model_sha256",
        ] {
            let mut artifact: Artifact = serde_json::from_slice(&bytes).unwrap();
            match field {
                "version" => artifact.report.version.push('x'),
                "upstream_commit" => artifact.report.upstream_commit.push('x'),
                "workspace_sha256" => artifact.report.workspace_sha256.push('x'),
                _ => artifact.report.model_sha256.push('x'),
            }
            assert!(artifact.verify(&expected).is_err());
        }
        let mut changed: Artifact = serde_json::from_slice(&bytes).unwrap();
        changed.lua.push_str("-- changed");
        assert!(changed.verify(&expected).is_err());
        let mut changed: Artifact = serde_json::from_slice(&bytes).unwrap();
        changed.report.compiler.library_identity.push('x');
        assert!(changed.verify(&expected).is_err());
        let mut changed: Artifact = serde_json::from_slice(&bytes).unwrap();
        changed.report.scenario.outputs[0].width -= 1;
        assert!(changed.verify(&expected).is_err());
        let mut changed: Artifact = serde_json::from_slice(&bytes).unwrap();
        changed.report.schema_pins.remove("scalar");
        assert!(changed.verify(&expected).is_err());
        let mut changed: Artifact = serde_json::from_slice(&bytes).unwrap();
        changed.report.entries[0].reason.push('x');
        assert!(changed.verify(&expected).is_err());
    }
}
