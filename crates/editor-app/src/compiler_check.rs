//! Read-only optional compiler check. Closing or changing the draft invalidates
//! the result; one check per window runs on demand, with no resident daemon.
use super::{
    compiler_client::{self, wire},
    Shared,
};
use adw::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
type Probe = Arc<dyn Fn(&AtomicBool) -> Result<String, String> + Send + Sync>;
pub(super) fn connect(window: &adw::ApplicationWindow, state: &Shared) {
    connect_probe(
        window,
        state,
        Arc::new(|cancelled| {
            if cancelled.load(Ordering::Acquire) {
                return Err("Compiler check was cancelled".into());
            }
            let compiler = compiler_client::Compiler::installed()?;
            let reply = compiler.run(wire::Operation::Profile {}, cancelled)?;
            let profile = reply.profile.ok_or("No compiler profile")?;
            Ok(format!("Lua {} compiler is available.\n\nSource analysis still uses Lua 5.4. This check does not validate the running Hyprland or change your configuration.",profile.release))
        }),
    );
}
fn connect_probe(window: &adw::ApplicationWindow, state: &Shared, probe: Probe) {
    let action = gtk::gio::SimpleAction::new("check-compiler", None);
    let weak_window = window.downgrade();
    let state = state.clone();
    action.connect_activate(move |action, _| {
        if let Some(window) = weak_window.upgrade() {
            launch(action, &window, &state, probe.clone());
        }
    });
    window.add_action(&action);
}
struct Check {
    generation: u64,
    state: Shared,
    cancelled: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    receiver: mpsc::Receiver<Result<String, String>>,
    window: gtk::glib::WeakRef<adw::ApplicationWindow>,
    dialog: gtk::glib::WeakRef<adw::MessageDialog>,
    action: gtk::glib::WeakRef<gtk::gio::SimpleAction>,
    close_handler: Option<gtk::glib::SignalHandlerId>,
}
impl Check {
    fn poll(&mut self) -> gtk::glib::ControlFlow {
        if self
            .window
            .upgrade()
            .is_none_or(|window| !window.is_visible())
        {
            self.cancelled.store(true, Ordering::Release);
            if let Some(dialog) = self.dialog.upgrade() {
                dialog.destroy();
            }
        }
        if self.state.borrow().generation != self.generation {
            self.cancelled.store(true, Ordering::Release);
            if let Some(dialog) = self.dialog.upgrade() {
                dialog.set_body("The draft changed. This compiler check was cancelled.");
            }
        }
        // GTK never waits for package reads, child I/O or cleanup.
        if !self.thread.as_ref().unwrap().is_finished() {
            return gtk::glib::ControlFlow::Continue;
        }
        let joined = self.thread.take().unwrap().join();
        if let Some(action) = self.action.upgrade() {
            action.set_enabled(true);
        }
        if !self.cancelled.load(Ordering::Acquire) {
            if let Some(dialog) = self.dialog.upgrade() {
                let body = match self.receiver.try_recv() {
                    Ok(Ok(body)) if joined.is_ok() => body,
                    Ok(Err(reason)) if joined.is_ok() => format!(
                        "The optional compiler could not be checked.\n\n{}\n\nSource analysis continues with Lua 5.4.",
                        reason.chars().take(300).collect::<String>()
                    ),
                    _ => "The compiler check stopped unexpectedly. Source analysis continues with Lua 5.4.".into(),
                };
                dialog.set_body(&body);
            }
        }
        gtk::glib::ControlFlow::Break
    }
}
impl Drop for Check {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let (Some(window), Some(handler)) = (self.window.upgrade(), self.close_handler.take()) {
            window.disconnect(handler);
        }
    }
}
fn launch(
    action: &gtk::gio::SimpleAction,
    window: &adw::ApplicationWindow,
    state: &Shared,
    probe: Probe,
) {
    action.set_enabled(false);
    let cancelled = Arc::new(AtomicBool::new(false));
    let dialog = adw::MessageDialog::new(
        Some(window),
        Some("Compiler check"),
        Some("Checking the optional Lua 5.5 compiler…"),
    );
    dialog.add_response("close", "Close");
    dialog.set_default_response(Some("close"));
    dialog.set_close_response("close");
    let flag = cancelled.clone();
    dialog.connect_response(None, move |_, _| {
        flag.store(true, Ordering::Release);
    });
    let flag = cancelled.clone();
    dialog.connect_destroy(move |_| {
        flag.store(true, Ordering::Release);
    });
    let flag = cancelled.clone();
    let close_handler = window.connect_destroy(move |_| {
        flag.store(true, Ordering::Release);
    });
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker_cancel = cancelled.clone();
    let thread = std::thread::spawn(move || {
        let result = probe(&worker_cancel);
        let _ = sender.send(result);
    });
    let mut check = Check {
        generation: state.borrow().generation,
        state: state.clone(),
        cancelled,
        receiver,
        thread: Some(thread),
        window: window.downgrade(),
        dialog: dialog.downgrade(),
        action: action.downgrade(),
        close_handler: Some(close_handler),
    };
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(20), move || check.poll());
    dialog.present();
}

#[cfg(test)]
pub(super) fn test_ui(window: &adw::ApplicationWindow) {
    use std::sync::atomic::AtomicUsize;
    let previous = window.lookup_action("check-compiler").unwrap();
    let state = super::initial_state();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    connect_probe(
        window,
        &state,
        Arc::new(move |cancelled| {
            count.fetch_add(1, Ordering::SeqCst);
            for _ in 0..20 {
                if cancelled.load(Ordering::Acquire) {
                    return Err("Cancelled fixture".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok("Available fixture".into())
        }),
    );
    let settle = || {
        let context = gtk::glib::MainContext::default();
        for _ in 0..50 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    let dialog = || {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<adw::MessageDialog>().ok())
            .find(|w| w.heading().as_deref() == Some("Compiler check"))
            .unwrap()
    };
    window
        .lookup_action("check-compiler")
        .unwrap()
        .activate(None);
    let first = dialog();
    window
        .lookup_action("check-compiler")
        .unwrap()
        .activate(None);
    assert_eq!(first.default_response().as_deref(), Some("close"));
    first.response("close");
    first.destroy();
    settle();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    window
        .lookup_action("check-compiler")
        .unwrap()
        .activate(None);
    let stale = dialog();
    state.borrow_mut().generation += 1;
    settle();
    assert!(!stale.body().contains("Available fixture"));
    stale.response("close");
    stale.destroy();
    settle();
    window
        .lookup_action("check-compiler")
        .unwrap()
        .activate(None);
    settle();
    let ready = dialog();
    assert_eq!(ready.body(), "Available fixture");
    ready.response("close");
    ready.destroy();
    settle();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    connect_probe(
        window,
        &state,
        Arc::new(|_| Err("Private package fixture unavailable".into())),
    );
    window
        .lookup_action("check-compiler")
        .unwrap()
        .activate(None);
    settle();
    let failed = dialog();
    assert!(failed
        .body()
        .contains("Private package fixture unavailable"));
    assert!(failed
        .body()
        .contains("Source analysis continues with Lua 5.4"));
    if let Ok(path) = std::env::var("HYPRLAND_EDITOR_TEST_CAPTURE") {
        let paintable = gtk::WidgetPaintable::new(Some(&failed));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, failed.width() as f64, failed.height() as f64);
        failed
            .renderer()
            .unwrap()
            .render_texture(snapshot.to_node().unwrap(), None)
            .save_to_png(format!("{path}.compiler.png"))
            .unwrap();
    }
    failed.response("close");
    failed.destroy();
    settle();
    let destroyed = Arc::new(AtomicBool::new(false));
    let observed = destroyed.clone();
    let child = adw::ApplicationWindow::new(&window.application().unwrap());
    child.present();
    connect_probe(
        &child,
        &state,
        Arc::new(move |cancelled| {
            for _ in 0..200 {
                if cancelled.load(Ordering::Acquire) {
                    observed.store(true, Ordering::Release);
                    return Err("Closed fixture".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err("Missed closure".into())
        }),
    );
    child
        .lookup_action("check-compiler")
        .unwrap()
        .activate(None);
    child.destroy();
    settle();
    assert!(destroyed.load(Ordering::Acquire));
    window.add_action(&previous);
}
