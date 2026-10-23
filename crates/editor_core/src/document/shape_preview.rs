//! A newly drawn shape stays configurable until selection or another edit changes.
use super::*;
use crate::shape_tool::{self, ShapeParameters};

pub(super) struct ShapePreview {
    parameters: ShapeParameters,
    bounds: Bbox3d,
    material: String,
    layer: usize,
    pub(super) nodes: Vec<Node>,
    pub(super) revision: u64,
}
impl Document {
    /// Draw a shape and keep its generator attached to this selection temporarily.
    /// The valid current geometry is saved/exported normally, including autosave.
    pub fn begin_shape_preview(
        &mut self,
        bounds: Bbox3d,
        parameters: ShapeParameters,
    ) -> Result<usize, String> {
        if self.in_transaction() {
            return Err("Finish the current drag first.".into());
        }
        let material = map_model::shapes::DEFAULT_MATERIAL.to_string();
        let brushes =
            shape_tool::create_brushes(&self.world_bounds(), &bounds, &parameters, &material)
                .map_err(|e| e.to_string())?;
        let brushes = self.with_default_uv_scale(brushes);
        if brushes.is_empty() {
            return Err("The shape produced no brushes.".into());
        }
        self.shape_preview = None;
        self.face_selection.clear();
        let layer = self.current_layer();
        self.add_brushes(brushes);
        let mut nodes: Vec<_> = self
            .selection
            .iter()
            .filter_map(|id| node::find_node(self.tree_world(), *id).cloned())
            .collect();
        // History stores the generator order, while selection is unordered.
        nodes.sort_by_key(|n| n.id);
        // Monotonic IDs preserve the insertion history order.
        let count = nodes.len();
        self.shape_preview = Some(ShapePreview {
            parameters,
            bounds,
            material,
            layer,
            nodes,
            revision: self.revision(),
        });
        Ok(count)
    }
    /// Whether the current selection still has editable generator parameters.
    pub fn has_shape_preview(&self) -> bool {
        self.shape_preview
            .as_ref()
            .is_some_and(|s| s.revision == self.revision())
    }
    /// Parameters last used successfully, for restoring controls after invalid input.
    pub fn shape_preview_parameters(&self) -> Option<ShapeParameters> {
        self.shape_preview.as_ref().map(|s| s.parameters)
    }
    /// Accept the current result. It remains selected as ordinary brushes.
    pub fn finish_shape_preview(&mut self) -> bool {
        if self.shape_preview.take().is_none() {
            return false;
        }
        self.mark_selection_changed();
        true
    }
    /// Regenerate from the original bounds, replacing the latest insertion in place.
    pub fn update_shape_preview(&mut self, parameters: ShapeParameters) -> Result<bool, String> {
        let Some(mut session) = self.shape_preview.take() else {
            return Ok(false);
        };
        if session.revision != self.revision() {
            return Ok(false);
        }
        if self.in_transaction() {
            self.shape_preview = Some(session);
            return Ok(false);
        }
        if session.parameters == parameters {
            self.shape_preview = Some(session);
            return Ok(false);
        }
        let brushes = match shape_tool::create_brushes(
            &self.world_bounds(),
            &session.bounds,
            &parameters,
            &session.material,
        ) {
            Ok(brushes) if !brushes.is_empty() => self.with_default_uv_scale(brushes),
            Ok(_) => {
                self.shape_preview = Some(session);
                return Err("The shape produced no brushes.".into());
            }
            Err(error) => {
                self.shape_preview = Some(session);
                return Err(error.to_string());
            }
        };
        let nodes: Vec<_> = brushes
            .into_iter()
            .enumerate()
            .map(|(i, brush)| match session.nodes.get(i) {
                Some(old) => Node::with_id(old.id, NodeKind::Brush(brush)),
                None => Node::new(NodeKind::Brush(brush)),
            })
            .collect();
        let ids: Vec<_> = session.nodes.iter().map(|n| n.id).collect();
        if !self.history.amend_added(session.layer, &ids, nodes.clone()) {
            return Ok(false);
        }
        let edit = Edit::Batch(vec![
            Edit::Removed {
                layer: session.layer,
                nodes: session.nodes,
            },
            Edit::Added {
                layer: session.layer,
                nodes: nodes.clone(),
            },
        ]);
        self.apply(&edit);
        self.selection = nodes.iter().map(|n| n.id).collect();
        self.face_selection.clear();
        self.mark_selection_changed();
        if let Some(listener) = self.on_edit.clone() {
            listener(&edit);
        }
        session.nodes = nodes;
        session.parameters = parameters;
        session.revision = self.revision();
        self.shape_preview = Some(session);
        Ok(true)
    }
    /// Cancel the provisional insertion. Undo/Redo still works as for an insertion.
    pub fn cancel_shape_preview(&mut self) -> bool {
        if !self.has_shape_preview() {
            return false;
        }
        self.shape_preview = None;
        self.undo().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape_tool::ShapeKind;
    fn bounds() -> Bbox3d {
        Bbox3d::new(Vec3d::new(-64.0, -64.0, 0.0), Vec3d::new(64.0, 64.0, 128.0))
    }
    #[test]
    fn shape_preview_all_generators_remain_editable_and_undo_as_one_insertion() {
        for kind in ShapeKind::ALL {
            let mut d = Document::new(Default::default());
            let before = d.to_map_text();
            let mut p = ShapeParameters {
                kind,
                ..Default::default()
            };
            d.begin_shape_preview(bounds(), p).unwrap();
            assert!(d.has_shape_preview());
            let ids = d.selection_ids().clone();
            p.num_sides = 12;
            p.num_rings = 12;
            p.accuracy = 0;
            p.sphere_thickness = 4.0;
            p.step_height = 8.0;
            assert!(d.update_shape_preview(p).unwrap());
            assert!(d.has_shape_preview());
            assert!(d.selection_ids().intersection(&ids).count() > 0);
            let after = d.to_map_text();
            assert_eq!(d.history.undo_len(), 1);
            d.finish_shape_preview();
            assert!(!d.has_shape_preview());
            d.undo();
            assert_eq!(d.to_map_text(), before);
            d.redo();
            assert_eq!(d.to_map_text(), after);
            assert!(!d.has_shape_preview(), "redo is ordinary geometry");
        }
    }
    #[test]
    fn shape_preview_rejects_invalid_parameters_without_losing_geometry_or_session() {
        let mut d = Document::new(Default::default());
        let mut p = ShapeParameters {
            kind: ShapeKind::HollowSphere,
            ..Default::default()
        };
        assert_eq!(d.begin_shape_preview(bounds(), p).unwrap(), 80);
        let before = d.to_map_text();
        let revision = d.revision();
        p.sphere_thickness = 128.0;
        assert!(d.update_shape_preview(p).is_err());
        assert_eq!(d.to_map_text(), before);
        assert_eq!(d.revision(), revision);
        assert!(d.has_shape_preview());
        p.sphere_thickness = 4.0;
        p.sphere_brushes = 24;
        d.update_shape_preview(p).unwrap();
        assert_eq!(d.selection_count(), 24);
        p.sphere_brushes = 28;
        d.update_shape_preview(p).unwrap();
        assert_eq!(d.selection_count(), 28);
        let final_map = d.to_map_text();
        d.undo();
        assert_eq!(d.selection_count(), 0);
        d.redo();
        assert_eq!(d.to_map_text(), final_map);
        assert!(!d.has_shape_preview());
    }
    /// Quetoo's profile scales new faces by 0.25, since its textures are four texels to
    /// a unit; a drawn shape and its regenerated forms get that scale on every face.
    #[test]
    fn shape_preview_gives_new_faces_the_games_default_uv_scale() {
        let scales = |d: &Document| -> Vec<Vec2f> {
            d.selection_ids()
                .iter()
                .filter_map(|id| match &node::find_node(d.tree_world(), *id)?.kind {
                    NodeKind::Brush(brush) => Some(brush.clone()),
                    _ => None,
                })
                .flat_map(|brush| {
                    brush
                        .faces()
                        .iter()
                        .map(|face| face.uv_attributes.scale)
                        .collect::<Vec<_>>()
                })
                .collect()
        };
        let mut d = Document::new(Default::default());
        let mut p = ShapeParameters {
            kind: ShapeKind::Cylinder,
            ..Default::default()
        };
        d.begin_shape_preview(bounds(), p).unwrap();
        assert!(scales(&d).iter().all(|s| *s == Vec2f::new(1.0, 1.0)));

        let mut d = Document::new(Default::default());
        d.set_default_uv_scale(Some((0.25, 0.5)));
        d.begin_shape_preview(bounds(), p).unwrap();
        let first = scales(&d);
        assert!(!first.is_empty());
        assert!(first.iter().all(|s| *s == Vec2f::new(0.25, 0.5)));
        p.num_sides = 12;
        assert!(d.update_shape_preview(p).unwrap());
        assert!(scales(&d).iter().all(|s| *s == Vec2f::new(0.25, 0.5)));
    }
    #[test]
    fn shape_preview_survives_same_selection_and_a_click_without_movement() {
        let mut d = Document::new(Default::default());
        let p = ShapeParameters {
            kind: ShapeKind::Cone,
            ..Default::default()
        };
        d.begin_shape_preview(bounds(), p).unwrap();
        let id = *d.selection_ids().iter().next().unwrap();
        d.select_only(id);
        assert!(d.has_shape_preview());
        d.begin_transaction();
        assert!(!d.update_shape_preview(p).unwrap());
        assert!(d.has_shape_preview());
        assert!(!d.commit_transaction());
        assert!(d.has_shape_preview());
        let changed = ShapeParameters { num_sides: 12, ..p };
        assert!(d.update_shape_preview(changed).unwrap());
        assert_eq!(d.history.undo_len(), 1);
    }
    #[test]
    fn shape_preview_deselection_other_edits_and_cancel_end_the_session() {
        let mut d = Document::new(Default::default());
        let p = ShapeParameters {
            kind: ShapeKind::Cone,
            ..Default::default()
        };
        let empty = d.to_map_text();
        d.begin_shape_preview(bounds(), p).unwrap();
        d.clear_selection();
        assert!(!d.has_shape_preview());
        assert!(!d.update_shape_preview(ShapeParameters::default()).unwrap());
        d.undo();
        assert_eq!(d.to_map_text(), empty);
        d.begin_shape_preview(bounds(), p).unwrap();
        assert!(d.cancel_shape_preview());
        assert_eq!(d.to_map_text(), empty);
        d.begin_shape_preview(bounds(), p).unwrap();
        d.delete_selection();
        assert!(!d.has_shape_preview());
        d.undo();
        assert!(!d.has_shape_preview());
    }
}
