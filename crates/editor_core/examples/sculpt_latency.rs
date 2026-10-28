//! Repeatable Freeform latency probe: cargo run --release -p editor_core --example sculpt_latency.
use editor_core::{
    document::Document,
    sculpt::{SculptDirection, SculptForm, SculptParameters},
    world::WorldBounds,
};
use std::time::{Duration, Instant};

fn main() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../gamedata/heron_brush/initial_valve.map"
    );
    println!("optimized={}", !cfg!(debug_assertions));
    let parameters = SculptParameters {
        radius: 8.0,
        direction: SculptDirection::Normal,
        form: SculptForm::Free,
        ..Default::default()
    };
    let mut document = Document::open(path, WorldBounds::default()).expect("starting room");
    document.clear_selection();
    let mut at = document.selectable_brushes()[0].1.bounds().max;
    at[0] = 0.3;
    at[1] = 0.7;
    at[2] = 144.0;
    let mut along = at;
    along[0] = 0.0;
    along[1] = 0.0;
    along[2] = 1.0;
    let mut stroke = document.begin_free_stroke(16.0, along);
    println!(
        "radius={}, effective_radius={}, spacing={}, dabs_for_64_units={}",
        parameters.radius,
        parameters.free_radius(16.0),
        stroke.dab_spacing(&parameters),
        (64.0 / stroke.dab_spacing(&parameters)).floor() as usize
    );
    let mut times = Vec::new();
    for n in 0..8 {
        let started = Instant::now();
        let mut point = at;
        point[0] += n as f64 * 8.0;
        let changed = document.sculpt_dab(point, &parameters, &mut stroke);
        let elapsed = started.elapsed();
        times.push(elapsed);
        println!(
            "dab {n}: {:.3} ms, brushes={}, changed={changed}",
            elapsed.as_secs_f64() * 1000.0,
            document.selectable_brushes().len()
        );
    }
    let started = Instant::now();
    document.end_free_stroke(&mut stroke);
    println!(
        "stroke end: {:.3} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    let total: Duration = times.iter().sum();
    times.sort();
    println!(
        "dabs total: {:.3} ms, median: {:.3} ms, max: {:.3} ms",
        total.as_secs_f64() * 1000.0,
        times[times.len() / 2].as_secs_f64() * 1000.0,
        times.last().unwrap().as_secs_f64() * 1000.0
    );
}
