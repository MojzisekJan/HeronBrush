//! Compare tree traversal costs for deferred UV resolution; see docs/performance.md.
use geometry_math::{bbox::Bbox3d, vec::Vec2f};
use map_model::{
    io::{map_reader::read_map, map_writer::write_map},
    node,
};
use std::{hint::black_box, time::Instant};

fn main() {
    let mut map = read_map(include_str!("../tests/fixtures/format.map"), None).unwrap();
    map.entities.truncate(1);
    map.entities[0].patches.clear();
    let mut brush = map.entities[0].brushes[0].clone();
    brush.id = None;
    for face in &mut brush.faces {
        face.primitive_uv = Some(Box::new([[0.01, 0.002, 0.25], [-0.003, 0.02, -0.5]]));
    }
    for count in [1_000, 4_000, 8_000] {
        map.entities[0].brushes = vec![brush.clone(); count];
        let tree = node::build_tree(&map, &Bbox3d::from_scalars(-16384., 16384.));
        assert!(tree.warnings.is_empty());
        let size = |_: &str| Some(Vec2f::new(128., 64.));
        let mut reference = None;
        let mut samples = [Vec::new(), Vec::new()];
        for round in 0..8 {
            for legacy in if round % 2 == 0 {
                [true, false]
            } else {
                [false, true]
            } {
                let mut world = tree.world.clone();
                let start = Instant::now();
                if legacy {
                    let resolved: Vec<_> = node::brushes_with_ids(&world)
                        .into_iter()
                        .filter_map(|(id, brush)| {
                            brush.resolved_primitive_uv(size).map(|b| (id, b))
                        })
                        .collect();
                    for (id, brush) in resolved {
                        node::replace_brush(&mut world, id, brush);
                    }
                } else {
                    node::map_brushes(&mut world, |brush| brush.resolved_primitive_uv(size));
                }
                let elapsed = start.elapsed().as_secs_f64() * 1000.;
                black_box(&world);
                if round > 0 {
                    samples[usize::from(!legacy)].push(elapsed);
                }
                if round == 0 {
                    let saved = write_map(&node::flatten_tree(&world, map.format, None));
                    if let Some(expected) = &reference {
                        assert_eq!(&saved, expected);
                    } else {
                        reference = Some(saved);
                    }
                }
            }
        }
        for values in &mut samples {
            values.sort_by(f64::total_cmp);
        }
        println!(
            "brushes={count} legacy_ms={:.3} single_pass_ms={:.3}",
            samples[0][3], samples[1][3]
        );
    }
}
