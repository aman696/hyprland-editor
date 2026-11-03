//! Source-pinned monitor argument models. No effective rule, hardware state,
//! source ownership, file capability or permission to execute a declaration.
use super::{
    replay::{self, Event, Observed},
    scalar_codec::{Codec, NativeValue, Outcome, Parser},
};
use crate::{AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::LazyLock};
const DATA: &str = include_str!("../../../../data/monitor-source-semantics.json");
const MAX_CALLS: usize = 256;
const MAX_ROWS: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub key: String,
    pub kind: String,
    /// C++ constructor argument text, never a resolved/effective default value.
    pub declared_default_cpp: String,
    pub minimum: Option<i64>,
    pub maximum: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    version: String,
    url: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: String,
    source: Source,
    fields: Vec<Field>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    format: String,
    scope: String,
    snapshots: Vec<Snapshot>,
}
static STORE: LazyLock<Store> = LazyLock::new(|| {
    let store: Store = serde_json::from_str(DATA).expect("Bundled monitor factory declarations");
    assert_eq!(store.format, "hye-monitor-source-semantics-1");
    assert!(!store.scope.is_empty());
    assert_eq!(store.snapshots.len(), 2);
    for row in &store.snapshots {
        assert!(matches!(row.version.as_str(), "baseline" | "current"));
        assert_eq!(row.source.version, row.version);
        assert!(row
            .source
            .url
            .starts_with("https://raw.githubusercontent.com/hyprwm/Hyprland/"));
        assert_eq!(row.source.sha256.len(), 64);
        assert_eq!(row.fields.len(), 22);
        let names: std::collections::BTreeSet<_> = row.fields.iter().map(|f| &f.key).collect();
        assert_eq!(names.len(), 22);
    }
    store
});
pub fn schema_sha256() -> String {
    super::source_hash(DATA)
}
pub fn fields(version: &str) -> Result<&'static [Field], String> {
    let name = match version {
        BASELINE => "baseline",
        AUDITED_MAIN => "current",
        _ => return Err("Unsupported monitor snapshot".into()),
    };
    Ok(&STORE
        .snapshots
        .iter()
        .find(|s| s.version == name)
        .expect("Bundled monitor snapshot")
        .fields)
}
pub fn codec(field: &Field) -> Result<Codec, String> {
    Ok(match field.kind.as_str() {
        "Bool" => Codec::Bool,
        "Int" => Codec::Int {
            minimum: field.minimum,
            maximum: field.maximum,
            names: None,
        },
        "Float" if field.minimum.is_none() && field.maximum.is_none() => Codec::Float {
            minimum: None,
            maximum: None,
        },
        "String" => Codec::String { validator: None },
        "CssGap" => Codec::CssGap {
            minimum: field.minimum,
            maximum: field.maximum,
        },
        _ => return Err("Unreviewed monitor value factory".into()),
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub event_order: usize,
    pub field: String,
    /// Both reserved spellings address this one requested property.
    pub target: String,
    pub row: usize,
    pub raw_key: Vec<u8>,
    pub expression: Option<u32>,
    pub ambiguous: bool,
    pub primary_value: Outcome,
}
impl Candidate {
    /// Coordinates only within the immutable correlated Event; never ownership.
    pub fn value<'a>(&self, event: &'a Event) -> Option<&'a Observed> {
        if event.order != self.event_order || event.api != "hl.monitor" {
            return None;
        }
        let Observed::Table(rows) = event.arguments.first()? else {
            return None;
        };
        let (key, value) = rows.get(self.row)?;
        matches!(key, Observed::Bytes(bytes) if bytes == &self.raw_key).then_some(value)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    InvalidArgument,
    RequiredOutput,
    UnresolvedOutput,
    IgnoredNonstringKey,
    UnknownField,
    DuplicateStringKey,
    AmbiguousProperty,
    RejectedPrimaryValue,
    UnresolvedPrimaryValue,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub kind: FindingKind,
    pub row: Option<usize>,
    pub raw_key: Vec<u8>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub event_order: usize,
    pub output_value: Option<Outcome>,
    pub output_bytes: Option<Vec<u8>>,
    pub candidates: Vec<Candidate>,
    pub findings: Vec<Finding>,
    /// Control flow under the handler/argument model, not actual submission.
    pub models_submission_and_refresh: bool,
    pub order_within_call_known: bool,
    pub existing_rule_state_known: bool,
    pub native_application_verified: bool,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inspection {
    pub version: String,
    pub schema_sha256: String,
    pub compiler: replay::CompilerProfile,
    pub numeric_locale: String,
    pub batches: Vec<Batch>,
    pub stopped: Option<String>,
    pub runtime_verified: bool,
}
fn cost(value: &impl Serialize, limit: usize) -> Result<usize, String> {
    struct Count(usize, usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > self.1 {
                return Err(std::io::Error::other("Monitor argument data limit"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0, limit);
    serde_json::to_writer(&mut count, value)
        .map_err(|_| "Monitor arguments exceed their data limit".to_string())?;
    Ok(count.0)
}
fn batch(event: &Event, fields: &[Field], parser: &mut Parser) -> Result<Batch, String> {
    cost(event, MAX_DOCUMENT_BYTES)?;
    let mut batch = Batch {
        event_order: event.order,
        output_value: None,
        output_bytes: None,
        candidates: vec![],
        findings: vec![],
        models_submission_and_refresh: false,
        order_within_call_known: false,
        existing_rule_state_known: false,
        native_application_verified: false,
    };
    let Observed::Table(rows) = event.arguments.first().unwrap_or(&Observed::Nil) else {
        batch.findings.push(Finding {
            kind: FindingKind::InvalidArgument,
            row: None,
            raw_key: vec![],
        });
        return Ok(batch);
    };
    if rows.len() > MAX_ROWS {
        return Err("Monitor table row count exceeds 4096".into());
    }
    let mut keys = std::collections::BTreeSet::new();
    for (index, (key, _)) in rows.iter().enumerate() {
        if let Observed::Bytes(key) = key {
            if !keys.insert(key) {
                batch.findings.push(Finding {
                    kind: FindingKind::DuplicateStringKey,
                    row: Some(index),
                    raw_key: key.clone(),
                });
            }
        }
    }
    if !batch.findings.is_empty() {
        return Ok(batch);
    }
    // The required lookup is exact, unlike subsequent descriptor dispatch's
    // C-string key conversion. output\0suffix cannot supply this field.
    let Some((_, output)) = rows
        .iter()
        .find(|(k, _)| matches!(k, Observed::Bytes(b) if b == b"output"))
    else {
        batch.findings.push(Finding {
            kind: FindingKind::RequiredOutput,
            row: None,
            raw_key: b"output".to_vec(),
        });
        return Ok(batch);
    };
    let outcome = parser.decode(&Codec::String { validator: None }, output)?;
    match &outcome {
        Outcome::Accepted {
            native: NativeValue::Bytes(bytes),
            ..
        } => batch.output_bytes = Some(bytes.clone()),
        Outcome::Unresolved { .. } => batch.findings.push(Finding {
            kind: FindingKind::UnresolvedOutput,
            row: None,
            raw_key: b"output".to_vec(),
        }),
        _ => batch.findings.push(Finding {
            kind: FindingKind::RequiredOutput,
            row: None,
            raw_key: b"output".to_vec(),
        }),
    }
    batch.output_value = Some(outcome);
    if batch.output_bytes.is_none() {
        return Ok(batch);
    }
    batch.models_submission_and_refresh = true;
    for (row, (key, value)) in rows.iter().enumerate() {
        let Observed::Bytes(raw) = key else {
            batch.findings.push(Finding {
                kind: FindingKind::IgnoredNonstringKey,
                row: Some(row),
                raw_key: vec![],
            });
            continue;
        };
        let native = raw.split(|b| *b == 0).next().unwrap_or_default();
        if native == b"output" {
            continue;
        }
        let Some(field) = fields.iter().find(|f| f.key.as_bytes() == native) else {
            batch.findings.push(Finding {
                kind: FindingKind::UnknownField,
                row: Some(row),
                raw_key: raw.clone(),
            });
            continue;
        };
        let primary_value = parser.decode(&codec(field)?, value)?;
        let finding = match &primary_value {
            Outcome::Rejected { .. } => Some(FindingKind::RejectedPrimaryValue),
            Outcome::Unresolved { .. } => Some(FindingKind::UnresolvedPrimaryValue),
            _ => None,
        };
        if let Some(kind) = finding {
            batch.findings.push(Finding {
                kind,
                row: Some(row),
                raw_key: raw.clone(),
            });
        }
        let expression = event
            .argument_sources
            .first()
            .and_then(|source| source.rows.get(row))
            .and_then(|source| source.expression);
        batch.candidates.push(Candidate {
            event_order: event.order,
            field: field.key.clone(),
            target: if field.key == "reserved_area" {
                "reserved".into()
            } else {
                field.key.clone()
            },
            row,
            raw_key: raw.clone(),
            expression,
            ambiguous: false,
            primary_value,
        });
    }
    let mut targets = BTreeMap::new();
    for candidate in &batch.candidates {
        *targets.entry(candidate.target.clone()).or_insert(0usize) += 1;
    }
    for candidate in &mut batch.candidates {
        if targets[&candidate.target] > 1 {
            candidate.ambiguous = true;
            batch.findings.push(Finding {
                kind: FindingKind::AmbiguousProperty,
                row: Some(candidate.row),
                raw_key: candidate.raw_key.clone(),
            });
        }
    }
    Ok(batch)
}
/// Run only in a restricted parser worker. Input rows are modeled observations;
/// unknown metatables/effects must never be filled with guessed source values.
pub fn inspect(events: &[Event], version: &str) -> Result<Inspection, String> {
    let fields = fields(version)?;
    if events.len() > MAX_CALLS || events.iter().any(|e| e.api != "hl.monitor") {
        return Err("Monitor inspection requires at most 256 pinned monitor observations".into());
    }
    let mut result = Inspection {
        version: version.into(),
        schema_sha256: schema_sha256(),
        compiler: replay::compiler_profile(),
        numeric_locale: "C".into(),
        batches: vec![],
        stopped: None,
        runtime_verified: false,
    };
    if events.is_empty() {
        return Ok(result);
    }
    let mut parser = Parser::new()?;
    let (mut retained, mut previous) = (0usize, None);
    for event in events {
        let next = (|| {
            if previous.is_some_and(|n| n >= event.order) {
                return Err("Monitor observations are not ordered".into());
            }
            previous = Some(event.order);
            let batch = batch(event, fields, &mut parser)?;
            let bytes = cost(&batch, MAX_DOCUMENT_BYTES / 2)?;
            if retained.saturating_add(bytes) > 2 * MAX_DOCUMENT_BYTES {
                return Err("Monitor models exceed 2 MiB".into());
            }
            retained += bytes;
            Ok(batch)
        })();
        match next {
            Ok(batch) => result.batches.push(batch),
            Err(reason) => {
                result.stopped = Some(reason);
                break;
            }
        }
    }
    Ok(result)
}
