//! Private piece-based revisions. Captured source bytes and identities are never rewritten.
use crate::{
    lua::{
        replay::{self, edits, Program},
        source_hash,
    },
    MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, ops::Range, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Buffer {
    Original,
    Inserted { index: usize },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Piece {
    pub buffer: Buffer,
    pub span: Range<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub document: String,
    pub base_sha256: String,
    pub sha256: String,
    pub buffers: Vec<Arc<str>>,
    pub pieces: Vec<Piece>,
}
fn bytes<'a>(base: &'a str, revision: &'a Revision, piece: &Piece) -> Result<&'a str, String> {
    let buffer = match piece.buffer {
        Buffer::Original => base,
        Buffer::Inserted { index } => revision
            .buffers
            .get(index)
            .ok_or("Unknown inserted source buffer")?,
    };
    buffer
        .get(piece.span.clone())
        .filter(|_| !piece.span.is_empty())
        .ok_or_else(|| "Invalid private source piece boundary".into())
}
pub fn validate(base: &Program, revisions: &[Revision]) -> Result<(), String> {
    replay::validate_program(base)?;
    if revisions.len() > base.documents.len() {
        return Err("Too many private source revisions".into());
    }
    let mut ids = BTreeSet::new();
    let mut count = 0usize;
    let mut inserted = 0usize;
    for revision in revisions {
        let doc = base
            .documents
            .iter()
            .find(|d| d.id == revision.document)
            .ok_or("Revision names an unknown logical document")?;
        count = count.saturating_add(revision.pieces.len());
        if !ids.insert(revision.document.as_str())
            || revision.base_sha256 != doc.sha256
            || revision.buffers.len() > 8192
            || count > 8192
        {
            return Err("Invalid private source revision identity or piece budget".into());
        }
        let mut unique = BTreeSet::new();
        for buffer in &revision.buffers {
            inserted = inserted.saturating_add(buffer.len());
            if buffer.is_empty()
                || buffer.len() > MAX_DOCUMENT_BYTES
                || buffer.contains('\0')
                || !unique.insert(buffer.as_ref())
                || inserted > 4 * MAX_DOCUMENT_BYTES
            {
                return Err("Inserted source buffers exceed their identity/byte budget".into());
            }
        }
        let mut used = BTreeSet::new();
        let mut last_original = 0usize;
        let mut size = 0usize;
        let mut hash = Sha256::new();
        let mut previous: Option<&Piece> = None;
        for piece in &revision.pieces {
            if let Some(previous) = previous {
                if previous.buffer == piece.buffer && previous.span.end == piece.span.start {
                    return Err("Adjacent source pieces must be coalesced".into());
                }
            }
            let text = bytes(&doc.source, revision, piece)?;
            size = size.saturating_add(text.len());
            if size > MAX_DOCUMENT_BYTES {
                return Err("Private source document exceeds 1 MiB".into());
            }
            match piece.buffer {
                Buffer::Original => {
                    if piece.span.start < last_original {
                        return Err(
                            "Captured source pieces cannot be duplicated or reordered".into()
                        );
                    }
                    last_original = piece.span.end;
                }
                Buffer::Inserted { index } => {
                    used.insert(index);
                }
            }
            hash.update(text.as_bytes());
            previous = Some(piece);
        }
        if used.len() != revision.buffers.len()
            || revision.sha256 != format!("{:x}", hash.finalize())
        {
            return Err("Private source hash or inserted-buffer reachability disagrees".into());
        }
    }
    Ok(())
}
fn original_span(revision: &Revision, span: &Range<usize>) -> Result<Range<usize>, String> {
    let mut offset = 0usize;
    for piece in &revision.pieces {
        if piece.buffer == Buffer::Original
            && piece.span.start <= span.start
            && piece.span.end >= span.end
        {
            return Ok(offset + span.start - piece.span.start..offset + span.end - piece.span.start);
        }
        offset = offset
            .checked_add(piece.span.len())
            .ok_or("Private source offset overflow")?;
    }
    Err("A source revision changes a loader request; renewed resolution input is required".into())
}
/// Structural materialization only: no code execution, parsing or file access.
pub fn program(mut base: Program, revisions: &[Revision]) -> Result<Program, String> {
    validate(&base, revisions)?;
    for binding in &mut base.bindings {
        if let Some(revision) = revisions
            .iter()
            .find(|r| r.document == binding.request.document)
        {
            binding.request.span = original_span(revision, &binding.request.span)?;
            binding.request.sha256 = revision.sha256.clone();
        }
    }
    for doc in &mut base.documents {
        let Some(revision) = revisions.iter().find(|r| r.document == doc.id) else {
            continue;
        };
        let mut content = String::with_capacity(revision.pieces.iter().map(|p| p.span.len()).sum());
        for piece in &revision.pieces {
            content.push_str(bytes(&doc.source, revision, piece)?);
        }
        doc.source = content;
        doc.sha256 = revision.sha256.clone();
    }
    replay::validate_program(&base)?;
    Ok(base)
}
fn slice(pieces: &[Piece], span: Range<usize>) -> Vec<Piece> {
    let mut offset = 0usize;
    let mut out = vec![];
    for piece in pieces {
        let next = offset + piece.span.len();
        let start = span.start.max(offset);
        let end = span.end.min(next);
        if start < end {
            out.push(Piece {
                buffer: piece.buffer.clone(),
                span: piece.span.start + start - offset..piece.span.start + end - offset,
            });
        }
        offset = next;
        if offset >= span.end {
            break;
        }
    }
    out
}
fn coalesce(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = vec![];
    for piece in pieces {
        if piece.span.is_empty() {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.buffer == piece.buffer && last.span.end == piece.span.start {
                last.span.end = piece.span.end;
                continue;
            }
        }
        out.push(piece);
    }
    out
}
fn compact(revision: &mut Revision) {
    let mut buffers: Vec<Arc<str>> = vec![];
    let mut remap = vec![None; revision.buffers.len()];
    for piece in &mut revision.pieces {
        let Buffer::Inserted { index } = piece.buffer else {
            continue;
        };
        let target = if let Some(target) = remap[index] {
            target
        } else {
            let buffer = &revision.buffers[index];
            let target = buffers
                .iter()
                .position(|b| b.as_ref() == buffer.as_ref())
                .unwrap_or_else(|| {
                    buffers.push(buffer.clone());
                    buffers.len() - 1
                });
            remap[index] = Some(target);
            target
        };
        piece.buffer = Buffer::Inserted { index: target };
    }
    revision.buffers = buffers;
    revision.pieces = coalesce(std::mem::take(&mut revision.pieces));
}
/// Apply a validated plan to private pieces; no comparison or ownership certificate is minted.
pub fn prepare_pieces(
    base: &Program,
    revisions: &[Revision],
    plan: &edits::EditPlan,
) -> Result<Vec<Revision>, String> {
    let current = program(base.clone(), revisions)?;
    edits::validate_plan(&current, plan)?;
    let mut result = revisions.to_vec();
    for doc in &current.documents {
        let changes = plan
            .edits
            .iter()
            .filter(|e| e.document == doc.id)
            .collect::<Vec<_>>();
        if changes.is_empty() {
            continue;
        }
        let original = base
            .documents
            .iter()
            .find(|d| d.id == doc.id)
            .ok_or("Missing original source")?;
        let growth = changes
            .iter()
            .map(|e| e.replacement.len() as isize - e.span.len() as isize)
            .sum::<isize>();
        if doc
            .source
            .len()
            .checked_add_signed(growth)
            .is_none_or(|n| n > MAX_DOCUMENT_BYTES)
        {
            return Err("Private source document exceeds 1 MiB".into());
        }
        let mut revision = result
            .iter()
            .find(|r| r.document == doc.id)
            .cloned()
            .unwrap_or_else(|| Revision {
                document: doc.id.clone(),
                base_sha256: original.sha256.clone(),
                sha256: original.sha256.clone(),
                buffers: vec![],
                pieces: if original.source.is_empty() {
                    vec![]
                } else {
                    vec![Piece {
                        buffer: Buffer::Original,
                        span: 0..original.source.len(),
                    }]
                },
            });
        let mut size = doc.source.len();
        for edit in changes.into_iter().rev() {
            let mut pieces = slice(&revision.pieces, 0..edit.span.start);
            if !edit.replacement.is_empty() {
                let index = revision
                    .buffers
                    .iter()
                    .position(|b| b.as_ref() == edit.replacement)
                    .unwrap_or_else(|| {
                        revision.buffers.push(Arc::from(edit.replacement.as_str()));
                        revision.buffers.len() - 1
                    });
                pieces.push(Piece {
                    buffer: Buffer::Inserted { index },
                    span: 0..edit.replacement.len(),
                });
            }
            pieces.extend(slice(&revision.pieces, edit.span.end..size));
            revision.pieces = coalesce(pieces);
            size = size - edit.span.len() + edit.replacement.len();
        }
        compact(&mut revision);
        let mut content = String::with_capacity(size);
        for piece in &revision.pieces {
            content.push_str(bytes(&original.source, &revision, piece)?);
        }
        revision.sha256 = source_hash(&content);
        result.retain(|r| r.document != doc.id);
        if content != original.source {
            result.push(revision);
        }
    }
    result.sort_by(|a, b| a.document.cmp(&b.document));
    program(base.clone(), &result)?;
    Ok(result)
}
pub fn apply(
    base: &Program,
    revisions: &[Revision],
    plan: &edits::EditPlan,
) -> Result<Vec<Revision>, String> {
    let result = prepare_pieces(base, revisions, plan)?;
    let proposed = program(base.clone(), &result)?;
    for revision in &result {
        let doc = proposed
            .documents
            .iter()
            .find(|d| d.id == revision.document)
            .ok_or("Missing proposed source")?;
        replay::validate_document(doc)?;
    }
    Ok(result)
}
