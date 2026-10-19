//! A curved wall built from full-thickness convex prisms and two straight tails.
use super::*;
use geometry_math::bbox::Bbox3d;

pub(super) fn preview_wedges(a: &Brush, b: &Brush, options: Rounding) -> Result<Join, String> {
    let Some((radius, segments)) = options.inner else {
        return Err("Enable both roundings for full-thickness wedges.".into());
    };
    if options.outer.is_none() {
        return Err("Enable both roundings for full-thickness wedges.".into());
    }
    if options.concentric {
        return Err("Choose concentric rounding or full-thickness wedges.".into());
    }
    if !radius.is_finite() || radius <= 0.0 || !(1..=64).contains(&segments) {
        return Err("Use a positive finite radius and 1 to 64 segments.".into());
    }
    let mut join = preview(a, b)?;
    let [u, v] = join.axes;
    let depth = 3 - u - v;
    let aa = a.bounds();
    let bb = b.bounds();
    let (outer_u, su) = end((aa.min[u], aa.max[u]), (bb.min[u], bb.max[u])).unwrap();
    let (outer_v, sv) = end((bb.min[v], bb.max[v]), (aa.min[v], aa.max[v])).unwrap();
    let wu = bb.max[u] - bb.min[u];
    let wv = aa.max[v] - aa.min[v];
    let limit = (aa.max[u] - aa.min[u] - wu).min(bb.max[v] - bb.min[v] - wv);
    if radius >= limit - EPS {
        return Err("The curved wall must leave a positive straight length in both arms; reduce the inner radius.".into());
    }
    let mut center = aa.min;
    center[u] = outer_u + su * (wu + radius);
    center[v] = outer_v + sv * (wv + radius);
    let bounds = Bbox3d {
        min: Vec3d::new(
            aa.min[0].min(bb.min[0]),
            aa.min[1].min(bb.min[1]),
            aa.min[2].min(bb.min[2]),
        ),
        max: Vec3d::new(
            aa.max[0].max(bb.max[0]),
            aa.max[1].max(bb.max[1]),
            aa.max[2].max(bb.max[2]),
        ),
    };
    let point = |angle: f64, outside: bool| {
        let mut p = center;
        p[u] -= su * (radius + if outside { wu } else { 0.0 }) * angle.sin();
        p[v] -= sv * (radius + if outside { wv } else { 0.0 }) * angle.cos();
        p
    };
    // Offset each inner chord by its desired perpendicular thickness. Adjacent
    // offset lines meet at a common vertex; terminal lines meet the straight
    // outer walls. This moves the contacts instead of thinning coarse facets.
    let inner: Vec<_> = (0..=segments).map(|i| point(i as f64*std::f64::consts::FRAC_PI_2/segments as f64,false)).collect();
    let mut outer: Vec<_> = (0..=segments).map(|i| point(i as f64*std::f64::consts::FRAC_PI_2/segments as f64,true)).collect();
    if options.preserve_thickness {
        let lines: Vec<_> = inner.windows(2).enumerate().map(|(i,p)| {
            let dx=(p[1][u]-p[0][u])*su;
            let dy=(p[1][v]-p[0][v])*sv;
            let length=dx.hypot(dy);
            let n=[-dy/length,dx/length];
            let angle=(i as f64+0.5)*std::f64::consts::FRAC_PI_2/segments as f64;
            let thickness=wv*angle.cos().powi(2)+wu*angle.sin().powi(2);
            (n,n[0]*(p[0][u]-outer_u)*su+n[1]*(p[0][v]-outer_v)*sv+thickness)
        }).collect();
        for i in 0..=segments {
            let (n,d)=if i==0 {([0.0,1.0],0.0)} else {lines[i-1]};
            let (m,e)=if i==segments {([1.0,0.0],0.0)} else {lines[i]};
            let det=n[0]*m[1]-n[1]*m[0];
            if det.abs()<1e-9 {return Err("The thickness-preserving contacts are parallel.".into());}
            outer[i][u]=outer_u+su*(d*m[1]-e*n[1])/det;
            outer[i][v]=outer_v+sv*(e*n[0]-d*m[0])/det;
        }
    }
    // Cut each straight member at the tangent cross-section, independently of
    // the original overlap. Their IDs remain attached to these two tails.
    for (index, endpoint, axis, direction) in [(0, 0, u, su), (1, segments, v, sv)] {
        let edge=inner[endpoint]-outer[endpoint];
        let mut normal=Vec3d::zero();
        normal[u]=-edge[v]; normal[v]=edge[u];
        normal=normal/vec::length(normal);
        if normal[axis]*direction>0.0 {normal=-normal;}
        let plane = Plane3d::from_anchor_and_normal(outer[endpoint], normal);
        let ClipOutcome::Clipped(tail) = join.before[index].clip(&plane) else {
            return Err("The curved wall could not retain both straight arms.".into());
        };
        let mut faces = tail.faces().to_vec();
        for face in &mut faces {
            if vec::length(face.normal() - normal) < EPS
                && (face.plane.distance - plane.distance).abs() < EPS
            {
                face.material_name = "skip".into();
            }
        }
        join.after[index] = Brush::from_faces(&bounds, faces)
            .map_err(|_| "The straight arm could not be rebuilt reliably.")?;
    }
    for index in 0..segments {
        let polygon = [
            outer[index],
            outer[index+1],
            inner[index+1],
            inner[index],
        ];
        let points: Vec<_> = polygon
            .iter()
            .flat_map(|p| {
                [aa.min[depth], aa.max[depth]].map(|z| {
                    let mut p = *p;
                    p[depth] = z;
                    p
                })
            })
            .collect();
        let hull = map_model::polyhedron::Polyhedron::from_points(&points, 1e-7).map_err(|_| {
            "The wedges are too small to build reliably; increase the radius or reduce segments."
        })?;
        let raw = Brush::from_polyhedron(hull, a.faces()[0].material_name.as_str());
        let on = |face: &map_model::brush_face::BrushFace, p: Vec3d| {
            (vec::dot(face.normal(), p) - face.plane.distance).abs() < 1e-6
        };
        let faces = raw
            .faces()
            .iter()
            .map(|face| {
                let source = a
                    .faces()
                    .iter()
                    .chain(b.faces())
                    .max_by(|a, b| {
                        vec::dot(a.normal(), face.normal())
                            .total_cmp(&vec::dot(b.normal(), face.normal()))
                    })
                    .unwrap();
                let mut result = source.clone();
                result.points = face.points;
                result.plane = face.plane;
                let cap = face.normal()[depth].abs() > 1.0 - EPS;
                let arc = (on(face, polygon[0]) && on(face, polygon[1]))
                    || (on(face, polygon[2]) && on(face, polygon[3]));
                if !cap && !arc {
                    result.material_name = "skip".into();
                }
                result
            })
            .collect();
        let wedge = Brush::from_faces(&bounds, faces)
            .map_err(|_| "The curved-wall wedge could not be rebuilt reliably.")?;
        // A convex hull must not silently fill a concavity or a crossed quad.
        let area = (1..3)
            .map(|i| vec::cross(polygon[i] - polygon[0], polygon[i + 1] - polygon[0])[depth])
            .sum::<f64>()
            .abs()
            * 0.5;
        let expected = area * (aa.max[depth] - aa.min[depth]);
        if expected <= 1e-9 || (wedge.volume() - expected).abs() > expected.max(1.0) * 1e-7 {
            return Err("The wall thicknesses cannot form reliable convex wedges at this radius and segment count.".into());
        }
        join.added.push(wedge);
    }
    super::wrap_arc(&mut join, false);
    super::wrap_arc(&mut join, true);
    join.removed_volume =
        a.volume() + b.volume() - join.after.iter().map(Brush::volume).sum::<f64>();
    Ok(join)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options(radius: f64, segments: usize) -> Rounding {
        Rounding {
            outer: Some((1.0, 1)),
            inner: Some((radius, segments)),
            wedges: true,
            ..Default::default()
        }
    }
    fn cuboid(min: Vec3d, max: Vec3d) -> Brush {
        map_model::shapes::create_cuboid(
            &Bbox3d::from_scalars(-4096.0, 4096.0),
            &Bbox3d { min, max },
            "brick",
        )
        .unwrap()
    }
    #[test]
    fn corner_wedges_span_both_arcs_for_unequal_widths_in_all_orientations() {
        for u in 0..3 {
            for v in 0..3 {
                if u == v {
                    continue;
                }
                let depth = 3 - u - v;
                for su in [-1.0, 1.0] {
                    for sv in [-1.0, 1.0] {
                        let origin = Vec3d::new(100.0, 200.0, 300.0);
                        let point = |x, y, z| {
                            let mut p = origin;
                            p[u] += su * x;
                            p[v] += sv * y;
                            p[depth] += z;
                            p
                        };
                        let member = |x, y| {
                            let start = point(0.0, 0.0, 0.0);
                            let end = point(x, y, 8.0);
                            let mut min = start;
                            let mut max = end;
                            for axis in 0..3 {
                                min[axis] = start[axis].min(end[axis]);
                                max[axis] = start[axis].max(end[axis]);
                            }
                            cuboid(min, max)
                        };
                        for (wu, wv) in [(16.0, 16.0), (24.0, 8.0), (8.0, 24.0)] {
                            let a = member(128.0, wv);
                            let b = member(wu, 96.0);
                            for (first, second) in [(&a, &b), (&b, &a)] {
                                let join =
                                    preview_with_rounding(first, second, options(48.0, 8)).unwrap();
                                assert_eq!(join.added.len(), 8);
                                let all: Vec<_> = join.after.iter().chain(&join.added).collect();
                                for (i, brush) in all.iter().enumerate() {
                                    assert!(brush.volume() > EPS);
                                    for other in all.iter().skip(i + 1) {
                                        assert!(
                                            map_model::csg::intersect(brush, other)
                                                .is_none_or(|b| b.volume() < 1e-5),
                                            "overlapping wedges"
                                        );
                                    }
                                }
                                assert!(all.iter().any(|b| b.contains_point(point(
                                    127.0,
                                    wv / 2.0,
                                    4.0
                                ))));
                                assert!(all.iter().any(|b| b.contains_point(point(
                                    wu / 2.0,
                                    95.0,
                                    4.0
                                ))));
                                assert!(!all
                                    .iter()
                                    .any(|b| b.contains_point(point(0.1, 0.1, 4.0))));
                                let center = point(wu + 48.0, wv + 48.0, 0.0);
                                for wedge in &join.added {
                                    assert_eq!(
                                        wedge.faces().len(),
                                        6,
                                        "each segment must be a full-thickness quad prism"
                                    );
                                    assert_eq!(
                                        wedge
                                            .faces()
                                            .iter()
                                            .filter(|f| f.material_name.as_str() == "skip")
                                            .count(),
                                        2
                                    );
                                    let mut arcs = 0;
                                    for (face, polygon) in wedge.geometry_faces() {
                                        if face.normal()[depth].abs() > EPS
                                            || face.material_name.as_str() == "skip"
                                        {
                                            continue;
                                        }
                                        arcs += 1;
                                        let outer = face.normal()[u] * su < 0.0;
                                        let rx = 48.0 + if outer { wu } else { 0.0 };
                                        let ry = 48.0 + if outer { wv } else { 0.0 };
                                        for vertex in polygon {
                                            let x = (vertex[u] - center[u]) / rx;
                                            let y = (vertex[v] - center[v]) / ry;
                                            assert!(
                                                (x * x + y * y - 1.0).abs() < 1e-6,
                                                "arc vertex off circle/ellipse"
                                            );
                                        }
                                    }
                                    assert_eq!(arcs, 2, "one inner and one outer arc per wedge");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn corner_unequal_widths_also_support_independent_rounding() {
        for (wu, wv) in [(24.0, 8.0), (8.0, 24.0)] {
            let a = cuboid(Vec3d::zero(), Vec3d::new(128.0, wv, 8.0));
            let b = cuboid(Vec3d::zero(), Vec3d::new(wu, 96.0, 8.0));
            for (outer, inner) in [
                (Some((4.0, 8)), None),
                (None, Some((32.0, 8))),
                (Some((4.0, 8)), Some((32.0, 8))),
            ] {
                let join = preview_with_rounding(
                    &a,
                    &b,
                    Rounding {
                        outer,
                        inner,
                        ..Default::default()
                    },
                )
                .unwrap();
                let all: Vec<_> = join.after.iter().chain(&join.added).collect();
                for (i, brush) in all.iter().enumerate() {
                    for other in all.iter().skip(i + 1) {
                        assert!(map_model::csg::intersect(brush, other)
                            .is_none_or(|b| b.volume() < 1e-5));
                    }
                }
            }
        }
    }

    #[test]
    fn corner_wedges_validate_limits_and_segment_extremes() {
        let a = cuboid(Vec3d::zero(), Vec3d::new(128.0, 8.0, 8.0));
        let b = cuboid(Vec3d::zero(), Vec3d::new(24.0, 96.0, 8.0));
        for segments in [1, 64] {
            let join = preview_with_rounding(&a, &b, options(48.0, segments)).unwrap();
            assert_eq!(join.added.len(), segments);
            let expected_tails =
                (128.0 - 24.0 - 48.0) * 8.0 * 8.0 + (96.0 - 8.0 - 48.0) * 24.0 * 8.0;
            assert!(
                (join.after.iter().map(Brush::volume).sum::<f64>() - expected_tails).abs() < 1e-6
            );
        }
        for radius in [0.0, -1.0, 88.0, 200.0, f64::NAN, f64::INFINITY] {
            assert!(preview_with_rounding(&a, &b, options(radius, 8)).is_err());
        }
        for segments in [0, 65, usize::MAX] {
            assert!(preview_with_rounding(&a, &b, options(8.0, segments)).is_err());
        }
        let mut missing = options(8.0, 8);
        missing.outer = None;
        assert!(preview_with_rounding(&a, &b, missing).is_err());
        let mut conflict = options(8.0, 8);
        conflict.concentric = true;
        assert!(preview_with_rounding(&a, &b, conflict).is_err());
    }
}

#[cfg(test)]
mod thickness_tests {
    use super::*;
    #[test]
    fn corner_preserved_thickness_matches_facet_offsets_and_contacts() {
        for segments in [1,2,8,64] { for (wu,wv) in [(16.0,16.0),(24.0,8.0),(8.0,24.0)] {
            let make=|x,y|map_model::shapes::create_cuboid(&Bbox3d::from_scalars(-4096.0,4096.0),&Bbox3d{min:Vec3d::zero(),max:Vec3d::new(x,y,8.0)},"brick").unwrap();
            let a=make(128.0,wv); let b=make(wu,96.0);
            let join=preview_with_rounding(&a,&b,Rounding{outer:Some((1.0,1)),inner:Some((24.0,segments)),wedges:true,preserve_thickness:true,..Default::default()}).unwrap();
            assert_eq!(join.added.len(),segments);
            for (i,brush) in join.added.iter().enumerate() {
                let angle=(i as f64+0.5)*std::f64::consts::FRAC_PI_2/segments as f64;
                let desired=wv*angle.cos().powi(2)+wu*angle.sin().powi(2);
                let inner=brush.faces().iter().find(|f|f.normal()[0]>EPS && f.normal()[1]>EPS && f.material_name.as_str()!="skip").unwrap();
                let outer=brush.faces().iter().find(|f|vec::dot(f.normal(),inner.normal())< -1.0+EPS && f.material_name.as_str()!="skip").unwrap();
                assert!((inner.plane.distance+outer.plane.distance-desired).abs()<1e-5,"{segments}: {:?} {:?}, {desired}",inner.plane,outer.plane);
            }
            let all:Vec<_>=join.after.iter().chain(&join.added).collect();
            for (i,a) in all.iter().enumerate() {for b in all.iter().skip(i+1) {assert!(map_model::csg::intersect(a,b).is_none_or(|b|b.volume()<1e-5));}}
            for (tail,cell) in [(&join.after[0],&join.added[0]),(&join.after[1],join.added.last().unwrap())] {
                assert!(tail.faces().iter().any(|a|cell.faces().iter().any(|b|vec::dot(a.normal(),b.normal())< -1.0+EPS && (a.plane.distance+b.plane.distance).abs()<EPS)),"missing contact");
            }
        }}
    }
}
