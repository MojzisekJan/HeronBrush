//! The editor grid.
//!
//! Grid snapping supports distinct rounding, inward and outward operations. Editing tools share
//! these rules so negative coordinates and boundary cases behave consistently.
#![allow(
    dead_code,
    reason = "shared grid operations include helpers used only by specialised tools"
)]
//!
//! The editor stores the grid size as a power of two exponent rather than a length, so
//! that halving and doubling it are single increments and every grid line always falls on
//! a representable coordinate.

use geometry_math::abstract_line::AbstractLine;
use geometry_math::line::Line3d;
use geometry_math::plane::Plane3d;
use geometry_math::scalar::{self, Float};
use geometry_math::vec::Vec3d;
use map_model::brush::Brush;
use map_model::polyhedron::FaceId;

/// The smallest grid exponent the editor offers, i.e. a grid of 2^-3 = 0.125 units.
pub const MIN_SIZE: i32 = -3;
/// The largest grid exponent the editor offers, i.e. a grid of 2^8 = 256 units.
pub const MAX_SIZE: i32 = 8;

/// The editor grid, sized as a power of two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    size: i32,
    visible: bool,
    snap: bool,
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(4)
    }
}

impl Grid {
    /// Creates a grid with the given exponent, clamped to the supported range.
    pub fn new(size: i32) -> Self {
        Self {
            size: size.clamp(MIN_SIZE, MAX_SIZE),
            visible: true,
            snap: true,
        }
    }

    /// Returns this grid at the given exponent, clamped to the supported range.
    pub fn with_size(&self, size: i32) -> Self {
        Self {
            size: size.clamp(MIN_SIZE, MAX_SIZE),
            ..*self
        }
    }

    /// Returns the grid exponent.
    pub fn size(&self) -> i32 {
        self.size
    }

    /// Returns the actual grid spacing in world units, i.e. 2 raised to the exponent.
    pub fn actual_size(&self) -> f64 {
        2.0f64.powi(self.size)
    }

    /// Doubles the grid spacing, up to [`MAX_SIZE`].
    pub fn increase(&mut self) {
        self.size = (self.size + 1).min(MAX_SIZE);
    }

    /// Halves the grid spacing, down to [`MIN_SIZE`].
    pub fn decrease(&mut self) {
        self.size = (self.size - 1).max(MIN_SIZE);
    }

    /// Returns the angle snapping increment, in radians.
    ///
    /// A rotation step of 15 degrees gives twenty-four orientations per full turn.
    pub fn angle(&self) -> f64 {
        15.0f64.to_radians()
    }

    /// Snaps an angle to a multiple of the increment, if snapping is on.
    ///
    /// `Grid::snapAngle`.
    pub fn snap_angle(&self, a: f64) -> f64 {
        self.snap_angle_to(a, self.angle())
    }

    /// Snaps an angle to a multiple of the given increment, if snapping is on.
    pub fn snap_angle_to(&self, a: f64, snap_angle: f64) -> f64 {
        if self.snap() && snap_angle != 0.0 {
            snap_angle * (a / snap_angle).round()
        } else {
            a
        }
    }

    /// Returns whether grid lines are drawn.
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Sets whether grid lines are drawn.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Returns whether snapping is enabled.
    pub fn snap(&self) -> bool {
        self.snap
    }

    /// Sets whether snapping is enabled.
    pub fn set_snap(&mut self, snap: bool) {
        self.snap = snap;
    }

    /// Snaps a scalar to the nearest grid line, or returns it unchanged when snapping is
    /// off.
    pub fn snap_scalar(&self, value: f64) -> f64 {
        if self.snap {
            scalar::snap(value, self.actual_size())
        } else {
            value
        }
    }

    /// Rounds a scalar down to the grid line at or below it.
    ///
    /// Uses `floor`, unlike [`geometry_math::scalar::snap_down`], which rounds towards zero. Expanding a box
    /// must always move its minimum towards negative infinity.
    ///
    /// With `skip`, a value already on a grid line moves to the next one down, which is
    /// what a ray cast wants so it does not stop where it started.
    pub fn snap_down_scalar(&self, value: f64, skip: bool) -> f64 {
        if !self.snap {
            return value;
        }
        let size = self.actual_size();
        let snapped = size * (value / size).floor();
        if skip && (snapped - value).abs() < f64::ALMOST_ZERO {
            snapped - size
        } else {
            snapped
        }
    }

    /// Rounds a scalar up to the grid line at or above it, the counterpart of
    /// [`Grid::snap_down_scalar`].
    pub fn snap_up_scalar(&self, value: f64, skip: bool) -> f64 {
        if !self.snap {
            return value;
        }
        let size = self.actual_size();
        let snapped = size * (value / size).ceil();
        if skip && (snapped - value).abs() < f64::ALMOST_ZERO {
            snapped + size
        } else {
            snapped
        }
    }

    /// Rounds each component of a point down to the grid.
    pub fn snap_down_point(&self, point: Vec3d) -> Vec3d {
        Vec3d::new(
            self.snap_down_scalar(point.x(), false),
            self.snap_down_scalar(point.y(), false),
            self.snap_down_scalar(point.z(), false),
        )
    }

    /// Rounds each component of a point up to the grid.
    pub fn snap_up_point(&self, point: Vec3d) -> Vec3d {
        Vec3d::new(
            self.snap_up_scalar(point.x(), false),
            self.snap_up_scalar(point.y(), false),
            self.snap_up_scalar(point.z(), false),
        )
    }

    /// Snaps each component of a point outwards along the given direction.
    ///
    /// A component with a positive direction rounds up, a negative one rounds down, and a
    /// zero one rounds to the nearest line. This is `Grid::snapTowards`, and it is what
    /// makes a handle dragged outwards land on the line beyond it rather than snapping
    /// back past the pointer.
    pub fn snap_towards(&self, point: Vec3d, direction: Vec3d, skip: bool) -> Vec3d {
        if !self.snap {
            return point;
        }
        let mut result = Vec3d::zero();
        for i in 0..3 {
            result[i] = if direction[i] > 0.0 {
                self.snap_up_scalar(point[i], skip)
            } else if direction[i] < 0.0 {
                self.snap_down_scalar(point[i], skip)
            } else {
                self.snap_scalar(point[i])
            };
        }
        result
    }

    /// Returns how far a point may actually move, given how far the drag asked it to.
    ///
    /// The destination is snapped to the grid, and then any component that would end up
    /// moving *against* the drag is dropped. This is `Grid::moveDeltaForPoint`, and it is
    /// what stops a small drag from jerking an object backwards to the nearer grid line —
    /// an object only ever moves the way the pointer went, or not at all.
    pub fn move_delta_for_point(&self, point: Vec3d, delta: Vec3d) -> Vec3d {
        let new_point = self.snap_point(point + delta);
        let mut actual = new_point - point;

        for i in 0..3 {
            if (actual[i] > 0.0) != (delta[i] > 0.0) {
                actual[i] = 0.0;
            }
        }
        actual
    }

    /// Returns how far a box must move to sit on a plane under a ray, on the grid.
    ///
    /// Snaps the ray-plane hit away from the viewer and aligns the box corner facing both the plane
    /// and the viewer. On the dominant axis, the box lies on the normal-facing side of the plane;
    /// on the other axes, it extends away from the ray origin.
    ///
    /// The ray missing the plane means there is nowhere to put the box, and it does not
    /// move.
    pub fn move_delta_for_bounds(
        &self,
        target_plane: &Plane3d,
        bounds: &geometry_math::bbox::Bbox3d,
        ray: &geometry_math::ray::Ray3d,
    ) -> Vec3d {
        let Some(distance) = geometry_math::intersection::intersect_ray_plane(ray, target_plane)
        else {
            return Vec3d::zero();
        };
        let hit = ray.origin + ray.direction * distance;

        // A local frame where z is the plane normal's largest component and x and y the
        // other two, largest first.
        let local_z = target_plane.normal.find_abs_max_component(0);
        let local_x = target_plane.normal.find_abs_max_component(1);
        let local_y = target_plane.normal.find_abs_max_component(2);

        let mut corner = self.snap_towards(hit, -ray.direction, false);
        if geometry_math::vec::is_equal(
            &target_plane.normal,
            &target_plane.normal.get_abs_max_component_axis(0),
            f64::ALMOST_ZERO,
        ) {
            // An axial plane: snapping along its own axis would lift the box off it, so
            // that coordinate is the hit point as it is.
            corner[local_z] = hit[local_z];
        }

        let size = bounds.size();
        let mut min = corner;
        if target_plane.normal[local_z] < 0.0 {
            min[local_z] -= size[local_z];
        }
        if ray.direction[local_x] < 0.0 {
            min[local_x] -= size[local_x];
        }
        if ray.direction[local_y] < 0.0 {
            min[local_y] -= size[local_y];
        }

        min - bounds.min
    }

    /// Returns the point a box is measured from when it is moved.
    ///
    /// Its centre, snapped — so a box whose size is an odd number of grid cells still
    /// moves in whole cells.
    pub fn reference_point(&self, bounds: &geometry_math::bbox::Bbox3d) -> Vec3d {
        self.snap_point(bounds.center())
    }

    /// Snaps a point to the nearest grid intersection, or returns it unchanged when
    /// snapping is off.
    pub fn snap_point(&self, point: Vec3d) -> Vec3d {
        Vec3d::new(
            self.snap_scalar(point.x()),
            self.snap_scalar(point.y()),
            self.snap_scalar(point.z()),
        )
    }

    /// The distance along a line that lands nearest a grid plane.
    ///
    /// `Grid::snapToGridPlane`. Given a point on the line, the nearest grid corner to it
    /// has three grid planes meeting at it; the answer is whichever of those three the
    /// line meets closest to where it started.
    pub fn snap_to_grid_plane(&self, line: &Line3d, distance: f64) -> f64 {
        let mut snapped = f64::MAX;

        // A point on the line, and the corner of the grid nearest it.
        let x = line.point_at_distance(distance);
        let c = self.snap_point(x);

        for axis in 0..3 {
            let mut normal = Vec3d::zero();
            normal[axis] = 1.0;
            let plane = Plane3d::from_anchor_and_normal(c, normal);
            if let Some(y) = geometry_math::intersection::intersect_line_plane(line, &plane) {
                if (y - distance).abs() < (snapped - distance).abs() {
                    snapped = y;
                }
            }
        }

        snapped
    }

    /// How far a face's plane should move so that the brush's corners land on the grid.
    ///
    /// `Grid::snapMoveDistanceForFace`. Extruding a face does not move its own vertices
    /// along its normal: they slide along the edges that leave the face, and those edges
    /// are usually at an angle. Snapping the distance the plane travels would therefore
    /// leave the corners off the grid. So each vertex is followed along each edge that
    /// leaves the face, that distance is snapped to a grid plane, and converted back into
    /// a distance along the normal; the one that changes the move least is the answer.
    ///
    /// Returns the distance unchanged when the face has no such edge to follow, which a
    /// well-formed brush does not.
    pub fn snap_move_distance_for_face(
        &self,
        brush: &Brush,
        face: FaceId,
        move_distance: f64,
    ) -> f64 {
        let geometry = brush.geometry();
        let move_direction = geometry.face_plane(face).normal;

        // The face's own edges are not followed: they lie in the face and move with it.
        let face_edges: Vec<_> = geometry
            .face_half_edges(face)
            .into_iter()
            .map(|half_edge| geometry.half_edge(half_edge).edge)
            .collect();

        let mut snapped = f64::MAX;
        for vertex in geometry.face_vertices(face) {
            for half_edge_id in geometry.vertex_half_edges(vertex) {
                let half_edge = geometry.half_edge(half_edge_id);
                if face_edges.contains(&half_edge.edge) {
                    continue;
                }

                let origin = geometry.vertex(half_edge.origin).position;
                let destination = geometry.vertex(geometry.destination(half_edge_id)).position;
                let along_edge = destination - origin;
                let length = geometry_math::vec::length(along_edge);
                if length <= f64::ALMOST_ZERO {
                    continue;
                }
                let edge_direction = along_edge / length;

                // How far the vertex travels along its edge for the face to move this far.
                let projection = geometry_math::vec::dot(edge_direction, move_direction);
                if projection.abs() <= f64::ALMOST_ZERO {
                    // The edge runs across the face's normal, so following it moves the
                    // face nowhere.
                    continue;
                }
                let distance_on_edge = move_distance / projection;

                let line = Line3d {
                    point: origin,
                    direction: edge_direction,
                };
                let snapped_on_edge = self.snap_to_grid_plane(&line, distance_on_edge);

                // Back into a distance along the face's normal, keeping whichever changes
                // the asked-for move least.
                let snapped_for_edge = snapped_on_edge * projection;
                if (snapped_for_edge - move_distance).abs() < (snapped - move_distance).abs() {
                    snapped = snapped_for_edge;
                }
            }
        }

        if snapped == f64::MAX {
            move_distance
        } else {
            snapped
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_brush() -> Brush {
        map_model::shapes::create_cuboid(
            &geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0),
            &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
            "stone",
        )
        .expect("a cube")
    }

    fn top_face(brush: &Brush) -> FaceId {
        let geometry = brush.geometry();
        (0..geometry.face_count())
            .map(|index| FaceId(index as u32))
            .find(|face| geometry.face_plane(*face).normal.z() > 0.9)
            .expect("a cube has a top")
    }

    #[test]
    fn snapping_to_a_grid_plane_lands_on_one() {
        let grid = Grid::default();
        // A line up the Z axis: the grid planes it meets are at multiples of the grid size.
        let line = Line3d {
            point: Vec3d::zero(),
            direction: Vec3d::new(0.0, 0.0, 1.0),
        };

        let size = grid.actual_size();
        let snapped = grid.snap_to_grid_plane(&line, size * 1.3);
        geometry_math::assert_approx_eq!(snapped, size, 1e-9);

        let snapped = grid.snap_to_grid_plane(&line, size * 1.7);
        geometry_math::assert_approx_eq!(snapped, size * 2.0, 1e-9);
    }

    #[test]
    fn snapping_to_a_grid_plane_takes_the_nearest_of_the_three() {
        let grid = Grid::default();
        let size = grid.actual_size();

        // A diagonal line meets a different grid plane along each axis; the answer is
        // whichever comes nearest the distance asked for.
        let line = Line3d {
            point: Vec3d::zero(),
            direction: geometry_math::vec::normalize(Vec3d::new(1.0, 2.0, 4.0)),
        };
        let asked = size * 1.5;
        let snapped = grid.snap_to_grid_plane(&line, asked);

        // The point it lands on has at least one component on a grid line.
        let landed = line.point_at_distance(snapped);
        let on_grid = (0..3).any(|axis| {
            let value = landed[axis];
            (value / size - (value / size).round()).abs() < 1e-9
        });
        assert!(on_grid, "{landed:?}");
        assert!((snapped - asked).abs() < size, "{snapped} vs {asked}");
    }

    #[test]
    fn a_face_of_a_box_snaps_the_way_a_point_would() {
        // A box's edges run along the axes, so following one is the same as following the
        // face's own normal: the answer is the plain grid snap.
        let grid = Grid::default();
        let brush = cube_brush();
        let face = top_face(&brush);
        let size = grid.actual_size();

        let snapped = grid.snap_move_distance_for_face(&brush, face, size * 1.3);
        geometry_math::assert_approx_eq!(snapped, size, 1e-6);
    }

    #[test]
    fn a_face_that_is_already_on_the_grid_does_not_move_at_all() {
        let grid = Grid::default();
        let brush = cube_brush();
        let face = top_face(&brush);

        let snapped = grid.snap_move_distance_for_face(&brush, face, 0.0);
        geometry_math::assert_approx_eq!(snapped, 0.0, 1e-6);
    }

    #[test]
    fn snapping_a_face_of_a_sloped_brush_puts_its_corners_on_the_grid() {
        // A cone's sides meet its base at an angle, so moving the base by a round number
        // would leave its corners between grid lines. The answer is the distance that
        // does put one there.
        let grid = Grid::default();
        let brush = map_model::shapes::create_cone(
            &geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0),
            &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
            map_model::shapes::CircleShape::EdgeAligned { num_sides: 4 },
            geometry_math::util::Axis::Z,
            "stone",
        )
        .expect("a cone");

        let geometry = brush.geometry();
        let base = (0..geometry.face_count())
            .map(|index| FaceId(index as u32))
            .find(|face| geometry.face_plane(*face).normal.z() < -0.9)
            .expect("a cone has a base");

        let asked = grid.actual_size() * 1.3;
        let snapped = grid.snap_move_distance_for_face(&brush, base, asked);
        // It moved somewhere sensible: a whole grid step away at most.
        assert!(
            (snapped - asked).abs() <= grid.actual_size(),
            "{snapped} vs {asked}"
        );
    }

    #[test]
    fn actual_size_is_a_power_of_two() {
        assert_eq!(Grid::new(0).actual_size(), 1.0);
        assert_eq!(Grid::new(4).actual_size(), 16.0);
        assert_eq!(Grid::new(-3).actual_size(), 0.125);
    }

    #[test]
    fn size_is_clamped_to_the_supported_range() {
        assert_eq!(Grid::new(100).size(), MAX_SIZE);
        assert_eq!(Grid::new(-100).size(), MIN_SIZE);

        let mut grid = Grid::new(MAX_SIZE);
        grid.increase();
        assert_eq!(grid.size(), MAX_SIZE);

        let mut grid = Grid::new(MIN_SIZE);
        grid.decrease();
        assert_eq!(grid.size(), MIN_SIZE);
    }

    #[test]
    fn snapping_rounds_to_the_nearest_grid_line() {
        let grid = Grid::new(4); // 16 units
        assert_eq!(grid.snap_scalar(7.0), 0.0);
        assert_eq!(grid.snap_scalar(9.0), 16.0);
        assert_eq!(grid.snap_scalar(-9.0), -16.0);
        assert_eq!(
            grid.snap_point(Vec3d::new(9.0, -9.0, 7.0)),
            Vec3d::new(16.0, -16.0, 0.0)
        );
    }

    #[test]
    fn a_move_never_goes_against_the_drag() {
        // The rule that stops a small drag from jerking an object backwards.
        let grid = Grid::new(4); // 16 units
        let point = Vec3d::new(4.0, 0.0, 0.0);

        // Dragging right by 1 would snap back to 0, which is the wrong way, so nothing
        // moves.
        let delta = grid.move_delta_for_point(point, Vec3d::new(1.0, 0.0, 0.0));
        assert_eq!(delta.x(), 0.0);

        // Dragging right far enough to reach the next line does move.
        let delta = grid.move_delta_for_point(point, Vec3d::new(9.0, 0.0, 0.0));
        assert_eq!(delta.x(), 12.0, "4 + 12 = 16, the next line");

        // And the same the other way.
        let delta = grid.move_delta_for_point(point, Vec3d::new(-1.0, 0.0, 0.0));
        assert_eq!(delta.x(), -4.0, "4 - 4 = 0, the line below");
    }

    #[test]
    fn a_move_lands_the_point_on_the_grid() {
        let grid = Grid::new(4);
        let point = Vec3d::new(5.0, -3.0, 100.0);
        let delta = grid.move_delta_for_point(point, Vec3d::new(40.0, -40.0, 40.0));

        let moved = point + delta;
        for axis in 0..3 {
            assert_eq!(moved[axis] % 16.0, 0.0, "axis {axis}: {}", moved[axis]);
        }
    }

    #[test]
    fn a_move_of_nothing_still_pulls_a_point_onto_the_grid() {
        // Not an oversight: the sign test only drops a component that moves *against* the
        // drag, and with no drag at all "against" is only the positive direction. So a
        // point above a grid line is pulled down onto it, and one below is left alone.
        // That asymmetry is why beginning a drag on an
        // off-grid object tugs it into line.
        let grid = Grid::new(4); // 16 units

        // 5 snaps down to 0.
        assert_eq!(
            grid.move_delta_for_point(Vec3d::new(5.0, 5.0, 5.0), Vec3d::zero()),
            Vec3d::new(-5.0, -5.0, -5.0)
        );

        // 12 would snap up to 16, which counts as moving the wrong way, so it stays.
        assert_eq!(
            grid.move_delta_for_point(Vec3d::new(12.0, 12.0, 12.0), Vec3d::zero()),
            Vec3d::zero()
        );
    }

    #[test]
    fn snapping_towards_a_direction_goes_that_way() {
        let grid = Grid::new(4);
        let point = Vec3d::new(5.0, 5.0, 5.0);

        // Outwards on x, inwards on y, nearest on z.
        let snapped = grid.snap_towards(point, Vec3d::new(1.0, -1.0, 0.0), false);
        assert_eq!(snapped, Vec3d::new(16.0, 0.0, 0.0));
    }

    #[test]
    fn the_reference_point_of_a_box_is_its_snapped_centre() {
        use geometry_math::bbox::Bbox3d;

        let grid = Grid::new(4);
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(40.0, 40.0, 40.0));
        assert_eq!(grid.reference_point(&bounds), Vec3d::new(16.0, 16.0, 16.0));
    }

    #[test]
    fn snapping_a_box_outwards_grows_it_to_whole_cells() {
        // This is what drawing a brush needs: whatever was dragged has to end up inside
        // the result, so the minimum goes down and the maximum up.
        let grid = Grid::new(4); // 16 units

        assert_eq!(grid.snap_down_scalar(1.0, false), 0.0);
        assert_eq!(grid.snap_up_scalar(1.0, false), 16.0);
        assert_eq!(grid.snap_down_scalar(15.9, false), 0.0);
        assert_eq!(grid.snap_up_scalar(0.1, false), 16.0);

        // A value already on the grid stays put.
        assert_eq!(grid.snap_down_scalar(32.0, false), 32.0);
        assert_eq!(grid.snap_up_scalar(32.0, false), 32.0);
    }

    #[test]
    fn snapping_down_goes_towards_negative_infinity_not_towards_zero() {
        // The distinction that `geometry_math::scalar::snap_down` gets the other way round, and the
        // one that decides whether a box drawn left of the origin grows or shrinks.
        let grid = Grid::new(4);

        assert_eq!(grid.snap_down_scalar(-1.0, false), -16.0);
        assert_eq!(grid.snap_up_scalar(-1.0, false), 0.0);
        assert_eq!(grid.snap_down_scalar(-17.0, false), -32.0);
        assert_eq!(grid.snap_up_scalar(-17.0, false), -16.0);

        // Which is not what rounding towards zero would give.
        assert_eq!(geometry_math::scalar::snap_down(-1.0, 16.0), 0.0);
    }

    #[test]
    fn skipping_moves_off_a_value_already_on_the_grid() {
        let grid = Grid::new(4);
        assert_eq!(grid.snap_down_scalar(32.0, true), 16.0);
        assert_eq!(grid.snap_up_scalar(32.0, true), 48.0);
    }

    #[test]
    fn snapping_a_point_outwards_covers_it() {
        let grid = Grid::new(4);
        let point = Vec3d::new(1.0, -1.0, 17.0);

        let down = grid.snap_down_point(point);
        let up = grid.snap_up_point(point);
        for axis in 0..3 {
            assert!(down[axis] <= point[axis], "axis {axis}");
            assert!(up[axis] >= point[axis], "axis {axis}");
        }
    }

    #[test]
    fn outward_snapping_respects_the_snap_toggle() {
        let mut grid = Grid::new(4);
        grid.set_snap(false);
        assert_eq!(grid.snap_down_scalar(1.0, false), 1.0);
        assert_eq!(grid.snap_up_scalar(1.0, false), 1.0);
    }

    #[test]
    fn snapping_can_be_disabled() {
        let mut grid = Grid::new(4);
        grid.set_snap(false);
        assert_eq!(grid.snap_scalar(7.0), 7.0);
        assert_eq!(
            grid.snap_point(Vec3d::new(9.0, -9.0, 7.0)),
            Vec3d::new(9.0, -9.0, 7.0)
        );
    }

    // Placement deltas for snapping entity bounds to a surface.
    mod move_delta_for_bounds {
        use super::*;
        use geometry_math::bbox::Bbox3d;
        use geometry_math::ray::Ray3d;

        fn grid16() -> Grid {
            Grid::new(4)
        }

        fn box_of_odd_size() -> Bbox3d {
            Bbox3d::new(Vec3d::zero(), Vec3d::new(95.0, 100.0, 105.0))
        }

        fn ray_from_to(from: Vec3d, to: Vec3d) -> Ray3d {
            Ray3d {
                origin: from,
                direction: geometry_math::vec::normalize(to - from),
            }
        }

        fn close(actual: Vec3d, expected: Vec3d) -> bool {
            geometry_math::vec::is_equal(&actual, &expected, 1e-6)
        }

        #[test]
        fn a_box_dropped_on_the_floor_lands_beyond_the_pointer() {
            // Looking towards +x +y: the hit snaps back towards the camera, and the box
            // starts there and runs away from it, so its min is the snapped corner.
            let floor = Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
            let ray = ray_from_to(
                Vec3d::new(512.0, 512.0, 200.0),
                Vec3d::new(1016.0, 1016.0, 0.0),
            );
            let delta = grid16().move_delta_for_bounds(&floor, &box_of_odd_size(), &ray);
            assert!(close(delta, Vec3d::new(1008.0, 1008.0, 0.0)), "{delta:?}");
        }

        #[test]
        fn looking_the_other_way_puts_the_box_on_the_other_side_of_the_corner() {
            let floor = Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
            let ray = ray_from_to(Vec3d::new(512.0, 512.0, 200.0), Vec3d::new(8.0, 8.0, 0.0));
            // The corner still rounds towards the camera, to (16, 16), but the box hangs
            // off it away from the camera, so its min is a box-size back.
            let size = box_of_odd_size().size();
            let delta = grid16().move_delta_for_bounds(&floor, &box_of_odd_size(), &ray);
            assert!(
                close(delta, Vec3d::new(16.0 - size.x(), 16.0 - size.y(), 0.0)),
                "{delta:?}"
            );
        }

        #[test]
        fn a_ray_along_the_plane_moves_nothing() {
            let floor = Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
            let ray = ray_from_to(Vec3d::new(0.0, 0.0, 50.0), Vec3d::new(10.0, 0.0, 50.0));
            assert_eq!(
                grid16().move_delta_for_bounds(&floor, &box_of_odd_size(), &ray),
                Vec3d::zero()
            );
        }

        #[test]
        fn a_box_dropped_on_a_ceiling_hangs_from_it() {
            let ceiling = Plane3d::from_anchor_and_normal(
                Vec3d::new(0.0, 0.0, 512.0),
                Vec3d::new(0.0, 0.0, -1.0),
            );
            let ray = ray_from_to(
                Vec3d::new(50.0, 50.0, 200.0),
                Vec3d::new(1016.0, 1016.0, 512.0),
            );
            let size = box_of_odd_size().size();
            let delta = grid16().move_delta_for_bounds(&ceiling, &box_of_odd_size(), &ray);
            assert!(
                close(delta, Vec3d::new(1008.0, 1008.0, 512.0 - size.z())),
                "{delta:?}"
            );
        }

        #[test]
        fn a_platform_off_the_grid_keeps_the_box_on_its_surface() {
            // The height is not a grid multiple, and the box still sits on the platform
            // rather than being snapped down through it.
            let platform = Plane3d::from_anchor_and_normal(
                Vec3d::new(0.0, 0.0, 4.0),
                Vec3d::new(0.0, 0.0, 1.0),
            );
            let ray = ray_from_to(Vec3d::new(0.0, 0.0, 200.0), Vec3d::new(17.0, 17.0, 4.0));
            let delta = grid16().move_delta_for_bounds(&platform, &box_of_odd_size(), &ray);
            assert!(close(delta, Vec3d::new(16.0, 16.0, 4.0)), "{delta:?}");
        }

        #[test]
        fn a_box_dropped_on_a_slope_floats_above_it() {
            // A sloped plane is not axial, so the height is snapped like the other two
            // and the box ends up above the surface rather than cutting into it.
            let slope = geometry_math::plane::from_points(
                Vec3d::zero(),
                Vec3d::new(0.0, 100.0, 5.0),
                Vec3d::new(100.0, 0.0, 0.0),
            )
            .expect("three points that are not in a line");
            assert!(slope.normal.z() > 0.0);

            let ray = ray_from_to(Vec3d::new(0.0, 0.0, 200.0), Vec3d::new(17.0, 17.0, 0.0));
            let delta = grid16().move_delta_for_bounds(&slope, &box_of_odd_size(), &ray);
            assert!(close(delta, Vec3d::new(16.0, 16.0, 16.0)), "{delta:?}");
        }
    }
}
