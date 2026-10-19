//! Round asset face coordinates without independently moving shared face planes.
//! Usage: cargo run -p map_model --example round_asset_coordinates -- assets [--write]
use map_model::{brush::Brush, io::map_reader::read_map};
use std::{collections::HashMap, path::PathBuf};
use geometry_math::{
    bbox::Bbox3d,
    vec::{self, Vec3d},
};
const CONTACT: f64 = 1e-6;
const MAX_MOVE: f64 = 0.002;
#[derive(Clone)]
struct PlaneGroup {
    normal: Vec3d,
    distance: f64,
    points: [Vec3d; 3],
}
struct Source {
    text: String,
    groups: Vec<PlaneGroup>,
    faces: Vec<(usize, bool)>,
    brushes: Vec<Brush>,
    vertices: Vec<Vec<Vec3d>>,
    contacts: Vec<Vec<(usize, usize)>>,
}
fn points(brush: &Brush) -> Vec<Vec3d> {
    brush
        .geometry()
        .vertices()
        .iter()
        .map(|v| v.position)
        .collect()
}
fn geometry(text: &str) -> Result<Vec<Brush>, String> {
    let map = read_map(text, None).map_err(|e| e.to_string())?;
    if !map.warnings.is_empty() {
        return Err(format!("parser reported {} warnings", map.warnings.len()));
    }
    if map.entities.iter().any(|e| !e.patches.is_empty()) {
        return Err("patches need a separate rounding policy".into());
    }
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    map.entities
        .into_iter()
        .flat_map(|e| e.brushes)
        .map(|b| Brush::from_faces(&world, b.faces).map_err(|e| e.to_string()))
        .collect()
}
fn root(parents: &mut [usize], n: usize) -> usize {
    if parents[n] != n {
        parents[n] = root(parents, parents[n]);
    }
    parents[n]
}
impl Source {
    fn read(text: String) -> Result<Self, String> {
        let brushes = geometry(&text)?;
        let vertices = brushes.iter().map(points).collect::<Vec<_>>();
        let map = read_map(&text, None).map_err(|e| e.to_string())?;
        let mut groups: Vec<PlaneGroup> = Vec::new();
        let mut faces = Vec::new();
        for face in map
            .entities
            .iter()
            .flat_map(|e| &e.brushes)
            .flat_map(|b| &b.faces)
        {
            let mut n = face.plane.normal;
            let mut d = face.plane.distance;
            let axis = (0..3)
                .max_by(|&a, &b| n[a].abs().total_cmp(&n[b].abs()))
                .unwrap();
            let reversed = n[axis] < 0.0;
            let mut p = face.points;
            if reversed {
                n = -n;
                d = -d;
                p.swap(1, 2);
            }
            let group = groups
                .iter()
                .position(|g| vec::length(g.normal - n) < 1e-12 && (g.distance - d).abs() < 1e-8)
                .unwrap_or_else(|| {
                    groups.push(PlaneGroup {
                        normal: n,
                        distance: d,
                        points: p,
                    });
                    groups.len() - 1
                });
            faces.push((group, reversed));
        }
        let flat = vertices
            .iter()
            .enumerate()
            .flat_map(|(b, vs)| vs.iter().enumerate().map(move |(v, p)| (b, v, *p)))
            .collect::<Vec<_>>();
        let mut parents = (0..flat.len()).collect::<Vec<_>>();
        let mut buckets: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
        for (index, &(b, _, p)) in flat.iter().enumerate() {
            let key = [
                (p.x() / CONTACT).floor() as i64,
                (p.y() / CONTACT).floor() as i64,
                (p.z() / CONTACT).floor() as i64,
            ];
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        if let Some(found) = buckets.get(&[key[0] + x, key[1] + y, key[2] + z]) {
                            for &other in found {
                                if flat[other].0 != b && vec::length(flat[other].2 - p) <= CONTACT {
                                    let a = root(&mut parents, index);
                                    let c = root(&mut parents, other);
                                    parents[a] = c;
                                }
                            }
                        }
                    }
                }
            }
            buckets.entry(key).or_default().push(index);
        }
        let mut clusters: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        for (index, &(b, v, _)) in flat.iter().enumerate() {
            let key = root(&mut parents, index);
            clusters.entry(key).or_default().push((b, v));
        }
        let contacts = clusters.into_values().filter(|c| c.len() > 1).collect();
        Ok(Self {
            text,
            groups,
            faces,
            brushes,
            vertices,
            contacts,
        })
    }
    fn keep_independent_planes(&mut self) -> Result<(), String> {
        let map = read_map(&self.text, None).map_err(|e| e.to_string())?;
        self.groups.clear();
        self.faces.clear();
        for face in map
            .entities
            .iter()
            .flat_map(|e| &e.brushes)
            .flat_map(|b| &b.faces)
        {
            self.faces.push((self.groups.len(), false));
            self.groups.push(PlaneGroup {
                normal: face.plane.normal,
                distance: face.plane.distance,
                points: face.points,
            });
        }
        Ok(())
    }
    fn rounded(&self, precision: usize) -> Result<String, String> {
        let scalar = |v: f64| {
            let mut s = format!("{v:.precision$}");
            if s.contains('.') {
                s = s.trim_end_matches('0').trim_end_matches('.').to_string();
            }
            if s == "-0" {
                s = "0".into();
            }
            s
        };
        let triplets = self
            .groups
            .iter()
            .map(|g| {
                g.points
                    .map(|p| format!("( {} {} {} )", scalar(p.x()), scalar(p.y()), scalar(p.z())))
            })
            .collect::<Vec<_>>();
        let mut face = 0;
        let mut out = String::with_capacity(self.text.len());
        for line in self.text.split_inclusive('\n') {
            if line.trim_start().starts_with('(') {
                let Some(&(group, reversed)) = self.faces.get(face) else {
                    return Err("face line count mismatch".into());
                };
                let ends = line.match_indices(')').take(3).collect::<Vec<_>>();
                if ends.len() != 3 {
                    return Err("unsupported face syntax".into());
                }
                let p = &triplets[group];
                let order = if reversed { [0, 2, 1] } else { [0, 1, 2] };
                out.push_str(&line[..line.len() - line.trim_start().len()]);
                out.push_str(&format!("{} {} {}", p[order[0]], p[order[1]], p[order[2]]));
                out.push_str(&line[ends[2].0 + 1..]);
                face += 1;
            } else {
                out.push_str(line);
            }
        }
        if face != self.faces.len() {
            return Err("not all parsed faces matched source lines".into());
        }
        Ok(out)
    }
    fn validate(&self, text: &str) -> Result<f64, String> {
        let candidate = geometry(text)?;
        if candidate.len() != self.brushes.len() {
            return Err("brush count changed".into());
        }
        let mut mapped = Vec::new();
        let mut max_move = 0.0_f64;
        for (b, (old, new)) in self.brushes.iter().zip(&candidate).enumerate() {
            let new_points = points(new);
            let old_points = &self.vertices[b];
            if new.geometry().face_count() != old.geometry().face_count()
                || new_points.len() != old_points.len()
            {
                return Err(format!("brush {b}: topology changed"));
            }
            if (new.volume() - old.volume()).abs() / old.volume().max(1e-12) > 1e-4 {
                return Err(format!("brush {b}: volume drift"));
            }
            let mut used = vec![false; new_points.len()];
            let mut matched = Vec::new();
            for &p in old_points {
                let (i, d) = new_points
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !used[*i])
                    .map(|(i, q)| (i, vec::length(p - *q)))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .ok_or("vertex matching failed")?;
                if d > MAX_MOVE {
                    return Err(format!("brush {b}: vertex drift {d}"));
                }
                used[i] = true;
                max_move = max_move.max(d);
                matched.push(new_points[i]);
            }
            mapped.push(matched);
        }
        for cluster in &self.contacts {
            let (b, v) = cluster[0];
            for &(c, w) in &cluster[1..] {
                let before = vec::length(self.vertices[b][v] - self.vertices[c][w]);
                let after = vec::length(mapped[b][v] - mapped[c][w]);
                if after > before + CONTACT {
                    return Err(format!(
                        "contact opened between brushes {b} and {c}: {after}"
                    ));
                }
            }
        }
        Ok(max_move)
    }
}
fn main() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let write = args.iter().any(|a| a == "--write");
    let path = PathBuf::from(
        args.iter()
            .find(|a| !a.starts_with('-'))
            .ok_or("give an asset folder or .map file")?,
    );
    let mut paths = if path.is_dir() {
        std::fs::read_dir(&path)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "map"))
            .collect::<Vec<_>>()
    } else {
        vec![path]
    };
    paths.sort();
    let mut changes = Vec::new();
    let mut before_bytes = 0;
    let mut after_bytes = 0;
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut source = Source::read(text).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut accepted = None;
        let mut reason = String::new();
        for precision in 3..=10 {
            let candidate = source.rounded(precision)?;
            match source.validate(&candidate) {
                Ok(displacement) => {
                    accepted = Some((candidate, precision, displacement));
                    break;
                }
                Err(e) => reason = e,
            }
        }
        if accepted.is_none() {
            source.keep_independent_planes()?;
            for precision in 3..=12 {
                let candidate = source.rounded(precision)?;
                match source.validate(&candidate) {
                    Ok(displacement) => {
                        accepted = Some((candidate, precision, displacement));
                        break;
                    }
                    Err(e) => reason = e,
                }
            }
        }
        let Some((candidate, precision, displacement)) = accepted else {
            eprintln!("{}: left unchanged; {reason}", path.display());
            before_bytes += source.text.len();
            after_bytes += source.text.len();
            continue;
        };
        if candidate.len() >= source.text.len() {
            println!(
                "{}: unchanged (no shorter safe representation)",
                path.file_name().unwrap().to_string_lossy()
            );
            before_bytes += source.text.len();
            after_bytes += source.text.len();
            continue;
        }
        before_bytes += source.text.len();
        after_bytes += candidate.len();
        println!("{}: {precision} decimals, {} brushes, {} contact clusters, max displacement {displacement:.9}, {} bytes saved{}",path.file_name().unwrap().to_string_lossy(),source.brushes.len(),source.contacts.len(),source.text.len() as i64-candidate.len() as i64,if candidate==source.text{" (unchanged)"}else{""});
        if candidate != source.text {
            changes.push((path, candidate));
        }
    }
    println!(
        "{} files change, {} -> {} bytes. {}",
        changes.len(),
        before_bytes,
        after_bytes,
        if write {
            "Writing validated assets."
        } else {
            "Dry run; no files changed."
        }
    );
    // No asset is touched until every input has passed validation.
    if write {
        for (path, text) in changes {
            std::fs::write(path, text).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
