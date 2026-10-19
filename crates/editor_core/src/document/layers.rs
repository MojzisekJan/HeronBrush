//! Document operations for layers.

use super::{Document, HashSet, LayerSummary, NodeId, node};

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
        self.current_layer.min(self.tree.world.layers.len().saturating_sub(1))
    }

    /// Sets which layer a newly drawn brush goes onto.
    pub fn set_current_layer(&mut self, index: usize) {
        if index < self.tree.world.layers.len() {
            self.current_layer = index;
        }
    }

    /// Adds an empty layer and makes it the current one.
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
        self.tree.world.layers.push(map_model::node::Layer {
            // A `_tb_id` distinguishes this layer from the worldspawn when it is written.
            id: Some(self.fresh_id()),
            name: name.into(),
            sort_index,
            hidden: false,
            locked: false,
            omit_from_export: false,
            color: None,
            children: Vec::new(),
        });
        self.current_layer = self.tree.world.layers.len() - 1;
        self.note_modification();
        self.mark_changed();
        self.current_layer
    }

    /// Removes a layer, moving what was on it onto the default one.
    ///
    /// Returns whether it was removed. The default layer cannot be: it is the worldspawn.
    pub fn remove_layer(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.tree.world.layers.len() {
            return false;
        }

        let orphans = std::mem::take(&mut self.tree.world.layers[index].children);
        self.tree.world.layers[0].children.extend(orphans);
        self.tree.world.layers.remove(index);
        self.current_layer = self.current_layer.min(self.tree.world.layers.len() - 1);
        self.note_modification();
        self.mark_changed();
        true
    }

    /// Renames a layer.
    pub fn rename_layer(&mut self, index: usize, name: impl Into<String>) -> bool {
        let Some(layer) = self.tree.world.layers.get_mut(index) else {
            return false;
        };
        layer.name = name.into();
        self.note_modification();
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
        self.face_selection.retain(|face| !on_layer.contains(&face.node));
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
        let moved = node::move_to_layer(&mut self.tree.world, &ids, index);
        if moved > 0 {
            self.note_modification();
            self.mark_changed();
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
            .filter(|(_, layer)| {
                indices.contains(layer) && self.layer_is_editable(*layer)
            })
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
