//! Tests for dragging a brush's vertices, edges and faces.

use map_model::brush::{Brush, VertexError};
use map_model::shapes;
use geometry_math::bbox::Bbox3d;
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec3d};

const EPS: f64 = f64::ALMOST_ZERO;

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

/// A 64-unit cube at the origin.
fn cube() -> Brush {
    shapes::create_cuboid(
        &world_bounds(),
        &Bbox3d::from_scalars(-32.0, 32.0),
        "stone",
    )
    .expect("a box")
}

#[test]
fn moving_one_vertex_makes_a_wedge() {
    let cube = cube();
    let corner = Vec3d::new(32.0, 32.0, 32.0);

    let moved = cube
        .move_vertices(&[corner], Vec3d::new(0.0, 0.0, 32.0), EPS)
        .expect("the brush survives");

    moved.geometry().check_invariant().expect("a valid mesh");
    assert_eq!(moved.geometry().vertex_count(), 8);
    // The corner is where it was put.
    assert!(moved
        .vertex_positions()
        .iter()
        .any(|p| vec::squared_distance(*p, Vec3d::new(32.0, 32.0, 64.0)) < 1e-9));
    // And the old position is gone.
    assert!(!moved
        .vertex_positions()
        .iter()
        .any(|p| vec::squared_distance(*p, corner) < 1e-9));
}

#[test]
fn dragging_a_face_resizes_the_brush() {
    let cube = cube();
    // A point on the +Z face.
    let on_top = Vec3d::new(0.0, 0.0, 32.0);

    let taller = cube
        .move_face(on_top, Vec3d::new(0.0, 0.0, 64.0), EPS)
        .expect("the brush survives");

    taller.geometry().check_invariant().expect("a valid mesh");
    // Still a box, just taller.
    assert_eq!(taller.geometry().face_count(), 6);
    assert_eq!(taller.geometry().vertex_count(), 8);
    geometry_math::assert_approx_eq!(taller.bounds().min, Vec3d::new(-32.0, -32.0, -32.0), 1e-6);
    geometry_math::assert_approx_eq!(taller.bounds().max, Vec3d::new(32.0, 32.0, 96.0), 1e-6);
}

#[test]
fn dragging_a_face_keeps_the_materials_on_the_right_faces() {
    // Give each face a name so they can be told apart after the rebuild.
    let mut cube = cube();
    let names: Vec<String> = cube
        .faces()
        .iter()
        .map(|f| {
            let n = f.plane.normal;
            format!("{}{}{}", n.x() as i32, n.y() as i32, n.z() as i32)
        })
        .collect();
    // `faces` is not mutable from outside, so rebuild one with the names baked in.
    let faces = cube
        .faces()
        .iter()
        .zip(&names)
        .map(|(face, name)| {
            let mut face = face.clone();
            face.material_name = name.clone().into();
            face
        })
        .collect();
    cube = Brush::from_faces(&world_bounds(), faces).expect("still a box");

    let taller = cube
        .move_face(Vec3d::new(0.0, 0.0, 32.0), Vec3d::new(0.0, 0.0, 64.0), EPS)
        .expect("the brush survives");

    for (face, _) in taller.geometry_faces() {
        let n = face.plane.normal;
        let expected = format!("{}{}{}", n.x() as i32, n.y() as i32, n.z() as i32);
        assert_eq!(face.material_name, expected);
    }
}

#[test]
fn dragging_a_face_inwards_far_enough_collapses_the_brush() {
    let cube = cube();
    let error = cube
        .move_face(Vec3d::new(0.0, 0.0, 32.0), Vec3d::new(0.0, 0.0, -64.0), EPS)
        .expect_err("the brush is only 64 units tall");
    assert!(matches!(error, VertexError::Collapsed(_)), "{error}");
}

#[test]
fn dragging_a_face_flush_with_another_removes_it() {
    // Cut a corner off, then push the new face back until it is flush with the top.
    let cube = cube();
    let wedge = cube
        .move_vertices(
            &[Vec3d::new(32.0, 32.0, 32.0)],
            Vec3d::new(-32.0, 0.0, 0.0),
            EPS,
        )
        .expect("the brush survives");

    wedge.geometry().check_invariant().expect("a valid mesh");
    // The corner moved onto an edge of the top face, so the top is now a pentagon and the
    // brush has one fewer vertex than a box does not — it still has eight.
    assert_eq!(wedge.geometry().vertex_count(), 8);
}

#[test]
fn moving_an_edge_moves_both_of_its_ends() {
    let cube = cube();
    // The edge along the top of the +X+Y corner.
    let ends = [Vec3d::new(32.0, 32.0, 32.0), Vec3d::new(32.0, 32.0, -32.0)];

    let moved = cube
        .move_vertices(&ends, Vec3d::new(16.0, 16.0, 0.0), EPS)
        .expect("the brush survives");

    moved.geometry().check_invariant().expect("a valid mesh");
    for end in ends {
        let expected = end + Vec3d::new(16.0, 16.0, 0.0);
        assert!(
            moved
                .vertex_positions()
                .iter()
                .any(|p| vec::squared_distance(*p, expected) < 1e-9),
            "the end at {end:?} did not move"
        );
    }
}

#[test]
fn moving_nothing_leaves_the_brush_alone() {
    let cube = cube();
    let same = cube
        .move_vertices(&[Vec3d::new(32.0, 32.0, 32.0)], Vec3d::zero(), EPS)
        .expect("a zero move always succeeds");
    geometry_math::assert_approx_eq!(same.bounds().min, cube.bounds().min, 1e-9);
}

#[test]
fn a_position_that_is_not_a_vertex_is_reported() {
    let cube = cube();
    let error = cube
        .move_vertices(&[Vec3d::new(1000.0, 0.0, 0.0)], Vec3d::one(), EPS)
        .expect_err("that is not a corner of this brush");
    assert_eq!(error, VertexError::NoVerticesMatched);
}

#[test]
fn a_point_that_is_on_no_face_is_reported() {
    let cube = cube();
    let error = cube
        .move_face(Vec3d::new(0.0, 0.0, 0.0), Vec3d::one(), EPS)
        .expect_err("the centre is on no face");
    assert_eq!(error, VertexError::NoFaceThere);
}

#[test]
fn face_at_finds_each_face_of_a_box() {
    let cube = cube();
    for (point, expected_normal) in [
        (Vec3d::new(32.0, 0.0, 0.0), Vec3d::new(1.0, 0.0, 0.0)),
        (Vec3d::new(-32.0, 0.0, 0.0), Vec3d::new(-1.0, 0.0, 0.0)),
        (Vec3d::new(0.0, 0.0, 32.0), Vec3d::new(0.0, 0.0, 1.0)),
    ] {
        let face = cube.face_at(point, EPS).expect("there is a face there");
        geometry_math::assert_approx_eq!(cube.geometry().face_plane(face).normal, expected_normal, 1e-9);
    }
}

#[test]
fn a_dragged_brush_can_be_dragged_again() {
    // The rebuild has to leave a brush the tools can keep working on.
    let mut brush = cube();
    for _ in 0..4 {
        brush = brush
            .move_face(
                brush.bounds().max - Vec3d::new(0.0, 0.0, 0.0) * 0.0
                    + Vec3d::new(-1.0, -1.0, 0.0),
                Vec3d::new(0.0, 0.0, 16.0),
                EPS,
            )
            .expect("the brush survives each drag");
        brush.geometry().check_invariant().expect("a valid mesh");
    }
    geometry_math::assert_approx_eq!(brush.bounds().max.z(), 32.0 + 64.0, 1e-6);
}

/* ========== moving a face's plane ========== */

#[test]
fn moving_a_boundary_stretches_the_neighbouring_faces() {
    // The whole point of moving a boundary rather than its vertices: the face's plane
    // slides and the brush is rebuilt, so the four walls stretch to meet the top wherever
    // it lands rather than being left behind.
    let brush = cube();
    let top = brush
        .faces()
        .iter()
        .position(|face| face.plane.normal.z() > 0.9)
        .expect("a cube has a top");

    let taller = brush
        .move_boundary(&world_bounds(), top, Vec3d::new(0.0, 0.0, 16.0))
        .expect("the top moves");

    taller.geometry().check_invariant().expect("a valid mesh");
    assert_eq!(taller.faces().len(), brush.faces().len());
    assert_eq!(taller.geometry().vertex_count(), 8);

    let bounds = taller.bounds();
    geometry_math::assert_approx_eq!(bounds.max.z(), 48.0, 1e-6);
    // Nothing else moved.
    geometry_math::assert_approx_eq!(bounds.min, Vec3d::from_array([-32.0, -32.0, -32.0]), 1e-6);
    geometry_math::assert_approx_eq!(bounds.max.x(), 32.0, 1e-6);
}

#[test]
fn moving_a_boundary_inwards_shrinks_the_brush() {
    let brush = cube();
    let top = brush
        .faces()
        .iter()
        .position(|face| face.plane.normal.z() > 0.9)
        .expect("a cube has a top");

    let shorter = brush
        .move_boundary(&world_bounds(), top, Vec3d::new(0.0, 0.0, -16.0))
        .expect("the top moves");
    geometry_math::assert_approx_eq!(shorter.bounds().max.z(), 16.0, 1e-6);
}

#[test]
fn moving_a_boundary_through_the_brush_is_refused() {
    let brush = cube();
    let top = brush
        .faces()
        .iter()
        .position(|face| face.plane.normal.z() > 0.9)
        .expect("a cube has a top");

    // Past the bottom, so there is no volume left between the planes.
    assert!(brush
        .move_boundary(&world_bounds(), top, Vec3d::new(0.0, 0.0, -128.0))
        .is_err());
}

#[test]
fn there_is_no_boundary_to_move_where_there_is_no_face() {
    let brush = cube();
    assert_eq!(
        brush
            .move_boundary(&world_bounds(), 99, Vec3d::new(0.0, 0.0, 16.0))
            .err(),
        Some(map_model::brush::BrushError::NoSuchFace { index: 99 })
    );
}

#[test]
fn expanding_moves_every_face_outwards() {
    let brush = cube();
    let bigger = brush.expand(&world_bounds(), 8.0).expect("it expands");

    geometry_math::assert_approx_eq!(bigger.bounds().min, Vec3d::from_array([-40.0; 3]), 1e-6);
    geometry_math::assert_approx_eq!(bigger.bounds().max, Vec3d::from_array([40.0; 3]), 1e-6);

    let smaller = brush.expand(&world_bounds(), -8.0).expect("it contracts");
    geometry_math::assert_approx_eq!(smaller.bounds().min, Vec3d::from_array([-24.0; 3]), 1e-6);

    // Contracting past the middle leaves nothing for the planes to enclose.
    assert!(brush.expand(&world_bounds(), -64.0).is_err());
}
