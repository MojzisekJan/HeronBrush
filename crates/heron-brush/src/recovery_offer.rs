//! Crash recovery in the window: keeps the open map's recovery copy
//! ([`editor_core::autosave::recovery`]) as the preferences say, and at start offers
//! the copies an editor left behind when it stopped without saving.

use crate::{dialog, theme};
use editor_core::document::Document;
use editor_core::i18n::{fill, tr};
use eframe::egui;
use web_time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub use editor_core::autosave::recovery::Orphan;

/// The browser keeps its own copies ([`crate::web`]); there is nothing to offer here.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, PartialEq)]
pub struct Orphan;

/// What the user chose in the offer.
pub enum Choice {
    /// Open the copy in place of the current map.
    Restore(Orphan),
}

/// The recovery copy of the open map, and copies to offer.
pub struct RecoveryState {
    #[cfg(not(target_arch = "wasm32"))]
    keeper: editor_core::autosave::recovery::Recovery,
    /// Copies left by editors that stopped without saving, newest first.
    pub offer: Vec<Orphan>,
    /// Answered "Later": not asked again until the next start.
    later: bool,
    /// The preferences version last read.
    preferences: u64,
}

fn options() -> (bool, Duration) {
    let preferences = editor_core::preferences::current();
    (
        preferences.recovery_enabled,
        Duration::from_secs_f32(preferences.recovery_interval.clamp(5.0, 600.0)),
    )
}

impl RecoveryState {
    /// Copies for `document`, nothing offered yet.
    pub fn new(document: &Document) -> Self {
        let (enabled, interval) = options();
        #[cfg(target_arch = "wasm32")]
        let _ = (document, enabled, interval);
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            keeper: editor_core::autosave::recovery::Recovery::new(
                editor_core::autosave::recovery::dir(),
                document,
                enabled,
                interval,
            ),
            offer: Vec::new(),
            later: false,
            preferences: editor_core::preferences::version(),
        }
    }

    /// The copies left behind by editors no longer running, to offer at start.
    pub fn look_for_copies(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.offer =
                editor_core::autosave::recovery::orphans(&editor_core::autosave::recovery::dir());
        }
    }

    /// Writes the copy when it is due, following the preferences. Call every frame.
    pub fn tick(&mut self, document: &Document) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let version = editor_core::preferences::version();
            if version != self.preferences {
                self.preferences = version;
                let (enabled, interval) = options();
                self.keeper.set_options(enabled, interval);
            }
            self.keeper.tick(document);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = document;
    }

    /// When the next copy is due, for the idle wake-up.
    pub fn next_due(&self, document: &Document) -> Option<Duration> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.keeper.next_due(document)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = document;
            None
        }
    }

    /// The map was saved, closed or its changes discarded: its copy goes.
    pub fn discard(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.keeper.discard();
    }

    /// The editor is closing cleanly: its copy goes and it is no longer marked running.
    pub fn close(&mut self) {
        self.discard();
        #[cfg(not(target_arch = "wasm32"))]
        editor_core::autosave::recovery::unregister(&editor_core::autosave::recovery::dir());
    }

    /// Takes over a restored copy's files.
    pub fn adopt(&mut self, orphan: &Orphan) {
        #[cfg(not(target_arch = "wasm32"))]
        self.keeper.adopt(orphan);
        #[cfg(target_arch = "wasm32")]
        let _ = orphan;
    }

    /// The offer, one copy at a time: Restore, Discard or Later.
    pub fn dialog(&mut self, ctx: &egui::Context) -> Option<Choice> {
        if self.later {
            return None;
        }
        let orphan = self.offer.first()?.clone();
        let mut choice = None;
        let mut restore = false;
        let mut discard = false;
        let mut later = false;
        dialog::show(
            ctx,
            "recovery",
            tr("Recover unsaved work"),
            [520.0, 250.0],
            |ui, part| {
                if part == dialog::Part::Actions {
                    restore = dialog::accent_button(ui, tr("Restore"), true).clicked();
                    discard = dialog::button(ui, tr("Discard"), true).clicked();
                    later = dialog::button(ui, tr("Later"), true).clicked();
                    return;
                }
                body(ui, &orphan);
            },
        );
        if restore {
            self.offer.remove(0);
            choice = Some(Choice::Restore(orphan));
        } else if discard {
            #[cfg(not(target_arch = "wasm32"))]
            orphan.discard();
            self.offer.remove(0);
        } else if later {
            self.later = true;
        }
        choice
    }
}

fn body(ui: &mut egui::Ui, orphan: &Orphan) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let name = orphan.path.as_deref().map_or_else(
            || tr("Untitled map").to_string(),
            |path| path.display().to_string(),
        );
        ui.label(egui::RichText::new(name).font(theme::body_bold()));
        ui.add_space(6.0);
        ui.add(
            egui::Label::new(fill(
                "Heron Brush closed without saving this map. Restore the copy made {} minutes ago?",
                &[&orphan.minutes_ago().to_string()],
            ))
            .wrap(),
        );
        if orphan.file_changed {
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(tr(
                        "The file has changed on disk since this copy was made; saving the restored map would overwrite those changes.",
                    ))
                    .color(theme::warning()),
                )
                .wrap(),
            );
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (ui, orphan);
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// The texts a frame of the offer drew.
    fn texts(ctx: &egui::Context, state: &mut RecoveryState) -> (Vec<String>, Option<Choice>) {
        let mut choice = None;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            choice = state.dialog(ui.ctx());
        });
        output.textures_delta.clear();
        let mut found = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                found.push(text.galley.text().to_string());
            }
        }
        (found, choice)
    }

    #[test]
    fn a_copy_left_behind_is_offered_with_a_warning_when_its_file_changed() {
        let dir = std::env::temp_dir().join(format!("heron-offer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let map = dir.join("hall.map");
        std::fs::write(&map, "").unwrap();
        std::fs::write(
            dir.join("hall-x.map"),
            Document::new(Default::default()).to_map_text(),
        )
        .unwrap();
        std::fs::write(
            dir.join("hall-x.meta"),
            format!(
                "path={}\nsaved=1\npid=4000000000\ninstance=gone\nname=hall.map\n",
                map.display()
            ),
        )
        .unwrap();
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut state = RecoveryState::new(&Document::new(Default::default()));
        state.offer = editor_core::autosave::recovery::orphans(&dir);
        assert_eq!(state.offer.len(), 1);
        // A window shows from its second frame, once it knows its size.
        texts(&ctx, &mut state);
        let (shown, choice) = texts(&ctx, &mut state);
        assert!(choice.is_none());
        let all = shown.join("\n");
        assert!(all.contains(&map.display().to_string()), "{all}");
        assert!(
            all.contains(tr("Restore")) && all.contains(tr("Later")),
            "{all}"
        );
        assert!(all.contains("changed on disk"), "{all}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
