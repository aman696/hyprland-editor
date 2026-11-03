//! Native rule declarations preserve strings, block keys and inline effect ordering.
use crate::{
    legacy_blocks::{self, Block},
    legacy_session::Site,
    legacy_syntax::{self, literal, trim, unescape},
    lua::source_hash,
    rule::{self, Effect, EffectValue, Kind, Predicate, Rule},
    Value, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Native {
    pub block: bool,
    pub enable: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_sha256: String,
    pub source_sha256: String,
    pub rules: Vec<Rule>,
    pub sites: Vec<Site>,
    pub unresolved: Vec<usize>,
    pub unresolved_count: usize,
}
pub fn properties() -> &'static [&'static str] {
    &[
        "class",
        "content",
        "float",
        "focus",
        "fullscreen",
        "fullscreen_state_client",
        "fullscreen_state_internal",
        "group",
        "initial_class",
        "initial_title",
        "modal",
        "namespace",
        "pin",
        "tag",
        "title",
        "workspace",
        "xdg_tag",
        "xwayland",
    ]
}
fn category(kind: Kind) -> &'static str {
    match kind {
        Kind::Window => "windowrule",
        Kind::Layer => "layerrule",
    }
}
fn text(v: &Value) -> Result<&str, String> {
    match v {
        Value::String(s) if s.len() <= 16384 && !s.contains('\0') => Ok(s),
        _ => Err("Native rule values require literal text within 16 KiB".into()),
    }
}
pub fn validate(r: &Rule, version: &str) -> Result<(), String> {
    let n = r.legacy.as_ref().ok_or("Native rule provenance missing")?;
    if version != BASELINE || r.enabled.is_some() {
        return Err("Native rule does not use Lua enabled semantics".into());
    }
    if n.block {
        if r.name.as_ref().is_none_or(|n| n.is_empty()) {
            return Err("Named native blocks need a nonempty name".into());
        }
    } else if r.name.is_some() || n.enable.is_some() {
        return Err("Inline native rules have no name/enable field".into());
    }
    let mut predicate_keys = BTreeSet::new();
    for p in &r.predicates {
        if !predicate_keys.insert(&p.property) {
            return Err("Duplicate native predicate must be represented by its last value".into());
        }
        if !properties().contains(&p.property.as_str()) {
            return Err("Unknown native match property".into());
        }
        text(&p.value)?;
    }
    let effects = rule::catalogue(r.kind, BASELINE);
    let mut keys = BTreeSet::new();
    for e in &r.effects {
        if !effects.iter().any(|f| f.lua_key == e.key) {
            return Err("Unknown native rule effect".into());
        }
        let EffectValue::Literal(v) = &e.value else {
            return Err("Native rule effects retain their text representation".into());
        };
        text(v)?;
        if n.block && !keys.insert(&e.key) {
            return Err("A block field has one final value; duplicate inline effects need separate declarations".into());
        }
    }
    Ok(())
}
fn add(r: &mut Rule, key: &str, raw: String, inline: bool) -> Result<(), String> {
    if let Some(p) = key.strip_prefix("match:") {
        if !properties().contains(&p) {
            return Err("Unknown native predicate".into());
        }
        if let Some(old) = r.predicates.iter_mut().find(|v| v.property == p) {
            old.value = Value::String(raw);
        } else {
            r.predicates.push(Predicate {
                property: p.into(),
                value: Value::String(raw),
            });
        }
    } else {
        if !rule::catalogue(r.kind, BASELINE)
            .iter()
            .any(|s| s.lua_key == key)
        {
            return Err("Unknown native effect".into());
        }
        let value = EffectValue::Literal(Value::String(raw));
        if !inline {
            if let Some(e) = r.effects.iter_mut().find(|v| v.key == key) {
                e.value = value;
                return Ok(());
            }
        }
        r.effects.push(Effect {
            key: key.into(),
            value,
        });
    }
    Ok(())
}
fn integer(raw: &str) -> Result<i64, String> {
    match raw {
        "true" | "yes" | "on" => Ok(1),
        "false" | "no" | "off" => Ok(0),
        _ => raw
            .parse()
            .map_err(|_| "Unresolved native enable integer".into()),
    }
}
fn block(b: &Block, kind: Kind) -> Result<Rule, String> {
    if !b.editable || b.fields.first().is_none_or(|f| f.key != "name") {
        return Err("Native rule block requires a first literal name and flat fields".into());
    }
    let mut r = Rule::new("native", kind);
    r.legacy = Some(Native {
        block: true,
        enable: None,
    });
    for f in &b.fields {
        let raw = unescape(&f.value)?;
        match f.key.as_str() {
            "name" => {
                if r.name.is_some() {
                    return Err("Duplicate native name field".into());
                }
                r.name = Some(raw);
            }
            "enable" => r.legacy.as_mut().unwrap().enable = Some(integer(&raw)?),
            key => add(&mut r, key, raw, false)?,
        }
    }
    r.validate(BASELINE)?;
    Ok(r)
}
fn inline(value: &str, kind: Kind) -> Result<Rule, String> {
    let raw = unescape(value)?;
    let mut r = Rule::new("native", kind);
    r.legacy = Some(Native {
        block: false,
        enable: None,
    });
    for part in raw.split(',') {
        let part = trim(part);
        let (key, value) = part
            .split_once(' ')
            .ok_or("Native inline field needs a value")?;
        // CVarList trims an element, but does not trim text after its first separating space.
        add(&mut r, key, value.into(), true)?;
    }
    r.validate(BASELINE)?;
    Ok(r)
}
pub fn inspect(content: &str) -> Result<Report, String> {
    let mut accepted = vec![];
    let mut unresolved = vec![];
    let mut unresolved_count = 0;
    let mut retained = 2usize;
    let mut receive = |site: Site, result: Result<Rule, String>| {
        if let Ok(r) = result {
            let n = serde_json::to_vec(&r).map_or(MAX_DOCUMENT_BYTES + 1, |s| s.len() + 64);
            if accepted.len() < rule::MAX_RULES && retained.saturating_add(n) <= MAX_DOCUMENT_BYTES
            {
                retained += n;
                accepted.push((site, r));
                return;
            }
        }
        unresolved_count += 1;
        if unresolved.len() < 4096 {
            unresolved.push(site.line);
        }
    };
    for kind in [Kind::Window, Kind::Layer] {
        legacy_blocks::scan(content, category(kind), |b| {
            let result = block(&b, kind);
            receive(b.site, result);
        })?;
    }
    legacy_syntax::walk(content, |s| {
        let kind = match s.key {
            "windowrule" => Kind::Window,
            "layerrule" => Kind::Layer,
            _ => return,
        };
        let r = if s.editable {
            inline(s.value, kind)
        } else {
            Err("Unsupported native rule context".into())
        };
        receive(
            Site {
                line: s.line,
                statement: s.statement,
            },
            r,
        );
    })?;
    accepted.sort_by_key(|(s, _)| s.statement.start);
    let (sites, rules): (Vec<_>, Vec<_>) = accepted
        .into_iter()
        .enumerate()
        .map(|(i, (s, mut r))| {
            r.id = format!("legacy-rule-{i}");
            (s, r)
        })
        .unzip();
    rule::validate(&rules, BASELINE)?;
    unresolved.sort_unstable();
    Ok(Report {
        schema_sha256: schema_hash(),
        source_sha256: source_hash(content),
        rules,
        sites,
        unresolved,
        unresolved_count,
    })
}
fn fields(r: &Rule) -> Result<BTreeMap<String, String>, String> {
    r.validate(BASELINE)?;
    let mut out = BTreeMap::new();
    if let Some(name) = &r.name {
        out.insert("name".into(), literal(name)?);
    }
    if let Some(enable) = r
        .legacy
        .as_ref()
        .ok_or("Native provenance required")?
        .enable
    {
        out.insert("enable".into(), enable.to_string());
    }
    for p in &r.predicates {
        out.insert(format!("match:{}", p.property), literal(text(&p.value)?)?);
    }
    for e in &r.effects {
        let EffectValue::Literal(v) = &e.value else {
            return Err("Native text required".into());
        };
        out.insert(e.key.clone(), literal(text(v)?)?);
    }
    Ok(out)
}
pub fn encode(r: &Rule) -> Result<String, String> {
    r.validate(BASELINE)?;
    let n = r
        .legacy
        .as_ref()
        .ok_or("Separate Lua rule needs explicit native conversion")?;
    if n.block {
        let mut values = fields(r)?;
        let name = values.remove("name").ok_or("Set native name")?;
        let mut s = format!("{} {{\n    name = {name}\n", category(r.kind));
        for (k, v) in values {
            s.push_str(&format!("    {k} = {v}\n"));
        }
        s.push('}');
        Ok(s)
    } else {
        let mut parts = vec![];
        for p in &r.predicates {
            parts.push((format!("match:{}", p.property), text(&p.value)?));
        }
        for e in &r.effects {
            let EffectValue::Literal(v) = &e.value else {
                return Err("Native text required".into());
            };
            parts.push((e.key.clone(), text(v)?));
        }
        if parts.is_empty() {
            return Err("Set at least one inline rule field".into());
        }
        let mut out = vec![];
        for (k, v) in parts {
            if v.contains(',') || v != v.trim_end_matches(|c: char| c.is_ascii_whitespace()) {
                return Err(
                    "Inline field cannot retain commas or trailing whitespace; use a named block"
                        .into(),
                );
            }
            out.push(format!("{k} {v}"));
        }
        Ok(format!(
            "{} = {}",
            category(r.kind),
            literal(&out.join(", "))?
        ))
    }
}
pub fn export(rules: &[Rule]) -> Result<String, String> {
    rule::validate(rules, BASELINE)?;
    let mut out = String::new();
    for r in rules {
        out.push_str(&encode(r)?);
        out.push('\n');
        if out.len() > MAX_DOCUMENT_BYTES {
            return Err("Native rules exceed 1 MiB".into());
        }
    }
    Ok(out)
}
pub fn edited_copy(content: &str, target: &[Rule]) -> Result<String, String> {
    rule::validate(target, BASELINE)?;
    let before = inspect(content)?;
    if rule::same_declarations(&before.rules, target) {
        return Ok(content.into());
    }
    let mut blocks = BTreeMap::new();
    for kind in [Kind::Window, Kind::Layer] {
        legacy_blocks::scan(content, category(kind), |b| {
            if before
                .sites
                .binary_search_by_key(&b.site.statement.start, |s| s.statement.start)
                .is_ok()
            {
                blocks.insert(b.site.statement.start, b);
            }
        })?;
    }
    let nl = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut edits = vec![];
    for (i, s) in before.sites.iter().enumerate() {
        if let Some(r) = target.get(i) {
            let old = &before.rules[i];
            if old.kind == r.kind
                && old.legacy.as_ref().is_some_and(|n| n.block)
                && r.legacy.as_ref().is_some_and(|n| n.block)
            {
                let b = blocks
                    .get(&s.statement.start)
                    .ok_or("Captured native block missing")?;
                edits.extend(legacy_blocks::patches(
                    content,
                    b,
                    &fields(old)?,
                    &fields(r)?,
                )?);
            } else {
                let mut text = encode(r)?.replace('\n', nl);
                if blocks.contains_key(&s.statement.start) {
                    text.push_str(nl);
                    text.push_str(&legacy_blocks::comments(content, s.statement.clone()));
                }
                edits.push((s.statement.clone(), text));
            }
        } else {
            edits.push((
                s.statement.clone(),
                legacy_blocks::comments(content, s.statement.clone()),
            ));
        }
    }
    edits.sort_by_key(|(s, _)| s.start);
    let mut out = content.to_owned();
    for (s, t) in edits.into_iter().rev() {
        out.replace_range(s, &t);
    }
    for r in target.iter().skip(before.rules.len()) {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(nl);
        }
        out.push_str(&encode(r)?.replace('\n', nl));
        out.push_str(nl);
    }
    if out.len() > MAX_DOCUMENT_BYTES {
        return Err("Native source exceeds 1 MiB".into());
    }
    let after = inspect(&out)?;
    if !rule::same_declarations(&after.rules, target)
        || after.unresolved_count != before.unresolved_count
    {
        return Err("Native rule copy did not reproduce requested declarations".into());
    }
    Ok(out)
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-rules.json"))
}
