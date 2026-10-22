//! Tests for geometry behavior.

#![allow(clippy::excessive_precision)]

use geometry_math::abstract_line::AbstractLine;
use geometry_math::assert_approx_eq;
use geometry_math::bbox::{Bbox3d, Bbox3f, Builder, Corner, Range};
use geometry_math::convex_hull::convex_hull;
use geometry_math::line::{Line2d, Line3d};
use geometry_math::mat_ext;
use geometry_math::plane::{self, Plane3d, Plane3f};
use geometry_math::polygon::{self, Polygon3d};
use geometry_math::ray::{Ray3d, Ray3f};
use geometry_math::scalar::Float;
use geometry_math::segment::{self, Segment3d};
use geometry_math::util::{Axis, PlaneStatus};
use geometry_math::vec::{self, Vec2d, Vec3d, Vec3f};

/* ========== bbox ========== */

#[test]
fn bbox_default_is_the_degenerate_box_at_the_origin() {
    let b = Bbox3d::default();
    assert_eq!(b.min, Vec3d::zero());
    assert_eq!(b.max, Vec3d::zero());
}

#[test]
fn bbox_is_valid_pair() {
    assert!(Bbox3d::is_valid_pair(Vec3d::zero(), Vec3d::zero()));
    assert!(Bbox3d::is_valid_pair(
        Vec3d::new(-1.0, -1.0, -1.0),
        Vec3d::new(1.0, 1.0, 1.0)
    ));
    assert!(!Bbox3d::is_valid_pair(
        Vec3d::new(1.0, -1.0, -1.0),
        Vec3d::new(-1.0, 1.0, 1.0)
    ));
    assert!(!Bbox3d::is_valid_pair(
        Vec3d::new(-1.0, 1.0, -1.0),
        Vec3d::new(1.0, -1.0, 1.0)
    ));
}

#[test]
#[should_panic(expected = "min <= max")]
fn bbox_new_rejects_an_inverted_box() {
    Bbox3d::new(Vec3d::one(), -Vec3d::one());
}

#[test]
fn bbox_new_repaired_swaps_inverted_components() {
    // Repairs bounds during construction.
    let b = Bbox3d::new_repaired(Vec3d::new(8.0, -8.0, 8.0), Vec3d::new(-8.0, 8.0, -8.0));
    assert_eq!(b, Bbox3d::from_scalars(-8.0, 8.0));
}

#[test]
fn bbox_is_empty() {
    assert!(Bbox3d::default().is_empty());
    assert!(!Bbox3d::from_scalars(-1.0, 1.0).is_empty());
    // Flat in one dimension still counts as empty.
    assert!(Bbox3d::new(Vec3d::new(-1.0, 0.0, -1.0), Vec3d::new(1.0, 0.0, 1.0)).is_empty());
}

#[test]
fn bbox_center_size_and_volume() {
    let b = Bbox3f::new(Vec3f::new(-1.0, -2.0, -3.0), Vec3f::new(1.0, 4.0, 5.0));
    assert_eq!(b.center(), Vec3f::new(0.0, 1.0, 1.0));

    let b = Bbox3f::new(Vec3f::new(-1.0, -2.0, -3.0), Vec3f::new(1.0, 3.0, 5.0));
    assert_eq!(b.size(), Vec3f::new(2.0, 5.0, 8.0));

    assert_eq!(Bbox3d::default().volume(), 0.0);
    assert_eq!(Bbox3d::from_scalars(-2.0, 2.0).volume(), 4.0 * 4.0 * 4.0);
}

#[test]
fn bbox_contains_encloses_and_intersects() {
    let b1 = Bbox3f::new(Vec3f::new(-12.0, -3.0, 4.0), Vec3f::new(8.0, 9.0, 8.0));
    let b2 = Bbox3f::new(Vec3f::new(-10.0, -2.0, 5.0), Vec3f::new(7.0, 8.0, 7.0));
    let b3 = Bbox3f::new(Vec3f::new(-13.0, -2.0, 5.0), Vec3f::new(7.0, 8.0, 7.0));

    assert!(b1.contains_point(Vec3f::new(2.0, 1.0, 7.0)));
    // The boundary counts as contained.
    assert!(b1.contains_point(Vec3f::new(-12.0, -3.0, 7.0)));
    assert!(!b1.contains_point(Vec3f::new(-13.0, -3.0, 7.0)));

    assert!(b1.contains(&b1));
    assert!(b1.contains(&b2));
    assert!(!b1.contains(&b3));

    // A box does not strictly enclose itself.
    assert!(!b1.encloses(&b1));
    assert!(b1.encloses(&b2));
    assert!(!b1.encloses(&Bbox3f::new(
        Vec3f::new(-10.0, -3.0, 5.0),
        Vec3f::new(7.0, 8.0, 7.0)
    )));

    assert!(b1.intersects(&b1));
    assert!(b1.intersects(&b2));
    assert!(b1.intersects(&b3));
    assert!(!b1.intersects(&Bbox3f::new(
        Vec3f::new(-15.0, 10.0, 9.0),
        Vec3f::new(-13.0, 12.0, 10.0)
    )));
}

#[test]
fn bbox_constrain() {
    let b = Bbox3d::from_scalars(-1024.0, 1024.0);
    assert_eq!(b.constrain(Vec3d::zero()), Vec3d::zero());
    assert_eq!(b.constrain(b.min), b.min);
    assert_eq!(b.constrain(b.min + Vec3d::new(-1.0, 0.0, 0.0)), b.min);
    assert_eq!(b.constrain(b.max + Vec3d::new(0.0, 0.0, 1.0)), b.max);
}

#[test]
fn bbox_corner_position() {
    let b = Bbox3f::new(Vec3f::new(-1.0, -2.0, -3.0), Vec3f::new(1.0, 3.0, 5.0));
    assert_eq!(b.corner(Corner::Min, Corner::Min, Corner::Min), b.min);
    assert_eq!(b.corner(Corner::Max, Corner::Max, Corner::Max), b.max);
    assert_eq!(
        b.corner(Corner::Min, Corner::Max, Corner::Min),
        Vec3f::new(-1.0, 3.0, -3.0)
    );
}

#[test]
fn bbox_relative_position() {
    let b = Bbox3d::from_scalars(-1.0, 1.0);
    assert_eq!(b.relative_position(Vec3d::zero()), [Range::Within; 3]);
    assert_eq!(
        b.relative_position(Vec3d::new(-2.0, 0.0, 2.0)),
        [Range::Less, Range::Within, Range::Greater]
    );
}

#[test]
fn bbox_expand_and_translate() {
    let b = Bbox3f::new(Vec3f::new(-12.0, -3.0, 4.0), Vec3f::new(8.0, 9.0, 8.0));
    assert_eq!(
        b.expand(2.0),
        Bbox3f::new(Vec3f::new(-14.0, -5.0, 2.0), Vec3f::new(10.0, 11.0, 10.0))
    );
    assert_eq!(
        b.translate(Vec3f::new(2.0, -1.0, -3.0)),
        Bbox3f::new(Vec3f::new(-10.0, -4.0, 1.0), Vec3f::new(10.0, 8.0, 5.0))
    );
}

#[test]
fn bbox_merge_and_intersect() {
    let b1 = Bbox3f::new(Vec3f::new(-12.0, -3.0, 4.0), Vec3f::new(7.0, 8.0, 9.0));
    let b2 = Bbox3f::new(Vec3f::new(-10.0, -5.0, 3.0), Vec3f::new(9.0, 9.0, 5.0));
    assert_eq!(
        b1.merged(&b2),
        Bbox3f::new(Vec3f::new(-12.0, -5.0, 3.0), Vec3f::new(9.0, 9.0, 9.0))
    );
    assert_eq!(
        b1.merged_with_point(Vec3f::new(-10.0, -6.0, 10.0)),
        Bbox3f::new(Vec3f::new(-12.0, -6.0, 4.0), Vec3f::new(7.0, 8.0, 10.0))
    );

    let a = Bbox3d::from_scalars(-10.0, 10.0);
    let b = Bbox3d::new(Vec3d::new(-5.0, -5.0, -5.0), Vec3d::new(20.0, 5.0, 10.0));
    let expected = Bbox3d::new(Vec3d::new(-5.0, -5.0, -5.0), Vec3d::new(10.0, 5.0, 10.0));
    assert_eq!(a.intersected(&b), Some(expected));
    assert_eq!(b.intersected(&a), Some(expected));

    // Disjoint boxes have no intersection.
    let disjoint = Bbox3d::new(Vec3d::new(12.0, 12.0, 12.0), Vec3d::new(15.0, 15.0, 15.0));
    assert_eq!(a.intersected(&disjoint), None);

    // Boxes that only touch intersect in a degenerate box.
    let touching = Bbox3d::new(Vec3d::new(10.0, -5.0, -5.0), Vec3d::new(15.0, 5.0, 5.0));
    assert_eq!(
        a.intersected(&touching),
        Some(Bbox3d::new(
            Vec3d::new(10.0, -5.0, -5.0),
            Vec3d::new(10.0, 5.0, 5.0)
        ))
    );
}

#[test]
fn bbox_vertices_edges_and_faces() {
    let b = Bbox3d::from_scalars(-1.0, 1.0);

    let vertices = b.vertices();
    assert_eq!(vertices.len(), 8);
    // Every vertex is a corner, and every corner appears exactly once.
    for v in vertices {
        assert!(v.v.iter().all(|c| *c == -1.0 || *c == 1.0));
    }
    let mut sorted = vertices;
    sorted.sort();
    sorted.windows(2).for_each(|w| assert_ne!(w[0], w[1]));

    // Every edge runs between two vertices and has the box's edge length.
    for (start, end) in b.edges() {
        assert!(vertices.contains(&start) && vertices.contains(&end));
        assert_eq!(vec::length(end - start), 2.0);
    }

    // Every face's normal is an axis, and all six are distinct.
    let faces = b.faces();
    let mut normals: Vec<Vec3d> = faces.iter().map(|f| f.normal).collect();
    normals.sort();
    normals.dedup();
    assert_eq!(normals.len(), 6);
}

#[test]
fn bbox_transform_bounds_the_transformed_box() {
    let b = Bbox3d::from_scalars(-2.0, 10.0);
    let transform = mat_ext::scaling_matrix(Vec3d::new(0.5, 2.0, 3.0));

    let expected = Bbox3d::merge_all(b.vertices().into_iter().map(|v| transform * v))
        .expect("a box has eight vertices");
    assert_eq!(b.transform(&transform), expected);
}

#[test]
fn bbox_builder() {
    let mut builder = Builder::<f64, 3>::new();
    assert!(!builder.initialized());
    assert_eq!(builder.bounds(), None);

    builder.add_point(Vec3d::new(8.0, -8.0, 8.0));
    assert!(builder.initialized());
    assert_eq!(
        builder.bounds(),
        Some(Bbox3d::from_point(Vec3d::new(8.0, -8.0, 8.0)))
    );

    builder.add_point(Vec3d::new(-8.0, 8.0, -8.0));
    assert_eq!(builder.bounds(), Some(Bbox3d::from_scalars(-8.0, 8.0)));

    // Adding a box already inside changes nothing.
    builder.add_bbox(&Bbox3d::from_scalars(-1.0, 1.0));
    assert_eq!(builder.bounds(), Some(Bbox3d::from_scalars(-8.0, 8.0)));
}

/* ========== plane ========== */

#[test]
fn plane_anchor_and_point_distance() {
    let p = Plane3d::new(5.0, Vec3d::new(0.0, 0.0, 1.0));
    assert_eq!(p.anchor(), Vec3d::new(0.0, 0.0, 5.0));
    assert_eq!(p.point_distance(Vec3d::new(3.0, 4.0, 8.0)), 3.0);
    assert_eq!(p.point_distance(Vec3d::new(3.0, 4.0, 2.0)), -3.0);
}

#[test]
fn plane_point_status() {
    let p = Plane3d::new(0.0, Vec3d::new(0.0, 0.0, 1.0));
    assert_eq!(
        p.point_status_default(Vec3d::new(0.0, 0.0, 1.0)),
        PlaneStatus::Above
    );
    assert_eq!(
        p.point_status_default(Vec3d::new(0.0, 0.0, -1.0)),
        PlaneStatus::Below
    );
    assert_eq!(p.point_status_default(Vec3d::zero()), PlaneStatus::Inside);
}

#[test]
fn plane_flip_keeps_the_plane_in_place() {
    let p = Plane3d::new(5.0, Vec3d::new(0.0, 0.0, 1.0));
    let flipped = p.flip();
    assert_eq!(flipped.normal, -p.normal);
    // The anchor is unchanged, so the plane has not moved.
    assert_eq!(flipped.anchor(), p.anchor());
}

#[test]
fn plane_project_point_and_vector() {
    let p = Plane3d::new(5.0, Vec3d::new(0.0, 0.0, 1.0));
    assert_approx_eq!(
        p.project_point(Vec3d::new(3.0, 4.0, 100.0)),
        Vec3d::new(3.0, 4.0, 5.0)
    );
    // A vector loses its component along the normal.
    assert_approx_eq!(
        p.project_vector(Vec3d::new(3.0, 4.0, 100.0)),
        Vec3d::new(3.0, 4.0, 0.0)
    );
}

#[test]
fn plane_project_point_along_a_direction() {
    let p = Plane3d::new(0.0, Vec3d::new(0.0, 0.0, 1.0));
    assert_approx_eq!(
        p.project_point_along(Vec3d::new(0.0, 0.0, 10.0), Vec3d::new(1.0, 0.0, -1.0)),
        Some(Vec3d::new(10.0, 0.0, 0.0))
    );
    // A direction parallel to the plane never reaches it.
    assert_eq!(
        p.project_point_along(Vec3d::new(0.0, 0.0, 10.0), Vec3d::new(1.0, 0.0, 0.0)),
        None
    );
}

#[test]
fn plane_at() {
    // The plane x + y + z = 3.
    let p =
        Plane3d::from_anchor_and_normal(Vec3d::new(1.0, 1.0, 1.0), vec::normalize(Vec3d::one()));
    // Given y and z, solve for x.
    assert_approx_eq!(p.at(Vec2d::new(1.0, 1.0), Axis::X), 1.0);
    assert_approx_eq!(p.z_at(Vec2d::new(0.0, 0.0)), 3.0);

    // A plane parallel to the axis has no unique answer, so 0 is returned.
    let flat = Plane3d::new(5.0, Vec3d::new(0.0, 0.0, 1.0));
    assert_eq!(flat.x_at(Vec2d::new(0.0, 0.0)), 0.0);
}

#[test]
fn plane_from_points() {
    let p = plane::from_points(
        Vec3d::zero(),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
    )
    .expect("three independent points define a plane");
    assert_approx_eq!(p.normal, Vec3d::new(0.0, 0.0, -1.0));
    assert_approx_eq!(p.distance, 0.0);

    // Colinear and coincident points do not.
    assert_eq!(
        plane::from_points(
            Vec3d::zero(),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(2.0, 0.0, 0.0)
        ),
        None
    );
    assert_eq!(
        plane::from_points(Vec3d::zero(), Vec3d::zero(), Vec3d::zero()),
        None
    );
}

#[test]
fn plane_constructors() {
    assert_eq!(
        plane::horizontal_plane(Vec3d::new(1.0, 2.0, 3.0)),
        Plane3d::new(3.0, Vec3d::new(0.0, 0.0, 1.0))
    );
    assert_approx_eq!(
        plane::orthogonal_plane(Vec3d::zero(), Vec3d::new(0.0, 0.0, 5.0)).normal,
        Vec3d::new(0.0, 0.0, 1.0)
    );
    // The aligned variant snaps the normal to the nearest axis.
    assert_eq!(
        plane::aligned_orthogonal_plane(Vec3d::zero(), Vec3d::new(0.1, 0.2, -0.9)).normal,
        Vec3d::new(0.0, 0.0, -1.0)
    );
}

#[test]
fn plane_transform() {
    let p = Plane3d::new(5.0, Vec3d::new(0.0, 0.0, 1.0));
    let t = mat_ext::translation_matrix(Vec3d::new(0.0, 0.0, 3.0));
    let moved = p.transform(&t);
    assert_approx_eq!(moved.normal, p.normal);
    assert_approx_eq!(moved.distance, 8.0);
}

/* ========== line, ray, segment ========== */

#[test]
fn abstract_line_operations() {
    let r = Ray3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
    assert_eq!(
        r.distance_to_projected_point(Vec3d::new(5.0, 5.0, 7.0)),
        7.0
    );
    assert_eq!(r.point_at_distance(3.0), Vec3d::new(0.0, 0.0, 3.0));
    assert_eq!(
        r.project_point(Vec3d::new(5.0, 5.0, 7.0)),
        Vec3d::new(0.0, 0.0, 7.0)
    );
}

#[test]
fn line_make_canonical_normalizes_the_representation() {
    // The same infinite line, anchored at two different points and pointing both ways.
    let a = Line3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
    let b = Line3d::new(Vec3d::new(0.0, 0.0, 5.0), Vec3d::new(0.0, 0.0, -1.0));
    assert_eq!(a, b);
    assert_eq!(a.make_canonical(), b.make_canonical());

    // The canonical anchor is the point closest to the origin.
    let offset = Line3d::new(Vec3d::new(3.0, 4.0, 9.0), Vec3d::new(0.0, 0.0, 1.0));
    assert_approx_eq!(offset.make_canonical().point, Vec3d::new(3.0, 4.0, 0.0));
}

#[test]
fn line_2d_transform() {
    let l = Line2d::new(Vec2d::zero(), Vec2d::new(1.0, 0.0));
    let t = mat_ext::translation_matrix_2d(Vec2d::new(0.0, 3.0));
    assert_approx_eq!(l.transform(&t).point, Vec2d::new(0.0, 3.0));
}

#[test]
fn ray_point_status() {
    let r = Ray3f::new(Vec3f::zero(), Vec3f::new(0.0, 0.0, 1.0));
    assert_eq!(
        r.point_status(Vec3f::new(0.0, 0.0, 1.0)),
        PlaneStatus::Above
    );
    assert_eq!(
        r.point_status(Vec3f::new(0.0, 0.0, -1.0)),
        PlaneStatus::Below
    );
    assert_eq!(
        r.point_status(Vec3f::new(5.0, 5.0, 0.0)),
        PlaneStatus::Inside
    );
}

#[test]
fn ray_transform() {
    let r = Ray3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));
    let t = mat_ext::rotation_matrix_about_axis(
        Vec3d::new(1.0, 0.0, 0.0),
        geometry_math::scalar::to_radians(90.0),
    );
    assert_approx_eq!(r.transform(&t).direction, Vec3d::new(0.0, -1.0, 0.0));
}

#[test]
fn segment_orders_its_endpoints() {
    let a = Vec3d::new(1.0, 2.0, 3.0);
    let b = Vec3d::new(4.0, 5.0, 6.0);
    // Built either way round, a segment is the same segment.
    assert_eq!(Segment3d::new(a, b), Segment3d::new(b, a));
    assert_eq!(Segment3d::new(b, a).start(), a);
    assert_eq!(Segment3d::new(b, a).end(), b);
}

#[test]
fn segment_measurements() {
    let s = Segment3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 10.0));
    assert_eq!(s.length(), 10.0);
    assert_eq!(s.squared_length(), 100.0);
    assert_eq!(s.center(), Vec3d::new(0.0, 0.0, 5.0));
    assert_eq!(s.direction_normalized(), Vec3d::new(0.0, 0.0, 1.0));
}

#[test]
fn segment_contains() {
    let s = Segment3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 10.0));
    assert!(s.contains(Vec3d::new(0.0, 0.0, 5.0), 0.0));
    assert!(s.contains(Vec3d::new(0.0, 0.0, 0.0), 0.0));
    assert!(s.contains(Vec3d::new(0.0, 0.0, 10.0), 0.0));
    // Off the end, and off to the side.
    assert!(!s.contains(Vec3d::new(0.0, 0.0, 11.0), 0.5));
    assert!(!s.contains(Vec3d::new(1.0, 0.0, 5.0), 0.5));
    assert!(s.contains(Vec3d::new(0.4, 0.0, 5.0), 0.5));
}

#[test]
fn segment_vertices() {
    let segments = [
        Segment3d::new(Vec3d::zero(), Vec3d::new(1.0, 0.0, 0.0)),
        Segment3d::new(Vec3d::new(2.0, 0.0, 0.0), Vec3d::new(3.0, 0.0, 0.0)),
    ];
    assert_eq!(segment::vertices(&segments).len(), 4);
}

/* ========== polygon ========== */

#[test]
fn polygon_rotates_the_smallest_vertex_to_the_front() {
    let a = Vec3d::new(0.0, 0.0, 0.0);
    let b = Vec3d::new(1.0, 0.0, 0.0);
    let c = Vec3d::new(1.0, 1.0, 0.0);

    // The same cyclic order, started at three different points.
    let p1 = Polygon3d::new(vec![a, b, c]);
    let p2 = Polygon3d::new(vec![b, c, a]);
    let p3 = Polygon3d::new(vec![c, a, b]);

    assert_eq!(p1.vertices()[0], a);
    assert_eq!(p1, p2);
    assert_eq!(p2, p3);
}

#[test]
fn polygon_center_and_invert() {
    let p = Polygon3d::new(vec![
        Vec3d::new(-1.0, -1.0, 0.0),
        Vec3d::new(1.0, -1.0, 0.0),
        Vec3d::new(1.0, 1.0, 0.0),
        Vec3d::new(-1.0, 1.0, 0.0),
    ]);
    assert_approx_eq!(p.center().unwrap(), Vec3d::zero());

    let inverted = p.invert();
    assert_eq!(inverted.vertex_count(), p.vertex_count());
    // Inverting twice is the identity.
    assert_eq!(inverted.invert(), p);
}

#[test]
fn polygon_compare_treats_a_polygon_and_its_inversion_as_equal() {
    let p = Polygon3d::new(vec![
        Vec3d::new(-1.0, -1.0, 0.0),
        Vec3d::new(1.0, -1.0, 0.0),
        Vec3d::new(1.0, 1.0, 0.0),
        Vec3d::new(-1.0, 1.0, 0.0),
    ]);
    assert!(polygon::is_equal(&p, &p.invert(), 0.0));
}

#[test]
fn polygon_compare_orders_by_vertex_count_first() {
    let two = Polygon3d::new(vec![Vec3d::zero(), Vec3d::one()]);
    let three = Polygon3d::new(vec![Vec3d::zero(), Vec3d::one(), Vec3d::new(2.0, 0.0, 0.0)]);
    assert!(two < three);
}

#[test]
fn polygon_translate() {
    let p = Polygon3d::new(vec![Vec3d::zero(), Vec3d::new(1.0, 0.0, 0.0)]);
    let moved = p.translate(Vec3d::new(0.0, 0.0, 5.0));
    assert!(moved.has_vertex(Vec3d::new(0.0, 0.0, 5.0)));
    assert!(moved.has_vertex(Vec3d::new(1.0, 0.0, 5.0)));
}

/* ========== convex hull ========== */

#[test]
fn convex_hull_of_a_square_with_an_interior_point() {
    let points = vec![
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(2.0, 0.0, 0.0),
        Vec3d::new(2.0, 2.0, 0.0),
        Vec3d::new(0.0, 2.0, 0.0),
        // Inside the square, so it must not survive.
        Vec3d::new(1.0, 1.0, 0.0),
    ];
    let hull = convex_hull(&points);
    assert_eq!(hull.len(), 4);
    assert!(!hull.contains(&Vec3d::new(1.0, 1.0, 0.0)));
    for corner in &points[..4] {
        assert!(hull.contains(corner));
    }
}

#[test]
fn convex_hull_drops_points_on_an_edge() {
    let hull = convex_hull(&[
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(2.0, 0.0, 0.0),
        Vec3d::new(2.0, 2.0, 0.0),
        Vec3d::new(0.0, 2.0, 0.0),
    ]);
    assert_eq!(hull.len(), 4);
    assert!(!hull.contains(&Vec3d::new(1.0, 0.0, 0.0)));
}

#[test]
fn convex_hull_of_degenerate_input_is_empty() {
    assert!(convex_hull::<f64>(&[]).is_empty());
    assert!(convex_hull(&[Vec3d::zero()]).is_empty());
    assert!(convex_hull(&[Vec3d::zero(), Vec3d::one()]).is_empty());
    // All colinear.
    assert!(convex_hull(&[
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(2.0, 0.0, 0.0),
    ])
    .is_empty());
}

#[test]
fn convex_hull_works_off_the_xy_plane() {
    // The same square, standing up in the XZ plane.
    let hull = convex_hull(&[
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(2.0, 0.0, 0.0),
        Vec3d::new(2.0, 0.0, 2.0),
        Vec3d::new(0.0, 0.0, 2.0),
        Vec3d::new(1.0, 0.0, 1.0),
    ]);
    assert_eq!(hull.len(), 4);
}

/* ========== a plane and a box together ========== */

#[test]
fn a_planes_normal_orients_the_box_face_it_matches() {
    let b = Bbox3d::from_scalars(-1.0, 1.0);
    for face in b.faces() {
        let p = Plane3d::from_anchor_and_normal(face.vertices[0], face.normal);
        // Every vertex of the face lies on its plane, and the box's centre is behind it.
        for v in face.vertices {
            assert_eq!(p.point_status_default(v), PlaneStatus::Inside);
        }
        assert_eq!(p.point_status_default(b.center()), PlaneStatus::Below);
    }
}

#[test]
fn plane_epsilons_are_the_upstream_values() {
    // These drive nearly every geometric decision, so they are worth pinning down.
    assert_eq!(f64::POINT_STATUS_EPSILON, 0.0001);
    assert_eq!(f64::COLINEAR_EPSILON, 0.00001);
    assert_eq!(Plane3f::default().distance, 0.0);
}
