//! Registry dispatch candidates for the pinned hl.config C-API table walk.
//! No value parser, runtime acceptance, traversal winner or editing authority.
use super::replay::{Event, Observed, ValueSource};
use crate::{Setting, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub event_order: usize,
    pub setting_id: String,
    pub lua_key: String,
    pub value_kind: String,
    /// Argument index, then table VALUE row indices. References the immutable Event.
    pub location: Vec<usize>,
    /// Full original string keys, before native NUL truncation or dotted joining.
    pub raw_keys: Vec<Vec<u8>>,
    pub expression: Option<u32>,
    pub ambiguous: bool,
    #[serde(default)]
    pub model_value: Option<super::scalar_codec::Outcome>,
}
impl Candidate {
    /// Resolve row coordinates only within the caller's immutable analysis.
    /// Matching order/API is a shape guard, not a freshness or ownership proof.
    pub fn value<'a>(&self, event: &'a Event) -> Option<&'a Observed> {
        if event.order != self.event_order || event.api != "hl.config" {
            return None;
        }
        let mut path = self.location.iter();
        let mut value = event.arguments.get(*path.next()?)?;
        for &row in path {
            let Observed::Table(rows) = value else {
                return None;
            };
            value = &rows.get(row)?.1;
        }
        Some(value)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    InvalidArgument,
    IgnoredNonstringKey,
    UnknownKey,
    AmbiguousTarget,
    UnresolvedCycle,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub kind: FindingKind,
    pub location: Vec<usize>,
    pub api_key: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub event_order: usize,
    pub candidates: Vec<Candidate>,
    pub findings: Vec<Finding>,
    pub traversal_resolved: bool,
    pub order_within_call_known: bool,
    pub native_values_verified: bool,
}
struct Walker<'a> {
    registry: BTreeMap<&'a [u8], &'a Setting>,
    batch: Batch,
    visits: usize,
    retained: usize,
}
#[derive(Default)]
struct Path {
    location: Vec<usize>,
    keys: Vec<Vec<u8>>,
    raw_bytes: usize,
}
impl Walker<'_> {
    fn retain(&mut self, value: &impl Serialize) -> Result<(), String> {
        struct Count(usize);
        impl std::io::Write for Count {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0 = self.0.saturating_add(b.len());
                if self.0 > MAX_DOCUMENT_BYTES / 2 {
                    return Err(std::io::Error::other("configuration table evidence budget"));
                }
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut count = Count(self.retained);
        serde_json::to_writer(&mut count, value)
            .map_err(|_| "Configuration table evidence exceeds 512 KiB".to_string())?;
        self.retained = count.0;
        Ok(())
    }
    fn finding(&mut self, kind: FindingKind, path: &Path, key: &[u8]) -> Result<(), String> {
        if self.batch.findings.len() >= 2048 {
            return Err("Configuration table finding limit exceeded".into());
        }
        let finding = Finding {
            kind,
            location: path.location.clone(),
            api_key: key.to_vec(),
        };
        self.retain(&finding)?;
        self.batch.findings.push(finding);
        Ok(())
    }
    fn table(
        &mut self,
        value: &Observed,
        source: Option<&ValueSource>,
        prefix: &[u8],
        path: &mut Path,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 32 {
            return Err("Configuration table nesting exceeds 32 levels".into());
        }
        if matches!(value, Observed::Cycle(_)) {
            self.batch.traversal_resolved = false;
            return self.finding(FindingKind::UnresolvedCycle, path, prefix);
        }
        let Observed::Table(rows) = value else {
            return Err("Invalid configuration table shape".into());
        };
        for (index, (key, value)) in rows.iter().enumerate() {
            self.visits += 1;
            if self.visits > 8192 {
                return Err("Configuration table visits exceed 8192".into());
            }
            path.location.push(index);
            let source = source.and_then(|s| s.rows.get(index));
            let Observed::Bytes(raw) = key else {
                self.finding(FindingKind::IgnoredNonstringKey, path, prefix)?;
                path.location.pop();
                continue;
            };
            // std::string(lua_tostring(key)) intentionally uses C-string length.
            let key = &raw[..raw.iter().position(|&b| b == 0).unwrap_or(raw.len())];
            let length = prefix
                .len()
                .saturating_add(usize::from(!prefix.is_empty()))
                .saturating_add(key.len());
            if length > 4096 || path.raw_bytes.saturating_add(raw.len()) > 65536 {
                return Err("Configuration table path evidence exceeds its byte limit".into());
            }
            let mut full = prefix.to_vec();
            if !prefix.is_empty() {
                full.push(b'.');
            }
            full.extend(key);
            path.raw_bytes += raw.len();
            path.keys.push(raw.clone());
            if let Some(setting) = self.registry.get(full.as_slice()) {
                if self.batch.candidates.len() >= 2048 {
                    return Err("Configuration table candidate limit exceeded".into());
                }
                let candidate = Candidate {
                    event_order: self.batch.event_order,
                    setting_id: setting.id.clone(),
                    lua_key: setting.lua_key.clone(),
                    value_kind: setting.kind.clone(),
                    location: path.location.clone(),
                    raw_keys: path.keys.clone(),
                    expression: source.and_then(|s| s.expression),
                    ambiguous: false,
                    model_value: None,
                };
                self.retain(&candidate)?;
                self.batch.candidates.push(candidate);
            } else if matches!(value, Observed::Table(_) | Observed::Cycle(_)) {
                self.table(value, source, &full, path, depth + 1)?;
            } else {
                self.finding(FindingKind::UnknownKey, path, &full)?;
            }
            path.keys.pop();
            path.raw_bytes -= raw.len();
            path.location.pop();
        }
        Ok(())
    }
}

pub fn inspect(event: &Event, version: &str, catalogue: &[Setting]) -> Result<Batch, String> {
    if event.api != "hl.config" || ![BASELINE, AUDITED_MAIN].contains(&version) {
        return Err("Configuration table inspection requires a pinned hl.config event".into());
    }
    let mut registry = BTreeMap::new();
    for setting in catalogue.iter().filter(|s| s.version == version) {
        if registry
            .insert(setting.lua_key.as_bytes(), setting)
            .is_some()
        {
            return Err("Ambiguous registry key in the selected catalogue".into());
        }
    }
    if registry.is_empty() || registry.len() > 1024 {
        return Err("Invalid configuration registry size".into());
    }
    let mut walker = Walker {
        registry,
        batch: Batch {
            event_order: event.order,
            candidates: vec![],
            findings: vec![],
            traversal_resolved: true,
            order_within_call_known: false,
            native_values_verified: false,
        },
        visits: 0,
        retained: 0,
    };
    let mut path = Path {
        location: vec![0],
        ..Default::default()
    };
    if let Some(value @ Observed::Table(_)) = event.arguments.first() {
        walker.table(value, event.argument_sources.first(), b"", &mut path, 0)?;
    } else {
        walker.finding(FindingKind::InvalidArgument, &path, b"")?;
    }
    let mut counts = BTreeMap::new();
    for c in &walker.batch.candidates {
        *counts.entry(c.setting_id.clone()).or_insert(0usize) += 1;
    }
    for c in &mut walker.batch.candidates {
        c.ambiguous = counts[&c.setting_id] > 1;
    }
    let collisions = walker
        .batch
        .candidates
        .iter()
        .filter(|c| c.ambiguous)
        .map(|c| (c.setting_id.clone(), c.lua_key.clone()))
        .collect::<BTreeMap<_, _>>();
    for key in collisions.values() {
        walker.finding(FindingKind::AmbiguousTarget, &path, key.as_bytes())?;
    }
    Ok(walker.batch)
}
