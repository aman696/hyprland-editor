//! Immutable, source-pinned registration metadata. No runtime or file authority.
use super::scalar_codec::{Codec, NamedInteger, NativeValue};
use crate::{AUDITED_MAIN, BASELINE};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Global,
    Device,
    Bootstrap,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredBounds {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub role: Role,
    pub version: String,
    pub id: String,
    pub lua_key: String,
    pub kind: String,
    pub native_default: Option<NativeValue>,
    pub codec: Option<Codec>,
    pub description: Option<String>,
    pub deprecation: Option<String>,
    pub declared_bounds: Option<DeclaredBounds>,
    pub declared_names: Option<Vec<NamedInteger>>,
    pub declared_strings: Option<Vec<Vec<u8>>>,
    pub parent_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    format: String,
    registration_sha256: String,
    meaning: String,
    entries: Vec<Entry>,
}
const DATA: &str = include_str!("../../../../data/scalar-semantics.json");
static STORE: LazyLock<Store> = LazyLock::new(|| {
    let store: Store =
        serde_json::from_str(DATA).expect("Bundled registration semantics must pass CI");
    assert_eq!(store.format, "hye-scalar-semantics-1");
    assert_eq!(store.registration_sha256.len(), 64);
    assert!(!store.meaning.is_empty());
    assert_eq!(store.entries.len(), 849);
    store
});
/// The thin generated schema is embedded; reference fixture/source files are not.
pub fn schema_sha256() -> String {
    super::source_hash(DATA)
}
pub fn entries(version: &str, role: Role) -> impl Iterator<Item = &'static Entry> + '_ {
    STORE.entries.iter().filter(move |e| {
        e.version == version && e.role == role && [BASELINE, AUDITED_MAIN].contains(&version)
    })
}
pub fn find(version: &str, role: Role, id: &str) -> Option<&'static Entry> {
    entries(version, role).find(|e| e.id == id)
}
pub fn by_lua_key(version: &str, key: &str) -> Option<&'static Entry> {
    STORE.entries.iter().find(|e| {
        e.version == version
            && e.role != Role::Device
            && e.lua_key == key
            && [BASELINE, AUDITED_MAIN].contains(&version)
    })
}
