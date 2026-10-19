//! Shared-boundary and history-independent geometry regressions.
use geometry_math::{
    bbox::Bbox3d,
    util::Axis,
    vec::{self, Vec3d},
};
use map_model::{
    brush::Brush,
    hollow::{self, Direction},
    shapes::{self, StairLayout, StairOptions},
};
fn world() -> Bbox3d {
    Bbox3d::from_scalars(-4096.0, 4096.0)
}
fn cube() -> Brush {
    shapes::create_cuboid(&world(), &Bbox3d::from_scalars(-32.0, 32.0), "brick").unwrap()
}
fn disjoint(brushes: &[Brush]) {
    for (i, a) in brushes.iter().enumerate() {
        for b in brushes.iter().skip(i + 1) {
            assert!(
                map_model::csg::intersect(a, b).is_none_or(|b| b.volume() < 1e-5),
                "solid intersection"
            );
        }
    }
}
#[test]
fn directional_hollow_keeps_the_requested_boundary_and_can_open_a_lid() {
    let brush = cube();
    for (mode, outer, inner) in [
        (Direction::Inward, 64.0f64, 48.0f64),
        (Direction::Outward, 80.0, 64.0),
        (Direction::Centered, 72.0, 56.0),
    ] {
        let walls = hollow::shell(&brush, &world(), 8.0, mode, &[]).unwrap();
        disjoint(&walls);
        let volume = walls.iter().map(Brush::volume).sum::<f64>();
        assert!((volume - (outer.powi(3) - inner.powi(3))).abs() < 1e-5);
        assert!(!walls.iter().any(|b| b.contains_point(Vec3d::zero())));
        let max = walls
            .iter()
            .map(|b| b.bounds().max[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((max - outer * 0.5).abs() < 1e-6);
    }
    assert!(hollow::shell(&brush,&Bbox3d::from_scalars(-32.0,32.0),8.0,Direction::Outward,&[]).is_err());
    let lid = brush
        .faces()
        .iter()
        .position(|f| f.normal()[2] > 0.99)
        .unwrap();
    let walls = hollow::shell(&brush, &world(), 8.0, Direction::Inward, &[lid]).unwrap();
    disjoint(&walls);
    assert!(!walls
        .iter()
        .any(|b| b.contains_point(Vec3d::new(0.0, 0.0, 31.0))));
    assert!(walls
        .iter()
        .any(|b| b.contains_point(Vec3d::new(31.0, 0.0, 31.0))));
    assert!(!walls.iter().any(|b| b.contains_point(Vec3d::zero())));
    for t in [0.0, -1.0, 32.0, f64::NAN, f64::INFINITY] {
        assert!(hollow::shell(&brush, &world(), t, Direction::Inward, &[]).is_err());
    }
}
#[test]
fn connected_bevel_preserves_shared_edges_and_is_order_independent() {
    let brush = cube();
    let x = brush
        .faces()
        .iter()
        .position(|f| f.normal()[0] > 0.99)
        .unwrap();
    let z = brush
        .faces()
        .iter()
        .position(|f| f.normal()[2] > 0.99)
        .unwrap();
    for depth in [0.0, 4.0] {
        let a = brush.bevel_face_region(&[x, z], 4.0, depth).unwrap();
        let b = brush.bevel_face_region(&[z, x, x], 4.0, depth).unwrap();
        assert!((a.volume() - b.volume()).abs() < 1e-7);
        for y in [-28.0, 28.0] {
            assert!(a
                .vertex_positions()
                .iter()
                .any(|p| vec::length(*p - Vec3d::new(32.0 + depth, y, 32.0 + depth)) < 1e-6));
        }
        assert!(a
            .vertex_positions()
            .iter()
            .any(|p| vec::length(*p - Vec3d::new(-32.0, -32.0, -32.0)) < 1e-6));
        a.geometry().check_invariant().unwrap();
    }
    assert!(brush.bevel_face_region(&[x, z], 40.0, 0.0).is_err());
    assert!(brush.bevel_face_region(&(0..brush.faces().len()).collect::<Vec<_>>(),4.0,0.0).is_err());
}
#[test]
fn stair_turns_landings_and_slabs_are_connected_disjoint_and_fit_bounds() {
    let bounds = Bbox3d::new(Vec3d::zero(), Vec3d::new(128.0, 160.0, 64.0));
    for layout in [
        StairLayout::Straight,
        StairLayout::Left,
        StairLayout::Right,
        StairLayout::Return,
    ] {
        for axis in [Axis::X, Axis::Y] {
            for ascending in [false, true] {
                let mut volumes = Vec::new();
                for slab in [0.0, 8.0] {
                    let options = StairOptions {
                        layout,
                        width: 32.0,
                        landing: 32.0,
                        slab,
                    };
                    let stairs = shapes::create_stairs_options(
                        &world(),
                        &bounds,
                        8.0,
                        axis,
                        ascending,
                        options,
                        "brick",
                    )
                    .unwrap();
                    assert_eq!(stairs.len(), 9);
                    disjoint(&stairs);
                    for brush in &stairs {
                        assert!(brush.volume() > 0.0);
                        brush.geometry().check_invariant().unwrap();
                        assert!(brush.vertex_positions().iter().all(|p| {
                            (0..3).all(|i| {
                                p[i] >= bounds.min[i] - 1e-6 && p[i] <= bounds.max[i] + 1e-6
                            })
                        }));
                    }
                    let u = axis.index();
                    let v = 1 - u;
                    let run = bounds.size()[u];
                    let side = bounds.size()[v];
                    let sign = if ascending { 1.0 } else { -1.0 };
                    let ss = (if u == 0 { 1.0 } else { -1.0 })
                        * sign
                        * if layout == StairLayout::Right {
                            -1.0
                        } else {
                            1.0
                        };
                    let point = |r: f64, s: f64, z: f64| {
                        let mut p = Vec3d::zero();
                        p[u] = if sign > 0.0 { r } else { run - r };
                        p[v] = if ss > 0.0 { s } else { side - s };
                        p[2] = z;
                        p
                    };
                    let length = if layout == StairLayout::Straight {
                        (run - 32.0) * 0.5
                    } else {
                        run - 32.0
                    };
                    let width = if layout == StairLayout::Straight {
                        side
                    } else {
                        32.0
                    };
                    let joint = point(length / 4.0, width * 0.5, 4.0);
                    assert!(
                        stairs[0].contains_point(joint) && stairs[1].contains_point(joint),
                        "disconnected underside"
                    );
                    let landing = point(length, width * 0.5, 28.0);
                    assert!(
                        stairs[3].contains_point(landing) && stairs[8].contains_point(landing),
                        "flight misses landing"
                    );
                    volumes.push(stairs.iter().map(Brush::volume).sum::<f64>());
                }
                assert!(volumes[1] < volumes[0]);
            }
        }
    }
    let invalid = StairOptions {
        layout: StairLayout::Return,
        width: 100.0,
        ..Default::default()
    };
    assert!(
        shapes::create_stairs_options(&world(), &bounds, 8.0, Axis::X, true, invalid, "brick")
            .is_err()
    );
}
#[test]
fn outer_fillet_wraps_texture_across_its_segment_strip() {
    let brush = cube();
    let mut faces = brush.faces().to_vec();
    for face in &mut faces {
        face.rotate_uv(27.0);
        face.uv_attributes.scale = geometry_math::vec::Vec2f::new(0.75, -1.5);
        face.uv_attributes.offset = geometry_math::vec::Vec2f::new(17.0, -31.0);
    }
    let brush = brush.with_exact_face_attributes(&faces).unwrap();
    let rounded = map_model::fillet::fillet_edge(
        &brush,
        Vec3d::new(32.0, 32.0, -32.0),
        Vec3d::new(32.0, 32.0, 32.0),
        8.0,
        8,
    )
    .unwrap();
    let faces: Vec<_> = rounded
        .geometry_faces()
        .filter(|(f, _)| {
            f.normal()[2].abs() < 1e-6 && f.normal()[0] >= -1e-6 && f.normal()[1] >= -1e-6
        })
        .collect();
    let mut checked = 0;
    for (i, (a, pa)) in faces.iter().enumerate() {
        for (b, pb) in faces.iter().skip(i + 1) {
            let shared: Vec<_> = pa
                .iter()
                .filter(|p| pb.iter().any(|q| vec::squared_distance(**p, *q) < 1e-12))
                .collect();
            if shared.len() < 2 {
                continue;
            }
            for p in shared {
                let d = a.uv_coords(*p) - b.uv_coords(*p);
                assert!(
                    d.x().abs() < 0.002 && d.y().abs() < 0.002,
                    "UV seam {d:?}: {:?} / {:?} at {p:?}",
                    a.plane,
                    b.plane
                );
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 9);
    assert!(map_model::fillet::fillet_edge(
        &brush,
        Vec3d::new(32.0, 32.0, -32.0),
        Vec3d::new(32.0, 32.0, 32.0),
        f64::NAN,
        8
    )
    .is_err());
}
