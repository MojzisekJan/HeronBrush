//! Persistent animation trigger connections at the top of the Entity inspector: the target from every animated
//! entity or picked in a view, the action, the clip, the delay and once, each applied the
//! moment it changes, and trying the trigger out.
use std::cell::RefCell;

use crate::editor::Editor;
use crate::{theme, widgets};
use editor_core::{
    animation_trigger::{self, Config, ACTIONS},
    document::Document,
    i18n::{tr, translation_key},
};
use eframe::egui;
use map_model::node::{self, NodeId};

/// The action drop-down's labels, in the order of [`ACTIONS`].
const ACTION_LABELS: [&str; 7] = [translation_key("Toggle"), translation_key("Play"), translation_key("Open"), translation_key("Close"), translation_key("Pause"), translation_key("Resume"), translation_key("Stop")];

#[derive(Default)]
pub struct TriggerPanel {
    /// The trigger being shown, so a change of selection ends a pick in progress.
    source: Option<NodeId>,
    /// What the last change or pick failed with, and the document revision it failed at:
    /// it stands until the document next changes, when the panel's own check takes over.
    failed: Option<(String, u64)>,
}

/// What the panel offers for one trigger, read fresh from the document.
struct View {
    config: Config,
    /// The animated entities a target can be, with their labels.
    targets: Vec<(NodeId, String)>,
    /// Which of them is chosen, counting "Choose target" as 0.
    target: usize,
    /// The clips of the target's library, "" first for the selected animation.
    clips: Vec<String>,
    /// What is wrong with the connection as it stands.
    problem: Option<String>,
}

/// The single selected button or trigger, which the panel is shown for.
pub(crate) fn source(document: &Document) -> Option<NodeId> {
    let selected = document.selected_entities();
    (selected.len() == 1 && animation_trigger::supported(selected[0].1.classname().unwrap_or(""))).then(|| selected[0].0)
}

fn read(document: &Document, source: NodeId) -> Option<View> {
    let config = Config::read(document, source).ok()?;
    let entities = node::entities_with_ids(document.tree_world());
    let targets: Vec<(NodeId, String)> = entities
        .iter()
        .filter(|(_, e)| e.property("animation_keys").is_some() || e.property("animation_curves").is_some())
        .map(|(n, e)| {
            let auto = format!("animation_{}", n.0);
            let name = e.property("targetname").filter(|name| *name != auto).unwrap_or(e.classname().unwrap_or("Entity"));
            (*n, format!("{name} · {:06x}", n.0 & 0xff_ffff))
        })
        .collect();
    let target = targets
        .iter()
        .position(|(n, _)| {
            !config.target.is_empty() && entities.iter().any(|(id, e)| id == n && e.property("targetname") == Some(config.target.as_str()))
        })
        .map_or(0, |i| i + 1);
    let found = config.targets(document);
    let mut clips = vec![String::new()];
    if let Some(first) = found.as_ref().ok().and_then(|ids| ids.first()) {
        if let Ok((library, _)) = animation_trigger::library(document, *first) {
            clips.extend(library.clips.keys().cloned());
        }
    }
    Some(View { config, targets, target, clips, problem: found.err() })
}

/// Applies the panel's choices to the trigger. `target` is the
/// chosen entity, or `None` for "Choose target", which clears it.
pub(crate) fn apply(document: &mut Document, source: NodeId, mut config: Config, target: Option<NodeId>) -> Result<(), String> {
    if document.in_transaction() {
        return Err(translation_key("Finish the current drag first").into());
    }
    match target {
        None => config.target.clear(),
        // A different target has different clips.
        Some(id) if !config.targets(document).unwrap_or_default().contains(&id) => config.clip.clear(),
        Some(_) => {}
    }
    animation_trigger::configure(document, source, config, target, false).map(|_| ())
}

/// Connects the trigger to what a click in a view hit: the entity it is, or the one it is
/// part of. The clip is cleared, as the new target's clips are its own.
pub(crate) fn pick(document: &mut Document, source: NodeId, hit: Option<NodeId>) -> Result<(), String> {
    let hit = hit.ok_or(translation_key("Click an animated object"))?;
    let id = node::entities_with_ids(document.tree_world())
        .into_iter()
        .find(|(id, _)| *id == hit || node::contents_of(document.tree_world(), *id).contains(&hit))
        .map(|(id, _)| id)
        .ok_or(translation_key("Click an animated object"))?;
    let mut config = Config::read(document, source)?;
    config.clip.clear();
    animation_trigger::configure(document, source, config, Some(id), false).map(|_| ())
}

impl TriggerPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let current = source(&editor.document.borrow());
        if self.source != current {
            // A different trigger, or none: a pick for the old one ends.
            *editor.target_pick.borrow_mut() = Default::default();
            self.source = current;
            self.failed = None;
        }
        let Some(source) = current else { return };
        self.take_pick(editor, source);
        let Some(view) = read(&editor.document.borrow(), source) else { return };

        ui.push_id("animation-trigger", |ui| {
            widgets::section(ui, "animation-trigger", tr("Animation trigger"), true, |ui| self.body(ui, editor, source, &view));
        });
    }

    fn body(&mut self, ui: &mut egui::Ui, editor: &Editor, source: NodeId, view: &View) {
        let gap = ui.spacing().item_spacing.x;
        let picking = editor.target_pick.borrow().source == Some(source);

        let mut target = view.target;
        let mut changed: Option<(Config, Option<NodeId>)> = None;
        ui.horizontal(|ui| {
            let width = ui.available_width();
            let label = tr(if picking { "Cancel" } else { "Pick target" });
            let button_width = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), theme::body(), theme::fg()).size().x) + 24.0;
            let mut labels = vec![tr("Choose target").to_string()];
            labels.extend(view.targets.iter().map(|(_, label)| label.clone()));
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            let response = ui.scope(|ui| widgets::dropdown(ui, "trigger-target", &mut target, &labels, (width - button_width - gap).max(80.0)));
            if !view.config.target.is_empty() {
                response.response.on_hover_text(&view.config.target);
            }
            if response.inner {
                let id = target.checked_sub(1).and_then(|i| view.targets.get(i)).map(|(id, _)| *id);
                changed = Some((view.config.clone(), id));
            }
            if theme::button(ui, label, button_width, true).clicked() {
                if picking {
                    *editor.target_pick.borrow_mut() = Default::default();
                } else {
                    // Motion previews would move what is to be clicked.
                    let mut filters = editor.filters.borrow_mut();
                    filters.preview_motion = false;
                    filters.animation_frame = None;
                    *editor.target_pick.borrow_mut() = crate::editor::TargetPick { source: Some(source), picked: None };
                }
            }
        });
        if picking && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            *editor.target_pick.borrow_mut() = Default::default();
        }
        // The chosen target, as the drop-down reads it, for the changes below.
        let chosen = view.target.checked_sub(1).and_then(|i| view.targets.get(i)).map(|(id, _)| *id);

        let mut action = ACTIONS.iter().position(|a| *a == view.config.action).unwrap_or(0);
        let labels: Vec<&str> = ACTION_LABELS.iter().map(|l| tr(l)).collect();
        if widgets::row(ui, tr("Action"), None, |ui| widgets::dropdown(ui, "trigger-action", &mut action, &labels, ui.available_width())) {
            let mut config = view.config.clone();
            config.action = ACTIONS[action].into();
            changed = Some((config, chosen));
        }

        let mut clip = view.clips.iter().position(|c| *c == view.config.clip).unwrap_or(0);
        let labels: Vec<&str> = view.clips.iter().map(|c| if c.is_empty() { tr("Selected animation") } else { c.as_str() }).collect();
        let play = view.config.action == "play";
        if widgets::row(ui, tr("Animation"), None, |ui| ui.add_enabled_ui(play, |ui| widgets::dropdown(ui, "trigger-clip", &mut clip, &labels, ui.available_width())).inner) {
            let mut config = view.config.clone();
            config.clip = view.clips[clip].clone();
            changed = Some((config, chosen));
        }

        let mut wait = view.config.wait;
        if widgets::row(ui, tr("Repeat delay (s)"), None, |ui| widgets::spin(ui, &mut wait, 0.0..=100000.0, 0.1, 2).changed()) {
            let mut config = view.config.clone();
            config.wait = wait;
            changed = Some((config, chosen));
        }

        let mut once = view.config.once;
        if widgets::toggle_row(ui, tr("Only once"), None, &mut once).changed() {
            let mut config = view.config.clone();
            config.once = once;
            changed = Some((config, chosen));
        }

        match widgets::segmented(ui, "trigger-preview", &[(tr("Try trigger"), "", true), (tr("Reset preview"), "", true)]) {
            Some(0) => {
                let result = editor.document.borrow_mut().preview_trigger(source);
                match result {
                    Ok(()) => {
                        self.failed = None;
                        let mut filters = editor.filters.borrow_mut();
                        filters.preview_motion = true;
                        filters.animation_frame = None;
                    }
                    Err(error) => self.fail(&editor.document, error),
                }
            }
            Some(_) => {
                editor.document.borrow_mut().reset_trigger_preview();
                editor.filters.borrow_mut().preview_motion = false;
            }
            None => {}
        }

        if let Some((config, target)) = changed {
            let result = apply(&mut editor.document.borrow_mut(), source, config, target);
            match result {
                Ok(()) => self.failed = None,
                Err(error) => self.fail(&editor.document, error),
            }
        }

        // The connection's own problem, unless a change or pick just failed.
        let revision = editor.document.borrow().revision();
        let status = match &self.failed {
            Some((text, at)) if *at == revision => Some(text.clone()),
            _ => read(&editor.document.borrow(), source).and_then(|v| v.problem),
        };
        if let Some(text) = status.filter(|t| !t.is_empty()) {
            ui.add(egui::Label::new(egui::RichText::new(animation_trigger::error_text(&text)).color(theme::error())).wrap().selectable(true));
        }
    }

    fn fail(&mut self, document: &RefCell<Document>, error: String) {
        self.failed = Some((error, document.borrow().revision()));
    }

    /// Takes a click a view made while picking for this trigger: a hit on an animated
    /// object connects it and ends the pick; a miss says so and keeps picking.
    fn take_pick(&mut self, editor: &Editor, source: NodeId) {
        let hit = {
            let mut picker = editor.target_pick.borrow_mut();
            if picker.source != Some(source) {
                return;
            }
            let Some(hit) = picker.picked.take() else { return };
            hit
        };
        let result = pick(&mut editor.document.borrow_mut(), source, hit);
        match result {
            Ok(()) => {
                self.failed = None;
                *editor.target_pick.borrow_mut() = Default::default();
            }
            Err(error) => self.fail(&editor.document, error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_core::animation_trigger::tests::fixture;

    #[test]
    fn a_change_applies_at_once_and_survives_a_revision_bump() {
        let (mut d, target, source, _) = fixture();
        d.select_only(source);
        { let c = Config::read(&d, source).unwrap(); apply(&mut d, source, c, Some(target)) }.unwrap();
        let mut config = Config::read(&d, source).unwrap();
        config.action = "play".into();
        config.clip = "route".into();
        config.wait = 0.5;
        config.once = false;
        apply(&mut d, source, config, Some(target)).unwrap();
        // An unrelated edit bumps the revision; the panel reads the trigger afresh, and
        // what was applied is still there.
        let brush = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &geometry_math::bbox::Bbox3d::from_scalars(100.0, 132.0),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![brush]);
        d.select_only(source);
        let view = read(&d, source).unwrap();
        assert_eq!(view.config.action, "play");
        assert_eq!(view.config.clip, "route");
        assert_eq!(view.config.wait, 0.5);
        assert!(!view.config.once);
        assert_eq!(view.target, view.targets.iter().position(|(id, _)| *id == target).unwrap() + 1);
        assert_eq!(view.clips, vec![String::new(), "route".to_string()]);
        assert!(view.problem.is_none());
    }

    #[test]
    fn choose_target_clears_it_and_lists_every_animated_entity() {
        let (mut d, target, source, _) = fixture();
        { let c = Config::read(&d, source).unwrap(); apply(&mut d, source, c, Some(target)) }.unwrap();
        let view = read(&d, source).unwrap();
        assert!(view.targets.iter().any(|(id, label)| *id == target && label.contains(" · ")));
        apply(&mut d, source, view.config, None).unwrap();
        let view = read(&d, source).unwrap();
        assert_eq!(view.target, 0);
        assert!(view.config.target.is_empty());
        assert_eq!(view.problem.as_deref(), Some("Choose an animation target"));
    }

    #[test]
    fn picking_connects_a_part_of_the_target_and_clears_the_clip() {
        let (mut d, target, source, brush) = fixture();
        assert_eq!(pick(&mut d, source, None).unwrap_err(), "Click an animated object");
        pick(&mut d, source, Some(brush)).unwrap();
        let config = Config::read(&d, source).unwrap();
        assert_eq!(config.targets(&d).unwrap(), vec![target]);
        assert!(config.clip.is_empty());
    }

    #[test]
    fn nothing_changes_during_a_drag() {
        let (mut d, target, source, _) = fixture();
        d.begin_transaction();
        let error = { let c = Config::read(&d, source).unwrap(); apply(&mut d, source, c, Some(target)) }.unwrap_err();
        assert_eq!(error, "Finish the current drag first");
        d.commit_transaction();
    }

    /// The whole panel in a frame: it sits in the Entity tab for a selected trigger, and a
    /// pick that hits an animated object applies at once and ends picking.
    #[test]
    fn the_panel_applies_a_pick_from_a_view() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        let (mut d, target, source, brush) = fixture();
        d.select_only(source);
        *editor.document.borrow_mut() = d;
        let mut panel = TriggerPanel::default();
        let frame = |panel: &mut TriggerPanel| {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| panel.show(ui, &editor));
            output.textures_delta.clear();
        };
        frame(&mut panel);
        *editor.target_pick.borrow_mut() = crate::editor::TargetPick { source: Some(source), picked: Some(None) };
        frame(&mut panel);
        assert_eq!(editor.target_pick.borrow().source, Some(source), "a miss keeps picking");
        assert_eq!(panel.failed.as_ref().map(|(t, _)| t.as_str()), Some("Click an animated object"));
        editor.target_pick.borrow_mut().picked = Some(Some(brush));
        frame(&mut panel);
        assert_eq!(editor.target_pick.borrow().source, None);
        let d = editor.document.borrow();
        assert_eq!(Config::read(&d, source).unwrap().targets(&d).unwrap(), vec![target]);
    }
}
