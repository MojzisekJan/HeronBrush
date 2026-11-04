//! MCP selection latency. Run with --release; setup is outside the timer.
use editor_core::{
    document::Document,
    mcp::{respond, Collab},
    world::WorldBounds,
};
use geometry_math::bbox::Bbox3d;
use map_model::{node, shapes};
use serde_json::json;
use std::{hint::black_box, time::Instant};

fn main() {
    assert!(!cfg!(debug_assertions), "measure a release build");
    for count in [1000, 10000] {
        let mut document = Document::new(WorldBounds::default());
        let brush = shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::from_scalars(0.0, 32.0),
            "stone",
        )
        .unwrap();
        document.add_brushes(vec![brush; count]);
        let ids: Vec<_> = node::brushes_with_ids(document.tree_world())
            .into_iter()
            .map(|(id, _)| id.0.to_string())
            .collect();
        let request = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call",
            "params":{"name":"select", "arguments":{"ids":ids}}})
        .to_string();
        let mut collab = Collab::default();
        let mut times = Vec::new();
        for _ in 0..5 {
            document.clear_selection();
            let start = Instant::now();
            let reply = respond(black_box(&mut document), &mut collab, &request, false).unwrap();
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            assert_ne!(reply["result"]["isError"], true, "{reply}");
            assert_eq!(document.selection_count(), count);
        }
        times.sort_by(f64::total_cmp);
        println!(
            "MCP select {count}: median={:.3}ms worst={:.3}ms",
            times[2], times[4]
        );
    }
}
