//! Release checks stay quiet until requested or enabled in preferences.
use crate::dialog;
use editor_core::{i18n::tr, preferences};
use eframe::egui;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
#[derive(Default)]
pub struct UpdatePage {
    pub open: bool,
    initialized: bool,
    #[cfg(not(target_arch = "wasm32"))]
    status: String,
    #[cfg(not(target_arch = "wasm32"))]
    result: Option<editor_core::updates::Check>,
    #[cfg(not(target_arch = "wasm32"))]
    checked_repository: String,
    #[cfg(not(target_arch = "wasm32"))]
    events: Option<mpsc::Receiver<Event>>,
    #[cfg(not(target_arch = "wasm32"))]
    cancel: Option<Arc<AtomicBool>>,
    #[cfg(not(target_arch = "wasm32"))]
    installing: bool,
}
#[cfg(not(target_arch = "wasm32"))]
enum Event {
    Checked(Result<editor_core::updates::Check, String>),
    Progress(u64, u64),
    Installed(Result<(), String>),
}
impl UpdatePage {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.initialized {
            self.initialized = true;
            #[cfg(not(target_arch = "wasm32"))]
            if preferences::pref(|p| p.update_on_startup) {
                self.check();
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(events) = &self.events {
            let mut done = false;
            while let Ok(event) = events.try_recv() {
                match event {
                    Event::Checked(result) => {
                        done = true;
                        match result {
                            Ok(result) => {
                                self.status = match &result {
                                    editor_core::updates::Check::NoPublishedRelease => {
                                        tr("No published release is available yet").into()
                                    }
                                    editor_core::updates::Check::Current => {
                                        tr("This version is current").into()
                                    }
                                    editor_core::updates::Check::Available(release) => {
                                        self.open = true;
                                        format!("{} {}", tr("Update available:"), release.version)
                                    }
                                };
                                self.result = Some(result);
                            }
                            Err(error) => {
                                self.status = error;
                                self.result = None;
                            }
                        }
                    }
                    Event::Progress(bytes, total) => {
                        self.status = format!(
                            "{} {:.1} / {:.1} MiB",
                            tr("Downloading…"),
                            bytes as f64 / 1048576.,
                            total as f64 / 1048576.
                        )
                    }
                    Event::Installed(result) => {
                        done = true;
                        self.installing = false;
                        self.status = match result {
                            Ok(()) => tr("Update installed. Restart the editor to use it.").into(),
                            Err(error) => error,
                        };
                        self.result = None;
                    }
                }
            }
            if done {
                self.events = None;
                self.cancel = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.result.is_some()
            && preferences::pref(|p| p.update_repository != self.checked_repository)
        {
            self.result = None;
            self.status.clear();
        }
        if !self.open {
            return;
        }
        let mut close = false;
        self.open = dialog::show_modal(
            ctx,
            "updates",
            tr("Application updates"),
            [640.0, 380.0],
            |ui, part| {
                if matches!(part, dialog::Part::Actions) {
                    if dialog::button(ui, tr("Close"), true).clicked() {
                        close = true;
                    }
                    return;
                }
                ui.label(format!("Heron Brush {}", env!("CARGO_PKG_VERSION")));
                let mut repo = preferences::pref(|p| p.update_repository.clone());
                ui.horizontal(|ui| {
                    ui.label(tr("GitHub repository"));
                    if ui
                        .add_enabled(
                            {
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    self.events.is_none()
                                }
                                #[cfg(target_arch = "wasm32")]
                                {
                                    true
                                }
                            },
                            egui::TextEdit::singleline(&mut repo).margin(crate::widgets::body_entry_margin(ui, 6, 6)).min_size(egui::vec2(0.0, crate::widgets::ROW_HEIGHT)),
                        )
                        .changed()
                    {
                        preferences::edit(|p| p.update_repository = repo.clone());
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            self.result = None;
                            self.status.clear();
                        }
                    }
                });
                for (label, value, kind) in [
                    (
                        tr("Check for updates on startup"),
                        preferences::pref(|p| p.update_on_startup),
                        0,
                    ),
                    (
                        tr("Include prereleases"),
                        preferences::pref(|p| p.update_prereleases),
                        1,
                    ),
                ] {
                    let mut value = value;
                    if crate::widgets::checkbox(ui, &mut value, label).changed() {
                        preferences::edit(|p| {
                            if kind == 0 {
                                p.update_on_startup = value
                            } else {
                                p.update_prereleases = value
                            }
                        });
                    }
                }
                #[cfg(target_arch = "wasm32")]
                ui.label(tr("The web editor is updated by its hosting site"));
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui
                        .add_enabled(
                            self.events.is_none(),
                            egui::Button::new(tr("Check for updates")),
                        )
                        .clicked()
                    {
                        self.check();
                    }
                    ui.label(&self.status);
                    if let Some(editor_core::updates::Check::Available(release)) = &self.result {
                        ui.hyperlink_to(tr("Open release downloads"), &release.url);
                        if let Some(artifact) = release.artifact.clone() {
                            ui.label(tr("Installation replaces this Linux executable and keeps a backup. Restart manually after saving your work."));
                            let clicked = ui
                                .add_enabled(
                                    self.events.is_none(),
                                    egui::Button::new(tr("Download and install update")),
                                )
                                .clicked();
                            if clicked {
                                match editor_core::updates::executable() {
                                    Ok(target) => {
                                        let (send, events) = mpsc::sync_channel(16);
                                        let cancel = Arc::new(AtomicBool::new(false));
                                        let flag = cancel.clone();
                                        self.events = Some(events);
                                        self.cancel = Some(cancel);
                                        self.installing = true;
                                        std::thread::spawn(move || {
                                            let result = editor_core::updates::install(
                                                &artifact,
                                                &target,
                                                &flag,
                                                |bytes, total| {
                                                    let _ = send
                                                        .try_send(Event::Progress(bytes, total));
                                                },
                                            );
                                            let _ = send.send(Event::Installed(result));
                                        });
                                    }
                                    Err(error) => self.status = error,
                                }
                            }
                        } else {
                            ui.label(tr("Use release downloads for this package or platform"));
                        }
                    }
                    if self.installing && ui.button(tr("Stop download")).clicked() {
                        if let Some(cancel) = &self.cancel {
                            cancel.store(true, Ordering::Relaxed);
                        }
                    }
                }
            },
        );
        self.open &= !close;
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn check(&mut self) {
        let (repo, pre) =
            preferences::pref(|p| (p.update_repository.clone(), p.update_prereleases));
        if let Err(error) = editor_core::updates::repository(&repo) {
            self.status = error;
            return;
        }
        self.checked_repository = repo.clone();
        let (send, events) = mpsc::channel();
        self.events = Some(events);
        self.status = tr("Checking…").into();
        std::thread::spawn(move || {
            let result = editor_core::updates::check(&repo, pre);
            let _ = send.send(Event::Checked(result));
        });
    }
}
impl Drop for UpdatePage {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}
