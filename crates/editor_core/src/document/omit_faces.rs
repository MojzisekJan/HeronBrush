//! Conservative automatic omission within static world geometry only.
use super::*;
impl Document {
    pub fn omit_covered_faces(&mut self) -> usize {
        use scene_export::{
            map,
            math::{Plane, Vec2, Vec3},
        };
        let entity_brushes: HashSet<_> = node::entities_with_ids(&self.tree.world)
            .into_iter()
            .flat_map(|(id, _)| node::contents_of(&self.tree.world, id))
            .collect();
        let candidates: Vec<_> = node::brushes_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, b, layer)| {
                !entity_brushes.contains(id)
                    && self.layer_is_editable(*layer)
                    && self.node_is_drawn(*id, *layer)
                    && b.faces().iter().all(|f| {
                        self.materials()
                            .get(&f.material_name)
                            .is_none_or(|m| m.decoded_alpha() == map_model::material::Alpha::Opaque)
                            && !f.export_hidden
                            && f.paint().is_none_or(|c| c.alpha() == 255)
                            && !matches!(
                                f.material_name
                                    .rsplit('/')
                                    .next()
                                    .unwrap_or("")
                                    .to_ascii_lowercase()
                                    .as_str(),
                                "skip" | "clip" | "origin" | "glass" | "water"
                            )
                    })
            })
            .collect();
        let convert = |p: Vec3d| Vec3::new(p.x(), p.y(), p.z());
        let brushes: Vec<_> = candidates
            .iter()
            .map(|(_, b, _)| map::Brush {
                faces: b
                    .faces()
                    .iter()
                    .map(|f| map::Face {
                        plane: Plane {
                            normal: convert(f.normal()),
                            d: f.plane.distance,
                        },
                        points: f.points.map(convert),
                        texture: f.material_name.to_string(),
                        format: map::UvFormat::Standard,
                        uv_axes: [Vec3::X, Vec3::Y],
                        offset: Vec2::ZERO,
                        rotation: 0.0,
                        scale: Vec2::ONE,
                        surface: None,
                    })
                    .collect(),
            })
            .collect();
        let mut hidden = scene_export::hidden::covered_faces(&brushes, |_| false);
        for b in scene_export::hidden::swallowed(&brushes) {
            hidden.extend((0..brushes[b].faces.len()).map(|f| (b, f)));
        }
        hidden.sort_unstable();
        hidden.dedup();
        let selected: HashSet<_> = self
            .selection
            .iter()
            .copied()
            .chain(self.selected_faces().iter().map(|f| f.node))
            .collect();
        let faces: Vec<_> = hidden
            .into_iter()
            .filter_map(|(b, f)| {
                let (id, brush, _) = candidates[b];
                if !selected.is_empty() && !selected.contains(&id) {
                    return None;
                }
                (0..brush.geometry().face_count())
                    .find(|index| brush.source_face_index(*index) == Some(f))
                    .map(|face| FaceRef { node: id, face })
            })
            .collect();
        self.set_export_hidden_on(&faces, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cube(d: &mut Document, min: Vec3d, max: Vec3d) {
        let brush =
            map_model::shapes::create_cuboid(&d.world_bounds(), &Bbox3d::new(min, max), "stone")
                .unwrap();
        d.add_brushes(vec![brush]);
    }
    #[test]
    fn covered_static_faces_are_undoable_and_preview_filter_keeps_rest_geometry() {
        let mut d = Document::new(Default::default());
        cube(&mut d, Vec3d::zero(), Vec3d::fill(64.));
        cube(&mut d, Vec3d::new(64., 0., 0.), Vec3d::new(128., 64., 64.));
        d.clear_selection();
        let before = d.to_map_text();
        let bounds = d.bounds();
        assert_eq!(d.omit_covered_faces(), 2);
        let after = d.to_map_text();
        assert_eq!(d.bounds(), bounds);
        let visible = crate::view_filters::ViewFilters::default();
        let hidden = crate::view_filters::ViewFilters {
            show_export_hidden: false,
            ..visible
        };
        assert_eq!(
            d.preview_mesh(&visible).triangles.len() - d.preview_mesh(&hidden).triangles.len(),
            4
        );
        let edges = d.animation_wireframe(&hidden);
        assert!(!edges.plain.is_empty());
        drop(edges);
        assert_eq!(d.omit_covered_faces(), 0);
        d.undo();
        assert_eq!(d.to_map_text(), before);
        d.redo();
        assert_eq!(d.to_map_text(), after);
    }
    #[test]
    fn moving_entities_and_transparent_faces_never_cover_static_faces() {
        let mut d = Document::new(Default::default());
        cube(&mut d, Vec3d::zero(), Vec3d::fill(64.));
        d.select_all();
        d.create_brush_entity("func_animate").unwrap();
        d.clear_selection();
        cube(&mut d, Vec3d::new(64., 0., 0.), Vec3d::new(128., 64., 64.));
        d.clear_selection();
        assert_eq!(d.omit_covered_faces(), 0);
        cube(&mut d, Vec3d::new(128., 0., 0.), Vec3d::new(192., 64., 64.));
        d.set_face_color(Some(
            map_model::brush_face::FaceColor::solid(map_model::brush_face::Rgb::new(1, 2, 3))
                .with_alpha(128),
        ));
        d.clear_selection();
        assert_eq!(d.omit_covered_faces(), 0);
    }
}
