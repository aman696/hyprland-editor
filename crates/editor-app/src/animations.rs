//! Selected animation forms and inert control-point diagrams; no compositor connection.
use super::{label, monitors::compact, Shared};
use adw::prelude::*;
use hyprland_editor_core::{
    animation::{self, Curve, CurveKind, Declaration, Draft, Node, Operation},
    Value,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Getter = Rc<dyn Fn() -> Result<(), String>>;
fn current(state: &Shared) -> Draft {
    let s = state.borrow();
    s.animations
        .get(&s.draft.version)
        .cloned()
        .unwrap_or_default()
}
fn unique(draft: &Draft) -> String {
    (0..)
        .map(|n| format!("animation-{n}"))
        .find(|id| draft.declarations.iter().all(|d| d.id != *id))
        .unwrap()
}
fn curve_name(draft: &Draft) -> String {
    (1..)
        .map(|n| format!("curve-{n}"))
        .find(|name| {
            draft
                .declarations
                .iter()
                .all(|d| !matches!(&d.operation, Operation::Curve(c) if c.name == *name))
        })
        .unwrap()
}
fn entry(panel: &gtk::Box, title: &str, initial: &str, name: &str) -> gtk::Entry {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let text = label(title);
    text.set_width_chars(12);
    row.append(&text);
    let e = gtk::Entry::builder()
        .text(initial)
        .max_length(16384)
        .hexpand(true)
        .build();
    e.set_widget_name(name);
    row.append(&e);
    panel.append(&row);
    e
}
fn number_form(
    panel: &gtk::Box,
    title: &str,
    original: Value,
    name: &str,
    bounds: Option<(f64, f64)>,
    native: bool,
) -> (gtk::Entry, Rc<dyn Fn() -> Result<Value, String>>) {
    let display = match &original {
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "1" } else { "0" }.into(),
        Value::Float(n) if native => (*n as f32).to_string(),
        v => v.lua(),
    };
    let e = entry(panel, title, &display, name);
    if let Some((mut min, mut max)) = bounds {
        if native {
            if let Ok(n) = animation::number(&original, true) {
                min = min.min(n);
                max = max.max(n);
            }
        }
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, 0.01);
        scale.set_draw_value(false);
        scale.set_hexpand(true);
        scale.set_value(
            animation::number(&original, true)
                .unwrap_or(min)
                .clamp(min, max),
        );
        panel.append(&scale);
        let weak = e.downgrade();
        scale.connect_value_changed(move |scale| {
            if let Some(e) = weak.upgrade() {
                e.set_text(&format!("{:.6}", scale.value()));
            }
        });
    } else {
        // Buttons provide GUI stepping; exact entry avoids imposing an artificial upper bound.
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        for (title, delta) in [("−1", -1.), ("+1", 1.)] {
            let button = gtk::Button::with_label(title);
            row.append(&button);
            let weak = e.downgrade();
            button.connect_clicked(move |_| {
                if let Some(e) = weak.upgrade() {
                    if let Ok(n) = e.text().parse::<f64>() {
                        e.set_text(&(n + delta).to_string());
                    }
                }
            });
        }
        panel.append(&row);
    }
    let field = e.clone();
    (
        e,
        Rc::new(move || {
            if field.text().as_str() == display {
                return Ok(original.clone());
            }
            let n = field
                .text()
                .parse::<f64>()
                .map_err(|_| "Enter a finite number".to_string())?;
            if !n.is_finite() {
                return Err("Enter a finite number".into());
            }
            Ok(Value::Float(n))
        }),
    )
}
fn plot_domain(fields: &[gtk::Entry; 4], wide: bool) -> [f64; 4] {
    let mut d = [-1.0f64, 2., -1., 2.];
    if wide {
        for (i, e) in fields.iter().enumerate() {
            if let Ok(n) = e.text().parse::<f64>() {
                if n.is_finite() && (n as f32).is_finite() {
                    let at = if i % 2 == 0 { 0 } else { 2 };
                    d[at] = d[at].min(n);
                    d[at + 1] = d[at + 1].max(n);
                }
            }
        }
    }
    d
}
fn diagram(panel: &gtk::Box, fields: [gtk::Entry; 4], wide: bool) {
    panel.append(&label("Bézier control points — drag either handle. This diagram shows curve geometry; it does not preview your desktop."));
    let area = gtk::DrawingArea::builder()
        .content_width(300)
        .content_height(160)
        .hexpand(true)
        .build();
    area.set_widget_name("animation-diagram");
    panel.append(&area);
    let entries = fields.clone();
    area.set_draw_func(move |_, cr, width, height| {
        let w = f64::from(width);
        let h = f64::from(height);
        let [xmin, xmax, ymin, ymax] = plot_domain(&entries, wide);
        let xy = |x: f64, y: f64| {
            (
                (x - xmin) / (xmax - xmin) * w,
                (ymax - y) / (ymax - ymin) * h,
            )
        };
        cr.set_source_rgb(0.15, 0.17, 0.22);
        let _ = cr.paint();
        cr.set_source_rgb(0.36, 0.4, 0.5);
        cr.set_line_width(1.);
        for n in [0., 1.] {
            let (x, _) = xy(n, 0.);
            cr.move_to(x, 0.);
            cr.line_to(x, h);
            let (_, y) = xy(0., n);
            cr.move_to(0., y);
            cr.line_to(w, y);
        }
        let _ = cr.stroke();
        let points = entries
            .iter()
            .map(|e| e.text().parse::<f64>())
            .collect::<Result<Vec<_>, _>>();
        let Ok(p) = points else {
            return;
        };
        if p.iter().any(|n| {
            !n.is_finite()
                || if wide {
                    !(*n as f32).is_finite()
                } else {
                    !(-1. ..=2.).contains(n)
                }
        }) {
            return;
        }
        let c = Curve {
            name: String::new(),
            shape: CurveKind::Bezier {
                points: [
                    [Value::Float(p[0]), Value::Float(p[1])],
                    [Value::Float(p[2]), Value::Float(p[3])],
                ],
            },
        };
        cr.set_source_rgb(0.72, 0.61, 0.94);
        cr.set_line_width(2.);
        for n in 0..=100 {
            let [x, y] = c.diagram_point(f64::from(n) / 100.).unwrap();
            let (x, y) = xy(x, y);
            if n == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        let _ = cr.stroke();
        cr.set_source_rgb(0.65, 0.7, 0.8);
        for (i, (x0, y0)) in [(0, (0., 0.)), (2, (1., 1.))] {
            let (x, y) = xy(p[i], p[i + 1]);
            let (a, b) = xy(x0, y0);
            cr.move_to(a, b);
            cr.line_to(x, y);
            let _ = cr.stroke();
            cr.arc(x, y, 6., 0., std::f64::consts::TAU);
            let _ = cr.fill();
        }
    });
    for e in &fields {
        let weak = area.downgrade();
        e.connect_changed(move |_| {
            if let Some(area) = weak.upgrade() {
                area.queue_draw();
            }
        });
    }
    let gesture = gtk::GestureDrag::new();
    let start = Rc::new(Cell::new(None::<(usize, f64, f64, [f64; 4])>));
    let weak = area.downgrade();
    let entries = fields.clone();
    let selected = start.clone();
    gesture.connect_drag_begin(move |_, x, y| {
        selected.set(None);
        let Some(area) = weak.upgrade() else {
            return;
        };
        let w = f64::from(area.width());
        let h = f64::from(area.height());
        let domain = plot_domain(&entries, wide);
        let [xmin, xmax, ymin, ymax] = domain;
        for i in [0, 2] {
            if let (Ok(px), Ok(py)) = (
                entries[i].text().parse::<f64>(),
                entries[i + 1].text().parse::<f64>(),
            ) {
                if ((px - xmin) / (xmax - xmin) * w - x).hypot((ymax - py) / (ymax - ymin) * h - y)
                    <= 18.
                {
                    selected.set(Some((i, x, y, domain)));
                    break;
                }
            }
        }
    });
    let weak = area.downgrade();
    gesture.connect_drag_update(move |_, dx, dy| {
        let (Some((i, x, y, [xmin, xmax, ymin, ymax])), Some(area)) = (start.get(), weak.upgrade())
        else {
            return;
        };
        if area.width() == 0 || area.height() == 0 {
            return;
        }
        fields[i].set_text(&format!(
            "{:.6}",
            ((x + dx) / f64::from(area.width()) * (xmax - xmin) + xmin).clamp(xmin, xmax)
        ));
        fields[i + 1].set_text(&format!(
            "{:.6}",
            (ymax - (y + dy) / f64::from(area.height()) * (ymax - ymin)).clamp(ymin, ymax)
        ));
    });
    area.add_controller(gesture);
}
fn editor(
    panel: &gtk::Box,
    state: &Shared,
    declaration: &Declaration,
    status: &gtk::Label,
    feedback: &gtk::Label,
    update_inheritance: &Rc<dyn Fn()>,
) -> Getter {
    let version = state.borrow().draft.version.clone();
    if declaration.legacy {
        panel.append(&label("Imported .conf animation. Review Source Copy includes these edits; no desktop animation is activated."));
    }
    let get: Rc<dyn Fn() -> Result<Operation, String>> = match &declaration.operation {
        Operation::Curve(curve) => {
            let name = entry(panel, "Curve name", &curve.name, "animation-name");
            match &curve.shape {
                CurveKind::Bezier { points } => {
                    let grid = gtk::FlowBox::builder()
                        .selection_mode(gtk::SelectionMode::None)
                        .min_children_per_line(1)
                        .max_children_per_line(2)
                        .row_spacing(8)
                        .column_spacing(12)
                        .homogeneous(true)
                        .build();
                    panel.append(&grid);
                    let mut entries = vec![];
                    let mut getters = vec![];
                    for (i, title) in ["First X", "First Y", "Second X", "Second Y"]
                        .iter()
                        .enumerate()
                    {
                        let cell = gtk::Box::new(gtk::Orientation::Vertical, 4);
                        grid.insert(&cell, -1);
                        let (e, g) = number_form(
                            &cell,
                            title,
                            points[i / 2][i % 2].clone(),
                            &format!("animation-point-{i}"),
                            Some((-1., 2.)),
                            declaration.legacy,
                        );
                        entries.push(e);
                        getters.push(g);
                    }
                    diagram(panel, entries.try_into().unwrap(), declaration.legacy);
                    Rc::new(move || {
                        Ok(Operation::Curve(Curve {
                            name: name.text().into(),
                            shape: CurveKind::Bezier {
                                points: [
                                    [getters[0]()?, getters[1]()?],
                                    [getters[2]()?, getters[3]()?],
                                ],
                            },
                        }))
                    })
                }
                CurveKind::Spring {
                    stiffness,
                    damping,
                    mass,
                    dampening_alias,
                } => {
                    panel.append(&label("Spring motion settles according to stiffness, damping and mass. Each must be greater than 0.5. Speed does not give a fixed spring duration."));
                    let (_, k) = number_form(
                        panel,
                        "Stiffness",
                        stiffness.clone(),
                        "animation-stiffness",
                        None,
                        false,
                    );
                    let (_, d) = number_form(
                        panel,
                        "Damping",
                        damping.clone(),
                        "animation-damping",
                        None,
                        false,
                    );
                    let (_, m) =
                        number_form(panel, "Mass", mass.clone(), "animation-mass", None, false);
                    let alias = *dampening_alias;
                    Rc::new(move || {
                        Ok(Operation::Curve(Curve {
                            name: name.text().into(),
                            shape: CurveKind::Spring {
                                stiffness: k()?,
                                damping: d()?,
                                mass: m()?,
                                dampening_alias: alias,
                            },
                        }))
                    })
                }
            }
        }
        Operation::Node(node) => {
            let nodes = animation::catalogue(&version);
            let names = nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>();
            let leaf = gtk::DropDown::from_strings(&names);
            compact(&leaf);
            leaf.set_widget_name("animation-leaf");
            leaf.set_selected(nodes.iter().position(|n| n.name == node.leaf).unwrap_or(0) as u32);
            panel.append(&label("Animation branch"));
            panel.append(&leaf);
            let enabled = gtk::CheckButton::with_label("Enable this branch");
            enabled.set_widget_name("animation-enabled");
            enabled.set_active(animation::enabled(&node.enabled).unwrap_or(false));
            panel.append(&enabled);
            let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
            panel.append(&fields);
            let original_speed = node.speed.clone();
            let (_, speed) = number_form(
                &fields,
                "Speed",
                node.speed.clone().unwrap_or(Value::Float(8.)),
                "animation-speed",
                Some((0.01, 100.)),
                declaration.legacy,
            );
            fields.append(&label("Bézier duration = speed × 100 ms. Disabled overrides keep optional fields but use the compositor's disabled defaults."));
            let draft = current(state);
            let index = draft
                .declarations
                .iter()
                .position(|d| d.id == declaration.id)
                .unwrap();
            let mut curves = vec![(false, "default".to_string()), (false, "linear".into())];
            if !declaration.legacy {
                curves.push((true, "default".into()));
            }
            for d in &draft.declarations[..index] {
                if d.legacy != declaration.legacy {
                    continue;
                }
                if let Operation::Curve(c) = &d.operation {
                    let item = (matches!(c.shape, CurveKind::Spring { .. }), c.name.clone());
                    if !curves.contains(&item) {
                        curves.push(item);
                    }
                }
            }
            let initial = node
                .bezier
                .as_ref()
                .map(|n| (false, n.clone()))
                .or_else(|| node.spring.as_ref().map(|n| (true, n.clone())));
            if let Some(item) = &initial {
                if !curves.contains(item) {
                    curves.push(item.clone());
                }
            }
            let titles = curves
                .iter()
                .map(|(s, n)| format!("{} · {n}", if *s { "Spring" } else { "Bézier" }))
                .collect::<Vec<_>>();
            let curve =
                gtk::DropDown::from_strings(&titles.iter().map(String::as_str).collect::<Vec<_>>());
            compact(&curve);
            curve.set_widget_name("animation-curve");
            let initial_index = initial
                .as_ref()
                .and_then(|c| curves.iter().position(|p| p == c))
                .unwrap_or(0) as u32;
            curve.set_selected(initial_index);
            fields.append(&curve);
            let style = entry(
                &fields,
                "Style",
                node.style.as_deref().unwrap_or(""),
                "animation-style",
            );
            let style_model = gtk::StringList::new(&[]);
            let style_choices = gtk::DropDown::builder().model(&style_model).build();
            let available = move |name: &str| {
                let mut styles = vec!["Choose a style…", "Default"];
                styles.extend(
                    [
                        "slide",
                        "slide left",
                        "slide right",
                        "slide top",
                        "slide bottom",
                        "popin",
                        "popin 80%",
                        "gnome",
                        "gnomed",
                        "fade",
                        "slidevert",
                        "slidefade",
                        "slidefadevert",
                        "loop",
                        "once",
                    ]
                    .into_iter()
                    .filter(|s| animation::validate_style(name, s).is_ok()),
                );
                styles
            };
            style_model.splice(0, 0, &available(&node.leaf));
            let model_c = style_model.clone();
            let style_pick = style_choices.downgrade();
            leaf.connect_selected_notify(move |leaf| {
                if let Some(name) = leaf.selected_item().and_downcast::<gtk::StringObject>() {
                    model_c.splice(0, model_c.n_items(), &available(&name.string()));
                    if let Some(pick) = style_pick.upgrade() {
                        pick.set_selected(0);
                    }
                }
            });
            compact(&style_choices);
            fields.append(&style_choices);
            let weak = style.downgrade();
            style_choices.connect_selected_notify(move |p| {
                if let Some(style) = weak.upgrade() {
                    if p.selected() == 1 {
                        style.set_text("");
                    } else if p.selected() > 1 {
                        if let Some(s) = p.selected_item().and_downcast::<gtk::StringObject>() {
                            style.set_text(&s.string());
                        }
                    }
                }
            });
            fields.set_sensitive(enabled.is_active());
            let weak = fields.downgrade();
            enabled.connect_toggled(move |e| {
                if let Some(f) = weak.upgrade() {
                    f.set_sensitive(e.is_active());
                }
            });
            if let Ok(resolved) = draft.resolve(&version) {
                if let Some(r) = resolved.get(&node.leaf) {
                    panel.append(&label(&format!(
                        "Draft inheritance: {} · {} · curve {} · origin {}{}",
                        if r.value.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        },
                        r.value
                            .bezier_duration_ms()
                            .map_or("spring settling".into(), |ms| format!("{ms} ms")),
                        r.value.curve,
                        r.origin,
                        if r.user_override {
                            " (draft override)"
                        } else {
                            " (built-in)"
                        }
                    )));
                }
            }
            let original = node.clone();
            Rc::new(move || {
                let mut n = original.clone();
                n.leaf = nodes[leaf.selected() as usize].name.clone();
                if enabled.is_active() != animation::enabled(&original.enabled)? {
                    n.enabled = Value::Bool(enabled.is_active());
                }
                let v = speed()?;
                if original_speed.is_some() || enabled.is_active() || v != Value::Float(8.) {
                    n.speed = Some(v);
                }
                if curve.selected() != initial_index || (enabled.is_active() && initial.is_none()) {
                    let (spring, name) = &curves[curve.selected() as usize];
                    n.bezier = (!spring).then(|| name.clone());
                    n.spring = (*spring).then(|| name.clone());
                }
                if style.text().as_str() != original.style.as_deref().unwrap_or("") {
                    n.style = Some(style.text().into());
                }
                Ok(Operation::Node(n))
            })
        }
    };
    let state = state.clone();
    let id = declaration.id.clone();
    let status = status.clone();
    let update_inheritance = update_inheritance.clone();
    let submit: Getter = Rc::new(move || {
        let mut draft = current(&state);
        let d = draft
            .declarations
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or("Animation declaration removed")?;
        d.operation = get()?;
        state.borrow_mut().replace_animations(draft)?;
        super::summary(&state, &status);
        update_inheritance();
        Ok(())
    });
    let update = gtk::Button::with_label("Update Animation Draft");
    update.set_widget_name("animation-update");
    update.add_css_class("suggested-action");
    panel.append(&update);
    let weak = Rc::downgrade(&submit);
    let feedback = feedback.clone();
    update.connect_clicked(move |_| {
        if let Some(g) = weak.upgrade() {
            feedback.set_text(&match g() {
                Ok(()) => "Animation draft updated".into(),
                Err(e) => e,
            });
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
    let parent = parent.downgrade();
    let state = state.clone();
    let status = status.clone();
    button.connect_clicked(move |_| {
        if let Some(parent) = parent.upgrade() {
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
        .title("Animations")
        .transient_for(parent)
        .modal(true)
        .default_width(700)
        .default_height(760)
        .build();
    win.set_size_request(350, 480);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for setter in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        setter(content.upcast_ref(), 16);
    }
    content.append(&label("Edit animation branches and named curves. Declarations run in order; branches inherit their nearest overridden ancestor."));
    let model = gtk::StringList::new(&[]);
    let pick = gtk::DropDown::builder().model(&model).build();
    compact(&pick);
    pick.set_widget_name("animation-declaration");
    content.append(&pick);
    let add_row = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(3)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    let controls = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(3)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    let buttons = [
        "Add Branch",
        "Add Bézier",
        "Add Spring",
        "Duplicate",
        "Remove",
        "↑",
        "↓",
        "Undo",
    ]
    .map(gtk::Button::with_label);
    buttons[2].set_visible(
        !state
            .borrow()
            .legacy_sources
            .contains_key(&state.borrow().draft.version),
    );
    for b in &buttons[..3] {
        add_row.insert(b, -1);
    }
    for b in &buttons[3..] {
        controls.insert(b, -1);
    }
    let management = gtk::Box::new(gtk::Orientation::Vertical, 6);
    management.append(&add_row);
    management.append(&controls);
    let manage = gtk::Expander::builder()
        .label("Manage Animations")
        .child(&management)
        .build();
    manage.set_widget_name("animation-management");
    content.append(&manage);
    let tree = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let nodes = animation::catalogue(&state.borrow().draft.version);
    let inspect =
        gtk::DropDown::from_strings(&nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>());
    compact(&inspect);
    inspect.set_widget_name("animation-inspect-node");
    tree.append(&inspect);
    let details = label("");
    tree.append(&details);
    let expander = gtk::Expander::builder()
        .label("Inspect Branch Inheritance")
        .child(&tree)
        .build();
    content.append(&expander);
    let update_inheritance: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let inspect = inspect.downgrade();
        let details = details.downgrade();
        move || {
            let (Some(inspect), Some(details)) = (inspect.upgrade(), details.upgrade()) else {
                return;
            };
            let Some(name) = inspect.selected_item().and_downcast::<gtk::StringObject>() else {
                return;
            };
            let s = state.borrow();
            let draft = s
                .animations
                .get(&s.draft.version)
                .cloned()
                .unwrap_or_default();
            match draft.resolve(&s.draft.version) {
                Ok(resolved) => {
                    if let Some(r) = resolved.get(name.string().as_str()) {
                        details.set_text(&format!(
                            "{} · source branch {} · {} · curve {} · style {}",
                            if r.value.enabled {
                                "Enabled"
                            } else {
                                "Disabled"
                            },
                            r.origin,
                            if !r.value.enabled {
                                "no motion".into()
                            } else {
                                r.value
                                    .bezier_duration_ms()
                                    .map_or("spring settling".into(), |ms| format!("{ms} ms"))
                            },
                            r.value.curve,
                            if r.value.style.is_empty() {
                                "default"
                            } else {
                                &r.value.style
                            }
                        ));
                    }
                }
                Err(e) => details.set_text(&e),
            }
        }
    });
    let weak = Rc::downgrade(&update_inheritance);
    inspect.connect_selected_notify(move |_| {
        if let Some(update) = weak.upgrade() {
            update();
        }
    });
    update_inheritance();
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.append(&panel);
    let feedback = label("");
    feedback.set_widget_name("animation-feedback");
    content.append(&feedback);
    let check = gtk::Button::with_label("Check Animation Relationships");
    content.append(&check);
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
            if let Some(g) = submit.borrow().as_ref() {
                g()?;
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
        let busy = busy.clone();
        let active = active.clone();
        let update_inheritance = update_inheritance.clone();
        move || {
            if busy.replace(true) {
                return;
            }
            let draft = current(&state);
            let index = if active.get() == gtk::INVALID_LIST_POSITION {
                0
            } else {
                (active.get() as usize).min(draft.declarations.len().saturating_sub(1))
            };
            let titles = draft
                .declarations
                .iter()
                .map(|d| match &d.operation {
                    Operation::Curve(c) => format!(
                        "{} · {}",
                        if matches!(c.shape, CurveKind::Spring { .. }) {
                            "Spring"
                        } else {
                            "Bézier"
                        },
                        c.name
                    ),
                    Operation::Node(n) => format!("Branch · {}", n.leaf),
                })
                .collect::<Vec<_>>();
            model.splice(
                0,
                model.n_items(),
                &titles.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            pick.set_selected(if draft.is_empty() {
                gtk::INVALID_LIST_POSITION
            } else {
                index as u32
            });
            active.set(pick.selected());
            while let Some(child) = panel.first_child() {
                panel.remove(&child);
            }
            *submit.borrow_mut() = draft
                .declarations
                .get(index)
                .map(|d| editor(&panel, &state, d, &status, &feedback, &update_inheritance));
            busy.set(false);
            update_inheritance();
        }
    });
    let weak = Rc::downgrade(&refresh);
    let f = flush.clone();
    let b = busy.clone();
    let a = active.clone();
    let message = feedback.clone();
    pick.connect_selected_notify(move |p| {
        if b.get() {
            return;
        }
        if let Err(e) = f() {
            b.set(true);
            p.set_selected(a.get());
            b.set(false);
            message.set_text(&e);
            return;
        }
        a.set(p.selected());
        if let Some(r) = weak.upgrade() {
            r();
        }
    });
    for (op, button) in buttons.iter().enumerate() {
        let state = state.clone();
        let status = status.clone();
        let feedback = feedback.clone();
        let pick = pick.clone();
        let flush = flush.clone();
        let refresh = refresh.clone();
        let active = active.clone();
        button.connect_clicked(move |_| {
            if op != 4 && op != 7 {
                if let Err(e) = flush() {
                    feedback.set_text(&e);
                    return;
                }
            }
            let mut draft = current(&state);
            let index = pick.selected() as usize;
            let mut target = index;
            if op == 7 {
                state.borrow_mut().undo_last();
            } else {
                match op {
                    0..=2 => {
                        let operation = match op {
                            0 => Operation::Node(Node::new("windows")),
                            1 => Operation::Curve(Curve::bezier(&curve_name(&draft))),
                            _ => Operation::Curve(Curve::spring(&curve_name(&draft))),
                        };
                        let id = unique(&draft);
                        target = draft.declarations.len();
                        draft.declarations.push(Declaration {
                            id,
                            operation,
                            legacy: state
                                .borrow()
                                .legacy_sources
                                .contains_key(&state.borrow().draft.version),
                        });
                    }
                    3 => {
                        let Some(d) = draft.declarations.get(index) else {
                            return;
                        };
                        let mut d = d.clone();
                        d.id = unique(&draft);
                        if let Operation::Curve(c) = &mut d.operation {
                            c.name = curve_name(&draft);
                        }
                        target = index + 1;
                        draft.declarations.insert(target, d);
                    }
                    4 => {
                        if index < draft.declarations.len() {
                            draft.declarations.remove(index);
                        }
                    }
                    5 | 6 => {
                        let Some(next) = (if op == 5 {
                            index.checked_sub(1)
                        } else {
                            index.checked_add(1)
                        }) else {
                            return;
                        };
                        if next >= draft.declarations.len() {
                            return;
                        }
                        draft.declarations.swap(index, next);
                        target = next;
                    }
                    _ => {}
                }
                if let Err(e) = state.borrow_mut().replace_animations(draft) {
                    feedback.set_text(&e);
                    return;
                }
            }
            active.set(target as u32);
            refresh();
            super::summary(&state, &status);
            feedback.set_text("Animation draft changed");
        });
    }
    let f = flush.clone();
    let state_c = state.clone();
    let feedback_c = feedback.clone();
    check.connect_clicked(move |_| {
        if let Err(e) = f() {
            feedback_c.set_text(&e);
            return;
        }
        let findings = current(&state_c).findings();
        feedback_c.set_text(&if findings.is_empty() {
            "No declaration conflicts found. Desktop behavior is unverified.".into()
        } else {
            findings.join("\n")
        });
    });
    refresh();
    win.connect_close_request(move |_| {
        let _ = &refresh;
        if let Err(e) = flush() {
            feedback.set_text(&e);
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    gtk::prelude::GtkWindowExt::set_focus(&win, Some(&pick));
    win.present();
    win
}
#[cfg(test)]
pub(super) fn test_editor(parent: &adw::ApplicationWindow) {
    fn descendants(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut out = vec![];
        let mut child = w.first_child();
        while let Some(w) = child {
            out.push(w.clone());
            out.extend(descendants(&w));
            child = w.next_sibling();
        }
        out
    }
    fn widget<T: IsA<gtk::Widget> + gtk::glib::object::ObjectType>(
        win: &adw::Window,
        name: &str,
    ) -> T {
        descendants(win.upcast_ref())
            .into_iter()
            .find(|w| w.widget_name() == name)
            .unwrap_or_else(|| panic!("Missing animation widget {name}"))
            .downcast()
            .unwrap()
    }
    fn click(win: &adw::Window, name: &str) {
        for expander in descendants(win.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Expander>().ok())
        {
            expander.set_expanded(true);
        }
        descendants(win.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(name))
            .unwrap()
            .emit_clicked();
    }
    let state = super::initial_state();
    let status = label("");
    let win = show(parent, &state, &status);
    click(&win, "Add Bézier");
    widget::<gtk::Entry>(&win, "animation-name").set_text("ease");
    widget::<gtk::Entry>(&win, "animation-point-0").set_text("0.35");
    click(&win, "Update Animation Draft");
    assert!(state.borrow().dirty());
    click(&win, "Add Branch");
    widget::<gtk::Entry>(&win, "animation-speed").set_text("5");
    widget::<gtk::DropDown>(&win, "animation-curve").set_selected(3);
    click(&win, "Update Animation Draft");
    let export = state.borrow().export_lua().unwrap();
    let report = hyprland_editor_core::lua::parse(
        &export,
        hyprland_editor_core::BASELINE,
        &state.borrow().catalogue,
    )
    .unwrap();
    assert!(!report.opaque_effects);
    assert!(report.animations.same_declarations(&current(&state)));
    let generation = state.borrow().generation;
    widget::<gtk::Entry>(&win, "animation-speed").set_text("0");
    widget::<gtk::DropDown>(&win, "animation-declaration").set_selected(0);
    assert_eq!(
        widget::<gtk::DropDown>(&win, "animation-declaration").selected(),
        1
    );
    assert_eq!(state.borrow().generation, generation);
    widget::<gtk::Entry>(&win, "animation-speed").set_text("6");
    click(&win, "Update Animation Draft");
    click(&win, "Undo");
    let Operation::Node(n) = &current(&state).declarations[1].operation else {
        panic!()
    };
    assert_eq!(
        animation::number(n.speed.as_ref().unwrap(), true).unwrap(),
        5.
    );
    click(&win, "Duplicate");
    assert_eq!(current(&state).declarations.len(), 3);
    click(&win, "Remove");
    assert_eq!(current(&state).declarations.len(), 2);
    widget::<gtk::DropDown>(&win, "animation-declaration").set_selected(0);
    click(&win, "↓");
    assert_eq!(
        widget::<gtk::DropDown>(&win, "animation-declaration").selected(),
        0
    );
    assert!(widget::<gtk::Label>(&win, "animation-feedback")
        .text()
        .contains("not defined"));
    let context = gtk::glib::MainContext::default();
    for _ in 0..10 {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, win.width() as f64, win.height() as f64);
        let node = snapshot.to_node().unwrap();
        win.renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(format!("{path}.animations.png"))
            .unwrap();
    }
    let area: gtk::DrawingArea = widget(&win, "animation-diagram");
    let controllers = area.observe_controllers();
    let drag = (0..controllers.n_items())
        .find_map(|i| controllers.item(i).and_downcast::<gtk::GestureDrag>())
        .unwrap();
    let width = f64::from(area.width());
    let height = f64::from(area.height());
    assert!(width > 0. && height > 0.);
    drag.emit_by_name::<()>("drag-begin", &[&(1.35 / 3. * width), &(1.9 / 3. * height)]);
    drag.emit_by_name::<()>("drag-update", &[&(0.1 * width), &0.]);
    click(&win, "Update Animation Draft");
    let Operation::Curve(c) = &current(&state).declarations[0].operation else {
        panic!()
    };
    let CurveKind::Bezier { points } = &c.shape else {
        panic!()
    };
    assert!((animation::number(&points[0][0], true).unwrap() - 0.65).abs() < 0.00001);
    click(&win, "Add Spring");
    widget::<gtk::Entry>(&win, "animation-mass").set_text("0.5");
    click(&win, "Update Animation Draft");
    assert!(widget::<gtk::Label>(&win, "animation-feedback")
        .text()
        .contains("greater than 0.5"));
    widget::<gtk::Entry>(&win, "animation-mass").set_text("1");
    click(&win, "Update Animation Draft");
    let text = state.borrow().export_lua().unwrap();
    assert!(text.contains("dampening"));
    let weak_state = Rc::downgrade(&state);
    win.close();
    drop(win);
    drop(state);
    while context.pending() {
        context.iteration(false);
    }
    assert!(
        weak_state.upgrade().is_none(),
        "closed animation editor retained its draft state"
    );
}
