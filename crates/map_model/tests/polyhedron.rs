//! Tests for the half-edge polyhedron.
//!
//! Checks that construction and clipping produce closed, consistently wound convex meshes with
//! valid arena links.

use geometry_math::bbox::Bbox3d;
use geometry_math::plane::{self, Plane3d};
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec3d};
use map_model::polyhedron::{ClipResult, FromPlanes, Polyhedron};

/// The epsilon used throughout, matching what the editor uses for vertex snapping.
const EPS: f64 = f64::ALMOST_ZERO;

fn unit_cube() -> Polyhedron<f64> {
    Polyhedron::from_bbox(&Bbox3d::from_scalars(-1.0, 1.0)).expect("a box is a polyhedron")
}

#[test]
fn a_box_has_the_expected_counts() {
    let cube = unit_cube();
    assert_eq!(cube.vertex_count(), 8);
    assert_eq!(cube.edge_count(), 12);
    assert_eq!(cube.face_count(), 6);
    assert!(cube.is_closed());
}

#[test]
fn a_box_satisfies_the_mesh_invariants() {
    unit_cube()
        .check_invariant()
        .expect("a box is a valid mesh");
}

#[test]
fn every_face_of_a_box_is_a_quad() {
    let cube = unit_cube();
    for (index, _) in cube.faces().iter().enumerate() {
        let face = map_model::polyhedron::FaceId(index as u32);
        assert_eq!(cube.face_vertices(face).len(), 4, "face {index}");
    }
}

#[test]
fn face_normals_point_outwards() {
    let cube = unit_cube();
    let center = cube.bounds().center();

    for (index, face) in cube.faces().iter().enumerate() {
        // The centre of a convex solid is behind every one of its faces.
        assert!(
            face.plane.point_distance(center) < 0.0,
            "face {index} has its normal pointing inwards"
        );
    }
}

#[test]
fn the_winding_of_a_face_agrees_with_its_plane() {
    let cube = unit_cube();

    for index in 0..cube.face_count() {
        let face = map_model::polyhedron::FaceId(index as u32);
        let positions = cube.face_positions(face);
        // Faces are wound clockwise seen from outside, so the outward normal is the
        // reversed cross product — the same one `geometry_math::plane::from_points` takes.
        let normal = vec::normalize(vec::cross(
            positions[2] - positions[0],
            positions[1] - positions[0],
        ));
        let expected = cube.face_plane(face).normal;
        assert!(
            vec::dot(normal, expected) > 0.99,
            "face {index}: winding and plane normal disagree"
        );
    }
}

#[test]
fn three_edges_meet_at_each_vertex_of_a_box() {
    let cube = unit_cube();
    for index in 0..cube.vertex_count() {
        let vertex = map_model::polyhedron::VertexId(index as u32);
        assert_eq!(cube.vertex_half_edges(vertex).len(), 3, "vertex {index}");
        assert_eq!(cube.vertex_faces(vertex).len(), 3, "vertex {index}");
    }
}

#[test]
fn contains_point() {
    let cube = unit_cube();
    assert!(cube.contains_point(Vec3d::zero(), EPS));
    assert!(cube.contains_point(Vec3d::new(0.9, 0.9, 0.9), EPS));
    // The boundary counts as inside.
    assert!(cube.contains_point(Vec3d::new(1.0, 0.0, 0.0), EPS));
    assert!(!cube.contains_point(Vec3d::new(1.1, 0.0, 0.0), EPS));
    assert!(!cube.contains_point(Vec3d::new(2.0, 2.0, 2.0), EPS));
}

#[test]
fn surface_area_of_a_unit_cube() {
    // Six faces, each 2 by 2.
    assert!((unit_cube().surface_area() - 24.0).abs() < 1e-9);
}

#[test]
fn building_from_too_few_faces_is_rejected() {
    let faces = vec![
        vec![
            Vec3d::zero(),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ],
        vec![
            Vec3d::zero(),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(0.0, 0.0, 1.0),
        ],
    ];
    assert!(Polyhedron::from_faces(&faces, EPS).is_err());
}

#[test]
fn building_from_an_open_mesh_is_rejected() {
    // Five faces of a cube: one is missing, so the mesh has a hole.
    let cube = Bbox3d::from_scalars(-1.0, 1.0);
    let faces: Vec<Vec<Vec3d>> = cube
        .faces()
        .iter()
        .take(5)
        .map(|f| f.vertices.to_vec())
        .collect();

    let error = Polyhedron::from_faces(&faces, EPS).expect_err("the mesh is not closed");
    assert!(
        matches!(error, map_model::polyhedron::BuildError::OpenEdge { .. }),
        "expected an open edge, got {error:?}"
    );
}

/* ========== clipping ========== */

#[test]
fn clipping_with_a_plane_that_misses_leaves_it_unchanged() {
    let cube = unit_cube();
    // Well above the cube, facing up.
    let above =
        Plane3d::from_anchor_and_normal(Vec3d::new(0.0, 0.0, 10.0), Vec3d::new(0.0, 0.0, 1.0));
    assert!(cube.clip(&above, EPS).unwrap().is_unchanged());
}

#[test]
fn clipping_with_a_plane_that_covers_it_empties_it() {
    let cube = unit_cube();
    // Well below the cube, facing up: the whole cube is in front of it.
    let below =
        Plane3d::from_anchor_and_normal(Vec3d::new(0.0, 0.0, -10.0), Vec3d::new(0.0, 0.0, 1.0));
    assert!(cube.clip(&below, EPS).unwrap().is_empty());
}

#[test]
fn clipping_a_cube_in_half_gives_a_valid_box() {
    let cube = unit_cube();
    let halving = Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0));

    let result = cube.clip(&halving, EPS).expect("the plane cuts the cube");
    let clipped = result.polyhedron().expect("the cube was cut");

    clipped
        .check_invariant()
        .expect("the result is a valid mesh");
    // Still a box: 8 vertices, 12 edges, 6 faces.
    assert_eq!(clipped.vertex_count(), 8);
    assert_eq!(clipped.edge_count(), 12);
    assert_eq!(clipped.face_count(), 6);

    // Half the height, and nothing left above the plane.
    let bounds = clipped.bounds();
    assert!((bounds.min.z() - -1.0).abs() < 1e-9);
    assert!((bounds.max.z() - 0.0).abs() < 1e-9);
    for v in clipped.vertices() {
        assert!(v.position.z() <= EPS, "a vertex survived above the plane");
    }
}

#[test]
fn slicing_a_corner_off_a_cube_adds_a_triangle() {
    let cube = unit_cube();
    // A plane cutting the +X+Y+Z corner off.
    let corner = Plane3d::from_anchor_and_normal(
        Vec3d::new(0.8, 0.8, 0.8),
        vec::normalize(Vec3d::new(1.0, 1.0, 1.0)),
    );

    let result = cube.clip(&corner, EPS).expect("the plane cuts the cube");
    let clipped = result.polyhedron().expect("the cube was cut");
    clipped
        .check_invariant()
        .expect("the result is a valid mesh");

    // The corner vertex is replaced by three, and a triangular face appears.
    assert_eq!(clipped.vertex_count(), 10);
    assert_eq!(clipped.face_count(), 7);

    let triangles = (0..clipped.face_count())
        .filter(|i| {
            clipped
                .face_vertices(map_model::polyhedron::FaceId(*i as u32))
                .len()
                == 3
        })
        .count();
    assert_eq!(triangles, 1, "expected exactly one new triangular face");
}

#[test]
fn the_new_face_lies_in_the_clipping_plane() {
    let cube = unit_cube();
    let halving =
        Plane3d::from_anchor_and_normal(Vec3d::new(0.0, 0.0, 0.25), Vec3d::new(0.0, 0.0, 1.0));

    let result = cube.clip(&halving, EPS).expect("the plane cuts the cube");
    let ClipResult::Clipped {
        polyhedron,
        new_face,
    } = result
    else {
        panic!("expected a clipped result");
    };

    for position in polyhedron.face_positions(new_face) {
        assert!(
            halving.point_distance(position).abs() < 1e-9,
            "a vertex of the new face is off the clipping plane"
        );
    }
    // Its normal matches the clipping plane's, not the other way round.
    let normal = polyhedron.face_plane(new_face).normal;
    assert!(vec::dot(normal, halving.normal) > 0.99);
}

/* ========== building from planes, the way a brush is defined ========== */

/// The six planes of an axis aligned box, facing outwards.
fn box_planes(min: Vec3d, max: Vec3d) -> Vec<Plane3d> {
    vec![
        Plane3d::from_anchor_and_normal(max, Vec3d::new(1.0, 0.0, 0.0)),
        Plane3d::from_anchor_and_normal(min, Vec3d::new(-1.0, 0.0, 0.0)),
        Plane3d::from_anchor_and_normal(max, Vec3d::new(0.0, 1.0, 0.0)),
        Plane3d::from_anchor_and_normal(min, Vec3d::new(0.0, -1.0, 0.0)),
        Plane3d::from_anchor_and_normal(max, Vec3d::new(0.0, 0.0, 1.0)),
        Plane3d::from_anchor_and_normal(min, Vec3d::new(0.0, 0.0, -1.0)),
    ]
}

#[test]
fn a_brush_shaped_box_can_be_built_from_its_planes() {
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let planes = box_planes(
        Vec3d::new(-32.0, -32.0, -32.0),
        Vec3d::new(32.0, 32.0, 32.0),
    );

    let FromPlanes::Built {
        polyhedron,
        redundant,
    } = Polyhedron::from_planes(&world, &planes, EPS).expect("the planes enclose a box")
    else {
        panic!("expected the planes to enclose a volume");
    };

    polyhedron
        .check_invariant()
        .expect("the result is a valid mesh");
    assert!(redundant.is_empty(), "no plane should be redundant");
    assert_eq!(polyhedron.vertex_count(), 8);
    assert_eq!(polyhedron.face_count(), 6);

    let bounds = polyhedron.bounds();
    assert!((bounds.min.x() - -32.0).abs() < 1e-9);
    assert!((bounds.max.z() - 32.0).abs() < 1e-9);
}

#[test]
fn a_wedge_can_be_built_by_adding_a_seventh_plane() {
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let mut planes = box_planes(
        Vec3d::new(-32.0, -32.0, -32.0),
        Vec3d::new(32.0, 32.0, 32.0),
    );
    // Slice the +X+Z edge off. The cut is shallow enough that every original face keeps
    // some area, so the result has the six original faces plus the new one.
    planes.push(Plane3d::from_anchor_and_normal(
        Vec3d::new(24.0, 0.0, 24.0),
        vec::normalize(Vec3d::new(1.0, 0.0, 1.0)),
    ));

    let built = Polyhedron::from_planes(&world, &planes, EPS).expect("the planes enclose a wedge");
    let polyhedron = built.polyhedron().expect("the planes enclose a volume");
    polyhedron
        .check_invariant()
        .expect("the result is a valid mesh");
    assert_eq!(polyhedron.face_count(), 7);
}

#[test]
fn a_plane_that_cuts_a_face_down_to_an_edge_removes_that_face() {
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let mut planes = box_planes(
        Vec3d::new(-32.0, -32.0, -32.0),
        Vec3d::new(32.0, 32.0, 32.0),
    );
    // Straight through the middle along the diagonal: the +X and +Z faces are left with
    // no area at all, so the wedge has five faces rather than seven.
    planes.push(Plane3d::from_anchor_and_normal(
        Vec3d::zero(),
        vec::normalize(Vec3d::new(1.0, 0.0, 1.0)),
    ));

    let built = Polyhedron::from_planes(&world, &planes, EPS).expect("the planes enclose a wedge");
    let polyhedron = built.polyhedron().expect("the planes enclose a volume");
    polyhedron
        .check_invariant()
        .expect("the result is a valid mesh");
    assert_eq!(polyhedron.face_count(), 5);
}

#[test]
fn a_redundant_plane_is_reported_rather_than_silently_dropped() {
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let mut planes = box_planes(
        Vec3d::new(-32.0, -32.0, -32.0),
        Vec3d::new(32.0, 32.0, 32.0),
    );
    // Far outside the box, so it cuts nothing.
    planes.push(Plane3d::from_anchor_and_normal(
        Vec3d::new(0.0, 0.0, 1000.0),
        Vec3d::new(0.0, 0.0, 1.0),
    ));

    let FromPlanes::Built { redundant, .. } =
        Polyhedron::from_planes(&world, &planes, EPS).expect("the box is still there")
    else {
        panic!("expected the planes to enclose a volume");
    };
    assert_eq!(redundant, vec![6]);
}

#[test]
fn contradictory_planes_enclose_nothing() {
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let planes = vec![
        // Keeps z <= 0 ...
        Plane3d::from_anchor_and_normal(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.0, 0.0, 1.0)),
        // ... and this keeps z >= 10. Nothing is behind both.
        Plane3d::from_anchor_and_normal(Vec3d::new(0.0, 0.0, 10.0), Vec3d::new(0.0, 0.0, -1.0)),
    ];

    let built = Polyhedron::from_planes(&world, &planes, EPS).expect("clipping succeeds");
    assert!(matches!(built, FromPlanes::Empty { plane: 1 }));
}

#[test]
fn a_plane_defined_from_three_points_clips_the_same_way() {
    // Map files define a face by three points on its plane, so this is the path a loaded
    // brush actually takes.
    let cube = unit_cube();
    let p = plane::from_points(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
    )
    .expect("three independent points");

    let result = cube.clip(&p, EPS).expect("the plane cuts the cube");
    let clipped = result.polyhedron().expect("the cube was cut");
    clipped
        .check_invariant()
        .expect("the result is a valid mesh");
    assert_eq!(clipped.face_count(), 6);
}

/* ========== convex hull from points ========== */

#[test]
fn the_hull_of_a_cubes_corners_is_a_cube() {
    let corners = Bbox3d::from_scalars(-1.0, 1.0).vertices();
    let hull = Polyhedron::from_points(&corners, EPS).expect("eight corners make a cube");

    hull.check_invariant().expect("a valid mesh");
    // Six quadrilateral faces, not twelve triangles: coplanar faces are merged.
    assert_eq!(hull.face_count(), 6);
    assert_eq!(hull.vertex_count(), 8);
    assert_eq!(hull.edge_count(), 12);

    for index in 0..hull.face_count() {
        assert_eq!(
            hull.face_vertices(map_model::polyhedron::FaceId(index as u32))
                .len(),
            4
        );
    }
}

#[test]
fn a_point_inside_the_hull_is_dropped() {
    let mut points = Bbox3d::from_scalars(-1.0, 1.0).vertices().to_vec();
    points.push(Vec3d::zero());
    points.push(Vec3d::new(0.5, 0.5, 0.5));

    let hull = Polyhedron::from_points(&points, EPS).expect("the corners still make a cube");
    hull.check_invariant().expect("a valid mesh");
    assert_eq!(hull.vertex_count(), 8);
}

#[test]
fn a_point_on_a_face_of_the_hull_is_dropped() {
    let mut points = Bbox3d::from_scalars(-1.0, 1.0).vertices().to_vec();
    // Exactly in the middle of the +Z face.
    points.push(Vec3d::new(0.0, 0.0, 1.0));

    let hull = Polyhedron::from_points(&points, EPS).expect("the cube is unchanged");
    hull.check_invariant().expect("a valid mesh");
    assert_eq!(hull.vertex_count(), 8);
    assert_eq!(hull.face_count(), 6);
}

#[test]
fn the_hull_of_four_points_is_a_tetrahedron() {
    let hull = Polyhedron::from_points(
        &[
            Vec3d::zero(),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(0.0, 0.0, 1.0),
        ],
        EPS,
    )
    .expect("four independent points make a tetrahedron");

    hull.check_invariant().expect("a valid mesh");
    assert_eq!(hull.vertex_count(), 4);
    assert_eq!(hull.edge_count(), 6);
    assert_eq!(hull.face_count(), 4);
}

#[test]
fn a_point_outside_extends_the_hull() {
    let mut points = Bbox3d::from_scalars(-1.0, 1.0).vertices().to_vec();
    // A spike above the +Z face.
    points.push(Vec3d::new(0.0, 0.0, 3.0));

    let hull = Polyhedron::from_points(&points, EPS).expect("a cube with a spike");
    hull.check_invariant().expect("a valid mesh");
    assert_eq!(hull.vertex_count(), 9);
    // The +Z face is replaced by four triangles meeting at the spike.
    assert_eq!(hull.face_count(), 9);
    assert!(hull.contains_point(Vec3d::new(0.0, 0.0, 2.0), EPS));
}

#[test]
fn hull_face_normals_point_outwards() {
    let hull =
        Polyhedron::from_points(&Bbox3d::from_scalars(-8.0, 8.0).vertices(), EPS).expect("a cube");

    for (index, face) in hull.faces().iter().enumerate() {
        assert!(
            face.plane.point_distance(Vec3d::zero()) < 0.0,
            "face {index} has its normal pointing inwards"
        );
    }
}

#[test]
fn degenerate_point_sets_are_rejected() {
    use map_model::polyhedron::HullError;

    // Too few.
    assert!(matches!(
        Polyhedron::from_points(&[Vec3d::zero(), Vec3d::one()], EPS),
        Err(HullError::NotEnoughPoints { distinct: 2 })
    ));
    // Duplicates do not count as distinct.
    assert!(matches!(
        Polyhedron::from_points(&[Vec3d::zero(); 8], EPS),
        Err(HullError::NotEnoughPoints { distinct: 1 })
    ));
    // Four points on one plane enclose nothing.
    assert!(matches!(
        Polyhedron::from_points(
            &[
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(1.0, 1.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ],
            EPS
        ),
        Err(HullError::Coplanar)
    ));
}

#[test]
fn the_hull_of_a_clipped_solid_matches_the_clip() {
    // Take a wedge built by clipping, feed its vertices back in, and check that the hull
    // is the same solid. This is the round trip every vertex edit relies on.
    let cube = unit_cube();
    let corner = Plane3d::from_anchor_and_normal(
        Vec3d::new(0.5, 0.5, 0.5),
        vec::normalize(Vec3d::new(1.0, 1.0, 1.0)),
    );
    let clipped = cube
        .clip(&corner, EPS)
        .unwrap()
        .into_polyhedron()
        .expect("the plane cuts the cube");

    let positions: Vec<Vec3d> = clipped.vertices().iter().map(|v| v.position).collect();
    let hull = Polyhedron::from_points(&positions, EPS).expect("the vertices make a solid");

    hull.check_invariant().expect("a valid mesh");
    assert_eq!(hull.vertex_count(), clipped.vertex_count());
    assert_eq!(hull.face_count(), clipped.face_count());
    geometry_math::assert_approx_eq!(hull.bounds().min, clipped.bounds().min, 1e-9);
    geometry_math::assert_approx_eq!(hull.bounds().max, clipped.bounds().max, 1e-9);
}
