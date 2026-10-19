//! Read-only solid intersection and corner-join audit using reconstructed geometry.
use geometry_math::{bbox::Bbox3d, vec::{self, Vec3d}};
use map_model::{brush::Brush, csg, io::map_reader::read_map};
use serde_json::{json, Value};
use std::path::PathBuf;

// Use exactly the editor's corner rules, rather than another approximation.
#[path = "../../editor_core/src/corner_join.rs"]
#[allow(dead_code)]
mod corner_join;

fn overlap(a: &Brush, b: &Brush) -> bool {
    let (aa, bb) = (a.bounds(), b.bounds());
    if (0..3).any(|i| aa.max[i].min(bb.max[i]) - aa.min[i].max(bb.min[i]) <= 1e-5) { return false; }
    let av = a.vertex_positions();
    let bv = b.vertex_positions();
    let separates = |axis: Vec3d| {
        let length = vec::length(axis);
        if length < 1e-10 { return false; }
        let axis = axis / length;
        let project = |points: &[Vec3d]| points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            let d = vec::dot(*p, axis); (lo.min(d), hi.max(d))
        });
        let (al, ah) = project(&av); let (bl, bh) = project(&bv);
        ah.min(bh) - al.max(bl) <= 1e-5
    };
    if a.face_planes().into_iter().chain(b.face_planes()).any(|p| separates(p.normal)) { return false; }
    let ae = a.edge_positions(); let be = b.edge_positions();
    !ae.iter().any(|(a0,a1)| be.iter().any(|(b0,b1)| separates(vec::cross(*a1-*a0, *b1-*b0))))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "assets".into()));
    let mut paths = if root.is_dir() { std::fs::read_dir(&root)?.map(|p| p.map(|p| p.path())).collect::<Result<Vec<_>, _>>()? } else { vec![root] };
    paths.retain(|p| p.extension().is_some_and(|e| e == "map")); paths.sort();
    let bounds = Bbox3d::from_scalars(-16384.0, 16384.0);
    let mut reports = Vec::new();
    for path in paths {
        let map = read_map(&std::fs::read_to_string(&path)?, None)?;
        assert!(map.warnings.is_empty(), "{}: {:?}", path.display(), map.warnings);
        let mut entries = Vec::new();
        for (entity, e) in map.entities.iter().enumerate() {
            for (index, raw) in e.brushes.iter().enumerate() {
                entries.push((entity, index, raw.line, Brush::from_faces(&bounds, raw.faces.clone())?));
            }
        }
        let describe = |i: usize| {
            let (entity, index, line, b) = &entries[i];
            json!({"entity": entity, "brush": index, "line": line, "min": b.bounds().min.v, "max": b.bounds().max.v,
                "paint": format!("{:?}", b.faces().iter().find(|f| f.material_name.as_str() != "skip").and_then(|f| f.paint()))})
        };
        let mut pairs: Vec<Value> = Vec::new();
        for i in 0..entries.len() {
            for j in i+1..entries.len() {
                let a = &entries[i].3; let b = &entries[j].3;
                if !overlap(a,b) { continue; }
                let same_entity = entries[i].0 == entries[j].0;
                let join = if same_entity { corner_join::preview(a,b).ok() } else { None };
                pairs.push(json!({"a": describe(i), "b": describe(j), "same_entity": same_entity,
                    "volume": csg::intersect(a,b).map(|v|v.volume()),
                    "corner": join.map(|join| json!({"normal": join.plane.normal.v, "distance": join.plane.distance})),
                    "contained": a.vertex_positions().iter().all(|p| b.contains_point(*p)) || b.vertex_positions().iter().all(|p| a.contains_point(*p))}));
            }
        }
        eprintln!("{}: {} brushes, {} solid overlaps, {} join candidates", path.display(), entries.len(), pairs.len(), pairs.iter().filter(|p|!p["corner"].is_null()).count());
        reports.push(json!({"asset": path.file_name().unwrap().to_string_lossy(), "brushes": entries.len(), "overlaps": pairs}));
    }
    println!("{}", serde_json::to_string_pretty(&json!({"tolerance": 1e-5, "assets": reports}))?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cuboid(min: Vec3d, max: Vec3d) -> Brush {
        map_model::shapes::create_cuboid(&Bbox3d::from_scalars(-16384.0,16384.0), &Bbox3d{min,max}, "stone").unwrap()
    }
    #[test]
    fn face_edge_contacts_and_small_gaps_are_not_solid_overlaps() {
        let a = cuboid(Vec3d::zero(),Vec3d::new(16.0,16.0,16.0));
        for start in [Vec3d::new(16.0,0.0,0.0),Vec3d::new(16.0,16.0,0.0),Vec3d::new(16.001,0.0,0.0)] {
            assert!(!overlap(&a,&cuboid(start,start+Vec3d::new(16.0,16.0,16.0))));
        }
        assert!(overlap(&a,&cuboid(Vec3d::new(15.99,0.0,0.0),Vec3d::new(32.0,16.0,16.0))));
    }
    #[test]
    fn overlapping_bounds_do_not_imply_intersecting_sloped_solids() {
        let a = map_model::shapes::create_cylinder(&Bbox3d::from_scalars(-16384.0,16384.0),
            &Bbox3d::from_scalars(-1.0,1.0),map_model::shapes::CircleShape::VertexAligned{num_sides:8}, geometry_math::util::Axis::Z,"stone").unwrap();
        let b = cuboid(Vec3d::new(0.9,0.9,-0.5),Vec3d::new(1.1,1.1,0.5));
        assert!(a.bounds().intersects(&b.bounds()));
        assert!(!overlap(&a,&b));
    }
    #[test]
    fn snack_machine_has_no_solid_intersections_after_the_frame_repair() {
        let map = read_map(include_str!("../../../assets/vending_snacks.map"),None).unwrap();
        let bounds = Bbox3d::from_scalars(-16384.0,16384.0);
        let brushes:Vec<_> = map.entities.iter().flat_map(|e|&e.brushes).map(|b|Brush::from_faces(&bounds,b.faces.clone()).unwrap()).collect();
        assert_eq!(brushes.len(),39);
        for i in 0..brushes.len() { for j in i+1..brushes.len() { assert!(!overlap(&brushes[i],&brushes[j]),"brushes {i},{j}"); } }
    }
}
