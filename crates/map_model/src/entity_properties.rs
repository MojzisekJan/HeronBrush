//! The entity property keys and values the editor gives meaning to.
//!
//! Most entity properties belong to the game rather than the editor, and are carried
//! through untouched. The ones named here are the exceptions: `classname` and `origin`,
//! which every editor needs, and the `_tb_` prefixed ones the `.map` convention uses to record
//! layers and groups — structure the `.map` format has no place for, so it is written as
//! ordinary entities and reassembled on load.

/// Property keys.
pub mod keys {
    /// The entity's class, which decides what it is.
    pub const CLASSNAME: &str = "classname";
    /// A point entity's position.
    pub const ORIGIN: &str = "origin";
    /// A point entity's orientation, as a single yaw angle.
    pub const ANGLE: &str = "angle";
    /// A point entity's orientation, as pitch, yaw and roll.
    pub const ANGLES: &str = "angles";
    /// The name other entities target this one by.
    pub const TARGETNAME: &str = "targetname";
    /// The name of the entity this one targets.
    pub const TARGET: &str = "target";
    /// The entity's per-class option bits.
    pub const SPAWNFLAGS: &str = "spawnflags";
    /// Set to `220` on the worldspawn of a Valve 220 format map.
    pub const MAP_VERSION: &str = "mapversion";

    /// The prefix on every key the editor owns.
    pub const TB_PREFIX: &str = "_tb_";

    /// Whether this entity stands for a layer or a group.
    pub const TB_TYPE: &str = "_tb_type";
    /// A layer's or group's own identifier.
    pub const TB_ID: &str = "_tb_id";
    /// A layer's or group's name.
    pub const TB_NAME: &str = "_tb_name";
    /// The id of the layer this node belongs to.
    pub const TB_LAYER: &str = "_tb_layer";
    /// The id of the group this node belongs to.
    pub const TB_GROUP: &str = "_tb_group";
    /// A group's transformation, as sixteen matrix elements.
    pub const TB_GROUP_TRANSFORMATION: &str = "_tb_transformation";
    /// The id linking a group to the others it is a copy of.
    pub const TB_LINKED_GROUP_ID: &str = "_tb_linked_group_id";
    /// A layer's colour in the layer list.
    pub const TB_LAYER_COLOR: &str = "_tb_layer_color";
    /// Set when a layer is hidden.
    pub const TB_LAYER_HIDDEN: &str = "_tb_layer_hidden";
    /// Set when a layer is locked against editing.
    pub const TB_LAYER_LOCKED: &str = "_tb_layer_locked";
    /// Set when a layer is left out of exports.
    pub const TB_LAYER_OMIT_FROM_EXPORT: &str = "_tb_layer_omit_from_export";
    /// A layer's position in the layer list.
    pub const TB_LAYER_SORT_INDEX: &str = "_tb_layer_sort_index";
    /// The material collections enabled for the map.
    pub const TB_ENABLED_MATERIAL_COLLECTIONS: &str = "_tb_textures";
    /// The entity definition files the map uses.
    pub const TB_ENTITY_DEFINITIONS: &str = "_tb_def";
    /// The bounds the editor warns about leaving.
    pub const TB_SOFT_MAP_BOUNDS: &str = "_tb_soft_map_bounds";
    /// The properties of an entity in a linked group that are its own rather than shared
    /// with the other members.
    pub const TB_PROTECTED_PROPERTIES: &str = "_tb_protected_properties";
}

/// Property values with a fixed meaning.
pub mod values {
    /// The class of the entity holding the world's own geometry.
    pub const WORLDSPAWN: &str = "worldspawn";
    /// The class a layer or group is written as.
    ///
    /// Both use the same class; [`super::keys::TB_TYPE`] tells them apart.
    pub const GROUP_CLASSNAME: &str = "func_group";
    /// The `_tb_type` of an entity standing for a layer.
    pub const TYPE_LAYER: &str = "_tb_layer";
    /// The `_tb_type` of an entity standing for a group.
    pub const TYPE_GROUP: &str = "_tb_group";
    /// The value a set boolean layer flag takes.
    pub const TRUE: &str = "1";
    /// The value of [`super::keys::TB_SOFT_MAP_BOUNDS`] when there are none.
    pub const NO_SOFT_MAP_BOUNDS: &str = "none";
}

/// Returns whether the given key is one the editor owns rather than the game.
#[inline]
pub fn is_editor_property(key: &str) -> bool {
    key.starts_with(keys::TB_PREFIX)
}

/// Returns whether the given key is one the node tree itself stands for.
///
/// These are the properties [`super::node::flatten_tree`] writes back out from the tree's
/// own shape, so keeping them on a node as well would mean two sources of truth that
/// could disagree. Every other `_tb_` property is data the tree does not represent — the
/// material collections, the definition file, an entity's protected properties — and is
/// carried through untouched rather than dropped.
pub fn is_structural_property(key: &str) -> bool {
    matches!(
        key,
        keys::TB_TYPE
            | keys::TB_ID
            | keys::TB_NAME
            | keys::TB_LAYER
            | keys::TB_GROUP
            | keys::TB_GROUP_TRANSFORMATION
            | keys::TB_LINKED_GROUP_ID
            | keys::TB_LAYER_COLOR
            | keys::TB_LAYER_HIDDEN
            | keys::TB_LAYER_LOCKED
            | keys::TB_LAYER_OMIT_FROM_EXPORT
            | keys::TB_LAYER_SORT_INDEX
    )
}
