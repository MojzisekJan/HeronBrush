//! The handles the vertex, edge and face tools drag.
//!
//! Vertex, edge and face tools share one handle manager. The handle kind determines its position
//! and the brush vertices affected by a drag.
//!
//! `Brush::transformEdges` and `Brush::transformFaces` both gather their vertices and
//! call `doTransformVertices`, so every drag here comes down to moving a set of vertex
//! positions -- which is exactly what [`map_model::brush::Brush::move_vertices`] does.
//!
//! Handles are addressed by position because editing rebuilds brush geometry and invalidates the
//! previous handle indices.

use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec2d, Vec3d};
use map_model::brush::Brush;

use crate::tool::Tool;

/// What a handle sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleKind {
    /// One handle per vertex.
    Vertex,
    /// One handle at the middle of each edge.
    Edge,
    /// One handle at the middle of each face.
    Face,
    /// One handle per control point of a patch.
    ControlPoint,
}

impl HandleKind {
    /// Returns the kind of handle a tool drags, if it drags handles at all.
    pub fn of(tool: Tool) -> Option<Self> {
        match tool {
            Tool::Vertex => Some(HandleKind::Vertex),
            Tool::Edge => Some(HandleKind::Edge),
            Tool::Face => Some(HandleKind::Face),
            Tool::ControlPoint => Some(HandleKind::ControlPoint),
            // The fillet picks edges, so it shows the edge tool's handles; what a click
            // on one does is the difference between the two tools.
            Tool::Fillet => Some(HandleKind::Edge),
            _ => None,
        }
    }

    /// Returns what to call an action on this many of these handles.
    ///
    /// `EdgeTool::actionName` and its siblings.
    pub fn action_name(self, count: usize) -> String {
        let (one, many) = match self {
            HandleKind::Vertex => ("Move Vertex", "Move Vertices"),
            HandleKind::Edge => ("Move Edge", "Move Edges"),
            HandleKind::Face => ("Move Face", "Move Faces"),
            HandleKind::ControlPoint => ("Move Control Point", "Move Control Points"),
        };
        text_utils::string_format::str_plural(count, one, many).to_string()
    }
}

/// One handle: where it is, and which brush vertices it takes with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Handle {
    /// Where the handle is drawn and picked.
    pub position: Vec3d,
    /// The vertex positions that move when it is dragged.
    pub vertices: Vec<Vec3d>,
    /// Whether dragging it adds a corner rather than moving the ones it stands for.
    ///
    /// The vertex tool offers the middle of every edge and the centre of every face as
    /// well as the corners; taking hold of one of those with Shift splits it, which is
    /// `VertexTool`'s own way of adding a vertex.
    pub splits: bool,
}

/// The lines that mark what a handle stands for.
///
/// A vertex handle marks nothing but itself, so the point is all there is to draw; an edge
/// handle marks its edge, and a face handle the outline of its face. This is what a view
/// draws round the handle under the pointer, so hovering says what a press would take
/// hold of.
pub fn outline_of(handle: &Handle) -> Vec<(Vec3d, Vec3d)> {
    match handle.vertices.as_slice() {
        [_] | [] => Vec::new(),
        [from, to] => vec![(*from, *to)],
        corners => corners
            .iter()
            .enumerate()
            .map(|(index, corner)| (*corner, corners[(index + 1) % corners.len()]))
            .collect(),
    }
}

/// How close two positions have to be to count as the same handle.
///
/// Two brushes that share a corner share the handle on it, so dragging it moves both.
/// This is what `NodeHandleManager` does by counting how many brushes each handle came
/// from.
const SAME_HANDLE: f64 = 0.001;

/// The handles of the selected brushes, and which of them are selected.
#[derive(Debug, Clone, Default)]
pub struct HandleManager {
    kind: Option<HandleKind>,
    handles: Vec<Handle>,
    /// The positions of the selected handles, kept as positions rather than indices so
    /// they survive the brushes being rebuilt by a drag.
    selected: Vec<Vec3d>,
}

impl HandleManager {
    /// Creates a manager with nothing collected.
    pub fn new() -> Self {
        Self::default()
    }

    /// Collects the handles of the given brushes.
    ///
    /// Keeps whichever handles were selected before, matched by position, so that
    /// rebuilding mid-drag does not drop the selection.
    pub fn collect<'a>(&mut self, kind: HandleKind, brushes: impl Iterator<Item = &'a Brush>) {
        self.kind = Some(kind);
        self.handles.clear();

        let mut split_sources = Vec::new();
        for brush in brushes {
            if kind == HandleKind::Vertex {
                // The middle of every edge and the centre of every face, which Shift and a
                // drag turn into a new corner: `VertexTool`'s `findDraggableHandle` looks
                // for these alongside the corners themselves.
                for (from, to) in brush.edge_positions() {
                    split_sources.push(((from + to) / 2.0, vec![from, to]));
                }
                let geometry = brush.geometry();
                for index in 0..geometry.face_count() {
                    let vertices =
                        geometry.face_positions(map_model::polyhedron::FaceId(index as u32));
                    split_sources.push((centroid(&vertices), vertices));
                }
            }
            match kind {
                // A control point is not on a brush; `collect_control_points` gathers
                // those instead.
                HandleKind::ControlPoint => break,
                HandleKind::Vertex => {
                    for position in brush.vertex_positions() {
                        self.add(position, vec![position]);
                    }
                }
                HandleKind::Edge => {
                    for (from, to) in brush.edge_positions() {
                        self.add((from + to) / 2.0, vec![from, to]);
                    }
                }
                HandleKind::Face => {
                    let geometry = brush.geometry();
                    for index in 0..geometry.face_count() {
                        let vertices =
                            geometry.face_positions(map_model::polyhedron::FaceId(index as u32));
                        self.add(centroid(&vertices), vertices);
                    }
                }
            }
        }

        for (position, vertices) in split_sources {
            self.add_split(position, vertices);
        }

        self.forget_handles_that_are_gone();
    }

    /// Collects the control points of the given patches.
    ///
    /// `ControlPointTool::addHandles`: one handle per control point, merged where two
    /// land in the same place, so dragging where two patches meet moves both.
    pub fn collect_control_points<'a>(
        &mut self,
        patches: impl Iterator<Item = &'a map_model::io::map_reader::Patch>,
    ) {
        self.kind = Some(HandleKind::ControlPoint);
        self.handles.clear();

        for patch in patches {
            for point in &patch.control_points {
                self.add(point.position, vec![point.position]);
            }
        }

        self.forget_handles_that_are_gone();
    }

    /// A handle that is gone -- merged away by a drag, or on something that is no longer
    /// selected -- takes its selection with it.
    fn forget_handles_that_are_gone(&mut self) {
        let handles = std::mem::take(&mut self.handles);
        self.selected.retain(|position| {
            handles
                .iter()
                .any(|handle| is_same(handle.position, *position))
        });
        self.handles = handles;
    }

    /// Adds a handle, merging it with one already at the same place.
    fn add(&mut self, position: Vec3d, vertices: Vec<Vec3d>) {
        self.add_handle(position, vertices, false);
    }

    /// Adds a handle that splits what it stands for rather than moving it.
    fn add_split(&mut self, position: Vec3d, vertices: Vec<Vec3d>) {
        // A corner already offered wins: dragging it moves it, which is the commoner
        // thing to want where the two land in the same place.
        if self
            .handles
            .iter()
            .any(|handle| is_same(handle.position, position))
        {
            return;
        }
        self.add_handle(position, vertices, true);
    }

    fn add_handle(&mut self, position: Vec3d, vertices: Vec<Vec3d>, splits: bool) {
        if let Some(existing) = self
            .handles
            .iter_mut()
            .find(|handle| is_same(handle.position, position))
        {
            for vertex in vertices {
                if !existing.vertices.iter().any(|v| is_same(*v, vertex)) {
                    existing.vertices.push(vertex);
                }
            }
            return;
        }

        self.handles.push(Handle {
            position,
            vertices,
            splits,
        });
    }

    /// Forgets everything, which is what leaving the tool does.
    pub fn clear(&mut self) {
        self.kind = None;
        self.handles.clear();
        self.selected.clear();
    }

    /// Returns which kind of handle is being shown, if any.
    pub fn kind(&self) -> Option<HandleKind> {
        self.kind
    }

    /// Returns every handle, for drawing.
    pub fn handles(&self) -> &[Handle] {
        &self.handles
    }

    /// The handles a drag moves: everything but the ones that would add a corner.
    pub fn movable(&self) -> impl Iterator<Item = &Handle> {
        self.handles.iter().filter(|handle| !handle.splits)
    }

    /// The handles a drag splits: the middle of an edge, the centre of a face.
    pub fn splitting(&self) -> impl Iterator<Item = &Handle> {
        self.handles.iter().filter(|handle| handle.splits)
    }

    /// Returns whether the handle at the given position is selected.
    pub fn is_selected(&self, position: Vec3d) -> bool {
        self.selected.iter().any(|p| is_same(*p, position))
    }

    /// Returns how many handles are selected.
    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    /// Returns the handle nearest the cursor, within `radius`.
    ///
    /// The comparison happens where the view draws the handles rather than in world
    /// space, since two handles at different depths can be under the same pixel and the
    /// nearer one on screen is the one meant. `project` is how the view puts a world
    /// position on screen, and `radius` is in whatever units it works in.
    #[cfg(test)]
    pub fn pick(
        &self,
        cursor: Vec2d,
        radius: f64,
        project: impl Fn(Vec3d) -> Vec2d,
    ) -> Option<Vec3d> {
        self.pick_among(self.movable(), cursor, radius, project)
    }

    /// Picks one handle in a flat view, preferring the camera-facing one in a stack.
    pub fn pick_orthographic(
        &self,
        cursor: Vec2d,
        radius: f64,
        view_direction: Vec3d,
        project: impl Fn(Vec3d) -> Vec2d,
    ) -> Option<Vec3d> {
        self.movable()
            .map(|handle| {
                (
                    handle.position,
                    vec::squared_distance(project(handle.position), cursor),
                )
            })
            .filter(|(_, distance)| *distance <= radius * radius)
            .min_by(|(a, da), (b, db)| {
                if (da - db).abs() <= 1e-9 {
                    vec::dot(*a, view_direction).total_cmp(&vec::dot(*b, view_direction))
                } else {
                    da.total_cmp(db)
                }
            })
            .map(|(position, _)| position)
    }

    /// Adds vertices hidden behind selected vertices along a flat view's depth axis.
    /// Call only when a click becomes a drag. The tolerance is in world units, so a
    /// zoomed-out view cannot accidentally pick up neighbouring, non-aligned vertices.
    pub fn expand_vertex_stacks(&mut self, depth_axis: usize, enabled: bool) {
        if !enabled || self.kind != Some(HandleKind::Vertex) {
            return;
        }
        let aligned = |a: Vec3d, b: Vec3d| {
            let mut delta = a - b;
            delta[depth_axis] = 0.0;
            vec::squared_distance(delta, Vec3d::zero()) <= SAME_HANDLE * SAME_HANDLE
        };
        let behind: Vec<_> = self
            .movable()
            .filter(|handle| {
                self.selected
                    .iter()
                    .any(|selected| aligned(handle.position, *selected))
            })
            .map(|handle| handle.position)
            .filter(|position| !self.is_selected(*position))
            .collect();
        self.selected.extend(behind);
    }

    /// The handle under the cursor, of any kind: one that moves or one that splits.
    ///
    /// What `findDraggableHandle` looks for when the vertex tool is asked to snap the one
    /// picked vertex onto something else: any handle will do as a destination.
    pub fn pick_any(
        &self,
        cursor: Vec2d,
        radius: f64,
        project: impl Fn(Vec3d) -> Vec2d,
    ) -> Option<Vec3d> {
        self.pick_among(self.handles.iter(), cursor, radius, project)
    }

    /// The handle under the cursor that a press would split, if there is one.
    pub fn pick_split(
        &self,
        cursor: Vec2d,
        radius: f64,
        project: impl Fn(Vec3d) -> Vec2d,
    ) -> Option<Vec3d> {
        self.pick_among(self.splitting(), cursor, radius, project)
    }

    fn pick_among<'a>(
        &self,
        among: impl Iterator<Item = &'a Handle>,
        cursor: Vec2d,
        radius: f64,
        project: impl Fn(Vec3d) -> Vec2d,
    ) -> Option<Vec3d> {
        among
            .map(|handle| (handle.position, project(handle.position)))
            .filter(|(_, on_screen)| vec::distance(*on_screen, cursor) <= radius)
            .min_by(|(_, a), (_, b)| {
                vec::squared_distance(*a, cursor)
                    .partial_cmp(&vec::squared_distance(*b, cursor))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(position, _)| position)
    }

    /// Selects a handle, or adds it to the selection.
    ///
    /// `VertexToolController`'s select part: a plain click replaces the selection, and
    /// shift adds to it, the same way clicking a brush does.
    pub fn select(&mut self, position: Vec3d, add: bool) {
        if !add {
            self.selected.clear();
            self.selected.push(position);
            return;
        }

        if let Some(index) = self.selected.iter().position(|p| is_same(*p, position)) {
            self.selected.remove(index);
        } else {
            self.selected.push(position);
        }
    }

    /// The handle a snapped drag is measured from.
    ///
    /// Uses the first selected handle as the drag anchor when multiple handles are selected.
    pub fn selected_anchor(&self) -> Option<Vec3d> {
        self.selected.first().copied()
    }

    /// Drops the selection.
    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Selects every handle a drag would move.
    pub fn select_all(&mut self) {
        self.selected = self.movable().map(|handle| handle.position).collect();
    }

    /// The ends of the selected edge handles, as pairs.
    ///
    /// Empty unless the handles are edges: a vertex or a face handle is not an edge, and
    /// what asks for this -- the fillet -- rounds edges.
    pub fn selected_edges(&self) -> Vec<(Vec3d, Vec3d)> {
        if self.kind != Some(HandleKind::Edge) {
            return Vec::new();
        }
        self.handles
            .iter()
            .filter(|handle| self.is_selected(handle.position))
            .filter_map(|handle| match handle.vertices.as_slice() {
                [from, to] => Some((*from, *to)),
                _ => None,
            })
            .collect()
    }

    /// Where the selected face handles are.
    ///
    /// Empty unless the handles are faces. A face handle sits at the centre of its face,
    /// so these centres are what names the faces again; the document turns them back into
    /// faces with `Document::faces_at`. Picking a face handle deliberately leaves the
    /// document's own face selection alone -- the brush has to stay selected for the
    /// handles to be there at all -- so this is how a panel learns what the face tool has
    /// hold of.
    pub fn selected_face_centres(&self) -> Vec<Vec3d> {
        if self.kind != Some(HandleKind::Face) {
            return Vec::new();
        }
        self.handles
            .iter()
            .filter(|handle| self.is_selected(handle.position))
            .map(|handle| handle.position)
            .collect()
    }

    /// The vertex positions a drag of the selected handles would move.
    pub fn selected_vertices(&self) -> Vec<Vec3d> {
        let mut vertices: Vec<Vec3d> = Vec::new();
        for handle in &self.handles {
            if !self.is_selected(handle.position) {
                continue;
            }
            for vertex in &handle.vertices {
                if !vertices.iter().any(|v| is_same(*v, *vertex)) {
                    vertices.push(*vertex);
                }
            }
        }
        vertices
    }

    /// Moves the selected handles' own positions, after their brushes have been moved.
    ///
    /// `m_dragHandlePosition = m_dragHandlePosition.transform(transform)`: the handles
    /// have to follow what they moved, or the next step of the drag picks up nothing.
    pub fn move_selection(&mut self, delta: Vec3d) {
        for position in &mut self.selected {
            *position += delta;
        }
    }
}

/// The middle of a ring of points.
fn centroid(points: &[Vec3d]) -> Vec3d {
    if points.is_empty() {
        return Vec3d::zero();
    }
    let sum = points
        .iter()
        .fold(Vec3d::zero(), |total, point| total + *point);
    sum / points.len() as f64
}

fn is_same(a: Vec3d, b: Vec3d) -> bool {
    vec::squared_distance(a, b) <= SAME_HANDLE * SAME_HANDLE
}

/// The epsilon a drag matches vertex positions with.
///
/// Looser than `SAME_HANDLE`, because the positions being matched came from the brush
/// itself and only have to survive being carried through a drag.
pub const DRAG_EPSILON: f64 = f64::ALMOST_ZERO;

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::bbox::Bbox3d;

    fn world() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    fn cube(min: f64, max: f64) -> Brush {
        map_model::shapes::create_cuboid(&world(), &Bbox3d::from_scalars(min, max), "stone")
            .expect("a cube")
    }

    fn collected(kind: HandleKind, brushes: &[Brush]) -> HandleManager {
        let mut manager = HandleManager::new();
        manager.collect(kind, brushes.iter());
        manager
    }

    #[test]
    fn a_cube_has_eight_vertex_handles_twelve_edge_handles_and_six_face_handles() {
        let brushes = vec![cube(-32.0, 32.0)];
        let vertices = collected(HandleKind::Vertex, &brushes);
        assert_eq!(vertices.movable().count(), 8);
        // The vertex tool also offers the middle of every edge and the centre of every
        // face, which a Shift-drag turns into a corner of its own.
        assert_eq!(vertices.splitting().count(), 12 + 6);
        assert_eq!(collected(HandleKind::Edge, &brushes).handles().len(), 12);
        assert_eq!(collected(HandleKind::Face, &brushes).handles().len(), 6);
    }

    #[test]
    fn an_edge_handle_sits_between_its_ends_and_moves_both() {
        let manager = collected(HandleKind::Edge, &[cube(-32.0, 32.0)]);

        for handle in manager.handles() {
            assert_eq!(handle.vertices.len(), 2);
            geometry_math::assert_approx_eq!(
                handle.position,
                (handle.vertices[0] + handle.vertices[1]) / 2.0,
                1e-9
            );
        }
    }

    #[test]
    fn a_face_handle_sits_at_the_middle_of_its_face_and_moves_its_ring() {
        let manager = collected(HandleKind::Face, &[cube(-32.0, 32.0)]);

        for handle in manager.handles() {
            assert_eq!(handle.vertices.len(), 4);
            // The middle of a face of a cube about the origin is 32 out along one axis.
            let distance = vec::length(handle.position);
            geometry_math::assert_approx_eq!(distance, 32.0, 1e-9);
        }
    }

    #[test]
    fn two_brushes_meeting_at_a_corner_share_the_handle_on_it() {
        // Two cubes touching along a face: the four vertices they share give one handle
        // each, so dragging one moves both brushes.
        let brushes = vec![cube(-32.0, 32.0), {
            let mut shifted = cube(-32.0, 32.0);
            shifted = shifted
                .translate(&world(), Vec3d::new(64.0, 0.0, 0.0))
                .expect("a translation");
            shifted
        }];

        let manager = collected(HandleKind::Vertex, &brushes);
        // Sixteen corners in all, four of them shared, so twelve handles.
        assert_eq!(manager.movable().count(), 12);

        let shared = manager
            .movable()
            .filter(|handle| (handle.position.x() - 32.0).abs() < SAME_HANDLE)
            .count();
        assert_eq!(shared, 4);
    }

    #[test]
    fn flat_view_click_picks_one_front_vertex_and_drag_expands_the_stack() {
        for depth in 0..3 {
            for sign in [-1.0, 1.0] {
                let brush = cube(-32.0, 32.0);
                let mut manager = collected(HandleKind::Vertex, std::slice::from_ref(&brush));
                let axes: Vec<_> = (0..3).filter(|axis| *axis != depth).collect();
                let project = |p: Vec3d| Vec2d::new(p[axes[0]], p[axes[1]]);
                let mut direction = Vec3d::zero();
                direction[depth] = sign;
                let picked = manager
                    .pick_orthographic(Vec2d::new(32.0, 32.0), 4.0, direction, project)
                    .unwrap();
                assert!((picked[depth] + sign * 32.0).abs() < 1e-6);
                manager.select(picked, false);
                assert_eq!(
                    manager.selected_count(),
                    1,
                    "a click only selects the front vertex"
                );
                manager.expand_vertex_stacks(depth, false);
                assert_eq!(
                    manager.selected_count(),
                    1,
                    "disabled preference keeps a single vertex"
                );
                manager.expand_vertex_stacks(depth, true);
                assert_eq!(manager.selected_count(), 2);
                assert_eq!(manager.selected_anchor(), Some(picked));
                let mut delta = Vec3d::zero();
                delta[axes[0]] = 16.0;
                let vertices = manager.selected_vertices();
                let moved = brush.move_vertices(&vertices, delta, DRAG_EPSILON).unwrap();
                manager.move_selection(delta);
                manager.collect(HandleKind::Vertex, std::iter::once(&moved));
                assert_eq!(manager.selected_count(), 2);
                for original in vertices {
                    assert!(manager.is_selected(original + delta));
                }
            }
        }
    }

    #[test]
    fn stack_expansion_excludes_nearby_vertices_and_split_handles() {
        let mut manager = collected(HandleKind::Vertex, &[cube(-32.0, 32.0)]);
        let picked = Vec3d::new(32.0, 32.0, 32.0);
        let nearby = picked + Vec3d::new(0.125, 0.0, -16.0);
        manager.add_handle(nearby, vec![nearby], false);
        let split = Vec3d::new(32.0, 32.0, 0.0);
        manager.add_handle(split, vec![split], true);
        manager.select(picked, false);
        manager.expand_vertex_stacks(2, true);
        assert_eq!(manager.selected_count(), 2);
        assert!(!manager.is_selected(nearby));
        assert!(!manager.is_selected(split));
        manager.expand_vertex_stacks(2, true);
        assert_eq!(
            manager.selected_count(),
            2,
            "expansion does not toggle existing selections"
        );
    }

    #[test]
    fn stack_expansion_does_not_change_edge_selection() {
        let mut manager = collected(HandleKind::Edge, &[cube(-32.0, 32.0)]);
        let position = manager.movable().next().unwrap().position;
        manager.select(position, false);
        manager.expand_vertex_stacks(2, true);
        assert_eq!(manager.selected_count(), 1);
    }

    #[test]
    fn picking_finds_the_nearest_handle_within_reach() {
        let manager = collected(HandleKind::Vertex, &[cube(-32.0, 32.0)]);

        // Seen from above, which is what a top view shows.
        let from_above = |p: Vec3d| Vec2d::new(p.x(), p.y());

        // A brush is defined by planes, so its corners land near round numbers rather
        // than on them; the comparison has to allow for that.
        let corner = Vec3d::new(32.0, 32.0, 32.0);
        let picked = manager
            .pick(Vec2d::new(33.0, 32.0), 4.0, from_above)
            .expect("a handle within reach");
        // Two corners share that spot from above; either is the right answer.
        assert!(is_same(picked, corner) || is_same(picked, Vec3d::new(32.0, 32.0, -32.0)));
        assert_eq!(manager.pick(Vec2d::zero(), 4.0, from_above), None);
    }

    #[test]
    fn a_plain_click_replaces_the_selection_and_shift_adds_to_it() {
        let mut manager = collected(HandleKind::Vertex, &[cube(-32.0, 32.0)]);
        let a = Vec3d::new(32.0, 32.0, 32.0);
        let b = Vec3d::new(-32.0, 32.0, 32.0);

        manager.select(a, false);
        assert!(manager.is_selected(a));
        assert_eq!(manager.selected_count(), 1);

        manager.select(b, false);
        assert!(!manager.is_selected(a));
        assert!(manager.is_selected(b));

        manager.select(a, true);
        assert_eq!(manager.selected_count(), 2);

        // Shift-clicking a selected handle drops it again.
        manager.select(a, true);
        assert_eq!(manager.selected_count(), 1);
    }

    #[test]
    fn dragging_an_edge_handle_moves_both_of_its_vertices() {
        let brush = cube(-32.0, 32.0);
        let mut manager = collected(HandleKind::Edge, std::slice::from_ref(&brush));

        let handle = manager.handles()[0].position;
        manager.select(handle, false);

        let vertices = manager.selected_vertices();
        assert_eq!(vertices.len(), 2);

        let delta = Vec3d::new(0.0, 0.0, 16.0);
        let moved = brush
            .move_vertices(&vertices, delta, DRAG_EPSILON)
            .expect("the edge moves");

        // The two vertices went up, and the brush is still a solid.
        moved.geometry().check_invariant().expect("a valid mesh");
        for vertex in &vertices {
            assert!(
                moved
                    .vertex_positions()
                    .iter()
                    .any(|p| is_same(*p, *vertex + delta)),
                "a vertex did not move"
            );
        }
    }

    #[test]
    fn the_selection_follows_the_handles_it_moved() {
        let brush = cube(-32.0, 32.0);
        let mut manager = collected(HandleKind::Face, std::slice::from_ref(&brush));

        // The top face.
        let top = Vec3d::new(0.0, 0.0, 32.0);
        manager.select(top, false);

        let delta = Vec3d::new(0.0, 0.0, 16.0);
        let moved = brush
            .move_vertices(&manager.selected_vertices(), delta, DRAG_EPSILON)
            .expect("the face moves");

        manager.move_selection(delta);
        manager.collect(HandleKind::Face, std::iter::once(&moved));

        // The handle is where the face went, and it is still selected.
        assert!(manager.is_selected(top + delta));
        assert_eq!(manager.selected_count(), 1);
    }

    #[test]
    fn a_handle_that_is_gone_takes_its_selection_with_it() {
        let brush = cube(-32.0, 32.0);
        let mut manager = collected(HandleKind::Vertex, std::slice::from_ref(&brush));

        manager.select(Vec3d::new(32.0, 32.0, 32.0), false);
        assert_eq!(manager.selected_count(), 1);

        // Collecting from a brush that has no such vertex drops it.
        manager.collect(HandleKind::Vertex, std::iter::once(&cube(-8.0, 8.0)));
        assert_eq!(manager.selected_count(), 0);
    }

    #[test]
    fn each_tool_names_what_it_moves() {
        assert_eq!(HandleKind::of(Tool::Vertex), Some(HandleKind::Vertex));
        assert_eq!(HandleKind::of(Tool::Edge), Some(HandleKind::Edge));
        assert_eq!(HandleKind::of(Tool::Face), Some(HandleKind::Face));
        assert_eq!(HandleKind::of(Tool::Select), None);

        assert_eq!(HandleKind::Vertex.action_name(1), "Move Vertex");
        assert_eq!(HandleKind::Vertex.action_name(2), "Move Vertices");
        assert_eq!(HandleKind::Edge.action_name(3), "Move Edges");
        assert_eq!(HandleKind::Face.action_name(1), "Move Face");
    }
}
