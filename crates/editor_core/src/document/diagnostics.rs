//! Explicit corrections of findings, scoped to their object and one Undo step.
use super::{brush_node, node, Document, Edit, Node, NodeKind};
use crate::validation::Issue;
impl Document {
    pub fn issue_fix_label(&self, issue: &Issue) -> Option<&'static str> {
        let id = issue.node?;
        if !self.is_editable(id) {
            return None;
        }
        match issue.kind {
            "invalid UV scale" if self.brush(id).is_some() => {
                Some("Repair invalid UV values in this brush")
            }
            "empty property key"
                if node::find_node(&self.tree.world, id)
                    .is_some_and(|n| matches!(n.kind, NodeKind::Entity(_))) =>
            {
                Some("Remove the property with an empty name")
            }
            _ => None,
        }
    }
    pub fn fix_issue(&mut self, issue: &Issue) -> Result<bool, String> {
        if self.issue_fix_label(issue).is_none() {
            return Err("No correction is available for this finding".into());
        }
        if !crate::validation::validate(self).contains(issue) {
            return Err("This finding changed; check the map again".into());
        }
        let id = issue.node.unwrap();
        match issue.kind {
            "invalid UV scale" => {
                let brush = self.brush(id).cloned().ok_or("Brush no longer exists")?;
                let mut faces = brush.faces().to_vec();
                for face in &mut faces {
                    let uv = &mut face.uv_attributes;
                    for axis in 0..2 {
                        if !uv.scale[axis].is_finite() || uv.scale[axis].abs() < 1e-6 {
                            uv.scale[axis] = 1.0;
                        }
                        if !uv.offset[axis].is_finite() {
                            uv.offset[axis] = 0.0;
                        }
                    }
                    if !uv.rotation.is_finite() {
                        uv.rotation = 0.0;
                    }
                }
                let changed = map_model::brush::Brush::from_faces(&self.world_bounds, faces)
                    .map_err(|e| e.to_string())?;
                self.apply_and_record(Edit::Changed {
                    before: vec![brush_node(id, brush)],
                    after: vec![brush_node(id, changed)],
                });
                Ok(true)
            }
            "empty property key" => {
                let before = node::find_node(&self.tree.world, id)
                    .cloned()
                    .ok_or("Entity no longer exists")?;
                let NodeKind::Entity(mut entity) = before.kind.clone() else {
                    return Ok(false);
                };
                entity.properties.retain(|(key, _)| !key.is_empty());
                self.apply_and_record(Edit::Changed {
                    before: vec![before],
                    after: vec![Node {
                        id,
                        kind: NodeKind::Entity(entity),
                    }],
                });
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    #[test]
    fn repair_is_scoped_preserves_selection_and_is_one_undo() {
        let mut doc = Document::new(WorldBounds::default());
        let cube = map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::from_scalars(0., 32.),
            "stone",
        )
        .unwrap();
        let mut faces = cube.faces().to_vec();
        faces[0].uv_attributes.scale[0] = 0.;
        let bad = map_model::brush::Brush::from_faces(&doc.world_bounds(), faces).unwrap();
        doc.add_brushes(vec![bad, cube]);
        let ids = doc
            .selectable_brushes()
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        doc.select_only(ids[1]);
        let before = doc.to_map_text();
        let selected: std::collections::HashSet<_> = doc.selection_ids().iter().copied().collect();
        let issue = crate::validation::validate(&doc)
            .into_iter()
            .find(|issue| issue.kind == "invalid UV scale")
            .unwrap();
        assert!(doc.fix_issue(&issue).unwrap());
        assert_eq!(doc.selection_ids(), &selected);
        assert!(!crate::validation::validate(&doc)
            .iter()
            .any(|i| i.kind == "invalid UV scale"));
        assert!(doc.fix_issue(&issue).is_err());
        doc.undo();
        assert_eq!(doc.to_map_text(), before);
    }
    #[test]
    fn removing_empty_key_keeps_other_properties_and_undo_restores_it() {
        let mut doc = Document::new(WorldBounds::default());
        let id = doc.create_point_entity("info_player_start", geometry_math::vec::Vec3d::zero());
        doc.set_entity_property_on(&[id], "", "bad");
        doc.set_entity_property_on(&[id], "targetname", "keep");
        let before = doc.to_map_text();
        let issue = crate::validation::validate(&doc)
            .into_iter()
            .find(|i| i.kind == "empty property key")
            .unwrap();
        doc.fix_issue(&issue).unwrap();
        assert!(doc.to_map_text().contains("keep"));
        assert!(!crate::validation::validate(&doc)
            .iter()
            .any(|i| i.kind == "empty property key"));
        doc.undo();
        assert_eq!(doc.to_map_text(), before);
    }
}
