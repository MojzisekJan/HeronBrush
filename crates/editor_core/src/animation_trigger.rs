//! Shared, persistent trigger settings and non-destructive local playback.
use crate::{
    animation_library::Library,
    document::Document,
    i18n::{tr_text, translation_key},
    motion_preview::Motion,
};
use map_model::node::{self, NodeId};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

pub const ACTIONS: [&str; 7] = ["toggle", "play", "open", "close", "pause", "resume", "stop"];

/// The errors that name what is missing, with `{}` where the name goes.
const MISSING_TARGET: &str = translation_key("Missing target: {}");
const MISSING_ANIMATION: &str = translation_key("Missing animation: {}");

/// One of this module's errors in the language in force. The errors are written in
/// English, as the MCP tools return them, and translated where a frontend shows them.
pub fn error_text(error: &str) -> String {
    for pattern in [MISSING_TARGET, MISSING_ANIMATION] {
        let head = pattern.trim_end_matches("{}");
        if let Some(name) = error.strip_prefix(head) {
            return tr_text(pattern).replacen("{}", name, 1);
        }
    }
    tr_text(error)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub target: String,
    pub action: String,
    pub clip: String,
    pub once: bool,
    pub wait: f64,
}
pub fn supported(class: &str) -> bool {
    matches!(class, "func_button" | "trigger_once" | "trigger_multiple")
}
impl Config {
    pub fn read(d: &Document, source: NodeId) -> Result<Self, String> {
        let (_, e) = node::entities_with_ids(d.tree_world())
            .into_iter()
            .find(|(id, _)| *id == source)
            .ok_or(translation_key("Trigger no longer exists"))?;
        let class = e.classname().unwrap_or("");
        if !supported(class) {
            return Err(translation_key("Select a button or trigger").into());
        }
        let wait = e
            .property("wait")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(if class == "trigger_multiple" {
                1.0
            } else {
                0.0
            });
        Ok(Self {
            target: e.property("target").unwrap_or("").into(),
            action: e.property("animation_action").unwrap_or("toggle").into(),
            clip: e.property("animation_clip").unwrap_or("").into(),
            once: e
                .property("animation_once")
                .map(|v| v == "1")
                .unwrap_or(class == "trigger_once" || wait < 0.0),
            wait: wait.max(0.0),
        })
    }
    pub fn targets(&self, d: &Document) -> Result<Vec<NodeId>, String> {
        if self.target.is_empty() {
            return Err(translation_key("Choose an animation target").into());
        }
        let ids: Vec<_> = node::entities_with_ids(d.tree_world())
            .into_iter()
            .filter(|(_, e)| e.property("targetname") == Some(self.target.as_str()))
            .map(|(id, _)| id)
            .collect();
        if ids.is_empty() {
            return Err(MISSING_TARGET.replacen("{}", &self.target, 1));
        }
        for id in &ids {
            let (lib, _) = library(d, *id)?;
            let clip = match self.action.as_str() {
                "open" => "open",
                "close" => "close",
                "play" => &self.clip,
                _ => "",
            };
            if !clip.is_empty() && !lib.clips.contains_key(clip) {
                return Err(MISSING_ANIMATION.replacen("{}", clip, 1));
            }
        }
        Ok(ids)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !ACTIONS.contains(&self.action.as_str()) {
            return Err(translation_key("Unknown animation action").into());
        }
        if !self.wait.is_finite() || self.wait < 0.0 || self.wait > 100000.0 {
            return Err(translation_key("Delay must be between 0 and 100000 seconds").into());
        }
        Ok(())
    }
}
pub fn library(d: &Document, id: NodeId) -> Result<(Library, String), String> {
    if !node::entities_with_ids(d.tree_world())
        .iter()
        .any(|(n, e)| {
            *n == id
                && (e.property("animation_keys").is_some()
                    || e.property("animation_curves").is_some())
        })
    {
        return Err(translation_key("Add an animation to the target first").into());
    }
    Library::read(d, id)
}
pub fn configure(
    d: &mut Document,
    source: NodeId,
    mut config: Config,
    target: Option<NodeId>,
    dry: bool,
) -> Result<Config, String> {
    if d.in_transaction() {
        return Err(translation_key("Finish the current drag first").into());
    }
    Config::read(d, source)?;
    config.validate()?;
    let target_name = if let Some(id) = target {
        library(d, id)?;
        let entities = node::entities_with_ids(d.tree_world());
        let e = entities.iter().find(|(n, _)| *n == id).unwrap().1;
        let name = e
            .property("targetname")
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                let base = format!("animation_{}", id.0);
                let mut name = base.clone();
                let mut i = 1;
                while entities
                    .iter()
                    .any(|(_, e)| e.property("targetname") == Some(name.as_str()))
                {
                    name = format!("{base}_{i}");
                    i += 1;
                }
                name
            });
        config.target = name.clone();
        Some((id, name))
    } else {
        None
    };
    // Validate the requested clip even when the target has no name yet.
    if let Some((id, _)) = &target_name {
        let (lib, _) = library(d, *id)?;
        let clip = match config.action.as_str() {
            "open" => "open",
            "close" => "close",
            "play" => &config.clip,
            _ => "",
        };
        if !clip.is_empty() && !lib.clips.contains_key(clip) {
            return Err(MISSING_ANIMATION.replacen("{}", clip, 1));
        }
    } else if !config.target.is_empty() {
        config.targets(d)?;
    }
    if !dry {
        d.run_as_one_edit(|d| {
            if let Some((id, name)) = &target_name {
                d.set_entity_property(*id, "targetname", name);
            }
            for (key, value) in [
                ("target", config.target.clone()),
                ("animation_action", config.action.clone()),
                ("animation_clip", config.clip.clone()),
                ("animation_once", if config.once { "1" } else { "0" }.into()),
                ("wait", config.wait.to_string()),
            ] {
                d.set_entity_property(source, key, &value);
            }
        });
    }
    Ok(config)
}
#[derive(Clone)]
struct Playback {
    name: String,
    motion: Motion,
}
#[derive(Default)]
pub struct Preview {
    players: HashMap<NodeId, Playback>,
    fired: HashMap<NodeId, f64>,
}
impl Preview {
    pub fn active(&self) -> bool {
        !self.players.is_empty()
    }
}
impl Document {
    pub fn preview_motions(&self) -> HashMap<NodeId, Motion> {
        let mut motions = crate::motion_preview::motions(self.tree_world());
        for (id, player) in &self.trigger_preview.players {
            for brush in node::contents_of(self.tree_world(), *id) {
                if self.brush(brush).is_some() {
                    let mut motion = player.motion.clone();
                    if let (
                        Motion::Playback {
                            authored, pivot, ..
                        },
                        Some(Motion::Keyframes {
                            track,
                            pivot: current,
                            ..
                        }),
                    ) = (&mut motion, motions.get(&brush))
                    {
                        *authored = track.clone();
                        *pivot = *current;
                    }
                    motions.insert(brush, motion);
                }
            }
        }
        motions
    }
    pub fn reset_trigger_preview(&mut self) {
        if !self.trigger_preview.players.is_empty() || !self.trigger_preview.fired.is_empty() {
            self.trigger_preview = Preview::default();
            self.mark_changed();
        }
    }
    pub fn preview_trigger(&mut self, source: NodeId) -> Result<(), String> {
        let now = self.animation_preview_time(true);
        self.preview_trigger_at(source, now)
    }
    fn preview_trigger_at(&mut self, source: NodeId, now: f64) -> Result<(), String> {
        let config = Config::read(self, source)?;
        config.validate()?;
        let targets = config.targets(self)?;
        if self
            .trigger_preview
            .fired
            .get(&source)
            .is_some_and(|last| config.once || now - *last < config.wait)
        {
            return Err(
                translation_key("Trigger is waiting or has already fired; reset the preview to try again").into(),
            );
        }
        let base = crate::motion_preview::motions(self.tree_world());
        let mut changes = Vec::new();
        for id in targets {
            let (lib, selected) = library(self, id)?;
            let motion = node::contents_of(self.tree_world(), id)
                .iter()
                .find_map(|n| base.get(n))
                .ok_or(translation_key("Target has no preview geometry"))?;
            let (pivot, track, autoplay) = match motion {
                Motion::Keyframes {
                    pivot,
                    track,
                    autoplay,
                } => (*pivot, track.clone(), *autoplay),
                _ => return Err(translation_key("Target requires keyframes").into()),
            };
            let mut player = self
                .trigger_preview
                .players
                .get(&id)
                .cloned()
                .unwrap_or(Playback {
                    name: selected.clone(),
                    motion: Motion::Playback {
                        pivot,
                        authored: track.clone(),
                        track,
                        offset: if autoplay { now } else { 0.0 },
                        epoch: now,
                        playing: autoplay,
                        posed: autoplay,
                    },
                });
            if let Motion::Playback {
                offset,
                epoch,
                playing,
                posed,
                ..
            } = &mut player.motion
            {
                let time = *offset
                    + if *playing {
                        (now - *epoch).max(0.0)
                    } else {
                        0.0
                    };
                match config.action.as_str() {
                    "pause" => {
                        *offset = time;
                        *epoch = now;
                        *playing = false;
                    }
                    "resume" => {
                        *epoch = now;
                        *playing = true;
                        *posed = true;
                    }
                    "stop" => {
                        *offset = 0.0;
                        *epoch = now;
                        *playing = false;
                        *posed = false;
                    }
                    _ => {
                        let name = match config.action.as_str() {
                            "open" => "open",
                            "close" => "close",
                            "play" => {
                                if config.clip.is_empty() {
                                    &selected
                                } else {
                                    &config.clip
                                }
                            }
                            _ => {
                                if lib.clips.contains_key("open") && lib.clips.contains_key("close")
                                {
                                    if player.name == "open" && *posed {
                                        "close"
                                    } else {
                                        "open"
                                    }
                                } else {
                                    &selected
                                }
                            }
                        }
                        .to_owned();
                        let entry = &lib.clips[&name];
                        let reverse = config.action == "toggle"
                            && *posed
                            && *playing
                            && ((player.name == "open" && name == "close")
                                || (player.name == "close" && name == "open"));
                        let offset = if reverse {
                            entry.clip.duration
                                * (1.0
                                    - (time / lib.clips[&player.name].clip.duration)
                                        .clamp(0.0, 1.0))
                        } else {
                            0.0
                        };
                        let track = Arc::new(crate::keyframe_animation::Track::from_properties(
                            "",
                            &entry.mode,
                            Some(&entry.clip.json()),
                        )?);
                        player = Playback {
                            name,
                            motion: Motion::Playback {
                                pivot,
                                authored: track.clone(),
                                track,
                                offset,
                                epoch: now,
                                playing: true,
                                posed: true,
                            },
                        };
                    }
                }
            }
            changes.push((id, player));
        }
        self.trigger_preview.players.extend(changes);
        self.trigger_preview.fired.insert(source, now);
        self.mark_changed();
        Ok(())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod tests {
    use super::*;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    pub fn fixture() -> (Document, NodeId, NodeId, NodeId) {
        let mut d = Document::new(Default::default());
        let b = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &Bbox3d::new(Vec3d::new(0., 0., 0.), Vec3d::new(32., 32., 32.)),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![b.clone()]);
        d.select_all();
        let animated = d.create_brush_entity("func_animate").unwrap();
        let brush = node::contents_of(d.tree_world(), animated)
            .into_iter()
            .find(|id| d.brush(*id).is_some())
            .unwrap();
        let mut clip = crate::animation_curves::Clip {
            version: 1,
            duration: 2.,
            fps: 30,
            tracks: std::array::from_fn(|_| vec![]),
        };
        clip.insert_position(0., [0., 0., 0.]).unwrap();
        clip.insert_position(1., [64., 0., 0.]).unwrap();
        clip.insert_position(2., [64., 64., 0.]).unwrap();
        let library = Library {
            version: 1,
            clips: std::collections::BTreeMap::from([(
                "route".into(),
                crate::animation_library::Entry {
                    clip,
                    mode: "once".into(),
                },
            )]),
        };
        library.write(&mut d, animated, "route");
        d.set_entity_property(animated, "start_on", "0");
        d.add_brushes(vec![b]);
        let source = d.create_brush_entity("trigger_once").unwrap();
        (d, animated, source, brush)
    }
    #[test]
    fn trigger_settings_validate_atomically_and_undo() {
        let (mut d, target, source, _) = fixture();
        let before = d.to_map_text();
        let mut c = Config::read(&d, source).unwrap();
        assert!(c.once);
        c.action = "play".into();
        c.clip = "route".into();
        c.wait = 0.25;
        c.once = false;
        configure(&mut d, source, c.clone(), Some(target), true).unwrap();
        assert_eq!(before, d.to_map_text());
        let mut bad = c.clone();
        bad.clip = "missing".into();
        assert!(configure(&mut d, source, bad, Some(target), false).is_err());
        assert_eq!(before, d.to_map_text());
        configure(&mut d, source, c, Some(target), false).unwrap();
        let saved = d.to_map_text();
        assert!(saved.contains("animation_action"));
        let c = Config::read(&d, source).unwrap();
        assert_eq!(c.targets(&d).unwrap(), vec![target]);
        assert!(!c.once);
        assert_eq!(c.wait, 0.25);
        d.undo();
        assert_eq!(before, d.to_map_text());
        d.redo();
        assert_eq!(saved, d.to_map_text());
        d.set_entity_property(target, "targetname", "renamed");
        assert!(
            Config::read(&d, source)
                .unwrap()
                .targets(&d)
                .unwrap_err()
                .contains("Missing target")
        );
    }
    #[test]
    fn preview_play_pause_resume_stop_and_once_never_edit_the_map() {
        let (mut d, target, source, brush) = fixture();
        let mut c = Config::read(&d, source).unwrap();
        c.action = "play".into();
        c.clip = "route".into();
        c.once = false;
        configure(&mut d, source, c, Some(target), false).unwrap();
        let path =
            std::env::temp_dir().join(format!("heron-trigger-preview-{}.map", std::process::id()));
        d.save_as(&path).unwrap();
        let original = d.to_map_text();
        let modified = d.is_modified();
        assert!(!modified);
        let filters = crate::view_filters::ViewFilters {
            animation_frame: Some(15),
            animation_fps: 30,
            ..Default::default()
        };
        let timeline_edges = d.animation_wireframe(&filters).plain.clone();
        let timeline_mesh = d.preview_mesh(&filters).at_frame(0.5);
        d.preview_trigger_at(source, 10.).unwrap();
        assert_eq!(
            d.animation_wireframe(&filters).plain,
            timeline_edges,
            "timeline must ignore trigger playback time"
        );
        assert_eq!(
            d.preview_mesh(&filters)
                .at_frame(0.5)
                .iter()
                .map(|t| t.positions)
                .collect::<Vec<_>>(),
            timeline_mesh
                .iter()
                .map(|t| t.positions)
                .collect::<Vec<_>>()
        );
        let m = d.preview_motions().remove(&brush).unwrap();
        assert_eq!(
            m.transform(Vec3d::new(0., 0., 0.), 10.5, false),
            Vec3d::new(32., 0., 0.)
        );
        assert_eq!(
            m.transform(Vec3d::new(0., 0., 0.), 11.5, false),
            Vec3d::new(64., 32., 0.)
        );
        assert_eq!(original, d.to_map_text());
        assert_eq!(modified, d.is_modified());
        std::fs::remove_file(path).unwrap();
        d.set_entity_property(source, "animation_action", "pause");
        d.preview_trigger_at(source, 10.5).unwrap();
        assert_eq!(
            d.preview_motions()[&brush].transform(Vec3d::new(0., 0., 0.), 30., false),
            Vec3d::new(32., 0., 0.)
        );
        d.set_entity_property(source, "animation_action", "resume");
        d.preview_trigger_at(source, 30.).unwrap();
        assert_eq!(
            d.preview_motions()[&brush].transform(Vec3d::new(0., 0., 0.), 31., false),
            Vec3d::new(64., 32., 0.)
        );
        d.set_entity_property(source, "animation_action", "stop");
        d.preview_trigger_at(source, 32.).unwrap();
        assert_eq!(
            d.preview_motions()[&brush].transform(Vec3d::new(0., 0., 0.), 40., false),
            Vec3d::new(0., 0., 0.)
        );
        d.reset_trigger_preview();
        d.set_entity_property(source, "animation_once", "1");
        d.set_entity_property(source, "animation_action", "play");
        d.preview_trigger_at(source, 50.).unwrap();
        assert!(d.preview_trigger_at(source, 55.).is_err());
        d.reset_trigger_preview();
        d.preview_trigger_at(source, 55.).unwrap();
        let saved = d.to_map_text();
        d.reset_trigger_preview();
        assert_eq!(saved, d.to_map_text());
        d.set_entity_property(source, "animation_once", "0");
        d.set_entity_property(source, "wait", "2");
        d.preview_trigger_at(source, 60.).unwrap();
        assert!(d.preview_trigger_at(source, 61.).is_err());
        d.preview_trigger_at(source, 62.).unwrap();
    }
}
