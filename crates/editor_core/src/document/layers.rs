//! Document operations for layers.

use super::node;
use super::Document;
use super::Edit;
use super::HashSet;
use super::LayerSummary;
use super::Node;
use super::NodeId;
use super::NodeKind;

impl Document {
    /// A summary of every layer, for the panel to show.
    pub fn layers(&self) -> Vec<LayerSummary> {
        let counts = self.counts_per_layer();
        self.tree
            .world
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| LayerSummary {
                index,
                name: layer.name.clone(),
                // The first layer is the worldspawn itself, which cannot be removed and
                // has no `_tb_id` of its own.
                is_default: index == 0,
                hidden: layer.hidden,
                locked: layer.locked,
                omit_from_export: layer.omit_from_export,
                objects: counts.get(index).copied().unwrap_or(0),
            })
            .collect()
    }

    /// How many objects sit on each layer.
    fn counts_per_layer(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.tree.world.layers.len()];
        for (_, _, layer) in node::brushes_by_layer(&self.tree.world) {
            counts[layer] += 1;
        }
        for (_, _, layer) in node::point_entities_by_layer(&self.tree.world) {
            counts[layer] += 1;
        }
        counts
    }

    /// Which layer a newly drawn brush goes onto.
    pub fn current_layer(&self) -> usize {
        self.current_layer
            .min(self.tree.world.layers.len().saturating_sub(1))
    }

    /// Sets which layer a newly drawn brush goes onto.
    pub fn set_current_layer(&mut self, index: usize) {
        if index < self.tree.world.layers.len() {
            self.current_layer = index;
        }
    }

    /// Adds an empty layer and makes it the current one, as one undo step.
    pub fn add_layer(&mut self, name: impl Into<String>) -> usize {
        let sort_index = self
            .tree
            .world
            .layers
            .iter()
            .map(|layer| layer.sort_index)
            .max()
            .unwrap_or(0)
            + 1;
        let layer = map_model::node::Layer {
            // A `_tb_id` distinguishes this layer from the worldspawn when it is written.
            id: Some(self.fresh_id()),
            name: name.into(),
            sort_index,
            hidden: false,
            locked: false,
            omit_from_export: false,
            color: None,
            children: Vec::new(),
        };
        let index = self.tree.world.layers.len();
        self.apply_and_record(Edit::LayerInserted { index, layer });
        self.current_layer
    }

    /// Removes a layer, moving what was on it onto the default one, as one undo step.
    ///
    /// Returns whether it was removed. The default layer cannot be: it is the worldspawn.
    pub fn remove_layer(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.tree.world.layers.len() {
            return false;
        }
        let mut layer = self.tree.world.layers[index].clone();
        let orphans = std::mem::take(&mut layer.children);
        let mut parts = Vec::new();
        if !orphans.is_empty() {
            parts.push(Edit::Removed {
                layer: index,
                nodes: orphans.clone(),
            });
            parts.push(Edit::Added {
                layer: 0,
                nodes: orphans,
            });
        }
        parts.push(Edit::LayerRemoved { index, layer });
        self.apply_and_record(Edit::Batch(parts));
        true
    }

    /// Renames a layer, as one undo step.
    pub fn rename_layer(&mut self, index: usize, name: impl Into<String>) -> bool {
        let Some(layer) = self.tree.world.layers.get(index) else {
            return false;
        };
        // The layer without its contents: a rename does not need them.
        let before = map_model::node::Layer {
            id: layer.id.clone(),
            name: layer.name.clone(),
            sort_index: layer.sort_index,
            hidden: layer.hidden,
            locked: layer.locked,
            omit_from_export: layer.omit_from_export,
            color: layer.color.clone(),
            children: Vec::new(),
        };
        let after = map_model::node::Layer {
            name: name.into(),
            ..before.clone()
        };
        if after.name == before.name {
            return true;
        }
        self.apply_and_record(Edit::LayerChanged {
            index,
            before,
            after,
        });
        true
    }

    /// Sets whether a layer is drawn.
    ///
    /// A hidden layer is not drawn and cannot be picked, which is what hiding is for.
    /// Anything on it that was selected is deselected, since a selection nobody can see is
    /// a selection that surprises whoever edits next.
    pub fn set_layer_hidden(&mut self, index: usize, hidden: bool) -> bool {
        let Some(layer) = self.tree.world.layers.get_mut(index) else {
            return false;
        };
        layer.hidden = hidden;
        if hidden {
            self.deselect_layer(index);
        }
        self.note_modification();
        self.mark_visibility_changed();
        true
    }

    /// Sets whether a layer can be edited.
    ///
    /// A locked layer is still drawn but cannot be picked or changed.
    pub fn set_layer_locked(&mut self, index: usize, locked: bool) -> bool {
        let Some(layer) = self.tree.world.layers.get_mut(index) else {
            return false;
        };
        layer.locked = locked;
        if locked {
            self.deselect_layer(index);
        }
        self.note_modification();
        self.mark_visibility_changed();
        true
    }

    /// Sets whether a layer is left out of exports.
    pub fn set_layer_omitted(&mut self, index: usize, omitted: bool) -> bool {
        let Some(layer) = self.tree.world.layers.get_mut(index) else {
            return false;
        };
        layer.omit_from_export = omitted;
        self.note_modification();
        true
    }

    /// Drops everything on the given layer from the selection.
    fn deselect_layer(&mut self, index: usize) {
        let on_layer: HashSet<NodeId> = node::brushes_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(_, _, layer)| *layer == index)
            .map(|(id, _, _)| id)
            .chain(
                node::point_entities_by_layer(&self.tree.world)
                    .into_iter()
                    .filter(|(_, _, layer)| *layer == index)
                    .map(|(id, _, _)| id),
            )
            .collect();
        self.selection.retain(|id| !on_layer.contains(id));
        self.face_selection
            .retain(|face| !on_layer.contains(&face.node));
    }

    /// Returns whether the given layer's contents can be picked and edited.
    pub(super) fn layer_is_editable(&self, index: usize) -> bool {
        self.tree
            .world
            .layers
            .get(index)
            .is_some_and(|layer| !layer.hidden && !layer.locked)
    }

    /// Returns whether the given layer's contents are drawn.
    fn layer_is_visible(&self, index: usize) -> bool {
        self.tree
            .world
            .layers
            .get(index)
            .is_some_and(|layer| !layer.hidden)
    }

    /// Returns whether the node has been hidden by "Hide" or "Isolate".
    #[inline]
    pub(super) fn is_hidden(&self, id: NodeId) -> bool {
        self.hidden_nodes.contains(&id)
    }

    /// Whether the node is drawn: on a visible layer and not individually hidden.
    #[inline]
    pub(super) fn node_is_drawn(&self, id: NodeId, layer: usize) -> bool {
        // The emptiness check first: this is asked per node per walk, and hiding is rare.
        self.layer_is_visible(layer) && (self.hidden_nodes.is_empty() || !self.is_hidden(id))
    }

    /// Hides the selected objects, `hideSelectedNodes`. They are deselected, since an
    /// edit should not reach something the views no longer show.
    pub fn hide_selection(&mut self) -> usize {
        let newly: Vec<NodeId> = self
            .selection
            .iter()
            .copied()
            .filter(|id| !self.hidden_nodes.contains(id))
            .collect();
        if newly.is_empty() {
            return 0;
        }
        self.hidden_nodes.extend(&newly);
        self.selection.clear();
        self.face_selection.clear();
        self.mark_visibility_changed();
        newly.len()
    }

    /// Hides everything except the selection, `isolateSelectedNodes`.
    pub fn isolate_selection(&mut self) -> usize {
        if self.selection.is_empty() {
            return 0;
        }
        self.hidden_nodes = self
            .all_object_ids()
            .into_iter()
            .filter(|id| !self.selection.contains(id))
            .collect();
        let count = self.hidden_nodes.len();
        self.mark_visibility_changed();
        count
    }

    /// Every brush, patch and point entity in the document, on any layer.
    fn all_object_ids(&self) -> Vec<NodeId> {
        node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .map(|(id, _)| id)
            .chain(
                node::patches_by_layer(&self.tree.world)
                    .into_iter()
                    .map(|(id, _, _)| id),
            )
            .chain(
                node::point_entities_by_layer(&self.tree.world)
                    .into_iter()
                    .map(|(id, _, _)| id),
            )
            .collect()
    }

    /// Shows everything hidden by "Hide" or "Isolate", `showAllNodes`. Returns how many
    /// objects were brought back.
    pub fn show_all(&mut self) -> usize {
        let count = self.hidden_nodes.len();
        if count > 0 {
            self.hidden_nodes.clear();
            self.mark_visibility_changed();
        }
        count
    }

    /// Moves the selection onto the given layer.
    ///
    /// Returns how many objects moved.
    pub fn move_selection_to_layer(&mut self, index: usize) -> usize {
        let ids: Vec<NodeId> = self.selection.iter().copied().collect();
        if ids.is_empty() {
            return 0;
        }
        if index >= self.tree.world.layers.len() {
            return 0;
        }
        // What moving takes out of each layer and container, as edits that undo can
        // reverse: a node at the top of another layer is removed from it, and a node
        // inside a group or an entity changes that container.
        let moving: HashSet<NodeId> = ids.iter().copied().collect();
        let mut taken: Vec<Node> = Vec::new();
        let mut parts = Vec::new();
        for (layer, contents) in self.tree.world.layers.iter().enumerate() {
            if layer == index {
                continue;
            }
            let roots: Vec<Node> = contents
                .children
                .iter()
                .filter(|node| moving.contains(&node.id))
                .cloned()
                .collect();
            if !roots.is_empty() {
                taken.extend(roots.iter().cloned());
                parts.push(Edit::Removed {
                    layer,
                    nodes: roots,
                });
            }
            let mut changed = Vec::new();
            for child in contents.children.iter().filter(|n| !moving.contains(&n.id)) {
                collect_containers_losing(child, &moving, &mut changed, &mut taken);
            }
            if !changed.is_empty() {
                let (before, after): (Vec<Node>, Vec<Node>) = changed.into_iter().unzip();
                parts.push(Edit::Changed { before, after });
            }
        }
        let moved = taken.len();
        if moved > 0 {
            parts.push(Edit::Added {
                layer: index,
                nodes: taken,
            });
            // Taking the nodes out deselects them; they are the same nodes, so they stay
            // selected.
            let selected = self.selection.clone();
            self.apply_and_record(Edit::Batch(parts));
            self.selection = selected;
        }
        moved
    }

    /// Selects everything on the given layer.
    ///
    /// Returns how many were selected. A hidden or locked layer selects nothing, since
    /// nothing on it can be edited.
    pub fn select_layer(&mut self, index: usize) -> usize {
        if !self.layer_is_editable(index) {
            return 0;
        }
        self.selection = node::brushes_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(_, _, layer)| *layer == index)
            .map(|(id, _, _)| id)
            .chain(
                node::point_entities_by_layer(&self.tree.world)
                    .into_iter()
                    .filter(|(_, _, layer)| *layer == index)
                    .map(|(id, _, _)| id),
            )
            .collect();
        self.face_selection.clear();
        self.mark_changed();
        self.selection.len()
    }

    /// Which layers the selection sits on, `collectContainingLayers`. Sorted, no repeats.
    pub fn layers_of_selection(&self) -> Vec<usize> {
        let mut layers: Vec<usize> = self
            .selection
            .iter()
            .filter_map(|id| node::layer_of(&self.tree.world, *id))
            .collect();
        layers.sort_unstable();
        layers.dedup();
        layers
    }

    /// Selects everything on all of the given layers, `selectAllInLayers`.
    pub fn select_layers(&mut self, indices: &[usize]) -> usize {
        let brushes = node::brushes_by_layer(&self.tree.world)
            .into_iter()
            .map(|(id, _, layer)| (id, layer));
        let entities = node::point_entities_by_layer(&self.tree.world)
            .into_iter()
            .map(|(id, _, layer)| (id, layer));
        self.selection = brushes
            .chain(entities)
            .filter(|(_, layer)| indices.contains(layer) && self.layer_is_editable(*layer))
            .map(|(id, _)| id)
            .collect();
        self.face_selection.clear();
        self.mark_changed();
        self.selection.len()
    }

    /// Hides every layer that is not in the given set, `isolateLayers`.
    pub fn isolate_layers(&mut self, keep: &[usize]) {
        for index in 0..self.tree.world.layers.len() {
            self.set_layer_hidden(index, !keep.contains(&index));
        }
    }
}

/// Walks `node` for the groups and entities that hold nodes in `moving` directly: each as
/// it is and as it will be without them, into `changed`, and the nodes themselves into
/// `taken`. A container that is itself moving is left whole.
fn collect_containers_losing(
    node: &Node,
    moving: &HashSet<NodeId>,
    changed: &mut Vec<(Node, Node)>,
    taken: &mut Vec<Node>,
) {
    let children = match &node.kind {
        NodeKind::Group(group) => &group.children,
        NodeKind::Entity(entity) => &entity.children,
        _ => return,
    };
    if children.iter().any(|child| moving.contains(&child.id)) {
        taken.extend(children.iter().filter(|c| moving.contains(&c.id)).cloned());
        let mut after = node.clone();
        match &mut after.kind {
            NodeKind::Group(group) => group.children.retain(|c| !moving.contains(&c.id)),
            NodeKind::Entity(entity) => entity.children.retain(|c| !moving.contains(&c.id)),
            _ => {}
        }
        changed.push((node.clone(), after));
        return;
    }
    for child in children {
        collect_containers_losing(child, moving, changed, taken);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;

    /// A document with two boxes on the default layer, the second grouped.
    fn two_boxes() -> (Document, NodeId, NodeId) {
        let mut d = Document::new(Default::default());
        for x in [0.0, 64.0] {
            let bounds = Bbox3d::new(Vec3d::new(x, 0.0, 0.0), Vec3d::new(x + 32.0, 32.0, 32.0));
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &bounds,
                map_model::shapes::DEFAULT_MATERIAL,
            )
            .expect("a box")]);
        }
        let ids: Vec<NodeId> = node::brushes_with_ids(&d.tree.world)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        d.select_only(ids[1]);
        d.group_selection("Group");
        (d, ids[0], ids[1])
    }

    fn layer_of(d: &Document, id: NodeId) -> Option<usize> {
        d.tree.world.layers.iter().position(|layer| {
            let mut found = Vec::new();
            fn walk(nodes: &[Node], out: &mut Vec<NodeId>) {
                for n in nodes {
                    out.push(n.id);
                    match &n.kind {
                        NodeKind::Group(g) => walk(&g.children, out),
                        NodeKind::Entity(e) => walk(&e.children, out),
                        _ => {}
                    }
                }
            }
            walk(&layer.children, &mut found);
            found.contains(&id)
        })
    }

    #[test]
    fn adding_and_renaming_a_layer_undo_and_redo() {
        let (mut d, _, _) = two_boxes();
        let index = d.add_layer("Props");
        assert_eq!(d.tree.world.layers.len(), 2);
        assert!(d.rename_layer(index, "Trees"));
        assert_eq!(d.tree.world.layers[index].name, "Trees");
        assert!(d.undo().is_some());
        assert_eq!(d.tree.world.layers[index].name, "Props");
        assert!(d.undo().is_some());
        assert_eq!(d.tree.world.layers.len(), 1);
        assert!(d.redo().is_some());
        assert_eq!(d.tree.world.layers.len(), 2);
        assert_eq!(d.tree.world.layers[index].name, "Props");
    }

    #[test]
    fn removing_a_layer_moves_its_contents_and_undo_puts_them_back() {
        let (mut d, plain, _) = two_boxes();
        let index = d.add_layer("Props");
        d.select_only(plain);
        assert_eq!(d.move_selection_to_layer(index), 1);
        assert_eq!(layer_of(&d, plain), Some(index));
        assert!(d.remove_layer(index));
        assert_eq!(d.tree.world.layers.len(), 1);
        assert_eq!(layer_of(&d, plain), Some(0));
        assert!(d.undo().is_some());
        assert_eq!(d.tree.world.layers.len(), 2);
        assert_eq!(layer_of(&d, plain), Some(index));
        assert!(d.redo().is_some());
        assert_eq!(layer_of(&d, plain), Some(0));
    }

    #[test]
    fn moving_a_brush_out_of_a_group_undoes_back_into_the_group() {
        let (mut d, _, grouped) = two_boxes();
        let index = d.add_layer("Props");
        let group_children = |d: &Document| {
            d.tree.world.layers[0]
                .children
                .iter()
                .find_map(|n| match &n.kind {
                    NodeKind::Group(g) => Some(g.children.iter().map(|c| c.id).collect::<Vec<_>>()),
                    _ => None,
                })
        };
        assert_eq!(group_children(&d), Some(vec![grouped]));
        d.select_only(grouped);
        assert_eq!(d.move_selection_to_layer(index), 1);
        assert_eq!(layer_of(&d, grouped), Some(index));
        assert_eq!(group_children(&d), Some(Vec::new()));
        assert!(d.undo().is_some());
        assert_eq!(
            group_children(&d),
            Some(vec![grouped]),
            "back inside its group"
        );
        assert_eq!(layer_of(&d, grouped), Some(0));
    }
}
