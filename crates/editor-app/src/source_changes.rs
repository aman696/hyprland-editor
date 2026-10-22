//! Private typed source comparisons and guarded draft revisions. No filesystem writer.
use super::{label, source_analysis::Model, source_graph::Analysis, worker, workflow};
use adw::prelude::*;
use hyprland_editor_core::{
    lua::{
        replay::scalar_edit::{Change, Comparison, Selection},
        scalar_codec::{Codec, Outcome, Representation, StringValidator},
    },
    source::display_path_bytes,
    Value,
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
};
pub(super) type Getter = Rc<dyn Fn() -> Result<Value, String>>;
pub(super) type RequestGetter = Rc<dyn Fn() -> Result<worker::Request, String>>;
pub(super) type Publish = Rc<dyn Fn(Option<RequestGetter>, &str)>;
type CustomForm = Box<dyn FnOnce(&gtk::Box, Rc<dyn Fn()>, Publish)>;
static COMPARING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
static HOLD_COMPARISON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
pub(super) struct HeldComparison;
#[cfg(test)]
pub(super) fn hold_comparisons() -> HeldComparison {
    HOLD_COMPARISON.store(true, std::sync::atomic::Ordering::Release);
    HeldComparison
}
#[cfg(test)]
impl Drop for HeldComparison {
    fn drop(&mut self) {
        HOLD_COMPARISON.store(false, std::sync::atomic::Ordering::Release);
    }
}
pub(super) struct Permit;
impl Permit {
    pub(super) fn acquire() -> Result<Self, String> {
        COMPARING
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .map(|_| Self)
            .map_err(|_| "A source operation is still finishing. Try again shortly.".into())
    }
    #[cfg(test)]
    pub(super) fn running() -> bool {
        COMPARING.load(std::sync::atomic::Ordering::Acquire)
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        COMPARING.store(false, std::sync::atomic::Ordering::Release);
    }
}
fn same_input(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        (Value::Vec2(a), Value::Vec2(b)) => {
            a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
        }
        (
            Value::Gradient {
                colors: a,
                angle: x,
            },
            Value::Gradient {
                colors: b,
                angle: y,
            },
        ) => a == b && x.to_bits() == y.to_bits(),
        _ => a == b,
    }
}
struct Session {
    epoch: u64,
    pending: Option<u64>,
    closed: bool,
    blocked: bool,
    rows: Vec<Pick>,
    action: Action,
    event: usize,
    monitor: bool,
    getter: Option<Getter>,
    custom: Option<RequestGetter>,
    initial: Option<Value>,
    result: Option<Box<Comparison>>,
}
#[derive(Clone)]
enum Pick {
    Scalar(usize, usize),
    Monitor {
        batch: usize,
        row: Option<usize>,
        field: String,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Set,
    Add,
    Remove,
}
struct Target {
    id: String,
    location: Vec<usize>,
    outcome: Option<Outcome>,
    expression: Option<u32>,
    ambiguous: bool,
    codec: Option<Codec>,
    present: bool,
    known: bool,
    raw_key: Vec<u8>,
}
fn target(analysis: &Analysis, pick: &Pick) -> Option<Target> {
    match pick {
        Pick::Monitor { batch, row, field } => {
            use hyprland_editor_core::lua::{monitor_inspection, replay::Observed};
            let batch = analysis.monitors.as_ref()?.batches.get(*batch)?;
            let event = analysis
                .report
                .events
                .iter()
                .find(|e| e.order == batch.event_order)?;
            let candidate = row.and_then(|row| batch.candidates.iter().find(|c| c.row == row));
            let metadata = monitor_inspection::fields(&analysis.program.version)
                .ok()?
                .iter()
                .find(|f| f.key == *field);
            let raw_key = if let Some(row) = row {
                let Observed::Table(rows) = event.arguments.first()? else {
                    return None;
                };
                let (Observed::Bytes(key), _) = rows.get(*row)? else {
                    return None;
                };
                key.clone()
            } else {
                field.as_bytes().to_vec()
            };
            let output = field == "output" && (row.is_none() || raw_key == b"output");
            let outcome = if output {
                batch.output_value.clone()
            } else {
                candidate.map(|c| c.primary_value.clone())
            };
            Some(Target {
                id: field.clone(),
                location: row.map_or_else(Vec::new, |row| vec![0, row]),
                outcome,
                expression: row
                    .and_then(|row| event.argument_sources.first()?.rows.get(row)?.expression),
                ambiguous: candidate.is_some_and(|c| c.ambiguous),
                codec: if output {
                    Some(Codec::String { validator: None })
                } else {
                    metadata.and_then(|f| monitor_inspection::codec(f).ok())
                },
                present: row.is_some(),
                known: output || metadata.is_some(),
                raw_key,
            })
        }
        Pick::Scalar(b, c) => {
            let candidate = analysis.config_batches.get(*b)?.candidates.get(*c)?;
            Some(Target {
                id: candidate.setting_id.clone(),
                location: candidate.location.clone(),
                outcome: candidate.model_value.clone(),
                expression: candidate.expression,
                ambiguous: candidate.ambiguous,
                codec: hyprland_editor_core::lua::registration::by_lua_key(
                    &analysis.program.version,
                    &candidate.lua_key,
                )
                .and_then(|e| e.codec.clone()),
                present: true,
                known: true,
                raw_key: vec![],
            })
        }
    }
}
fn picks(analysis: &Analysis, event: usize, monitor: bool) -> Vec<Pick> {
    if !monitor {
        return analysis
            .config_batches
            .iter()
            .enumerate()
            .filter(|(_, b)| b.event_order == event)
            .flat_map(|(b, batch)| (0..batch.candidates.len()).map(move |c| Pick::Scalar(b, c)))
            .collect();
    }
    use hyprland_editor_core::lua::{monitor_inspection, replay::Observed};
    let Some(inspection) = &analysis.monitors else {
        return vec![];
    };
    let Some((b, batch)) = inspection
        .batches
        .iter()
        .enumerate()
        .find(|(_, b)| b.event_order == event)
    else {
        return vec![];
    };
    let Some(event) = analysis.report.events.iter().find(|e| e.order == event) else {
        return vec![];
    };
    let Some(Observed::Table(rows)) = event.arguments.first() else {
        return vec![];
    };
    let output = rows
        .iter()
        .position(|(key, _)| matches!(key,Observed::Bytes(k) if k==b"output"));
    let mut picks = vec![Pick::Monitor {
        batch: b,
        row: output,
        field: "output".into(),
    }];
    picks.extend(batch.candidates.iter().map(|c| Pick::Monitor {
        batch: b,
        row: Some(c.row),
        field: c.field.clone(),
    }));
    let fields = monitor_inspection::fields(&analysis.program.version).unwrap_or_default();
    // The handler rejects a missing/bad output before dispatching any fields.
    // Their raw presence still matters for repair; do not offer duplicate adds.
    for (row, (key, _)) in rows.iter().enumerate() {
        if batch.candidates.iter().any(|c| c.row == row) {
            continue;
        }
        let Observed::Bytes(key) = key else {
            continue;
        };
        let native = key.split(|b| *b == 0).next().unwrap_or_default();
        if let Some(field) = fields.iter().find(|f| f.key.as_bytes() == native) {
            picks.push(Pick::Monitor {
                batch: b,
                row: Some(row),
                field: field.key.clone(),
            });
        }
    }
    for field in fields {
        if !picks
            .iter()
            .any(|p| matches!(p,Pick::Monitor {field:name,..} if *name==field.key))
        {
            picks.push(Pick::Monitor {
                batch: b,
                row: None,
                field: field.key.clone(),
            });
        }
    }
    for (row, (key, _)) in rows.iter().enumerate() {
        if Some(row) == output
            || picks
                .iter()
                .any(|p| matches!(p,Pick::Monitor {row:Some(r),..} if *r==row))
        {
            continue;
        }
        if let Observed::Bytes(key) = key {
            picks.push(Pick::Monitor {
                batch: b,
                row: Some(row),
                field: display_path_bytes(&key[..key.len().min(128)]),
            });
        }
    }
    picks
}
// Monitor builders construct all child controls up front. Value changes must
// invalidate an in-flight comparison just like the scalar controls do.
fn watch_inputs(root: &gtk::Widget, changed: &Rc<dyn Fn()>) {
    if let Some(entry) = root.downcast_ref::<gtk::Entry>() {
        let f = changed.clone();
        entry.connect_changed(move |_| f());
    }
    if let Some(spin) = root.downcast_ref::<gtk::SpinButton>() {
        let f = changed.clone();
        spin.connect_value_changed(move |_| f());
    }
    if let Some(pick) = root.downcast_ref::<gtk::DropDown>() {
        let f = changed.clone();
        pick.connect_selected_notify(move |_| f());
    }
    if let Some(toggle) = root.downcast_ref::<gtk::Switch>() {
        let f = changed.clone();
        toggle.connect_active_notify(move |_| f());
    }
    if let Some(toggle) = root.downcast_ref::<gtk::CheckButton>() {
        let f = changed.clone();
        toggle.connect_toggled(move |_| f());
    }
    let mut child = root.first_child();
    while let Some(widget) = child {
        watch_inputs(&widget, changed);
        child = widget.next_sibling();
    }
}
#[derive(Clone)]
struct Controls {
    state: Weak<RefCell<super::State>>,
    refresh: Rc<dyn Fn()>,
    session: Weak<RefCell<Session>>,
    model: Weak<RefCell<Option<Arc<Analysis>>>>,
    analysis: std::sync::Weak<Analysis>,
    targets: gtk::glib::WeakRef<gtk::DropDown>,
    scope: gtk::glib::WeakRef<gtk::DropDown>,
    scope_label: gtk::glib::WeakRef<gtk::Label>,
    operation: gtk::glib::WeakRef<gtk::DropDown>,
    field: gtk::glib::WeakRef<gtk::Box>,
    input_expander: gtk::glib::WeakRef<gtk::Expander>,
    status: gtk::glib::WeakRef<gtk::Label>,
    output: gtk::glib::WeakRef<gtk::TextView>,
    compare: gtk::glib::WeakRef<gtk::Button>,
    keep: gtk::glib::WeakRef<gtk::Button>,
}
pub(super) fn input_form(
    initial: &Value,
    codec: Option<&Codec>,
    body: &gtk::Box,
    changed: Rc<dyn Fn()>,
) -> Getter {
    match initial {
        Value::Bool(value) => {
            let input = gtk::Switch::builder()
                .active(*value)
                .halign(gtk::Align::Start)
                .build();
            input.set_widget_name("source-change-value");
            body.append(&input);
            input.update_property(&[gtk::accessible::Property::Label("Proposed value")]);
            input.connect_active_notify(move |_| changed());
            Rc::new(move || Ok(Value::Bool(input.is_active())))
        }
        Value::Int(_) | Value::Float(_) | Value::FontWeight(_) => {
            let weight = matches!(initial, Value::FontWeight(_));
            let integer = !matches!(initial, Value::Float(_));
            let initial = match initial {
                Value::Int(v) | Value::FontWeight(v) => v.to_string(),
                Value::Float(v) => format!("{v:?}"),
                _ => unreachable!(),
            };
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let minus = gtk::Button::from_icon_name("list-remove-symbolic");
            let plus = gtk::Button::from_icon_name("list-add-symbolic");
            minus.set_tooltip_text(Some("Decrease by one"));
            plus.set_tooltip_text(Some("Increase by one"));
            minus.update_property(&[gtk::accessible::Property::Label("Decrease by one")]);
            plus.update_property(&[gtk::accessible::Property::Label("Increase by one")]);
            let input = gtk::Entry::builder()
                .text(initial)
                .hexpand(true)
                .max_length(128)
                .input_purpose(gtk::InputPurpose::Number)
                .build();
            input.set_widget_name("source-change-value");
            input.update_property(&[gtk::accessible::Property::Label("Proposed numeric value")]);
            row.append(&minus);
            row.append(&input);
            row.append(&plus);
            body.append(&row);
            if let Some(Codec::Int {
                names: Some(names), ..
            }) = codec
            {
                let values = names
                    .iter()
                    .map(|n| (display_path_bytes(&n.name), n.value))
                    .collect::<Vec<_>>();
                let labels = std::iter::once("Choose a named value".to_string())
                    .chain(
                        values
                            .iter()
                            .map(|(name, value)| format!("{name} · {value}")),
                    )
                    .collect::<Vec<_>>();
                let pick = gtk::DropDown::from_strings(
                    &labels.iter().map(String::as_str).collect::<Vec<_>>(),
                );
                super::monitors::compact(&pick);
                pick.update_property(&[gtk::accessible::Property::Label("Named numeric value")]);
                body.append(&pick);
                let input = input.downgrade();
                pick.connect_selected_notify(move |pick| {
                    if let Some((_, value)) = pick
                        .selected()
                        .checked_sub(1)
                        .and_then(|i| values.get(i as usize))
                    {
                        if let Some(input) = input.upgrade() {
                            input.set_text(&value.to_string());
                        }
                        pick.set_selected(0);
                    }
                });
            }
            if weight {
                let options = hyprland_editor_core::lua::scalar_codec::complex::FONT_WEIGHTS;
                let names = std::iter::once("Weight presets".into())
                    .chain(
                        options
                            .iter()
                            .map(|(name, n)| format!("{} · {n}", display_path_bytes(name))),
                    )
                    .collect::<Vec<String>>();
                let pick = gtk::DropDown::from_strings(
                    &names.iter().map(String::as_str).collect::<Vec<_>>(),
                );
                super::monitors::compact(&pick);
                pick.update_property(&[gtk::accessible::Property::Label("Font weight presets")]);
                body.append(&pick);
                let entry = input.downgrade();
                pick.connect_selected_notify(move |pick| {
                    if let Some((_, v)) = pick
                        .selected()
                        .checked_sub(1)
                        .and_then(|i| options.get(i as usize))
                    {
                        if let Some(entry) = entry.upgrade() {
                            entry.set_text(&v.to_string());
                        }
                        pick.set_selected(0);
                    }
                });
            }
            for (button, delta) in [(minus, -1), (plus, 1)] {
                let input = input.downgrade();
                button.connect_clicked(move |_| {
                    let Some(input) = input.upgrade() else {
                        return;
                    };
                    let text = if integer {
                        input
                            .text()
                            .parse::<i64>()
                            .ok()
                            .and_then(|v| v.checked_add(delta))
                            .filter(|v| !weight || *v >= 0)
                            .map(|v| v.to_string())
                    } else {
                        input
                            .text()
                            .parse::<f64>()
                            .ok()
                            .map(|v| v + delta as f64)
                            .filter(|v| v.is_finite())
                            .map(|v| format!("{v:?}"))
                    };
                    if let Some(text) = text {
                        input.set_text(&text);
                    }
                });
            }
            input.connect_changed(move |_| changed());
            Rc::new(move || {
                if integer {
                    let value = input
                        .text()
                        .parse::<i64>()
                        .map_err(|_| "Enter a whole number within the signed 64-bit range")?;
                    if weight && value < 0 {
                        return Err("Font weight must not be negative".into());
                    }
                    Ok(if weight {
                        Value::FontWeight(value)
                    } else {
                        Value::Int(value)
                    })
                } else {
                    input
                        .text()
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .map(Value::Float)
                        .ok_or_else(|| "Enter a finite number".into())
                }
            })
        }
        Value::String(value) => {
            if let Some(Codec::String {
                validator: Some(StringValidator::Choices(choices)),
            }) = codec
            {
                let values = choices
                    .choices
                    .iter()
                    .map(|s| {
                        std::str::from_utf8(s)
                            .ok()
                            .filter(|s| s.len() <= 16384 && !s.contains('\0'))
                            .map(String::from)
                    })
                    .collect::<Option<Vec<_>>>();
                if let Some(values) = values.filter(|v| !v.is_empty() && v.contains(value)) {
                    let pick = gtk::DropDown::from_strings(
                        &values.iter().map(String::as_str).collect::<Vec<_>>(),
                    );
                    super::monitors::compact(&pick);
                    pick.set_selected(values.iter().position(|v| v == value).unwrap() as u32);
                    pick.set_widget_name("source-change-value");
                    pick.update_property(&[gtk::accessible::Property::Label("Proposed choice")]);
                    body.append(&pick);
                    pick.connect_selected_notify(move |_| changed());
                    return Rc::new(move || {
                        values
                            .get(pick.selected() as usize)
                            .cloned()
                            .map(Value::String)
                            .ok_or_else(|| "Choose a supported value".into())
                    });
                }
            }
            let input = gtk::Entry::builder()
                .text(value)
                .max_length(16384)
                .hexpand(true)
                .build();
            input.set_widget_name("source-change-value");
            input.update_property(&[gtk::accessible::Property::Label("Proposed text value")]);
            body.append(&input);
            input.connect_changed(move |_| changed());
            Rc::new(move || Ok(Value::String(input.text().into())))
        }
        _ => super::source_compound_forms::form(initial, body, changed),
    }
}
impl Controls {
    fn current(&self) -> bool {
        let Some(original) = self.analysis.upgrade() else {
            return false;
        };
        self.model
            .upgrade()
            .and_then(|m| m.borrow().clone())
            .is_some_and(|a| Arc::ptr_eq(&a, &original))
    }
    fn message(&self, text: &str) {
        if let Some(status) = self.status.upgrade() {
            status.set_text(text);
        }
    }
    fn sensitive(&self) {
        let ready = self.session.upgrade().is_some_and(|s| {
            let s = s.borrow();
            !s.closed
                && !s.blocked
                && s.pending.is_none()
                && (s.custom.is_some() || s.getter.is_some() || s.action == Action::Remove)
        });
        if let Some(button) = self.compare.upgrade() {
            button.set_sensitive(ready);
        }
        let keep = self.session.upgrade().is_some_and(|s| {
            let s = s.borrow();
            !s.closed && !s.blocked && s.pending.is_none() && s.result.is_some()
        });
        if let Some(button) = self.keep.upgrade() {
            button.set_sensitive(keep);
        }
    }
    fn invalidate(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let mut s = session.borrow_mut();
        s.epoch += 1;
        s.result = None;
        let pending = s.pending.is_some();
        drop(s);
        if let Some(output) = self.output.upgrade() {
            output.buffer().set_text("");
        }
        self.message(if pending {
            "Values changed while the comparison is running. Its result will be discarded."
        } else {
            "Choose a value, then compare the proposed source change."
        });
        self.sensitive();
    }
    fn stale(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let mut s = session.borrow_mut();
        s.blocked = true;
        s.epoch += 1;
        s.getter = None;
        s.custom = None;
        s.result = None;
        drop(s);
        for weak in [&self.targets, &self.scope, &self.operation] {
            if let Some(w) = weak.upgrade() {
                w.set_sensitive(false);
            }
        }
        if let Some(field) = self.field.upgrade() {
            field.set_sensitive(false);
        }
        if let Some(output) = self.output.upgrade() {
            output.buffer().set_text("");
        }
        self.message("The source analysis changed. Close this comparison and open it from the current analysis.");
        self.sensitive();
    }
    fn choose(&self) {
        let (Some(session), Some(analysis), Some(pick), Some(operation)) = (
            self.session.upgrade(),
            self.analysis.upgrade(),
            self.targets.upgrade(),
            self.operation.upgrade(),
        ) else {
            return;
        };
        let candidate = session
            .borrow()
            .rows
            .get(pick.selected() as usize)
            .and_then(|p| target(&analysis, p));
        let labels = match candidate {
            Some(c) if !c.known => vec!["Remove field"],
            Some(c) if !c.present => vec!["Add field"],
            Some(c) if c.raw_key != b"output" && session.borrow().monitor => {
                vec!["Set value", "Remove field"]
            }
            _ => vec!["Set value"],
        };
        operation.set_model(Some(&gtk::StringList::new(&labels)));
        operation.set_selected(0);
        self.select();
    }
    fn select(&self) {
        if let Some(expander) = self.input_expander.upgrade() {
            expander.set_expanded(true);
        }
        self.invalidate();
        let (Some(session), Some(analysis), Some(pick), Some(field), Some(scope), Some(operation)) = (
            self.session.upgrade(),
            self.analysis.upgrade(),
            self.targets.upgrade(),
            self.field.upgrade(),
            self.scope.upgrade(),
            self.operation.upgrade(),
        ) else {
            return;
        };
        while let Some(child) = field.first_child() {
            field.remove(&child);
        }
        let row = session.borrow().rows.get(pick.selected() as usize).cloned();
        let monitor = session.borrow().monitor;
        let candidate = row.as_ref().and_then(|row| target(&analysis, row));
        let action = match &candidate {
            Some(c)
                if monitor
                    && (!c.known
                        || c.present && c.raw_key != b"output" && operation.selected() == 1) =>
            {
                Action::Remove
            }
            Some(c) if monitor && !c.present => Action::Add,
            _ => Action::Set,
        };
        {
            let mut s = session.borrow_mut();
            s.getter = None;
            s.initial = None;
            s.action = action;
        }
        scope.set_sensitive(action != Action::Remove);
        scope.set_visible(action != Action::Remove);
        if let Some(label) = self.scope_label.upgrade() {
            label.set_visible(action != Action::Remove);
        }
        if let Some(expander) = self.input_expander.upgrade() {
            expander.set_label(Some(if action == Action::Remove {
                "Removal details"
            } else {
                "Proposed value"
            }));
        }
        if action == Action::Remove {
            scope.set_model(Some(&gtk::StringList::new(&["Recorded field write"])));
            scope.set_selected(0);
            field.append(&label("Remove this explicit field at its recorded write. Comments are retained. This does not force a default or remove other alias spellings; the comparison reports additional effects."));
            self.sensitive();
            return;
        }
        let Some(candidate) = candidate else {
            self.sensitive();
            return;
        };
        if candidate.ambiguous {
            field.append(&label("Several key spellings address this property. Remove the conflicting spelling before setting a value."));
            self.sensitive();
            return;
        }
        let metadata = if monitor {
            hyprland_editor_core::monitor::catalogue(&analysis.program.version)
                .into_iter()
                .find(|f| f.lua_key == candidate.id)
        } else {
            None
        };
        let current = match &candidate.outcome {
            Some(Outcome::Accepted {
                representation: Representation::Value { value },
                ..
            }) => match value {
                Value::String(v) if v.len() > 16384 || v.contains('\0') => None,
                Value::Float(v) if !v.is_finite() => None,
                _ => Some(value.clone()),
            },
            _ => None,
        };
        let current = if monitor && candidate.id != "output" {
            current.filter(|value| {
                let mut rule = hyprland_editor_core::monitor::Rule::new("input");
                rule.fields.insert(candidate.id.clone(), value.clone());
                rule.validate(&analysis.program.version).is_ok()
            })
        } else {
            current
        };
        let initial = if action == Action::Set {
            current.clone()
        } else {
            None
        };
        let value = current.or_else(|| {
            if monitor {
                if candidate.id == "output" {
                    Some(Value::String(String::new()))
                } else {
                    metadata.as_ref().and_then(|f| f.known_default())
                }
            } else {
                None
            }
        });
        let Some(value) = value else {
            field.append(&label(
                "This source value has no supported graphical representation yet.",
            ));
            self.sensitive();
            return;
        };
        if monitor && initial.is_none() && action == Action::Set {
            field.append(&label("The current input cannot be represented by these controls. The value below is a proposed starting value; compare it before keeping a revision."));
        }
        if action == Action::Add {
            scope.set_model(Some(&gtk::StringList::new(&[
                "Selected table constructor",
                "Original constructor, if traceable",
                "Recorded last write, if traceable",
            ])));
            let direct = analysis
                .report
                .events
                .iter()
                .find(|e| e.order == session.borrow().event)
                .and_then(|e| e.argument_sources.first()?.expression)
                .and_then(|i| analysis.report.expression_sources.get(i as usize))
                .is_some_and(|s| {
                    s.kind == hyprland_editor_core::lua::replay::ExpressionKind::Table
                });
            let selected_event = analysis
                .report
                .events
                .iter()
                .find(|e| e.order == session.borrow().event);
            let identity = selected_event
                .and_then(|e| e.argument_sources.first()?.expression)
                .and_then(|id| analysis.report.expression_sources.get(id as usize))
                .and_then(|s| s.table_identity);
            let last=identity.zip(selected_event).is_some_and(|(identity,event)| analysis.report.storage_writes.iter().rev().find(|w| w.at_order<=event.order && matches!(&w.place,hyprland_editor_core::lua::replay::StoragePlace::Table {identity:table,key:hyprland_editor_core::lua::replay::StorageKey::Bytes(key)} if *table==identity && key==candidate.id.as_bytes())).is_some_and(|w| w.deletes_slot));
            scope.set_selected(if last {
                2
            } else if direct {
                0
            } else {
                1
            });
            field.append(&label("This field is absent. The control value is an explicit proposal, not an inferred effective default."));
        } else {
            let table = candidate
                .expression
                .and_then(|id| analysis.report.expression_sources.get(id as usize))
                .is_some_and(|s| {
                    s.kind == hyprland_editor_core::lua::replay::ExpressionKind::Table
                });
            let compound = !monitor
                && matches!(
                    &value,
                    Value::Vec2(_) | Value::CssGap(_) | Value::Gradient { .. }
                );
            let mut labels = vec![
                "Selected value expression",
                "Original literal, if traceable",
            ];
            if compound {
                labels.extend([
                    "Preserve table layout and comments",
                    "Original table layout, if traceable",
                ]);
            }
            scope.set_model(Some(&gtk::StringList::new(&labels)));
            scope.set_selected(if table && compound { 2 } else { 0 });
        }
        let ui = self.clone();
        let changed: Rc<dyn Fn()> = Rc::new(move || ui.invalidate());
        // Keep exact values outside the draft builder's spin ranges/precision.
        // These controls accept typed numbers, not source code or expressions.
        let exact_number = match &value {
            Value::Float(_) => true,
            Value::Int(v) => i32::try_from(*v).is_err(),
            Value::CssGap(v) => v.iter().any(|v| i32::try_from(*v).is_err()),
            _ => false,
        };
        let metadata = if exact_number { None } else { metadata };
        let getter = if let (Some(metadata), Some(window)) =
            (metadata, field.root().and_downcast::<adw::Window>())
        {
            let builder =
                super::monitors::builder(&metadata, Some(value.clone()), &field, &window, &[]);
            watch_inputs(field.upcast_ref(), &changed);
            Rc::from(builder)
        } else {
            input_form(&value, candidate.codec.as_ref(), &field, changed)
        };
        {
            let mut s = session.borrow_mut();
            s.initial = initial;
            s.getter = Some(getter);
        }
        self.sensitive();
    }
    fn start(&self) {
        if !self.current() {
            self.stale();
            return;
        }
        let (Some(session), Some(analysis), Some(pick), Some(scope)) = (
            self.session.upgrade(),
            self.analysis.upgrade(),
            self.targets.upgrade(),
            self.scope.upgrade(),
        ) else {
            return;
        };
        let (epoch, event, row, getter, initial, action, monitor) = {
            let s = session.borrow();
            if s.closed || s.blocked || s.pending.is_some() {
                return;
            }
            (
                s.epoch,
                s.event,
                s.rows.get(pick.selected() as usize).cloned(),
                s.getter.clone(),
                s.initial.clone(),
                s.action,
                s.monitor,
            )
        };
        let custom = session.borrow().custom.clone();
        let request = if let Some(getter) = custom {
            match getter() {
                Ok(request) => request,
                Err(error) => {
                    self.message(&error);
                    return;
                }
            }
        } else {
            let Some(candidate) = row.as_ref().and_then(|p| target(&analysis, p)) else {
                return;
            };
            let value = if action == Action::Remove {
                None
            } else {
                let Some(getter) = getter else {
                    return;
                };
                match getter() {
                    Ok(value) => Some(value),
                    Err(error) => {
                        self.message(&error);
                        return;
                    }
                }
            };
            if action == Action::Set
                && initial
                    .as_ref()
                    .zip(value.as_ref())
                    .is_some_and(|(a, b)| same_input(a, b))
            {
                self.message("Change the value before starting a comparison.");
                return;
            }
            if monitor {
                use hyprland_editor_core::lua::replay::monitor_edit::{
                    self, FieldChange, TableSelection,
                };
                match action {
                    Action::Add => worker::Request::CompareMonitorFieldSource {
                        program: analysis.program.clone(),
                        change: FieldChange::Add {
                            event_order: event,
                            field: candidate.id,
                            value: value.unwrap(),
                            selection: match scope.selected() {
                                1 => TableSelection::OriginalConstructor,
                                2 => TableSelection::LatestWrite,
                                _ => TableSelection::SelectedConstructor,
                            },
                        },
                    },
                    Action::Remove => worker::Request::CompareMonitorFieldSource {
                        program: analysis.program.clone(),
                        change: FieldChange::Remove {
                            event_order: event,
                            row: candidate.location[1],
                            raw_key: candidate.raw_key,
                        },
                    },
                    Action::Set => worker::Request::CompareMonitorSource {
                        program: analysis.program.clone(),
                        change: monitor_edit::Change {
                            event_order: event,
                            row: candidate.location[1],
                            field: candidate.id,
                            selection: if scope.selected() == 1 {
                                monitor_edit::Selection::OriginalLiteral
                            } else {
                                monitor_edit::Selection::SelectedExpression
                            },
                            value: value.unwrap(),
                        },
                    },
                }
            } else {
                worker::Request::CompareScalarSource {
                    program: analysis.program.clone(),
                    change: Change {
                        event_order: event,
                        location: candidate.location,
                        setting_id: candidate.id,
                        selection: match scope.selected() {
                            1 => Selection::OriginalLiteral,
                            2 => Selection::PreserveTableLayout,
                            3 => Selection::OriginalTableLayout,
                            _ => Selection::SelectedExpression,
                        },
                        value: value.unwrap(),
                    },
                }
            }
        };
        let permit = match Permit::acquire() {
            Ok(p) => p,
            Err(e) => {
                self.message(&e);
                return;
            }
        };
        {
            let mut s = session.borrow_mut();
            s.pending = Some(epoch);
            s.result = None;
        }
        if let Some(output) = self.output.upgrade() {
            output.buffer().set_text("");
        }
        self.message("Comparing private source copies…");
        self.sensitive();
        let ui = self.clone();
        // A closed inspector must release its reports/captures immediately.
        // Only the immutable evaluator input is needed by this bounded task.
        workflow::background(
            move || {
                let _permit = permit;
                #[cfg(test)]
                while HOLD_COMPARISON.load(std::sync::atomic::Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                match worker::run(request)? {
                    worker::Response::ScalarSourceComparison(result) => Ok(result),
                    _ => Err("Unexpected source comparison response".into()),
                }
            },
            move |result| {
                let Some(session) = ui.session.upgrade() else {
                    return;
                };
                let mut s = session.borrow_mut();
                if s.closed || s.pending != Some(epoch) {
                    return;
                }
                s.pending = None;
                if !ui.current() {
                    drop(s);
                    ui.stale();
                    return;
                }
                if s.epoch != epoch {
                    drop(s);
                    ui.message("Values changed during comparison. Compare again to use the current controls.");
                    ui.sensitive();
                    return;
                }
                match result {
                    Ok(result) => {
                        let Some(analysis) = ui.analysis.upgrade() else {
                            return;
                        };
                        match describe(&analysis, &result) {
                            Ok(text) => {
                                if let Some(output) = ui.output.upgrade() {
                                    output.buffer().set_text(&text);
                                }
                                if let Some(expander) = ui.input_expander.upgrade() {
                                    expander.set_expanded(false);
                                }
                                ui.message(if result.review.modeled_observations_match {"Only the requested modeled change was observed. Ownership and live behavior are still unverified."}else{"Additional modeled changes or incomplete analysis need review. No source change has been saved."});
                                s.result = Some(result);
                            }
                            Err(error) => ui.message(&error),
                        }
                    }
                    Err(error) => ui.message(&display_path_bytes(
                        &error.as_bytes()[..error.len().min(1024)],
                    )),
                }
                drop(s);
                ui.sensitive();
            },
        );
    }
    fn keep(&self) {
        if !self.current() {
            self.stale();
            return;
        }
        let (Some(session), Some(analysis), Some(state)) = (
            self.session.upgrade(),
            self.analysis.upgrade(),
            self.state.upgrade(),
        ) else {
            return;
        };
        let (epoch, plan) = {
            let s = session.borrow();
            if s.closed || s.blocked || s.pending.is_some() {
                return;
            }
            let Some(result) = &s.result else {
                return;
            };
            (s.epoch, result.plan.clone())
        };
        let bundle = match super::source_graph::freeze(&analysis) {
            Ok(bundle) => bundle,
            Err(error) => {
                self.message(&error);
                return;
            }
        };
        let fingerprint = match bundle.fingerprint() {
            Ok(v) => v,
            Err(error) => {
                self.message(&error);
                return;
            }
        };
        let version = analysis.program.version.clone();
        let (generation, expected, shared_bundle) = {
            let s = state.borrow();
            let root = bundle
                .sources
                .iter()
                .find(|source| source.id == bundle.graph.root);
            if s.draft.version != version
                || root.is_none_or(|root| {
                    s.source_origins.get(&version) != Some(&root.origin)
                        || s.sources
                            .get(&version)
                            .is_none_or(|d| d.source != root.content)
                })
            {
                self.message("Import this captured root at its recorded revision before keeping source changes in the draft.");
                return;
            }
            let expected = s.source_graphs.get(&version).map(|g| g.fingerprint.clone());
            if expected.as_ref().is_some_and(|fp| fp != &fingerprint) {
                self.message("The retained source draft changed. Open its current source view before editing again.");
                return;
            }
            (
                s.generation,
                expected,
                s.source_graphs.get(&version).map(|g| g.bundle.clone()),
            )
        };
        let bundle = shared_bundle.unwrap_or(bundle);
        let permit = match Permit::acquire() {
            Ok(p) => p,
            Err(e) => {
                self.message(&e);
                return;
            }
        };
        session.borrow_mut().pending = Some(epoch);
        self.message("Preparing a private source revision…");
        self.sensitive();
        let ui = self.clone();
        let work_version = version.clone();
        workflow::background(
            move || {
                let _permit = permit;
                super::source_graph::apply_revision(&bundle, &work_version, plan)
            },
            move |result| {
                let (Some(session), Some(state), Some(model)) =
                    (ui.session.upgrade(), ui.state.upgrade(), ui.model.upgrade())
                else {
                    return;
                };
                let mut s = session.borrow_mut();
                if s.closed || s.pending != Some(epoch) {
                    return;
                }
                s.pending = None;
                if s.epoch != epoch || !ui.current() {
                    drop(s);
                    ui.message("The comparison changed before the source revision was kept. Compare again.");
                    ui.sensitive();
                    return;
                }
                let mut draft = state.borrow_mut();
                if draft.generation != generation
                    || draft.draft.version != version
                    || draft
                        .source_graphs
                        .get(&version)
                        .map(|g| g.fingerprint.clone())
                        != expected
                {
                    drop(draft);
                    drop(s);
                    ui.message("The draft changed while preparing this revision. No source change was kept.");
                    ui.sensitive();
                    return;
                }
                match result {
                    Ok((saved, analysis)) => match draft.keep_graph(saved) {
                        Ok(()) => {
                            *model.borrow_mut() = Some(Arc::new(analysis));
                            drop(draft);
                            s.result = None;
                            drop(s);
                            if let Some(status) = state.borrow().main_status.upgrade() {
                                super::summary(&state, &status);
                            }
                            (ui.refresh)();
                        }
                        Err(error) => {
                            drop(draft);
                            drop(s);
                            ui.message(&error);
                            ui.sensitive();
                        }
                    },
                    Err(error) => {
                        drop(draft);
                        drop(s);
                        ui.message(&display_path_bytes(
                            &error.as_bytes()[..error.len().min(1024)],
                        ));
                        ui.sensitive();
                    }
                }
            },
        );
    }
}
fn describe(analysis: &Analysis, result: &Comparison) -> Result<String, String> {
    use std::fmt::Write;
    if result.plan.edits.is_empty()
        || result.plan.edits.len() > 512
        || result.review.ownership_verified
        || result.review.runtime_verified
        || result.review.base_program_sha256 != result.plan.base_program_sha256
        || result.review.schema_sha256 != result.plan.schema_sha256
    {
        return Err(
            "The comparison response has inconsistent source identity or verification claims."
                .into(),
        );
    }
    let mut text = String::from("Proposed source change\n");
    for (index, edit) in result.plan.edits.iter().enumerate() {
        let document = analysis
            .program
            .documents
            .iter()
            .find(|d| d.id == edit.document)
            .ok_or("The proposed source document is unavailable")?;
        let old = document
            .source
            .get(edit.span.clone())
            .ok_or("The proposed source range is invalid")?;
        if edit.base_sha256 != document.sha256
            || edit.old_bytes_sha256 != hyprland_editor_core::lua::source_hash(old)
        {
            return Err("The proposed source bytes no longer match this analysis".into());
        }
        if text.len() < 24 * 1024 {
            let line = document.source[..edit.span.start]
                .bytes()
                .filter(|b| *b == b'\n')
                .count()
                + 1;
            if result.plan.edits.len() > 1 {
                let _ = writeln!(text, "\nPatch {} of {}", index + 1, result.plan.edits.len());
            }
            let _ = writeln!(
                text,
                "Before\n{}\n\nAfter\n{}\n\nSource: {} · line {line}\n",
                if old.is_empty() {
                    "(insertion point)".into()
                } else {
                    super::source_access::excerpt(old)
                },
                if edit.replacement.trim().is_empty() {
                    "(removed)".into()
                } else {
                    super::source_access::excerpt(&edit.replacement)
                },
                display_path_bytes(&document.load_name)
            );
        } else if index + 1 == result.plan.edits.len() {
            text.push_str("\n… Additional patches are retained but omitted from this excerpt.\n");
        }
    }
    if let Outcome::Accepted {
        representation: Representation::Value { value },
        ..
    } = &result.proposed_native_value
    {
        let _ = writeln!(
            text,
            "\nModeled parsed value: {}",
            super::source_access::excerpt(&value.lua())
        );
    }
    if let Outcome::Unresolved { reason } = &result.proposed_native_value {
        let _ = writeln!(text, "\n{}", display_path_bytes(reason.as_bytes()));
    }
    if result.table_layout_preserved {
        text.push_str("\nExisting comments, spacing and unrelated source fields are retained. Changed components and added/removed fields are shown above.\n");
    } else if result
        .plan
        .edits
        .iter()
        .any(|edit| edit.replacement.starts_with('{'))
    {
        text.push_str("\nThis replaces the selected value expression. Comments and extra fields inside that expression are included in the replacement.\n");
    }
    if result.review.differences.is_empty() {
        text.push_str(
            "\nThe other modeled calls, loader inputs and executed branch/loop outcomes match.\n",
        );
    } else {
        text.push_str("\nAdditional modeled findings\n");
        for finding in result.review.differences.iter().take(16) {
            use hyprland_editor_core::lua::replay::edits::DifferenceKind;
            let description = match finding.kind {
                DifferenceKind::CallSiteOrApi => "A declaration changed",
                DifferenceKind::RequestedDeclaration => {
                    "The requested declaration was not reached or did not match"
                }
                DifferenceKind::Arguments => "Other setting values changed",
                DifferenceKind::MissingCall => "A declaration no longer runs",
                DifferenceKind::AddedCall => "An additional declaration runs",
                DifferenceKind::Loader => "A module load changed",
                DifferenceKind::PathHistory => "Branch or loop behavior changed",
                DifferenceKind::ReturnValues => "A returned value changed",
                DifferenceKind::LoggedModuleErrors => "Module error handling changed",
                DifferenceKind::IncompleteModel => {
                    "Analysis could not cover the full configuration"
                }
            };
            writeln!(
                text,
                "{}{}",
                description,
                finding
                    .order
                    .map_or(String::new(), |n| format!(" · call {}", n + 1))
            )
            .unwrap();
            for (title, values) in [
                ("Before", &finding.before_arguments),
                ("After", &finding.after_arguments),
            ] {
                if let Some(values) = values {
                    write!(text, "{title}: ").unwrap();
                    for value in values.iter().take(4) {
                        super::source_analysis::value(&mut text, value, 0);
                        text.push(' ');
                    }
                    text.push('\n');
                }
            }
            if text.len() > 24 * 1024 {
                text.push_str("… More findings are omitted from this excerpt.\n");
                break;
            }
        }
        if result.review.differences.len() > 16 {
            text.push_str("… More findings are omitted from this excerpt.\n");
        }
    }
    text.push_str("\nThis compares a supported source model. Complete source ownership, future effects and compositor behavior are not verified. Original files and drafts remain unchanged.");
    if text.len() > 32 * 1024 {
        let mut end = 32 * 1024;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n… Comparison excerpt truncated.");
    }
    Ok(text)
}
pub(super) fn open(
    parent: &gtk::Window,
    model: &Model,
    event: usize,
    state: &super::Shared,
    refresh: Rc<dyn Fn()>,
) -> adw::Window {
    open_impl(parent, model, Some(event), state, refresh, None)
}
pub(super) fn open_custom(
    parent: &gtk::Window,
    model: &Model,
    state: &super::Shared,
    refresh: Rc<dyn Fn()>,
    build: CustomForm,
) -> adw::Window {
    open_impl(parent, model, None, state, refresh, Some(build))
}
fn open_impl(
    parent: &gtk::Window,
    model: &Model,
    event: Option<usize>,
    state: &super::Shared,
    refresh: Rc<dyn Fn()>,
    custom: Option<CustomForm>,
) -> adw::Window {
    let is_custom = custom.is_some();
    let event = event.unwrap_or_default();
    let analysis = model
        .borrow()
        .clone()
        .expect("Analysis selected before opening source comparison");
    let monitor = is_custom
        || analysis
            .report
            .events
            .iter()
            .any(|e| e.order == event && e.api == "hl.monitor");
    let rows = if is_custom {
        vec![]
    } else {
        picks(&analysis, event, monitor)
    };
    let session = Rc::new(RefCell::new(Session {
        epoch: 0,
        pending: None,
        closed: false,
        blocked: false,
        rows,
        event,
        monitor,
        action: Action::Set,
        getter: None,
        custom: None,
        initial: None,
        result: None,
    }));
    let window = adw::Window::builder()
        .title(if is_custom {
            "Manage Monitor Declarations"
        } else {
            "Compare a Source Change"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(620)
        .default_height(740)
        .build();
    window.set_size_request(350, 420);
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    let title = gtk::Label::new(Some("Source change"));
    title.add_css_class("title");
    header.set_title_widget(Some(&title));
    outer.append(&header);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for set in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        set(body.upcast_ref(), 16);
    }
    body.append(&label("Compare a change, then keep it in your private draft. Original files and the desktop stay unchanged."));
    let chooser = gtk::Box::new(gtk::Orientation::Vertical, 12);
    chooser.set_visible(!is_custom);
    body.append(&chooser);
    let names = session
        .borrow()
        .rows
        .iter()
        .map(|pick| {
            let Some(c) = target(&analysis, pick) else {
                return "Unavailable field".into();
            };
            if monitor {
                format!(
                    "Monitor · {}{}{}",
                    if c.id == "output" {
                        "Output match".into()
                    } else {
                        super::human_name(&c.id)
                    },
                    if c.present { "" } else { " · absent" },
                    if c.present && c.raw_key != c.id.as_bytes() {
                        format!(
                            " · key {}",
                            display_path_bytes(&c.raw_key[..c.raw_key.len().min(96)])
                        )
                    } else {
                        String::new()
                    }
                )
            } else {
                format!(
                    "{} · {}",
                    super::presentation::context(&c.id),
                    super::human_name(&c.id)
                )
            }
        })
        .collect::<Vec<_>>();
    chooser.append(&label("Setting"));
    let targets =
        gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
    super::monitors::compact(&targets);
    targets.set_enable_search(true);
    targets.set_widget_name("source-change-setting");
    targets.update_property(&[gtk::accessible::Property::Label("Setting to compare")]);
    chooser.append(&targets);
    let operation = gtk::DropDown::from_strings(&["Set value"]);
    operation.set_widget_name("source-change-operation");
    operation.update_property(&[gtk::accessible::Property::Label("Field operation")]);
    super::monitors::compact(&operation);
    operation.set_visible(monitor);
    chooser.append(&operation);
    let scope_label = label("Source to change");
    chooser.append(&scope_label);
    let scope = gtk::DropDown::from_strings(&[
        "Selected value expression",
        "Original literal, if traceable",
    ]);
    super::monitors::compact(&scope);
    scope.set_widget_name("source-change-scope");
    scope.update_property(&[gtk::accessible::Property::Label(
        "Source expression selection",
    )]);
    chooser.append(&scope);
    chooser.append(&label("Shared source expressions can affect several declarations. The comparison reports additional modeled changes."));
    let field = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let input_expander = gtk::Expander::builder()
        .label("Proposed value")
        .expanded(true)
        .child(&field)
        .build();
    input_expander.set_widget_name("source-change-inputs");
    body.append(&input_expander);
    let output = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();
    output.set_widget_name("source-change-comparison");
    output.update_property(&[gtk::accessible::Property::Label(
        "Proposed source change and additional effects",
    )]);
    let scroll = gtk::ScrolledWindow::builder()
        .child(&output)
        .min_content_height(140)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    body.append(&scroll);
    outer.append(
        &gtk::ScrolledWindow::builder()
            .child(&body)
            .vexpand(true)
            .min_content_height(140)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    body.set_vexpand(true);
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 10);
    footer.set_margin_start(16);
    footer.set_margin_end(16);
    footer.set_margin_bottom(16);
    let status = label("");
    status.set_widget_name("source-change-status");
    footer.append(&status);
    let compare = gtk::Button::with_label("Compare");
    compare.set_widget_name("source-change-compare");
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    compare.set_hexpand(true);
    actions.append(&compare);
    let keep = gtk::Button::with_label("Keep in Draft");
    keep.set_widget_name("source-change-keep");
    keep.set_sensitive(false);
    keep.set_hexpand(true);
    keep.add_css_class("suggested-action");
    actions.append(&keep);
    footer.append(&actions);
    footer.append(&label(if monitor {
        "Monitor source changes and monitor drafts stay separate. Their reconciliation is still pending; Save Draft keeps the private source revision."
    } else {
        "Source revisions and value overrides stay separate. Review Value Overrides can reconcile scalar layers; Save Draft keeps the private workspace."
    }));
    outer.append(&footer);
    window.set_content(Some(&outer));
    let ui = Controls {
        state: Rc::downgrade(state),
        refresh,
        session: Rc::downgrade(&session),
        model: Rc::downgrade(model),
        analysis: Arc::downgrade(&analysis),
        targets: targets.downgrade(),
        scope: scope.downgrade(),
        scope_label: scope_label.downgrade(),
        operation: operation.downgrade(),
        field: field.downgrade(),
        input_expander: input_expander.downgrade(),
        status: status.downgrade(),
        output: output.downgrade(),
        compare: compare.downgrade(),
        keep: keep.downgrade(),
    };
    let u = ui.clone();
    targets.connect_selected_notify(move |_| u.choose());
    let u = ui.clone();
    operation.connect_selected_notify(move |_| u.select());
    let u = ui.clone();
    scope.connect_selected_notify(move |_| u.invalidate());
    let u = ui.clone();
    compare.connect_clicked(move |_| u.start());
    let u = ui.clone();
    keep.connect_clicked(move |_| u.keep());
    window.connect_close_request(move |_| {
        let mut s = session.borrow_mut();
        s.closed = true;
        s.epoch += 1;
        s.result = None;
        s.getter = None;
        s.custom = None;
        gtk::glib::Propagation::Proceed
    });
    let weak = window.downgrade();
    let u = ui.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
        if weak.upgrade().is_none_or(|w| !w.is_visible()) {
            return gtk::glib::ControlFlow::Break;
        }
        if !u.current() {
            u.stale();
            return gtk::glib::ControlFlow::Break;
        }
        gtk::glib::ControlFlow::Continue
    });
    if let Some(build) = custom {
        input_expander.set_label(Some("Declaration"));
        let changed_ui = ui.clone();
        let changed: Rc<dyn Fn()> = Rc::new(move || changed_ui.invalidate());
        let publish_ui = ui.clone();
        let publish: Publish = Rc::new(move |getter, message| {
            let Some(session) = publish_ui.session.upgrade() else {
                return;
            };
            if session.borrow().closed {
                return;
            }
            if !publish_ui.current() {
                publish_ui.stale();
                return;
            }
            session.borrow_mut().custom = getter;
            publish_ui.message(message);
            publish_ui.sensitive();
        });
        build(&field, changed, publish);
        ui.sensitive();
    } else {
        ui.choose();
    }
    if !is_custom {
        gtk::prelude::GtkWindowExt::set_focus(&window, Some(&targets));
    }
    window.present();
    window
}

#[cfg(test)]
pub(super) fn test_monitor_workflow(parent: &adw::ApplicationWindow) {
    use hyprland_editor_core::{
        lua::LuaDocument, module_path::Context, storage, workspace::Workspace, AUDITED_MAIN,
        BASELINE,
    };
    fn named(root: &gtk::Widget, name: &str) -> gtk::Widget {
        fn find(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
            if root.widget_name() == name {
                return Some(root.clone());
            }
            let mut child = root.first_child();
            while let Some(widget) = child {
                if let Some(result) = find(&widget, name) {
                    return Some(result);
                }
                child = widget.next_sibling();
            }
            None
        }
        find(root, name).unwrap_or_else(|| panic!("Missing {name}"))
    }
    #[track_caller]
    fn wait(mut check: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if check() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "Monitor source operation timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    fn dropdown(root: &gtk::Widget) -> Option<gtk::DropDown> {
        if let Some(pick) = root.downcast_ref::<gtk::DropDown>() {
            return Some(pick.clone());
        }
        let mut child = root.first_child();
        while let Some(widget) = child {
            if let Some(pick) = dropdown(&widget) {
                return Some(pick);
            }
            child = widget.next_sibling();
        }
        None
    }
    fn capture(window: &adw::Window, suffix: &str) {
        if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
            let paintable = gtk::WidgetPaintable::new(Some(window));
            let node = RefCell::new(None);
            wait(|| {
                let snapshot = gtk::Snapshot::new();
                paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
                *node.borrow_mut() = snapshot.to_node();
                node.borrow().is_some()
            });
            window
                .renderer()
                .unwrap()
                .render_texture(node.into_inner().unwrap(), None)
                .save_to_png(format!("{path}.{suffix}.png"))
                .unwrap();
        }
    }
    for version in [BASELINE, AUDITED_MAIN] {
        let directory = std::env::temp_dir().join(format!(
            "hye-monitor-source-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let source="local part=require('part');hl.monitor{output='DP-1',transform=part.turn};hl.config{general={border_size=1}}";
        let part = "-- monitor source comment\nreturn {turn=2, other='keep'}\n";
        std::fs::write(directory.join("main.lua"), source).unwrap();
        std::fs::write(directory.join("part.lua"), part).unwrap();
        let captured = Arc::new(storage::read_source(&directory.join("main.lua")).unwrap());
        let context = Context {
            main_file: captured.identity.selected_path.clone(),
            home: None,
            inherited_package_path: None,
        };
        let grant = Arc::new(super::directory_worker::approve(&directory).unwrap());
        let analysis = super::source_graph::analyze(
            captured.clone(),
            context,
            vec![grant],
            version.into(),
            None,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .unwrap();
        assert!(analysis.blocked.is_none(), "{:?}", analysis.blocked);
        assert_eq!(analysis.monitors.as_ref().unwrap().batches.len(), 1);
        let event = analysis.report.events[0].order;
        let model: Model = Rc::new(RefCell::new(Some(Arc::new(analysis))));
        let state = super::initial_state();
        let worker::Response::Parsed(report) = worker::run(worker::Request::Parse {
            source: source.into(),
            version: version.into(),
        })
        .unwrap() else {
            panic!()
        };
        {
            let mut s = state.borrow_mut();
            s.draft = hyprland_editor_core::Draft::new(version);
            let doc = LuaDocument::from_report(source.into(), *report, &s.catalogue).unwrap();
            s.sources.insert(version.into(), Arc::new(doc));
            s.source_origins
                .insert(version.into(), captured.identity.clone());
        }
        let changes = Rc::new(std::cell::Cell::new(0));
        let done = changes.clone();
        let window = open(
            parent.upcast_ref(),
            &model,
            event,
            &state,
            Rc::new(move || done.set(done.get() + 1)),
        );
        let targets = named(window.upcast_ref(), "source-change-setting")
            .downcast::<gtk::DropDown>()
            .unwrap();
        targets.set_selected(1);
        let input = dropdown(&named(window.upcast_ref(), "source-change-inputs")).unwrap();
        assert_eq!(input.selected(), 2);
        let scope = named(window.upcast_ref(), "source-change-scope")
            .downcast::<gtk::DropDown>()
            .unwrap();
        scope.set_selected(1);
        let compare = named(window.upcast_ref(), "source-change-compare")
            .downcast::<gtk::Button>()
            .unwrap();
        let keep = named(window.upcast_ref(), "source-change-keep")
            .downcast::<gtk::Button>()
            .unwrap();
        let output = named(window.upcast_ref(), "source-change-comparison")
            .downcast::<gtk::TextView>()
            .unwrap();
        input.set_selected(3);
        HOLD_COMPARISON.store(true, std::sync::atomic::Ordering::Release);
        compare.emit_clicked();
        input.set_selected(4);
        HOLD_COMPARISON.store(false, std::sync::atomic::Ordering::Release);
        wait(|| compare.is_sensitive());
        assert!(
            !keep.is_sensitive(),
            "Changed monitor inputs kept a stale result"
        );
        assert_eq!(output.buffer().char_count(), 0);
        input.set_selected(3);
        compare.emit_clicked();
        wait(|| keep.is_sensitive());
        let text = output.buffer().text(
            &output.buffer().start_iter(),
            &output.buffer().end_iter(),
            true,
        );
        assert!(text.contains("part.lua"), "{text}");
        assert!(text.contains("After\n3"), "{text}");
        capture(&window, "monitor-source");
        keep.emit_clicked();
        wait(|| changes.get() == 1);
        assert_eq!(
            state.borrow().source_graphs[version]
                .bundle
                .graph
                .revisions
                .len(),
            1
        );
        window.close();
        // Add an absent field, then remove it through actual GTK actions. Both
        // operations use fresh source revisions and the multi-patch review.
        for (removing, expected_count) in [(false, 2), (true, 3)] {
            let done = changes.clone();
            let window = open(
                parent.upcast_ref(),
                &model,
                event,
                &state,
                Rc::new(move || done.set(done.get() + 1)),
            );
            let analysis = model.borrow().clone().unwrap();
            let index = picks(&analysis, event, true)
                .iter()
                .position(|p| target(&analysis, p).is_some_and(|c| c.id == "scale"))
                .unwrap();
            let targets = named(window.upcast_ref(), "source-change-setting")
                .downcast::<gtk::DropDown>()
                .unwrap();
            targets.set_selected(index as u32);
            let operation = named(window.upcast_ref(), "source-change-operation")
                .downcast::<gtk::DropDown>()
                .unwrap();
            if removing {
                operation.set_selected(1);
            }
            let compare = named(window.upcast_ref(), "source-change-compare")
                .downcast::<gtk::Button>()
                .unwrap();
            let keep = named(window.upcast_ref(), "source-change-keep")
                .downcast::<gtk::Button>()
                .unwrap();
            assert!(compare.is_sensitive());
            compare.emit_clicked();
            wait(|| keep.is_sensitive());
            if removing {
                let output = named(window.upcast_ref(), "source-change-comparison")
                    .downcast::<gtk::TextView>()
                    .unwrap();
                let text = output.buffer().text(
                    &output.buffer().start_iter(),
                    &output.buffer().end_iter(),
                    true,
                );
                assert!(text.contains("Patch 2 of 2"), "{text}");
                assert!(text.contains("Inherited and effective values remain unresolved"));
                let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
                assert!(minimum <= 350, "Monitor field review needs {minimum}px");
                // GTK's client-side shadow can sit outside the allocation.
                window.set_default_size(360, 740);
                wait(|| (350..=360).contains(&window.width()));
                let bounds = keep.compute_bounds(&window).unwrap();
                assert!(bounds.x() >= 0. && bounds.x() + bounds.width() <= window.width() as f32);
                assert!(bounds.y() >= 0. && bounds.y() + bounds.height() <= window.height() as f32);
                capture(&window, "monitor-field-remove");
            }
            keep.emit_clicked();
            wait(|| changes.get() == expected_count);
            let analysis = model.borrow().clone().unwrap();
            let has_scale = analysis.monitors.as_ref().unwrap().batches[0]
                .candidates
                .iter()
                .any(|c| c.field == "scale");
            assert_eq!(has_scale, !removing);
            window.close();
        }
        assert_eq!(
            std::fs::read_to_string(directory.join("main.lua")).unwrap(),
            source
        );
        assert_eq!(
            std::fs::read_to_string(directory.join("part.lua")).unwrap(),
            part
        );
        let bundle = state.borrow().source_graphs[version].bundle.clone();
        let saved = Workspace::new(state.borrow().draft.clone())
            .with_graph(&bundle)
            .to_json(&state.borrow().catalogue)
            .unwrap();
        // Repair a missing output and an invalid existing field using the same
        // source inspector, with proposed seeds explicitly distinct from state.
        let invalid = "local rule={transform=99,bitdepth=9223372036854775807};rule.scale=nil;hl.monitor(rule)";
        let repair_path = directory.join("repair.lua");
        std::fs::write(&repair_path, invalid).unwrap();
        let capture = Arc::new(storage::read_source(&repair_path).unwrap());
        let repair = super::source_graph::analyze(
            capture.clone(),
            Context {
                main_file: capture.identity.selected_path.clone(),
                home: None,
                inherited_package_path: None,
            },
            vec![],
            version.into(),
            None,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .unwrap();
        let repair_model: Model = Rc::new(RefCell::new(Some(Arc::new(repair))));
        let repair_state = super::initial_state();
        let worker::Response::Parsed(report) = worker::run(worker::Request::Parse {
            source: invalid.into(),
            version: version.into(),
        })
        .unwrap() else {
            panic!()
        };
        {
            let mut s = repair_state.borrow_mut();
            s.draft = hyprland_editor_core::Draft::new(version);
            let doc = LuaDocument::from_report(invalid.into(), *report, &s.catalogue).unwrap();
            s.sources.insert(version.into(), Arc::new(doc));
            s.source_origins
                .insert(version.into(), capture.identity.clone());
        }
        for field in ["output", "transform", "scale", "bitdepth"] {
            let kept = Rc::new(std::cell::Cell::new(false));
            let done = kept.clone();
            let window = open(
                parent.upcast_ref(),
                &repair_model,
                0,
                &repair_state,
                Rc::new(move || done.set(true)),
            );
            let analysis = repair_model.borrow().clone().unwrap();
            let index = picks(&analysis, 0, true)
                .iter()
                .position(|p| target(&analysis, p).is_some_and(|c| c.id == field))
                .unwrap();
            named(window.upcast_ref(), "source-change-setting")
                .downcast::<gtk::DropDown>()
                .unwrap()
                .set_selected(index as u32);
            if field == "output" {
                named(window.upcast_ref(), "source-change-value")
                    .downcast::<gtk::Entry>()
                    .unwrap()
                    .set_text("DP-1");
            } else if field == "transform" {
                let form = named(window.upcast_ref(), "source-change-inputs");
                assert_eq!(
                    dropdown(&form).unwrap().selected(),
                    0,
                    "Invalid transform must use an explicit proposed seed"
                );
            } else if field == "scale" {
                assert_eq!(
                    named(window.upcast_ref(), "source-change-scope")
                        .downcast::<gtk::DropDown>()
                        .unwrap()
                        .selected(),
                    2,
                    "A later nil assignment must select the recorded write"
                );
            } else {
                let input = named(window.upcast_ref(), "source-change-value")
                    .downcast::<gtk::Entry>()
                    .unwrap();
                assert_eq!(
                    input.text(),
                    i64::MAX.to_string(),
                    "Source numeric controls must not clamp or round an existing integer"
                );
                input.set_text(&(i64::MAX - 1).to_string());
            }
            let compare = named(window.upcast_ref(), "source-change-compare")
                .downcast::<gtk::Button>()
                .unwrap();
            let keep = named(window.upcast_ref(), "source-change-keep")
                .downcast::<gtk::Button>()
                .unwrap();
            compare.emit_clicked();
            wait(|| keep.is_sensitive());
            keep.emit_clicked();
            wait(|| kept.get());
            window.close();
        }
        assert_eq!(std::fs::read_to_string(&repair_path).unwrap(), invalid);
        let analysis = repair_model.borrow().clone().unwrap();
        assert_eq!(
            analysis.monitors.as_ref().unwrap().batches[0]
                .output_bytes
                .as_deref(),
            Some(b"DP-1".as_slice())
        );
        let fields = &analysis.monitors.as_ref().unwrap().batches[0].candidates;
        assert!(matches!(
            fields
                .iter()
                .find(|c| c.field == "transform")
                .unwrap()
                .primary_value,
            Outcome::Accepted {
                native: hyprland_editor_core::lua::scalar_codec::NativeValue::Int(0),
                ..
            }
        ));
        assert!(
            matches!(&fields.iter().find(|c| c.field=="scale").unwrap().primary_value, Outcome::Accepted {native:hyprland_editor_core::lua::scalar_codec::NativeValue::Bytes(b),..} if b==b"auto")
        );
        assert!(
            matches!(fields.iter().find(|c| c.field=="bitdepth").unwrap().primary_value, Outcome::Accepted {native:hyprland_editor_core::lua::scalar_codec::NativeValue::Int(v),..} if v==i64::MAX-1)
        );
        std::fs::remove_dir_all(&directory).unwrap();
        let restored = Workspace::from_json(
            std::str::from_utf8(&saved).unwrap(),
            &state.borrow().catalogue,
        )
        .unwrap();
        let bundle = hyprland_editor_core::workspace_graph::Bundle {
            graph: restored.source_graph.unwrap(),
            sources: Arc::new(restored.sources),
        };
        let restored = super::source_graph::restore(&bundle, version).unwrap();
        assert!(restored
            .program
            .documents
            .iter()
            .any(|d| d.source == "-- monitor source comment\nreturn {turn=3, other='keep'}\n"));
        assert!(!restored.monitors.as_ref().unwrap().runtime_verified);
        state.borrow_mut().undo_last();
        assert!(state.borrow().source_graphs.contains_key(version));
        state.borrow_mut().undo_last();
        assert!(state.borrow().source_graphs.contains_key(version));
        state.borrow_mut().undo_last();
        assert!(!state.borrow().source_graphs.contains_key(version));
        window.close();
    }
}

#[cfg(test)]
pub(super) fn test_editor(window: &adw::Window, model: &Model) -> HeldComparison {
    fn named(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
        if root.widget_name() == name {
            return Some(root.clone());
        }
        let mut child = root.first_child();
        while let Some(widget) = child {
            if let Some(found) = named(&widget, name) {
                return Some(found);
            }
            child = widget.next_sibling();
        }
        None
    }
    fn wait(mut check: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if check() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(6),
                "Source comparison timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    let input = named(window.upcast_ref(), "source-change-value")
        .unwrap()
        .downcast::<gtk::Entry>()
        .unwrap();
    let compare = named(window.upcast_ref(), "source-change-compare")
        .unwrap()
        .downcast::<gtk::Button>()
        .unwrap();
    let scope = named(window.upcast_ref(), "source-change-scope")
        .unwrap()
        .downcast::<gtk::DropDown>()
        .unwrap();
    let status = named(window.upcast_ref(), "source-change-status")
        .unwrap()
        .downcast::<gtk::Label>()
        .unwrap();
    let output = named(window.upcast_ref(), "source-change-comparison")
        .unwrap()
        .downcast::<gtk::TextView>()
        .unwrap();
    let text = || {
        let b = output.buffer();
        b.text(&b.start_iter(), &b.end_iter(), true).to_string()
    };
    assert_eq!(input.text(), "4");
    compare.emit_clicked();
    assert!(status.text().contains("Change the value"));
    input.set_text("6");
    compare.emit_clicked();
    assert!(!compare.is_sensitive());
    input.set_text("7");
    wait(|| compare.is_sensitive());
    assert!(status.text().contains("Values changed during"));
    assert!(text().is_empty());
    input.set_text("6");
    compare.emit_clicked();
    wait(|| compare.is_sensitive());
    assert!(
        status.text().contains("Only the requested"),
        "{}",
        status.text()
    );
    assert!(text().contains("main.lua"));
    assert!(text().contains("Before\npart\n\nAfter\n6"));
    scope.set_selected(1);
    assert!(text().is_empty());
    compare.emit_clicked();
    wait(|| compare.is_sensitive());
    assert!(
        status.text().contains("Additional modeled"),
        "{}",
        status.text()
    );
    assert!(
        text().contains("part.lua")
            && text().contains("A declaration no longer runs")
            && text().contains("Branch or loop behavior changed"),
        "{}",
        text()
    );
    window.set_default_size(360, 820);
    let (minimum, _, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
    assert!(minimum <= 350, "Comparison needs {minimum}px");
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        window.set_default_size(620, 800);
        let start = std::time::Instant::now();
        wait(|| start.elapsed() >= std::time::Duration::from_millis(300));
        let paintable = gtk::WidgetPaintable::new(Some(window));
        let node = RefCell::new(None);
        wait(|| {
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
            *node.borrow_mut() = snapshot.to_node();
            node.borrow().is_some()
        });
        window
            .renderer()
            .unwrap()
            .render_texture(node.into_inner().unwrap(), None)
            .save_to_png(format!("{path}.source-change.png"))
            .unwrap();
    }
    // Replacing the model while a comparison is in flight must block its result,
    // even while the worker still owns its private evaluator input.
    scope.set_selected(0);
    input.set_text("5");
    HOLD_COMPARISON.store(true, std::sync::atomic::Ordering::Release);
    let held = HeldComparison;
    compare.emit_clicked();
    let previous = model.borrow_mut().take();
    wait(|| status.text().contains("source analysis changed"));
    *model.borrow_mut() = previous;
    assert!(!compare.is_sensitive());
    assert!(text().is_empty());
    window.close();
    wait(|| !window.is_visible());
    // Exact i64 and negative-zero inputs survive the controls without f64 casts.
    for initial in [Value::Int(i64::MAX), Value::Float(-0.0)] {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let getter = input_form(&initial, None, &body, Rc::new(|| {}));
        assert!(same_input(&initial, &getter().unwrap()));
    }
    let codec = Codec::String {
        validator: Some(StringValidator::Choices(
            hyprland_editor_core::lua::scalar_codec::StringChoices {
                choices: vec![b"one".to_vec(), b"two".to_vec()],
            },
        )),
    };
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let getter = input_form(
        &Value::String("one".into()),
        Some(&codec),
        &body,
        Rc::new(|| {}),
    );
    let pick = body
        .first_child()
        .unwrap()
        .downcast::<gtk::DropDown>()
        .unwrap();
    pick.set_selected(1);
    assert_eq!(getter().unwrap(), Value::String("two".into()));
    held
}

#[cfg(test)]
pub(super) fn test_keep_in_draft(parent: &adw::Window, state: &super::Shared) {
    fn named(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
        if root.widget_name() == name {
            return Some(root.clone());
        }
        let mut child = root.first_child();
        while let Some(w) = child {
            if let Some(found) = named(&w, name) {
                return Some(found);
            }
            child = w.next_sibling();
        }
        None
    }
    fn wait(mut check: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        loop {
            while gtk::glib::MainContext::default().pending() {
                gtk::glib::MainContext::default().iteration(false);
            }
            if check() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "Private revision timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    let before = state.borrow().source_graphs[hyprland_editor_core::BASELINE].clone();
    let calls = named(parent.upcast_ref(), "dependency-analysis-call")
        .unwrap()
        .downcast::<gtk::DropDown>()
        .unwrap();
    let stack = calls
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .downcast::<gtk::Stack>()
        .unwrap();
    stack.set_visible_child_name("calls");
    calls.set_selected(1);
    let open = named(parent.upcast_ref(), "source-analysis-compare")
        .unwrap()
        .downcast::<gtk::Button>()
        .unwrap();
    assert!(open.is_sensitive());
    open.emit_clicked();
    let window = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<adw::Window>().ok())
        .find(|w| w.is_visible() && w.title().as_deref() == Some("Compare a Source Change"))
        .unwrap();
    let input = named(window.upcast_ref(), "source-change-value")
        .unwrap()
        .downcast::<gtk::Entry>()
        .unwrap();
    assert_eq!(input.text(), "4");
    input.set_text("6");
    let compare = named(window.upcast_ref(), "source-change-compare")
        .unwrap()
        .downcast::<gtk::Button>()
        .unwrap();
    compare.emit_clicked();
    wait(|| compare.is_sensitive());
    let keep = named(window.upcast_ref(), "source-change-keep")
        .unwrap()
        .downcast::<gtk::Button>()
        .unwrap();
    assert!(keep.is_sensitive());
    // A source operation must not commit across an independently edited draft.
    keep.emit_clicked();
    {
        let mut s = state.borrow_mut();
        s.draft
            .values
            .insert("general:border_size".into(), Value::Int(9));
        s.generation += 1;
    }
    wait(|| keep.is_sensitive());
    let status = named(window.upcast_ref(), "source-change-status")
        .unwrap()
        .downcast::<gtk::Label>()
        .unwrap();
    assert!(status.text().contains("draft changed"), "{}", status.text());
    assert_eq!(
        state.borrow().source_graphs[hyprland_editor_core::BASELINE].fingerprint,
        before.fingerprint
    );
    let generation = state.borrow().generation;
    keep.emit_clicked();
    wait(|| !window.is_visible());
    let after = state.borrow().source_graphs[hyprland_editor_core::BASELINE].clone();
    assert_eq!(state.borrow().generation, generation + 1);
    assert!(state.borrow().dirty());
    assert_eq!(
        state.borrow().draft.values["general:border_size"],
        Value::Int(9),
        "Keeping a source revision must not overwrite a separate value override"
    );
    assert_eq!(after.bundle.graph.revisions.len(), 1);
    assert!(Arc::ptr_eq(&before.bundle.sources, &after.bundle.sources));
    assert!(state.borrow().export_lua().is_err());
    assert_eq!(
        before.bundle.sources[0].content,
        after.bundle.sources[0].content
    );
    assert_eq!(
        before.bundle.sources[0].origin,
        after.bundle.sources[0].origin
    );
    assert!(after.bundle.graph.revisions[0].sha256 != after.bundle.graph.revisions[0].base_sha256);
    let model = super::source_graph::restore(&after.bundle, &after.version).unwrap();
    assert!(model.program.documents[0].source.contains("border_size=6"));
    assert!(model.frozen.is_some());
    assert!(!model.report.runtime_verified);
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        stack.set_visible_child_name("files");
        let start = std::time::Instant::now();
        wait(|| start.elapsed() >= std::time::Duration::from_millis(300));
        let paintable = gtk::WidgetPaintable::new(Some(parent));
        let node = RefCell::new(None);
        wait(|| {
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, parent.width() as f64, parent.height() as f64);
            *node.borrow_mut() = snapshot.to_node();
            node.borrow().is_some()
        });
        parent
            .renderer()
            .unwrap()
            .render_texture(node.into_inner().unwrap(), None)
            .save_to_png(format!("{path}.source-revision.png"))
            .unwrap();
    }
}
