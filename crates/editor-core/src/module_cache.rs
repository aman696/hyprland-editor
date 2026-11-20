//! Inert cache transitions for the pinned initial require wrappers. Never runs a module.
//! Values should be thin evaluator references: cached tables retain identity and mutations.
use crate::{lua::source_hash, AUDITED_MAIN, BASELINE};
use std::collections::BTreeMap;

pub const MAX_KEYS: usize = 1024;
pub const MAX_KEY_BYTES: usize = 1_048_576;
const STDLIB: &[&[u8]] = &[
    b"_G",
    b"coroutine",
    b"debug",
    b"io",
    b"math",
    b"os",
    b"package",
    b"string",
    b"table",
    b"utf8",
    b"bit32",
    b"jit",
];
pub trait CachedValue: Clone {
    fn is_nil(&self) -> bool;
    fn is_truthy(&self) -> bool;
    fn is_lua_true(&self) -> bool;
    fn is_table(&self) -> bool;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lookup<V> {
    Hit(V),
    Miss,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Missing {
    Absent,
    Unknown,
}
#[derive(Clone, Debug)]
struct Map<V> {
    values: BTreeMap<Vec<u8>, V>,
    missing: Missing,
}
impl<V: CachedValue> Map<V> {
    fn validate_key(key: &[u8]) -> Result<(), String> {
        if key.len() > 16384 || key.contains(&0) {
            return Err("Module cache key exceeds 16 KiB or contains NUL".into());
        }
        Ok(())
    }
    fn lookup(&self, key: &[u8]) -> Result<Lookup<V>, String> {
        Self::validate_key(key)?;
        Ok(match self.values.get(key) {
            Some(v) if v.is_truthy() => Lookup::Hit(v.clone()),
            Some(_) => Lookup::Miss,
            None if self.missing == Missing::Unknown => Lookup::Unknown,
            None => Lookup::Miss,
        })
    }
    fn set(&mut self, key: &[u8], value: V) -> Result<(), String> {
        Self::validate_key(key)?;
        let exists = self.values.contains_key(key);
        if self.values.len() + usize::from(!exists) > MAX_KEYS
            || self.values.keys().map(Vec::len).sum::<usize>() + if exists { 0 } else { key.len() }
                > MAX_KEY_BYTES
        {
            return Err("Module cache exceeds 1024 keys or 1 MiB retained key bytes".into());
        }
        // Nil tombstones preserve an explicitly known absence after invalidating known table contents.
        self.values.insert(key.to_vec(), value);
        Ok(())
    }
    fn invalidate(&mut self) {
        self.values.clear();
        self.missing = Missing::Unknown;
    }
    fn reload(&mut self) -> Result<(), String> {
        // Unknown keys can include numeric keys whose conversion during the
        // upstream lua_next loop aborts reset. Do not infer absence from that state.
        if self.missing != Missing::Absent {
            return Err("Cache reset cannot be certified with unresolved keys".into());
        }
        self.values
            .retain(|key, _| STDLIB.contains(&key.as_slice()));
        Ok(())
    }
}
#[derive(Clone, Debug)]
enum Global<V> {
    RegistryAlias,
    Detached(Map<V>),
    NotTable,
}
#[derive(Clone, Debug)]
/// Known initial loader storage identities only. Arbitrary code that can replace
/// registry tables requires stopping replay or discarding this whole model;
/// entry invalidation alone does not prove those identities are unchanged.
pub struct Cache<V> {
    registry: Map<V>,
    global: Global<V>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protection {
    OriginalRequire,
    SafeRequire,
    WildcardElement,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    ModuleNotFound,
    Other,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failed<V> {
    Propagate,
    Fallback { value: V, logged: bool },
}

pub fn schema_hash() -> String {
    source_hash(&format!(
        "{}{}",
        include_str!("../../../data/source-graph-semantics.json"),
        include_str!("../../../data/module-load-fixtures.json")
    ))
}
pub fn classify_error(module: &[u8], message: &[u8]) -> Failure {
    let prefix = [b"module '".as_slice(), module, b"' not found"].concat();
    if message.starts_with(&prefix) {
        Failure::ModuleNotFound
    } else {
        Failure::Other
    }
}
impl<V: CachedValue> Cache<V> {
    /// Caller supplies known initial values, including any captured standard/preloaded state.
    /// Unknown portions must remain Unknown; an empty map is not a runtime observation.
    pub fn new(version: &str, known: BTreeMap<Vec<u8>, V>, complete: bool) -> Result<Self, String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Module cache needs an audited snapshot".into());
        }
        let mut registry = Map {
            values: BTreeMap::new(),
            missing: if complete {
                Missing::Absent
            } else {
                Missing::Unknown
            },
        };
        for (key, value) in known {
            registry.set(&key, value)?;
        }
        Ok(Self {
            registry,
            global: Global::RegistryAlias,
        })
    }
    pub fn regular_lookup(&self, module: &[u8]) -> Result<Lookup<V>, String> {
        self.registry.lookup(module)
    }
    pub fn wildcard_lookup(&self, pattern: &[u8]) -> Result<Lookup<V>, String> {
        match &self.global {
            Global::RegistryAlias => self.registry.lookup(pattern),
            Global::Detached(map) => map.lookup(pattern),
            Global::NotTable => {
                Map::<V>::validate_key(pattern)?;
                Ok(Lookup::Miss)
            }
        }
    }
    pub fn set_registry(&mut self, module: &[u8], value: V) -> Result<(), String> {
        self.registry.set(module, value)
    }
    pub fn set_global(&mut self, module: &[u8], value: V) -> Result<(), String> {
        match &mut self.global {
            Global::RegistryAlias => self.registry.set(module, value),
            Global::Detached(map) => map.set(module, value),
            Global::NotTable => Map::<V>::validate_key(module),
        }
    }
    pub fn rebind_global(
        &mut self,
        known: BTreeMap<Vec<u8>, V>,
        complete: bool,
    ) -> Result<(), String> {
        let mut map = Map {
            values: BTreeMap::new(),
            missing: if complete {
                Missing::Absent
            } else {
                Missing::Unknown
            },
        };
        for (key, value) in known {
            map.set(&key, value)?;
        }
        self.global = Global::Detached(map);
        Ok(())
    }
    pub fn global_not_table(&mut self) {
        self.global = Global::NotTable;
    }
    /// Original Lua require stores its first non-nil return. A nil return uses the
    /// existing registry value (even false), otherwise inserts true if absence is known.
    pub fn returned(&mut self, module: &[u8], value: V, true_value: V) -> Result<V, String> {
        Map::<V>::validate_key(module)?;
        if !value.is_nil() {
            self.registry.set(module, value.clone())?;
            return Ok(value);
        }
        if let Some(existing) = self.registry.values.get(module).filter(|v| !v.is_nil()) {
            return Ok(existing.clone());
        }
        if !self.registry.values.contains_key(module) && self.registry.missing == Missing::Unknown {
            return Err("Module returned nil with unknown registry state".into());
        }
        if !true_value.is_lua_true() {
            return Err("Nil-module completion requires the evaluator's true value".into());
        }
        self.registry.set(module, true_value.clone())?;
        Ok(true_value)
    }
    /// Factory supplies a fresh empty table reference for each captured failure.
    /// Wildcard element failures are logged but do not cache that fallback per file.
    pub fn failed(
        &mut self,
        module: &[u8],
        failure: Failure,
        protection: Protection,
        empty: impl FnOnce() -> V,
    ) -> Result<Failed<V>, String> {
        Map::<V>::validate_key(module)?;
        if protection == Protection::OriginalRequire
            || (protection == Protection::SafeRequire && failure == Failure::ModuleNotFound)
        {
            return Ok(Failed::Propagate);
        }
        let value = empty();
        if !value.is_table() {
            return Err("Failure fallback must be a fresh empty table reference".into());
        }
        if protection == Protection::SafeRequire && !module.is_empty() {
            self.set_global(module, value.clone())?;
        }
        Ok(Failed::Fallback {
            value,
            logged: true,
        })
    }
    /// The wrapper caches its constructed result table by the original pattern string.
    pub fn wildcard_returned(&mut self, pattern: &[u8], table: V) -> Result<(), String> {
        if !table.is_table() {
            return Err("Wildcard result must be its constructed table reference".into());
        }
        self.set_global(pattern, table)
    }
    /// Invalidate entries while the registry table identity remains known.
    pub fn invalidate_registry(&mut self) {
        self.registry.invalidate();
    }
    /// Invalidate entries while the global table/alias identity remains known.
    pub fn invalidate_global(&mut self) {
        match &mut self.global {
            Global::RegistryAlias => self.registry.invalidate(),
            Global::Detached(map) => map.invalidate(),
            Global::NotTable => {}
        }
    }
    /// Exact cache-reset scope only. It does not perform a compositor reload.
    pub fn reset_user_cache(&mut self) -> Result<(), String> {
        match &mut self.global {
            Global::RegistryAlias => self.registry.reload()?,
            Global::Detached(map) => map.reload()?,
            Global::NotTable => return Err("Global package.loaded is not a table".into()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum V {
        Nil,
        Bool(bool),
        Number(i64),
        Table(usize),
    }
    impl CachedValue for V {
        fn is_nil(&self) -> bool {
            *self == V::Nil
        }
        fn is_truthy(&self) -> bool {
            !matches!(self, V::Nil | V::Bool(false))
        }
        fn is_lua_true(&self) -> bool {
            *self == V::Bool(true)
        }
        fn is_table(&self) -> bool {
            matches!(self, V::Table(_))
        }
    }
    fn cache(version: &str) -> Cache<V> {
        Cache::new(version, BTreeMap::new(), true).unwrap()
    }
    fn observation(version: &str, name: &str) -> serde_json::Value {
        let file: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/module-load-fixtures.json")).unwrap();
        file["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["version"] == version && c["case"] == name)
            .unwrap()
            .clone()
    }
    #[test]
    fn independent_source_nil_false_number_and_alias_cache_observations() {
        for (version, label) in [(BASELINE, "baseline"), (AUDITED_MAIN, "current")] {
            for (name, key, value, json_value, reloads) in [
                (
                    "nil_cached_as_true",
                    b"nilmod".as_slice(),
                    V::Nil,
                    serde_json::json!(true),
                    1,
                ),
                (
                    "false_reexecutes",
                    b"falsemod",
                    V::Bool(false),
                    serde_json::json!(false),
                    2,
                ),
                (
                    "number_cached",
                    b"number",
                    V::Number(7),
                    serde_json::json!(7),
                    1,
                ),
            ] {
                let mut c = cache(version);
                let mut loads = 0;
                for _ in 0..2 {
                    if c.regular_lookup(key).unwrap() == Lookup::Miss {
                        loads += 1;
                        c.returned(key, value.clone(), V::Bool(true)).unwrap();
                    }
                }
                let obs = observation(label, name);
                assert_eq!(
                    obs["registry_loaded"][std::str::from_utf8(key).unwrap()],
                    json_value
                );
                assert_eq!(
                    obs["trace"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|v| v.as_str() == std::str::from_utf8(key).ok())
                        .count(),
                    loads
                );
                assert_eq!(loads, reloads);
            }
            let mut c = cache(version);
            for (id, key) in [b"value".as_slice(), b"./value", b"./value.lua"]
                .iter()
                .enumerate()
            {
                assert_eq!(c.regular_lookup(key).unwrap(), Lookup::Miss);
                c.returned(key, V::Table(id), V::Bool(true)).unwrap();
            }
            let obs = observation(label, "path_aliases_distinct_keys");
            assert_eq!(obs["trace"][3]["ab"], false);
            assert_eq!(obs["trace"][3]["bc"], false);
            assert_ne!(
                c.regular_lookup(b"value").unwrap(),
                c.regular_lookup(b"./value").unwrap()
            );
        }
    }
    #[test]
    fn failure_protection_and_wildcard_element_identity_match_source() {
        for (version, label) in [(BASELINE, "baseline"), (AUDITED_MAIN, "current")] {
            let mut c = cache(version);
            assert_eq!(
                c.failed(
                    b"missing",
                    Failure::ModuleNotFound,
                    Protection::SafeRequire,
                    || panic!()
                )
                .unwrap(),
                Failed::Propagate
            );
            assert_eq!(
                observation(label, "not_found_still_throws")["error_count"],
                0
            );
            assert_eq!(
                c.failed(
                    b"runtime_error",
                    Failure::Other,
                    Protection::SafeRequire,
                    || V::Table(1)
                )
                .unwrap(),
                Failed::Fallback {
                    value: V::Table(1),
                    logged: true
                }
            );
            assert_eq!(
                c.regular_lookup(b"runtime_error").unwrap(),
                Lookup::Hit(V::Table(1))
            );
            assert_eq!(
                observation(label, "runtime_error_becomes_cached_empty_table")["trace"][1]["same"],
                true
            );
            for _ in 0..2 {
                assert_eq!(
                    c.failed(
                        b"original",
                        Failure::Other,
                        Protection::OriginalRequire,
                        || panic!()
                    )
                    .unwrap(),
                    Failed::Propagate
                );
                assert_eq!(c.regular_lookup(b"original").unwrap(), Lookup::Miss);
            }
            assert_eq!(
                observation(label, "original_require_bypasses_error_fallback")["error_count"],
                0
            );
            c.failed(b"part", Failure::Other, Protection::WildcardElement, || {
                V::Table(2)
            })
            .unwrap();
            assert_eq!(c.regular_lookup(b"part").unwrap(), Lookup::Miss);
            c.wildcard_returned(b"./parts/*.lua", V::Table(3)).unwrap();
            assert_eq!(
                c.wildcard_lookup(b"./parts/*.lua").unwrap(),
                Lookup::Hit(V::Table(3))
            );
            let obs = observation(label, "wildcard_order_results_and_cached_error");
            assert_eq!(obs["error_count"], 2);
            assert!(obs["registry_loaded"]
                .get("<root>/config/parts/d.lua")
                .is_none());
            assert!(obs["registry_loaded"]
                .get("<root>/config/parts/f.lua")
                .is_none());
        }
    }
    #[test]
    fn global_rebinding_and_reset_follow_actual_source_scope() {
        for (version, label) in [(BASELINE, "baseline"), (AUDITED_MAIN, "current")] {
            let file: serde_json::Value =
                serde_json::from_str(include_str!("../../../data/module-load-fixtures.json"))
                    .unwrap();
            assert_eq!(
                STDLIB
                    .iter()
                    .map(|k| std::str::from_utf8(k).unwrap())
                    .collect::<Vec<_>>(),
                file["standard_keys"][label]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect::<Vec<_>>()
            );
            let mut c = cache(version);
            c.set_registry(b"math", V::Table(1)).unwrap();
            c.returned(b"value", V::Table(2), V::Bool(true)).unwrap();
            c.reset_user_cache().unwrap();
            assert_eq!(c.regular_lookup(b"value").unwrap(), Lookup::Miss);
            assert_eq!(c.regular_lookup(b"math").unwrap(), Lookup::Hit(V::Table(1)));
            let obs = observation(label, "reload_clears_user_cache_preserves_stdlibs");
            assert_eq!(obs["trace"][2]["same"], false);
            assert_eq!(obs["trace"][2]["math_same"], true);
            c.returned(b"value", V::Table(3), V::Bool(true)).unwrap();
            c.rebind_global(BTreeMap::new(), true).unwrap();
            c.reset_user_cache().unwrap();
            assert_eq!(
                c.regular_lookup(b"value").unwrap(),
                Lookup::Hit(V::Table(3))
            );
            assert_eq!(c.wildcard_lookup(b"value").unwrap(), Lookup::Miss);
            assert_eq!(
                observation(label, "reload_after_loaded_rebind")["trace"][1],
                true
            );
            c.failed(b"bad", Failure::Other, Protection::SafeRequire, || {
                V::Table(4)
            })
            .unwrap();
            assert_eq!(c.regular_lookup(b"bad").unwrap(), Lookup::Miss);
            assert_eq!(c.wildcard_lookup(b"bad").unwrap(), Lookup::Hit(V::Table(4)));
        }
    }
    #[test]
    fn nil_return_retains_module_assignment_and_unknown_state_is_not_absence() {
        let mut c = cache(BASELINE);
        assert!(c.returned(b"wrong", V::Nil, V::Number(7)).is_err());
        assert!(c
            .failed(b"wrong", Failure::Other, Protection::SafeRequire, || {
                V::Bool(true)
            })
            .is_err());
        assert!(c.wildcard_returned(b"wrong", V::Bool(true)).is_err());
        c.set_registry(b"x", V::Bool(false)).unwrap();
        assert_eq!(
            c.returned(b"x", V::Nil, V::Bool(true)).unwrap(),
            V::Bool(false)
        );
        c.invalidate_registry();
        assert_eq!(c.regular_lookup(b"x").unwrap(), Lookup::Unknown);
        assert!(c.returned(b"x", V::Nil, V::Bool(true)).is_err());
        c.set_registry(b"x", V::Nil).unwrap();
        assert_eq!(
            c.returned(b"x", V::Nil, V::Bool(true)).unwrap(),
            V::Bool(true)
        );
        assert!(c.reset_user_cache().is_err());
        assert_eq!(c.regular_lookup(b"user").unwrap(), Lookup::Unknown);
        assert_eq!(c.regular_lookup(b"math").unwrap(), Lookup::Unknown);
        for label in ["baseline", "current"] {
            assert_eq!(
                observation(label, "reload_numeric_loaded_key_can_fail")["trace"][0],
                false
            );
            assert_eq!(
                observation(label, "reload_numeric_string_key_is_removed")["trace"],
                serde_json::json!([true, true])
            );
        }
        assert_eq!(
            classify_error(b"x", b"module 'x' not found: fixture"),
            Failure::ModuleNotFound
        );
        assert_eq!(
            classify_error(b"x", b"module 'y' not found"),
            Failure::Other
        );
    }
    #[test]
    fn cache_limits_are_atomic_and_keys_never_become_file_capabilities() {
        let mut c = cache(BASELINE);
        for i in 0..MAX_KEYS {
            c.set_registry(i.to_string().as_bytes(), V::Bool(false))
                .unwrap();
        }
        assert!(c.set_registry(b"overflow", V::Table(1)).is_err());
        assert_eq!(c.regular_lookup(b"overflow").unwrap(), Lookup::Miss);
        assert!(c.regular_lookup(b"bad\0path").is_err());
        assert!(Cache::<V>::new("unknown", BTreeMap::new(), true).is_err());
        c.global_not_table();
        assert_eq!(c.wildcard_lookup(b"*.lua").unwrap(), Lookup::Miss);
        assert!(c.reset_user_cache().is_err());
    }
    #[test]
    fn assigned_cache_values_survive_nil_and_wildcard_failure_but_not_safe_failure() {
        for (version, label) in [(BASELINE, "baseline"), (AUDITED_MAIN, "current")] {
            let mut c = cache(version);
            c.set_registry(b"assigned_false", V::Bool(false)).unwrap();
            assert_eq!(
                c.returned(b"assigned_false", V::Nil, V::Bool(true))
                    .unwrap(),
                V::Bool(false)
            );
            assert_eq!(
                observation(label, "nil_return_retains_assigned_false")["registry_loaded"]
                    ["assigned_false"],
                false
            );
            c.set_registry(b"failed", V::Bool(true)).unwrap();
            c.failed(
                b"failed",
                Failure::Other,
                Protection::WildcardElement,
                || V::Table(9),
            )
            .unwrap();
            assert_eq!(
                c.regular_lookup(b"failed").unwrap(),
                Lookup::Hit(V::Bool(true))
            );
            assert_eq!(
                observation(label, "wildcard_error_retains_module_cache_assignment")["trace"][2],
                true
            );
            c.failed(b"failed", Failure::Other, Protection::SafeRequire, || {
                V::Table(10)
            })
            .unwrap();
            assert_eq!(
                c.regular_lookup(b"failed").unwrap(),
                Lookup::Hit(V::Table(10))
            );
            assert_eq!(
                observation(label, "safe_error_overwrites_module_cache_assignment")["trace"][1]
                    ["same"],
                true
            );
            assert_eq!(
                observation(label, "error_prefix_is_compared_not_resolver_category")["error_count"],
                0
            );
        }
    }
}
