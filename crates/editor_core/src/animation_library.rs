//! Named clips. The current clip stays in the legacy properties so all editing
//! tools and older maps keep working; inactive clips are snapshots in the library.
use crate::{animation_curves::Clip, document::Document, keyframe_animation::Track};
use map_model::node::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub clip: Clip,
    pub mode: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    pub clips: BTreeMap<String, Entry>,
}
pub fn valid_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 64
        || name == "RESET"
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(crate::i18n::translation_key("Animation names use 1–64 letters A–Z, digits, underscores or hyphens; RESET is reserved.").into());
    }
    Ok(())
}
impl Library {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 8_000_000 {
            return Err(crate::i18n::translation_key("Animation library is too large").into());
        }
        let library: Self = if text.is_empty() {
            Self {
                version: 1,
                clips: BTreeMap::new(),
            }
        } else {
            serde_json::from_str(text).map_err(|e| e.to_string())?
        };
        library.validate()?;
        Ok(library)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.clips.len() > 32 {
            return Err(crate::i18n::translation_key(
                "Use at most 32 named animations (library version 1)",
            )
            .into());
        }
        for (name, entry) in &self.clips {
            valid_name(name)?;
            entry.clip.validate()?;
            if !["once", "loop", "pingpong"].contains(&entry.mode.as_str()) {
                return Err(crate::i18n::translation_key("Invalid animation mode").into());
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 8_000_000 {
            return Err(crate::i18n::translation_key("Animation library is too large").into());
        }
        Ok(())
    }
    pub fn read(d: &Document, id: NodeId) -> Result<(Self, String), String> {
        let (_, e) = map_model::node::entities_with_ids(d.tree_world())
            .into_iter()
            .find(|(n, _)| *n == id)
            .ok_or(crate::i18n::translation_key("Entity no longer exists"))?;
        let mut library = Self::parse(e.property("animation_library").unwrap_or(""))?;
        let name = e
            .property("animation_name")
            .filter(|s| !s.is_empty())
            .unwrap_or("default")
            .to_string();
        valid_name(&name)?;
        let mode = e.property("animation_mode").unwrap_or("loop").to_string();
        let track = Track::from_properties(
            e.property("animation_keys")
                .unwrap_or("0 0 0 0 0 0 0; 2 0 0 0 0 0 0"),
            &mode,
            e.property("animation_curves"),
        )?;
        let clip = track
            .curves
            .clone()
            .unwrap_or_else(|| Clip::from_legacy(&track));
        library.clips.insert(name.clone(), Entry { clip, mode });
        library.validate()?;
        Ok((library, name))
    }
    pub fn write(&self, d: &mut Document, id: NodeId, name: &str) {
        let entry = &self.clips[name];
        d.set_entity_property(
            id,
            "animation_library",
            &serde_json::to_string(self).unwrap(),
        );
        d.set_entity_property(id, "animation_name", name);
        d.set_entity_property(id, "animation_curves", &entry.clip.json());
        d.set_entity_property(id, "animation_keys", &entry.clip.legacy_text());
        d.set_entity_property(id, "animation_mode", &entry.mode);
    }
    pub fn edit(&mut self, current: &str, action: &str, name: &str) -> Result<String, String> {
        valid_name(name)?;
        let mut next = self.clone();
        let selected = match action {
            "select" => {
                if !next.clips.contains_key(name) {
                    return Err(crate::i18n::translation_key("Animation does not exist").into());
                }
                name
            }
            "save_as" => {
                if next.clips.contains_key(name) {
                    return Err(
                        crate::i18n::translation_key("Animation name already exists").into(),
                    );
                }
                next.clips.insert(name.into(), next.clips[current].clone());
                name
            }
            "delete" => {
                if next.clips.len() < 2 {
                    return Err(crate::i18n::translation_key("Keep at least one animation").into());
                }
                if next.clips.remove(name).is_none() {
                    return Err(crate::i18n::translation_key("Animation does not exist").into());
                }
                if current == name {
                    next.clips.keys().next().unwrap()
                } else {
                    current
                }
            }
            _ => {
                return Err(
                    crate::i18n::translation_key("Unknown animation library operation").into(),
                )
            }
        }
        .to_string();
        next.validate()?;
        *self = next;
        Ok(selected)
    }
    pub fn door(clip: Clip) -> Self {
        // The door preset uses a smooth curve, so reversing has no discontinuities.
        let mut close = clip.clone();
        for keys in &mut close.tracks {
            let original = keys.clone();
            *keys = original
                .iter()
                .rev()
                .map(|k| {
                    let mut n = k.clone();
                    n.time = clip.duration - k.time;
                    n.incoming = -k.outgoing;
                    n.outgoing = -k.incoming;
                    n
                })
                .collect();
        }
        Self {
            version: 1,
            clips: BTreeMap::from([
                (
                    "open".into(),
                    Entry {
                        clip,
                        mode: "once".into(),
                    },
                ),
                (
                    "close".into(),
                    Entry {
                        clip: close,
                        mode: "once".into(),
                    },
                ),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    #[test]
    fn named_clips_switch_without_losing_edits_and_undo() {
        let mut d = Document::new(crate::world::WorldBounds::default());
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.)),
            "stone",
        )
        .unwrap()]);
        d.select_all();
        let id = d.create_brush_entity("func_animate").unwrap();
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 1 32 0 0 0 0 0");
        let (mut lib, current) = Library::read(&d, id).unwrap();
        let chosen = lib.edit(&current, "save_as", "open").unwrap();
        d.run_as_one_edit(|d| lib.write(d, id, &chosen));
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 1 64 0 0 0 0 0");
        d.remove_entity_property_on(&[id], "animation_curves");
        let before = d.to_map_text();
        let (mut lib, current) = Library::read(&d, id).unwrap();
        let chosen = lib.edit(&current, "select", "default").unwrap();
        d.run_as_one_edit(|d| lib.write(d, id, &chosen));
        assert_eq!(
            Library::read(&d, id).unwrap().0.clips["open"]
                .clip
                .sample(0, 1.),
            64.
        );
        assert_eq!(
            Library::read(&d, id).unwrap().0.clips["default"]
                .clip
                .sample(0, 1.),
            32.
        );
        d.undo();
        assert_eq!(d.to_map_text(), before);
        d.redo();
        let serialized = serde_json::to_string(&lib).unwrap();
        assert!(Library::parse(&serialized).is_ok());
        assert!(lib.edit("default", "save_as", "open").is_err());
        assert!(lib.edit("default", "save_as", "../bad").is_err());
    }
}
