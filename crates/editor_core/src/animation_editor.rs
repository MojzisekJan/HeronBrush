//! A checked animation draft for frontends. Geometry edits never stand in for pose edits.
use crate::{animation_curves::Clip, document::Document, keyframe_animation::Track};
use map_model::node::{self, NodeId};
use std::collections::HashSet;

pub struct Draft {
    pub entity: Option<NodeId>,
    pub clip: Clip,
    pub mode: String,
    pub pivot: [f64; 3],
    revision: u64,
    selection: HashSet<NodeId>,
}
impl Draft {
    pub fn read(document: &Document) -> Result<Self, String> {
        let selected = document.selection_ids();
        let bounds = document
            .selection_bounds()
            .ok_or(crate::i18n::translation_key("Select brushes to animate"))?;
        let entities = node::entities_with_ids(document.tree_world());
        let entity = entities.iter().find(|(id, entity)| {
            entity.is_brush_entity()
                && !selected.is_empty()
                && selected
                    .iter()
                    .all(|s| *s == *id || node::contents_of(document.tree_world(), *id).contains(s))
        });
        if entity.is_none()
            && entities.iter().any(|(id, e)| {
                e.is_brush_entity()
                    && selected
                        .iter()
                        .any(|s| node::contents_of(document.tree_world(), *id).contains(s))
            })
        {
            return Err(crate::i18n::translation_key("Select brushes from one entity.").into());
        }
        let mut mode = "loop".to_owned();
        let mut pivot = bounds.center().v;
        let clip = if let Some((_, e)) = entity {
            mode = e.property("animation_mode").unwrap_or("loop").to_owned();
            if let Some(text) = e.property("animation_pivot") {
                let values = text
                    .split_whitespace()
                    .map(str::parse::<f64>)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| crate::i18n::translation_key("Invalid animation pivot"))?;
                if values.len() != 3 {
                    return Err(crate::i18n::translation_key("Invalid animation pivot").into());
                }
                pivot = [values[0], values[1], values[2]];
            }
            if let Some(curves) = e.property("animation_curves").filter(|s| !s.is_empty()) {
                Clip::parse(curves)?
            } else if let Some(keys) = e.property("animation_keys").filter(|s| !s.is_empty()) {
                Clip::from_legacy(&Track::parse(keys, &mode)?)
            } else {
                empty_clip()
            }
        } else {
            empty_clip()
        };
        Ok(Self {
            entity: entity.map(|(id, _)| *id),
            clip,
            mode,
            pivot,
            revision: document.revision(),
            selection: selected.clone(),
        })
    }
    pub fn install_preset(&mut self, document: &mut Document, door: bool) -> Result<(), String> {
        use crate::animation_curves::Interpolation;
        use crate::animation_library::{Entry, Library};
        if !self.is_current(document) || document.in_transaction() {
            return Err(crate::i18n::translation_key(
                "Finish editing and select the animation again",
            )
            .into());
        }
        let mut clip = empty_clip();
        clip.duration = 1.0;
        let channel = if door { 5 } else { 0 };
        clip.insert(channel, 0.0, 0.0)?;
        clip.insert(channel, 1.0, if door { 90.0 } else { 128.0 })?;
        for key in &mut clip.tracks[channel] {
            key.interpolation = Interpolation::Bezier;
        }
        let mut library = if let Some(id) = self.entity {
            Library::read(document, id)?.0
        } else {
            Library::parse("")?
        };
        let name = if door {
            library.clips.extend(Library::door(clip).clips);
            "open"
        } else {
            library.clips.insert(
                "translate".into(),
                Entry {
                    clip,
                    mode: "pingpong".into(),
                },
            );
            "translate"
        };
        library.validate()?;
        document.run_as_one_edit(|document| -> Result<(), String> {
            self.commit(document)?;
            let id = self
                .entity
                .ok_or(crate::i18n::translation_key("Select brushes to animate"))?;
            library.write(document, id, name);
            document.set_entity_property(id, "start_on", "0");
            Ok(())
        })?;
        *self = Self::read(document)?;
        Ok(())
    }
    /// Whether the document still selects what this draft was read from; a newer
    /// revision with the same selection is a reload, not a different animation.
    pub fn same_selection(&self, document: &Document) -> bool {
        &self.selection == document.selection_ids()
    }
    fn property(&self, document: &Document, key: &str) -> Option<String> {
        let id = self.entity?;
        node::entities_with_ids(document.tree_world())
            .into_iter()
            .find(|(n, _)| *n == id)
            .and_then(|(_, e)| e.property(key).map(str::to_owned))
    }
    /// The name of the clip being edited, as the library calls it.
    pub fn name(&self, document: &Document) -> String {
        self.property(document, "animation_name")
            .unwrap_or_else(|| "default".into())
    }
    /// Whether no animation has been written yet: no entity, or one without keys.
    pub fn blank(&self, document: &Document) -> bool {
        ["animation_keys", "animation_curves"]
            .iter()
            .all(|key| self.property(document, key).is_none_or(|v| v.is_empty()))
    }
    pub fn is_current(&self, document: &Document) -> bool {
        self.revision == document.revision() && &self.selection == document.selection_ids()
    }
    pub fn commit(&mut self, document: &mut Document) -> Result<(), String> {
        if !self.is_current(document) {
            return Err(crate::i18n::translation_key(
                "Selection or animation changed. Reload the animation.",
            )
            .into());
        }
        if document.in_transaction() {
            return Err(crate::i18n::translation_key("Finish the current drag first").into());
        }
        self.clip.validate()?;
        if !["loop", "once", "pingpong"].contains(&self.mode.as_str()) {
            return Err(crate::i18n::translation_key("Invalid animation mode").into());
        }
        crate::animation_pose::Pose::validate_pivot(geometry_math::vec::Vec3d { v: self.pivot })?;
        let mut entity = self.entity;
        document.run_as_one_edit(|d| {
            if entity.is_none() {
                entity = d.create_brush_entity("func_animate");
            }
            if let Some(id) = entity {
                d.set_entity_property(id, "animation_curves", &self.clip.json());
                d.set_entity_property(id, "animation_keys", &self.clip.legacy_text());
                d.set_entity_property(id, "animation_mode", &self.mode);
                d.set_entity_property(
                    id,
                    "animation_pivot",
                    &self.pivot.map(|v| v.to_string()).join(" "),
                );
            }
        });
        self.entity =
            Some(entity.ok_or(crate::i18n::translation_key("Select brushes to animate"))?);
        self.revision = document.revision();
        self.selection = document.selection_ids().clone();
        Ok(())
    }
}
fn empty_clip() -> Clip {
    let mut clip = Clip {
        version: 1,
        duration: 2.0,
        fps: 30,
        tracks: std::array::from_fn(|_| Vec::new()),
    };
    for axis in 0..6 {
        clip.insert(axis, 0.0, 0.0).unwrap();
    }
    clip
}
#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Document {
        let mut d = Document::open(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"),
            Default::default(),
        )
        .unwrap();
        d.select_all();
        d
    }
    #[test]
    fn creating_an_animation_is_one_undo_and_preserves_geometry() {
        let mut d = document();
        let before: std::collections::HashMap<_, _> = node::brushes_with_ids(d.tree_world())
            .into_iter()
            .map(|(id, b)| (id, b.faces().to_vec()))
            .collect();
        let bounds = d.selection_bounds();
        let mut draft = Draft::read(&d).unwrap();
        draft.clip.insert_position(1.0, [64.0, 0.0, 0.0]).unwrap();
        draft.commit(&mut d).unwrap();
        assert_eq!(d.selection_bounds(), bounds);
        assert!(Draft::read(&d).unwrap().entity.is_some());
        d.undo();
        assert!(node::entities_with_ids(d.tree_world())
            .iter()
            .all(|(_, e)| e.classname() != Some("func_animate")));
        let after: std::collections::HashMap<_, _> = node::brushes_with_ids(d.tree_world())
            .into_iter()
            .map(|(id, b)| (id, b.faces().to_vec()))
            .collect();
        assert_eq!(after, before);
    }
    #[test]
    fn stale_and_invalid_drafts_cannot_overwrite_edits() {
        let mut d = document();
        let mut draft = Draft::read(&d).unwrap();
        d.clear_selection();
        let before = d.to_map_text();
        assert!(draft.commit(&mut d).is_err());
        assert_eq!(d.to_map_text(), before);
        d.select_all();
        let mut draft = Draft::read(&d).unwrap();
        draft.clip.duration = -1.0;
        assert!(draft.commit(&mut d).is_err());
        assert_eq!(d.to_map_text(), before);
    }
    #[test]
    fn door_preset_creates_reversed_clips_in_one_undo() {
        let mut d = document();
        let count = node::brushes(d.tree_world()).len();
        let mut draft = Draft::read(&d).unwrap();
        draft.install_preset(&mut d, true).unwrap();
        assert!(draft.is_current(&d));
        let (library, current) =
            crate::animation_library::Library::read(&d, draft.entity.unwrap()).unwrap();
        assert_eq!(current, "open");
        assert_eq!(library.clips["open"].clip.sample(5, 1.0), 90.0);
        assert_eq!(library.clips["close"].clip.sample(5, 0.0), 90.0);
        assert_eq!(library.clips["close"].clip.sample(5, 1.0), 0.0);
        d.undo();
        assert_eq!(node::brushes(d.tree_world()).len(), count);
        assert!(node::entities_with_ids(d.tree_world())
            .iter()
            .all(|(_, e)| e.classname() != Some("func_animate")));
    }
}
