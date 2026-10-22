//! Tests for intersection behavior.

#![allow(clippy::excessive_precision)]

use geometry_math::abstract_line::AbstractLine;
use geometry_math::assert_approx_eq;
use geometry_math::bbox::Bbox3f;
use geometry_math::distance::*;
use geometry_math::intersection::*;
use geometry_math::line::{Line2d, Line3f};
use geometry_math::plane::{Plane3d, Plane3f};
use geometry_math::quat::Quatf;
use geometry_math::ray::{Ray3d, Ray3f};
use geometry_math::scalar::{self, Float};
use geometry_math::segment::Segment3d;
use geometry_math::util::PlaneStatus;
use geometry_math::vec::{self, Vec2d, Vec3d, Vec3f};

/// The unit square in the XY plane.
fn square() -> [Vec3d; 4] {
    [
        Vec3d::new(-1.0, -1.0, 0.0),
        Vec3d::new(-1.0, 1.0, 0.0),
        Vec3d::new(1.0, 1.0, 0.0),
        Vec3d::new(1.0, -1.0, 0.0),
    ]
}

/// A triangular test polygon.
fn triangle() -> [Vec3d; 3] {
    [
        Vec3d::new(-1.0, 1.0, 0.0),
        Vec3d::new(-1.0, -1.0, 0.0),
        Vec3d::new(1.0, -1.0, 0.0),
    ]
}

fn contains(vertices: &[Vec3d], point: Vec3d) -> bool {
    polygon_contains_point_with_normal(point, Vec3d::new(0.0, 0.0, 1.0), vertices)
}

/// Returns whether the whole line lies on the plane, by checking two points on it.
fn line_on_plane(plane: &Plane3f, line: &Line3f) -> bool {
    plane.point_status_default(line.point) == PlaneStatus::Inside
        && plane.point_status_default(line.point_at_distance(16.0)) == PlaneStatus::Inside
}

/* ========== polygon_contains_point ========== */

#[test]
fn square_contains_its_center_corners_and_edge_midpoints() {
    let s = square();
    assert!(contains(&s, Vec3d::zero()));

    for corner in s {
        assert!(contains(&s, corner), "corner {corner:?} should be inside");
    }

    for midpoint in [
        Vec3d::new(-1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, -1.0, 0.0),
    ] {
        assert!(contains(&s, midpoint), "edge midpoint {midpoint:?}");
    }
}

#[test]
fn square_does_not_contain_outside_points() {
    let s = square();
    assert!(!contains(&s, Vec3d::new(2.0, 0.0, 0.0)));
    assert!(!contains(&s, Vec3d::new(-2.0, -2.0, 0.0)));
    assert!(!contains(&s, Vec3d::new(1.1, 1.1, 0.0)));
}

#[test]
fn triangle_contains_its_corners_and_edge_midpoints() {
    let t = triangle();
    assert!(contains(&t, Vec3d::new(-0.5, -0.5, 0.0)));

    for corner in t {
        assert!(contains(&t, corner), "corner {corner:?} should be inside");
    }

    for midpoint in [
        Vec3d::new(-1.0, 0.0, 0.0),
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(0.0, -1.0, 0.0),
    ] {
        assert!(contains(&t, midpoint), "edge midpoint {midpoint:?}");
    }

    // The corner the triangle cuts off is outside.
    assert!(!contains(&t, Vec3d::new(1.0, 1.0, 0.0)));
}

#[test]
fn polygon_contains_point_derives_the_plane_when_it_is_not_given() {
    assert!(polygon_contains_point(Vec3d::zero(), &square()));
    assert!(!polygon_contains_point(
        Vec3d::new(2.0, 0.0, 0.0),
        &square()
    ));
    // Too few vertices to define a plane.
    assert!(!polygon_contains_point(Vec3d::zero(), &square()[..2]));
}

/* ========== ray intersections ========== */

#[test]
fn a_ray_hits_a_plane_in_front_of_it() {
    let r = Ray3f::new(Vec3f::zero(), Vec3f::new(0.0, 0.0, 1.0));
    let z = Vec3f::new(0.0, 0.0, 1.0);

    // Behind the ray.
    assert_eq!(
        intersect_ray_plane(
            &r,
            &Plane3f::from_anchor_and_normal(Vec3f::new(0.0, 0.0, -1.0), z)
        ),
        None
    );
    assert_approx_eq!(
        intersect_ray_plane(&r, &Plane3f::from_anchor_and_normal(Vec3f::zero(), z)),
        Some(0.0f32)
    );
    assert_approx_eq!(
        intersect_ray_plane(
            &r,
            &Plane3f::from_anchor_and_normal(Vec3f::new(0.0, 0.0, 1.0), z)
        ),
        Some(1.0f32)
    );

    // Parallel to the plane.
    assert_eq!(
        intersect_ray_plane(
            &Ray3f::new(Vec3f::zero(), Vec3f::new(1.0, 0.0, 0.0)),
            &Plane3f::from_anchor_and_normal(Vec3f::new(0.0, 0.0, 1.0), z)
        ),
        None
    );
}

#[test]
fn a_ray_hits_a_triangle() {
    let p0 = Vec3d::new(2.0, 5.0, 2.0);
    let p1 = Vec3d::new(4.0, 7.0, 2.0);
    let p2 = Vec3d::new(3.0, 2.0, 2.0);
    let hit =
        |origin: Vec3d, dir: Vec3d| intersect_ray_triangle(&Ray3d::new(origin, dir), p0, p1, p2);

    assert_eq!(hit(Vec3d::zero(), Vec3d::new(1.0, 0.0, 0.0)), None);
    assert_eq!(hit(Vec3d::zero(), Vec3d::new(0.0, 1.0, 0.0)), None);
    assert_eq!(hit(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0)), None);
    assert_eq!(
        hit(Vec3d::new(0.0, 0.0, 2.0), Vec3d::new(0.0, 1.0, 0.0)),
        None
    );

    // Straight up through the interior, and through each vertex.
    let z = Vec3d::new(0.0, 0.0, 1.0);
    assert_approx_eq!(hit(Vec3d::new(3.0, 5.0, 0.0), z), Some(2.0));
    assert_approx_eq!(hit(Vec3d::new(2.0, 5.0, 0.0), z), Some(2.0));
    assert_approx_eq!(hit(Vec3d::new(4.0, 7.0, 0.0), z), Some(2.0));
    assert_approx_eq!(hit(Vec3d::new(3.0, 2.0, 0.0), z), Some(2.0));
}

#[test]
fn a_ray_hits_a_polygon() {
    // The square lifted one unit up the Z axis.
    let poly: Vec<Vec3d> = square()
        .iter()
        .map(|v| *v + Vec3d::new(0.0, 0.0, 1.0))
        .collect();
    let z = Vec3d::new(0.0, 0.0, 1.0);

    assert_approx_eq!(
        intersect_ray_polygon(&Ray3d::new(Vec3d::zero(), z), &poly),
        Some(1.0)
    );
    // Aimed away from the polygon.
    assert_eq!(
        intersect_ray_polygon(&Ray3d::new(Vec3d::zero(), -z), &poly),
        None
    );
    // Aimed past it.
    assert_eq!(
        intersect_ray_polygon(&Ray3d::new(Vec3d::new(2.0, 2.0, 0.0), z), &poly),
        None
    );
}

#[test]
fn a_ray_hits_a_box() {
    let bounds = Bbox3f::new(Vec3f::new(-12.0, -3.0, 4.0), Vec3f::new(8.0, 9.0, 8.0));

    assert_eq!(
        intersect_ray_bbox(
            &Ray3f::new(Vec3f::zero(), Vec3f::new(0.0, 0.0, -1.0)),
            &bounds
        ),
        None
    );
    assert_approx_eq!(
        intersect_ray_bbox(
            &Ray3f::new(Vec3f::zero(), Vec3f::new(0.0, 0.0, 1.0)),
            &bounds
        ),
        Some(4.0f32)
    );

    // A ray aimed at a known point on the box surface reports exactly that distance.
    let origin = Vec3f::new(-10.0, -7.0, 14.0);
    let diff = Vec3f::new(-2.0, 3.0, 8.0) - origin;
    let r = Ray3f::new(origin, vec::normalize(diff));
    assert_approx_eq!(intersect_ray_bbox(&r, &bounds), Some(vec::length(diff)));
}

#[test]
fn a_ray_hits_a_sphere() {
    let r = Ray3f::new(Vec3f::zero(), Vec3f::new(0.0, 0.0, 1.0));

    // Originating inside the sphere, hitting the far side.
    assert_approx_eq!(intersect_ray_sphere(&r, Vec3f::zero(), 2.0), Some(2.0f32));
    // Originating outside, hitting the near side.
    assert_approx_eq!(
        intersect_ray_sphere(&r, Vec3f::new(0.0, 0.0, 5.0), 2.0),
        Some(3.0f32)
    );
    assert_eq!(
        intersect_ray_sphere(&r, Vec3f::new(3.0, 2.0, 2.0), 1.0),
        None
    );
    // Entirely behind the ray.
    assert_eq!(
        intersect_ray_sphere(&r, Vec3f::new(0.0, 0.0, -5.0), 2.0),
        None
    );
}

#[test]
fn a_ray_hits_a_torus() {
    let hit = |origin: Vec3f, dir: Vec3f, center: Vec3f| {
        intersect_ray_torus(&Ray3f::new(origin, dir), center, 5.0, 1.0)
    };
    let o = Vec3f::zero();

    // From the middle of the hole outwards, in both axes.
    assert_approx_eq!(hit(o, Vec3f::new(0.0, 1.0, 0.0), o), Some(4.0f32));
    assert_approx_eq!(hit(o, Vec3f::new(1.0, 0.0, 0.0), o), Some(4.0f32));

    // From outside, inwards.
    assert_approx_eq!(
        hit(Vec3f::new(0.0, -10.0, 0.0), Vec3f::new(0.0, 1.0, 0.0), o),
        Some(4.0f32)
    );
    assert_approx_eq!(
        hit(Vec3f::new(-10.0, 0.0, 0.0), Vec3f::new(1.0, 0.0, 0.0), o),
        Some(4.0f32)
    );

    // Down onto the tube.
    assert_approx_eq!(
        hit(Vec3f::new(0.0, -5.0, 5.0), Vec3f::new(0.0, 0.0, -1.0), o),
        Some(4.0f32)
    );
    // The same, with the torus moved.
    assert_approx_eq!(
        hit(
            Vec3f::new(5.0, -5.0, 5.0),
            Vec3f::new(0.0, 0.0, -1.0),
            Vec3f::new(5.0, 0.0, 0.0)
        ),
        Some(4.0f32)
    );

    // Straight up through the hole.
    assert_eq!(hit(o, Vec3f::new(0.0, 0.0, 1.0), o), None);
}

/* ========== line and plane intersections ========== */

#[test]
fn two_lines_meet_at_one_point() {
    let vertical = Line2d::new(Vec2d::zero(), Vec2d::new(0.0, 1.0));

    // Parallel and coincident lines both report no unique intersection.
    assert_eq!(intersect_line_line(&vertical, &vertical), None);
    assert_eq!(
        intersect_line_line(
            &vertical,
            &Line2d::new(Vec2d::new(1.0, 0.0), Vec2d::new(0.0, 1.0))
        ),
        None
    );

    assert_eq!(
        intersect_line_line(&vertical, &Line2d::new(Vec2d::zero(), Vec2d::new(1.0, 0.0))),
        Some(0.0)
    );
    assert_eq!(
        intersect_line_line(
            &vertical,
            &Line2d::new(Vec2d::new(0.0, 1.0), Vec2d::new(1.0, 0.0))
        ),
        Some(1.0)
    );
}

#[test]
fn a_line_meets_a_plane() {
    let p = Plane3f::new(5.0, Vec3f::new(0.0, 0.0, 1.0));
    let l = Line3f::new(
        Vec3f::new(0.0, 0.0, 15.0),
        vec::normalize(Vec3f::new(1.0, 0.0, -1.0)),
    );
    let d = intersect_line_plane(&l, &p).expect("the line is not parallel to the plane");
    assert_approx_eq!(l.point_at_distance(d), Vec3f::new(10.0, 0.0, 5.0));
}

#[test]
fn two_planes_meet_in_a_line() {
    let p1 = Plane3f::new(10.0, Vec3f::new(0.0, 0.0, 1.0));
    let p2 = Plane3f::new(20.0, Vec3f::new(1.0, 0.0, 0.0));
    let line = intersect_plane_plane(&p1, &p2).expect("the planes are not parallel");

    assert!(line_on_plane(&p1, &line));
    assert!(line_on_plane(&p2, &line));
}

#[test]
fn intersect_plane_plane_reports_parallel_planes() {
    let p1 = Plane3f::new(10.0, Vec3f::new(0.0, 0.0, 1.0));
    let p2 = Plane3f::new(11.0, Vec3f::new(0.0, 0.0, 1.0));
    assert_eq!(intersect_plane_plane(&p1, &p2), None);
}

#[test]
fn intersect_plane_plane_handles_nearly_parallel_planes() {
    let anchor = Vec3f::new(100.0, 100.0, 100.0);
    let p1 = Plane3f::from_anchor_and_normal(anchor, Vec3f::new(1.0, 0.0, 0.0));

    // Half a degree apart: still a usable intersection.
    let rotated = Quatf::from_axis_angle(Vec3f::new(0.0, -1.0, 0.0), scalar::to_radians(0.5))
        * Vec3f::new(1.0, 0.0, 0.0);
    let p2 = Plane3f::from_anchor_and_normal(anchor, rotated);
    let line = intersect_plane_plane(&p1, &p2).expect("half a degree is enough");
    assert!(line_on_plane(&p1, &line));
    assert!(line_on_plane(&p2, &line));

    // A ten-thousandth of a degree apart: too close to tell apart in f32.
    let barely = Quatf::from_axis_angle(Vec3f::new(0.0, -1.0, 0.0), scalar::to_radians(0.0001))
        * Vec3f::new(1.0, 0.0, 0.0);
    let p3 = Plane3f::from_anchor_and_normal(anchor, barely);
    assert_eq!(intersect_plane_plane(&p1, &p3), None);
}

#[test]
fn colinear_segments_overlap() {
    let along_x = |a: f64, b: f64| Segment3d::new(Vec3d::new(a, 0.0, 0.0), Vec3d::new(b, 0.0, 0.0));
    let eps = f64::ALMOST_ZERO;

    assert!(segments_overlap(
        &along_x(0.0, 10.0),
        &along_x(5.0, 15.0),
        eps
    ));
    assert!(segments_overlap(
        &along_x(0.0, 10.0),
        &along_x(2.0, 8.0),
        eps
    ));
    // Touching at a point is not an overlap.
    assert!(!segments_overlap(
        &along_x(0.0, 10.0),
        &along_x(10.0, 20.0),
        eps
    ));
    // Disjoint.
    assert!(!segments_overlap(
        &along_x(0.0, 10.0),
        &along_x(11.0, 20.0),
        eps
    ));
    // Parallel but not colinear.
    assert!(!segments_overlap(
        &along_x(0.0, 10.0),
        &Segment3d::new(Vec3d::new(0.0, 1.0, 0.0), Vec3d::new(10.0, 1.0, 0.0)),
        eps
    ));
}

/* ========== clipping and overlap ========== */

#[test]
fn a_plane_clips_a_polygon() {
    let poly = square();

    // A plane the polygon lies in does not clip it.
    let in_plane = Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
    assert!(polygon_clip_by_plane(&in_plane, &poly).is_empty());

    // A plane through the middle keeps the half behind it.
    let halving = Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(1.0, 0.0, 0.0));
    let clipped = polygon_clip_by_plane(&halving, &poly);
    assert_eq!(clipped.len(), 4);
    for v in &clipped {
        assert!(
            v.x() <= 0.0 + f64::ALMOST_ZERO,
            "kept vertex {v:?} is in front"
        );
    }
}

#[test]
fn two_coplanar_polygons_overlap() {
    let normal = Vec3d::new(0.0, 0.0, 1.0);
    let a = square();

    // A copy of itself overlaps.
    assert!(polygons_overlap(&a, &a, normal));

    // Shifted half a unit: still overlapping.
    let shifted: Vec<Vec3d> = a.iter().map(|v| *v + Vec3d::new(0.5, 0.0, 0.0)).collect();
    assert!(polygons_overlap(&a, &shifted, normal));

    // Shifted clear of it: not overlapping.
    let apart: Vec<Vec3d> = a.iter().map(|v| *v + Vec3d::new(10.0, 0.0, 0.0)).collect();
    assert!(!polygons_overlap(&a, &apart, normal));
}

#[test]
fn a_box_and_a_polygon_intersect() {
    let b = geometry_math::bbox::Bbox3d::from_scalars(-1.0, 1.0);

    // A polygon with a vertex inside the box.
    let inside: Vec<Vec3d> = square().iter().map(|v| *v * 0.5).collect();
    assert!(intersect_bbox_polygon(&b, &inside));

    // A large polygon cutting straight through the box, with no vertex inside it.
    let through: Vec<Vec3d> = square().iter().map(|v| *v * 10.0).collect();
    assert!(intersect_bbox_polygon(&b, &through));

    // Well clear of the box.
    let away: Vec<Vec3d> = square()
        .iter()
        .map(|v| *v + Vec3d::new(100.0, 0.0, 0.0))
        .collect();
    assert!(!intersect_bbox_polygon(&b, &away));
}

/* ========== distance ========== */

#[test]
fn distance_from_a_ray_to_a_point() {
    let r = Ray3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));

    // Straight out to the side.
    let d = distance_ray_point(&r, Vec3d::new(5.0, 0.0, 10.0));
    assert_approx_eq!(d.position, 10.0);
    assert_approx_eq!(d.distance, 5.0);

    // Behind the origin: the closest point on the ray is the origin itself.
    let d = distance_ray_point(&r, Vec3d::new(0.0, 0.0, -10.0));
    assert_approx_eq!(d.position, 0.0);
    assert_approx_eq!(d.distance, 10.0);

    // The squared form agrees.
    let sq = squared_distance_ray_point(&r, Vec3d::new(5.0, 0.0, 10.0));
    assert_approx_eq!(sq.distance, 25.0);
}

#[test]
fn distance_from_a_segment_to_a_point() {
    let s = Segment3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 10.0));

    let d = distance_segment_point(&s, Vec3d::new(3.0, 0.0, 5.0));
    assert_approx_eq!(d.position, 5.0);
    assert_approx_eq!(d.distance, 3.0);

    // Past the end: clamped to the endpoint.
    let d = distance_segment_point(&s, Vec3d::new(0.0, 0.0, 20.0));
    assert_approx_eq!(d.position, 10.0);
    assert_approx_eq!(d.distance, 10.0);
}

#[test]
fn distance_between_two_crossing_rays() {
    // Two rays that pass 5 units apart, one along X and one along Y.
    let a = Ray3d::new(Vec3d::zero(), Vec3d::new(1.0, 0.0, 0.0));
    let b = Ray3d::new(Vec3d::new(10.0, -10.0, 5.0), Vec3d::new(0.0, 1.0, 0.0));

    let d = distance_ray_ray(&a, &b);
    assert!(!d.parallel);
    assert_approx_eq!(d.distance, 5.0);
    assert_approx_eq!(d.position1, 10.0);
    assert_approx_eq!(d.position2, 10.0);
}

#[test]
fn distance_between_two_parallel_rays() {
    let a = Ray3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
    let b = Ray3d::new(Vec3d::new(3.0, 4.0, 0.0), Vec3d::new(0.0, 0.0, 1.0));

    let d = distance_ray_ray(&a, &b);
    assert!(d.parallel);
    assert_approx_eq!(d.distance, 5.0);
    assert!(!d.is_colinear(f64::ALMOST_ZERO));
}

#[test]
fn colinear_rays_are_reported_as_colinear() {
    let a = Ray3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
    let b = Ray3d::new(Vec3d::new(0.0, 0.0, 5.0), Vec3d::new(0.0, 0.0, 1.0));

    let d = distance_ray_ray(&a, &b);
    assert!(d.parallel);
    assert!(d.is_colinear(f64::ALMOST_ZERO));
}

#[test]
fn distance_between_a_ray_and_a_segment() {
    let r = Ray3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));

    // A segment crossing the ray's path 3 units to the side.
    let crossing = Segment3d::new(Vec3d::new(3.0, -5.0, 5.0), Vec3d::new(3.0, 5.0, 5.0));
    let d = distance_ray_segment(&r, &crossing);
    assert!(!d.parallel);
    assert_approx_eq!(d.distance, 3.0);
    assert_approx_eq!(d.position1, 5.0);

    // A segment running alongside the ray takes the parallel branch.
    let alongside = Segment3d::new(Vec3d::new(3.0, 0.0, 5.0), Vec3d::new(3.0, 0.0, 15.0));
    let d = distance_ray_segment(&r, &alongside);
    assert!(d.parallel);
    assert_approx_eq!(d.distance, 3.0);
    assert_approx_eq!(d.position1, 5.0);

    // A segment entirely behind the ray's origin is measured from that origin.
    let behind = Segment3d::new(Vec3d::new(0.0, 0.0, -20.0), Vec3d::new(0.0, 0.0, -10.0));
    let d = distance_ray_segment(&r, &behind);
    assert!(d.parallel);
    assert_approx_eq!(d.distance, 10.0);
}

#[test]
fn distance_between_a_ray_and_a_line() {
    let r = Ray3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
    let l = geometry_math::line::Line3d::new(Vec3d::new(4.0, 0.0, 5.0), Vec3d::new(0.0, 1.0, 0.0));

    let d = distance_ray_line(&r, &l);
    assert!(!d.parallel);
    assert_approx_eq!(d.distance, 4.0);
    assert_approx_eq!(d.position1, 5.0);

    // A line, unlike a ray, extends backwards, so its position can be negative.
    let parallel =
        geometry_math::line::Line3d::new(Vec3d::new(3.0, 0.0, 0.0), Vec3d::new(0.0, 0.0, 1.0));
    let d = distance_ray_line(
        &Ray3d::new(Vec3d::new(0.0, 0.0, 7.0), Vec3d::new(0.0, 0.0, 1.0)),
        &parallel,
    );
    assert!(d.parallel);
    assert_approx_eq!(d.distance, 3.0);
    assert_approx_eq!(d.position2, 7.0);
}
