//! The document tree: what the editor shows in its outliner.
//!
//! The `.map` format has no notion of layers or groups — it is a flat list of entities.
//! Editors record the structure anyway, by writing each layer and group as a
//! `func_group` entity carrying `_tb_` properties, and having every node name the layer or
//! group it belongs to. [`build_tree`] puts that structure back together, and
//! [`flatten_tree`] takes it apart again for writing.
//!
//! Nodes that name a parent which is not in the file are attached to the default layer
//! rather than dropped, and the discrepancy is reported. A map that has lost a layer
//! definition is still worth opening.

use rustc_hash::FxHashMap;
use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

use crate::brush::{Brush, BrushError};
use crate::entity_properties::{keys, values};
use crate::io::map_reader::{Brush as RawBrush, Entity, Entity as RawEntity, MapFile, Patch};
use crate::map_format::MapFormat;

/// A handle to a node that stays valid while the node is edited.
///
/// Selection, undo and linking all need to name a node without holding a reference to it
/// or relying on its position in the tree, both of which an edit can invalidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Returns an id that no node has been given yet -- in this process or, with
    /// overwhelming likelihood, in any other editor's, since the high bits are a random
    /// per-process prefix. That is what lets a live session apply an edit that names a
    /// node by an id another person's editor minted. See [`crate::ids::session_prefix`].
    pub fn next() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::OnceLock;
        static COUNTER: OnceLock<AtomicU64> = OnceLock::new();
        let counter = COUNTER.get_or_init(|| {
            AtomicU64::new((u64::from(crate::ids::session_prefix()) << 40) | 1)
        });
        NodeId(counter.fetch_add(1, Ordering::Relaxed))
    }
}

/// A node in the document tree.
#[derive(Debug, Clone)]
pub struct Node {
    /// The node's handle.
    pub id: NodeId,
    /// What the node is.
    pub kind: NodeKind,
}

impl Node {
    /// Creates a node with a fresh id.
    pub fn new(kind: NodeKind) -> Self {
        Self {
            id: NodeId::next(),
            kind,
        }
    }

    /// A node that already has an identity: one read back from a file that carried it,
    /// or one an edit from another editor named.
    pub fn with_id(id: NodeId, kind: NodeKind) -> Self {
        Self { id, kind }
    }

    /// Returns the brush this node holds, if it is a brush.
    #[inline]
    pub fn as_brush(&self) -> Option<&Brush> {
        match &self.kind {
            NodeKind::Brush(brush) => Some(brush),
            _ => None,
        }
    }
}

/// What a node is.
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// A layer: the top level grouping, always a child of the world.
    Layer(Layer),
    /// A group: a named collection the user can select and move as one.
    Group(Group),
    /// An entity, with or without brushes of its own.
    Entity(EntityNode),
    /// A brush.
    Brush(Brush),
    /// A Bézier patch.
    ///
    /// Nothing here edits one, but the tree has to hold it: a patch that the tree does
    /// not know about is a patch that is lost the next time the map is saved.
    Patch(Patch),
}

/// The world: the document's root.
#[derive(Debug, Clone)]
pub struct World {
    /// The worldspawn's properties, with the editor's own ones removed.
    pub properties: Vec<(String, String)>,
    /// The layers, in their sort order. The first is the default layer.
    pub layers: Vec<Layer>,
}

/// A layer.
#[derive(Debug, Clone)]
pub struct Layer {
    /// The layer's identifier, as written in `_tb_id`.
    ///
    /// The default layer has none, since it is the worldspawn itself.
    pub id: Option<String>,
    /// The layer's name.
    pub name: String,
    /// Where the layer sits in the layer list.
    pub sort_index: i32,
    /// Whether the layer is hidden.
    pub hidden: bool,
    /// Whether the layer is locked against editing.
    pub locked: bool,
    /// Whether the layer is left out of exports.
    pub omit_from_export: bool,
    /// The colour the layer list shows it in, as written in `_tb_layer_color`.
    ///
    /// Kept as it was read rather than parsed: nothing here changes it, and a colour
    /// written back from four rounded floats would not be the one the file had.
    pub color: Option<String>,
    /// The layer's contents.
    pub children: Vec<Node>,
}

/// A group.
#[derive(Debug, Clone)]
pub struct Group {
    /// The group's identifier, as written in `_tb_id`.
    pub id: String,
    /// The group's name.
    pub name: String,
    /// The id linking this group to the others it is a copy of, if it is linked.
    pub linked_group_id: Option<String>,
    /// The transformation applied to the group as a whole, as written in
    /// `_tb_transformation`.
    ///
    /// Kept as it was read: it is what tells a linked group how it sits relative to the
    /// others, and rewriting it from something derived would only lose precision.
    pub transformation: Option<String>,
    /// The group's contents.
    pub children: Vec<Node>,
}

/// An entity.
#[derive(Debug, Clone)]
pub struct EntityNode {
    /// The entity's properties, with the editor's own ones removed.
    pub properties: Vec<(String, String)>,
    /// The entity's brushes.
    pub children: Vec<Node>,
}

impl EntityNode {
    /// Returns the value of the given property.
    pub fn property(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Returns the entity's class.
    #[inline]
    pub fn classname(&self) -> Option<&str> {
        self.property(keys::CLASSNAME)
    }

    /// Returns whether this entity has brushes of its own.
    #[inline]
    pub fn is_brush_entity(&self) -> bool {
        !self.children.is_empty()
    }

    /// Returns where a point entity sits, from its `origin` property.
    ///
    /// Returns `None` for an entity with no origin, which is either a brush entity — its
    /// position is its brushes' — or a malformed one.
    pub fn origin(&self) -> Option<Vec3d> {
        parse_vec3(self.property(keys::ORIGIN)?)
    }

    /// Sets where a point entity sits.
    pub fn set_origin(&mut self, origin: Vec3d) {
        let value = format!(
            "{} {} {}",
            format_coordinate(origin.x()),
            format_coordinate(origin.y()),
            format_coordinate(origin.z())
        );
        self.set_property(keys::ORIGIN, value);
    }

    /// Sets a property, replacing the value it had or adding it at the end.
    ///
    /// `Entity::addOrUpdateProperty`. Adding at the end rather than in sorted order is
    /// what keeps a file's own property order when only one of them changes.
    pub fn set_property(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        match self.properties.iter_mut().find(|(k, _)| k == key) {
            Some((_, existing)) => *existing = value,
            None => self.properties.push((key.to_string(), value)),
        }
    }

    /// Removes a property, if it is there. `Entity::removeProperty`.
    pub fn remove_property(&mut self, key: &str) {
        self.properties.retain(|(k, _)| k != key);
    }

    /// The keys this entity keeps its own value for when its linked group is rebuilt from
    /// another member -- its own `targetname`, say. `Entity::protectedProperties`, read
    /// from the `;`-separated `_tb_protected_properties` list the way `MapReader` reads it.
    pub fn protected_properties(&self) -> Vec<String> {
        match self.property(keys::TB_PROTECTED_PROPERTIES) {
            Some(value) => text_utils::string_utils::str_split(value, ";")
                .iter()
                .map(|key| text_utils::string_format::str_unescape(key, ";", '\\'))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Sets the protected-property list, writing it back to `_tb_protected_properties` (or
    /// dropping the key when the list is empty), the way `NodeWriter` writes it.
    pub fn set_protected_properties(&mut self, keys: &[String]) {
        if keys.is_empty() {
            self.remove_property(keys::TB_PROTECTED_PROPERTIES);
            return;
        }
        let joined = keys
            .iter()
            .map(|key| text_utils::string_format::str_escape(key, ";", '\\'))
            .collect::<Vec<_>>()
            .join(";");
        self.set_property(keys::TB_PROTECTED_PROPERTIES, joined);
    }
}

/// Reads three whitespace-separated numbers, as an entity property writes them.
fn parse_vec3(text: &str) -> Option<Vec3d> {
    let numbers: Vec<f64> = text
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    (numbers.len() == 3).then(|| Vec3d::new(numbers[0], numbers[1], numbers[2]))
}

/// Formats a coordinate the way a map file writes one: whole numbers without a point.
fn format_coordinate(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Walks the tree and returns every point entity in it, with the id of its node.
///
/// A point entity is one with an origin and no brushes: a light, a spawn point, a
/// trigger's target. These have no geometry of their own, so the editor draws them as a
/// box at their origin.
pub fn point_entities(world: &World) -> Vec<(NodeId, &EntityNode)> {
    let mut result = Vec::new();
    for layer in &world.layers {
        collect_point_entities(&layer.children, &mut result);
    }
    result
}

fn collect_point_entities<'a>(
    nodes: &'a [Node],
    out: &mut Vec<(NodeId, &'a EntityNode)>,
) {
    for node in nodes {
        match &node.kind {
            NodeKind::Entity(entity) => {
                if !entity.is_brush_entity() && entity.origin().is_some() {
                    out.push((node.id, entity));
                }
                collect_point_entities(&entity.children, out);
            }
            NodeKind::Group(group) => collect_point_entities(&group.children, out),
            NodeKind::Layer(layer) => collect_point_entities(&layer.children, out),
            NodeKind::Brush(_) | NodeKind::Patch(_) => {}
        }
    }
}

/// The name given to the default layer, which the `.map` format does not record.
pub const DEFAULT_LAYER_NAME: &str = "Default Layer";

/// Something [`build_tree`] worked around rather than giving up on.
#[derive(Debug, Clone, PartialEq)]
pub enum TreeWarning {
    /// A node named a parent that is not in the file, so it went to the default layer.
    MissingParent {
        /// The id that was named.
        id: String,
        /// The line of the node that named it.
        line: usize,
    },
    /// A brush's faces do not enclose a volume, so the brush was dropped.
    InvalidBrush {
        /// Why it could not be built.
        cause: BrushError,
        /// The line the brush starts on.
        line: usize,
    },
    /// The file has no worldspawn, so an empty one was assumed.
    NoWorldspawn,
}

impl std::fmt::Display for TreeWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeWarning::MissingParent { id, line } => write!(
                f,
                "line {line}: no layer or group has id \"{id}\", so its contents went to the default layer"
            ),
            TreeWarning::InvalidBrush { cause, line } => {
                write!(f, "line {line}: dropped a brush: {cause}")
            }
            TreeWarning::NoWorldspawn => {
                f.write_str("the file has no worldspawn entity, so an empty world was assumed")
            }
        }
    }
}

/// A document tree, and whatever had to be worked around to build it.
#[derive(Debug, Clone)]
pub struct Tree {
    /// The world.
    pub world: World,
    /// What the reader worked around.
    pub warnings: Vec<TreeWarning>,
}

/// Builds the document tree from a map file.
pub fn build_tree(map: &MapFile, world_bounds: &Bbox3d) -> Tree {
    build_tree_with(map, world_bounds, |_, entity, bounds, warnings| brush_nodes(entity, bounds, warnings))
}

/// Consumes parsed geometry instead of copying every face and patch on map open.
pub fn build_tree_owned(mut map: MapFile, world_bounds: &Bbox3d) -> Tree {
    let mut geometry: Vec<_> = map.entities.iter_mut().map(|entity| {
        Some((std::mem::take(&mut entity.brushes), std::mem::take(&mut entity.patches)))
    }).collect();
    build_tree_with(&map, world_bounds, |index, _, bounds, warnings| {
        let (brushes, patches) = geometry[index].take().expect("entity geometry is used once");
        let mut nodes = Vec::with_capacity(brushes.len() + patches.len());
        for patch in patches {
            nodes.push(match patch.id {
                Some(id) => Node::with_id(NodeId(id), NodeKind::Patch(patch)),
                None => Node::new(NodeKind::Patch(patch)),
            });
        }
        for brush in brushes {
            match Brush::from_faces(bounds, brush.faces) {
                Ok(built) => nodes.push(match brush.id {
                    Some(id) => Node::with_id(NodeId(id), NodeKind::Brush(built)),
                    None => Node::new(NodeKind::Brush(built)),
                }),
                Err(cause) => warnings.push(TreeWarning::InvalidBrush { cause, line: brush.line }),
            }
        }
        nodes
    })
}

fn build_tree_with(
    map: &MapFile,
    world_bounds: &Bbox3d,
    mut geometry: impl FnMut(usize, &RawEntity, &Bbox3d, &mut Vec<TreeWarning>) -> Vec<Node>,
) -> Tree {
    let mut warnings = Vec::new();

    // The worldspawn is the first entity with that classname, not simply the first
    // entity: a hand-edited file may have moved it.
    let worldspawn = map
        .entities
        .iter()
        .enumerate()
        .find(|(_, e)| e.classname() == Some(values::WORLDSPAWN));
    if worldspawn.is_none() {
        warnings.push(TreeWarning::NoWorldspawn);
    }

    let mut default_layer = Layer {
        id: None,
        name: DEFAULT_LAYER_NAME.to_string(),
        sort_index: 0,
        hidden: false,
        locked: false,
        omit_from_export: false,
        color: None,
        children: Vec::new(),
    };

    // The worldspawn is the default layer, so it carries that layer's own properties as
    // well as the world's, and its brushes are the layer's contents.
    if let Some((index, worldspawn)) = worldspawn {
        default_layer.hidden = worldspawn.property(keys::TB_LAYER_HIDDEN) == Some(values::TRUE);
        default_layer.locked = worldspawn.property(keys::TB_LAYER_LOCKED) == Some(values::TRUE);
        default_layer.omit_from_export =
            worldspawn.property(keys::TB_LAYER_OMIT_FROM_EXPORT) == Some(values::TRUE);
        default_layer.color = worldspawn
            .property(keys::TB_LAYER_COLOR)
            .map(str::to_string);
        default_layer
            .children
            .extend(geometry(index, worldspawn, world_bounds, &mut warnings));
    }

    // First pass: pick out the entities that stand for layers and groups.
    let mut layers: Vec<Layer> = Vec::new();
    let mut groups: PendingGroups = Vec::new();
    let mut contents: Vec<(usize, &RawEntity, Option<String>)> = Vec::new();

    for (index, entity) in map.entities.iter().enumerate() {
        if entity.classname() == Some(values::WORLDSPAWN) {
            continue;
        }

        match entity.property(keys::TB_TYPE) {
            Some(values::TYPE_LAYER) => {
                let mut layer = read_layer(entity);
                layer.children = geometry(index, entity, world_bounds, &mut warnings);
                layers.push(layer)
            }
            Some(values::TYPE_GROUP) => {
                let mut group = read_group(entity);
                group.group.children = geometry(index, entity, world_bounds, &mut warnings);
                groups.push(group)
            }
            _ => {
                let parent = parent_id(entity);
                contents.push((index, entity, parent));
            }
        }
    }

    // Keep the first occurrence of duplicate ids, matching the previous linear search.
    let mut parents = ParentIndex::default();
    for (index, group) in groups.iter().enumerate() {
        parents.groups.entry(group.group.id.clone()).or_insert(index);
    }
    for (index, layer) in layers.iter().enumerate() {
        if let Some(id) = &layer.id {
            parents.layers.entry(id.clone()).or_insert(index);
        }
    }

    // Second pass: fill each group with the entities that name it. Groups are filled
    // before layers because a group may itself sit inside another group.
    for (index, entity, parent) in contents {
        // The file's own id where it has one, so an entity is the same entity across a
        // save; a fresh one otherwise.
        let kind = NodeKind::Entity(EntityNode {
            properties: game_properties(&entity.properties),
            children: geometry(index, entity, world_bounds, &mut warnings),
        });
        let node = match entity.id {
            Some(id) => Node::with_id(NodeId(id), kind),
            None => Node::new(kind),
        };
        place(
            node,
            parent,
            entity.line,
            &parents,
            &mut groups,
            &mut layers,
            &mut default_layer,
            &mut warnings,
        );
    }

    // Groups nest inside their parent group or layer. Working from the back means a
    // group is placed only after anything nested in it has been.
    while let Some(group) = groups.pop() {
        // Popping preserves all remaining indices. Remove only this occurrence:
        // an earlier group with the same id must remain the preferred parent.
        if parents.groups.get(&group.group.id) == Some(&groups.len()) {
            parents.groups.remove(&group.group.id);
        }
        let parent = group.parent.clone();
        let line = group.line;
        let node = Node::new(NodeKind::Group(group.group));
        place(
            node,
            parent,
            line,
            &parents,
            &mut groups,
            &mut layers,
            &mut default_layer,
            &mut warnings,
        );
    }

    layers.sort_by_key(|layer| layer.sort_index);
    let mut all_layers = vec![default_layer];
    all_layers.extend(layers);

    let properties = worldspawn
        .map(|(_, w)| game_properties(&w.properties))
        .unwrap_or_default();

    Tree {
        world: World {
            properties,
            layers: all_layers,
        },
        warnings,
    }
}

/// A group together with where it belongs, while the tree is being assembled.
struct PendingGroup {
    group: Group,
    parent: Option<String>,
    line: usize,
}

// The two collections the placer works over hold different things, so it takes them
// separately rather than behind a trait.
type PendingGroups = Vec<PendingGroup>;

#[derive(Default)]
struct ParentIndex {
    groups: FxHashMap<String, usize>,
    layers: FxHashMap<String, usize>,
}

/// Puts a node under the group or layer with the given id, falling back to the default
/// layer and reporting the discrepancy when there is no such parent.
fn place(
    node: Node,
    parent: Option<String>,
    line: usize,
    parents: &ParentIndex,
    groups: &mut PendingGroups,
    layers: &mut [Layer],
    default_layer: &mut Layer,
    warnings: &mut Vec<TreeWarning>,
) {
    let Some(id) = parent else {
        default_layer.children.push(node);
        return;
    };

    if let Some(&index) = parents.groups.get(&id) {
        groups[index].group.children.push(node);
        return;
    }
    if let Some(&index) = parents.layers.get(&id) {
        layers[index].children.push(node);
        return;
    }

    warnings.push(TreeWarning::MissingParent { id, line });
    default_layer.children.push(node);
}

/// Returns the id of the group or layer an entity says it belongs to.
///
/// A node names both when it is inside a group that is itself inside a layer; the group
/// is the immediate parent, so it wins.
fn parent_id(entity: &RawEntity) -> Option<String> {
    entity
        .property(keys::TB_GROUP)
        .or_else(|| entity.property(keys::TB_LAYER))
        .map(str::to_string)
}

/// Reads a layer from the `func_group` entity standing for it.
///
/// The entity's own brushes are the layer's contents; anything else in the layer names it
/// by id and is attached later.
fn read_layer(entity: &RawEntity) -> Layer {
    Layer {
        id: entity.property(keys::TB_ID).map(str::to_string),
        name: entity
            .property(keys::TB_NAME)
            .unwrap_or("Unnamed Layer")
            .to_string(),
        sort_index: entity
            .property(keys::TB_LAYER_SORT_INDEX)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0),
        hidden: entity.property(keys::TB_LAYER_HIDDEN) == Some(values::TRUE),
        locked: entity.property(keys::TB_LAYER_LOCKED) == Some(values::TRUE),
        omit_from_export: entity.property(keys::TB_LAYER_OMIT_FROM_EXPORT)
            == Some(values::TRUE),
        color: entity.property(keys::TB_LAYER_COLOR).map(str::to_string),
        children: Vec::new(),
    }
}

/// Reads a group from the `func_group` entity standing for it.
fn read_group(entity: &RawEntity) -> PendingGroup {
    PendingGroup {
        group: Group {
            id: entity.property(keys::TB_ID).unwrap_or_default().to_string(),
            name: entity
                .property(keys::TB_NAME)
                .unwrap_or("Unnamed Group")
                .to_string(),
            linked_group_id: entity
                .property(keys::TB_LINKED_GROUP_ID)
                .map(str::to_string),
            transformation: entity
                .property(keys::TB_GROUP_TRANSFORMATION)
                .map(str::to_string),
            children: Vec::new(),
        },
        parent: parent_id(entity),
        line: entity.line,
    }
}

/// Builds the brushes of an entity, reporting and dropping any that do not enclose a
/// volume.
/// The solid contents of an entity: its brushes, and its patches.
///
/// Patches are carried through unchanged. Nothing here edits one, but a patch the tree
/// does not hold is a patch that vanishes the next time the map is saved.
fn brush_nodes(
    entity: &RawEntity,
    world_bounds: &Bbox3d,
    warnings: &mut Vec<TreeWarning>,
) -> Vec<Node> {
    let mut nodes = patch_nodes(entity);
    nodes.extend(built_brushes(entity, world_bounds, warnings));
    nodes
}

/// The entity's patches, as nodes.
fn patch_nodes(entity: &RawEntity) -> Vec<Node> {
    entity
        .patches
        .iter()
        .map(|patch| match patch.id {
            Some(id) => Node::with_id(NodeId(id), NodeKind::Patch(patch.clone())),
            None => Node::new(NodeKind::Patch(patch.clone())),
        })
        .collect()
}

fn built_brushes(
    entity: &RawEntity,
    world_bounds: &Bbox3d,
    warnings: &mut Vec<TreeWarning>,
) -> Vec<Node> {
    entity
        .brushes
        .iter()
        .filter_map(
            |brush| match Brush::from_faces(world_bounds, brush.faces.clone()) {
                // The file's own id where it has one, so a brush is the same brush
                // across a save; a fresh one for a brush from an editor that writes
                // none, which is every other editor in the lineage.
                Ok(built) => Some(match brush.id {
                    Some(id) => Node::with_id(NodeId(id), NodeKind::Brush(built)),
                    None => Node::new(NodeKind::Brush(built)),
                }),
                Err(cause) => {
                    warnings.push(TreeWarning::InvalidBrush {
                        cause,
                        line: brush.line,
                    });
                    None
                }
            },
        )
        .collect()
}

/// Strips the properties the tree itself stands for, leaving everything else.
///
/// The structural `_tb_` properties are represented by the tree once it is built; keeping
/// them as well would mean two sources of truth that could disagree. The rest — the
/// material collections, the definition file, an entity's protected properties — is data
/// nothing here reads, and dropping it would lose it on the next save.
fn game_properties(properties: &[(String, String)]) -> Vec<(String, String)> {
    properties
        .iter()
        .filter(|(key, _)| !crate::entity_properties::is_structural_property(key))
        .cloned()
        .collect()
}

/// Takes the document tree apart into the flat entity list a `.map` file holds.
///
/// This is the inverse of [`build_tree`]. The structure the format cannot express —
/// layers and groups — goes back to being `func_group` entities carrying `_tb_`
/// properties, with every node naming its parent by id.
///
/// Layers and groups that have no id, because the editor made them rather than a file,
/// are given one here.
pub fn flatten_tree(world: &World, format: MapFormat, game: Option<String>) -> MapFile {
    let mut entities = Vec::new();
    let mut next_id = 1u32;

    // The worldspawn comes first and holds the default layer's contents directly.
    let default_layer = world.layers.first();
    let mut worldspawn = Entity {
        properties: world.properties.clone(),
        ..Entity::default()
    };
    if worldspawn.property(keys::CLASSNAME).is_none() {
        worldspawn
            .properties
            .insert(0, (keys::CLASSNAME.to_string(), values::WORLDSPAWN.to_string()));
    }
    if let Some(layer) = default_layer {
        set_layer_properties(&mut worldspawn.properties, layer);
        collect_children(&layer.children, None, &mut worldspawn, &mut entities, &mut next_id);
    }
    entities.insert(0, worldspawn);

    // Every other layer becomes an entity of its own.
    for layer in world.layers.iter().skip(1) {
        let id = layer
            .id
            .clone()
            .unwrap_or_else(|| take_id(&mut next_id));

        let mut entity = Entity {
            properties: vec![
                (keys::CLASSNAME.to_string(), values::GROUP_CLASSNAME.to_string()),
                (keys::TB_TYPE.to_string(), values::TYPE_LAYER.to_string()),
                (keys::TB_NAME.to_string(), layer.name.clone()),
                (keys::TB_ID.to_string(), id.clone()),
                (
                    keys::TB_LAYER_SORT_INDEX.to_string(),
                    layer.sort_index.to_string(),
                ),
            ],
            ..Entity::default()
        };
        set_layer_properties(&mut entity.properties, layer);

        collect_children(
            &layer.children,
            Some(Parent::Layer(&id)),
            &mut entity,
            &mut entities,
            &mut next_id,
        );
        entities.push(entity);
    }

    MapFile {
        format,
        game,
        entities,
        // Flattening the tree is what a save writes, so what comes out carries this
        // editor's own version whatever the file it was read from said.
        notes_version: Some(crate::map_format::NOTES_VERSION),
        warnings: Vec::new(),
    }
}

/// Writes out the properties a layer carries beyond its name and id.
///
/// The default layer is the worldspawn, so these go on that entity rather than one of its
/// own; either way they are the same properties.
fn set_layer_properties(properties: &mut Vec<(String, String)>, layer: &Layer) {
    for (key, set) in [
        (keys::TB_LAYER_HIDDEN, layer.hidden),
        (keys::TB_LAYER_LOCKED, layer.locked),
        (keys::TB_LAYER_OMIT_FROM_EXPORT, layer.omit_from_export),
    ] {
        if set {
            properties.push((key.to_string(), values::TRUE.to_string()));
        }
    }
    if let Some(color) = &layer.color {
        properties.push((keys::TB_LAYER_COLOR.to_string(), color.clone()));
    }
}

/// Writes the children of a layer or group out.
///
/// Brushes go into `owner`, since a `.map` entity holds its brushes directly. Everything
/// else becomes an entity of its own, naming `parent` as where it belongs.
fn collect_children(
    nodes: &[Node],
    parent: Option<Parent<'_>>,
    owner: &mut Entity,
    entities: &mut Vec<Entity>,
    next_id: &mut u32,
) {
    for node in nodes {
        match &node.kind {
            // The node's own id goes out with it, so the brush that comes back is the
            // same brush: a merge can then tell an edit from a removal and an addition,
            // and a live session can name a node the other editor also holds.
            NodeKind::Brush(brush) => owner.brushes.push(RawBrush {
                faces: brush.faces().to_vec(),
                id: Some(node.id.0),
                line: 0,
            }),
            // A patch goes back out exactly as it came in: nothing here edits one, and
            // rewriting it from something derived would only lose precision.
            NodeKind::Patch(patch) => owner.patches.push(Patch {
                id: Some(node.id.0),
                ..patch.clone()
            }),
            NodeKind::Entity(entity) => {
                let mut written = Entity {
                    properties: entity.properties.clone(),
                    // As for a brush: the node's own identity goes out with it.
                    id: Some(node.id.0),
                    ..Entity::default()
                };
                set_parent(&mut written, parent);
                collect_children(&entity.children, parent, &mut written, entities, next_id);
                entities.push(written);
            }
            NodeKind::Group(group) => {
                let id = if group.id.is_empty() {
                    take_id(next_id)
                } else {
                    group.id.clone()
                };

                let mut written = Entity {
                    properties: vec![
                        (keys::CLASSNAME.to_string(), values::GROUP_CLASSNAME.to_string()),
                        (keys::TB_TYPE.to_string(), values::TYPE_GROUP.to_string()),
                        (keys::TB_NAME.to_string(), group.name.clone()),
                        (keys::TB_ID.to_string(), id.clone()),
                    ],
                    ..Entity::default()
                };
                if let Some(linked) = &group.linked_group_id {
                    written
                        .properties
                        .push((keys::TB_LINKED_GROUP_ID.to_string(), linked.clone()));
                }
                if let Some(transformation) = &group.transformation {
                    written.properties.push((
                        keys::TB_GROUP_TRANSFORMATION.to_string(),
                        transformation.clone(),
                    ));
                }
                set_parent(&mut written, parent);

                collect_children(
                    &group.children,
                    Some(Parent::Group(&id)),
                    &mut written,
                    entities,
                    next_id,
                );
                entities.push(written);
            }
            // A layer inside a layer is not something the format can express, and
            // `build_tree` never produces one.
            NodeKind::Layer(layer) => {
                collect_children(&layer.children, parent, owner, entities, next_id)
            }
        }
    }
}

/// Which node a child names as the one it belongs to.
///
/// The two are written under different keys: `_tb_layer` for a layer and `_tb_group` for
/// a group. A node inside a group that is itself inside a layer names only the group,
/// since that is its immediate parent.
#[derive(Clone, Copy)]
enum Parent<'a> {
    /// The node is directly in a layer.
    Layer(&'a str),
    /// The node is in a group.
    Group(&'a str),
}

/// Records which layer or group an entity belongs to.
///
/// A node in the default layer names nothing, since the worldspawn is that layer.
fn set_parent(entity: &mut Entity, parent: Option<Parent<'_>>) {
    let (key, id) = match parent {
        Some(Parent::Layer(id)) => (keys::TB_LAYER, id),
        Some(Parent::Group(id)) => (keys::TB_GROUP, id),
        None => return,
    };
    entity.properties.push((key.to_string(), id.to_string()));
}

/// Returns an id no layer or group in this file has used.
fn take_id(next: &mut u32) -> String {
    // Ids read from a file are arbitrary strings, so a fresh one only has to be distinct
    // from the others in the same file. Starting high keeps it clear of small ids a
    // hand-written file is likely to use.
    let id = format!("{}", 1_000_000 + *next);
    *next += 1;
    id
}

/// Walks the tree and replaces every brush with the result of the given function.
///
/// A brush the function declines to replace is left as it is, so an edit that cannot be
/// applied to one brush still applies to the rest.
pub fn map_brushes<F>(world: &mut World, mut f: F)
where
    F: FnMut(&Brush) -> Option<Brush>,
{
    for layer in &mut world.layers {
        map_brushes_in(&mut layer.children, &mut f);
    }
}

fn map_brushes_in<F>(nodes: &mut [Node], f: &mut F)
where
    F: FnMut(&Brush) -> Option<Brush>,
{
    for node in nodes {
        match &mut node.kind {
            NodeKind::Brush(brush) => {
                if let Some(replacement) = f(brush) {
                    *brush = replacement;
                }
            }
            NodeKind::Group(group) => map_brushes_in(&mut group.children, f),
            NodeKind::Entity(entity) => map_brushes_in(&mut entity.children, f),
            NodeKind::Layer(layer) => map_brushes_in(&mut layer.children, f),
            NodeKind::Patch(_) => {}
        }
    }
}

/// Walks the tree and returns every brush in it.
pub fn brushes(world: &World) -> Vec<&Brush> {
    let mut result = Vec::new();
    for layer in &world.layers {
        collect_brushes(&layer.children, &mut result);
    }
    result
}

fn collect_brushes<'a>(nodes: &'a [Node], out: &mut Vec<&'a Brush>) {
    for node in nodes {
        match &node.kind {
            NodeKind::Brush(brush) => out.push(brush),
            NodeKind::Group(group) => collect_brushes(&group.children, out),
            NodeKind::Entity(entity) => collect_brushes(&entity.children, out),
            NodeKind::Layer(layer) => collect_brushes(&layer.children, out),
            NodeKind::Patch(_) => {}
        }
    }
}

/// Removes the nodes with the given ids from anywhere in the given list, including
/// inside groups and entities.
pub fn remove_nodes(nodes: &mut Vec<Node>, ids: &[NodeId]) {
    // A handful of ids is quicker to scan than to hash; a big deletion (thousands of
    // brushes out of tens of thousands) was a scan of the list per node in the tree.
    if ids.len() <= 16 {
        remove_where(nodes, &|id| ids.contains(&id));
    } else {
        let set: rustc_hash::FxHashSet<NodeId> = ids.iter().copied().collect();
        remove_where(nodes, &|id| set.contains(&id));
    }
}

/// [`remove_nodes`] over every layer of the world, hashing the ids once rather than once
/// per layer.
pub fn remove_nodes_from_world(world: &mut World, ids: &[NodeId]) {
    if ids.len() <= 16 {
        for layer in &mut world.layers {
            remove_where(&mut layer.children, &|id| ids.contains(&id));
        }
    } else {
        let set: rustc_hash::FxHashSet<NodeId> = ids.iter().copied().collect();
        for layer in &mut world.layers {
            remove_where(&mut layer.children, &|id| set.contains(&id));
        }
    }
}

fn remove_where(nodes: &mut Vec<Node>, doomed: &dyn Fn(NodeId) -> bool) {
    nodes.retain(|node| !doomed(node.id));
    for node in nodes {
        match &mut node.kind {
            NodeKind::Group(group) => remove_where(&mut group.children, doomed),
            NodeKind::Entity(entity) => remove_where(&mut entity.children, doomed),
            NodeKind::Layer(layer) => remove_where(&mut layer.children, doomed),
            NodeKind::Brush(_) | NodeKind::Patch(_) => {}
        }
    }
}

/// The id of every node in the world, for asking "is this still there?" of many ids at
/// once rather than walking the tree for each.
pub fn all_node_ids(world: &World) -> rustc_hash::FxHashSet<NodeId> {
    fn collect(nodes: &[Node], out: &mut rustc_hash::FxHashSet<NodeId>) {
        for node in nodes {
            out.insert(node.id);
            match &node.kind {
                NodeKind::Group(group) => collect(&group.children, out),
                NodeKind::Entity(entity) => collect(&entity.children, out),
                NodeKind::Layer(layer) => collect(&layer.children, out),
                NodeKind::Brush(_) | NodeKind::Patch(_) => {}
            }
        }
    }
    let mut out = rustc_hash::FxHashSet::default();
    for layer in &world.layers {
        collect(&layer.children, &mut out);
    }
    out
}

/// Walks the tree and returns every brush in it, with the id of the node holding it.
pub fn brushes_with_ids(world: &World) -> Vec<(NodeId, &Brush)> {
    brushes_by_layer(world)
        .into_iter()
        .map(|(id, brush, _)| (id, brush))
        .collect()
}

/// Every brush in the world, with its id and which layer it is on.
///
/// The layer is what says whether a brush is drawn, picked or editable, and finding it
/// afterwards would mean walking the tree again for each brush.
pub fn brushes_by_layer(world: &World) -> Vec<(NodeId, &Brush, usize)> {
    let mut result = Vec::new();
    for (index, layer) in world.layers.iter().enumerate() {
        let mut in_layer = Vec::new();
        collect_brushes_with_ids(&layer.children, &mut in_layer);
        result.extend(
            in_layer
                .into_iter()
                .map(|(id, brush)| (id, brush, index)),
        );
    }
    result
}

/// Every patch in the world, with its id and which layer it is on.
pub fn patches_by_layer(world: &World) -> Vec<(NodeId, &Patch, usize)> {
    fn collect<'a>(nodes: &'a [Node], out: &mut Vec<(NodeId, &'a Patch)>) {
        for node in nodes {
            match &node.kind {
                NodeKind::Patch(patch) => out.push((node.id, patch)),
                NodeKind::Group(group) => collect(&group.children, out),
                NodeKind::Entity(entity) => collect(&entity.children, out),
                NodeKind::Layer(layer) => collect(&layer.children, out),
                NodeKind::Brush(_) => {}
            }
        }
    }

    let mut result = Vec::new();
    for (index, layer) in world.layers.iter().enumerate() {
        let mut in_layer = Vec::new();
        collect(&layer.children, &mut in_layer);
        result.extend(
            in_layer
                .into_iter()
                .map(|(id, patch)| (id, patch, index)),
        );
    }
    result
}

/// Every point entity in the world, with its id and which layer it is on.
pub fn point_entities_by_layer(world: &World) -> Vec<(NodeId, &EntityNode, usize)> {
    let mut result = Vec::new();
    for (index, layer) in world.layers.iter().enumerate() {
        let mut in_layer = Vec::new();
        collect_point_entities(&layer.children, &mut in_layer);
        result.extend(
            in_layer
                .into_iter()
                .map(|(id, entity)| (id, entity, index)),
        );
    }
    result
}

/// Every entity in the world, with its id.
///
/// Point entities and brush entities alike: a validator cares about both, and which one a
/// class is meant to be is one of the things being checked.
pub fn entities_with_ids(world: &World) -> Vec<(NodeId, &EntityNode)> {
    let mut result = Vec::new();
    for layer in &world.layers {
        collect_entities_with_ids(&layer.children, &mut result);
    }
    result
}

fn collect_entities_with_ids<'a>(
    nodes: &'a [Node],
    out: &mut Vec<(NodeId, &'a EntityNode)>,
) {
    for node in nodes {
        match &node.kind {
            NodeKind::Entity(entity) => {
                out.push((node.id, entity));
                collect_entities_with_ids(&entity.children, out);
            }
            NodeKind::Group(group) => collect_entities_with_ids(&group.children, out),
            NodeKind::Layer(layer) => collect_entities_with_ids(&layer.children, out),
            NodeKind::Brush(_) | NodeKind::Patch(_) => {}
        }
    }
}

/// Which layer the node with the given id is on.
pub fn layer_of(world: &World, id: NodeId) -> Option<usize> {
    for (index, layer) in world.layers.iter().enumerate() {
        if contains_node(&layer.children, id) {
            return Some(index);
        }
    }
    None
}

fn contains_node(nodes: &[Node], id: NodeId) -> bool {
    nodes.iter().any(|node| {
        node.id == id
            || match &node.kind {
                NodeKind::Group(group) => contains_node(&group.children, id),
                NodeKind::Entity(entity) => contains_node(&entity.children, id),
                NodeKind::Layer(layer) => contains_node(&layer.children, id),
                NodeKind::Brush(_) | NodeKind::Patch(_) => false,
            }
    })
}

/// The outermost group containing the given node, if it is in one.
///
/// Outermost rather than innermost: clicking a brush in a group nested in another group
/// takes the whole of the outer one, and opening a group is what reaches inside it.
pub fn outermost_group_of(world: &World, id: NodeId, open: Option<NodeId>) -> Option<NodeId> {
    fn walk(
        nodes: &[Node],
        id: NodeId,
        open: Option<NodeId>,
        outermost: Option<NodeId>,
    ) -> Option<Option<NodeId>> {
        for node in nodes {
            if node.id == id {
                return Some(outermost);
            }
            let found = match &node.kind {
                NodeKind::Group(group) => {
                    // An open group is transparent: what is in it is reached directly.
                    let outermost = if outermost.is_none() && Some(node.id) != open {
                        Some(node.id)
                    } else {
                        outermost
                    };
                    walk(&group.children, id, open, outermost)
                }
                NodeKind::Entity(entity) => walk(&entity.children, id, open, outermost),
                NodeKind::Layer(layer) => walk(&layer.children, id, open, outermost),
                NodeKind::Brush(_) | NodeKind::Patch(_) => None,
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    for layer in &world.layers {
        if let Some(found) = walk(&layer.children, id, open, None) {
            return found;
        }
    }
    None
}

/// The ids of every direct child of the container the given node sits in — its layer,
/// its group or its brush entity — the node itself included.
///
/// This is what `selectSiblingNodes` reaches for through `node->parent()->children()`.
/// An id that is not in the tree returns nothing.
pub fn siblings_of(world: &World, id: NodeId) -> Vec<NodeId> {
    fn search(nodes: &[Node], id: NodeId) -> Option<Vec<NodeId>> {
        if nodes.iter().any(|node| node.id == id) {
            return Some(nodes.iter().map(|node| node.id).collect());
        }
        for node in nodes {
            let children = match &node.kind {
                NodeKind::Group(group) => &group.children,
                NodeKind::Entity(entity) => &entity.children,
                NodeKind::Layer(layer) => &layer.children,
                NodeKind::Brush(_) | NodeKind::Patch(_) => continue,
            };
            if let Some(found) = search(children, id) {
                return Some(found);
            }
        }
        None
    }

    for layer in &world.layers {
        if let Some(found) = search(&layer.children, id) {
            return found;
        }
    }
    Vec::new()
}

/// Every brush, patch and entity inside the given node, however deeply.
pub fn contents_of(world: &World, id: NodeId) -> Vec<NodeId> {
    fn collect(nodes: &[Node], out: &mut Vec<NodeId>) {
        for node in nodes {
            match &node.kind {
                NodeKind::Brush(_) => out.push(node.id),
                NodeKind::Entity(entity) => {
                    out.push(node.id);
                    collect(&entity.children, out);
                }
                NodeKind::Group(group) => collect(&group.children, out),
                NodeKind::Layer(layer) => collect(&layer.children, out),
                // A patch is content as much as a brush is. It was left out here once,
                // which made a group holding only patches report itself empty -- so it
                // had no outline, could not be selected by a click, and refused to be
                // linked-duplicated.
                NodeKind::Patch(_) => out.push(node.id),
            }
        }
    }

    fn find(nodes: &[Node], id: NodeId) -> Option<&Node> {
        for node in nodes {
            if node.id == id {
                return Some(node);
            }
            let inside = match &node.kind {
                NodeKind::Group(group) => find(&group.children, id),
                NodeKind::Entity(entity) => find(&entity.children, id),
                NodeKind::Layer(layer) => find(&layer.children, id),
                NodeKind::Brush(_) | NodeKind::Patch(_) => None,
            };
            if inside.is_some() {
                return inside;
            }
        }
        None
    }

    let mut out = Vec::new();
    for layer in &world.layers {
        if let Some(node) = find(&layer.children, id) {
            match &node.kind {
                NodeKind::Group(group) => collect(&group.children, &mut out),
                NodeKind::Entity(entity) => {
                    out.push(node.id);
                    collect(&entity.children, &mut out);
                }
                _ => out.push(node.id),
            }
            return out;
        }
    }
    out
}

/// Every group in the world with its id and how many objects it holds however deeply
/// (what [`contents_of`] would return for it), in [`groups_with_ids`] order: one walk of
/// the tree rather than one per group.
pub fn groups_with_content_counts(world: &World) -> Vec<(NodeId, &Group, usize)> {
    fn collect<'a>(nodes: &'a [Node], out: &mut Vec<(NodeId, &'a Group, usize)>) -> usize {
        let mut count = 0;
        for node in nodes {
            match &node.kind {
                NodeKind::Group(group) => {
                    let at = out.len();
                    out.push((node.id, group, 0));
                    let inside = collect(&group.children, out);
                    out[at].2 = inside;
                    count += inside;
                }
                NodeKind::Entity(entity) => count += 1 + collect(&entity.children, out),
                NodeKind::Layer(layer) => count += collect(&layer.children, out),
                NodeKind::Brush(_) | NodeKind::Patch(_) => count += 1,
            }
        }
        count
    }

    let mut out = Vec::new();
    for layer in &world.layers {
        collect(&layer.children, &mut out);
    }
    out
}

/// One group as [`groups_with_contents`] finds it.
pub struct GroupEntry<'a> {
    /// The group's node id.
    pub id: NodeId,
    /// The group's node, children and all.
    pub node: &'a Node,
    /// The group itself.
    pub group: &'a Group,
    /// The index of the layer it is on.
    pub layer: usize,
    /// What [`contents_of`] returns for it.
    pub contents: Vec<NodeId>,
}

/// Every group in [`groups_with_ids`] order with its node, its layer and its contents,
/// in one walk of the tree: what the group commands used to find with a `find_node`,
/// a `layer_of` and a `contents_of` per group, which made them quadratic in the number
/// of groups.
pub fn groups_with_contents(world: &World) -> Vec<GroupEntry<'_>> {
    fn collect<'a>(nodes: &'a [Node], layer: usize, open: &mut Vec<usize>, out: &mut Vec<GroupEntry<'a>>) {
        for node in nodes {
            match &node.kind {
                NodeKind::Group(group) => {
                    open.push(out.len());
                    out.push(GroupEntry { id: node.id, node, group, layer, contents: Vec::new() });
                    collect(&group.children, layer, open, out);
                    open.pop();
                }
                NodeKind::Entity(entity) => {
                    for &at in open.iter() {
                        out[at].contents.push(node.id);
                    }
                    collect(&entity.children, layer, open, out);
                }
                NodeKind::Layer(inner) => collect(&inner.children, layer, open, out),
                NodeKind::Brush(_) | NodeKind::Patch(_) => {
                    for &at in open.iter() {
                        out[at].contents.push(node.id);
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    let mut open = Vec::new();
    for (index, layer) in world.layers.iter().enumerate() {
        collect(&layer.children, index, &mut open, &mut out);
    }
    out
}

/// For every object inside a closed group, the outermost closed group it is in, and for
/// each of those groups its contents ([`contents_of`]): [`outermost_group_of`] and
/// [`contents_of`] for the whole tree in one walk, for selecting many objects at once.
/// `open` is transparent, as there.
pub fn outermost_groups(world: &World, open: Option<NodeId>) -> (rustc_hash::FxHashMap<NodeId, NodeId>, rustc_hash::FxHashMap<NodeId, Vec<NodeId>>) {
    fn walk(
        nodes: &[Node],
        open: Option<NodeId>,
        outermost: Option<NodeId>,
        members: &mut rustc_hash::FxHashMap<NodeId, NodeId>,
        contents: &mut rustc_hash::FxHashMap<NodeId, Vec<NodeId>>,
    ) {
        for node in nodes {
            match &node.kind {
                NodeKind::Group(group) => {
                    let inner = if outermost.is_none() && Some(node.id) != open { Some(node.id) } else { outermost };
                    if let Some(g) = outermost {
                        members.insert(node.id, g);
                    }
                    if inner.is_some() && outermost.is_none() {
                        contents.entry(node.id).or_default();
                    }
                    walk(&group.children, open, inner, members, contents);
                }
                NodeKind::Entity(entity) => {
                    if let Some(g) = outermost {
                        members.insert(node.id, g);
                        contents.entry(g).or_default().push(node.id);
                    }
                    walk(&entity.children, open, outermost, members, contents);
                }
                NodeKind::Layer(layer) => walk(&layer.children, open, outermost, members, contents),
                NodeKind::Brush(_) | NodeKind::Patch(_) => {
                    if let Some(g) = outermost {
                        members.insert(node.id, g);
                        contents.entry(g).or_default().push(node.id);
                    }
                }
            }
        }
    }

    let mut members = rustc_hash::FxHashMap::default();
    let mut contents = rustc_hash::FxHashMap::default();
    for layer in &world.layers {
        walk(&layer.children, open, None, &mut members, &mut contents);
    }
    (members, contents)
}

/// Every group in the world, with its id.
pub fn groups_with_ids(world: &World) -> Vec<(NodeId, &Group)> {
    fn collect<'a>(nodes: &'a [Node], out: &mut Vec<(NodeId, &'a Group)>) {
        for node in nodes {
            match &node.kind {
                NodeKind::Group(group) => {
                    out.push((node.id, group));
                    collect(&group.children, out);
                }
                NodeKind::Entity(entity) => collect(&entity.children, out),
                NodeKind::Layer(layer) => collect(&layer.children, out),
                NodeKind::Brush(_) | NodeKind::Patch(_) => {}
            }
        }
    }

    let mut out = Vec::new();
    for layer in &world.layers {
        collect(&layer.children, &mut out);
    }
    out
}

/// Puts the nodes with the given ids into a new group on the given layer.
///
/// `groupSelectedNodes`. Returns the group's id, or nothing if none of the ids were
/// found. The nodes keep their own identities; only where they hang from changes.
pub fn make_group(
    world: &mut World,
    ids: &[NodeId],
    name: impl Into<String>,
    id: String,
    layer: usize,
) -> Option<NodeId> {
    if layer >= world.layers.len() {
        return None;
    }

    let mut taken = Vec::new();
    for existing in &mut world.layers {
        take_nodes(&mut existing.children, ids, &mut taken);
    }
    if taken.is_empty() {
        return None;
    }

    let group = Node::new(NodeKind::Group(Group {
        id,
        name: name.into(),
        linked_group_id: None,
        transformation: None,
        children: taken,
    }));
    let group_id = group.id;
    world.layers[layer].children.push(group);
    Some(group_id)
}

/// Puts the given brushes under a new entity of the given class.
///
/// `createBrushEntity`: a door, a platform or a trigger is brushes with a classname on
/// them, so making one is moving those brushes under an entity node. Returns the entity's
/// id, or nothing if none of the ids were found.
pub fn make_brush_entity(
    world: &mut World,
    ids: &[NodeId],
    classname: &str,
    layer: usize,
) -> Option<NodeId> {
    if layer >= world.layers.len() {
        return None;
    }

    let mut taken = Vec::new();
    for existing in &mut world.layers {
        take_nodes(&mut existing.children, ids, &mut taken);
    }
    if taken.is_empty() {
        return None;
    }

    let entity = Node::new(NodeKind::Entity(EntityNode {
        properties: vec![(
            crate::entity_properties::keys::CLASSNAME.to_string(),
            classname.to_string(),
        )],
        children: taken,
    }));
    let entity_id = entity.id;
    world.layers[layer].children.push(entity);
    Some(entity_id)
}

/// Takes the given group apart, leaving its contents where it was.
///
/// `ungroupSelectedNodes`. Returns the ids of what came out of it.
pub fn dissolve_group(world: &mut World, group_id: NodeId) -> Vec<NodeId> {
    /// Takes the group apart wherever it is, leaving its contents in its place.
    fn dissolve(nodes: &mut Vec<Node>, group_id: NodeId) -> Option<Vec<NodeId>> {
        if let Some(index) = nodes.iter().position(|node| node.id == group_id) {
            let NodeKind::Group(group) = &mut nodes[index].kind else {
                // That id is not a group, so there is nothing to take apart.
                return None;
            };
            let children = std::mem::take(&mut group.children);
            let ids = children.iter().map(|node| node.id).collect();
            nodes.remove(index);
            nodes.extend(children);
            return Some(ids);
        }

        for node in nodes.iter_mut() {
            let inside = match &mut node.kind {
                NodeKind::Group(group) => dissolve(&mut group.children, group_id),
                NodeKind::Entity(entity) => dissolve(&mut entity.children, group_id),
                NodeKind::Layer(layer) => dissolve(&mut layer.children, group_id),
                NodeKind::Brush(_) | NodeKind::Patch(_) => None,
            };
            if inside.is_some() {
                return inside;
            }
        }
        None
    }

    for layer in &mut world.layers {
        if let Some(ids) = dissolve(&mut layer.children, group_id) {
            return ids;
        }
    }
    Vec::new()
}

/// Moves the nodes with the given ids onto the given layer.
///
/// Returns how many moved. A node already on that layer is left where it is, so its place
/// in the layer's order is not disturbed.
pub fn move_to_layer(world: &mut World, ids: &[NodeId], layer: usize) -> usize {
    if layer >= world.layers.len() {
        return 0;
    }

    let mut moved = Vec::new();
    for (index, existing) in world.layers.iter_mut().enumerate() {
        if index == layer {
            continue;
        }
        take_nodes(&mut existing.children, ids, &mut moved);
    }

    let count = moved.len();
    world.layers[layer].children.extend(moved);
    count
}

/// Takes the nodes with the given ids out of the tree.
fn take_nodes(nodes: &mut Vec<Node>, ids: &[NodeId], out: &mut Vec<Node>) {
    let mut index = 0;
    while index < nodes.len() {
        if ids.contains(&nodes[index].id) {
            out.push(nodes.remove(index));
            continue;
        }
        match &mut nodes[index].kind {
            NodeKind::Group(group) => take_nodes(&mut group.children, ids, out),
            NodeKind::Entity(entity) => take_nodes(&mut entity.children, ids, out),
            NodeKind::Layer(layer) => take_nodes(&mut layer.children, ids, out),
            NodeKind::Brush(_) | NodeKind::Patch(_) => {}
        }
        index += 1;
    }
}

fn collect_brushes_with_ids<'a>(nodes: &'a [Node], out: &mut Vec<(NodeId, &'a Brush)>) {
    for node in nodes {
        match &node.kind {
            NodeKind::Brush(brush) => out.push((node.id, brush)),
            NodeKind::Group(group) => collect_brushes_with_ids(&group.children, out),
            NodeKind::Entity(entity) => collect_brushes_with_ids(&entity.children, out),
            NodeKind::Layer(layer) => collect_brushes_with_ids(&layer.children, out),
            NodeKind::Patch(_) => {}
        }
    }
}

/// [`replace_node`] for each of `replacements` in turn, in one walk of the tree rather
/// than one per node: an edit of a few hundred brushes in a map of fifty thousand was a
/// few hundred full walks. Returns how many were found.
///
/// The result is the one the calls in turn would give: where a node and one inside it
/// are both replaced, the later of the two in the list wins for the inner one.
pub fn replace_nodes(world: &mut World, replacements: &[Node]) -> usize {
    match replacements {
        [] => return 0,
        [only] => return usize::from(replace_node(world, only.clone())),
        _ => {}
    }
    // The last of each id, as replacing in turn leaves it.
    let wanted: rustc_hash::FxHashMap<NodeId, usize> =
        replacements.iter().enumerate().map(|(index, node)| (node.id, index)).collect();

    fn walk(nodes: &mut [Node], replacements: &[Node], wanted: &rustc_hash::FxHashMap<NodeId, usize>, floor: Option<usize>, found: &mut usize) {
        for node in nodes.iter_mut() {
            let mut floor = floor;
            if let Some(&index) = wanted.get(&node.id) {
                // A replacement listed before its replaced ancestor was overwritten by it.
                if floor.is_none_or(|ancestor| index > ancestor) {
                    *node = replacements[index].clone();
                    *found += 1;
                    floor = Some(index);
                }
            }
            match &mut node.kind {
                NodeKind::Group(group) => walk(&mut group.children, replacements, wanted, floor, found),
                NodeKind::Entity(entity) => walk(&mut entity.children, replacements, wanted, floor, found),
                NodeKind::Layer(layer) => walk(&mut layer.children, replacements, wanted, floor, found),
                NodeKind::Brush(_) | NodeKind::Patch(_) => {}
            }
        }
    }

    let mut found = 0;
    for layer in &mut world.layers {
        walk(&mut layer.children, replacements, &wanted, None, &mut found);
    }
    found
}

/// [`replace_nodes`], moving the replacements into the tree rather than copying them:
/// what a drag applies on every pointer move and then throws away.
pub fn replace_nodes_owned(world: &mut World, replacements: Vec<Node>) -> usize {
    if replacements.len() <= 1 {
        return replace_nodes(world, &replacements);
    }
    let wanted: rustc_hash::FxHashMap<NodeId, usize> =
        replacements.iter().enumerate().map(|(index, node)| (node.id, index)).collect();
    let mut slots: Vec<Option<Node>> = replacements.into_iter().map(Some).collect();

    fn walk(nodes: &mut [Node], slots: &mut [Option<Node>], wanted: &rustc_hash::FxHashMap<NodeId, usize>, floor: Option<usize>, found: &mut usize) {
        for node in nodes.iter_mut() {
            let mut floor = floor;
            if let Some(&index) = wanted.get(&node.id) {
                if floor.is_none_or(|ancestor| index > ancestor) {
                    if let Some(replacement) = slots[index].take() {
                        *node = replacement;
                        *found += 1;
                        floor = Some(index);
                    }
                }
            }
            match &mut node.kind {
                NodeKind::Group(group) => walk(&mut group.children, slots, wanted, floor, found),
                NodeKind::Entity(entity) => walk(&mut entity.children, slots, wanted, floor, found),
                NodeKind::Layer(layer) => walk(&mut layer.children, slots, wanted, floor, found),
                NodeKind::Brush(_) | NodeKind::Patch(_) => {}
            }
        }
    }

    let mut found = 0;
    for layer in &mut world.layers {
        walk(&mut layer.children, &mut slots, &wanted, None, &mut found);
    }
    found
}

/// Replaces the node with the given one's id, keeping its place in the tree.
///
/// Returns whether such a node was found. Replacing rather than removing and re-adding is
/// what keeps a node's position in its layer stable across an edit.
pub fn replace_node(world: &mut World, replacement: Node) -> bool {
    for layer in &mut world.layers {
        if replace_node_in(&mut layer.children, &replacement) {
            return true;
        }
    }
    false
}

fn replace_node_in(nodes: &mut [Node], replacement: &Node) -> bool {
    for node in nodes.iter_mut() {
        if node.id == replacement.id {
            *node = replacement.clone();
            return true;
        }
        let found = match &mut node.kind {
            NodeKind::Group(group) => replace_node_in(&mut group.children, replacement),
            NodeKind::Entity(entity) => replace_node_in(&mut entity.children, replacement),
            NodeKind::Layer(layer) => replace_node_in(&mut layer.children, replacement),
            NodeKind::Brush(_) | NodeKind::Patch(_) => false,
        };
        if found {
            return true;
        }
    }
    false
}

/// Replaces the brush held by the node with the given id.
///
/// Returns whether such a node was found.
pub fn replace_brush(world: &mut World, id: NodeId, brush: Brush) -> bool {
    replace_node(
        world,
        Node {
            id,
            kind: NodeKind::Brush(brush),
        },
    )
}

/// Returns the node with the given id, wherever it is in the tree.
pub fn find_node(world: &World, id: NodeId) -> Option<&Node> {
    fn search(nodes: &[Node], id: NodeId) -> Option<&Node> {
        for node in nodes {
            if node.id == id {
                return Some(node);
            }
            let found = match &node.kind {
                NodeKind::Group(group) => search(&group.children, id),
                NodeKind::Entity(entity) => search(&entity.children, id),
                NodeKind::Layer(layer) => search(&layer.children, id),
                NodeKind::Brush(_) | NodeKind::Patch(_) => None,
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    world.layers.iter().find_map(|layer| search(&layer.children, id))
}
