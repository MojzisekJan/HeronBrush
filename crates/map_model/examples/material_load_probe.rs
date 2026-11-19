//! Time material discovery without GPU uploads or pixel decoding.
use map_model::material::MaterialCollection;
use std::{hint::black_box, time::Instant};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "material_load_probe <directory> <runs>");
    let runs: usize = args[2].parse().expect("run count");
    assert!((1..=31).contains(&runs));
    let mut samples = Vec::new();
    let mut count = 0;
    let mut normals = 0;
    for round in 0..=runs {
        let start = Instant::now();
        let collection = MaterialCollection::load_directory(&args[1]).expect("read fixture");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert!(
            collection.warnings().is_empty(),
            "{:?}",
            collection.warnings()
        );
        count = collection.len();
        normals = collection
            .materials()
            .iter()
            .filter(|m| m.normal_map.is_some())
            .count();
        black_box(&collection);
        if round > 0 {
            samples.push(elapsed);
        }
    }
    println!(
        "{}",
        serde_json::json!({"materials":count,"normals":normals,"samples_ms":samples})
    );
}
