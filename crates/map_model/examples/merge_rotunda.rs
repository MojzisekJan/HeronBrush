//! Merge convex neighbours without snapping or deleting detail. Input and output map paths.
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
    if b.faces().iter().any(|f| {
        f.paint().is_some_and(|c| {
            c.alpha() != 255 || !matches!(c, map_model::brush_face::FaceColor::Solid { .. })
        })
    }) {
        return None;
    }
    let mut keys: Vec<_> = b
        .faces()
        .iter()
        .filter(|f| f.material_name.as_str() != "skip")
        .map(|f| {
            format!(
                "{} {:?} {:?} {:?} {:?} {:?} {:?}",
                f.material_name,
                f.color,
                f.surface_material,
                f.uv_attributes,
                f.uv_coord_system,
                f.primitive_uv,
                f.surface_attributes
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
        let matching: Vec<_> = sources
            .iter()
            .flat_map(|b| b.faces())
            .filter(|f| {
                vec::length(f.plane.normal - plane.normal) < 1e-7
                    && (f.plane.distance - plane.distance).abs() < 1e-6
            })
            .collect();
        let fallback = sources[0]
            .faces()
            .iter()
            .find(|f| f.material_name.as_str() != "skip")?;
        let src = matching
            .iter()
            .min_by_key(|f| f.material_name.as_str() == "skip")
            .copied()
            .unwrap_or(fallback);
        let mut f = (*src).clone();
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
    let (mut total, mut merged) = (0, 0);
    for entity in &mut map.entities {
        let mut entries = Vec::new();
        for raw in std::mem::take(&mut entity.brushes) {
            total += 1;
            let b = Brush::from_faces(&world, raw.faces.clone()).unwrap();
            entries.push((raw, b));
        }
        let mut failed = std::collections::HashSet::new();
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
                    let pair = (entries[i].0.id.unwrap(), entries[j].0.id.unwrap());
                    if !failed.insert(pair) {
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
                    if excess > 0.000001 {
                        continue;
                    }
                    println!("MERGE {:?} {:?}", entries[i].0.id, entries[j].0.id);
                    if [a, b].iter().any(|b| {
                        csg::subtract(b, &h).iter().map(Brush::volume).sum::<f64>() > 0.000001
                    }) {
                        continue;
                    }
                    // Preserve every existing exposed plane and its material; do not bridge an opening.

                    found = Some((i, j, h));
                    break 'pairs;
                }
            }
            let Some((i, j, b)) = found else {
                break;
            };
            let changed = entries[i].0.id.unwrap();
            failed.retain(|(a, b)| *a != changed && *b != changed);
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
        assert!(b.volume() > 0.);
        count += 1;
    }
    std::fs::write(&args[1], text).unwrap();
    println!("before={total} merges={merged} after={count}");
}
