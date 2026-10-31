//! Restricted-worker configuration value models, never effective state or edits.
use super::{
    config_table::{self, Batch},
    registration::{self, Role},
    replay::{self, CompilerProfile, Event},
    scalar_codec::{Outcome, Parser},
};
use crate::{Bounds, Setting, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelContext {
    pub version: String,
    pub schema_sha256: String,
    pub profile: Option<CompilerProfile>,
    pub numeric_locale: Option<String>,
    pub runtime_verified: bool,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inspection {
    pub context: ModelContext,
    pub batches: Vec<Batch>,
    pub stopped: Option<String>,
}
fn cost(value: &impl Serialize, limit: usize) -> Result<usize, String> {
    struct Count(usize, usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > self.1 {
                return Err(std::io::Error::other("Configuration model output budget"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0, limit);
    serde_json::to_writer(&mut count, value)
        .map_err(|_| "Configuration value models exceed their data budget".to_string())?;
    Ok(count.0)
}
/// The caller supplies immutable observations, not code for execution. Production
/// calls this only in the existing restricted parser process.
pub fn inspect(events: &[Event], version: &str) -> Result<Inspection, String> {
    if ![BASELINE, AUDITED_MAIN].contains(&version)
        || events.len() > 2048
        || events.iter().any(|e| e.api != "hl.config")
    {
        return Err(
            "Configuration inspection requires at most 2048 pinned hl.config observations".into(),
        );
    }
    let mut result = Inspection {
        context: ModelContext {
            version: version.into(),
            schema_sha256: registration::schema_sha256(),
            profile: None,
            numeric_locale: None,
            runtime_verified: false,
        },
        batches: vec![],
        stopped: None,
    };
    if events.is_empty() {
        return Ok(result);
    }
    let catalogue = registration::entries(version, Role::Global)
        .chain(registration::entries(version, Role::Bootstrap))
        .map(|e| Setting {
            id: e.id.clone(),
            version: e.version.clone(),
            kind: e.kind.clone(),
            lua_key: e.lua_key.clone(),
            default_expression: String::new(),
            bounds: Bounds {
                min: None,
                max: None,
            },
            source: "pinned registration model".into(),
        })
        .collect::<Vec<_>>();
    let mut parser = Parser::new()?;
    result.context.profile = Some(replay::compiler_profile());
    result.context.numeric_locale = Some("C".into());
    let mut retained = 0usize;
    let mut previous = None;
    for event in events {
        let batch = (|| {
            if previous.is_some_and(|n| n >= event.order) {
                return Err("Configuration observations are not ordered".into());
            }
            previous = Some(event.order);
            let mut batch = config_table::inspect(event, version, &catalogue)?;
            for candidate in &mut batch.candidates {
                let meta = registration::by_lua_key(version, &candidate.lua_key)
                    .ok_or("Registered model metadata is missing")?;
                let value = candidate
                    .value(event)
                    .ok_or("Registered model row coordinates are invalid")?;
                let outcome = if let Some(codec) = &meta.codec {
                    parser.decode(codec, value)?
                } else {
                    Outcome::Unresolved {
                        reason: format!("{} native value parsing is not modeled", meta.kind),
                    }
                };
                if let Outcome::Unresolved { reason } = &outcome {
                    result.stopped.get_or_insert(reason.clone());
                }
                candidate.model_value = Some(outcome);
            }
            // The original evidence bound now includes typed outcomes. Global
            // retained output is separately limited below; no clipping as success.
            let bytes = cost(&batch, MAX_DOCUMENT_BYTES / 2)?;
            if retained.saturating_add(bytes) > 2 * MAX_DOCUMENT_BYTES {
                return Err("Configuration value models exceed 2 MiB".into());
            }
            retained += bytes;
            Ok(batch)
        })();
        match batch {
            Ok(batch) => {
                if !batch.traversal_resolved {
                    result
                        .stopped
                        .get_or_insert("Configuration table traversal is unresolved".into());
                }
                result.batches.push(batch);
                if result.stopped.is_some() {
                    // Later inert argument observations are not typed as native
                    // values after an unresolved API/effect boundary.
                    break;
                }
            }
            Err(reason) => {
                result.stopped = Some(reason);
                break;
            }
        }
    }
    Ok(result)
}
