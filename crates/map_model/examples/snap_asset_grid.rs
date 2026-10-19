//! Snap built-in asset vertices to eighth units and validate saved geometry.
use map_model::{
    brush::Brush,
    io::{map_reader::read_map, map_writer::write_map},
};
use std::path::PathBuf;
use geometry_math::{bbox::Bbox3d, vec};
const GRID: f64 = 0.125;
fn snapped_brush(
    brush: &Brush,
    bounds: &Bbox3d,
) -> Result<(Brush, bool), Box<dyn std::error::Error>> {
    if let Ok(result) = brush.snap_vertices(GRID) {
        return Ok((result, false));
    }
    // Explicitly permitted for sub-grid details: keep the rounded surface and
    // give a collapsed hull one grid step of thickness along its thinnest axis.
    let source: Vec<_> = brush
        .geometry()
        .vertices()
        .iter()
        .map(|v| v.position)
        .collect();
    let thin = brush
        .faces()
        .iter()
        .min_by(|a, b| {
            let width = |n| {
                let values: Vec<_> = source.iter().map(|p| vec::dot(n, *p)).collect();
                values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                    - values.iter().copied().fold(f64::INFINITY, f64::min)
            };
            width(a.plane.normal).total_cmp(&width(b.plane.normal))
        })
        .unwrap()
        .plane
        .normal;
    let axis = (0..3)
        .max_by(|a, b| thin[*a].abs().total_cmp(&thin[*b].abs()))
        .unwrap();
    let mut points: Vec<_> = source
        .iter()
        .map(|p| vec::round(*p / GRID) * GRID)
        .collect();
    let extra: Vec<_> = points
        .iter()
        .map(|p| {
            let mut q = *p;
            q[axis] += GRID;
            q
        })
        .collect();
    points.extend(extra);
    let hull = map_model::polyhedron::Polyhedron::from_points(&points, 1e-7)?;
    let mut faces = Vec::new();
    for i in 0..hull.face_count() {
        let id = map_model::polyhedron::FaceId(i as u32);
        let plane = hull.face_plane(id);
        let mut face = brush
            .faces()
            .iter()
            .max_by(|a, b| {
                vec::dot(a.plane.normal, plane.normal)
                    .total_cmp(&vec::dot(b.plane.normal, plane.normal))
            })
            .unwrap()
            .clone();
        let p = hull.face_positions(id);
        face.points = [p[0], p[1], p[2]];
        face.plane = plane;
        faces.push(face);
    }
    Ok((Brush::from_faces(bounds, faces)?, true))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let write = args.iter().any(|x| x == "--write");
    let input = PathBuf::from(
        args.iter()
            .find(|x| !x.starts_with("--"))
            .map(String::as_str)
            .unwrap_or("assets"),
    );
    let mut paths = if input.is_dir() {
        std::fs::read_dir(input)?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![input]
    };
    paths.retain(|p| p.extension().is_some_and(|e| e == "map"));
    paths.sort();
    let bounds = Bbox3d::from_scalars(-16384., 16384.);
    let mut outputs = Vec::new();
    let mut failed = 0;
    for path in paths {
        let result =
            (|| -> Result<_, Box<dyn std::error::Error>> {
                let source = std::fs::read_to_string(&path)?;
                let mut map = read_map(&source, None)?;
                if !map.warnings.is_empty() {
                    return Err("parser warnings".into());
                }
                let mut old_volume = 0f64;
                let mut new_volume = 0f64;
                let mut simplified = 0;
                let mut original = Vec::new();
                let mut count = 0;
                let mut off = 0;
                let mut max_move = 0f64;
                for entity in &mut map.entities {
                    if !entity.patches.is_empty() {
                        return Err("patches unsupported".into());
                    }
                    for entry in &mut entity.brushes {
                        let brush = Brush::from_faces(&bounds, entry.faces.clone())?;
                        if !brush.vertices_are_on_grid(GRID) {
                            off += 1;
                        }
                        let already_exact = brush.geometry().vertices().iter().all(|v| {
                            vec::length(v.position - vec::round(v.position / GRID) * GRID) < 1e-7
                        }) && entry.faces.iter().all(|f| {
                            f.points
                                .iter()
                                .all(|p| vec::length(*p - vec::round(*p / GRID) * GRID) < 1e-7)
                        });
                        if already_exact {
                            original.push(brush);
                            count += 1;
                            continue;
                        }
                        let (snapped, changed_detail) = snapped_brush(&brush, &bounds)?;
                        simplified += usize::from(changed_detail);
                        let mut faces = snapped.faces().to_vec();
                        for face in &mut faces {
                            for p in &mut face.points {
                                *p = vec::round(*p / GRID) * GRID;
                            }
                        }
                        entry.faces = faces;
                        original.push(brush);
                        count += 1;
                    }
                }
                let output = write_map(&map);
                let parsed = read_map(&output, None)?;
                if !parsed.warnings.is_empty() {
                    return Err("saved parser warnings".into());
                }
                let mut index = 0;
                let mut contacts = std::collections::HashMap::new();
                for entry in parsed.entities.iter().flat_map(|e| &e.brushes) {
                    let rebuilt = Brush::from_faces(&bounds, entry.faces.clone())?;
                    if !rebuilt.geometry().vertices().iter().all(|v| {
                        vec::length(v.position - vec::round(v.position / GRID) * GRID) < 1e-5
                    }) {
                        return Err(format!("brush {index}: saved vertices off grid").into());
                    }
                    let before = &original[index];
                    for old in before.geometry().vertices() {
                        let want = vec::round(old.position / GRID) * GRID;
                        let key: [i64; 3] =
                            std::array::from_fn(|a| (old.position[a] * 1e5).round() as i64);
                        let target: [i64; 3] =
                            std::array::from_fn(|a| (want[a] / GRID).round() as i64);
                        if let Some(previous) = contacts.insert(key, target) {
                            if previous != target {
                                return Err(
                                    format!("brush {index}: shared vertex would separate").into()
                                );
                            }
                        }
                        if !rebuilt
                            .faces()
                            .iter()
                            .all(|f| vec::dot(f.plane.normal, want) - f.plane.distance < 1e-5)
                        {
                            return Err(format!("brush {index}: snapped point outside hull").into());
                        }
                        max_move = max_move.max(vec::length(old.position - want));
                    }
                    old_volume += before.volume();
                    new_volume += rebuilt.volume();
                    for v in rebuilt.geometry().vertices() {
                        let distance = before
                            .geometry()
                            .vertices()
                            .iter()
                            .map(|old| vec::length(old.position - v.position))
                            .fold(f64::INFINITY, f64::min);
                        if distance > 0.234 {
                            return Err(format!("brush {index}: vertex moved {distance}").into());
                        }
                        max_move = max_move.max(distance);
                    }
                    index += 1;
                }
                if index != count {
                    return Err("brush count changed".into());
                }
                let volume_delta = (new_volume / old_volume - 1.).abs();
                println!(
                    "  summed brush volume change: {:.5}%; {simplified} sub-grid details thickened",
                    volume_delta * 100.
                );
                if volume_delta > 0.03 {
                    return Err("asset volume changed by over 3%".into());
                }
                Ok((output, source, count, off, max_move))
            })();
        match result {
            Ok((output, source, count, off, movement)) => {
                println!(
                    "{}: {count} brushes, {off} off grid, max move {movement:.6}",
                    path.display()
                );
                if output != source {
                    outputs.push((path, output));
                }
            }
            Err(e) => {
                eprintln!("FAIL {}: {e}", path.display());
                failed += 1;
            }
        }
    }
    if failed > 0 && !args.iter().any(|a| a == "--keep-failed") {
        return Err(format!("{failed} assets failed; no files written").into());
    }
    println!(
        "{} assets to update; {failed} left unchanged",
        outputs.len()
    );
    if write {
        for (path, output) in outputs {
            std::fs::write(path, output)?;
        }
    }
    Ok(())
}
