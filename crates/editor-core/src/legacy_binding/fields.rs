//! Reuse raw variable references only for unchanged semantic fields.
use super::{encode, variables::Locals, Entry};
use crate::legacy_syntax::trim;
fn values(entry: &Entry) -> Vec<(&'static str, String)> {
    match entry {
        Entry::Bind {
            chord,
            flags,
            description,
            device,
            dispatcher,
            argument,
        } => {
            let mut fields = vec![
                (
                    "modifiers",
                    chord.modifiers.join(if chord.multi { "&" } else { " " }),
                ),
                ("key", chord.keys.join("&")),
            ];
            if let Some(device) = device {
                fields.push((
                    "device",
                    format!(
                        "{}{}",
                        if device.inclusive || device.items.is_empty() {
                            ""
                        } else {
                            "!"
                        },
                        device.items.join(" ")
                    ),
                ));
            }
            if let Some(description) = description {
                fields.push(("description", description.clone()));
            }
            if !flags.contains(&'m') {
                fields.push(("dispatcher", dispatcher.clone()));
            }
            fields.push(("argument", argument.clone()));
            fields
        }
        Entry::Submap { name, reset } => {
            let mut fields = vec![(
                "name",
                if name.is_empty() {
                    "reset".into()
                } else {
                    name.clone()
                },
            )];
            if let Some(reset) = reset.as_ref().filter(|s| !s.is_empty()) {
                fields.push(("reset", reset.clone()));
            }
            fields
        }
        Entry::Unbind { all: true, .. } => vec![("all", "all".into())],
        Entry::Unbind { chord, .. } => vec![
            ("modifiers", chord.modifiers.join(" ")),
            ("key", chord.keys[0].clone()),
        ],
    }
}
pub(super) fn boundaries(raw_value: &str, entry: &Entry, locals: &Locals) -> bool {
    let fields = values(entry);
    let parts: Vec<_> = raw_value.splitn(fields.len(), ',').collect();
    parts.iter().zip(fields).all(|(raw, (id, _))| {
        locals
            .expand(raw)
            .is_ok_and(|(expanded, _)| id == "argument" || !expanded.contains(','))
    })
}
fn same_keyword(before: &Entry, after: &Entry) -> bool {
    match (before, after) {
        (
            Entry::Bind {
                chord: a,
                flags: af,
                description: ad,
                device: av,
                ..
            },
            Entry::Bind {
                chord: b,
                flags: bf,
                description: bd,
                device: bv,
                ..
            },
        ) => {
            a.multi == b.multi
                && af == bf
                && ad.is_some() == bd.is_some()
                && av.is_some() == bv.is_some()
        }
        (Entry::Submap { .. }, Entry::Submap { .. })
        | (Entry::Unbind { .. }, Entry::Unbind { .. }) => true,
        _ => false,
    }
}
pub(super) fn render(raw: &str, before: &Entry, target: &Entry) -> Result<String, String> {
    let encoded = encode(target)?;
    let (new_key, new_value) = encoded.split_once('=').ok_or("Invalid encoded shortcut")?;
    let Some((old_key, old_value)) = raw.split_once('=') else {
        return Err("Captured shortcut has no assignment".into());
    };
    let old_fields = values(before);
    let new_fields = values(target);
    let old_parts: Vec<_> = old_value.splitn(old_fields.len(), ',').collect();
    let new_parts: Vec<_> = new_value.splitn(new_fields.len(), ',').collect();
    let output = new_fields
        .iter()
        .zip(new_parts)
        .map(|((id, value), canonical)| {
            old_fields
                .iter()
                .position(|(old_id, old_value)| old_id == id && old_value == value)
                .and_then(|index| old_parts.get(index))
                .map_or(trim(canonical), |raw| trim(raw))
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{} = {}",
        trim(if same_keyword(before, target) {
            old_key
        } else {
            new_key
        }),
        output
    ))
}
