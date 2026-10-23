//! Document operations for entities.

use super::{
    node, DefaultProperties, Document, Edit, EntityNode, EntityUnder, Node, NodeId, NodeKind, Vec3d,
};

impl Document {
    /// Stands for worldspawn in the entity property calls, which name entities by node
    /// and worldspawn has none: `set_entity_property_on(&[Document::WORLDSPAWN], ...)`
    /// sets a key of the map itself. Minted ids never take it.
    pub const WORLDSPAWN: NodeId = NodeId(0);

    /// The class of the entity with the given id, if it is one.
    pub fn classname_of(&self, id: NodeId) -> Option<String> {
        node::entities_with_ids(&self.tree.world)
            .into_iter()
            .find(|(other, _)| *other == id)
            .and_then(|(_, entity)| entity.classname().map(ToString::to_string))
    }

    /// The entity the given node is or belongs to, worldspawn aside, with its class.
    ///
    /// `findContainingEntity`: a point entity is the node itself, a brush reports the
    /// brush entity it is part of. Worldspawn is left out, since "the brush belongs to the
    /// world" is not an entity worth naming.
    pub fn entity_around(&self, id: NodeId) -> Option<EntityUnder> {
        node::entities_with_ids(&self.tree.world)
            .into_iter()
            .find(|(entity_id, entity)| {
                *entity_id == id || entity.children.iter().any(|child| child.id == id)
            })
            .and_then(|(entity_id, entity)| {
                entity
                    .classname()
                    .filter(|name| !name.eq_ignore_ascii_case("worldspawn"))
                    .map(|name| EntityUnder {
                        id: entity_id,
                        classname: name.to_string(),
                        is_brush_entity: entity.is_brush_entity(),
                    })
            })
    }

    /// Moves the selected brushes and patches sitting straight on a layer into the given
    /// brush entity.
    ///
    /// `moveToEntity` with the entity `findNewParentEntityForNodes` picks, which the menu
    /// works out from what the pointer is over. The brushes stay selected; one undo step
    /// puts them back on the layer. Returns how many moved.
    pub fn move_selection_to_entity(&mut self, target: NodeId) -> usize {
        let Some(layer) = node::layer_of(&self.tree.world, target) else {
            return 0;
        };
        let Some(before) = node::find_node(&self.tree.world, target).cloned() else {
            return 0;
        };
        if !matches!(before.kind, NodeKind::Entity(_)) {
            return 0;
        }

        let taken: Vec<Node> = self.tree.world.layers[layer]
            .children
            .iter()
            .filter(|node| {
                self.selection.contains(&node.id)
                    && matches!(node.kind, NodeKind::Brush(_) | NodeKind::Patch(_))
            })
            .cloned()
            .collect();
        if taken.is_empty() {
            return 0;
        }
        let moved_ids: Vec<NodeId> = taken.iter().map(|node| node.id).collect();
        let count = moved_ids.len();

        let mut after = before.clone();
        if let NodeKind::Entity(entity) = &mut after.kind {
            entity.children.extend(taken.iter().cloned());
        }

        // The brushes leave the layer, then the entity gains them: undo does the reverse,
        // taking them back off the entity and dropping them onto the layer.
        self.apply_and_record(Edit::Batch(vec![
            Edit::Removed {
                layer,
                nodes: taken,
            },
            Edit::Changed {
                before: vec![before],
                after: vec![after],
            },
        ]));
        self.selection = moved_ids.into_iter().collect();
        self.mark_changed();
        count
    }

    /// Selects every entity of the given class, replacing what was selected.
    ///
    /// `selectEntitiesWithClassname`: the class is matched without regard to case, and
    /// worldspawn is left out since "select all worldspawn" would be "select the whole
    /// map". A point entity is taken by its own node; a brush entity is taken by its
    /// brushes, which is how a brush entity is held in the selection here. Anything on a
    /// hidden or locked layer is skipped, as it is for `select_all`.
    pub fn select_entities_with_classname(&mut self, classname: &str) -> usize {
        let wanted = classname.to_ascii_lowercase();
        let mut picked: Vec<NodeId> = Vec::new();
        for (id, entity) in node::entities_with_ids(&self.tree.world) {
            let Some(name) = entity.classname() else {
                continue;
            };
            if name.eq_ignore_ascii_case("worldspawn") || name.to_ascii_lowercase() != wanted {
                continue;
            }
            if !node::layer_of(&self.tree.world, id)
                .is_some_and(|layer| self.layer_is_editable(layer))
            {
                continue;
            }
            if entity.is_brush_entity() {
                picked.extend(entity.children.iter().map(|child| child.id));
            } else if !self.is_hidden(id) {
                picked.push(id);
            }
        }
        self.selection = picked.into_iter().collect();
        self.face_selection.clear();
        self.mark_changed();
        self.selection.len()
    }

    /// Places a point entity of the given class at the given position, and selects it.
    ///
    /// The origin is snapped by the caller; this puts it where it is told.
    pub fn create_point_entity(&mut self, classname: &str, origin: Vec3d) -> NodeId {
        let mut entity = EntityNode {
            properties: vec![(
                map_model::entity_properties::keys::CLASSNAME.to_string(),
                classname.to_string(),
            )],
            children: Vec::new(),
        };
        entity.set_origin(origin);

        // Properties the class defines with a default are written out, so the entity
        // looks in the editor the way it will behave in the game.
        if let Some(definition) = self.definitions.as_ref().and_then(|d| d.find(classname)) {
            for property in &definition.properties {
                if let Some(default) = &property.default {
                    if !default.is_empty() {
                        entity
                            .properties
                            .push((property.key.clone(), default.clone()));
                    }
                }
            }
        }

        let node = Node::new(NodeKind::Entity(entity));
        let id = node.id;
        self.selection = std::iter::once(id).collect();
        self.apply_and_record(Edit::Added {
            layer: 0,
            nodes: vec![node],
        });
        id
    }

    /// Returns the entity the selection consists of, if it is exactly one.
    ///
    /// The property editor shows one entity at a time, since two entities of different
    /// classes have nothing in common to edit.
    pub fn selected_entity(&self) -> Option<(NodeId, &EntityNode)> {
        if self.selection.len() != 1 {
            return None;
        }
        let id = *self.selection.iter().next()?;
        match &node::find_node(&self.tree.world, id)?.kind {
            NodeKind::Entity(entity) => Some((id, entity)),
            _ => None,
        }
    }

    /// Sets a property on the given entity, adding it if it is not there.
    ///
    /// Returns whether the node was found and is an entity.
    pub fn set_entity_property(&mut self, id: NodeId, key: &str, value: &str) -> bool {
        self.change_entity(id, |entity| {
            match entity.properties.iter_mut().find(|(k, _)| k == key) {
                Some((_, existing)) => *existing = value.to_string(),
                None => entity.properties.push((key.to_string(), value.to_string())),
            }
        })
    }

    /// Which of a class's default values to write onto the entities that are selected.
    ///
    /// Controls how default properties are applied.
    pub fn set_default_properties(&mut self, mode: DefaultProperties) -> usize {
        let selected: Vec<NodeId> = self.selection.iter().copied().collect();
        let mut written = 0;

        for id in selected {
            let Some(NodeKind::Entity(entity)) =
                node::find_node(&self.tree.world, id).map(|node| &node.kind)
            else {
                continue;
            };
            let Some(definition) = entity
                .classname()
                .and_then(|name| self.definitions.as_ref()?.find(name))
            else {
                continue;
            };

            // What to write, gathered before anything is: the entity is borrowed from the
            // tree the write goes into.
            let defaults: Vec<(String, String)> = definition
                .properties
                .iter()
                .filter_map(|property| Some((property.key.clone(), property.default.clone()?)))
                .filter(|(key, _)| {
                    let present = entity.property(key).is_some();
                    match mode {
                        DefaultProperties::Existing => present,
                        DefaultProperties::Missing => !present,
                    }
                })
                .collect();

            for (key, value) in defaults {
                if self.set_entity_property(id, &key, &value) {
                    written += 1;
                }
            }
        }
        written
    }

    /// Whether `key` is in the given entity's `_tb_protected_properties` list -- the keys
    /// it keeps its own value for when its linked group is synced from another member.
    pub fn is_entity_property_protected(&self, id: NodeId, key: &str) -> bool {
        match node::find_node(&self.tree.world, id).map(|node| &node.kind) {
            Some(NodeKind::Entity(entity)) => {
                entity.protected_properties().iter().any(|k| k == key)
            }
            _ => false,
        }
    }

    /// Whether the given node sits inside a linked group -- the only place a protected
    /// property does anything.
    pub fn node_is_in_linked_group(&self, id: NodeId) -> bool {
        node::groups_with_ids(&self.tree.world)
            .iter()
            .filter(|(_, group)| group.linked_group_id.is_some())
            .any(|(group_id, _)| node::contents_of(&self.tree.world, *group_id).contains(&id))
    }

    /// Applies a change to the entity held by the given node, recording it for undo.
    fn change_entity(&mut self, id: NodeId, edit: impl FnOnce(&mut EntityNode)) -> bool {
        let mut edit = Some(edit);
        self.change_entities(&[id], move |entity| {
            if let Some(edit) = edit.take() {
                edit(entity);
            }
        })
    }

    /// Applies the same change to every entity in `ids`, as one undo step. The `edit`
    /// closure runs once per entity. Returns whether any were changed.
    ///
    /// [`Document::WORLDSPAWN`] among `ids` changes worldspawn's own properties, in the
    /// same step; its classname stays what it is.
    fn change_entities(&mut self, ids: &[NodeId], mut edit: impl FnMut(&mut EntityNode)) -> bool {
        let mut world = None;
        if ids.contains(&Self::WORLDSPAWN) {
            let mut changed = self.worldspawn();
            edit(&mut changed);
            let classname = map_model::entity_properties::keys::CLASSNAME;
            changed.properties.retain(|(key, _)| key != classname);
            let before = self.tree.world.properties.clone();
            // A map read from a file holds its classname among the keys; it stays put.
            if let Some(at) = before.iter().position(|(key, _)| key == classname) {
                let entry = before[at].clone();
                changed
                    .properties
                    .insert(at.min(changed.properties.len()), entry);
            }
            if changed.properties != before {
                world = Some(Edit::WorldChanged {
                    before,
                    after: changed.properties,
                });
            }
        }
        let mut before = Vec::new();
        let mut after = Vec::new();
        for &id in ids {
            let Some(node) = node::find_node(&self.tree.world, id).cloned() else {
                continue;
            };
            let NodeKind::Entity(entity) = &node.kind else {
                continue;
            };
            let mut changed = entity.clone();
            edit(&mut changed);
            before.push(node.clone());
            after.push(Node {
                id,
                kind: NodeKind::Entity(changed),
            });
        }
        let nodes = (!before.is_empty()).then_some(Edit::Changed { before, after });
        match (world, nodes) {
            (None, None) => return false,
            (Some(edit), None) | (None, Some(edit)) => self.apply_and_record(edit),
            (Some(world), Some(nodes)) => self.apply_and_record(Edit::Batch(vec![world, nodes])),
        }
        true
    }

    /// Worldspawn as an entity: its properties, classname first, and no brushes. What the
    /// Entity panel shows when nothing is selected.
    pub fn worldspawn(&self) -> EntityNode {
        let mut properties = vec![(
            map_model::entity_properties::keys::CLASSNAME.to_string(),
            map_model::entity_properties::values::WORLDSPAWN.to_string(),
        )];
        properties.extend(
            self.tree
                .world
                .properties
                .iter()
                .filter(|(key, _)| key != map_model::entity_properties::keys::CLASSNAME)
                .cloned(),
        );
        EntityNode {
            properties,
            children: Vec::new(),
        }
    }

    /// The entities that are selected -- one, or several for a multi-entity property edit.
    pub fn selected_entities(&self) -> Vec<(NodeId, &EntityNode)> {
        // A walk of the tree per selected node is fine for a few; for thousands of
        // selected lights it was a quarter of a second every frame the Entity tab showed.
        if self.selection.len() > 8 {
            return node::entities_with_ids(&self.tree.world)
                .into_iter()
                .filter(|(id, _)| self.selection.contains(id))
                .collect();
        }
        self.selection
            .iter()
            .filter_map(|id| match &node::find_node(&self.tree.world, *id)?.kind {
                NodeKind::Entity(entity) => Some((*id, entity)),
                _ => None,
            })
            .collect()
    }

    /// Whether any of `ids` sits inside a linked group: [`Self::node_is_in_linked_group`]
    /// for many nodes in one walk of the tree.
    pub fn any_in_linked_group(&self, ids: &[NodeId]) -> bool {
        node::groups_with_contents(&self.tree.world)
            .iter()
            .filter(|entry| entry.group.linked_group_id.is_some())
            .any(|entry| entry.contents.iter().any(|id| ids.contains(id)))
    }

    /// [`Self::set_entity_property`] across several entities, one undo step.
    pub fn set_entity_property_on(&mut self, ids: &[NodeId], key: &str, value: &str) -> bool {
        self.change_entities(ids, |entity| {
            match entity.properties.iter_mut().find(|(k, _)| k == key) {
                Some((_, existing)) => *existing = value.to_string(),
                None => entity.properties.push((key.to_string(), value.to_string())),
            }
        })
    }

    /// Sets or clears one bit of a flags property on every entity in `ids`, each keeping
    /// its own value for the other bits, in one undo step. `SmartFlagsEditor`'s per-bit
    /// edit, which is what makes a multi-entity flags edit safe.
    pub fn set_entity_flag_on(&mut self, ids: &[NodeId], key: &str, bit: i64, set: bool) -> bool {
        self.set_entity_flag_seeded_on(ids, key, 0, bit, set)
    }

    /// [`Self::set_entity_flag_on`], but an entity that does not carry the property yet
    /// starts from `seed` rather than 0 -- the class's own default bits, so adding a flag
    /// from the defaults row does not drop the ones the class sets by default.
    pub fn set_entity_flag_seeded_on(
        &mut self,
        ids: &[NodeId],
        key: &str,
        seed: i64,
        bit: i64,
        set: bool,
    ) -> bool {
        self.change_entities(ids, |entity| {
            let current: i64 = entity
                .property(key)
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(seed);
            let updated = if set { current | bit } else { current & !bit };
            match entity.properties.iter_mut().find(|(k, _)| k == key) {
                Some((_, existing)) => *existing = updated.to_string(),
                None => entity
                    .properties
                    .push((key.to_string(), updated.to_string())),
            }
        })
    }

    /// Removing an entity property across several entities, one undo step.
    pub fn remove_entity_property_on(&mut self, ids: &[NodeId], key: &str) -> bool {
        self.change_entities(ids, |entity| entity.properties.retain(|(k, _)| k != key))
    }

    /// Renaming an entity property across several entities, one undo step.
    pub fn rename_entity_property_on(&mut self, ids: &[NodeId], from: &str, to: &str) -> bool {
        self.change_entities(ids, |entity| {
            if let Some((key, _)) = entity.properties.iter_mut().find(|(k, _)| k == from) {
                *key = to.to_string();
            }
            let mut protected = entity.protected_properties();
            if let Some(slot) = protected.iter_mut().find(|k| *k == from) {
                *slot = to.to_string();
                entity.set_protected_properties(&protected);
            }
        })
    }

    /// Changing entity property protection across several entities, one undo step.
    pub fn set_entity_property_protected_on(
        &mut self,
        ids: &[NodeId],
        key: &str,
        protected: bool,
    ) -> bool {
        self.change_entities(ids, |entity| {
            let mut keys = entity.protected_properties();
            let present = keys.iter().any(|k| k == key);
            if protected && !present {
                keys.push(key.to_string());
            } else if !protected && present {
                keys.retain(|k| k != key);
            } else {
                return;
            }
            entity.set_protected_properties(&keys);
        })
    }

    /// Moves the selected brushes and patches out of any brush entity they are in and back
    /// onto their layer -- `makeStructural` / "Move to World". A brush entity left with
    /// nothing goes too. One undo step. Returns how many objects moved.
    pub fn make_structural(&mut self) -> usize {
        let mut changes: Vec<Edit> = Vec::new();
        let mut additions: Vec<Edit> = Vec::new();
        let mut moved_ids: Vec<NodeId> = Vec::new();

        for layer_index in 0..self.tree.world.layers.len() {
            let entity_ids: Vec<NodeId> = self.tree.world.layers[layer_index]
                .children
                .iter()
                .filter(|node| matches!(&node.kind, NodeKind::Entity(e) if e.is_brush_entity()))
                .map(|node| node.id)
                .collect();

            for entity_id in entity_ids {
                let Some(node) = node::find_node(&self.tree.world, entity_id).cloned() else {
                    continue;
                };
                let NodeKind::Entity(entity) = &node.kind else {
                    continue;
                };

                let (leaving, staying): (Vec<Node>, Vec<Node>) = entity
                    .children
                    .iter()
                    .cloned()
                    .partition(|child| self.selection.contains(&child.id));
                if leaving.is_empty() {
                    continue;
                }
                moved_ids.extend(leaving.iter().map(|child| child.id));
                additions.push(Edit::Added {
                    layer: layer_index,
                    nodes: leaving,
                });

                if staying.is_empty() {
                    // The entity has nothing left; it goes with its brushes.
                    changes.push(Edit::Removed {
                        layer: layer_index,
                        nodes: vec![node],
                    });
                } else {
                    let mut trimmed = node.clone();
                    if let NodeKind::Entity(e) = &mut trimmed.kind {
                        e.children = staying;
                    }
                    changes.push(Edit::Changed {
                        before: vec![node],
                        after: vec![trimmed],
                    });
                }
            }
        }

        if moved_ids.is_empty() {
            return 0;
        }
        let count = moved_ids.len();
        changes.append(&mut additions);
        self.apply_and_record(Edit::Batch(changes));
        self.selection = moved_ids.into_iter().collect();
        self.mark_changed();
        count
    }

    /// Clears the protected-property list on every selected entity, and gives each
    /// formerly-protected key the value the entity's linked siblings carry -- so the
    /// entity is fully back in step with its set. `clearProtectedEntityProperties`.
    /// Returns how many entities changed.
    pub fn clear_protected_properties(&mut self) -> usize {
        // For every selected entity, the values its linked siblings hold for its protected
        // keys -- gathered before the tree is edited.
        let selected: Vec<NodeId> = self.selection.iter().copied().collect();
        let mut plans: Vec<(NodeId, Vec<(String, String)>)> = Vec::new();
        for id in &selected {
            let Some(NodeKind::Entity(entity)) =
                node::find_node(&self.tree.world, *id).map(|n| &n.kind)
            else {
                continue;
            };
            let protected = entity.protected_properties();
            if protected.is_empty() {
                continue;
            }
            let restored: Vec<(String, String)> = protected
                .iter()
                .filter_map(|key| {
                    self.linked_sibling_property(*id, key)
                        .map(|v| (key.clone(), v))
                })
                .collect();
            plans.push((*id, restored));
        }
        if plans.is_empty() {
            return 0;
        }

        let mut changed = 0;
        for (id, restored) in plans {
            let did = self.change_entities(&[id], |entity| {
                for (key, value) in &restored {
                    entity.set_property(key, value.clone());
                }
                entity.set_protected_properties(&[]);
            });
            if did {
                changed += 1;
            }
        }
        changed
    }

    /// The value a linked sibling of the given entity carries for `key` -- the entity at
    /// the same position in another member of its linked set.
    fn linked_sibling_property(&self, entity_id: NodeId, key: &str) -> Option<String> {
        // The linked set and the group the entity sits in.
        let (_, members) = self.linked_sets().into_iter().find(|(_, ids)| {
            ids.iter()
                .any(|group_id| node::contents_of(&self.tree.world, *group_id).contains(&entity_id))
        })?;
        let own_group = *members.iter().find(|group_id| {
            node::contents_of(&self.tree.world, **group_id).contains(&entity_id)
        })?;

        let position = self.entity_position_in_group(own_group, entity_id)?;
        for sibling_group in members.iter().filter(|id| **id != own_group) {
            if let Some(NodeKind::Group(group)) =
                node::find_node(&self.tree.world, *sibling_group).map(|n| &n.kind)
            {
                if let Some(NodeKind::Entity(sibling)) =
                    group.children.get(position).map(|node| &node.kind)
                {
                    if let Some(value) = sibling.property(key) {
                        return Some(value.to_string());
                    }
                }
            }
        }
        None
    }

    /// Which child of `group_id` the entity is, by position.
    fn entity_position_in_group(&self, group_id: NodeId, entity_id: NodeId) -> Option<usize> {
        match node::find_node(&self.tree.world, group_id).map(|n| &n.kind) {
            Some(NodeKind::Group(group)) => group
                .children
                .iter()
                .position(|child| child.id == entity_id),
            _ => None,
        }
    }
}
