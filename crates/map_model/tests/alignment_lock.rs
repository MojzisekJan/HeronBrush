//! Whether a material stays where it is on a face as the face moves.
//!
//! `Preferences::AlignmentLock` decides this, and getting it wrong is the kind of thing
//! that is only noticed after a room has been built: every wall's material has crept.

use geometry_math::bbox::Bbox3d;
use geometry_math::mat::Mat4x4d;
use geometry_math::mat_ext;
use geometry_math::scalar::Float as _;
use geometry_math::util::Axis;
use geometry_math::vec::{self, Vec2f, Vec3d};
use map_model::brush::Brush;
use map_model::uv::UvCoordSystem;

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

fn cube() -> Brush {
    map_model::shapes::create_cuboid(
        &world_bounds(),
        &Bbox3d::new(
            Vec3d::new(-32.0, -32.0, -32.0),
            Vec3d::new(32.0, 32.0, 32.0),
        ),
        "stone",
    )
    .expect("a cube")
}

/// A 64 by 64 material, which is the usual size in a Quake-lineage map.
fn material_size(_: &str) -> Vec2f {
    Vec2f::new(64.0, 64.0)
}

/// The index of the face pointing along the given direction.
fn face_facing(brush: &Brush, direction: Vec3d) -> usize {
    brush
        .faces()
        .iter()
        .position(|face| vec::dot(face.plane.normal, direction) > 0.99)
        .expect("a face facing that way")
}

/// Where the given world point falls on the given face's material, in texels.
fn uv_at(brush: &Brush, face: usize, point: Vec3d) -> Vec2f {
    let face = &brush.faces()[face];
    face.uv_coord_system
        .uv_coords(point, face.plane.normal, &face.uv_attributes)
}

#[test]
fn with_the_lock_on_a_moved_face_wears_the_same_part_of_the_material() {
    let brush = cube();
    let up = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
    // A corner of the top face, which is a point the move carries with it.
    let corner = Vec3d::new(-32.0, -32.0, 32.0);
    let before = uv_at(&brush, up, corner);

    let delta = Vec3d::new(16.0, 48.0, 0.0);
    let moved = brush
        .translate_with_alignment(&world_bounds(), delta, true, material_size)
        .expect("still a box");
    let up = face_facing(&moved, Vec3d::new(0.0, 0.0, 1.0));
    let after = uv_at(&moved, up, corner + delta);

    // The same corner wears the same texel, up to the whole repeats the offset is kept
    // inside.
    let difference = after - before;
    assert!(
        (difference.x() / 64.0).fract().abs() < 0.001
            && (difference.y() / 64.0).fract().abs() < 0.001,
        "the material moved by {difference:?}"
    );
}

#[test]
fn with_the_lock_off_a_moved_face_slides_across_the_material() {
    let brush = cube();
    let up = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
    let corner = Vec3d::new(-32.0, -32.0, 32.0);
    let before = uv_at(&brush, up, corner);

    let delta = Vec3d::new(16.0, 0.0, 0.0);
    let moved = brush
        .translate_with_alignment(&world_bounds(), delta, false, material_size)
        .expect("still a box");
    let up = face_facing(&moved, Vec3d::new(0.0, 0.0, 1.0));
    let after = uv_at(&moved, up, corner + delta);

    // Without the lock the material stays put in the world, so the corner has slid across
    // it by exactly the distance moved.
    assert!(
        (after.x() - before.x() - 16.0).abs() < 0.001,
        "{before:?} {after:?}"
    );
}

#[test]
fn a_moved_face_keeps_its_offset_small() {
    // The offset is brought back inside the material's own repeat each time, so dragging
    // a brush a long way does not leave a number that grows without bound.
    let mut brush = cube();
    for _ in 0..40 {
        brush = brush
            .translate_with_alignment(
                &world_bounds(),
                Vec3d::new(64.0, 0.0, 0.0),
                true,
                material_size,
            )
            .expect("still a box");
    }

    for face in brush.faces() {
        assert!(
            face.uv_attributes.offset.x().abs() <= 64.0
                && face.uv_attributes.offset.y().abs() <= 64.0,
            "the offset drifted to {:?}",
            face.uv_attributes.offset
        );
    }
}

#[test]
fn with_the_lock_on_a_turned_face_wears_the_same_part_of_the_material() {
    // Turning is the harder case: the face ends up over a different coordinate plane, so
    // the paraxial axes it maps with are different ones.
    let brush = cube();
    let front = face_facing(&brush, Vec3d::new(1.0, 0.0, 0.0));
    let corner = Vec3d::new(32.0, -32.0, 32.0);
    let before = uv_at(&brush, front, corner);

    let turn =
        mat_ext::rotation_matrix_about_axis(Vec3d::new(0.0, 0.0, 1.0), std::f64::consts::FRAC_PI_2);
    let turned = brush
        .transform_with_alignment(&world_bounds(), &turn, true, material_size)
        .expect("still a box");
    // The face that pointed along X now points along Y.
    let front = face_facing(&turned, Vec3d::new(0.0, 1.0, 0.0));
    let after = uv_at(&turned, front, turn * corner);

    let difference = after - before;
    assert!(
        (difference.x() / 64.0).fract().abs() < 0.01
            && (difference.y() / 64.0).fract().abs() < 0.01,
        "the material moved by {difference:?}"
    );
}

/// Whether every corner of the face that pointed along `from` wears the same texel
/// after `transform` as before, on the face that now points along `to`.
fn corners_keep_their_texels(transform: &Mat4x4d, from: Vec3d, to: Vec3d) {
    let brush = cube();
    let before_face = face_facing(&brush, from);
    let after = brush
        .transform_with_alignment(&world_bounds(), transform, true, material_size)
        .expect("still a box");
    let after_face = face_facing(&after, to);
    for corner in brush.faces()[before_face].points {
        let difference =
            uv_at(&after, after_face, *transform * corner) - uv_at(&brush, before_face, corner);
        assert!(
            (difference.x() / 64.0).fract().abs() < 0.01
                && (difference.y() / 64.0).fract().abs() < 0.01,
            "{corner:?} moved on the material by {difference:?}"
        );
    }
}

#[test]
fn with_the_lock_on_a_mirrored_face_wears_the_mirror_image_of_the_material() {
    // Mirroring across Y leaves the face looking along X but runs its U axis the other
    // way. Text on it was coming out upside down instead of reading the other way round.
    let x = Vec3d::new(1.0, 0.0, 0.0);
    corners_keep_their_texels(&mat_ext::mirror_matrix(Axis::Y), x, x);
}

#[test]
fn with_the_lock_on_a_face_turned_half_round_on_itself_turns_its_material() {
    // A half turn about the face's own normal flips both axes. Both scales came out
    // negative on top of the 180 degree rotation, which undid the turn.
    let x = Vec3d::new(1.0, 0.0, 0.0);
    let turn = mat_ext::rotation_matrix_about_axis(x, std::f64::consts::PI);
    corners_keep_their_texels(&turn, x, x);
}

#[test]
fn a_valve_face_keeps_its_own_axes_through_a_move() {
    // A parallel system stores its axes rather than looking them up, so the lock has to
    // rewrite them; this checks it writes axes that still lie in the face's plane.
    let plain = cube();
    let up = face_facing(&plain, Vec3d::new(0.0, 0.0, 1.0));
    let normal = plain.faces()[up].plane.normal;
    let (u_axis, v_axis) = map_model::uv::parallel_axes(normal);

    let mut faces = plain.faces().to_vec();
    faces[up].uv_coord_system = UvCoordSystem::Parallel { u_axis, v_axis };
    let brush = Brush::from_faces(&world_bounds(), faces).expect("still a box");

    let moved = brush
        .translate_with_alignment(
            &world_bounds(),
            Vec3d::new(24.0, 0.0, 0.0),
            true,
            material_size,
        )
        .expect("still a box");
    let up = face_facing(&moved, Vec3d::new(0.0, 0.0, 1.0));
    let UvCoordSystem::Parallel { u_axis, v_axis } = moved.faces()[up].uv_coord_system else {
        panic!("the face is still a parallel one");
    };

    assert!(vec::dot(u_axis, normal).abs() < 0.001, "{u_axis:?}");
    assert!(vec::dot(v_axis, normal).abs() < 0.001, "{v_axis:?}");
}

#[test]
fn with_the_uv_lock_off_a_dragged_edge_leaves_the_material_alone() {
    // With UV lock disabled, the moving face slides across its existing material mapping.
    let brush = cube();
    let up = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
    let before = brush.faces()[up].uv_attributes;

    let stretched = brush
        .move_vertices_with_uv_lock(
            &[Vec3d::new(32.0, 32.0, 32.0), Vec3d::new(32.0, -32.0, 32.0)],
            Vec3d::new(32.0, 0.0, 0.0),
            f64::ALMOST_ZERO,
            false,
            material_size,
        )
        .expect("still a brush");
    let up = face_facing(&stretched, Vec3d::new(0.0, 0.0, 1.0));

    assert_eq!(stretched.faces()[up].uv_attributes, before);
}

#[test]
fn with_the_uv_lock_on_a_dragged_edge_stretches_the_material_with_it() {
    let brush = cube();
    let up = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
    let before = brush.faces()[up].uv_attributes;

    // A whole edge of the top face, so two of its four corners stay put. Dragging one
    // corner alone leaves three, which `findTransformForUvLock` gives up on: there is no
    // one transformation that holds all four mappings.
    let stretched = brush
        .move_vertices_with_uv_lock(
            &[Vec3d::new(32.0, 32.0, 32.0), Vec3d::new(32.0, -32.0, 32.0)],
            Vec3d::new(32.0, 0.0, 0.0),
            f64::ALMOST_ZERO,
            true,
            material_size,
        )
        .expect("still a brush");
    let up = face_facing(&stretched, Vec3d::new(0.0, 0.0, 1.0));
    let after = stretched.faces()[up].uv_attributes;

    // The top face was stretched, so its mapping is not the one it started with.
    assert_ne!(after, before, "the material did not follow the corner");
    // And it is still a usable mapping rather than something degenerate.
    assert!(after.scale.x().is_finite() && after.scale.y().is_finite());
    assert!(after.scale.x() != 0.0 && after.scale.y() != 0.0);
    assert!(after.offset.x().is_finite() && after.offset.y().is_finite());
}

#[test]
fn a_face_with_three_corners_staying_put_keeps_its_material_under_the_uv_lock() {
    // `findTransformForUvLock` gives up when three or more corners do not move: picture a
    // square with one corner dragged, and there is no one transformation that holds all
    // four mappings. The face keeps what it had rather than getting a wrong answer.
    let brush = cube();
    let up = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
    let before = brush.faces()[up].uv_attributes;

    let stretched = brush
        .move_vertices_with_uv_lock(
            // Sideways, so the top face stays flat and still faces upwards; only three
            // of its four corners stay put.
            &[Vec3d::new(32.0, 32.0, 32.0)],
            Vec3d::new(16.0, 0.0, 0.0),
            f64::ALMOST_ZERO,
            true,
            material_size,
        )
        .expect("still a brush");
    let up = face_facing(&stretched, Vec3d::new(0.0, 0.0, 1.0));

    assert_eq!(stretched.faces()[up].uv_attributes, before);
}

#[test]
fn the_uv_lock_never_writes_a_mapping_with_no_scale() {
    // A zero scale would divide by zero everywhere the mapping is read, so every drag is
    // checked rather than the one the other tests happen to make.
    let brush = cube();
    for corner in [
        Vec3d::new(32.0, 32.0, 32.0),
        Vec3d::new(-32.0, 32.0, 32.0),
        Vec3d::new(-32.0, -32.0, -32.0),
    ] {
        for delta in [
            Vec3d::new(16.0, 0.0, 0.0),
            Vec3d::new(0.0, 0.0, 24.0),
            Vec3d::new(-8.0, 12.0, 4.0),
        ] {
            let moved = brush
                .move_vertices_with_uv_lock(&[corner], delta, f64::ALMOST_ZERO, true, material_size)
                .expect("still a brush");
            for face in moved.faces() {
                assert!(
                    face.uv_attributes.scale.x() != 0.0 && face.uv_attributes.scale.y() != 0.0,
                    "corner {corner:?} by {delta:?} left {:?}",
                    face.uv_attributes
                );
                assert!(
                    face.uv_attributes.offset.x().is_finite()
                        && face.uv_attributes.offset.y().is_finite()
                        && face.uv_attributes.rotation.is_finite(),
                    "corner {corner:?} by {delta:?} left {:?}",
                    face.uv_attributes
                );
            }
        }
    }
}
