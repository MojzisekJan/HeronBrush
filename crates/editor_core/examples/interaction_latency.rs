//! Reproducible pointer-picking and duplication workloads, excluding map setup.
//! cargo run --release -p editor_core --example interaction_latency
use editor_core::{document::Document, world::WorldBounds};
use geometry_math::{
    bbox::Bbox3d,
    ray::Ray3d,
    vec::{Vec2f, Vec3d},
};
use map_model::{
    io::map_reader::{Entity, MapFile, Patch, PatchControlPoint},
    map_format::MapFormat,
    node,
};
use std::{hint::black_box, time::Instant};

fn document(brushes: usize, patches: usize, stacked: bool) -> Document {
    let bounds = Document::new(WorldBounds::default()).world_bounds();
    let mut entity = Entity {
        properties: vec![("classname".into(), "worldspawn".into())],
        ..Default::default()
    };
    for i in 0..brushes {
        let at = if stacked {
            Vec3d::new(0.0, 0.0, i as f64 * 1.5)
        } else {
            Vec3d::new((i % 100) as f64 * 64.0, (i / 100) as f64 * 64.0, 0.0)
        };
        let size = if stacked { 1.0 } else { 32.0 };
        let brush = map_model::shapes::create_cuboid(
            &bounds,
            &Bbox3d::new(at, at + Vec3d::fill(size)),
            "stone",
        )
        .unwrap();
        entity.brushes.push(map_model::io::map_reader::Brush {
            faces: brush.faces().to_vec(),
            ..Default::default()
        });
    }
    for i in 0..patches {
        let at = if i == 0 {
            Vec3d::zero()
        } else {
            Vec3d::new(1000.0 + (i % 64) as f64 * 64.0, (i / 64) as f64 * 64.0, 0.0)
        };
        let mut control_points = Vec::new();
        for row in 0..3 {
            for column in 0..3 {
                control_points.push(PatchControlPoint {
                    position: at
                        + Vec3d::new(
                            column as f64 * 16.0,
                            row as f64 * 16.0,
                            if column == 1 && row == 1 { 16.0 } else { 0.0 },
                        ),
                    uv: Vec2f::new(column as f32, row as f32),
                });
            }
        }
        entity.patches.push(Patch {
            material_name: "stone".into(),
            rows: 3,
            columns: 3,
            control_points,
            id: None,
            line: 0,
        });
    }
    let map = MapFile {
        entities: vec![entity],
        format: MapFormat::Valve,
        ..Default::default()
    };
    Document::from_world(
        node::build_tree_owned(map, &bounds).world,
        WorldBounds::default(),
        MapFormat::Valve,
        None,
    )
}

fn report(name: &str, mut samples: Vec<f64>) {
    samples.sort_by(f64::total_cmp);
    println!(
        "{name}: median={:.3}ms p95={:.3}ms",
        samples[samples.len() / 2],
        samples[(samples.len() * 95 / 100).min(samples.len() - 1)]
    );
}

fn main() {
    assert!(
        !cfg!(debug_assertions),
        "measure an optimized release build"
    );
    for n in [32, 256, 2048] {
        let d = document(0, n, false);
        for (label, at) in [
            ("hit", Vec3d::new(16.0, 16.0, 128.0)),
            ("miss", Vec3d::new(-100.0, -100.0, 128.0)),
        ] {
            let ray = Ray3d::new(at, Vec3d::new(0.0, 0.0, -1.0));
            assert_eq!(d.pick(&ray).is_some(), label == "hit");
            let mut samples = Vec::new();
            for _ in 0..50 {
                let t = Instant::now();
                black_box(d.pick(&ray));
                samples.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            report(&format!("pick {n} patches {label}"), samples);
        }
    }
    for n in [1000, 10000] {
        let mut d = document(n, 0, false);
        let mut samples = Vec::new();
        for _ in 0..5 {
            d.select_all();
            let t = Instant::now();
            assert_eq!(d.duplicate_selection_by(Vec3d::new(0.0, 0.0, 64.0)), n);
            samples.push(t.elapsed().as_secs_f64() * 1000.0);
            d.undo();
        }
        report(&format!("duplicate {n} brushes"), samples);
        let d = document(n, 0, true);
        let ray = Ray3d::new(Vec3d::new(0.5, 0.5, 16000.0), Vec3d::new(0.0, 0.0, -1.0));
        let mut samples = Vec::new();
        for _ in 0..5 {
            let t = Instant::now();
            assert!(black_box(d.pick_flat(&ray)).is_some());
            samples.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        report(&format!("flat pick {n} stacked brushes"), samples);
    }
}
