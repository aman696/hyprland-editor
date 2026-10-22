//! Lossless three-way text reconciliation for captured sources. This module has
//! no filesystem/parser/IPC authority and never infers semantic ownership.
mod diff;
use crate::{lua::source_hash, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
};
pub const MAX_ITEMS: usize = 4096;
const MAX_WORK: usize = 32_000_000;
const MAX_FRONTIER: usize = 4_194_312;
const MAX_ANCHORS: usize = 8192;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    Base,
    Private,
    Incoming,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fragment {
    pub input: Input,
    pub span: Range<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Conflict {
    Overlap,
    AmbiguousAlignment,
    ConcurrentInsertions,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub base: Range<usize>,
    pub private: Vec<Fragment>,
    pub incoming: Vec<Fragment>,
    pub automatic: Option<Input>,
    pub conflict: Option<Conflict>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub bytes: usize,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub format: String,
    pub schema_sha256: String,
    pub base: Pin,
    pub private: Pin,
    pub incoming: Pin,
    pub regions: Vec<Region>,
    pub work: usize,
    pub peak_frontier_cells: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    Private,
    Incoming,
    Base,
    PrivateThenIncoming,
    IncomingThenPrivate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    pub review_sha256: String,
    pub choices_sha256: String,
    pub sha256: String,
    pub content: String,
    pub fragments: Vec<Fragment>,
    pub syntax_verified: bool,
    pub ownership_verified: bool,
    pub runtime_verified: bool,
}
pub(super) struct Budget {
    remaining: usize,
    peak: usize,
    anchors: usize,
}
impl Budget {
    fn new() -> Self {
        Self {
            remaining: MAX_WORK,
            peak: 0,
            anchors: 0,
        }
    }
    fn work(&mut self, n: usize) -> Result<(), String> {
        self.remaining = self
            .remaining
            .checked_sub(n)
            .ok_or("Source merge exceeds its deterministic work budget")?;
        Ok(())
    }
    fn frontier(&mut self, n: usize) -> Result<(), String> {
        if n > MAX_FRONTIER {
            return Err("Source merge exceeds its frontier memory budget".into());
        }
        self.peak = self.peak.max(n);
        Ok(())
    }
}
fn hash(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|v| source_hash(std::str::from_utf8(&v).expect("Serialized JSON")))
        .map_err(|e| e.to_string())
}
fn input<'a>(which: Input, base: &'a str, private: &'a str, incoming: &'a str) -> &'a str {
    match which {
        Input::Base => base,
        Input::Private => private,
        Input::Incoming => incoming,
    }
}
fn push(out: &mut Vec<Fragment>, part: Fragment) -> Result<(), String> {
    if part.span.is_empty() {
        return Ok(());
    }
    if let Some(last) = out.last_mut() {
        if last.input == part.input && last.span.end == part.span.start {
            last.span.end = part.span.end;
            return Ok(());
        }
    }
    if out.len() >= MAX_ITEMS * 2 {
        return Err("Source merge exceeds its fragment budget".into());
    }
    out.push(part);
    Ok(())
}
fn render(parts: &[Fragment], base: &str, private: &str, incoming: &str) -> Result<String, String> {
    let mut text = String::new();
    for part in parts {
        let piece = input(part.input, base, private, incoming)
            .get(part.span.clone())
            .ok_or("Merge fragment is outside its UTF-8 source")?;
        if piece.is_empty() || text.len() + piece.len() > MAX_DOCUMENT_BYTES {
            return Err("Merge fragment/output exceeds source bounds".into());
        }
        text.push_str(piece);
    }
    Ok(text)
}
struct Frame<'a> {
    text: &'a str,
    parts: Vec<Fragment>,
}
impl Frame<'_> {
    fn project(&self, range: Range<usize>) -> Result<Vec<Fragment>, String> {
        if self.text.get(range.clone()).is_none() {
            return Err("Merge projection is outside UTF-8 boundaries".into());
        }
        let mut out = vec![];
        let mut at = 0;
        for part in &self.parts {
            let end = at + part.span.len();
            let lo = range.start.max(at);
            let hi = range.end.min(end);
            if lo < hi {
                push(
                    &mut out,
                    Fragment {
                        input: part.input,
                        span: part.span.start + lo - at..part.span.start + hi - at,
                    },
                )?;
            }
            at = end;
        }
        if at != self.text.len() || out.iter().map(|p| p.span.len()).sum::<usize>() != range.len() {
            return Err("Merge projection does not cover the requested input".into());
        }
        Ok(out)
    }
}
struct Tokens {
    ids: Vec<u32>,
    // Every frame is capped at 1 MiB. Avoid pointer-width offsets for large
    // character refinements on 64-bit machines.
    offsets: Vec<u32>,
}
impl Tokens {
    fn bytes(&self, range: Range<usize>) -> Range<usize> {
        self.offsets[range.start] as usize..self.offsets[range.end] as usize
    }
}
fn tokens(
    text: &str,
    lines: bool,
    intern: &mut HashMap<String, u32>,
    budget: &mut Budget,
) -> Result<Tokens, String> {
    budget.work(text.len())?;
    let mut ids = vec![];
    let mut offsets = vec![0];
    if lines {
        let mut at = 0;
        for line in text.split_inclusive('\n') {
            let id = if let Some(id) = intern.get(line) {
                *id
            } else {
                let id = intern.len() as u32;
                intern.insert(line.into(), id);
                id
            };
            ids.push(id);
            at += line.len();
            offsets.push(at as u32);
        }
    } else {
        for (at, c) in text.char_indices() {
            ids.push(c as u32);
            offsets.push((at + c.len_utf8()) as u32);
        }
    }
    Ok(Tokens { ids, offsets })
}
fn intersects(a: &Range<usize>, b: &Range<usize>) -> bool {
    if a.is_empty() && b.is_empty() {
        a.start == b.start
    } else if a.is_empty() {
        b.start < a.start && a.start < b.end
    } else if b.is_empty() {
        a.start < b.start && b.start < a.end
    } else {
        a.start < b.end && b.start < a.end
    }
}
fn guard_anchors(
    a: &[u32],
    b: &[u32],
    script: &mut diff::Script,
    other: &mut diff::Script,
    budget: &mut Budget,
) -> Result<(), String> {
    let mut cache = BTreeMap::new();
    for hunk in &mut other.hunks {
        budget.work(script.hunks.len())?;
        if script.hunks.is_empty()
            || script
                .hunks
                .iter()
                .any(|h| intersects(&h.footprint, &hunk.footprint))
        {
            continue;
        }
        let positions = if hunk.old.is_empty() {
            hunk.old.start.saturating_sub(1)..(hunk.old.start + 1).min(a.len())
        } else {
            hunk.old.clone()
        };
        let mut uncertain = false;
        for old in positions {
            let Some(new) = script.matched(old) else {
                continue;
            };
            let alternate = if let Some(cached) = cache.get(&old) {
                *cached
            } else {
                budget.anchors += 1;
                if budget.anchors > MAX_ANCHORS {
                    return Err("Source merge exceeds its anchor-proof budget".into());
                }
                let result = diff::alternative(a, b, (old, new), script.distance, budget)?;
                cache.insert(old, result);
                result
            };
            if alternate {
                uncertain = true;
                break;
            }
        }
        if uncertain {
            budget.work(script.hunks.len() * 3)?;
            let lo = script
                .hunks
                .iter()
                .map(|h| h.footprint.start)
                .min()
                .unwrap()
                .min(hunk.footprint.start);
            let hi = script
                .hunks
                .iter()
                .map(|h| h.footprint.end)
                .max()
                .unwrap()
                .max(hunk.footprint.end);
            hunk.footprint = lo..hi;
            hunk.uncertain = true;
            for h in &mut script.hunks {
                h.footprint.start = h.footprint.start.min(lo);
                h.footprint.end = h.footprint.end.max(hi);
                h.uncertain = true;
            }
        }
    }
    Ok(())
}
fn equal_parts(
    frame: &Frame<'_>,
    tokens: &Tokens,
    script: &diff::Script,
    span: Range<usize>,
) -> Result<Vec<Fragment>, String> {
    let mut out = vec![];
    let mut at = span.start;
    while at < span.end {
        let i = script.equals.partition_point(|(old, _)| old.end <= at);
        let (old, new) = script
            .equals
            .get(i)
            .filter(|(old, _)| old.contains(&at))
            .ok_or("Merge gap has no unchanged source projection")?;
        let end = span.end.min(old.end);
        for part in
            frame.project(tokens.bytes(new.start + at - old.start..new.start + end - old.start))?
        {
            push(&mut out, part)?;
        }
        at = end;
    }
    Ok(out)
}
fn candidate(
    frame: &Frame<'_>,
    tokens: &Tokens,
    script: &diff::Script,
    span: Range<usize>,
    selected: &[usize],
) -> Result<Vec<Fragment>, String> {
    let mut out = vec![];
    let mut at = span.start;
    let mut selected = selected.to_vec();
    selected.sort_by_key(|i| script.hunks[*i].old.start);
    for i in selected {
        let h = &script.hunks[i];
        if h.old.start < at || h.old.end > span.end {
            return Err("Merge candidate has overlapping/outside edits".into());
        }
        for part in equal_parts(frame, tokens, script, at..h.old.start)? {
            push(&mut out, part)?;
        }
        for part in frame.project(tokens.bytes(h.new.clone()))? {
            push(&mut out, part)?;
        }
        at = h.old.end;
    }
    for part in equal_parts(frame, tokens, script, at..span.end)? {
        push(&mut out, part)?;
    }
    Ok(out)
}
fn add_region(out: &mut Vec<Region>, region: Region) -> Result<(), String> {
    if out.len() >= MAX_ITEMS {
        return Err("Source merge exceeds its region budget".into());
    }
    out.push(region);
    Ok(())
}
struct Group {
    span: Range<usize>,
    private: Vec<usize>,
    incoming: Vec<usize>,
    uncertain: bool,
}
fn stage(
    frames: [Frame<'_>; 3],
    original: [&str; 3],
    lines: bool,
    base_start: usize,
    budget: &mut Budget,
) -> Result<Vec<Region>, String> {
    let [base, private, incoming] = frames;
    let mut intern = HashMap::new();
    let bt = tokens(base.text, lines, &mut intern, budget)?;
    let pt = tokens(private.text, lines, &mut intern, budget)?;
    let it = tokens(incoming.text, lines, &mut intern, budget)?;
    drop(intern);
    let mut p = diff::script(&bt.ids, &pt.ids, budget)?;
    let mut i = diff::script(&bt.ids, &it.ids, budget)?;
    guard_anchors(&bt.ids, &pt.ids, &mut p, &mut i, budget)?;
    guard_anchors(&bt.ids, &it.ids, &mut i, &mut p, budget)?;
    let mut items = p
        .hunks
        .iter()
        .enumerate()
        .map(|(n, h)| (false, n, h.footprint.clone()))
        .chain(
            i.hunks
                .iter()
                .enumerate()
                .map(|(n, h)| (true, n, h.footprint.clone())),
        )
        .collect::<Vec<_>>();
    items.sort_by_key(|(side, index, span)| (span.start, span.end, *side, *index));
    let mut groups: Vec<Group> = vec![];
    for (side, n, span) in items {
        let uncertain = if side {
            i.hunks[n].uncertain
        } else {
            p.hunks[n].uncertain
        };
        if let Some(group) = groups.last_mut() {
            if intersects(&group.span, &span) {
                group.span.end = group.span.end.max(span.end);
                group.span.start = group.span.start.min(span.start);
                if side {
                    group.incoming.push(n);
                } else {
                    group.private.push(n);
                }
                group.uncertain |= uncertain;
                continue;
            }
        }
        groups.push(Group {
            span,
            private: if side { vec![] } else { vec![n] },
            incoming: if side { vec![n] } else { vec![] },
            uncertain,
        });
    }
    let mut out = vec![];
    let mut at = 0;
    for Group {
        span,
        private: ps,
        incoming: is,
        uncertain,
    } in groups
    {
        if at < span.start {
            let bytes = bt.bytes(at..span.start);
            let global = base_start + bytes.start..base_start + bytes.end;
            add_region(
                &mut out,
                Region {
                    base: global,
                    private: equal_parts(&private, &pt, &p, at..span.start)?,
                    incoming: equal_parts(&incoming, &it, &i, at..span.start)?,
                    automatic: Some(Input::Incoming),
                    conflict: None,
                },
            )?;
        }
        let bytes = bt.bytes(span.clone());
        let bp = base.project(bytes.clone())?;
        let global = base_start + bytes.start..base_start + bytes.end;
        let pp = candidate(&private, &pt, &p, span.clone(), &ps)?;
        let ip = candidate(&incoming, &it, &i, span.clone(), &is)?;
        let old = &base.text[bytes.clone()];
        let left = render(&pp, original[0], original[1], original[2])?;
        let right = render(&ip, original[0], original[1], original[2])?;
        if lines && !uncertain && left != right {
            let child = stage(
                [
                    Frame {
                        text: old,
                        parts: bp,
                    },
                    Frame {
                        text: &left,
                        parts: pp,
                    },
                    Frame {
                        text: &right,
                        parts: ip,
                    },
                ],
                original,
                false,
                global.start,
                budget,
            )?;
            for region in child {
                add_region(&mut out, region)?;
            }
        } else {
            let automatic = if left == right || left == old {
                Some(Input::Incoming)
            } else if right == old {
                Some(Input::Private)
            } else {
                None
            };
            add_region(
                &mut out,
                Region {
                    base: global,
                    private: pp,
                    incoming: ip,
                    automatic,
                    conflict: automatic.is_none().then_some(if uncertain {
                        Conflict::AmbiguousAlignment
                    } else if span.is_empty() {
                        Conflict::ConcurrentInsertions
                    } else {
                        Conflict::Overlap
                    }),
                },
            )?;
        }
        at = span.end;
    }
    if at < bt.ids.len() {
        let global = base_start + bt.bytes(at..bt.ids.len()).start
            ..base_start + bt.bytes(at..bt.ids.len()).end;
        add_region(
            &mut out,
            Region {
                base: global,
                private: equal_parts(&private, &pt, &p, at..bt.ids.len())?,
                incoming: equal_parts(&incoming, &it, &i, at..bt.ids.len())?,
                automatic: Some(Input::Incoming),
                conflict: None,
            },
        )?;
    }
    Ok(out)
}
fn pin(text: &str) -> Pin {
    Pin {
        bytes: text.len(),
        sha256: source_hash(text),
    }
}
fn checked(text: &str) -> Result<(), String> {
    if text.len() > MAX_DOCUMENT_BYTES || text.contains('\0') {
        Err("Source merge input exceeds its byte/text bounds".into())
    } else {
        Ok(())
    }
}
pub fn prepare(base: &str, private: &str, incoming: &str) -> Result<Review, String> {
    for text in [base, private, incoming] {
        checked(text)?;
    }
    let mut budget = Budget::new();
    let frames = [
        (Input::Base, base),
        (Input::Private, private),
        (Input::Incoming, incoming),
    ]
    .map(|(input, text)| Frame {
        text,
        parts: if text.is_empty() {
            vec![]
        } else {
            vec![Fragment {
                input,
                span: 0..text.len(),
            }]
        },
    });
    let regions = stage(frames, [base, private, incoming], true, 0, &mut budget)?;
    let review = Review {
        format: "hye-three-way-source-1".into(),
        schema_sha256: crate::workspace_graph::schema_sha256(),
        base: pin(base),
        private: pin(private),
        incoming: pin(incoming),
        regions,
        work: MAX_WORK - budget.remaining,
        peak_frontier_cells: budget.peak,
    };
    validate_review(base, private, incoming, &review)?;
    hash(&review)?;
    Ok(review)
}
/// Cheap input/provenance checks; this does not prove diff correctness or grant
/// source ownership. Each branch must reconstruct its complete input exactly.
pub fn validate_review(
    base: &str,
    private: &str,
    incoming: &str,
    review: &Review,
) -> Result<(), String> {
    for text in [base, private, incoming] {
        checked(text)?;
    }
    if review.format != "hye-three-way-source-1"
        || review.schema_sha256 != crate::workspace_graph::schema_sha256()
        || review.base != pin(base)
        || review.private != pin(private)
        || review.incoming != pin(incoming)
        || review.regions.len() > MAX_ITEMS
        || review.work > MAX_WORK
        || review.peak_frontier_cells > MAX_FRONTIER
    {
        return Err("Source merge identity or budget differs from its inputs".into());
    }
    let (mut at, mut left, mut right, mut fragments) = (0, 0, 0, 0);
    for region in &review.regions {
        if region.base.start != at
            || base.get(region.base.clone()).is_none()
            || region.automatic.is_some() == region.conflict.is_some()
            || region.automatic == Some(Input::Base)
        {
            return Err("Source merge regions do not cover the original revision".into());
        }
        at = region.base.end;
        for (parts, side, cursor) in [
            (&region.private, Input::Private, &mut left),
            (&region.incoming, Input::Incoming, &mut right),
        ] {
            fragments += parts.len();
            if fragments > MAX_ITEMS * 4 {
                return Err("Source merge review exceeds its fragment budget".into());
            }
            for part in parts {
                if part.input != side
                    || part.span.start != *cursor
                    || part.span.is_empty()
                    || input(side, base, private, incoming)
                        .get(part.span.clone())
                        .is_none()
                {
                    return Err(
                        "Source merge branch does not reconstruct its exact revision".into(),
                    );
                }
                *cursor = part.span.end;
            }
        }
    }
    if at != base.len() || left != private.len() || right != incoming.len() {
        return Err("Source merge review leaves input bytes unaccounted for".into());
    }
    Ok(())
}
fn selected_fragments(
    review: &Review,
    choices: &BTreeMap<usize, Choice>,
) -> Result<Vec<Fragment>, String> {
    if choices.keys().any(|id| {
        review
            .regions
            .get(*id)
            .is_none_or(|r| r.automatic.is_some())
    }) {
        return Err("A merge choice names an unknown or automatically resolved region".into());
    }
    let mut fragments = vec![];
    for (id, region) in review.regions.iter().enumerate() {
        let choice = match region.automatic {
            Some(Input::Incoming) => Choice::Incoming,
            Some(Input::Private) => Choice::Private,
            Some(Input::Base) => Choice::Base,
            None => *choices
                .get(&id)
                .ok_or("Every source conflict requires an explicit choice")?,
        };
        let parts = match choice {
            Choice::Private => region.private.clone(),
            Choice::Incoming => region.incoming.clone(),
            Choice::Base => {
                if region.base.is_empty() {
                    vec![]
                } else {
                    vec![Fragment {
                        input: Input::Base,
                        span: region.base.clone(),
                    }]
                }
            }
            Choice::PrivateThenIncoming | Choice::IncomingThenPrivate => {
                if !region.base.is_empty() {
                    return Err("Combining both branches is offered only for concurrent insertions at one boundary".into());
                }
                let (a, b) = if choice == Choice::PrivateThenIncoming {
                    (&region.private, &region.incoming)
                } else {
                    (&region.incoming, &region.private)
                };
                a.iter().chain(b.iter()).cloned().collect()
            }
        };
        for part in parts {
            push(&mut fragments, part)?;
        }
    }
    Ok(fragments)
}
/// Independently check every selected byte range before a caller rebuilds syntax
/// and dependency evidence. A text merge never certifies those later stages.
pub fn validate_resolution(
    base: &str,
    private: &str,
    incoming: &str,
    review: &Review,
    choices: &BTreeMap<usize, Choice>,
    result: &Resolution,
) -> Result<(), String> {
    validate_review(base, private, incoming, review)?;
    if result.syntax_verified
        || result.ownership_verified
        || result.runtime_verified
        || result.review_sha256 != hash(review)?
        || result.choices_sha256 != hash(choices)?
        || result.fragments != selected_fragments(review, choices)?
        || result.content != render(&result.fragments, base, private, incoming)?
        || result.sha256 != source_hash(&result.content)
    {
        return Err("Source merge result differs from its reviewed choices and input bytes".into());
    }
    Ok(())
}
pub fn resolve(
    base: &str,
    private: &str,
    incoming: &str,
    review: &Review,
    choices: &BTreeMap<usize, Choice>,
) -> Result<Resolution, String> {
    let fresh = prepare(base, private, incoming)?;
    if *review != fresh {
        return Err(
            "Source merge review changed; rebuild it before selecting conflict choices".into(),
        );
    }
    let fragments = selected_fragments(review, choices)?;
    let content = render(&fragments, base, private, incoming)?;
    Ok(Resolution {
        review_sha256: hash(review)?,
        choices_sha256: hash(choices)?,
        sha256: source_hash(&content),
        content,
        fragments,
        syntax_verified: false,
        ownership_verified: false,
        runtime_verified: false,
    })
}
#[cfg(test)]
mod tests;
