//! Graphical static workspace selectors. Navigation/action selectors are separate.
use super::{label, monitors::choices};
use adw::prelude::*;
use hyprland_editor_core::workspace_selector::{Counts, Predicate, Selector, WindowKind};
use std::{cell::RefCell, rc::Rc};
fn integer(panel: &gtk::Box, title: &str, value: u64) -> gtk::Entry {
    panel.append(&label(title));
    let e = gtk::Entry::builder()
        .text(value.to_string())
        .input_purpose(gtk::InputPurpose::Digits)
        .build();
    panel.append(&e);
    e
}
fn number(e: &gtk::Entry) -> Result<u64, String> {
    e.text()
        .parse()
        .map_err(|_| "Enter an unsigned whole number".into())
}
type GetPredicate = Box<dyn Fn() -> Result<Predicate, String>>;
fn predicate(parent: &gtk::Box, initial: &Predicate, version: &str) -> GetPredicate {
    let kind = choices(
        parent,
        &[
            "Workspace Number Range",
            "Special Workspace",
            "Named Workspace",
            "Display Name Starts With",
            "Display Name Ends With",
            "Monitor",
            "Window Count",
            "Fullscreen State",
        ],
        match initial {
            Predicate::Range { .. } => 0,
            Predicate::Special(_) => 1,
            Predicate::Named(_) => 2,
            Predicate::NamePrefix(_) => 3,
            Predicate::NameSuffix(_) => 4,
            Predicate::Monitor(_) => 5,
            Predicate::Count(_) => 6,
            Predicate::Fullscreen(_) => 7,
        },
    );
    let ranges = gtk::Box::new(gtk::Orientation::Vertical, 6);
    parent.append(&ranges);
    let (min, max) = match initial {
        Predicate::Range { min, max } => (*min, *max),
        Predicate::Count(c) => (c.min, c.max),
        _ => (1, 1),
    };
    let min = integer(&ranges, "Minimum", min);
    let max = integer(&ranges, "Maximum", max);
    let boolean = choices(
        parent,
        &["Yes", "No"],
        u32::from(matches!(
            initial,
            Predicate::Special(false) | Predicate::Named(false)
        )),
    );
    let text = gtk::Entry::builder()
        .text(match initial {
            Predicate::NamePrefix(s) | Predicate::NameSuffix(s) | Predicate::Monitor(s) => {
                s.as_str()
            }
            _ => "",
        })
        .build();
    parent.append(&text);
    let count = gtk::Box::new(gtk::Orientation::Vertical, 6);
    parent.append(&count);
    let classes = choices(
        &count,
        &["All Windows", "Tiled Windows", "Floating Windows"],
        match initial {
            Predicate::Count(c) => match c.kind {
                WindowKind::All => 0,
                WindowKind::Tiled => 1,
                WindowKind::Floating => 2,
            },
            _ => 0,
        },
    );
    let pinned = gtk::CheckButton::with_label("Only Pinned");
    let groups = gtk::CheckButton::with_label("Count Groups");
    let visible = gtk::CheckButton::with_label("Only Visible");
    if let Predicate::Count(c) = initial {
        pinned.set_active(c.pinned);
        groups.set_active(c.groups);
        visible.set_active(c.visible);
    }
    for b in [&pinned, &groups, &visible] {
        count.append(b);
    }
    let fullscreen = choices(
        parent,
        &[
            "No Fullscreen",
            "Fullscreen",
            "Maximized",
            "Internal State 2",
        ],
        match initial {
            Predicate::Fullscreen(n) => (*n + 1) as u32,
            _ => 0,
        },
    );
    let update: Rc<dyn Fn()> = Rc::new({
        let kind = kind.clone();
        let ranges = ranges.clone();
        let boolean = boolean.clone();
        let text = text.clone();
        let count = count.clone();
        let fullscreen = fullscreen.clone();
        move || {
            let k = kind.selected();
            ranges.set_visible(k == 0 || k == 6);
            boolean.set_visible(k == 1 || k == 2);
            text.set_visible((3..=5).contains(&k));
            count.set_visible(k == 6);
            fullscreen.set_visible(k == 7);
        }
    });
    update();
    let weak = Rc::downgrade(&update);
    kind.connect_selected_notify(move |_| {
        if let Some(f) = weak.upgrade() {
            f();
        }
    });
    let version = version.to_string();
    Box::new(move || {
        let _ = &update;
        let p = match kind.selected() {
            0 => Predicate::Range {
                min: number(&min)?,
                max: number(&max)?,
            },
            1 => Predicate::Special(boolean.selected() == 0),
            2 => Predicate::Named(boolean.selected() == 0),
            3 => Predicate::NamePrefix(text.text().into()),
            4 => Predicate::NameSuffix(text.text().into()),
            5 => Predicate::Monitor(text.text().into()),
            6 => Predicate::Count(Counts {
                min: number(&min)?,
                max: number(&max)?,
                kind: [WindowKind::All, WindowKind::Tiled, WindowKind::Floating]
                    [classes.selected() as usize],
                pinned: pinned.is_active(),
                groups: groups.is_active(),
                visible: visible.is_active(),
            }),
            _ => Predicate::Fullscreen(fullscreen.selected() as i8 - 1),
        };
        p.encode(&version)?;
        Ok(p)
    })
}
pub(super) fn workspace(
    panel: &gtk::Box,
    initial: &str,
    version: &str,
) -> Box<dyn Fn() -> Result<String, String>> {
    let decoded = Selector::parse(initial, version);
    let mode = choices(
        panel,
        &[
            "All Existing Workspaces",
            "Workspace Number",
            "Named Workspace",
            "Special Workspace",
            "Match All Filter Clauses",
            "Retained Literal (Unresolved)",
        ],
        match decoded {
            Selector::All => 0,
            Selector::Number(_) => 1,
            Selector::Named(_) => 2,
            Selector::Special { .. } => 3,
            Selector::Filters(_) => 4,
            Selector::Unresolved(_) => 5,
        },
    );
    mode.set_widget_name("workspace-selector-kind");
    let numeric = gtk::Box::new(gtk::Orientation::Vertical, 6);
    panel.append(&numeric);
    let id = integer(
        &numeric,
        "Workspace Number",
        if let Selector::Number(n) = decoded {
            n
        } else {
            1
        },
    );
    let name = gtk::Entry::builder()
        .text(match &decoded {
            Selector::Named(s) => s.as_str(),
            Selector::Special { name: Some(s) } => s.as_str(),
            _ => "",
        })
        .placeholder_text("Workspace name")
        .build();
    panel.append(&name);
    let default_special = gtk::CheckButton::with_label("Default Special Workspace");
    default_special.set_active(matches!(decoded, Selector::Special { name: None }));
    panel.append(&default_special);
    let literal = gtk::Entry::builder().text(initial).build();
    literal.set_widget_name("workspace-selector-literal");
    panel.append(&literal);
    let filters = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.append(&filters);
    filters.append(&label("All clauses must match an existing workspace. These filters do not create a new workspace."));
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 8);
    filters.append(&rows);
    let getters = Rc::new(RefCell::new(Vec::<(gtk::Box, GetPredicate)>::new()));
    let add: Rc<dyn Fn(Predicate)> = Rc::new({
        let rows = rows.downgrade();
        let getters = getters.clone();
        let version = version.to_string();
        move |value| {
            let Some(rows) = rows.upgrade() else {
                return;
            };
            if getters.borrow().len() >= 32 {
                return;
            }
            let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let get = predicate(&row, &value, &version);
            let remove = gtk::Button::with_label("Remove Clause");
            row.append(&remove);
            rows.append(&row);
            getters.borrow_mut().push((row.clone(), get));
            let weak_row = row.downgrade();
            let weak_rows = rows.downgrade();
            let weak_get = Rc::downgrade(&getters);
            remove.connect_clicked(move |_| {
                if let (Some(row), Some(rows), Some(getters)) =
                    (weak_row.upgrade(), weak_rows.upgrade(), weak_get.upgrade())
                {
                    getters.borrow_mut().retain(|(r, _)| *r != row);
                    rows.remove(&row);
                }
            });
        }
    });
    if let Selector::Filters(v) = &decoded {
        for p in v {
            add(p.clone());
        }
    }
    let button = gtk::Button::with_label("Add Filter Clause");
    filters.append(&button);
    let add_c = add.clone();
    button.connect_clicked(move |_| add_c(Predicate::Range { min: 1, max: 1 }));
    let update: Rc<dyn Fn()> = Rc::new({
        let mode = mode.clone();
        let numeric = numeric.clone();
        let name = name.clone();
        let default_special = default_special.clone();
        let literal = literal.clone();
        let filters = filters.clone();
        move || {
            let m = mode.selected();
            numeric.set_visible(m == 1);
            name.set_visible(m == 2 || m == 3);
            default_special.set_visible(m == 3);
            literal.set_visible(m == 5);
            filters.set_visible(m == 4);
        }
    });
    update();
    let weak = Rc::downgrade(&update);
    mode.connect_selected_notify(move |_| {
        if let Some(f) = weak.upgrade() {
            f();
        }
    });
    let original = initial.to_string();
    let version = version.to_string();
    Box::new(move || {
        let _ = (&add, &update);
        let next = match mode.selected() {
            0 => Selector::All,
            1 => Selector::Number(number(&id)?),
            2 => Selector::Named(name.text().into()),
            3 => Selector::Special {
                name: (!default_special.is_active()).then(|| name.text().into()),
            },
            4 => Selector::Filters(
                getters
                    .borrow()
                    .iter()
                    .map(|(_, g)| g())
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            _ => Selector::Unresolved(literal.text().into()),
        };
        if next == decoded {
            return Ok(original.clone());
        }
        next.encode(&version)
    })
}
