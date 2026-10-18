//! Typed scalar source comparisons; no claim of exclusive ownership or write authority.
use super::{edits, replay, ExpressionKind, Observed, Program};
use crate::{
    lua::{config_inspection, registration, scalar_codec},
    Value,
};
use serde::{Deserialize, Serialize};
mod layout;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Selection {
    SelectedExpression,
    OriginalLiteral,
    PreserveTableLayout,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub event_order: usize,
    pub location: Vec<usize>,
    pub setting_id: String,
    pub selection: Selection,
    pub value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub plan: edits::EditPlan,
    pub review: edits::EditReview,
    pub proposed_native_value: scalar_codec::Outcome,
    pub table_layout_preserved: bool,
}
// Match the evaluator's canonical byte-key ordering for expected observations.
fn fields(entries: impl IntoIterator<Item = (&'static str, Observed)>) -> Observed {
    Observed::Table(
        entries
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>()
            .into_iter()
            .map(|(name, value)| (Observed::Bytes(name.as_bytes().to_vec()), value))
            .collect(),
    )
}
fn input(value: &Value, kind: &str) -> Result<Observed, String> {
    Ok(match (kind, value) {
        ("Bool", Value::Bool(v)) => Observed::Bool(*v),
        ("Int", Value::Int(v)) => Observed::Int(*v),
        ("Float", Value::Float(v)) if v.is_finite() => Observed::Float(*v),
        ("String", Value::String(v)) if v.len() <= 16384 && !v.contains('\0') => {
            Observed::Bytes(v.as_bytes().to_vec())
        }
        ("Color", Value::Color(v)) => Observed::Bytes(format!("0x{v:08X}").into_bytes()),
        ("FontWeight", Value::FontWeight(v)) if *v >= 0 => Observed::Int(*v),
        ("Vec2", Value::Vec2(v)) if v.iter().all(|v| v.is_finite()) => Observed::Table(
            v.iter()
                .enumerate()
                .map(|(i, v)| (Observed::Int(i as i64 + 1), Observed::Float(*v)))
                .collect(),
        ),
        ("CssGap", Value::CssGap(v)) => fields(
            ["top", "right", "bottom", "left"]
                .into_iter()
                .zip(v)
                .map(|(name, v)| (name, Observed::Int(*v))),
        ),
        ("Gradient", Value::Gradient { colors, angle })
            if !colors.is_empty() && colors.len() <= 32 && angle.is_finite() =>
        {
            fields([
                (
                    "colors",
                    Observed::Table(
                        colors
                            .iter()
                            .enumerate()
                            .map(|(i, v)| {
                                (
                                    Observed::Int(i as i64 + 1),
                                    Observed::Bytes(format!("0x{v:08X}").into_bytes()),
                                )
                            })
                            .collect(),
                    ),
                ),
                ("angle", Observed::Float(*angle)),
            ])
        }
        _ => return Err(
            "The requested value is not a supported finite, representable value of this setting"
                .into(),
        ),
    })
}
fn print(value: &Observed) -> Result<String, String> {
    Ok(match value {
        Observed::Bool(v) => v.to_string(),
        Observed::Int(i64::MIN) => "(-9223372036854775807 - 1)".into(),
        Observed::Int(v) => v.to_string(),
        Observed::Float(v) if v.is_finite() => format!("{v:?}"),
        Observed::Bytes(bytes) => {
            if let Ok(text) = std::str::from_utf8(bytes) {
                return Ok(Value::String(text.into()).lua());
            }
            let mut text = String::from("\"");
            for byte in bytes {
                text.push_str(&format!("\\{byte:03}"));
            }
            text.push('"');
            text
        }
        Observed::Table(rows) if rows.len() <= 4096 => {
            let fields = rows
                .iter()
                .map(|(key, value)| Ok(format!("[{}] = {}", print(key)?, print(value)?)))
                .collect::<Result<Vec<_>, String>>()?;
            let text = format!("{{{}}}", fields.join(", "));
            if text.len() > crate::MAX_DOCUMENT_BYTES {
                return Err("Compound literal exceeds the source size limit".into());
            }
            text
        }
        _ => return Err("Unsupported typed source printer".into()),
    })
}
pub(super) fn proposed(
    meta: &registration::Entry,
    value: &Value,
) -> Result<(Observed, scalar_codec::Outcome), String> {
    let codec = meta
        .codec
        .as_ref()
        .ok_or("This setting's native value codec is unfinished")?;
    let mut proposed = input(value, &meta.kind)?;
    let mut outcomes = scalar_codec::decode_batch(&[scalar_codec::Case {
        codec: codec.clone(),
        input: proposed.clone(),
    }])?;
    // Native names may represent values which the numeric overload rejects.
    // Preserve that native route rather than silently loosening numeric bounds.
    if !matches!(
        outcomes[0],
        scalar_codec::Outcome::Accepted {
            representation: scalar_codec::Representation::Value { .. },
            ..
        }
    ) {
        if let (
            Value::Int(value),
            scalar_codec::Codec::Int {
                names: Some(names), ..
            },
        ) = (value, codec)
        {
            for name in names.iter().filter(|n| n.value == *value) {
                let trial = Observed::Bytes(name.name.clone());
                let outcome = scalar_codec::decode_batch(&[scalar_codec::Case {
                    codec: codec.clone(),
                    input: trial.clone(),
                }])?
                .remove(0);
                if matches!(
                    outcome,
                    scalar_codec::Outcome::Accepted {
                        representation: scalar_codec::Representation::Value { .. },
                        ..
                    }
                ) {
                    proposed = trial;
                    outcomes[0] = outcome;
                    break;
                }
            }
        }
    }
    if !matches!(
        outcomes[0],
        scalar_codec::Outcome::Accepted {
            representation: scalar_codec::Representation::Value { .. },
            ..
        }
    ) {
        return Err("The pinned native parser rejects this value or cannot verify it".into());
    }
    Ok((proposed, outcomes.remove(0)))
}
pub fn compare(program: &Program, change: &Change) -> Result<Comparison, String> {
    if change.location.is_empty() || change.location.len() > 33 || change.setting_id.len() > 128 {
        return Err("Invalid selected setting coordinates".into());
    }
    let before = replay(program)?;
    let event_index = before
        .events
        .iter()
        .position(|e| e.order == change.event_order)
        .ok_or("The selected call is unavailable")?;
    if before.events[..=event_index]
        .iter()
        .any(|e| e.api != "hl.config")
    {
        return Err("An earlier unsupported API prevents typed source comparison".into());
    }
    let inspection = config_inspection::inspect(&before.events[..=event_index], &program.version)?;
    let candidate = inspection
        .batches
        .iter()
        .find(|b| b.event_order == change.event_order)
        .and_then(|b| {
            b.candidates
                .iter()
                .find(|c| c.location == change.location && c.setting_id == change.setting_id)
        })
        .ok_or("Selected setting was not reached by typed inspection")?;
    if candidate.ambiguous {
        return Err("The selected native key is ambiguous".into());
    }
    let meta = registration::by_lua_key(&program.version, &candidate.lua_key)
        .ok_or("Missing registered setting metadata")?;
    let (mut proposed, outcome) = proposed(meta, &change.value)?;
    let mut id = candidate
        .expression
        .ok_or("No exact source expression is recorded for this value")?;
    if matches!(change.selection, Selection::OriginalLiteral) {
        for _ in 0..8192 {
            let expression = before
                .expression_sources
                .get(id as usize)
                .ok_or("Missing source-expression evidence")?;
            if expression.kind == ExpressionKind::Literal {
                break;
            }
            id=expression.selected.ok_or("This computed value has no single original literal; compare its selected expression instead")?;
        }
        if before.expression_sources[id as usize].kind != ExpressionKind::Literal {
            return Err("Original literal traversal exceeded its limit".into());
        }
    }
    let expression = before
        .expression_sources
        .get(id as usize)
        .ok_or("Missing expression evidence")?;
    let document = program
        .documents
        .get(expression.document)
        .ok_or("Missing source document")?;
    let old = document
        .source
        .get(expression.span.clone())
        .ok_or("Invalid source-expression span")?;
    let replacement = if matches!(change.selection, Selection::PreserveTableLayout) {
        let current = candidate
            .value(&before.events[event_index])
            .ok_or("Missing selected value")?;
        let (replacement, expected) = layout::prepare(
            &document.source,
            expression.span.clone(),
            &meta.kind,
            current,
            &proposed,
        )?;
        let decoded = scalar_codec::decode_batch(&[scalar_codec::Case {
            codec: meta.codec.clone().ok_or("Missing codec")?,
            input: expected.clone(),
        }])?
        .remove(0);
        if !matches!((&decoded, &outcome), (scalar_codec::Outcome::Accepted {native:a, ..}, scalar_codec::Outcome::Accepted {native:b, ..}) if a==b)
        {
            return Err(
                "Preserving unrelated fields does not establish the requested native value".into(),
            );
        }
        proposed = expected;
        replacement
    } else {
        print(&proposed)?
    };
    let source_edit = edits::SourceEdit {
        document: document.id.clone(),
        base_sha256: document.sha256.clone(),
        span: expression.span.clone(),
        old_bytes_sha256: crate::lua::source_hash(old),
        replacement,
        semantic_targets: vec![meta.id.clone()],
    };
    let event = &before.events[event_index];
    let mut after_arguments = event.arguments.clone();
    let mut path = change.location.iter();
    let mut value = after_arguments
        .get_mut(*path.next().ok_or("Missing argument coordinate")?)
        .ok_or("Invalid argument coordinate")?;
    for &index in path {
        let Observed::Table(rows) = value else {
            return Err("Invalid value row coordinate".into());
        };
        value = &mut rows.get_mut(index).ok_or("Missing value row")?.1;
    }
    *value = proposed;
    let plan = edits::prepare(
        program,
        vec![source_edit],
        vec![edits::ExpectedCall {
            order: event.order,
            before_arguments: event.arguments.clone(),
            after_arguments,
        }],
    )?;
    let review = edits::review_with_before(program, &plan, before)?;
    let comparison = Comparison {
        plan,
        review,
        proposed_native_value: outcome,
        table_layout_preserved: matches!(change.selection, Selection::PreserveTableLayout),
    };
    edits::fingerprint(&comparison)?;
    Ok(comparison)
}
