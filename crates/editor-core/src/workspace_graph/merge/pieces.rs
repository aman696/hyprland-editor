//! Convert explicit text choices to a private layer on the incoming captures.
//! Equal bytes from a private/base choice never gain incoming loader provenance.
use super::*;
use crate::lua;
use crate::workspace_graph::revisions::{Buffer, Piece, Revision};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub review: source_merge::Review,
    pub choices: BTreeMap<usize, source_merge::Choice>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Composed {
    pub bundle: Bundle,
    pub review_sha256: String,
    pub selection_sha256: String,
    pub decisions_sha256: String,
    pub fingerprint: String,
    /// A surviving captured request was changed by the private pieces. Fresh
    /// resolution inputs are needed before this candidate can be materialized.
    pub dependency_renewal: Option<String>,
    pub syntax_verified: bool,
    pub dependency_replay_verified: bool,
    pub ownership_verified: bool,
}
fn revision(
    document: &str,
    incoming: &str,
    result: source_merge::Resolution,
    base: &str,
    private: &str,
) -> Result<Option<Revision>, String> {
    let mut buffers: Vec<Arc<str>> = vec![];
    let mut pieces: Vec<Piece> = vec![];
    for fragment in result.fragments {
        let piece = match fragment.input {
            source_merge::Input::Incoming => Piece {
                buffer: Buffer::Original,
                span: fragment.span,
            },
            side => {
                let input = match side {
                    source_merge::Input::Base => base,
                    source_merge::Input::Private => private,
                    source_merge::Input::Incoming => unreachable!(),
                };
                let bytes = input
                    .get(fragment.span)
                    .ok_or("Invalid selected merge fragment")?;
                let index = buffers
                    .iter()
                    .position(|b| b.as_ref() == bytes)
                    .unwrap_or_else(|| {
                        buffers.push(Arc::from(bytes));
                        buffers.len() - 1
                    });
                Piece {
                    buffer: Buffer::Inserted { index },
                    span: 0..bytes.len(),
                }
            }
        };
        if let Some(previous) = pieces.last_mut() {
            if previous.buffer == piece.buffer && previous.span.end == piece.span.start {
                previous.span.end = piece.span.end;
                continue;
            }
        }
        pieces.push(piece);
    }
    // A byte-equal private choice is still private. Only full original-range
    // provenance permits dropping the revision, including the empty document.
    if buffers.is_empty()
        && (incoming.is_empty() && pieces.is_empty()
            || pieces.len() == 1
                && pieces[0].buffer == Buffer::Original
                && pieces[0].span == (0..incoming.len()))
    {
        return Ok(None);
    }
    Ok(Some(Revision {
        document: document.into(),
        base_sha256: lua::source_hash(incoming),
        sha256: result.sha256,
        buffers,
        pieces,
    }))
}
/// Recompute every mapped file's exact text review and explicit choices, then
/// compose private pieces onto the immutable incoming baseline. This performs
/// no parsing, resolution, execution, file access or application-state mutation.
/// A fresh whole-graph reconstruction (including renewal) is still required.
pub fn compose(
    old: &Bundle,
    incoming: &Bundle,
    review: &Review,
    selection: &Selection,
    decisions: &BTreeMap<String, Decision>,
) -> Result<Composed, String> {
    if hash(&prepare(old, incoming, &review.version)?)? != hash(review)? {
        return Err("Captured graph correspondence changed; rebuild the review".into());
    }
    validate_selection(review, selection)?;
    let review_sha256 = hash(review)?;
    let selection_sha256 = hash(selection)?;
    let decisions_sha256 = hash(decisions)?;
    let mapped = selection
        .pairs
        .iter()
        .filter_map(|(old, new)| new.as_ref().map(|new| (old, new)))
        .collect::<Vec<_>>();
    if decisions.len() != mapped.len() || mapped.iter().any(|(id, _)| !decisions.contains_key(*id))
    {
        return Err("Supply an exact text decision for every mapped captured view".into());
    }
    let original = program(old, &review.version)?;
    let private = revisions::program(original, &old.graph.revisions)?;
    let mut bundle = incoming.clone();
    let (mut inserted, mut pieces) = (0usize, 0usize);
    for (id, target) in mapped {
        let base = &source(&old.sources, id)?.content;
        let private = &private
            .documents
            .iter()
            .find(|d| d.id == *id)
            .ok_or("Missing private graph view")?
            .source;
        let incoming = &source(&incoming.sources, target)?.content;
        let decision = &decisions[id];
        let result =
            source_merge::resolve(base, private, incoming, &decision.review, &decision.choices)?;
        if let Some(revision) = revision(target, incoming, result, base, private)? {
            inserted =
                inserted.saturating_add(revision.buffers.iter().map(|b| b.len()).sum::<usize>());
            pieces = pieces.saturating_add(revision.pieces.len());
            if inserted > 4 * crate::MAX_DOCUMENT_BYTES || pieces > 8192 {
                return Err(
                    "Merged private layer exceeds its inserted-byte or piece budget".into(),
                );
            }
            bundle.graph.revisions.push(revision);
        }
    }
    let base = bundle
        .graph
        .program(&bundle.sources, Some(&bundle.graph.root), &review.version)?;
    let dependency_renewal = revisions::program(base, &bundle.graph.revisions).err();
    let fingerprint = bundle.fingerprint()?;
    Ok(Composed {
        bundle,
        review_sha256,
        selection_sha256,
        decisions_sha256,
        fingerprint,
        dependency_renewal,
        syntax_verified: false,
        dependency_replay_verified: false,
        ownership_verified: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn byte_equality_does_not_turn_private_choice_into_captured_provenance() {
        let text = "require('part')";
        let result = |input| source_merge::Resolution {
            review_sha256: String::new(),
            choices_sha256: String::new(),
            sha256: lua::source_hash(text),
            content: text.into(),
            fragments: vec![source_merge::Fragment {
                input,
                span: 0..text.len(),
            }],
            syntax_verified: false,
            ownership_verified: false,
            runtime_verified: false,
        };
        for input in [source_merge::Input::Private, source_merge::Input::Base] {
            let r = revision("root", text, result(input), text, text)
                .unwrap()
                .unwrap();
            assert_eq!(r.sha256, r.base_sha256);
            assert_eq!(r.buffers[0].as_ref(), text);
            assert!(r
                .pieces
                .iter()
                .all(|p| matches!(p.buffer, Buffer::Inserted { .. })));
        }
        assert!(revision(
            "root",
            text,
            result(source_merge::Input::Incoming),
            text,
            text
        )
        .unwrap()
        .is_none());
    }
}
