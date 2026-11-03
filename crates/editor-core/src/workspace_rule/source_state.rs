//! Closed, modeled workspace-rule state. References are nonserializable local
//! values, never compositor handles, source ownership or filesystem authority.
use crate::{
    lua::{self, scalar_codec::NativeValue},
    AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, LazyLock},
};

const DATA: &str = include_str!("../../../../data/workspace-argument-semantics.json");
const OBJECT_DATA: &str = include_str!("../../../../data/workspace-object-semantics.json");
const MAX_STRING: usize = 65_536;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Factory {
    key: String,
    kind: String,
    declared_default_cpp: String,
    minimum: Option<i64>,
    maximum: Option<i64>,
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
struct Pack {
    version: String,
    source: Source,
    fields: Vec<Factory>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    format: String,
    scope: String,
    snapshots: Vec<Pack>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectPack {
    version: String,
    source: Source,
    metatable: String,
    methods: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectStore {
    format: String,
    scope: String,
    snapshots: Vec<ObjectPack>,
}
static OBJECTS: LazyLock<ObjectStore> = LazyLock::new(|| {
    let store: ObjectStore =
        serde_json::from_str(OBJECT_DATA).expect("Bundled workspace object routing");
    assert_eq!(store.format, "hye-workspace-object-semantics-1");
    assert!(!store.scope.is_empty());
    assert_eq!(store.snapshots.len(), 2);
    for pack in &store.snapshots {
        assert!(matches!(pack.version.as_str(), "baseline" | "current"));
        assert_eq!(pack.source.version, pack.version);
        assert!(pack
            .source
            .url
            .starts_with("https://raw.githubusercontent.com/hyprwm/Hyprland/"));
        assert_eq!(pack.source.sha256.len(), 64);
        assert_eq!(pack.metatable, "HL.WorkspaceRule");
        assert_eq!(pack.methods.len(), 2);
        assert_eq!(pack.methods["set_enabled"], "workspaceRuleSetEnabled");
        assert_eq!(pack.methods["is_enabled"], "workspaceRuleIsEnabled");
    }
    store
});
static STORE: LazyLock<Store> = LazyLock::new(|| {
    let store: Store = serde_json::from_str(DATA).expect("Bundled workspace factories");
    assert_eq!(store.format, "hye-workspace-argument-semantics-1");
    assert!(!store.scope.is_empty());
    assert_eq!(store.snapshots.len(), 2);
    for pack in &store.snapshots {
        assert!(matches!(pack.version.as_str(), "baseline" | "current"));
        assert_eq!(pack.source.version, pack.version);
        assert!(pack
            .source
            .url
            .starts_with("https://raw.githubusercontent.com/hyprwm/Hyprland/"));
        assert_eq!(pack.source.sha256.len(), 64);
        assert_eq!(pack.fields.len(), 15);
        assert_eq!(
            pack.fields
                .iter()
                .map(|f| &f.key)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            15
        );
        assert!(pack
            .fields
            .iter()
            .all(|f| !f.declared_default_cpp.is_empty()));
    }
    store
});
pub fn schema_sha256() -> String {
    lua::source_hash(concat!(
        include_str!("../../../../data/workspace-argument-semantics.json"),
        include_str!("../../../../data/workspace-object-semantics.json")
    ))
}
fn factories(version: &str) -> Result<&'static [Factory], Error> {
    let name = match version {
        BASELINE => "baseline",
        AUDITED_MAIN => "current",
        _ => return Err(Error::UnsupportedVersion),
    };
    assert!(OBJECTS.snapshots.iter().any(|p| p.version == name));
    Ok(&STORE
        .snapshots
        .iter()
        .find(|p| p.version == name)
        .expect("Bundled workspace pack")
        .fields)
}

/// Explicit uncertainty remains in the state. `cause` correlates with the
/// caller's immutable argument finding; it is not an accepted native value.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FieldValue {
    Known(NativeValue),
    Unresolved { cause: u32 },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum OptionValue {
    Known(Vec<u8>),
    Unresolved { cause: u32 },
}
/// Inputs follow argument inspection: keys/selector are consumed C-string
/// bytes, known fields passed primary parsing, and option strings were converted.
/// This structure itself does not establish any of those source preconditions.
#[derive(Clone, Debug, Serialize)]
pub struct Declaration {
    pub selector: Vec<u8>,
    pub enabled: bool,
    pub fields: BTreeMap<String, FieldValue>,
    #[serde(serialize_with = "byte_map")]
    pub layout_options: BTreeMap<Vec<u8>, OptionValue>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RuleRecord {
    pub id: u32,
    pub selector: Vec<u8>,
    pub enabled: bool,
    /// Native monitor storage is a nonoptional string, initially empty.
    pub monitor: FieldValue,
    /// Optional native properties retain absent/false/zero/empty distinctions.
    pub fields: BTreeMap<String, FieldValue>,
    #[serde(serialize_with = "byte_map")]
    pub layout_options: BTreeMap<Vec<u8>, OptionValue>,
}
fn byte_map<S: serde::Serializer, V: Serialize>(
    map: &BTreeMap<Vec<u8>, V>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(map.iter())
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub rules: usize,
    pub wrappers: usize,
    pub steps: usize,
    pub retained_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            rules: 1024,
            wrappers: 4096,
            steps: 8192,
            retained_bytes: 4 * MAX_DOCUMENT_BYTES,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedVersion,
    InvalidDeclaration,
    WrongReceiver,
    ExpectedBoolean,
    ExpectedMemberString,
    ForeignReference,
    UnknownReference,
    Budget,
}
#[derive(Debug)]
struct Realm;
/// Opaque model-local identity. There is intentionally no Deserialize/Serialize
/// implementation and no constructor from an integer, path or saved object.
#[derive(Clone, Debug)]
pub struct Reference {
    realm: Arc<Realm>,
    wrapper: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    IsEnabled,
    SetEnabled,
}
/// Numeric member names must first undergo the selected Lua profile's string
/// conversion. This layer receives either converted bytes or a known bad type.
pub enum MemberKey<'a> {
    Converted(&'a [u8]),
    WrongType,
}
#[derive(Clone, Debug)]
pub enum Argument {
    Reference(Reference),
    Bool(bool),
    Other,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallResult {
    Nothing,
    Nil,
    Bool(bool),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisplayValue {
    Expired,
    LiveAddressDependent { rule: u32 },
}
impl DisplayValue {
    pub fn known_prefix(&self) -> &'static [u8] {
        match self {
            Self::Expired => b"HL.WorkspaceRule(expired)",
            Self::LiveAddressDependent { .. } => b"HL.WorkspaceRule(",
        }
    }
}

pub struct State {
    realm: Arc<Realm>,
    version: String,
    limits: Limits,
    rules: Vec<Option<RuleRecord>>,
    wrappers: Vec<u32>,
    retained: usize,
    steps: usize,
    refresh_requests: usize,
}
fn c_bytes(bytes: &[u8]) -> bool {
    bytes.len() <= MAX_STRING && !bytes.contains(&0)
}
fn cost(value: &impl Serialize, limit: usize) -> Result<usize, Error> {
    struct Counter(usize, usize);
    impl std::io::Write for Counter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(b.len());
            if self.0 > self.1 {
                return Err(std::io::Error::other("Workspace state byte budget"));
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut n = Counter(0, limit);
    serde_json::to_writer(&mut n, value).map_err(|_| Error::Budget)?;
    Ok(n.0)
}
impl State {
    pub fn new(version: &str, limits: Limits) -> Result<Self, Error> {
        factories(version)?;
        let maximum = Limits::default();
        if limits.rules == 0
            || limits.rules > maximum.rules
            || limits.wrappers == 0
            || limits.wrappers > maximum.wrappers
            || limits.steps == 0
            || limits.steps > maximum.steps
            || limits.retained_bytes == 0
            || limits.retained_bytes > maximum.retained_bytes
        {
            return Err(Error::Budget);
        }
        Ok(Self {
            realm: Arc::new(Realm),
            version: version.into(),
            limits,
            rules: vec![],
            wrappers: vec![],
            retained: 0,
            steps: 0,
            refresh_requests: 0,
        })
    }
    fn tick(&mut self) -> Result<(), Error> {
        if self.steps >= self.limits.steps {
            return Err(Error::Budget);
        }
        self.steps += 1;
        Ok(())
    }
    fn rule_id(&self, r: &Reference) -> Result<u32, Error> {
        if !Arc::ptr_eq(&r.realm, &self.realm) {
            return Err(Error::ForeignReference);
        }
        self.wrappers
            .get(r.wrapper)
            .copied()
            .ok_or(Error::UnknownReference)
    }
    fn locked(&self, r: &Reference) -> Result<Option<&RuleRecord>, Error> {
        let id = self.rule_id(r)?;
        self.rules
            .get(id as usize)
            .map(Option::as_ref)
            .ok_or(Error::UnknownReference)
    }
    fn validate(&self, input: &Declaration) -> Result<(), Error> {
        if !c_bytes(&input.selector) || input.fields.len() > 15 || input.layout_options.len() > 4096
        {
            return Err(Error::InvalidDeclaration);
        }
        // Validate before making a candidate copy. The count is streaming and
        // does not allocate another full serialized input.
        cost(input, MAX_DOCUMENT_BYTES)?;
        let fields = factories(&self.version)?;
        for (key, value) in &input.fields {
            let field = fields
                .iter()
                .find(|f| f.key == *key)
                .ok_or(Error::InvalidDeclaration)?;
            if let FieldValue::Known(value) = value {
                let valid = match (field.kind.as_str(), value) {
                    ("Bool", NativeValue::Bool(_)) => true,
                    ("String", NativeValue::Bytes(b)) => c_bytes(b),
                    ("Int", NativeValue::Int(n)) => {
                        field.minimum.is_none_or(|v| *n >= v)
                            && field.maximum.is_none_or(|v| *n <= v)
                    }
                    ("CssGap", NativeValue::CssGap(values)) => values.iter().all(|n| {
                        field.minimum.is_none_or(|v| *n >= v)
                            && field.maximum.is_none_or(|v| *n <= v)
                    }),
                    _ => false,
                };
                if !valid {
                    return Err(Error::InvalidDeclaration);
                }
            }
        }
        for (key, value) in &input.layout_options {
            if !c_bytes(key) || matches!(value,OptionValue::Known(b) if !c_bytes(b)) {
                return Err(Error::InvalidDeclaration);
            }
        }
        Ok(())
    }
    /// Native replaceOrAdd/mergeLeft's literal-state model. A declaration's
    /// enabled flag does not replace a matching enabled record's current flag.
    /// Each successful call creates a distinct userdata wrapper.
    pub fn declare(&mut self, input: Declaration) -> Result<Reference, Error> {
        self.tick()?;
        self.validate(&input)?;
        if self.wrappers.len() >= self.limits.wrappers {
            return Err(Error::Budget);
        }
        let existing = self.rules.iter().position(|r| {
            r.as_ref()
                .is_some_and(|r| r.enabled && r.selector == input.selector)
        });
        if existing.is_none() && self.rules.len() >= self.limits.rules {
            return Err(Error::Budget);
        }
        let index = existing.unwrap_or(self.rules.len());
        let mut candidate = existing
            .map(|i| self.rules[i].as_ref().unwrap().clone())
            .unwrap_or_else(|| RuleRecord {
                id: index as u32,
                selector: input.selector.clone(),
                enabled: input.enabled,
                monitor: FieldValue::Known(NativeValue::Bytes(vec![])),
                fields: BTreeMap::new(),
                layout_options: BTreeMap::new(),
            });
        for (key, value) in input.fields {
            if key == "monitor" {
                if !matches!(&value,FieldValue::Known(NativeValue::Bytes(b)) if b.is_empty()) {
                    candidate.monitor = value;
                }
            } else {
                candidate.fields.insert(key, value);
            }
        }
        candidate.layout_options.extend(input.layout_options);
        let before = existing
            .map(|i| cost(self.rules[i].as_ref().unwrap(), self.limits.retained_bytes))
            .transpose()?
            .unwrap_or(0);
        let after = cost(&candidate, self.limits.retained_bytes)?;
        // Wrapper metadata is charged even when no new rule record is added.
        let retained = self
            .retained
            .checked_sub(before)
            .and_then(|n| n.checked_add(after))
            .and_then(|n| n.checked_add(32))
            .filter(|n| *n <= self.limits.retained_bytes)
            .ok_or(Error::Budget)?;
        if let Some(i) = existing {
            self.rules[i] = Some(candidate);
        } else {
            self.rules.push(Some(candidate));
        }
        let wrapper = self.wrappers.len();
        self.wrappers.push(index as u32);
        self.retained = retained;
        self.refresh_requests += 1;
        Ok(Reference {
            realm: self.realm.clone(),
            wrapper,
        })
    }
    /// Model the native userdata push for an already held local rule reference.
    /// The new wrapper does not create or re-enable a rule.
    pub fn wrap(&mut self, reference: &Reference) -> Result<Reference, Error> {
        self.tick()?;
        let rule = self.rule_id(reference)?;
        if self.wrappers.len() >= self.limits.wrappers {
            return Err(Error::Budget);
        }
        let retained = self
            .retained
            .checked_add(32)
            .filter(|n| *n <= self.limits.retained_bytes)
            .ok_or(Error::Budget)?;
        let wrapper = self.wrappers.len();
        self.wrappers.push(rule);
        self.retained = retained;
        Ok(Reference {
            realm: self.realm.clone(),
            wrapper,
        })
    }
    pub fn raw_equal(&self, a: &Reference, b: &Reference) -> Result<bool, Error> {
        self.rule_id(a)?;
        self.rule_id(b)?;
        Ok(a.wrapper == b.wrapper)
    }
    pub fn equal(&self, a: &Reference, b: &Reference) -> Result<bool, Error> {
        Ok(self.locked(a)?.map(|r| r.id) == self.locked(b)?.map(|r| r.id))
    }
    pub fn lookup(&self, r: &Reference, key: &[u8]) -> Result<Option<Method>, Error> {
        self.member(r, MemberKey::Converted(key))
    }
    pub fn member(&self, r: &Reference, key: MemberKey<'_>) -> Result<Option<Method>, Error> {
        self.rule_id(r)?;
        let MemberKey::Converted(key) = key else {
            return Err(Error::ExpectedMemberString);
        };
        if key.len() > MAX_STRING {
            return Err(Error::Budget);
        }
        Ok(match key.split(|b| *b == 0).next().unwrap_or_default() {
            b"is_enabled" => Some(Method::IsEnabled),
            b"set_enabled" => Some(Method::SetEnabled),
            _ => None,
        })
    }
    /// Methods are unbound. The first explicit argument supplies the receiver;
    /// setters validate their boolean before testing the weak target's lifetime.
    pub fn call(&mut self, method: Method, args: &[Argument]) -> Result<CallResult, Error> {
        self.tick()?;
        if args.len() > 4096 {
            return Err(Error::Budget);
        }
        let Some(Argument::Reference(reference)) = args.first() else {
            return Err(Error::WrongReceiver);
        };
        let id = self.rule_id(reference)? as usize;
        match method {
            Method::IsEnabled => Ok(self.rules[id]
                .as_ref()
                .map_or(CallResult::Nil, |r| CallResult::Bool(r.enabled))),
            Method::SetEnabled => {
                let Some(Argument::Bool(value)) = args.get(1) else {
                    return Err(Error::ExpectedBoolean);
                };
                if let Some(old) = &self.rules[id] {
                    // Only the JSON boolean changes length. Do not copy all
                    // retained strings/options just to toggle this flag.
                    let before = if old.enabled { 4 } else { 5 };
                    let after = if *value { 4 } else { 5 };
                    let retained = self
                        .retained
                        .checked_sub(before)
                        .and_then(|n| n.checked_add(after))
                        .filter(|n| *n <= self.limits.retained_bytes)
                        .ok_or(Error::Budget)?;
                    self.rules[id].as_mut().unwrap().enabled = *value;
                    self.retained = retained;
                    self.refresh_requests += 1;
                }
                Ok(CallResult::Nothing)
            }
        }
    }
    /// Live pointer spelling is deliberately unavailable. Logical IDs must not
    /// masquerade as a native memory address or become a cross-analysis handle.
    pub fn display(&self, r: &Reference) -> Result<DisplayValue, Error> {
        Ok(self.locked(r)?.map_or(DisplayValue::Expired, |r| {
            DisplayValue::LiveAddressDependent { rule: r.id }
        }))
    }
    pub fn records(&self) -> impl Iterator<Item = &RuleRecord> {
        self.rules.iter().filter_map(Option::as_ref)
    }
    pub fn retained_bytes(&self) -> usize {
        // Encoded records plus a fixed wrapper metadata charge. This is not
        // measured allocator usage or a whole-process memory certificate.
        self.retained
    }
    /// Modeled refresh requests only; no native refresher is contacted.
    pub fn refresh_requests(&self) -> usize {
        self.refresh_requests
    }
}

#[cfg(test)]
mod tests;
