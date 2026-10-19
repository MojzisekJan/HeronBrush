//! Tests for transforming a brush.

use map_model::brush::Brush;
use map_model::shapes;
use geometry_math::bbox::Bbox3d;
use geometry_math::mat_ext;
use geometry_math::scalar;
use geometry_math::util::Axis;
use geometry_math::vec::Vec3d;

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

/// A box twice as long in X as in Y or Z, so a rotation is visible in its bounds.
fn oblong() -> Brush {
    shapes::create_cuboid(
        &world_bounds(),
        &Bbox3d::new(Vec3d::new(-64.0, -16.0, -16.0), Vec3d::new(64.0, 16.0, 16.0)),
        "stone",
    )
    .expect("a box")
}

#[test]
fn translating_by_a_matrix_matches_translating_by_a_delta() {
    let brush = oblong();
    let delta = Vec3d::new(10.0, 20.0, 30.0);

    let by_matrix = brush
        .transform(&world_bounds(), &mat_ext::translation_matrix(delta))
        .expect("still a box");
    let by_delta = brush.translate(&world_bounds(), delta).expect("still a box");

    geometry_math::assert_approx_eq!(by_matrix.bounds().min, by_delta.bounds().min, 1e-6);
    geometry_math::assert_approx_eq!(by_matrix.bounds().max, by_delta.bounds().max, 1e-6);
}

#[test]
fn rotating_a_quarter_turn_swaps_two_of_the_extents() {
    let brush = oblong();
    let rotation = mat_ext::rotation_matrix_about_axis(
        Vec3d::new(0.0, 0.0, 1.0),
        scalar::to_radians(90.0),
    );

    let turned = brush.transform(&world_bounds(), &rotation).expect("still a box");
    turned.geometry().check_invariant().expect("a valid mesh");

    // What was 128 long in X is now 128 long in Y.
    let size = turned.bounds().size();
    geometry_math::assert_approx_eq!(size.x(), 32.0, 1e-6);
    geometry_math::assert_approx_eq!(size.y(), 128.0, 1e-6);
    geometry_math::assert_approx_eq!(size.z(), 32.0, 1e-6);
}

#[test]
fn rotating_four_quarter_turns_returns_the_brush_to_where_it_was() {
    let brush = oblong();
    let rotation = mat_ext::rotation_matrix_about_axis(
        Vec3d::new(0.0, 1.0, 0.0),
        scalar::to_radians(90.0),
    );

    let mut turned = brush.clone();
    for _ in 0..4 {
        turned = turned
            .transform(&world_bounds(), &rotation)
            .expect("still a box");
    }

    geometry_math::assert_approx_eq!(turned.bounds().min, brush.bounds().min, 1e-6);
    geometry_math::assert_approx_eq!(turned.bounds().max, brush.bounds().max, 1e-6);
}

#[test]
fn mirroring_keeps_the_normals_pointing_outwards() {
    // This is what the winding fix is for: a mirror reverses which way round a face's
    // points run, and without putting that back every normal would point inwards.
    let brush = oblong();
    let mirror = mat_ext::mirror_matrix::<f64>(Axis::X);

    let mirrored = brush.transform(&world_bounds(), &mirror).expect("still a box");
    mirrored.geometry().check_invariant().expect("a valid mesh");

    let center = mirrored.bounds().center();
    for (index, (face, _)) in mirrored.geometry_faces().enumerate() {
        assert!(
            face.plane.point_distance(center) < 0.0,
            "face {index} points inwards after mirroring"
        );
    }
}

#[test]
fn mirroring_twice_returns_the_brush_to_where_it_was() {
    let brush = oblong();
    let mirror = mat_ext::mirror_matrix::<f64>(Axis::Y);

    let back = brush
        .transform(&world_bounds(), &mirror)
        .and_then(|m| m.transform(&world_bounds(), &mirror))
        .expect("still a box");

    geometry_math::assert_approx_eq!(back.bounds().min, brush.bounds().min, 1e-6);
    geometry_math::assert_approx_eq!(back.bounds().max, brush.bounds().max, 1e-6);
}

#[test]
fn mirroring_actually_mirrors() {
    // A box offset from the origin lands on the other side of it.
    let brush = shapes::create_cuboid(
        &world_bounds(),
        &Bbox3d::new(Vec3d::new(32.0, -16.0, -16.0), Vec3d::new(96.0, 16.0, 16.0)),
        "stone",
    )
    .expect("a box");

    let mirrored = brush
        .transform(&world_bounds(), &mat_ext::mirror_matrix::<f64>(Axis::X))
        .expect("still a box");

    geometry_math::assert_approx_eq!(mirrored.bounds().min.x(), -96.0, 1e-6);
    geometry_math::assert_approx_eq!(mirrored.bounds().max.x(), -32.0, 1e-6);
}

#[test]
fn scaling_changes_the_size_but_not_the_shape() {
    let brush = oblong();
    let scaled = brush
        .transform(
            &world_bounds(),
            &mat_ext::scaling_matrix(Vec3d::new(2.0, 2.0, 2.0)),
        )
        .expect("still a box");

    scaled.geometry().check_invariant().expect("a valid mesh");
    assert_eq!(scaled.geometry().face_count(), 6);
    geometry_math::assert_approx_eq!(scaled.bounds().size(), brush.bounds().size() * 2.0, 1e-6);
}

#[test]
fn a_transform_keeps_the_materials_on_the_right_faces() {
    let brush = oblong();
    let turned = brush
        .transform(
            &world_bounds(),
            &mat_ext::rotation_matrix_about_axis(
                Vec3d::new(0.0, 0.0, 1.0),
                scalar::to_radians(45.0),
            ),
        )
        .expect("still a box");

    assert_eq!(turned.geometry().face_count(), 6);
    for (face, _) in turned.geometry_faces() {
        assert_eq!(face.material_name, "stone");
    }
}

#[test]
fn transforming_a_shape_that_is_not_a_box() {
    let cylinder = shapes::create_cylinder(
        &world_bounds(),
        &Bbox3d::new(Vec3d::new(-32.0, -32.0, 0.0), Vec3d::new(32.0, 32.0, 64.0)),
        shapes::CircleShape::EdgeAligned { num_sides: 12 },
        Axis::Z,
        "stone",
    )
    .expect("a cylinder");

    let turned = cylinder
        .transform(
            &world_bounds(),
            &mat_ext::rotation_matrix_about_axis(
                Vec3d::new(1.0, 0.0, 0.0),
                scalar::to_radians(90.0),
            ),
        )
        .expect("still a cylinder");

    turned.geometry().check_invariant().expect("a valid mesh");
    assert_eq!(turned.geometry().face_count(), 14);
    // It now lies along Y rather than standing along Z.
    let size = turned.bounds().size();
    geometry_math::assert_approx_eq!(size.y(), 64.0, 1e-6);
    geometry_math::assert_approx_eq!(size.z(), 64.0, 1e-6);
}

/// Whether two brushes have the same corners, whatever order they come in.
///
/// Sorting them and comparing pairwise looks simpler and is wrong: two corners that
/// differ only in their last coordinate sort either way round once a transformation has
/// moved the earlier ones by a millionth, and the comparison then reads as a mismatch of
/// the whole solid.
fn same_corners(a: &map_model::brush::Brush, b: &map_model::brush::Brush) -> bool {
    let (ours, theirs) = (a.vertex_positions(), b.vertex_positions());
    ours.len() == theirs.len()
        && ours.iter().all(|corner| {
            theirs
                .iter()
                .any(|other| geometry_math::vec::distance(*corner, *other) < 1e-6)
        })
}

/// Turning or scaling a brush by carrying its geometry gives what rebuilding would.
///
/// The same claim as for a move, for every transformation that keeps the arrangement of
/// the solid: a turn, a scale, and the two together with a move.
#[test]
fn a_turned_or_scaled_brush_is_what_rebuilding_it_would_give() {
    let world = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
    let brush = map_model::shapes::create_cuboid(
        &world,
        &geometry_math::bbox::Bbox3d::new(Vec3d::new(-16.0, -8.0, -4.0), Vec3d::new(16.0, 8.0, 4.0)),
        "stone",
    )
    .expect("a box");

    let turn = |axis: Vec3d, radians: f64| geometry_math::mat_ext::rotation_matrix_about_axis(axis, radians);
    let transforms = [
        turn(Vec3d::new(0.0, 0.0, 1.0), 0.4),
        turn(Vec3d::new(1.0, 2.0, 3.0), 1.1),
        geometry_math::mat_ext::scaling_matrix(Vec3d::new(2.0, 0.5, 3.0)),
        geometry_math::mat_ext::translation_matrix(Vec3d::new(100.0, 0.0, 0.0))
            * turn(Vec3d::new(0.0, 1.0, 0.0), 0.7),
    ];

    for (which, transform) in transforms.iter().enumerate() {
        for lock in [false, true] {
            let quick = brush
                .transform_with_alignment(&world, transform, lock, |_| {
                    geometry_math::vec::Vec2f::new(64.0, 64.0)
                })
                .expect("it transforms");
            let slow = map_model::brush::Brush::from_faces(&world, quick.faces().to_vec())
                .expect("it rebuilds");

            assert!(same_corners(&quick, &slow), "the two disagree");
            // The box it reports has to be the box it is, since a turn changes which
            // corners are extreme and the old one would be stale.
            let corners = quick.vertex_positions();
            let mut measured = geometry_math::bbox::Bbox3d::new(corners[0], corners[0]);
            for corner in &corners {
                measured = measured.merged_with_point(*corner);
            }
            assert!(
                geometry_math::vec::distance(measured.min, quick.bounds().min) < 1e-6
                    && geometry_math::vec::distance(measured.max, quick.bounds().max) < 1e-6,
                "transform {which}: the bounds are stale"
            );
        }
    }
}

/// Moving a brush by carrying its geometry gives the same brush as working it out again.
///
/// A translation takes a convex solid to the same solid somewhere else, so there is
/// nothing to rebuild -- and a drag of a large selection spends all its time rebuilding.
/// This holds the shortcut to the long way round: same corners, same planes, same
/// materials, same texture coordinates.
#[test]
fn a_translated_brush_is_what_rebuilding_it_would_give() {
    use map_model::brush_face::BrushFace;

    let world = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
    let brush = map_model::shapes::create_cuboid(
        &world,
        &geometry_math::bbox::Bbox3d::new(Vec3d::new(-16.0, -8.0, -4.0), Vec3d::new(16.0, 8.0, 4.0)),
        "stone",
    )
    .expect("a box");

    for by in [
        Vec3d::new(64.0, 0.0, 0.0),
        Vec3d::new(-13.0, 7.0, 129.0),
        Vec3d::new(0.5, 0.5, 0.5),
    ] {
        for lock in [false, true] {
            let shift = geometry_math::mat_ext::translation_matrix(by);
            let quick = brush
                .transform_with_alignment(&world, &shift, lock, |_| geometry_math::vec::Vec2f::new(64.0, 64.0))
                .expect("it moves");

            // The long way round: the same faces, put back through the plane intersection.
            let faces: Vec<BrushFace> = quick.faces().to_vec();
            let slow = map_model::brush::Brush::from_faces(&world, faces).expect("it rebuilds");

            assert!(same_corners(&quick, &slow), "the two disagree");

            // And it really moved.
            let moved = geometry_math::vec::distance(quick.bounds().center(), brush.bounds().center() + by);
            assert!(moved < 1e-9, "by {by:?}: it landed at {:?}", quick.bounds().center());
        }
    }
}

/// A thin brush moved a long way arrives, rather than being refused.
///
/// Rebuilding a brush from its face planes loses relative precision as the coordinates
/// grow, and a brush four units thick collapses to nothing long before it reaches the
/// edge of the world. A translation does not need rebuilding, and the difference is not
/// academic: it is a drag that stops following the pointer the moment the pointer gets
/// far enough ahead, and does not start again until it comes back.
#[test]
fn a_thin_brush_can_be_moved_a_long_way() {
    let world = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
    // As thin as a wedge of the monkey's shell, and as wide.
    let thin = map_model::shapes::create_cuboid(
        &world,
        &geometry_math::bbox::Bbox3d::new(Vec3d::new(-64.0, -64.0, 0.0), Vec3d::new(64.0, 64.0, 4.0)),
        "stone",
    )
    .expect("a slab");

    for far in [16.0, 256.0, 1024.0, 8192.0, 16000.0] {
        let moved = thin
            .translate_with_alignment(
                &world,
                Vec3d::new(far, 0.0, 0.0),
                true,
                |_| geometry_math::vec::Vec2f::new(64.0, 64.0),
            )
            .unwrap_or_else(|why| panic!("a move of {far} was refused: {why}"));

        // It arrived, and it is still four units thick.
        let bounds = moved.bounds();
        assert!(
            (bounds.center().x() - far).abs() < 1e-6,
            "{far}: landed at {:?}",
            bounds.center()
        );
        assert!((bounds.size().z() - 4.0).abs() < 1e-6, "{far}: {bounds:?}");
    }
}

/// Moving a brush by carrying its geometry gives the same brush as rebuilding it.
#[test]
fn a_moved_brush_is_what_rebuilding_it_would_give() {
    let world = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
    let brush = map_model::shapes::create_cuboid(
        &world,
        &geometry_math::bbox::Bbox3d::new(Vec3d::new(-16.0, -8.0, -4.0), Vec3d::new(16.0, 8.0, 4.0)),
        "stone",
    )
    .expect("a box");

    for by in [Vec3d::new(64.0, 0.0, 0.0), Vec3d::new(-13.0, 7.0, 129.0)] {
        for lock in [false, true] {
            let quick = brush
                .translate_with_alignment(&world, by, lock, |_| geometry_math::vec::Vec2f::new(64.0, 64.0))
                .expect("it moves");
            let slow = map_model::brush::Brush::from_faces(&world, quick.faces().to_vec())
                .expect("it rebuilds");

            assert!(same_corners(&quick, &slow), "the two disagree");
        }
    }
}

