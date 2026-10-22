//! The wire format for an [`Edit`].
//!
//! An `Edit` is already a self-contained, reversible operation on whole nodes; [`WireEdit`]
//! is its serializable form. A brush is reduced to its faces on the wire and rebuilt with
//! [`map_model::brush::Brush::from_faces`] on the far side, so the half-edge geometry never
//! travels. Node ids carry a random per-process prefix ([`map_model::ids::session_prefix`]), so
//! a `WireEdit` names the nodes it touches by id and the receiving editor applies it to
//! those nodes directly.

use geometry_math::bbox::Bbox3d;
use map_model::brush::Brush;
use map_model::brush_face::BrushFace;
use map_model::io::map_reader::Patch;
use map_model::node::{EntityNode, Group, Layer, Node, NodeId, NodeKind};
use serde::{Deserialize, Serialize};

use crate::history::Edit;

/// Why a [`WireEdit`] could not be turned back into an [`Edit`].
#[derive(Debug, Clone, PartialEq)]
pub enum WireError {
    /// The JSON did not parse.
    Json(String),
    /// A brush's faces did not rebuild into a solid.
    Brush(String),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::Json(m) => write!(f, "bad edit: {m}"),
            WireError::Brush(m) => write!(f, "a brush in the edit does not rebuild: {m}"),
        }
    }
}

impl std::error::Error for WireError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireLayer {
    id: Option<String>,
    name: String,
    sort_index: i32,
    hidden: bool,
    locked: bool,
    omit_from_export: bool,
    color: Option<String>,
    children: Vec<WireNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGroup {
    id: String,
    name: String,
    linked_group_id: Option<String>,
    transformation: Option<String>,
    children: Vec<WireNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireEntity {
    properties: Vec<(String, String)>,
    children: Vec<WireNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireKind {
    Layer(WireLayer),
    Group(WireGroup),
    Entity(WireEntity),
    Brush { faces: Vec<BrushFace> },
    Patch(Patch),
}

/// A [`Node`] as it travels: its id, and its contents with brushes reduced to faces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireNode {
    id: u64,
    kind: WireKind,
}

impl WireNode {
    fn from_node(node: &Node) -> Self {
        let kind = match &node.kind {
            NodeKind::Layer(layer) => WireKind::Layer(WireLayer {
                id: layer.id.clone(),
                name: layer.name.clone(),
                sort_index: layer.sort_index,
                hidden: layer.hidden,
                locked: layer.locked,
                omit_from_export: layer.omit_from_export,
                color: layer.color.clone(),
                children: layer.children.iter().map(WireNode::from_node).collect(),
            }),
            NodeKind::Group(group) => WireKind::Group(WireGroup {
                id: group.id.clone(),
                name: group.name.clone(),
                linked_group_id: group.linked_group_id.clone(),
                transformation: group.transformation.clone(),
                children: group.children.iter().map(WireNode::from_node).collect(),
            }),
            NodeKind::Entity(entity) => WireKind::Entity(WireEntity {
                properties: entity.properties.clone(),
                children: entity.children.iter().map(WireNode::from_node).collect(),
            }),
            NodeKind::Brush(brush) => WireKind::Brush {
                faces: brush.faces().to_vec(),
            },
            NodeKind::Patch(patch) => WireKind::Patch(patch.clone()),
        };
        WireNode {
            id: node.id.0,
            kind,
        }
    }

    fn into_node(self, bounds: &Bbox3d) -> Result<Node, WireError> {
        let kind = match self.kind {
            WireKind::Layer(layer) => NodeKind::Layer(Layer {
                id: layer.id,
                name: layer.name,
                sort_index: layer.sort_index,
                hidden: layer.hidden,
                locked: layer.locked,
                omit_from_export: layer.omit_from_export,
                color: layer.color,
                children: into_nodes(layer.children, bounds)?,
            }),
            WireKind::Group(group) => NodeKind::Group(Group {
                id: group.id,
                name: group.name,
                linked_group_id: group.linked_group_id,
                transformation: group.transformation,
                children: into_nodes(group.children, bounds)?,
            }),
            WireKind::Entity(entity) => NodeKind::Entity(EntityNode {
                properties: entity.properties,
                children: into_nodes(entity.children, bounds)?,
            }),
            WireKind::Brush { faces } => NodeKind::Brush(
                Brush::from_faces(bounds, faces).map_err(|e| WireError::Brush(format!("{e:?}")))?,
            ),
            WireKind::Patch(patch) => NodeKind::Patch(patch),
        };
        Ok(Node {
            id: NodeId(self.id),
            kind,
        })
    }
}

fn into_nodes(wire: Vec<WireNode>, bounds: &Bbox3d) -> Result<Vec<Node>, WireError> {
    wire.into_iter().map(|n| n.into_node(bounds)).collect()
}

fn from_nodes(nodes: &[Node]) -> Vec<WireNode> {
    nodes.iter().map(WireNode::from_node).collect()
}

fn wire_layer(layer: &Layer) -> WireLayer {
    WireLayer {
        id: layer.id.clone(),
        name: layer.name.clone(),
        sort_index: layer.sort_index,
        hidden: layer.hidden,
        locked: layer.locked,
        omit_from_export: layer.omit_from_export,
        color: layer.color.clone(),
        children: from_nodes(&layer.children),
    }
}

fn unwire_layer(layer: WireLayer, bounds: &Bbox3d) -> Result<Layer, WireError> {
    Ok(Layer {
        id: layer.id,
        name: layer.name,
        sort_index: layer.sort_index,
        hidden: layer.hidden,
        locked: layer.locked,
        omit_from_export: layer.omit_from_export,
        color: layer.color,
        children: into_nodes(layer.children, bounds)?,
    })
}

/// A whole [`World`](map_model::node::World) on the wire, node ids and all. The host sends one
/// of these when a guest joins, so the guest adopts the host's ids rather than minting
/// its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireWorld {
    properties: Vec<(String, String)>,
    layers: Vec<WireLayer>,
}

impl WireWorld {
    /// The wire form of `world`.
    pub fn from_world(world: &map_model::node::World) -> Self {
        WireWorld {
            properties: world.properties.clone(),
            layers: world.layers.iter().map(wire_layer).collect(),
        }
    }

    /// Rebuilds the world, every brush against `bounds`.
    pub fn into_world(self, bounds: &Bbox3d) -> Result<map_model::node::World, WireError> {
        Ok(map_model::node::World {
            properties: self.properties,
            layers: self
                .layers
                .into_iter()
                .map(|l| unwire_layer(l, bounds))
                .collect::<Result<_, _>>()?,
        })
    }

    /// This world as one line of JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a WireWorld serializes")
    }

    /// Parses one from a line of JSON.
    pub fn from_json(line: &str) -> Result<Self, WireError> {
        serde_json::from_str(line).map_err(|e| WireError::Json(e.to_string()))
    }
}

/// One thing sent over a live link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireMessage {
    /// The host's whole world, for a joining guest to adopt.
    World(WireWorld),
    /// One edit.
    Edit(WireEdit),
    /// A guest edit with a delivery identity. The host echoes it in order; the
    /// sender keeps it for reconnect until that echo arrives.
    TrackedEdit {
        /// Unique across processes and submissions.
        id: String,
        /// The reversible edit.
        edit: WireEdit,
    },
    /// The sender's current selection, by node id, so the other editor can show where
    /// they are working.
    Selection(Vec<u64>),
    /// The name the other person should show for the sender. Sent once on connect and
    /// again whenever it changes.
    Hello(String),
    /// Where the sender's 3D camera stands and looks, so the other editor can draw
    /// their cursor in space. Sent whenever it has moved enough to notice.
    Camera {
        /// Where the viewer stands, in map units.
        position: [f64; 3],
        /// The way they look, unit length.
        direction: [f64; 3],
    },
    /// A message from one participant, passed on by the host with who it came from.
    ///
    /// With more than two people in a session the host is the hub: everyone talks to
    /// it, and it hands each message on to the rest wrapped in this. `from` 0 is the
    /// host itself; edits also come back to their sender as acknowledgements.
    Relayed {
        /// Who sent the inner message.
        from: u32,
        /// What they sent.
        message: Box<WireMessage>,
    },
    /// A participant left; the host tells everyone else.
    Left {
        /// Who is gone.
        peer: u32,
    },
}

impl WireMessage {
    /// This message as one line of JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a WireMessage serializes")
    }

    /// Parses one from a line of JSON.
    pub fn from_json(line: &str) -> Result<Self, WireError> {
        serde_json::from_str(line).map_err(|e| WireError::Json(e.to_string()))
    }
}

/// An [`Edit`] in serializable form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireEdit {
    /// Nodes replaced with new versions of themselves.
    Changed {
        /// What they were.
        before: Vec<WireNode>,
        /// What they became.
        after: Vec<WireNode>,
    },
    /// Nodes added to a layer.
    Added {
        /// The layer's position in the world's layer list.
        layer: usize,
        /// The nodes.
        nodes: Vec<WireNode>,
    },
    /// Nodes removed from a layer.
    Removed {
        /// The layer's position.
        layer: usize,
        /// The nodes.
        nodes: Vec<WireNode>,
    },
    /// Root order in a layer, using stable node identifiers.
    Reordered {
        layer: usize,
        before: Vec<u64>,
        after: Vec<u64>,
    },
    /// Several edits that are one step.
    Batch(Vec<WireEdit>),
}

impl WireEdit {
    /// The wire form of `edit`.
    pub fn from_edit(edit: &Edit) -> Self {
        match edit {
            Edit::Changed { before, after } => WireEdit::Changed {
                before: from_nodes(before),
                after: from_nodes(after),
            },
            Edit::Added { layer, nodes } => WireEdit::Added {
                layer: *layer,
                nodes: from_nodes(nodes),
            },
            Edit::Removed { layer, nodes } => WireEdit::Removed {
                layer: *layer,
                nodes: from_nodes(nodes),
            },
            Edit::Reordered {
                layer,
                before,
                after,
            } => WireEdit::Reordered {
                layer: *layer,
                before: before.iter().map(|id| id.0).collect(),
                after: after.iter().map(|id| id.0).collect(),
            },
            Edit::Batch(parts) => WireEdit::Batch(parts.iter().map(WireEdit::from_edit).collect()),
        }
    }

    /// The [`Edit`] this stands for, rebuilding every brush against `bounds`.
    pub fn into_edit(self, bounds: &Bbox3d) -> Result<Edit, WireError> {
        Ok(match self {
            WireEdit::Changed { before, after } => Edit::Changed {
                before: into_nodes(before, bounds)?,
                after: into_nodes(after, bounds)?,
            },
            WireEdit::Added { layer, nodes } => Edit::Added {
                layer,
                nodes: into_nodes(nodes, bounds)?,
            },
            WireEdit::Removed { layer, nodes } => Edit::Removed {
                layer,
                nodes: into_nodes(nodes, bounds)?,
            },
            WireEdit::Reordered {
                layer,
                before,
                after,
            } => Edit::Reordered {
                layer,
                before: before.into_iter().map(NodeId).collect(),
                after: after.into_iter().map(NodeId).collect(),
            },
            WireEdit::Batch(parts) => Edit::Batch(
                parts
                    .into_iter()
                    .map(|p| p.into_edit(bounds))
                    .collect::<Result<_, _>>()?,
            ),
        })
    }

    /// This edit as a single line of JSON, ready to send.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a WireEdit serializes")
    }

    /// Parses one from a line of JSON.
    pub fn from_json(line: &str) -> Result<Self, WireError> {
        serde_json::from_str(line).map_err(|e| WireError::Json(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_model::io::map_reader::read_map;
    use map_model::node::{build_tree, World};

    fn bounds() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    const A_BOX: &str = "\
{
\"classname\" \"worldspawn\"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1
}
}
";

    fn a_world() -> World {
        build_tree(&read_map(A_BOX, None).unwrap(), &bounds()).world
    }

    fn a_brush_node() -> Node {
        let world = a_world();
        map_model::node::brushes(&world)
            .first()
            .map(|b| Node {
                id: NodeId(42),
                kind: NodeKind::Brush((*b).clone()),
            })
            .unwrap()
    }

    #[test]
    fn a_brush_node_round_trips_through_the_wire() {
        let node = a_brush_node();
        let wire = WireNode::from_node(&node);
        let back = wire.into_node(&bounds()).unwrap();

        assert_eq!(back.id, node.id);
        let (NodeKind::Brush(a), NodeKind::Brush(b)) = (&node.kind, &back.kind) else {
            panic!("expected brushes");
        };
        assert_eq!(a.faces(), b.faces());
    }

    #[test]
    fn an_entity_with_a_brush_round_trips() {
        let entity = Node {
            id: NodeId(7),
            kind: NodeKind::Entity(EntityNode {
                properties: vec![
                    ("classname".into(), "func_detail".into()),
                    ("_phong".into(), "1".into()),
                ],
                children: vec![a_brush_node()],
            }),
        };
        let back = WireNode::from_node(&entity).into_node(&bounds()).unwrap();
        assert_eq!(back.id, entity.id);
        let NodeKind::Entity(e) = &back.kind else {
            panic!()
        };
        assert_eq!(e.properties.len(), 2);
        assert_eq!(e.children.len(), 1);
        assert_eq!(e.children[0].id, NodeId(42));
    }

    #[test]
    fn an_edit_round_trips_through_json() {
        let edit = Edit::Batch(vec![
            Edit::Added {
                layer: 0,
                nodes: vec![a_brush_node()],
            },
            Edit::Changed {
                before: vec![a_brush_node()],
                after: vec![a_brush_node()],
            },
            Edit::Removed {
                layer: 2,
                nodes: vec![a_brush_node()],
            },
        ]);

        let json = WireEdit::from_edit(&edit).to_json();
        assert!(!json.contains('\n'), "one line: {json}");

        let back = WireEdit::from_json(&json)
            .unwrap()
            .into_edit(&bounds())
            .unwrap();
        let Edit::Batch(parts) = back else {
            panic!("expected a batch")
        };
        assert!(matches!(parts[0], Edit::Added { layer: 0, .. }));
        assert!(matches!(parts[1], Edit::Changed { .. }));
        assert!(matches!(parts[2], Edit::Removed { layer: 2, .. }));
    }

    #[test]
    fn a_broken_brush_is_an_error_not_a_panic() {
        let bad = WireEdit::Added {
            layer: 0,
            nodes: vec![WireNode {
                id: 1,
                kind: WireKind::Brush { faces: vec![] },
            }],
        };
        let json = bad.to_json();
        assert!(matches!(
            WireEdit::from_json(&json).unwrap().into_edit(&bounds()),
            Err(WireError::Brush(_))
        ));
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(matches!(
            WireEdit::from_json("not json"),
            Err(WireError::Json(_))
        ));
    }
}
