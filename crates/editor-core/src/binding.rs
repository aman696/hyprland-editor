//! Version-scoped binding declarations. This module never subscribes to input or dispatches actions.
use crate::{action::Action, quote_lua, AUDITED_MAIN, BASELINE};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
};
pub const MAX_BYTES: usize = 4 * crate::MAX_DOCUMENT_BYTES;
pub fn schema_hash() -> String {
    crate::lua::source_hash(concat!(
        include_str!("../../../data/actions.json"),
        include_str!("../../../data/bindings.json")
    ))
}
pub const FLAGS: &[&str] = &[
    "repeating",
    "locked",
    "release",
    "non_consuming",
    "auto_consuming",
    "transparent",
    "ignore_mods",
    "dont_inhibit",
    "long_press",
    "submap_universal",
    "click",
    "drag",
    "allow_input_capture",
    "mouse",
];
pub const MODIFIERS: &[&str] = &[
    "SHIFT", "CAPS", "CTRL", "ALT", "MOD2", "MOD3", "SUPER", "MOD5",
];
pub const SIDED: &[&str] = &[
    "SHIFT_L",
    "SHIFT_R",
    "CONTROL_L",
    "CONTROL_R",
    "CTRL_L",
    "CTRL_R",
    "ALT_L",
    "ALT_R",
    "SUPER_L",
    "SUPER_R",
];
#[link(name = "xkbcommon")]
unsafe extern "C" {
    fn xkb_keysym_from_name(name: *const libc::c_char, flags: u32) -> u32;
}
pub fn keysym(name: &str) -> Option<u32> {
    if name.len() > 256 {
        return None;
    }
    let name = CString::new(name).ok()?;
    let sym = unsafe { xkb_keysym_from_name(name.as_ptr(), 1) };
    (sym != 0).then_some(sym)
}
fn modifier(s: &str) -> bool {
    [
        "SHIFT", "CAPS", "CTRL", "CONTROL", "ALT", "MOD1", "MOD2", "MOD3", "SUPER", "WIN", "LOGO",
        "MOD4", "META", "MOD5",
    ]
    .contains(&s)
}
fn external(s: &str) -> bool {
    s == "catchall"
        || s.starts_with("mouse:")
        || s.starts_with("switch:")
        || ["mouse_down", "mouse_up", "mouse_left", "mouse_right"].contains(&s)
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chord {
    pub tokens: Vec<String>,
}
impl Chord {
    pub fn parse(text: &str) -> Self {
        Self {
            tokens: text
                .split('+')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        }
    }
    pub fn validate(&self, version: &str, in_submap: bool) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version)
            || self.tokens.is_empty()
            || self.tokens.len() > 32
        {
            return Err("Choose 1–32 chord tokens in a known snapshot".into());
        }
        let (mut keys, mut sided_keys, mut events, mut ended) = (0, 0, 0, false);
        let mut seen = BTreeSet::new();
        for token in &self.tokens {
            if token.is_empty()
                || token.len() > 256
                || token.contains('+')
                || token.chars().any(char::is_control)
            {
                return Err("Invalid chord token".into());
            }
            let upper = token.to_ascii_uppercase();
            if version == AUDITED_MAIN && !seen.insert(upper.clone()) {
                return Err("This snapshot rejects repeated chord tokens".into());
            }
            let is_mod = modifier(if version == AUDITED_MAIN {
                &upper
            } else {
                token
            });
            let sided = version == AUDITED_MAIN && SIDED.contains(&upper.as_str());
            if is_mod || sided {
                if ended {
                    return Err("Modifiers must precede keys".into());
                }
                if sided {
                    sided_keys += 1;
                }
                continue;
            }
            ended = true;
            if external(token) {
                events += 1;
                if token == "catchall" && (!in_submap || self.tokens.len() != 1) {
                    return Err("Catchall must be alone inside a submap".into());
                }
            } else if let Some(code) = token.strip_prefix("code:") {
                let code = code.parse::<u32>().map_err(|_| "Invalid XKB keycode")?;
                if code == 0 && version == AUDITED_MAIN {
                    return Err("This snapshot rejects keycode zero".into());
                }
                keys += 1;
            } else {
                keysym(token)
                    .ok_or_else(|| format!("Unknown keysym {token}; use Return for Enter"))?;
                keys += 1;
            }
        }
        if events > 1 || (events > 0 && keys > 0) {
            return Err("External events cannot be combined with ordinary keys".into());
        }
        if keys + sided_keys + events == 0 {
            return Err("Choose a trigger, not only unsided modifiers".into());
        }
        Ok(())
    }
    pub fn text(&self) -> String {
        self.tokens.join(" + ")
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceFilter {
    pub inclusive: bool,
    pub items: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bind {
    pub id: String,
    pub chord: Chord,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub flags: BTreeSet<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub description_alias: bool,
    #[serde(default)]
    pub device: Option<DeviceFilter>,
    #[serde(default)]
    pub submap: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Submap {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub reset: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy: Option<crate::legacy_binding::Draft>,
    pub binds: Vec<Bind>,
    pub submaps: Vec<Submap>,
    #[serde(default)]
    pub order: Vec<String>,
}
fn identity(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
impl Bind {
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if !identity(&self.id) {
            return Err("Invalid bind identity".into());
        }
        self.chord.validate(version, self.submap.is_some())?;
        if self.actions.len() > 64 {
            return Err("A binding supports at most 64 ordered actions".into());
        }
        for a in &self.actions {
            a.validate(version)?;
        }
        for flag in &self.flags {
            if !FLAGS.contains(&flag.as_str()) || (flag == "mouse" && version != AUDITED_MAIN) {
                return Err(format!("Unsupported bind flag {flag}"));
            }
        }
        if self.flags.contains("click") && self.flags.contains("drag") {
            return Err("Click and drag are exclusive".into());
        }
        let release = self.flags.contains("release")
            || self.flags.contains("click")
            || self.flags.contains("drag");
        if self.flags.contains("repeating") && (release || self.flags.contains("long_press")) {
            return Err("Repeating conflicts with release/click/drag/long press".into());
        }
        if self.flags.contains("mouse")
            && (release || self.flags.contains("repeating") || self.flags.contains("locked"))
        {
            return Err("Mouse conflicts with repeating, release and locked".into());
        }
        if self
            .description
            .as_ref()
            .is_some_and(|d| d.len() > 16384 || d.contains('\0'))
        {
            return Err("Invalid binding description".into());
        }
        if let Some(d) = &self.device {
            if d.items.len() > 128
                || d.items
                    .iter()
                    .any(|s| s.is_empty() || s.len() > 4096 || s.chars().any(char::is_control))
            {
                return Err("Invalid device/tag targeting list".into());
            }
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut opts = self
            .flags
            .iter()
            .map(|f| format!("[ {} ] = true", quote_lua(f)))
            .collect::<Vec<_>>();
        if let Some(d) = &self.description {
            opts.push(format!(
                "{} = {}",
                if self.description_alias {
                    "desc"
                } else {
                    "description"
                },
                quote_lua(d)
            ));
        }
        if let Some(d) = &self.device {
            opts.push(format!(
                "device = {{ inclusive = {}, list = {{ {} }} }}",
                d.inclusive,
                d.items
                    .iter()
                    .map(|s| quote_lua(s))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if self.actions.len() == 1 && self.actions[0].at_load {
            return Ok(format!(
                "hl.bind({}, {}, {{ {} }})\n",
                quote_lua(&self.chord.text()),
                self.actions[0].export_lua(version)?,
                opts.join(", ")
            ));
        }
        let mut before = String::new();
        let mut body = vec![];
        for (i, a) in self.actions.iter().enumerate() {
            let constructor = a.export_lua(version)?;
            if a.at_load {
                before.push_str(&format!("local _hye_action_{i} = {constructor}\n"));
                body.push(format!("hl.dispatch(_hye_action_{i})"));
            } else {
                body.push(format!("hl.dispatch({constructor})"));
            }
        }
        let bind = format!(
            "hl.bind({}, function() {} end, {{ {} }})\n",
            quote_lua(&self.chord.text()),
            body.join("; "),
            opts.join(", ")
        );
        if before.is_empty() {
            Ok(bind)
        } else {
            Ok(format!("do\n{before}{bind}end\n"))
        }
    }
}
impl Draft {
    pub fn is_empty(&self) -> bool {
        self.binds.is_empty()
            && self.submaps.is_empty()
            && self.order.is_empty()
            && self
                .legacy
                .as_ref()
                .is_none_or(crate::legacy_binding::Draft::is_empty)
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if let Some(legacy) = &self.legacy {
            legacy.validate(version)?;
        }
        if self.binds.len() > 10000 || self.submaps.len() > 128 {
            return Err("Binding draft exceeds 10,000 binds / 128 submaps".into());
        }
        let mut ids = BTreeSet::new();
        for m in &self.submaps {
            if !identity(&m.id)
                || m.name.is_empty()
                || m.name.len() > 4096
                || m.name.chars().any(char::is_control)
                || m.reset
                    .as_ref()
                    .is_some_and(|r| r.len() > 4096 || r.chars().any(char::is_control))
                || !ids.insert(&m.id)
            {
                return Err("Invalid or duplicate submap identity/name".into());
            }
        }
        let maps: BTreeMap<_, _> = self.submaps.iter().map(|m| (&m.id, m)).collect();
        for m in &self.submaps {
            let mut seen = BTreeSet::new();
            let mut next = Some(&m.id);
            while let Some(id) = next {
                if !seen.insert(id) {
                    return Err("Submap parent cycle".into());
                }
                let map = maps.get(id).ok_or("Missing submap parent")?;
                next = map.parent.as_ref();
            }
        }
        for b in &self.binds {
            b.validate(version)?;
            if !ids.insert(&b.id) {
                return Err("Duplicate binding identity".into());
            }
            if b.submap.as_ref().is_some_and(|id| !maps.contains_key(id)) {
                return Err("Binding refers to a missing submap".into());
            }
        }
        if self.order.len() != ids.len()
            || self.order.iter().collect::<BTreeSet<_>>().len() != ids.len()
            || self.order.iter().any(|id| !ids.contains(id))
        {
            return Err("Declaration order must contain every bind/submap exactly once".into());
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_BYTES {
            return Err("Binding draft exceeds 4 MiB".into());
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        if self.legacy.as_ref().is_some_and(|draft| !draft.is_empty()) {
            return Err("Legacy bindings require legacy source review; implicit Lua conversion is unavailable".into());
        }
        // Build per-scope ordered indexes once; avoid rescanning 10,000 bindings
        // for every declaration and nested scope.
        enum Node<'a> {
            Bind(&'a Bind),
            Map(&'a Submap),
        }
        let binds: BTreeMap<_, _> = self.binds.iter().map(|b| (b.id.as_str(), b)).collect();
        let maps: BTreeMap<_, _> = self.submaps.iter().map(|m| (m.id.as_str(), m)).collect();
        let mut scopes: BTreeMap<Option<&str>, Vec<Node<'_>>> = BTreeMap::new();
        for id in &self.order {
            if let Some(b) = binds.get(id.as_str()) {
                scopes
                    .entry(b.submap.as_deref())
                    .or_default()
                    .push(Node::Bind(b));
            }
            if let Some(m) = maps.get(id.as_str()) {
                scopes
                    .entry(m.parent.as_deref())
                    .or_default()
                    .push(Node::Map(m));
            }
        }
        fn scope(
            scopes: &BTreeMap<Option<&str>, Vec<Node<'_>>>,
            parent: Option<&str>,
            version: &str,
        ) -> Result<String, String> {
            let mut text = String::new();
            if let Some(nodes) = scopes.get(&parent) {
                for node in nodes {
                    match node {
                        Node::Bind(b) => text.push_str(&b.export_lua(version)?),
                        Node::Map(m) => {
                            let reset = m
                                .reset
                                .as_ref()
                                .map(|r| format!("{}, ", quote_lua(r)))
                                .unwrap_or_default();
                            text.push_str(&format!(
                                "hl.define_submap({}, {reset}function()\n{}end)\n",
                                quote_lua(&m.name),
                                scope(scopes, Some(&m.id), version)?
                            ));
                        }
                    }
                }
            }
            Ok(text)
        }
        scope(&scopes, None, version)
    }
}

impl Draft {
    /// Compare emitted declarations, excluding private IDs. Invalid projections
    /// are never considered equal when deciding whether a scalar copy omits edits.
    pub fn same_declarations(&self, other: &Self, version: &str) -> bool {
        match (self.export_lua(version), other.export_lua(version)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }
    /// Rename exact literal references only. Duplicate source names are ambiguous,
    /// so refuse before mutation instead of guessing which declarations they target.
    pub fn rename_submap(&mut self, id: &str, new_name: &str) -> Result<usize, String> {
        let old = self
            .submaps
            .iter()
            .find(|m| m.id == id)
            .ok_or("Missing submap")?
            .name
            .clone();
        if old == new_name {
            return Ok(0);
        }
        if new_name.is_empty()
            || new_name == "reset"
            || new_name.len() > 4096
            || new_name.chars().any(char::is_control)
        {
            return Err("Choose a nonempty, non-reserved submap name".into());
        }
        if self.submaps.iter().filter(|m| m.name == old).count() != 1
            || self.submaps.iter().any(|m| m.name == new_name)
        {
            return Err(
                "Rename is ambiguous because the source or destination name is shared".into(),
            );
        }
        let mut references = 0;
        for m in &mut self.submaps {
            if m.id == id {
                m.name = new_name.into();
            }
            if m.reset.as_deref() == Some(&old) {
                m.reset = Some(new_name.into());
                references += 1;
            }
        }
        for b in &mut self.binds {
            for a in &mut b.actions {
                if a.constructor == "submap"
                    && a.fields.get("argument") == Some(&crate::Value::String(old.clone()))
                {
                    a.fields
                        .insert("argument".into(), crate::Value::String(new_name.into()));
                    references += 1;
                }
            }
        }
        Ok(references)
    }
    /// Duplicate a declaration subtree with new IDs/names and rebase only
    /// its literal internal transitions. Validate the whole result before publish.
    pub fn duplicate_submap(&mut self, id: &str, version: &str) -> Result<String, String> {
        self.validate(version)?;
        if !self.submaps.iter().any(|m| m.id == id) {
            return Err("Missing submap".into());
        }
        let mut members = BTreeSet::from([id.to_string()]);
        loop {
            let before = members.len();
            for m in &self.submaps {
                if m.parent.as_ref().is_some_and(|p| members.contains(p)) {
                    members.insert(m.id.clone());
                }
            }
            if members.len() == before {
                break;
            }
        }
        let mut ids = self.order.iter().cloned().collect::<BTreeSet<_>>();
        let mut names = self
            .submaps
            .iter()
            .map(|m| m.name.clone())
            .collect::<BTreeSet<_>>();
        let mut mapped = BTreeMap::new();
        let mut renamed = BTreeMap::new();
        fn fresh(used: &mut BTreeSet<String>, stem: &str) -> String {
            for n in 0.. {
                let value = format!("{stem}-{n}");
                if used.insert(value.clone()) {
                    return value;
                }
            }
            unreachable!()
        }
        for m in self.submaps.iter().filter(|m| members.contains(&m.id)) {
            if self
                .submaps
                .iter()
                .filter(|other| other.name == m.name)
                .count()
                != 1
            {
                return Err("Shared submap names make graph duplication ambiguous".into());
            }
            mapped.insert(m.id.clone(), fresh(&mut ids, "map-copy"));
            renamed.insert(
                m.name.clone(),
                fresh(&mut names, &format!("{}-copy", m.name)),
            );
        }
        let result = mapped[id].clone();
        let mut next = self.clone();
        let maps: BTreeMap<_, _> = self.submaps.iter().map(|m| (&m.id, m)).collect();
        let binds: BTreeMap<_, _> = self.binds.iter().map(|b| (&b.id, b)).collect();
        for declaration in &self.order {
            if let Some(m) = maps.get(declaration).filter(|m| members.contains(&m.id)) {
                let mut m = (*m).clone();
                m.id = mapped[&m.id].clone();
                m.name = renamed[&m.name].clone();
                m.parent = m.parent.map(|p| mapped.get(&p).cloned().unwrap_or(p));
                m.reset = m.reset.map(|r| renamed.get(&r).cloned().unwrap_or(r));
                next.order.push(m.id.clone());
                next.submaps.push(m);
            }
            if let Some(b) = binds
                .get(declaration)
                .filter(|b| b.submap.as_ref().is_some_and(|p| members.contains(p)))
            {
                let mut b = (*b).clone();
                b.id = fresh(&mut ids, "bind-copy");
                b.submap = b.submap.map(|p| mapped[&p].clone());
                for a in &mut b.actions {
                    if a.constructor == "submap" {
                        if let Some(crate::Value::String(r)) = a.fields.get_mut("argument") {
                            if let Some(new) = renamed.get(r) {
                                *r = new.clone();
                            }
                        }
                    }
                }
                next.order.push(b.id.clone());
                next.binds.push(b);
            }
        }
        next.validate(version)?;
        *self = next;
        Ok(result)
    }
    /// Findings describe literal draft relationships, never effective runtime
    /// collisions or a guarantee that an escape shortcut will be delivered.
    pub fn findings(&self) -> Vec<String> {
        let maps: BTreeMap<_, _> = self.submaps.iter().map(|m| (m.id.as_str(), m)).collect();
        let names: BTreeSet<_> = self.submaps.iter().map(|m| m.name.as_str()).collect();
        let mut warnings = BTreeSet::new();
        let mut counts = BTreeMap::new();
        for m in &self.submaps {
            *counts.entry(&m.name).or_insert(0) += 1;
        }
        for (name, count) in counts {
            if count > 1 {
                warnings.insert(format!("Submap name {name:?} is shared by {count} declarations; their runtime scope is shared."));
            }
        }
        let mut signatures = BTreeMap::new();
        let mut routes: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for b in &self.binds {
            let scope = b
                .submap
                .as_deref()
                .and_then(|id| maps.get(id))
                .map_or("", |m| m.name.as_str());
            // Exact spelled chord/options only: conservative, no hardware inference.
            let signature =
                serde_json::to_string(&(scope, &b.chord, &b.flags, &b.device)).unwrap_or_default();
            let count = signatures
                .entry(signature)
                .or_insert((0, scope, b.chord.text()));
            count.0 += 1;
            for a in &b.actions {
                if a.constructor == "submap" {
                    if let Some(crate::Value::String(target)) = a.fields.get("argument") {
                        routes.entry(scope).or_default().insert(target.as_str());
                        if target != "reset" && !names.contains(target.as_str()) {
                            warnings.insert(format!("Binding {} targets undeclared submap {target:?}; opaque or external definitions may exist.",b.id));
                        }
                    }
                }
            }
        }
        for (_, (count, scope, chord)) in signatures {
            if count > 1 {
                warnings.insert(format!("{count} bindings share {chord:?} and identical options in scope {scope:?}; review their actions/order."));
            }
        }
        for m in &self.submaps {
            if let Some(target) = m.reset.as_deref().filter(|s| !s.is_empty()) {
                routes.entry(&m.name).or_default().insert(target);
                if target != "reset" && !names.contains(target) {
                    warnings.insert(format!(
                        "Submap {:?} resets to undeclared {target:?}.",
                        m.name
                    ));
                }
            }
        }
        for name in names {
            let mut seen = BTreeSet::new();
            let mut stack = vec![name];
            let mut escape = false;
            while let Some(current) = stack.pop() {
                if current == "reset" {
                    escape = true;
                    break;
                }
                if seen.insert(current) {
                    if let Some(next) = routes.get(current) {
                        stack.extend(next.iter().copied());
                    }
                }
            }
            if !escape {
                warnings.insert(format!("Submap {name:?} has no visible literal transition path to reset; add an escape binding or review opaque behavior."));
            }
        }
        warnings.into_iter().take(256).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{action, catalogue, lua};
    fn bind() -> Bind {
        let def = action::catalogue(BASELINE)
            .into_iter()
            .find(|d| d.name == "no_op")
            .unwrap();
        Bind {
            id: "b".into(),
            chord: Chord::parse("SUPER + Q"),
            actions: vec![Action::new(&def)],
            flags: Default::default(),
            description: None,
            description_alias: false,
            device: None,
            submap: None,
        }
    }
    #[test]
    fn chord_rules_preserve_version_differences_and_external_events() {
        for version in [BASELINE, AUDITED_MAIN] {
            for text in [
                "SUPER + Q",
                "CTRL + SHIFT + Return",
                "SUPER + A + B",
                "ALT + mouse:272",
                "code:30",
                "switch:on:lid",
                "mouse_down",
            ] {
                assert!(
                    Chord::parse(text).validate(version, false).is_ok(),
                    "{version} {text}"
                );
            }
            for text in [
                "Q + SUPER",
                "mouse_down + Q",
                "Enter",
                "SUPER",
                "unknown_hye_keysym",
                "code:4294967296",
            ] {
                assert!(
                    Chord::parse(text).validate(version, false).is_err(),
                    "{version} {text}"
                );
            }
        }
        assert!(Chord::parse("ctrl + Q").validate(BASELINE, false).is_err());
        assert!(Chord::parse("ctrl + Q")
            .validate(AUDITED_MAIN, false)
            .is_ok());
        assert!(Chord::parse("SUPER + q + Q")
            .validate(AUDITED_MAIN, false)
            .is_err());
        assert!(Chord::parse("SHIFT_L + mouse_down")
            .validate(AUDITED_MAIN, false)
            .is_ok());
        assert!(Chord::parse("SHIFT_L")
            .validate(AUDITED_MAIN, false)
            .is_ok());
        assert!(Chord::parse("catchall").validate(BASELINE, false).is_err());
        assert!(Chord::parse("catchall").validate(BASELINE, true).is_ok());
    }
    #[test]
    fn incompatible_flags_and_current_only_mouse_are_rejected() {
        let mut b = bind();
        b.flags.extend(["click".into(), "drag".into()]);
        assert!(b.validate(BASELINE).is_err());
        b.flags = BTreeSet::from(["repeating".into(), "click".into()]);
        assert!(b.validate(BASELINE).is_err());
        b.flags = BTreeSet::from(["mouse".into()]);
        assert!(b.validate(BASELINE).is_err());
        assert!(b.validate(AUDITED_MAIN).is_ok());
        b.flags.insert("locked".into());
        assert!(b.validate(AUDITED_MAIN).is_err());
    }
    #[test]
    fn static_binding_roundtrip_keeps_ordered_actions_scopes_and_options() {
        let source="hl.bind('SUPER + Q',hl.dsp.no_op(),{description='Global',device={inclusive=false,list={'work'}}})\nhl.define_submap('resize','reset',function() hl.bind('R',function() hl.dispatch(hl.dsp.no_op());hl.dispatch(hl.dsp.submap('reset')) end,{release=true,desc='Resize'}) end)";
        for version in [BASELINE, AUDITED_MAIN] {
            let report = lua::parse(source, version, &catalogue()).unwrap();
            assert!(!report.opaque_effects, "{:?}", report.diagnostics);
            assert_eq!(report.bindings.binds.len(), 2);
            assert_eq!(report.bindings.submaps.len(), 1);
            assert!(report.bindings.binds[0].actions[0].at_load);
            assert_eq!(report.bindings.binds[1].actions.len(), 2);
            assert!(report.bindings.binds[1].description_alias);
            let exported = report.bindings.export_lua(version).unwrap();
            let other = lua::parse(&exported, version, &catalogue()).unwrap();
            assert!(
                !other.opaque_effects,
                "{} {:?}",
                exported, other.diagnostics
            );
            assert_eq!(other.bindings, report.bindings);
        }
    }
    #[test]
    fn captured_constructor_and_deferred_actions_round_trip_without_execution() {
        let mut b = bind();
        b.actions[0].at_load = true;
        b.actions.push(Action::new(
            &action::catalogue(BASELINE)
                .into_iter()
                .find(|d| d.name == "exit")
                .unwrap(),
        ));
        let draft = Draft {
            legacy: None,
            binds: vec![b],
            submaps: vec![],
            order: vec!["b".into()],
        };
        let source = draft.export_lua(BASELINE).unwrap();
        let report = lua::parse(&source, BASELINE, &catalogue()).unwrap();
        assert!(
            !report.opaque_effects,
            "{} {:?}",
            source, report.diagnostics
        );
        assert_eq!(report.bindings.binds.len(), 1);
        assert!(report.bindings.binds[0].actions[0].at_load);
        assert!(!report.bindings.binds[0].actions[1].at_load);
    }
    #[test]
    fn unknown_callbacks_stay_opaque_and_do_not_create_guessed_bindings() {
        let r = lua::parse(
            "hl.bind('Q',function() os.execute('never execute') end)",
            BASELINE,
            &catalogue(),
        )
        .unwrap();
        assert!(r.opaque_effects);
        assert!(r.bindings.binds.is_empty());
        let r = lua::parse(
            "hl.dsp.exit();hl.bind('Q',hl.dsp.no_op())",
            BASELINE,
            &catalogue(),
        )
        .unwrap();
        assert!(r.opaque_effects);
        assert!(r.bindings.binds.is_empty());
    }
    #[test]
    fn rename_propagates_literal_references_and_inverse_restores_changed_records() {
        let source="hl.bind('R',hl.dsp.submap('resize'));hl.define_submap('resize','reset',function() hl.bind('Escape',hl.dsp.submap('reset')) end);hl.define_submap('other','resize',function() end)";
        let before = lua::parse(source, BASELINE, &catalogue()).unwrap().bindings;
        let mut after = before.clone();
        let id = after.submaps[0].id.clone();
        assert_eq!(after.rename_submap(&id, "sizing").unwrap(), 2);
        assert_eq!(after.submaps[1].reset.as_deref(), Some("sizing"));
        assert_eq!(
            after.binds[0].actions[0].fields["argument"],
            crate::Value::String("sizing".into())
        );
        let inverse = Inverse::between(&before, &after);
        assert_eq!(inverse.binds.len(), 1);
        assert_eq!(inverse.submaps.len(), 2);
        assert!(inverse.order.is_none());
        assert!(inverse.bind_ids.is_none());
        inverse.restore(&mut after);
        assert_eq!(after, before);
        let untouched = after.clone();
        assert!(after.rename_submap(&id, "other").is_err());
        assert_eq!(after, untouched);
        after.binds.pop();
        after.submaps.reverse();
        after.order.retain(|id| {
            after.binds.iter().any(|b| b.id == *id) || after.submaps.iter().any(|m| m.id == *id)
        });
        after.order.reverse();
        let inverse = Inverse::between(&before, &after);
        inverse.restore(&mut after);
        assert_eq!(after, before);
        let mut added = before.clone();
        let mut new = bind();
        new.id = "new".into();
        added.order.push(new.id.clone());
        added.binds.push(new);
        Inverse::between(&before, &added).restore(&mut added);
        assert_eq!(added, before);
    }
    #[test]
    fn static_findings_distinguish_literal_routes_shared_names_and_duplicate_declarations() {
        let source="hl.bind('Q',hl.dsp.submap('missing'));hl.bind('Q',hl.dsp.no_op());hl.define_submap('resize',function() hl.bind('R',hl.dsp.no_op()) end)";
        let mut draft = lua::parse(source, BASELINE, &catalogue()).unwrap().bindings;
        let warnings = draft.findings();
        assert!(warnings.iter().any(|w| w.contains("undeclared")));
        assert!(warnings.iter().any(|w| w.contains("identical options")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("no visible literal transition")));
        draft.binds[2].actions = vec![Action::new(action::find("submap", BASELINE).unwrap())];
        draft.binds[2].actions[0]
            .fields
            .insert("argument".into(), crate::Value::String("reset".into()));
        assert!(!draft
            .findings()
            .iter()
            .any(|w| w.contains("no visible literal transition")));
        let mut other = draft.submaps[0].clone();
        other.id = "shared".into();
        draft.order.push(other.id.clone());
        draft.submaps.push(other);
        assert!(draft
            .findings()
            .iter()
            .any(|w| w.contains("runtime scope is shared")));
        let before = draft.clone();
        let id = draft.submaps[0].id.clone();
        assert!(draft.rename_submap(&id, "new").is_err());
        assert_eq!(draft, before);
    }
    #[test]
    fn unknown_evaluated_bind_arguments_invalidate_later_load_time_api_trust() {
        for call in [
            "hl.bind(untrusted(),hl.dsp.no_op())",
            "hl.bind('Q',hl.dsp.unknown())",
            "hl.bind('Q',hl.dsp.no_op(),{unknown=untrusted()})",
        ] {
            let source = format!(
                "{call};hl.bind('W',hl.dsp.no_op());hl.config{{general={{border_size=7}}}}"
            );
            let report = lua::parse(&source, BASELINE, &catalogue()).unwrap();
            assert!(report.opaque_effects, "{call}");
            assert!(report.bindings.binds.is_empty(), "{call}");
            assert!(report.draft.values.is_empty(), "{call}");
        }
        // Merely defining an opaque bind callback does not execute its body.
        let report = lua::parse(
            "hl.bind('Q',function() untrusted() end);hl.bind('W',hl.dsp.no_op())",
            BASELINE,
            &catalogue(),
        )
        .unwrap();
        assert!(report.opaque_effects);
        assert_eq!(report.bindings.binds.len(), 1);
    }
    #[test]
    fn nested_and_empty_submap_scopes_preserve_captured_actions_and_restored_locals() {
        let source="local key='Q';local action=hl.dsp.no_op();hl.define_submap('outer',function() local key='R';hl.bind(key,action);hl.define_submap('inner',function() end) end);hl.bind(key,action);do end";
        let report = lua::parse(source, BASELINE, &catalogue()).unwrap();
        assert!(!report.opaque_effects, "{:?}", report.diagnostics);
        assert_eq!(report.bindings.submaps.len(), 2);
        assert_eq!(report.bindings.binds[0].chord.text(), "R");
        assert_eq!(report.bindings.binds[1].chord.text(), "Q");
        let export = report.bindings.export_lua(BASELINE).unwrap();
        let other = lua::parse(&export, BASELINE, &catalogue()).unwrap();
        assert_eq!(other.bindings, report.bindings);
    }
    #[test]
    fn large_ordered_exports_and_quotas_are_bounded_without_quadratic_lookup() {
        let mut draft = Draft::default();
        for n in 0..10000 {
            let mut b = bind();
            b.id = format!("b-{n}");
            draft.order.push(b.id.clone());
            draft.binds.push(b);
        }
        draft.validate(BASELINE).unwrap();
        let output = draft.export_lua(BASELINE).unwrap();
        assert_eq!(output.matches("hl.bind(").count(), 10000);
        let mut b = bind();
        b.id = "overflow".into();
        draft.order.push(b.id.clone());
        draft.binds.push(b);
        assert!(draft.validate(BASELINE).is_err());
        let mut draft = Draft::default();
        draft.submaps.push(Submap {
            id: "m".into(),
            name: "map".into(),
            parent: None,
            reset: Some("bad\0name".into()),
        });
        draft.order.push("m".into());
        assert!(draft.validate(BASELINE).is_err());
    }
    #[test]
    fn duplicate_submap_rebases_subtree_transitions_but_keeps_external_targets() {
        let source="hl.define_submap('outer','reset',function()hl.bind('I',hl.dsp.submap('inner'));hl.define_submap('inner','outer',function()hl.bind('Escape',hl.dsp.submap('reset'))end)end)";
        let before = lua::parse(source, BASELINE, &catalogue()).unwrap().bindings;
        let mut draft = before.clone();
        let id = draft.submaps[0].id.clone();
        let root = draft.duplicate_submap(&id, BASELINE).unwrap();
        assert_eq!(draft.submaps.len(), 4);
        assert_eq!(draft.binds.len(), 4);
        assert_eq!(draft.submaps[2].id, root);
        assert_eq!(draft.submaps[3].parent.as_deref(), Some(root.as_str()));
        assert_eq!(
            draft.submaps[3].reset.as_deref(),
            Some(draft.submaps[2].name.as_str())
        );
        assert_eq!(
            draft.binds[2].actions[0].fields["argument"],
            crate::Value::String(draft.submaps[3].name.clone())
        );
        assert_eq!(
            draft.binds[3].actions[0].fields["argument"],
            crate::Value::String("reset".into())
        );
        assert_eq!(draft.binds[..2], before.binds);
        draft.validate(BASELINE).unwrap();
        Inverse::between(&before, &draft).restore(&mut draft);
        assert_eq!(draft, before);
    }
    #[test]
    fn metadata_flags_and_editor_limits_match_the_model() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/bindings.json")).unwrap();
        for pack in schema["packs"].as_array().unwrap() {
            let version = pack["version"].as_str().unwrap();
            let flags = pack["flags"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| f.as_str().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                flags,
                FLAGS
                    .iter()
                    .copied()
                    .filter(|f| *f != "mouse" || version == AUDITED_MAIN)
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(schema["editor_limits"]["serialized_bytes"], MAX_BYTES);
    }
    #[test]
    fn empty_bind_callbacks_are_visible_and_zero_keycode_is_versioned() {
        let report = lua::parse("hl.bind('Q',function() end)", BASELINE, &catalogue()).unwrap();
        assert!(!report.opaque_effects);
        assert_eq!(report.bindings.binds.len(), 1);
        assert!(report.bindings.binds[0].actions.is_empty());
        let output = report.bindings.export_lua(BASELINE).unwrap();
        assert_eq!(
            lua::parse(&output, BASELINE, &catalogue())
                .unwrap()
                .bindings,
            report.bindings
        );
        assert!(Chord::parse("code:0").validate(BASELINE, false).is_ok());
        assert!(Chord::parse("code:0")
            .validate(AUDITED_MAIN, false)
            .is_err());
    }
    #[test]
    fn source_copy_equivalence_ignores_private_ids_but_never_accepts_invalid_graphs() {
        let a = Draft {
            legacy: None,
            binds: vec![bind()],
            submaps: vec![],
            order: vec!["b".into()],
        };
        let mut b = a.clone();
        b.binds[0].id = "renamed-id".into();
        b.order = vec!["renamed-id".into()];
        assert!(a.same_declarations(&b, BASELINE));
        b.binds[0].description = Some("changed".into());
        assert!(!a.same_declarations(&b, BASELINE));
        b.order.clear();
        assert!(!b.same_declarations(&b, BASELINE));
    }
    #[test]
    fn graph_order_missing_parents_and_cycles_are_validated() {
        let mut d = Draft {
            legacy: None,
            binds: vec![bind()],
            submaps: vec![],
            order: vec!["b".into()],
        };
        assert!(d.validate(BASELINE).is_ok());
        d.order.clear();
        assert!(d.validate(BASELINE).is_err());
        d.order = vec!["b".into()];
        d.binds[0].submap = Some("missing".into());
        assert!(d.validate(BASELINE).is_err());
        d.binds.clear();
        d.submaps = vec![Submap {
            id: "a".into(),
            name: "map".into(),
            parent: Some("a".into()),
            reset: None,
        }];
        d.order = vec!["a".into()];
        assert!(d.validate(BASELINE).is_err());
    }
}

/// An inverse retains changed records; a normal field edit does not duplicate the graph.
#[derive(Clone, Debug)]
pub struct Inverse {
    pub legacy: Option<Option<crate::legacy_binding::Draft>>,
    pub binds: Vec<(String, Option<Bind>)>,
    pub submaps: Vec<(String, Option<Submap>)>,
    pub bind_ids: Option<Vec<String>>,
    pub submap_ids: Option<Vec<String>>,
    pub order: Option<Vec<String>>,
}
impl Inverse {
    pub fn between(before: &Draft, after: &Draft) -> Self {
        let a: BTreeMap<_, _> = before.binds.iter().map(|b| (&b.id, b)).collect();
        let b: BTreeMap<_, _> = after.binds.iter().map(|b| (&b.id, b)).collect();
        let bind_keys: BTreeSet<_> = a.keys().chain(b.keys()).copied().collect();
        let binds = bind_keys
            .into_iter()
            .filter(|id| a.get(id) != b.get(id))
            .map(|id| (id.clone(), a.get(id).map(|b| (*b).clone())))
            .collect();
        let a: BTreeMap<_, _> = before.submaps.iter().map(|m| (&m.id, m)).collect();
        let b: BTreeMap<_, _> = after.submaps.iter().map(|m| (&m.id, m)).collect();
        let keys: BTreeSet<_> = a.keys().chain(b.keys()).copied().collect();
        let submaps = keys
            .into_iter()
            .filter(|id| a.get(id) != b.get(id))
            .map(|id| (id.clone(), a.get(id).map(|m| (*m).clone())))
            .collect();
        let before_bind = before
            .binds
            .iter()
            .map(|b| b.id.clone())
            .collect::<Vec<_>>();
        let after_bind = after.binds.iter().map(|b| b.id.clone()).collect::<Vec<_>>();
        let before_map = before
            .submaps
            .iter()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>();
        let after_map = after
            .submaps
            .iter()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>();
        Self {
            legacy: (before.legacy != after.legacy).then(|| before.legacy.clone()),
            binds,
            submaps,
            bind_ids: (before_bind != after_bind).then_some(before_bind),
            submap_ids: (before_map != after_map).then_some(before_map),
            order: (before.order != after.order).then(|| before.order.clone()),
        }
    }
    pub fn restore(self, draft: &mut Draft) {
        if let Some(legacy) = self.legacy {
            draft.legacy = legacy;
        }
        let bind_ids = self
            .bind_ids
            .unwrap_or_else(|| draft.binds.iter().map(|b| b.id.clone()).collect());
        let map_ids = self
            .submap_ids
            .unwrap_or_else(|| draft.submaps.iter().map(|m| m.id.clone()).collect());
        let mut binds = draft
            .binds
            .drain(..)
            .map(|b| (b.id.clone(), b))
            .collect::<BTreeMap<_, _>>();
        let mut maps = draft
            .submaps
            .drain(..)
            .map(|m| (m.id.clone(), m))
            .collect::<BTreeMap<_, _>>();
        for (id, b) in self.binds {
            if let Some(b) = b {
                binds.insert(id, b);
            } else {
                binds.remove(&id);
            }
        }
        for (id, m) in self.submaps {
            if let Some(m) = m {
                maps.insert(id, m);
            } else {
                maps.remove(&id);
            }
        }
        draft.binds = bind_ids
            .into_iter()
            .filter_map(|id| binds.remove(&id))
            .collect();
        draft.submaps = map_ids
            .into_iter()
            .filter_map(|id| maps.remove(&id))
            .collect();
        if let Some(order) = self.order {
            draft.order = order;
        }
    }
}
