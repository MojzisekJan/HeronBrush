//! Insert a complete text object with grid-aligned bounds and hidden internal seams.
use super::*;
use crate::grid::Grid;

pub(super) struct TextPreview {
    layer: usize,
    nodes: Vec<Node>,
    selection: HashSet<NodeId>,
    faces: HashSet<FaceRef>,
}

impl Document {
    /// Fits the overall bounding box to the active grid without rounding glyph vertices.
    /// A failed transform leaves the document unchanged.
    fn text_brushes(
        &self,
        text: &str,
        height: f64,
        depth: f64,
        origin: Vec3d,
        grid: &Grid,
    ) -> Vec<Brush> {
        let material = self.current_material();
        let material = match material
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "skip" | "clip" | "origin" | "trigger" => map_model::shapes::DEFAULT_MATERIAL,
            _ => material,
        };
        let world = self.world_bounds();
        let mut brushes =
            map_model::stylized_text::brushes(text, height, depth, material, origin, &world);
        let Some(first) = brushes.first() else {
            return Vec::new();
        };
        if grid.snap() {
            let bounds = brushes
                .iter()
                .skip(1)
                .fold(first.bounds(), |b, brush| b.merged(&brush.bounds()));
            let min = grid.snap_point(bounds.min);
            let rounded_max = grid.snap_point(bounds.max);
            let max = Vec3d::new(
                rounded_max.x().max(min.x() + grid.actual_size()),
                rounded_max.y().max(min.y() + grid.actual_size()),
                rounded_max.z().max(min.z() + grid.actual_size()),
            );
            if !world.contains(&Bbox3d::new(min, max)) {
                return Vec::new();
            }
            let old_size = bounds.max - bounds.min;
            let size = max - min;
            let scale = Vec3d::new(
                size.x() / old_size.x(),
                size.y() / old_size.y(),
                size.z() / old_size.z(),
            );
            let matrix = mat_ext::translation_matrix(min)
                * mat_ext::scaling_matrix(scale)
                * mat_ext::translation_matrix(-bounds.min);
            let Ok(fitted) = brushes
                .iter()
                .map(|b| b.transform(&world, &matrix))
                .collect::<Result<Vec<_>, _>>()
            else {
                return Vec::new();
            };
            brushes = fitted;
        }
        let exported = super::omit_faces::export_brushes(brushes.iter());
        let hidden = scene_export::hidden::covered_faces(&exported, |_| false);
        let mut faces: Vec<_> = brushes.iter().map(|b| b.faces().to_vec()).collect();
        for (brush, face) in hidden {
            faces[brush][face].export_hidden = true;
            faces[brush][face].material_name = "skip".into();
        }
        for (brush, faces) in brushes.iter_mut().zip(faces) {
            *brush = brush
                .with_exact_face_attributes(&faces)
                .expect("Only face attributes changed");
        }
        brushes
    }

    /// Inserts a complete grouped text object in one Undo step.
    pub fn insert_text(
        &mut self,
        text: &str,
        height: f64,
        depth: f64,
        origin: Vec3d,
        grid: &Grid,
    ) -> usize {
        let brushes = self.text_brushes(text, height, depth, origin, grid);
        self.insert_text_brushes(text, brushes)
    }

    fn insert_text_brushes(&mut self, text: &str, brushes: Vec<Brush>) -> usize {
        let count = brushes.len();
        if count == 0 {
            return 0;
        }
        self.run_as_one_edit(|d| {
            d.add_brushes(brushes);
            d.group_selection(text.trim());
        });
        count
    }

    /// Starts a provisional text object. It is rendered normally but has no history or
    /// collaboration edits; autosave and incoming edits wait until it is resolved.
    pub fn begin_text_preview(&mut self) -> bool {
        if self.in_transaction() {
            return false;
        }
        self.text_preview = Some(TextPreview {
            layer: self.current_layer(),
            nodes: vec![],
            selection: self.selection.clone(),
            faces: self.face_selection.clone(),
        });
        true
    }

    /// Whether a provisional text object is active, even when its input is empty.
    pub fn has_text_preview(&self) -> bool {
        self.text_preview.is_some()
    }

    /// Regenerates only the provisional object, leaving existing map geometry intact.
    pub fn update_text_preview(
        &mut self,
        text: &str,
        height: f64,
        depth: f64,
        origin: Vec3d,
        grid: &Grid,
    ) -> usize {
        if self.text_preview.is_none() {
            return 0;
        }
        let brushes = self.text_brushes(text, height, depth, origin, grid);
        let mut preview = self.text_preview.take().expect("checked");
        self.remove_text_preview_nodes(&preview);
        preview.nodes = brushes
            .into_iter()
            .map(|b| Node::new(NodeKind::Brush(b)))
            .collect();
        let count = preview.nodes.len();
        if let Some(layer) = self.tree.world.layers.get_mut(preview.layer) {
            layer.children.extend(preview.nodes.iter().cloned());
        } else {
            preview.nodes.clear();
        }
        self.selection = preview.nodes.iter().map(|n| n.id).collect();
        self.face_selection.clear();
        self.text_preview = Some(preview);
        self.mark_changed();
        count
    }

    fn remove_text_preview_nodes(&mut self, preview: &TextPreview) {
        let ids: Vec<_> = preview.nodes.iter().map(|n| n.id).collect();
        node::remove_nodes_from_world(&mut self.tree.world, &ids);
    }

    /// Accepts exactly the displayed geometry as one grouped insertion.
    pub fn finish_text_preview(&mut self, text: &str) -> usize {
        if self
            .text_preview
            .as_ref()
            .is_none_or(|p| p.nodes.is_empty())
        {
            return 0;
        }
        let preview = self.text_preview.take().expect("checked");
        self.remove_text_preview_nodes(&preview);
        let brushes = preview
            .nodes
            .into_iter()
            .filter_map(|n| match n.kind {
                NodeKind::Brush(b) => Some(b),
                _ => None,
            })
            .collect();
        self.selection = preview.selection;
        self.face_selection = preview.faces;
        self.insert_text_brushes(text, brushes)
    }

    /// Discards the provisional object and restores selection without changing Undo/Redo.
    pub fn cancel_text_preview(&mut self) -> bool {
        let Some(preview) = self.text_preview.take() else {
            return false;
        };
        self.remove_text_preview_nodes(&preview);
        self.selection = preview.selection;
        self.face_selection = preview.faces;
        self.mark_changed();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_bounds_snap_and_internal_faces_are_skipped_in_one_undo() {
        let mut d = Document::new(Default::default());
        d.set_current_material("skip");
        let before = d.to_map_text();
        assert!(d.insert_text("O8", 65., 9., Vec3d::new(13., 7., -19.), &Grid::default()) > 0);
        let bounds = d.bounds().unwrap();
        for v in [bounds.min, bounds.max] {
            for axis in [v.x(), v.y(), v.z()] {
                assert!((axis / 16. - (axis / 16.).round()).abs() < 1e-6);
            }
        }
        let brushes = node::brushes(&d.tree.world);
        assert!(brushes
            .iter()
            .flat_map(|b| b.faces())
            .any(|f| f.export_hidden && f.material_name == "skip"));
        assert!(
            brushes
                .iter()
                .flat_map(|b| b.faces())
                .filter(|f| !f.export_hidden)
                .all(|f| f.material_name == map_model::shapes::DEFAULT_MATERIAL
                    && f.paint().is_none())
        );
        d.undo();
        assert_eq!(d.to_map_text(), before);
        d.redo();
        assert!(d.bounds().is_some());
    }
    #[test]
    fn disabled_snap_preserves_geometry_material_and_exposed_caps() {
        let mut grid = Grid::default();
        grid.set_snap(false);
        for height in [4., 65.] {
            let mut d = Document::new(Default::default());
            d.set_current_material("stone");
            let origin = Vec3d::new(13., 7., -19.);
            let expected = map_model::stylized_text::brushes(
                "O8AB",
                height,
                9.,
                "stone",
                origin,
                &d.world_bounds(),
            );
            let expected_bounds = expected
                .iter()
                .skip(1)
                .fold(expected[0].bounds(), |a, b| a.merged(&b.bounds()));
            assert_eq!(
                d.insert_text("O8AB", height, 9., origin, &grid),
                expected.len()
            );
            assert_eq!(d.bounds(), Some(expected_bounds));
            for brush in node::brushes(d.tree_world()) {
                for face in brush.faces() {
                    if face.export_hidden {
                        assert!(
                            face.normal().y().abs() < 1e-8,
                            "Front/back caps must remain visible"
                        );
                        assert_eq!(face.material_name, "skip");
                    } else {
                        assert_eq!(face.material_name, "stone");
                    }
                }
            }
            let shown = crate::view_filters::ViewFilters {
                show_export_hidden: true,
                ..Default::default()
            };
            let hidden = crate::view_filters::ViewFilters {
                show_export_hidden: false,
                ..Default::default()
            };
            assert!(
                d.preview_mesh(&hidden).triangles.len() < d.preview_mesh(&shown).triangles.len()
            );
        }
    }
    #[test]
    fn preview_is_visible_in_both_views_without_history_dirty_state_or_broadcasts() {
        let mut d = Document::new(Default::default());
        let before = d.to_map_text();
        let changes = Rc::new(std::cell::Cell::new(0));
        let seen = changes.clone();
        d.set_on_edit(Rc::new(move |_| seen.set(seen.get() + 1)));
        let modifications = d.modification_count();
        let mark = d.edit_mark();
        assert!(d.begin_text_preview());
        assert!(d.in_transaction());
        for text in ["O", "HELLO", "A8"] {
            assert!(d.update_text_preview(text, 64., 16., Vec3d::zero(), &Grid::default()) > 0);
            let filters = crate::view_filters::ViewFilters::default();
            assert!(!d.preview_mesh(&filters).triangles.is_empty());
            assert!(!d.animation_wireframe(&filters).selected.is_empty());
        }
        assert_eq!(d.modification_count(), modifications);
        assert_eq!(d.edit_mark(), mark);
        assert!(!d.is_modified());
        assert_eq!(changes.get(), 0);
        assert!(d.cancel_text_preview());
        assert!(!d.in_transaction());
        assert_eq!(d.to_map_text(), before);
        assert_eq!(changes.get(), 0);
    }

    #[test]
    fn accepting_last_preview_records_one_insertion_and_cancel_preserves_redo_selection() {
        let mut d = Document::new(Default::default());
        d.insert_text("I", 64., 16., Vec3d::zero(), &Grid::default());
        let before = d.to_map_text();
        let selection = d.selection.clone();
        d.begin_text_preview();
        assert!(d.undo().is_none());
        assert!(d.redo().is_none());
        d.update_text_preview("O8", 80., 32., Vec3d::new(128., 0., 0.), &Grid::default());
        d.cancel_text_preview();
        assert_eq!(d.to_map_text(), before);
        assert_eq!(d.selection, selection);
        d.undo();
        let empty = d.to_map_text();
        d.begin_text_preview();
        d.update_text_preview("HI", 64., 16., Vec3d::zero(), &Grid::default());
        d.cancel_text_preview();
        assert_eq!(d.to_map_text(), empty);
        assert!(d.redo().is_some());
        assert_eq!(d.to_map_text(), before);
        d.begin_text_preview();
        d.update_text_preview("OLD", 64., 16., Vec3d::zero(), &Grid::default());
        d.update_text_preview("O8", 80., 32., Vec3d::new(128., 0., 0.), &Grid::default());
        let bounds = d.bounds();
        assert!(d.finish_text_preview("O8") > 0);
        assert_eq!(d.bounds(), bounds);
        assert!(!d.has_text_preview());
        d.undo();
        assert_eq!(d.to_map_text(), before);
        d.redo();
        assert_eq!(d.bounds(), bounds);
    }

    #[test]
    fn empty_preview_cannot_confirm_and_adopting_world_discards_it() {
        let mut d = Document::new(Default::default());
        let world = d.tree_world().clone();
        d.begin_text_preview();
        assert_eq!(d.finish_text_preview(""), 0);
        assert!(d.has_text_preview());
        d.update_text_preview("O", 64., 16., Vec3d::zero(), &Grid::default());
        assert_eq!(
            d.update_text_preview("", 64., 16., Vec3d::zero(), &Grid::default()),
            0
        );
        assert_eq!(d.brush_count(), 0);
        d.update_text_preview("O", 64., 16., Vec3d::zero(), &Grid::default());
        d.adopt_world(world);
        assert!(!d.has_text_preview());
        assert!(!d.cancel_text_preview());
        assert_eq!(d.brush_count(), 0);
    }
    #[test]
    fn out_of_bounds_text_is_rejected_as_a_whole_instead_of_losing_letters() {
        let mut d = Document::new(crate::world::WorldBounds::new(128.));
        let mut grid = Grid::default();
        grid.set_snap(false);
        let before = d.to_map_text();
        assert_eq!(
            d.insert_text("IIII", 64., 16., Vec3d::new(100., 0., 0.), &grid),
            0
        );
        assert_eq!(d.to_map_text(), before);
        assert_eq!(d.edit_mark(), 0);
    }

    #[test]
    fn invalid_dimensions_positions_and_unsupported_input_never_change_the_map() {
        let mut d = Document::new(Default::default());
        let before = d.to_map_text();
        let grid = Grid::default();
        for text in ["", "   ", "🦆"] {
            assert_eq!(d.insert_text(text, 64., 16., Vec3d::zero(), &grid), 0);
        }
        for size in [0., -1., f64::NAN, f64::INFINITY] {
            assert_eq!(d.insert_text("O", size, 16., Vec3d::zero(), &grid), 0);
            assert_eq!(d.insert_text("O", 64., size, Vec3d::zero(), &grid), 0);
        }
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for axis in 0..3 {
                let mut origin = Vec3d::zero();
                origin[axis] = bad;
                assert_eq!(d.insert_text("O", 64., 16., origin, &grid), 0);
            }
        }
        assert_eq!(d.to_map_text(), before);
        assert_eq!(d.edit_mark(), 0);
    }

    #[test]
    fn varied_grids_sizes_and_accents_preserve_filled_regions_and_holes() {
        for text in ["O8", "ÁČĚŘŠŤŽŮ", "ABCD0123"] {
            for height in [4., 64., 512.] {
                for exponent in [-3, 0, 4, 8] {
                    let mut d = Document::new(Default::default());
                    let origin = Vec3d::new(-123.25, 5.5, -31.75);
                    let original = map_model::stylized_text::brushes(
                        text,
                        height,
                        9.,
                        "stone",
                        origin,
                        &d.world_bounds(),
                    );
                    let old = original
                        .iter()
                        .skip(1)
                        .fold(original[0].bounds(), |b, x| b.merged(&x.bounds()));
                    let grid = Grid::new(exponent);
                    assert_eq!(
                        d.insert_text(text, height, 9., origin, &grid),
                        original.len(),
                        "{text}, {height}, {exponent}"
                    );
                    let new = d.bounds().unwrap();
                    for corner in [new.min, new.max] {
                        for axis in 0..3 {
                            let units = corner[axis] / grid.actual_size();
                            assert!((units - units.round()).abs() < 1e-5, "{text}, height={height}, grid={exponent}, axis={axis}, units={units}, bounds={new:?}");
                        }
                    }
                    let brushes = node::brushes(d.tree_world());
                    let old_size = old.max - old.min;
                    let new_size = new.max - new.min;
                    let old_volume: f64 = original.iter().map(Brush::volume).sum();
                    let new_volume: f64 = brushes.iter().map(|b| b.volume()).sum();
                    let expected = old_volume * new_size.x() / old_size.x() * new_size.y()
                        / old_size.y()
                        * new_size.z()
                        / old_size.z();
                    assert!((new_volume - expected).abs() < expected * 1e-6);
                    for x in 0..13 {
                        for z in 0..13 {
                            let u = (x as f64 + 0.37) / 13.;
                            let v = (z as f64 + 0.29) / 13.;
                            let a = old.min
                                + Vec3d::new(
                                    old_size.x() * u,
                                    old_size.y() * 0.5,
                                    old_size.z() * v,
                                );
                            let b = new.min
                                + Vec3d::new(
                                    new_size.x() * u,
                                    new_size.y() * 0.5,
                                    new_size.z() * v,
                                );
                            assert_eq!(
                                original
                                    .iter()
                                    .any(|x| x.geometry().contains_point(a, 1e-8)),
                                brushes.iter().any(|x| x.geometry().contains_point(b, 1e-8)),
                                "Changed glyph fill: {text}, {height}, {exponent}, {x}, {z}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn committed_text_round_trips_through_map_and_collaboration_with_internal_skip_faces() {
        let mut d = Document::new(Default::default());
        d.set_current_material("stone");
        let edits = Rc::new(RefCell::new(Vec::<Edit>::new()));
        let captured = edits.clone();
        d.set_on_edit(Rc::new(move |e| captured.borrow_mut().push(e.clone())));
        d.begin_text_preview();
        d.update_text_preview("O8Á", 65., 9., Vec3d::new(13., 7., -19.), &Grid::default());
        assert!(edits.borrow().is_empty());
        assert!(d.finish_text_preview("O8Á") > 0);
        let mut peer = Document::new(Default::default());
        for edit in edits.borrow().iter() {
            peer.apply_remote(edit);
        }
        assert_eq!(peer.to_map_text(), d.to_map_text());
        let map = read_map(&d.to_map_text(), None).unwrap();
        let tree = node::build_tree(&map, &d.world_bounds());
        assert!(tree.warnings.is_empty());
        let wire = crate::collab::wire::WireWorld::from_world(d.tree_world());
        let worlds = [tree.world, wire.into_world(&d.world_bounds()).unwrap()];
        let filters = crate::view_filters::ViewFilters {
            show_export_hidden: false,
            ..Default::default()
        };
        let triangles = d.preview_mesh(&filters).triangles.len();
        let hidden = node::brushes(d.tree_world())
            .iter()
            .flat_map(|b| b.faces())
            .filter(|f| f.export_hidden)
            .count();
        assert!(hidden > 0);
        for world in worlds {
            let restored = Document::from_world(world, Default::default(), MapFormat::Valve, None);
            let a = restored.bounds().unwrap();
            let b = d.bounds().unwrap();
            for axis in 0..3 {
                assert!(
                    (a.min[axis] - b.min[axis]).abs() < 1e-6
                        && (a.max[axis] - b.max[axis]).abs() < 1e-6
                );
            }
            assert_eq!(restored.preview_mesh(&filters).triangles.len(), triangles);
            assert_eq!(
                node::brushes(restored.tree_world())
                    .iter()
                    .flat_map(|b| b.faces())
                    .filter(|f| f.export_hidden && f.material_name == "skip")
                    .count(),
                hidden
            );
        }
    }
    #[test]
    fn repeated_preview_updates_do_not_leave_orphaned_brushes_or_history() {
        let mut d = Document::new(Default::default());
        let before = d.to_map_text();
        d.begin_text_preview();
        for i in 0..30 {
            let text = ["A", "O8Á", "HERON BRUSH", "", "🦆"][i % 5];
            let count = d.update_text_preview(
                text,
                32. + i as f64,
                9.,
                Vec3d::new(i as f64, 7., -19.),
                &Grid::default(),
            );
            assert_eq!(d.brush_count(), count);
            assert_eq!(node::all_node_ids(d.tree_world()).len(), count);
            assert_eq!(d.selection_count(), count);
            assert_eq!(d.edit_mark(), 0);
        }
        assert!(d.cancel_text_preview());
        assert_eq!(d.to_map_text(), before);
    }
}
