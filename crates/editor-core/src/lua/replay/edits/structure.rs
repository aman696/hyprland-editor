//! Explicit call insertion/removal intentions. Source locations and the complete
//! remaining API/loader sequence are compared; ordinal coincidence is not identity.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum StructuralCall {
    Insert {
        document: String,
        edit_span: Range<usize>,
        call_span: Range<usize>,
        api: String,
        arguments: Vec<Observed>,
    },
    Remove {
        order: usize,
        api: String,
        arguments: Vec<Observed>,
    },
    RemoveSite {
        document: String,
        span: Range<usize>,
        api: String,
    },
}
pub(super) fn validate(plan: &EditPlan, nodes: &mut usize) -> Result<(), String> {
    if plan.structural_calls.len() > 256 {
        return Err("Too many requested structural calls".into());
    }
    let (mut insertions, mut removals) = (BTreeSet::new(), BTreeSet::new());
    for call in &plan.structural_calls {
        let (api, arguments): (&String, &[Observed]) = match call {
            StructuralCall::Insert {
                document,
                edit_span,
                call_span,
                api,
                arguments,
            } => {
                let edit = plan
                    .edits
                    .iter()
                    .find(|e| e.document == *document && e.span == *edit_span)
                    .ok_or("Inserted call has no corresponding source edit")?;
                if !edit_span.is_empty()
                    || call_span.is_empty()
                    || edit.replacement.get(call_span.clone()).is_none()
                    || !insertions.insert((
                        document,
                        edit_span.start,
                        call_span.start,
                        call_span.end,
                    ))
                {
                    return Err("Invalid or duplicate inserted call location".into());
                }
                (api, arguments)
            }
            StructuralCall::RemoveSite {
                document,
                span,
                api,
            } => {
                if span.is_empty()
                    || !insertions.insert((document, span.start, span.start, span.end))
                    || !plan.edits.iter().any(|e| {
                        e.document == *document
                            && e.span.start <= span.start
                            && e.span.end >= span.end
                    })
                {
                    return Err(
                        "Removed declaration site is not bound to a unique source patch".into(),
                    );
                }
                (api, &[])
            }
            StructuralCall::Remove {
                order,
                api,
                arguments,
            } => {
                if !removals.insert(*order) || plan.expected_calls.iter().any(|c| c.order == *order)
                {
                    return Err("Conflicting or duplicate removed calls".into());
                }
                (api, arguments)
            }
        };
        if api.is_empty()
            || api.len() > 64
            || !api
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_')
            || arguments.len() > 1024
        {
            return Err("Invalid structural call identity".into());
        }
        for value in arguments {
            validate_value(value, 0, nodes)?;
        }
    }
    Ok(())
}
fn inserted_span(
    plan: &EditPlan,
    document: &str,
    span: &Range<usize>,
    call: &Range<usize>,
) -> Result<Range<usize>, String> {
    let mut delta = 0_i64;
    for edit in &plan.edits {
        if edit.document != document {
            continue;
        }
        if edit.span == *span {
            return Ok(shift(span.start, delta)?
                .checked_add(call.start)
                .ok_or("Inserted call offset overflow")?
                ..shift(span.start, delta)?
                    .checked_add(call.end)
                    .ok_or("Inserted call offset overflow")?);
        }
        delta += edit.replacement.len() as i64 - edit.span.len() as i64;
    }
    Err("Missing inserted source edit".into())
}
#[derive(Clone, Copy)]
enum Step<'a> {
    Call(&'a Event),
    Load(&'a super::super::LoadEvent),
}
impl Step<'_> {
    fn order(self) -> usize {
        match self {
            Self::Call(e) => e.order,
            Self::Load(e) => e.order,
        }
    }
}
fn steps<'a>(report: &'a Report, excluded: &BTreeSet<usize>) -> Vec<Step<'a>> {
    let mut steps = report
        .events
        .iter()
        .filter(|e| !excluded.contains(&e.order))
        .map(Step::Call)
        .chain(report.loads.iter().map(Step::Load))
        .collect::<Vec<_>>();
    steps.sort_by_key(|s| s.order());
    steps
}
fn difference(kind: DifferenceKind, before: Option<&Event>, after: Option<&Event>) -> Difference {
    Difference {
        kind,
        order: before.map(|e| e.order).or_else(|| after.map(|e| e.order)),
        before_arguments: before.map(|e| e.arguments.clone()),
        after_arguments: after.map(|e| e.arguments.clone()),
    }
}
pub(super) fn compare(
    plan: &EditPlan,
    proposed: &Program,
    before: &Report,
    after: &Report,
) -> Result<(Vec<Difference>, usize, usize), String> {
    let (mut removed, mut inserted) = (BTreeSet::new(), BTreeSet::new());
    let (mut differences, mut matched, mut structural_matched) = (vec![], 0, 0);
    for expected in &plan.expected_calls {
        let event = before
            .events
            .iter()
            .find(|e| e.order == expected.order)
            .ok_or("Expected call is absent from original model")?;
        if !same_arguments(&event.arguments, &expected.before_arguments) {
            return Err("Expected arguments differ from original model".into());
        }
    }
    for call in &plan.structural_calls {
        match call {
            StructuralCall::RemoveSite {
                document,
                span,
                api,
            } => {
                let events = before
                    .events
                    .iter()
                    .filter(|e| e.document == *document && e.span == *span && e.api == *api)
                    .collect::<Vec<_>>();
                if events
                    .iter()
                    .any(|e| after.events.iter().any(|next| same_site(plan, e, next)))
                {
                    differences.push(difference(
                        DifferenceKind::RequestedDeclaration,
                        events.first().copied(),
                        None,
                    ));
                } else {
                    structural_matched += 1;
                }
                for event in events {
                    if !removed.insert(event.order) {
                        return Err("Overlapping structural removal intentions".into());
                    }
                }
            }
            StructuralCall::Remove {
                order,
                api,
                arguments,
            } => {
                let event = before
                    .events
                    .iter()
                    .find(|e| e.order == *order)
                    .ok_or("Requested removed call was not observed")?;
                if event.api != *api
                    || !same_arguments(&event.arguments, arguments)
                    || !plan.edits.iter().any(|edit| {
                        edit.document == event.document
                            && edit.span.start <= event.span.start
                            && edit.span.end >= event.span.end
                    })
                {
                    return Err("Requested removed call is not bound to its original arguments and source patch".into());
                }
                // Any surviving call in this replacement remains in the after
                // sequence and is reported as an extra/unmatched observation.
                if !removed.insert(*order) {
                    return Err("Overlapping structural removal intentions".into());
                }
                if after.events.iter().any(|next| same_site(plan, event, next)) {
                    differences.push(difference(
                        DifferenceKind::RequestedDeclaration,
                        Some(event),
                        None,
                    ));
                } else {
                    structural_matched += 1;
                }
            }
            StructuralCall::Insert {
                document,
                edit_span,
                call_span,
                api,
                arguments,
            } => {
                let span = inserted_span(plan, document, edit_span, call_span)?;
                let events = after
                    .events
                    .iter()
                    .filter(|e| e.document == *document && e.span == span)
                    .collect::<Vec<_>>();
                let valid = !events.is_empty()
                    && events
                        .iter()
                        .all(|e| e.api == *api && same_arguments(&e.arguments, arguments));
                if valid {
                    structural_matched += 1;
                } else {
                    differences.push(difference(
                        DifferenceKind::RequestedDeclaration,
                        None,
                        events.first().copied(),
                    ));
                }
                for event in events {
                    inserted.insert(event.order);
                }
            }
        }
    }
    let old = steps(before, &removed);
    let new = steps(after, &inserted);
    let pairs = align(plan, proposed, &old, &new)?;
    let paired_old = pairs.iter().map(|(a, _)| *a).collect::<BTreeSet<_>>();
    let paired_new = pairs.iter().map(|(_, b)| *b).collect::<BTreeSet<_>>();
    for &(i, j) in &pairs {
        match (old[i], new[j]) {
            (Step::Call(a), Step::Call(b)) => {
                let expected = plan.expected_calls.iter().find(|e| e.order == a.order);
                if same_arguments(
                    &b.arguments,
                    expected.map_or(&a.arguments, |e| &e.after_arguments),
                ) {
                    matched += usize::from(expected.is_some());
                } else {
                    differences.push(difference(DifferenceKind::Arguments, Some(a), Some(b)));
                }
            }
            (Step::Load(a), Step::Load(b)) => {
                if !same_load(&a.outcome, &b.outcome) {
                    differences.push(Difference {
                        kind: DifferenceKind::Loader,
                        order: Some(a.order),
                        before_arguments: None,
                        after_arguments: None,
                    });
                }
            }
            _ => unreachable!("Alignment pairs only equal source identities"),
        }
    }
    for (steps, paired, is_before) in [(&old, &paired_old, true), (&new, &paired_new, false)] {
        for (index, step) in steps.iter().enumerate() {
            if paired.contains(&index) {
                continue;
            }
            match step {
                Step::Call(e) => differences.push(if is_before {
                    difference(DifferenceKind::MissingCall, Some(e), None)
                } else {
                    difference(DifferenceKind::AddedCall, None, Some(e))
                }),
                Step::Load(e) => differences.push(Difference {
                    kind: DifferenceKind::Loader,
                    order: Some(e.order),
                    before_arguments: None,
                    after_arguments: None,
                }),
            }
        }
    }
    // Decisions refer to the *next* observation ordinal. Compare the number of
    // retained observations before that point, including loaders and excluding
    // only the explicit intended call changes. Sequence comparison above still
    // detects moved, added, missing and changed retained operations.
    let path_equal = before.path_decisions.len() == after.path_decisions.len()
        && before
            .path_decisions
            .iter()
            .zip(&after.path_decisions)
            .all(|(a, b)| {
                pairs
                    .iter()
                    .filter(|(i, _)| old[*i].order() < a.at_order)
                    .count()
                    == pairs
                        .iter()
                        .filter(|(_, j)| new[*j].order() < b.at_order)
                        .count()
                    && a.kind == b.kind
                    && a.outcome == b.outcome
                    && proposed.documents.get(a.site.document).is_some_and(|doc| {
                        a.site.document == b.site.document
                            && map_span(plan, &doc.id, &a.site.span, false)
                                .is_ok_and(|s| s == b.site.span)
                    })
            });
    if !path_equal {
        differences.push(difference(DifferenceKind::PathHistory, None, None));
    }
    if !same_arguments(&before.returns, &after.returns) {
        differences.push(Difference {
            kind: DifferenceKind::ReturnValues,
            order: None,
            before_arguments: Some(before.returns.clone()),
            after_arguments: Some(after.returns.clone()),
        });
    }
    if before.logged_module_errors != after.logged_module_errors {
        differences.push(difference(DifferenceKind::LoggedModuleErrors, None, None));
    }
    if before.halt.is_some() || after.halt.is_some() {
        differences.push(difference(DifferenceKind::IncompleteModel, None, None));
    }
    Ok((differences, matched, structural_matched))
}

#[derive(PartialEq, Eq)]
enum SourceKey {
    Call(String, String, Range<usize>),
    Load(super::super::Request),
}
fn key(plan: &EditPlan, proposed: &Program, step: Step<'_>, original: bool) -> Option<SourceKey> {
    match step {
        Step::Call(e) => Some(SourceKey::Call(
            e.api.clone(),
            e.document.clone(),
            if original {
                map_span(plan, &e.document, &e.span, false).ok()?
            } else {
                e.span.clone()
            },
        )),
        Step::Load(e) => Some(SourceKey::Load(if original {
            rebase_request(proposed, plan, &e.request).ok()?
        } else {
            e.request.clone()
        })),
    }
}
fn align(
    plan: &EditPlan,
    proposed: &Program,
    old: &[Step<'_>],
    new: &[Step<'_>],
) -> Result<Vec<(usize, usize)>, String> {
    // Bounded LCS preserves execution order without treating shifted ordinals
    // as identities. Compare arguments only after alignment, so changed values
    // cannot be hidden by choosing a different equal-valued occurrence.
    let cells = (old.len() + 1)
        .checked_mul(new.len() + 1)
        .ok_or("Structural sequence size overflow")?;
    if cells > 4_000_000 {
        return Err("Structural comparison exceeds its sequence memory/work budget".into());
    }
    let a = old
        .iter()
        .map(|s| key(plan, proposed, *s, true))
        .collect::<Vec<_>>();
    let b = new
        .iter()
        .map(|s| key(plan, proposed, *s, false))
        .collect::<Vec<_>>();
    let width = new.len() + 1;
    let mut lengths = vec![0_u16; cells];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lengths[i * width + j] = if a[i].is_some() && a[i] == b[j] {
                1 + lengths[(i + 1) * width + j + 1]
            } else {
                lengths[(i + 1) * width + j].max(lengths[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j, mut pairs) = (0, 0, vec![]);
    while i < old.len() && j < new.len() {
        if a[i].is_some() && a[i] == b[j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if lengths[(i + 1) * width + j] >= lengths[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    Ok(pairs)
}
