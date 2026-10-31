//! Focused private rule declarations; no compositor mutation or expression execution.
use super::{
    label,
    monitors::{choices, compact},
    Shared,
};
use adw::prelude::*;
use hyprland_editor_core::{
    rule::{self, Effect, EffectValue, Geometry, Kind, MatchKind, Predicate, Rule},
    Setting, Value,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Getter = Rc<dyn Fn() -> Result<(), String>>;
type ValueGetter = Box<dyn Fn() -> Result<EffectValue, String>>;
fn current(state: &Shared) -> Vec<Rule> {
    let s = state.borrow();
    s.rules.get(&s.draft.version).cloned().unwrap_or_default()
}
fn unique(rules: &[Rule]) -> String {
    (0..)
        .map(|n| format!("rule-{n}"))
        .find(|id| rules.iter().all(|r| r.id != *id))
        .unwrap()
}
fn text(panel: &gtk::Box, title: &str, initial: &str) -> gtk::Entry {
    panel.append(&label(title));
    let e = gtk::Entry::builder()
        .text(initial)
        .max_length(16384)
        .build();
    panel.append(&e);
    e
}
fn literal(
    panel: &gtk::Box,
    setting: &Setting,
    value: Option<Value>,
) -> Box<dyn Fn() -> Result<Value, String>> {
    let initial = value.or_else(|| setting.known_default());
    if ["Int", "Float"].contains(&setting.kind.as_str()) {
        let raw = initial.as_ref().map_or("0".into(), Value::lua);
        let e = text(panel, "Value", &raw);
        let integer = setting.kind == "Int";
        return Box::new(move || {
            if integer {
                e.text()
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| "Enter a whole number".into())
            } else {
                e.text()
                    .parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| "Enter a finite number".into())
            }
        });
    }
    let original = initial.clone();
    let get = super::controls::form(setting, initial, panel);
    let shown = get().ok();
    Box::new(move || {
        let value = get()?;
        if Some(&value) == shown.as_ref() {
            if let Some(original) = &original {
                return Ok(original.clone());
            }
        }
        Ok(value)
    })
}
fn effect_form(
    panel: &gtk::Box,
    setting: &Setting,
    value: Option<EffectValue>,
    version: &str,
) -> ValueGetter {
    if setting.lua_key == "border_color" {
        return super::rule_forms::border(panel, value, version);
    }
    if setting.kind == "String" {
        let raw = match &value {
            Some(EffectValue::Literal(Value::String(s))) => Some(s.clone()),
            _ => None,
        };
        if let Some(form) = super::rule_forms::string(panel, &setting.lua_key, raw) {
            return form;
        }
    }
    if setting.kind == "ExpressionVec2" {
        let geometry = match value {
            Some(EffectValue::Geometry(g)) => g,
            _ => Geometry::Axes([Value::String("0".into()), Value::String("0".into())]),
        };
        panel.append(&label(
            "Expressions are stored in your draft and are not evaluated here.",
        ));
        let mode = choices(
            panel,
            &["Two Axes", "Combined Expression", "Clear Geometry"],
            match &geometry {
                Geometry::Axes(_) => 0,
                Geometry::Text(s) if s.is_empty() => 2,
                _ => 1,
            },
        );
        let axes = match &geometry {
            Geometry::Axes(v) => v.clone(),
            Geometry::Text(_) => [Value::String("0".into()), Value::String("0".into())],
        };
        let display = |v: &Value| match v {
            Value::String(s) => s.clone(),
            v => v.lua(),
        };
        let two_axes = gtk::Box::new(gtk::Orientation::Vertical, 8);
        panel.append(&two_axes);
        let combined = gtk::Box::new(gtk::Orientation::Vertical, 8);
        panel.append(&combined);
        let x = text(&two_axes, "X / Width Expression", &display(&axes[0]));
        let y = text(&two_axes, "Y / Height Expression", &display(&axes[1]));
        let raw = text(
            &combined,
            "Combined Expression",
            match &geometry {
                Geometry::Text(s) => s,
                _ => "",
            },
        );
        let axes_box = two_axes.downgrade();
        let raw_box = combined.downgrade();
        let update = move |mode: &gtk::DropDown| {
            if let (Some(axes), Some(raw)) = (axes_box.upgrade(), raw_box.upgrade()) {
                axes.set_visible(mode.selected() == 0);
                raw.set_visible(mode.selected() == 1);
            }
        };
        update(&mode);
        mode.connect_selected_notify(update);
        return Box::new(move || {
            Ok(EffectValue::Geometry(match mode.selected() {
                0 => {
                    let values = [x.text().to_string(), y.text().to_string()];
                    let keep = |i: usize| {
                        if values[i] == display(&axes[i]) {
                            axes[i].clone()
                        } else {
                            Value::String(values[i].clone())
                        }
                    };
                    Geometry::Axes([keep(0), keep(1)])
                }
                1 => Geometry::Text(raw.text().into()),
                _ => Geometry::Text(String::new()),
            }))
        });
    }
    let initial = if let Some(EffectValue::Literal(v)) = value {
        Some(v)
    } else {
        None
    };
    if setting.kind == "String" {
        panel.append(&label("Literal effect text is retained. Specialized action/selector/style builders and runtime interpretation remain in progress."));
    }
    let get = literal(panel, setting, initial);
    Box::new(move || get().map(EffectValue::Literal))
}
fn predicate_form(
    panel: &gtk::Box,
    kind: Kind,
    property: &str,
    value: Option<Value>,
) -> Box<dyn Fn() -> Result<Value, String>> {
    let authoring = value.is_none();
    let engine = rule::match_kind(kind, property).expect("audited predicate");
    let initial = value.unwrap_or_else(|| match engine {
        MatchKind::Bool => Value::Bool(false),
        MatchKind::Int => Value::Int(0),
        _ => Value::String(String::new()),
    });
    if let Value::Bool(b) = initial {
        let switch = gtk::Switch::builder()
            .active(b)
            .halign(gtk::Align::Start)
            .build();
        panel.append(&switch);
        return Box::new(move || Ok(Value::Bool(switch.is_active())));
    }
    if let Value::Int(n) = initial {
        let e = text(panel, "Whole Number", &n.to_string());
        return Box::new(move || {
            e.text()
                .parse()
                .map(Value::Int)
                .map_err(|_| "Enter a whole number".into())
        });
    }
    let Value::String(raw) = initial else {
        unreachable!()
    };
    let (pattern, negated) = if engine == MatchKind::Regex {
        raw.strip_prefix("negative:")
            .map_or((raw.as_str(), false), |s| (s, true))
    } else {
        (raw.as_str(), false)
    };
    let mode = if engine == MatchKind::Regex {
        let mode = choices(
            panel,
            &["Exact Text", "Regular Expression"],
            u32::from(!authoring),
        );
        mode.set_widget_name("rule-pattern-mode");
        Some(mode)
    } else {
        None
    };
    let negative = gtk::CheckButton::with_label("Does Not Match");
    negative.set_active(negated);
    if engine == MatchKind::Regex {
        panel.append(&negative);
    }
    let e = text(
        panel,
        if engine == MatchKind::Regex {
            "Text or Pattern"
        } else {
            "Match Value"
        },
        pattern,
    );
    let get_pattern: Rc<dyn Fn() -> String> = Rc::new({
        let e = e.clone();
        let mode = mode.clone();
        let negative = negative.clone();
        move || {
            if mode.as_ref().is_some_and(|m| m.selected() == 0) {
                hyprland_editor_core::regex::rule_literal(&e.text(), negative.is_active())
            } else if mode.is_some() && negative.is_active() {
                format!("negative:{}", e.text())
            } else {
                e.text().into()
            }
        }
    });
    if engine == MatchKind::Regex {
        panel.append(&label(
            "Matches the whole sample. Sample checks do not detect live windows or layers.",
        ));
        let sample = text(panel, "Sample Text", "");
        sample.set_max_length(65536);
        let button = gtk::Button::with_label("Check Sample");
        panel.append(&button);
        let result = label("");
        panel.append(&result);
        let get_sample_pattern = get_pattern.clone();
        let sample = sample.downgrade();
        let result = result.downgrade();
        button.connect_clicked(move |button| {
            let (Some(sample), Some(result)) = (sample.upgrade(), result.upgrade()) else {
                return;
            };
            let pattern_value = get_sample_pattern();
            let sample_value = sample.text().to_string();
            let (expression, negated) = pattern_value
                .strip_prefix("negative:")
                .map_or((pattern_value.as_str(), false), |s| (s, true));
            let request = hyprland_editor_core::regex::Sample {
                pattern: expression.into(),
                text: sample_value.clone(),
                negated,
            };
            let (send, receive) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = send.send(super::worker::run(super::worker::Request::CheckRegex(
                    request,
                )));
            });
            button.set_sensitive(false);
            result.set_text("Checking isolated sample…");
            let get_sample_pattern = get_sample_pattern.clone();
            let sample = sample.downgrade();
            let result = result.downgrade();
            let button = button.downgrade();
            gtk::glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
                let (Some(sample), Some(result), Some(button)) =
                    (sample.upgrade(), result.upgrade(), button.upgrade())
                else {
                    return gtk::glib::ControlFlow::Break;
                };
                if button.root().is_none() {
                    return gtk::glib::ControlFlow::Break;
                }
                match receive.try_recv() {
                    Ok(response) => {
                        button.set_sensitive(true);
                        if get_sample_pattern() != pattern_value || sample.text() != sample_value {
                            result.set_text("Inputs changed; check the current sample again.");
                        } else {
                            let message = match response {
                                Ok(super::worker::Response::RegexMatch(true)) => {
                                    "Sample matches".into()
                                }
                                Ok(super::worker::Response::RegexMatch(false)) => {
                                    "Sample does not match".into()
                                }
                                Ok(_) => "Unexpected sample response".into(),
                                Err(e) => format!("Pattern check failed: {e}"),
                            };
                            result.set_text(&message);
                        }
                        gtk::glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                    Err(_) => {
                        button.set_sensitive(true);
                        result.set_text("Sample worker stopped");
                        gtk::glib::ControlFlow::Break
                    }
                }
            });
        });
    }
    Box::new(move || Ok(Value::String(get_pattern())))
}
fn properties(kind: Kind) -> Vec<&'static str> {
    match kind {
        Kind::Window => vec![
            "class",
            "title",
            "initial_class",
            "initial_title",
            "float",
            "tag",
            "xwayland",
            "fullscreen",
            "pin",
            "focus",
            "group",
            "modal",
            "fullscreen_state_internal",
            "fullscreen_state_client",
            "workspace",
            "content",
            "xdg_tag",
        ],
        Kind::Layer => vec!["namespace"],
    }
}
fn editor(
    panel: &gtk::Box,
    state: &Shared,
    rule: &Rule,
    status: &gtk::Label,
    feedback: &gtk::Label,
) -> Getter {
    let metadata = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.append(
        &gtk::Expander::builder()
            .label("Name and Enabled State")
            .child(&metadata)
            .build(),
    );
    let named = gtk::CheckButton::with_label("Named Rule");
    named.set_widget_name("rule-named");
    named.set_active(rule.name.is_some());
    metadata.append(&named);
    let name_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    metadata.append(&name_box);
    name_box.set_visible(named.is_active());
    let name = text(&name_box, "Rule Name", rule.name.as_deref().unwrap_or(""));
    name.set_widget_name("rule-name");
    let weak = name_box.downgrade();
    named.connect_toggled(move |b| {
        if let Some(e) = weak.upgrade() {
            e.set_visible(b.is_active());
        }
    });
    let enabled = choices(
        &metadata,
        &["Default Enabled", "Enabled", "Disabled"],
        match rule.enabled {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        },
    );
    enabled.set_widget_name("rule-enabled");
    let selection = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    panel.append(&selection);
    let section = choices(&selection, &["Conditions", "Effects"], 0);
    section.set_hexpand(true);
    section.set_widget_name("rule-section");
    let model = gtk::StringList::new(&[]);
    let field = gtk::DropDown::builder().model(&model).build();
    compact(&field);
    field.set_widget_name("rule-field");
    field.set_hexpand(true);
    selection.append(&field);
    let set = gtk::CheckButton::with_label("Set This Field");
    set.set_widget_name("rule-field-enabled");
    panel.append(&set);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_widget_name("rule-field-body");
    panel.append(&body);
    let draft = Rc::new(RefCell::new(rule.clone()));
    let get = Rc::new(RefCell::new(None::<ValueGetter>));
    let active = Rc::new(RefCell::new(None::<(bool, String)>));
    let busy = Rc::new(Cell::new(false));
    let version = state.borrow().draft.version.clone();
    let fields = rule::catalogue(rule.kind, &version);
    let kind = rule.kind;
    let flush: Getter = Rc::new({
        let draft = draft.clone();
        let get = get.clone();
        let active = active.clone();
        let set = set.clone();
        let version = version.clone();
        move || {
            if let Some((effect, key)) = active.borrow().as_ref() {
                let value = if set.is_active() {
                    Some(get.borrow().as_ref().ok_or("No field form")?()?)
                } else {
                    None
                };
                let mut d = draft.borrow().clone();
                if *effect {
                    if let Some(value) = value {
                        if let Some(effect) = d.effects.iter_mut().find(|e| e.key == *key) {
                            effect.value = value;
                        } else {
                            d.effects.push(Effect {
                                key: key.clone(),
                                value,
                            });
                        }
                    } else {
                        d.effects.retain(|e| e.key != *key);
                    }
                } else if let Some(EffectValue::Literal(value)) = value {
                    if let Some(predicate) = d.predicates.iter_mut().find(|p| p.property == *key) {
                        predicate.value = value;
                    } else {
                        d.predicates.push(Predicate {
                            property: key.clone(),
                            value,
                        });
                    }
                } else {
                    d.predicates.retain(|p| p.property != *key);
                }
                d.validate(&version)?;
                *draft.borrow_mut() = d;
            }
            Ok(())
        }
    });
    let render: Rc<dyn Fn(bool)> = Rc::new({
        let version = version.clone();
        let draft = draft.clone();
        let get = get.clone();
        let active = active.clone();
        let section = section.clone();
        let model = model.clone();
        let field = field.clone();
        let set = set.clone();
        let body = body.clone();
        let busy = busy.clone();
        move |reset| {
            busy.set(true);
            let effect = section.selected() == 1;
            let keys = if effect {
                fields.iter().map(|s| s.lua_key.clone()).collect::<Vec<_>>()
            } else {
                properties(kind).iter().map(|s| (*s).into()).collect()
            };
            if reset {
                model.splice(
                    0,
                    model.n_items(),
                    &keys
                        .iter()
                        .map(|k| super::human_name(k))
                        .collect::<Vec<_>>()
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                );
                field.set_selected(0);
            }
            let index = (field.selected() as usize).min(keys.len() - 1);
            let key = &keys[index];
            *active.borrow_mut() = Some((effect, key.clone()));
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            let d = draft.borrow();
            let getter: ValueGetter = if effect {
                let value = d
                    .effects
                    .iter()
                    .find(|e| e.key == *key)
                    .map(|e| e.value.clone());
                set.set_active(value.is_some());
                body.append(&label(&super::human_name(key)));
                effect_form(&body, &fields[index], value, &version)
            } else {
                let value = d
                    .predicates
                    .iter()
                    .find(|p| p.property == *key)
                    .map(|p| p.value.clone());
                set.set_active(value.is_some());
                body.append(&label(&format!("Condition: {key}")));
                let predicate = predicate_form(&body, kind, key, value);
                Box::new(move || predicate().map(EffectValue::Literal))
            };
            *get.borrow_mut() = Some(getter);
            body.set_sensitive(set.is_active());
            busy.set(false);
        }
    });
    for (picker, reset) in [(&section, true), (&field, false)] {
        let weak = Rc::downgrade(&render);
        let flush = flush.clone();
        let busy = busy.clone();
        let active = active.clone();
        let feedback = feedback.clone();
        let model = model.clone();
        picker.connect_selected_notify(move |picker| {
            if busy.get() {
                return;
            }
            if let Err(e) = flush() {
                busy.set(true);
                if let Some((effect, key)) = active.borrow().as_ref() {
                    picker.set_selected(if reset {
                        u32::from(*effect)
                    } else {
                        (0..model.n_items())
                            .find(|i| {
                                model.string(*i).as_deref() == Some(super::human_name(key).as_str())
                            })
                            .unwrap_or(0)
                    });
                }
                busy.set(false);
                feedback.set_text(&e);
                return;
            }
            if let Some(r) = weak.upgrade() {
                r(reset);
            }
        });
    }
    let weak = body.downgrade();
    set.connect_toggled(move |set| {
        if let Some(body) = weak.upgrade() {
            body.set_sensitive(set.is_active());
        }
    });
    render(true);
    let id = rule.id.clone();
    let state = state.clone();
    let status = status.clone();
    let submit: Getter = Rc::new(move || {
        let _ = &render;
        flush()?;
        let mut updated = draft.borrow().clone();
        updated.name = if named.is_active() {
            Some(name.text().into())
        } else {
            None
        };
        updated.enabled = match enabled.selected() {
            0 => None,
            1 => Some(true),
            _ => Some(false),
        };
        let mut rules = current(&state);
        *rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Declaration removed")? = updated;
        state.borrow_mut().replace_rules(rules)?;
        super::summary(&state, &status);
        Ok(())
    });
    let update = gtk::Button::with_label("Update Rule Draft");
    update.add_css_class("suggested-action");
    panel.append(&update);
    let weak = Rc::downgrade(&submit);
    let feedback = feedback.clone();
    update.connect_clicked(move |_| {
        if let Some(submit) = weak.upgrade() {
            match submit() {
                Ok(()) => feedback.set_text("Rule draft updated; desktop state is unchanged."),
                Err(e) => feedback.set_text(&e),
            }
        }
    });
    submit
}
pub(super) fn connect(
    button: &gtk::Button,
    parent: &adw::ApplicationWindow,
    state: &Shared,
    status: &gtk::Label,
) {
    let weak = parent.downgrade();
    let state = state.clone();
    let status = status.clone();
    button.connect_clicked(move |_| {
        if let Some(parent) = weak.upgrade() {
            show(&parent, &state, &status);
        }
    });
}
pub(super) fn show(
    parent: &impl IsA<gtk::Window>,
    state: &Shared,
    status: &gtk::Label,
) -> adw::Window {
    let win = adw::Window::builder()
        .title("Window and Layer Rules")
        .transient_for(parent)
        .modal(true)
        .default_width(700)
        .default_height(760)
        .build();
    win.set_size_request(350, 480);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for setter in [
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
    ] {
        setter(content.upcast_ref(), 16);
    }
    content.append(&label("Configure private rule drafts. Named declarations accumulate effects; effects inside one table have unspecified order."));
    let model = gtk::StringList::new(&[]);
    let pick = gtk::DropDown::builder().model(&model).build();
    compact(&pick);
    pick.set_widget_name("match-effect-rule");
    content.append(&pick);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add");
    let layer = gtk::Button::with_label("Add Layer");
    let duplicate = gtk::Button::with_label("Duplicate");
    let remove = gtk::Button::with_label("Remove");
    for b in [&add, &layer, &duplicate, &remove] {
        row.append(b);
    }
    content.append(&row);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let up = gtk::Button::with_label("↑");
    let down = gtk::Button::with_label("↓");
    let undo = gtk::Button::with_label("Undo");
    for b in [&up, &down, &undo] {
        row.append(b);
    }
    content.append(&row);
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(&panel);
    let feedback = label("");
    feedback.set_widget_name("rule-feedback");
    content.append(&feedback);
    let split = gtk::Button::with_label("Order Named Effects Explicitly");
    content.append(&label("Explicit ordering splits a named table into single-effect declarations in the displayed order. This changes previously unspecified ordering."));
    content.append(&split);
    let check = gtk::Button::with_label("Check Rule Relationships");
    content.append(&check);
    let save = gtk::Button::with_label("Save Draft");
    save.set_visible(
        parent
            .upcast_ref::<gtk::Window>()
            .is::<adw::ApplicationWindow>(),
    );
    content.append(&save);
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    win.set_content(Some(&root));
    let busy = Rc::new(Cell::new(false));
    let active = Rc::new(Cell::new(gtk::INVALID_LIST_POSITION));
    let submit = Rc::new(RefCell::new(None::<Getter>));
    let flush: Getter = Rc::new({
        let submit = submit.clone();
        move || {
            if let Some(submit) = submit.borrow().as_ref() {
                submit()?;
            }
            Ok(())
        }
    });
    let refresh: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let model = model.clone();
        let pick = pick.clone();
        let panel = panel.clone();
        let submit = submit.clone();
        let active = active.clone();
        let busy = busy.clone();
        move || {
            if busy.replace(true) {
                return;
            }
            let rules = current(&state);
            let index = (pick.selected() as usize).min(rules.len().saturating_sub(1));
            let names = rules
                .iter()
                .map(|r| {
                    format!(
                        "{:?} · {}{}",
                        r.kind,
                        if r.enabled == Some(false) {
                            "Disabled · "
                        } else {
                            ""
                        },
                        r.name.as_deref().filter(|n| !n.is_empty()).unwrap_or(&r.id)
                    )
                })
                .collect::<Vec<_>>();
            if model.n_items() as usize != names.len()
                || names
                    .iter()
                    .enumerate()
                    .any(|(i, n)| model.string(i as u32).as_deref() != Some(n))
            {
                model.splice(
                    0,
                    model.n_items(),
                    &names.iter().map(String::as_str).collect::<Vec<_>>(),
                );
            }
            pick.set_selected(if rules.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            active.set(pick.selected());
            while let Some(child) = panel.first_child() {
                panel.remove(&child);
            }
            *submit.borrow_mut() = rules
                .get(index)
                .map(|r| editor(&panel, &state, r, &status, &feedback));
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&refresh);
    let flush_c = flush.clone();
    let busy_c = busy.clone();
    let active_c = active.clone();
    let feedback_c = feedback.clone();
    pick.connect_selected_notify(move |p| {
        if busy_c.get() {
            return;
        }
        if let Err(e) = flush_c() {
            busy_c.set(true);
            p.set_selected(active_c.get());
            busy_c.set(false);
            feedback_c.set_text(&e);
            return;
        }
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    for (button, op) in [
        (&add, 0),
        (&layer, 5),
        (&split, 6),
        (&duplicate, 1),
        (&remove, 2),
        (&up, 3),
        (&down, 4),
    ] {
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let pick = pick.clone();
        let flush = flush.clone();
        let refresh = refresh.clone();
        button.connect_clicked(move |_| {
            if op != 2 {
                if let Err(e) = flush() {
                    feedback.set_text(&e);
                    return;
                }
            }
            let mut rules = current(&state);
            let index = pick.selected() as usize;
            let target = match op {
                0 | 5 => {
                    let r = Rule::new(
                        &unique(&rules),
                        if op == 5 { Kind::Layer } else { Kind::Window },
                    );
                    rules.push(r);
                    rules.len() - 1
                }
                6 if index < rules.len() => {
                    let mut first = rules[index].clone();
                    if first.name.as_ref().is_none_or(|n| n.is_empty()) {
                        feedback
                            .set_text("Choose a nonempty name before ordering effects explicitly.");
                        return;
                    }
                    if first.effects.len() < 2 {
                        feedback.set_text("This declaration already has at most one effect.");
                        return;
                    }
                    let effects = std::mem::take(&mut first.effects);
                    let name = first.name.clone();
                    let kind = first.kind;
                    let enabled = first.enabled;
                    first.effects.push(effects[0].clone());
                    rules[index] = first;
                    for (n, effect) in effects.into_iter().enumerate().skip(1) {
                        let mut next = Rule::new(&unique(&rules), kind);
                        next.name = name.clone();
                        next.enabled = enabled;
                        next.effects.push(effect);
                        rules.insert(index + n, next);
                    }
                    index
                }
                1 if index < rules.len() => {
                    let mut r = rules[index].clone();
                    r.id = unique(&rules);
                    if r.name.as_ref().is_some_and(|n| !n.is_empty()) {
                        let base = r.name.clone().unwrap();
                        r.name = Some(
                            (1..)
                                .map(|n| format!("{base} copy {n}"))
                                .find(|name| {
                                    rules.iter().all(|other| {
                                        other.kind != r.kind || other.name.as_ref() != Some(name)
                                    })
                                })
                                .unwrap(),
                        );
                    }
                    rules.push(r);
                    rules.len() - 1
                }
                2 if index < rules.len() => {
                    rules.remove(index);
                    index.min(rules.len().saturating_sub(1))
                }
                3 if index > 0 && index < rules.len() => {
                    rules.swap(index, index - 1);
                    index - 1
                }
                4 if index + 1 < rules.len() => {
                    rules.swap(index, index + 1);
                    index + 1
                }
                _ => return,
            };
            let result = state.borrow_mut().replace_rules(rules);
            match result {
                Ok(()) => {
                    super::summary(&state, &status);
                    refresh();
                    pick.set_selected(target as u32);
                }
                Err(e) => feedback.set_text(&e),
            }
        });
    }
    let state_c = state.clone();
    let status_c = status.clone();
    let refresh_c = refresh.clone();
    undo.connect_clicked(move |_| {
        state_c.borrow_mut().undo_last();
        super::summary(&state_c, &status_c);
        refresh_c();
    });
    let flush_c = flush.clone();
    let state_c = state.clone();
    let feedback_c = feedback.clone();
    check.connect_clicked(move |_| {
        if let Err(e) = flush_c() {
            feedback_c.set_text(&e);
            return;
        }
        let messages = rule::findings(&current(&state_c));
        feedback_c.set_text(&if messages.is_empty() {
            "No literal relationship warnings. Runtime matching and activation remain unverified."
                .into()
        } else {
            messages.join("\n")
        });
    });
    let flush_c = flush.clone();
    let feedback_c = feedback.clone();
    let weak = parent.downgrade();
    save.connect_clicked(move |_| {
        if let Err(e) = flush_c() {
            feedback_c.set_text(&e);
            return;
        }
        if let Some(parent) = weak.upgrade() {
            if let Some(a) = parent
                .upcast_ref::<gtk::Window>()
                .downcast_ref::<adw::ApplicationWindow>()
                .and_then(|window| window.lookup_action("save-draft"))
            {
                a.activate(None);
            }
        }
    });
    let flush_c = flush.clone();
    win.connect_close_request(move |win| match flush_c() {
        Ok(()) => gtk::glib::Propagation::Proceed,
        Err(e) => {
            let dialog =
                adw::MessageDialog::new(Some(win), Some("Discard invalid form changes?"), Some(&e));
            dialog.add_response("keep", "Keep Editing");
            dialog.add_response("discard", "Discard Form Changes");
            dialog.set_default_response(Some("keep"));
            dialog.set_close_response("keep");
            let weak = win.downgrade();
            dialog.connect_response(None, move |_, response| {
                if response == "discard" {
                    if let Some(win) = weak.upgrade() {
                        win.destroy();
                    }
                }
            });
            dialog.present();
            gtk::glib::Propagation::Stop
        }
    });
    refresh();
    win.present();
    win
}

#[cfg(test)]
pub(super) fn test_editor(parent: &adw::ApplicationWindow) {
    fn descendants(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut out = vec![];
        let mut c = w.first_child();
        while let Some(w) = c {
            out.push(w.clone());
            out.extend(descendants(&w));
            c = w.next_sibling();
        }
        out
    }
    fn widget<T: IsA<gtk::Widget> + gtk::glib::object::ObjectType>(
        win: &adw::Window,
        name: &str,
    ) -> T {
        if ["rule-named", "rule-name", "rule-enabled"].contains(&name) {
            for expander in descendants(win.upcast_ref())
                .into_iter()
                .filter_map(|w| w.downcast::<gtk::Expander>().ok())
            {
                if expander.label().as_deref() == Some("Name and Enabled State") {
                    expander.set_expanded(true);
                }
            }
        }
        descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap()
            .downcast()
            .unwrap()
    }
    fn click(win: &adw::Window, title: &str) {
        descendants(win.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(title))
            .unwrap()
            .emit_clicked();
    }
    let state = super::initial_state();
    let status = label("");
    let win = show(parent, &state, &status);
    click(&win, "Add");
    assert!(state.borrow().dirty());
    let field: gtk::DropDown = widget(&win, "rule-field");
    let section: gtk::DropDown = widget(&win, "rule-section");
    widget::<gtk::CheckButton>(&win, "rule-named").set_active(true);
    assert!(widget::<gtk::Entry>(&win, "rule-name").is_sensitive());
    widget::<gtk::Entry>(&win, "rule-name").set_text("test");
    widget::<gtk::CheckButton>(&win, "rule-field-enabled").set_active(true);
    let body: gtk::Box = widget(&win, "rule-field-body");
    let entries = descendants(body.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Entry>().ok())
        .collect::<Vec<_>>();
    widget::<gtk::DropDown>(&win, "rule-pattern-mode").set_selected(1);
    entries[0].set_text("foo.*");
    entries[1].set_text("foobar");
    let context = gtk::glib::MainContext::default();
    let settle = || {
        for _ in 0..5 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    settle();
    click(&win, "Check Sample");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        while context.pending() {
            context.iteration(false);
        }
        if descendants(body.upcast_ref())
            .iter()
            .filter_map(|w| w.clone().downcast::<gtk::Label>().ok())
            .any(|l| l.text() == "Sample matches")
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Isolated sample did not complete: {:?}",
            descendants(body.upcast_ref())
                .iter()
                .filter_map(|w| w.clone().downcast::<gtk::Label>().ok())
                .map(|l| l.text().to_string())
                .collect::<Vec<_>>()
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    section.set_selected(1);
    let fields = rule::catalogue(Kind::Window, hyprland_editor_core::BASELINE);
    for (i, s) in fields.iter().enumerate() {
        field.set_selected(i as u32);
        widget::<gtk::CheckButton>(&win, "rule-field-enabled").set_active(true);
        click(&win, "Update Rule Draft");
        assert!(
            current(&state)[0]
                .effects
                .iter()
                .any(|e| e.key == s.lua_key),
            "{}",
            s.id
        );
    }
    widget::<gtk::DropDown>(&win, "rule-enabled").set_selected(2);
    click(&win, "Update Rule Draft");
    assert_eq!(current(&state)[0].enabled, Some(false));
    let int_index = fields
        .iter()
        .position(|s| s.lua_key == "border_size")
        .unwrap();
    field.set_selected(int_index as u32);
    let body: gtk::Box = widget(&win, "rule-field-body");
    let entry = descendants(body.upcast_ref())
        .into_iter()
        .find_map(|w| w.downcast::<gtk::Entry>().ok())
        .unwrap();
    entry.set_text("9223372036854775807");
    click(&win, "Update Rule Draft");
    assert!(current(&state)[0]
        .effects
        .iter()
        .any(|e| e.key == "border_size" && e.value == EffectValue::Literal(Value::Int(i64::MAX))));
    entry.set_text("invalid");
    field.set_selected(0);
    assert_eq!(field.selected(), int_index as u32);
    entry.set_text("9223372036854775807");
    click(&win, "Duplicate");
    assert_eq!(current(&state)[1].name.as_deref(), Some("test copy 1"));
    click(&win, "Remove");
    assert_eq!(current(&state).len(), 1);
    click(&win, "Order Named Effects Explicitly");
    assert_eq!(current(&state).len(), 57);
    assert!(current(&state)
        .iter()
        .all(|r| r.effects.len() == 1 && r.enabled == Some(false)));
    click(&win, "Undo");
    assert_eq!(current(&state).len(), 1);
    click(&win, "Add Layer");
    assert_eq!(current(&state)[1].kind, Kind::Layer);
    widget::<gtk::DropDown>(&win, "rule-section").set_selected(1);
    let fields = rule::catalogue(Kind::Layer, hyprland_editor_core::BASELINE);
    for (i, s) in fields.iter().enumerate() {
        widget::<gtk::DropDown>(&win, "rule-field").set_selected(i as u32);
        widget::<gtk::CheckButton>(&win, "rule-field-enabled").set_active(true);
        click(&win, "Update Rule Draft");
        assert!(current(&state)[1]
            .effects
            .iter()
            .any(|e| e.key == s.lua_key));
    }
    let exported = state.borrow().export_lua().unwrap();
    let parsed = hyprland_editor_core::lua::parse(
        &exported,
        hyprland_editor_core::BASELINE,
        &hyprland_editor_core::catalogue(),
    )
    .unwrap();
    assert!(!parsed.opaque_effects, "{:?}", parsed.diagnostics);
    assert!(rule::same_declarations(&current(&state), &parsed.rules));
    let weak = widget::<gtk::DropDown>(&win, "rule-field").downgrade();
    drop(field);
    drop(section);
    drop(body);
    drop(entries);
    drop(entry);
    win.destroy();
    drop(win);
    settle();
    assert!(
        weak.upgrade().is_none(),
        "Rule controls leaked after closing"
    );
    // Imported text/number geometry and false conditions survive an untouched submit.
    let source="hl.window_rule{name='kept',match={class=true,float='off'},move={9223372036854775807,'monitor_h - window_h'},border_color='0xFF000000 45deg 0xFFFFFFFF 90deg'}";
    let rules = hyprland_editor_core::lua::parse(
        source,
        hyprland_editor_core::BASELINE,
        &hyprland_editor_core::catalogue(),
    )
    .unwrap()
    .rules;
    state.borrow_mut().replace_rules(rules.clone()).unwrap();
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let submit = editor(&panel, &state, &rules[0], &status, &label(""));
    submit().unwrap();
    assert_eq!(current(&state), rules);
    drop(submit);
    drop(panel);
    // Build every current-only field form without invoking any compositor.
    let state = super::initial_state();
    state.borrow_mut().draft.version = hyprland_editor_core::AUDITED_MAIN.into();
    let win = show(parent, &state, &status);
    click(&win, "Add");
    widget::<gtk::DropDown>(&win, "rule-section").set_selected(1);
    for (i, s) in rule::catalogue(Kind::Window, hyprland_editor_core::AUDITED_MAIN)
        .iter()
        .enumerate()
    {
        widget::<gtk::DropDown>(&win, "rule-field").set_selected(i as u32);
        widget::<gtk::CheckButton>(&win, "rule-field-enabled").set_active(true);
        click(&win, "Update Rule Draft");
        assert!(current(&state)[0]
            .effects
            .iter()
            .any(|e| e.key == s.lua_key));
    }
    widget::<gtk::DropDown>(&win, "rule-field").set_selected(
        rule::catalogue(Kind::Window, hyprland_editor_core::AUDITED_MAIN)
            .iter()
            .position(|s| s.lua_key == "move")
            .unwrap() as u32,
    );
    settle();
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
        let node = snapshot.to_node().unwrap();
        win.renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(format!("{path}.rules.png"))
            .unwrap();
    }
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        for (key, suffix) in [("opacity", "opacity"), ("border_color", "borders")] {
            widget::<gtk::DropDown>(&win, "rule-field").set_selected(
                rule::catalogue(Kind::Window, hyprland_editor_core::AUDITED_MAIN)
                    .iter()
                    .position(|s| s.lua_key == key)
                    .unwrap() as u32,
            );
            settle();
            let paintable = gtk::WidgetPaintable::new(Some(&win));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
            let node = snapshot.to_node().unwrap();
            win.renderer()
                .unwrap()
                .render_texture(&node, None)
                .save_to_png(format!("{path}.rules-{suffix}.png"))
                .unwrap();
        }
    }
    win.destroy();
    drop(win);
    settle();
}
