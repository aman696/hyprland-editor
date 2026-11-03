//! Explicit captured data and resolution inputs for a private source revision.
//! Paths remain historical data; this layer has no filesystem authority.
use super::{revisions, View};
use crate::{
    lua::replay::{self, edits, Binding, Document, LoadOutcome, Program, Report, Resolution},
    workspace::{SourceLanguage, SourceSnapshot},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    pub sources: Arc<Vec<SourceSnapshot>>,
    pub views: Vec<View>,
    /// Requests refer to the final private bytes and complete observed context.
    pub bindings: Vec<Binding>,
    /// Preserved private data omitted from evaluation until explicitly reused.
    pub archived: BTreeSet<String>,
}
impl Inputs {
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
            && self.views.is_empty()
            && self.bindings.is_empty()
            && self.archived.is_empty()
    }
    /// Original data for piece construction, including archived private data.
    /// This is not the program to evaluate: explicit bindings attach only after
    /// revisions are materialized and archived documents are removed.
    pub fn base(&self, original: &Program) -> Result<Program, String> {
        replay::validate_program(original)?;
        if self.sources.len() != self.views.len() || self.bindings.len() > 128 {
            return Err("Invalid private input/view/binding limits".into());
        }
        let mut base = original.clone();
        let mut ids = BTreeSet::new();
        for view in &self.views {
            if !ids.insert(view.document.as_str()) {
                return Err("Duplicate private source view".into());
            }
            let source = self
                .sources
                .iter()
                .find(|s| s.id == view.document)
                .ok_or("Private view source missing")?;
            if source.language != SourceLanguage::Lua
                || source.id.is_empty()
                || source.id.len() > 64
                || !source
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err("Invalid private captured source identity or language".into());
            }
            source.origin.validate(&source.content)?;
            base.documents.push(Document {
                id: source.id.clone(),
                source: source.content.clone(),
                sha256: source.origin.sha256.clone(),
                load_name: view.load_name.clone(),
            });
        }
        if self
            .sources
            .iter()
            .map(|s| &s.id)
            .collect::<BTreeSet<_>>()
            .len()
            != self.sources.len()
            || self.archived.iter().any(|id| !ids.contains(id.as_str()))
        {
            return Err("Unknown archived source or duplicate private input".into());
        }
        replay::validate_program(&base)?;
        Ok(base)
    }
    pub fn validate(
        &self,
        original: &Program,
        pieces: &[revisions::Revision],
    ) -> Result<(), String> {
        if self.is_empty() {
            revisions::validate(original, pieces)
        } else {
            self.program(original, pieces).map(|_| ())
        }
    }
    /// Pure materialization. Partial changes to inherited loader ranges DROP
    /// their binding in this explicit layer; they never inherit its resolution.
    /// Fresh replay and check_reached are mandatory before retention/reopening.
    pub fn program(
        &self,
        original: &Program,
        pieces: &[revisions::Revision],
    ) -> Result<Program, String> {
        if self.is_empty() {
            return revisions::program(original.clone(), pieces);
        }
        self.pending_program(original, pieces)
    }
    /// Inspect candidate private bytes before the first new input is supplied.
    /// Changed inherited requests lose their binding. This query is not saved
    /// acceptance; normal program/reached checks still apply after resolution.
    pub fn pending_program(
        &self,
        original: &Program,
        pieces: &[revisions::Revision],
    ) -> Result<Program, String> {
        let mut program = revisions::for_renewal(self.base(original)?, pieces)?;
        for binding in &self.bindings {
            if self.archived.contains(&binding.request.document)
                || matches!(&binding.resolution, Resolution::Loaded {document,..} if self.archived.contains(document))
            {
                return Err("An archived private source cannot supply an active resolution".into());
            }
        }
        program.documents.retain(|d| !self.archived.contains(&d.id));
        program
            .bindings
            .retain(|b| !self.bindings.iter().any(|p| p.request == b.request));
        program.bindings.extend(self.bindings.iter().cloned());
        replay::validate_program(&program)?;
        Ok(program)
    }
    /// Check a report freshly rebuilt from program, never a saved report.
    pub fn check_reached(&self, program: &Program, report: &Report) -> Result<(), String> {
        if report.runtime_verified
            || report.version != program.version
            || report.profile != program.profile
            || report.compiler_profile != replay::compiler_profile()
        {
            return Err("Private input replay context differs".into());
        }
        for binding in &self.bindings {
            if !report.loads.iter().any(|l| {
                l.request == binding.request
                    && match (&l.outcome, &binding.resolution) {
                        (
                            LoadOutcome::SuppliedSnapshot {
                                document: a,
                                loader_data: x,
                            },
                            Resolution::Loaded {
                                document: b,
                                loader_data: y,
                            },
                        ) => a == b && x == y,
                        (LoadOutcome::SuppliedMissing, Resolution::Missing) => true,
                        _ => false,
                    }
            }) {
                return Err(
                    "Private graph contains an unreachable or inconsistent resolution input".into(),
                );
            }
        }
        for source in self
            .sources
            .iter()
            .filter(|s| !self.archived.contains(&s.id))
        {
            if !report.loads.iter().any(|l| matches!(&l.outcome, LoadOutcome::SuppliedSnapshot {document,..} if document==&source.id)) {
                return Err("Private graph contains an active source no loader reaches; archive it explicitly".into());
            }
        }
        Ok(())
    }
    /// Keep explicit inputs through a reviewed source edit. Surviving requests
    /// rebase only across non-overlapping patches; changed requests need renewal.
    pub fn apply(
        &self,
        original: &Program,
        pieces: &[revisions::Revision],
        plan: &edits::EditPlan,
    ) -> Result<(Vec<revisions::Revision>, Self), String> {
        if self.is_empty() {
            return revisions::prepare_pieces(original, pieces, plan).map(|p| (p, self.clone()));
        }
        let base = self.base(original)?;
        let current = self.program(original, pieces)?;
        let next = revisions::prepare_from_current(&base, pieces, &current, plan)?;
        let proposed = revisions::for_renewal(base, &next)?;
        let mut inputs = self.clone();
        inputs.bindings.clear();
        for binding in &self.bindings {
            match edits::rebase_request(&proposed, plan, &binding.request) {
                Ok(request) => inputs.bindings.push(Binding {
                    request,
                    resolution: binding.resolution.clone(),
                }),
                Err(_) if edits::removed_loader(plan, &binding.request) => {}
                Err(error) => return Err(error),
            }
        }
        // Retain no longer reachable private captures explicitly as archives.
        // Reachability here is conservative supplied-input structure only;
        // fresh replay still refuses bindings hidden by changed control flow.
        let mut effective = inputs.program(original, &next)?;
        edits::prune_unreachable_bindings(&mut effective);
        inputs
            .bindings
            .retain(|b| effective.bindings.iter().any(|e| e.request == b.request));
        let mut reached = BTreeSet::from([effective.root.clone()]);
        loop {
            let count = reached.len();
            for b in &effective.bindings {
                if reached.contains(&b.request.document) {
                    if let Resolution::Loaded { document, .. } = &b.resolution {
                        reached.insert(document.clone());
                    }
                }
            }
            if reached.len() == count {
                break;
            }
        }
        inputs.archived.extend(
            inputs
                .sources
                .iter()
                .filter(|s| !reached.contains(&s.id))
                .map(|s| s.id.clone()),
        );
        inputs.program(original, &next)?;
        Ok((next, inputs))
    }
}
