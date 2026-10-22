//! What the 3D view draws, said without a GPU API: per-object meshes with dirty
//! tracking, fed to a retained renderer as deltas.
//!
//! The GL renderer in `render` takes the whole map as one triangle list and is handed a
//! new one on every change. A retained renderer (`render_wgpu`) keeps one mesh per object
//! instead, and only wants to hear what changed: [`SceneTracker::sync`] tells a
//! [`SceneSink`] which objects appeared, changed, went away, or only changed state
//! (selected, locked). A selection click on a brush is then a few bytes of state, and a
//! drag re-sends only the brushes it moves.
//!
//! The document keeps its preview per node ([`crate::document::SurfaceCache`]) and logs
//! which nodes each rebuild changed, so a sync looks only at those: its cost follows the
//! edit, not the map. Each node's surface carries a stamp that changes with its
//! triangles, so a tracker that fell behind the log (or a new one) compares stamps over
//! the drawn nodes instead -- still without hashing any geometry.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::clip_tool::ClipTool;
use crate::document::{Document, SurfaceCache};
use crate::view3d::{FrameOverlay, Scene3d};
use crate::view_filters::ViewFilters;
use render::Triangle;

/// Names an object across syncs: a node id, with a disambiguator in the high bits for
/// the rare node whose triangles come in more than one run.
pub type ObjectKey = u64;

/// The per-object state a renderer keeps outside the geometry, so changing it does not
/// re-send any vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ObjectState {
    /// The whole object is selected (a face selection is carried per triangle instead).
    pub selected: bool,
    /// On a locked layer.
    pub locked: bool,
    /// Not drawn, though kept.
    pub hidden: bool,
}

/// Map units per metre, which light entities' ranges (in engine units) are converted
/// with; the export's default scale.
pub const MAP_UNITS_PER_METRE: f64 = 32.0;

/// A point or spot light from a light entity, in map units and linear light; see
/// [`crate::document::Document::scene_lights`] for how entity keys map onto it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneLight {
    /// The entity's node id: names the light across syncs, so a renderer can keep what it
    /// cached for it (a shadow map).
    pub id: u64,
    /// Where it is.
    pub position: geometry_math::vec::Vec3d,
    /// Linear colour times energy (times pi, as engines scale light energy).
    pub color: [f32; 3],
    /// How far it reaches, in map units: nothing is lit beyond.
    pub range: f32,
    /// The exponent of the falloff with distance in metres (Godot's attenuation): 1 by
    /// default, 2 the physical inverse square; windowed to zero at `range`.
    pub decay: f32,
    /// For a spot light: the way it points (unit), the cosine of the cone's half-angle,
    /// and how sharply it fades towards the rim (Godot's angle attenuation).
    pub spot: Option<([f32; 3], f32, f32)>,
    /// Whether it casts shadows.
    pub shadows: bool,
}

/// Bits a renderer stores per vertex, worked out from a triangle and its object's state:
/// what the object-level state cannot say on its own.
pub mod triangle_flags {
    /// The triangle is tinted as selected although its object is not: a selected face.
    pub const FACE_SELECTED: u32 = 1;
    /// The triangle is not tinted although its object is selected: a painted face, whose
    /// colour is the point.
    pub const NO_TINT: u32 = 2;
    /// The skip material's hatching.
    pub const SKIP: u32 = 4;
}

/// The per-vertex flags of `triangle` in an object whose state says `selected`.
///
/// A painted face carries `NO_TINT` whatever the object's state (it only matters while
/// the object is selected), so selecting a painted brush changes its state, not its
/// vertices.
pub fn flags_of(triangle: &Triangle, selected: bool) -> u32 {
    let mut flags = 0;
    if triangle.selected && !selected {
        flags |= triangle_flags::FACE_SELECTED;
    }
    if triangle.painted || (selected && !triangle.selected) {
        flags |= triangle_flags::NO_TINT;
    }
    if triangle.material == Some(render::SKIP_PREVIEW_MATERIAL) {
        flags |= triangle_flags::SKIP;
    }
    flags
}

/// Where a retained renderer hears about changes. Calls come in this order within one
/// sync: `set_materials`, `clear`, then any mix of `remove`, `upsert` and `set_state`,
/// then `finish`.
pub trait SceneSink {
    /// The material collection was replaced. Indices in later triangles refer to it.
    fn set_materials(&mut self, materials: &[map_model::material::Material]);
    /// Drop every object: the map, the filters or the materials changed wholesale.
    fn clear(&mut self);
    /// An object is gone.
    fn remove(&mut self, key: ObjectKey);
    /// An object is new or its triangles changed. Per-vertex flags come from
    /// [`flags_of`] with `state.selected`.
    fn upsert(&mut self, key: ObjectKey, triangles: &[Triangle], state: ObjectState);
    /// Only an object's state changed.
    fn set_state(&mut self, key: ObjectKey, state: ObjectState);
    /// The point-entity sprites (camera-facing icons) to draw, replacing the last set.
    /// Sent whenever something other than the selection changed; empty while the Entity
    /// models view filter is off.
    fn set_sprites(&mut self, _sprites: &[render::SpriteInstance]) {}
    /// The light entities' point and spot lights, replacing the last set. Sent with the
    /// sprites (whenever something other than the selection changed).
    fn set_lights(&mut self, _lights: &[SceneLight]) {}
    /// The sync is over: a good moment to write to the GPU.
    fn finish(&mut self) {}
}

/// What one sync did, for profiling and tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncStats {
    /// Everything was dropped and sent again.
    pub full: bool,
    /// Objects whose triangles were sent.
    pub upserted: usize,
    /// Objects that went away.
    pub removed: usize,
    /// Objects whose state alone changed.
    pub states: usize,
    /// Triangles sent.
    pub triangles: usize,
}

/// Which pose of the animation preview to show.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pose {
    /// The geometry as saved.
    Rest,
    /// A scrubbed frame, as a fraction of a second.
    Frame(f64),
    /// Playing, at this many seconds.
    Playing(f64),
}

/// What a tracker remembers of one object it sent.
#[derive(Debug, Clone, Copy)]
struct Sent {
    /// The surface's stamp, or 0 for a posed copy (which the rest pose has to replace).
    stamp: u64,
    /// The hash of the per-vertex flags it was sent with.
    flags: u64,
    state: ObjectState,
}

/// Remembers what a [`SceneSink`] was last told, so the next sync can send the difference.
#[derive(Default)]
pub struct SceneTracker {
    generation: Option<u64>,
    materials: Option<u64>,
    filters: Option<ViewFilters>,
    revision: Option<u64>,
    content: Option<u64>,
    pose: Option<Pose>,
    /// The surfaces' epoch at the last sync: what the document's change log is read from.
    epoch: Option<u64>,
    objects: HashMap<ObjectKey, Sent>,
    /// Objects last sent in an animated pose.
    posed: HashSet<ObjectKey>,
    /// What the last sync did.
    pub stats: SyncStats,
}

/// An object's key: its node's id. Each drawn node is one object.
fn key_of(node: map_model::node::NodeId) -> ObjectKey {
    node.0
}

impl SceneTracker {
    /// A tracker that has sent nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets everything, so the next sync sends the whole scene (a new device, say).
    pub fn invalidate(&mut self) {
        *self = Self::default();
    }

    /// Brings `sink` up to date with what `document` shows under `filters`, with the
    /// clip tool's preview when `clip` is given. Returns whether anything was sent.
    pub fn sync(
        &mut self,
        document: &Document,
        filters: &ViewFilters,
        clip: Option<&ClipTool>,
        pose: Pose,
        sink: &mut dyn SceneSink,
    ) -> bool {
        self.sync_tool_preview(document, filters, clip, None, pose, sink)
    }

    pub fn sync_tool_preview(
        &mut self,
        document: &Document,
        filters: &ViewFilters,
        clip: Option<&ClipTool>,
        fillet: Option<&std::rc::Rc<crate::fillet_tool::SurfacePreview>>,
        pose: Pose,
        sink: &mut dyn SceneSink,
    ) -> bool {
        self.stats = SyncStats::default();
        let filters = filters.geometry_only();
        let generation = document.generation();
        let materials = document.materials_revision();
        let mut full = false;
        if self.materials != Some(materials) {
            sink.set_materials(document.materials().materials());
            full = true;
        }
        if full || self.generation != Some(generation) || self.filters != Some(filters) {
            full = true;
        }
        let revision = document.revision();
        let content = document.content_revision();
        if !full
            && self.revision == Some(revision)
            && self.pose == Some(pose)
            && !matches!(pose, Pose::Playing(_))
        {
            // The clip preview is not part of the revision: let the surfaces say.
            let clip_moved =
                document.with_tool_preview_surfaces(&filters, clip, fillet, |surfaces| {
                    Some(surfaces.epoch()) != self.epoch
                });
            if !clip_moved {
                return false;
            }
        }
        if full {
            sink.clear();
            self.objects.clear();
            self.posed.clear();
            self.epoch = None;
            self.stats.full = true;
        }
        let selection = document.selection_ids();
        let pose_moved = self.pose != Some(pose) || matches!(pose, Pose::Playing(_));
        document.with_tool_preview_surfaces(&filters, clip, fillet, |surfaces| {
            match self.epoch.and_then(|epoch| surfaces.changes_since(epoch)) {
                Some(changes) => {
                    let mut seen = HashSet::new();
                    for id in changes {
                        if seen.insert(id) {
                            self.visit(surfaces, id, selection, sink);
                        }
                    }
                }
                None => {
                    // Too far behind the log (or never synced): compare every drawn
                    // node's stamp, and drop what is no longer drawn.
                    for &id in surfaces.order() {
                        self.visit(surfaces, id, selection, sink);
                    }
                    let drawn: HashSet<ObjectKey> =
                        surfaces.order().iter().map(|id| key_of(*id)).collect();
                    let gone: Vec<ObjectKey> = self
                        .objects
                        .keys()
                        .filter(|key| !drawn.contains(key))
                        .copied()
                        .collect();
                    for key in gone {
                        self.objects.remove(&key);
                        self.posed.remove(&key);
                        sink.remove(key);
                        self.stats.removed += 1;
                    }
                }
            }
            // The animation preview moves only the nodes that have a motion.
            match pose {
                Pose::Rest => {
                    for key in std::mem::take(&mut self.posed) {
                        if let Some(sent) = self.objects.get_mut(&key) {
                            sent.stamp = 0;
                            self.visit(surfaces, map_model::node::NodeId(key), selection, sink);
                        }
                    }
                }
                Pose::Frame(time) | Pose::Playing(time) if pose_moved => {
                    let scrubbed = matches!(pose, Pose::Frame(_));
                    for (id, motion) in surfaces.motions() {
                        let Some(surface) = surfaces.get(*id) else {
                            continue;
                        };
                        let mut triangles = surface.triangles.clone();
                        crate::motion_preview::pose(&mut triangles, motion, time, scrubbed);
                        let state = ObjectState {
                            selected: selection.contains(id),
                            locked: surface.locked,
                            hidden: false,
                        };
                        let key = key_of(*id);
                        self.objects.insert(
                            key,
                            Sent {
                                stamp: 0,
                                flags: hash_flags(&triangles, state.selected),
                                state,
                            },
                        );
                        self.posed.insert(key);
                        sink.upsert(key, &triangles, state);
                        self.stats.upserted += 1;
                        self.stats.triangles += triangles.len();
                    }
                }
                _ => {}
            }
            self.epoch = Some(surfaces.epoch());
        });
        if full || self.content != Some(content) {
            // As the GL path uploads them: with the geometry, and only with entity models
            // on. Not per animation frame or selection click: neither moves an entity.
            sink.set_sprites(&if filters.show_point_entity_models {
                document.sprites()
            } else {
                Vec::new()
            });
            sink.set_lights(&document.scene_lights());
        }
        sink.finish();
        self.generation = Some(generation);
        self.materials = Some(materials);
        self.filters = Some(filters);
        self.revision = Some(revision);
        self.content = Some(content);
        self.pose = Some(pose);
        true
    }

    /// Sends what changed about one node: its triangles, its state, or that it is gone.
    fn visit(
        &mut self,
        surfaces: &SurfaceCache,
        id: map_model::node::NodeId,
        selection: &HashSet<map_model::node::NodeId>,
        sink: &mut dyn SceneSink,
    ) {
        let key = key_of(id);
        let Some(surface) = surfaces.get(id) else {
            if self.objects.remove(&key).is_some() {
                self.posed.remove(&key);
                sink.remove(key);
                self.stats.removed += 1;
            }
            return;
        };
        if self.posed.contains(&key) {
            // The animation preview sends this one; the rest pose puts it back.
            return;
        }
        let state = ObjectState {
            selected: selection.contains(&id),
            locked: surface.locked,
            hidden: false,
        };
        let flags = hash_flags(&surface.triangles, state.selected);
        match self.objects.get_mut(&key) {
            Some(sent) if sent.stamp == surface.stamp && sent.flags == flags => {
                if sent.state != state {
                    sent.state = state;
                    sink.set_state(key, state);
                    self.stats.states += 1;
                }
            }
            _ => {
                self.objects.insert(
                    key,
                    Sent {
                        stamp: surface.stamp,
                        flags,
                        state,
                    },
                );
                sink.upsert(key, &surface.triangles, state);
                self.stats.upserted += 1;
                self.stats.triangles += surface.triangles.len();
            }
        }
    }
}

/// The hash of an object's per-vertex flags, so a selection change can tell a moved face
/// selection (which changes them) from a plain state change.
fn hash_flags(triangles: &[Triangle], selected: bool) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for triangle in triangles {
        flags_of(triangle, selected).hash(&mut hasher);
    }
    hasher.finish()
}

/// Everything a backend needs for one frame besides the geometry, from
/// [`Scene3d::prepare_frame`].
pub struct PreparedFrame {
    /// The render settings from the preferences.
    pub settings: render::RenderSettings,
    /// The tools' handles and guides; empty while an animation plays.
    pub overlay: FrameOverlay,
    /// Wireframe instead of shaded.
    pub wireframe: bool,
    /// An animation is playing, so another frame is wanted.
    pub playing: bool,
    /// [`crate::view3d::frame_key`] of this frame.
    pub frame_key: u64,
    /// [`crate::view3d::picture_key`] of this frame.
    pub picture_key: u64,
    /// The world's bounds, for the axes.
    pub world_bounds: geometry_math::bbox::Bbox3d,
}

impl Scene3d {
    /// Whether an animation preview is playing, which wants a frame every frame.
    pub fn playing(&self) -> bool {
        let filters = self.filters.borrow();
        filters.animation_frame.is_none()
            && filters.preview_motion
            && !self.preview_mesh.borrow().motions.is_empty()
    }

    /// [`Scene3d::render_kept`] for a retained renderer: syncs `sink` through `tracker`
    /// and builds the overlay, for a frame `width` by `height` logical pixels at `scale`.
    /// Leaves the drawing to the caller.
    pub fn prepare_frame(
        &self,
        tracker: &mut SceneTracker,
        sink: &mut dyn SceneSink,
        width: f64,
        height: f64,
        scale: i32,
    ) -> PreparedFrame {
        let settings = self.begin_frame(width, height);
        let document = self.document.borrow();
        let filters = *self.filters.borrow();
        let clip_state =
            (*self.tool.borrow() == crate::tool::Tool::Clip).then(|| self.clip.borrow().clone());

        let key = (document.generation(), document.revision());
        if tracker.revision != Some(key.1) || tracker.generation != Some(key.0) {
            *self.preview_motions.borrow_mut() = document.preview_motions();
            // Only whether there are any is read on this path ([`Scene3d::playing`]); the
            // GL path's triangle ranges do not exist here.
            let motions =
                document.with_preview_surfaces(&filters, clip_state.as_ref(), |surfaces| {
                    surfaces
                        .motions()
                        .values()
                        .map(|motion| (0..0, motion.clone()))
                        .collect()
                });
            self.preview_mesh.borrow_mut().motions = motions;
        }
        let playing = filters.animation_frame.is_none()
            && filters.preview_motion
            && !self.preview_mesh.borrow().motions.is_empty();
        document.animation_preview_time(playing);
        let pose = if let Some(frame) = filters.animation_frame {
            Pose::Frame(f64::from(frame) / f64::from(filters.animation_fps.max(1)))
        } else if playing {
            Pose::Playing(document.animation_preview_time(true))
        } else {
            Pose::Rest
        };
        let fillet_preview = self.fillet_surface_preview(&document, width, height);
        tracker.sync_tool_preview(
            &document,
            &filters,
            clip_state.as_ref(),
            fillet_preview.as_ref(),
            pose,
            sink,
        );

        let mut overlay = self.build_overlay(&document, width, height, playing);
        if playing {
            overlay.points.clear();
            overlay.guides.clear();
        }
        let (w, h) = (
            (width * f64::from(scale)).round() as i32,
            (height * f64::from(scale)).round() as i32,
        );
        let mut frame_key = crate::view3d::frame_key(
            &self.camera.borrow(),
            key,
            &filters,
            w,
            h,
            &overlay.points,
            &overlay.guides,
        );
        // Include temporary surface changes even when map revision and guide lines match.
        if let Some(epoch) = tracker.epoch {
            frame_key ^= epoch.rotate_left(29);
        }
        let picture_key =
            crate::view3d::picture_key(frame_key, &settings, document.materials_revision());
        PreparedFrame {
            settings,
            overlay,
            wireframe: filters.wireframe,
            playing,
            frame_key,
            picture_key,
            world_bounds: document.world_bounds(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Log {
        cleared: usize,
        upserts: Vec<ObjectKey>,
        removes: Vec<ObjectKey>,
        states: Vec<(ObjectKey, ObjectState)>,
    }

    impl SceneSink for Log {
        fn set_materials(&mut self, _: &[map_model::material::Material]) {}
        fn clear(&mut self) {
            self.cleared += 1;
        }
        fn remove(&mut self, key: ObjectKey) {
            self.removes.push(key);
        }
        fn upsert(&mut self, key: ObjectKey, _: &[Triangle], _: ObjectState) {
            self.upserts.push(key);
        }
        fn set_state(&mut self, key: ObjectKey, state: ObjectState) {
            self.states.push((key, state));
        }
    }

    fn two_brushes() -> Document {
        let mut document = Document::new(crate::world::WorldBounds::default());
        for (min, max) in [([0, 0, 0], [64, 64, 64]), ([128, 0, 0], [192, 64, 64])] {
            crate::mcp::call_tool(
                &mut document,
                &mut crate::mcp::Collab::default(),
                "add_shape",
                &serde_json::json!({"kind":"cuboid","min":min,"max":max}),
            )
            .unwrap();
        }
        document.clear_selection();
        document
    }

    #[test]
    fn the_first_sync_sends_everything_and_the_next_nothing() {
        let document = two_brushes();
        let mut tracker = SceneTracker::new();
        let mut log = Log::default();
        assert!(tracker.sync(
            &document,
            &ViewFilters::default(),
            None,
            Pose::Rest,
            &mut log
        ));
        assert_eq!(log.cleared, 1);
        assert_eq!(log.upserts.len(), 2);
        let mut again = Log::default();
        assert!(!tracker.sync(
            &document,
            &ViewFilters::default(),
            None,
            Pose::Rest,
            &mut again
        ));
        assert!(again.upserts.is_empty() && again.states.is_empty());
    }

    #[test]
    fn selecting_a_brush_sends_only_its_state() {
        let mut document = two_brushes();
        let mut tracker = SceneTracker::new();
        tracker.sync(
            &document,
            &ViewFilters::default(),
            None,
            Pose::Rest,
            &mut Log::default(),
        );
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        let mut log = Log::default();
        tracker.sync(
            &document,
            &ViewFilters::default(),
            None,
            Pose::Rest,
            &mut log,
        );
        assert_eq!(log.cleared, 0);
        assert!(log.upserts.is_empty(), "{:?}", log.upserts);
        assert_eq!(log.states.len(), 1);
        assert!(log.states[0].1.selected);
    }

    #[test]
    fn moving_a_brush_resends_only_that_brush() {
        let mut document = two_brushes();
        let mut tracker = SceneTracker::new();
        tracker.sync(
            &document,
            &ViewFilters::default(),
            None,
            Pose::Rest,
            &mut Log::default(),
        );
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        document.translate_selection(geometry_math::vec::Vec3d::new(16.0, 0.0, 0.0));
        let mut log = Log::default();
        tracker.sync(
            &document,
            &ViewFilters::default(),
            None,
            Pose::Rest,
            &mut log,
        );
        assert_eq!(log.cleared, 0);
        assert_eq!(log.upserts.len(), 1);
        assert!(log.removes.is_empty());
    }

    #[test]
    fn selecting_a_painted_brush_sends_only_state() {
        // The crate's faces are painted.
        let path = format!("{}/../../assets/crate.map", env!("CARGO_MANIFEST_DIR"));
        let mut document = Document::open(&path, crate::world::WorldBounds::default()).unwrap();
        let mut tracker = SceneTracker::new();
        let filters = ViewFilters::default();
        tracker.sync(&document, &filters, None, Pose::Rest, &mut Log::default());
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        let mut log = Log::default();
        tracker.sync(&document, &filters, None, Pose::Rest, &mut log);
        assert_eq!(tracker.stats.upserted, 0, "{:?}", tracker.stats);
        assert_eq!(tracker.stats.states, 1);
    }

    #[test]
    fn the_clip_preview_resends_the_cut_brush_and_puts_it_back() {
        let mut document = two_brushes();
        let mut tracker = SceneTracker::new();
        let filters = ViewFilters::default();
        tracker.sync(&document, &filters, None, Pose::Rest, &mut Log::default());
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        tracker.sync(&document, &filters, None, Pose::Rest, &mut Log::default());
        let mut clip = ClipTool::new();
        clip.set_face([
            geometry_math::vec::Vec3d::new(32.0, 0.0, 0.0),
            geometry_math::vec::Vec3d::new(32.0, 64.0, 0.0),
            geometry_math::vec::Vec3d::new(32.0, 0.0, 64.0),
        ]);
        let mut log = Log::default();
        assert!(tracker.sync(&document, &filters, Some(&clip), Pose::Rest, &mut log));
        assert_eq!((log.cleared, log.upserts.len()), (0, 1));
        let mut log = Log::default();
        assert!(tracker.sync(&document, &filters, None, Pose::Rest, &mut log));
        assert_eq!((log.cleared, log.upserts.len()), (0, 1));
    }

    #[test]
    fn fillet_preview_sends_only_changed_surfaces_and_clears_on_tool_exit() {
        let mut document = two_brushes();
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        let filters = ViewFilters::default();
        let mut tracker = SceneTracker::new();
        tracker.sync(&document, &filters, None, Pose::Rest, &mut Log::default());
        let edge = document.brush(id).unwrap().edge_positions()[0];
        let mut cache = crate::fillet_tool::PreviewCache::default();
        let settings = crate::fillet_tool::FilletParameters::default();
        let preview = cache.get(&document, vec![edge], settings).unwrap();
        assert!(std::rc::Rc::ptr_eq(
            &preview,
            &cache.get(&document, vec![edge], settings).unwrap()
        ));
        let mut log = Log::default();
        assert!(tracker.sync_tool_preview(
            &document,
            &filters,
            None,
            Some(&preview),
            Pose::Rest,
            &mut log
        ));
        assert_eq!((log.cleared, log.upserts.len()), (0, 1));
        assert!(!tracker.sync_tool_preview(
            &document,
            &filters,
            None,
            Some(&preview),
            Pose::Rest,
            &mut Log::default()
        ));
        let revised = cache
            .get(
                &document,
                vec![edge],
                crate::fillet_tool::FilletParameters {
                    radius: 12.0,
                    ..settings
                },
            )
            .unwrap();
        let mut log = Log::default();
        assert!(tracker.sync_tool_preview(
            &document,
            &filters,
            None,
            Some(&revised),
            Pose::Rest,
            &mut log
        ));
        assert_eq!(log.upserts.len(), 1);
        let mut log = Log::default();
        assert!(tracker.sync(&document, &filters, None, Pose::Rest, &mut log));
        assert_eq!(log.upserts.len(), 1);
        assert!(cache
            .get(
                &document,
                vec![edge],
                crate::fillet_tool::FilletParameters {
                    radius: 10000.0,
                    ..settings
                }
            )
            .is_none());
    }

    #[test]
    fn a_tracker_behind_the_log_compares_stamps_and_sends_only_the_difference() {
        let mut document = two_brushes();
        let filters = ViewFilters::default();
        let mut behind = SceneTracker::new();
        behind.sync(&document, &filters, None, Pose::Rest, &mut Log::default());
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        // Many edits the tracker never sees, so the log no longer reaches back to it.
        for _ in 0..3000 {
            document.translate_selection(geometry_math::vec::Vec3d::new(1.0, 0.0, 0.0));
            document.with_preview_surfaces(&filters, None, |_| ());
        }
        let mut log = Log::default();
        behind.sync(&document, &filters, None, Pose::Rest, &mut log);
        assert_eq!(log.cleared, 0);
        // The moved brush, with its new state; the other one is as it was.
        assert_eq!(log.upserts, vec![id.0]);
        assert!(log.states.is_empty() && log.removes.is_empty());
    }
}
