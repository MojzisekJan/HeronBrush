//! Small release-mode map loading probe; see docs/performance.md.
use geometry_math::bbox::Bbox3d;
use map_model::{io::map_reader::read_map, node};
use std::{hint::black_box, time::Instant};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        4,
        "map_load_probe <map> <parse|load|borrowed> <runs>"
    );
    let source = std::fs::read_to_string(&args[1]).expect("read fixture");
    let runs: usize = args[3].parse().expect("run count");
    assert!((1..=31).contains(&runs));
    let bounds = Bbox3d::from_scalars(-16384.0, 16384.0);
    let mut samples = Vec::new();
    let mut count = 0;
    for round in 0..=runs {
        let start = Instant::now();
        let map = read_map(black_box(&source), None).expect("parse fixture");
        if args[2] == "parse" {
            let elapsed = start.elapsed();
            count = map.entities.iter().map(|entity| entity.brushes.len()).sum();
            black_box(&map);
            if round > 0 {
                samples.push(elapsed.as_secs_f64() * 1000.0);
            }
        } else {
            let tree = match args[2].as_str() {
                "load" => node::build_tree_owned(map, &bounds),
                "borrowed" => node::build_tree(&map, &bounds),
                _ => panic!("unknown mode"),
            };
            let elapsed = start.elapsed();
            assert!(
                tree.warnings.is_empty(),
                "fixture must build without warnings: {:?}",
                tree.warnings
            );
            count = node::brushes(&tree.world).len();
            black_box(&tree);
            if round > 0 {
                samples.push(elapsed.as_secs_f64() * 1000.0);
            }
        }
    }
    let peak_kib = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("VmHWM:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|value| value.parse::<u64>().ok())
        });
    println!(
        "{}",
        serde_json::json!({"mode": args[2], "source_bytes": source.len(),
        "brushes": count, "samples_ms": samples, "peak_rss_kib": peak_kib,
        "warmup_runs": 1, "timing_includes_result_drop": false})
    );
}
