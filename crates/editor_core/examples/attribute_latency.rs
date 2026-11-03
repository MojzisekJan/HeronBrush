//! Attribute editing and copy-allocation probe. Run with --release.
use editor_core::{
    document::{Document, FaceAttributeTransfer, FaceRef},
    world::WorldBounds,
};
use geometry_math::{
    bbox::Bbox3d,
    util::Axis,
    vec::{Vec2f, Vec3d},
};
use map_model::{
    brush::Brush,
    brush_face::{FaceColor, Rgb},
    map_format::MapFormat,
    node::{Group, Node, NodeKind},
    shapes::{self, CircleShape},
};
use std::{hint::black_box, time::Instant};

fn boxes(n: usize) -> Document {
    let mut d = Document::new(WorldBounds::default());
    let brush = shapes::create_cuboid(&d.world_bounds(), &Bbox3d::from_scalars(0.0, 32.0), "stone")
        .unwrap();
    let brushes = (0..n)
        .map(|i| {
            brush
                .translate(
                    &d.world_bounds(),
                    Vec3d::new((i % 100) as f64 * 64.0, (i / 100) as f64 * 64.0, 0.0),
                )
                .unwrap()
        })
        .collect();
    d.add_brushes(brushes);
    d
}
fn wrap(children: Vec<Node>, id: usize) -> Node {
    Node::new(NodeKind::Group(Group {
        id: format!("group-{id}"),
        name: format!("Group {id}"),
        linked_group_id: None,
        transformation: None,
        children,
    }))
}
fn grouped(n: usize, depth: usize, sparse: bool) -> Document {
    let d = boxes(n);
    let mut world = d.tree_world().clone();
    let mut children = std::mem::take(&mut world.layers[0].children);
    if sparse {
        let free = children.remove(0);
        world.layers[0].children = std::iter::once(free)
            .chain(
                children
                    .into_iter()
                    .enumerate()
                    .map(|(i, node)| wrap(vec![node], i)),
            )
            .collect();
    } else {
        for i in 0..depth {
            children = vec![wrap(children, i)];
        }
        world.layers[0].children = children;
    }
    Document::from_world(world, WorldBounds::default(), MapFormat::Valve, None)
}
fn measure(label: &str, d: &mut Document, mut edit: impl FnMut(&mut Document)) {
    let mut times = Vec::new();
    for _ in 0..5 {
        let t = Instant::now();
        edit(black_box(d));
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        d.undo().expect("one undo per measurement");
    }
    times.sort_by(f64::total_cmp);
    println!("{label}: median={:.3}ms p95={:.3}ms", times[2], times[4]);
}
fn cylinder(sides: usize) -> (Document, Brush) {
    let mut d = Document::new(WorldBounds::default());
    let b = shapes::create_cylinder(
        &d.world_bounds(),
        &Bbox3d::from_scalars(-128.0, 128.0),
        CircleShape::EdgeAligned { num_sides: sides },
        Axis::Z,
        "stone",
    )
    .unwrap();
    d.add_brushes(vec![b.clone()]);
    (d, b)
}
fn main() {
    assert!(
        !cfg!(debug_assertions),
        "measure an optimized release build"
    );
    for sides in [64, 256, 512] {
        let (mut d, _) = cylinder(sides);
        d.select_all();
        measure(&format!("UV edit cylinder {sides}"), &mut d, |d| {
            d.edit_faces(|f| f.move_uv(Vec2f::new(8.0, 16.0)))
        });
        d.select_all();
        let refs = d.selectable_brushes();
        let target = FaceRef {
            node: refs[0].0,
            face: 0,
        };
        drop(refs);
        measure(&format!("Paint one face cylinder {sides}"), &mut d, |d| {
            assert_eq!(
                d.set_face_color_on(&[target], Some(FaceColor::solid(Rgb::new(30, 80, 150)))),
                1
            );
        });
        d.clear_selection();
        let source = d.selectable_brushes()[0].0;
        d.select_face_only(FaceRef {
            node: source,
            face: 0,
        });
        d.edit_faces(|f| f.rotate_uv(37.0));
        let source_face = FaceRef {
            node: source,
            face: 0,
        };
        let target_face = FaceRef {
            node: source,
            face: 1,
        };
        measure(&format!("Transfer texture cylinder {sides}"), &mut d, |d| {
            assert!(d.copy_face_attributes(
                source_face,
                target_face,
                FaceAttributeTransfer::Projection,
                false
            ));
        });
    }
    let mut d = boxes(10000);
    d.select_all();
    measure("UV edit 10000 boxes", &mut d, |d| {
        d.edit_faces(|f| f.move_uv(Vec2f::new(8.0, 16.0)))
    });
    for depth in [0, 8, 32] {
        let mut d = grouped(1000, depth, false);
        let mut times = Vec::new();
        for _ in 0..5 {
            d.select_all();
            let t = Instant::now();
            assert_eq!(d.duplicate_selection_by(Vec3d::new(0.0, 0.0, 64.0)), 1000);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            d.undo().unwrap();
        }
        times.sort_by(f64::total_cmp);
        println!(
            "Duplicate 1000 boxes group depth {depth}: median={:.3}ms p95={:.3}ms",
            times[2], times[4]
        );
    }
    let mut d = grouped(10000, 1, true);
    let free = d.selectable_brushes()[0].0;
    let mut times = Vec::new();
    for _ in 0..5 {
        d.select_only(free);
        let t = Instant::now();
        assert_eq!(d.duplicate_selection_by(Vec3d::new(0.0, 0.0, 64.0)), 1);
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        d.undo().unwrap();
    }
    times.sort_by(f64::total_cmp);
    println!(
        "Duplicate one box beside 9999 groups: median={:.3}ms p95={:.3}ms",
        times[2], times[4]
    );
}
