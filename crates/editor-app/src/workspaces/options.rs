//! Focused option editor; only one value form is resident at a time.
use super::*;
use std::collections::BTreeMap;
type Options = BTreeMap<String, Value>;
pub(super) fn form(
    panel: &gtk::Box,
    initial: &Options,
    legacy: bool,
) -> Rc<dyn Fn() -> Result<Options, String>> {
    let values = Rc::new(RefCell::new(initial.clone()));
    let model = gtk::StringList::new(&[]);
    let pick = gtk::DropDown::builder().model(&model).build();
    pick.set_widget_name("workspace-option");
    compact(&pick);
    panel.append(&pick);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add Option");
    let remove = gtk::Button::with_label("Remove Option");
    row.append(&add);
    row.append(&remove);
    panel.append(&row);
    panel.append(&label("Option Name"));
    let key = gtk::Entry::builder().max_length(256).build();
    key.set_widget_name("workspace-option-name");
    panel.append(&key);
    let kind = choices(
        panel,
        if legacy {
            &["Text"]
        } else {
            &["Text", "Boolean", "Whole Number", "Number"]
        },
        0,
    );
    kind.set_widget_name("workspace-option-type");
    kind.set_visible(!legacy);
    panel.append(&label("Option Value"));
    let value = gtk::Entry::builder().max_length(16384).build();
    value.set_widget_name("workspace-option-value");
    panel.append(&value);
    let boolean = choices(panel, &["False", "True"], 0);
    boolean.set_widget_name("workspace-option-boolean");
    let presets_model = gtk::StringList::new(&[]);
    let presets = gtk::DropDown::builder().model(&presets_model).build();
    presets.set_widget_name("workspace-option-presets");
    compact(&presets);
    panel.append(&presets);
    let presets_busy = Rc::new(Cell::new(false));
    let refresh_presets: Rc<dyn Fn()> = Rc::new({
        let key = key.clone();
        let value = value.clone();
        let kind = kind.clone();
        let model = presets_model.clone();
        let picker = presets.clone();
        let busy = presets_busy.clone();
        move || {
            busy.set(true);
            let names: &[&str] = match key.text().as_str() {
                "orientation" => &["left", "top", "right", "bottom", "center"],
                "direction" => &["right", "left", "up", "down"],
                _ => &[],
            };
            let mut labels = vec!["Custom Text"];
            labels.extend(names);
            model.splice(0, model.n_items(), &labels);
            picker.set_selected(
                names
                    .iter()
                    .position(|n| *n == value.text())
                    .map_or(0, |i| i as u32 + 1),
            );
            picker.set_visible(!names.is_empty() && kind.selected() == 0);
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&refresh_presets);
    key.connect_changed(move |_| {
        if let Some(refresh) = weak.upgrade() {
            refresh();
        }
    });
    let weak = Rc::downgrade(&refresh_presets);
    value.connect_changed(move |_| {
        if let Some(refresh) = weak.upgrade() {
            refresh();
        }
    });
    let value_c = value.clone();
    let busy_c = presets_busy.clone();
    presets.connect_selected_notify(move |p| {
        if busy_c.get() || p.selected() == 0 {
            return;
        }
        if let Some(item) = p
            .selected_item()
            .and_then(|o| o.downcast::<gtk::StringObject>().ok())
        {
            value_c.set_text(&item.string());
        }
    });
    for (widget, name) in [
        (pick.upcast_ref::<gtk::Widget>(), "Layout option"),
        (key.upcast_ref(), "Layout option name"),
        (kind.upcast_ref(), "Layout option value type"),
        (value.upcast_ref(), "Layout option value"),
        (boolean.upcast_ref(), "Layout option boolean value"),
        (presets.upcast_ref(), "Common layout option values"),
    ] {
        widget.update_property(&[gtk::accessible::Property::Label(name)]);
    }
    let feedback = label("");
    panel.append(&feedback);
    let active = Rc::new(RefCell::new(None::<String>));
    let busy = Rc::new(Cell::new(false));
    let flush: Getter = Rc::new({
        let values = values.clone();
        let active = active.clone();
        let key = key.clone();
        let kind = kind.clone();
        let value = value.clone();
        let boolean = boolean.clone();
        move || {
            let Some(old) = active.borrow().clone() else {
                return Ok(());
            };
            let name = key.text().to_string();
            if name.is_empty()
                || name.len() > 256
                || name.chars().any(char::is_control)
                || (legacy && name.contains(':'))
            {
                return Err("Enter a valid option name".into());
            }
            if name != old && values.borrow().contains_key(&name) {
                return Err("This option name already exists".into());
            }
            let parsed = match kind.selected() {
                0 => Value::String(value.text().into()),
                1 => Value::Bool(boolean.selected() == 1),
                2 => Value::Int(value.text().parse().map_err(|_| "Enter a whole number")?),
                _ => {
                    let n: f64 = value.text().parse().map_err(|_| "Enter a finite number")?;
                    if !n.is_finite() {
                        return Err("Enter a finite number".into());
                    }
                    Value::Float(n)
                }
            };
            if matches!(&parsed, Value::String(s) if s.len()>16384 || s.contains('\0')) {
                return Err("Option text exceeds its limit".into());
            }
            let mut options = values.borrow_mut();
            options.remove(&old);
            options.insert(name.clone(), parsed);
            *active.borrow_mut() = Some(name);
            Ok(())
        }
    });
    let render: Rc<dyn Fn(Option<String>)> = Rc::new({
        let values = values.clone();
        let model = model.clone();
        let pick = pick.clone();
        let active = active.clone();
        let busy = busy.clone();
        let key = key.clone();
        let kind = kind.clone();
        let value = value.clone();
        let boolean = boolean.clone();
        let remove = remove.clone();
        move |wanted| {
            busy.set(true);
            let options = values.borrow();
            let keys: Vec<_> = options.keys().map(String::as_str).collect();
            model.splice(0, model.n_items(), &keys);
            let index = wanted
                .as_ref()
                .and_then(|k| keys.iter().position(|v| *v == k))
                .or_else(|| (!keys.is_empty()).then_some(0));
            *active.borrow_mut() = index.map(|i| keys[i].to_string());
            pick.set_selected(index.map_or(gtk::INVALID_LIST_POSITION, |i| i as u32));
            for w in [
                key.upcast_ref::<gtk::Widget>(),
                kind.upcast_ref(),
                value.upcast_ref(),
                boolean.upcast_ref(),
                remove.upcast_ref(),
            ] {
                w.set_sensitive(index.is_some());
            }
            if let Some(i) = index {
                key.set_text(keys[i]);
                let v = &options[keys[i]];
                kind.set_selected(match v {
                    Value::Bool(_) => 1,
                    Value::Int(_) => 2,
                    Value::Float(_) => 3,
                    _ => 0,
                });
                value.set_text(&match v {
                    Value::String(s) => s.clone(),
                    _ => v.lua(),
                });
                boolean.set_selected(u32::from(*v == Value::Bool(true)));
            } else {
                key.set_text("");
                value.set_text("");
                kind.set_selected(0);
            }
            boolean.set_visible(kind.selected() == 1);
            value.set_visible(kind.selected() != 1);
            busy.set(false);
        }
    });
    let weak = Rc::downgrade(&render);
    let flush_c = flush.clone();
    let busy_c = busy.clone();
    let active_c = active.clone();
    let feedback_c = feedback.clone();
    let model_c = model.clone();
    pick.connect_selected_notify(move |p| {
        if busy_c.get() {
            return;
        }
        let wanted = p
            .selected_item()
            .and_then(|o| o.downcast::<gtk::StringObject>().ok())
            .map(|s| s.string().to_string());
        if let Err(e) = flush_c() {
            feedback_c.set_text(&e);
            busy_c.set(true);
            let old = active_c.borrow();
            let at =
                (0..model_c.n_items()).find(|i| model_c.string(*i).as_deref() == old.as_deref());
            p.set_selected(at.unwrap_or(gtk::INVALID_LIST_POSITION));
            busy_c.set(false);
            return;
        }
        if let Some(render) = weak.upgrade() {
            render(wanted);
        }
    });
    let value_c = value.clone();
    let boolean_c = boolean.clone();
    let weak = Rc::downgrade(&refresh_presets);
    kind.connect_selected_notify(move |k| {
        value_c.set_visible(k.selected() != 1);
        boolean_c.set_visible(k.selected() == 1);
        if let Some(refresh) = weak.upgrade() {
            refresh();
        }
    });
    let values_c = values.clone();
    let flush_c = flush.clone();
    let render_c = render.clone();
    let feedback_c = feedback.clone();
    add.connect_clicked(move |_| {
        if let Err(e) = flush_c() {
            feedback_c.set_text(&e);
            return;
        }
        let mut values = values_c.borrow_mut();
        if values.len() >= 128 {
            feedback_c.set_text("At most 128 options per workspace");
            return;
        }
        let name = ["orientation", "direction"]
            .into_iter()
            .find(|k| !values.contains_key(*k))
            .map(str::to_string)
            .unwrap_or_else(|| {
                (1..)
                    .map(|n| format!("option-{n}"))
                    .find(|k| !values.contains_key(k))
                    .unwrap()
            });
        values.insert(name.clone(), Value::String(String::new()));
        drop(values);
        render_c(Some(name));
    });
    let values_c = values.clone();
    let active_c = active.clone();
    let weak = Rc::downgrade(&render);
    remove.connect_clicked(move |_| {
        if let Some(key) = active_c.borrow().as_ref() {
            values_c.borrow_mut().remove(key);
        }
        if let Some(render) = weak.upgrade() {
            render(None);
        }
    });
    render(None);
    Rc::new(move || {
        let _ = (&render, &refresh_presets);
        flush()?;
        Ok(values.borrow().clone())
    })
}
