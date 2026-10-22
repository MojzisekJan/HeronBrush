//! Tests for the UV operations a material editor drives.

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::{self, Vec2f, Vec3d};
use map_model::brush::Brush;
use map_model::shapes;
use map_model::uv::{self, UvCoordSystem};

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

fn cube() -> Brush {
    shapes::create_cuboid(&world_bounds(), &Bbox3d::from_scalars(-32.0, 32.0), "stone")
        .expect("a box")
}

/// The +Z face of a cube, which the paraxial system maps as a floor.
fn top_face(brush: &Brush) -> map_model::brush_face::BrushFace {
    brush
        .faces()
        .iter()
        .find(|f| f.normal().z() > 0.9)
        .expect("a box has a top")
        .clone()
}

#[test]
fn moving_the_uv_shifts_the_coordinates() {
    let mut face = top_face(&cube());
    let point = Vec3d::new(0.0, 0.0, 32.0);
    let before = face.uv_coords(point);

    face.move_uv(Vec2f::new(16.0, -8.0));
    let after = face.uv_coords(point);

    geometry_math::assert_approx_eq!(after - before, Vec2f::new(16.0, -8.0), 1e-4);
}

#[test]
fn scaling_the_uv_multiplies_the_coordinates() {
    let mut face = top_face(&cube());
    let point = Vec3d::new(16.0, 16.0, 32.0);
    let before = face.uv_coords(point);

    face.scale_uv(Vec2f::new(2.0, 2.0));
    let after = face.uv_coords(point);

    // A material twice as large covers the same face in half as many texels.
    geometry_math::assert_approx_eq!(after, before / 2.0, 1e-4);
}

#[test]
fn a_zero_scale_factor_is_ignored() {
    let mut face = top_face(&cube());
    face.scale_uv(Vec2f::new(0.0, 4.0));
    // The x scale is untouched, since a zero scale has no meaning.
    assert_eq!(face.uv_attributes.scale.x(), 1.0);
    assert_eq!(face.uv_attributes.scale.y(), 4.0);
}

#[test]
fn flipping_negates_one_scale_axis() {
    let mut face = top_face(&cube());

    face.flip_uv(true);
    assert_eq!(face.uv_attributes.scale, Vec2f::new(-1.0, 1.0));
    face.flip_uv(false);
    assert_eq!(face.uv_attributes.scale, Vec2f::new(-1.0, -1.0));
    // Flipping twice on the same axis puts it back.
    face.flip_uv(true);
    assert_eq!(face.uv_attributes.scale, Vec2f::new(1.0, -1.0));
}

#[test]
fn rotating_a_paraxial_face_turns_the_material() {
    let mut face = top_face(&cube());
    let point = Vec3d::new(32.0, 0.0, 32.0);
    let before = face.uv_coords(point);

    face.rotate_uv(90.0);
    assert_eq!(face.uv_attributes.rotation, 90.0);

    let after = face.uv_coords(point);
    // The point is the same distance from the material's origin, in a different direction.
    geometry_math::assert_approx_eq!(
        geometry_math::vec::length(after),
        geometry_math::vec::length(before),
        1e-3
    );
    assert!(
        geometry_math::vec::distance(after, before) > 1.0,
        "rotating should have moved it: {before:?} to {after:?}"
    );
}

#[test]
fn rotation_wraps_around() {
    let mut face = top_face(&cube());
    face.rotate_uv(350.0);
    face.rotate_uv(20.0);
    // 370 degrees is 10, not 370.
    assert!((face.uv_attributes.rotation - 10.0).abs() < 1e-3);
}

#[test]
fn rotating_a_valve_face_turns_its_axes() {
    let mut face = top_face(&cube());
    let (u, v) = uv::parallel_axes(face.normal());
    face.uv_coord_system = UvCoordSystem::Parallel {
        u_axis: u,
        v_axis: v,
    };

    face.rotate_uv(90.0);

    let UvCoordSystem::Parallel { u_axis, v_axis } = face.uv_coord_system else {
        panic!("it should still be a parallel system");
    };
    // The axes turned, and are still perpendicular and in the face's plane.
    assert!(vec::distance(u_axis, u) > 0.5, "the U axis did not turn");
    geometry_math::assert_approx_eq!(vec::dot(u_axis, v_axis), 0.0, 1e-9);
    geometry_math::assert_approx_eq!(vec::dot(u_axis, face.normal()), 0.0, 1e-9);
}

#[test]
fn resetting_returns_the_defaults() {
    let mut face = top_face(&cube());
    face.move_uv(Vec2f::new(37.0, -12.0));
    face.scale_uv(Vec2f::new(3.0, 3.0));
    face.rotate_uv(45.0);

    face.reset_uv();

    assert_eq!(face.uv_attributes.offset, Vec2f::zero());
    assert_eq!(face.uv_attributes.scale, Vec2f::new(1.0, 1.0));
    assert_eq!(face.uv_attributes.rotation, 0.0);
}

#[test]
fn a_plain_reset_keeps_a_valve_face_on_free_axes() {
    let mut face = top_face(&cube());
    let (u, v) = uv::parallel_axes(face.normal());
    face.uv_coord_system = UvCoordSystem::Parallel {
        u_axis: u,
        v_axis: v,
    };
    face.rotate_uv(30.0); // turn the axes off the world

    face.reset_uv();

    assert!(
        matches!(face.uv_coord_system, UvCoordSystem::Parallel { .. }),
        "a plain reset leaves a Valve face parallel"
    );
}

#[test]
fn resetting_to_world_forces_the_paraxial_axes() {
    let mut face = top_face(&cube());
    let (u, v) = uv::parallel_axes(face.normal());
    face.uv_coord_system = UvCoordSystem::Parallel {
        u_axis: u,
        v_axis: v,
    };
    face.move_uv(Vec2f::new(9.0, 9.0));
    face.rotate_uv(30.0);

    face.reset_uv_to_paraxial();

    assert_eq!(
        face.uv_coord_system,
        UvCoordSystem::Paraxial,
        "dropped to world-aligned axes"
    );
    assert_eq!(face.uv_attributes, map_model::uv::UvAttributes::default());
}

#[test]
fn parallel_axes_lie_in_the_face_plane_and_are_perpendicular() {
    // Including faces at an angle, which is the case the parallel system exists for.
    for normal in [
        Vec3d::new(0.0, 0.0, 1.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, -1.0, 0.0),
        vec::normalize(Vec3d::new(1.0, 1.0, 1.0)),
        vec::normalize(Vec3d::new(-2.0, 0.5, 3.0)),
    ] {
        let (u, v) = uv::parallel_axes(normal);

        geometry_math::assert_approx_eq!(vec::length(u), 1.0, 1e-9);
        geometry_math::assert_approx_eq!(vec::length(v), 1.0, 1e-9);
        geometry_math::assert_approx_eq!(vec::dot(u, normal), 0.0, 1e-9);
        geometry_math::assert_approx_eq!(vec::dot(v, normal), 0.0, 1e-9);
        geometry_math::assert_approx_eq!(vec::dot(u, v), 0.0, 1e-9);
    }
}

#[test]
fn fitting_covers_the_face_with_exactly_one_copy() {
    let cube = cube();
    let mut face = top_face(&cube);
    let polygon: Vec<Vec3d> = cube
        .geometry_faces()
        .find(|(f, _)| f.normal().z() > 0.9)
        .map(|(_, polygon)| polygon)
        .expect("a box has a top");

    let size = Vec2f::new(64.0, 64.0);
    face.fit_uv_to_face(&polygon, size);

    // Every corner maps into the material, and the extremes reach its edges.
    let coords: Vec<Vec2f> = polygon.iter().map(|p| face.uv_coords(*p)).collect();
    let min = coords
        .iter()
        .fold(coords[0], |a, c| geometry_math::vec::min(a, *c));
    let max = coords
        .iter()
        .fold(coords[0], |a, c| geometry_math::vec::max(a, *c));

    geometry_math::assert_approx_eq!(min, Vec2f::zero(), 1e-3);
    geometry_math::assert_approx_eq!(max, size, 1e-3);
}

#[test]
fn fitting_a_face_to_a_material_of_a_different_shape() {
    let cube = cube();
    let mut face = top_face(&cube);
    let polygon: Vec<Vec3d> = cube
        .geometry_faces()
        .find(|(f, _)| f.normal().z() > 0.9)
        .map(|(_, polygon)| polygon)
        .expect("a box has a top");

    // A material twice as wide as it is tall still covers the square face exactly once.
    let size = Vec2f::new(128.0, 64.0);
    face.fit_uv_to_face(&polygon, size);

    let coords: Vec<Vec2f> = polygon.iter().map(|p| face.uv_coords(*p)).collect();
    let max = coords
        .iter()
        .fold(coords[0], |a, c| geometry_math::vec::max(a, *c));
    geometry_math::assert_approx_eq!(max, size, 1e-3);
}

#[test]
fn fitting_ignores_a_material_with_no_size() {
    let mut face = top_face(&cube());
    face.move_uv(Vec2f::new(5.0, 5.0));
    let before = face.uv_attributes;

    face.fit_uv_to_face(&[Vec3d::zero()], Vec2f::zero());
    // Nothing to fit to, so nothing changed.
    assert_eq!(face.uv_attributes.offset, before.offset);
}

#[test]
fn uv_coordinates_are_continuous_across_a_face() {
    // Two points a known distance apart on a face map that far apart in texels, at the
    // default scale.
    let face = top_face(&cube());
    let a = face.uv_coords(Vec3d::new(0.0, 0.0, 32.0));
    let b = face.uv_coords(Vec3d::new(32.0, 0.0, 32.0));
    geometry_math::assert_approx_eq!(geometry_math::vec::distance(a, b), 32.0, 1e-3);
}

#[test]
fn the_two_uv_systems_agree_on_an_axis_aligned_face() {
    // On a face square to the world, the parallel system's default axes give the same
    // mapping the paraxial one does. That is what makes converting a map between the two
    // formats leave axis-aligned geometry looking unchanged.
    let cube = cube();
    let paraxial = top_face(&cube);

    let mut parallel = paraxial.clone();
    let (u, v) = uv::parallel_axes(parallel.normal());
    parallel.uv_coord_system = UvCoordSystem::Parallel {
        u_axis: u,
        v_axis: v,
    };

    for point in [
        Vec3d::new(0.0, 0.0, 32.0),
        Vec3d::new(32.0, -16.0, 32.0),
        Vec3d::new(-8.0, 24.0, 32.0),
    ] {
        geometry_math::assert_approx_eq!(
            parallel.uv_coords(point),
            paraxial.uv_coords(point),
            1e-4
        );
    }
}

/* ========== turning and scaling about a chosen point ========== */

/// A face mapped by the paraxial system, which is what the Standard format uses.
fn paraxial_face() -> map_model::brush_face::BrushFace {
    let mut face = top_face(&cube());
    face.uv_coord_system = UvCoordSystem::Paraxial;
    face.uv_attributes.offset = Vec2f::new(5.0, -3.0);
    face.uv_attributes.rotation = 15.0;
    face
}

/// A face mapped by the parallel system, which is what Valve 220 stores.
fn parallel_face() -> map_model::brush_face::BrushFace {
    let mut face = top_face(&cube());
    let (u_axis, v_axis) = uv::parallel_axes(face.normal());
    face.uv_coord_system = UvCoordSystem::Parallel { u_axis, v_axis };
    face.uv_attributes.offset = Vec2f::new(-8.0, 12.0);
    face.uv_attributes.scale = Vec2f::new(1.5, 0.75);
    face
}

/// The UV coordinates of a world point on the given face.
fn uv_of(face: &map_model::brush_face::BrushFace, point: Vec3d) -> Vec2f {
    face.uv_coords(point)
}

#[test]
fn turning_about_a_point_leaves_that_point_where_it_was() {
    for mut face in [paraxial_face(), parallel_face()] {
        // Somewhere off the mapping's own zero, so the correction has something to do.
        let origin = Vec2f::new(37.0, -19.0);
        let anchor = map_model::uv::point_at_uv(
            &face.uv_coord_system,
            origin,
            &face.plane,
            &face.uv_attributes,
        )
        .expect("a point under the handle");

        face.rotate_uv_about(origin, 37.0);

        // The handle still sits on the same point of the face.
        geometry_math::assert_approx_eq!(uv_of(&face, anchor), origin, 1e-2);
    }
}

#[test]
fn scaling_about_a_point_leaves_that_point_where_it_was() {
    for mut face in [paraxial_face(), parallel_face()] {
        let origin = Vec2f::new(-11.0, 23.0);
        let anchor = map_model::uv::point_at_uv(
            &face.uv_coord_system,
            origin,
            &face.plane,
            &face.uv_attributes,
        )
        .expect("a point under the handle");

        face.scale_uv_about(origin, Vec2f::new(2.0, 0.5));

        geometry_math::assert_approx_eq!(uv_of(&face, anchor), origin, 1e-2);
    }
}

#[test]
fn a_handle_on_a_point_the_turn_does_not_move_needs_no_correction() {
    // The top of the cube is mapped by the X and Y axes, so UV zero sits on the face's own
    // axis of rotation when there is no offset -- turning leaves it exactly there, and the
    // correction has nothing to do.
    let mut about = top_face(&cube());
    let mut plain = top_face(&cube());

    about.rotate_uv_about(Vec2f::zero(), 45.0);
    plain.rotate_uv(45.0);

    geometry_math::assert_approx_eq!(
        about.uv_attributes.rotation,
        plain.uv_attributes.rotation,
        1e-6
    );
    geometry_math::assert_approx_eq!(about.uv_attributes.offset, plain.uv_attributes.offset, 1e-3);
}

#[test]
fn a_handle_away_from_that_point_does_need_one() {
    // Anywhere else, turning drags the point out from under the handle, and the offsets
    // have to come back by however far it went.
    let mut about = top_face(&cube());
    let mut plain = top_face(&cube());

    about.rotate_uv_about(Vec2f::new(48.0, 0.0), 45.0);
    plain.rotate_uv(45.0);

    assert_ne!(about.uv_attributes.offset, plain.uv_attributes.offset);
}

#[test]
fn turning_about_a_point_still_turns_the_material() {
    let mut face = paraxial_face();
    let before = face.uv_attributes.rotation;
    face.rotate_uv_about(Vec2f::new(8.0, 8.0), 90.0);
    assert_ne!(face.uv_attributes.rotation, before);
}
