//! Where the exporter's time goes: parsing the `.map`, building an engine level from it
//! (the geometry, then the buried-face, coplanar-merge and overlap passes), and the
//! writer stage that turns the built level into engine files.
//!
//! `cargo bench -p scene_export`. The dragon (127 brushes) is the standing input.

// `criterion_group!` writes a public function of its own, which the crate's
// `missing_docs` would otherwise ask us to document.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};

use scene_export::builder::{self, BuildOptions, LevelOptions, MeshFormat};
use scene_export::geometry::{self, SolidOptions, TextureSizes};
use scene_export::indexmap::IndexMap;
use scene_export::settings::Settings;
use scene_export::writer;

const DRAGON: &str = include_str!("../../../assets/dragon.map");

fn parse_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("map_parse");
    group.throughput(Throughput::Bytes(DRAGON.len() as u64));
    group.bench_function("dragon", |b| {
        b.iter(|| scene_export::map::parse(black_box(DRAGON), "dragon").expect("the dragon parses"))
    });
    group.finish();
}

fn build_bench(c: &mut Criterion) {
    let map = scene_export::map::parse(DRAGON, "dragon").expect("the dragon parses");
    let classes = IndexMap::new();
    let settings = Settings::default();
    let options = BuildOptions {
        scene_name: "dragon".to_string(),
        mesh_dir: "dragon".to_string(),
    };

    c.bench_function("build_level_dragon", |b| {
        b.iter(|| {
            builder::build(
                black_box(&map),
                black_box(&classes),
                black_box(&settings),
                black_box(&options),
            )
        })
    });
}

/// The engine-neutral level build (`build_level`), which every non-Godot target goes
/// through: the geometry passes plus packing each solid into a `.glb`.
fn build_level_bench(c: &mut Criterion) {
    let map = scene_export::map::parse(DRAGON, "dragon").expect("the dragon parses");
    let classes = IndexMap::new();
    let settings = Settings::default();
    let options = level_options();

    c.bench_function("build_level_glb_dragon", |b| {
        b.iter(|| {
            builder::build_level(
                black_box(&map),
                black_box(&classes),
                black_box(&settings),
                black_box(&options),
            )
        })
    });
}

/// The writer stage alone, on a level built once: what a target format costs on top of
/// the shared build.
fn writers_bench(c: &mut Criterion) {
    let map = scene_export::map::parse(DRAGON, "dragon").expect("the dragon parses");
    let classes = IndexMap::new();
    let settings = Settings::default();

    let scene = builder::build(
        &map,
        &classes,
        &settings,
        &BuildOptions {
            scene_name: "dragon".to_string(),
            mesh_dir: "dragon".to_string(),
        },
    )
    .scene;
    let level = builder::build_level(&map, &classes, &settings, &level_options());

    let mut group = c.benchmark_group("writers");
    group.bench_function("tscn_dragon", |b| {
        b.iter(|| writer::tscn::write(black_box(&scene)))
    });
    group.bench_function("three_dragon", |b| {
        b.iter(|| writer::three::write(black_box(&level.level), black_box(&level.collision)))
    });
    group.bench_function("raylib_dragon", |b| {
        b.iter(|| writer::raylib::write(black_box(&level.level), black_box(&level.collision)))
    });
    group.finish();
}

fn level_options() -> LevelOptions {
    LevelOptions {
        name: "dragon".to_string(),
        mesh_dir: "dragon".to_string(),
        collision: true,
        inline_collision: true,
        mesh_collision: true,
        convex_collision: false,
        // What every target but Unity writes; Unity's .obj path is a separate cost.
        mesh_format: MeshFormat::Glb,
    }
}

/// Serialize the same prebuilt visual meshes in both formats, without parsing,
/// geometry construction, material embedding or filesystem writes in the timing.
fn mesh_writers_bench(c: &mut Criterion) {
    let map = scene_export::map::parse(DRAGON, "dragon").expect("the dragon parses");
    let settings = Settings::default();
    let sizes = TextureSizes::new(64.0);
    let meshes: Vec<_> = map
        .entities
        .iter()
        .map(|entity| {
            geometry::build_solid(entity, &settings, SolidOptions::default(), &sizes).mesh
        })
        .filter(|mesh| !mesh.is_empty())
        .collect();
    assert!(
        !meshes.is_empty(),
        "the fixture must exercise mesh serialization"
    );
    let triangles: usize = meshes.iter().map(|mesh| mesh.triangle_count()).sum();
    let collision = writer::gltf::Collision::default();
    let mut group = c.benchmark_group("mesh_writers");
    group.throughput(Throughput::Elements(triangles as u64));
    group.bench_function("glb_dragon", |b| {
        b.iter(|| {
            for mesh in &meshes {
                black_box(
                    writer::gltf::write(
                        black_box(mesh),
                        &collision,
                        "dragon",
                        writer::gltf::GltfOptions::default(),
                    )
                    .expect("a visual mesh produces a GLB"),
                );
            }
        })
    });
    group.bench_function("obj_dragon", |b| {
        b.iter(|| {
            for mesh in &meshes {
                black_box(writer::obj::write(
                    black_box(mesh),
                    "dragon",
                    writer::obj::ObjOptions::default(),
                ));
            }
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    parse_bench,
    build_bench,
    build_level_bench,
    writers_bench,
    mesh_writers_bench
);
criterion_main!(benches);
