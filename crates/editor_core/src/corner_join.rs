//! Complementary mitre cuts for two overlapping, axis-aligned rectangular members.
use geometry_math::{
    plane::Plane3d,
    vec::{self, Vec3d},
};
use map_model::brush::{Brush, ClipOutcome};
const EPS: f64 = 1e-6;
#[path = "corner_join_wedges.rs"]
mod wedges;
use wedges::preview_wedges;
#[derive(Clone)]
pub struct Join {
    pub before: [Brush; 2],
    pub after: [Brush; 2],
    pub added: Vec<Brush>,
    pub axes: [usize; 2],
    pub plane: Plane3d,
    pub removed_volume: f64,
}
fn rectangular(brush: &Brush) -> bool {
    brush.faces().len() == 6
        && brush
            .face_planes()
            .iter()
            .all(|p| p.normal.v.iter().filter(|v| v.abs() > EPS).count() == 1)
}
// The narrower interval occupies exactly one end of the longer interval.
fn end(long: (f64, f64), short: (f64, f64)) -> Option<(f64, f64)> {
    if short.1 - short.0 <= EPS || long.1 - long.0 <= short.1 - short.0 + EPS {
        return None;
    }
    if (long.0 - short.0).abs() < EPS && short.1 < long.1 - EPS {
        Some((long.0, 1.0))
    } else if (long.1 - short.1).abs() < EPS && short.0 > long.0 + EPS {
        Some((long.1, -1.0))
    } else {
        None
    }
}
pub fn preview(a: &Brush, b: &Brush) -> Result<Join, String> {
    if !rectangular(a) || !rectangular(b) {
        return Err("The first version requires two axis-aligned rectangular brushes.".into());
    }
    let aa = a.bounds();
    let bb = b.bounds();
    for u in 0..3 {
        for v in 0..3 {
            if u == v {
                continue;
            }
            let depth = 3 - u - v;
            if (aa.min[depth] - bb.min[depth]).abs() > EPS
                || (aa.max[depth] - bb.max[depth]).abs() > EPS
            {
                continue;
            }
            let Some((outer_u, su)) = end((aa.min[u], aa.max[u]), (bb.min[u], bb.max[u])) else {
                continue;
            };
            let Some((outer_v, sv)) = end((bb.min[v], bb.max[v]), (aa.min[v], aa.max[v])) else {
                continue;
            };
            let wu = bb.max[u] - bb.min[u];
            let wv = aa.max[v] - aa.min[v];
            let mut anchor = aa.min;
            anchor[u] = outer_u;
            anchor[v] = outer_v;
            let mut normal = Vec3d::zero();
            normal[u] = -su / wu;
            normal[v] = sv / wv;
            normal = normal / vec::length(normal);
            let plane = Plane3d::from_anchor_and_normal(anchor, normal);
            let opposite = Plane3d::from_anchor_and_normal(anchor, -normal);
            let (ClipOutcome::Clipped(first), ClipOutcome::Clipped(second)) =
                (a.clip(&plane), b.clip(&opposite))
            else {
                return Err("The corner could not be cut reliably at this scale.".into());
            };
            let overlap = wu * wv * (aa.max[depth] - aa.min[depth]);
            let expected = a.volume() + b.volume() - overlap;
            let actual = first.volume() + second.volume();
            if first.volume() <= EPS
                || second.volume() <= EPS
                || (expected - actual).abs() > expected.abs().max(1.0) * 1e-7
            {
                return Err(
                    "Corner validation failed; the original brushes were preserved.".into(),
                );
            }
            return Ok(Join {
                before: [a.clone(), b.clone()],
                after: [first, second],
                added: Vec::new(),
                axes: [u, v],
                plane,
                removed_volume: overlap,
            });
        }
    }
    Err("Select two overlapping rectangular ends forming an L, with the same depth. Touching ends, T-junctions and crossings are not supported yet.".into())
}

/// Rounds the outside of the mitred corner with the same cuts on both members.
/// Every cut intersects a convex brush, so the result still contains two brushes.
pub fn preview_rounded(a: &Brush, b: &Brush, radius: f64, segments: usize) -> Result<Join, String> {
    round_outer(a, b, radius, segments, false)
}

fn round_outer(
    a: &Brush,
    b: &Brush,
    radius: f64,
    segments: usize,
    concentric: bool,
) -> Result<Join, String> {
    let mut join = preview(a, b)?;
    if !radius.is_finite() || radius <= 0.0 || !(1..=64).contains(&segments) {
        return Err("Use a positive finite radius and 1 to 64 segments.".into());
    }
    let [u, v] = join.axes;
    let aa = a.bounds();
    let bb = b.bounds();
    let (outer_u, su) = end((aa.min[u], aa.max[u]), (bb.min[u], bb.max[u])).unwrap();
    let (outer_v, sv) = end((bb.min[v], bb.max[v]), (aa.min[v], aa.max[v])).unwrap();
    let limit = if concentric {
        (aa.max[u] - aa.min[u]).min(bb.max[v] - bb.min[v])
    } else {
        (bb.max[u] - bb.min[u]).min(aa.max[v] - aa.min[v])
    };
    if radius > limit {
        return Err(if concentric {
            "The outer radius must fit within both member lengths."
        } else {
            "The radius must not exceed the narrower member width."
        }
        .into());
    }
    let mut center = aa.min;
    center[u] = outer_u + su * radius;
    center[v] = outer_v + sv * radius;
    let step = std::f64::consts::FRAC_PI_2 / segments as f64;
    for index in 0..segments {
        let angle = (index as f64 + 0.5) * step;
        let mut normal = Vec3d::zero();
        normal[u] = -su * angle.cos();
        normal[v] = -sv * angle.sin();
        let anchor = center + normal * (radius * (step * 0.5).cos());
        let plane = Plane3d::from_anchor_and_normal(anchor, normal);
        for brush in &mut join.after {
            match brush.clip(&plane) {
                ClipOutcome::Clipped(result) => *brush = result,
                ClipOutcome::Unchanged => {}
                _ => return Err("The rounded corner could not be built; reduce its radius.".into()),
            }
        }
    }
    wrap_arc(&mut join, false);
    join.removed_volume =
        a.volume() + b.volume() - join.after.iter().map(Brush::volume).sum::<f64>();
    Ok(join)
}

/// Independent outer cuts and inner filling, both expressed as radius/segments.
#[derive(Clone, Copy, Default)]
pub struct Rounding {
    pub outer: Option<(f64, usize)>,
    pub inner: Option<(f64, usize)>,
    /// Derive outer radius as inner radius plus the common member width.
    pub concentric: bool,
    /// Replace the corner with full-thickness convex segments.
    pub wedges: bool,
    /// Offset polygon facets to preserve perpendicular wall thickness.
    pub preserve_thickness: bool,
}

/// Adds a concave quarter-circle fillet as non-overlapping triangular prisms.
pub fn preview_with_rounding(a: &Brush, b: &Brush, rounding: Rounding) -> Result<Join, String> {
    if rounding.wedges {
        return preview_wedges(a, b, rounding);
    }
    let mut join = if rounding.concentric {
        let base = preview(a, b)?;
        let [u, v] = base.axes;
        let wu = b.bounds().max[u] - b.bounds().min[u];
        let wv = a.bounds().max[v] - a.bounds().min[v];
        if (wu - wv).abs() > EPS {
            return Err("Concentric rounding requires equal member widths.".into());
        }
        let Some((radius, segments)) = rounding.inner else {
            return Err("Enable both roundings for concentric mode.".into());
        };
        if rounding.outer.is_none() {
            return Err("Enable both roundings for concentric mode.".into());
        }
        round_outer(a, b, wu + radius, segments, true)?
    } else {
        match rounding.outer {
            Some((radius, segments)) => preview_rounded(a, b, radius, segments)?,
            None => preview(a, b)?,
        }
    };
    let Some((radius, segments)) = rounding.inner else {
        return Ok(join);
    };
    if !radius.is_finite() || radius <= 0.0 || !(1..=64).contains(&segments) {
        return Err("Use a positive finite radius and 1 to 64 segments.".into());
    }
    let [u, v] = join.axes;
    let depth = 3 - u - v;
    let aa = a.bounds();
    let bb = b.bounds();
    let (outer_u, su) = end((aa.min[u], aa.max[u]), (bb.min[u], bb.max[u])).unwrap();
    let (outer_v, sv) = end((bb.min[v], bb.max[v]), (aa.min[v], aa.max[v])).unwrap();
    let wu = bb.max[u] - bb.min[u];
    let wv = aa.max[v] - aa.min[v];
    let limit = (aa.max[u] - aa.min[u] - wu).min(bb.max[v] - bb.min[v] - wv);
    if radius > limit {
        return Err(
            "The inner radius must fit between the inside corner and both member ends.".into(),
        );
    }
    let mut inner = aa.min;
    inner[u] = outer_u + su * wu;
    inner[v] = outer_v + sv * wv;
    let point = |angle: f64| {
        let mut p = inner;
        p[u] += su * radius * (1.0 - angle.sin());
        p[v] += sv * radius * (1.0 - angle.cos());
        p
    };
    for index in 0..segments {
        let start = point(index as f64 * std::f64::consts::FRAC_PI_2 / segments as f64);
        let end = point((index + 1) as f64 * std::f64::consts::FRAC_PI_2 / segments as f64);
        let points: Vec<_> = [inner, start, end]
            .into_iter()
            .flat_map(|p| {
                [aa.min[depth], aa.max[depth]].map(|z| {
                    let mut p = p;
                    p[depth] = z;
                    p
                })
            })
            .collect();
        let hull = map_model::polyhedron::Polyhedron::from_points(&points,1e-7)
            .map_err(|_| "The inner fillet is too small to build reliably; increase the radius or reduce segments.")?;
        let mut raw = Brush::from_polyhedron(hull, a.faces()[0].material_name.as_str());
        if rounding.concentric {
            let mut center = inner;
            center[u] += su * radius;
            center[v] += sv * radius;
            let outer_radius = wu + radius;
            let step = std::f64::consts::FRAC_PI_2 / segments as f64;
            for cut in 0..segments {
                let angle = (cut as f64 + 0.5) * step;
                let mut normal = Vec3d::zero();
                normal[u] = -su * angle.cos();
                normal[v] = -sv * angle.sin();
                let plane = Plane3d::from_anchor_and_normal(
                    center + normal * (outer_radius * (step * 0.5).cos()),
                    normal,
                );
                match raw.clip(&plane) {
                    ClipOutcome::Clipped(brush) => raw = brush,
                    ClipOutcome::Unchanged => {}
                    ClipOutcome::Empty => {
                        return Err("The concentric fillet is too small to build reliably.".into())
                    }
                }
            }
        }
        let faces = raw
            .faces()
            .iter()
            .map(|face| {
                let source = a
                    .faces()
                    .iter()
                    .chain(b.faces())
                    .max_by(|a, b| {
                        vec::dot(a.plane.normal, face.plane.normal)
                            .total_cmp(&vec::dot(b.plane.normal, face.plane.normal))
                    })
                    .unwrap();
                let mut result = source.clone();
                result.points = face.points;
                result.plane = face.plane;
                // Radial/contact faces pass through the inside corner. Caps and the
                // arc chord remain visible; every radial face is covered by a neighbour.
                if face.plane.normal[depth].abs() < EPS
                    && (vec::dot(face.plane.normal, inner) - face.plane.distance).abs() < 1e-7
                {
                    result.material_name = "skip".into();
                }
                result
            })
            .collect();
        let bounds = geometry_math::bbox::Bbox3d {
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
        let brush = Brush::from_faces(&bounds, faces)
            .map_err(|_| "The inner fillet could not be built reliably.")?;
        if brush.volume() <= 1e-9 {
            return Err("The inner fillet is too small; reduce segments.".into());
        }
        join.added.push(brush);
    }
    if rounding.concentric {
        wrap_arc(&mut join, false);
    }
    wrap_arc(&mut join, true);
    Ok(join)
}

/// Unfold each arc from its straight wall. Anchor every rotation at the shared
/// edge, including the coplanar split between the two original members.
fn wrap_arc(join: &mut Join, inner: bool) {
    use map_model::{
        brush_face::BrushFace,
        uv::{UvCoordSystem, WrapStyle},
    };
    let [u, v] = join.axes;
    let depth = 3 - u - v;
    let aa = join.before[0].bounds();
    let bb = join.before[1].bounds();
    let (_, su) = end((aa.min[u], aa.max[u]), (bb.min[u], bb.max[u])).unwrap();
    let (_, sv) = end((bb.min[v], bb.max[v]), (aa.min[v], aa.max[v])).unwrap();
    let sign = if inner { 1.0 } else { -1.0 };
    let source = |member: usize, axis: usize, direction: f64| -> BrushFace {
        join.before[member]
            .faces()
            .iter()
            .find(|f| f.plane.normal[axis] * direction > 1.0 - EPS)
            .unwrap()
            .clone()
    };
    let first = source(0, v, sign * sv);
    let last = source(1, u, sign * su);
    let mut brushes: Vec<_> = join.after.iter().chain(&join.added).cloned().collect();
    let mut faces: Vec<_> = brushes.iter().map(|b| b.faces().to_vec()).collect();
    let mut arc = Vec::new();
    for (bi, brush) in brushes.iter().enumerate() {
        for (fi, face) in brush.faces().iter().enumerate() {
            let n = face.plane.normal;
            if n[depth].abs() < EPS
                && n[u] * sign * su > EPS
                && n[v] * sign * sv > EPS
                && face.material_name.as_str() != "skip"
            {
                arc.push((bi, fi, n[u].abs().atan2(n[v].abs())));
            }
        }
    }
    arc.sort_by(|a, b| a.2.total_cmp(&b.2));
    if arc.is_empty() {
        return;
    }
    let same = first.material_name == last.material_name;
    // Different materials keep their boundary. Unfold each material from its own arm.
    for reverse in [false, true] {
        if reverse && same {
            continue;
        }
        let mut previous = if reverse { last.clone() } else { first.clone() };
        let entries: Vec<_> = if reverse {
            arc.iter().rev().collect()
        } else {
            arc.iter().collect()
        };
        for &&(bi, fi, _) in &entries {
            if !same && faces[bi][fi].material_name != previous.material_name {
                continue;
            }
            let mut target = faces[bi][fi].clone();
            carry_mapping(&previous, &mut target, u, v, depth, aa.min[depth]);
            previous = target.clone();
            faces[bi][fi] = target;
        }
        if same {
            for brush_faces in &mut faces {
                for target in brush_faces {
                    if vec::dot(target.plane.normal, last.plane.normal) > 1.0 - EPS
                        && (target.plane.distance - last.plane.distance).abs() < EPS
                    {
                        carry_mapping(&previous, target, u, v, depth, aa.min[depth]);
                    }
                }
            }
        }
    }
    for (brush, updated) in brushes.iter_mut().zip(faces) {
        *brush = brush.with_face_attributes_from(&updated);
    }
    join.after = [brushes[0].clone(), brushes[1].clone()];
    join.added = brushes.into_iter().skip(2).collect();

    fn carry_mapping(
        source: &BrushFace,
        target: &mut BrushFace,
        u: usize,
        v: usize,
        depth: usize,
        z: f64,
    ) {
        let p = source.plane;
        let q = target.plane;
        let determinant = p.normal[u] * q.normal[v] - p.normal[v] * q.normal[u];
        let mut anchor = target.points[0];
        if determinant.abs() > 1e-9 {
            anchor[u] = (p.distance * q.normal[v] - q.distance * p.normal[v]) / determinant;
            anchor[v] = (q.distance * p.normal[u] - p.distance * q.normal[u]) / determinant;
            anchor[depth] = z;
        }
        let (u_axis, v_axis) = source
            .uv_coord_system
            .axes(source.normal(), &source.uv_attributes);
        target.uv_attributes = source.uv_attributes;
        target.primitive_uv = None;
        target.uv_coord_system = UvCoordSystem::Parallel { u_axis, v_axis }.updated_for_normal(
            source.normal(),
            target.normal(),
            WrapStyle::Rotation,
        );
        target.uv_attributes.offset += source.uv_coords(anchor) - target.uv_coords(anchor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::bbox::Bbox3d;
    #[test]
    fn rounded_texture_coordinates_match_along_every_shared_edge() {
        for preserve_thickness in [false, true] {
            for (inner, concentric, wedges, wu, wv) in [
                (false, false, false, 16.0, 16.0),
                (true, false, false, 16.0, 16.0),
                (false, true, false, 16.0, 16.0),
                (true, true, false, 16.0, 16.0),
                (false, false, true, 16.0, 16.0),
                (true, false, true, 16.0, 16.0),
                (false, false, true, 24.0, 8.0),
                (true, false, true, 24.0, 8.0),
                (false, false, true, 8.0, 24.0),
                (true, false, true, 8.0, 24.0),
            ] {
                let textured = |max| {
                    let brush = cube(Vec3d::zero(), max, "brick");
                    let mut faces = brush.faces().to_vec();
                    for face in &mut faces {
                        face.uv_attributes.offset = geometry_math::vec::Vec2f::new(17.0, -31.0);
                        face.rotate_uv(27.0);
                        face.uv_attributes.scale = geometry_math::vec::Vec2f::new(0.75, -1.5);
                    }
                    Brush::from_faces(
                        &geometry_math::bbox::Bbox3d::from_scalars(-1024.0, 1024.0),
                        faces,
                    )
                    .unwrap()
                };
                let a = textured(Vec3d::new(128.0, wv, 8.0));
                let b = textured(Vec3d::new(wu, 96.0, 8.0));
                let join = preview_with_rounding(
                    &a,
                    &b,
                    Rounding {
                        outer: if inner && !concentric && !wedges {
                            None
                        } else {
                            Some((12.0, 8))
                        },
                        inner: if inner || concentric || wedges {
                            Some((24.0, 8))
                        } else {
                            None
                        },
                        concentric,
                        wedges,
                        preserve_thickness,
                    },
                )
                .unwrap();
                let sign = if inner { 1.0 } else { -1.0 };
                let walls: Vec<_> = join
                    .after
                    .iter()
                    .chain(&join.added)
                    .flat_map(|b| b.geometry_faces())
                    .filter(|(f, _)| {
                        f.normal()[2].abs() < EPS
                            && f.material_name.as_str() == "brick"
                            && f.normal()[0] * sign >= -EPS
                            && f.normal()[1] * sign >= -EPS
                            && (f.normal()[0].abs() < 1.0 - EPS && f.normal()[1].abs() < 1.0 - EPS
                                || f.plane.distance.abs() <= wu.max(wv) + EPS)
                    })
                    .collect();
                let mut edges = 0;
                for (i, (a, pa)) in walls.iter().enumerate() {
                    for (b, pb) in walls.iter().skip(i + 1) {
                        // The old straight walls still meet under the inner fill.
                        // Their buried junction is not part of the visible arc.
                        if inner
                            && a.normal()[0].abs().max(a.normal()[1].abs()) > 1.0 - EPS
                            && b.normal()[0].abs().max(b.normal()[1].abs()) > 1.0 - EPS
                            && vec::dot(a.normal(), b.normal()).abs() < EPS
                        {
                            continue;
                        }
                        let contains = |face: &map_model::brush_face::BrushFace,
                                        polygon: &Vec<Vec3d>,
                                        p: Vec3d| {
                            if (vec::dot(face.normal(), p) - face.plane.distance).abs() > 1e-5 {
                                return false;
                            }
                            let signs: Vec<_> = polygon
                                .iter()
                                .zip(polygon.iter().cycle().skip(1))
                                .map(|(a, b)| vec::dot(vec::cross(*b - *a, p - *a), face.normal()))
                                .collect();
                            signs.iter().all(|s| *s >= -1e-5) || signs.iter().all(|s| *s <= 1e-5)
                        };
                        let mut common = Vec::new();
                        for &p in pa.iter().chain(pb) {
                            if contains(a, pa, p)
                                && contains(b, pb, p)
                                && !common.iter().any(|q| vec::length(p - *q) < 1e-5)
                            {
                                common.push(p);
                            }
                        }
                        if common.len() < 2 {
                            continue;
                        }
                        for p in common {
                            let delta = a.uv_coords(p) - b.uv_coords(p);
                            assert!(
                                delta.x().abs() < 0.001 && delta.y().abs() < 0.001,
                                "UV seam inner={inner}: {delta:?}, {:?} {:?} at {p:?}",
                                a.plane,
                                b.plane
                            );
                        }
                        edges += 1;
                    }
                }
                assert!(edges >= 8, "not enough shared edges checked: {edges}");
            }
        }
    }

    fn cube(min: Vec3d, max: Vec3d, material: &str) -> Brush {
        map_model::shapes::create_cuboid(
            &Bbox3d::from_scalars(-8192.0, 8192.0),
            &Bbox3d { min, max },
            material,
        )
        .unwrap()
    }
    #[test]
    fn mitres_preserve_union_in_all_planes_quadrants_and_widths() {
        for u in 0..3 {
            for v in 0..3 {
                if u == v {
                    continue;
                }
                let depth = 3 - u - v;
                for su in [-1.0, 1.0] {
                    for sv in [-1.0, 1.0] {
                        for width in [16.0, 24.0] {
                            let map = |x: f64, y: f64, z: f64| {
                                let mut p = Vec3d::new(128.0, 256.0, 32.0);
                                p[u] += su * x;
                                p[v] += sv * y;
                                p[depth] += z;
                                p
                            };
                            let box_ = |x: f64, y: f64, mat: &str| {
                                let a = map(0.0, 0.0, 0.0);
                                let b = map(x, y, 8.0);
                                let mut min = a;
                                let mut max = b;
                                for i in 0..3 {
                                    min[i] = a[i].min(b[i]);
                                    max[i] = a[i].max(b[i]);
                                }
                                cube(min, max, mat)
                            };
                            let a = box_(128.0, 16.0, "first");
                            let b = box_(width, 96.0, "second");
                            for (a, b) in [(&a, &b), (&b, &a)] {
                                let join = preview(a, b).unwrap();
                                assert!(join.after[0]
                                    .faces()
                                    .iter()
                                    .all(|f| f.material_name == a.faces()[0].material_name));
                                assert!(join.after[1]
                                    .faces()
                                    .iter()
                                    .all(|f| f.material_name == b.faces()[0].material_name));
                                assert!((join.removed_volume - width * 16.0 * 8.0).abs() < 1e-6);
                                for x in 0..34 {
                                    for y in 0..26 {
                                        let p =
                                            map(x as f64 * 4.0 - 1.0, y as f64 * 4.0 - 1.0, 4.0);
                                        assert_eq!(
                                            a.contains_point(p) || b.contains_point(p),
                                            join.after.iter().any(|b| b.contains_point(p))
                                        );
                                        if (vec::dot(join.plane.normal, p) - join.plane.distance)
                                            .abs()
                                            > EPS
                                        {
                                            assert!(
                                                !(join.after[0].contains_point(p)
                                                    && join.after[1].contains_point(p))
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn rounded_join_cuts_only_the_outer_corner_in_every_orientation() {
        for u in 0..3 {
            for v in 0..3 {
                if u == v {
                    continue;
                }
                for su in [-1.0, 1.0] {
                    for sv in [-1.0, 1.0] {
                        let depth = 3 - u - v;
                        let point = |x: f64, y: f64, z: f64| {
                            let mut p = Vec3d::new(100.0, 200.0, 300.0);
                            p[u] += su * x;
                            p[v] += sv * y;
                            p[depth] += z;
                            p
                        };
                        let member = |x, y, material| {
                            let a = point(0.0, 0.0, 0.0);
                            let b = point(x, y, 8.0);
                            let mut min = a;
                            let mut max = a;
                            for axis in 0..3 {
                                min[axis] = a[axis].min(b[axis]);
                                max[axis] = a[axis].max(b[axis]);
                            }
                            cube(min, max, material)
                        };
                        let a = member(128.0, 16.0, "a");
                        let b = member(24.0, 96.0, "b");
                        for (a, b) in [(&a, &b), (&b, &a)] {
                            let sharp = preview(a, b).unwrap();
                            let rounded = preview_rounded(a, b, 8.0, 8).unwrap();
                            assert!(rounded.after.iter().all(|b| b.volume() > EPS));
                            assert!(!rounded
                                .after
                                .iter()
                                .any(|b| b.contains_point(point(0.1, 0.1, 4.0))));
                            for p in [
                                point(8.0, 8.0, 4.0),
                                point(100.0, 8.0, 4.0),
                                point(12.0, 80.0, 4.0),
                            ] {
                                assert!(rounded.after.iter().any(|b| b.contains_point(p)));
                            }
                            assert!(rounded.removed_volume > sharp.removed_volume);
                            for (original, result) in [a, b].into_iter().zip(&rounded.after) {
                                assert!(result
                                    .faces()
                                    .iter()
                                    .all(|f| f.material_name == original.faces()[0].material_name));
                            }
                        }
                    }
                }
            }
        }
        let a = cube(Vec3d::zero(), Vec3d::new(128.0, 16.0, 8.0), "a");
        let b = cube(Vec3d::zero(), Vec3d::new(24.0, 96.0, 8.0), "b");
        for radius in [0.0, -1.0, 17.0, f64::NAN, f64::INFINITY] {
            assert!(preview_rounded(&a, &b, radius, 8).is_err());
        }
        for segments in [0, 65, usize::MAX] {
            assert!(preview_rounded(&a, &b, 8.0, segments).is_err());
        }
    }
    #[test]
    fn inner_fill_and_concentric_arcs_work_in_all_planes_and_quadrants() {
        for u in 0..3 {
            for v in 0..3 {
                if u == v {
                    continue;
                }
                let depth = 3 - u - v;
                for su in [-1.0, 1.0] {
                    for sv in [-1.0, 1.0] {
                        let point = |x: f64, y: f64, z: f64| {
                            let mut p = Vec3d::new(100.0, 200.0, 300.0);
                            p[u] += su * x;
                            p[v] += sv * y;
                            p[depth] += z;
                            p
                        };
                        let member = |x, y| {
                            let a = point(0.0, 0.0, 0.0);
                            let b = point(x, y, 8.0);
                            let mut min = a;
                            let mut max = b;
                            for axis in 0..3 {
                                min[axis] = a[axis].min(b[axis]);
                                max[axis] = a[axis].max(b[axis]);
                            }
                            cube(min, max, "stone")
                        };
                        let a = member(128.0, 16.0);
                        let b = member(16.0, 96.0);
                        for (a, b) in [(&a, &b), (&b, &a)] {
                            for options in [
                                Rounding {
                                    inner: Some((8.0, 8)),
                                    ..Default::default()
                                },
                                Rounding {
                                    outer: Some((8.0, 8)),
                                    inner: Some((8.0, 8)),
                                    concentric: false,
                                    wedges: false,
                                    preserve_thickness: false,
                                },
                                Rounding {
                                    outer: Some((1.0, 1)),
                                    inner: Some((48.0, 8)),
                                    concentric: true,
                                    wedges: false,
                                    preserve_thickness: false,
                                },
                            ] {
                                let join = preview_with_rounding(a, b, options).unwrap();
                                assert_eq!(join.added.len(), 8);
                                let all: Vec<_> = join.after.iter().chain(&join.added).collect();
                                for i in 0..all.len() {
                                    for j in i + 1..all.len() {
                                        assert!(
                                            map_model::csg::intersect(all[i], all[j])
                                                .is_none_or(|brush| brush.volume() < 1e-5),
                                            "overlap {i},{j}"
                                        );
                                    }
                                }
                                assert!(join.added.iter().all(|b| b.volume() > 0.0));
                                if options.concentric {
                                    let center = point(64.0, 64.0, 0.0);
                                    let factor = (std::f64::consts::FRAC_PI_2 / 16.0).cos();
                                    let mut arcs = 0;
                                    for brush in &all {
                                        for face in brush.faces() {
                                            let n = face.plane.normal;
                                            if n[u].abs() < EPS
                                                || n[v].abs() < EPS
                                                || face.material_name.as_str() == "skip"
                                            {
                                                continue;
                                            }
                                            if vec::length(n - join.plane.normal) < EPS
                                                || vec::length(n + join.plane.normal) < EPS
                                            {
                                                continue;
                                            }
                                            let distance =
                                                (vec::dot(n, center) - face.plane.distance).abs();
                                            assert!(
                                                (distance - 64.0 * factor).abs() < 1e-5
                                                    || (distance - 48.0 * factor).abs() < 1e-5,
                                                "nonconcentric arc {distance}"
                                            );
                                            arcs += 1;
                                        }
                                    }
                                    assert!(arcs >= 16);
                                } else {
                                    assert!(join
                                        .added
                                        .iter()
                                        .any(|b| b.contains_point(point(16.1, 16.1, 4.0))));
                                    assert!(!join
                                        .added
                                        .iter()
                                        .any(|b| b.contains_point(point(24.0, 24.0, 4.0))));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn inner_radius_and_concentric_width_are_validated() {
        let a = cube(Vec3d::zero(), Vec3d::new(128.0, 16.0, 8.0), "a");
        let b = cube(Vec3d::zero(), Vec3d::new(24.0, 96.0, 8.0), "b");
        for radius in [0.0, -1.0, 81.0, f64::NAN, f64::INFINITY] {
            assert!(preview_with_rounding(
                &a,
                &b,
                Rounding {
                    inner: Some((radius, 8)),
                    ..Default::default()
                }
            )
            .is_err());
        }
        for segments in [0, 65] {
            assert!(preview_with_rounding(
                &a,
                &b,
                Rounding {
                    inner: Some((8.0, segments)),
                    ..Default::default()
                }
            )
            .is_err());
        }
        assert!(preview_with_rounding(
            &a,
            &b,
            Rounding {
                outer: Some((8.0, 8)),
                inner: Some((8.0, 8)),
                concentric: true,
                wedges: false,
                preserve_thickness: false
            }
        )
        .is_err());
        let b = cube(Vec3d::zero(), Vec3d::new(16.0, 96.0, 8.0), "b");
        assert!(preview_with_rounding(
            &a,
            &b,
            Rounding {
                inner: Some((8.0, 8)),
                concentric: true,
                ..Default::default()
            }
        )
        .is_err());
        for segments in [1, 64] {
            let join = preview_with_rounding(
                &a,
                &b,
                Rounding {
                    outer: Some((8.0, 8)),
                    inner: Some((8.0, segments)),
                    concentric: true,
                    wedges: false,
                    preserve_thickness: false,
                },
            )
            .unwrap();
            assert_eq!(join.added.len(), segments);
        }
    }
    #[test]
    fn unsupported_junctions_are_rejected() {
        let a = cube(Vec3d::zero(), Vec3d::new(128.0, 16.0, 8.0), "a");
        for (min, max) in [
            (Vec3d::new(32.0, 0.0, 0.0), Vec3d::new(48.0, 96.0, 8.0)), // T
            (Vec3d::new(0.0, 16.0, 0.0), Vec3d::new(16.0, 96.0, 8.0)), // only touching
            (Vec3d::zero(), Vec3d::new(16.0, 96.0, 12.0)),             // different depth
            (Vec3d::zero(), Vec3d::new(128.0, 16.0, 8.0)),             // identical
        ] {
            assert!(preview(&a, &cube(min, max, "b")).is_err());
        }
    }
}
