//! Grid-four tower variant: snap, drop collapsed details, merge convex neighbours.
use geometry_math::{
    bbox::Bbox3d,
    vec::{self, Vec3d},
};
use map_model::{
    brush::Brush,
    csg,
    io::{map_reader::read_map, map_writer::write_map},
    polyhedron::{FaceId, Polyhedron},
};
fn signature(b: &Brush) -> Option<String> {
    let mut keys: Vec<_> = b
        .faces()
        .iter()
        .filter(|f| f.material_name.as_str() != "skip")
        .map(|f| {
            format!(
                "{} {:?} {:?} {:?}",
                f.material_name, f.color, f.surface_material, f.uv_attributes
            )
        })
        .collect();
    keys.sort();
    keys.dedup();
    if keys.len() == 1 {
        keys.pop()
    } else {
        None
    }
}
fn hull(points: &[Vec3d], sources: &[&Brush], world: &Bbox3d) -> Option<Brush> {
    let h = Polyhedron::from_points(points, 1e-7).ok()?;
    let mut faces = Vec::new();
    for i in 0..h.face_count() {
        let id = FaceId(i as u32);
        let p = h.face_positions(id);
        let plane = h.face_plane(id);
        let src = sources.iter().flat_map(|b| b.faces()).max_by(|a, b| {
            let score = |f: &map_model::brush_face::BrushFace| {
                vec::dot(f.plane.normal, plane.normal) * 1000.
                    - (f.plane.distance - plane.distance).abs()
                    + if f.material_name.as_str() == "skip" {
                        0.
                    } else {
                        0.001
                    }
            };
            score(a).total_cmp(&score(b))
        })?;
        let mut f = src.clone();
        // Snapping invalidates the old occlusion classification. Recompute skip after geometry edits.
        if f.material_name.as_str() == "skip" {
            if let Some(visible) = sources
                .iter()
                .flat_map(|b| b.faces())
                .filter(|f| f.material_name.as_str() != "skip")
                .max_by(|a, b| {
                    vec::dot(a.plane.normal, plane.normal)
                        .total_cmp(&vec::dot(b.plane.normal, plane.normal))
                })
            {
                f = visible.clone();
            }
        }
        f.points = [p[0], p[1], p[2]];
        f.plane = plane;
        faces.push(f);
    }
    Brush::from_faces(world, faces).ok()
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2);
    assert!(!std::path::Path::new(&args[1]).exists());
    let source = std::fs::read_to_string(&args[0]).unwrap();
    let mut map = read_map(&source, None).unwrap();
    let world = Bbox3d::from_scalars(-16384., 16384.);
    let (mut total, mut collapsed, mut merged) = (0, 0, 0);
    for entity in &mut map.entities {
        let mut entries = Vec::new();
        for mut raw in std::mem::take(&mut entity.brushes) {
            total += 1;
            let b = Brush::from_faces(&world, raw.faces.clone()).unwrap();
            let points: Vec<_> = b
                .vertex_positions()
                .iter()
                .map(|p| {
                    Vec3d::new(
                        ((p.x() / 4.) + 1e-8).round() * 4.,
                        ((p.y() / 4.) + 1e-8).round() * 4.,
                        ((p.z() / 4.) + 1e-8).round() * 4.,
                    )
                })
                .collect();
            let Some(snapped) = hull(&points, &[&b], &world) else {
                collapsed += 1;
                continue;
            };
            raw.faces = snapped.faces().to_vec();
            entries.push((raw, snapped));
        }
        loop {
            let mut found = None;
            'pairs: for i in 0..entries.len() {
                let Some(key) = signature(&entries[i].1) else {
                    continue;
                };
                for j in i + 1..entries.len() {
                    let (a, b) = (&entries[i].1, &entries[j].1);
                    let (ba, bb) = (a.bounds(), b.bounds());
                    if (0..3).any(|k| ba.max[k] < bb.min[k] - 1e-5 || bb.max[k] < ba.min[k] - 1e-5)
                    {
                        continue;
                    }
                    if signature(b).as_ref() != Some(&key) {
                        continue;
                    }
                    let mut points = a.vertex_positions();
                    points.extend(b.vertex_positions());
                    let Some(h) = hull(&points, &[a, b], &world) else {
                        continue;
                    };
                    let excess: f64 = csg::subtract_all(&h, &[a.clone(), b.clone()])
                        .iter()
                        .map(Brush::volume)
                        .sum();
                    if excess > 0.0001 {
                        continue;
                    }
                    // Preserve every existing exposed plane and its material; do not bridge an opening.
                    if h.faces().iter().any(|f| {
                        ![a, b].iter().flat_map(|b| b.faces()).any(|old| {
                            vec::length(old.plane.normal - f.plane.normal) < 1e-6
                                && (old.plane.distance - f.plane.distance).abs() < 1e-5
                        })
                    }) {
                        continue;
                    }
                    found = Some((i, j, h));
                    break 'pairs;
                }
            }
            let Some((i, j, b)) = found else {
                break;
            };
            entries[i].0.faces = b.faces().to_vec();
            entries[i].1 = b;
            entries.remove(j);
            merged += 1;
            if merged % 100 == 0 {
                eprintln!("merged {merged}");
            }
        }
        entity.brushes = entries.into_iter().map(|(r, _)| r).collect();
    }
    let mut text = String::new();
    for l in source.lines().take_while(|l| !l.starts_with("// Format:")) {
        text.push_str(l);
        text.push('\n');
    }
    text.push_str(&write_map(&map));
    let check = read_map(&text, None).unwrap();
    let mut count = 0;
    for raw in check.entities.iter().flat_map(|e| &e.brushes) {
        let b = Brush::from_faces(&world, raw.faces.clone()).unwrap();
        assert!(b.vertices_are_on_grid(4.));
        count += 1;
    }
    std::fs::write(&args[1], text).unwrap();
    println!("before={total} collapsed={collapsed} merges={merged} after={count}");
}
