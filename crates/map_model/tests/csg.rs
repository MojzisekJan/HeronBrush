//! Tests for cutting brushes against each other.

use map_model::brush::{Brush, ClipOutcome};
use map_model::csg;
use map_model::shapes;
use geometry_math::bbox::Bbox3d;
use geometry_math::plane::Plane3d;
use geometry_math::vec::Vec3d;

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

fn boxed(min: Vec3d, max: Vec3d) -> Brush {
    shapes::create_cuboid(&world_bounds(), &Bbox3d::new(min, max), "stone").expect("a box")
}

/// The total volume of a set of brushes, by summing their bounding boxes.
///
/// Only meaningful for axis-aligned pieces that do not overlap, which is what the
/// subtractions below produce.
fn total_volume(brushes: &[Brush]) -> f64 {
    brushes.iter().map(|b| b.bounds().volume()).sum()
}

/* ========== clipping one brush ========== */

#[test]
fn clipping_a_box_in_half_keeps_the_half_behind_the_plane() {
    let cube = boxed(Vec3d::new(-32.0, -32.0, -32.0), Vec3d::new(32.0, 32.0, 32.0));
    let plane = Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));

    let ClipOutcome::Clipped(half) = cube.clip(&plane) else {
        panic!("the plane cuts the cube");
    };
    half.geometry().check_invariant().expect("a valid mesh");
    geometry_math::assert_approx_eq!(half.bounds().max.z(), 0.0, 1e-6);
    geometry_math::assert_approx_eq!(half.bounds().min.z(), -32.0, 1e-6);
}

#[test]
fn a_plane_that_misses_leaves_the_brush_unchanged() {
    let cube = boxed(Vec3d::new(-32.0, -32.0, -32.0), Vec3d::new(32.0, 32.0, 32.0));

    let above = Plane3d::from_anchor_and_normal(
        Vec3d::new(0.0, 0.0, 100.0),
        Vec3d::new(0.0, 0.0, 1.0),
    );
    assert!(matches!(cube.clip(&above), ClipOutcome::Unchanged));

    let below = Plane3d::from_anchor_and_normal(
        Vec3d::new(0.0, 0.0, -100.0),
        Vec3d::new(0.0, 0.0, 1.0),
    );
    assert!(matches!(cube.clip(&below), ClipOutcome::Empty));
}

#[test]
fn a_clipped_brush_keeps_its_materials() {
    let cube = boxed(Vec3d::new(-32.0, -32.0, -32.0), Vec3d::new(32.0, 32.0, 32.0));
    let plane = Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));

    let ClipOutcome::Clipped(half) = cube.clip(&plane) else {
        panic!("the plane cuts the cube");
    };
    for (face, _) in half.geometry_faces() {
        assert_eq!(face.material_name, "stone");
    }
}

/* ========== subtraction ========== */

#[test]
fn subtracting_a_brush_that_is_nowhere_near_changes_nothing() {
    let target = boxed(Vec3d::zero(), Vec3d::new(64.0, 64.0, 64.0));
    let far_away = boxed(
        Vec3d::new(1000.0, 1000.0, 1000.0),
        Vec3d::new(1064.0, 1064.0, 1064.0),
    );

    let pieces = csg::subtract(&target, &far_away);
    assert_eq!(pieces.len(), 1);
    geometry_math::assert_approx_eq!(pieces[0].bounds().min, target.bounds().min, 1e-6);
}

#[test]
fn subtracting_a_brush_that_swallows_the_target_leaves_nothing() {
    let target = boxed(Vec3d::new(-8.0, -8.0, -8.0), Vec3d::new(8.0, 8.0, 8.0));
    let swallower = boxed(
        Vec3d::new(-64.0, -64.0, -64.0),
        Vec3d::new(64.0, 64.0, 64.0),
    );

    assert!(csg::subtract(&target, &swallower).is_empty());
}

#[test]
fn cutting_a_notch_out_of_a_slab() {
    // A wall with a doorway cut out of one edge: the classic use.
    let wall = boxed(
        Vec3d::new(-64.0, -8.0, 0.0),
        Vec3d::new(64.0, 8.0, 64.0),
    );
    let doorway = boxed(
        Vec3d::new(-16.0, -16.0, 0.0),
        Vec3d::new(16.0, 16.0, 40.0),
    );

    let pieces = csg::subtract(&wall, &doorway);
    assert!(!pieces.is_empty());
    for piece in &pieces {
        piece.geometry().check_invariant().expect("a valid mesh");
    }

    // The pieces together fill the wall, less the doorway.
    let expected = wall.bounds().volume() - 32.0 * 16.0 * 40.0;
    let actual = total_volume(&pieces);
    assert!(
        (actual - expected).abs() < 1.0,
        "the pieces should account for {expected}, not {actual}"
    );
}

#[test]
fn a_hole_through_the_middle_leaves_pieces_all_around_it() {
    let slab = boxed(
        Vec3d::new(-64.0, -64.0, 0.0),
        Vec3d::new(64.0, 64.0, 16.0),
    );
    let hole = boxed(
        Vec3d::new(-16.0, -16.0, -8.0),
        Vec3d::new(16.0, 16.0, 24.0),
    );

    let pieces = csg::subtract(&slab, &hole);
    // Four pieces around the hole, since the hole goes right through.
    assert_eq!(pieces.len(), 4, "{:?}", pieces.iter().map(|p| p.bounds()).collect::<Vec<_>>());

    let expected = slab.bounds().volume() - 32.0 * 32.0 * 16.0;
    assert!((total_volume(&pieces) - expected).abs() < 1.0);
}

#[test]
fn the_pieces_of_a_subtraction_do_not_overlap() {
    let slab = boxed(
        Vec3d::new(-64.0, -64.0, 0.0),
        Vec3d::new(64.0, 64.0, 16.0),
    );
    let hole = boxed(
        Vec3d::new(-16.0, -16.0, -8.0),
        Vec3d::new(16.0, 16.0, 24.0),
    );
    let pieces = csg::subtract(&slab, &hole);

    for (i, a) in pieces.iter().enumerate() {
        for b in &pieces[i + 1..] {
            assert!(
                csg::intersect(a, b).is_none(),
                "two pieces share volume: {:?} and {:?}",
                a.bounds(),
                b.bounds()
            );
        }
    }
}

#[test]
fn nothing_of_the_tool_survives_in_the_pieces() {
    let slab = boxed(
        Vec3d::new(-64.0, -64.0, 0.0),
        Vec3d::new(64.0, 64.0, 16.0),
    );
    let hole = boxed(
        Vec3d::new(-16.0, -16.0, -8.0),
        Vec3d::new(16.0, 16.0, 24.0),
    );
    let pieces = csg::subtract(&slab, &hole);

    // The middle of the hole is in none of them.
    let inside_hole = Vec3d::new(0.0, 0.0, 8.0);
    for piece in &pieces {
        assert!(!piece.contains_point(inside_hole), "{:?}", piece.bounds());
    }
    // But a point in the slab outside the hole is in one of them.
    let outside_hole = Vec3d::new(48.0, 0.0, 8.0);
    assert!(pieces.iter().any(|p| p.contains_point(outside_hole)));
}

#[test]
fn subtracting_several_tools_in_turn() {
    let slab = boxed(
        Vec3d::new(-64.0, -64.0, 0.0),
        Vec3d::new(64.0, 64.0, 16.0),
    );
    let holes = [
        boxed(Vec3d::new(-48.0, -16.0, -8.0), Vec3d::new(-16.0, 16.0, 24.0)),
        boxed(Vec3d::new(16.0, -16.0, -8.0), Vec3d::new(48.0, 16.0, 24.0)),
    ];

    let pieces = csg::subtract_all(&slab, &holes);
    for piece in &pieces {
        piece.geometry().check_invariant().expect("a valid mesh");
        for hole in &holes {
            assert!(csg::intersect(piece, hole).is_none());
        }
    }

    let expected = slab.bounds().volume() - 2.0 * (32.0 * 32.0 * 16.0);
    assert!((total_volume(&pieces) - expected).abs() < 1.0);
}

/* ========== intersection ========== */

#[test]
fn the_intersection_of_two_overlapping_boxes_is_their_overlap() {
    let a = boxed(Vec3d::zero(), Vec3d::new(64.0, 64.0, 64.0));
    let b = boxed(Vec3d::new(32.0, 32.0, 32.0), Vec3d::new(96.0, 96.0, 96.0));

    let overlap = csg::intersect(&a, &b).expect("they overlap");
    overlap.geometry().check_invariant().expect("a valid mesh");

    geometry_math::assert_approx_eq!(overlap.bounds().min, Vec3d::new(32.0, 32.0, 32.0), 1e-6);
    geometry_math::assert_approx_eq!(overlap.bounds().max, Vec3d::new(64.0, 64.0, 64.0), 1e-6);
}

#[test]
fn boxes_that_do_not_overlap_have_no_intersection() {
    let a = boxed(Vec3d::zero(), Vec3d::new(32.0, 32.0, 32.0));
    let b = boxed(Vec3d::new(64.0, 64.0, 64.0), Vec3d::new(96.0, 96.0, 96.0));
    assert!(csg::intersect(&a, &b).is_none());
}

#[test]
fn a_brush_intersected_with_itself_is_itself() {
    let a = boxed(Vec3d::zero(), Vec3d::new(64.0, 64.0, 64.0));
    let same = csg::intersect(&a, &a).expect("a brush overlaps itself");

    geometry_math::assert_approx_eq!(same.bounds().min, a.bounds().min, 1e-6);
    geometry_math::assert_approx_eq!(same.bounds().max, a.bounds().max, 1e-6);
}

#[test]
fn intersecting_works_on_shapes_that_are_not_boxes() {
    use geometry_math::util::Axis;

    let cylinder = shapes::create_cylinder(
        &world_bounds(),
        &Bbox3d::new(Vec3d::new(-32.0, -32.0, 0.0), Vec3d::new(32.0, 32.0, 64.0)),
        shapes::CircleShape::EdgeAligned { num_sides: 12 },
        Axis::Z,
        "stone",
    )
    .expect("a cylinder");
    let slab = boxed(
        Vec3d::new(-64.0, -64.0, 16.0),
        Vec3d::new(64.0, 64.0, 32.0),
    );

    let disc = csg::intersect(&cylinder, &slab).expect("they overlap");
    disc.geometry().check_invariant().expect("a valid mesh");
    // A slice of the cylinder: as tall as the slab, as wide as the cylinder.
    geometry_math::assert_approx_eq!(disc.bounds().min.z(), 16.0, 1e-6);
    geometry_math::assert_approx_eq!(disc.bounds().max.z(), 32.0, 1e-6);
}

#[test]
fn intersects_agrees_with_intersect() {
    let a = boxed(Vec3d::zero(), Vec3d::new(64.0, 64.0, 64.0));
    let overlapping = boxed(Vec3d::new(32.0, 32.0, 32.0), Vec3d::new(96.0, 96.0, 96.0));
    let apart = boxed(Vec3d::new(200.0, 0.0, 0.0), Vec3d::new(264.0, 64.0, 64.0));

    assert!(a.intersects(&overlapping));
    assert!(!a.intersects(&apart));
}
