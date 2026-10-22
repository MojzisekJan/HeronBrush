//! Document operations for history.

use super::{
    brush_node, brush_vertices, node, remove_nodes, touched_ids, Document, Edit, EditListener,
    HashMap, HashSet, History, Node, NodeId, Rc, Repeatable,
};

impl Document {
    pub fn set_face_color_live(
        &mut self,
        color: Option<map_model::brush_face::FaceColor>,
        picked: &[super::FaceRef],
        session: &mut super::LivePaint,
    ) -> usize {
        let selection = self.selection_snapshot();
        let mark = session
            .state
            .as_ref()
            .filter(|(revision, _, old, faces, time)| {
                *revision == self.revision()
                    && old == &selection
                    && faces == picked
                    && time.elapsed() < web_time::Duration::from_millis(600)
            })
            .map(|(_, mark, ..)| *mark)
            .unwrap_or_else(|| self.history.undo_len());
        let changed = if picked.is_empty() {
            self.set_face_color(color)
        } else {
            self.set_face_color_on(picked, color)
        };
        if changed > 0 {
            self.history.coalesce_since(mark);
            session.state = Some((
                self.revision(),
                mark,
                selection,
                picked.to_vec(),
                web_time::Instant::now(),
            ));
        }
        changed
    }

    /// Where a gesture that edits as it goes starts in the history, for
    /// [`Document::set_face_color_since`].
    pub fn history_mark(&self) -> usize {
        self.history.undo_len()
    }

    /// Paints `picked`, folding this edit and every one since `mark` into one undo step:
    /// a handle dragged or a colour picked live, which undoes in one go however long it
    /// took.
    pub fn set_face_color_since(
        &mut self,
        color: Option<map_model::brush_face::FaceColor>,
        picked: &[super::FaceRef],
        mark: usize,
    ) -> usize {
        let changed = self.set_face_color_on(picked, color);
        if changed > 0 && self.history.undo_len() > mark {
            self.history.coalesce_since(mark);
        }
        changed
    }

    pub fn set_surface_material_live(
        &mut self,
        color: Option<map_model::brush_face::SurfaceMaterial>,
        picked: &[super::FaceRef],
        session: &mut super::LivePaint,
    ) -> usize {
        let selection = self.selection_snapshot();
        let mark = session
            .state
            .as_ref()
            .filter(|(revision, _, old, faces, time)| {
                *revision == self.revision()
                    && old == &selection
                    && faces == picked
                    && time.elapsed() < web_time::Duration::from_millis(600)
            })
            .map(|(_, mark, ..)| *mark)
            .unwrap_or_else(|| self.history.undo_len());
        let changed = if picked.is_empty() {
            self.set_surface_material(color)
        } else {
            self.set_surface_material_on(picked, color)
        };
        if changed > 0 {
            self.history.coalesce_since(mark);
            session.state = Some((
                self.revision(),
                mark,
                selection,
                picked.to_vec(),
                web_time::Instant::now(),
            ));
        }
        changed
    }

    /// Applies an edit and adds it to the history.
    ///
    /// While a transaction is open the edit is applied but not recorded, since committing
    /// the transaction records the whole of it as one step. Outside one, an edit that
    /// touches a linked group carries the rebuild of its siblings into the same step, so
    /// linked groups keep in step without the menu action -- `updateLinkedGroups`.
    pub(super) fn apply_and_record(&mut self, edit: Edit) {
        if self.transaction.is_some() {
            self.apply_owned(edit);
            return;
        }
        let pre = self.linked_contents_snapshot();
        self.apply(&edit);
        let full = match pre {
            Some(pre) => self.with_linked_propagation(edit, Some(&pre)),
            None => edit,
        };
        self.record_edit(full);
    }

    /// Wraps an edit that has just been applied with the linked-group rebuild it sets
    /// off, applying that rebuild too and folding both into one history step. `pre` gives
    /// each linked group's contents from before the edit, so a node it removed from one
    /// still ties the change to its set.
    fn with_linked_propagation(
        &mut self,
        edit: Edit,
        pre: Option<&HashMap<NodeId, HashSet<NodeId>>>,
    ) -> Edit {
        let touched = touched_ids(&edit);
        // What the edit's changed nodes were before it: lets the sync check that a copy's
        // child is the copy it takes it for.
        let mut previous = HashMap::new();
        collect_before(&edit, &mut previous);
        match self.linked_sync_edit(&touched, pre, Some(&previous)) {
            Some(prop) => {
                self.apply(&prop);
                Edit::Batch(vec![edit, prop])
            }
            None => edit,
        }
    }

    /// Keeps an edit for Repeat to play again.
    ///
    /// `RepeatStack::push`. A run of edits on one selection is kept together, and the
    /// first edit after the selection changed starts a new run -- which is what makes
    /// Repeat mean "do that to this as well" rather than "do everything I have ever
    /// done".
    pub(super) fn remember_repeatable(&mut self, what: Repeatable) {
        if self.repeating {
            return;
        }
        if self.selection != self.repeat_selection {
            self.repeat_stack.clear();
            self.repeat_selection = self.selection.clone();
        }
        self.repeat_stack.push(what);
    }

    /// Whether there is anything to repeat.
    pub fn can_repeat(&self) -> bool {
        !self.repeat_stack.is_empty()
    }

    /// Does the last run of edits again, to whatever is selected now.
    ///
    /// `Menu/Edit/Repeat`, Ctrl+R: move a brush, copy it, turn it -- then select something
    /// else and have the same happen to it. Returns whether anything was done.
    pub fn repeat(&mut self) -> bool {
        if self.repeat_stack.is_empty() {
            return false;
        }

        // What a repeat does is not itself recorded, or repeating would grow the run each
        // time it was asked for.
        let again = self.repeat_stack.clone();
        self.repeating = true;
        let mut did = false;
        for what in again {
            did |= match what {
                Repeatable::Transform(transform) => self.transform_selection(&transform),
                Repeatable::Duplicate(delta) => self.duplicate_selection_by(delta) > 0,
            };
        }
        self.repeating = false;
        // The repeat left its own selection -- a duplicate selects the copies -- so the
        // run stays with it and another repeat carries on from there.
        self.repeat_selection = self.selection.clone();
        did
    }

    /// Forgets what Repeat would play again.
    ///
    /// `Menu/Edit/Clear Repeatable Commands`.
    pub fn clear_repeatable(&mut self) {
        self.repeat_stack.clear();
        self.repeat_selection = self.selection.clone();
    }

    /// Starts a transaction over the selected brushes.
    ///
    /// `Map::startTransaction` with `TransactionScope::LongRunning`. Everything edited
    /// until the matching commit becomes one step in the history.
    pub fn begin_entity_transaction(&mut self, id: NodeId) -> bool {
        if self.transaction.is_some() {
            return false;
        }
        let Some(node) = node::find_node(&self.tree.world, id)
            .filter(|n| matches!(n.kind, node::NodeKind::Entity(_)))
            .cloned()
        else {
            return false;
        };
        self.transaction = Some(vec![node]);
        true
    }

    pub fn commit_entity_transaction(&mut self) -> bool {
        let Some(before) = self.transaction.take() else {
            return false;
        };
        let after: Vec<_> = before
            .iter()
            .filter_map(|n| node::find_node(&self.tree.world, n.id).cloned())
            .collect();
        let changed = before
            .iter()
            .zip(&after)
            .any(|(a, b)| match (&a.kind, &b.kind) {
                (node::NodeKind::Entity(a), node::NodeKind::Entity(b)) => {
                    a.properties != b.properties
                }
                _ => false,
            });
        if changed {
            self.record_edit(Edit::Changed { before, after });
        }
        changed
    }

    pub fn begin_transaction(&mut self) {
        self.transaction = Some(self.selected_brush_nodes());
    }

    /// Runs `run`, then folds every undo step it recorded into one, so whatever it did
    /// undoes in a single press. Used by the plugin runner, whose script makes many small
    /// edits that are one action to the person who pressed the button.
    pub fn run_as_one_edit<R>(&mut self, run: impl FnOnce(&mut Self) -> R) -> R {
        let mark = self.history.undo_len();
        let result = run(self);
        self.history.coalesce_since(mark);
        self.mark_changed();
        result
    }

    /// Puts the brushes back as they were when the transaction began, keeping it open.
    ///
    /// `Map::rollbackTransaction`. A drag rolls back and re-applies from the original
    /// each step rather than compounding, so the result depends only on where the pointer
    /// is now and not on the path it took.
    pub fn rollback_transaction(&mut self) {
        let Some(before) = &self.transaction else {
            return;
        };
        let staleness = self.edge_staleness();
        let selected_only = before.iter().all(|node| {
            matches!(node.kind, super::NodeKind::Brush(_)) && self.selection.contains(&node.id)
        });
        let touched = super::ids_under(before);
        node::replace_nodes(&mut self.tree.world, before);
        self.mark_nodes_changed(touched);
        if selected_only {
            self.note_selected_geometry_only(staleness.0, staleness.1);
        }
    }

    /// Records everything done since the transaction began as one step.
    ///
    /// Returns whether anything actually changed.
    pub fn commit_transaction(&mut self) -> bool {
        let Some(before) = self.transaction.take() else {
            return false;
        };

        let after = self.selected_brush_nodes();
        // Nothing to record if the brushes came back to where they started.
        let unchanged = before.len() == after.len()
            && before.iter().zip(&after).all(|(was, now)| {
                brush_vertices(was) == brush_vertices(now)
                    && match (was.as_brush(), now.as_brush()) {
                        (Some(a), Some(b)) => {
                            a.faces().len() == b.faces().len()
                                && a.faces().iter().zip(b.faces()).all(|(a, b)| {
                                    a.uv_attributes == b.uv_attributes
                                        && a.uv_coord_system == b.uv_coord_system
                                        && a.primitive_uv == b.primitive_uv
                                        && a.material_name == b.material_name
                                        && a.color == b.color
                                        && a.surface_material == b.surface_material
                                        && a.surface_attributes == b.surface_attributes
                                })
                        }
                        _ => true,
                    }
            });
        if unchanged {
            return false;
        }

        let edit = Edit::Changed { before, after };
        let full = match self.linked_contents_snapshot() {
            Some(pre) => self.with_linked_propagation(edit, Some(&pre)),
            None => edit,
        };
        self.record_edit(full);
        true
    }

    /// Puts the brushes back and abandons the transaction.
    ///
    /// `Map::cancelTransaction`.
    pub fn cancel_transaction(&mut self) {
        self.rollback_transaction();
        self.transaction = None;
    }

    /// Returns whether a transaction is open.
    pub fn in_transaction(&self) -> bool {
        self.transaction.is_some()
    }

    /// Brushes selected directly or through their faces, in a stable order.
    fn selected_brush_nodes(&self) -> Vec<Node> {
        let mut nodes: Vec<(NodeId, Node)> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| {
                self.selection.contains(id)
                    || self.face_selection.iter().any(|face| face.node == *id)
            })
            .map(|(id, brush)| (id, brush_node(id, brush.clone())))
            .collect();
        nodes.sort_by_key(|(id, _)| *id);
        nodes.into_iter().map(|(_, node)| node).collect()
    }

    /// Marks the document changed since its last save and bumps the edit counter.
    ///
    /// `Map` raises `modificationCount` on every command; here the geometry edits go
    /// through [`Document::apply`] and the layer and group operations call this directly.
    pub(super) fn note_modification(&mut self) {
        self.modified = true;
        self.modification_count = self.modification_count.wrapping_add(1);
    }

    /// Applies an edit without recording it, which is what undo and redo do.
    pub(super) fn apply(&mut self, edit: &Edit) {
        let staleness = self.edge_staleness();
        let selected_only = self.touches_only_selected_brushes(edit);
        self.apply_parts(edit);
        // The trace was found in geometry that has just changed, so it no longer says
        // anything about the level; leaving it drawn would be a line pointing at a hole
        // that may have been filled.
        self.clear_leak_trace();
        self.note_modification();
        // Only the nodes the edit names (and what is under them) can look different, so
        // only their 3D surfaces are built again.
        self.mark_nodes_changed(touched_ids(edit));
        // A move of the selection leaves every other brush's edges as they were, so the
        // next draw walks only the selection's (and the flat views keep their picture of
        // the rest).
        if selected_only {
            self.note_selected_geometry_only(staleness.0, staleness.1);
        }
    }

    /// [`Document::apply`] for an edit nobody keeps (a step of a drag inside a
    /// transaction): its new nodes are moved into the tree rather than copied.
    fn apply_owned(&mut self, edit: Edit) {
        let Edit::Changed { after, .. } = edit else {
            self.apply(&edit);
            return;
        };
        let staleness = self.edge_staleness();
        let selected_only = after.iter().all(|node| {
            matches!(node.kind, super::NodeKind::Brush(_)) && self.selection.contains(&node.id)
        });
        let touched = super::ids_under(&after);
        node::replace_nodes_owned(&mut self.tree.world, after);
        self.clear_leak_trace();
        self.note_modification();
        self.mark_nodes_changed(touched);
        if selected_only {
            self.note_selected_geometry_only(staleness.0, staleness.1);
        }
    }

    /// Applies an edit without the bookkeeping, so a batch does that once rather than
    /// once per part.
    fn apply_parts(&mut self, edit: &Edit) {
        // A run of removals is done in one walk of the tree: ungrouping five thousand
        // groups is five thousand `Removed` parts, and a walk each made it quadratic.
        let mut removing = Vec::new();
        self.apply_flattened(edit, &mut removing);
        self.flush_removals(&mut removing);
    }

    fn apply_flattened(&mut self, edit: &Edit, removing: &mut Vec<NodeId>) {
        match edit {
            Edit::Changed { after, .. } => {
                self.flush_removals(removing);
                node::replace_nodes(&mut self.tree.world, after);
            }
            Edit::Added { layer, nodes } => {
                self.flush_removals(removing);
                self.insert_nodes(*layer, nodes);
            }
            Edit::Reordered { layer, after, .. } => {
                self.flush_removals(removing);
                if let Some(layer) = self.tree.world.layers.get_mut(*layer) {
                    let positions: HashMap<_, _> =
                        after.iter().enumerate().map(|(i, id)| (*id, i)).collect();
                    // Stable sorting keeps concurrent additions in their existing order.
                    layer
                        .children
                        .sort_by_key(|n| positions.get(&n.id).copied().unwrap_or(usize::MAX));
                }
            }
            Edit::Removed { nodes, .. } => removing.extend(nodes.iter().map(|node| node.id)),
            Edit::Batch(parts) => {
                for part in parts {
                    self.apply_flattened(part, removing);
                }
            }
        }
    }

    fn flush_removals(&mut self, removing: &mut Vec<NodeId>) {
        if !removing.is_empty() {
            let ids = std::mem::take(removing);
            self.remove_nodes_by_id(&ids);
        }
    }

    /// Undoes the last edit. Returns what it undid, or `None` if there was nothing to
    /// undo.
    pub fn undo(&mut self) -> Option<String> {
        self.reset_trigger_preview();
        let edit = self.history.undo()?;
        let description = edit.description();
        self.apply(&edit);
        self.forget_vanished_selection();
        if let Some(on_edit) = self.on_edit.clone() {
            on_edit(&edit);
        }
        // `Map::transactionUndone`: what Repeat would play again is edits that have just
        // been taken back, so it is emptied rather than left pointing at them.
        self.clear_repeatable();
        Some(description)
    }

    /// Redoes the last undone edit.
    pub fn redo(&mut self) -> Option<String> {
        self.reset_trigger_preview();
        let edit = self.history.redo()?;
        let description = edit.description();
        self.apply(&edit);
        self.forget_vanished_selection();
        if let Some(on_edit) = self.on_edit.clone() {
            on_edit(&edit);
        }
        self.clear_repeatable();
        Some(description)
    }

    /// Drops from the selection whatever an undo or redo took out of the tree.
    fn forget_vanished_selection(&mut self) {
        // A walk of the tree per selected node is fine for a few; for a big selection it
        // was the whole cost of the undo.
        if self.selection.len() + self.face_selection.len() <= 8 {
            self.selection
                .retain(|id| node::find_node(&self.tree.world, *id).is_some());
            self.face_selection
                .retain(|face| node::find_node(&self.tree.world, face.node).is_some());
        } else {
            let present = node::all_node_ids(&self.tree.world);
            self.selection.retain(|id| present.contains(id));
            self.face_selection
                .retain(|face| present.contains(&face.node));
        }
    }

    /// Adds nodes to the given layer, keeping the ids they were given.
    ///
    /// Reusing the ids is what lets an undone deletion be selected and edited again as
    /// the same objects.
    fn insert_nodes(&mut self, layer: usize, nodes: &[Node]) {
        let Some(layer) = self.tree.world.layers.get_mut(layer) else {
            return;
        };
        layer.children.extend(nodes.iter().cloned());
    }

    /// Removes the nodes with the given ids from wherever they are.
    fn remove_nodes_by_id(&mut self, ids: &[NodeId]) {
        node::remove_nodes_from_world(&mut self.tree.world, ids);
        for id in ids {
            self.selection.remove(id);
        }
        if !self.face_selection.is_empty() {
            let gone: HashSet<NodeId> = ids.iter().copied().collect();
            self.face_selection
                .retain(|face| !gone.contains(&face.node));
        }
    }

    /// Sets what to call when the document changes.
    ///
    /// Set once, when the window is built. A document opened later inherits it, so
    /// opening a file does not silently stop the views from updating.
    pub fn set_on_changed(&mut self, on_changed: Rc<dyn Fn()>) {
        self.on_changed = Some(on_changed);
    }

    /// Replace a map without dropping callbacks or reusing a GUI revision.
    pub fn replace_for_agent(&mut self, mut replacement: Self) {
        replacement.on_edit = self.on_edit.clone();
        replacement.on_changed = self.on_changed.clone();
        replacement.revision = self.revision;
        *self = replacement;
        self.mark_changed();
    }

    pub fn has_edit_listener(&self) -> bool {
        self.on_edit.is_some()
    }

    /// Sets the callback used to send local edits to a live collaboration session.
    pub fn set_on_edit(&mut self, on_edit: EditListener) {
        self.on_edit = Some(on_edit);
    }

    /// Drops the live-session callback, so leaving a session stops sending edits.
    pub fn clear_on_edit(&mut self) {
        self.on_edit = None;
    }

    /// Records an edit on the undo stack and hands it to the `on_edit` callback, if a
    /// live session set one.
    pub(super) fn record_edit(&mut self, edit: Edit) {
        if let Some(on_edit) = self.on_edit.clone() {
            on_edit(&edit);
        }
        self.history.record(edit);
    }

    /// Applies an edit that came from the other person in a live session. It is applied
    /// to the tree and the views are told, but it does not go on this editor's undo stack
    /// and it is not sent back out.
    ///
    /// Replaces this document's whole tree with one handed over by a live session's host,
    /// so a joining guest works on the host's nodes and ids. The callbacks, the file path
    /// and the preferences stay; the selection and the undo history do not, since they
    /// were about the tree that is gone.
    pub fn adopt_world(&mut self, world: node::World) {
        self.tree = node::Tree {
            world,
            warnings: Vec::new(),
        };
        self.selection.clear();
        self.face_selection.clear();
        self.hidden_nodes.clear();
        self.open_group = None;
        self.leak_trace = None;
        self.history = History::new();
        self.repeat_stack.clear();
        self.note_modification();
        self.mark_changed();
    }

    /// The edit already carries whatever linked-group rebuild the sender computed, so it
    /// is applied whole.
    pub fn apply_remote(&mut self, edit: &Edit) {
        self.apply_remote_parts(edit);
        self.clear_leak_trace();
        self.note_modification();
        self.mark_changed();
        // Drop any selection on nodes the edit took away, so the views do not keep
        // pointing at something that is gone.
        self.selection
            .retain(|id| node::find_node(&self.tree.world, *id).is_some());
        self.face_selection
            .retain(|face| node::find_node(&self.tree.world, face.node).is_some());
    }

    fn apply_remote_parts(&mut self, edit: &Edit) {
        match edit {
            Edit::Added { layer, nodes } if self.tree.world.layers.get(*layer).is_some() => {
                let ids: Vec<_> = nodes.iter().map(|node| node.id).collect();
                for existing in &mut self.tree.world.layers {
                    remove_nodes(&mut existing.children, &ids);
                }
                self.insert_nodes(*layer, nodes);
            }
            Edit::Batch(parts) => {
                for part in parts {
                    self.apply_remote_parts(part);
                }
            }
            _ => self.apply_parts(edit),
        }
    }

    /// Returns what to call when the document changes, so a newly opened document can
    /// take it over.
    pub fn on_changed(&self) -> Option<Rc<dyn Fn()>> {
        self.on_changed.clone()
    }
}

/// The nodes an edit changed, as they were before it, by id.
fn collect_before<'a>(edit: &'a Edit, into: &mut HashMap<NodeId, &'a Node>) {
    match edit {
        Edit::Changed { before, .. } => into.extend(before.iter().map(|node| (node.id, node))),
        Edit::Batch(parts) => parts.iter().for_each(|part| collect_before(part, into)),
        Edit::Added { .. } | Edit::Removed { .. } | Edit::Reordered { .. } => {}
    }
}
