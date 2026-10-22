//! The document the editor has open.
//!
//! Owns map data, selection and change tracking. Child modules implement editing
//! operations against the same private state and history. Views consume derived geometry.

mod clipboard;
mod corner_join;
pub use corner_join::CornerDraft;
mod animation_wireframe;
mod diagnostics;
mod entities;
mod export_snapshot;
mod geometry;
mod imported_models;
mod omit_faces;
pub use export_snapshot::ExportSnapshot;
mod groups;
mod history;
mod layers;
mod links;
pub use links::{EntityGraph, GraphEdge, GraphNode};
mod array;
mod light_marker;
mod materials;
mod scene_lights;
mod sculpt;
mod selection;
mod shape_preview;
pub use array::{array_transforms, ArrayKind, ArrayParameters, MAX_COPIES as MAX_ARRAY_COPIES};
pub use sculpt::{SculptStroke, MAX_COLUMNS};
mod surfaces;
pub use surfaces::{NodeSurface, SurfaceCache};
mod tags;
mod transforms;

#[cfg(test)]
mod tests;

use std::cell::{Cell, Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use geometry_math::bbox::Bbox3d;
use geometry_math::mat::Mat4x4d;
use geometry_math::mat_ext;
use geometry_math::plane::Plane3d;
use geometry_math::ray::Ray3d;
use geometry_math::scalar::{self, Float};
use geometry_math::util::Axis;
use geometry_math::vec::{self, Vec2f, Vec3d};
use map_model::brush::Brush;
use map_model::csg;
use map_model::io::map_reader::{read_map, MapFile};
use map_model::io::map_writer::write_map;
use map_model::map_format::MapFormat;
use map_model::material::MaterialCollection;
use map_model::node::{self, remove_nodes, EntityNode, Group, Node, NodeId, NodeKind, Tree};
use map_model::pick;
use map_model::polyhedron::Polyhedron;

use crate::history::{Edit, History};

/// How closely a face's vertices must match for it to be the same face on another brush.
///
/// A brush is defined by planes, so its corners land near round numbers rather than on
/// them, and two brushes meeting at a face have corners that agree only to within the
/// error each accumulated.
const EXTRUDE_EPSILON: f64 = 0.001;

/// Marks clipboard text that carries a face's attributes rather than geometry.
const FACE_CLIPBOARD_MARKER: &str = "// tb-face-attributes";

use crate::world::WorldBounds;

/// Selection at pointer-down, independent of the changing marquee preview.
#[derive(Clone, PartialEq)]
pub struct SelectionSnapshot {
    nodes: HashSet<NodeId>,
    faces: HashSet<FaceRef>,
}

/// Consecutive live paint changes share one undo entry until another edit intervenes.
#[derive(Default)]
pub struct LivePaint {
    state: Option<(
        u64,
        usize,
        SelectionSnapshot,
        Vec<FaceRef>,
        web_time::Instant,
    )>,
}

/// One held-button material stroke, with a fixed material and one Undo step.
pub struct MaterialStroke {
    material: String,
    visited: HashSet<FaceRef>,
    mark: usize,
    revision: u64,
}

/// An edit that Repeat plays again.
///
/// The supported document effects, represented as data.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Repeatable {
    /// A move, a turn, a scale or a shear -- everything `transformSelection` covers.
    Transform(Mat4x4d),
    /// A duplicate, offset by where it was put.
    Duplicate(Vec3d),
}

/// Which of a class's default values a "set defaults" writes.
///
/// Controls whether default properties replace existing values or fill missing ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultProperties {
    /// Only the keys the entity already carries, put back to what the class says.
    Existing,
    /// Only the keys it has not got, added with what the class says.
    Missing,
}

/// What turning a tag on did, for the panel to report.
///
/// `SmartTag::enable`'s outcomes: it may find nothing to do, or several materials where it
/// wants one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagOutcome {
    /// This many faces changed.
    Changed(usize),
    /// The tag matches no material in the loaded set, so there is nothing to put on.
    NoMaterial,
    /// Several materials match the tag; the first was used. Its name, and the face count.
    Ambiguous {
        /// The material that was applied.
        used: String,
        /// How many faces changed.
        changed: usize,
    },
    /// The tag cannot be turned on this way -- it is not a material tag.
    Unsupported,
    /// Nothing is selected to act on.
    NoSelection,
}

/// Which of a face's two flag fields a flag tag acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagFlagField {
    Surface,
    Content,
}

/// Where a document's materials came from.
///
/// Removing a source reloads the remaining material collections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialSource {
    /// A directory of images.
    Folder(PathBuf),
    /// A `.wad`, which is how the Quake lineage ships its textures.
    Package(PathBuf),
}

impl MaterialSource {
    /// Where it is on disk.
    pub fn path(&self) -> &Path {
        match self {
            MaterialSource::Folder(path) | MaterialSource::Package(path) => path,
        }
    }

    /// What to call it in a list, which is the folder's or the file's own name.
    pub fn name(&self) -> String {
        self.path()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path().display().to_string())
    }
}

/// A map open in the editor.
/// The callback a live editing session installs to send each local edit out.
type EditListener = Rc<dyn Fn(&Edit)>;

/// The view geometry the flat views draw, walked out of the tree and kept until an edit
/// makes it wrong. See [`Document::edge_cache`].
#[derive(Default)]
struct EdgeCache {
    /// Brushes that are neither selected, locked, nor another participant's.
    plain: Vec<(Vec3d, Vec3d)>,
    /// The brushes that are selected here.
    selected: Vec<(Vec3d, Vec3d)>,
    /// What each other participant in a live session has selected, so their work area is
    /// visible. Empty when there is no session.
    peer: Vec<(u32, Vec3d, Vec3d)>,
    /// Brushes on a locked layer, drawn in their own colour so that what cannot be
    /// changed is visibly so.
    locked: Vec<(Vec3d, Vec3d)>,
    /// Bumped whenever `plain`, `locked` or `peer` change, so a flat view can keep the
    /// picture it drew of them until they do -- an edit of the selection alone (a drag)
    /// leaves them as they were.
    static_generation: u64,
    /// The selection the cache was last filled for in full, which is what makes refilling
    /// only `selected` safe.
    selection: HashSet<NodeId>,
}

/// The class of a render camera entity: an editing helper the 3D view places, never
/// exported and never drawn as a box.
pub const RENDER_CAMERA_CLASS: &str = "_tb_render_camera";

/// A run of preview triangles and what made it: the node, and for a brush the face.
pub type PreviewOwner = (std::ops::Range<usize>, NodeId, Option<usize>);

pub struct Document {
    /// Where the document was loaded from, if it came from a file.
    pub path: Option<PathBuf>,
    /// The map format the document uses.
    pub format: MapFormat,
    /// The game the document names, if any.
    pub game: Option<String>,
    /// The node tree.
    pub tree: Tree,
    /// The edges of every brush, in world space, ready for the flat views to draw.
    ///
    /// Cached rather than walked per frame: a view redraws on every pointer move, and the
    /// tree does not change between those redraws. Filled lazily -- an edit marks it
    /// stale ([`Document::mark_changed`]) and the next reader pays for the walk, so a
    /// drag that emits an edit per pointer move, or a plugin that touches every face,
    /// rebuilds once per frame rather than once per edit.
    edge_cache: RefCell<EdgeCache>,
    animation_wireframe: RefCell<animation_wireframe::Cache>,
    pub trigger_preview: crate::animation_trigger::Preview,
    animation_path: RefCell<crate::animation_path::Cache>,
    /// The 3D view's surfaces, kept per node; an edit rebuilds only the nodes it reached.
    /// See [`SurfaceCache`].
    surface_cache: RefCell<SurfaceCache>,
    /// Whether [`Document::edge_cache`] has to be walked again before it is read.
    edges_stale: Cell<bool>,
    /// Whether everything that went stale since the last walk was the geometry of selected
    /// brushes, so only their edges need walking again.
    edges_stale_selected_only: Cell<bool>,
    /// The selection `last_selection_bounds` was taken from, so an edit that left the
    /// selection alone does not pay for the walk again.
    bounds_remembered_for: HashSet<NodeId>,
    /// The way out of the level the last leak check found, if it found one.
    ///
    /// Kept rather than recomputed, since the check is a flood fill over the whole level
    /// and is asked for rather than run on every redraw.
    leak_trace: Option<map_model::leak::Trace>,
    /// The point file the trace was read from, if it came from one.
    ///
    /// `Map::canReloadPointFile`: a compiler is run again while the editor is open, and
    /// the trace is read again from the file it came from.
    point_file: Option<PathBuf>,
    /// The tags the game configuration gives brushes and faces.
    ///
    /// `map_model::Tag`: a name for everything that matches a rule -- a trigger, a clip face --
    /// so that they can be hidden together. See [`crate::games::Tag`].
    brush_tags: Vec<crate::games::Tag>,
    /// The same for faces.
    face_tags: Vec<crate::games::Tag>,
    /// The tags whose brushes and faces are not drawn, by name.
    hidden_tags: HashSet<String>,
    /// `faceattribs.surfaceflags` / `contentflags` names from the game configuration, in
    /// bit order. What the face attribute editor labels its ticks with.
    surface_flag_names: Vec<String>,
    content_flag_names: Vec<String>,
    /// Where a class's model is looked for, and what has been loaded from there.
    ///
    /// `EntityModelManager`. See [`crate::entity_models`].
    model_store: crate::entity_models::ModelStore,
    game_asset_key: Option<(String, String, String)>,
    game_materials: Vec<map_model::material::Material>,
    game_asset_warnings: Vec<String>,
    /// The model each class is drawn as, by classname, once it has been loaded.
    ///
    /// A class whose definition names no model, or whose model is not on this machine, is
    /// not in here and is drawn as its box.
    models: HashMap<String, std::rc::Rc<map_model::entity_model::EntityModel>>,
    /// The sprite each class is drawn as -- classname to `(material name, width, height)`.
    /// A class with an `iconsprite(...)` whose `.spr` was found; drawn as a camera-facing
    /// quad rather than a box.
    sprites: HashMap<String, (String, u32, u32)>,
    /// The edits Repeat plays again, oldest first.
    ///
    /// `RepeatStack`. See [`Document::repeat`].
    repeat_stack: Vec<Repeatable>,
    /// What was selected when the last repeatable edit was recorded.
    ///
    /// Compares the current selection with the selection recorded for the previous operation. A
    /// changed selection starts a new operation run.
    repeat_selection: HashSet<NodeId>,
    /// Whether a repeat is under way, so that what it does is not recorded again.
    repeating: bool,
    /// The portals a compiler wrote, once a `.prt` has been read.
    ///
    /// `Map::portalFile`: the boundaries `vis` cut the world into, drawn over the map.
    portals: Option<map_model::portal::Portals>,
    /// The file those came from, so they can be read again.
    portal_file: Option<PathBuf>,
    /// Which point of the trace the camera was last sent to.
    ///
    /// The current position along the leak trace from an entity to the void.
    trace_point: usize,
    /// Which nodes are selected.
    ///
    /// A document selects either whole brushes or individual faces, never both: the
    /// operations that apply to one do not apply to the other, and showing both selected
    /// at once would leave it ambiguous which an edit would act on.
    selection: HashSet<NodeId>,
    /// Which faces are selected, as a node and an index into its geometry faces.
    face_selection: HashSet<FaceRef>,
    /// The face tool keeps its source brush selected; property editors target its handles.
    face_edit_targets: Vec<FaceRef>,
    /// Objects hidden by "Hide" or "Isolate". Not written to the map -- a view state, the
    /// way the leak trace is -- and cleared by "Show All".
    hidden_nodes: HashSet<NodeId>,
    /// What the other person in a live session has selected, so their work area shows in
    /// the views. Empty when there is no session.
    peer_selection: std::collections::HashMap<u32, HashSet<NodeId>>,
    /// Where each other participant's 3D camera stands and looks, when a live session
    /// has told us: their cursors in space, drawn by the views.
    peer_cameras: std::collections::HashMap<u32, (Vec3d, Vec3d)>,
    peer_names: std::collections::HashMap<u32, String>,
    /// The world bounds every geometry rebuild is clipped against.
    world_bounds: Bbox3d,
    /// Where the materials came from, in the order they were loaded.
    ///
    /// Retaining source paths allows individual material collections to be removed and the
    /// remainder reloaded.
    material_sources: Vec<MaterialSource>,
    material_defaults: HashSet<PathBuf>,
    /// Each folder's files as they were when it was last read (a hash of their names,
    /// sizes and times) and the materials it gave, so a rebuild moves an unchanged
    /// folder's materials across rather than decoding every image again.
    material_folders: HashMap<PathBuf, (u64, Vec<String>)>,
    /// The entity definitions in use, if an FGD has been loaded.
    definitions: Option<map_model::fgd::EntityDefinitions>,
    /// The file those definitions were read from.
    ///
    /// Kept so the export can hand the same file to the builder, which reads it with its
    /// own reader: what an entity becomes in an engine is what its class says it is.
    definitions_path: Option<PathBuf>,
    pub(crate) export_classes_override: Option<(
        scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>,
        Vec<String>,
    )>,
    /// What was selected before the selection was last emptied.
    ///
    /// New geometry is sized against this when nothing is selected, so drawing a run of
    /// brushes keeps them the same depth.
    last_selection_bounds: Option<Bbox3d>,
    /// The materials a face may wear.
    materials: MaterialCollection,
    /// Which material a newly drawn brush gets, and which one the browser has selected.
    current_material: String,
    /// Which layer a newly drawn brush goes onto.
    current_layer: usize,
    /// Which group is open, so what is inside it can be edited on its own.
    open_group: Option<NodeId>,
    /// Bumped whenever the collection is replaced, so the renderer knows to re-upload.
    materials_revision: u64,
    /// The brushes as they were when the open transaction began.
    ///
    /// A drag rolls back and reapplies changes from its original state, then commits the entire
    /// gesture as one undo step.
    transaction: Option<Vec<Node>>,
    shape_preview: Option<shape_preview::ShapePreview>,
    array_preview: Option<array::ArrayPreview>,
    /// Called whenever anything the views draw has changed.
    ///
    /// The document announces its own changes rather than every edit remembering to ask
    /// for a redraw. Forgetting one of those is invisible until something fails to
    /// update, and there is no reason for a call site to know which views exist.
    on_changed: Option<Rc<dyn Fn()>>,
    /// Called with every edit as it goes on the undo stack, so a live editing session can
    /// send it to the other person. See [`crate::collab`].
    on_edit: Option<EditListener>,
    /// Bumped whenever the geometry or the selection changes.
    revision: u64,
    selection_change_count: u64,
    /// Whether the document has changed since it was opened or last saved.
    modified: bool,
    /// Counts edits made to the document.
    ///
    /// Unlike [`Document::revision`] this does not move when only the selection changes,
    /// so the autosaver can tell an edit from a click and skip a backup that would be
    /// identical to the last.
    modification_count: u64,
    /// What can be undone and redone.
    history: History,
    /// Counts how many documents have been loaded into this editor session.
    ///
    /// A view uses this to notice that the document it is showing has been replaced, so
    /// it can point itself at the new one. Comparing the documents themselves would not
    /// do: two different maps can be identical.
    generation: u64,
    /// Session-scoped `_tb_id` generator, so a group or layer made here does not collide
    /// with one made in someone else's session on the same map. See [`map_model::ids`].
    ids: map_model::ids::IdSource,
}

/// What [`node::contents_of`] lists for a node with these children, summed up in one walk
/// of them: the bounds of the brushes among them, and whether every member (and every
/// brush) is selected.
///
/// The views ask this of every group and brush entity on every frame. Asking
/// `contents_of` and then [`Document::brush`] for each member walked the whole tree once
/// per member, which on a big map was most of a flat view's frame.
struct Contents {
    bounds: Option<Bbox3d>,
    /// Members as `contents_of` counts them: brushes, patches and entities.
    members: usize,
    all_selected: bool,
    brushes: usize,
    brushes_selected: bool,
}

/// Whether the entity `id`, with `children`, counts as selected for its links: itself, or
/// any brush or patch inside it. Selecting a brush entity in a view selects its brushes,
/// and its links should show then too.
fn entity_touched(id: NodeId, children: &[Node], selection: &HashSet<NodeId>) -> bool {
    fn any(nodes: &[Node], selection: &HashSet<NodeId>) -> bool {
        nodes.iter().any(|node| {
            selection.contains(&node.id)
                || match &node.kind {
                    NodeKind::Group(group) => any(&group.children, selection),
                    NodeKind::Entity(entity) => any(&entity.children, selection),
                    NodeKind::Layer(layer) => any(&layer.children, selection),
                    NodeKind::Brush(_) | NodeKind::Patch(_) => false,
                }
        })
    }
    selection.contains(&id) || any(children, selection)
}

fn fold_contents(children: &[Node], selection: &HashSet<NodeId>) -> Contents {
    fn walk(nodes: &[Node], selection: &HashSet<NodeId>, out: &mut Contents) {
        for node in nodes {
            match &node.kind {
                NodeKind::Brush(brush) => {
                    let selected = selection.contains(&node.id);
                    out.members += 1;
                    out.all_selected &= selected;
                    out.brushes += 1;
                    out.brushes_selected &= selected;
                    out.bounds = Some(match out.bounds {
                        Some(bounds) => bounds.merged(&brush.bounds()),
                        None => brush.bounds(),
                    });
                }
                NodeKind::Patch(_) => {
                    out.members += 1;
                    out.all_selected &= selection.contains(&node.id);
                }
                NodeKind::Entity(entity) => {
                    out.members += 1;
                    out.all_selected &= selection.contains(&node.id);
                    walk(&entity.children, selection, out);
                }
                NodeKind::Group(group) => walk(&group.children, selection, out),
                NodeKind::Layer(layer) => walk(&layer.children, selection, out),
            }
        }
    }
    let mut out = Contents {
        bounds: None,
        members: 0,
        all_selected: true,
        brushes: 0,
        brushes_selected: true,
    };
    walk(children, selection, &mut out);
    out
}

/// The next value for [`Document::generation`].
///
/// A plain counter rather than an id per document: the views only ever ask whether this
/// is still the document they last framed.
fn next_generation() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// How large a point entity is drawn when its class has no definition.
pub const DEFAULT_ENTITY_RADIUS: f64 = 8.0;

/// The outline drawn around a group's contents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupBounds {
    /// The box the group's contents fill.
    pub bounds: Bbox3d,
    /// Whether everything in the group is selected.
    pub selected: bool,
    /// Whether the group is one of a linked set.
    pub linked: bool,
}

/// What colour a point entity is drawn in when its class has no definition.
fn default_entity_color() -> [u8; 3] {
    let colour = crate::preferences::pref(|p| p.undefined_entity_color).rgb_f32();
    colour.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The colour a surface is drawn in under its material.
///
/// Textured surfaces use white so their material colours remain unchanged. Untextured surfaces use
/// `Preferences::FaceColor`.
fn surface_color(material: Option<usize>) -> [f32; 3] {
    match material {
        Some(_) => [1.0, 1.0, 1.0],
        None => crate::preferences::pref(|p| p.face_color).rgb_f32(),
    }
}

/// The material name before a class's name for the editor's own icon, drawn as a sprite
/// where the game gives the class no model.
const BUILTIN_SPRITE_PREFIX: &str = "#model/builtin/";

/// How opaque a point entity's box is in the 3D view: see-through, as the flat views
/// draw only its outline, so what it stands in or on still shows.
const ENTITY_BOX_ALPHA: f32 = 0.35;

/// The six faces of a box, as triangles wound clockwise seen from outside.
fn box_triangles(
    bounds: &Bbox3d,
    color: [f32; 3],
    alpha: f32,
    selected: bool,
    locked: bool,
) -> Vec<Triangle> {
    let (min, max) = (bounds.min, bounds.max);
    let corner = |x: bool, y: bool, z: bool| {
        Vec3d::new(
            if x { max.x() } else { min.x() },
            if y { max.y() } else { min.y() },
            if z { max.z() } else { min.z() },
        )
    };

    // Each face as its four corners in order, and the way it faces.
    let faces = [
        // -X, +X
        (
            [
                corner(false, false, false),
                corner(false, false, true),
                corner(false, true, true),
                corner(false, true, false),
            ],
            Vec3d::new(-1.0, 0.0, 0.0),
        ),
        (
            [
                corner(true, false, false),
                corner(true, true, false),
                corner(true, true, true),
                corner(true, false, true),
            ],
            Vec3d::new(1.0, 0.0, 0.0),
        ),
        // -Y, +Y
        (
            [
                corner(false, false, false),
                corner(true, false, false),
                corner(true, false, true),
                corner(false, false, true),
            ],
            Vec3d::new(0.0, -1.0, 0.0),
        ),
        (
            [
                corner(false, true, false),
                corner(false, true, true),
                corner(true, true, true),
                corner(true, true, false),
            ],
            Vec3d::new(0.0, 1.0, 0.0),
        ),
        // -Z, +Z
        (
            [
                corner(false, false, false),
                corner(false, true, false),
                corner(true, true, false),
                corner(true, false, false),
            ],
            Vec3d::new(0.0, 0.0, -1.0),
        ),
        (
            [
                corner(false, false, true),
                corner(true, false, true),
                corner(true, true, true),
                corner(false, true, true),
            ],
            Vec3d::new(0.0, 0.0, 1.0),
        ),
    ];

    let mut triangles = Vec::with_capacity(12);
    for (polygon, normal) in faces {
        for i in 1..3 {
            triangles.push(Triangle {
                corner_normals: None,
                surface_material: None,
                positions: [polygon[0], polygon[i], polygon[i + 1]],
                normal,
                // No material, so no coordinates to give it.
                uv: [geometry_math::vec::Vec2f::zero(); 3],
                material: None,
                color,
                alpha,
                corner_colors: None,
                locked,
                selected,
                painted: false,
            });
        }
    }
    triangles
}

/// Whether a face's flags carry the bit a tag names.
///
/// A face with no surface attributes -- which is every face of a Quake map, where the
/// format has no room for them -- carries no flags, and so no flag tag.
fn flags_match(tag: &crate::games::Tag, face: &map_model::brush_face::BrushFace) -> bool {
    let Some(attributes) = &face.surface_attributes else {
        return false;
    };
    match tag.rule {
        crate::games::TagRule::ContentFlag => tag.matches_flags(attributes.content_flags),
        crate::games::TagRule::SurfaceFlag => tag.matches_flags(attributes.surface_flags),
        _ => false,
    }
}

/// A model's triangles, put where the entity is and turned the way it faces.
///
/// `EntityModelRenderer`: the model is drawn at the entity's origin, turned by its `angle`
/// property, wearing its material skins. Supplied vertex normals control shading;
/// models without them use geometric face normals.
fn model_triangles(
    model: &map_model::entity_model::EntityModel,
    materials: Vec<Option<usize>>,
    placement: (Vec3d, f64),
    scale: Vec3d,
    selected: bool,
    locked: bool,
) -> Vec<Triangle> {
    let (origin, angle) = placement;
    let (sin, cos) = geometry_math::scalar::to_radians(angle).sin_cos();
    // The model matrix is translation * rotation * scale: scale in model space, rotate, then place
    // at the entity origin.
    let place = |point: Vec3d| {
        let point = Vec3d::new(
            point.x() * scale.x(),
            point.y() * scale.y(),
            point.z() * scale.z(),
        );
        Vec3d::new(
            point.x() * cos - point.y() * sin,
            point.x() * sin + point.y() * cos,
            point.z(),
        ) + origin
    };

    model
        .triangles
        .iter()
        .filter_map(|triangle| {
            let positions = [
                place(triangle.positions[0]),
                place(triangle.positions[1]),
                place(triangle.positions[2]),
            ];
            let normal =
                geometry_math::vec::cross(positions[1] - positions[0], positions[2] - positions[0]);
            // A triangle with no area has no normal and nothing to draw.
            if geometry_math::vec::length(normal) < 1e-9 {
                return None;
            }
            Some(Triangle {
                corner_normals: triangle.normals.map(|normals| {
                    normals.map(|n| {
                        let n = Vec3d::new(n.x() / scale.x(), n.y() / scale.y(), n.z() / scale.z());
                        geometry_math::vec::normalize(Vec3d::new(
                            n.x() * cos - n.y() * sin,
                            n.x() * sin + n.y() * cos,
                            n.z(),
                        ))
                    })
                }),
                surface_material: None,
                positions,
                normal: geometry_math::vec::normalize(normal),
                uv: triangle.uv,
                material: materials[triangle.material],
                // A model wears its skin, so the colour under it is plain white rather
                // than the class's -- which would tint the skin.
                color: [1.0, 1.0, 1.0],
                alpha: 1.0,
                corner_colors: None,
                locked,
                selected,
                painted: false,
            })
        })
        .collect()
}

/// Wraps a brush in a node with the given id, for building an edit.
/// Where a node's brush has its corners, for telling one snapshot from another.
///
/// A drag is over when the geometry has moved; comparing the vertices is enough to see
/// that, and is what the views draw anyway.
fn brush_vertices(node: &Node) -> Vec<Vec3d> {
    match &node.kind {
        NodeKind::Brush(brush) => brush.vertex_positions(),
        _ => Vec::new(),
    }
}

fn brush_node(id: NodeId, brush: Brush) -> Node {
    Node {
        id,
        kind: NodeKind::Brush(brush),
    }
}

/// Puts one face's material, and by the mode its mapping, onto another face.
///
/// The body `transferFaceAttributes` shares between its single-face and its many-face
/// forms: the material always, and the mapping turned to the target's normal when the
/// mode is not material-only.
fn apply_face_attributes(
    face: &mut map_model::brush_face::BrushFace,
    source: &map_model::brush_face::BrushFace,
    mode: FaceAttributeTransfer,
) {
    let target_normal = face.plane.normal;
    face.material_name = source.material_name.clone();
    // The source's paint goes across with its material, a gradient placed on the face too
    // (its ends are fractions of the face, so they fit any other); a source without paint
    // reveals the material instead of leaving opaque paint over it.
    face.set_paint(source.paint());
    face.surface_material = source.surface_material;

    if mode != FaceAttributeTransfer::MaterialOnly {
        let style = match mode {
            FaceAttributeTransfer::Rotation => map_model::uv::WrapStyle::Rotation,
            _ => map_model::uv::WrapStyle::Projection,
        };
        face.uv_attributes = source.uv_attributes;
        face.uv_coord_system =
            source
                .uv_coord_system
                .updated_for_normal(source.plane.normal, target_normal, style);
    }
}

/// Gathers everything a paste should bring across, flattening groups away.
///
/// A group is a thing the pasting document does not have, so what comes across is what
/// was in it.
fn collect_for_paste(nodes: &[Node], out: &mut Vec<Node>) {
    for node in nodes {
        match &node.kind {
            NodeKind::Brush(_) | NodeKind::Patch(_) | NodeKind::Entity(_) => {
                out.push(node.clone());
            }
            NodeKind::Group(group) => collect_for_paste(&group.children, out),
            NodeKind::Layer(layer) => collect_for_paste(&layer.children, out),
        }
    }
}

/// The box a set of nodes about to be pasted fills, so they can be placed by their middle.
fn paste_bounds(nodes: &[Node]) -> Option<Bbox3d> {
    fn visit(nodes: &[Node], points: &mut Vec<Vec3d>) {
        for node in nodes {
            match &node.kind {
                NodeKind::Brush(brush) => {
                    let b = brush.bounds();
                    points.push(b.min);
                    points.push(b.max);
                }
                NodeKind::Patch(patch) => {
                    points.extend(patch.control_points.iter().map(|p| p.position));
                }
                NodeKind::Entity(entity) => {
                    if let Some(origin) = entity.origin() {
                        points.push(origin);
                    }
                    visit(&entity.children, points);
                }
                NodeKind::Group(group) => visit(&group.children, points),
                NodeKind::Layer(layer) => visit(&layer.children, points),
            }
        }
    }
    let mut points = Vec::new();
    visit(nodes, &mut points);
    Bbox3d::merge_all(points)
}

/// Moves a node about to be pasted, and everything under it, by a delta.
fn translate_paste(node: &mut Node, delta: Vec3d, world_bounds: &Bbox3d) {
    match &mut node.kind {
        NodeKind::Brush(brush) => {
            if let Ok(moved) = brush.translate(world_bounds, delta) {
                *brush = moved;
            }
        }
        NodeKind::Patch(patch) => {
            for point in &mut patch.control_points {
                point.position += delta;
            }
        }
        NodeKind::Entity(entity) => {
            if let Some(origin) = entity.origin() {
                entity.set_origin(origin + delta);
            }
            for child in &mut entity.children {
                translate_paste(child, delta, world_bounds);
            }
        }
        NodeKind::Group(group) => {
            for child in &mut group.children {
                translate_paste(child, delta, world_bounds);
            }
        }
        NodeKind::Layer(layer) => {
            for child in &mut layer.children {
                translate_paste(child, delta, world_bounds);
            }
        }
    }
}

/// Gives a pasted node and everything under it identifiers of its own.
///
/// The ids in the text are the ones the copy was made with, and pasting twice would
/// otherwise put the same identifier in the map twice over.
fn renumber_for_paste(node: Node) -> Node {
    let mut fresh = Node::new(node.kind);
    if let NodeKind::Entity(entity) = &mut fresh.kind {
        let children = std::mem::take(&mut entity.children);
        entity.children = children.into_iter().map(renumber_for_paste).collect();
    }
    fresh
}

/// Gives a node and everything under it fresh identifiers, through every kind of
/// container, for a linked-group copy.
fn deep_renumber(node: Node) -> Node {
    let mut kind = node.kind;
    match &mut kind {
        NodeKind::Entity(entity) => {
            let children = std::mem::take(&mut entity.children);
            entity.children = children.into_iter().map(deep_renumber).collect();
        }
        NodeKind::Group(group) => {
            let children = std::mem::take(&mut group.children);
            group.children = children.into_iter().map(deep_renumber).collect();
        }
        NodeKind::Layer(layer) => {
            let children = std::mem::take(&mut layer.children);
            layer.children = children.into_iter().map(deep_renumber).collect();
        }
        NodeKind::Brush(_) | NodeKind::Patch(_) => {}
    }
    Node::new(kind)
}

/// Restores protected properties from the replaced linked-group member, traversing both child lists
/// together.
///
/// The pairing is positional: a rebuilt group's children are a renumbered copy of the
/// source group's, in the same order the sibling's own children were themselves built in,
/// so the entity at a given position is the same entity in every member of the set.
fn preserve_protected_properties(rebuilt: &mut [Node], corresponding: &[Node]) {
    for (rebuilt_node, corr_node) in rebuilt.iter_mut().zip(corresponding) {
        match (&mut rebuilt_node.kind, &corr_node.kind) {
            (NodeKind::Entity(rebuilt_entity), NodeKind::Entity(corr_entity)) => {
                preserve_entity_protected_properties(rebuilt_entity, corr_entity);
            }
            (NodeKind::Group(rebuilt_group), NodeKind::Group(corr_group)) => {
                preserve_protected_properties(&mut rebuilt_group.children, &corr_group.children);
            }
            _ => {}
        }
    }
}

/// `preserveEntityProperties` for one entity: the union of both sides' protected keys is
/// taken from `corresponding`, and the protected-property list itself becomes its list.
fn preserve_entity_protected_properties(
    rebuilt: &mut map_model::node::EntityNode,
    corresponding: &map_model::node::EntityNode,
) {
    let rebuilt_protected = rebuilt.protected_properties();
    let corresponding_protected = corresponding.protected_properties();
    if rebuilt_protected.is_empty() && corresponding_protected.is_empty() {
        return;
    }

    let mut all_protected = rebuilt_protected;
    all_protected.extend(corresponding_protected.iter().cloned());
    all_protected.sort();
    all_protected.dedup();

    rebuilt.set_protected_properties(&corresponding_protected);
    for key in &all_protected {
        rebuilt.remove_property(key);
        if let Some(value) = corresponding.property(key) {
            rebuilt.set_property(key, value.to_string());
        }
    }
}

/// A 4x4 identity matrix as the sixteen row-major numbers `_tb_transformation` holds.
fn identity_transformation() -> String {
    "1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1".to_string()
}

/// A pure translation as `_tb_transformation`'s sixteen numbers.
fn translation_transformation(delta: Vec3d) -> String {
    format!(
        "1 0 0 {} 0 1 0 {} 0 0 1 {} 0 0 0 1",
        delta.x(),
        delta.y(),
        delta.z()
    )
}

/// Reads `_tb_transformation`'s sixteen row-major numbers back into a matrix.
fn parse_transformation(text: &str) -> Option<Mat4x4d> {
    let n: Vec<f64> = text
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if n.len() != 16 {
        return None;
    }
    Some(Mat4x4d::from_rows([
        [n[0], n[1], n[2], n[3]],
        [n[4], n[5], n[6], n[7]],
        [n[8], n[9], n[10], n[11]],
        [n[12], n[13], n[14], n[15]],
    ]))
}

/// Transforms a node and everything under it by an affine matrix.
fn transform_node(node: &mut Node, matrix: &Mat4x4d, world_bounds: &Bbox3d) {
    match &mut node.kind {
        NodeKind::Brush(brush) => {
            if let Ok(transformed) = brush.transform(world_bounds, matrix) {
                *brush = transformed;
            }
        }
        NodeKind::Patch(patch) => {
            for point in &mut patch.control_points {
                point.position = *matrix * point.position;
            }
        }
        NodeKind::Entity(entity) => {
            if let Some(origin) = entity.origin() {
                entity.set_origin(*matrix * origin);
            }
            for child in &mut entity.children {
                transform_node(child, matrix, world_bounds);
            }
        }
        NodeKind::Group(group) => {
            for child in &mut group.children {
                transform_node(child, matrix, world_bounds);
            }
        }
        NodeKind::Layer(layer) => {
            for child in &mut layer.children {
                transform_node(child, matrix, world_bounds);
            }
        }
    }
}

/// Adds `node`'s id and those of everything under it to `out`.
fn add_ids_under(node: &Node, out: &mut HashSet<NodeId>) {
    out.insert(node.id);
    match &node.kind {
        NodeKind::Group(group) => group.children.iter().for_each(|c| add_ids_under(c, out)),
        NodeKind::Entity(entity) => entity.children.iter().for_each(|c| add_ids_under(c, out)),
        NodeKind::Layer(layer) => layer.children.iter().for_each(|c| add_ids_under(c, out)),
        NodeKind::Brush(_) | NodeKind::Patch(_) => {}
    }
}

/// The ids of `nodes` and of everything under them.
fn ids_under(nodes: &[Node]) -> HashSet<NodeId> {
    let mut out = HashSet::with_capacity(nodes.len());
    nodes.iter().for_each(|n| add_ids_under(n, &mut out));
    out
}

/// Every node id an edit reaches: the nodes it names and everything under them, from
/// both sides of a change so a node that has just left a group still counts.
fn touched_ids(edit: &Edit) -> HashSet<NodeId> {
    fn walk(edit: &Edit, out: &mut HashSet<NodeId>) {
        match edit {
            Edit::Changed { before, after } => {
                before.iter().for_each(|n| add_ids_under(n, out));
                after.iter().for_each(|n| add_ids_under(n, out));
            }
            Edit::Added { nodes, .. } | Edit::Removed { nodes, .. } => {
                nodes.iter().for_each(|n| add_ids_under(n, out));
            }
            Edit::Reordered { .. } => {}
            Edit::Batch(parts) => parts.iter().for_each(|p| walk(p, out)),
        }
    }
    let mut out = HashSet::new();
    walk(edit, &mut out);
    out
}

/// An entity back in the tree, keeping the id it had, so an edit replaces it rather than
/// adding one beside it.
fn entity_node(id: NodeId, entity: map_model::node::EntityNode) -> Node {
    Node {
        id,
        kind: NodeKind::Entity(entity),
    }
}

/// A patch back in the tree, keeping the id it had, so an edit replaces it rather than
/// adding one beside it.
fn patch_node(id: NodeId, patch: map_model::io::map_reader::Patch) -> Node {
    Node {
        id,
        kind: NodeKind::Patch(patch),
    }
}

/// Something a click landed on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picked {
    /// The node that was hit.
    pub node: NodeId,
    /// Which face was hit, for a brush. A point entity has no faces.
    pub face: Option<usize>,
    /// How far along the ray the hit is.
    pub distance: f64,
}

/// Which property keys make a link between two entities.
///
/// `EntityLinkManager`'s two sets; [`Document::link_keys`] builds them from the loaded
/// entity definitions.
#[derive(Debug, Clone, Default)]
pub struct LinkKeys {
    targets: std::collections::HashSet<String>,
    sources: std::collections::HashSet<String>,
}

impl LinkKeys {
    /// Whether a property under this key aims at another entity.
    pub fn is_target(&self, key: &str) -> bool {
        self.targets.contains(&Self::base(key))
    }

    /// Whether a property under this key gives this entity a name others aim at.
    pub fn is_source(&self, key: &str) -> bool {
        self.sources.contains(&Self::base(key))
    }

    /// The key without its trailing number: `target2` links the same way `target` does.
    fn base(key: &str) -> String {
        key.trim_end_matches(|c: char| c.is_ascii_digit())
            .to_string()
    }
}

/// A point entity as the views draw it.
#[derive(Debug, Clone, Copy)]
pub struct EntityBox {
    /// The node holding the entity.
    pub node: NodeId,
    /// The box it occupies, in world space.
    pub bounds: Bbox3d,
    /// The colour to draw it in.
    pub color: [u8; 3],
    /// Whether it is selected.
    pub selected: bool,
}

/// A reference to one face of one brush.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceRef {
    /// The node holding the brush.
    pub node: NodeId,
    /// Which of the brush's geometry faces, by index.
    pub face: usize,
}

/// One triangle of the document's surface, for the 3D view. The struct lives with
/// the renderer that reads it; the document is what fills it in.
pub use render::Triangle;

/// One layer, as the panel shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerSummary {
    /// Where it sits in the list.
    pub index: usize,
    /// What it is called.
    pub name: String,
    /// Whether it is the default layer, which is the worldspawn and cannot be removed.
    pub is_default: bool,
    /// Whether it is drawn.
    pub hidden: bool,
    /// Whether it can be edited.
    pub locked: bool,
    /// Whether it is left out of exports.
    pub omit_from_export: bool,
    /// How many objects are on it.
    pub objects: usize,
}

/// The entity the pointer is over, for the right-click menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityUnder {
    /// The entity node.
    pub id: NodeId,
    /// Its class, for the menu labels.
    pub classname: String,
    /// Whether it holds brushes, so the selection can be moved into it.
    pub is_brush_entity: bool,
}

/// What a face-attribute copy carries across.
///
/// The three the modifiers choose between in `SetBrushFaceAttributesTool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceAttributeTransfer {
    /// The material, and nothing else. Alt+Ctrl.
    MaterialOnly,
    /// The material and the mapping, kept flat. Alt.
    Projection,
    /// The material and the mapping, turned to follow the face. Alt+Shift.
    Rotation,
}

/// What snapping the document's vertices to the grid did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SnapReport {
    /// How many brushes were moved onto the grid.
    pub snapped: usize,
    /// How many were already on it.
    pub already_aligned: usize,
    /// How many were left alone because snapping would have collapsed them.
    pub refused: usize,
}

impl std::fmt::Display for SnapReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "snapped {}, already aligned {}, too small to snap {}",
            self.snapped, self.already_aligned, self.refused
        )
    }
}

/// Why a document could not be saved.
#[derive(Debug)]
pub enum SaveError {
    /// The document has never been saved, so there is no path to write to.
    NoPath,
    /// The file could not be written.
    Io(std::io::Error),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::NoPath => {
                f.write_str("the document has never been saved, so it has no file to save to")
            }
            SaveError::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SaveError::NoPath => None,
            SaveError::Io(error) => Some(error),
        }
    }
}

/// Why a document could not be opened.
#[derive(Debug)]
pub enum OpenError {
    /// The file could not be read from disk.
    Io(std::io::Error),
    /// The file's contents are not a valid map.
    Map(map_model::io::map_reader::ReadError),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Io(error) => write!(f, "{error}"),
            OpenError::Map(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::Io(error) => Some(error),
            OpenError::Map(error) => Some(error),
        }
    }
}

impl Document {
    pub fn animation_path(
        &self,
        pose: &crate::animation_pose::Pose,
    ) -> std::rc::Rc<Option<crate::animation_path::Path>> {
        self.animation_path.borrow_mut().get(self.revision(), pose)
    }

    /// Creates an empty document.
    pub fn new(bounds: WorldBounds) -> Self {
        let map = MapFile::default();
        Self::from_map_owned(map, bounds, None)
    }

    /// Opens a `.map` file.
    pub fn open(path: impl AsRef<Path>, bounds: WorldBounds) -> Result<Self, OpenError> {
        let path = path.as_ref();
        let source = crate::files::read_to_string(path).map_err(OpenError::Io)?;
        let map = read_map(&source, None).map_err(OpenError::Map)?;
        let mut document = Self::from_map_owned(map, bounds, Some(path.to_path_buf()));
        crate::assets::load_library_materials(&mut document);
        if !document.restore_material_paths(&source, path) {
            if let Some(textures) = crate::document_setup::project_material_directory(path) {
                match document.load_materials(&textures) {
                    Ok(warnings) => {
                        for warning in warnings {
                            crate::complain!("materials: {warning}");
                        }
                    }
                    Err(error) => crate::complain!("{}: {error}", textures.display()),
                }
            }
        }
        Ok(document)
    }

    #[cfg(test)]
    fn from_map(map: &MapFile, bounds: WorldBounds, path: Option<PathBuf>) -> Self {
        let world_bounds = Bbox3d::from_scalars(-bounds.half_extent(), bounds.half_extent());
        let tree = node::build_tree(map, &world_bounds);
        Self::from_tree(tree, bounds, map.format, map.game.clone(), path)
    }

    fn from_map_owned(mut map: MapFile, bounds: WorldBounds, path: Option<PathBuf>) -> Self {
        let world_bounds = Bbox3d::from_scalars(-bounds.half_extent(), bounds.half_extent());
        let format = map.format;
        let game = map.game.take();
        let tree = node::build_tree_owned(map, &world_bounds);
        Self::from_tree(tree, bounds, format, game, path)
    }

    /// Builds a document around a tree whose node ids are already assigned. A live guest
    /// that already has a document open uses [`Document::adopt_world`] instead; this is
    /// for opening straight into a session's world.
    #[allow(dead_code)]
    pub fn from_world(
        world: node::World,
        bounds: WorldBounds,
        format: MapFormat,
        game: Option<String>,
    ) -> Self {
        Self::from_tree(
            node::Tree {
                world,
                warnings: Vec::new(),
            },
            bounds,
            format,
            game,
            None,
        )
    }

    fn from_tree(
        tree: node::Tree,
        bounds: WorldBounds,
        format: MapFormat,
        game: Option<String>,
        path: Option<PathBuf>,
    ) -> Self {
        let world_bounds = Bbox3d::from_scalars(-bounds.half_extent(), bounds.half_extent());

        let mut document = Self {
            path,
            format,
            game,
            tree,
            // Nothing selected yet, so the first read walks the tree into the plain set.
            edge_cache: RefCell::new(EdgeCache::default()),
            animation_wireframe: RefCell::new(animation_wireframe::Cache::default()),
            trigger_preview: Default::default(),
            animation_path: RefCell::new(crate::animation_path::Cache::default()),
            surface_cache: RefCell::new(SurfaceCache::default()),
            edges_stale: Cell::new(true),
            edges_stale_selected_only: Cell::new(false),
            bounds_remembered_for: HashSet::new(),
            leak_trace: None,
            point_file: None,
            brush_tags: Vec::new(),
            face_tags: Vec::new(),
            hidden_tags: HashSet::new(),
            surface_flag_names: Vec::new(),
            content_flag_names: Vec::new(),
            model_store: crate::entity_models::ModelStore::default(),
            game_asset_key: None,
            game_materials: Vec::new(),
            game_asset_warnings: Vec::new(),
            models: HashMap::new(),
            sprites: HashMap::new(),
            repeat_stack: Vec::new(),
            repeat_selection: HashSet::new(),
            repeating: false,
            portals: None,
            portal_file: None,
            trace_point: 0,
            selection: HashSet::new(),
            face_selection: HashSet::new(),
            face_edit_targets: Vec::new(),
            hidden_nodes: HashSet::new(),
            peer_selection: std::collections::HashMap::new(),
            peer_cameras: std::collections::HashMap::new(),
            peer_names: std::collections::HashMap::new(),
            material_sources: Vec::new(),
            material_defaults: HashSet::new(),
            material_folders: HashMap::new(),
            definitions: Some(crate::builtin_entities::definitions()),
            definitions_path: None,
            export_classes_override: None,
            last_selection_bounds: None,
            materials: materials::initial_materials(),
            current_material: map_model::shapes::DEFAULT_MATERIAL.to_string(),
            current_layer: 0,
            open_group: None,
            materials_revision: 0,
            transaction: None,
            shape_preview: None,
            array_preview: None,
            on_changed: None,
            on_edit: None,
            revision: 0,
            selection_change_count: 0,
            modified: false,
            modification_count: 0,
            history: History::new(),
            world_bounds,
            generation: next_generation(),
            ids: map_model::ids::IdSource::new(),
        };
        document.load_entity_models();
        document.revision = 0;
        document
    }

    /// A `_tb_id` no group or layer in any session is using. See [`map_model::ids`].
    fn fresh_id(&self) -> String {
        self.ids.fresh()
    }

    /// Which property keys make a link, `EntityLinkManager`'s two sets.
    ///
    /// `target` and `killtarget` aim at a `targetname` -- the Quake default -- and an FGD
    /// may add more by declaring a property `target_destination` (does the aiming) or
    /// `target_source` (is aimed at). A trailing number, as in `target2`, is the same key.
    /// The drawing and the issue browser ask this the same question, so a map cannot show
    /// a link the validator does not know about, or the other way round.
    pub fn link_keys(&self) -> LinkKeys {
        let mut targets: std::collections::HashSet<String> = ["target", "killtarget"]
            .iter()
            .map(|key| key.to_string())
            .collect();
        let mut sources: std::collections::HashSet<String> =
            std::iter::once("targetname".to_string()).collect();
        if let Some(definitions) = self.definitions() {
            for definition in definitions.all() {
                for property in &definition.properties {
                    match property.kind {
                        map_model::fgd::PropertyType::TargetDestination => {
                            targets.insert(property.key.clone());
                        }
                        map_model::fgd::PropertyType::TargetSource => {
                            sources.insert(property.key.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
        LinkKeys { targets, sources }
    }

    /// Every link one entity makes to another.
    ///
    /// A targeting key names a name; the line runs from the one doing the targeting to
    /// the one being targeted. An entity can be targeted by several, so this is every
    /// pair rather than one line each.
    pub fn entity_links(&self) -> Vec<crate::view_filters::EntityLink> {
        let entities = node::entities_with_ids(&self.tree.world);
        let keys = self.link_keys();

        // Where each entity is, and what it answers to.
        let positions: Vec<(NodeId, Vec3d, Option<&str>)> = entities
            .iter()
            .filter_map(|(id, entity)| {
                let origin = entity.origin().or_else(|| {
                    // A brush entity has no origin of its own; the middle of its brushes
                    // is where a link to it should point.
                    fold_contents(&entity.children, &self.selection)
                        .bounds
                        .map(|bounds| bounds.center())
                })?;
                Some((*id, origin, None))
            })
            .collect();

        // What each entity answers to: every value under a name key it carries.
        let names: Vec<(NodeId, Vec<&str>)> = entities
            .iter()
            .map(|(id, entity)| {
                let held = entity
                    .properties
                    .iter()
                    .filter(|(key, value)| keys.is_source(key) && !value.is_empty())
                    .map(|(_, value)| value.as_str())
                    .collect();
                (*id, held)
            })
            .collect();

        let mut links = Vec::new();
        // The pairs first, then which of them the selection reaches, since transitive
        // reach is a question about the whole graph rather than about one link.
        let mut pairs: Vec<(NodeId, NodeId, Vec3d, Vec3d, &str, &str)> = Vec::new();
        // Looked up by id rather than searched for, once per entity and per target.
        let position_of: HashMap<NodeId, Vec3d> =
            positions.iter().map(|(id, at, _)| (*id, *at)).collect();
        let names_of: HashMap<NodeId, &[&str]> = names
            .iter()
            .map(|(id, held)| (*id, held.as_slice()))
            .collect();
        for (id, entity) in &entities {
            let Some(from) = position_of.get(id) else {
                continue;
            };
            for (key, target) in &entity.properties {
                if !keys.is_target(key) || target.is_empty() {
                    continue;
                }
                for (target_id, to, _) in &positions {
                    let answers = names_of
                        .get(target_id)
                        .is_some_and(|held| held.iter().any(|name| name == target));
                    if !answers || target_id == id {
                        continue;
                    }
                    pairs.push((*id, *target_id, *from, *to, key.as_str(), target.as_str()));
                }
            }
        }

        // `CollectTransitiveSelectedLinksVisitor`: from the selection, out along links
        // either way, and on from whatever those reach.
        let touched: std::collections::HashSet<NodeId> = entities
            .iter()
            .filter(|(id, entity)| entity_touched(*id, &entity.children, &self.selection))
            .map(|(id, _)| *id)
            .collect();
        let mut reached = touched.clone();
        loop {
            let before = reached.len();
            for (source, target, _, _, _, _) in &pairs {
                if reached.contains(source) || reached.contains(target) {
                    reached.insert(*source);
                    reached.insert(*target);
                }
            }
            if reached.len() == before {
                break;
            }
        }

        for (source, target, from, to, key, name) in pairs {
            links.push(crate::view_filters::EntityLink {
                from,
                to,
                key: key.to_string(),
                name: name.to_string(),
                touches_selection: touched.contains(&source) || touched.contains(&target),
                reaches_selection: reached.contains(&source) || reached.contains(&target),
            });
        }
        links
    }

    /// The lines from one group of a linked set to its other copies.
    ///
    /// `GroupLinkRenderer::getLinks`: with exactly one group selected -- or, failing
    /// that, with one open -- a line runs from the middle of that group to the middle of
    /// every other group sharing its link id, which is what says where the copies of what
    /// is being edited are.
    pub fn linked_group_links(&self) -> Vec<(Vec3d, Vec3d)> {
        self.linked_group_links_inner().unwrap_or_default()
    }

    fn linked_group_links_inner(&self) -> Option<Vec<(Vec3d, Vec3d)>> {
        let groups = node::groups_with_ids(&self.tree.world);

        // The sole selected group, or the currently open group as a fallback.
        let selected: Vec<NodeId> = groups
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| self.selection.contains(id))
            .collect();
        let subject = match selected.as_slice() {
            [only] => Some(*only),
            _ => self.open_group,
        }?;

        let link_id = groups
            .iter()
            .find(|(id, _)| *id == subject)
            .and_then(|(_, group)| group.linked_group_id.clone())?;

        let centre = |group: &node::Group| -> Option<Vec3d> {
            fold_contents(&group.children, &self.selection)
                .bounds
                .map(|bounds| bounds.center())
        };

        let from = groups
            .iter()
            .find(|(id, _)| *id == subject)
            .and_then(|(_, group)| centre(group))?;
        Some(
            groups
                .iter()
                .filter(|(id, group)| {
                    *id != subject && group.linked_group_id.as_deref() == Some(link_id.as_str())
                })
                .filter_map(|(_, group)| centre(group).map(|to| (from, to)))
                .collect(),
        )
    }

    /// The outline of every group, for drawing what belongs together.
    pub fn group_bounds(&self) -> Vec<GroupBounds> {
        node::groups_with_ids(&self.tree.world)
            .into_iter()
            .filter_map(|(_, group)| {
                let contents = fold_contents(&group.children, &self.selection);
                contents.bounds.map(|bounds| GroupBounds {
                    bounds,
                    selected: contents.members > 0 && contents.all_selected,
                    // A linked group is drawn apart from an ordinary one, since editing
                    // one member changes every other.
                    linked: group.linked_group_id.is_some(),
                })
            })
            .collect()
    }

    /// Looks for a way from a point entity out of the level.
    ///
    /// There is no compiler behind this editor to find leaks, so it finds its own; see
    /// [`map_model::leak`] for what the answer can and cannot see. The trace it finds is kept
    /// so the views can draw it, and is cleared when the check comes back sealed.
    pub fn check_for_leaks(&mut self, cell_size: f64) -> map_model::leak::Leak {
        // Every brush that seals, which is every brush in the world: a brush entity's
        // brushes hold the world out as well as a worldspawn brush does, and a hidden
        // layer is still solid to the game.
        let solids: Vec<Brush> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .map(|(_, brush)| brush.clone())
            .collect();
        // Where the check starts: every point entity, since any of them reaching the void
        // is a leak.
        let starts: Vec<Vec3d> = node::entities_with_ids(&self.tree.world)
            .into_iter()
            .filter_map(|(_, entity)| entity.origin())
            .collect();

        let outcome = map_model::leak::find_leak(
            &solids,
            &starts,
            cell_size,
            map_model::leak::DEFAULT_CELL_BUDGET,
        );
        self.leak_trace = match &outcome {
            map_model::leak::Leak::Leaked(trace) => Some(trace.clone()),
            _ => None,
        };
        outcome
    }

    /// The way out of the level the last leak check found.
    #[inline]
    pub fn leak_trace(&self) -> Option<&map_model::leak::Trace> {
        self.leak_trace.as_ref()
    }

    /// Whether a node can be picked and edited at all.
    ///
    /// An object is unselectable when its layer is hidden or locked, it is individually hidden, or
    /// it is outside the currently open group.
    fn is_editable(&self, id: NodeId) -> bool {
        if self.is_hidden(id) {
            return false;
        }
        if !node::layer_of(&self.tree.world, id).is_some_and(|layer| self.layer_is_editable(layer))
        {
            return false;
        }
        self.inside_open_group(id)
    }

    /// [`Document::is_editable`] for many nodes whose layers are already known: `open`
    /// is [`Document::open_group_contents`], gathered once by the caller. Asking
    /// `is_editable` of every node walked the tree for each one's layer, which made
    /// Select All on a big map take seconds.
    pub(super) fn is_editable_on(
        &self,
        id: NodeId,
        layer: usize,
        open: &Option<HashSet<NodeId>>,
    ) -> bool {
        !self.is_hidden(id)
            && self.layer_is_editable(layer)
            && open.as_ref().is_none_or(|inside| inside.contains(&id))
    }

    /// Everything inside the open group and the group itself, or `None` with no group
    /// open: what [`Document::inside_open_group`] asks, gathered for many questions.
    pub(super) fn open_group_contents(&self) -> Option<HashSet<NodeId>> {
        self.open_group.map(|open| {
            let mut inside: HashSet<NodeId> = node::contents_of(&self.tree.world, open)
                .into_iter()
                .collect();
            inside.insert(open);
            inside
        })
    }

    /// Whether a node is inside the open group, or there is none open.
    fn inside_open_group(&self, id: NodeId) -> bool {
        let Some(open) = self.open_group else {
            return true;
        };
        // A brush entity's brushes are its children, so a node counts as inside when it
        // is one of the group's contents at any depth.
        id == open || node::contents_of(&self.tree.world, open).contains(&id)
    }

    /// The brushes a lasso can catch: every one that is drawn and can be edited.
    ///
    /// `Lasso::selectedNodes` walks the editable, visible nodes; the same filter the ray
    /// pick uses, so what a rectangle catches is what a click could have hit.
    pub fn selectable_brushes(&self) -> Vec<(NodeId, &Brush)> {
        let open = self.open_group_contents();
        node::brushes_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, layer)| self.is_editable_on(*id, *layer, &open))
            .map(|(id, brush, _)| (id, brush))
            .collect()
    }

    /// Selects the brush the given line of the map file belongs to.
    ///
    /// `Menu/Edit/Select by Line Number`: a compiler reports a problem at a line, and this
    /// is how that line becomes something to look at. A brush's faces carry the line they
    /// were read from, so the brush whose faces bracket the line is the one meant; a line
    /// between two brushes takes the one that started before it.
    ///
    /// Only brushes: an entity's own lines are not kept once the tree is built.
    pub fn select_by_line(&mut self, line: usize) -> bool {
        let mut best: Option<(usize, NodeId)> = None;
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            let Some(first) = brush.faces().iter().map(|face| face.line).min() else {
                continue;
            };
            let last = brush
                .faces()
                .iter()
                .map(|face| face.line)
                .max()
                .unwrap_or(first);
            if line >= first && line <= last {
                best = Some((first, id));
                break;
            }
            // Otherwise the nearest brush that starts before the line, which is what a
            // line naming the entity above it should reach.
            if first <= line && best.is_none_or(|(started, _)| first > started) {
                best = Some((first, id));
            }
        }

        match best {
            Some((_, id)) => {
                self.select_only(id);
                true
            }
            None => false,
        }
    }

    /// Loads a compiler's point file as the trace the views draw.
    ///
    /// Loads a compiler `.pts` leak trace containing one `x y z` point per line. Non-point lines
    /// are skipped so headers do not prevent valid points from loading.
    ///
    /// Returns how many points were read.
    pub fn load_point_file(&mut self, path: &Path) -> std::io::Result<usize> {
        let text = std::fs::read_to_string(path)?;
        let points: Vec<Vec3d> = text
            .lines()
            .filter_map(|line| {
                let mut numbers = line.split_whitespace().map(str::parse::<f64>);
                let mut next = || numbers.next()?.ok();
                Some(Vec3d::new(next()?, next()?, next()?))
            })
            .collect();

        let read = points.len();
        self.leak_trace = (read >= 2).then_some(map_model::leak::Trace { points });
        self.point_file = self.leak_trace.is_some().then(|| path.to_path_buf());
        self.trace_point = 0;
        self.revision = self.revision.wrapping_add(1);
        Ok(read)
    }

    /// Whether there is a point file to read again.
    pub fn can_reload_point_file(&self) -> bool {
        self.point_file.as_ref().is_some_and(|path| path.is_file())
    }

    /// Reads the point file again, for when the compiler has been run since.
    ///
    /// `Menu/File/Reload Point File`.
    pub fn reload_point_file(&mut self) -> Option<std::io::Result<usize>> {
        let path = self.point_file.clone()?;
        Some(self.load_point_file(&path))
    }

    /// Puts the point file away.
    ///
    /// `Menu/File/Unload Point File`, which is the same thing as forgetting a leak check's
    /// own trace: both are the one line the views draw.
    pub fn unload_point_file(&mut self) {
        self.clear_leak_trace();
        self.revision = self.revision.wrapping_add(1);
    }

    /// Whether a trace is loaded at all.
    pub fn point_file_loaded(&self) -> bool {
        self.leak_trace.is_some()
    }

    /// Steps along the trace, and says where to look from and which way.
    ///
    /// Places the camera sixteen units above the next or previous trace point and looks along the
    /// trace. Returns `None` when already at that end.
    pub fn step_along_trace(&mut self, forwards: bool) -> Option<(Vec3d, Vec3d)> {
        let points = &self.leak_trace.as_ref()?.points;
        let last = points.len().checked_sub(1)?;
        let at = self.trace_point.min(last);
        let next = if forwards {
            (at < last).then_some(at + 1)?
        } else {
            at.checked_sub(1)?
        };
        self.trace_point = next;

        let here = points[next];
        // The way the trace goes on from here, or the way it came in at its end.
        let direction = if next < last {
            points[next + 1] - here
        } else {
            here - points[next - 1]
        };
        Some((here + Vec3d::new(0.0, 0.0, 16.0), vec::normalize(direction)))
    }

    /// The model a point entity is drawn as, and the material its skin is.
    ///
    /// Nothing for a class with no model, or one whose file was not found: those are drawn
    /// as their box.
    fn model_of(
        &self,
        node: NodeId,
    ) -> Option<(
        std::rc::Rc<map_model::entity_model::EntityModel>,
        Vec<Option<usize>>,
        Vec3d,
    )> {
        match &node::find_node(&self.tree.world, node)?.kind {
            NodeKind::Entity(entity) => self.model_of_entity(entity),
            _ => None,
        }
    }

    /// [`Document::model_of`] for an entity already in hand.
    fn model_of_entity(
        &self,
        entity: &EntityNode,
    ) -> Option<(
        std::rc::Rc<map_model::entity_model::EntityModel>,
        Vec<Option<usize>>,
        Vec3d,
    )> {
        if let Some(model) = self.imported_model(entity) {
            let named = self
                .imported_model_path(entity)?
                .to_string_lossy()
                .into_owned();
            let material = model
                .skins()
                .enumerate()
                .map(|(i, _)| self.materials.index_of(&Self::model_skin_name(&named, i)))
                .collect();
            return Some((model, material, Vec3d::new(1.0, 1.0, 1.0)));
        }
        let classname = entity.classname()?;
        let model = self.models.get(classname)?.clone();
        let definition = self.definitions.as_ref()?.find(classname)?.model.as_ref()?;
        // Evaluate the model expression against this entity's own properties, so a class
        // that picks its model from a `spawnflags` bit draws the right one.
        let named = definition
            .resolve(&entity.properties)
            .ok()
            .map(|spec| spec.path.to_string_lossy().into_owned())
            .filter(|path| !path.is_empty())?;
        let material = model
            .skins()
            .enumerate()
            .map(|(i, _)| self.materials.index_of(&Self::model_skin_name(&named, i)))
            .collect();
        // A `model({ …, scale: … })` or a class that scales its model by a property; the
        // game configuration's own default scale expression is not plumbed here yet.
        let scale = definition.resolve_scale(&entity.properties, None);
        Some((model, material, scale))
    }

    /// Where a point entity's model stands, and which way it faces.
    ///
    /// The origin, and the yaw its `angle` property gives -- which is what the game does
    /// with that property, and so what the editor has to do to show where a monster will
    /// be looking.
    fn entity_placement(&self, node: NodeId) -> (Vec3d, f64) {
        let Some(NodeKind::Entity(entity)) =
            node::find_node(&self.tree.world, node).map(|n| &n.kind)
        else {
            return (Vec3d::zero(), 0.0);
        };
        Self::entity_placement_of(entity)
    }

    /// [`Document::entity_placement`] for an entity already in hand.
    fn entity_placement_of(entity: &EntityNode) -> (Vec3d, f64) {
        let origin = entity.origin().unwrap_or_else(Vec3d::zero);
        // `angle` is degrees about Z; the two negative values Quake gives it mean straight
        // up and straight down, which a model in an editor is not turned by.
        if entity.property("_heron_model").is_some() {
            return (origin, 0.0);
        }
        let angle = entity
            .property("angle")
            .and_then(|value| value.trim().parse::<f64>().ok())
            .filter(|angle| *angle >= 0.0)
            .unwrap_or(0.0);
        (origin, angle)
    }

    /// The name a model's skin is kept under in the material collection.
    ///
    /// A material the browser does not list: the collection is what the renderer uploads
    /// its textures from, so a model's skin has to be in it, but it is not something a
    /// face can be given. The leading `#` is what marks it as the editor's own rather than
    /// the map's; see [`MaterialCollection`] and the material browser.
    fn model_material_name(named: &str) -> String {
        format!("#model/{named}")
    }

    fn model_skin_name(named: &str, index: usize) -> String {
        if index == 0 {
            Self::model_material_name(named)
        } else {
            format!("#model/{named}#skin/{index}")
        }
    }

    fn register_model_skins(&mut self, named: &str, model: &map_model::entity_model::EntityModel) {
        for (index, skin) in model.skins().enumerate() {
            let material = Self::model_skin_name(named, index);
            if self.materials.get(&material).is_none() {
                let mut loaded = map_model::material::Material::rgba(
                    material,
                    PathBuf::from(named),
                    skin.width,
                    skin.height,
                    skin.pixels.clone(),
                );
                if let Some(pbr) = model.pbr_at(index) {
                    loaded.model_pbr = Some(std::sync::Arc::new(pbr.clone()));
                    loaded.normal_map = pbr.normal.as_ref().map(|n| {
                        map_model::material::NormalMap::rgba(
                            PathBuf::from(named),
                            n.width,
                            n.height,
                            n.pixels.clone(),
                            false,
                        )
                    });
                }
                self.materials.insert(loaded);
                self.materials_revision = self.materials_revision.wrapping_add(1);
            }
        }
    }

    /// Loads the model every point class names, and keeps its skin where the renderer can
    /// reach it.
    ///
    /// Called after the definitions or the materials change, and when the game path does:
    /// the definitions say which models, the game path says where they are, and the
    /// collection is emptied and refilled whenever a source is added or taken away.
    pub fn load_entity_models(&mut self) {
        if self.sync_game_assets() {
            self.rebuild_materials();
            return;
        }

        let Some(definitions) = &self.definitions else {
            self.load_imported_models();
            return;
        };
        let wanted: Vec<(String, String)> = definitions
            .all()
            .iter()
            .filter_map(|definition| {
                let path = definition
                    .model
                    .as_ref()?
                    .default_model_specification()
                    .ok()?
                    .path
                    .to_string_lossy()
                    .into_owned();
                (!path.is_empty()).then(|| (definition.name.clone(), path))
            })
            .collect();

        // Built-in symbols are fallbacks; project models and sprites take precedence.
        self.sprites.clear();
        self.models.retain(|key, _| key.starts_with("#import/"));
        let has_points = !node::point_entities_by_layer(&self.tree.world).is_empty();
        for definition in definitions.all() {
            if !has_points
                || definition.name == "light"
                || definition.kind != map_model::fgd::DefinitionKind::Point
                || definition.model.is_some()
                || crate::entity_icons::kind(&definition.name).is_none()
            {
                continue;
            }
            let material = format!("{BUILTIN_SPRITE_PREFIX}{}", definition.name);
            if self.materials.get(&material).is_none() {
                let sprite = crate::entity_icons::sprite(&definition.name);
                self.materials.insert(map_model::material::Material::rgba(
                    material.clone(),
                    std::path::PathBuf::new(),
                    sprite.width,
                    sprite.height,
                    sprite.pixels,
                ));
                self.materials_revision = self.materials_revision.wrapping_add(1);
            }
            self.sprites
                .insert(definition.name.clone(), (material, 48, 48));
        }

        for (classname, named) in wanted {
            if named.to_lowercase().ends_with(".spr") {
                let Some(sprite) = self.model_store.sprite(&named) else {
                    continue;
                };
                let material = Self::model_material_name(&named);
                if self.materials.get(&material).is_none() {
                    self.materials.insert(map_model::material::Material::rgba(
                        material.clone(),
                        std::path::PathBuf::from(&named),
                        sprite.width,
                        sprite.height,
                        sprite.pixels.clone(),
                    ));
                    self.materials_revision = self.materials_revision.wrapping_add(1);
                }
                self.sprites
                    .insert(classname, (material, sprite.width, sprite.height));
                continue;
            }

            let Some(model) = self.model_store.model(&named) else {
                continue;
            };
            self.register_model_skins(&named, &model);
            self.models.insert(classname, model);
        }
        self.load_imported_models();
        self.revision = self.revision.wrapping_add(1);
    }

    /// Where each drawn point entity's sprite sits and how big it is, for the renderer's
    /// billboard pass. Empty unless a class names an `iconsprite(...)` whose `.spr` was
    /// found and the Entity models view filter is on.
    pub fn sprites(&self) -> Vec<crate::renderer::SpriteInstance> {
        if self.sprites.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (id, entity, layer) in node::point_entities_by_layer(&self.tree.world) {
            if !self.node_is_drawn(id, layer) {
                continue;
            }
            let Some(classname) = entity.classname() else {
                continue;
            };
            let Some((material, width, height)) = self.sprites.get(classname) else {
                continue;
            };
            let Some(origin) = entity.origin() else {
                continue;
            };
            let Some(material_index) = self.materials.index_of(material) else {
                continue;
            };
            // A fixed on-screen size would need the camera; a fixed world size keeps the
            // sprite in proportion with the level, which is what an editor icon wants.
            const SPRITE_HEIGHT: f32 = 16.0;
            let aspect = *width as f32 / (*height).max(1) as f32;
            out.push(crate::renderer::SpriteInstance {
                position: origin,
                half_width: SPRITE_HEIGHT * aspect * 0.5,
                half_height: SPRITE_HEIGHT * 0.5,
                material: material_index,
            });
        }
        out
    }

    /// Reads a compiler's portal file, to be drawn over the map.
    ///
    /// `Menu/File/Load Portal File`: `vis` writes a `.prt` beside the compiled level with
    /// the portals it cut the world into, and drawing them says where the compiler thinks
    /// one room ends and the next begins. Returns how many portals were read.
    pub fn load_portal_file(&mut self, path: &Path) -> Result<usize, String> {
        let text = std::fs::read_to_string(path).map_err(|why| why.to_string())?;
        let portals = map_model::portal::read(&text).map_err(|why| why.to_string())?;
        let read = portals.polygons.len();
        self.portals = Some(portals);
        self.portal_file = Some(path.to_path_buf());
        self.revision = self.revision.wrapping_add(1);
        Ok(read)
    }

    /// The portals to draw, if a portal file has been read.
    pub fn portals(&self) -> Option<&map_model::portal::Portals> {
        self.portals.as_ref()
    }

    /// Whether there is a portal file to read again.
    pub fn can_reload_portal_file(&self) -> bool {
        self.portal_file.as_ref().is_some_and(|path| path.is_file())
    }

    /// Whether portals are being drawn at all.
    pub fn portal_file_loaded(&self) -> bool {
        self.portals.is_some()
    }

    /// Reads the portal file again, for when the compiler has been run since.
    pub fn reload_portal_file(&mut self) -> Option<Result<usize, String>> {
        let path = self.portal_file.clone()?;
        Some(self.load_portal_file(&path))
    }

    /// Puts the portals away.
    pub fn unload_portal_file(&mut self) {
        self.portals = None;
        self.portal_file = None;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Forgets the last leak check's trace.
    pub fn clear_leak_trace(&mut self) {
        self.leak_trace = None;
        self.point_file = None;
        self.trace_point = 0;
    }

    /// The box each brush entity's brushes fill, and whether all of them are selected.
    ///
    /// `Preferences::ShowBrushEntityBounds`. A brush entity's brushes are drawn like any
    /// others, so without this there is nothing saying which door they belong to.
    pub fn brush_entity_bounds(&self) -> Vec<(Bbox3d, bool)> {
        node::entities_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(_, entity)| entity.is_brush_entity())
            .filter_map(|(_, entity)| {
                // It is the brushes that are drawn and selected.
                let contents = fold_contents(&entity.children, &self.selection);
                contents
                    .bounds
                    .map(|bounds| (bounds, contents.brushes > 0 && contents.brushes_selected))
            })
            .collect()
    }

    /// What a sweep would start from: the selected faces, their middle and their normal.
    ///
    /// `SweepSource`. A face selection is what the tool sweeps; with whole brushes
    /// selected there is no one face to sweep, so there is nothing to start from.
    pub fn sweep_source(&self) -> Option<crate::sweep_tool::SweepSource> {
        let mut faces = Vec::new();
        let mut attributes = Vec::new();
        let mut total = Vec3d::zero();
        let mut count = 0usize;
        let mut normal = Vec3d::zero();

        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            for index in 0..brush.faces().len() {
                if !self.is_face_selected(FaceRef {
                    node: id,
                    face: index,
                }) {
                    continue;
                }
                let Some(polygon) = brush.face_polygon(index) else {
                    continue;
                };
                for corner in &polygon {
                    total += *corner;
                    count += 1;
                }
                let source = &brush.faces()[index];
                normal += source.normal();
                attributes.push(source.clone());
                faces.push(polygon);
            }
        }

        if faces.is_empty() || count == 0 {
            return None;
        }

        Some(crate::sweep_tool::SweepSource {
            faces,
            attributes,
            center: total / count as f64,
            normal: if vec::length(normal) > f64::ALMOST_ZERO {
                vec::normalize(normal)
            } else {
                Vec3d::new(0.0, 0.0, 1.0)
            },
        })
    }

    /// Adds the brushes a sweep makes, as one step in the history.
    ///
    /// Returns how many were added. The source faces are left alone: a sweep adds what the
    /// face travelled through, it does not consume the face.
    pub fn sweep_selection(
        &mut self,
        transform: &crate::sweep_tool::SweepTransform,
        parameters: &crate::sweep_tool::SweepParameters,
    ) -> usize {
        self.sweep_selection_checked(transform, parameters)
            .unwrap_or(0)
    }

    /// Rejects the whole operation rather than inserting a partial or intersecting path.
    pub fn sweep_selection_checked(
        &mut self,
        transform: &crate::sweep_tool::SweepTransform,
        parameters: &crate::sweep_tool::SweepParameters,
    ) -> Result<usize, String> {
        if self.in_transaction() {
            return Err("Finish the current drag first.".into());
        }
        let source = self.sweep_source().ok_or("Select faces to sweep.")?;
        let mut ids = Vec::new();
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            for face in 0..brush.faces().len() {
                if brush.geometry_face_of(face).is_some()
                    && self.is_face_selected(FaceRef { node: id, face })
                {
                    ids.push(id);
                }
            }
        }
        if ids.iter().any(|id| !self.is_editable(*id)) {
            return Err("Every source face must be editable.".into());
        }
        let brushes = crate::sweep_tool::generate_checked(
            &source,
            transform,
            parameters,
            &self.current_material,
        )?;
        if brushes
            .iter()
            .flat_map(|b| b.vertex_positions())
            .any(|p| !self.world_bounds.contains_point(p))
        {
            return Err("The sweep extends outside the world bounds.".into());
        }
        let count = brushes.len();
        let per_face = parameters.segments * parameters.iterations;
        let mut replacements: HashMap<NodeId, Vec<Brush>> = HashMap::new();
        for (id, cells) in ids.iter().zip(brushes.chunks(per_face)) {
            let initial = self
                .brush(*id)
                .ok_or("A source brush no longer exists.")?
                .clone();
            replacements
                .entry(*id)
                .or_insert_with(|| vec![initial])
                .extend(cells.iter().cloned());
        }
        self.replace_geometry_in_place(replacements);
        self.selection.retain(|id| !ids.contains(id));
        Ok(count)
    }

    /// Returns the face the UV editor should show.
    ///
    /// Displays the first selected face, or the first face of the first selected brush. UV
    /// operations still apply to the entire selection.
    pub fn face_to_edit(&self) -> Option<(FaceRef, &map_model::brush_face::BrushFace)> {
        if self.face_selection.is_empty() {
            if let Some(reference) =
                self.face_edit_targets.iter().copied().find(|face| {
                    self.selection.contains(&face.node) && self.face_at(*face).is_some()
                })
            {
                return Some((reference, self.face_at(reference)?));
            }
        }
        let brushes = node::brushes_with_ids(&self.tree.world);

        // A face selection is more specific than a brush selection, so it wins.
        for (id, brush) in &brushes {
            for index in 0..brush.geometry().face_count() {
                let reference = FaceRef {
                    node: *id,
                    face: index,
                };
                if self.is_face_selected(reference) {
                    let source = brush.source_face_index(index)?;
                    return Some((reference, &brush.faces()[source]));
                }
            }
        }

        for (id, brush) in &brushes {
            if !self.selection.contains(id) {
                continue;
            }
            let source = brush.source_face_index(0)?;
            return Some((FaceRef { node: *id, face: 0 }, &brush.faces()[source]));
        }

        None
    }

    /// Returns the vertices of the face the UV editor is showing.
    pub fn face_to_edit_polygon(&self) -> Option<Vec<Vec3d>> {
        let (reference, _) = self.face_to_edit()?;
        let brush = self.brush(reference.node)?;
        Some(
            brush
                .geometry()
                .face_positions(map_model::polyhedron::FaceId(reference.face as u32)),
        )
    }

    // The host echoes edits to their sender in canonical order. Added nodes may
    // already exist from optimistic local editing, so replace rather than duplicate.

    /// Returns which document this is, counting from the start of the session.
    #[inline]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the edges of every unselected brush, for the 2D views to draw.
    #[inline]
    pub fn edges(&self) -> Ref<'_, Vec<(Vec3d, Vec3d)>> {
        Ref::map(self.edge_cache_ref(), |cache| &cache.plain)
    }

    /// Returns the edges of every selected brush, drawn differently from the rest.
    #[inline]
    pub fn selected_edges(&self) -> Ref<'_, Vec<(Vec3d, Vec3d)>> {
        Ref::map(self.edge_cache_ref(), |cache| &cache.selected)
    }

    /// Returns the edges of every brush on a locked layer.
    ///
    /// Drawn in `Preferences::LockedEdgeColor`, so that geometry a click will not touch
    /// looks different from geometry it will.
    #[inline]
    pub fn locked_edges(&self) -> Ref<'_, Vec<(Vec3d, Vec3d)>> {
        Ref::map(self.edge_cache_ref(), |cache| &cache.locked)
    }

    /// Returns the edges of every brush the other person in a live session has selected.
    #[inline]
    pub fn peer_edges(&self) -> Ref<'_, Vec<(u32, Vec3d, Vec3d)>> {
        Ref::map(self.edge_cache_ref(), |cache| &cache.peer)
    }

    /// Where each other participant's 3D camera stands and looks, if a live session
    /// has said.
    #[inline]
    pub fn peer_cameras(&self) -> &std::collections::HashMap<u32, (Vec3d, Vec3d)> {
        &self.peer_cameras
    }

    /// Updates the name shown beside a participant's camera.
    pub fn set_peer_name(&mut self, peer: u32, name: String) {
        self.peer_names.insert(peer, name);
    }

    pub fn peer_name(&self, peer: u32) -> String {
        self.peer_names
            .get(&peer)
            .cloned()
            .unwrap_or_else(|| format!("Peer {peer}"))
    }

    /// Updates presence without changing map geometry or undo history.
    pub fn set_peer_camera(&mut self, peer: u32, camera: Option<(Vec3d, Vec3d)>) {
        match camera {
            Some(camera) => {
                self.peer_cameras.insert(peer, camera);
            }
            None => {
                self.peer_cameras.remove(&peer);
            }
        }
    }

    /// Sets which nodes one participant in a live session has selected, and rebuilds
    /// the view geometry so their work area shows. See [`crate::collab`].
    pub fn set_peer_selection(&mut self, peer: u32, ids: HashSet<NodeId>) {
        if self
            .peer_selection
            .get(&peer)
            .is_some_and(|held| *held == ids)
        {
            return;
        }
        if ids.is_empty() {
            self.peer_selection.remove(&peer);
        } else {
            self.peer_selection.insert(peer, ids);
        }
        self.mark_changed();
    }

    /// Forgets one participant wholesale -- their selection and their camera -- e.g.
    /// when they leave the session.
    pub fn remove_peer(&mut self, peer: u32) {
        self.peer_names.remove(&peer);
        self.peer_cameras.remove(&peer);
        if self.peer_selection.remove(&peer).is_some() {
            self.mark_changed();
        }
    }

    /// Forgets every participant, e.g. when the session itself ends.
    pub fn clear_peers(&mut self) {
        self.peer_names.clear();
        self.peer_cameras.clear();
        if !self.peer_selection.is_empty() {
            self.peer_selection.clear();
            self.mark_changed();
        }
    }

    /// The nodes this editor has selected, for a live session to broadcast.
    pub fn selection_ids(&self) -> &HashSet<NodeId> {
        &self.selection
    }

    /// Returns each point entity's box and colour, for the views to draw.
    ///
    /// A point entity has no geometry, so it is drawn as a box at its origin. The size and
    /// colour come from its entity definition when one is loaded, and from a default
    /// otherwise — a light with no FGD should still be visible.
    pub fn entity_boxes(&self) -> Vec<EntityBox> {
        node::point_entities_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, layer)| self.node_is_drawn(*id, *layer))
            .filter_map(|(id, entity, _)| self.entity_box_of(id, entity))
            .collect()
    }

    /// [`Document::entity_boxes`] for one entity, drawn or not; `None` without an origin.
    fn entity_box_of(&self, id: NodeId, entity: &EntityNode) -> Option<EntityBox> {
        let origin = entity.origin()?;
        let definition = entity
            .classname()
            .and_then(|name| self.definitions.as_ref()?.find(name));

        let local = self
            .imported_model(entity)
            .map(|m| m.bounds)
            .or_else(|| definition.and_then(|d| d.bounds))
            .unwrap_or_else(|| Bbox3d::from_scalars(-DEFAULT_ENTITY_RADIUS, DEFAULT_ENTITY_RADIUS));
        let color = definition
            .and_then(|d| d.color)
            .unwrap_or(default_entity_color());

        Some(EntityBox {
            node: id,
            bounds: local.translate(origin),
            color,
            selected: self.selection.contains(&id),
        })
    }

    /// Loads entity definitions from an FGD file.
    ///
    /// Returns whatever the reader worked around. Definitions are optional: without them
    /// the editor still opens a map, it just cannot say what an entity's properties mean.
    pub fn load_definitions(&mut self, path: impl AsRef<Path>) -> Result<Vec<String>, OpenError> {
        let path = path.as_ref();

        // Chooses the entity-definition reader by extension: `.def` uses annotated C-style comments
        // and `.ent` uses XML.
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
        let definitions = if extension.as_deref() == Some("def") {
            let source = std::fs::read_to_string(path).map_err(OpenError::Io)?;
            map_model::def::read_def(&source).map_err(|error| {
                OpenError::Map(map_model::io::map_reader::ReadError::Parse {
                    message: error.message,
                    line: error.line,
                })
            })?
        } else if extension.as_deref() == Some("ent") {
            let source = std::fs::read_to_string(path).map_err(OpenError::Io)?;
            map_model::ent::read_ent(&source).map_err(|error| {
                OpenError::Map(map_model::io::map_reader::ReadError::Parse {
                    message: error.message,
                    line: error.line,
                })
            })?
        } else {
            // `read_fgd_file` follows `@include`, resolving each against this file.
            map_model::fgd::read_fgd_file(path).map_err(|error| {
                OpenError::Map(map_model::io::map_reader::ReadError::Parse {
                    message: error.message,
                    line: error.line,
                })
            })?
        };

        let warnings = definitions.warnings.clone();
        self.definitions = Some(crate::builtin_entities::with_project(definitions));
        self.definitions_path = Some(path.to_path_buf());
        self.export_classes_override = None;
        self.revision = self.revision.wrapping_add(1);
        // The definitions are what name the models, so this is where they are loaded.
        self.load_entity_models();
        Ok(warnings)
    }

    /// The file the entity definitions were read from, if any were.
    pub fn definitions_path(&self) -> Option<&Path> {
        self.definitions_path.as_deref()
    }

    /// Returns the entity definitions in use, if any have been loaded.
    #[inline]
    pub fn definitions(&self) -> Option<&map_model::fgd::EntityDefinitions> {
        self.definitions.as_ref()
    }

    /// Returns the document's surfaces as triangles, for the 3D view.
    ///
    /// Each brush face is triangulated as a fan from its first vertex, which is valid
    /// because a convex solid's faces are themselves convex. The normal is the face's own,
    /// so the shading is flat and the brush's edges stay legible.
    pub fn triangles(&self, filters: &crate::view_filters::ViewFilters) -> Vec<Triangle> {
        self.preview_mesh(filters).triangles
    }

    pub fn preview_mesh(
        &self,
        filters: &crate::view_filters::ViewFilters,
    ) -> crate::motion_preview::Mesh {
        self.preview_mesh_with_clip(filters, None)
    }

    pub fn preview_mesh_with_clip(
        &self,
        filters: &crate::view_filters::ViewFilters,
        clip: Option<&crate::clip_tool::ClipTool>,
    ) -> crate::motion_preview::Mesh {
        self.with_preview_surfaces(filters, clip, SurfaceCache::triangles)
    }

    /// Clean scene surfaces for image output: visible brushes, patches and loaded models.
    /// Editor entity boxes, axes, selection colours and locked-layer tint are omitted.
    pub fn render_triangles(&self) -> Vec<Triangle> {
        self.render_triangles_at(None)
    }

    /// [`Document::render_triangles`] with the animated brushes where playback has them
    /// `time` seconds in, as the 3D view's animation preview draws them. Entity models stay
    /// at rest.
    pub fn render_triangles_at(&self, time: Option<f64>) -> Vec<Triangle> {
        let filters = crate::view_filters::ViewFilters {
            show_point_entities: false,
            show_edges: false,
            ..Default::default()
        };
        let mesh = self.preview_mesh(&filters);
        let mut triangles = match time {
            Some(time) => mesh.at(time),
            None => mesh.triangles,
        };
        for entity in self.entity_boxes() {
            if let Some((model, material, scale)) = self.model_of(entity.node) {
                triangles.extend(model_triangles(
                    &model,
                    material,
                    self.entity_placement(entity.node),
                    scale,
                    false,
                    false,
                ));
            }
        }
        for triangle in &mut triangles {
            triangle.selected = false;
            triangle.locked = false;
        }
        triangles
    }

    /// Returns a counter that changes whenever the document's geometry does.
    ///
    /// The 3D view uploads its buffers only when this changes, rather than on every frame.
    #[inline]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Changes on edits and resource updates, but not selection-only notifications.
    pub fn content_revision(&self) -> u64 {
        self.revision.wrapping_sub(self.selection_change_count)
    }

    /// Returns the number of brushes in the document. Part of the document's public
    /// surface; the views stopped painting it but the tests still lean on it.
    #[allow(dead_code)]
    pub fn brush_count(&self) -> usize {
        node::brushes(&self.tree.world).len()
    }

    /// Returns the smallest box containing every brush, or `None` if there are none.
    pub fn bounds(&self) -> Option<Bbox3d> {
        let brushes = node::brushes(&self.tree.world);
        let mut iter = brushes.iter();
        let first = iter.next()?.bounds();
        Some(iter.fold(first, |acc, brush| acc.merged(&brush.bounds())))
    }

    /// Returns the document's name for the window title.
    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".to_string())
    }

    /// The full window title: the document's name, a bullet in front of it while there
    /// are unsaved changes, and the application name after it.
    ///
    /// `MapFrame::updateTitle`, which does the same with Qt's `[*]` placeholder. A
    /// leading mark rather than a trailing one so the name still reads first.
    pub fn window_title(&self) -> String {
        let mark = if self.modified { "• " } else { "" };
        format!("{mark}{} — Heron Brush", self.name())
    }

    /// Moves every brush's vertices onto the given grid.
    ///
    /// Returns how many brushes were snapped and how many were left alone because
    /// snapping would have collapsed them.
    pub fn snap_vertices_to_grid(&mut self, grid_size: f64) -> SnapReport {
        let mut report = SnapReport::default();
        let mut changed = Vec::new();

        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if brush.vertices_are_on_grid(grid_size) {
                report.already_aligned += 1;
                continue;
            }
            match brush.snap_vertices(grid_size) {
                Ok(snapped) => {
                    report.snapped += 1;
                    changed.push((id, brush.clone(), snapped));
                }
                Err(_) => report.refused += 1,
            }
        }

        if !changed.is_empty() {
            let before = changed
                .iter()
                .map(|(id, b, _)| brush_node(*id, b.clone()))
                .collect();
            let after = changed
                .into_iter()
                .map(|(id, _, a)| brush_node(id, a))
                .collect();
            self.apply_and_record(Edit::Changed { before, after });
        }
        report
    }

    /// Recomputes the cached geometry the views draw.
    ///
    /// Preserve tessellation on selection-only changes; all other revisions miss the cache.
    fn mark_selection_changed(&mut self) {
        let preview = self.shape_preview.take().filter(|s| {
            self.face_selection.is_empty()
                && s.nodes.len() == self.selection.len()
                && s.nodes.iter().all(|n| self.selection.contains(&n.id))
        });
        self.selection_change_count = self.selection_change_count.wrapping_add(1);
        // The surfaces stay: the next rebuild patches the selection flags of the nodes
        // whose selection changed.
        self.note_changed();
        self.shape_preview = preview.map(|mut s| {
            s.revision = self.revision();
            s
        });
    }

    /// Selected and unselected edges are kept apart so that drawing does not have to
    /// consult the selection per edge.
    /// Says that the view geometry is now wrong and the map has changed.
    ///
    /// The walk itself is *not* done here. An edit marks the edge cache stale and the
    /// next reader rebuilds it ([`Document::edge_cache_ref`]), so a drag emitting one
    /// edit per pointer move, or a plugin touching every face of a thousand brushes,
    /// pays for one walk per frame rather than one per edit. What stays eager is what is
    /// cheap and what a later reader could not reconstruct: the revision, the note of
    /// what was selected, and telling the views they are out of date.
    pub fn mark_changed(&mut self) {
        // Nobody said what changed, so every surface is rebuilt.
        self.surface_cache.get_mut().dirty = surfaces::Dirty::All;
        self.note_changed();
    }

    /// [`Document::mark_changed`] minus what it says about the 3D surfaces, for the
    /// changes that say that themselves.
    fn note_changed(&mut self) {
        // Selection changes and other edits finish a provisional shape. Its own
        // regeneration temporarily takes this state out before applying geometry.
        self.shape_preview = None;
        if self.sprites.is_empty() && !node::point_entities_by_layer(&self.tree.world).is_empty() {
            self.load_entity_models();
        }
        self.load_imported_models();
        self.remember_selection_bounds();
        self.revision = self.revision.wrapping_add(1);
        self.surface_cache.get_mut().dirty_at = self.revision;
        self.edges_stale.set(true);
        self.edges_stale_selected_only.set(false);

        // Tell the views. This only marks them dirty, so it is safe to call while the
        // document is still borrowed -- they draw later, once it is not.
        if let Some(on_changed) = &self.on_changed {
            on_changed();
        }
    }

    /// Remembers the bounds of what is selected, so new geometry can be sized against it
    /// after the selection has gone ([`Document::reference_bounds`]).
    ///
    /// This cannot wait for a reader the way the edge walk can: once the selection is
    /// cleared, the bounds it had are unrecoverable. It can be skipped while the
    /// selection is unchanged, though, which is every edit of a drag -- and comparing
    /// two id sets is far cheaper than the three tree walks the bounds cost.
    fn remember_selection_bounds(&mut self) {
        if self.selection == self.bounds_remembered_for {
            return;
        }
        self.bounds_remembered_for.clone_from(&self.selection);
        if let Some(bounds) = self.selection_bounds() {
            self.last_selection_bounds = Some(bounds);
        }
    }

    /// The edge cache, walked again first if an edit has made it stale.
    fn edge_cache_ref(&self) -> Ref<'_, EdgeCache> {
        if self.edges_stale.replace(false) {
            let selected_only = self.edges_stale_selected_only.replace(false);
            if !(selected_only && self.rebuild_selected_edges()) {
                self.rebuild_edges();
            }
        }
        self.edge_cache.borrow()
    }

    /// A number that changes whenever the edges of unselected, locked or other people's
    /// brushes do: what a flat view's cached picture of them is keyed on.
    pub fn static_edges_generation(&self) -> u64 {
        self.edge_cache_ref().static_generation
    }

    /// Notes, just after a change was marked, that it only moved selected brushes, so
    /// the next read of the edges walks those again rather than the whole map. Only
    /// narrows a staleness that began with this change: `was_stale` and
    /// `was_selected_only` are how the cache stood before it.
    pub(super) fn note_selected_geometry_only(&self, was_stale: bool, was_selected_only: bool) {
        if !was_stale || was_selected_only {
            self.edges_stale_selected_only.set(true);
        }
    }

    /// How the edge cache stands: stale, and stale only in the selection's geometry.
    pub(super) fn edge_staleness(&self) -> (bool, bool) {
        (self.edges_stale.get(), self.edges_stale_selected_only.get())
    }

    /// Whether every part of `edit` replaces a selected brush and nothing else.
    pub(super) fn touches_only_selected_brushes(&self, edit: &Edit) -> bool {
        match edit {
            Edit::Changed { after, .. } => after.iter().all(|node| {
                matches!(node.kind, NodeKind::Brush(_)) && self.selection.contains(&node.id)
            }),
            Edit::Batch(parts) => parts
                .iter()
                .all(|part| self.touches_only_selected_brushes(part)),
            Edit::Added { .. } | Edit::Removed { .. } | Edit::Reordered { .. } => false,
        }
    }

    /// Refills only the selected brushes' edges, when the selection is the one the cache
    /// was filled for and every selected brush is drawn on an editable layer. Returns
    /// false, having done nothing, when a full walk is needed instead.
    fn rebuild_selected_edges(&self) -> bool {
        let mut cache = self.edge_cache.borrow_mut();
        if cache.selection != self.selection || !self.hidden_tags.is_empty() {
            return false;
        }
        let mut selected = Vec::with_capacity(cache.selected.len());
        for (id, brush, layer) in node::brushes_by_layer(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }
            if !self.node_is_drawn(id, layer) || !self.layer_is_editable(layer) {
                return false;
            }
            selected.extend(brush.edge_positions());
        }
        cache.selected = selected;
        true
    }

    /// Walks the tree and refills the edge cache. Called only from
    /// [`Document::edge_cache_ref`], on a read that found it stale.
    fn rebuild_edges(&self) {
        let mut cache = self.edge_cache.borrow_mut();
        // The old lists are kept to compare with, so the generation only moves when the
        // unselected edges really changed.
        let old_plain = std::mem::take(&mut cache.plain);
        let old_locked = std::mem::take(&mut cache.locked);
        let old_peer = std::mem::take(&mut cache.peer);
        cache.selected.clear();
        let hidden_by_tag = self.brushes_hidden_by_tags();
        for (id, brush, layer) in node::brushes_by_layer(&self.tree.world) {
            if !self.node_is_drawn(id, layer) || hidden_by_tag.contains(&id) {
                continue;
            }
            let target = if !self.layer_is_editable(layer) {
                &mut cache.locked
            } else if self.selection.contains(&id) {
                &mut cache.selected
            } else if let Some(peer) = self
                .peer_selection
                .iter()
                .find(|(_, ids)| ids.contains(&id))
                .map(|(peer, _)| *peer)
            {
                cache.peer.extend(
                    brush
                        .edge_positions()
                        .into_iter()
                        .map(|(from, to)| (peer, from, to)),
                );
                continue;
            } else {
                &mut cache.plain
            };
            target.extend(brush.edge_positions());
        }
        if cache.plain != old_plain || cache.locked != old_locked || cache.peer != old_peer {
            cache.static_generation = cache.static_generation.wrapping_add(1);
        }
        cache.selection.clone_from(&self.selection);
    }

    /// What to call the level an export writes: the map's own name, or "level".
    pub fn level_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|path| path.file_stem())
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "level".to_string())
    }

    /// The whole document as `.map` text.
    ///
    /// What [`Self::save`] would write, without writing it: the export builds from a map
    /// file, and this is how it is handed one without a trip through the disk.
    pub fn to_map_text(&self) -> String {
        // Always the dialect that can hold everything, whatever flavour the document was
        // read as. This text is not a file anyone asked for: it is what the exporters,
        // the MCP server and the preview read the map back out of. Writing it in a
        // dialect with nowhere to put a face's own UV axes would flatten them on the way
        // to a Godot or a raylib level, which is work thrown away for having picked an
        // export target -- and the two round-trip tests in `map_model/tests/roundtrip.rs` are
        // there to say which dialect that is.
        write_map(&node::flatten_tree(
            &self.tree.world,
            map_model::map_format::MapFormat::LOSSLESS,
            None,
        ))
    }

    /// The document as `.map` text, without the layers marked as left out of exports.
    ///
    /// `Menu/File/Export/Map...`, which is what the layer switch is for: a layer of
    /// lighting hints or of notes to yourself stays in the map being edited and is left
    /// out of the one handed to a compiler. An omitted layer is written as an empty
    /// layer rather than dropped, so opening the export in this editor still shows the
    /// layers the map has.
    pub fn export_map_text(&self, format: map_model::map_format::MapFormat) -> String {
        let mut world = self.tree.world.clone();
        for layer in &mut world.layers {
            if layer.omit_from_export {
                layer.children.clear();
            }
        }
        write_map(&node::flatten_tree(&world, format, self.game.clone()))
    }

    /// Native Quake III brush primitives, using loaded texture dimensions.
    /// Faces with pending normalized matrices can be exported without textures.
    pub fn export_quake3_map_text(&self) -> Result<String, String> {
        let mut world = self.tree.world.clone();
        for layer in &mut world.layers {
            if layer.omit_from_export {
                layer.children.clear();
            }
        }
        let map = node::flatten_tree(
            &world,
            map_model::map_format::MapFormat::Quake2,
            self.game.clone(),
        );
        map_model::io::map_writer::write_quake3_map(&map, &mut |name| {
            self.materials
                .get(name)
                .map(|m| Vec2f::new(m.width as f32, m.height as f32))
        })
    }

    /// Writes a native Quake III map only after all UV conversions succeed.
    pub fn export_quake3_map_to(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let text = self
            .export_quake3_map_text()
            .map_err(|why| std::io::Error::new(std::io::ErrorKind::InvalidInput, why))?;
        crate::files::write(path.as_ref(), text)
    }

    /// Doom 3 Version 2 export with local brush planes and literal strings.
    pub fn export_doom3_map_text(&self) -> Result<String, String> {
        let mut world = self.tree.world.clone();
        for layer in &mut world.layers {
            if layer.omit_from_export {
                layer.children.clear();
            }
        }
        let map = node::flatten_tree(
            &world,
            map_model::map_format::MapFormat::Quake2,
            self.game.clone(),
        );
        map_model::io::map_writer::write_doom3_map(&map, &mut |name| {
            self.materials
                .get(name)
                .map(|m| Vec2f::new(m.width as f32, m.height as f32))
        })
    }

    /// Writes only after Doom 3 map validation succeeds.
    pub fn export_doom3_map_to(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let text = self
            .export_doom3_map_text()
            .map_err(|why| std::io::Error::new(std::io::ErrorKind::InvalidInput, why))?;
        crate::files::write(path.as_ref(), text)
    }

    /// Missing dimensions and Heron-only metadata relevant to native Quake III export.
    pub fn quake3_map_export_problems(&self) -> Vec<String> {
        let mut missing = std::collections::BTreeSet::new();
        let mut annotated = 0;
        for (_, brush, layer) in node::brushes_by_layer(&self.tree.world) {
            if self.tree.world.layers[layer].omit_from_export {
                continue;
            }
            for face in brush.faces() {
                if face.primitive_uv.is_none() && self.materials.get(&face.material_name).is_none()
                {
                    missing.insert(face.material_name.to_string());
                }
                if face.paint().is_some() || face.surface_material.is_some() || face.export_hidden {
                    annotated += 1;
                }
            }
        }
        let mut notes = Vec::new();
        if !missing.is_empty() {
            notes.push(format!(
                "Load textures before export; dimensions are unavailable for: {}",
                missing.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        if annotated > 0 {
            notes.push(format!("{annotated} faces carry paint, PBR or export visibility in Heron comments; Quake III tools ignore those comments"));
        }
        notes
    }

    /// Writes that text to a file.
    pub fn export_map_to(
        &self,
        path: impl AsRef<Path>,
        format: map_model::map_format::MapFormat,
    ) -> std::io::Result<()> {
        crate::files::write(path.as_ref(), self.export_map_text(format))
    }

    /// What a map would lose by being written in `format`, or nothing where it would
    /// lose nothing.
    ///
    /// The export window says this before it writes, because the answer depends on the
    /// map: a dialect with nowhere to put a face's UV axes costs nothing at all until
    /// some face actually carries some.
    pub fn what_a_dialect_would_drop(
        &self,
        format: map_model::map_format::MapFormat,
    ) -> Vec<String> {
        let mut lost = Vec::new();
        let faces = || {
            node::brushes_with_ids(&self.tree.world)
                .into_iter()
                .flat_map(|(_, brush)| brush.faces().iter())
        };

        if !format.has_explicit_uv_axes() {
            let carrying = faces()
                .filter(|face| {
                    matches!(
                        face.uv_coord_system,
                        map_model::uv::UvCoordSystem::Parallel { .. }
                    )
                })
                .count();
            if carrying > 0 {
                lost.push(format!(
                    "{carrying} faces carry their own UV axes, which this dialect cannot write; they come back aligned to their dominant plane"
                ));
            }
        }
        if !format.has_surface_attributes() {
            let flagged = faces()
                .filter(|face| {
                    face.surface_attributes.as_ref().is_some_and(|s| {
                        s.content_flags != 0 || s.surface_flags != 0 || s.value != 0
                    })
                })
                .count();
            if flagged > 0 {
                lost.push(format!(
                    "{flagged} faces carry surface or content flags, which this dialect has no room for"
                ));
            }
        }
        lost
    }

    /// Returns the three points defining a face's plane.
    ///
    /// This is what the clip tool takes when a face is double-clicked: cutting along a
    /// face that is already there is easier than placing three points on it.
    pub fn face_points(&self, face: FaceRef) -> Option<[Vec3d; 3]> {
        let node = node::find_node(&self.tree.world, face.node)?;
        let NodeKind::Brush(brush) = &node.kind else {
            return None;
        };
        Some(brush.faces().get(face.face)?.points)
    }

    /// The world the document's nodes hang from.
    ///
    /// Validation walks the tree in ways nothing else needs, so it reads it directly
    /// rather than growing the document an accessor per question.
    pub fn tree_world(&self) -> &map_model::node::World {
        &self.tree.world
    }

    /// Returns the brush stored under the given id, if that node is one.
    pub fn brush(&self, id: NodeId) -> Option<&Brush> {
        match &node::find_node(&self.tree.world, id)?.kind {
            NodeKind::Brush(brush) => Some(brush),
            _ => None,
        }
    }

    /// Returns the bounds every geometry rebuild is clipped against.
    #[inline]
    pub fn world_bounds(&self) -> Bbox3d {
        self.world_bounds
    }

    /// The bounds the map is meant to stay inside, from worldspawn's
    /// `_tb_soft_map_bounds`, or `None` when it does not set them.
    ///
    /// `Game::extractSoftMapBounds`. Six numbers -- min then max -- and anything else,
    /// including the empty string a map writes to say "no soft bounds", is `None`.
    pub fn soft_map_bounds(&self) -> Option<Bbox3d> {
        let text = self
            .tree_world()
            .properties
            .iter()
            .find(|(key, _)| key == map_model::entity_properties::keys::TB_SOFT_MAP_BOUNDS)
            .map(|(_, value)| value.as_str())?;

        let numbers: Vec<f64> = text
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<Result<_, _>>()
            .ok()?;
        if numbers.len() != 6 {
            return None;
        }
        Some(Bbox3d::new(
            Vec3d::new(numbers[0], numbers[1], numbers[2]),
            Vec3d::new(numbers[3], numbers[4], numbers[5]),
        ))
    }

    /// Returns the bounds new geometry is sized and placed against.
    ///
    /// This is `Map::referenceBounds`: whatever is selected, or failing that whatever was
    /// selected last, or failing that a box of 16 units either side of the origin.
    /// Remembering the last selection is what lets a brush drawn after deselecting still
    /// match the depth of the one before it.
    pub fn reference_bounds(&self) -> Bbox3d {
        self.selection_bounds()
            .or(self.last_selection_bounds)
            .unwrap_or_else(|| Bbox3d::from_scalars(-16.0, 16.0))
    }

    /// Writes the document back to the file it came from.
    ///
    /// Returns an error if the document has no path, since there is then nothing to write
    /// to; the caller asks for one and calls [`Document::save_as`] instead.
    pub fn save(&mut self) -> Result<(), SaveError> {
        let path = self.path.clone().ok_or(SaveError::NoPath)?;
        self.save_as(path)
    }

    /// Writes the document to the given path, and remembers it as the document's own.
    pub fn save_as(&mut self, path: impl Into<PathBuf>) -> Result<(), SaveError> {
        let path = path.into();
        let imported_refs = self.imported_references_for(&path).map_err(SaveError::Io)?;
        self.save_copy_to(&path).map_err(SaveError::Io)?;
        self.path = Some(path);
        self.adopt_imported_references(&imported_refs);
        self.modified = false;
        Ok(())
    }

    /// Writes the document to a path without adopting it as the document's own.
    ///
    /// `Map::saveTo`: the plain write the autosaver's numbered backups use. The document
    /// keeps pointing at the file it was opened from and stays marked modified.
    pub fn save_copy_to(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        // The lossless dialect, whatever the map was read as. A save is the editor's own
        // file and has to hold everything the editor knows; the flavour a level is
        // written *for* belongs to the export, where it can say what it drops. Saving a
        // Quake-lineage map therefore rewrites its face lines with explicit UV axes --
        // the same geometry and the same mapping, said in the dialect that can say all
        // of it. `format` is what the file was read as, and is now only the dialect the
        // export window starts on.
        let mut map = node::flatten_tree(
            &self.tree.world,
            map_model::map_format::MapFormat::LOSSLESS,
            self.game.clone(),
        );
        let refs = self.imported_references_for(path.as_ref())?;
        for entity in &mut map.entities {
            for (key, value) in &mut entity.properties {
                if key == "_heron_model" {
                    if let Some(new) = refs.get(value) {
                        *value = new.clone();
                    }
                }
            }
        }
        crate::files::write(
            path.as_ref(),
            format!(
                "{}{}",
                self.saved_material_paths(path.as_ref()),
                write_map(&map)
            ),
        )
    }

    /// The number of edits made to the document.
    #[inline]
    pub fn modification_count(&self) -> u64 {
        self.modification_count
    }

    /// Returns whether the document has changed since it was opened or last saved.
    #[inline]
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Returns everything the reader and the tree builder worked around, as text.
    ///
    /// A map that needed working around still opens, so the user is told rather than
    /// stopped.
    pub fn warnings(&self) -> Vec<String> {
        self.tree.warnings.iter().map(ToString::to_string).collect()
    }
}

#[path = "document/hollow.rs"]
mod hollow;
#[cfg(test)]
#[path = "document/tool_upgrade_tests.rs"]
mod tool_upgrade_tests;

#[cfg(test)]
#[path = "document/basic_workflow_tests.rs"]
mod basic_workflow_tests;
