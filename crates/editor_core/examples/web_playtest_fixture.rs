//! Generate an independent native authoring snapshot/golden level for browser worker checks.
use editor_core::{
    document::Document,
    export::{self, Contents, Options, Target},
    world::WorldBounds,
};
use std::path::{Path, PathBuf};

fn main() {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let input = args.next();
    let mut files = Vec::new();
    let document = if let Some(count) = input.as_deref().and_then(|s| s.strip_prefix("large:")) {
        use geometry_math::{bbox::Bbox3d, vec::Vec3d};
        use map_model::io::map_reader::{Brush, Entity, MapFile};
        let count: usize = count.parse().expect("large:BRUSH_COUNT");
        assert!((1..=10000).contains(&count));
        let side = (count as f64).sqrt().ceil() as usize;
        let bounds = WorldBounds::default();
        let world_bounds = Bbox3d::from_scalars(-bounds.half_extent(), bounds.half_extent());
        let mut world = Entity {
            properties: vec![("classname".into(), "worldspawn".into())],
            ..Default::default()
        };
        for index in 0..count {
            let min = Vec3d::new(
                (index % side) as f64 * 160.0 - side as f64 * 80.0,
                (index / side) as f64 * 160.0 - side as f64 * 80.0,
                0.0,
            );
            let brush = map_model::shapes::create_cuboid(
                &world_bounds,
                &Bbox3d::new(
                    min,
                    min + Vec3d::new(128.0, 128.0, 32.0 + (index % 8) as f64 * 16.0),
                ),
                map_model::shapes::DEFAULT_MATERIAL,
            )
            .unwrap();
            world.brushes.push(Brush {
                faces: brush.faces().to_vec(),
                id: Some(index as u64 + 1),
                line: 0,
            });
        }
        let map = output.join("large.map");
        std::fs::write(
            &map,
            map_model::io::map_writer::write_map(&MapFile {
                entities: vec![world],
                ..Default::default()
            }),
        )
        .unwrap();
        let mut document = Document::open(map, bounds).unwrap();
        document.path = None;
        document
    } else if input.as_deref() == Some("pbr") {
        let model = output
            .join("model.glb")
            .canonicalize()
            .unwrap_or_else(|_| output.join("model.glb"));
        std::fs::copy(root.join("examples/editor-checks/fixtures/pbr.glb"), &model).unwrap();
        let model = model.canonicalize().unwrap();
        files.push(serde_json::json!({"path":model,"url":"model.glb"}));
        let mut document = Document::new(WorldBounds::default());
        document
            .import_model(&model, geometry_math::vec::Vec3d::zero())
            .unwrap();
        document
    } else {
        let map = input
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("demos/motion_lab/motion_lab.map"));
        let mut document = Document::open(&map, WorldBounds::default()).unwrap();
        let definitions = map.with_extension("fgd");
        if definitions.is_file() {
            document.load_definitions(definitions).unwrap();
        }
        document
    };
    let directory = Path::new("/heron-brush/files/worker-check");
    let options = Options {
        target: Target::ThreeJs,
        name: "worker".into(),
        ..Default::default()
    };
    let prepare = std::time::Instant::now();
    let snapshot_text =
        serde_json::to_string(&document.export_snapshot(&directory.join("worker.map"))).unwrap();
    let native_prepare_ms = prepare.elapsed().as_secs_f64() * 1000.0;
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot_text).unwrap();
    let (report, exported) = export::export_files(&document, directory, &options).unwrap();
    let level = exported
        .into_iter()
        .find_map(|(path, data)| match data {
            Contents::Text(data) if path.file_name().is_some_and(|p| p == "worker.json") => {
                Some(serde_json::from_str::<serde_json::Value>(&data).unwrap())
            }
            _ => None,
        })
        .unwrap();
    let job = serde_json::json!({"snapshot":snapshot,"directory":directory,"options":options,"expectedLevel":level,"files":files,"triangles":report.triangles,"nativePrepareMs":native_prepare_ms,"snapshotBytes":snapshot_text.len(),"brushes":document.brush_count()});
    std::fs::write(output.join("job.json"), serde_json::to_vec(&job).unwrap()).unwrap();
    println!(
        "Native worker fixture: {} triangles, {} entities",
        report.triangles,
        job["expectedLevel"]["entities"].as_array().unwrap().len()
    );
}
