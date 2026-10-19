//! Apply the editor's sharp corner cuts to an asset copy, validating union coverage.
use geometry_math::{bbox::Bbox3d, plane::Plane3d, vec::{self, Vec3d}};
use map_model::{brush::{Brush, ClipOutcome}, csg, io::{map_reader::read_map, map_writer::write_map}};
use serde_json::json;
use std::{collections::BTreeSet, path::Path};
#[path = "../../editor_core/src/corner_join.rs"]
#[allow(dead_code)]
mod corner_join;

fn clip(brush: &Brush, plane: &Plane3d) -> Brush {
    match brush.clip(plane) {
        ClipOutcome::Clipped(result) => result,
        ClipOutcome::Unchanged => brush.clone(),
        ClipOutcome::Empty => panic!("repair would remove a brush"),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() == 3 || args.len() == 4, "input.map output.map [clip JSON]");
    let output = Path::new(&args[2]);
    assert!(!output.exists(), "output already exists");
    let source = std::fs::read_to_string(&args[1])?;
    let mut map = read_map(&source, None)?;
    assert!(map.warnings.is_empty());
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let extra = args.get(3).map(|s| serde_json::from_str::<serde_json::Value>(s)).transpose()?;
    if let Some(extra) = &extra {
        let entity = extra["entity"].as_u64().ok_or("clip entity required")? as usize;
        let brush = extra["brush"].as_u64().ok_or("clip brush required")? as usize;
        map.entities.get(entity).and_then(|e| e.brushes.get(brush)).ok_or("clip target out of range")?;
    }
    let mut joined = Vec::new();
    let mut touched = BTreeSet::new();
    for (entity_index, entity) in map.entities.iter_mut().enumerate() {
        let original: Vec<_> = entity.brushes.iter().map(|b| Brush::from_faces(&world,b.faces.clone())).collect::<Result<_,_>>()?;
        let mut planes: Vec<Vec<Plane3d>> = vec![Vec::new(); original.len()];
        for i in 0..original.len() {
            for j in i+1..original.len() {
                let Ok(join) = corner_join::preview(&original[i],&original[j]) else { continue; };
                // Batch disjoint ends from the original rectangles, so a four-sided
                // frame can receive all four mitres in one pass.
                planes[i].push(join.plane); planes[j].push(join.plane.flip());
                joined.push(json!({"entity":entity_index,"brushes":[i,j]}));
            }
        }
        if let Some(extra) = &extra {
            if extra["entity"].as_u64() == Some(entity_index as u64) {
                let index = extra["brush"].as_u64().ok_or("clip brush required")? as usize;
                let n = extra["normal"].as_array().filter(|v|v.len()==3).ok_or("clip normal required")?;
                let normal = Vec3d::new(n[0].as_f64().ok_or("normal x")?,n[1].as_f64().ok_or("normal y")?,n[2].as_f64().ok_or("normal z")?);
                let distance = extra["distance"].as_f64().ok_or("clip distance required")?;
                assert!((vec::length(normal)-1.0).abs()<1e-8 && distance.is_finite());
                planes.get_mut(index).ok_or("clip brush out of range")?.push(Plane3d{normal,distance});
            }
        }
        let mut final_brushes = original.clone();
        for (index, cuts) in planes.iter().enumerate() {
            if cuts.is_empty() { continue; }
            let mut brush = original[index].clone();
            for plane in cuts { brush = clip(&brush,plane); }
            let mut faces = brush.faces().to_vec();
            for face in &mut faces {
                if cuts.iter().any(|p| vec::length(p.normal-face.plane.normal)<1e-6 && (p.distance-face.plane.distance).abs()<1e-5)
                    && !original[index].faces().iter().any(|old| vec::length(old.plane.normal-face.plane.normal)<1e-6 && (old.plane.distance-face.plane.distance).abs()<1e-5) {
                    face.material_name = "skip".into();
                }
            }
            final_brushes[index] = Brush::from_faces(&world,faces)?;
            touched.insert((entity_index,index));
        }
        // All operations only cut. Every removed solid must still be covered by
        // the final neighbours, otherwise a batch could open a hole at a triple joint.
        for &(owner,index) in &touched {
            if owner != entity_index { continue; }
            let target = &original[index];
            let neighbours: Vec<_> = final_brushes.iter().filter(|b|target.bounds().intersects(&b.bounds())).cloned().collect();
            let uncovered: f64 = csg::subtract_all(target,&neighbours).iter().map(Brush::volume).sum();
            assert!(uncovered <= target.volume().max(1.0)*1e-8, "union changed: entity={owner} brush={index} missing={uncovered}");
            assert!(final_brushes[index].volume()>1e-6);
            entity.brushes[index].faces = final_brushes[index].faces().to_vec();
        }
    }
    let prefix: String = source.lines().take_while(|line| line.trim().is_empty() || line.trim_start().starts_with("//"))
        .filter(|line| !line.starts_with("// Format:") && !line.starts_with("// Game:") && !line.starts_with("// heron_brush format ") && !line.starts_with("// entity ") && !line.starts_with("// heron_brush id "))
        .map(|line|format!("{line}\n")).collect();
    let text = prefix + &write_map(&map);
    let check = read_map(&text,None)?;
    assert!(check.warnings.is_empty());
    for (before,after) in map.entities.iter().zip(&check.entities) {
        assert_eq!(before.id,after.id); assert_eq!(before.properties,after.properties);
        assert_eq!(before.brushes.len(),after.brushes.len());
        for (a,b) in before.brushes.iter().zip(&after.brushes) { assert_eq!(a.id,b.id); Brush::from_faces(&world,b.faces.clone())?; }
    }
    std::fs::write(output,text)?;
    println!("{}",json!({"joins":joined,"changed_brushes":touched.len(),"union_preserved":true}));
    Ok(())
}
