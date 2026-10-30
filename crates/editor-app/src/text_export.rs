//! Review an immutable configuration snapshot and save a separate chosen copy.
use adw::prelude::*;
use gtk::{gio, glib};
use std::rc::Rc;

const MAX_EXPORT_BYTES: usize = 4 * hyprland_editor_core::MAX_DOCUMENT_BYTES;

async fn save_new(file: &gio::File, bytes: Vec<u8>) -> Result<(), String> {
    save_new_bounded(file, bytes, MAX_EXPORT_BYTES).await
}

pub(super) async fn save_project_new(file: &gio::File, bytes: Vec<u8>) -> Result<(), String> {
    if file
        .path()
        .is_none_or(|path| path.extension().is_none_or(|ext| ext != "hye"))
    {
        return Err("Choose a new local filename ending in .hye for this project.".into());
    }
    save_new_bounded(
        file,
        bytes,
        hyprland_editor_core::workspace::MAX_WORKSPACE_BYTES,
    )
    .await
}

async fn save_new_bounded(file: &gio::File, bytes: Vec<u8>, limit: usize) -> Result<(), String> {
    if !file.is_native() {
        return Err("Choose a local folder for this configuration copy.".into());
    }
    if bytes.len() > limit {
        return Err(format!(
            "This copy exceeds the {} MiB file limit.",
            limit / hyprland_editor_core::MAX_DOCUMENT_BYTES
        ));
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
                format!(
                    "Could not create the configuration copy: {}",
                    error.message()
                )
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
    parent: &impl IsA<gtk::Window>,
    title: &str,
    description: &str,
    filename: &str,
    text: String,
) -> adw::Window {
    review_format(parent, title, description, filename, text, false)
}

pub(super) fn review_legacy(parent: &impl IsA<gtk::Window>, text: String) -> adw::Window {
    review_format(parent,"Review edited legacy source copy","Supported scalar, session and classic monitor edits are included; unknown directives and comments are preserved. Includes and variable expansion remain unresolved. Save with a new filename. Other structured draft groups are not exported here.","hyprland-edited.conf",text,true)
}

fn review_format(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    description: &str,
    filename: &str,
    text: String,
    legacy: bool,
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
    let copy = gtk::Button::with_label(if legacy { "Copy Config" } else { "Copy Lua" });
    let save = gtk::Button::with_label(if legacy {
        "Save Config File…"
    } else {
        "Save Lua File…"
    });
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
            copied.set_text("Reviewed configuration copied to the clipboard.");
        }
    });
    let window = dialog.downgrade();
    let filename = filename.to_owned();
    save.connect_clicked(move |button| {
        let Some(parent) = window.upgrade() else {
            return;
        };
        let chooser = gtk::FileDialog::builder()
            .title(if legacy {
                "Save Legacy Config Copy"
            } else {
                "Save Lua Copy"
            })
            .initial_name(&filename)
            .accept_label("Save Copy")
            .build();
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(if legacy {
            "Legacy configuration (*.conf)"
        } else {
            "Lua configuration (*.lua)"
        }));
        filter.add_pattern(if legacy { "*.conf" } else { "*.lua" });
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
                status.set_text("Saving the reviewed configuration copy…");
            }
            let result = save_new(&file, text.as_bytes().to_vec()).await;
            if let Some(button) = button.upgrade() {
                button.set_sensitive(true);
            }
            if let Some(status) = status.upgrade() {
                match result {
                    Ok(()) => status.set_text(&format!(
                        "Configuration copy saved to {}",
                        file.parse_name()
                    )),
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

    #[test]
    fn named_project_accepts_the_workspace_limit_and_refuses_wrong_names_or_replacement() {
        let root = super::super::test_support::TempDir::new("project-export");
        let path = root.path().join("complete.hye");
        let catalogue = hyprland_editor_core::catalogue();
        let mut bytes = hyprland_editor_core::workspace::Workspace::new(
            hyprland_editor_core::Draft::new(hyprland_editor_core::BASELINE),
        )
        .to_json(&catalogue)
        .unwrap();
        // A complete graph envelope can exceed the single-source export limit.
        bytes.resize(MAX_EXPORT_BYTES + 32, b' ');
        let context = glib::MainContext::new();
        context
            .with_thread_default(|| {
                context
                    .block_on(save_project_new(&gio::File::for_path(&path), bytes.clone()))
                    .unwrap();
                assert_eq!(fs::read(&path).unwrap(), bytes);
                assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
                hyprland_editor_core::workspace::Workspace::from_json(
                    std::str::from_utf8(&bytes).unwrap(),
                    &catalogue,
                )
                .unwrap();
                assert!(context
                    .block_on(save_project_new(
                        &gio::File::for_path(&path),
                        b"replacement".to_vec()
                    ))
                    .is_err());
                assert_eq!(fs::read(&path).unwrap(), bytes);
                let wrong = root.path().join("hyprland.conf");
                assert!(context
                    .block_on(save_project_new(
                        &gio::File::for_path(&wrong),
                        bytes.clone()
                    ))
                    .is_err());
                assert!(!wrong.exists());
                let oversized = root.path().join("oversized.hye");
                assert!(context
                    .block_on(save_project_new(
                        &gio::File::for_path(&oversized),
                        vec![0; hyprland_editor_core::workspace::MAX_WORKSPACE_BYTES + 1]
                    ))
                    .is_err());
                assert!(!oversized.exists());
            })
            .unwrap();
    }
}
