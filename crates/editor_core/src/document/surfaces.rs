//! The 3D view's surfaces, kept per node.
//!
//! The preview used to be one triangle list for the whole map, rebuilt after every edit:
//! a drag step of a hundred brushes re-triangulated fifty thousand. Now each drawn brush,
//! patch and point entity keeps its own triangles ([`NodeSurface`]), and a rebuild only
//! triangulates the nodes an edit reached. What an edit reached is said by the edit path
//! itself ([`Document::mark_nodes_changed`], from the nodes an [`Edit`] names); every
//! other kind of change -- or a revision that moved without anyone saying why -- rebuilds
//! every node, which is what the old cache did on every change.
//!
//! Selection is not geometry: a selection change patches the `selected` flags of just the
//! nodes whose selection changed. Visibility (layers, hidden nodes, tags, filters) is
//! decided again by a walk of the tree on every rebuild, which is cheap next to
//! triangulating, so hiding a layer reuses every surface.
//!
//! Each rebuild appends the nodes it changed to a log, so a retained renderer
//! ([`crate::render_scene::SceneTracker`]) can ask what changed since it last looked
//! instead of comparing the whole map. In debug builds an incremental rebuild of a small
//! map is checked against a rebuild from nothing.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use map_model::node::{self, EntityNode, NodeId};
use render::Triangle;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    box_triangles, light_marker, model_triangles, surface_color, Document, EntityBox, FaceRef,
    PreviewOwner, BUILTIN_SPRITE_PREFIX, ENTITY_BOX_ALPHA, RENDER_CAMERA_CLASS,
};
use crate::clip_tool::ClipTool;
use crate::motion_preview::{Mesh, Motion};
use crate::view_filters::ViewFilters;
use geometry_math::vec::Vec2f;

/// One brush face's run of triangles in a [`NodeSurface`].
#[derive(Debug, Clone, PartialEq)]
struct FaceRun {
    range: Range<u32>,
    face: u32,
    /// A painted face is never tinted as selected: its colour is the point.
    painted: bool,
}

/// One node's part of the preview.
#[derive(Debug, Clone)]
pub struct NodeSurface {
    /// The node's triangles, `selected` flags included.
    pub triangles: Vec<Triangle>,
    /// For a brush, which face each run of triangles came from.
    faces: Vec<FaceRun>,
    /// Changes whenever the triangles change for any reason but the selection, so a
    /// renderer that keeps a copy can tell whether to send it again without comparing.
    pub stamp: u64,
    /// On a locked layer.
    pub locked: bool,
    /// An editor-only symbol (a light entity's bulb) that casts no shadow: it stands
    /// where the light is, so it would otherwise shadow its own light and the sun's.
    pub shadowless: bool,
    /// The walk that last found the node drawn; older ones are dropped after a walk.
    walk: u64,
}

/// Which surfaces an edit made wrong.
#[derive(Debug, Default)]
pub(super) enum Dirty {
    /// Any of them: rebuild every node.
    #[default]
    All,
    /// Only these nodes (and so far, nothing else).
    Nodes(FxHashSet<NodeId>),
}

/// What every surface depends on besides its own node: a change rebuilds them all.
#[derive(Debug, Clone, PartialEq)]
struct Context {
    generation: u64,
    materials: u64,
    models: u64,
    filters: ViewFilters,
    colours: [u32; 6],
}

/// The preview surfaces of a document, kept between rebuilds; see the module notes.
#[derive(Default)]
pub struct SurfaceCache {
    context: Option<Context>,
    /// The document revision the surfaces show.
    revision: Option<u64>,
    pub(super) dirty: Dirty,
    /// The revision at the last change that said what it changed. A revision past it
    /// moved without saying, and rebuilds everything.
    pub(super) dirty_at: u64,
    /// Something may have changed what is drawn (a node's presence, layer or visibility),
    /// so the next rebuild walks the tree. Without it (a selection change) the drawn set
    /// and its order stay as they are.
    pub(super) rewalk: bool,
    clip: Option<ClipTool>,
    fillet: Option<std::rc::Rc<crate::fillet_tool::SurfacePreview>>,
    /// Brushes drawn cut by `clip`.
    clipped: FxHashSet<NodeId>,
    selection: HashSet<NodeId>,
    faces: HashSet<FaceRef>,
    nodes: FxHashMap<NodeId, NodeSurface>,
    order: Vec<NodeId>,
    motions: HashMap<NodeId, Motion>,
    walk: u64,
    stamp: u64,
    epoch: u64,
    /// The nodes each rebuild changed (new, changed, gone, or selected differently), with
    /// the epoch of that rebuild, oldest first.
    log: Vec<(u64, NodeId)>,
    /// Changes before this epoch are no longer in `log`.
    log_floor: u64,
    /// Nodes the last rebuild triangulated, for tests and profiling.
    pub rebuilt: usize,
}

impl SurfaceCache {
    /// Every drawn node, in drawing order: brushes by layer, patches, point entities.
    pub fn order(&self) -> &[NodeId] {
        &self.order
    }

    /// A drawn node's surface.
    pub fn get(&self, id: NodeId) -> Option<&NodeSurface> {
        self.nodes.get(&id)
    }

    /// The animation preview's motions, by brush.
    pub fn motions(&self) -> &HashMap<NodeId, Motion> {
        &self.motions
    }

    /// Counts rebuilds that changed anything; see [`SurfaceCache::changes_since`].
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// The nodes whose surface, selection or presence changed after `epoch` (possibly
    /// more than once each), or `None` when that is too long ago to say and the caller
    /// has to look at every node.
    pub fn changes_since(&self, epoch: u64) -> Option<impl Iterator<Item = NodeId> + '_> {
        if epoch < self.log_floor || epoch > self.epoch {
            return None;
        }
        let start = self.log.partition_point(|(at, _)| *at <= epoch);
        Some(self.log[start..].iter().map(|(_, id)| *id))
    }

    /// The whole preview as one triangle list, with each run's owner and the motions'
    /// ranges in it: what the GL renderer uploads.
    pub fn mesh(&self) -> (Mesh, Vec<PreviewOwner>) {
        let total = self
            .order
            .iter()
            .map(|id| self.nodes[id].triangles.len())
            .sum();
        let mut triangles = Vec::with_capacity(total);
        let mut owners = Vec::with_capacity(self.order.len());
        let mut motions = Vec::new();
        for id in &self.order {
            let surface = &self.nodes[id];
            let start = triangles.len();
            triangles.extend_from_slice(&surface.triangles);
            if surface.faces.is_empty() {
                owners.push((start..triangles.len(), *id, None));
            } else {
                for run in &surface.faces {
                    owners.push((
                        start + run.range.start as usize..start + run.range.end as usize,
                        *id,
                        Some(run.face as usize),
                    ));
                }
            }
            if let Some(motion) = self.motions.get(id) {
                motions.push((start..triangles.len(), motion.clone()));
            }
        }
        (Mesh { triangles, motions }, owners)
    }

    /// [`SurfaceCache::mesh`] without the owners: what the GL renderer uploads, one copy
    /// of each node's triangles.
    pub fn triangles(&self) -> Mesh {
        let surfaces: Vec<(&NodeId, &NodeSurface)> =
            self.order.iter().map(|id| (id, &self.nodes[id])).collect();
        let mut triangles =
            Vec::with_capacity(surfaces.iter().map(|(_, s)| s.triangles.len()).sum());
        let mut motions = Vec::new();
        for (id, surface) in surfaces {
            let start = triangles.len();
            triangles.extend_from_slice(&surface.triangles);
            if !self.motions.is_empty() {
                if let Some(motion) = self.motions.get(id) {
                    motions.push((start..triangles.len(), motion.clone()));
                }
            }
        }
        Mesh { triangles, motions }
    }

    fn next_stamp(&mut self) -> u64 {
        self.stamp += 1;
        self.stamp
    }

    fn note(&mut self, id: NodeId) {
        self.log.push((self.epoch, id));
    }
}

/// Sets the `selected` flags of a surface as a fresh build would.
fn patch_selection(
    surface: &mut NodeSurface,
    id: NodeId,
    selection: &HashSet<NodeId>,
    faces: &HashSet<FaceRef>,
) {
    let selected = selection.contains(&id);
    if surface.faces.is_empty() {
        for triangle in &mut surface.triangles {
            triangle.selected = selected;
        }
        return;
    }
    for run in &surface.faces {
        let on = (selected
            || faces.contains(&FaceRef {
                node: id,
                face: run.face as usize,
            }))
            && !run.painted;
        for triangle in &mut surface.triangles[run.range.start as usize..run.range.end as usize] {
            triangle.selected = on;
        }
    }
}

impl Document {
    /// Calls `f` with the preview surfaces, brought up to date for `filters` and, with
    /// the clip tool's preview, `clip`.
    pub fn with_preview_surfaces<R>(
        &self,
        filters: &ViewFilters,
        clip: Option<&ClipTool>,
        f: impl FnOnce(&SurfaceCache) -> R,
    ) -> R {
        self.with_tool_preview_surfaces(filters, clip, None, f)
    }

    /// View-only fillet replacement surfaces. Exports and ordinary previews pass none.
    pub fn with_tool_preview_surfaces<R>(
        &self,
        filters: &ViewFilters,
        clip: Option<&ClipTool>,
        fillet: Option<&std::rc::Rc<crate::fillet_tool::SurfacePreview>>,
        f: impl FnOnce(&SurfaceCache) -> R,
    ) -> R {
        let clip = clip.filter(|clip| clip.can_clip());
        let mut cache = self.surface_cache.borrow_mut();
        self.refresh_surfaces(&mut cache, filters, clip, fillet);
        f(&cache)
    }

    /// Says that the nodes `ids` changed (and nothing else), so the next rebuild of the
    /// preview triangulates only them; then what [`Document::mark_changed`] does.
    pub(super) fn mark_nodes_changed(&mut self, ids: impl IntoIterator<Item = NodeId>) {
        let cache = self.surface_cache.get_mut();
        if let Dirty::Nodes(dirty) = &mut cache.dirty {
            dirty.extend(ids);
        }
        cache.rewalk = true;
        self.note_changed();
    }

    /// [`Document::mark_changed`] for a change of what is shown rather than of any
    /// node: hiding, showing, locking. The surfaces are kept; the next rebuild's walk of
    /// the tree decides again what is drawn.
    pub(super) fn mark_visibility_changed(&mut self) {
        self.surface_cache.get_mut().rewalk = true;
        self.note_changed();
    }

    fn surface_context(&self, filters: &ViewFilters) -> Context {
        let face = crate::preferences::pref(|p| p.face_color).rgb_f32();
        let entity = crate::preferences::pref(|p| p.undefined_entity_color).rgb_f32();
        Context {
            generation: self.generation,
            materials: self.materials_revision,
            models: self.models_fingerprint(),
            filters: *filters,
            colours: [face[0], face[1], face[2], entity[0], entity[1], entity[2]].map(f32::to_bits),
        }
    }

    /// Changes when the loaded entity models or sprites do: they are what point entities
    /// are drawn as.
    fn models_fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut models: Vec<(&String, usize)> = self
            .models
            .iter()
            .map(|(k, m)| (k, std::rc::Rc::as_ptr(m) as usize))
            .collect();
        models.sort_unstable();
        let mut sprites: Vec<_> = self.sprites.iter().collect();
        sprites.sort_unstable();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        models.hash(&mut hasher);
        sprites.hash(&mut hasher);
        hasher.finish()
    }

    /// Brings `cache` up to date.
    fn refresh_surfaces(
        &self,
        cache: &mut SurfaceCache,
        filters: &ViewFilters,
        clip: Option<&ClipTool>,
        fillet: Option<&std::rc::Rc<crate::fillet_tool::SurfacePreview>>,
    ) {
        let filters = filters.geometry_only();
        let context = self.surface_context(&filters);
        let clip_owned = clip.cloned();
        let fillet_changed = match (cache.fillet.as_ref(), fillet) {
            (None, None) => false,
            (Some(a), Some(b)) => !std::rc::Rc::ptr_eq(a, b),
            _ => true,
        };
        if !fillet_changed
            && cache.revision == Some(self.revision)
            && cache.context.as_ref() == Some(&context)
            && cache.clip == clip_owned
        {
            return;
        }
        let incremental = cache.context.as_ref() == Some(&context);
        if !incremental {
            // Nothing kept is right any more; a renderer following the log starts over.
            cache.nodes.clear();
            cache.order.clear();
            cache.log.clear();
            cache.log_floor = cache.epoch + 1;
            cache.dirty = Dirty::All;
        } else if cache.dirty_at != self.revision {
            cache.dirty = Dirty::All;
        }
        cache.epoch += 1;
        cache.rebuilt = 0;
        let all = matches!(cache.dirty, Dirty::All);
        let mut dirty =
            match std::mem::replace(&mut cache.dirty, Dirty::Nodes(FxHashSet::default())) {
                Dirty::All => FxHashSet::default(),
                Dirty::Nodes(ids) => ids,
            };
        let rewalk = std::mem::take(&mut cache.rewalk);
        if fillet_changed {
            for preview in cache.fillet.iter().chain(fillet) {
                dirty.extend(preview.brushes.iter().map(|(id, _)| *id));
            }
        }

        // The clip preview cuts the selected brushes: when it changes, they and the ones
        // it cut before are built again.
        if cache.clip != clip_owned {
            dirty.extend(cache.clipped.iter().copied());
            if clip.is_some() {
                dirty.extend(self.selection.iter().copied());
            }
        }

        // The selection: patch the flags of the nodes it changed for, or with the clip
        // preview on (which follows the selection), build them again. Logged either way:
        // a renderer keeps the selection as object state, which changes even for a
        // brush whose faces are all painted and so never tinted.
        let mut changed: Vec<NodeId> = cache
            .selection
            .symmetric_difference(&self.selection)
            .copied()
            .collect();
        changed.extend(
            cache
                .faces
                .symmetric_difference(&self.face_selection)
                .map(|face| face.node),
        );
        for id in changed {
            if clip.is_some() {
                dirty.insert(id);
            } else if let Some(surface) = cache.nodes.get_mut(&id).filter(|_| !all) {
                patch_selection(surface, id, &self.selection, &self.face_selection);
            }
            cache.log.push((cache.epoch, id));
        }
        if cache.selection != self.selection {
            cache.selection.clone_from(&self.selection);
        }
        if cache.faces != self.face_selection {
            cache.faces.clone_from(&self.face_selection);
        }

        // A change of the selection alone leaves what is drawn, and its order, as it was.
        if all || rewalk || !dirty.is_empty() || cache.clip != clip_owned {
            cache.walk += 1;
            let motions_stale = all
                || dirty.is_empty()
                || dirty
                    .iter()
                    .any(|id| !cache.nodes.contains_key(id) || cache.motions.contains_key(id));
            // The walk: what is drawn, in order. Kept surfaces are taken as they are; brushes
            // to build are collected and built afterwards, on all cores when there are many
            // (a drag of thousands), patches and entities as they come.
            let walk = cache.walk;
            let mut order = Vec::with_capacity(cache.order.len());
            let mut clipped = FxHashSet::default();
            let mut jobs: Vec<(
                NodeId,
                &map_model::brush::Brush,
                bool,
                bool,
                Option<&ClipTool>,
            )> = Vec::new();
            // Whether a node has to be built; if not, its kept surface is brought up to date
            // with its layer's lock and marked drawn.
            let keep = |cache: &mut SurfaceCache, id: NodeId, locked: bool| -> bool {
                if all || dirty.contains(&id) {
                    return false;
                }
                let stamp = cache.stamp + 1;
                let Some(surface) = cache.nodes.get_mut(&id) else {
                    return false;
                };
                surface.walk = walk;
                if surface.locked != locked {
                    surface.locked = locked;
                    for triangle in &mut surface.triangles {
                        triangle.locked = locked;
                    }
                    surface.stamp = stamp;
                    cache.stamp = stamp;
                    cache.note(id);
                }
                true
            };

            if filters.show_brushes {
                let hidden_by_tag = self.brushes_hidden_by_tags();
                let shown_through = self.brushes_shown_through();
                for (id, brush, layer) in node::brushes_by_layer(&self.tree.world) {
                    if !self.node_is_drawn(id, layer)
                        || (!hidden_by_tag.is_empty() && hidden_by_tag.contains(&id))
                    {
                        continue;
                    }
                    let locked = !self.layer_is_editable(layer);
                    let cut = clip.filter(|_| self.selection.contains(&id));
                    if cut.is_some() {
                        clipped.insert(id);
                    }
                    let see_through = shown_through.contains(&id);
                    if !keep(cache, id, locked) {
                        if let Some((_, replacements)) = fillet.and_then(|preview| {
                            preview.brushes.iter().find(|(node, _)| *node == id)
                        }) {
                            for replacement in replacements {
                                jobs.push((id, replacement, locked, see_through, None));
                            }
                        } else {
                            jobs.push((id, brush, locked, see_through, cut));
                        }
                    }
                    order.push(id);
                }
            }
            if filters.show_patches {
                for (id, patch, layer) in node::patches_by_layer(&self.tree.world) {
                    if !self.node_is_drawn(id, layer) {
                        continue;
                    }
                    let locked = !self.layer_is_editable(layer);
                    if !keep(cache, id, locked) {
                        let (triangles, faces) = self.patch_surface(id, patch, locked, &filters);
                        commit(cache, id, locked, triangles, faces);
                    }
                    order.push(id);
                }
            }
            if filters.show_point_entities {
                for (id, entity, layer) in node::point_entities_by_layer(&self.tree.world) {
                    if !self.node_is_drawn(id, layer)
                        || !self.entity_is_drawn_as_surface(entity, &filters)
                    {
                        continue;
                    }
                    let locked = !self.layer_is_editable(layer);
                    if !keep(cache, id, locked) {
                        let Some((triangles, faces, shadowless)) =
                            self.entity_surface(id, entity, locked, &filters)
                        else {
                            continue;
                        };
                        commit(cache, id, locked, triangles, faces);
                        if let Some(surface) = cache.nodes.get_mut(&id) {
                            surface.shadowless = shadowless;
                        }
                    }
                    order.push(id);
                }
            }

            let inputs = BrushInputs {
                selection: &self.selection,
                faces: &self.face_selection,
                materials: &self.materials,
                filters,
                face_color: crate::preferences::pref(|p| p.face_color).rgb_f32(),
                world_bounds: self.world_bounds(),
                see_through_faces: self
                    .face_tags
                    .iter()
                    .filter(|tag| tag.transparent)
                    .collect(),
            };
            // Face tags ask the document, which cannot be shared between threads; with none
            // hidden (the usual case) a face is never hidden by one.
            let built: Vec<(Vec<Triangle>, Vec<FaceRun>)> = if jobs.len() >= PARALLEL_BUILDS
                && self.hidden_tags.is_empty()
            {
                use rayon::prelude::*;
                jobs.par_iter()
                    .map(|(id, brush, locked, see_through, cut)| {
                        brush_surface(&inputs, *id, brush, *locked, *see_through, *cut, |_| false)
                    })
                    .collect()
            } else {
                jobs.iter()
                    .map(|(id, brush, locked, see_through, cut)| {
                        brush_surface(&inputs, *id, brush, *locked, *see_through, *cut, |face| {
                            self.face_hidden_by_tag(face)
                        })
                    })
                    .collect()
            };
            let mut combined: Vec<(NodeId, bool, Vec<Triangle>, Vec<FaceRun>)> = Vec::new();
            for ((id, _, locked, _, _), (triangles, mut faces)) in jobs.iter().zip(built) {
                if let Some((_, _, out, runs)) =
                    combined.last_mut().filter(|(last, _, _, _)| last == id)
                {
                    let offset = out.len() as u32;
                    for face in &mut faces {
                        face.range.start += offset;
                        face.range.end += offset;
                    }
                    out.extend(triangles);
                    runs.extend(faces);
                } else {
                    combined.push((*id, *locked, triangles, faces));
                }
            }
            for (id, locked, triangles, faces) in combined {
                commit(cache, id, locked, triangles, faces);
            }

            // Whatever the walk did not reach is no longer drawn: drop it, freeing its memory.
            let mut gone = Vec::new();
            cache.nodes.retain(|id, surface| {
                let keep = surface.walk == walk;
                if !keep {
                    gone.push(*id);
                }
                keep
            });
            for id in gone {
                cache.note(id);
            }
            cache.order = order;
            cache.clipped = clipped;
            cache.clip = clip_owned;
            cache.fillet = fillet.cloned();
            // Motions belong to brushes in animated entities, and their pivots to all of an
            // entity's brushes: an edit of brushes that were drawn before, none of them
            // animated, leaves them as they were. Anything else asks again.
            if motions_stale {
                cache.motions = self.preview_motions();
            }
            cache.motions.retain(|id, _| cache.nodes.contains_key(id));
        }
        cache.revision = Some(self.revision);
        cache.context = Some(context);
        cache.dirty_at = self.revision;

        // Keep the log about as long as the map; a renderer that fell further behind
        // compares every node instead.
        if cache.log.len() > 4096.max(cache.nodes.len() * 2) {
            cache.log.clear();
            cache.log_floor = cache.epoch;
        }

        #[cfg(debug_assertions)]
        if incremental && cache.nodes.len() <= 4096 {
            let mut fresh = SurfaceCache::default();
            self.refresh_surfaces(&mut fresh, &filters, clip, fillet);
            let (mesh, owners) = cache.mesh();
            let (expected, expected_owners) = fresh.mesh();
            assert!(
                mesh.triangles == expected.triangles
                    && owners == expected_owners
                    && mesh
                        .motions
                        .iter()
                        .map(|m| &m.0)
                        .eq(expected.motions.iter().map(|m| &m.0)),
                "the incrementally rebuilt preview differs from a rebuild from nothing"
            );
        }
    }

    /// Whether a point entity is drawn as triangles: not a render camera (an editing
    /// helper that would enclose the camera), not a class drawn as the game's own sprite
    /// by the billboard pass -- the box would show through the sprite from the side. The
    /// editor's built-in icons keep their see-through box, as they keep their outline in
    /// the flat views: the icon alone is a small mark easily lost.
    fn entity_is_drawn_as_surface(&self, entity: &EntityNode, filters: &ViewFilters) -> bool {
        if entity.classname() == Some(RENDER_CAMERA_CLASS) {
            return false;
        }
        let game_sprite = entity
            .classname()
            .and_then(|classname| self.sprites.get(classname))
            .is_some_and(|(material, _, _)| !material.starts_with(BUILTIN_SPRITE_PREFIX));
        !(filters.show_point_entity_models && game_sprite)
    }

    /// Whether a point entity is drawn as its box, having no model or light symbol to show.
    fn entity_is_drawn_as_box(&self, entity: &EntityNode, filters: &ViewFilters) -> bool {
        !filters.show_point_entity_models
            || (entity.classname() != Some("light") && self.model_of_entity(entity).is_none())
    }

    /// The point entities the 3D view draws as boxes, for the outlines it draws round them
    /// as the flat views do.
    pub fn entity_box_outlines(&self, filters: &ViewFilters) -> Vec<EntityBox> {
        if !filters.show_point_entities {
            return Vec::new();
        }
        node::point_entities_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, entity, layer)| {
                self.node_is_drawn(*id, *layer)
                    && self.entity_is_drawn_as_surface(entity, filters)
                    && self.entity_is_drawn_as_box(entity, filters)
            })
            .filter_map(|(id, entity, _)| self.entity_box_of(id, entity))
            .collect()
    }

    /// A patch, tessellated. A patch is a surface rather than a solid, so it is drawn
    /// from both sides.
    fn patch_surface(
        &self,
        id: NodeId,
        patch: &map_model::io::map_reader::Patch,
        locked: bool,
        filters: &ViewFilters,
    ) -> (Vec<Triangle>, Vec<FaceRun>) {
        let selected = self.selection.contains(&id);
        let material = filters
            .textured_faces
            .then(|| self.materials.index_of(&patch.material_name))
            .flatten();
        // A patch's UV coordinates count whole copies of the material; a face's count
        // texels, and the shader divides by the material's size because of that. So a
        // patch's are turned into texels here, or the material came out stretched across
        // the whole surface however large it is.
        let size = self
            .materials
            .get(&patch.material_name)
            .map(|m| (m.width.max(1) as f32, m.height.max(1) as f32))
            .unwrap_or((1.0, 1.0));
        let triangles =
            map_model::patch_mesh::tessellate(patch, map_model::patch_mesh::DEFAULT_SUBDIVISIONS)
                .into_iter()
                .map(|triangle| Triangle {
                    corner_normals: None,
                    surface_material: None,
                    positions: triangle.positions,
                    normal: triangle.normal,
                    uv: triangle
                        .uv
                        .map(|uv| Vec2f::new(uv.x() * size.0, uv.y() * size.1)),
                    material,
                    color: surface_color(material),
                    alpha: 1.0,
                    corner_colors: None,
                    locked,
                    selected,
                    painted: false,
                })
                .collect();
        (triangles, Vec::new())
    }

    /// A point entity: the model its class names, where there is one to draw, and
    /// otherwise the box, in the class's own colour, which is what says one entity from
    /// another when there is no model. `None` for one without an origin. The flag says
    /// the triangles are an editor-only symbol that casts no shadow (a light's bulb).
    fn entity_surface(
        &self,
        id: NodeId,
        entity: &EntityNode,
        locked: bool,
        filters: &ViewFilters,
    ) -> Option<(Vec<Triangle>, Vec<FaceRun>, bool)> {
        let entity_box = self.entity_box_of(id, entity)?;
        let color = entity_box.color.map(|channel| f32::from(channel) / 255.0);
        let model = filters
            .show_point_entity_models
            .then(|| self.model_of_entity(entity))
            .flatten();
        let mut shadowless = false;
        let triangles = match model {
            Some((model, material, scale)) => model_triangles(
                &model,
                material,
                Self::entity_placement_of(entity),
                scale,
                entity_box.selected,
                locked,
            ),
            None => filters
                .show_point_entity_models
                .then(|| {
                    light_marker::triangles(entity, &entity_box.bounds, entity_box.selected, locked)
                })
                .flatten()
                .inspect(|_| shadowless = true)
                .unwrap_or_else(|| {
                    box_triangles(
                        &entity_box.bounds,
                        color,
                        ENTITY_BOX_ALPHA,
                        entity_box.selected,
                        locked,
                    )
                }),
        };
        Some((triangles, Vec::new(), shadowless))
    }
}

/// From this many brushes up, a rebuild builds them on all cores.
const PARALLEL_BUILDS: usize = 64;

/// What building a brush's surface reads from the document, in a form worker threads
/// can share.
struct BrushInputs<'a> {
    selection: &'a HashSet<NodeId>,
    faces: &'a HashSet<FaceRef>,
    materials: &'a map_model::material::MaterialCollection,
    filters: ViewFilters,
    face_color: [f32; 3],
    world_bounds: geometry_math::bbox::Bbox3d,
    /// The face tags marked `transparent`, such as clip and origin.
    see_through_faces: Vec<&'a crate::games::Tag>,
}

/// How opaque a brush or face tagged `transparent` is drawn: a trigger, a clip face.
/// TrenchBroom's "Transparent face alpha" default.
pub(super) const SEE_THROUGH_ALPHA: f32 = 0.4;

/// Puts a freshly built surface into the cache, keeping the old stamp when it came out
/// the same (an undo, a hide and show, an edit that changed nothing drawn).
fn commit(
    cache: &mut SurfaceCache,
    id: NodeId,
    locked: bool,
    triangles: Vec<Triangle>,
    faces: Vec<FaceRun>,
) {
    let walk = cache.walk;
    cache.rebuilt += 1;
    match cache.nodes.get_mut(&id) {
        Some(old) if old.triangles == triangles && old.faces == faces && old.walk + 1 == walk => {
            old.walk = walk;
            old.locked = locked;
        }
        _ => {
            let stamp = cache.next_stamp();
            cache.nodes.insert(
                id,
                NodeSurface {
                    triangles,
                    faces,
                    stamp,
                    locked,
                    shadowless: false,
                    walk,
                },
            );
            cache.note(id);
        }
    }
}

/// A brush's surface: each face fanned into triangles, wearing its material or paint.
/// `see_through` draws the whole brush see-through, as a face tagged `transparent` is.
#[allow(clippy::too_many_arguments)]
fn brush_surface(
    inputs: &BrushInputs,
    id: NodeId,
    brush: &map_model::brush::Brush,
    locked: bool,
    see_through: bool,
    clip: Option<&ClipTool>,
    hidden_by_tag: impl Fn(&map_model::brush_face::BrushFace) -> bool,
) -> (Vec<Triangle>, Vec<FaceRun>) {
    let filters = &inputs.filters;
    let mut result = Vec::new();
    let mut runs = Vec::new();
    let brush_selected = inputs.selection.contains(&id);
    let halves: Vec<_> = if let Some(clip) = clip {
        clip.clip_brush(&inputs.world_bounds, brush)
            .into_iter()
            .map(|b| (std::borrow::Cow::Owned(b), 1.0))
            .chain(
                clip.clipped_away(brush)
                    .into_iter()
                    .map(|b| (std::borrow::Cow::Owned(b), 0.25)),
            )
            .collect()
    } else {
        vec![(std::borrow::Cow::Borrowed(brush), 1.0)]
    };
    for (brush, opacity) in halves {
        brush.for_each_geometry_face(|index, face, polygon| {
            if hidden_by_tag(face) || (!filters.show_export_hidden && face.export_hidden) {
                return;
            }
            let selected = brush_selected
                || inputs.faces.contains(&FaceRef {
                    node: id,
                    face: index,
                });
            let normal = face.normal();
            // A painted face is drawn in its colour instead of its material: a solid
            // colour that a material still showed through would not be a solid
            // colour. What it wears is kept, so taking the paint off puts it back.
            // Skip is an editor helper, even when old paint metadata remains.
            let is_skip = face
                .material_name
                .rsplit('/')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("skip"));
            let paint = if is_skip { None } else { face.paint() };
            let material = if is_skip {
                Some(render::SKIP_PREVIEW_MATERIAL)
            } else {
                match paint {
                    Some(_) => None,
                    None => filters
                        .textured_faces
                        .then(|| inputs.materials.index_of(&face.material_name))
                        .flatten(),
                }
            };
            let gradient = paint
                .filter(|p| !matches!(p, map_model::brush_face::FaceColor::Solid { .. }))
                .map(|p| map_model::brush_face::PaintOnFace::new(face, polygon, p));
            let color = match paint {
                Some(map_model::brush_face::FaceColor::Solid { color, .. }) => color.to_f32(),
                // A gradient's colour comes per corner; this stands in for anything
                // that draws the surface without them.
                Some(map_model::brush_face::FaceColor::Gradient { from, .. }) => from.to_f32(),
                Some(map_model::brush_face::FaceColor::MultiGradient { stops, .. }) => {
                    stops.at(0.0)
                }
                None if is_skip => [1.0, 1.0, 1.0],
                // `surface_color`, with the preference read once on the calling thread.
                None => {
                    if material.is_some() {
                        [1.0, 1.0, 1.0]
                    } else {
                        inputs.face_color
                    }
                }
            };
            let alpha = if is_skip {
                0.75
            } else {
                paint.map_or(1.0, |c| f32::from(c.alpha()) / 255.0)
            };
            // Skip has a look of its own, hatched.
            let shown_through = !is_skip
                && (see_through
                    || inputs
                        .see_through_faces
                        .iter()
                        .any(|tag| super::tags::face_carries(tag, face, inputs.materials)));
            let alpha = if shown_through {
                alpha.min(SEE_THROUGH_ALPHA)
            } else {
                alpha
            };
            let face_start = result.len();
            for corners in map_model::brush_face::paint_triangles(face, polygon, paint) {
                result.push(Triangle {
                    corner_normals: None,
                    surface_material: if is_skip { None } else { face.surface_material },
                    positions: corners,
                    normal,
                    uv: corners.map(|corner| face.uv_coords(corner)),
                    material,
                    color,
                    alpha: alpha * opacity,
                    corner_colors: gradient
                        .as_ref()
                        .map(|gradient| corners.map(|corner| gradient.at(face, corner))),
                    locked,
                    selected: selected && paint.is_none(),
                    painted: paint.is_some(),
                });
            }
            runs.push(FaceRun {
                range: face_start as u32..result.len() as u32,
                face: index as u32,
                painted: paint.is_some(),
            });
        });
    }
    (result, runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;
    use map_model::brush_face::{FaceColor, Rgb};

    fn boxes(count: usize) -> Document {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        for i in 0..count {
            let x = i as f64 * 128.0;
            let brush = map_model::shapes::create_cuboid(
                &world,
                &Bbox3d::new(Vec3d::new(x, 0.0, 0.0), Vec3d::new(x + 64.0, 64.0, 64.0)),
                "stone",
            )
            .unwrap();
            document.add_brushes(vec![brush]);
        }
        document.clear_selection();
        document
    }

    #[test]
    fn fillet_solid_preview_matches_apply_and_restores_without_editing() {
        use crate::fillet_tool::{surface_preview, FilletParameters};
        let mut document = boxes(3);
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        let edges = vec![document.brush(id).unwrap().edge_positions()[0]];
        let settings = FilletParameters::default();
        let filters = ViewFilters::default();
        let original = document.preview_mesh(&filters).triangles;
        let saved = document.to_map_text();
        let revision = document.revision();
        let preview = std::rc::Rc::new(surface_preview(&document, &edges, settings).unwrap());
        let shown = document
            .with_tool_preview_surfaces(&filters, None, Some(&preview), |s| s.mesh().0.triangles);
        assert_ne!(shown, original);
        assert_eq!(
            rebuilt(&document),
            1,
            "only the affected brush is triangulated"
        );
        let epoch =
            document.with_tool_preview_surfaces(&filters, None, Some(&preview), |s| s.epoch());
        assert_eq!(
            epoch,
            document.with_tool_preview_surfaces(&filters, None, Some(&preview), |s| s.epoch())
        );
        assert_eq!(document.to_map_text(), saved);
        assert_eq!(document.revision(), revision);
        assert_eq!(
            document.preview_mesh(&filters).triangles,
            original,
            "ordinary preview/export must show saved geometry"
        );
        assert_eq!(rebuilt(&document), 1);
        document.fillet_edges(&edges, &settings.shape()).unwrap();
        assert_eq!(
            shown,
            document.preview_mesh(&filters).triangles,
            "preview surfaces are precisely what Apply produces"
        );
        document.undo().unwrap();
        assert_eq!(document.to_map_text(), saved);
    }

    #[test]
    fn fillet_face_loop_preview_matches_apply() {
        let mut document = boxes(1);
        let id = document.selectable_brushes()[0].0;
        document.select_face_only(FaceRef { node: id, face: 0 });
        let settings = crate::fillet_tool::FilletParameters::default();
        let preview = std::rc::Rc::new(
            crate::fillet_tool::surface_preview(&document, &[], settings).unwrap(),
        );
        assert_eq!(preview.brushes.len(), 1);
        let filters = ViewFilters::default();
        let shown = document
            .with_tool_preview_surfaces(&filters, None, Some(&preview), |s| s.mesh().0.triangles);
        document.fillet_face_loops(&settings.shape()).unwrap();
        assert_eq!(shown, document.preview_mesh(&filters).triangles);
    }

    #[test]
    fn inner_fillet_solid_preview_contains_the_added_brushes() {
        use crate::fillet_tool::{surface_preview, FilletParameters};
        let mut document = Document::new(WorldBounds::default());
        for (min, max) in [
            (Vec3d::zero(), Vec3d::new(128.0, 16.0, 32.0)),
            (Vec3d::zero(), Vec3d::new(24.0, 96.0, 32.0)),
        ] {
            let brush = map_model::shapes::create_cuboid(
                &document.world_bounds(),
                &Bbox3d::new(min, max),
                "stone",
            )
            .unwrap();
            document.add_brushes(vec![brush]);
        }
        document.select_all();
        let settings = FilletParameters {
            inner: true,
            radius: 24.0,
            ..Default::default()
        };
        let preview = std::rc::Rc::new(surface_preview(&document, &[], settings).unwrap());
        assert!(preview.brushes.iter().map(|(_, b)| b.len()).sum::<usize>() > 2);
        let filters = ViewFilters::default();
        let shown = document
            .with_tool_preview_surfaces(&filters, None, Some(&preview), |s| s.mesh().0.triangles);
        let revision = document.revision();
        document
            .join_corner_options(
                revision,
                crate::corner_join::Rounding {
                    inner: Some((settings.radius, settings.faces)),
                    ..Default::default()
                },
            )
            .unwrap();
        let applied = document.preview_mesh(&filters).triangles;
        // Added brushes share their source owner's render ID only during the preview.
        assert_eq!(shown.len(), applied.len());
        for triangle in shown {
            assert!(applied.contains(&triangle));
        }
    }

    /// The kept surfaces, as one mesh, equal a rebuild from nothing.
    fn assert_fresh(document: &Document, filters: &ViewFilters, clip: Option<&ClipTool>) {
        let (mesh, owners) = document.with_preview_surfaces(filters, clip, SurfaceCache::mesh);
        let mut fresh = SurfaceCache::default();
        document.refresh_surfaces(
            &mut fresh,
            &filters.geometry_only(),
            clip.filter(|c| c.can_clip()),
            None,
        );
        let (expected, expected_owners) = fresh.mesh();
        assert_eq!(mesh.triangles.len(), expected.triangles.len());
        assert!(mesh.triangles == expected.triangles, "triangles differ");
        assert_eq!(owners, expected_owners);
        assert_eq!(mesh.motions.len(), expected.motions.len());
    }

    fn rebuilt(document: &Document) -> usize {
        document.surface_cache.borrow().rebuilt
    }

    #[test]
    fn a_drag_triangulates_only_what_it_moves() {
        let mut document = boxes(20);
        let filters = ViewFilters::default();
        assert_fresh(&document, &filters, None);
        let ids: Vec<NodeId> = document
            .selectable_brushes()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        document.select_only(ids[3]);
        document.toggle_selected(ids[7]);
        assert_fresh(&document, &filters, None);
        assert_eq!(rebuilt(&document), 0, "a selection change patches flags");
        document.begin_transaction();
        for step in 1..=3 {
            document.rollback_transaction();
            assert!(document.translate_selection(Vec3d::new(16.0 * f64::from(step), 0.0, 0.0)));
            assert_fresh(&document, &filters, None);
            assert_eq!(rebuilt(&document), 2, "step {step}");
        }
        document.commit_transaction();
        assert_fresh(&document, &filters, None);
        document.undo();
        assert_fresh(&document, &filters, None);
        assert_eq!(rebuilt(&document), 2);
        document.redo();
        assert_fresh(&document, &filters, None);
        // Something that does not say what it changed rebuilds everything.
        document.mark_changed();
        assert_fresh(&document, &filters, None);
        assert_eq!(rebuilt(&document), 20);
    }

    #[test]
    fn kept_surfaces_follow_paint_faces_layers_hiding_filters_and_clip() {
        let mut document = boxes(8);
        let mut filters = ViewFilters::default();
        let ids: Vec<NodeId> = document
            .selectable_brushes()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_fresh(&document, &filters, None);

        // Paint: a solid colour and a gradient; then a selection over painted faces, and
        // a face selection on a painted brush.
        document.select_only(ids[0]);
        document.set_face_color(Some(FaceColor::solid(Rgb {
            r: 200,
            g: 10,
            b: 10,
        })));
        assert_fresh(&document, &filters, None);
        document.select_only(ids[1]);
        document.set_face_color(Some(FaceColor::gradient(
            Rgb { r: 0, g: 0, b: 0 },
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            30.0,
        )));
        assert_fresh(&document, &filters, None);
        document.select_only(ids[0]);
        assert_fresh(&document, &filters, None);
        document.select_face_only(FaceRef {
            node: ids[2],
            face: 1,
        });
        assert_fresh(&document, &filters, None);
        document.select_face_only(FaceRef {
            node: ids[0],
            face: 2,
        });
        assert_fresh(&document, &filters, None);
        document.clear_selection();
        assert_fresh(&document, &filters, None);

        // Layers: move two brushes onto a new layer, hide it, lock it.
        let layer = document.add_layer("second");
        document.select_only(ids[4]);
        document.toggle_selected(ids[5]);
        document.move_selection_to_layer(layer);
        assert_fresh(&document, &filters, None);
        document.set_layer_hidden(layer, true);
        assert_fresh(&document, &filters, None);
        assert_eq!(rebuilt(&document), 0, "hiding a layer triangulates nothing");
        document.set_layer_hidden(layer, false);
        assert_fresh(&document, &filters, None);
        document.set_layer_locked(layer, true);
        assert_fresh(&document, &filters, None);
        document.set_layer_locked(layer, false);
        assert_fresh(&document, &filters, None);

        // Hide and isolate.
        document.select_only(ids[6]);
        document.hide_selection();
        assert_fresh(&document, &filters, None);
        document.select_only(ids[7]);
        document.isolate_selection();
        assert_fresh(&document, &filters, None);
        document.show_all();
        assert_fresh(&document, &filters, None);

        // A point entity, moved.
        let light = document.create_point_entity("light", Vec3d::new(0.0, 256.0, 32.0));
        assert_fresh(&document, &filters, None);
        document.select_only(light);
        assert!(document.translate_selection(Vec3d::new(32.0, 0.0, 0.0)));
        assert_fresh(&document, &filters, None);

        // Filters.
        filters.textured_faces = !filters.textured_faces;
        assert_fresh(&document, &filters, None);
        filters.show_point_entities = false;
        assert_fresh(&document, &filters, None);
        filters.show_point_entities = true;

        // The clip preview cuts the selection, follows it, and goes away again.
        document.select_only(ids[3]);
        let mut clip = ClipTool::new();
        clip.set_face([
            Vec3d::new(416.0, 0.0, 0.0),
            Vec3d::new(416.0, 64.0, 0.0),
            Vec3d::new(416.0, 0.0, 64.0),
        ]);
        assert!(clip.can_clip());
        assert_fresh(&document, &filters, Some(&clip));
        document.select_only(ids[2]);
        assert_fresh(&document, &filters, Some(&clip));
        assert_fresh(&document, &filters, None);
    }

    #[test]
    fn selecting_on_an_opened_map_triangulates_nothing() {
        let path = format!("{}/../../assets/crate.map", env!("CARGO_MANIFEST_DIR"));
        let mut document = Document::open(&path, WorldBounds::default()).unwrap();
        let filters = ViewFilters::default();
        assert_fresh(&document, &filters, None);
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        assert_fresh(&document, &filters, None);
        assert_eq!(rebuilt(&document), 0);
    }

    #[test]
    fn the_log_says_what_changed() {
        let mut document = boxes(5);
        let filters = ViewFilters::default();
        let ids: Vec<NodeId> = document
            .selectable_brushes()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        document.select_only(ids[1]);
        let epoch = document.with_preview_surfaces(&filters, None, SurfaceCache::epoch);
        assert!(document.translate_selection(Vec3d::new(8.0, 0.0, 0.0)));
        let changed: HashSet<NodeId> = document.with_preview_surfaces(&filters, None, |s| {
            s.changes_since(epoch).unwrap().collect()
        });
        assert_eq!(changed, HashSet::from([ids[1]]));
    }
}
