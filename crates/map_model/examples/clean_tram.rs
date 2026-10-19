//! Tram cleanup: exact convex subtraction, material-compatible merges and small joint mitres.
//! Mitres may remove at most 0.5 map units near a joint; no volume is added.
//! Run the native-editor SAT audit afterwards because interactive reconstruction uses a larger tolerance.
use geometry_math::{
    bbox::Bbox3d,
    vec::{self, Vec3d},
};
use map_model::{
    brush::Brush as NativeBrush,
    io::{
        map_reader::{read_map, Brush as Raw},
        map_writer::write_map,
    },
    polyhedron::{FaceId, Polyhedron},
};
// Work below the editor's interactive 0.001-unit clipping tolerance: ornament rods
// are small enough that the interactive tolerance would discard real fragments.
use geometry_math::plane::Plane3d;
use map_model::brush_face::BrushFace;
#[derive(Clone)]
struct Brush {
    faces: Vec<BrushFace>,
    poly: Polyhedron<f64>,
}
impl Brush {
    fn from_faces(world: &Bbox3d, faces: Vec<BrushFace>) -> Result<Self, String> {
        let planes: Vec<_> = faces.iter().map(|f| f.plane).collect();
        let mut points = Vec::new();
        for i in 0..planes.len() {
            for j in i + 1..planes.len() {
                for k in j + 1..planes.len() {
                    let (a, b, c) = (planes[i], planes[j], planes[k]);
                    let cross = vec::cross(b.normal, c.normal);
                    let det = vec::dot(a.normal, cross);
                    if det.abs() < 1e-10 {
                        continue;
                    }
                    let p = (cross * a.distance
                        + vec::cross(c.normal, a.normal) * b.distance
                        + vec::cross(a.normal, b.normal) * c.distance)
                        / det;
                    if world.contains_point(p)
                        && planes
                            .iter()
                            .all(|f| vec::dot(f.normal, p) - f.distance < 1e-7)
                    {
                        points.push(p);
                    }
                }
            }
        }
        let poly = Polyhedron::from_points(&points, 1e-7).map_err(|e| format!("{e:?}"))?;
        Ok(Self { faces, poly })
    }
    fn faces(&self) -> &[BrushFace] {
        &self.faces
    }
    fn bounds(&self) -> Bbox3d {
        self.poly.bounds()
    }
    fn vertex_positions(&self) -> Vec<Vec3d> {
        self.poly.vertices().iter().map(|v| v.position).collect()
    }
    fn face_planes(&self) -> Vec<Plane3d> {
        self.poly.faces().iter().map(|f| f.plane).collect()
    }
    fn geometry_faces(&self) -> impl Iterator<Item = (&BrushFace, Vec<Vec3d>)> + '_ {
        (0..self.poly.face_count())
            .map(|i| (&self.faces[0], self.poly.face_positions(FaceId(i as u32))))
    }
    fn volume(&self) -> f64 {
        let mut sum = 0.;
        let origin = self.vertex_positions()[0];
        for (_, p) in self.geometry_faces() {
            for i in 1..p.len() - 1 {
                sum += vec::dot(p[0] - origin, vec::cross(p[i] - origin, p[i + 1] - origin));
            }
        }
        (sum / 6.).abs()
    }
    fn clipped(&self, plane: &Plane3d) -> Option<Self> {
        let vertices = self.vertex_positions();
        let distances: Vec<_> = vertices
            .iter()
            .map(|v| vec::dot(plane.normal, *v) - plane.distance)
            .collect();
        if distances.iter().all(|d| *d <= 1e-8) {
            return Some(self.clone());
        }
        if distances.iter().all(|d| *d >= -1e-8) {
            return None;
        }
        let mut kept: Vec<_> = vertices
            .iter()
            .zip(&distances)
            .filter(|(_, d)| **d <= 1e-8)
            .map(|(v, _)| *v)
            .collect();
        for (_, points) in self.geometry_faces() {
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                let da = vec::dot(plane.normal, a) - plane.distance;
                let db = vec::dot(plane.normal, b) - plane.distance;
                if (da > 1e-8 && db < -1e-8) || (db > 1e-8 && da < -1e-8) {
                    kept.push(a + (b - a) * (da / (da - db)));
                }
            }
        }
        let polyhedron = Polyhedron::from_points(&kept, 1e-8).ok()?;
        let mut faces = Vec::new();
        for i in 0..polyhedron.face_count() {
            let id = FaceId(i as u32);
            let plane = polyhedron.face_plane(id);
            let points = polyhedron.face_positions(id);
            let source = self
                .faces
                .iter()
                .min_by(|a, b| {
                    let error = |f: &BrushFace| {
                        vec::length(f.plane.normal - plane.normal) * 1000.
                            + (f.plane.distance - plane.distance).abs()
                    };
                    error(a).total_cmp(&error(b))
                })
                .unwrap();
            let mut f = source.clone();
            f.points = [points[0], points[1], points[2]];
            f.plane = plane;
            faces.push(f);
        }
        Some(Self {
            faces,
            poly: polyhedron,
        })
    }
}
mod csg {
    use super::*;
    pub fn intersect(a: &Brush, b: &Brush) -> Option<Brush> {
        if !near(a, b) {
            return None;
        }
        let mut r = a.clone();
        for p in b.face_planes() {
            r = r.clipped(&p)?;
        }
        Some(r)
    }
    pub fn subtract(a: &Brush, b: &Brush) -> Vec<Brush> {
        if !near(a, b) {
            return vec![a.clone()];
        }
        let mut remainder = a.clone();
        let mut planes = b.face_planes();
        let mut pieces = Vec::new();
        while !planes.is_empty() {
            // Prefer a large intact outside piece over repeatedly shaving it into slivers.
            let candidate = planes
                .iter()
                .enumerate()
                .filter_map(|(i, p)| remainder.clipped(&p.flip()).map(|q| (i, q)))
                .max_by(|a, b| a.1.volume().total_cmp(&b.1.volume()));
            let Some((i, outside)) = candidate else {
                break;
            };
            let plane = planes.remove(i);
            pieces.push(outside);
            let Some(inside) = remainder.clipped(&plane) else {
                return pieces;
            };
            remainder = inside;
        }
        pieces
    }
    pub fn subtract_all(a: &Brush, tools: &[Brush]) -> Vec<Brush> {
        let mut pieces = vec![a.clone()];
        for tool in tools {
            pieces = pieces.iter().flat_map(|p| subtract(p, tool)).collect();
            if pieces.is_empty() {
                break;
            }
        }
        pieces
    }
}
#[derive(Clone)]
struct Part {
    origin: u64,
    entity: usize,
    raw: Raw,
    b: Brush,
}
fn near(a: &Brush, b: &Brush) -> bool {
    let (a, b) = (a.bounds(), b.bounds());
    (0..3).all(|k| a.max[k] >= b.min[k] - 1e-7 && b.max[k] >= a.min[k] - 1e-7)
}
fn transparent(b: &Brush) -> bool {
    b.faces()
        .iter()
        .any(|f| f.paint().is_some_and(|c| c.alpha() < 255))
}
fn overlap(a: &Brush, b: &Brush) -> f64 {
    if !near(a, b) {
        return 0.;
    }
    // Full separating-axis test, including edge cross products. Contacts are not overlaps.
    let av = a.vertex_positions();
    let bv = b.vertex_positions();
    let separates = |n: Vec3d| {
        let len = vec::length(n);
        if len < 1e-9 {
            return false;
        }
        let n = n / len;
        let range = |v: &Vec<Vec3d>| {
            v.iter()
                .map(|p| vec::dot(*p, n))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), d| {
                    (lo.min(d), hi.max(d))
                })
        };
        let (x, y) = (range(&av), range(&bv));
        x.1.min(y.1) - x.0.max(y.0) < 1e-7
    };
    if a.face_planes()
        .iter()
        .chain(b.face_planes().iter())
        .any(|p| separates(p.normal))
    {
        return 0.;
    }
    let edges = |b: &Brush| {
        let mut e = Vec::new();
        for (_, p) in b.geometry_faces() {
            for i in 0..p.len() {
                e.push(p[(i + 1) % p.len()] - p[i]);
            }
        }
        e
    };
    for x in edges(a) {
        for y in edges(b) {
            if separates(vec::cross(x, y)) {
                return 0.;
            }
        }
    }
    csg::intersect(a, b).map_or(0., |b| b.volume())
}
fn signature(b: &Brush) -> Option<String> {
    let mut keys: Vec<_> = b
        .faces()
        .iter()
        .map(|f| {
            if f.color.is_some() {
                format!("{:?} {:?}", f.color, f.surface_material)
            } else {
                format!(
                    "{} {:?} {:?} {:?} {:?}",
                    f.material_name,
                    f.surface_material,
                    f.uv_attributes,
                    f.uv_coord_system,
                    f.primitive_uv
                )
            }
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
fn hull(a: &Brush, b: &Brush, world: &Bbox3d) -> Option<Brush> {
    let mut points = a.vertex_positions();
    points.extend(b.vertex_positions());
    let h = Polyhedron::from_points(&points, 1e-8).ok()?;
    let mut faces = Vec::new();
    for i in 0..h.face_count() {
        let id = FaceId(i as u32);
        let p = h.face_positions(id);
        let plane = h.face_plane(id);
        let src = a.faces().iter().chain(b.faces()).find(|f| {
            vec::length(f.plane.normal - plane.normal) < 1e-7
                && (f.plane.distance - plane.distance).abs() < 1e-6
        })?;
        let mut f = src.clone();
        f.points = [p[0], p[1], p[2]];
        f.plane = plane;
        f.export_hidden = false;
        faces.push(f);
    }
    let h = Brush::from_faces(world, faces).ok()?;
    let excess = csg::subtract_all(&h, &[a.clone(), b.clone()])
        .iter()
        .map(Brush::volume)
        .sum::<f64>();
    if excess > 1e-6 {
        return None;
    }
    if [a, b]
        .iter()
        .any(|b| csg::subtract(b, &h).iter().map(Brush::volume).sum::<f64>() > 1e-6)
    {
        return None;
    }
    Some(h)
}
fn merges(parts: &mut Vec<Part>, world: &Bbox3d) -> usize {
    let mut n = 0;
    loop {
        let mut found = None;
        'outer: for i in 0..parts.len() {
            let Some(key) = signature(&parts[i].b) else {
                continue;
            };
            for j in i + 1..parts.len() {
                if parts[i].entity != parts[j].entity
                    || !near(&parts[i].b, &parts[j].b)
                    || signature(&parts[j].b).as_ref() != Some(&key)
                {
                    continue;
                }
                if let Some(b) = hull(&parts[i].b, &parts[j].b, world) {
                    found = Some((i, j, b));
                    break 'outer;
                }
            }
        }
        let Some((i, j, b)) = found else { break };
        parts[i].b = b;
        parts.remove(j);
        n += 1;
    }
    n
}
fn miter(a: &Brush, b: &Brush, shared: f64, world: &Bbox3d) -> Option<(Brush, Brush, f64)> {
    if transparent(a) || transparent(b) {
        return None;
    }
    let expected = a.volume() + b.volume() - shared;
    let mut planes = a.face_planes();
    planes.extend(b.face_planes());
    let ab = a.bounds();
    let bb = b.bounds();
    let center = (ab.min + ab.max + bb.min + bb.max) / 4.;
    let direction = (bb.min + bb.max - ab.min - ab.max) / 2.;
    let len = vec::length(direction);
    if len > 1e-8 {
        let n = direction / len;
        planes.push(Plane3d {
            normal: n,
            distance: vec::dot(n, center),
        });
    }
    for p in a.face_planes() {
        for q in b.face_planes() {
            let n = p.normal - q.normal;
            let len = vec::length(n);
            if len > 1e-8 {
                planes.push(Plane3d {
                    normal: n / len,
                    distance: (p.distance - q.distance) / len,
                });
            }
        }
    }
    let mut best = None;
    let mut score = (usize::MAX, f64::INFINITY);
    for p in planes {
        for p in [p, p.flip()] {
            let Some(aa) = a.clipped(&p) else { continue };
            let Some(bbb) = b.clipped(&p.flip()) else {
                continue;
            };
            let missing = expected - aa.volume() - bbb.volume();
            if missing < -1e-6 || missing > 20.0 || missing > a.volume().min(b.volume()) * 0.2 {
                continue;
            }
            if [&aa, &bbb]
                .iter()
                .any(|b| NativeBrush::from_faces(world, b.faces.clone()).is_err())
            {
                continue;
            }
            if missing > 1e-6 {
                let removed = csg::subtract_all(a, &[aa.clone(), bbb.clone()])
                    .into_iter()
                    .chain(csg::subtract_all(b, &[aa.clone(), bbb.clone()]));
                let close = removed
                    .flat_map(|fragment| fragment.vertex_positions())
                    .all(|p| {
                        let distance = |b: &Brush| {
                            b.face_planes()
                                .iter()
                                .map(|f| vec::dot(f.normal, p) - f.distance)
                                .fold(0., f64::max)
                        };
                        distance(&aa).min(distance(&bbb)) <= 0.5
                    });
                if !close {
                    continue;
                }
            }
            let quality = (aa.faces.len() + bbb.faces.len(), missing);
            if quality < score {
                best = Some((aa, bbb, missing));
                score = quality;
            }
        }
    }
    best
}
fn merge_families(parts: &mut Vec<Part>, world: &Bbox3d) -> usize {
    let origins: std::collections::BTreeSet<_> = parts.iter().map(|p| p.origin).collect();
    let mut merged = 0;
    for origin in origins {
        let indices: Vec<_> = (0..parts.len())
            .filter(|i| parts[*i].origin == origin)
            .collect();
        if indices.len() < 3 {
            continue;
        }
        let Some(key) = signature(&parts[indices[0]].b) else {
            continue;
        };
        if indices
            .iter()
            .any(|i| signature(&parts[*i].b).as_ref() != Some(&key))
        {
            continue;
        }
        let points: Vec<_> = indices
            .iter()
            .flat_map(|i| parts[*i].b.vertex_positions())
            .collect();
        let Ok(poly) = Polyhedron::from_points(&points, 1e-7) else {
            continue;
        };
        let mut faces = Vec::new();
        for k in 0..poly.face_count() {
            let id = FaceId(k as u32);
            let plane = poly.face_plane(id);
            let p = poly.face_positions(id);
            let Some(src) = indices.iter().flat_map(|i| parts[*i].b.faces()).find(|f| {
                vec::length(f.plane.normal - plane.normal) < 1e-7
                    && (f.plane.distance - plane.distance).abs() < 1e-6
            }) else {
                break;
            };
            let mut f = src.clone();
            f.points = [p[0], p[1], p[2]];
            f.plane = plane;
            f.export_hidden = false;
            faces.push(f);
        }
        if faces.len() != poly.face_count() {
            continue;
        }
        let hull = Brush { faces, poly };
        let volume: f64 = indices.iter().map(|i| parts[*i].b.volume()).sum();
        if (hull.volume() - volume).abs() > 1e-5
            || NativeBrush::from_faces(world, hull.faces.clone()).is_err()
        {
            continue;
        }
        parts[indices[0]].b = hull;
        for i in indices.iter().skip(1).rev() {
            parts.remove(*i);
            merged += 1;
        }
    }
    merged
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(args.len() == 1 || args.len() == 2);
    let source = std::fs::read_to_string(&args[0]).unwrap();
    let mut map = read_map(&source, None).unwrap();
    assert!(map.warnings.is_empty());
    let world = Bbox3d::from_scalars(-16384., 16384.);
    let mut parts = Vec::new();
    for (ei, e) in map.entities.iter().enumerate() {
        for raw in &e.brushes {
            parts.push(Part {
                entity: ei,
                origin: raw.id.unwrap(),
                raw: raw.clone(),
                b: Brush::from_faces(&world, raw.faces.clone()).unwrap(),
            });
        }
    }
    let original = parts.clone();
    let mut before = 0;
    for i in 0..parts.len() {
        for j in i + 1..parts.len() {
            let v = overlap(&parts[i].b, &parts[j].b);
            if v > 1e-7 {
                before += 1;
                println!(
                    "OVERLAP {i} {j} entities={} {} volume={v:.9} bounds={:?} {:?}",
                    parts[i].entity,
                    parts[j].entity,
                    parts[i].b.bounds(),
                    parts[j].b.bounds()
                );
            }
        }
    }
    println!("BEFORE brushes={} pairs={before}", parts.len());
    if args.len() == 1 {
        return;
    }
    assert!(!std::path::Path::new(&args[1]).exists());
    let mut merged = merges(&mut parts, &world);
    println!("PREMERGE {merged} remaining={}", parts.len());
    let mut miters = 0;
    let mut miter_loss = 0.;
    let mut cuts = 0;
    let mut next_id = parts.iter().filter_map(|p| p.raw.id).max().unwrap() + 1;
    'work: loop {
        let mut found = None;
        'outer: for i in 0..parts.len() {
            for j in i + 1..parts.len() {
                let (a, b) = (&parts[i].b, &parts[j].b);
                let v = overlap(a, b);
                if v < 1e-7 {
                    continue;
                }
                let pa = csg::subtract(a, b);
                let pb = csg::subtract(b, a);
                let valid = |pieces: &Vec<Brush>, target: &Brush| {
                    (pieces.iter().map(Brush::volume).sum::<f64>() + v - target.volume()).abs()
                        < 1e-5
                        && pieces.iter().all(|p| p.volume() > 1e-9)
                };
                if pa.len().min(pb.len()) > 1 {
                    if let Some((aa, bb, loss)) = miter(a, b, v, &world) {
                        parts[i].b = aa;
                        parts[j].b = bb;
                        miters += 1;
                        miter_loss += loss;
                        continue 'work;
                    }
                }
                let va = valid(&pa, a) && (!transparent(b) || transparent(a));
                let vb = valid(&pb, b) && (!transparent(a) || transparent(b));
                let cost =
                    |p: &Vec<Brush>| (p.len(), p.iter().map(|b| b.faces().len()).sum::<usize>());
                let (target, tool, pieces) = if va && (!vb || cost(&pa) <= cost(&pb)) {
                    (i, j, pa)
                } else if vb {
                    (j, i, pb)
                } else {
                    panic!("Cannot safely cut {i} {j} ids={:?} {:?} volume={v} av={} bv={} pa={:?} pb={:?} bounds={:?} {:?}",parts[i].raw.id,parts[j].raw.id,a.volume(),b.volume(),pa.iter().map(Brush::volume).collect::<Vec<_>>(),pb.iter().map(Brush::volume).collect::<Vec<_>>(),a.bounds(),b.bounds());
                };
                found = Some((target, tool, pieces));
                break 'outer;
            }
        }
        let Some((target, tool, pieces)) = found else {
            break;
        };
        let src = parts[target].clone();
        let cutter = parts[tool].b.clone();
        parts.remove(target);
        for (k, b) in pieces.into_iter().enumerate() {
            let mut raw = src.raw.clone();
            if k > 0 {
                raw.id = Some(next_id);
                next_id += 1;
            }
            let mut faces = b.faces().to_vec();
            for f in &mut faces {
                if !src.b.faces().iter().any(|old| {
                    vec::length(old.plane.normal - f.plane.normal) < 1e-7
                        && (old.plane.distance - f.plane.distance).abs() < 1e-6
                }) {
                    f.export_hidden = !transparent(&cutter);
                }
            }
            let b = Brush::from_faces(&world, faces).unwrap();
            parts.push(Part {
                entity: src.entity,
                origin: src.origin,
                raw,
                b,
            });
        }
        cuts += 1;
        if cuts % 50 == 0 {
            println!("CUTS {cuts} brushes={}", parts.len());
        }
        assert!(cuts < 10000);
    }
    let families = merge_families(&mut parts, &world);
    println!("FAMILY MERGES {families}");
    merged += families;
    merged += merges(&mut parts, &world);
    let mut sliver_loss = 0.;
    parts.retain(|p| {
        if NativeBrush::from_faces(&world, p.b.faces.clone()).is_err() {
            let volume = p.b.volume();
            assert!(volume < 0.0001, "large invalid piece {volume}");
            sliver_loss += volume;
            false
        } else {
            true
        }
    });
    miter_loss += sliver_loss;
    println!("SLIVERS volume={sliver_loss}");
    println!(
        "FINALPARTS {} merges={merged} miters={miters} miter_loss={miter_loss}",
        parts.len()
    );
    let mut after = 0;
    for i in 0..parts.len() {
        for j in i + 1..parts.len() {
            if overlap(&parts[i].b, &parts[j].b) > 1e-7 {
                after += 1;
            }
        }
    }
    assert_eq!(after, 0);
    // Verify both directions of the geometric union with native convex subtraction.
    for (label, from, to) in [("missing", &original, &parts), ("added", &parts, &original)] {
        let mut total = 0.;
        for p in from {
            let tools: Vec<_> = to
                .iter()
                .filter(|q| near(&p.b, &q.b))
                .map(|q| q.b.clone())
                .collect();
            total += csg::subtract_all(&p.b, &tools)
                .iter()
                .map(Brush::volume)
                .sum::<f64>();
        }
        println!("UNION {label} volume={total:.12}");
        assert!(
            total
                < if label == "missing" {
                    miter_loss * 4. + 0.0001
                } else {
                    0.0001
                }
        );
    }
    for e in &mut map.entities {
        e.brushes.clear();
    }
    for p in &parts {
        let mut raw = p.raw.clone();
        raw.faces = p.b.faces().to_vec();
        map.entities[p.entity].brushes.push(raw);
    }
    let mut text = String::new();
    for l in source.lines().take_while(|l| !l.starts_with("// Format:")) {
        text.push_str(l);
        text.push('\n');
    }
    text.push_str(&write_map(&map));
    let check = read_map(&text, None).unwrap();
    for raw in check.entities.iter().flat_map(|e| &e.brushes) {
        assert!(NativeBrush::from_faces(&world, raw.faces.clone()).is_ok());
    }
    std::fs::write(&args[1], text).unwrap();
    println!(
        "AFTER brushes={} pairs={after} cuts={cuts} merges={merged}",
        parts.len()
    );
}
