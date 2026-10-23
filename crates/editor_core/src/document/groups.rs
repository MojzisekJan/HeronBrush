//! Document operations for groups.

use super::{
    brush_node, deep_renumber, identity_transformation, mat_ext, node, parse_transformation,
    preserve_protected_properties, transform_node, translate_paste, translation_transformation,
    Document, Edit, EntityNode, Group, HashMap, HashSet, Mat4x4d, Node, NodeId, NodeKind, Vec3d,
};

impl Document {
    /// Makes a group out of the selection.
    ///
    /// `groupSelectedNodes`. Returns the group's id. What was selected stays selected: the
    /// objects are the same objects, only what they hang from has changed.
    ///
    /// Selecting a group selects its contents, so editing operations can use the same selection
    /// representation for grouped and ungrouped objects.
    pub fn group_selection(&mut self, name: impl Into<String>) -> Option<NodeId> {
        if self.selection.is_empty() {
            return None;
        }

        // The selected nodes that sit directly on a layer -- what can be grouped. A brush
        // inside an entity is not one of them; nor is one already in a group.
        let mut by_layer: Vec<(usize, Vec<Node>)> = Vec::new();
        for (index, layer) in self.tree.world.layers.iter().enumerate() {
            let taken: Vec<Node> = layer
                .children
                .iter()
                .filter(|node| self.selection.contains(&node.id))
                .cloned()
                .collect();
            if !taken.is_empty() {
                by_layer.push((index, taken));
            }
        }
        let members: Vec<Node> = by_layer
            .iter()
            .flat_map(|(_, nodes)| nodes.iter().cloned())
            .collect();
        if members.is_empty() {
            return None;
        }

        let target_layer = self.current_layer().min(self.tree.world.layers.len() - 1);
        let member_ids: Vec<NodeId> = members.iter().map(|node| node.id).collect();
        let group = Node::new(NodeKind::Group(Group {
            id: self.fresh_id(),
            name: name.into(),
            linked_group_id: None,
            transformation: None,
            children: members,
        }));
        let group_id = group.id;

        let mut parts: Vec<Edit> = by_layer
            .into_iter()
            .map(|(layer, nodes)| Edit::Removed { layer, nodes })
            .collect();
        parts.push(Edit::Added {
            layer: target_layer,
            nodes: vec![group],
        });
        self.apply_and_record(Edit::Batch(parts));

        // The objects are the same objects, only what they hang from has changed, so what
        // was selected stays selected -- `remove` cleared it as the nodes left the layer.
        self.selection = member_ids.into_iter().collect();
        self.mark_changed();
        Some(group_id)
    }

    /// Turns the selected brushes into an entity of the given class.
    ///
    /// `createBrushEntity`: a door, a platform or a trigger is brushes with a classname on
    /// them. The brushes stay selected, since what was just made is what is being worked
    /// on. Returns the entity's id.
    ///
    /// A brush already in an entity is re-classed rather than moved: choosing another
    /// class for a door is changing that door's class, not making a second one.
    pub fn create_brush_entity(&mut self, classname: &str) -> Option<NodeId> {
        if self.selection.is_empty() {
            return None;
        }

        // Every entity all of whose brushes are selected is re-classed where it stands.
        let existing: Vec<NodeId> = node::entities_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(_, entity)| entity.is_brush_entity())
            .map(|(id, _)| id)
            .filter(|id| {
                // `contents_of` names the entity as well as what is in it, and it is the
                // brushes that are selected.
                let brushes: Vec<NodeId> = node::contents_of(&self.tree.world, *id)
                    .into_iter()
                    .filter(|member| self.brush(*member).is_some())
                    .collect();
                !brushes.is_empty() && brushes.iter().all(|brush| self.selection.contains(brush))
            })
            .collect();
        if !existing.is_empty() {
            for id in &existing {
                self.set_entity_property(
                    *id,
                    map_model::entity_properties::keys::CLASSNAME,
                    classname,
                );
            }
            return existing.first().copied();
        }

        // Keep a selected brush group in place. Removing individual members and
        // re-adding them at layer level would dissolve the group, including on Undo.
        let group = node::groups_with_ids(&self.tree.world)
            .into_iter()
            .find(|(_, group)| {
                !group.children.is_empty()
                    && group
                        .children
                        .iter()
                        .all(|n| matches!(n.kind, NodeKind::Brush(_)))
                    && group.children.len() == self.selection.len()
                    && group
                        .children
                        .iter()
                        .all(|n| self.selection.contains(&n.id))
            })
            .map(|(id, _)| id);
        if let Some(group_id) = group {
            let defaults = self.new_entity_defaults(classname);
            let before = node::find_node(&self.tree.world, group_id)?.clone();
            let mut after = before.clone();
            let NodeKind::Group(group) = &mut after.kind else {
                unreachable!()
            };
            let entity = Node::new(NodeKind::Entity(EntityNode {
                properties: std::iter::once((
                    map_model::entity_properties::keys::CLASSNAME.into(),
                    classname.into(),
                ))
                .chain(defaults)
                .collect(),
                children: std::mem::take(&mut group.children),
            }));
            let id = entity.id;
            group.children.push(entity);
            self.apply_and_record(Edit::Changed {
                before: vec![before],
                after: vec![after],
            });
            return Some(id);
        }

        let loose: Vec<NodeId> = self
            .selection
            .iter()
            .copied()
            .filter(|id| self.brush(*id).is_some())
            .collect();
        if loose.is_empty() {
            return None;
        }

        // The brushes come out of the layer and go back in inside an entity, which is two
        // halves of one edit: undo takes the entity away and puts the brushes back.
        let taken: Vec<Node> = loose
            .iter()
            .filter_map(|id| self.brush(*id).map(|brush| brush_node(*id, brush.clone())))
            .collect();
        if taken.is_empty() {
            return None;
        }

        let entity = Node::new(NodeKind::Entity(EntityNode {
            properties: std::iter::once((
                map_model::entity_properties::keys::CLASSNAME.to_string(),
                classname.to_string(),
            ))
            .chain(self.new_entity_defaults(classname))
            .collect(),
            children: taken.clone(),
        }));
        let id = entity.id;
        let layer = self.current_layer();
        self.apply_and_record(Edit::Batch(vec![
            Edit::Removed {
                layer,
                nodes: taken,
            },
            Edit::Added {
                layer,
                nodes: vec![entity],
            },
        ]));
        self.selection = loose.into_iter().collect();
        self.mark_changed();
        Some(id)
    }

    /// Takes apart every group whose contents are entirely selected.
    ///
    /// `ungroupSelectedNodes`. Returns how many groups were dissolved. What was in them
    /// stays selected, which is what makes ungrouping and regrouping a way of changing a
    /// group's membership.
    pub fn ungroup_selection(&mut self) -> usize {
        // One walk finds every group with its contents and layer; asking per group was
        // quadratic in the number of groups.
        let entries: Vec<_> = node::groups_with_contents(&self.tree.world)
            .into_iter()
            .filter(|entry| {
                !entry.contents.is_empty()
                    && entry.contents.iter().all(|id| self.selection.contains(id))
            })
            .collect();
        let dissolving: Vec<NodeId> = entries.iter().map(|entry| entry.id).collect();

        // All the removals first, then all the re-adds -- both act on a layer's own
        // children, so a group has to be gone before its contents land beside it.
        let mut removals = Vec::new();
        let mut additions = Vec::new();
        let mut freed = Vec::new();
        for entry in &entries {
            let (node, group, layer) = (entry.node.clone(), entry.group, entry.layer);
            freed.extend(group.children.iter().map(|child| child.id));
            additions.push(Edit::Added {
                layer,
                nodes: group.children.clone(),
            });
            removals.push(Edit::Removed {
                layer,
                nodes: vec![node],
            });
        }
        drop(entries);
        if removals.is_empty() {
            return 0;
        }

        removals.append(&mut additions);
        self.apply_and_record(Edit::Batch(removals));

        // The group the user was editing may be one of the ones that just went.
        if self
            .open_group
            .is_some_and(|open| dissolving.contains(&open))
        {
            self.open_group = None;
        }
        self.selection = freed.into_iter().collect();
        self.mark_changed();
        dissolving.len()
    }

    /// Folds every selected group into one. `mergeSelectedGroupsWithGroup`: the contents
    /// of the others move into the last group in document order and the emptied groups go,
    /// in one undo step. Returns how many groups were folded in, or 0 if there are fewer
    /// than two, or any of them is linked (merging would break the link sets).
    pub fn merge_selected_groups(&mut self) -> usize {
        let groups = self.fully_selected_group_ids();
        if groups.len() < 2 {
            return 0;
        }
        let entries: HashMap<NodeId, (&Node, usize)> = node::groups_with_contents(&self.tree.world)
            .into_iter()
            .map(|entry| (entry.id, (entry.node, entry.layer)))
            .collect();
        if groups.iter().any(|id| {
            matches!(
                entries.get(id).map(|(n, _)| &n.kind),
                Some(NodeKind::Group(g)) if g.linked_group_id.is_some()
            )
        }) {
            return 0;
        }

        let target = *groups.last().expect("checked len >= 2");
        let Some(target_node) = entries.get(&target).map(|(n, _)| (*n).clone()) else {
            return 0;
        };
        let NodeKind::Group(target_group) = &target_node.kind else {
            return 0;
        };

        let mut merged_children = target_group.children.clone();
        let mut removals = Vec::new();
        for source in groups.iter().copied().filter(|id| *id != target) {
            let Some((node, layer)) = entries
                .get(&source)
                .map(|(n, layer)| ((*n).clone(), *layer))
            else {
                continue;
            };
            let NodeKind::Group(group) = &node.kind else {
                continue;
            };
            merged_children.extend(group.children.iter().cloned());
            removals.push(Edit::Removed {
                layer,
                nodes: vec![node],
            });
        }
        drop(entries);
        if removals.is_empty() {
            return 0;
        }

        let mut new_target = target_node.clone();
        if let NodeKind::Group(group) = &mut new_target.kind {
            group.children = merged_children;
        }
        let contents: Vec<NodeId> = match &new_target.kind {
            NodeKind::Group(group) => group.children.iter().map(|c| c.id).collect(),
            _ => Vec::new(),
        };
        let folded = removals.len();

        removals.push(Edit::Changed {
            before: vec![target_node],
            after: vec![new_target],
        });
        self.apply_and_record(Edit::Batch(removals));

        self.selection = contents.into_iter().collect();
        self.mark_changed();
        folded
    }

    /// The groups whose every descendant is in the selection -- what the group commands
    /// act on, since the selection here holds member ids, never a group's own.
    fn fully_selected_group_ids(&self) -> Vec<NodeId> {
        let selected = &self.selection;
        node::groups_with_contents(&self.tree.world)
            .into_iter()
            .filter(|entry| {
                !entry.contents.is_empty() && entry.contents.iter().all(|c| selected.contains(c))
            })
            .map(|entry| entry.id)
            .collect()
    }

    /// The link ids of the given groups.
    fn link_ids_of(&self, groups: &[NodeId]) -> HashSet<String> {
        groups
            .iter()
            .filter_map(|id| node::find_node(&self.tree.world, *id))
            .filter_map(|node| match &node.kind {
                NodeKind::Group(group) => group.linked_group_id.clone(),
                _ => None,
            })
            .collect()
    }

    /// Renames every group whose contents are entirely selected, and every group linked
    /// to one, in one undo step.
    ///
    /// Linked copies share a name, so renaming one renames the set.
    pub fn rename_selected_groups(&mut self, name: &str) -> usize {
        let name = name.trim();
        if name.is_empty() {
            return 0;
        }
        let fully_selected = self.fully_selected_group_ids();
        let link_ids = self.link_ids_of(&fully_selected);

        let mut before = Vec::new();
        let mut after = Vec::new();
        for (id, group) in node::groups_with_ids(&self.tree.world) {
            let in_set = fully_selected.contains(&id)
                || group
                    .linked_group_id
                    .as_ref()
                    .is_some_and(|link| link_ids.contains(link));
            if !in_set || group.name == name {
                continue;
            }
            before.push(
                node::find_node(&self.tree.world, id)
                    .expect("the group was just listed")
                    .clone(),
            );
            let mut renamed = group.clone();
            renamed.name = name.to_string();
            after.push(Node {
                id,
                kind: NodeKind::Group(renamed),
            });
        }

        if after.is_empty() {
            return 0;
        }
        let count = after.len();
        self.apply_and_record(Edit::Changed { before, after });
        count
    }

    /// Extends the selection to every member of every linked set a fully-selected group
    /// belongs to. `selectLinkedGroups` / "Select Linked Groups".
    pub fn select_linked_groups(&mut self) {
        let chosen = self.fully_selected_group_ids();
        let links = self.link_ids_of(&chosen);
        if links.is_empty() {
            return;
        }
        let mut result = self.selection.clone();
        for (link, members) in self.linked_sets() {
            if !links.contains(&link) {
                continue;
            }
            for member in members {
                result.extend(node::contents_of(&self.tree.world, member));
            }
        }
        self.selection = result;
        self.mark_changed();
    }

    /// Makes the selected linked groups independent again, dropping the link id and the
    /// transformation, and does the same for any link set that would be left with one
    /// member. `separateSelectedLinkedGroups` without the relink option. Returns how many
    /// groups were freed.
    pub fn separate_selected_linked_groups(&mut self) -> usize {
        let chosen: HashSet<NodeId> = self.fully_selected_group_ids().into_iter().collect();
        let links = self.link_ids_of(&chosen.iter().copied().collect::<Vec<_>>());

        let mut to_unlink: HashSet<NodeId> = HashSet::new();
        for (link, members) in self.linked_sets() {
            if !links.contains(&link) {
                continue;
            }
            for member in &members {
                if chosen.contains(member) {
                    to_unlink.insert(*member);
                }
            }
            // A set left with a single member is no set at all.
            let remaining: Vec<NodeId> = members
                .iter()
                .copied()
                .filter(|id| !to_unlink.contains(id))
                .collect();
            if remaining.len() < 2 {
                to_unlink.extend(remaining);
            }
        }
        if to_unlink.is_empty() {
            return 0;
        }

        let mut before = Vec::new();
        let mut after = Vec::new();
        for id in &to_unlink {
            let Some(node) = node::find_node(&self.tree.world, *id) else {
                continue;
            };
            let NodeKind::Group(group) = &node.kind else {
                continue;
            };
            before.push(node.clone());
            let mut freed = group.clone();
            freed.linked_group_id = None;
            freed.transformation = None;
            after.push(Node {
                id: *id,
                kind: NodeKind::Group(freed),
            });
        }

        let count = after.len();
        // Recorded directly: this is the linked-group machinery itself and wants no
        // propagation pass on top, as `create_linked_duplicate` does.
        let edit = Edit::Changed { before, after };
        self.apply(&edit);
        self.record_edit(edit);
        count
    }

    /// Every group in the document, as its id and name.
    pub fn groups(&self) -> Vec<(NodeId, String, usize)> {
        node::groups_with_content_counts(&self.tree.world)
            .into_iter()
            .map(|(id, group, count)| (id, group.name.clone(), count))
            .collect()
    }

    /// Which group is open, if any.
    ///
    /// `EditorContext::currentGroup`. While a group is open, clicking something inside it
    /// selects that thing rather than the whole group -- which is the only way to edit one
    /// member of a group.
    pub fn open_group(&self) -> Option<NodeId> {
        self.open_group
    }

    /// Opens the group the selection is in, so its members can be edited on their own.
    ///
    /// Returns whether one was opened.
    pub fn open_selected_group(&mut self) -> bool {
        let Some(first) = self.selection.iter().next().copied() else {
            return false;
        };
        let Some(group) = node::outermost_group_of(&self.tree.world, first, self.open_group) else {
            return false;
        };
        self.open_group = Some(group);
        self.mark_changed();
        true
    }

    /// Moves the selected layer-level objects into the open group. `addSelectedObjectsTo
    /// Group`. Returns how many moved. Nothing happens with no group open, nor to an
    /// object already inside a group.
    pub fn add_selection_to_open_group(&mut self) -> usize {
        let Some(group_id) = self.open_group else {
            return 0;
        };
        let Some(group_node) = node::find_node(&self.tree.world, group_id).cloned() else {
            return 0;
        };
        let NodeKind::Group(_) = &group_node.kind else {
            return 0;
        };

        // The selected nodes sitting directly on a layer -- what can be moved in.
        let mut by_layer: Vec<(usize, Vec<Node>)> = Vec::new();
        for (index, layer) in self.tree.world.layers.iter().enumerate() {
            let taken: Vec<Node> = layer
                .children
                .iter()
                .filter(|node| self.selection.contains(&node.id))
                .cloned()
                .collect();
            if !taken.is_empty() {
                by_layer.push((index, taken));
            }
        }
        let incoming: Vec<Node> = by_layer
            .iter()
            .flat_map(|(_, nodes)| nodes.iter().cloned())
            .collect();
        if incoming.is_empty() {
            return 0;
        }
        let moved = incoming.len();
        let moved_ids: Vec<NodeId> = incoming.iter().map(|node| node.id).collect();

        let mut new_group = group_node.clone();
        if let NodeKind::Group(group) = &mut new_group.kind {
            group.children.extend(incoming);
        }

        let mut parts: Vec<Edit> = by_layer
            .into_iter()
            .map(|(layer, nodes)| Edit::Removed { layer, nodes })
            .collect();
        parts.push(Edit::Changed {
            before: vec![group_node],
            after: vec![new_group],
        });
        self.apply_and_record(Edit::Batch(parts));

        self.selection = moved_ids.into_iter().collect();
        self.mark_changed();
        moved
    }

    /// Moves the selected members of the open group back onto its layer.
    /// `removeSelectedObjectsFromGroup`. Returns how many moved.
    pub fn remove_selection_from_open_group(&mut self) -> usize {
        let Some(group_id) = self.open_group else {
            return 0;
        };
        let Some(group_node) = node::find_node(&self.tree.world, group_id).cloned() else {
            return 0;
        };
        let NodeKind::Group(group) = &group_node.kind else {
            return 0;
        };
        let Some(layer) = node::layer_of(&self.tree.world, group_id) else {
            return 0;
        };

        let (leaving, staying): (Vec<Node>, Vec<Node>) = group
            .children
            .iter()
            .cloned()
            .partition(|child| self.selection.contains(&child.id));
        if leaving.is_empty() {
            return 0;
        }
        let moved = leaving.len();
        let leaving_ids: Vec<NodeId> = leaving.iter().map(|node| node.id).collect();

        let mut new_group = group_node.clone();
        if let NodeKind::Group(g) = &mut new_group.kind {
            g.children = staying;
        }

        // Take the objects out of the group first, then drop them on the layer.
        self.apply_and_record(Edit::Batch(vec![
            Edit::Changed {
                before: vec![group_node],
                after: vec![new_group],
            },
            Edit::Added {
                layer,
                nodes: leaving,
            },
        ]));

        self.selection = leaving_ids.into_iter().collect();
        self.mark_changed();
        moved
    }

    /// Closes the open group.
    ///
    /// `closeGroup`. Returns whether one was open.
    pub fn close_group(&mut self) -> bool {
        if self.open_group.take().is_none() {
            return false;
        }
        self.mark_changed();
        true
    }

    /// What clicking the given node should select.
    ///
    /// A node in a closed group selects the whole group's contents; a node in the open
    /// group, or in none, selects itself.
    pub(super) fn selection_for(&self, id: NodeId) -> Vec<NodeId> {
        match node::outermost_group_of(&self.tree.world, id, self.open_group) {
            Some(group) => node::contents_of(&self.tree.world, group),
            None => vec![id],
        }
    }

    /// [`Document::selection_for`] of each of `ids`, together: one walk of the tree
    /// however many there are, where calling it per id walked the tree twice for each --
    /// a marquee over ten thousand brushes of a big map took seconds a frame.
    pub(super) fn selection_for_many(&self, ids: &[NodeId]) -> HashSet<NodeId> {
        if ids.len() <= 2 {
            return ids.iter().flat_map(|id| self.selection_for(*id)).collect();
        }
        let (outermost, contents) = node::outermost_groups(&self.tree.world, self.open_group);
        let mut taken: HashSet<NodeId> = HashSet::new();
        let mut out: HashSet<NodeId> = HashSet::with_capacity(ids.len());
        for id in ids {
            match outermost.get(id).copied() {
                Some(group) => {
                    if taken.insert(group) {
                        out.extend(contents.get(&group).into_iter().flatten().copied());
                    }
                }
                None => {
                    out.insert(*id);
                }
            }
        }
        out
    }

    /// Makes a linked copy of the one selected group, a grid step to the side.
    ///
    /// `Map_Groups`'s "Create Linked Duplicate". The copy and the original share a
    /// `_tb_linked_group_id` -- a fresh one if the original had none -- and each carries a
    /// `_tb_transformation` saying where it sits relative to the source, so a save writes
    /// them as a linked set and the views draw them apart. The copy is selected and its
    /// id returned.
    ///
    /// Edits do not yet propagate between the members: this creates the set, and keeping
    /// them in step afterwards is a regroup.
    pub fn create_linked_duplicate(&mut self, grid_size: f64) -> Option<NodeId> {
        let (source_id, layer, source_node) = node::groups_with_contents(&self.tree.world)
            .into_iter()
            .find(|entry| {
                !entry.contents.is_empty()
                    && entry.contents.iter().all(|c| self.selection.contains(c))
            })
            .map(|entry| (entry.id, entry.layer, entry.node.clone()))?;
        let NodeKind::Group(source_group) = &source_node.kind else {
            return None;
        };

        let n = self.fresh_id();
        let link_id = source_group
            .linked_group_id
            .clone()
            .unwrap_or_else(|| format!("link_{n}"));
        let delta = Vec3d::new(grid_size, 0.0, 0.0);

        // The copy: the same group with a fresh subtree, shifted, and its transformation
        // set to the shift.
        let mut copy_group = source_group.clone();
        copy_group.id = n.clone();
        copy_group.linked_group_id = Some(link_id.clone());
        copy_group.transformation = Some(translation_transformation(delta));
        copy_group.children = source_group
            .children
            .iter()
            .cloned()
            .map(deep_renumber)
            .collect();
        let world_bounds = self.world_bounds;
        let mut copy_node = Node::new(NodeKind::Group(copy_group));
        translate_paste(&mut copy_node, delta, &world_bounds);
        let copy_id = copy_node.id;

        // The original keeps its contents; it only gains the link id, and an identity
        // transformation if it had none, so it is the set's reference.
        let mut updated_source = source_group.clone();
        updated_source.linked_group_id = Some(link_id);
        if updated_source.transformation.is_none() {
            updated_source.transformation = Some(identity_transformation());
        }
        let after_source = Node {
            id: source_id,
            kind: NodeKind::Group(updated_source),
        };

        // Recorded directly rather than through `apply_and_record`: this call *is* the
        // linked-group machinery, and does not want its own propagation pass on top.
        let edit = Edit::Batch(vec![
            Edit::Changed {
                before: vec![source_node],
                after: vec![after_source],
            },
            Edit::Added {
                layer,
                nodes: vec![copy_node],
            },
        ]);
        self.apply(&edit);
        self.record_edit(edit);

        self.selection = node::contents_of(&self.tree.world, copy_id)
            .into_iter()
            .collect();
        self.face_selection.clear();
        self.mark_changed();
        Some(copy_id)
    }

    /// Pulls the selected objects out of the linked group they are in and into a new
    /// linked set of their own -- `extractLinkedGroups`. Every member of the original set
    /// loses those objects, and each gets a new sibling holding just them, at its own
    /// position. Returns the ids of the new groups, or `None` if the selection is not a
    /// proper part of one member of a set of two or more.
    pub fn extract_linked_groups(&mut self) -> Option<Vec<NodeId>> {
        // The linked set whose one member's direct children the selection is a proper,
        // non-empty part of.
        let is_selected = |child: &Node| {
            self.selection.contains(&child.id)
                || node::contents_of(&self.tree.world, child.id)
                    .iter()
                    .all(|c| self.selection.contains(c))
        };

        let (_link_id, members) = self.linked_sets().into_iter().find(|(_, ids)| {
            ids.len() >= 2
                && ids.iter().any(|group_id| {
                    match node::find_node(&self.tree.world, *group_id).map(|n| &n.kind) {
                        Some(NodeKind::Group(group)) => {
                            let flags: Vec<bool> =
                                group.children.iter().map(&is_selected).collect();
                            flags.iter().any(|f| *f) && !flags.iter().all(|f| *f)
                        }
                        _ => false,
                    }
                })
        })?;

        // Which child positions come out, from the member the selection is inside.
        let reference = *members.iter().find(|group_id| {
            matches!(
                node::find_node(&self.tree.world, **group_id).map(|n| &n.kind),
                Some(NodeKind::Group(group)) if group.children.iter().any(&is_selected)
            )
        })?;
        let extract_positions: Vec<usize> =
            match &node::find_node(&self.tree.world, reference)?.kind {
                NodeKind::Group(group) => group
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(_, child)| is_selected(child))
                    .map(|(index, _)| index)
                    .collect(),
                _ => return None,
            };
        if extract_positions.is_empty() {
            return None;
        }

        let new_link = format!("link_{}", self.fresh_id());
        let mut changes = Vec::new();
        let mut additions = Vec::new();
        let mut new_ids = Vec::new();

        for group_id in &members {
            let Some(node) = node::find_node(&self.tree.world, *group_id).cloned() else {
                continue;
            };
            let NodeKind::Group(group) = &node.kind else {
                continue;
            };
            let Some(layer) = node::layer_of(&self.tree.world, *group_id) else {
                continue;
            };
            if group.children.len() <= extract_positions.iter().copied().max().unwrap_or(0) {
                continue; // the members disagree on their shape; nothing safe to do
            }

            let mut extracted = Vec::new();
            let mut kept = Vec::new();
            for (index, child) in group.children.iter().cloned().enumerate() {
                if extract_positions.contains(&index) {
                    extracted.push(child);
                } else {
                    kept.push(child);
                }
            }

            let mut trimmed = node.clone();
            if let NodeKind::Group(g) = &mut trimmed.kind {
                g.children = kept;
            }
            changes.push(Edit::Changed {
                before: vec![node.clone()],
                after: vec![trimmed],
            });

            let new_group = Node::new(NodeKind::Group(Group {
                id: self.fresh_id(),
                name: group.name.clone(),
                linked_group_id: Some(new_link.clone()),
                transformation: group
                    .transformation
                    .clone()
                    .or_else(|| Some(identity_transformation())),
                children: extracted,
            }));
            new_ids.push(new_group.id);
            additions.push(Edit::Added {
                layer,
                nodes: vec![new_group],
            });
        }

        if new_ids.is_empty() {
            return None;
        }

        changes.append(&mut additions);
        let edit = Edit::Batch(changes);
        self.apply(&edit);
        self.record_edit(edit);

        self.selection = new_ids
            .iter()
            .flat_map(|id| node::contents_of(&self.tree.world, *id))
            .collect();
        self.face_selection.clear();
        self.mark_changed();
        Some(new_ids)
    }

    /// The linked groups in the world, gathered by their shared id.
    pub(super) fn linked_sets(&self) -> Vec<(String, Vec<NodeId>)> {
        let mut sets: Vec<(String, Vec<NodeId>)> = Vec::new();
        let mut index: HashMap<&str, usize> = HashMap::new();
        for (id, group) in node::groups_with_ids(&self.tree.world) {
            let Some(link) = &group.linked_group_id else {
                continue;
            };
            match index.get(link.as_str()) {
                Some(&at) => sets[at].1.push(id),
                None => {
                    index.insert(link, sets.len());
                    sets.push((link.clone(), vec![id]));
                }
            }
        }
        sets
    }

    /// Each linked group's id mapped to the ids of its contents, or `None` when no linked
    /// set has more than one member and so no edit could propagate. Taken before an edit
    /// is applied, it lets a node the edit removes from a group still be traced to its
    /// set.
    pub(super) fn linked_contents_snapshot(&self) -> Option<HashMap<NodeId, HashSet<NodeId>>> {
        let sets = self.linked_sets();
        if !sets.iter().any(|(_, ids)| ids.len() >= 2) {
            return None;
        }
        // One walk for every member's contents rather than one per member.
        Some(
            node::groups_with_contents(&self.tree.world)
                .into_iter()
                .filter(|entry| entry.group.linked_group_id.is_some())
                .map(|entry| (entry.id, entry.contents.into_iter().collect()))
                .collect(),
        )
    }

    /// Turns the facing of every point entity under `node` by `rotation`, recursing
    /// through groups and nested entities. Used when a rebuilt linked group sits at a
    /// rotation from its source: `transform_node` moves where the entities are, this
    /// turns where they point.
    fn turn_entity_facings(&self, node: &mut Node, rotation: &Mat4x4d) {
        match &mut node.kind {
            NodeKind::Entity(entity) => {
                let change = {
                    let definition = entity
                        .classname()
                        .and_then(|name| self.definitions.as_ref()?.find(name));
                    let declared: Vec<String> = definition
                        .map(|d| d.properties.iter().map(|p| p.key.clone()).collect())
                        .unwrap_or_default();
                    let facts = map_model::entity_rotation::EntityFacts {
                        classname: entity.classname().unwrap_or(""),
                        point_entity: entity.origin().is_some(),
                        definition_bounds_center: definition
                            .and_then(|d| d.bounds)
                            .map_or_else(Vec3d::zero, |bounds| bounds.center()),
                        has_target: self.entity_has_target(entity, definition),
                        definition_keys: &declared,
                    };
                    let info =
                        map_model::entity_rotation::rotation_info(&entity.properties, &facts);
                    map_model::entity_rotation::apply_rotation(&entity.properties, &info, rotation)
                };
                if let Some((key, value)) = change {
                    entity.set_property(&key, &value);
                }
                for child in &mut entity.children {
                    self.turn_entity_facings(child, rotation);
                }
            }
            NodeKind::Group(group) => {
                for child in &mut group.children {
                    self.turn_entity_facings(child, rotation);
                }
            }
            NodeKind::Layer(layer) => {
                for child in &mut layer.children {
                    self.turn_entity_facings(child, rotation);
                }
            }
            NodeKind::Brush(_) | NodeKind::Patch(_) => {}
        }
    }

    /// The edit that rebuilds the siblings of every linked set whose source a change has
    /// reached.
    ///
    /// `signal` names the nodes that pick out a set's source -- the selection for the
    /// menu action, the nodes an edit just touched for automatic propagation. `pre` gives
    /// each member's contents from before the edit, so a removed node still ties the
    /// change to its set. The siblings take the source's contents transformed by where
    /// their own `_tb_transformation` sits relative to it, geometry and entity facings
    /// alike. Returns `None` when nothing needs bringing into line.
    pub(super) fn linked_sync_edit(
        &self,
        signal: &HashSet<NodeId>,
        pre: Option<&HashMap<NodeId, HashSet<NodeId>>>,
        previous: Option<&HashMap<NodeId, &Node>>,
    ) -> Option<Edit> {
        let world_bounds = self.world_bounds;
        let mut before = Vec::new();
        let mut after = Vec::new();

        let sets = self.linked_sets();
        if !sets.iter().any(|(_, ids)| ids.len() >= 2) {
            return None;
        }
        // Every linked member's node and contents from one walk of the tree; looking
        // each up on its own walked the tree twice per member.
        let members: HashMap<NodeId, (&Node, Vec<NodeId>)> =
            node::groups_with_contents(&self.tree.world)
                .into_iter()
                .filter(|entry| entry.group.linked_group_id.is_some())
                .map(|entry| (entry.id, (entry.node, entry.contents)))
                .collect();
        for (_, ids) in sets {
            if ids.len() < 2 {
                continue;
            }
            let source_id = ids.iter().copied().find(|id| {
                let now = members
                    .get(id)
                    .map(|(_, contents)| contents.as_slice())
                    .unwrap_or_default();
                let was = pre.and_then(|pre| pre.get(id));
                now.iter().any(|c| signal.contains(c))
                    || was.is_some_and(|was| was.iter().any(|c| signal.contains(c)))
            });
            let Some(source_id) = source_id else {
                continue;
            };

            let Some(source_node) = members.get(&source_id).map(|(node, _)| (*node).clone()) else {
                continue;
            };
            let NodeKind::Group(source_group) = &source_node.kind else {
                continue;
            };
            let Some(source_matrix) = source_group
                .transformation
                .as_deref()
                .and_then(parse_transformation)
            else {
                continue;
            };
            let Some(source_inverse) = geometry_math::mat::invert(source_matrix) else {
                continue;
            };

            // Each sibling's copy of the source is built on its own, so they are built
            // side by side: five hundred linked copies are five hundred independent
            // rebuilds. Turning entity facings needs the definitions and stays serial.
            let work: Vec<(NodeId, &Node, &Group, Mat4x4d)> = ids
                .iter()
                .filter(|id| **id != source_id)
                .filter_map(|id| {
                    let sibling_node = members.get(id).map(|(node, _)| *node)?;
                    let NodeKind::Group(sibling_group) = &sibling_node.kind else {
                        return None;
                    };
                    let sibling_matrix = sibling_group
                        .transformation
                        .as_deref()
                        .and_then(parse_transformation)?;
                    Some((
                        *id,
                        sibling_node,
                        sibling_group,
                        sibling_matrix * source_inverse,
                    ))
                })
                .collect();

            // An edit that changed only some brushes or patches directly in the source
            // (not a nested group or entity, not the set of children) is carried to each
            // sibling's matching child alone: the children were built in the source's
            // order, so child i is the copy of source child i. Each copy keeps its id, and
            // the edit names only those children, so the rest of every sibling -- and
            // its 3D surfaces -- is left alone. The menu action (no `pre`) and anything
            // structural rebuild every child, as before.
            let leaf = |node: &Node| matches!(node.kind, NodeKind::Brush(_) | NodeKind::Patch(_));
            let changed: Vec<usize> = source_group
                .children
                .iter()
                .enumerate()
                .filter(|(_, child)| {
                    let mut under = HashSet::new();
                    super::add_ids_under(child, &mut under);
                    under.iter().any(|id| signal.contains(id))
                })
                .map(|(at, _)| at)
                .collect();
            let was: Option<Vec<&Node>> = previous.and_then(|previous| {
                changed
                    .iter()
                    .map(|&at| previous.get(&source_group.children[at].id).copied())
                    .collect()
            });
            let incremental = pre.is_some()
                && !changed.is_empty()
                && changed.iter().all(|&at| leaf(&source_group.children[at]))
                && was.is_some();
            // A copy's child i must be the copy of source child i as it was: same kind,
            // and where the source child stood, moved to the copy -- otherwise (a copy
            // built some other way, children reordered) every child is rebuilt.
            // Compared by the corners alone, moved by `relative` (exact for a convex
            // brush's box), which costs a fraction of transforming the brush itself.
            let same_place =
                |copy: &Node, was: &Node, relative: &Mat4x4d| match (&copy.kind, &was.kind) {
                    (NodeKind::Brush(copy), NodeKind::Brush(was)) => {
                        let moved = geometry_math::bbox::Bbox3d::merge_all(
                            was.vertex_positions().into_iter().map(|v| *relative * v),
                        );
                        moved.is_some_and(|moved| {
                            let bounds = copy.bounds();
                            geometry_math::vec::length(bounds.min - moved.min) < 0.01
                                && geometry_math::vec::length(bounds.max - moved.max) < 0.01
                        })
                    }
                    (NodeKind::Patch(copy), NodeKind::Patch(was)) => {
                        copy.control_points.len() == was.control_points.len()
                            && copy
                                .control_points
                                .iter()
                                .zip(&was.control_points)
                                .all(|(a, b)| {
                                    geometry_math::vec::length(a.position - *relative * b.position)
                                        < 0.01
                                })
                    }
                    _ => false,
                };
            let matches = |sibling: &Group, relative: &Mat4x4d| {
                let Some(was) = &was else { return false };
                sibling.children.len() == source_group.children.len()
                    && changed
                        .iter()
                        .zip(was)
                        .all(|(&at, was)| same_place(&sibling.children[at], was, relative))
            };
            let (piecewise, work): (Vec<_>, Vec<_>) = work
                .into_iter()
                .partition(|(_, _, sibling, relative)| incremental && matches(sibling, relative));
            let copy = |(_, _, sibling, relative): &(NodeId, &Node, &Group, Mat4x4d)| -> Vec<(Node, Node)> {
                changed
                    .iter()
                    .map(|&at| {
                        let mut copy = source_group.children[at].clone();
                        copy.id = sibling.children[at].id;
                        transform_node(&mut copy, relative, &world_bounds);
                        (sibling.children[at].clone(), copy)
                    })
                    .collect()
            };
            let copied: Vec<Vec<(Node, Node)>> = {
                use rayon::prelude::*;
                if piecewise.len() >= 8 {
                    piecewise.par_iter().map(copy).collect()
                } else {
                    piecewise.iter().map(copy).collect()
                }
            };
            for (was, now) in copied.into_iter().flatten() {
                before.push(was);
                after.push(now);
            }
            let build = |relative: &Mat4x4d| -> Vec<Node> {
                let mut children: Vec<Node> = source_group
                    .children
                    .iter()
                    .cloned()
                    .map(deep_renumber)
                    .collect();
                for child in &mut children {
                    transform_node(child, relative, &world_bounds);
                }
                children
            };
            let built: Vec<Vec<Node>> = {
                use rayon::prelude::*;
                if work.len() >= 8 {
                    work.par_iter()
                        .map(|(_, _, _, relative)| build(relative))
                        .collect()
                } else {
                    work.iter()
                        .map(|(_, _, _, relative)| build(relative))
                        .collect()
                }
            };

            for ((sibling_id, sibling_node, sibling_group, relative), children) in
                work.into_iter().zip(built)
            {
                let mut rebuilt = sibling_group.clone();
                rebuilt.children = children;
                // Geometry moves with `relative`; a point entity's facing has to be
                // turned in its own property, and only when there is a turn to make.
                let turn = mat_ext::strip_translation(&relative);
                if turn != Mat4x4d::identity() {
                    for child in &mut rebuilt.children {
                        self.turn_entity_facings(child, &turn);
                    }
                }

                // An entity's protected properties are its own -- a per-copy `targetname`,
                // say -- so the rebuilt copy takes those back from the sibling it replaces.
                // `preserveEntityProperties`.
                preserve_protected_properties(&mut rebuilt.children, &sibling_group.children);

                before.push(sibling_node.clone());
                after.push(Node {
                    id: sibling_id,
                    kind: NodeKind::Group(rebuilt),
                });
            }
        }

        if before.is_empty() {
            None
        } else {
            Some(Edit::Changed { before, after })
        }
    }

    /// Rebuilds every linked group from the one being edited, in one undo step.
    ///
    /// `updateLinkedGroups`. Every ordinary edit now carries this out on its own; the
    /// menu action is the way to run it deliberately -- from the member holding the
    /// selection -- after something the automatic pass does not see, such as a manual
    /// regroup. Returns how many groups were brought into line.
    pub fn sync_linked_groups(&mut self) -> usize {
        let signal = self.selection.clone();
        let Some(edit) = self.linked_sync_edit(&signal, None, None) else {
            return 0;
        };
        let count = match &edit {
            Edit::Changed { after, .. } => after.len(),
            _ => 0,
        };
        // The rebuilt groups have fresh child ids, so the old selection is stale.
        self.selection.clear();
        self.face_selection.clear();
        // Recorded directly: the sync is itself the linked-group machinery.
        self.apply(&edit);
        self.record_edit(edit);
        count
    }
}
