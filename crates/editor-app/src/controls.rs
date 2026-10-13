use super::label;
use adw::prelude::*;
use hyprland_editor_core::{Setting, Value};
use std::{cell::RefCell, rc::Rc};
fn accessible_name(widget: &impl IsA<gtk::Accessible>, text: &str) {
    widget.update_property(&[gtk::accessible::Property::Label(text)]);
}
fn spin(value: f64, min: f64, max: f64, digits: u32) -> gtk::SpinButton {
    let w = gtk::SpinButton::with_range(min, max, if digits == 0 { 1. } else { 0.1 });
    w.set_digits(digits);
    w.set_value(value);
    w.set_width_chars(9);
    w.connect_output(|w| {
        w.set_text(&w.value().to_string());
        gtk::glib::Propagation::Stop
    });
    w
}
fn color_button(color: u32) -> gtk::ColorDialogButton {
    let dialog = gtk::ColorDialog::builder().with_alpha(true).build();
    let b = gtk::ColorDialogButton::new(Some(dialog));
    b.set_rgba(&gtk::gdk::RGBA::new(
        ((color >> 16) & 255) as f32 / 255.,
        ((color >> 8) & 255) as f32 / 255.,
        (color & 255) as f32 / 255.,
        (color >> 24) as f32 / 255.,
    ));
    b
}
fn color_value(b: &gtk::ColorDialogButton) -> u32 {
    let c = b.rgba();
    let byte = |x: f32| (x.clamp(0., 1.) * 255.).round() as u32;
    (byte(c.alpha()) << 24) | (byte(c.red()) << 16) | (byte(c.green()) << 8) | byte(c.blue())
}
pub(super) fn form(
    setting: &Setting,
    value: Option<Value>,
    box_: &gtk::Box,
) -> Box<dyn Fn() -> Result<Value, String>> {
    let value = value.or_else(|| setting.known_default());
    let name = super::human_name(&setting.id);
    match setting.kind.as_str() {
        "Bool" => {
            let w = gtk::Switch::builder()
                .halign(gtk::Align::Start)
                .active(matches!(value, Some(Value::Bool(true))))
                .build();
            accessible_name(&w, &name);
            box_.append(&w);
            Box::new(move || Ok(Value::Bool(w.is_active())))
        }
        "Int" => {
            let v = if let Some(Value::Int(v)) = value {
                v as f64
            } else {
                0.
            };
            let w = spin(
                v,
                setting.bounds.min.unwrap_or(-2_147_483_648.),
                setting.bounds.max.unwrap_or(2_147_483_647.),
                0,
            );
            accessible_name(&w, &name);
            box_.append(&w);
            Box::new(move || {
                w.update();
                Ok(Value::Int(w.value() as i64))
            })
        }
        "Float" => {
            let v = if let Some(Value::Float(v)) = value {
                v
            } else {
                0.
            };
            let w = spin(
                v,
                setting.bounds.min.unwrap_or(-1_000_000.),
                setting.bounds.max.unwrap_or(1_000_000.),
                6,
            );
            accessible_name(&w, &name);
            box_.append(&w);
            Box::new(move || {
                w.update();
                Ok(Value::Float(w.value()))
            })
        }
        "String" => {
            let v = if let Some(Value::String(v)) = value {
                v
            } else {
                String::new()
            };
            let w = gtk::Entry::builder().text(v).max_length(4096).build();
            accessible_name(&w, &name);
            box_.append(&w);
            Box::new(move || Ok(Value::String(w.text().into())))
        }
        "Color" => {
            let v = if let Some(Value::Color(v)) = value {
                v
            } else {
                0xff000000
            };
            let w = color_button(v);
            accessible_name(&w, &name);
            box_.append(&w);
            Box::new(move || Ok(Value::Color(color_value(&w))))
        }
        "FontWeight" => {
            let v = if let Some(Value::FontWeight(v)) = value {
                v
            } else {
                400
            };
            let w = spin(v.into(), 0., 65535., 0);
            accessible_name(&w, &name);
            box_.append(&w);
            Box::new(move || {
                w.update();
                Ok(Value::FontWeight(w.value() as u16))
            })
        }
        "Vec2" => {
            let v = if let Some(Value::Vec2(v)) = value {
                v
            } else {
                [0.; 2]
            };
            let grid = gtk::Grid::builder()
                .column_spacing(16)
                .row_spacing(12)
                .build();
            box_.append(&grid);
            let mut fields = vec![];
            for (i, n) in v.into_iter().enumerate() {
                let caption = label(if i == 0 {
                    "X coordinate"
                } else {
                    "Y coordinate"
                });
                grid.attach(&caption, 0, i as i32, 1, 1);
                let w = spin(n, -1_000_000., 1_000_000., 6);
                caption.set_mnemonic_widget(Some(&w));
                accessible_name(&w, &format!("{name}: {}", caption.text()));
                w.set_hexpand(true);
                grid.attach(&w, 1, i as i32, 1, 1);
                fields.push(w);
            }
            Box::new(move || {
                for f in &fields {
                    f.update();
                }
                Ok(Value::Vec2([fields[0].value(), fields[1].value()]))
            })
        }
        "CssGap" => {
            let v = if let Some(Value::CssGap(v)) = value {
                v
            } else {
                [0; 4]
            };
            let grid = gtk::Grid::builder()
                .column_spacing(16)
                .row_spacing(12)
                .build();
            box_.append(&grid);
            let mut fields = vec![];
            for (i, (name, n)) in ["Top", "Right", "Bottom", "Left"]
                .into_iter()
                .zip(v)
                .enumerate()
            {
                let caption = label(name);
                grid.attach(&caption, 0, i as i32, 1, 1);
                let w = spin(n as f64, 0., 100_000., 0);
                caption.set_mnemonic_widget(Some(&w));
                accessible_name(&w, &format!("{name}: {}", caption.text()));
                w.set_hexpand(true);
                grid.attach(&w, 1, i as i32, 1, 1);
                fields.push(w);
            }
            Box::new(move || {
                for f in &fields {
                    f.update();
                }
                Ok(Value::CssGap(std::array::from_fn(|i| {
                    fields[i].value() as i64
                })))
            })
        }
        "Gradient" => {
            let (colors, angle) = if let Some(Value::Gradient { colors, angle }) = value {
                (colors, angle)
            } else {
                (vec![0xff000000], 0.)
            };
            let stops = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            stops.set_halign(gtk::Align::Start);
            let fields = Rc::new(RefCell::new(Vec::new()));
            for (i, color) in colors.into_iter().enumerate() {
                let w = color_button(color);
                accessible_name(&w, &format!("{name}: Color {}", i + 1));
                stops.append(&w);
                fields.borrow_mut().push(w);
            }
            box_.append(&label("Colors (left to right)"));
            box_.append(
                &gtk::ScrolledWindow::builder()
                    .child(&stops)
                    .hscrollbar_policy(gtk::PolicyType::Automatic)
                    .vscrollbar_policy(gtk::PolicyType::Never)
                    .propagate_natural_height(true)
                    .build(),
            );
            let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let add = gtk::Button::with_label("Add Color");
            let remove = gtk::Button::with_label("Remove Last");
            actions.append(&add);
            actions.append(&remove);
            box_.append(&actions);
            let f = fields.clone();
            let b = stops.clone();
            let gradient_name = name.clone();
            add.connect_clicked(move |_| {
                if f.borrow().len() < 32 {
                    let w = color_button(0xffffffff);
                    accessible_name(
                        &w,
                        &format!("{gradient_name}: Color {}", f.borrow().len() + 1),
                    );
                    b.append(&w);
                    f.borrow_mut().push(w);
                }
            });
            let f = fields.clone();
            remove.connect_clicked(move |_| {
                if f.borrow().len() > 1 {
                    let w = f.borrow_mut().pop().unwrap();
                    stops.remove(&w);
                }
            });
            box_.append(&label("Angle in degrees"));
            let angle = spin(angle, -36000., 36000., 2);
            accessible_name(&angle, &format!("{name}: Angle in degrees"));
            box_.append(&angle);
            Box::new(move || {
                angle.update();
                Ok(Value::Gradient {
                    colors: fields.borrow().iter().map(color_value).collect(),
                    angle: angle.value(),
                })
            })
        }
        _ => Box::new(|| Err("No editor for this type".into())),
    }
}
