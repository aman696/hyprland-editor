//! Private reconciliation of scalar overrides with modeled source declarations.
//! Parsed values are not effective compositor observations or ownership certificates.
use super::{edits, replay, scalar_edit, Program};
use crate::{
    lua::{
        config_inspection, registration,
        scalar_codec::{NativeValue, Outcome, Representation},
    },
    workspace_graph, Draft, Value,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceValue {
    Parsed {
        value: Value,
        native: NativeValue,
        provisional: bool,
    },
    Absent,
    Unresolved {
        reason: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub event_order: usize,
    pub location: Vec<usize>,
    pub preserve_table_layout: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub setting_id: String,
    pub override_value: Value,
    pub source: SourceValue,
    pub declaration_count: usize,
    pub target: Option<Target>,
    pub can_write: bool,
    /// Primary-value equality under complete modeled scalar-call analysis;
    /// never an effective compositor observation.
    pub already_represented: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub schema_sha256: String,
    pub program_sha256: String,
    pub draft_sha256: String,
    pub version: String,
    pub compiler: super::CompilerProfile,
    pub model_complete: bool,
    pub rows: Vec<Row>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    KeepSeparate,
    UseSource,
    WriteOverride,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    pub review_sha256: String,
    pub choices_sha256: String,
    pub program_sha256: String,
    pub plans: Vec<edits::EditPlan>,
    pub draft: Draft,
    pub remaining: usize,
    pub model_complete: bool,
    pub ownership_verified: bool,
    pub runtime_verified: bool,
}
pub fn prepare(program: &Program, draft: &Draft) -> Result<Review, String> {
    inspect(program, draft, true)
}
fn inspect(program: &Program, draft: &Draft, certify_noops: bool) -> Result<Review, String> {
    edits::fingerprint(draft)?;
    draft.validate(&crate::catalogue())?;
    if program.profile != super::Profile::HyprlandLua54 {
        return Err("Source reconciliation requires the pinned Hyprland Lua model".into());
    }
    if program.version != draft.version {
        return Err("Source and overrides use different snapshots".into());
    }
    let report = replay(program)?;
    let prefix = report
        .events
        .iter()
        .take_while(|e| matches!(e.api.as_str(), "hl.config" | "hl.monitor"))
        .count();
    // Monitor routing has fixed zero modeled results. Keep its full argument
    // effects in the replay/edit comparison, but don't pass its table to the
    // scalar registry or let it truncate later configuration observations.
    let scalar_events = report.events[..prefix]
        .iter()
        .filter(|e| e.api == "hl.config")
        .cloned()
        .collect::<Vec<_>>();
    let inspection = config_inspection::inspect(&scalar_events, &program.version)?;
    let complete = report.halt.is_none()
        && prefix == report.events.len()
        && inspection.stopped.is_none()
        && inspection.batches.iter().all(|b| {
            b.traversal_resolved
                && b.findings.is_empty()
                && b.candidates.iter().all(|c| {
                    !c.ambiguous && matches!(c.model_value, Some(Outcome::Accepted { .. }))
                })
        });
    let mut rows = vec![];
    for (id, value) in &draft.values {
        let candidates = inspection
            .batches
            .iter()
            .flat_map(|b| &b.candidates)
            .filter(|c| c.setting_id == *id)
            .collect::<Vec<_>>();
        let last = candidates.last().copied();
        let source = match last {
            Some(c) if c.ambiguous => SourceValue::Unresolved {
                reason: "Several native keys address this setting".into(),
            },
            Some(c) => match &c.model_value {
                Some(Outcome::Accepted {
                    native,
                    representation: Representation::Value { value },
                }) => SourceValue::Parsed {
                    value: value.clone(),
                    native: native.clone(),
                    provisional: !complete,
                },
                _ => SourceValue::Unresolved {
                    reason:
                        "The last reached declaration is rejected, unrepresentable or not modeled"
                            .into(),
                },
            },
            None if complete => SourceValue::Absent,
            None => SourceValue::Unresolved {
                reason: "Analysis does not establish that this setting is absent".into(),
            },
        };
        let target = last
            .filter(|c| !c.ambiguous && c.expression.is_some())
            .map(|c| Target {
                event_order: c.event_order,
                location: c.location.clone(),
                preserve_table_layout: c
                    .expression
                    .and_then(|id| report.expression_sources.get(id as usize))
                    .is_some_and(|e| e.kind == super::ExpressionKind::Table)
                    && registration::by_lua_key(&program.version, &c.lua_key)
                        .is_some_and(|m| matches!(m.kind.as_str(), "Vec2" | "CssGap" | "Gradient")),
            });
        let can_write = target.is_some()
            && last
                .and_then(|c| registration::by_lua_key(&program.version, &c.lua_key))
                .is_some_and(|m| {
                    m.codec.is_some()
                        && matches!(
                            m.kind.as_str(),
                            "Bool"
                                | "Int"
                                | "Float"
                                | "String"
                                | "Color"
                                | "Gradient"
                                | "Vec2"
                                | "CssGap"
                                | "FontWeight"
                        )
                });
        let already_represented = certify_noops
            && complete
            && matches!(&source,SourceValue::Parsed {native:observed,..} if native(program,id,value).is_ok_and(|desired|desired==*observed));
        rows.push(Row {
            setting_id: id.clone(),
            override_value: value.clone(),
            source,
            declaration_count: candidates.len(),
            target,
            can_write,
            already_represented,
        });
    }
    let review = Review {
        schema_sha256: workspace_graph::schema_sha256(),
        program_sha256: workspace_graph::program_fingerprint(program)?,
        draft_sha256: edits::fingerprint(draft)?,
        version: draft.version.clone(),
        compiler: super::compiler_profile(),
        model_complete: complete,
        rows,
    };
    edits::fingerprint(&review)?;
    Ok(review)
}
fn native(program: &Program, id: &str, value: &Value) -> Result<NativeValue, String> {
    let meta = registration::find(&program.version, registration::Role::Global, id)
        .or_else(|| registration::find(&program.version, registration::Role::Bootstrap, id))
        .ok_or("Unknown registered scalar override")?;
    match scalar_edit::proposed(meta, value)?.1 {
        Outcome::Accepted {
            native,
            representation: Representation::Value { .. },
        } => Ok(native),
        _ => Err("The pinned native parser rejects this override or cannot represent it".into()),
    }
}
/// Builds sequential source plans and a reduced override layer. Callers keep the
/// exact reviewed plans through trusted piece operations; no file is written.
pub fn resolve(
    program: &Program,
    draft: &Draft,
    review: &Review,
    choices: &BTreeMap<String, Choice>,
) -> Result<Resolution, String> {
    edits::fingerprint(review)?;
    edits::fingerprint(choices)?;
    let fresh = prepare(program, draft)?;
    if edits::fingerprint(&fresh)? != edits::fingerprint(review)?
        || choices.keys().any(|id| !draft.values.contains_key(id))
    {
        return Err("The source/override review changed or names an unknown setting".into());
    }
    let mut current = program.clone();
    let mut reduced = draft.clone();
    let mut plans = vec![];
    let mut pieces = vec![];
    for row in &review.rows {
        match choices
            .get(&row.setting_id)
            .copied()
            .unwrap_or(Choice::KeepSeparate)
        {
            Choice::KeepSeparate => {}
            Choice::UseSource => {
                reduced.values.remove(&row.setting_id);
            }
            Choice::WriteOverride => {
                // Only the outward review needs a per-row no-op certificate.
                // Internal refreshes compare the selected native value below;
                // don't repeat every override conversion for every source edit.
                let now = inspect(&current, &reduced, false)?;
                let target = now
                    .rows
                    .iter()
                    .find(|r| r.setting_id == row.setting_id)
                    .ok_or("Override disappeared during reconciliation")?;
                if !target.can_write {
                    return Err("This override needs declaration insertion or an unfinished native/source codec".into());
                }
                let desired = native(&current, &row.setting_id, &row.override_value)?;
                let same =
                    matches!(&target.source,SourceValue::Parsed{native,..} if native==&desired);
                if same && !now.model_complete {
                    return Err("Incomplete configuration analysis cannot establish that the override is already represented".into());
                }
                if !same {
                    let target = target
                        .target
                        .as_ref()
                        .ok_or("No unambiguous reached source expression")?;
                    let compared = scalar_edit::compare(
                        &current,
                        &scalar_edit::Change {
                            event_order: target.event_order,
                            location: target.location.clone(),
                            setting_id: row.setting_id.clone(),
                            selection: if target.preserve_table_layout {
                                scalar_edit::Selection::PreserveTableLayout
                            } else {
                                scalar_edit::Selection::SelectedExpression
                            },
                            value: row.override_value.clone(),
                        },
                    )?;
                    if !compared.review.modeled_observations_match {
                        return Err("Writing this override changes other modeled effects or crosses incomplete analysis. Review it in the source-change editor before reconciling.".into());
                    }
                    pieces = workspace_graph::revisions::prepare_pieces(
                        program,
                        &pieces,
                        &compared.plan,
                    )?;
                    current = workspace_graph::revisions::program(program.clone(), &pieces)?;
                    let after = inspect(&current, &reduced, false)?;
                    if !after.model_complete || !after.rows.iter().any(|r|r.setting_id==row.setting_id&&matches!(&r.source,SourceValue::Parsed{native,..} if native==&desired)) {
                        return Err("The proposed source does not establish the intended native value under complete configuration-call analysis".into());
                    }
                    plans.push(compared.plan);
                    edits::fingerprint(&plans)?;
                }
                reduced.values.remove(&row.setting_id);
            }
        }
    }
    let final_review = inspect(&current, &reduced, false)?;
    let result = Resolution {
        review_sha256: edits::fingerprint(review)?,
        choices_sha256: edits::fingerprint(choices)?,
        program_sha256: workspace_graph::program_fingerprint(&current)?,
        remaining: reduced.values.len(),
        model_complete: final_review.model_complete,
        plans,
        draft: reduced,
        ownership_verified: false,
        runtime_verified: false,
    };
    edits::fingerprint(&result)?;
    Ok(result)
}

/// Pure client-side envelope/layer checks, without parsing or native conversion.
/// The caller must also materialize the plans through trusted piece operations
/// and compare the resulting program fingerprint before keeping a revision.
pub fn validate_resolution(
    draft: &Draft,
    review: &Review,
    choices: &BTreeMap<String, Choice>,
    result: &Resolution,
) -> Result<(), String> {
    edits::fingerprint(result)?;
    if result.ownership_verified
        || result.runtime_verified
        || result.review_sha256 != edits::fingerprint(review)?
        || result.choices_sha256 != edits::fingerprint(choices)?
        || review.draft_sha256 != edits::fingerprint(draft)?
        || review.version != draft.version
        || choices.keys().any(|id| !draft.values.contains_key(id))
    {
        return Err("The reconciliation result does not match its reviewed inputs".into());
    }
    let mut expected = draft.clone();
    for (id, choice) in choices {
        if *choice != Choice::KeepSeparate {
            expected.values.remove(id);
        }
    }
    if edits::fingerprint(&expected)? != edits::fingerprint(&result.draft)?
        || result.remaining != expected.values.len()
    {
        return Err("The result changes overrides beyond the explicit choices".into());
    }
    let mut written = std::collections::BTreeSet::new();
    for plan in &result.plans {
        let Some(edit) = plan.edits.first() else {
            return Err("Missing reconciliation edit".into());
        };
        let Some(id) = edit.semantic_targets.first() else {
            return Err("Missing reconciliation setting".into());
        };
        if plan.edits.len() != 1
            || plan.expected_calls.len() != 1
            || !plan.structural_calls.is_empty()
            || edit.semantic_targets.len() != 1
            || !written.insert(id.clone())
            || choices.get(id) != Some(&Choice::WriteOverride)
        {
            return Err("The result contains an unselected or repeated source change".into());
        }
    }
    for (id, choice) in choices {
        if *choice == Choice::WriteOverride {
            let row = review
                .rows
                .iter()
                .find(|r| r.setting_id == *id)
                .ok_or("Missing reviewed scalar row")?;
            if !row.can_write
                || written.contains(id) == row.already_represented
                || !result.model_complete
            {
                return Err("The result omits a required scalar source edit or invents a no-op/effective-value claim".into());
            }
        }
    }
    if result.plans.is_empty() && result.program_sha256 != review.program_sha256 {
        return Err("A result without source edits changed the program identity".into());
    }
    Ok(())
}
