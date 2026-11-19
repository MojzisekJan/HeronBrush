//! Isolated UV sampling comparison; see docs/performance.md.
use geometry_math::vec::{Vec2f, Vec3d};
use map_model::uv::{UvAttributes, UvCoordSystem};
use std::{hint::black_box, time::Instant};

fn main() {
    for system in [
        UvCoordSystem::Paraxial,
        UvCoordSystem::Parallel {
            u_axis: Vec3d::new(0.3, -0.8, 0.2),
            v_axis: Vec3d::new(-0.1, 0.4, 1.2),
        },
    ] {
        for corners in [6, 48] {
            let mut samples = [Vec::new(), Vec::new()];
            for round in 0..8 {
                let mut checksums = [0.0f64; 2];
                for prepared in if round % 2 == 0 {
                    [false, true]
                } else {
                    [true, false]
                } {
                    let index = usize::from(prepared);
                    let start = Instant::now();
                    for face in 0..20_000 {
                        let attributes = black_box(UvAttributes {
                            offset: Vec2f::new(13.25, -29.5),
                            scale: Vec2f::new(-0.125, 3.7),
                            rotation: (face % 360) as f32,
                        });
                        let system = black_box(system);
                        let normal = black_box(Vec3d::new(0., 0., 1.));
                        let projection = prepared.then(|| system.projection(normal, &attributes));
                        for vertex in 0..corners {
                            let point = black_box(Vec3d::new(vertex as f64 * 1.7, -32., 11.));
                            let uv = match projection {
                                Some(projection) => projection.uv_coords(point),
                                None => system.uv_coords(point, normal, &attributes),
                            };
                            checksums[index] += f64::from(black_box(uv.x() + uv.y()));
                        }
                    }
                    if round > 0 {
                        samples[index].push(start.elapsed().as_secs_f64() * 1000.);
                    }
                }
                assert_eq!(checksums[0].to_bits(), checksums[1].to_bits());
            }
            for sample in &mut samples {
                sample.sort_by(f64::total_cmp);
            }
            println!(
                "system={system:?} corners={corners} scalar_ms={:.3} prepared_ms={:.3}",
                samples[0][3], samples[1][3]
            );
        }
    }
}
