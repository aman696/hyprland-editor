//! Structural correspondence of two captured graphs. Historical paths suggest
//! logical views; they never authorize reads, source ownership or execution.
use super::{revisions, Bundle};
use crate::{lua::replay::Program, source_merge, workspace::SourceSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
mod pieces;
pub use pieces::{compose, Composed, Decision};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub document: String,
    pub selected_path: Vec<u8>,
    pub load_name: Vec<u8>,
    pub captured_sha256: String,
    pub private_sha256: String,
    pub private_changed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub old: String,
    pub candidates: Vec<String>,
    pub recommended: Option<String>,
    pub root_pair: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub schema_sha256: String,
    pub version: String,
    pub old_fingerprint: String,
    pub incoming_fingerprint: String,
    pub context_changed: bool,
    pub old_views: Vec<View>,
    pub incoming_views: Vec<View>,
    pub rows: Vec<Row>,
    pub syntax_verified: bool,
    pub dependency_replay_verified: bool,
    pub ownership_verified: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    /// Every old view needs an explicit mapped counterpart or removal choice.
    pub pairs: BTreeMap<String, Option<String>>,
    /// Required only for removed views with private byte changes.
    pub discard_private: BTreeSet<String>,
    pub accept_context_change: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileInputs {
    pub review_sha256: String,
    pub selection_sha256: String,
    pub old: String,
    pub incoming_document: String,
    pub base: String,
    pub private: String,
    pub incoming: String,
}
fn hash(value: &impl Serialize) -> Result<String, String> {
    super::encoded(value).map(|v| v.0)
}
fn program(bundle: &Bundle, version: &str) -> Result<Program, String> {
    bundle
        .graph
        .program(&bundle.sources, Some(&bundle.graph.root), version)
}
fn views(bundle: &Bundle, program: &Program, current: &Program) -> Result<Vec<View>, String> {
    let mut result = vec![];
    for doc in &program.documents {
        let origin = &bundle
            .sources
            .iter()
            .find(|s| s.id == doc.id)
            .ok_or("Missing captured graph source")?
            .origin;
        let private = current
            .documents
            .iter()
            .find(|d| d.id == doc.id)
            .ok_or("Missing private graph view")?;
        result.push(View {
            document: doc.id.clone(),
            selected_path: origin.selected_path.clone(),
            load_name: doc.load_name.clone(),
            captured_sha256: doc.sha256.clone(),
            private_sha256: private.sha256.clone(),
            private_changed: doc.sha256 != private.sha256,
        });
    }
    Ok(result)
}
pub fn prepare(old: &Bundle, incoming: &Bundle, version: &str) -> Result<Review, String> {
    if !old.graph.private.is_empty() || !incoming.graph.private.is_empty() {
        return Err("Private dependency data needs explicit graph correspondence and preservation choices before external reconciliation".into());
    }
    let old_program = program(old, version)?;
    let incoming_program = program(incoming, version)?;
    if !incoming.graph.revisions.is_empty() {
        return Err(
            "Incoming graph must describe fresh captured bytes, not another private piece layer"
                .into(),
        );
    }
    if old.graph.compiler != incoming.graph.compiler {
        return Err("Captured graph compiler assumptions differ".into());
    }
    let private = revisions::program(old_program.clone(), &old.graph.revisions)?;
    let old_views = views(old, &old_program, &private)?;
    let incoming_views = views(incoming, &incoming_program, &incoming_program)?;
    let mut rows = vec![];
    for view in &old_views {
        let root_pair = view.document == old.graph.root;
        let candidates = if root_pair {
            vec![incoming.graph.root.clone()]
        } else {
            incoming_views
                .iter()
                .filter(|v| {
                    v.document != incoming.graph.root
                        && v.selected_path == view.selected_path
                        && v.load_name == view.load_name
                })
                .map(|v| v.document.clone())
                .collect()
        };
        rows.push(Row {
            old: view.document.clone(),
            candidates,
            recommended: None,
            root_pair,
        });
    }
    let mut counts = BTreeMap::new();
    for row in &rows {
        for candidate in &row.candidates {
            *counts.entry(candidate.clone()).or_insert(0) += 1;
        }
    }
    for row in &mut rows {
        if row.candidates.len() == 1 && counts[&row.candidates[0]] == 1 {
            row.recommended = Some(row.candidates[0].clone());
        }
    }
    let context_changed = hash(&old.graph.context)? != hash(&incoming.graph.context)?
        || old.graph.working_directory != incoming.graph.working_directory;
    let review = Review {
        schema_sha256: super::schema_sha256(),
        version: version.into(),
        old_fingerprint: old.fingerprint()?,
        incoming_fingerprint: incoming.fingerprint()?,
        context_changed,
        old_views,
        incoming_views,
        rows,
        syntax_verified: false,
        dependency_replay_verified: false,
        ownership_verified: false,
    };
    hash(&review)?;
    Ok(review)
}
pub fn validate_selection(review: &Review, selection: &Selection) -> Result<(), String> {
    if review.syntax_verified
        || review.dependency_replay_verified
        || review.ownership_verified
        || review.schema_sha256 != super::schema_sha256()
    {
        return Err(
            "Graph correspondence cannot claim syntax, dependency replay or ownership verification"
                .into(),
        );
    }
    let old = review
        .old_views
        .iter()
        .map(|v| v.document.as_str())
        .collect::<BTreeSet<_>>();
    if selection.pairs.len() != old.len()
        || selection.pairs.keys().any(|id| !old.contains(id.as_str()))
    {
        return Err("Choose a counterpart or removal for every old captured view".into());
    }
    let mut used = BTreeSet::new();
    let mut discarded = BTreeSet::new();
    for row in &review.rows {
        let mapped = selection
            .pairs
            .get(&row.old)
            .ok_or("Missing graph view choice")?;
        if let Some(id) = mapped {
            if !review.incoming_views.iter().any(|v| v.document == *id) || !used.insert(id) {
                return Err("A graph counterpart is unknown or assigned more than once".into());
            }
            if row.root_pair && row.candidates.first() != Some(id)
                || !row.root_pair
                    && review
                        .rows
                        .iter()
                        .any(|r| r.root_pair && r.candidates.first() == Some(id))
            {
                return Err("The explicitly selected graph roots must remain paired".into());
            }
        } else {
            if row.root_pair {
                return Err("Cannot remove the selected graph root".into());
            }
            if review
                .old_views
                .iter()
                .any(|v| v.document == row.old && v.private_changed)
            {
                discarded.insert(row.old.clone());
            }
        }
    }
    if discarded != selection.discard_private {
        return Err("Removed private changes require their exact explicit discard choices".into());
    }
    if review.context_changed && !selection.accept_context_change {
        return Err(
            "Review the changed root/home/search-path/working-folder context before merging".into(),
        );
    }
    Ok(())
}
fn source<'a>(sources: &'a [SourceSnapshot], id: &str) -> Result<&'a SourceSnapshot, String> {
    sources
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "Selected graph source no longer exists".into())
}
/// Bind one lazy text-merge job to both complete graph identities and the whole
/// explicit correspondence. This does not rebuild or accept the merged graph.
pub fn file_inputs(
    old: &Bundle,
    incoming: &Bundle,
    review: &Review,
    selection: &Selection,
    document: &str,
) -> Result<FileInputs, String> {
    let fresh = prepare(old, incoming, &review.version)?;
    if hash(&fresh)? != hash(review)? {
        return Err("Captured graph correspondence changed; rebuild the review".into());
    }
    validate_selection(review, selection)?;
    let mapped = selection
        .pairs
        .get(document)
        .and_then(Option::as_ref)
        .ok_or("Choose a mapped captured view for text reconciliation")?;
    let base = program(old, &review.version)?;
    let private = revisions::program(base.clone(), &old.graph.revisions)?;
    Ok(FileInputs {
        review_sha256: hash(review)?,
        selection_sha256: hash(selection)?,
        old: document.into(),
        incoming_document: mapped.clone(),
        base: source(&old.sources, document)?.content.clone(),
        private: private
            .documents
            .iter()
            .find(|d| d.id == document)
            .ok_or("Missing private source view")?
            .source
            .clone(),
        incoming: source(&incoming.sources, mapped)?.content.clone(),
    })
}
impl FileInputs {
    pub fn text_review(&self) -> Result<source_merge::Review, String> {
        source_merge::prepare(&self.base, &self.private, &self.incoming)
    }
}
