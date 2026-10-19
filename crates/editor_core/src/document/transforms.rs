//! Document operations for transforms.

use map_model::brush::Brush;
use map_model::material::MaterialCollection;

/// [`Document::material_size`] from the collection alone, for work spread over threads.
pub(super) fn material_size_in(materials: &MaterialCollection, name: &str) -> geometry_math::vec::Vec2f {
    match materials.get(name) {
        Some(material) => geometry_math::vec::Vec2f::new(material.width.max(1) as f32, material.height.max(1) as f32),
        None => geometry_math::vec::Vec2f::new(1.0, 1.0),
    }
}

use super::{Axis, Document, Edit, HashSet, Mat4x4d, Node, NodeId, NodeKind, Repeatable, Vec3d, brush_node, entity_node, mat_ext, node, patch_node, scalar};

impl Document {
    /// Moves everything selected by the given delta.
    ///
    /// Returns whether the move was applied. A move that would leave any selected brush
    /// invalid is refused in full rather than in part, so the selection cannot end up
    /// half moved.
    pub fn translate_selection(&mut self, delta: Vec3d) -> bool {
        if self.selection.is_empty() || delta == Vec3d::zero() {
            return false;
        }

        // Build every replacement before applying any of them, so a failure part way
        // through leaves the document as it was. The entities go first, for the reason
        // `transform_selection` gives.
        let translation = mat_ext::translation_matrix(delta);
        let (mut before, mut after): (Vec<Node>, Vec<Node>) =
            self.transformed_entities(&translation).into_iter().unzip();
        let (world_bounds, lock, materials) = (self.world_bounds, self.alignment_lock(), &self.materials);
        let Some(moved) = self.map_selected_brushes(|brush| {
            brush.translate_with_alignment(&world_bounds, delta, lock, |name| material_size_in(materials, name)).ok()
        }) else {
            return false;
        };
        self.push_brush_changes(moved, &mut before, &mut after);

        // A patch moves by the same matrix, which for a move is a pure translation.
        for (was, is) in self.transformed_patches(&translation) {
            before.push(was);
            after.push(is);
        }

        if after.is_empty() {
            return false;
        }

        self.apply_and_record(Edit::Changed { before, after });
        self.remember_repeatable(Repeatable::Transform(translation));
        true
    }

    /// `change` applied to every selected brush, in document order, or `None` if it
    /// refused any of them. Spread over the cores for a big selection: turning ten
    /// thousand brushes is ten thousand independent rebuilds.
    fn map_selected_brushes<'a>(
        &'a self,
        change: impl Fn(&Brush) -> Option<Brush> + Sync,
    ) -> Option<Vec<(NodeId, &'a Brush, Brush)>> {
        use rayon::prelude::*;
        let selected: Vec<(NodeId, &Brush)> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .collect();
        let changed: Option<Vec<Brush>> = if selected.len() >= 256 {
            selected.par_iter().map(|(_, brush)| change(brush)).collect()
        } else {
            selected.iter().map(|(_, brush)| change(brush)).collect()
        };
        Some(selected.into_iter().zip(changed?).map(|((id, was), now)| (id, was, now)).collect())
    }

    /// Adds brush changes to an edit's two lists. Inside a transaction only the new
    /// state is applied and nothing is recorded (the commit compares with the state the
    /// transaction began from), so the old brushes are not copied: on a drag of thousands
    /// of brushes that copy was half the cost of every pointer move.
    fn push_brush_changes(&self, changes: Vec<(NodeId, &Brush, Brush)>, before: &mut Vec<Node>, after: &mut Vec<Node>) {
        let record = self.transaction.is_none();
        for (id, was, now) in changes {
            if record {
                before.push(brush_node(id, was.clone()));
            }
            after.push(brush_node(id, now));
        }
    }

    /// Every selected patch, before and after the given transformation.
    ///
    /// A patch is its control points, so transforming it is transforming those; the
    /// surface follows because it is drawn from them.
    fn transformed_patches(&self, transform: &Mat4x4d) -> Vec<(Node, Node)> {
        node::patches_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, _)| self.selection.contains(id))
            .map(|(id, patch, _)| {
                let mut moved = patch.clone();
                for point in &mut moved.control_points {
                    point.position = *transform * point.position;
                }
                (patch_node(id, patch.clone()), patch_node(id, moved))
            })
            .collect()
    }

    /// Every selected entity, before and after the given transformation.
    ///
    /// `Entity::transform`: a point entity's origin moves with the middle of the box it
    /// is drawn as rather than with the origin itself, since that is what a person sees
    /// being moved. The facing follows in whichever property the class writes it in --
    /// see [`map_model::entity_rotation`] -- and only when there is a rotation to apply, since
    /// writing one has side effects even when it changes nothing.
    fn transformed_entities(&self, transform: &Mat4x4d) -> Vec<(Node, Node)> {
        let rotation = mat_ext::strip_translation(transform);
        let turning = rotation != Mat4x4d::identity();

        node::entities_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .filter_map(|(id, entity)| {
                let definition = entity
                    .classname()
                    .and_then(|name| self.definitions.as_ref()?.find(name));
                let point_entity = entity.origin().is_some();
                let mut moved = entity.clone();
                let mut changed = false;

                if let Some(origin) = entity.origin() {
                    let offset = definition
                        .and_then(|d| d.bounds)
                        .map_or_else(Vec3d::zero, |bounds| bounds.center());
                    let new_origin = *transform * (origin + offset) - offset;
                    if new_origin != origin {
                        moved.set_origin(new_origin);
                        changed = true;
                    }
                }

                if turning {
                    let declared: Vec<String> = definition
                        .map(|d| d.properties.iter().map(|p| p.key.clone()).collect())
                        .unwrap_or_default();
                    let facts = map_model::entity_rotation::EntityFacts {
                        classname: entity.classname().unwrap_or(""),
                        point_entity,
                        definition_bounds_center: definition
                            .and_then(|d| d.bounds)
                            .map_or_else(Vec3d::zero, |bounds| bounds.center()),
                        has_target: self.entity_has_target(entity, definition),
                        definition_keys: &declared,
                    };
                    let info = map_model::entity_rotation::rotation_info(&moved.properties, &facts);
                    if let Some((key, value)) = map_model::entity_rotation::apply_rotation(
                        &moved.properties,
                        &info,
                        &rotation,
                    ) {
                        moved.set_property(&key, &value);
                        changed = true;
                    }
                }

                changed.then(|| (entity_node(id, entity.clone()), entity_node(id, moved)))
            })
            .collect()
    }

    /// Whether the entity points at another one.
    ///
    /// `hasTarget`: a spotlight with a target aims at it rather than by an angle, so its
    /// facing is not something a rotation should rewrite.
    pub(super) fn entity_has_target(
        &self,
        entity: &map_model::node::EntityNode,
        definition: Option<&map_model::fgd::EntityDefinition>,
    ) -> bool {
        let Some(definition) = definition else {
            return false;
        };
        definition
            .properties
            .iter()
            .filter(|property| property.kind == map_model::fgd::PropertyType::TargetDestination)
            .any(|property| entity.property(&property.key).is_some())
    }

    /// Applies a transform to everything selected.
    ///
    /// Returns whether it was applied. A transform that would leave any selected brush
    /// invalid is refused in full, so the selection cannot end up half transformed.
    pub fn transform_selection(&mut self, transform: &Mat4x4d) -> bool {
        // An entity node carries its own brushes, so putting one back puts back the
        // brushes it had. It goes in before them, so that a brush changed by the same
        // edit is the one that lands.
        let (mut before, mut after): (Vec<Node>, Vec<Node>) =
            self.transformed_entities(transform).into_iter().unzip();

        let (world_bounds, lock, materials) = (self.world_bounds, self.alignment_lock(), &self.materials);
        let Some(transformed) = self.map_selected_brushes(|brush| {
            brush.transform_with_alignment(&world_bounds, transform, lock, |name| material_size_in(materials, name)).ok()
        }) else {
            return false;
        };
        self.push_brush_changes(transformed, &mut before, &mut after);

        for (was, is) in self.transformed_patches(transform) {
            before.push(was);
            after.push(is);
        }

        if after.is_empty() {
            return false;
        }
        self.apply_and_record(Edit::Changed { before, after });
        self.remember_repeatable(Repeatable::Transform(*transform));
        true
    }

    /// Rotates the selection about its own centre.
    ///
    /// Rotating about the centre rather than the origin is what makes the operation feel
    /// local: whatever is selected turns in place.
    pub fn rotate_selection(&mut self, axis: Vec3d, degrees: f64) -> bool {
        let Some(bounds) = self.selection_bounds() else {
            return false;
        };
        self.rotate_selection_about(bounds.center(), axis, degrees)
    }

    /// Turns the selection about the given point.
    ///
    /// `rotateSelection` in `Map_Geometry`, which the rotate tool calls with the point its
    /// ring is drawn around rather than with the selection's own middle.
    pub fn rotate_selection_about(&mut self, center: Vec3d, axis: Vec3d, degrees: f64) -> bool {
        let transform = mat_ext::translation_matrix(center)
            * mat_ext::rotation_matrix_about_axis(axis, scalar::to_radians(degrees))
            * mat_ext::translation_matrix(-center);
        self.transform_selection(&transform)
    }

    /// Mirrors the selection about its own centre, along the given axis.
    pub fn flip_selection(&mut self, axis: Axis) -> bool {
        let Some(bounds) = self.selection_bounds() else {
            return false;
        };
        let center = bounds.center();

        let transform = mat_ext::translation_matrix(center)
            * mat_ext::mirror_matrix::<f64>(axis)
            * mat_ext::translation_matrix(-center);
        self.transform_selection(&transform)
    }

    /// Copies the selection, offset by one step of the grid, and selects the copies.
    ///
    /// Offsetting rather than copying in place is what makes a duplicate visible: two
    /// brushes in the same spot look like one.
    pub fn duplicate_selection(&mut self, grid_size: f64) -> usize {
        self.duplicate_selection_by(Vec3d::new(grid_size, 0.0, 0.0))
    }

    /// Copies the selection, offset by the given delta, and selects the copies.
    ///
    /// Creates duplicates at their final positions so duplication and movement form one undo step.
    pub fn duplicate_selection_by(&mut self, delta: Vec3d) -> usize {
        if !delta.v.iter().all(|v| v.is_finite()) { return 0; }
        let roots: HashSet<NodeId> = self.selection.iter().map(|id| {
            node::outermost_group_of(&self.tree.world, *id, self.open_group).unwrap_or(*id)
        }).collect();
        let mut selected = HashSet::new();

        fn copy_node(document: &Document, source: &Node, delta: Vec3d,
            selected: &mut HashSet<NodeId>) -> Option<Node>
        {
            let mut copy = Node::new(source.kind.clone());
            match &mut copy.kind {
                NodeKind::Brush(brush) => {
                    *brush = brush.translate(&document.world_bounds, delta).ok()?;
                    selected.insert(copy.id);
                }
                NodeKind::Patch(patch) => {
                    for point in &mut patch.control_points {
                        point.position += delta;
                        if !document.world_bounds.contains_point(point.position) { return None; }
                    }
                    selected.insert(copy.id);
                }
                NodeKind::Entity(entity) => {
                    if let Some(origin) = entity.origin() {
                        let moved = origin + delta;
                        if !document.world_bounds.contains_point(moved) { return None; }
                        entity.set_origin(moved);
                    }
                    entity.children = entity.children.iter()
                        .map(|child| copy_node(document, child, delta, selected)).collect::<Option<_>>()?;
                    selected.insert(copy.id);
                }
                NodeKind::Group(group) => {
                    group.id = document.fresh_id();
                    // Ordinary duplication creates an independent copy. Linked copies
                    // have their own explicit command.
                    group.linked_group_id = None;
                    group.transformation = None;
                    group.children = group.children.iter()
                        .map(|child| copy_node(document, child, delta, selected)).collect::<Option<_>>()?;
                }
                NodeKind::Layer(_) => return None,
            }
            Some(copy)
        }

        fn duplicate_children(document: &Document, source: &Node, roots: &HashSet<NodeId>,
            delta: Vec3d, selected: &mut HashSet<NodeId>) -> Option<Option<Node>>
        {
            let children = match &source.kind {
                NodeKind::Group(g) => &g.children,
                NodeKind::Entity(e) => &e.children,
                NodeKind::Layer(l) => &l.children,
                _ => return Some(None),
            };
            let mut updated = Vec::new();
            let mut changed = false;
            for child in children {
                if roots.contains(&child.id) {
                    updated.push(child.clone());
                    updated.push(copy_node(document, child, delta, selected)?);
                    changed = true;
                } else if let Some(replacement) = duplicate_children(document, child, roots, delta, selected)? {
                    updated.push(replacement);
                    changed = true;
                } else {
                    updated.push(child.clone());
                }
            }
            if !changed { return Some(None); }
            let mut replacement = source.clone();
            match &mut replacement.kind {
                NodeKind::Group(g) => g.children = updated,
                NodeKind::Entity(e) => e.children = updated,
                NodeKind::Layer(l) => l.children = updated,
                _ => unreachable!(),
            }
            Some(Some(replacement))
        }

        let mut edits = Vec::new();
        for (layer, source_layer) in self.tree.world.layers.iter().enumerate() {
            let mut additions = Vec::new();
            for source in &source_layer.children {
                if roots.contains(&source.id) {
                    let Some(copy) = copy_node(self, source, delta, &mut selected) else { return 0; };
                    additions.push(copy);
                } else {
                    let Some(changed) = duplicate_children(self, source, &roots, delta, &mut selected) else { return 0; };
                    if let Some(after) = changed {
                        edits.push(Edit::Changed { before: vec![source.clone()], after: vec![after] });
                    }
                }
            }
            if !additions.is_empty() { edits.push(Edit::Added { layer, nodes: additions }); }
        }
        if edits.is_empty() { return 0; }
        let count = selected.len();
        self.selection = selected;
        self.face_selection.clear();
        let edit = if edits.len() == 1 { edits.pop().unwrap() } else { Edit::Batch(edits) };
        self.apply_and_record(edit);
        self.remember_repeatable(Repeatable::Duplicate(delta));
        count
    }
}
