//! Extruding: dragging a face of a selected brush to stretch, slide, split or stamp it.
//!
//! This is not a tool the toolbar switches to. It lives in the tool chain under Shift, so
//! Shift-dragging a face of a selected brush extrudes it whatever else is going on, which
//! is how a room is shaped: draw one box, then push its walls out.
//!
//! What the drag does depends on the other modifiers, and the four are genuinely
//! different operations rather than variations on one:
//!
//! - **Extrude** moves the face's plane, and the brush stretches to follow.
//! - **Slide** (Alt) moves the face's vertices, and the brush does not.
//! - **Split** (Ctrl) leaves the brush alone and makes a new one out of what the face
//!   sweeps -- or, dragged inwards, cuts the brush in two.
//! - **Stamp** (Ctrl+Alt) makes a new brush and leaves the selection on it, so the next
//!   drag carries on from there.

use geometry_math::line::Line3d;
use geometry_math::vec::{self, Vec3d};

use crate::grid::Grid;
use crate::handle_drag::HandlePicker;
use geometry_math::abstract_line::AbstractLine;
use geometry_math::ray::Ray3d;
use geometry_math::scalar::Float;
use map_model::brush::Brush;
use map_model::polyhedron::FaceId;

/// What a Shift-drag on a face does.
///
/// `ExtrudeToolController::acceptMouseDrag`, where Shift is the gate and the rest choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeMode {
    /// Move the face's plane; the brush stretches.
    #[default]
    Extrude,
    /// Move the face's vertices; the brush does not.
    Slide,
    /// Split off what the face sweeps, or cut the brush in two.
    Split,
    /// Stamp out a new brush and leave the original alone.
    Stamp,
}

/// Which of the four a set of modifiers asks for.
///
/// Shift is what lets the drag happen at all and is the caller's to check; these are the
/// keys held with it.
pub fn mode_for(ctrl: bool, alt: bool) -> ExtrudeMode {
    match (ctrl, alt) {
        (true, true) => ExtrudeMode::Stamp,
        (false, true) => ExtrudeMode::Slide,
        (true, false) => ExtrudeMode::Split,
        (false, false) => ExtrudeMode::Extrude,
    }
}

/// Where the dragged face may go.
///
/// `ExtrudeToolController::makePicker` for a face hit: the handle runs along the face's
/// own normal, through the point the press landed on. A face is extruded outwards and
/// inwards and nowhere else.
pub fn picker(hit_point: Vec3d, face_normal: Vec3d, offset: Vec3d) -> HandlePicker {
    HandlePicker::line(
        Line3d {
            point: hit_point,
            direction: face_normal,
        },
        offset,
    )
}

/// Where the handle should come to rest, so that the brush's corners land on the grid.
///
/// The snapper in `ExtrudeToolController::start`. Snapping the distance the face travels
/// is not enough: the corners slide along the edges leaving the face, and those are
/// usually at an angle, so the distance is snapped in terms of where those corners land
/// and converted back.
pub fn snap_handle_position(
    grid: &Grid,
    brush: &Brush,
    face: FaceId,
    face_normal: Vec3d,
    initial: Vec3d,
    proposed: Vec3d,
) -> Vec3d {
    if !grid.snap() {
        return proposed;
    }

    let move_delta = proposed - initial;
    let move_distance = vec::length(move_delta);
    if move_distance <= 0.0 {
        return proposed;
    }
    let move_direction = move_delta / move_distance;

    let on_face_normal = vec::dot(move_delta, face_normal);
    let snapped_on_face_normal = grid.snap_move_distance_for_face(brush, face, on_face_normal);

    // Back from a distance along the face's normal to one along the drag.
    let along_drag = vec::dot(move_direction, face_normal);
    if along_drag.abs() <= f64::EPSILON {
        return proposed;
    }

    initial + move_direction * (snapped_on_face_normal / along_drag)
}

/// One edge where a face turned towards the viewer meets one turned away.
///
/// `EdgeInfo`. In a flat view a face is seen edge-on, so there is nothing to click on;
/// what there is, is the silhouette -- the edges where the brush turns away -- and
/// grabbing one of those is how a face is extruded from a 2D view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonEdge {
    /// Which face of the brush the drag moves.
    pub face_index: usize,
    /// The way that face points, which is the direction the drag counts along.
    pub normal: Vec3d,
    /// The point on the edge nearest the ray, which is where the handle sits.
    pub handle: Vec3d,
    /// How far along the ray the edge is, for choosing between several.
    pub along_ray: f64,
    /// How far the ray passes from the edge, for choosing between several.
    pub from_ray: f64,
}

/// The horizon edge of the given brush nearest the ray, if it has one.
///
/// `getEdgeInfo` and `findClosestHorizonEdge`. An edge is on the horizon when one of the
/// two faces meeting at it faces the viewer and the other does not; when both do, or
/// neither does, the edge is somewhere in the middle of the silhouette rather than on it.
pub fn find_closest_horizon_edge(brush: &Brush, ray: &Ray3d) -> Option<HorizonEdge> {
    let geometry = brush.geometry();
    let mut best: Option<HorizonEdge> = None;

    for edge in geometry.edges() {
        let left_geometry_face = geometry.half_edge(edge.first).face;
        let right_geometry_face = geometry.half_edge(edge.second).face;

        let left_dot = vec::dot(geometry.face_plane(left_geometry_face).normal, ray.direction);
        let right_dot = vec::dot(geometry.face_plane(right_geometry_face).normal, ray.direction);
        if (left_dot < 0.0) == (right_dot < 0.0) {
            // Either both faces are visible or neither is.
            continue;
        }

        let from = geometry.vertex(geometry.half_edge(edge.first).origin).position;
        let to = geometry
            .vertex(geometry.destination(edge.first))
            .position;
        let segment = geometry_math::segment::Segment3d::new(from, to);
        let distance = geometry_math::distance::distance_ray_segment(ray, &segment);
        if distance.parallel {
            continue;
        }

        // `pick2D`: the face to drag is the one turned away from the viewer, or the one
        // seen exactly edge-on.
        let take_left = left_dot >= -f64::ALMOST_ZERO && right_dot.abs() > f64::ALMOST_ZERO;
        let geometry_face = if take_left {
            left_geometry_face
        } else {
            right_geometry_face
        };
        let Some(face_index) = brush.source_face_index(geometry_face.index()) else {
            continue;
        };

        let candidate = HorizonEdge {
            face_index,
            normal: geometry.face_plane(geometry_face).normal,
            handle: segment.point_at_distance(distance.position2),
            along_ray: distance.position1,
            from_ray: distance.distance,
        };
        if best.is_none_or(|best| candidate.from_ray < best.from_ray) {
            best = Some(candidate);
        }
    }

    best
}

/// Where a horizon edge's handle may go.
///
/// The plane faces the viewer, so the pointer can be anywhere on the screen; what it means
/// is how far along the face's own normal it has gone.
pub fn horizon_picker(edge: &HorizonEdge, view_direction: Vec3d, press: &Ray3d) -> HandlePicker {
    let plane = geometry_math::plane::Plane3d::from_anchor_and_normal(edge.handle, view_direction);
    // Preserves the cursor-to-handle offset throughout a drag, so the first movement does not jump
    // to the cursor and a one-grid-step motion changes the face by one grid step.
    let offset = geometry_math::intersection::intersect_ray_plane(press, &plane)
        .map(|distance| edge.handle - press.point_at_distance(distance))
        .unwrap_or(Vec3d::zero());
    HandlePicker::canonical(plane, edge.handle, edge.normal, offset)
}

/// Two adjacent, edge-on faces dragged together in a flat view.
#[derive(Debug, Clone, PartialEq)]
pub struct HorizonCorner {
    /// The two faces, each with the way it points and so the way it moves.
    pub faces: [(usize, Vec3d); 2],
    /// The corner of the brush the two faces meet at, on the view plane.
    pub handle: Vec3d,
}

/// Which two ways a press asks the selection to grow, if it is diagonally outside it.
///
/// The press has to be past the bounds on both of the view's axes: that is what makes it
/// diagonal rather than merely beside, and beside is a single face's drag. `axes` are the
/// two world axes the view shows.
pub fn corner_directions(
    bounds: &geometry_math::bbox::Bbox3d,
    press: Vec3d,
    axes: [Vec3d; 2],
) -> Option<[Vec3d; 2]> {
    let mut directions = [Vec3d::zero(); 2];
    for (direction, axis) in directions.iter_mut().zip(axes) {
        let along = vec::dot(press, axis);
        let a = vec::dot(bounds.min, axis);
        let b = vec::dot(bounds.max, axis);
        if along > a.max(b) {
            *direction = axis;
        } else if along < a.min(b) {
            *direction = -axis;
        } else {
            return None;
        }
    }
    Some(directions)
}

/// Finds two adjacent edge-on faces. Ambiguous or bevelled corners fall back to one face.
pub fn find_horizon_corner(
    brush: &Brush,
    directions: [Vec3d; 2],
    press: Vec3d,
) -> Option<HorizonCorner> {
    let view_direction = vec::cross(directions[0], directions[1]);
    let facing = |direction: Vec3d| {
        brush
            .faces()
            .iter()
            .enumerate()
            .map(|(index, face)| {
                (
                    index,
                    face.plane.normal,
                    vec::dot(face.plane.normal, direction),
                )
            })
            .filter(|(_, normal, towards)| {
                *towards > f64::ALMOST_ZERO
                    && vec::dot(*normal, view_direction).abs() < f64::ALMOST_ZERO
            })
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(index, normal, _)| (index, normal))
    };

    let first = facing(directions[0])?;
    let second = facing(directions[1])?;
    // One face cannot be dragged twice; a brush presenting the same face to both
    // directions has no corner between them.
    if first.0 == second.0 {
        return None;
    }

    // The bounding-box corner need not lie on the brush. Use a real shared edge.
    let geometry = brush.geometry();
    let shared = geometry.edges().iter().find(|edge| {
        let a = brush.source_face_index(geometry.half_edge(edge.first).face.index());
        let b = brush.source_face_index(geometry.half_edge(edge.second).face.index());
        (a == Some(first.0) && b == Some(second.0)) || (a == Some(second.0) && b == Some(first.0))
    })?;
    let anchor = geometry
        .vertex(geometry.half_edge(shared.first).origin)
        .position;
    // The cursor must be outside both actual walls, including slanted ones.
    if [first, second]
        .iter()
        .any(|(_, normal)| vec::dot(press - anchor, *normal) <= f64::ALMOST_ZERO)
    {
        return None;
    }
    let handle = anchor + view_direction * vec::dot(press - anchor, view_direction);

    Some(HorizonCorner {
        faces: [first, second],
        handle,
    })
}

/// Where a corner handle may go.
///
/// Unlike a single face, a corner moves both ways at once, so the pointer's whole
/// movement across the view counts and the picker is a plain plane. What each of the two
/// faces makes of that movement is the part of it along that face's own normal.
pub fn corner_picker(corner: &HorizonCorner, view_direction: Vec3d, press: &Ray3d) -> HandlePicker {
    let plane = geometry_math::plane::Plane3d::from_anchor_and_normal(corner.handle, view_direction);
    let offset = geometry_math::intersection::intersect_ray_plane(press, &plane)
        .map(|distance| corner.handle - press.point_at_distance(distance))
        .unwrap_or(Vec3d::zero());
    HandlePicker::plane(plane, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::bbox::Bbox3d;

    fn world() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    fn cube() -> Brush {
        map_model::shapes::create_cuboid(&world(), &Bbox3d::from_scalars(-32.0, 32.0), "stone")
            .expect("a cube")
    }

    fn top_of(brush: &Brush) -> (FaceId, Vec3d) {
        let geometry = brush.geometry();
        let face = (0..geometry.face_count())
            .map(|index| FaceId(index as u32))
            .find(|face| geometry.face_plane(*face).normal.z() > 0.9)
            .expect("a cube has a top");
        (face, geometry.face_plane(face).normal)
    }

    fn grid(snap: bool) -> Grid {
        let mut grid = Grid::default();
        grid.set_snap(snap);
        grid
    }

    #[test]
    fn corners_work_in_every_flat_view_without_a_press_jump() {
        for depth in 0..3 {
            let axes: Vec<_> = (0..3)
                .filter(|i| *i != depth)
                .map(|i| {
                    let mut axis = Vec3d::zero();
                    axis[i] = 1.0;
                    axis
                })
                .collect();
            for sign_a in [-1.0, 1.0] {
                for sign_b in [-1.0, 1.0] {
                    let directions = [axes[0] * sign_a, axes[1] * sign_b];
                    let mut press = (directions[0] + directions[1]) * 64.0;
                    press[depth] = 200.0;
                    let looking = vec::cross(axes[0], axes[1]);
                    let corner = find_horizon_corner(&cube(), directions, press).unwrap();
                    let picker = corner_picker(&corner, looking, &ray(press, looking));
                    geometry_math::assert_approx_eq!(
                        picker.pick(&ray(press, looking)).unwrap(),
                        corner.handle,
                        1e-6
                    );
                    let delta = directions[0] * 16.0 + directions[1] * 8.0;
                    geometry_math::assert_approx_eq!(
                        picker.pick(&ray(press + delta, looking)).unwrap(),
                        corner.handle + delta,
                        1e-6
                    );
                }
            }
        }
    }

    #[test]
    fn slanted_adjacent_walls_keep_their_angles_and_move_the_real_corner() {
        let brush = cube()
            .transform(
                &world(),
                &geometry_math::mat_ext::rotation_matrix_from_euler(0.0, 0.0, 0.3),
            )
            .unwrap();
        let directions = [Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(0.0, 1.0, 0.0)];
        let corner =
            find_horizon_corner(&brush, directions, Vec3d::new(100.0, 100.0, 32.0)).unwrap();
        let delta = Vec3d::new(16.0, 8.0, 0.0);
        let moved = brush
            .move_boundaries(&world(), &corner.faces.map(|(index, _)| (index, delta)))
            .unwrap();
        assert!(moved
            .geometry()
            .vertices()
            .iter()
            .any(|point| vec::distance(point.position, corner.handle + delta) < 1e-6));
        for face in brush.faces() {
            assert!(moved
                .faces()
                .iter()
                .any(|other| vec::distance(face.plane.normal, other.plane.normal) < 1e-6));
        }
    }

    #[test]
    fn a_bevel_between_the_two_walls_is_not_a_corner() {
        let points: Vec<_> = [-32.0, 32.0]
            .into_iter()
            .flat_map(|z| {
                [
                    (-32.0, -32.0),
                    (32.0, -32.0),
                    (32.0, 16.0),
                    (16.0, 32.0),
                    (-32.0, 32.0),
                ]
                .into_iter()
                .map(move |(x, y)| Vec3d::new(x, y, z))
            })
            .collect();
        let brush = Brush::from_polyhedron(
            map_model::polyhedron::Polyhedron::from_points(&points, 1e-6).unwrap(),
            "stone",
        );
        assert!(find_horizon_corner(
            &brush,
            [Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(0.0, 1.0, 0.0)],
            Vec3d::new(64.0, 64.0, 200.0)
        )
        .is_none());
    }

    #[test]
    fn the_modifiers_choose_between_four_different_operations() {
        assert_eq!(mode_for(false, false), ExtrudeMode::Extrude);
        assert_eq!(mode_for(true, false), ExtrudeMode::Split);
        assert_eq!(mode_for(false, true), ExtrudeMode::Slide);
        assert_eq!(mode_for(true, true), ExtrudeMode::Stamp);
    }

    #[test]
    fn a_face_is_dragged_along_its_own_normal_and_nowhere_else() {
        let hit = Vec3d::new(8.0, 16.0, 32.0);
        let normal = Vec3d::new(0.0, 0.0, 1.0);
        let picker = picker(hit, normal, Vec3d::zero());

        let HandlePicker::Line { line, .. } = picker else {
            panic!("a face runs along a line");
        };
        geometry_math::assert_approx_eq!(line.direction, normal, 1e-9);
        geometry_math::assert_approx_eq!(line.point, hit, 1e-9);
    }

    #[test]
    fn without_snapping_the_handle_goes_where_it_was_asked() {
        let brush = cube();
        let (face, normal) = top_of(&brush);
        let initial = Vec3d::new(0.0, 0.0, 32.0);
        let proposed = Vec3d::new(0.0, 0.0, 45.0);

        let snapped =
            snap_handle_position(&grid(false), &brush, face, normal, initial, proposed);
        geometry_math::assert_approx_eq!(snapped, proposed, 1e-9);
    }

    #[test]
    fn snapping_puts_the_face_of_a_box_on_a_grid_line() {
        let brush = cube();
        let (face, normal) = top_of(&brush);
        let size = grid(true).actual_size();

        let initial = Vec3d::new(0.0, 0.0, 32.0);
        let proposed = initial + normal * (size * 1.3);
        let snapped =
            snap_handle_position(&grid(true), &brush, face, normal, initial, proposed);

        // A box's edges run along the axes, so the face lands a whole grid step away.
        geometry_math::assert_approx_eq!(snapped, initial + normal * size, 1e-6);
    }

    #[test]
    fn a_drag_that_has_not_moved_snaps_to_nothing() {
        let brush = cube();
        let (face, normal) = top_of(&brush);
        let initial = Vec3d::new(0.0, 0.0, 32.0);

        let snapped =
            snap_handle_position(&grid(true), &brush, face, normal, initial, initial);
        geometry_math::assert_approx_eq!(snapped, initial, 1e-9);
    }

    /* ========== the horizon edge ========== */

    fn ray(origin: Vec3d, direction: Vec3d) -> Ray3d {
        Ray3d {
            origin,
            direction: vec::normalize(direction),
        }
    }

    #[test]
    fn a_ray_at_the_top_of_a_cube_finds_the_edge_it_grazes() {
        // Looking along -Y at the cube's top edge, as a front view does.
        let brush = cube();
        let edge = find_closest_horizon_edge(
            &brush,
            &ray(Vec3d::new(0.0, 128.0, 32.0), Vec3d::new(0.0, -1.0, 0.0)),
        )
        .expect("the cube has a silhouette");

        // The handle is on the top edge of the cube.
        geometry_math::assert_approx_eq!(edge.handle.z(), 32.0, 1e-6);
        // And the face it drags is one of the two meeting there.
        let normal = brush.faces()[edge.face_index].plane.normal;
        assert!(
            normal.z().abs() > 0.9 || normal.y().abs() > 0.9,
            "{normal:?}"
        );
    }

    #[test]
    fn a_ray_that_misses_the_brush_still_finds_its_nearest_silhouette_edge() {
        // The pick is by distance from the ray, not by hitting anything, which is what
        // lets an edge be grabbed from just outside the shape.
        let brush = cube();
        let edge = find_closest_horizon_edge(
            &brush,
            &ray(Vec3d::new(0.0, 128.0, 40.0), Vec3d::new(0.0, -1.0, 0.0)),
        )
        .expect("the cube has a silhouette");
        assert!(edge.from_ray > 0.0);
        geometry_math::assert_approx_eq!(edge.handle.z(), 32.0, 1e-6);
    }

    #[test]
    fn the_edge_nearest_the_ray_is_the_one_found() {
        let brush = cube();
        // Along the top edge at +X: the nearest silhouette edge is the one at that corner.
        let edge = find_closest_horizon_edge(
            &brush,
            &ray(Vec3d::new(31.0, 128.0, 31.0), Vec3d::new(0.0, -1.0, 0.0)),
        )
        .expect("the cube has a silhouette");
        assert!(edge.from_ray < 4.0, "{}", edge.from_ray);
    }

    /// A resize follows how far the pointer moved, not where it is.
    ///
    /// Pressing is never exactly on the edge -- a few pixels away is several units out in
    /// a zoomed-out view -- and measuring from the edge to the pointer makes the face
    /// jump that gap the moment the drag starts, before the pointer has moved at all.
    /// Moving the pointer one step means
    /// the face moves one step.
    #[test]
    fn a_resize_follows_the_pointers_movement_and_not_its_position() {
        let brush = cube();
        let looking = Vec3d::new(0.0, -1.0, 0.0);
        // Pressed well off the edge, as a press always is.
        let press = ray(Vec3d::new(24.0, 128.0, 48.0), looking);
        let edge = find_closest_horizon_edge(&brush, &press).expect("a silhouette");
        let picker = horizon_picker(&edge, looking, &press);

        // Still pressed: nothing has moved, so the handle has not moved either.
        let held = picker.pick(&press).expect("the ray meets the plane");
        geometry_math::assert_approx_eq!(vec::distance(held, edge.handle), 0.0, 1e-6);

        // Moved sixteen units along the face's normal: the handle moves sixteen, not the
        // distance from the edge to wherever the pointer happens to be.
        let moved_by = edge.normal * 16.0;
        let after = picker
            .pick(&ray(press.origin + moved_by, looking))
            .expect("the ray meets the plane");
        geometry_math::assert_approx_eq!(vec::distance(after, edge.handle + moved_by), 0.0, 1e-6);
    }

    #[test]
    fn the_handle_only_moves_along_the_face_it_drags() {
        let brush = cube();
        let looking = Vec3d::new(0.0, -1.0, 0.0);
        let edge = find_closest_horizon_edge(
            &brush,
            &ray(Vec3d::new(0.0, 128.0, 32.0), looking),
        )
        .expect("the cube has a silhouette");

        let press = ray(Vec3d::new(0.0, 128.0, 32.0), looking);
        let picker = horizon_picker(&edge, looking, &press);
        // The pointer has moved sideways as well as along the normal; only the latter
        // counts, so the handle stays on the line through it.
        let moved = picker
            .pick(&ray(
                edge.handle + Vec3d::new(40.0, 128.0, 16.0),
                looking,
            ))
            .expect("the ray meets the plane");
        let along = moved - edge.handle;
        let sideways = along - edge.normal * vec::dot(along, edge.normal);
        geometry_math::assert_approx_eq!(vec::length(sideways), 0.0, 1e-6);
    }

    #[test]
    fn snapping_works_the_same_pulling_a_face_inwards() {
        let brush = cube();
        let (face, normal) = top_of(&brush);
        let size = grid(true).actual_size();

        let initial = Vec3d::new(0.0, 0.0, 32.0);
        let proposed = initial - normal * (size * 1.3);
        let snapped =
            snap_handle_position(&grid(true), &brush, face, normal, initial, proposed);

        geometry_math::assert_approx_eq!(snapped, initial - normal * size, 1e-6);
    }
}
