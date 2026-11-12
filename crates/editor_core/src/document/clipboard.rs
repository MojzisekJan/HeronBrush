//! Document operations for clipboard.

use super::{
    apply_face_attributes, brush_node, collect_for_paste, node, paste_bounds, renumber_for_paste,
    translate_paste, vec, Brush, Document, Edit, FaceAttributeTransfer, FaceRef, Group, Node,
    NodeId, NodeKind, Vec3d, FACE_CLIPBOARD_MARKER,
};

impl Document {
    /// Writes the selection out as `.map` text, for the clipboard.
    ///
    /// `serializeSelectedNodes`. The clipboard holds the map format itself rather than
    /// something private, which is what lets a selection be pasted into another editor,
    /// or into a text file and back.
    ///
    /// Returns `None` when no object is selected.
    pub fn copy_selection(&self) -> Option<String> {
        if self.selection.is_empty() {
            return None;
        }

        // A world of its own holding only what is selected, so the writer has something
        // whole to write. Groups and layers are flattened away: what is pasted is the
        // geometry, and where it goes is the pasting document's business.
        let mut layer = map_model::node::Layer {
            id: None,
            name: map_model::node::DEFAULT_LAYER_NAME.to_string(),
            sort_index: 0,
            hidden: false,
            locked: false,
            omit_from_export: false,
            color: None,
            children: Vec::new(),
        };
        fn collect(
            nodes: &[Node],
            selected: &super::HashSet<NodeId>,
            all: bool,
            out: &mut Vec<Node>,
        ) {
            for node in nodes {
                let take = all || selected.contains(&node.id);
                match &node.kind {
                    NodeKind::Group(group) => collect(&group.children, selected, take, out),
                    NodeKind::Layer(layer) => collect(&layer.children, selected, take, out),
                    NodeKind::Entity(entity) if !take => {
                        let mut children = Vec::new();
                        collect(&entity.children, selected, false, &mut children);
                        if !children.is_empty() {
                            out.push(Node::with_id(
                                node.id,
                                NodeKind::Entity(node::EntityNode {
                                    properties: entity.properties.clone(),
                                    children,
                                }),
                            ));
                        }
                    }
                    _ if take => out.push(node.clone()),
                    _ => {}
                }
            }
        }
        for source in &self.tree.world.layers {
            collect(
                &source.children,
                &self.selection,
                false,
                &mut layer.children,
            );
        }
        if layer.children.is_empty() {
            return None;
        }
        self.absolute_model_references(&mut layer.children);

        let world = map_model::node::World {
            properties: vec![(
                map_model::entity_properties::keys::CLASSNAME.to_string(),
                map_model::entity_properties::values::WORLDSPAWN.to_string(),
            )],
            layers: vec![layer],
        };
        Some(map_model::io::map_writer::write_map(&node::flatten_tree(
            &world,
            map_model::map_format::MapFormat::LOSSLESS,
            None,
        )))
    }

    /// Reads `.map` text and adds what it holds to the current layer, selecting it.
    ///
    /// With `target` set to `None`, preserves the stored positions. With `Some`, moves the centre
    /// of the pasted contents to the requested grid position.
    ///
    /// Returns how many objects were added, and nothing is added at all if the text does
    /// not read as a map.
    pub fn paste_at(&mut self, text: &str, target: Option<Vec3d>) -> usize {
        self.paste_nodes_at(text, target, None, None)
    }

    /// Inserts a prefab as one named group and one undoable edit.
    #[cfg(test)]
    pub fn paste_asset_at(&mut self, text: &str, target: Vec3d, name: &str) -> usize {
        self.paste_nodes_at(text, Some(target), Some(name), None)
    }

    /// Places an asset with its bottom at the target height and its footprint
    /// near the target, aligning the footprint minimum to the enabled grid.
    pub fn paste_asset_on_grid(
        &mut self,
        text: &str,
        target: Vec3d,
        name: &str,
        grid: &crate::grid::Grid,
    ) -> usize {
        self.paste_nodes_at(text, Some(target), Some(name), Some(grid))
    }

    fn paste_nodes_at(
        &mut self,
        text: &str,
        target: Option<Vec3d>,
        group_name: Option<&str>,
        placement_grid: Option<&crate::grid::Grid>,
    ) -> usize {
        // A face-attributes copy carries a marker; pasting it puts the material and
        // mapping on the selected faces rather than adding geometry.
        if text.trim_start().starts_with(FACE_CLIPBOARD_MARKER) {
            return if group_name.is_none() {
                self.paste_face_attributes(text)
            } else {
                0
            };
        }
        let Ok(map) = map_model::io::map_reader::read_map(text, None) else {
            return 0;
        };
        let tree = node::build_tree(&map, &self.world_bounds);

        // Everything in the pasted map, whichever layer it was written on: the layers it
        // came from mean nothing here.
        let mut pasted: Vec<Node> = Vec::new();
        for layer in &tree.world.layers {
            collect_for_paste(&layer.children, &mut pasted);
        }
        if pasted.is_empty() {
            return 0;
        }

        // New ids, so pasting twice does not put the same object in twice over.
        let mut pasted: Vec<Node> = pasted.into_iter().map(renumber_for_paste).collect();
        self.relative_model_references(&mut pasted);

        if let (Some(target), Some(bounds)) = (target, paste_bounds(&pasted)) {
            // A NaN component means "leave this axis alone", which is how a 2D view says
            // it has no value to give for the axis it looks along.
            let center = bounds.center();
            let mut delta = Vec3d::zero();
            for axis in 0..3 {
                if target[axis].is_finite() {
                    delta[axis] = target[axis] - center[axis];
                }
            }
            if let Some(grid) = placement_grid {
                for axis in 0..3 {
                    if !target[axis].is_finite() {
                        continue;
                    }
                    let minimum = if axis == 2 {
                        target[axis]
                    } else {
                        bounds.min[axis] + delta[axis]
                    };
                    let minimum = if grid.snap() {
                        (minimum / grid.actual_size()).round() * grid.actual_size()
                    } else {
                        minimum
                    };
                    delta[axis] = minimum - bounds.min[axis];
                }
            }
            // A prefab centre can sit halfway between two lattice points even
            // when all its vertices are exact. Snap the translation, not each
            // brush independently, to retain their shared contacts and shape.
            if group_name.is_some() {
                let finest = 2.0f64.powi(crate::grid::MIN_SIZE);
                if placement_grid.is_some() {
                    delta = vec::round(delta / finest) * finest;
                } else {
                    // Keep integer assets on integer coordinates even when their
                    // bounding-box centre lies between grid points. Finer assets
                    // retain their existing lattice on each axis.
                    let brushes = node::brushes(&tree.world);
                    for axis in 0..3 {
                        let step = [1.0, 0.5, 0.25, finest]
                            .into_iter()
                            .find(|step| {
                                brushes.iter().all(|brush| {
                                    brush.geometry().vertices().iter().all(|v| {
                                        let coordinate = v.position[axis] / step;
                                        (coordinate - coordinate.round()).abs() < 1e-5
                                    })
                                })
                            })
                            .unwrap_or(finest);
                        delta[axis] = (delta[axis] / step).round() * step;
                    }
                }
            }
            let world_bounds = self.world_bounds;
            for node in &mut pasted {
                if !translate_paste(node, delta, &world_bounds) {
                    return 0;
                }
            }
        }

        let count = pasted.len();
        // Selection tracks editable group members, not the container node. Keep
        // these IDs before wrapping an asset so bounds and transforms work at once.
        self.selection = pasted.iter().map(|node| node.id).collect();
        if let Some(name) = group_name {
            pasted = vec![Node::new(NodeKind::Group(Group {
                id: self.fresh_id(),
                name: name.to_string(),
                linked_group_id: None,
                transformation: None,
                children: pasted,
            }))];
        }
        self.face_selection.clear();
        self.apply_and_record(Edit::Added {
            layer: self.current_layer(),
            nodes: pasted,
        });
        count
    }

    /// Writes the selected face's material, mapping and paint out for the clipboard.
    ///
    /// Copies selected face attributes into a one-brush `.map` payload with a marker comment.
    /// Returns `None` when no face is selected.
    pub fn copy_face_attributes_text(&self) -> Option<String> {
        let reference = self.selected_edit_faces().into_iter().next()?;
        let source = self.brush(reference.node)?;
        let selected = source.source_face_index(reference.face)?;
        // The first face is the copied one. Preserve its actual plane too: pending
        // brush-primitive matrices are expressed in that plane's own UV basis.
        let mut faces = source.faces().to_vec();
        faces.swap(0, selected);
        let brush = Brush::from_faces(&self.world_bounds, faces).ok()?;

        let layer = map_model::node::Layer {
            id: None,
            name: map_model::node::DEFAULT_LAYER_NAME.to_string(),
            sort_index: 0,
            hidden: false,
            locked: false,
            omit_from_export: false,
            color: None,
            children: vec![brush_node(NodeId(0), brush)],
        };
        let world = map_model::node::World {
            properties: vec![(
                map_model::entity_properties::keys::CLASSNAME.to_string(),
                map_model::entity_properties::values::WORLDSPAWN.to_string(),
            )],
            layers: vec![layer],
        };
        let body = map_model::io::map_writer::write_map(&node::flatten_tree(
            &world,
            map_model::map_format::MapFormat::LOSSLESS,
            None,
        ));
        Some(format!("{FACE_CLIPBOARD_MARKER}\n{body}"))
    }

    /// Puts a copied face's material and mapping on every selected face, in one undo step.
    fn paste_face_attributes(&mut self, text: &str) -> usize {
        let targets = self.selected_edit_faces();
        self.paste_face_attributes_on(text, &targets)
    }

    /// Paste only copied face attributes onto explicit targets, preserving selection.
    /// Object clipboard contents never insert geometry through this operation.
    pub fn paste_face_attributes_on(&mut self, text: &str, targets: &[FaceRef]) -> usize {
        if !text.trim_start().starts_with(FACE_CLIPBOARD_MARKER) {
            return 0;
        }
        let body = text
            .trim_start()
            .strip_prefix(FACE_CLIPBOARD_MARKER)
            .unwrap_or(text);
        let Ok(map) = map_model::io::map_reader::read_map(body, None) else {
            return 0;
        };
        let tree = node::build_tree(&map, &self.world_bounds);
        let Some(source) = node::brushes_with_ids(&tree.world)
            .first()
            .and_then(|(_, brush)| brush.faces().first())
            .cloned()
        else {
            return 0;
        };

        self.set_faces_from(&source, targets, FaceAttributeTransfer::Projection)
    }

    /// Sets the given faces' material and mapping from a source face, in one undo step.
    ///
    /// Shared by the drag transfer -- where the source is a `FaceRef` into the tree -- and
    /// the clipboard paste, where it is the face that was copied. The target faces are
    /// gathered per brush so each brush is rebuilt once, and a brush the change would
    /// leave alone is left out of the edit.
    pub(super) fn set_faces_from(
        &mut self,
        source: &map_model::brush_face::BrushFace,
        targets: &[FaceRef],
        mode: FaceAttributeTransfer,
    ) -> usize {
        let world_bounds = self.world_bounds;
        let mut by_brush: Vec<(NodeId, Vec<usize>)> = Vec::new();
        for target in targets {
            let Some(brush) = self.brush(target.node) else {
                continue;
            };
            let Some(index) = brush.source_face_index(target.face) else {
                continue;
            };
            match by_brush.iter_mut().find(|(node, _)| *node == target.node) {
                Some((_, indices)) => {
                    if !indices.contains(&index) {
                        indices.push(index);
                    }
                }
                None => by_brush.push((target.node, vec![index])),
            }
        }

        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut changed = 0;
        for (node, indices) in by_brush {
            let Some(brush) = self.brush(node) else {
                continue;
            };
            let brush = brush.clone();
            let mut faces = brush.faces().to_vec();
            for &index in &indices {
                apply_face_attributes(&mut faces[index], source, mode);
            }
            if faces.as_slice() == brush.faces() {
                continue;
            }
            let Ok(edited) = brush.with_updated_faces(&world_bounds, faces) else {
                continue;
            };
            changed += indices.len();
            before.push(brush_node(node, brush));
            after.push(brush_node(node, edited));
        }
        if before.is_empty() {
            return 0;
        }
        self.apply_and_record(Edit::Changed { before, after });
        changed
    }

    /// Writes the selection out and takes it out of the map.
    ///
    /// `MapWindow::cutSelection`. Returns nothing, and removes nothing, when there is
    /// nothing selected.
    pub fn cut_selection(&mut self) -> Option<String> {
        let text = self.copy_selection()?;
        self.delete_selection();
        Some(text)
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;
    use geometry_math::bbox::Bbox3d;
    fn source() -> String {
        let mut d = Document::new(crate::world::WorldBounds::new(128.));
        for x in [-30., 20.] {
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &Bbox3d::new(Vec3d::new(x, 0., 0.), Vec3d::new(x + 8., 8., 8.)),
                "stone",
            )
            .unwrap()]);
        }
        d.select_all();
        d.copy_selection().unwrap()
    }
    #[test]
    fn clipboard_boundary_failure_preserves_map_selection_and_history() {
        let text = source();
        let mut d = Document::new(crate::world::WorldBounds::new(128.));
        assert_eq!(d.paste_at(&text, None), 2);
        let before = d.to_map_text();
        let selection = d.selection.clone();
        let mark = d.edit_mark();
        for target in [120., 100., -120.] {
            assert_eq!(d.paste_at(&text, Some(Vec3d::new(target, 4., 4.))), 0);
            assert_eq!(d.to_map_text(), before);
            assert_eq!(d.selection, selection);
            assert_eq!(d.edit_mark(), mark);
        }
        assert_eq!(d.paste_at(&text, Some(Vec3d::new(40., 4., 4.))), 2);
        let added: Vec<_> = node::brushes_with_ids(d.tree_world())
            .into_iter()
            .filter(|(id, _)| d.selection.contains(id))
            .collect();
        assert!((added[1].1.bounds().min.x() - added[0].1.bounds().min.x() - 50.).abs() < 1e-8);
        d.undo();
        assert_eq!(d.to_map_text(), before);
    }
    #[test]
    fn clipboard_after_adopting_fewer_layers_inserts_real_objects() {
        let mut d = Document::new(Default::default());
        d.add_layer("old");
        d.adopt_world(Document::new(Default::default()).tree_world().clone());
        assert_eq!(d.current_layer, 0);
        assert_eq!(d.paste_at(&source(), None), 2);
        assert_eq!(d.brush_count(), 2);
        assert_eq!(d.selection_count(), 2);
        d.undo();
        assert_eq!(d.brush_count(), 0);
    }
    #[test]
    fn paste_patch_and_linked_duplicate_cannot_partly_leave_world_bounds() {
        use map_model::io::map_reader::{Patch, PatchControlPoint};
        let mut source_doc = Document::new(crate::world::WorldBounds::new(128.));
        let patch = Patch {
            material_name: "stone".into(),
            rows: 3,
            columns: 3,
            control_points: (0..3)
                .flat_map(|row| {
                    (0..3).map(move |column| PatchControlPoint {
                        position: Vec3d::new(column as f64 * 8. - 8., row as f64 * 8. - 8., 0.),
                        uv: geometry_math::vec::Vec2f::new(column as f32 / 2., row as f32 / 2.),
                    })
                })
                .collect(),
            id: None,
            line: 0,
        };
        source_doc.tree.world.layers[0]
            .children
            .push(Node::new(NodeKind::Patch(patch)));
        source_doc.select_all();
        let text = source_doc.copy_selection().unwrap();
        let mut d = Document::new(crate::world::WorldBounds::new(128.));
        let before = d.to_map_text();
        assert_eq!(d.paste_at(&text, Some(Vec3d::new(124., 0., 0.))), 0);
        assert_eq!(d.to_map_text(), before);
        assert_eq!(d.paste_at(&text, Some(Vec3d::new(32., 0., 0.))), 1);
        assert!(node::patches_by_layer(d.tree_world())
            .iter()
            .all(|(_, p, _)| p
                .control_points
                .iter()
                .all(|point| d.world_bounds().contains_point(point.position))));
        let mut d = Document::new(crate::world::WorldBounds::new(128.));
        d.paste_at(&source(), Some(Vec3d::new(64., 4., 4.)));
        d.group_selection("edge");
        let before = d.to_map_text();
        let mark = d.edit_mark();
        assert!(d.create_linked_duplicate(64.).is_none());
        assert_eq!(d.to_map_text(), before);
        assert_eq!(d.edit_mark(), mark);
    }
}
