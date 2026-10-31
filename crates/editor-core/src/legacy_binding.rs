//! Native legacy binding declarations. No dispatch, key capture or host expansion.
use crate::{
    binding::DeviceFilter,
    legacy_session::Site,
    legacy_syntax::{self, csv, literal, trim, unescape},
    lua::source_hash,
    BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
pub const FLAGS: &[(char, &str)] = &[
    ('l', "Locked"),
    ('r', "On release"),
    ('e', "Repeat"),
    ('m', "Mouse drag"),
    ('n', "Non consuming"),
    ('a', "Auto consuming"),
    ('t', "Transparent"),
    ('i', "Ignore modifiers"),
    ('o', "Long press"),
    ('p', "Ignore inhibitors"),
    ('c', "Click"),
    ('g', "Drag"),
    ('u', "Universal submap"),
    ('x', "Allow input capture"),
];
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chord {
    pub modifiers: Vec<String>,
    pub keys: Vec<String>,
    pub multi: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Entry {
    Bind {
        chord: Chord,
        flags: BTreeSet<char>,
        description: Option<String>,
        device: Option<DeviceFilter>,
        dispatcher: String,
        argument: String,
    },
    Submap {
        name: String,
        reset: Option<String>,
    },
    Unbind {
        all: bool,
        chord: Chord,
    },
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub entries: Vec<Entry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_sha256: String,
    pub source_sha256: String,
    pub draft: Draft,
    pub sites: Vec<Site>,
    pub unresolved: Vec<usize>,
    pub unresolved_count: usize,
    pub barriers_sha256: String,
}
pub fn dispatchers() -> &'static [String] {
    static ALL: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../data/legacy-bindings.json"
        ))
        .unwrap()["dispatchers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().into())
            .collect()
    })
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/legacy-bindings.json"))
}
fn string(value: &str, empty: bool) -> Result<(), String> {
    if (!empty && value.is_empty()) || value.len() > 16384 || value.contains('\0') {
        Err("Use a bounded string without NUL".into())
    } else {
        Ok(())
    }
}
fn normal_modifiers(value: &str) -> Result<Vec<String>, String> {
    let value = value.to_ascii_uppercase();
    let groups = [
        ("SHIFT", vec!["SHIFT"]),
        ("CAPS", vec!["CAPS"]),
        ("CTRL", vec!["CTRL", "CONTROL"]),
        ("ALT", vec!["ALT", "MOD1"]),
        ("MOD2", vec!["MOD2"]),
        ("MOD3", vec!["MOD3"]),
        ("SUPER", vec!["SUPER", "WIN", "LOGO", "MOD4", "META"]),
        ("MOD5", vec!["MOD5"]),
    ];
    let mods: Vec<_> = groups
        .iter()
        .filter(|(_, aliases)| aliases.iter().any(|alias| value.contains(alias)))
        .map(|(name, _)| name.to_string())
        .collect();
    if !value.is_empty() && mods.is_empty() {
        return Err("Unrecognized legacy modifiers".into());
    }
    Ok(mods)
}
impl Chord {
    pub fn validate(&self) -> Result<(), String> {
        if self.keys.is_empty()
            || self.keys.len() > if self.multi { 8 } else { 1 }
            || self.modifiers.len() > 8
        {
            return Err("Choose one key, or up to eight multi-key symbols and modifiers".into());
        }
        for token in self.keys.iter().chain(&self.modifiers) {
            string(token, false)?;
            if token.len() > 256
                || token.chars().any(char::is_control)
                || token.contains([',', '&'])
            {
                return Err("Invalid legacy chord token".into());
            }
        }
        if self.multi {
            for token in self.keys.iter().chain(&self.modifiers) {
                if crate::binding::keysym(token).is_none() {
                    return Err(format!("Unknown multi-key symbol {token}"));
                }
            }
        } else if self
            .modifiers
            .iter()
            .any(|m| !crate::binding::MODIFIERS.contains(&m.as_str()))
        {
            return Err("Choose known modifier masks".into());
        }
        Ok(())
    }
    pub fn text(&self) -> String {
        format!(
            "{}{}{}",
            self.modifiers.join(" + "),
            if self.modifiers.is_empty() { "" } else { " + " },
            self.keys.join(if self.multi { " & " } else { "" })
        )
    }
}
impl Entry {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Bind {
                chord,
                flags,
                description,
                device,
                dispatcher,
                argument,
            } => {
                chord.validate()?;
                string(argument, true)?;
                if !dispatchers().contains(dispatcher) {
                    return Err("Choose an audited legacy dispatcher".into());
                }
                if flags
                    .iter()
                    .any(|f| !FLAGS.iter().any(|(flag, _)| flag == f))
                {
                    return Err("Unknown binding flag".into());
                }
                let release = flags.contains(&'r') || flags.contains(&'c') || flags.contains(&'g');
                if (flags.contains(&'e') && (release || flags.contains(&'o')))
                    || (flags.contains(&'m')
                        && (release || flags.contains(&'e') || flags.contains(&'l')))
                    || (flags.contains(&'c') && flags.contains(&'g'))
                {
                    return Err("These binding flags conflict".into());
                }
                if flags.contains(&'m') && dispatcher != "mouse" {
                    return Err("Mouse drag bindings use the Mouse dispatcher".into());
                }
                if let Some(value) = description {
                    string(value, true)?;
                }
                if let Some(value) = device {
                    if value.items.len() > 128 {
                        return Err("Use at most 128 device filters".into());
                    }
                    if value.inclusive && value.items.is_empty() {
                        return Err(
                            "Add at least one device name or tag to an inclusion filter".into()
                        );
                    }
                    for item in &value.items {
                        string(item, false)?;
                        if item.len() > 4096 || item.chars().any(char::is_whitespace) {
                            return Err("Device filters need one name or tag per item".into());
                        }
                    }
                }
            }
            Self::Submap { name, reset } => {
                string(name, true)?;
                if name == "reset" {
                    return Err("Use Global / reset for the reserved reset name".into());
                }
                if let Some(reset) = reset {
                    string(reset, true)?;
                }
            }
            Self::Unbind { all, chord } => {
                if !all {
                    chord.validate()?;
                    if chord.multi {
                        return Err("Legacy unbind accepts one key".into());
                    }
                }
            }
        }
        Ok(())
    }
    pub fn label(&self) -> String {
        match self {
            Self::Bind {
                chord, dispatcher, ..
            } => format!("{} · {dispatcher}", chord.text()),
            Self::Submap { name, .. } => {
                if name.is_empty() {
                    "Global submap / reset".into()
                } else {
                    format!("Submap · {name}")
                }
            }
            Self::Unbind { all: true, .. } => "Remove all bindings".into(),
            Self::Unbind { chord, .. } => format!("Remove · {}", chord.text()),
        }
    }
}
impl Draft {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if version != BASELINE || self.entries.len() > 10000 {
            return Err(
                "Legacy binding drafts need the baseline pack and at most 10,000 entries".into(),
            );
        }
        let mut scope = "";
        for entry in &self.entries {
            entry.validate()?;
            match entry {
                Entry::Submap { name, .. } => scope = name,
                Entry::Bind { chord, .. }
                    if !chord.multi && chord.keys == ["catchall"] && scope.is_empty() =>
                {
                    return Err("Catchall requires a named submap".into())
                }
                _ => {}
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT_BYTES {
            return Err("Legacy binding draft exceeds 1 MiB".into());
        }
        Ok(())
    }
    pub fn retained_bytes(&self) -> usize {
        use std::mem::size_of;
        self.entries.capacity() * size_of::<Entry>()
            + self
                .entries
                .iter()
                .map(|entry| {
                    let chord = |c: &Chord| {
                        c.modifiers.capacity() * size_of::<String>()
                            + c.keys.capacity() * size_of::<String>()
                            + c.modifiers
                                .iter()
                                .chain(&c.keys)
                                .map(String::capacity)
                                .sum::<usize>()
                    };
                    match entry {
                        Entry::Bind {
                            chord: c,
                            flags,
                            description,
                            device,
                            dispatcher,
                            argument,
                        } => {
                            chord(c)
                                + flags.len() * 128
                                + description.as_ref().map_or(0, String::capacity)
                                + device.as_ref().map_or(0, |d| {
                                    d.items.capacity() * size_of::<String>()
                                        + d.items.iter().map(String::capacity).sum::<usize>()
                                })
                                + dispatcher.capacity()
                                + argument.capacity()
                        }
                        Entry::Submap { name, reset } => {
                            name.capacity() + reset.as_ref().map_or(0, String::capacity)
                        }
                        Entry::Unbind { chord: c, .. } => chord(c),
                    }
                })
                .sum::<usize>()
    }
}
fn parse_chord(mods: &str, key: &str, multi: bool) -> Result<Chord, String> {
    let result = if multi {
        normal_modifiers(mods)?;
        Chord {
            modifiers: mods
                .split('&')
                .map(trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
            keys: key
                .split('&')
                .map(trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
            multi,
        }
    } else {
        let key = if !key.starts_with("code:") && key.parse::<u32>().is_ok_and(|n| n > 9) {
            format!("code:{key}")
        } else {
            key.into()
        };
        Chord {
            modifiers: normal_modifiers(mods)?,
            keys: vec![key],
            multi,
        }
    };
    result.validate()?;
    Ok(result)
}
fn decode(key: &str, value: &str) -> Result<Entry, String> {
    if key == "submap" {
        let parts = csv(value);
        if parts.len() > 2 {
            return Err("Unsupported submap fields".into());
        }
        let name = parts.first().cloned().unwrap_or_default();
        return Ok(Entry::Submap {
            name: if name == "reset" { String::new() } else { name },
            reset: parts.get(1).cloned(),
        });
    }
    if key == "unbind" {
        let parts: Vec<_> = value.split(',').map(trim).collect();
        if parts == ["all"] {
            return Ok(Entry::Unbind {
                all: true,
                chord: Chord::default(),
            });
        }
        if parts.len() != 2 {
            return Err("Unbind needs modifiers and one key".into());
        }
        return Ok(Entry::Unbind {
            all: false,
            chord: parse_chord(parts[0], parts[1], false)?,
        });
    }
    let suffix = key.strip_prefix("bind").ok_or("Unknown directive")?;
    if suffix
        .chars()
        .any(|f| !FLAGS.iter().any(|(flag, _)| *flag == f) && !['d', 'k', 's'].contains(&f))
    {
        return Err("Unknown legacy binding suffix".into());
    }
    let description = suffix.contains('d');
    let device = suffix.contains('k');
    let mouse = suffix.contains('m');
    let count = 4 + usize::from(description) + usize::from(device);
    let parts: Vec<_> = value.splitn(count, ',').map(trim).collect();
    let offset = usize::from(description) + usize::from(device);
    if parts.len() < 3 + offset || (mouse && parts.len() != 3 + offset) {
        return Err("Unsupported binding arguments".into());
    }
    let arg = |at| parts.get(at).copied().unwrap_or("");
    let device = device.then(|| {
        let text = arg(2);
        DeviceFilter {
            inclusive: !text.is_empty() && !text.starts_with('!'),
            items: text
                .strip_prefix('!')
                .unwrap_or(text)
                .split(' ')
                .map(str::to_owned)
                .collect::<Vec<_>>()
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect(),
        }
    });
    let description = description.then(|| arg(2 + usize::from(device.is_some())).to_string());
    let entry = Entry::Bind {
        chord: parse_chord(arg(0), arg(1), suffix.contains('s'))?,
        flags: suffix
            .chars()
            .filter(|f| !['d', 'k', 's'].contains(f))
            .collect(),
        description,
        device,
        dispatcher: if mouse {
            "mouse".into()
        } else {
            arg(2 + offset).to_ascii_lowercase()
        },
        argument: arg(if mouse { 2 + offset } else { 3 + offset }).into(),
    };
    entry.validate()?;
    Ok(entry)
}
fn component(value: &str) -> Result<String, String> {
    if value.contains(',') {
        return Err("This prefix field cannot contain a comma in legacy syntax".into());
    }
    literal(value)
}
pub fn encode(entry: &Entry) -> Result<String, String> {
    entry.validate()?;
    match entry {
        Entry::Submap { name, reset } => Ok(format!(
            "submap = {}{}",
            component(if name.is_empty() { "reset" } else { name })?,
            match reset {
                Some(s) if !s.is_empty() => format!(",{}", component(s)?),
                _ => String::new(),
            }
        )),
        Entry::Unbind { all: true, .. } => Ok("unbind = all".into()),
        Entry::Unbind { chord, .. } => Ok(format!(
            "unbind = {},{}",
            component(&chord.modifiers.join(" "))?,
            component(&chord.keys[0])?
        )),
        Entry::Bind {
            chord,
            flags,
            description,
            device,
            dispatcher,
            argument,
        } => {
            let mut suffix: String = flags.iter().collect();
            if chord.multi {
                suffix.push('s');
            }
            if description.is_some() {
                suffix.push('d');
            }
            if device.is_some() {
                suffix.push('k');
            }
            let mut fields = vec![
                component(&chord.modifiers.join(if chord.multi { "&" } else { " " }))?,
                component(&chord.keys.join("&"))?,
            ];
            if let Some(filter) = device {
                fields.push(component(&format!(
                    "{}{}",
                    if filter.inclusive || filter.items.is_empty() {
                        ""
                    } else {
                        "!"
                    },
                    filter.items.join(" ")
                ))?);
            }
            if let Some(text) = description {
                fields.push(component(text)?);
            }
            if !flags.contains(&'m') {
                fields.push(component(dispatcher)?);
                fields.push(literal(argument)?);
            } else {
                fields.push(component(argument)?);
            }
            Ok(format!("bind{suffix} = {}", fields.join(",")))
        }
    }
}
pub fn export(draft: &Draft) -> Result<String, String> {
    draft.validate(BASELINE)?;
    let mut out = String::new();
    for entry in &draft.entries {
        out.push_str(&encode(entry)?);
        out.push('\n');
        if out.len() > MAX_DOCUMENT_BYTES {
            return Err("Legacy binding output exceeds 1 MiB".into());
        }
    }
    Ok(out)
}
fn scope(entry: &Entry, current: &str) -> String {
    if let Entry::Submap { name, reset } = entry {
        format!(
            "known:{}",
            source_hash(&serde_json::to_string(&(name, reset.as_deref().unwrap_or(""))).unwrap())
        )
    } else {
        current.into()
    }
}
pub fn inspect(content: &str) -> Result<Report, String> {
    let mut report = Report {
        schema_sha256: schema_hash(),
        source_sha256: source_hash(content),
        draft: Draft::default(),
        sites: vec![],
        unresolved: vec![],
        unresolved_count: 0,
        barriers_sha256: String::new(),
    };
    let mut context = scope(
        &Entry::Submap {
            name: String::new(),
            reset: None,
        },
        "",
    );
    let mut barriers = Sha256::new();
    let mut known = true;
    let mut named = false;
    legacy_syntax::walk(content, |line| {
        // Hyprlang substitutes inside non-variable LHS strings too: a key such
        // as s$tail can become submap/source. Never assume following scope.
        let dynamic_keyword = line.key.contains('$') && !line.key.starts_with('$');
        let relevant = dynamic_keyword
            || line.key.starts_with("bind")
            || ["submap", "unbind", "source"].contains(&line.key);
        if !relevant {
            return;
        }
        let parsed = if line.editable
            && line.key != "source"
            && (known || line.key == "submap" || line.key == "unbind")
            && report.draft.entries.len() < 10000
        {
            unescape(line.value)
                .and_then(|value| decode(line.key, &value))
                .ok()
        } else {
            None
        };
        let parsed=parsed.filter(|entry|!matches!(entry,Entry::Bind{chord,..}if !chord.multi&&chord.keys==["catchall"]&&!named));
        if let Some(entry) = parsed {
            if let Entry::Submap { name, .. } = &entry {
                known = true;
                named = !name.is_empty();
                context = scope(&entry, &context);
            }
            report.draft.entries.push(entry);
            report.sites.push(Site {
                line: line.line,
                statement: line.statement,
            });
        } else {
            if dynamic_keyword || line.key == "source" || line.key == "submap" {
                context = source_hash(&format!(
                    "{context}:{}",
                    source_hash(&content[line.statement.clone()])
                ));
                known = false;
            }
            report.unresolved_count += 1;
            if report.unresolved.len() < 4096 {
                report.unresolved.push(line.line);
            }
            barriers.update(source_hash(&content[line.statement]).as_bytes());
            barriers.update(source_hash(&context).as_bytes());
        }
    })?;
    report.barriers_sha256 = format!("{:x}", barriers.finalize());
    // JSON size also bounds temporary graph/state retention.
    if serde_json::to_vec(&report.draft)
        .map_err(|e| e.to_string())?
        .len()
        > MAX_DOCUMENT_BYTES
    {
        return Err("Legacy bindings exceed their projection budget".into());
    }
    Ok(report)
}
pub fn edited_copy(content: &str, target: &Draft) -> Result<String, String> {
    target.validate(BASELINE)?;
    let before = inspect(content)?;
    if before.draft == *target {
        return Ok(content.into());
    }
    let mut output = content.to_string();
    for (index, site) in before.sites.iter().enumerate().rev() {
        if target.entries.get(index) == before.draft.entries.get(index) {
            continue;
        }
        let text = target
            .entries
            .get(index)
            .map(|entry| {
                encode(entry).map(|text| {
                    format!(
                        "{text}{}",
                        if content.as_bytes().get(site.statement.end) == Some(&b'#') {
                            " "
                        } else {
                            ""
                        }
                    )
                })
            })
            .transpose()?
            .unwrap_or_default();
        output.replace_range(site.statement.clone(), &text);
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    for entry in target.entries.iter().skip(before.sites.len()) {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(&encode(entry)?);
        output.push_str(newline);
    }
    if output.len() > MAX_DOCUMENT_BYTES {
        return Err("Legacy source copy exceeds 1 MiB".into());
    }
    let after = inspect(&output)?;
    if export(&after.draft)? != export(target)? || after.barriers_sha256 != before.barriers_sha256 {
        return Err("The copy changes unresolved binding context or cannot reproduce the draft. Keep your project; add an explicit submap switch/reset where needed before exporting.".into());
    }
    Ok(output)
}
