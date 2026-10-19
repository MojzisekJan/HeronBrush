//! Tests for the shape generators.
//!
//! Checks bounds, face geometry and construction errors.

use map_model::brush::Brush;
use map_model::shapes::{self, CircleShape, ShapeError};
use geometry_math::bbox::Bbox3d;
use geometry_math::util::Axis;
use geometry_math::vec::Vec3d;

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

fn bounds() -> Bbox3d {
    Bbox3d::new(Vec3d::new(-32.0, -32.0, 0.0), Vec3d::new(32.0, 32.0, 64.0))
}

fn cube_32() -> Bbox3d {
    Bbox3d::from_scalars(-32.0, 32.0)
}

fn edge_aligned(num_sides: usize) -> CircleShape {
    CircleShape::EdgeAligned { num_sides }
}

/// `getMergedBounds`: the box every one of the brushes fits in.
fn merged_bounds(brushes: &[Brush]) -> Bbox3d {
    let mut merged = brushes.first().expect("at least one brush").bounds();
    for brush in &brushes[1..] {
        merged = merged.merged(&brush.bounds());
    }
    merged
}

fn assert_bounds_eq(actual: &Bbox3d, expected: &Bbox3d) {
    geometry_math::assert_approx_eq!(actual.min, expected.min, 1e-6);
    geometry_math::assert_approx_eq!(actual.max, expected.max, 1e-6);
}

/* ========== cuboid ========== */

#[test]
fn a_cuboid_fills_its_bounds() {
    let brush = shapes::create_cuboid(&world_bounds(), &bounds(), "stone").expect("a box");

    assert_eq!(brush.faces().len(), 6);
    assert_eq!(brush.geometry().vertex_count(), 8);
    brush.geometry().check_invariant().expect("a valid mesh");

    assert_bounds_eq(&brush.bounds(), &bounds());
}

/* ========== cylinder ========== */

#[test]
fn a_cylinder_has_two_caps_and_one_face_per_side() {
    for sides in [3, 6, 8, 16, 32] {
        let brush = shapes::create_cylinder(
            &world_bounds(),
            &bounds(),
            edge_aligned(sides),
            Axis::Z,
            "stone",
        )
        .unwrap_or_else(|e| panic!("{sides} sides: {e}"));

        brush.geometry().check_invariant().expect("a valid mesh");
        assert_eq!(
            brush.geometry().face_count(),
            sides + 2,
            "a {sides}-sided cylinder"
        );
        // Two rings of vertices, one per side at each end.
        assert_eq!(brush.geometry().vertex_count(), sides * 2);
    }
}

#[test]
fn a_four_sided_edge_aligned_cylinder_is_the_box_itself() {
    // The edges touch the bounds at their midpoints, so with four sides the shape is the
    // box, not a diamond inscribed in it.
    let brush = shapes::create_cylinder(
        &world_bounds(),
        &cube_32(),
        edge_aligned(4),
        Axis::Z,
        "someName",
    )
    .expect("a cylinder");

    assert_bounds_eq(&brush.bounds(), &cube_32());
    assert_eq!(brush.geometry().face_count(), 6);

    let mut corners: Vec<Vec3d> = brush.vertex_positions();
    corners.sort_by(|a, b| {
        (a.x(), a.y(), a.z())
            .partial_cmp(&(b.x(), b.y(), b.z()))
            .expect("finite coordinates")
    });
    for corner in &corners {
        for axis in 0..3 {
            geometry_math::assert_approx_eq!(corner[axis].abs(), 32.0, 1e-6);
        }
    }
    assert_eq!(corners.len(), 8);
}

#[test]
fn a_scalable_cylinder_has_twelve_sides_at_the_coarsest() {
    for box_bounds in [
        cube_32(),
        Bbox3d::new(
            Vec3d::new(-64.0, -32.0, -32.0),
            Vec3d::new(64.0, 32.0, 32.0),
        ),
    ] {
        let brush = shapes::create_cylinder(
            &world_bounds(),
            &box_bounds,
            CircleShape::Scalable { precision: 0 },
            Axis::Z,
            "someName",
        )
        .expect("a cylinder");

        assert_bounds_eq(&brush.bounds(), &box_bounds);
        // Twelve sides plus the two caps.
        assert_eq!(brush.geometry().face_count(), 14);
    }
}

#[test]
fn a_cylinder_fits_inside_its_bounds() {
    let brush = shapes::create_cylinder(
        &world_bounds(),
        &bounds(),
        edge_aligned(16),
        Axis::Z,
        "stone",
    )
    .expect("a cylinder");

    assert_bounds_eq(&brush.bounds(), &bounds());
}

#[test]
fn a_cylinder_can_run_along_any_axis() {
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let brush = shapes::create_cylinder(
            &world_bounds(),
            &bounds(),
            edge_aligned(8),
            axis,
            "stone",
        )
        .unwrap_or_else(|e| panic!("{axis:?}: {e}"));

        brush.geometry().check_invariant().expect("a valid mesh");
        assert_eq!(brush.geometry().face_count(), 10);
    }
}

/* ========== hollow cylinder ========== */

#[test]
fn a_hollow_cylinder_is_one_brush_per_side_filling_the_bounds() {
    let brushes = shapes::create_hollow_cylinder(
        &world_bounds(),
        &cube_32(),
        8.0,
        edge_aligned(8),
        Axis::Z,
        "someName",
    )
    .expect("a hollow cylinder");

    assert_eq!(brushes.len(), 8);
    assert_bounds_eq(&merged_bounds(&brushes), &cube_32());
    for brush in &brushes {
        brush.geometry().check_invariant().expect("a valid mesh");
    }
}

#[test]
fn a_wall_too_thick_for_an_inner_circle_becomes_wedges() {
    // With the wall thicker than the radius there is no room for an inside, so the
    // segments meet in the middle instead.
    let brushes = shapes::create_hollow_cylinder(
        &world_bounds(),
        &cube_32(),
        32.0,
        edge_aligned(8),
        Axis::Z,
        "someName",
    )
    .expect("a hollow cylinder");

    assert_eq!(brushes.len(), 8);
    assert_bounds_eq(&merged_bounds(&brushes), &cube_32());
}

/* ========== arch ========== */

#[test]
fn an_arch_is_one_brush_per_voussoir_filling_the_bounds() {
    let arch_bounds = Bbox3d::new(
        Vec3d::new(-128.0, -64.0, 0.0),
        Vec3d::new(128.0, 64.0, 64.0),
    );
    let brushes = shapes::create_arch(
        &world_bounds(),
        &arch_bounds,
        16.0,
        edge_aligned(16),
        Axis::Y,
        "someName",
    )
    .expect("an arch");

    assert_eq!(brushes.len(), 9);
    assert_bounds_eq(&merged_bounds(&brushes), &arch_bounds);
}

#[test]
fn degenerate_bounds_make_no_arch_rather_than_an_error() {
    // The bounds are degenerate for most of a drag, which is not an error to report.
    for degenerate in [
        Bbox3d::new(Vec3d::new(-128.0, -64.0, 0.0), Vec3d::new(128.0, 64.0, 0.0)),
        Bbox3d::new(Vec3d::new(-128.0, 0.0, 0.0), Vec3d::new(128.0, 0.0, 64.0)),
        Bbox3d::new(Vec3d::new(0.0, -64.0, 0.0), Vec3d::new(0.0, 64.0, 64.0)),
    ] {
        assert!(
            shapes::create_arch(
                &world_bounds(),
                &degenerate,
                16.0,
                edge_aligned(12),
                Axis::X,
                "someName",
            )
            .is_ok_and(|brushes| brushes.is_empty())
        );
        assert!(
            shapes::create_spandrel_for_arch(
                &world_bounds(),
                &degenerate,
                edge_aligned(12),
                Axis::X,
                "someName",
            )
            .is_ok_and(|brushes| brushes.is_empty())
        );
    }
}

#[test]
fn a_spandrel_fills_the_two_corners_an_arch_leaves() {
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let brushes = shapes::create_spandrel_for_arch(
            &world_bounds(),
            &Bbox3d::from_scalars(-64.0, 64.0),
            edge_aligned(8),
            axis,
            "someName",
        )
        .unwrap_or_else(|e| panic!("{axis:?}: {e}"));

        assert_eq!(brushes.len(), 2, "{axis:?}");
    }
}

/* ========== cone ========== */

#[test]
fn a_cone_has_a_base_and_one_face_per_side() {
    let brush = shapes::create_cone(
        &world_bounds(),
        &cube_32(),
        edge_aligned(4),
        Axis::Z,
        "someName",
    )
    .expect("a cone");

    assert_bounds_eq(&brush.bounds(), &cube_32());
    // One face per side of the base circle, plus the base itself.
    assert_eq!(brush.geometry().face_count(), 5);
}

#[test]
fn a_cone_can_point_along_any_axis() {
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let brush = shapes::create_cone(
            &world_bounds(),
            &cube_32(),
            CircleShape::VertexAligned { num_sides: 6 },
            axis,
            "someName",
        )
        .unwrap_or_else(|e| panic!("{axis:?}: {e}"));

        assert_eq!(brush.geometry().face_count(), 7, "{axis:?}");
    }
}

/* ========== spheres ========== */

#[test]
fn a_uv_sphere_fills_its_bounds() {
    for circle_shape in [
        edge_aligned(8),
        CircleShape::VertexAligned { num_sides: 8 },
        CircleShape::Scalable { precision: 0 },
    ] {
        let brush = shapes::create_uv_sphere(
            &world_bounds(),
            &cube_32(),
            circle_shape,
            2,
            Axis::Z,
            "someName",
        )
        .unwrap_or_else(|e| panic!("{circle_shape:?}: {e}"));

        brush.geometry().check_invariant().expect("a valid mesh");
        assert_bounds_eq(&brush.bounds(), &cube_32());
    }
}

#[test]
fn an_ico_sphere_splits_each_triangle_into_four() {
    let brush =
        shapes::create_ico_sphere(&world_bounds(), &cube_32(), 1, "someName").expect("a sphere");

    brush.geometry().check_invariant().expect("a valid mesh");
    // Every one of the icosahedron's 20 triangles is split into four.
    assert_eq!(brush.geometry().face_count(), 80);
    assert_bounds_eq(&brush.bounds(), &cube_32());
}

#[test]
fn subdividing_an_ico_sphere_multiplies_its_faces() {
    let counts: Vec<usize> = (0..3)
        .map(|n| {
            shapes::create_ico_sphere(&world_bounds(), &cube_32(), n, "someName")
                .expect("a sphere")
                .geometry()
                .face_count()
        })
        .collect();

    assert_eq!(counts, vec![20, 80, 320]);
}

/* ========== stairs ========== */

#[test]
fn stairs_are_one_box_per_step_filling_the_bounds() {
    let stair_bounds = Bbox3d::new(Vec3d::zero(), Vec3d::new(128.0, 64.0, 64.0));
    let brushes = shapes::create_stairs(
        &world_bounds(),
        &stair_bounds,
        16.0,
        Axis::X,
        true,
        "someName",
    )
    .expect("stairs");

    assert_eq!(brushes.len(), 4);
    assert_bounds_eq(&merged_bounds(&brushes), &stair_bounds);

    // Each step is one tread deep and rises by one step height above the last.
    for (index, brush) in brushes.iter().enumerate() {
        let step = brush.bounds();
        geometry_math::assert_approx_eq!(step.min.x(), 32.0 * index as f64, 1e-6);
        geometry_math::assert_approx_eq!(step.max.x(), 32.0 * (index + 1) as f64, 1e-6);
        geometry_math::assert_approx_eq!(step.max.z(), 16.0 * (index + 1) as f64, 1e-6);
    }
}

#[test]
fn stairs_can_descend_instead() {
    let stair_bounds = Bbox3d::new(Vec3d::zero(), Vec3d::new(128.0, 64.0, 64.0));
    let brushes = shapes::create_stairs(
        &world_bounds(),
        &stair_bounds,
        16.0,
        Axis::X,
        false,
        "someName",
    )
    .expect("stairs");

    // The lowest step is now at the far end.
    geometry_math::assert_approx_eq!(brushes[0].bounds().max.x(), 128.0, 1e-6);
    geometry_math::assert_approx_eq!(brushes[3].bounds().min.x(), 0.0, 1e-6);
}

/* ========== errors ========== */

#[test]
fn a_shape_with_too_few_sides_is_refused() {
    for sides in [0, 1, 2] {
        assert_eq!(
            shapes::create_cylinder(
                &world_bounds(),
                &bounds(),
                edge_aligned(sides),
                Axis::Z,
                "stone"
            )
            .err(),
            Some(ShapeError::TooFewSides { given: sides })
        );
        assert_eq!(
            shapes::create_cone(
                &world_bounds(),
                &bounds(),
                edge_aligned(sides),
                Axis::Z,
                "stone"
            )
            .err(),
            Some(ShapeError::TooFewSides { given: sides })
        );
    }
}

#[test]
fn every_shape_has_the_material_it_was_given() {
    let brushes = vec![
        shapes::create_cuboid(&world_bounds(), &bounds(), "marble").unwrap(),
        shapes::create_cylinder(&world_bounds(), &bounds(), edge_aligned(8), Axis::Z, "marble")
            .unwrap(),
        shapes::create_cone(&world_bounds(), &bounds(), edge_aligned(8), Axis::Z, "marble")
            .unwrap(),
        shapes::create_ico_sphere(&world_bounds(), &bounds(), 0, "marble").unwrap(),
        shapes::create_uv_sphere(
            &world_bounds(),
            &bounds(),
            edge_aligned(8),
            2,
            Axis::Z,
            "marble",
        )
        .unwrap(),
    ];

    for brush in brushes {
        for face in brush.faces() {
            assert_eq!(face.material_name, "marble");
        }
    }
}

#[test]
fn every_shape_has_outward_normals() {
    let bounds = bounds();
    let brushes = vec![
        shapes::create_cuboid(&world_bounds(), &bounds, "stone").unwrap(),
        shapes::create_cylinder(&world_bounds(), &bounds, edge_aligned(12), Axis::Z, "stone")
            .unwrap(),
        shapes::create_cone(&world_bounds(), &bounds, edge_aligned(12), Axis::Z, "stone")
            .unwrap(),
        shapes::create_ico_sphere(&world_bounds(), &bounds, 1, "stone").unwrap(),
    ];

    let center = bounds.center();
    for brush in brushes {
        for face in brush.faces() {
            // Every face's plane has the brush's own centre behind it.
            assert!(
                face.plane.point_distance(center) < 0.0,
                "a face points inwards"
            );
        }
    }
}

#[test]
fn hollow_sphere_is_closed_empty_and_has_exactly_paired_contacts() {
    for count in [20,24,28,44,80,84,180,320] {
        let bounds=Bbox3d::new(Vec3d::new(100.0,200.0,300.0),Vec3d::new(228.0,328.0,428.0));
        let center=(bounds.min+bounds.max)*0.5;
        let shell=shapes::create_hollow_sphere(&world_bounds(),&bounds,8.0,count,"stone").unwrap();
        assert_eq!(shell.len(),count);
        let points:Vec<_> = shell.iter().flat_map(Brush::vertex_positions).collect();
        let hull=map_model::polyhedron::Polyhedron::from_points(&points,1e-9).unwrap();
        let solid=Brush::from_polyhedron(hull,"stone");
        let expected=solid.volume()*(1.0-(1.0-8.0/64.0f64).powi(3));
        assert!((shell.iter().map(Brush::volume).sum::<f64>()-expected).abs()<expected*1e-7);
        let mut contacts=std::collections::HashMap::new();
        for brush in &shell {
            assert!(brush.volume()>0.0);
            assert!(!brush.contains_point(center));
            assert_eq!(brush.faces().len(),5);
            assert_eq!(brush.faces().iter().filter(|f|f.material_name.as_str()=="stone").count(),2);
            for (face,polygon) in brush.geometry_faces() {
                if face.material_name.as_str()!="skip" {continue;}
                let mut key:Vec<_>=polygon.iter().map(|p|[
                    (p[0]*1e6).round() as i64,(p[1]*1e6).round() as i64,(p[2]*1e6).round() as i64]).collect();
                key.sort();
                *contacts.entry(key).or_insert(0usize)+=1;
            }
        }
        assert_eq!(contacts.len(),shell.len()*3/2);
        assert!(contacts.values().all(|count|*count==2),"unpaired contact leaves a hole");
        if count<=84 {
            for (i,brush) in shell.iter().enumerate() {
                for other in shell.iter().skip(i+1) {
                    assert!(map_model::csg::intersect(brush,other).is_none_or(|b|b.volume()<1e-5));
                }
            }
        }
    }
}

#[test]
fn hollow_sphere_validates_inputs_and_can_be_stretched() {
    for thickness in [0.0,-1.0,32.0,64.0,f64::NAN,f64::INFINITY] {
        assert!(shapes::create_hollow_sphere(&world_bounds(),&cube_32(),thickness,80,"stone").is_err());
    }
    for count in [0,19,21,22,23,321,324,usize::MAX] {
        assert!(shapes::create_hollow_sphere(&world_bounds(),&cube_32(),8.0,count,"stone").is_err());
    }
    assert!(shapes::create_hollow_sphere(&world_bounds(),&Bbox3d::from_scalars(0.0,0.0),1.0,20,"stone").is_err());
    let bounds=Bbox3d::new(Vec3d::zero(),Vec3d::new(64.0,128.0,256.0));
    let brushes=shapes::create_hollow_sphere(&world_bounds(),&bounds,8.0,84,"stone").unwrap();
    assert_eq!(brushes.len(),84);
    assert!(brushes.iter().all(|b|!b.contains_point(Vec3d::new(32.0,64.0,128.0))));
    assert!(shapes::create_hollow_sphere(&Bbox3d::from_scalars(-1.0,1.0),&bounds,8.0,84,"stone").is_err());
}
