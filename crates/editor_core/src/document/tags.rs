//! Document operations for tags.

use super::{Document, HashSet, NodeId, TagFlagField, TagOutcome, flags_match, node};

impl Document {
    /// Takes the tags the game configuration lists.
    pub fn set_tags(&mut self, brush: Vec<crate::games::Tag>, face: Vec<crate::games::Tag>) {
        self.brush_tags = brush;
        self.face_tags = face;
        self.hidden_tags
            .retain(|name| self.brush_tags.iter().chain(&self.face_tags).any(|tag| &tag.name == name));
        self.mark_changed();
    }

    /// The tags in force: the brushes' and the faces'.
    pub fn tags(&self) -> (&[crate::games::Tag], &[crate::games::Tag]) {
        (&self.brush_tags, &self.face_tags)
    }

    /// Takes the game configuration's `faceattribs` flag names -- entry `i` names bit
    /// `1 << i`, an empty string an unused bit.
    pub fn set_face_flag_names(&mut self, surface: Vec<String>, content: Vec<String>) {
        self.surface_flag_names = surface;
        self.content_flag_names = content;
    }

    /// The surface-flag names the face attribute editor labels its ticks with.
    pub fn surface_flag_names(&self) -> &[String] {
        &self.surface_flag_names
    }

    /// The content-flag names.
    pub fn content_flag_names(&self) -> &[String] {
        &self.content_flag_names
    }

    /// Whether a tag's brushes and faces are drawn.
    pub fn tag_is_hidden(&self, name: &str) -> bool {
        self.hidden_tags.contains(name)
    }

    /// Hides or shows everything carrying one tag.
    pub fn set_tag_hidden(&mut self, name: &str, hidden: bool) {
        let changed = if hidden {
            self.hidden_tags.insert(name.to_string())
        } else {
            self.hidden_tags.remove(name)
        };
        if changed {
            // The flat views draw edges gathered ahead of time, so what is drawn changes
            // only when they are gathered again.
            self.mark_changed();
        }
    }

    /// The materials in the loaded set whose name matches `tag`'s glob, sorted, no repeats.
    fn materials_matching_tag(&self, tag: &crate::games::Tag) -> Vec<String> {
        let mut names: Vec<String> = self
            .materials
            .materials()
            .iter()
            .map(|material| material.name.clone())
            .filter(|name| tag.matches_name(name))
            .collect();
        names.sort_unstable_by_key(|name| name.to_ascii_lowercase());
        names.dedup();
        names
    }

    /// The bit(s) a flag tag names, if it is one and the map format carries flags.
    fn tag_flag_bits(&self, tag: &crate::games::Tag) -> Option<(TagFlagField, i32)> {
        let field = match tag.rule {
            crate::games::TagRule::SurfaceFlag => TagFlagField::Surface,
            crate::games::TagRule::ContentFlag => TagFlagField::Content,
            _ => return None,
        };
        if !self.format.has_surface_attributes() {
            return None;
        }
        let bits = tag.pattern.trim().parse::<i32>().ok().filter(|bits| *bits != 0)?;
        Some((field, bits))
    }

    /// Whether `Turn selection into <tag>` can act -- `SmartTag::canEnable`. A material tag
    /// can when the loaded set holds a matching material; a flag tag can when the format
    /// carries flags.
    pub fn can_enable_tag(&self, tag: &crate::games::Tag) -> bool {
        match tag.rule {
            crate::games::TagRule::Material => !self.materials_matching_tag(tag).is_empty(),
            crate::games::TagRule::SurfaceFlag | crate::games::TagRule::ContentFlag => {
                self.tag_flag_bits(tag).is_some()
            }
            _ => false,
        }
    }

    /// Whether `Turn selection off <tag>` can act -- `SmartTag::canDisable`. Only a flag
    /// tag: a material tag has no way back, there being no telling what to wear instead.
    pub fn can_disable_tag(&self, tag: &crate::games::Tag) -> bool {
        self.tag_flag_bits(tag).is_some()
    }

    /// Turns a tag on for the selected faces, or every face of the selected brushes --
    /// `SmartTag::enable`. A material tag puts its material on them ("Turn Face Into
    /// Clip"); a flag tag sets its bit in their surface or content flags.
    pub fn enable_tag(&mut self, tag: &crate::games::Tag) -> TagOutcome {
        if !self.has_face_selection() && self.selection.is_empty() {
            return TagOutcome::NoSelection;
        }

        match tag.rule {
            crate::games::TagRule::Material => {
                let matches = self.materials_matching_tag(tag);
                let Some(material) = matches.first().cloned() else {
                    return TagOutcome::NoMaterial;
                };
                let mut changed = 0;
                self.edit_faces(|face| {
                    if face.material_name != material {
                        face.material_name = material.clone().into();
                        changed += 1;
                    }
                });
                if matches.len() > 1 {
                    TagOutcome::Ambiguous { used: material, changed }
                } else {
                    TagOutcome::Changed(changed)
                }
            }
            crate::games::TagRule::SurfaceFlag | crate::games::TagRule::ContentFlag => {
                let Some((field, bits)) = self.tag_flag_bits(tag) else {
                    return TagOutcome::Unsupported;
                };
                TagOutcome::Changed(self.change_face_flags(field, bits, true))
            }
            _ => TagOutcome::Unsupported,
        }
    }

    /// Turns a flag tag off for the selection -- `FlagsTagMatcher::disable`. Clears its
    /// bit. A material tag cannot be turned off this way.
    pub fn disable_tag(&mut self, tag: &crate::games::Tag) -> TagOutcome {
        if !self.has_face_selection() && self.selection.is_empty() {
            return TagOutcome::NoSelection;
        }
        let Some((field, bits)) = self.tag_flag_bits(tag) else {
            return TagOutcome::Unsupported;
        };
        TagOutcome::Changed(self.change_face_flags(field, bits, false))
    }

    /// Sets or clears `bits` in the given flag field of every selected face. Returns how
    /// many faces changed.
    fn change_face_flags(&mut self, field: TagFlagField, bits: i32, set: bool) -> usize {
        let mut changed = 0;
        self.edit_faces(|face| {
            let mut attributes = face.surface_attributes.unwrap_or_default();
            let slot = match field {
                TagFlagField::Surface => &mut attributes.surface_flags,
                TagFlagField::Content => &mut attributes.content_flags,
            };
            let updated = if set { *slot | bits } else { *slot & !bits };
            if updated == *slot {
                return;
            }
            *slot = updated;
            face.surface_attributes = Some(attributes);
            changed += 1;
        });
        changed
    }

    /// The brushes a hidden tag takes out of what is drawn.
    ///
    /// Worked out in one pass rather than per brush: a brush tag is matched against the
    /// entity a brush belongs to, and finding that entity means walking the tree, which is
    /// not something to do once per brush of a large map.
    pub(super) fn brushes_hidden_by_tags(&self) -> HashSet<NodeId> {
        let hidden: Vec<&crate::games::Tag> = self
            .brush_tags
            .iter()
            .filter(|tag| self.hidden_tags.contains(&tag.name))
            .collect();
        if hidden.is_empty() {
            return HashSet::new();
        }

        let mut out = HashSet::new();

        // The tags that go by the class of the entity a brush is in.
        let by_class: Vec<&&crate::games::Tag> = hidden
            .iter()
            .filter(|tag| tag.rule == crate::games::TagRule::Classname)
            .collect();
        if !by_class.is_empty() {
            for (id, entity) in node::entities_with_ids(&self.tree.world) {
                let Some(classname) = entity.classname() else {
                    continue;
                };
                if !by_class.iter().any(|tag| tag.matches_name(classname)) {
                    continue;
                }
                out.extend(
                    node::contents_of(&self.tree.world, id)
                        .into_iter()
                        .filter(|member| self.brush(*member).is_some()),
                );
            }
        }

        // The tags that go by a flag on one of the brush's faces.
        let by_flag: Vec<&&crate::games::Tag> = hidden
            .iter()
            .filter(|tag| tag.rule == crate::games::TagRule::ContentFlag)
            .collect();
        if !by_flag.is_empty() {
            for (id, brush) in node::brushes_with_ids(&self.tree.world) {
                if brush
                    .faces()
                    .iter()
                    .any(|face| by_flag.iter().any(|tag| flags_match(tag, face)))
                {
                    out.insert(id);
                }
            }
        }

        out
    }

    /// Whether the shader the face's material came from carries a `surfaceparm` the tag
    /// names, `SurfaceParmTagMatcher::matchesMaterial`.
    ///
    /// A material that is a plain image has no parms and so carries no such tag; only a
    /// material that came from a `.shader` can.
    fn face_has_surface_parm(
        &self,
        face: &map_model::brush_face::BrushFace,
        tag: &crate::games::Tag,
    ) -> bool {
        self.materials
            .get(&face.material_name)
            .is_some_and(|material| {
                material.surface_parms.iter().any(|parm| tag.matches_name(parm))
            })
    }

    /// Whether a face is hidden by one of the tags it carries.
    pub(super) fn face_hidden_by_tag(&self, face: &map_model::brush_face::BrushFace) -> bool {
        if self.hidden_tags.is_empty() {
            return false;
        }
        self.face_tags
            .iter()
            .filter(|tag| self.hidden_tags.contains(&tag.name))
            .any(|tag| match tag.rule {
                crate::games::TagRule::Material => tag.matches_name(&face.material_name),
                crate::games::TagRule::ContentFlag | crate::games::TagRule::SurfaceFlag => {
                    flags_match(tag, face)
                }
                crate::games::TagRule::SurfaceParm => self.face_has_surface_parm(face, tag),
                _ => false,
            })
    }
}
