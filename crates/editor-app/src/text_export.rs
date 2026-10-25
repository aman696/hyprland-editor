//! Review an immutable Lua snapshot and save a separate, explicitly chosen copy.
use adw::prelude::*;
use gtk::{gio, glib};
use std::rc::Rc;

const MAX_EXPORT_BYTES: usize = 4 * hyprland_editor_core::MAX_DOCUMENT_BYTES;

async fn save_new(file: &gio::File, bytes: Vec<u8>) -> Result<(), String> {
    if !file.is_native() {
        return Err("Choose a local folder for this Lua copy.".into());
    }
    if bytes.len() > MAX_EXPORT_BYTES {
        return Err("This Lua copy exceeds the 4 MiB export limit.".into());
    }
    // GIO create refuses existing files, including symlinks. An export cannot
    // overwrite the original configuration or an earlier reviewed copy.
    let stream = file
        .create_future(gio::FileCreateFlags::PRIVATE, glib::Priority::DEFAULT)
        .await
        .map_err(|error| {
            if error.matches(gio::IOErrorEnum::Exists) {
                "That file already exists. Choose a new filename.".into()
            } else {
                format!("Could not create the Lua copy: {}", error.message())
            }
        })?;
    let expected = bytes.len();
    let written = stream
        .write_all_future(bytes, glib::Priority::DEFAULT)
        .await;
    // Close on both success and failure; never report success before close.
    let closed = stream.close_future(glib::Priority::DEFAULT).await;
    match written {
        Ok((_, count, None)) if count == expected && closed.is_ok() => Ok(()),
        _ => Err("Could not finish saving. A partial copy may remain at the chosen destination. Choose another filename to retry.".into()),
    }
}

pub(super) fn review(
    parent: &adw::ApplicationWindow,
    title: &str,
    description: &str,
    filename: &str,
    text: String,
) -> adw::Window {
    let dialog = adw::Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(720)
        .default_height(560)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.append(&super::label(description));
    let view = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    view.buffer().set_text(&text);
    content.append(
        &gtk::ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .build(),
    );
    let status = gtk::Label::builder().wrap(true).xalign(0.0).build();
    content.append(&status);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let copy = gtk::Button::with_label("Copy Lua");
    let save = gtk::Button::with_label("Save Lua File…");
    save.add_css_class("suggested-action");
    actions.append(&copy);
    actions.append(&save);
    content.append(&actions);
    let text: Rc<str> = text.into();
    let snapshot = text.clone();
    let window = dialog.downgrade();
    let copied = status.clone();
    copy.connect_clicked(move |_| {
        if let Some(window) = window.upgrade() {
            window.clipboard().set_text(&snapshot);
            copied.set_text("Reviewed Lua copied to the clipboard.");
        }
    });
    let window = dialog.downgrade();
    let filename = filename.to_owned();
    save.connect_clicked(move |button| {
        let Some(parent) = window.upgrade() else {
            return;
        };
        let chooser = gtk::FileDialog::builder()
            .title("Save Lua Copy")
            .initial_name(&filename)
            .accept_label("Save Copy")
            .build();
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Lua configuration (*.lua)"));
        filter.add_pattern("*.lua");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        chooser.set_filters(Some(&filters));
        chooser.set_default_filter(Some(&filter));
        button.set_sensitive(false);
        let button = button.downgrade();
        let status = status.downgrade();
        let text = text.clone();
        glib::MainContext::default().spawn_local(async move {
            let selected = chooser.save_future(Some(&parent)).await;
            if let Some(button) = button.upgrade() {
                button.set_sensitive(true);
            }
            if !parent.is_visible() {
                return;
            }
            let file = match selected {
                Ok(file) => file,
                Err(error)
                    if error.matches(gtk::DialogError::Dismissed)
                        || error.matches(gtk::DialogError::Cancelled) =>
                {
                    return
                }
                Err(error) => {
                    if let Some(status) = status.upgrade() {
                        status.set_text(&format!(
                            "Could not choose an export file: {}",
                            error.message()
                        ));
                    }
                    return;
                }
            };
            if let Some(button) = button.upgrade() {
                button.set_sensitive(false);
            }
            if let Some(status) = status.upgrade() {
                status.set_text("Saving the reviewed Lua copy…");
            }
            let result = save_new(&file, text.as_bytes().to_vec()).await;
            if let Some(button) = button.upgrade() {
                button.set_sensitive(true);
            }
            if let Some(status) = status.upgrade() {
                match result {
                    Ok(()) => status.set_text(&format!("Lua copy saved to {}", file.parse_name())),
                    Err(error) => status.set_text(&error),
                }
            }
        });
    });
    dialog.set_content(Some(&content));
    dialog.present();
    dialog
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
    };
    #[test]
    fn exported_snapshot_preserves_bytes_and_never_overwrites_files_or_symlink_targets() {
        let root = super::super::test_support::TempDir::new("lua-export");
        let path = root.path().join("reviewed.lua");
        let snapshot = "-- café\r\nhl.config{general={border_size=3}}\r\n"
            .as_bytes()
            .to_vec();
        let context = glib::MainContext::new();
        context
            .with_thread_default(|| {
                context
                    .block_on(save_new(&gio::File::for_path(&path), snapshot.clone()))
                    .unwrap();
                assert_eq!(fs::read(&path).unwrap(), snapshot);
                assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
                assert!(context
                    .block_on(save_new(
                        &gio::File::for_path(&path),
                        b"replacement".to_vec()
                    ))
                    .is_err());
                assert_eq!(fs::read(&path).unwrap(), snapshot);
                let link = root.path().join("linked.lua");
                symlink(&path, &link).unwrap();
                assert!(context
                    .block_on(save_new(
                        &gio::File::for_path(&link),
                        b"replacement".to_vec()
                    ))
                    .is_err());
                assert_eq!(fs::read(&path).unwrap(), snapshot);
                assert_eq!(fs::read_link(&link).unwrap(), path);
                let oversized = root.path().join("oversized.lua");
                assert!(context
                    .block_on(save_new(
                        &gio::File::for_path(&oversized),
                        vec![0; MAX_EXPORT_BYTES + 1]
                    ))
                    .is_err());
                assert!(!oversized.exists());
            })
            .unwrap();
    }
}
