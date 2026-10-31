//! Inverse field/source operations with shared immutable source snapshots.
use hyprland_editor_core::{lua::LuaDocument, source::SourceIdentity, Draft, Value};
use std::{
    collections::{BTreeSet, VecDeque},
    mem::size_of,
    sync::Arc,
};

const MAX_OPERATIONS: usize = 1000;
const MAX_BYTES: usize = 8 * 1024 * 1024;
pub(super) struct SourceInverse {
    pub document: Arc<LuaDocument>,
    pub origin: SourceIdentity,
    document_bytes: usize,
}
pub(super) enum Undo {
    Graph {
        previous: Option<Arc<super::source_graph::Saved>>,
        source: Option<SourceInverse>,
        changes: Vec<(String, Option<Value>)>,
        monitor_changed: bool,
        previous_monitors: Option<Vec<hyprland_editor_core::monitor::Rule>>,
        retained_bytes: usize,
    },
    Field {
        id: String,
        previous: Option<Value>,
    },
    Source {
        changes: Vec<(String, Option<Value>)>,
        document: Arc<LuaDocument>,
        origin: SourceIdentity,
        document_bytes: usize,
    },
    Session {
        previous: hyprland_editor_core::session::Draft,
        retained_bytes: usize,
    },
    Gestures {
        previous: hyprland_editor_core::gesture::Draft,
        retained_bytes: usize,
    },
    Animations {
        previous: hyprland_editor_core::animation::Draft,
        retained_bytes: usize,
    },
    Rules {
        previous: Vec<hyprland_editor_core::rule::Rule>,
        retained_bytes: usize,
    },
    Workspaces {
        previous: Vec<hyprland_editor_core::workspace_rule::Rule>,
        retained_bytes: usize,
    },
    Bindings {
        inverse: hyprland_editor_core::binding::Inverse,
        retained_bytes: usize,
    },
    Devices {
        previous: Vec<hyprland_editor_core::device::Spec>,
        retained_bytes: usize,
    },
    Monitors {
        previous: Vec<hyprland_editor_core::monitor::Rule>,
        retained_bytes: usize,
    },
}
fn value_bytes(value: &Value) -> usize {
    match value {
        Value::String(s) => s.capacity(),
        Value::Gradient { colors, .. } => colors.capacity() * size_of::<u32>(),
        _ => 0,
    }
}
fn changes_bytes(changes: &[(String, Option<Value>)]) -> usize {
    changes
        .iter()
        .map(|(id, v)| id.capacity() + v.as_ref().map_or(0, value_bytes))
        .sum()
}
fn origin_bytes(origin: &SourceIdentity) -> usize {
    origin.selected_path.capacity()
        + origin.sha256.capacity()
        + origin.resolved_path.as_ref().map_or(0, Vec::capacity)
        + origin.links.capacity() * size_of::<hyprland_editor_core::source::LinkIdentity>()
        + origin
            .links
            .iter()
            .map(|l| l.encountered_path.capacity() + l.target.capacity())
            .sum::<usize>()
}
fn monitor_bytes(rules: &Vec<hyprland_editor_core::monitor::Rule>) -> usize {
    rules.capacity() * size_of::<hyprland_editor_core::monitor::Rule>()
        + rules
            .iter()
            .map(|r| {
                512 + r.id.capacity()
                    + match &r.output {
                        hyprland_editor_core::monitor::Output::Fallback => 0,
                        hyprland_editor_core::monitor::Output::Connector(s)
                        | hyprland_editor_core::monitor::Output::Description(s) => s.capacity(),
                    }
                    + r.fields
                        .iter()
                        .map(|(k, v)| 256 + size_of::<Value>() + k.capacity() + value_bytes(v))
                        .sum::<usize>()
            })
            .sum::<usize>()
}
fn workspace_bytes(rules: &Vec<hyprland_editor_core::workspace_rule::Rule>) -> usize {
    rules.capacity() * size_of::<hyprland_editor_core::workspace_rule::Rule>()
        + rules
            .iter()
            .map(|r| {
                512 + r.id.capacity()
                    + r.selector.capacity()
                    + r.fields
                        .iter()
                        .chain(r.layout_options.iter())
                        .map(|(k, v)| 256 + size_of::<Value>() + k.capacity() + value_bytes(v))
                        .sum::<usize>()
            })
            .sum::<usize>()
}
fn action_bytes(a: &hyprland_editor_core::action::Action) -> usize {
    a.constructor.capacity()
        + a.variant.capacity()
        + a.fields
            .iter()
            .map(|(k, v)| 256 + size_of::<Value>() + k.capacity() + value_bytes(v))
            .sum::<usize>()
}
fn gesture_bytes(draft: &hyprland_editor_core::gesture::Draft) -> usize {
    use hyprland_editor_core::gesture::GestureAction;
    let workflow = |w: &hyprland_editor_core::gesture::Workflow| {
        w.actions.capacity() * size_of::<hyprland_editor_core::action::Action>()
            + w.actions.iter().map(action_bytes).sum::<usize>()
            + w.event_parameter.as_ref().map_or(0, String::capacity)
    };
    draft.declarations.capacity() * size_of::<hyprland_editor_core::gesture::Spec>()
        + draft
            .declarations
            .iter()
            .map(|s| {
                s.id.capacity()
                    + s.direction.capacity()
                    + value_bytes(&s.fingers)
                    + [
                        &s.mods,
                        &s.scale,
                        &s.disable_inhibit,
                        &s.mode,
                        &s.zoom_level,
                        &s.workspace_name,
                    ]
                    .into_iter()
                    .flatten()
                    .map(value_bytes)
                    .sum::<usize>()
                    + match &s.action {
                        GestureAction::Builtin(name) => name.capacity(),
                        GestureAction::LegacyFinish(w) => workflow(w),
                        GestureAction::Callbacks(c) => c
                            .phases()
                            .into_iter()
                            .flatten()
                            .map(workflow)
                            .sum::<usize>(),
                    }
            })
            .sum::<usize>()
}
fn animation_bytes(draft: &hyprland_editor_core::animation::Draft) -> usize {
    use hyprland_editor_core::animation::{CurveKind, Operation};
    draft.declarations.capacity() * size_of::<hyprland_editor_core::animation::Declaration>()
        + draft
            .declarations
            .iter()
            .map(|d| {
                d.id.capacity()
                    + match &d.operation {
                        Operation::Node(n) => {
                            n.leaf.capacity()
                                + value_bytes(&n.enabled)
                                + n.speed.as_ref().map_or(0, value_bytes)
                                + [&n.bezier, &n.spring, &n.style]
                                    .into_iter()
                                    .flatten()
                                    .map(String::capacity)
                                    .sum::<usize>()
                        }
                        Operation::Curve(c) => {
                            c.name.capacity()
                                + match &c.shape {
                                    CurveKind::Bezier { points } => {
                                        points.iter().flatten().map(value_bytes).sum::<usize>()
                                    }
                                    CurveKind::Spring {
                                        stiffness,
                                        damping,
                                        mass,
                                        ..
                                    } => [stiffness, damping, mass]
                                        .into_iter()
                                        .map(value_bytes)
                                        .sum::<usize>(),
                                }
                        }
                    }
            })
            .sum::<usize>()
}
fn rule_bytes(rules: &Vec<hyprland_editor_core::rule::Rule>) -> usize {
    use hyprland_editor_core::rule::{EffectValue, Geometry};
    rules.capacity() * size_of::<hyprland_editor_core::rule::Rule>()
        + rules
            .iter()
            .map(|r| {
                r.id.capacity()
                    + r.name.as_ref().map_or(0, String::capacity)
                    + r.predicates.capacity() * size_of::<hyprland_editor_core::rule::Predicate>()
                    + r.predicates
                        .iter()
                        .map(|p| p.property.capacity() + value_bytes(&p.value))
                        .sum::<usize>()
                    + r.effects.capacity() * size_of::<hyprland_editor_core::rule::Effect>()
                    + r.effects
                        .iter()
                        .map(|e| {
                            e.key.capacity()
                                + match &e.value {
                                    EffectValue::Literal(v) => value_bytes(v),
                                    EffectValue::BorderText(s) => s.capacity(),
                                    EffectValue::Geometry(Geometry::Text(s)) => s.capacity(),
                                    EffectValue::Geometry(Geometry::Axes(v)) => {
                                        v.iter().map(value_bytes).sum()
                                    }
                                }
                        })
                        .sum::<usize>()
            })
            .sum::<usize>()
}
fn device_bytes(specs: &Vec<hyprland_editor_core::device::Spec>) -> usize {
    specs.capacity() * size_of::<hyprland_editor_core::device::Spec>()
        + specs
            .iter()
            .map(|s| {
                512 + s.id.capacity()
                    + s.name.capacity()
                    + s.fields
                        .iter()
                        .map(|(k, v)| 256 + size_of::<Value>() + k.capacity() + value_bytes(v))
                        .sum::<usize>()
            })
            .sum::<usize>()
}
fn string_list_bytes(items: &Vec<String>) -> usize {
    items.capacity() * size_of::<String>() + items.iter().map(String::capacity).sum::<usize>()
}
fn bind_bytes(bind: &hyprland_editor_core::binding::Bind) -> usize {
    1024 + bind.id.capacity()
        + string_list_bytes(&bind.chord.tokens)
        + bind.description.as_ref().map_or(0, String::capacity)
        + bind.submap.as_ref().map_or(0, String::capacity)
        + bind.flags.iter().map(|f| 128 + f.capacity()).sum::<usize>()
        + bind
            .device
            .as_ref()
            .map_or(0, |d| string_list_bytes(&d.items))
        + bind.actions.capacity() * size_of::<hyprland_editor_core::action::Action>()
        + bind
            .actions
            .iter()
            .map(|a| {
                a.constructor.capacity()
                    + a.variant.capacity()
                    + a.fields
                        .iter()
                        .map(|(k, v)| 256 + size_of::<Value>() + k.capacity() + value_bytes(v))
                        .sum::<usize>()
            })
            .sum::<usize>()
}
fn submap_bytes(m: &hyprland_editor_core::binding::Submap) -> usize {
    256 + m.id.capacity()
        + m.name.capacity()
        + m.parent.as_ref().map_or(0, String::capacity)
        + m.reset.as_ref().map_or(0, String::capacity)
}
fn bindings_bytes(b: &hyprland_editor_core::binding::Draft) -> usize {
    b.legacy.as_ref().map_or(
        0,
        hyprland_editor_core::legacy_binding::Draft::retained_bytes,
    ) + string_list_bytes(&b.order)
        + b.binds.capacity() * size_of::<hyprland_editor_core::binding::Bind>()
        + b.binds.iter().map(bind_bytes).sum::<usize>()
        + b.submaps.capacity() * size_of::<hyprland_editor_core::binding::Submap>()
        + b.submaps.iter().map(submap_bytes).sum::<usize>()
}
fn session_bytes(draft: &hyprland_editor_core::session::Draft) -> usize {
    use hyprland_editor_core::session::Entry;
    draft.entries.capacity() * size_of::<Entry>()
        + draft
            .entries
            .iter()
            .map(|e| match e {
                Entry::Environment { name, value, .. } => name.capacity() + value.capacity(),
                Entry::Command { command, .. } | Entry::LegacyCommand { command, .. } => {
                    command.capacity()
                }
                Entry::Permission { binary, .. } => binary.capacity(),
            })
            .sum::<usize>()
}
fn document_bytes(doc: &LuaDocument) -> usize {
    let r = &doc.report;
    // Include Arc headers and conservative slack for partly filled B-tree nodes.
    size_of::<LuaDocument>()
        + 1056
        + doc.source.capacity()
        + monitor_bytes(&r.monitors)
        + device_bytes(&r.devices)
        + bindings_bytes(&r.bindings)
        + workspace_bytes(&r.workspace_rules)
        + rule_bytes(&r.rules)
        + animation_bytes(&r.animations)
        + gesture_bytes(&r.gestures)
        + session_bytes(&r.session)
        + r.source_sha256.capacity()
        + r.draft.version.capacity()
        + r.draft
            .values
            .iter()
            .map(|(key, value)| 256 + size_of::<Value>() + key.capacity() + value_bytes(value))
            .sum::<usize>()
        + r.declarations.capacity() * size_of::<hyprland_editor_core::lua::Declaration>()
        + r.declarations
            .iter()
            .map(|d| d.id.capacity() + d.value.as_ref().map_or(0, value_bytes))
            .sum::<usize>()
        + r.diagnostics.capacity() * size_of::<hyprland_editor_core::lua::Diagnostic>()
        + r.diagnostics
            .iter()
            .map(|d| d.message.capacity())
            .sum::<usize>()
}
impl Undo {
    fn bytes(&self) -> usize {
        size_of::<Self>()
            + match self {
                Self::Field { id, previous } => {
                    id.capacity() + previous.as_ref().map_or(0, value_bytes)
                }
                Self::Source {
                    changes, origin, ..
                } => {
                    changes.capacity() * size_of::<(String, Option<Value>)>()
                        + changes_bytes(changes)
                        + origin_bytes(origin)
                }
                Self::Graph {
                    changes,
                    retained_bytes,
                    source,
                    ..
                } => {
                    *retained_bytes
                        + changes.capacity() * size_of::<(String, Option<Value>)>()
                        + changes_bytes(changes)
                        + source.as_ref().map_or(0, |s| origin_bytes(&s.origin))
                }
                Self::Session { retained_bytes, .. }
                | Self::Gestures { retained_bytes, .. }
                | Self::Animations { retained_bytes, .. }
                | Self::Rules { retained_bytes, .. }
                | Self::Bindings { retained_bytes, .. }
                | Self::Workspaces { retained_bytes, .. } => *retained_bytes,
                Self::Monitors { retained_bytes, .. } | Self::Devices { retained_bytes, .. } => {
                    *retained_bytes
                }
            }
    }
}
#[derive(Default)]
pub(super) struct History {
    entries: VecDeque<Undo>,
}
impl History {
    #[cfg(test)]
    pub(super) fn previous_graph(&self) -> Option<Option<String>> {
        match self.entries.back() {
            Some(Undo::Graph { previous, .. }) => {
                Some(previous.as_ref().map(|g| g.fingerprint.clone()))
            }
            _ => None,
        }
    }
    pub fn graph(
        &mut self,
        previous: Option<Arc<super::source_graph::Saved>>,
        changes: Vec<(String, Option<Value>)>,
        source: Option<(Arc<LuaDocument>, SourceIdentity)>,
    ) -> Result<(), String> {
        let retained_bytes = previous.as_ref().map_or(0, |g| g.history_cost);
        self.push(Undo::Graph {
            previous,
            source: source.map(|(document, origin)| SourceInverse {
                document_bytes: document_bytes(&document),
                document,
                origin,
            }),
            changes,
            monitor_changed: false,
            previous_monitors: None,
            retained_bytes,
        })
    }
    pub fn graph_monitors(
        &mut self,
        previous: Option<Arc<super::source_graph::Saved>>,
        monitors: Option<Vec<hyprland_editor_core::monitor::Rule>>,
    ) -> Result<(), String> {
        let retained_bytes = previous.as_ref().map_or(0, |g| g.history_cost)
            + monitors.as_ref().map_or(0, monitor_bytes);
        self.push(Undo::Graph {
            previous,
            source: None,
            changes: vec![],
            monitor_changed: true,
            previous_monitors: monitors,
            retained_bytes,
        })
    }
    pub fn session(
        &mut self,
        previous: hyprland_editor_core::session::Draft,
    ) -> Result<(), String> {
        let retained_bytes = session_bytes(&previous);
        self.push(Undo::Session {
            previous,
            retained_bytes,
        })
    }
    pub fn gestures(
        &mut self,
        previous: hyprland_editor_core::gesture::Draft,
    ) -> Result<(), String> {
        let retained_bytes = gesture_bytes(&previous);
        self.push(Undo::Gestures {
            previous,
            retained_bytes,
        })
    }
    pub fn animations(
        &mut self,
        previous: hyprland_editor_core::animation::Draft,
    ) -> Result<(), String> {
        let retained_bytes = animation_bytes(&previous);
        self.push(Undo::Animations {
            previous,
            retained_bytes,
        })
    }
    pub fn rules(&mut self, previous: Vec<hyprland_editor_core::rule::Rule>) -> Result<(), String> {
        let retained_bytes = rule_bytes(&previous);
        self.push(Undo::Rules {
            previous,
            retained_bytes,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn clear(&mut self) {
        self.entries = VecDeque::new();
    }
    pub fn pop(&mut self) -> Option<Undo> {
        self.entries.pop_back()
    }
    fn bytes(&self) -> usize {
        let mut sources = BTreeSet::new();
        // Count retained source allocations once, even when several inverse operations share them.
        self.entries
            .iter()
            .map(|entry| {
                entry.bytes()
                    + match entry {
                        Undo::Source {
                            document,
                            document_bytes,
                            ..
                        } if sources.insert((0, Arc::as_ptr(document) as usize)) => *document_bytes,
                        Undo::Graph {
                            previous, source, ..
                        } => {
                            previous.as_ref().map_or(0, |graph| {
                                if sources.insert((1, Arc::as_ptr(&graph.bundle.sources) as usize))
                                {
                                    graph.shared_cost
                                } else {
                                    0
                                }
                            }) + previous.as_ref().map_or(0, |graph| {
                                if sources.insert((
                                    2,
                                    Arc::as_ptr(&graph.bundle.graph.private.sources) as usize,
                                )) {
                                    graph.private_shared_cost
                                } else {
                                    0
                                }
                            }) + source.as_ref().map_or(0, |s| {
                                if sources.insert((0, Arc::as_ptr(&s.document) as usize)) {
                                    s.document_bytes
                                } else {
                                    0
                                }
                            })
                        }
                        _ => 0,
                    }
            })
            .sum::<usize>()
            + self.entries.capacity() * size_of::<Undo>()
    }
    fn push(&mut self, entry: Undo) -> Result<(), String> {
        let single = entry.bytes()
            + match &entry {
                Undo::Source { document_bytes, .. } => *document_bytes,
                Undo::Graph {
                    previous, source, ..
                } => {
                    previous
                        .as_ref()
                        .map_or(0, |g| g.shared_cost + g.private_shared_cost)
                        + source.as_ref().map_or(0, |s| s.document_bytes)
                }
                _ => 0,
            };
        if single + 2 * size_of::<Undo>() > MAX_BYTES {
            return Err(
                "This inverse operation exceeds the undo memory budget; the draft was not changed"
                    .into(),
            );
        }
        self.entries.push_back(entry);
        if self.bytes() > MAX_BYTES {
            self.entries.shrink_to_fit();
        }
        while self.entries.len() > MAX_OPERATIONS || self.bytes() > MAX_BYTES {
            self.entries.pop_front();
            self.entries.shrink_to_fit();
        }
        Ok(())
    }
    pub fn field(&mut self, id: &str, previous: Option<Value>) -> Result<(), String> {
        self.push(Undo::Field {
            id: id.into(),
            previous,
        })
    }
    pub fn workspaces(
        &mut self,
        previous: Vec<hyprland_editor_core::workspace_rule::Rule>,
    ) -> Result<(), String> {
        let retained_bytes = workspace_bytes(&previous);
        self.push(Undo::Workspaces {
            previous,
            retained_bytes,
        })
    }
    pub fn bindings(
        &mut self,
        before: &hyprland_editor_core::binding::Draft,
        after: &hyprland_editor_core::binding::Draft,
    ) -> Result<(), String> {
        let inverse = hyprland_editor_core::binding::Inverse::between(before, after);
        let retained_bytes =
            1024 + inverse.legacy.as_ref().and_then(Option::as_ref).map_or(
                0,
                hyprland_editor_core::legacy_binding::Draft::retained_bytes,
            ) + inverse
                .binds
                .iter()
                .map(|(id, b)| id.capacity() + b.as_ref().map_or(0, bind_bytes))
                .sum::<usize>()
                + inverse
                    .submaps
                    .iter()
                    .map(|(id, m)| id.capacity() + m.as_ref().map_or(0, submap_bytes))
                    .sum::<usize>()
                + inverse.bind_ids.as_ref().map_or(0, string_list_bytes)
                + inverse.submap_ids.as_ref().map_or(0, string_list_bytes)
                + inverse.order.as_ref().map_or(0, string_list_bytes);
        self.push(Undo::Bindings {
            inverse,
            retained_bytes,
        })
    }
    pub fn devices(
        &mut self,
        previous: Vec<hyprland_editor_core::device::Spec>,
    ) -> Result<(), String> {
        let retained_bytes = device_bytes(&previous);
        self.push(Undo::Devices {
            previous,
            retained_bytes,
        })
    }
    pub fn monitors(
        &mut self,
        previous: Vec<hyprland_editor_core::monitor::Rule>,
    ) -> Result<(), String> {
        let retained_bytes = monitor_bytes(&previous);
        self.push(Undo::Monitors {
            previous,
            retained_bytes,
        })
    }
    pub fn source(
        &mut self,
        before: &Draft,
        after: &Draft,
        document: Arc<LuaDocument>,
        origin: SourceIdentity,
    ) -> Result<(), String> {
        let keys: BTreeSet<_> = before.values.keys().chain(after.values.keys()).collect();
        let changes = keys
            .into_iter()
            .filter(|id| before.values.get(*id) != after.values.get(*id))
            .map(|id| (id.clone(), before.values.get(id).cloned()))
            .collect();
        let document_bytes = document_bytes(&document);
        self.push(Undo::Source {
            changes,
            document,
            origin,
            document_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyprland_editor_core::{catalogue, lua, storage, BASELINE};
    #[test]
    fn count_and_memory_caps_drop_oldest_field_operations() {
        let mut history = History::default();
        for n in 0..1100 {
            history.field("test", Some(Value::Int(n))).unwrap();
        }
        assert_eq!(history.entries.len(), 1000);
        assert!(matches!(
            history.pop(),
            Some(Undo::Field {
                previous: Some(Value::Int(1099)),
                ..
            })
        ));
        history.clear();
        for _ in 0..1000 {
            history
                .field("text", Some(Value::String("x".repeat(16384))))
                .unwrap();
        }
        assert!(history.entries.len() < 1000);
        assert!(history.bytes() <= MAX_BYTES);
    }
    #[test]
    fn source_inverse_retains_only_changed_values_and_shared_document_once() {
        let c = catalogue();
        let source = "hl.config{general={border_size=2}}";
        let doc = Arc::new(
            LuaDocument::from_report(source.into(), lua::parse(source, BASELINE, &c).unwrap(), &c)
                .unwrap(),
        );
        let root = std::env::temp_dir().join(format!(
            "he-history-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("source.lua");
        std::fs::write(&path, source).unwrap();
        let origin = storage::read_source(&path).unwrap().identity;
        let before = doc.report.draft.clone();
        let mut after = before.clone();
        after
            .values
            .insert("general:border_size".into(), Value::Int(5));
        let mut history = History::default();
        for _ in 0..3 {
            history
                .source(&before, &after, doc.clone(), origin.clone())
                .unwrap();
        }
        assert_eq!(Arc::strong_count(&doc), 4);
        history
            .graph(None, vec![], Some((doc.clone(), origin.clone())))
            .unwrap();
        assert_eq!(Arc::strong_count(&doc), 5);
        let expected = history.entries.iter().map(Undo::bytes).sum::<usize>()
            + document_bytes(&doc)
            + history.entries.capacity() * size_of::<Undo>();
        assert_eq!(history.bytes(), expected);
        let Undo::Graph {
            source: Some(source),
            ..
        } = history.pop().unwrap()
        else {
            panic!("combined graph/root inverse")
        };
        assert!(Arc::ptr_eq(&source.document, &doc));
        let Undo::Source {
            changes, document, ..
        } = history.pop().unwrap()
        else {
            panic!("source inverse")
        };
        assert!(Arc::ptr_eq(&document, &doc));
        assert_eq!(
            changes,
            vec![("general:border_size".into(), Some(Value::Int(2)))]
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn monitor_inverse_preserves_rules_and_accounts_for_allocated_text_capacity() {
        let mut history = History::default();
        let rule = hyprland_editor_core::monitor::Rule::new("inverse");
        history.monitors(vec![rule.clone()]).unwrap();
        let Undo::Monitors { previous, .. } = history.pop().unwrap() else {
            panic!("monitor inverse")
        };
        assert_eq!(previous, vec![rule]);
        history.field("kept", None).unwrap();
        let mut oversized = hyprland_editor_core::monitor::Rule::new("too-large");
        oversized.fields.insert(
            "mirror".into(),
            Value::String(String::with_capacity(MAX_BYTES + 1)),
        );
        assert!(history.monitors(vec![oversized]).is_err());
        assert!(matches!(history.pop(),Some(Undo::Field {id,..})if id=="kept"));
    }
    #[test]
    fn combined_monitor_graph_inverse_budgets_before_mutation_and_keeps_absence() {
        let mut history = History::default();
        history.graph_monitors(None, None).unwrap();
        assert!(matches!(
            history.pop(),
            Some(Undo::Graph {
                monitor_changed: true,
                previous_monitors: None,
                previous: None,
                ..
            })
        ));
        history.field("kept", None).unwrap();
        let mut large = hyprland_editor_core::monitor::Rule::new("large");
        large.fields.insert(
            "mirror".into(),
            Value::String(String::with_capacity(MAX_BYTES + 1)),
        );
        assert!(history.graph_monitors(None, Some(vec![large])).is_err());
        assert!(matches!(history.pop(),Some(Undo::Field {id,..}) if id=="kept"));
    }
    #[test]
    fn device_inverse_preserves_order_and_refuses_excess_capacity() {
        let mut history = History::default();
        let a = hyprland_editor_core::device::Spec::new("a", "keyboard");
        let b = hyprland_editor_core::device::Spec::new("b", "pointer");
        history.devices(vec![a.clone(), b.clone()]).unwrap();
        let Undo::Devices { previous, .. } = history.pop().unwrap() else {
            panic!("device undo")
        };
        assert_eq!(previous, vec![a, b]);
        history.field("kept", None).unwrap();
        let mut oversized = hyprland_editor_core::device::Spec::new("large", "pointer");
        oversized.fields.insert(
            "tags".into(),
            Value::String(String::with_capacity(MAX_BYTES + 1)),
        );
        assert!(history.devices(vec![oversized]).is_err());
        assert!(matches!(history.pop(),Some(Undo::Field{id,..})if id=="kept"));
    }
    #[test]
    fn binding_inverse_keeps_only_changed_records_and_refuses_oversized_history() {
        let source = "hl.bind('Q',hl.dsp.no_op());hl.bind('W',hl.dsp.no_op())";
        let before = hyprland_editor_core::lua::parse(
            source,
            hyprland_editor_core::BASELINE,
            &hyprland_editor_core::catalogue(),
        )
        .unwrap()
        .bindings;
        let mut after = before.clone();
        after.binds[0].description = Some("updated".into());
        let mut history = History::default();
        history.bindings(&before, &after).unwrap();
        let Undo::Bindings { inverse, .. } = history.pop().unwrap() else {
            panic!("binding inverse")
        };
        assert_eq!(inverse.binds.len(), 1);
        assert!(inverse.bind_ids.is_none());
        inverse.restore(&mut after);
        assert_eq!(after, before);
        history.field("kept", None).unwrap();
        let mut oversized = before.clone();
        oversized.binds[0].description = Some("x".repeat(MAX_BYTES));
        assert!(history.bindings(&oversized, &before).is_err());
        assert!(matches!(history.pop(),Some(Undo::Field{id,..})if id=="kept"));
    }
    #[test]
    fn oversized_inverse_is_refused_without_discarding_existing_history() {
        let mut history = History::default();
        history.field("first", None).unwrap();
        assert!(history
            .field("too-large", Some(Value::String("x".repeat(MAX_BYTES))))
            .is_err());
        assert_eq!(history.entries.len(), 1);
    }
    #[test]
    fn graph_and_override_inverse_budget_failure_preserves_existing_history() {
        let mut history = History::default();
        history.field("previous", Some(Value::Int(3))).unwrap();
        assert!(history
            .graph(
                None,
                vec![("large".into(), Some(Value::String("x".repeat(MAX_BYTES))))],
                None,
            )
            .is_err());
        assert!(
            matches!(history.pop(),Some(Undo::Field{id,previous:Some(Value::Int(3))}) if id=="previous")
        );
        assert!(history.is_empty());
    }
}
