//! Three-way reconciliation of static declarations, never effective compositor state.
use crate::{
    lua::{source_hash, LuaDocument},
    Draft, Setting, Value,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub enum Observation {
    Absent,
    Known(Value),
    /// A syntactic literal may be visible even when the projection is incomplete.
    Unresolved(Option<Value>),
}
impl Observation {
    pub fn can_use(&self) -> bool {
        !matches!(self, Self::Unresolved(None))
    }
    fn selected(&self) -> Result<Option<Value>, String> {
        match self {
            Self::Absent => Ok(None),
            Self::Known(v) | Self::Unresolved(Some(v)) => Ok(Some(v.clone())),
            Self::Unresolved(None) => Err("Unresolved source is not an absent setting; retain the draft or edit it explicitly".into()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    KeepDraft,
    UseSelectedFile,
}
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub base: Observation,
    pub draft: Option<Value>,
    pub incoming: Observation,
    /// None requires an explicit decision, including all unresolved comparisons.
    pub automatic: Option<Choice>,
}
#[derive(Clone, Debug)]
pub struct Plan {
    rows: Vec<Row>,
    base_pin: String,
    incoming_pin: String,
    draft_pin: String,
    schema_pin: String,
    partial: bool,
}
fn fingerprint<T: serde::Serialize>(data: &T) -> Result<String, String> {
    serde_json::to_string(data)
        .map(|s| source_hash(&s))
        .map_err(|e| e.to_string())
}
fn document_pin(doc: &LuaDocument, catalogue: &[Setting]) -> Result<String, String> {
    if source_hash(&doc.source) != doc.report.source_sha256 {
        return Err("Source changed since its parser report".into());
    }
    doc.report.draft.validate(catalogue)?;
    fingerprint(&doc.report)
}
fn schema_pin(catalogue: &[Setting], version: &str) -> Result<String, String> {
    let mut pack: Vec<_> = catalogue.iter().filter(|s| s.version == version).collect();
    pack.sort_by(|a, b| a.id.cmp(&b.id));
    if pack.is_empty()
        || pack.len() > 512
        || pack.windows(2).any(|p| p[0].id == p[1].id)
        || pack.iter().any(|s| {
            s.bounds.min.is_some_and(|n| !n.is_finite())
                || s.bounds.max.is_some_and(|n| !n.is_finite())
        })
    {
        return Err("Catalogue has duplicate settings or invalid schema bounds".into());
    }
    fingerprint(&pack)
}
fn partial(doc: &LuaDocument) -> bool {
    doc.report.opaque_effects || !doc.report.diagnostics.is_empty()
}
fn observe(doc: &LuaDocument, id: &str) -> Observation {
    let value = doc.report.draft.values.get(id).cloned();
    if partial(doc) {
        // Opaque code can invalidate cached locals. Offer only a visible, unambiguous
        // literal span, never a derived projection masquerading as a source literal.
        let literal = value.filter(|value| {
            doc.report
                .declarations
                .iter()
                .rev()
                .find(|d| d.id == id)
                .is_some_and(|d| d.editable && d.value.as_ref() == Some(value))
        });
        Observation::Unresolved(literal)
    } else {
        value.map_or(Observation::Absent, Observation::Known)
    }
}
pub fn prepare(
    base: &LuaDocument,
    draft: &Draft,
    incoming: &LuaDocument,
    catalogue: &[Setting],
) -> Result<Plan, String> {
    draft.validate(catalogue)?;
    if base.report.draft.version != draft.version || incoming.report.draft.version != draft.version
    {
        return Err(
            "Reconciliation needs the same exact catalogue snapshot on both sources and the draft"
                .into(),
        );
    }
    let base_pin = document_pin(base, catalogue)?;
    let incoming_pin = document_pin(incoming, catalogue)?;
    let mut keys: BTreeSet<_> = base
        .report
        .draft
        .values
        .keys()
        .chain(draft.values.keys())
        .chain(incoming.report.draft.values.keys())
        .cloned()
        .collect();
    keys.extend(
        base.report
            .declarations
            .iter()
            .chain(&incoming.report.declarations)
            .map(|d| d.id.clone()),
    );
    if keys.len() > 512 {
        return Err("Reconciliation setting budget exceeded".into());
    }
    let mut rows = vec![];
    for id in keys {
        if !catalogue
            .iter()
            .any(|s| s.version == draft.version && s.id == id)
        {
            return Err("Parser report names a setting outside this catalogue".into());
        }
        let old = observe(base, &id);
        let new = observe(incoming, &id);
        let mine = draft.values.get(&id).cloned();
        let automatic = if matches!(old, Observation::Unresolved(_))
            || matches!(new, Observation::Unresolved(_))
        {
            None
        } else if mine == old.selected()? {
            Some(Choice::UseSelectedFile)
        } else if new == old || mine == new.selected()? {
            Some(Choice::KeepDraft)
        } else {
            None
        };
        rows.push(Row {
            id,
            base: old,
            draft: mine,
            incoming: new,
            automatic,
        });
    }
    Ok(Plan {
        rows,
        base_pin,
        incoming_pin,
        draft_pin: fingerprint(draft)?,
        schema_pin: schema_pin(catalogue, &draft.version)?,
        partial: partial(base) || partial(incoming),
    })
}
impl Plan {
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }
    pub fn partial(&self) -> bool {
        self.partial
    }
    /// Every input is rebound by content/report/draft pin at acceptance, not by file path.
    pub fn resolve(
        &self,
        base: &LuaDocument,
        draft: &Draft,
        incoming: &LuaDocument,
        choices: &BTreeMap<String, Choice>,
        acknowledge_partial: bool,
        catalogue: &[Setting],
    ) -> Result<Draft, String> {
        draft.validate(catalogue)?;
        if self.schema_pin != schema_pin(catalogue, &draft.version)? {
            return Err(
                "Catalogue changed since this review; compare again with the current schema".into(),
            );
        }
        if self.base_pin != document_pin(base, catalogue)?
            || self.incoming_pin != document_pin(incoming, catalogue)?
            || self.draft_pin != fingerprint(draft)?
        {
            return Err(
                "Reconciliation review is stale; compare the current draft and sources again"
                    .into(),
            );
        }
        if self.partial && !acknowledge_partial {
            return Err("Acknowledge the incomplete static projection before accepting this source revision".into());
        }
        if choices
            .keys()
            .any(|key| !self.rows.iter().any(|r| &r.id == key))
        {
            return Err("Review choice refers to an unknown setting".into());
        }
        let mut result = draft.clone();
        for row in &self.rows {
            let choice = choices
                .get(&row.id)
                .copied()
                .or(row.automatic)
                .ok_or_else(|| format!("Choose how to resolve {}", row.id))?;
            let value = match choice {
                Choice::KeepDraft => row.draft.clone(),
                Choice::UseSelectedFile => row.incoming.selected()?,
            };
            if let Some(value) = value {
                result.values.insert(row.id.clone(), value);
            } else {
                result.values.remove(&row.id);
            }
        }
        result.validate(catalogue)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue, lua, AUDITED_MAIN, BASELINE};
    fn doc(source: &str) -> LuaDocument {
        let c = catalogue();
        LuaDocument::from_report(source.into(), lua::parse(source, BASELINE, &c).unwrap(), &c)
            .unwrap()
    }
    #[test]
    fn independent_changes_additions_and_source_deletions_merge_without_losing_edits() {
        let c = catalogue();
        let base = doc("hl.config{general={border_size=2}, decoration={rounding=3}}");
        let mut draft = base.report.draft.clone();
        draft
            .values
            .insert("general:border_size".into(), Value::Int(5));
        let incoming = doc(
            "hl.config{general={border_size=2}, animations={enabled=false}, input={kb_layout=''}}",
        );
        let plan = prepare(&base, &draft, &incoming, &c).unwrap();
        let result = plan
            .resolve(&base, &draft, &incoming, &BTreeMap::new(), false, &c)
            .unwrap();
        assert_eq!(result.values["general:border_size"], Value::Int(5));
        assert!(!result.values.contains_key("decoration:rounding"));
        assert_eq!(result.values["animations:enabled"], Value::Bool(false));
        assert_eq!(
            result.values["input:kb_layout"],
            Value::String(String::new())
        );
    }
    #[test]
    fn conflicting_edit_and_delete_require_explicit_choices() {
        let c = catalogue();
        let base = doc("hl.config{general={border_size=2},decoration={rounding=3}}");
        let mut draft = base.report.draft.clone();
        draft
            .values
            .insert("general:border_size".into(), Value::Int(5));
        draft.values.remove("decoration:rounding");
        let incoming = doc("hl.config{general={border_size=7},decoration={rounding=4}}");
        let plan = prepare(&base, &draft, &incoming, &c).unwrap();
        assert!(plan.rows().iter().all(|r| r.automatic.is_none()));
        assert!(plan
            .resolve(&base, &draft, &incoming, &BTreeMap::new(), false, &c)
            .is_err());
        let choices = BTreeMap::from([
            ("general:border_size".into(), Choice::KeepDraft),
            ("decoration:rounding".into(), Choice::UseSelectedFile),
        ]);
        let result = plan
            .resolve(&base, &draft, &incoming, &choices, false, &c)
            .unwrap();
        assert_eq!(result.values["general:border_size"], Value::Int(5));
        assert_eq!(result.values["decoration:rounding"], Value::Int(4));
    }
    #[test]
    fn opaque_code_never_turns_missing_projection_into_unset() {
        let c = catalogue();
        let base = doc("hl.config{general={border_size=2}}");
        let draft = base.report.draft.clone();
        let incoming = doc("require('not-executed'); hl.config{general={border_size=7}}");
        let plan = prepare(&base, &draft, &incoming, &c).unwrap();
        assert!(plan.partial());
        assert_eq!(plan.rows()[0].incoming, Observation::Unresolved(None));
        let mut choices = BTreeMap::from([("general:border_size".into(), Choice::UseSelectedFile)]);
        assert!(plan
            .resolve(&base, &draft, &incoming, &choices, true, &c)
            .is_err());
        choices.insert("general:border_size".into(), Choice::KeepDraft);
        assert!(plan
            .resolve(&base, &draft, &incoming, &choices, false, &c)
            .is_err());
        assert_eq!(
            plan.resolve(&base, &draft, &incoming, &choices, true, &c)
                .unwrap()
                .values,
            draft.values
        );
    }
    #[test]
    fn syntactic_literal_in_partial_source_requires_choice_and_acknowledgement() {
        let c = catalogue();
        let base = doc("hl.config{general={border_size=2}}");
        let draft = base.report.draft.clone();
        let incoming = doc("hl.config{general={border_size=7}}; os.execute('never run')");
        let plan = prepare(&base, &draft, &incoming, &c).unwrap();
        assert_eq!(
            plan.rows()[0].incoming,
            Observation::Unresolved(Some(Value::Int(7)))
        );
        let choices = BTreeMap::from([("general:border_size".into(), Choice::UseSelectedFile)]);
        assert_eq!(
            plan.resolve(&base, &draft, &incoming, &choices, true, &c)
                .unwrap()
                .values["general:border_size"],
            Value::Int(7)
        );
    }
    #[test]
    fn pins_reject_new_edits_changed_reports_sources_and_version_mixing() {
        let c = catalogue();
        let base = doc("hl.config{general={border_size=2}}");
        let incoming = doc("hl.config{general={border_size=3}}");
        let mut draft = base.report.draft.clone();
        let plan = prepare(&base, &draft, &incoming, &c).unwrap();
        draft
            .values
            .insert("general:border_size".into(), Value::Int(5));
        assert!(plan
            .resolve(&base, &draft, &incoming, &BTreeMap::new(), false, &c)
            .unwrap_err()
            .contains("stale"));
        let draft = base.report.draft.clone();
        let mut changed = incoming.clone();
        changed
            .report
            .draft
            .values
            .insert("general:border_size".into(), Value::Int(4));
        assert!(plan
            .resolve(&base, &draft, &changed, &BTreeMap::new(), false, &c)
            .is_err());
        changed.source.push(' ');
        assert!(prepare(&base, &draft, &changed, &c).is_err());
        assert!(prepare(&base, &Draft::new(AUDITED_MAIN), &incoming, &c).is_err());
    }
    #[test]
    fn opaque_empty_sources_need_acknowledgement_even_without_rows() {
        let c = catalogue();
        let base = doc("-- empty");
        let incoming = doc("require('opaque')");
        let draft = Draft::new(BASELINE);
        let plan = prepare(&base, &draft, &incoming, &c).unwrap();
        assert!(plan.rows().is_empty());
        assert!(plan
            .resolve(&base, &draft, &incoming, &BTreeMap::new(), false, &c)
            .is_err());
        assert!(plan
            .resolve(&base, &draft, &incoming, &BTreeMap::new(), true, &c)
            .is_ok());
    }
    #[test]
    fn derived_projection_in_partial_source_is_not_an_adoptable_literal() {
        let c = catalogue();
        let base = doc("hl.config{general={border_size=2}}");
        let draft = base.report.draft.clone();
        let incoming = doc("local n=7;hl.config{general={border_size=n}};require('opaque')");
        assert_eq!(
            incoming.report.draft.values["general:border_size"],
            Value::Int(7)
        );
        let plan = prepare(&base, &draft, &incoming, &c).unwrap();
        assert_eq!(plan.rows()[0].incoming, Observation::Unresolved(None));
        let choices = BTreeMap::from([("general:border_size".into(), Choice::UseSelectedFile)]);
        assert!(plan
            .resolve(&base, &draft, &incoming, &choices, true, &c)
            .is_err());
    }
    #[test]
    fn changed_schema_duplicates_and_nonfinite_bounds_do_not_reuse_a_review() {
        let mut c = catalogue();
        let base = doc("hl.config{general={border_size=2}}");
        let incoming = doc("hl.config{general={border_size=3}}");
        let draft = base.report.draft.clone();
        let plan = prepare(&base, &draft, &incoming, &c).unwrap();
        c.iter_mut()
            .find(|s| s.version == BASELINE && s.id == "general:border_size")
            .unwrap()
            .bounds
            .max = Some(21.);
        assert!(plan
            .resolve(&base, &draft, &incoming, &BTreeMap::new(), false, &c)
            .unwrap_err()
            .contains("Catalogue changed"));
        c.iter_mut()
            .find(|s| s.version == BASELINE && s.id == "general:border_size")
            .unwrap()
            .bounds
            .max = Some(f64::NAN);
        assert!(prepare(&base, &draft, &incoming, &c).is_err());
        let mut c = catalogue();
        c.push(c[0].clone());
        assert!(prepare(&base, &draft, &incoming, &c).is_err());
    }
}
