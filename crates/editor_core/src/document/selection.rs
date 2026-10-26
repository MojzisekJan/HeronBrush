//! Document operations for selection.

use super::{
    apply_face_attributes, brush_node, node, pick, Bbox3d, Brush, Document, Edit,
    FaceAttributeTransfer, FaceRef, Float, HashMap, HashSet, Node, NodeId, Picked, Polyhedron,
    Ray3d, SelectionSnapshot, Vec3d,
};

impl Document {
    /// Returns whether the given node is selected.
    #[inline]
    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selection.contains(&id)
    }

    /// Returns how many nodes are selected.
    #[inline]
    pub fn selection_count(&self) -> usize {
        self.selection.len()
    }

    /// Returns whether anything is selected.
    #[inline]
    pub fn has_selection(&self) -> bool {
        !self.selection.is_empty()
    }

    /// Selects only the given node.
    pub fn select_only(&mut self, id: NodeId) {
        self.face_selection.clear();
        self.selection.clear();
        // Clicking one member of a closed group takes the whole group, which is what a
        // group is for.
        self.selection.extend(self.selection_for(id));
        self.mark_selection_changed();
    }

    /// Selects only the given face.
    pub fn select_face_only(&mut self, face: FaceRef) {
        self.selection.clear();
        self.face_selection.clear();
        self.face_selection.insert(face);
        self.mark_selection_changed();
    }

    /// Adds the given face to the selection, or removes it if it is already there.
    ///
    /// Converts selected brushes to their faces before extending a face selection, preserving
    /// everything already selected.
    pub fn toggle_face_selected(&mut self, face: FaceRef) {
        self.convert_selection_to_faces();
        if !self.face_selection.remove(&face) {
            self.face_selection.insert(face);
        }
        self.mark_selection_changed();
    }

    /// Turns a selection of whole brushes into a selection of all their faces,
    /// `convertToFaceSelection`. A selection that holds no brushes is simply dropped, as
    /// a patch or a point entity has no faces to convert to.
    pub fn convert_selection_to_faces(&mut self) -> usize {
        let selected: Vec<NodeId> = self.selection.iter().copied().collect();
        self.selection.clear();
        let mut faces = Vec::new();
        for id in selected {
            let Some(brush) = self.brush(id) else {
                continue;
            };
            faces.extend(
                (0..brush.faces().len())
                    .filter(|index| brush.geometry_face_of(*index).is_some())
                    .map(|face| FaceRef { node: id, face }),
            );
        }
        self.face_selection.extend(faces);
        self.face_selection.len()
    }

    /// Returns whether the given face is selected.
    #[inline]
    pub fn is_face_selected(&self, face: FaceRef) -> bool {
        self.face_selection.contains(&face)
    }

    /// The faces picked one by one, as a list.
    pub fn selected_faces(&self) -> Vec<FaceRef> {
        self.face_selection.iter().copied().collect()
    }

    /// Returns how many faces are selected.
    #[inline]
    pub fn face_selection_count(&self) -> usize {
        self.face_selection.len()
    }

    /// Returns whether any face is selected.
    #[inline]
    pub fn has_face_selection(&self) -> bool {
        !self.face_selection.is_empty()
    }

    /// Synchronizes transient face-tool handles without changing brush selection/history.
    /// The frontend clears these when leaving the Face tool. Invalid or unselected
    /// source brushes never participate; an explicit face selection takes precedence.
    pub fn set_face_edit_targets(&mut self, targets: Vec<FaceRef>) {
        self.face_edit_targets = targets;
    }

    pub fn selected_edit_faces(&self) -> Vec<FaceRef> {
        if !self.face_selection.is_empty() {
            return self.selected_faces();
        }
        self.face_edit_targets
            .iter()
            .copied()
            .filter(|face| self.selection.contains(&face.node) && self.face_at(*face).is_some())
            .collect()
    }

    /// Applies the given change to every selected face.
    ///
    /// With no face selected this falls back to every face of every selected brush, so
    /// the material commands do something useful before a face has been picked.
    pub fn edit_faces(&mut self, mut edit: impl FnMut(&mut map_model::brush_face::BrushFace)) {
        let mut before = Vec::new();
        let mut after = Vec::new();
        let targets: HashSet<_> = self.selected_edit_faces().into_iter().collect();

        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            let whole_brush = targets.is_empty() && self.selection.contains(&id);
            let any_face = (0..brush.geometry().face_count())
                .any(|face| targets.contains(&FaceRef { node: id, face }));
            if !whole_brush && !any_face {
                continue;
            }

            // A pick names a geometry face; the material lives on the brush's own face,
            // and the brush knows which is which.
            let mut faces = brush.faces().to_vec();
            for index in 0..brush.geometry().face_count() {
                let selected = whole_brush
                    || targets.contains(&FaceRef {
                        node: id,
                        face: index,
                    });
                if !selected {
                    continue;
                }
                if let Some(source) = brush.source_face_index(index) {
                    let face = &mut faces[source];
                    let uv_before = (face.uv_attributes, face.uv_coord_system);
                    let pending_before = face.primitive_uv.as_deref().copied();
                    edit(face);
                    // An explicit UV edit replaces the placeholder alignment. Do not
                    // let later material discovery silently restore the old matrix.
                    // Paint/material-only edits retain it, as does an edit supplying
                    // a new pending matrix intentionally.
                    if uv_before != (face.uv_attributes, face.uv_coord_system)
                        && pending_before == face.primitive_uv.as_deref().copied()
                    {
                        face.primitive_uv = None;
                    }
                }
            }

            if faces.as_slice() == brush.faces() {
                continue;
            }
            let Ok(edited) = Brush::from_faces(&self.world_bounds, faces) else {
                continue;
            };
            before.push(brush_node(id, brush.clone()));
            after.push(brush_node(id, edited));
        }

        if !before.is_empty() {
            self.apply_and_record(Edit::Changed { before, after });
        }
    }

    /// The faces standing at the given centres, among the brushes that are selected.
    ///
    /// A face handle sits at the centre of its face
    /// ([`crate::handle_tool::HandleManager::selected_face_centres`]), and picking one
    /// leaves the face selection alone -- the brush has to stay selected for the handles
    /// to exist. This turns those centres back into faces, so a panel can act on what the
    /// face tool has hold of rather than on the whole brush.
    pub fn faces_at(&self, centres: &[Vec3d]) -> Vec<FaceRef> {
        if centres.is_empty() {
            return Vec::new();
        }
        let mut found = Vec::new();
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }
            let geometry = brush.geometry();
            for index in 0..geometry.face_count() {
                let corners = geometry.face_positions(map_model::polyhedron::FaceId(index as u32));
                if corners.is_empty() {
                    continue;
                }
                let centre = corners
                    .iter()
                    .fold(Vec3d::zero(), |total, corner| total + *corner)
                    / corners.len() as f64;
                if centres
                    .iter()
                    .any(|wanted| geometry_math::vec::squared_distance(*wanted, centre) < 0.01)
                {
                    found.push(FaceRef {
                        node: id,
                        face: index,
                    });
                }
            }
        }
        found
    }

    /// Paints a colour on the given faces, whether or not they are the selection.
    ///
    /// Returns how many faces changed. [`Self::set_face_color`] is the same thing for
    /// what is selected; this is for the faces the face tool has hold of.
    pub fn set_face_color_on(
        &mut self,
        faces: &[FaceRef],
        color: Option<map_model::brush_face::FaceColor>,
    ) -> usize {
        self.edit_faces_on(faces, |face| {
            let changed = face.paint() != color;
            face.set_paint(color);
            changed
        })
    }

    /// Edits the given faces, whether or not they are the selection, as one undo step.
    /// `edit` returns whether it changed the face. Returns how many faces changed.
    pub(crate) fn edit_faces_on(
        &mut self,
        faces: &[FaceRef],
        mut edit: impl FnMut(&mut map_model::brush_face::BrushFace) -> bool,
    ) -> usize {
        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut changed = 0;

        let mut by_brush: HashMap<NodeId, Vec<usize>> = HashMap::new();
        for face in faces {
            by_brush.entry(face.node).or_default().push(face.face);
        }

        for (id, indices) in by_brush {
            let Some(brush) = self.brush(id).cloned() else {
                continue;
            };
            let mut painted = brush.faces().to_vec();
            let mut touched = false;
            for index in indices {
                if let Some(source) = brush.source_face_index(index) {
                    let face = &mut painted[source];
                    let uv_before = (face.uv_attributes, face.uv_coord_system);
                    let pending_before = face.primitive_uv.as_deref().copied();
                    if edit(face) {
                        // As in `edit_faces`: an explicit UV edit replaces a pending
                        // brush-primitive alignment.
                        if uv_before != (face.uv_attributes, face.uv_coord_system)
                            && pending_before == face.primitive_uv.as_deref().copied()
                        {
                            face.primitive_uv = None;
                        }
                        touched = true;
                        changed += 1;
                    }
                }
            }
            if !touched {
                continue;
            }
            let Ok(edited) = Brush::from_faces(&self.world_bounds, painted) else {
                continue;
            };
            before.push(brush_node(id, brush));
            after.push(brush_node(id, edited));
        }

        if before.is_empty() {
            return 0;
        }
        self.apply_and_record(Edit::Changed { before, after });
        changed
    }

    /// Paints a colour on every selected face, or takes it off with `None`.
    ///
    /// Face colours override materials when drawing and are stored as face-line `.map` comments.
    /// [`Self::edit_faces`] includes individually selected faces and every face of a selected
    /// brush.
    ///
    /// Returns how many faces changed.
    pub fn set_face_color(&mut self, color: Option<map_model::brush_face::FaceColor>) -> usize {
        let mut changed = 0;
        self.edit_faces(|face| {
            if face.paint() != color {
                face.set_paint(color);
                changed += 1;
            }
        });
        changed
    }

    /// The colour on the first selected face, for a panel to show.
    ///
    /// Several faces can be selected wearing different colours; the first one stands in,
    /// the way the UV editor shows the first selected face's mapping.
    pub fn first_selected_face_color(&self) -> Option<map_model::brush_face::FaceColor> {
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            let whole_brush = self.selection.contains(&id);
            for index in 0..brush.geometry().face_count() {
                let selected = whole_brush
                    || self.is_face_selected(FaceRef {
                        node: id,
                        face: index,
                    });
                if !selected {
                    continue;
                }
                if let Some(source) = brush.source_face_index(index) {
                    return brush.faces()[source].paint();
                }
            }
        }
        None
    }

    pub fn set_export_hidden_on(&mut self, faces: &[FaceRef], hidden: bool) -> usize {
        self.edit_faces_on(faces, |face| {
            let changed = face.export_hidden != hidden;
            face.export_hidden = hidden;
            changed
        })
    }

    /// Sets PBR parameters on explicitly picked faces as one undoable edit.
    pub fn set_surface_material_on(
        &mut self,
        faces: &[FaceRef],
        surface: Option<map_model::brush_face::SurfaceMaterial>,
    ) -> usize {
        self.edit_faces_on(faces, |face| {
            let changed = face.surface_material != surface;
            face.surface_material = surface;
            changed
        })
    }

    /// Sets PBR parameters on the current brush/face selection.
    pub fn set_surface_material(
        &mut self,
        surface: Option<map_model::brush_face::SurfaceMaterial>,
    ) -> usize {
        let mut changed = 0;
        self.edit_faces(|face| {
            if face.surface_material != surface {
                face.surface_material = surface;
                changed += 1;
            }
        });
        changed
    }

    /// Surface parameters on the first selected face, for the inspector.
    pub fn first_selected_surface_material(
        &self,
    ) -> Option<map_model::brush_face::SurfaceMaterial> {
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            let whole_brush = self.selection.contains(&id);
            for index in 0..brush.geometry().face_count() {
                let selected = whole_brush
                    || self.is_face_selected(FaceRef {
                        node: id,
                        face: index,
                    });
                if !selected {
                    continue;
                }
                if let Some(source) = brush.source_face_index(index) {
                    return brush.faces()[source].surface_material;
                }
            }
        }
        None
    }

    /// How many faces a paint would reach: the selected ones, and every face of a brush
    /// selected whole.
    pub fn paintable_face_count(&self) -> usize {
        let mut count = 0;
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            let whole_brush = self.selection.contains(&id);
            for index in 0..brush.geometry().face_count() {
                if whole_brush
                    || self.is_face_selected(FaceRef {
                        node: id,
                        face: index,
                    })
                {
                    count += 1;
                }
            }
        }
        count
    }

    /// Copies a face's material, and optionally its mapping, onto another.
    ///
    /// `transferFaceAttributes`. What is copied is the mode: the material alone, or the
    /// material with the mapping carried across by one of the two wrap styles. Returns
    /// whether anything changed.
    ///
    /// `to_whole_brush` is the double click, which puts it on every face of the brush
    /// under the pointer rather than only the one clicked.
    pub fn copy_face_attributes(
        &mut self,
        source: FaceRef,
        target: FaceRef,
        mode: FaceAttributeTransfer,
        to_whole_brush: bool,
    ) -> bool {
        let Some(source_face) = self.face_at(source).cloned() else {
            return false;
        };

        let world_bounds = self.world_bounds;
        let Some(brush) = self.brush(target.node) else {
            return false;
        };
        let brush = brush.clone();

        let targets: Vec<usize> = if to_whole_brush {
            (0..brush.faces().len()).collect()
        } else {
            match brush.source_face_index(target.face) {
                Some(index) => vec![index],
                None => return false,
            }
        };

        let mut faces = brush.faces().to_vec();
        for index in targets {
            if target.node == source.node && brush.source_face_index(source.face) == Some(index) {
                continue;
            }
            apply_face_attributes(&mut faces[index], &source_face, mode);
        }
        if faces.as_slice() == brush.faces() {
            return false;
        }

        let Ok(edited) = Brush::from_faces(&world_bounds, faces) else {
            return false;
        };
        self.apply_and_record(Edit::Changed {
            before: vec![brush_node(target.node, brush)],
            after: vec![brush_node(target.node, edited)],
        });
        true
    }

    /// Copies a face's attributes onto many faces in one undo step.
    ///
    /// `SetBrushFaceAttributesToolController`'s drag: Alt held, the pointer swept across
    /// faces, each taking the reference face's material and, by the mode, its mapping.
    /// Returns how many faces changed.
    pub fn copy_face_attributes_across(
        &mut self,
        source: FaceRef,
        targets: &[FaceRef],
        mode: FaceAttributeTransfer,
    ) -> usize {
        let Some(source_face) = self.face_at(source).cloned() else {
            return 0;
        };
        let targets: Vec<_> = targets
            .iter()
            .copied()
            .filter(|face| *face != source)
            .collect();
        self.set_faces_from(&source_face, &targets, mode)
    }

    /// Returns the face a reference names.
    pub fn face_at(&self, reference: FaceRef) -> Option<&map_model::brush_face::BrushFace> {
        let brush = self.brush(reference.node)?;
        let source = brush.source_face_index(reference.face)?;
        brush.faces().get(source)
    }

    /// Returns the one selected face, if exactly one is selected.
    ///
    /// `canCopyAttributesFromSelection` requires exactly one: with none there is nothing
    /// to copy and with several there is no saying which.
    pub fn single_selected_face(&self) -> Option<FaceRef> {
        let faces = self.selected_edit_faces();
        if faces.len() == 1 {
            faces.into_iter().next()
        } else {
            None
        }
    }

    /// Adds the given node to the selection, or removes it if it is already there.
    pub fn toggle_selected(&mut self, id: NodeId) {
        let members = self.selection_for(id);
        if members.iter().all(|member| self.selection.contains(member)) {
            for member in members {
                self.selection.remove(&member);
            }
        } else {
            self.selection.extend(members);
        }
        self.mark_selection_changed();
    }

    pub fn selection_snapshot(&self) -> SelectionSnapshot {
        SelectionSnapshot {
            nodes: self.selection.clone(),
            faces: self.face_selection.clone(),
        }
    }

    pub fn restore_selection(&mut self, snapshot: &SelectionSnapshot) {
        if self.selection != snapshot.nodes || self.face_selection != snapshot.faces {
            self.selection = snapshot.nodes.clone();
            self.face_selection = snapshot.faces.clone();
            self.mark_selection_changed();
        }
    }

    /// Recompute from pointer-down, never from the previous preview. Group members
    /// are unioned once so multiple caught brushes cannot toggle their group off.
    pub fn preview_marquee(&mut self, initial: &SelectionSnapshot, caught: &[NodeId], add: bool) {
        let mut nodes = if add {
            initial.nodes.clone()
        } else {
            HashSet::new()
        };
        nodes.extend(self.selection_for_many(caught));
        self.restore_selection(&SelectionSnapshot {
            nodes,
            faces: HashSet::new(),
        });
    }

    /// Clears the selection, whether it holds nodes or faces.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.face_selection.clear();
        self.mark_selection_changed();
    }

    /// Selects everything.
    pub fn select_all(&mut self) {
        // Only what can be edited and is drawn: selecting something on a hidden or locked
        // layer, or something hidden by "Hide", would let the next command change what the
        // user cannot see or has asked to leave alone.
        self.face_selection.clear();
        self.selection = self.selectable_ids().into_iter().collect();
        self.mark_selection_changed();
    }

    /// Every node `select_all` would take: a brush, patch or point entity on a layer that
    /// is neither hidden nor locked, and not itself hidden.
    fn selectable_ids(&self) -> Vec<NodeId> {
        let open = self.open_group_contents();
        let brushes = node::brushes_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, layer)| self.is_editable_on(*id, *layer, &open))
            .map(|(id, _, _)| id);
        let patches = node::patches_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, layer)| self.is_editable_on(*id, *layer, &open))
            .map(|(id, _, _)| id);
        let layers: HashMap<NodeId, usize> = node::point_entities_by_layer(&self.tree.world)
            .into_iter()
            .map(|(id, _, layer)| (id, layer))
            .collect();
        let entities = self
            .entity_boxes()
            .into_iter()
            .filter(|entity| {
                layers
                    .get(&entity.node)
                    .is_some_and(|layer| self.is_editable_on(entity.node, *layer, &open))
            })
            .map(|entity| entity.node);
        brushes.chain(patches).chain(entities).collect()
    }

    /// Selects every selectable node that is not already selected.
    ///
    /// `invertNodeSelection`. The flat node model here has no partially selected parent
    /// to reason about, so this is the plain complement over what `select_all` would
    /// take.
    pub fn invert_selection(&mut self) {
        self.selection = self
            .selectable_ids()
            .into_iter()
            .filter(|id| !self.selection.contains(id))
            .collect();
        self.mark_selection_changed();
    }

    /// The box a patch or a point entity is bounded by, for the select-by-brush tests.
    fn node_bounds(&self, id: NodeId) -> Option<Bbox3d> {
        if let Some((_, patch, _)) = node::patches_by_layer(&self.tree.world)
            .into_iter()
            .find(|(patch_id, _, _)| *patch_id == id)
        {
            return Bbox3d::merge_all(patch.control_points.iter().map(|point| point.position));
        }
        self.entity_boxes()
            .into_iter()
            .find(|entity| entity.node == id)
            .map(|entity| entity.bounds)
    }

    /// Whether every selected node is a brush, which is what the select-by-brush actions
    /// need. `Map::selection().hasOnlyBrushes()`.
    pub(super) fn selection_is_only_brushes(&self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let brush_ids: HashSet<NodeId> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        self.selection.iter().all(|id| brush_ids.contains(id))
    }

    /// Replaces a brush-only selection with every node those brushes reach, consuming the
    /// brushes in the process.
    ///
    /// `test` decides what "reach" means: touching for `selectTouchingNodes`, wholly
    /// containing for `selectContainedNodes`. Both delete the source brushes first, so
    /// the drawn brush is a throwaway lasso, and both fold the delete and the reselect
    /// into one undo step.
    fn select_by_brush(
        &mut self,
        touch: impl Fn(&Brush, &Brush) -> bool,
        reach_bounds: impl Fn(&Brush, &Bbox3d) -> bool,
    ) -> bool {
        let lasso: Vec<Brush> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .map(|(_, brush)| brush.clone())
            .collect();
        self.select_by_lasso(lasso, touch, reach_bounds)
    }

    /// The shared body of the select-by-brush actions: with a brush-only selection, find
    /// every node the `lasso` brushes reach, delete the selection and select what was
    /// found, all in one undo step.
    fn select_by_lasso(
        &mut self,
        lasso: Vec<Brush>,
        touch: impl Fn(&Brush, &Brush) -> bool,
        reach_bounds: impl Fn(&Brush, &Bbox3d) -> bool,
    ) -> bool {
        if !self.selection_is_only_brushes() {
            return false;
        }

        let mut to_select: Vec<NodeId> = Vec::new();
        for (id, brush, layer) in node::brushes_by_layer(&self.tree.world) {
            if self.selection.contains(&id) || !self.layer_is_editable(layer) {
                continue;
            }
            if lasso.iter().any(|source| touch(source, brush)) {
                to_select.push(id);
            }
        }
        for id in self.selectable_ids() {
            if self.selection.contains(&id) || to_select.contains(&id) {
                continue;
            }
            let Some(bounds) = self.node_bounds(id) else {
                continue;
            };
            if lasso.iter().any(|source| reach_bounds(source, &bounds)) {
                to_select.push(id);
            }
        }

        let doomed: Vec<Node> = self
            .selection
            .iter()
            .filter_map(|id| node::find_node(&self.tree.world, *id).cloned())
            .collect();
        self.apply_and_record(Edit::Removed {
            layer: 0,
            nodes: doomed,
        });

        self.selection = to_select.into_iter().collect();
        self.mark_selection_changed();
        true
    }

    /// Selects every node the selected brushes touch, deleting the brushes.
    /// `selectTouchingNodes(map, true)`, bound to Ctrl+T.
    pub fn select_touching(&mut self) -> bool {
        self.select_by_brush(Brush::intersects, Brush::intersects_bbox)
    }

    /// Selects every node the selected brushes wholly contain, deleting the brushes.
    /// `selectContainedNodes(map, true)`, bound to Ctrl+E.
    pub fn select_inside(&mut self) -> bool {
        self.select_by_brush(Brush::contains_brush, Brush::contains_bbox)
    }

    /// Selects every node inside the selected brushes stretched the full height of the
    /// world along `axis`, deleting the brushes.
    ///
    /// `selectTouchingNodes(map, cameraAxis, true)` -- the 2D view's "Select Tall", where
    /// the drawn footprint is extruded along the axis the view looks down and everything
    /// it then contains is selected. `axis` is a world axis index, from the flat view the
    /// command was invoked in.
    pub fn select_tall(&mut self, axis: usize) -> bool {
        if !self.selection_is_only_brushes() || axis > 2 {
            return false;
        }

        let low = self.world_bounds.min[axis] + 1.0;
        let high = self.world_bounds.max[axis] - 1.0;
        let mut tall: Vec<Brush> = Vec::new();
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }
            let mut points: Vec<Vec3d> = Vec::new();
            for mut vertex in brush.vertex_positions() {
                vertex[axis] = low;
                points.push(vertex);
                vertex[axis] = high;
                points.push(vertex);
            }
            let Ok(geometry) = Polyhedron::from_points(&points, f64::ALMOST_ZERO) else {
                return false;
            };
            tall.push(Brush::from_polyhedron(
                geometry,
                map_model::shapes::DEFAULT_MATERIAL,
            ));
        }

        self.select_by_lasso(tall, Brush::contains_brush, Brush::contains_bbox)
    }

    /// Extends the selection to every node sharing a container with something selected.
    ///
    /// `selectSiblingNodes`, bound to Ctrl+B: select one brush of a group and this takes
    /// the rest, or one brush on a layer and this takes the layer. What is not on an
    /// editable layer is left out.
    pub fn select_siblings(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let seeds: Vec<NodeId> = self.selection.iter().copied().collect();
        let mut result: HashSet<NodeId> = HashSet::new();
        for seed in seeds {
            result.extend(node::siblings_of(&self.tree.world, seed));
        }
        result.retain(|id| {
            node::layer_of(&self.tree.world, *id).is_some_and(|layer| self.layer_is_editable(layer))
        });
        self.selection = result;
        self.mark_selection_changed();
    }

    /// A double click on `hit`, or on empty space: `SelectionTool::mouseDoubleClick`.
    ///
    /// Shift and Alt take the faces connected to the clicked one in its plane, Shift alone
    /// every face of the brush, and Ctrl adds them to the face selection. A plain double
    /// click opens the closed group the hit is in, or selects everything sharing the hit's
    /// container -- its layer, group or brush entity -- added to the selection with Ctrl;
    /// with a group open, one on anything outside it, or on empty space, closes it.
    /// Returns whether anything changed.
    pub fn double_click(
        &mut self,
        hit: Option<Picked>,
        shift: bool,
        alt: bool,
        ctrl: bool,
    ) -> bool {
        if shift {
            let Some(face) = hit.and_then(|hit| {
                hit.face.map(|face| FaceRef {
                    node: hit.node,
                    face,
                })
            }) else {
                return false;
            };
            let faces = if alt {
                self.connected_coplanar_faces(face)
            } else {
                let count = self.brush(face.node).map_or(0, |brush| brush.faces().len());
                (0..count)
                    .map(|face_index| FaceRef {
                        node: face.node,
                        face: face_index,
                    })
                    .collect()
            };
            if !ctrl {
                self.selection.clear();
                self.face_selection.clear();
            } else {
                self.convert_selection_to_faces();
            }
            self.face_selection.extend(faces);
            self.mark_selection_changed();
            return true;
        }
        let Some(hit) = hit else {
            return self.close_group();
        };
        if let Some(group) = node::outermost_group_of(&self.tree.world, hit.node, self.open_group) {
            self.open_group = Some(group);
            self.selection.clear();
            self.face_selection.clear();
            self.mark_changed();
            return true;
        }
        let siblings = node::siblings_of(&self.tree.world, hit.node);
        if !ctrl {
            self.selection.clear();
        }
        self.face_selection.clear();
        let open = self.open_group_contents();
        for id in siblings {
            if !node::layer_of(&self.tree.world, id)
                .is_some_and(|layer| self.is_editable_on(id, layer, &open))
            {
                continue;
            }
            // A group among the siblings is selected as its contents, as a click on it is.
            let group = node::find_node(&self.tree.world, id)
                .is_some_and(|node| matches!(node.kind, map_model::node::NodeKind::Group(_)));
            if group {
                self.selection
                    .extend(node::contents_of(&self.tree.world, id));
            } else {
                self.selection.extend(self.selection_for(id));
            }
        }
        self.mark_selection_changed();
        true
    }

    /// Every face that lies in the plane of `start` and can be reached from it across
    /// shared edges, on any editable brush: `collectConnectedCoplanarFaces`.
    pub fn connected_coplanar_faces(&self, start: FaceRef) -> Vec<FaceRef> {
        const EPSILON: f64 = 0.01;
        let Some(start_polygon) = self
            .brush(start.node)
            .and_then(|b| b.face_polygon(start.face))
        else {
            return Vec::new();
        };
        let normal = self
            .brush(start.node)
            .map(|b| b.faces()[start.face].normal());
        let Some(normal) = normal else {
            return Vec::new();
        };
        let distance = geometry_math::vec::dot(normal, start_polygon[0]);
        let open = self.open_group_contents();
        // Every coplanar face of an editable brush, with its edges.
        let mut candidates: Vec<(FaceRef, Vec<(Vec3d, Vec3d)>)> = Vec::new();
        for (id, brush, layer) in node::brushes_by_layer(&self.tree.world) {
            if !self.is_editable_on(id, layer, &open) {
                continue;
            }
            for (index, face) in brush.faces().iter().enumerate() {
                let n = face.normal();
                if geometry_math::vec::length(n - normal) > EPSILON {
                    continue;
                }
                let Some(polygon) = brush.face_polygon(index) else {
                    continue;
                };
                if (geometry_math::vec::dot(normal, polygon[0]) - distance).abs() > EPSILON {
                    continue;
                }
                let edges = (0..polygon.len())
                    .map(|i| (polygon[i], polygon[(i + 1) % polygon.len()]))
                    .collect();
                candidates.push((
                    FaceRef {
                        node: id,
                        face: index,
                    },
                    edges,
                ));
            }
        }
        let Some(first) = candidates.iter().position(|(face, _)| *face == start) else {
            return vec![start];
        };
        let mut reached = vec![false; candidates.len()];
        reached[first] = true;
        let mut queue = vec![first];
        while let Some(at) = queue.pop() {
            for other in 0..candidates.len() {
                if reached[other] {
                    continue;
                }
                let touching = candidates[at].1.iter().any(|a| {
                    candidates[other]
                        .1
                        .iter()
                        .any(|b| segments_overlap(*a, *b, EPSILON))
                });
                if touching {
                    reached[other] = true;
                    queue.push(other);
                }
            }
        }
        candidates
            .into_iter()
            .zip(reached)
            .filter(|(_, reached)| *reached)
            .map(|((face, _), _)| face)
            .collect()
    }

    /// Selects every brush that wears the given material on any face.
    ///
    /// Selects brushes using a material, comparing names case-insensitively.
    pub fn select_brushes_with_material(&mut self, material: &str) {
        self.face_selection.clear();
        let open = self.open_group_contents();
        self.selection = node::brushes_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, layer)| self.is_editable_on(*id, *layer, &open))
            .filter(|(_, brush, _)| {
                brush
                    .faces()
                    .iter()
                    .any(|face| face.material_name.eq_ignore_ascii_case(material))
            })
            .map(|(id, _, _)| id)
            .collect();
        self.mark_selection_changed();
    }

    /// Selects every *face* wearing the given material, `selectBrushFacesWithMaterial`.
    ///
    /// The other half of what the material browser offers: the brushes that use a
    /// material, or the faces that do. Selecting the faces is what lets the next command
    /// change only the surfaces that wear it and leave the rest of each brush alone.
    /// Returns how many faces were taken.
    pub fn select_faces_with_material(&mut self, material: &str) -> usize {
        self.selection.clear();
        self.face_selection.clear();
        let open = self.open_group_contents();
        for (id, brush, layer) in node::brushes_by_layer(&self.tree.world) {
            if !self.is_editable_on(id, layer, &open) {
                continue;
            }
            // A pick names a geometry face; the material lives on the brush's own face,
            // and the brush knows which is which.
            for index in 0..brush.geometry().face_count() {
                let wears = brush
                    .source_face_index(index)
                    .and_then(|source| brush.faces().get(source))
                    .is_some_and(|face| face.material_name.eq_ignore_ascii_case(material));
                if wears {
                    self.face_selection.insert(FaceRef {
                        node: id,
                        face: index,
                    });
                }
            }
        }
        self.mark_selection_changed();
        self.face_selection.len()
    }

    /// Returns the smallest box containing the selection, or `None` if nothing is
    /// selected.
    pub fn selection_bounds(&self) -> Option<Bbox3d> {
        let brushes = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .map(|(_, brush)| brush.bounds());
        // A Bézier patch stays inside the convex hull of its control points, so their bounds
        // contain its surface.
        let patches = node::patches_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, _)| self.selection.contains(id))
            .filter_map(|(_, patch, _)| {
                Bbox3d::merge_all(patch.control_points.iter().map(|point| point.position))
            });
        let entities = self
            .entity_boxes()
            .into_iter()
            .filter(|entity| self.selection.contains(&entity.node))
            .map(|entity| entity.bounds);

        brushes
            .chain(patches)
            .chain(entities)
            .reduce(|acc, bounds| acc.merged(&bounds))
    }

    /// Returns what the given ray hits first, whether a brush or a point entity.
    pub fn pick(&self, ray: &Ray3d) -> Option<Picked> {
        self.pick_matching(ray, |_| true, None).into_iter().next()
    }

    /// Hits selected geometry even when an unselected object is in front of it.
    /// Orthographic move gestures use this to retain the existing selection.
    pub fn pick_selected(&self, ray: &Ray3d) -> Option<Picked> {
        self.pick_matching(ray, |id| self.is_selected(id), None)
            .into_iter()
            .next()
    }

    /// What a click in a flat view picks along `ray`: the hit whose face looks smallest
    /// from the view, nearest of those alike, as `MapView2D::pick` sorts hits by size.
    /// A crate behind a wall in the front view is what a click on it means, though the
    /// wall is nearer.
    pub fn pick_flat(&self, ray: &Ray3d) -> Option<Picked> {
        self.pick_matching(ray, |_| true, Some(flat_axis(ray)))
            .into_iter()
            .next()
    }

    /// [`Self::pick_flat`] among the selected objects alone.
    pub fn pick_selected_flat(&self, ray: &Ray3d) -> Option<Picked> {
        self.pick_matching(ray, |id| self.is_selected(id), Some(flat_axis(ray)))
            .into_iter()
            .next()
    }

    /// Every hit, sorted by distance, or with `by_size` the axis a flat view looks along,
    /// by how big each looks along it first.
    fn pick_matching(
        &self,
        ray: &Ray3d,
        accepts: impl Fn(NodeId) -> bool,
        by_size: Option<usize>,
    ) -> Vec<Picked> {
        // How big a hit looks along the flat view's axis; nothing to sort by in 3D.
        let looks = |area: &dyn Fn(usize) -> f64| by_size.map_or(0.0, area);
        // A hidden layer cannot be seen and a locked one cannot be changed, so neither is
        // what a click meant; the same for something hidden by "Hide", and for anything
        // outside a group that is open.
        let brushes = pick::pick(&self.tree.world, ray)
            .into_iter()
            .filter(|hit| self.is_editable(hit.node) && accepts(hit.node))
            .map(|hit| {
                let size = looks(&|axis| {
                    let mut area = 0.0;
                    if let Some(brush) = self.brush(hit.node) {
                        brush.for_each_geometry_face(|index, _, polygon| {
                            if index == hit.face {
                                area = pick::projected_area(polygon, axis);
                            }
                        });
                    }
                    area
                });
                let picked = Picked {
                    node: hit.node,
                    face: Some(hit.face),
                    distance: hit.distance,
                };
                (picked, size)
            });

        // A point entity has no geometry, so it is picked by the box it is drawn as.
        // The ray is asked first: it is cheap, and few boxes are on it, where whether an
        // entity can be edited walks the tree.
        let entities = self
            .entity_boxes()
            .into_iter()
            .filter_map(|entity| {
                geometry_math::intersection::intersect_ray_bbox(ray, &entity.bounds).map(
                    |distance| {
                        let picked = Picked {
                            node: entity.node,
                            face: None,
                            distance,
                        };
                        let size = looks(&|axis| pick::projected_box_area(&entity.bounds, axis));
                        (picked, size)
                    },
                )
            })
            .filter(|(hit, _)| self.is_editable(hit.node) && accepts(hit.node));

        // A patch is picked against the surface it is drawn as, since that is what a
        // click is aiming at; its control points sit off the surface and are not it.
        let open = self.open_group_contents();
        let patches = node::patches_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, layer)| self.is_editable_on(*id, *layer, &open) && accepts(*id))
            .filter_map(|(id, patch, _)| {
                let triangles = map_model::patch_mesh::tessellate(
                    patch,
                    map_model::patch_mesh::DEFAULT_SUBDIVISIONS,
                );
                let distance = triangles
                    .iter()
                    .filter_map(|triangle| {
                        geometry_math::intersection::intersect_ray_triangle(
                            ray,
                            triangle.positions[0],
                            triangle.positions[1],
                            triangle.positions[2],
                        )
                    })
                    .min_by(|a, b| a.total_cmp(b))?;
                let size = looks(&|axis| {
                    Bbox3d::merge_all(triangles.iter().flat_map(|t| t.positions))
                        .map_or(0.0, |bounds| pick::projected_box_area(&bounds, axis))
                });
                let picked = Picked {
                    node: id,
                    face: None,
                    distance,
                };
                Some((picked, size))
            });

        let mut hits: Vec<(Picked, f64)> = brushes.chain(entities).chain(patches).collect();
        hits.sort_by(|(a, size_a), (b, size_b)| {
            size_a
                .total_cmp(size_b)
                .then(a.distance.total_cmp(&b.distance))
        });
        hits.into_iter().map(|(hit, _)| hit).collect()
    }

    /// Ctrl and the wheel: the selection moves to the next thing under the pointer along
    /// `ray`, or the one before going back (`SelectionTool::drillSelection`). What is in a
    /// closed group counts as the group. Hits are in depth order, or in a flat view by how
    /// small they look. Returns whether the selection changed.
    pub fn drill_selection(&mut self, ray: &Ray3d, forward: bool, flat: bool) -> bool {
        let hits = self.pick_matching(ray, |_| true, flat.then(|| flat_axis(ray)));
        // Each hit as what a click on it selects, the same group once.
        let mut targets: Vec<Vec<NodeId>> = Vec::new();
        for hit in hits {
            let mut target = self.selection_for(hit.node);
            target.sort();
            if !targets.contains(&target) {
                targets.push(target);
            }
        }
        if !forward {
            targets.reverse();
        }
        let Some(first) = targets
            .iter()
            .position(|target| target.iter().all(|id| self.selection.contains(id)))
        else {
            return false;
        };
        let Some(next) = targets.get(first + 1) else {
            return false;
        };
        for id in &targets[first] {
            self.selection.remove(id);
        }
        self.selection.extend(next.iter().copied());
        self.mark_selection_changed();
        true
    }
}

/// The axis a flat view looks along: the ray's largest component.
fn flat_axis(ray: &Ray3d) -> usize {
    let d = ray.direction;
    (0..3)
        .max_by(|a, b| d[*a].abs().total_cmp(&d[*b].abs()))
        .unwrap_or(2)
}

/// Whether two segments lie on one line and share more than a point:
/// `vm::segments_overlap`.
fn segments_overlap(a: (Vec3d, Vec3d), b: (Vec3d, Vec3d), epsilon: f64) -> bool {
    use geometry_math::vec;
    let along = a.1 - a.0;
    let length = vec::length(along);
    if length <= epsilon {
        return false;
    }
    let direction = along / length;
    // Both ends of `b` on `a`'s line.
    let off_line = |p: Vec3d| {
        let t = vec::dot(p - a.0, direction);
        vec::length(p - (a.0 + direction * t))
    };
    if off_line(b.0) > epsilon || off_line(b.1) > epsilon {
        return false;
    }
    let (s0, s1) = (
        vec::dot(b.0 - a.0, direction),
        vec::dot(b.1 - a.0, direction),
    );
    let (low, high) = (s0.min(s1), s0.max(s1));
    high.min(length) - low.max(0.0) > epsilon
}
