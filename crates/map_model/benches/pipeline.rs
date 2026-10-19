//! Where the document layer's time goes: reading a `.map`, turning it into the node
//! tree (every brush validated and given a polyhedron), the convex hull, CSG, and the
//! shape generators.
//!
//! `cargo bench -p map_model`. The dragon in `assets/` is the standing input: 127 brushes,
//! many of them convex merges and cones, so it exercises the hull and the face maths
//! rather than just axis-aligned boxes.

// `criterion_group!` writes a public function of its own, which the crate's
// `missing_docs` would otherwise ask us to document.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};

use map_model::brush::Brush;
use map_model::io::map_reader::read_map;
use map_model::node::build_tree;
use map_model::polyhedron::Polyhedron;
use map_model::shapes::{create_cuboid, create_cylinder, CircleShape};
use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

const DRAGON: &str = include_str!("../../../assets/dragon.map");

fn world() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

fn read_map_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_map");
    group.throughput(Throughput::Bytes(DRAGON.len() as u64));
    group.bench_function("dragon", |b| {
        b.iter(|| read_map(black_box(DRAGON), None).expect("the dragon parses"))
    });
    group.finish();
}

fn build_tree_bench(c: &mut Criterion) {
    let map = read_map(DRAGON, None).expect("the dragon parses");
    let bounds = world();
    let mut group = c.benchmark_group("build_tree");
    group.throughput(Throughput::Elements(127));
    group.bench_function("dragon", |b| {
        b.iter(|| build_tree(black_box(&map), black_box(&bounds)))
    });
    group.finish();
}

fn hull_bench(c: &mut Criterion) {
    // A point cloud the size a convex merge of a few brushes gives the hull.
    let mut seed = 0x1234_5678_u64;
    let mut rand = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as f64 / u32::MAX as f64) * 2.0 - 1.0
    };
    let cloud: Vec<Vec3d> = (0..96)
        .map(|_| Vec3d::new(rand() * 64.0, rand() * 64.0, rand() * 64.0))
        .collect();

    c.bench_function("hull_from_96_points", |b| {
        b.iter(|| Polyhedron::from_points(black_box(&cloud), f64::EPSILON).expect("a hull"))
    });
}

fn csg_bench(c: &mut Criterion) {
    let bounds = world();
    let target = create_cuboid(&bounds, &Bbox3d::from_scalars(-64.0, 64.0), "stone").unwrap();
    let tool = create_cuboid(
        &bounds,
        &Bbox3d::new(Vec3d::new(-16.0, -16.0, -16.0), Vec3d::new(80.0, 80.0, 80.0)),
        "stone",
    )
    .unwrap();

    c.bench_function("csg_subtract_box_from_box", |b| {
        b.iter(|| map_model::csg::subtract(black_box(&target), black_box(&tool)))
    });
}

fn shapes_bench(c: &mut Criterion) {
    let bounds = world();
    let inside = Bbox3d::from_scalars(-64.0, 64.0);

    c.bench_function("create_cuboid", |b| {
        b.iter(|| create_cuboid(black_box(&bounds), black_box(&inside), "stone").unwrap())
    });
    c.bench_function("create_cylinder_64_sides", |b| {
        b.iter(|| {
            create_cylinder(
                black_box(&bounds),
                black_box(&inside),
                CircleShape::EdgeAligned { num_sides: 64 },
                geometry_math::util::Axis::Z,
                "stone",
            )
            .unwrap()
        })
    });
}

fn map_text_roundtrip(c: &mut Criterion) {
    let map = read_map(DRAGON, None).expect("the dragon parses");
    let tree = build_tree(&map, &world());
    let brushes: Vec<Brush> = map_model::node::brushes(&tree.world).into_iter().cloned().collect();
    c.bench_function("write_then_read_dragon_brushes", |b| {
        b.iter(|| {
            let flat = map_model::node::flatten_tree(&tree.world, map_model::map_format::MapFormat::Standard, None);
            let text = map_model::io::map_writer::write_map(&flat);
            black_box(read_map(&text, None).expect("round-trips"));
            black_box(brushes.len())
        })
    });
}

/// Writing alone, which the round-trip above cannot separate from reading.
///
/// The writer turns a flattened tree into `.map` text, and it is what a save costs after
/// the tree is in hand -- so it is worth a number of its own.
fn write_map_bench(c: &mut Criterion) {
    let map = read_map(DRAGON, None).expect("the dragon parses");
    let tree = build_tree(&map, &world());
    let flat = map_model::node::flatten_tree(&tree.world, map_model::map_format::MapFormat::Standard, None);
    let bytes = map_model::io::map_writer::write_map(&flat).len();

    let mut group = c.benchmark_group("write_map");
    group.throughput(Throughput::Bytes(bytes as u64));
    group.bench_function("dragon", |b| {
        b.iter(|| black_box(map_model::io::map_writer::write_map(black_box(&flat))))
    });
    group.finish();
}

fn tree_walk_bench(c: &mut Criterion) {
    // A map with many small brushes spread over a grid, to time walking the tree rather
    // than building geometry.
    //
    // The winding matters and used to be wrong here: every one of the 2,000 brushes came
    // out `InvalidBrush { cause: Empty }`, so the tree held nothing and both benchmarks
    // timed a walk over an empty world -- 10.6 ns and 36.4 ns, which is what an empty
    // walk costs. The assertion below is what stops that being invisible again.
    let mut text = String::from("{\n\"classname\" \"worldspawn\"\n");
    for i in 0..2000 {
        let (x, y, z) = ((i % 40) * 32, ((i / 40) % 40) * 32, (i / 1600) * 32);
        let (x1, y1, z1) = (x + 16, y + 16, z + 16);
        text.push_str(&format!(
            "{{\n\
             ( {x} {y} {z} ) ( {x} {y1} {z} ) ( {x} {y} {z1} ) stone 0 0 0 1 1\n\
             ( {x} {y} {z} ) ( {x} {y} {z1} ) ( {x1} {y} {z} ) stone 0 0 0 1 1\n\
             ( {x} {y} {z} ) ( {x1} {y} {z} ) ( {x} {y1} {z} ) stone 0 0 0 1 1\n\
             ( {x1} {y1} {z1} ) ( {x1} {y} {z1} ) ( {x} {y1} {z1} ) stone 0 0 0 1 1\n\
             ( {x1} {y1} {z1} ) ( {x} {y1} {z1} ) ( {x1} {y1} {z} ) stone 0 0 0 1 1\n\
             ( {x1} {y1} {z1} ) ( {x1} {y1} {z} ) ( {x1} {y} {z1} ) stone 0 0 0 1 1\n\
             }}\n"
        ));
    }
    text.push_str("}\n");

    let map = read_map(&text, None).expect("the grid parses");
    let tree = build_tree(&map, &world());
    assert_eq!(
        map_model::node::brushes(&tree.world).len(),
        2000,
        "the grid must reach the tree, or these benchmarks time an empty walk"
    );

    // Return the collected vector so the walk cannot be hoisted out of the loop.
    c.bench_function("walk_2000_brushes", |b| {
        b.iter(|| black_box(map_model::node::brushes(black_box(&tree.world))))
    });
    c.bench_function("walk_2000_brushes_by_layer", |b| {
        b.iter(|| black_box(map_model::node::brushes_by_layer(black_box(&tree.world))))
    });
}

criterion_group!(
    benches,
    read_map_bench,
    build_tree_bench,
    hull_bench,
    csg_bench,
    shapes_bench,
    map_text_roundtrip,
    write_map_bench,
    tree_walk_bench,
);
criterion_main!(benches);
