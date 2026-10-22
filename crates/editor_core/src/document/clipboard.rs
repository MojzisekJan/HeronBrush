//! Document operations for clipboard.

use super::{
    apply_face_attributes, brush_node, collect_for_paste, entity_node, node, paste_bounds,
    patch_node, renumber_for_paste, translate_paste, vec, Bbox3d, Brush, Document, Edit,
    FaceAttributeTransfer, FaceRef, Group, Node, NodeId, NodeKind, Vec3d, FACE_CLIPBOARD_MARKER,
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
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if self.selection.contains(&id) {
                layer.children.push(brush_node(id, brush.clone()));
            }
        }
        for (id, patch, _) in node::patches_by_layer(&self.tree.world) {
            if self.selection.contains(&id) {
                layer.children.push(patch_node(id, patch.clone()));
            }
        }
        for (id, entity) in node::entities_with_ids(&self.tree.world) {
            if self.selection.contains(&id) {
                layer.children.push(entity_node(id, entity.clone()));
            }
        }
        if layer.children.is_empty() {
            return None;
        }

        let world = map_model::node::World {
            properties: vec![(
                map_model::entity_properties::keys::CLASSNAME.to_string(),
                map_model::entity_properties::values::WORLDSPAWN.to_string(),
            )],
            layers: vec![layer],
        };
        Some(map_model::io::map_writer::write_map(&node::flatten_tree(
            &world,
            self.format,
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
                translate_paste(node, delta, &world_bounds);
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
            layer: self.current_layer,
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
        let source = self.face_at(reference)?.clone();

        let template = map_model::shapes::create_cuboid(
            &self.world_bounds,
            &Bbox3d::new(Vec3d::zero(), Vec3d::new(64.0, 64.0, 64.0)),
            &source.material_name,
        )
        .ok()?;
        let mut faces = template.faces().to_vec();
        for face in &mut faces {
            face.material_name = source.material_name.clone();
            face.uv_attributes = source.uv_attributes;
            face.uv_coord_system = source.uv_coord_system;
            face.surface_attributes = source.surface_attributes;
            face.surface_material = source.surface_material;
            face.set_paint(source.paint());
        }
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
        let body =
            map_model::io::map_writer::write_map(&node::flatten_tree(&world, self.format, None));
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
            let Ok(edited) = Brush::from_faces(&world_bounds, faces) else {
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
