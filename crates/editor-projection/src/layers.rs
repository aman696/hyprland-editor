//! Explicit scalar input selection for a fresh sample. Captured argument values
//! are never effective compositor state or source-write authority.
use super::*;
use editor_core::lua::{
    config_inspection, registration, replay,
    scalar_codec::{NativeValue, Outcome, Representation},
};
use editor_core::workspace::SourceLanguage;
pub const MAX_ROWS: usize = 512;
pub const MAX_REVIEW_BYTES: usize = 4 * MAX_DOCUMENT_BYTES;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Captured {
    Value {
        value: Value,
        native: NativeValue,
        provisional: bool,
        selectable: bool,
        document: String,
        event_order: usize,
    },
    Absent,
    Unknown {
        reason: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub setting_id: String,
    pub pending: Option<Value>,
    pub captured: Captured,
    /// Source-pinned registration metadata, not a detected or resolved live value.
    pub pack_default: Option<NativeValue>,
    pub declaration_count: usize,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub workspace_sha256: String,
    pub model_sha256: String,
    pub graph_schema_sha256: String,
    pub registration_sha256: String,
    pub version: String,
    pub compiler: replay::CompilerProfile,
    pub source_present: bool,
    pub model_complete: bool,
    pub note: Option<String>,
    #[serde(deserialize_with = "rows")]
    pub rows: Vec<Row>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    Pending,
    Captured,
    LeaveUnset,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub review_sha256: String,
    pub acknowledge_partial: bool,
    #[serde(deserialize_with = "choices")]
    pub choices: BTreeMap<String, Choice>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub review_sha256: String,
    pub selection_sha256: String,
    pub resolved_workspace_sha256: String,
    pub captured_values: usize,
    pub pending_values: usize,
    pub unset_values: usize,
    pub unresolved_source_rows: usize,
    pub model_complete: bool,
    pub source_value_roundtrips: usize,
}
pub(super) fn rows<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Row>, D::Error> {
    super::wire::bounded_vec::<D, Row, MAX_ROWS>(d)
}
fn choices<'de, D: serde::Deserializer<'de>>(d: D) -> Result<BTreeMap<String, Choice>, D::Error> {
    super::wire::bounded_map::<D, Choice, MAX_ROWS>(d)
}
fn encoded(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, String> {
    struct Count(usize, usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > self.1 {
                return Err(std::io::Error::other("Layer data budget exceeded"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Count(0, limit), value)
        .map_err(|_| "Layer data exceeds its bounded envelope")?;
    serde_json::to_vec(value).map_err(|e| e.to_string())
}
pub(super) fn hash(value: &impl Serialize, limit: usize) -> Result<String, String> {
    let bytes = encoded(value, limit)?;
    Ok(editor_core::lua::source_hash(
        std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
    ))
}
impl Selection {
    pub fn fingerprint(&self) -> Result<String, String> {
        if self.choices.len() > MAX_ROWS
            || self.review_sha256.len() != 64
            || !self
                .review_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err("Invalid layer selection identity/count".into());
        }
        hash(self, 64 * 1024)
    }
}
impl Review {
    pub fn fingerprint(&self) -> Result<String, String> {
        hash(self, MAX_REVIEW_BYTES)
    }
    /// Correlation and bounded shape, not trusted source execution evidence.
    pub fn verify(&self, workspace: &Workspace) -> Result<(), String> {
        let catalogue = editor_core::catalogue();
        let expected_ids: BTreeSet<_> = catalogue
            .iter()
            .filter(|s| s.version == workspace.draft.version)
            .map(|s| s.id.as_str())
            .collect();
        let ids: BTreeSet<_> = self.rows.iter().map(|r| r.setting_id.as_str()).collect();
        if self.workspace_sha256 != workspace_hash(workspace)?
            || self.model_sha256 != model_sha256()
            || self.graph_schema_sha256 != editor_core::workspace_graph::schema_sha256()
            || self.registration_sha256 != registration::schema_sha256()
            || self.version != workspace.draft.version
            || self.compiler != replay::compiler_profile()
            || ids != expected_ids
            || self.rows.len() != ids.len()
            || self.source_present == workspace.sources.is_empty()
            || self.note.as_ref().is_some_and(|s| s.len() > 1024)
            || self.rows.iter().any(|r| {
                r.pending.as_ref() != workspace.draft.values.get(&r.setting_id)
                    || r.declaration_count > 2048
                    || matches!(&r.captured, Captured::Unknown {reason} if reason.len()>1024)
            })
        {
            return Err("Layer review does not match this bounded workspace/model/profile".into());
        }
        self.fingerprint()?;
        for row in &self.rows {
            encoded(row, 512 * 1024)?;
        }
        Ok(())
    }
}
pub(super) fn workspace_hash(workspace: &Workspace) -> Result<String, String> {
    let bytes = workspace.to_json(&editor_core::catalogue())?;
    Ok(editor_core::lua::source_hash(
        std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
    ))
}
fn check_reached(program: &replay::Program, report: &replay::Report) -> Result<(), String> {
    for binding in &program.bindings {
        if !report.loads.iter().any(|load| {
            load.request == binding.request
                && match (&load.outcome, &binding.resolution) {
                    (
                        replay::LoadOutcome::SuppliedSnapshot {
                            document: a,
                            loader_data: x,
                        },
                        replay::Resolution::Loaded {
                            document: b,
                            loader_data: y,
                        },
                    ) => a == b && x == y,
                    (replay::LoadOutcome::SuppliedMissing, replay::Resolution::Missing) => true,
                    _ => false,
                }
        }) {
            return Err(
                "Captured graph has an unreachable or inconsistent dependency input".into(),
            );
        }
    }
    for doc in &program.documents {
        if doc.id != program.root && !report.loads.iter().any(|load| matches!(&load.outcome, replay::LoadOutcome::SuppliedSnapshot {document,..} if document == &doc.id)) {
            return Err("Captured graph has a source view that no loader reaches".into());
        }
    }
    Ok(())
}
fn rebuild(workspace: &Workspace) -> Result<Option<replay::Report>, String> {
    let Some(root) = workspace.root() else {
        return Ok(None);
    };
    if root.language != SourceLanguage::Lua {
        return Ok(None);
    }
    let mut program = if let Some(graph) = &workspace.source_graph {
        graph.program(
            &workspace.sources,
            workspace.root_source.as_deref(),
            &workspace.draft.version,
        )?
    } else {
        if workspace.sources.len() != 1 {
            return Err("Multiple saved sources require their captured graph".into());
        }
        let context = editor_core::module_path::Context {
            main_file: root.origin.selected_path.clone(),
            home: None,
            inherited_package_path: None,
        };
        let (path, unknown) = context.initial_path(&workspace.draft.version)?;
        replay::Program {
            version: workspace.draft.version.clone(),
            profile: replay::Profile::HyprlandLua54,
            root: root.id.clone(),
            documents: vec![replay::Document {
                id: root.id.clone(),
                source: root.content.clone(),
                sha256: root.origin.sha256.clone(),
                load_name: root.origin.selected_path.clone(),
            }],
            bindings: vec![],
            initial_package_path: Some(path),
            initial_package_path_unknown_tail: unknown,
            working_directory: None,
        }
    };
    for doc in &program.documents {
        replay::validate_document(doc)?;
    }
    let base = replay::replay(&program)?;
    if base.runtime_verified
        || base.compiler_profile != replay::compiler_profile()
        || workspace
            .source_graph
            .as_ref()
            .is_some_and(|g| g.compiler != base.compiler_profile)
    {
        return Err("Captured compiler profile changed; explicit reanalysis is required".into());
    }
    if let Some(graph) = &workspace.source_graph {
        check_reached(&program, &base)?;
        if !graph.revisions.is_empty() || !graph.private.is_empty() {
            program = graph.private.program(&program, &graph.revisions)?;
            for doc in &program.documents {
                replay::validate_document(doc)?;
            }
            let revised = replay::replay(&program)?;
            if revised.runtime_verified || revised.compiler_profile != graph.compiler {
                return Err("Private revision compiler profile changed".into());
            }
            graph.private.check_reached(&program, &revised)?;
            return Ok(Some(revised));
        }
    }
    Ok(Some(base))
}
/// Run only in the restricted staging role. No saved report, span or source
/// path is trusted as authority; all observations are reconstructed from bytes.
pub fn review(workspace: &Workspace) -> Result<Review, String> {
    let workspace_sha256 = workspace_hash(workspace)?;
    let report = rebuild(workspace)?;
    let events = report
        .as_ref()
        .map(|r| {
            r.events
                .iter()
                .filter(|e| e.api == "hl.config")
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let inspection = config_inspection::inspect(&events, &workspace.draft.version)?;
    let complete = if let Some(report) = &report {
        report.halt.is_none()
            && report.logged_module_errors == 0
            && report.events.len() == events.len()
            && inspection.stopped.is_none()
            && inspection.batches.iter().all(|b| {
                b.traversal_resolved
                    && b.findings.is_empty()
                    && b.candidates.iter().all(|c| {
                        !c.ambiguous && matches!(c.model_value, Some(Outcome::Accepted { .. }))
                    })
            })
    } else {
        workspace.sources.is_empty()
    };
    let mut rows = Vec::new();
    for setting in editor_core::catalogue()
        .iter()
        .filter(|s| s.version == workspace.draft.version)
    {
        let candidates: Vec<_> = inspection
            .batches
            .iter()
            .flat_map(|b| &b.candidates)
            .filter(|c| c.setting_id == setting.id)
            .collect();
        let captured = match candidates.last().copied() {
            Some(c) if !c.ambiguous => match &c.model_value {
                Some(Outcome::Accepted {
                    native,
                    representation: Representation::Value { value },
                }) => {
                    let document = events
                        .iter()
                        .find(|e| e.order == c.event_order)
                        .ok_or("Source observation lost its event")?
                        .document
                        .clone();
                    Captured::Value {
                        value: value.clone(),
                        native: native.clone(),
                        provisional: !complete,
                        selectable: setting.validate(value).is_ok(),
                        document,
                        event_order: c.event_order,
                    }
                }
                _ => Captured::Unknown {
                    reason: "The last observed argument is rejected, unrepresentable or unmodeled"
                        .into(),
                },
            },
            Some(_) => Captured::Unknown {
                reason: "Several native keys address the same setting".into(),
            },
            None if complete => Captured::Absent,
            None => Captured::Unknown {
                reason: "Analysis cannot establish an absent source value".into(),
            },
        };
        let meta = registration::find(&setting.version, registration::Role::Global, &setting.id)
            .or_else(|| {
                registration::find(&setting.version, registration::Role::Bootstrap, &setting.id)
            });
        rows.push(Row {
            setting_id: setting.id.clone(),
            pending: workspace.draft.values.get(&setting.id).cloned(),
            captured,
            pack_default: meta.and_then(|m| m.native_default.clone()),
            declaration_count: candidates.len(),
        });
    }
    let review = Review {workspace_sha256,model_sha256:model_sha256(),graph_schema_sha256:editor_core::workspace_graph::schema_sha256(),registration_sha256:registration::schema_sha256(),version:workspace.draft.version.clone(),compiler:replay::compiler_profile(),source_present:!workspace.sources.is_empty(),model_complete:complete,
        note:(!complete).then(||"Source behavior is incomplete. Values are modeled arguments, not verified effective settings; explicit acknowledgement is required to sample them.".into()),rows};
    review.verify(workspace)?;
    Ok(review)
}
pub(super) fn resolve(
    workspace: &Workspace,
    selection: &Selection,
) -> Result<(Workspace, Summary, BTreeMap<String, NativeValue>), String> {
    let selection_sha256 = selection.fingerprint()?;
    let fresh = review(workspace)?;
    let review_sha256 = fresh.fingerprint()?;
    if selection.review_sha256 != review_sha256
        || selection
            .choices
            .keys()
            .any(|id| !fresh.rows.iter().any(|r| &r.setting_id == id))
    {
        return Err("Layer choices are stale or name an unknown setting".into());
    }
    let mut resolved = workspace.clone();
    let mut captured_native = BTreeMap::new();
    resolved.draft.values.clear();
    let mut summary = Summary {
        review_sha256,
        selection_sha256,
        resolved_workspace_sha256: String::new(),
        captured_values: 0,
        pending_values: 0,
        unset_values: 0,
        unresolved_source_rows: 0,
        model_complete: fresh.model_complete,
        source_value_roundtrips: 0,
    };
    for row in &fresh.rows {
        if matches!(row.captured, Captured::Unknown { .. }) {
            summary.unresolved_source_rows += 1;
        }
        let choice =
            selection
                .choices
                .get(&row.setting_id)
                .copied()
                .unwrap_or(if row.pending.is_some() {
                    Choice::Pending
                } else {
                    Choice::LeaveUnset
                });
        let value = match choice {
            Choice::Pending => { summary.pending_values+=1; Some(row.pending.clone().ok_or("Selected pending value is absent")?) }
            Choice::Captured => match &row.captured {
                Captured::Value {value,native,provisional,selectable,..} if *selectable && (!*provisional || selection.acknowledge_partial) => {summary.captured_values+=1;captured_native.insert(row.setting_id.clone(),native.clone());Some(value.clone())}
                _ => return Err("Selected captured value is unavailable, outside draft constraints or needs partial-analysis acknowledgement".into()),
            },
            Choice::LeaveUnset => {summary.unset_values+=1;None}
        };
        if let Some(value) = value {
            resolved.draft.values.insert(row.setting_id.clone(), value);
        }
    }
    summary.resolved_workspace_sha256 = workspace_hash(&resolved)?;
    Ok((resolved, summary, captured_native))
}

pub(super) fn check_projection(
    lua: &str,
    expected: &BTreeMap<String, NativeValue>,
    projected: &BTreeSet<String>,
    version: &str,
) -> Result<usize, String> {
    let program = replay::Program {
        version: version.into(),
        profile: replay::Profile::HyprlandLua54,
        root: "projection".into(),
        documents: vec![replay::Document {
            id: "projection".into(),
            source: lua.into(),
            sha256: editor_core::lua::source_hash(lua),
            load_name: b"@hye-projection".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    };
    let report = replay::replay(&program)?;
    let events: Vec<_> = report
        .events
        .into_iter()
        .filter(|e| e.api == "hl.config")
        .collect();
    let inspection = config_inspection::inspect(&events, version)?;
    if inspection.stopped.is_some()
        || inspection.batches.iter().any(|b| {
            !b.traversal_resolved
                || !b.findings.is_empty()
                || b.candidates.iter().any(|c| {
                    c.ambiguous
                        || !matches!(
                            c.model_value,
                            Some(Outcome::Accepted {
                                representation: Representation::Value { .. },
                                ..
                            })
                        )
                })
        })
        || inspection
            .batches
            .iter()
            .map(|b| b.candidates.len())
            .sum::<usize>()
            != projected.len()
    {
        return Err("Generated scalar argument models are unresolved".into());
    }
    let mut count = 0;
    for (id, native) in expected.iter().filter(|(id, _)| projected.contains(*id)) {
        let last = inspection
            .batches
            .iter()
            .flat_map(|b| &b.candidates)
            .filter(|c| &c.setting_id == id)
            .next_back();
        if !matches!(last,Some(c) if !c.ambiguous && matches!(&c.model_value,Some(Outcome::Accepted{native:value,..}) if value==native))
        {
            return Err(format!(
                "The generated scalar argument changes the selected source representation for {id}"
            ));
        }
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_core::{
        source::{RecordedOwnership, SourceIdentity},
        Draft, AUDITED_MAIN, BASELINE,
    };
    fn workspace(version: &str, source: &str) -> Workspace {
        let identity = SourceIdentity {
            selected_path: b"/deleted/private/main.lua".to_vec(),
            resolved_path: None,
            links: vec![],
            device: 1,
            inode: 2,
            owner: 1000,
            mode: 0o100600,
            size: source.len() as u64,
            modified_seconds: 0,
            modified_nanoseconds: 0,
            changed_seconds: 0,
            changed_nanoseconds: 0,
            sha256: editor_core::lua::source_hash(source),
            recorded_ownership: RecordedOwnership::UserOwned,
        };
        Workspace::new(Draft::new(version)).with_root(source.into(), identity)
    }
    #[test]
    fn explicit_pending_captured_and_unset_choices_preserve_the_entire_input() {
        for version in [BASELINE, AUDITED_MAIN] {
            let mut workspace = workspace(
                version,
                "hl.config{general={border_size=2}};hl.config{general={border_size=3}}\n",
            );
            workspace
                .draft
                .values
                .insert("general:border_size".into(), Value::Int(9));
            let original = workspace.to_json(&editor_core::catalogue()).unwrap();
            let review = review(&workspace).unwrap();
            assert!(review.model_complete);
            let row = review
                .rows
                .iter()
                .find(|r| r.setting_id == "general:border_size")
                .unwrap();
            assert_eq!(row.declaration_count, 2);
            assert!(matches!(
                row.captured,
                Captured::Value {
                    value: Value::Int(3),
                    provisional: false,
                    ..
                }
            ));
            assert!(row.pack_default.is_some());
            for (choice, value) in [
                (Choice::Pending, Some(Value::Int(9))),
                (Choice::Captured, Some(Value::Int(3))),
                (Choice::LeaveUnset, None),
            ] {
                let selection = Selection {
                    review_sha256: review.fingerprint().unwrap(),
                    acknowledge_partial: false,
                    choices: BTreeMap::from([("general:border_size".into(), choice)]),
                };
                let projection =
                    compile_with_layers(&workspace, &Scenario::default(), &selection).unwrap();
                let parsed =
                    editor_core::lua::parse(projection.lua(), version, &editor_core::catalogue())
                        .unwrap();
                assert_eq!(
                    parsed.draft.values.get("general:border_size"),
                    value.as_ref()
                );
                assert_eq!(
                    workspace.to_json(&editor_core::catalogue()).unwrap(),
                    original
                );
                let expected =
                    wire::Expected::with_layers(&workspace, &Scenario::default(), &selection)
                        .unwrap();
                let artifact = wire::Artifact::from(projection);
                artifact.verify(&expected).unwrap();
                assert!(artifact
                    .verify(&wire::Expected::new(&workspace, &Scenario::default()).unwrap())
                    .is_err());
            }
        }
    }
    #[test]
    fn incomplete_observations_require_acknowledgement_and_stale_choices_never_fall_back() {
        let workspace = workspace(
            BASELINE,
            "hl.config{general={border_size=3}};os.execute('HYE_NEVER_EXECUTE')\n",
        );
        let review = review(&workspace).unwrap();
        assert!(!review.model_complete);
        let mut selection = Selection {
            review_sha256: review.fingerprint().unwrap(),
            acknowledge_partial: false,
            choices: BTreeMap::from([("general:border_size".into(), Choice::Captured)]),
        };
        assert!(compile_with_layers(&workspace, &Scenario::default(), &selection).is_err());
        selection.acknowledge_partial = true;
        let projection = compile_with_layers(&workspace, &Scenario::default(), &selection).unwrap();
        assert!(!projection.lua().contains("HYE_NEVER_EXECUTE"));
        selection.review_sha256 = "0".repeat(64);
        assert!(compile_with_layers(&workspace, &Scenario::default(), &selection).is_err());
        selection.review_sha256 = review.fingerprint().unwrap();
        selection
            .choices
            .insert("unknown".into(), Choice::LeaveUnset);
        assert!(compile_with_layers(&workspace, &Scenario::default(), &selection).is_err());
    }
    #[test]
    fn absent_unknown_and_rejected_values_are_distinct_and_duplicate_choices_are_refused() {
        let empty = Workspace::new(Draft::new(BASELINE));
        let absent = review(&empty).unwrap();
        assert!(!absent.source_present && absent.model_complete);
        assert!(absent
            .rows
            .iter()
            .all(|r| matches!(r.captured, Captured::Absent)));
        let rejected = workspace(BASELINE, "hl.config{general={border_size='bad'}}\n");
        let review = review(&rejected).unwrap();
        assert!(!review.model_complete);
        assert!(matches!(
            review
                .rows
                .iter()
                .find(|r| r.setting_id == "general:border_size")
                .unwrap()
                .captured,
            Captured::Unknown { .. }
        ));
        let duplicate = format!(
            r#"{{"review_sha256":"{}","acknowledge_partial":false,"choices":{{"same":"pending","same":"captured"}}}}"#,
            "0".repeat(64)
        );
        assert!(serde_json::from_str::<Selection>(&duplicate).is_err());
        let mut invalid_pending = Workspace::new(Draft::new(BASELINE));
        invalid_pending.draft.values.insert(
            "input:follow_mouse_threshold".into(),
            Value::Float(f64::MAX),
        );
        let review = super::review(&invalid_pending).unwrap();
        let selection = Selection {
            review_sha256: review.fingerprint().unwrap(),
            acknowledge_partial: false,
            choices: BTreeMap::new(),
        };
        assert!(compile_with_layers(&invalid_pending, &Scenario::default(), &selection).is_err());
    }
    #[test]
    fn all_nine_selected_source_families_keep_their_modeled_native_representation() {
        for version in [BASELINE, AUDITED_MAIN] {
            let mut input = Draft::new(version);
            input.values = BTreeMap::from([
                ("general:no_focus_fallback".into(), Value::Bool(true)),
                ("general:border_size".into(), Value::Int(3)),
                ("decoration:active_opacity".into(), Value::Float(0.7)),
                ("input:kb_layout".into(), Value::String("us".into())),
                ("group:groupbar:text_color".into(), Value::Color(0xff112233)),
                (
                    "general:col.active_border".into(),
                    Value::Gradient {
                        colors: vec![0xff112233, 0xff998877],
                        angle: 0.4,
                    },
                ),
                ("decoration:shadow:offset".into(), Value::Vec2([-0.0, 2.5])),
                ("general:gaps_in".into(), Value::CssGap([1, 2, 3, 4])),
                (
                    "group:groupbar:font_weight_active".into(),
                    Value::FontWeight(400),
                ),
            ]);
            let source = input.export_lua(&editor_core::catalogue()).unwrap();
            let workspace = workspace(version, &source);
            let review = review(&workspace).unwrap();
            assert!(review.model_complete);
            let selection = Selection {
                review_sha256: review.fingerprint().unwrap(),
                acknowledge_partial: false,
                choices: input
                    .values
                    .keys()
                    .map(|id| (id.clone(), Choice::Captured))
                    .collect(),
            };
            let projection =
                compile_with_layers(&workspace, &Scenario::default(), &selection).unwrap();
            assert_eq!(
                projection
                    .report()
                    .layers
                    .as_ref()
                    .unwrap()
                    .source_value_roundtrips,
                9
            );
            assert!(workspace.draft.values.is_empty());
        }
    }
    #[test]
    fn captured_negative_zero_keeps_its_float_and_vector_sign_bits_in_the_sample() {
        let workspace = workspace(
            BASELINE,
            "hl.config{decoration={active_opacity=-0.0,shadow={offset={-0.0,2.0}}}}\n",
        );
        let review = review(&workspace).unwrap();
        assert!(review.model_complete);
        let selection = Selection {
            review_sha256: review.fingerprint().unwrap(),
            acknowledge_partial: false,
            choices: BTreeMap::from([
                ("decoration:active_opacity".into(), Choice::Captured),
                ("decoration:shadow:offset".into(), Choice::Captured),
            ]),
        };
        let projection = compile_with_layers(&workspace, &Scenario::default(), &selection).unwrap();
        assert!(projection.lua().contains("-0.0"));
        assert_eq!(
            projection
                .report()
                .layers
                .as_ref()
                .unwrap()
                .source_value_roundtrips,
            2
        );
    }
}
