//! User-supplied analysis hypotheses. These paths never grant filesystem access.
use super::label;
use adw::prelude::*;
use hyprland_editor_core::{module_path::MAX_SEARCH_PATH_BYTES, source::display_path_bytes};
use serde::Serialize;
use std::{cell::RefCell, os::unix::ffi::OsStrExt, rc::Rc};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub(super) struct Options {
    pub(super) home: Option<Vec<u8>>,
    pub(super) working_directory: Option<Vec<u8>>,
    pub(super) inherited_package_path: Option<Vec<u8>>,
}
impl Options {
    pub(super) fn validate(&self) -> Result<(), String> {
        for path in [&self.home, &self.working_directory].into_iter().flatten() {
            if !path.starts_with(b"/") || path.len() > 4096 || path.contains(&0) {
                return Err(
                    "Choose an absolute context folder within the 4096-byte path limit.".into(),
                );
            }
        }
        if self
            .inherited_package_path
            .as_ref()
            .is_some_and(|p| p.len() > MAX_SEARCH_PATH_BYTES || p.contains(&0))
        {
            return Err(
                "The inherited search path must contain at most 64 KiB and no NUL bytes.".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone)]
struct FolderField {
    value: Rc<RefCell<Option<Vec<u8>>>>,
    display: gtk::glib::WeakRef<gtk::Label>,
}
impl FolderField {
    fn set(&self, value: Option<Vec<u8>>) {
        if let Some(label) = self.display.upgrade() {
            label.set_text(
                &value
                    .as_deref()
                    .map(display_path_bytes)
                    .unwrap_or_else(|| "Unknown".into()),
            );
        }
        *self.value.borrow_mut() = value;
    }
}

fn folder_field(
    parent: &adw::Window,
    body: &gtk::Box,
    title: &str,
    value: Option<Vec<u8>>,
) -> FolderField {
    body.append(&label(title));
    let display = label("");
    display.set_selectable(true);
    body.append(&display);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let choose = gtk::Button::with_label("Choose Folder…");
    choose.set_hexpand(true);
    let unknown = gtk::Button::with_label("Use Unknown");
    row.append(&choose);
    row.append(&unknown);
    body.append(&row);
    let field = FolderField {
        value: Rc::new(RefCell::new(None)),
        display: display.downgrade(),
    };
    field.set(value);
    let f = field.clone();
    unknown.connect_clicked(move |_| f.set(None));
    let p = parent.downgrade();
    let f = field.clone();
    let title = format!("Choose {title} for analysis only");
    choose.connect_clicked(move |_| {
        let Some(parent) = p.upgrade() else {
            return;
        };
        let p = p.clone();
        let f = f.clone();
        gtk::FileDialog::builder()
            .title(&title)
            .build()
            .select_folder(
                Some(&parent),
                None::<&gtk::gio::Cancellable>,
                move |result| {
                    if !p.upgrade().is_some_and(|w| w.is_visible()) {
                        return;
                    }
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            let bytes = path.as_os_str().as_bytes();
                            if bytes.starts_with(b"/") && bytes.len() <= 4096 && !bytes.contains(&0)
                            {
                                f.set(Some(bytes.to_vec()));
                            }
                        }
                    }
                },
            );
    });
    field
}

// A GUI folder action adds both conventional templates without rewriting any
// existing separators, empty templates or relative spelling.
fn append_folder(path: &[u8], current: &[u8]) -> Result<Vec<u8>, String> {
    if !path.starts_with(b"/") || path.len() > 4096 || path.contains(&0) || path.contains(&b';') {
        return Err("This folder cannot be represented as one Lua search template.".into());
    }
    if path.contains(&b'?') {
        return Err("A question mark in this folder would become a module substitution. Use exact templates to review it.".into());
    }
    let separator = if path.ends_with(b"/") {
        b"".as_slice()
    } else {
        b"/".as_slice()
    };
    let mut result = current.to_vec();
    if !result.is_empty() {
        result.push(b';');
    }
    result.extend_from_slice(path);
    result.extend_from_slice(separator);
    result.extend_from_slice(b"?.lua;");
    result.extend_from_slice(path);
    result.extend_from_slice(separator);
    result.extend_from_slice(b"?/init.lua");
    if result.len() > MAX_SEARCH_PATH_BYTES {
        return Err("The search path would exceed 64 KiB.".into());
    }
    Ok(result)
}

// Lossless presentation for non-UTF-8/control bytes, including filenames picked
// through GIO. Backslashes are doubled so byte escapes cannot change meaning.
fn encode(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &byte in bytes {
        match byte {
            b'\\' => result.push_str("\\\\"),
            0x20..=0x7e => result.push(char::from(byte)),
            _ => {
                use std::fmt::Write;
                write!(result, "\\x{byte:02x}").unwrap();
            }
        }
    }
    result
}
fn decode(text: &str) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            match bytes.get(index + 1) {
                Some(b'\\') => {
                    result.push(b'\\');
                    index += 2;
                }
                Some(b'x')
                    if bytes
                        .get(index + 2..index + 4)
                        .is_some_and(|p| p.iter().all(u8::is_ascii_hexdigit)) =>
                {
                    let hex = std::str::from_utf8(&bytes[index + 2..index + 4]).unwrap();
                    result.push(u8::from_str_radix(hex, 16).unwrap());
                    index += 4;
                }
                _ => {
                    return Err(
                        "Use \\\\ for a literal backslash or \\xNN for a byte escape.".into(),
                    )
                }
            }
        } else {
            result.push(bytes[index]);
            index += 1;
        }
        if result.len() > MAX_SEARCH_PATH_BYTES || result.last() == Some(&0) {
            return Err(
                "The inherited search path must contain at most 64 KiB and no NUL bytes.".into(),
            );
        }
    }
    Ok(result)
}

#[derive(Clone)]
pub(super) struct Editor {
    home: FolderField,
    cwd: FolderField,
    known: gtk::glib::WeakRef<gtk::CheckButton>,
    templates: gtk::glib::WeakRef<gtk::TextView>,
    #[cfg(test)]
    pub(super) save: gtk::glib::WeakRef<gtk::Button>,
}
impl Editor {
    fn options(&self) -> Result<Options, String> {
        let known = self.known.upgrade().ok_or("Context editor closed")?;
        let view = self.templates.upgrade().ok_or("Context editor closed")?;
        let buffer = view.buffer();
        let result = Options {
            home: self.home.value.borrow().clone(),
            working_directory: self.cwd.value.borrow().clone(),
            inherited_package_path: if known.is_active() {
                Some(decode(&buffer.text(
                    &buffer.start_iter(),
                    &buffer.end_iter(),
                    true,
                ))?)
            } else {
                None
            },
        };
        result.validate()?;
        Ok(result)
    }
    #[cfg(test)]
    pub(super) fn test_set(&self, options: Options) {
        self.home.set(options.home);
        self.cwd.set(options.working_directory);
        self.known
            .upgrade()
            .unwrap()
            .set_active(options.inherited_package_path.is_some());
        self.templates.upgrade().unwrap().buffer().set_text(&encode(
            options
                .inherited_package_path
                .as_deref()
                .unwrap_or_default(),
        ));
    }
}

pub(super) fn show(
    parent: &adw::Window,
    options: Options,
    commit: impl Fn(Options) -> Result<(), String> + 'static,
) -> (adw::Window, Editor) {
    let win = adw::Window::builder()
        .title("Analysis context")
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .default_width(600)
        .default_height(620)
        .build();
    win.set_size_request(350, 420);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    let save = gtk::Button::with_label("Use Context");
    save.add_css_class("suggested-action");
    header.pack_end(&save);
    root.append(&header);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for set in [
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
    ] {
        set(body.upcast_ref(), 16);
    }
    body.append(&label("Supply the context to model for this selected snapshot. These assumptions are not detected from Hyprland. Choosing a folder here does not allow reading it; allow dependency folders separately."));
    let home = folder_field(&win, &body, "Home folder", options.home);
    let cwd = folder_field(&win, &body, "Working directory", options.working_directory);
    let known = gtk::CheckButton::with_label("Inherited Lua search path is known");
    known.set_active(options.inherited_package_path.is_some());
    body.append(&known);
    let search = gtk::Box::new(gtk::Orientation::Vertical, 8);
    search.append(&label("Add search folders, or edit exact templates below. Separate templates with ; and use ? for the module name. An empty field means no inherited templates. Hyprland adds the main file’s directory first. Byte escapes use \\xNN; a literal backslash uses \\\\."));
    let add = gtk::Button::with_label("Add Search Folder…");
    search.append(&add);
    let view = gtk::TextView::builder()
        .editable(true)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(8)
        .right_margin(8)
        .top_margin(8)
        .bottom_margin(8)
        .build();
    view.set_widget_name("dependency-context-search-path");
    view.update_property(&[gtk::accessible::Property::Label(
        "Exact inherited Lua search templates",
    )]);
    let feedback = label("");
    let f = feedback.downgrade();
    view.buffer().connect_insert_text(move |buffer, _, text| {
        let current = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        if current.len().saturating_add(text.len()) > 4 * MAX_SEARCH_PATH_BYTES || text.contains('\0') {
            buffer.stop_signal_emission_by_name("insert-text");
            if let Some(f) = f.upgrade() { f.set_text("The encoded field must fit within 256 KiB and contain no NUL bytes. Nothing was inserted."); }
        }
    });
    view.buffer().set_text(&encode(
        options
            .inherited_package_path
            .as_deref()
            .unwrap_or_default(),
    ));
    search.append(
        &gtk::ScrolledWindow::builder()
            .child(&view)
            .min_content_height(100)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    search.set_visible(known.is_active());
    let s = search.downgrade();
    known.connect_toggled(move |known| {
        if let Some(s) = s.upgrade() {
            s.set_visible(known.is_active());
        }
    });
    body.append(&search);
    body.append(&feedback);
    let p = win.downgrade();
    let v = view.downgrade();
    let f = feedback.downgrade();
    add.connect_clicked(move |_| {
        let Some(parent) = p.upgrade() else {
            return;
        };
        let p = p.clone();
        let v = v.clone();
        let f = f.clone();
        gtk::FileDialog::builder()
            .title("Add inherited Lua search folder")
            .build()
            .select_folder(
                Some(&parent),
                None::<&gtk::gio::Cancellable>,
                move |result| {
                    if !p.upgrade().is_some_and(|w| w.is_visible()) {
                        return;
                    }
                    let (Some(view), Some(feedback)) = (v.upgrade(), f.upgrade()) else {
                        return;
                    };
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            let buffer = view.buffer();
                            let current =
                                buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
                            match decode(&current).and_then(|current| {
                                append_folder(path.as_os_str().as_bytes(), &current)
                            }) {
                                Ok(value) => {
                                    buffer.set_text(&encode(&value));
                                    feedback.set_text("");
                                }
                                Err(error) => feedback.set_text(&error),
                            }
                        }
                    }
                },
            );
    });
    root.append(
        &gtk::ScrolledWindow::builder()
            .child(&body)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    );
    win.set_content(Some(&root));
    let editor = Editor {
        home,
        cwd,
        known: known.downgrade(),
        templates: view.downgrade(),
        #[cfg(test)]
        save: save.downgrade(),
    };
    let e = editor.clone();
    let f = feedback.downgrade();
    let w = win.downgrade();
    save.connect_clicked(move |_| match e.options().and_then(&commit) {
        Ok(()) => {
            if let Some(w) = w.upgrade() {
                w.close();
            }
        }
        Err(error) => {
            if let Some(f) = f.upgrade() {
                f.set_text(&error);
            }
        }
    });
    win.present();
    (win, editor)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_preserves_unknown_empty_and_byte_paths_and_builder_preserves_templates() {
        let mut options = Options::default();
        options.validate().unwrap();
        assert_ne!(options.inherited_package_path, Some(vec![]));
        options.home = Some(b"/home/byte\xff".to_vec());
        options.working_directory = Some(b"/fixture/../working/".to_vec());
        options.inherited_package_path = Some(vec![]);
        options.validate().unwrap();
        assert_eq!(
            append_folder(b"/library/", b";relative/?;;").unwrap(),
            b";relative/?;;;/library/?.lua;/library/?/init.lua"
        );
        assert_eq!(append_folder(b"/", b"").unwrap(), b"/?.lua;/?/init.lua");
        let all = (1..=255).collect::<Vec<u8>>();
        assert_eq!(decode(&encode(&all)).unwrap(), all);
        assert_eq!(decode("/home/é/?.lua").unwrap(), "/home/é/?.lua".as_bytes());
        assert!(decode("\\q").is_err());
        assert!(decode("\\x0").is_err());
        assert!(decode("\\x00").is_err());
        assert!(decode(&"x".repeat(MAX_SEARCH_PATH_BYTES + 1)).is_err());
        for folder in [
            b"relative".as_slice(),
            b"/bad;folder",
            b"/bad?folder",
            b"/nul\0",
        ] {
            assert!(append_folder(folder, b"").is_err());
        }
        options.inherited_package_path = Some(vec![b'x'; MAX_SEARCH_PATH_BYTES + 1]);
        assert!(options.validate().is_err());
        options.inherited_package_path = Some(b"?;\0".to_vec());
        assert!(options.validate().is_err());
        options.inherited_package_path = None;
        options.working_directory = Some(b"relative".to_vec());
        assert!(options.validate().is_err());
    }
}
