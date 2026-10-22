//! Miter rectangular window jambs and lintels without changing their union.
//! Usage: miter_tower input.map output.map. Never overwrites either existing file.
use geometry_math::{
    bbox::Bbox3d,
    plane::Plane3d,
    vec::{self, Vec3d},
};
use map_model::{
    brush::{Brush, ClipOutcome},
    csg,
    io::{map_reader::read_map, map_writer::write_map},
    polyhedron::Polyhedron,
};
use std::collections::HashSet;
fn clip(b: &Brush, p: &Plane3d) -> Option<Brush> {
    match b.clip(p) {
        ClipOutcome::Unchanged => Some(b.clone()),
        ClipOutcome::Clipped(b) => Some(b),
        ClipOutcome::Empty => None,
    }
}
fn material(b: &Brush) -> Option<String> {
    let names: HashSet<_> = b
        .faces()
        .iter()
        .filter(|f| f.material_name.as_str() != "skip")
        .map(|f| f.material_name.to_string())
        .collect();
    if names.len() == 1 {
        names.into_iter().next()
    } else {
        None
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2);
    assert!(!std::path::Path::new(&args[1]).exists());
    let source = std::fs::read_to_string(&args[0]).unwrap();
    let mut map = read_map(&source, None).unwrap();
    let world = Bbox3d::from_scalars(-16384., 16384.);
    let e = &mut map.entities[0];
    let mut changed = 0;
    for side in 0..4 {
        let mut arches = vec![(-54., 766., 9., 776., 5.), (54., 766., 9., 776., 5.)];
        if side == 0 {
            arches.extend([
                (0., 24., 28., 100., 10.),
                (0., 408., 15., 438., 16.),
                (0., 676., 13., 700., 14.),
            ]);
        } else {
            arches.push((0., 678., 12., 700., 14.));
        }
        for (cx, z0, w, spring, rise) in arches {
            let axis = side % 2;
            let sign = if side < 2 { 1. } else { -1. };
            let depaxis = 1 - axis;
            let depsign = if side == 0 || side == 3 { 1. } else { -1. };
            let depth = 145. - (((z0 + spring) * 0.5f64).min(320.) / 40.).round();
            let all: Vec<_> = e
                .brushes
                .iter()
                .map(|b| Brush::from_faces(&world, b.faces.clone()).unwrap())
                .collect();
            let ix: Vec<_> = all
                .iter()
                .enumerate()
                .filter(|(_, b)| {
                    let pts = b.vertex_positions();
                    material(b).as_deref() == Some("piast/ashlar")
                        && pts.iter().all(|p| {
                            p.z() >= spring - 1e-5
                                && p.z() <= spring + rise + 4. + 1e-5
                                && (p[axis] * sign - cx).abs() <= w + 4. + 1e-5
                                && (p[depaxis] * depsign - (depth - 24.)) >= -1e-5
                                && (p[depaxis] * depsign - (depth + 1.)) <= 1e-5
                        })
                })
                .map(|(i, _)| i)
                .collect();
            if ix.len() <= 8 {
                continue;
            }
            let originals: Vec<_> = ix.iter().map(|i| all[*i].clone()).collect();
            let mut output = Vec::new();
            let mut valid = true;
            for sector in 0..8 {
                let mut parts = originals.clone();
                let mut cuts = Vec::new();
                for (boundary, keep) in [(sector, 1.), (sector + 1, -1.)] {
                    let angle = boundary as f64 * std::f64::consts::PI / 8.;
                    let round = |v: f64| (v * 8.).round() / 8.;
                    let u0 = round(cx + w * angle.cos());
                    let z0 = round(spring + rise * angle.sin());
                    let u1 = round(cx + (w + 4.) * angle.cos());
                    let z1 = round(spring + (rise + 4.) * angle.sin());
                    let mut normal = Vec3d::new(0., 0., 0.);
                    normal[axis] = (z1 - z0) * sign * keep;
                    normal[2] = -(u1 - u0) * keep;
                    let len = vec::length(normal);
                    let p = Plane3d {
                        normal: normal / len,
                        distance: ((z1 - z0) * u0 - (u1 - u0) * z0) * keep / len,
                    };
                    cuts.push(p);
                    parts = parts.iter().filter_map(|b| clip(b, &p)).collect();
                }
                let pts: Vec<_> = parts.iter().flat_map(|b| b.vertex_positions()).collect();
                if pts.len() < 4 {
                    valid = false;
                    break;
                }
                let Ok(poly) = Polyhedron::from_points(&pts, 1e-7) else {
                    valid = false;
                    break;
                };
                let raw = Brush::from_polyhedron(poly, "skip");
                let mut faces = Vec::new();
                for f in raw.faces() {
                    let mut face = f.clone();
                    if let Some(old) = originals
                        .iter()
                        .flat_map(|b| b.faces())
                        .filter(|o| {
                            vec::length(o.plane.normal - f.plane.normal) < 1e-6
                                && (o.plane.distance - f.plane.distance).abs() < 1e-5
                        })
                        .min_by_key(|o| o.material_name.as_str() == "skip")
                    {
                        let points = face.points;
                        let plane = face.plane;
                        face = old.clone();
                        face.points = points;
                        face.plane = plane;
                    }
                    if face.material_name.as_str() == "skip"
                        && !cuts.iter().any(|p| {
                            vec::length(p.normal - f.plane.normal) < 1e-6
                                && (p.distance - f.plane.distance).abs() < 1e-5
                        })
                        && !originals.iter().flat_map(|b| b.faces()).any(|o| {
                            vec::length(o.plane.normal - f.plane.normal) < 1e-6
                                && (o.plane.distance - f.plane.distance).abs() < 1e-5
                        })
                    {
                        let old = originals
                            .iter()
                            .flat_map(|b| b.faces())
                            .filter(|o| o.material_name.as_str() != "skip")
                            .min_by(|a, b| {
                                vec::length(a.plane.normal - f.plane.normal)
                                    .total_cmp(&vec::length(b.plane.normal - f.plane.normal))
                            })
                            .unwrap();
                        face = old.clone();
                        face.points = f.points;
                        face.plane = f.plane;
                    }
                    faces.push(face);
                }
                let Ok(b) = Brush::from_faces(&world, faces) else {
                    valid = false;
                    break;
                };
                let extra: f64 = csg::subtract_all(&b, &originals)
                    .iter()
                    .map(Brush::volume)
                    .sum();
                if extra > originals.iter().map(Brush::volume).sum::<f64>() * 0.01 {
                    valid = false;
                    break;
                }
                let deviation = b
                    .vertex_positions()
                    .iter()
                    .map(|p| {
                        originals
                            .iter()
                            .map(|b| {
                                b.faces()
                                    .iter()
                                    .map(|f| {
                                        (vec::dot(f.plane.normal, *p) - f.plane.distance).max(0.)
                                    })
                                    .fold(0., f64::max)
                            })
                            .fold(f64::INFINITY, f64::min)
                    })
                    .fold(0., f64::max);
                if deviation > 0.125001 {
                    valid = false;
                    break;
                }
                output.push(b);
            }
            if !valid || output.len() != 8 {
                println!("REJECT side={side} spring={spring} pieces={}", ix.len());
                continue;
            }
            if originals.iter().any(|b| {
                csg::subtract_all(b, &output)
                    .iter()
                    .map(Brush::volume)
                    .sum::<f64>()
                    > 0.0001
            }) {
                continue;
            }
            println!("REBUILD side={side} spring={spring} {} -> 8", ix.len());
            for (k, b) in output.iter().enumerate() {
                e.brushes[ix[k]].faces = b.faces().to_vec();
            }
            for i in ix.iter().skip(8).rev() {
                e.brushes.remove(*i);
            }
            changed += 1;
        }
    }
    let mut text = String::new();
    for l in source.lines().take_while(|l| !l.starts_with("// Format:")) {
        text.push_str(l);
        text.push('\n');
    }
    text.push_str(&write_map(&map));
    let check = read_map(&text, None).unwrap();
    for b in check.entities.iter().flat_map(|e| &e.brushes) {
        assert!(Brush::from_faces(&world, b.faces.clone()).unwrap().volume() > 0.);
    }
    std::fs::write(&args[1], text).unwrap();
    println!(
        "DONE arches={changed} brushes={}",
        check.entities[0].brushes.len()
    );
}
