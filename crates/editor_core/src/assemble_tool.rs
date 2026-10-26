//! Building a brush by clicking the points it should enclose.
//!
//! Every other way of making a brush starts from a box. This one starts from nothing: the
//! points are clicked one at a time, hulled as they go, and once they enclose a volume
//! there is a brush. That is what it is for -- a wedge, a ramp, anything whose shape is
//! easier to point at than to drag a box into.
//!
//! The tool holds only the points. The hull is worked out from them each time, because a
//! point added inside what is already there changes nothing and a point outside changes
//! everything, and neither is worth trying to track incrementally.

use geometry_math::bbox::Bbox3d;
use geometry_math::intersection;
use geometry_math::plane::Plane3d;
use geometry_math::ray::Ray3d;
use geometry_math::scalar::Float;
use geometry_math::vec;
use geometry_math::vec::Vec2d;
use geometry_math::vec::Vec3d;
use map_model::brush::Brush;
use map_model::polyhedron::Polyhedron;

use crate::grid::Grid;

/// How close two points have to be to count as the same one.
const SAME_POINT: f64 = 0.001;

/// The points a brush is being assembled from.
#[derive(Debug, Clone, Default)]
pub struct AssembleBrushTool {
    points: Vec<Vec3d>,
}

impl AssembleBrushTool {
    /// Creates a tool with nothing placed yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the points placed so far, for drawing them.
    pub fn points(&self) -> &[Vec3d] {
        &self.points
    }

    /// Adds a point, unless one is already there.
    ///
    /// Returns whether it was added. Two points in the same place would make the hull no
    /// larger and give the drawing two handles that cannot be told apart.
    pub fn add_point(&mut self, point: Vec3d) -> bool {
        if self
            .points
            .iter()
            .any(|placed| vec::squared_distance(*placed, point) <= SAME_POINT * SAME_POINT)
        {
            return false;
        }
        self.points.push(point);
        true
    }

    /// Takes back the last point placed.
    pub fn remove_last_point(&mut self) -> bool {
        self.points.pop().is_some()
    }

    /// Forgets every point, which is what leaving the tool and committing both do.
    pub fn reset(&mut self) {
        self.points.clear();
    }

    /// The solid the points enclose, if they enclose one.
    ///
    /// `AssembleBrushTool::update`, which builds a brush only once the polyhedron is
    /// closed: fewer than four points, or four that lie in a plane, enclose nothing.
    pub fn hull(&self) -> Option<Polyhedron<f64>> {
        if self.points.len() < 4 {
            return None;
        }
        Polyhedron::from_points(&self.points, f64::ALMOST_ZERO)
            .ok()
            .filter(|hull| hull.is_closed())
    }

    /// The brush the points make, if they make one.
    pub fn brush(&self, _world_bounds: &Bbox3d, material: &str) -> Option<Brush> {
        self.hull()
            .map(|hull| Brush::from_polyhedron(hull, material))
    }

    /// Puts back the points as they were, for a drag that is undone or measured again
    /// from where it began.
    pub fn set_points(&mut self, points: &[Vec3d]) {
        self.points.clear();
        for point in points {
            self.add_point(*point);
        }
    }

    /// The edges of the solid so far, for drawing what the points have come to. Points
    /// that lie in one plane are drawn as the outline of the polygon they make.
    pub fn edges(&self) -> Vec<(Vec3d, Vec3d)> {
        if let Some(hull) = self.hull() {
            return Brush::from_polyhedron(hull, "").edge_positions();
        }
        let Some((ring, _)) = self.polygon() else {
            return Vec::new();
        };
        (0..ring.len())
            .map(|index| (ring[index], ring[(index + 1) % ring.len()]))
            .collect()
    }

    /// The polygon the points make when they lie in one plane without lying on one line:
    /// its corners in order around it, and its normal. `Polyhedron3::polygon`, the state
    /// a face drawn by dragging leaves the tool in.
    pub fn polygon(&self) -> Option<(Vec<Vec3d>, Vec3d)> {
        let first = *self.points.first()?;
        let second = *self
            .points
            .iter()
            .find(|point| vec::squared_distance(**point, first) > SAME_POINT * SAME_POINT)?;
        let normal = self.points.iter().find_map(|point| {
            let normal = vec::cross(second - first, *point - first);
            (vec::length(normal) > SAME_POINT).then(|| vec::normalize(normal))
        })?;
        let plane = Plane3d::from_anchor_and_normal(first, normal);
        if self
            .points
            .iter()
            .any(|point| plane.point_distance(*point).abs() > SAME_POINT)
        {
            return None;
        }
        Some((convex_ring(&self.points, normal), normal))
    }

    /// Where `ray` meets the polygon the points make, as a distance along it, and the
    /// polygon's normal facing the ray. `Polyhedron3::pickFace`.
    pub fn pick_polygon(&self, ray: &Ray3d) -> Option<(f64, Vec3d)> {
        let (ring, normal) = self.polygon()?;
        let plane = Plane3d::from_anchor_and_normal(ring[0], normal);
        let distance = intersection::intersect_ray_polygon_in_plane(ray, &plane, &ring)?;
        let facing = if vec::dot(normal, ray.direction) > 0.0 {
            -normal
        } else {
            normal
        };
        Some((distance, facing))
    }
}

/// The four corners of the rectangle a drag from `from` to `to` draws on a face in
/// `plane`: `DrawFaceDragDelegate::updatePolyhedron`. The two points are snapped outwards
/// to the grid on the two axes the face is seen best along, and the corners put back on
/// the face's plane along the third.
pub fn face_rectangle(plane: &Plane3d, from: Vec3d, to: Vec3d, grid: &Grid) -> [Vec3d; 4] {
    let axis = plane.normal.find_abs_max_component(0);
    let swizzled = Plane3d::from_anchor_and_normal(
        vec::swizzle(plane.anchor(), axis),
        vec::swizzle(plane.normal, axis),
    );
    let low = vec::swizzle(grid.snap_down_point(vec::min(from, to)), axis);
    let high = vec::swizzle(grid.snap_up_point(vec::max(from, to)), axis);
    let corner = |x: f64, y: f64| {
        let z = swizzled.z_at(Vec2d::new(x, y));
        vec::unswizzle(Vec3d::new(x, y, z), axis)
    };
    [
        corner(low.x(), low.y()),
        corner(high.x(), low.y()),
        corner(low.x(), high.y()),
        corner(high.x(), high.y()),
    ]
}

/// The corners of the convex polygon around `points`, which lie in the plane with
/// `normal`, in order around it.
fn convex_ring(points: &[Vec3d], normal: Vec3d) -> Vec<Vec3d> {
    // Monotone chain on the two axes the plane is seen best along.
    let axis = normal.find_abs_max_component(0);
    let mut sorted: Vec<Vec3d> = Vec::new();
    for point in points {
        if !sorted
            .iter()
            .any(|kept| vec::squared_distance(*kept, *point) <= SAME_POINT * SAME_POINT)
        {
            sorted.push(*point);
        }
    }
    let flat = |point: Vec3d| {
        let swizzled = vec::swizzle(point, axis);
        (swizzled.x(), swizzled.y())
    };
    sorted.sort_by(|a, b| {
        let (ax, ay) = flat(*a);
        let (bx, by) = flat(*b);
        ax.total_cmp(&bx).then(ay.total_cmp(&by))
    });
    let turn = |o: Vec3d, a: Vec3d, b: Vec3d| {
        let (ox, oy) = flat(o);
        let (ax, ay) = flat(a);
        let (bx, by) = flat(b);
        (ax - ox) * (by - oy) - (ay - oy) * (bx - ox)
    };
    let mut ring: Vec<Vec3d> = Vec::new();
    for pass in [false, true] {
        let start = ring.len();
        let ordered: Vec<Vec3d> = if pass {
            sorted.iter().rev().copied().collect()
        } else {
            sorted.clone()
        };
        for point in ordered {
            while ring.len() >= start + 2
                && turn(ring[ring.len() - 2], ring[ring.len() - 1], point) <= SAME_POINT
            {
                ring.pop();
            }
            ring.push(point);
        }
        ring.pop();
    }
    ring
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    /// Four points that do enclose a volume: a tetrahedron.
    fn tetrahedron() -> [Vec3d; 4] {
        [
            Vec3d::zero(),
            Vec3d::new(64.0, 0.0, 0.0),
            Vec3d::new(0.0, 64.0, 0.0),
            Vec3d::new(0.0, 0.0, 64.0),
        ]
    }

    #[test]
    fn a_new_tool_has_nothing_to_build_with() {
        let tool = AssembleBrushTool::new();
        assert!(tool.points().is_empty());
        assert!(tool.hull().is_none());
        assert!(tool.brush(&world(), "stone").is_none());
        assert!(tool.edges().is_empty());
    }

    #[test]
    fn fewer_than_four_points_enclose_nothing() {
        let mut tool = AssembleBrushTool::new();
        for point in &tetrahedron()[..3] {
            assert!(tool.add_point(*point));
            assert!(tool.hull().is_none());
        }
    }

    #[test]
    fn four_points_out_of_one_plane_make_a_brush() {
        let mut tool = AssembleBrushTool::new();
        for point in tetrahedron() {
            tool.add_point(point);
        }

        let brush = tool.brush(&world(), "stone").expect("a tetrahedron");
        assert_eq!(brush.faces().len(), 4);
        assert_eq!(brush.geometry().vertex_count(), 4);
        brush.geometry().check_invariant().expect("a valid mesh");
        assert_eq!(tool.edges().len(), 6);
    }

    #[test]
    fn four_points_in_one_plane_do_not() {
        let mut tool = AssembleBrushTool::new();
        for point in [
            Vec3d::zero(),
            Vec3d::new(64.0, 0.0, 0.0),
            Vec3d::new(0.0, 64.0, 0.0),
            Vec3d::new(64.0, 64.0, 0.0),
        ] {
            tool.add_point(point);
        }
        assert!(tool.hull().is_none());
    }

    #[test]
    fn a_point_already_there_is_not_added_twice() {
        let mut tool = AssembleBrushTool::new();
        assert!(tool.add_point(Vec3d::zero()));
        assert!(!tool.add_point(Vec3d::zero()));
        assert!(!tool.add_point(Vec3d::new(0.0005, 0.0, 0.0)));
        assert_eq!(tool.points().len(), 1);
    }

    #[test]
    fn a_point_inside_the_solid_leaves_it_the_same_shape() {
        let mut tool = AssembleBrushTool::new();
        for point in tetrahedron() {
            tool.add_point(point);
        }
        let before = tool
            .brush(&world(), "stone")
            .expect("a tetrahedron")
            .bounds();

        // Well inside the tetrahedron.
        tool.add_point(Vec3d::new(8.0, 8.0, 8.0));
        let after = tool
            .brush(&world(), "stone")
            .expect("a tetrahedron")
            .bounds();

        geometry_math::assert_approx_eq!(after.min, before.min, 1e-9);
        geometry_math::assert_approx_eq!(after.max, before.max, 1e-9);
    }

    #[test]
    fn a_point_outside_the_solid_grows_it() {
        let mut tool = AssembleBrushTool::new();
        for point in tetrahedron() {
            tool.add_point(point);
        }
        tool.add_point(Vec3d::new(0.0, 0.0, 128.0));

        let brush = tool.brush(&world(), "stone").expect("a solid");
        geometry_math::assert_approx_eq!(brush.bounds().max.z(), 128.0, 1e-6);
    }

    #[test]
    fn points_can_be_taken_back_one_at_a_time() {
        let mut tool = AssembleBrushTool::new();
        for point in tetrahedron() {
            tool.add_point(point);
        }
        assert!(tool.hull().is_some());

        assert!(tool.remove_last_point());
        assert!(tool.hull().is_none());

        for _ in 0..3 {
            assert!(tool.remove_last_point());
        }
        assert!(!tool.remove_last_point());
        assert!(tool.points().is_empty());
    }

    #[test]
    fn resetting_forgets_everything() {
        let mut tool = AssembleBrushTool::new();
        for point in tetrahedron() {
            tool.add_point(point);
        }
        tool.reset();
        assert!(tool.points().is_empty());
        assert!(tool.hull().is_none());
    }
}
