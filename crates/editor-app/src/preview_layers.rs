//! Lazy scalar-layer review. Choices affect a sample only, never stored drafts
//! or original configuration. The staging role independently rebuilds them.
use super::{
    projection_worker::{self, Cancellation},
    Shared,
};
use adw::prelude::*;
use hyprland_editor_core::{workspace::Workspace, Value};
use hyprland_editor_projection::layers::{Captured, Choice, Review, Row, Selection};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::Arc,
};
fn label(value: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(value));
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_xalign(0.0);
    label
}
fn short(value: &str) -> String {
    let mut text: String = value.chars().take(100).collect();
    if value.chars().count() > 100 {
        text.push('…');
    }
    text
}
fn row_text(row: &Row) -> String {
    let pending = row
        .pending
        .as_ref()
        .map(Value::lua)
        .map(|s| short(&s))
        .unwrap_or_else(|| "Not set".into());
    let captured = match &row.captured {
        Captured::Value {
            value,
            provisional,
            selectable,
            ..
        } => format!(
            "{}{}{}",
            short(&value.lua()),
            if *provisional { " · partial" } else { "" },
            if !*selectable {
                " · outside current draft constraints"
            } else {
                ""
            }
        ),
        Captured::Absent => "No captured declaration".into(),
        Captured::Unknown { reason } => format!("Unresolved · {}", short(reason)),
    };
    let default = row
        .pack_default
        .as_ref()
        .map(default_text)
        .unwrap_or_else(|| "Unresolved".into());
    format!(
        "{}\nPending: {pending}\nCaptured: {captured}\nPinned default: {}",
        row.setting_id,
        short(&default)
    )
}
fn default_text(value: &hyprland_editor_core::lua::scalar_codec::NativeValue) -> String {
    use hyprland_editor_core::lua::scalar_codec::NativeValue as N;
    match value {
        N::Bool(v) => v.to_string(),
        N::Int(v) | N::FontWeight(v) => v.to_string(),
        N::FloatBits(v) => f32::from_bits(*v).to_string(),
        N::Bytes(v) => std::str::from_utf8(v)
            .map(|text| Value::String(text.into()).lua())
            .unwrap_or_else(|_| "Non-UTF-8 bytes".into()),
        N::Color(v) => format!("0x{v:08X}"),
        N::Vec2Bits(v) => format!("{}, {}", f32::from_bits(v[0]), f32::from_bits(v[1])),
        N::CssGap(v) => format!("{}, {}, {}, {}", v[0], v[1], v[2], v[3]),
        N::Gradient { colors, angle_bits } => format!(
            "{} colors, {}°",
            colors.len(),
            f64::from(f32::from_bits(*angle_bits)) * 180.0 / std::f64::consts::PI
        ),
    }
}
struct Controls {
    title: gtk::Label,
    dropdown: gtk::DropDown,
    details: gtk::Expander,
    index: Cell<Option<usize>>,
    binding: Cell<bool>,
    options: RefCell<Vec<Choice>>,
}
fn default_choice(row: &Row) -> Choice {
    if row.pending.is_some() {
        Choice::Pending
    } else {
        Choice::LeaveUnset
    }
}
fn choice_list(row: &Row) -> Vec<(Choice, &'static str)> {
    let mut values = vec![(Choice::LeaveUnset, "Leave unset in sample")];
    if row.pending.is_some() {
        values.push((Choice::Pending, "Use pending value"));
    }
    if matches!(
        row.captured,
        Captured::Value {
            selectable: true,
            ..
        }
    ) {
        values.push((Choice::Captured, "Use captured value"));
    }
    values
}
fn list(
    review: Rc<Review>,
    choices: Rc<RefCell<BTreeMap<String, Choice>>>,
    strings: gtk::StringList,
) -> gtk::ListView {
    let factory = gtk::SignalListItemFactory::new();
    let controls: Rc<RefCell<HashMap<usize, Rc<Controls>>>> = Rc::new(RefCell::new(HashMap::new()));
    let (setup_controls, setup_review, setup_choices) =
        (controls.clone(), review.clone(), choices.clone());
    factory.connect_setup(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let title = label("");
        let dropdown = gtk::DropDown::from_strings(&["Leave unset in sample"]);
        let details = gtk::Expander::new(Some("Full values and provenance"));
        let group = gtk::Box::new(gtk::Orientation::Vertical, 8);
        group.set_margin_top(12);
        group.set_margin_bottom(12);
        group.append(&title);
        group.append(&dropdown);
        group.append(&details);
        item.set_child(Some(&group));
        let row = Rc::new(Controls {
            title,
            dropdown,
            details,
            index: Cell::new(None),
            binding: Cell::new(false),
            options: RefCell::new(vec![]),
        });
        let (weak, review, choices) = (
            Rc::downgrade(&row),
            setup_review.clone(),
            setup_choices.clone(),
        );
        row.dropdown.connect_selected_notify(move |dropdown| {
            let Some(row) = weak.upgrade() else {
                return;
            };
            if row.binding.get() {
                return;
            }
            if let (Some(index), Some(choice)) = (
                row.index.get(),
                row.options
                    .borrow()
                    .get(dropdown.selected() as usize)
                    .copied(),
            ) {
                choices
                    .borrow_mut()
                    .insert(review.rows[index].setting_id.clone(), choice);
            };
        });
        let (weak, review) = (Rc::downgrade(&row), setup_review.clone());
        row.details.connect_expanded_notify(move |details| {
            if !details.is_expanded() {
                details.set_child(None::<&gtk::Widget>);
                return;
            }
            let Some(row) = weak.upgrade() else {
                return;
            };
            let Some(index) = row.index.get() else {
                return;
            };
            let text = gtk::TextView::builder()
                .editable(false)
                .monospace(true)
                .wrap_mode(gtk::WrapMode::WordChar)
                .build();
            text.buffer().set_text(
                &serde_json::to_string_pretty(&review.rows[index])
                    .unwrap_or_else(|_| "Value display unavailable".into()),
            );
            details.set_child(Some(&text));
        });
        setup_controls
            .borrow_mut()
            .insert(item.as_ptr() as usize, row);
    });
    let (bind_controls, bind_review, bind_choices) =
        (controls.clone(), review.clone(), choices.clone());
    factory.connect_bind(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let Some(value) = item.item().and_downcast::<gtk::StringObject>() else {
            return;
        };
        let Some(index) = value
            .string()
            .split('\t')
            .next()
            .and_then(|n| n.parse::<usize>().ok())
        else {
            return;
        };
        let controls = bind_controls.borrow();
        let Some(row) = controls.get(&(item.as_ptr() as usize)) else {
            return;
        };
        let Some(data) = bind_review.rows.get(index) else {
            return;
        };
        row.binding.set(true);
        row.index.set(Some(index));
        row.title.set_text(&row_text(data));
        row.details.set_expanded(false);
        row.details.set_child(None::<&gtk::Widget>);
        let options = choice_list(data);
        row.dropdown.set_model(Some(&gtk::StringList::new(
            &options.iter().map(|(_, name)| *name).collect::<Vec<_>>(),
        )));
        let selected = bind_choices
            .borrow()
            .get(&data.setting_id)
            .copied()
            .unwrap_or(default_choice(data));
        row.dropdown.set_selected(
            options
                .iter()
                .position(|(choice, _)| *choice == selected)
                .unwrap_or(0) as u32,
        );
        row.dropdown
            .update_property(&[gtk::accessible::Property::Label(&format!(
                "{} sample value",
                data.setting_id
            ))]);
        *row.options.borrow_mut() = options.into_iter().map(|(choice, _)| choice).collect();
        row.binding.set(false);
    });
    factory.connect_teardown(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        controls.borrow_mut().remove(&(item.as_ptr() as usize));
    });
    gtk::ListView::new(Some(gtk::NoSelection::new(Some(strings))), Some(factory))
}

pub(super) fn open(
    parent: &adw::Window,
    state: &Shared,
    generation: u64,
    workspace: Arc<Workspace>,
    on_keep: impl FnOnce(Selection) + 'static,
    on_close: impl FnOnce() + 'static,
) {
    let window = adw::Window::builder()
        .title("Review setting values")
        .transient_for(parent)
        .destroy_with_parent(true)
        .modal(true)
        .default_width(700)
        .default_height(660)
        .width_request(350)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(16);
    body.set_margin_end(16);
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&body)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    body.append(&label("Choose values for the sample only. Your draft and source files stay unchanged. Leaving a setting unset lets the target choose its default; its effective value is unverified."));
    let status = label("Rebuilding captured values in the restricted worker…");
    body.append(&status);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    footer.set_margin_start(16);
    footer.set_margin_end(16);
    footer.set_margin_bottom(16);
    root.append(&footer);
    let keep = gtk::Button::with_label("Use these inputs");
    keep.add_css_class("suggested-action");
    keep.set_sensitive(false);
    let cancel = gtk::Button::with_label("Cancel");
    footer.append(&keep);
    footer.append(&cancel);
    window.set_content(Some(&root));
    let cancellation = Cancellation::default();
    let stop = cancellation.clone();
    let closed = Rc::new(RefCell::new(Some(on_close)));
    let cleanup = closed.clone();
    window.connect_close_request(move |_| {
        stop.cancel();
        if let Some(close) = cleanup.borrow_mut().take() {
            close();
        }
        gtk::glib::Propagation::Proceed
    });
    let stop = cancellation.clone();
    window.connect_destroy(move |_| {
        stop.cancel();
        if let Some(close) = closed.borrow_mut().take() {
            close();
        }
    });
    let weak = window.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(window) = weak.upgrade() {
            window.close();
        }
    });
    let (weak_window, weak_body, weak_status, weak_keep, weak_state) = (
        window.downgrade(),
        body.downgrade(),
        status.downgrade(),
        keep.downgrade(),
        Rc::downgrade(state),
    );
    super::workflow::background(
        move || {
            let review = projection_worker::review_layers(workspace, cancellation)?;
            let identity = review.fingerprint()?;
            Ok((review, identity))
        },
        move |result| {
            let (Some(window), Some(body), Some(status), Some(keep), Some(state)) = (
                weak_window.upgrade(),
                weak_body.upgrade(),
                weak_status.upgrade(),
                weak_keep.upgrade(),
                weak_state.upgrade(),
            ) else {
                return;
            };
            if !window.is_visible() {
                return;
            }
            if state.borrow().generation != generation {
                status.set_text("Draft changed. Close this review and prepare again.");
                return;
            }
            let (review, identity) = match result {
                Ok(value) => value,
                Err(error) => {
                    status.set_text(&format!("Source review stopped: {error}"));
                    return;
                }
            };
            status.set_text(if review.model_complete {"Captured values were reconstructed. Pinned defaults are metadata, not live observations."}else{"Some source behavior is unresolved. Captured values are partial observations."});
            let review = Rc::new(review);
            let choices: Rc<RefCell<BTreeMap<String, Choice>>> = Rc::new(RefCell::new(
                review
                    .rows
                    .iter()
                    .map(|row| (row.setting_id.clone(), default_choice(row)))
                    .collect(),
            ));
            let names: Vec<String> = review
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| format!("{index}\t{}", row_text(row)))
                .collect();
            let strings =
                gtk::StringList::new(&names.iter().map(String::as_str).collect::<Vec<_>>());
            let search = gtk::SearchEntry::new();
            search.set_placeholder_text(Some("Find settings or values"));
            body.append(&search);
            // Filter the string model, then build controls only for visible rows.
            let filter = gtk::StringFilter::new(Some(gtk::PropertyExpression::new(
                gtk::StringObject::static_type(),
                None::<gtk::Expression>,
                "string",
            )));
            let changed = filter.clone();
            search.connect_search_changed(move |search| changed.set_search(Some(&search.text())));
            let filtered = gtk::FilterListModel::new(Some(strings.clone()), Some(filter));
            let view = list(review.clone(), choices.clone(), strings.clone());
            view.set_model(Some(&gtk::NoSelection::new(Some(filtered))));
            body.append(
                &gtk::ScrolledWindow::builder()
                    .child(&view)
                    .vexpand(true)
                    .min_content_height(180)
                    .hscrollbar_policy(gtk::PolicyType::Never)
                    .build(),
            );
            body.set_vexpand(true);
            let partial = gtk::CheckButton::with_label("Include values from incomplete analysis");
            partial.set_visible(!review.model_complete);
            body.append(&partial);
            let available = gtk::Button::with_label("Use available captured values");
            body.append(&available);
            let (bulk_review, bulk_choices, bulk_strings) =
                (review.clone(), choices.clone(), strings.clone());
            available.connect_clicked(move |_| {
                for row in &bulk_review.rows {
                    if matches!(
                        row.captured,
                        Captured::Value {
                            selectable: true,
                            ..
                        }
                    ) {
                        bulk_choices
                            .borrow_mut()
                            .insert(row.setting_id.clone(), Choice::Captured);
                    }
                }
                bulk_strings.splice(
                    0,
                    bulk_strings.n_items(),
                    &names.iter().map(String::as_str).collect::<Vec<_>>(),
                );
            });
            keep.set_sensitive(true);
            let (weak_window, weak_state) = (window.downgrade(), weak_state.clone());
            let on_keep = RefCell::new(Some(on_keep));
            keep.connect_clicked(move |_| {
                let Some(state) = weak_state.upgrade() else {
                    return;
                };
                if state.borrow().generation != generation {
                    status.set_text("Draft changed. This review can no longer be used.");
                    return;
                }
                if !partial.is_active()
                    && review.rows.iter().any(|row| {
                        matches!(
                            row.captured,
                            Captured::Value {
                                provisional: true,
                                ..
                            }
                        ) && choices.borrow().get(&row.setting_id) == Some(&Choice::Captured)
                    })
                {
                    status.set_text(
                        "Acknowledge incomplete analysis before sampling its captured values.",
                    );
                    return;
                }
                if let Some(keep) = on_keep.borrow_mut().take() {
                    keep(Selection {
                        review_sha256: identity.clone(),
                        acknowledge_partial: partial.is_active(),
                        choices: choices.borrow().clone(),
                    });
                }
                if let Some(window) = weak_window.upgrade() {
                    window.close();
                }
            });
            search.grab_focus();
        },
    );
    window.present();
}
