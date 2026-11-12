//! Complementary mitre cuts for two overlapping members whose overlap is an axis-aligned
//! block in both. Away from the corner a member may have any convex shape, such as an
//! end already mitred by an earlier join.
use geometry_math::{
    bbox::Bbox3d,
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
    /// How many added brushes snapping to the corner grid left without volume, and so
    /// left out.
    pub snapped_away: usize,
    /// What kind of meeting this is.
    pub kind: Junction,
    /// For a crossing, which of the two `added` is cut from: its pieces go beside it.
    pub split: Option<usize>,
    /// A choice the join made that the user should hear about.
    pub note: Option<&'static str>,
}
/// How two members meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Junction {
    /// Ends overlapping: an L, mitred.
    Corner,
    /// One member ends inside the other's side: it is cut flush with that side.
    Tee,
    /// The members pass through each other: one stays whole and the other is cut in
    /// two, each piece flush with a side of the whole one.
    Crossing,
}

/// How much two convex brushes share: one cut by every face plane of the other.
pub(crate) fn shared_volume(a: &Brush, b: &Brush) -> f64 {
    let mut inside = a.clone();
    for plane in b.face_planes() {
        match inside.clip(&plane) {
            ClipOutcome::Clipped(rest) => inside = rest,
            ClipOutcome::Unchanged => {}
            ClipOutcome::Empty => return 0.0,
        }
    }
    inside.volume()
}

/// `brush` with what lies behind `plane` cut away: the part in front of it.
fn in_front_of(brush: &Brush, plane: &Plane3d) -> Option<Brush> {
    match brush.clip(&Plane3d::from_anchor_and_normal(
        plane.anchor(),
        -plane.normal,
    )) {
        ClipOutcome::Clipped(piece) => Some(piece),
        _ => None,
    }
}

/// A T or a crossing, for two members that do not meet as an L. A T is a member (the
/// stem) that loses exactly the shared volume when cut flush with one face of the
/// other (the bar); a crossing is a member that loses it when cut flush with two
/// opposite faces of the other, which leaves two pieces. Volumes are compared with the
/// same tolerance as the L check, so grid noise does not decide.
fn tee_or_crossing(a: &Brush, b: &Brush) -> Option<Join> {
    let shared = shared_volume(a, b);
    if shared <= EPS {
        return None;
    }
    let close = |x: f64, y: f64| (x - y).abs() <= y.abs().max(1.0) * 1e-7;
    let axes = view_axes(&[a.clone(), b.clone()]);
    let make = |after: [Brush; 2], added: Vec<Brush>, kind, split, note, plane| Join {
        before: [a.clone(), b.clone()],
        after,
        added,
        axes,
        plane,
        removed_volume: shared,
        snapped_away: 0,
        kind,
        split,
        note,
    };
    // A T: the stem ends inside the bar.
    let mut tees = Vec::new();
    for (stem, bar) in [(1, 0), (0, 1)] {
        let (stem_brush, bar_brush) = if stem == 0 { (a, b) } else { (b, a) };
        let wanted = stem_brush.volume() - shared;
        for plane in bar_brush.face_planes() {
            if let Some(piece) = in_front_of(stem_brush, &plane) {
                if close(piece.volume(), wanted) {
                    tees.push((stem, bar, piece, plane));
                    break;
                }
            }
        }
    }
    if let Some((stem, _bar, piece, plane)) = tees.first().cloned() {
        let mut after = [a.clone(), b.clone()];
        after[stem] = piece;
        let note = (tees.len() > 1)
            .then_some("Either brush could be the stem of this T; the second one was cut.");
        return Some(make(after, Vec::new(), Junction::Tee, None, note, plane));
    }
    // A crossing: one member passes through the other.
    let mut crossings = Vec::new();
    for (cut, whole) in [(1, 0), (0, 1)] {
        let (cut_brush, whole_brush) = if cut == 0 { (a, b) } else { (b, a) };
        let wanted = cut_brush.volume() - shared;
        let planes = whole_brush.face_planes();
        'faces: for (i, first) in planes.iter().enumerate() {
            for second in &planes[i + 1..] {
                if vec::dot(first.normal, second.normal) > -1.0 + 1e-9 {
                    continue;
                }
                if let (Some(one), Some(two)) = (
                    in_front_of(cut_brush, first),
                    in_front_of(cut_brush, second),
                ) {
                    if close(one.volume() + two.volume(), wanted) {
                        crossings.push((cut, whole, one, two, *first));
                        break 'faces;
                    }
                }
            }
        }
    }
    // Either member could be cut through: the longer one stays whole, as a through wall
    // does; with two of a length the first selected does.
    let pick = match crossings.len() {
        0 => return None,
        1 => 0,
        _ => {
            let whole = |c: &(usize, usize, Brush, Brush, Plane3d)| {
                let size = [a, b][c.1].bounds().size();
                size.x().max(size.y()).max(size.z())
            };
            if whole(&crossings[1]) > whole(&crossings[0]) + EPS {
                1
            } else {
                0
            }
        }
    };
    let note = (crossings.len() > 1).then_some(
        "Either brush could be cut through at this crossing; the longer one stays whole.",
    );
    let (cut, _whole, one, two, plane) = crossings.swap_remove(pick);
    let mut after = [a.clone(), b.clone()];
    after[cut] = one;
    Some(make(
        after,
        vec![two],
        Junction::Crossing,
        Some(cut),
        note,
        plane,
    ))
}

/// The two axes to look at a set of members along: the depth axis left out is the one
/// they all share, bottom and top, as an L's members do; failing that, the one they
/// span least.
pub fn view_axes(brushes: &[Brush]) -> [usize; 2] {
    let Some(first) = brushes.first().map(Brush::bounds) else {
        return [0, 1];
    };
    let union = brushes
        .iter()
        .skip(1)
        .fold(first, |all, b| all.merged(&b.bounds()));
    let extent = |axis: usize| union.max[axis] - union.min[axis];
    let shared = |axis: usize| {
        brushes.iter().all(|b| {
            (b.bounds().min[axis] - first.min[axis]).abs() < EPS
                && (b.bounds().max[axis] - first.max[axis]).abs() < EPS
        })
    };
    let depth = (0..3)
        .filter(|axis| shared(*axis))
        .min_by(|a, b| extent(*a).total_cmp(&extent(*b)))
        .unwrap_or_else(|| {
            (0..3)
                .min_by(|a, b| extent(*a).total_cmp(&extent(*b)))
                .expect("three axes")
        });
    let mut axes = [0, 1, 2].into_iter().filter(|axis| *axis != depth);
    [axes.next().expect("two"), axes.next().expect("two")]
}

/// The members joined however they meet: an L is mitred, a T and a crossing are cut
/// flush. See [`preview`] and [`Junction`].
pub fn preview_junction(a: &Brush, b: &Brush) -> Result<Join, String> {
    match preview(a, b) {
        Ok(join) => Ok(join),
        Err(why) => tee_or_crossing(a, b).ok_or(why),
    }
}
/// The block where the two members' bounds overlap, when it has volume.
fn overlap_block(aa: &Bbox3d, bb: &Bbox3d) -> Option<Bbox3d> {
    let mut block = *aa;
    for axis in 0..3 {
        block.min[axis] = aa.min[axis].max(bb.min[axis]);
        block.max[axis] = aa.max[axis].min(bb.max[axis]);
    }
    (0..3)
        .all(|axis| block.max[axis] - block.min[axis] > EPS)
        .then_some(block)
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
    let aa = a.bounds();
    let bb = b.bounds();
    // The cut runs only through the block where the bounds overlap, so that block has
    // to be solid in both members. The rest of each may be any shape.
    let block = overlap_block(&aa, &bb);
    if block.is_some_and(|block| !a.contains_bbox(&block) || !b.contains_bbox(&block)) {
        return Err("The corner part of both brushes must be an axis-aligned rectangular block; rotated or slanted members where they overlap are not supported.".into());
    }
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
                snapped_away: 0,
                kind: Junction::Corner,
                split: None,
                note: None,
            });
        }
    }
    Err("Select two overlapping members that meet as an L, a T or a crossing, with the same depth. Touching members are not joined.".into())
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
    /// The grid the corner's new vertices snap to, in the corner's plane, if any.
    pub snap: Option<f64>,
}

/// The joined corner with `rounding`, its new vertices snapped to `rounding.snap`.
pub fn preview_with_rounding(a: &Brush, b: &Brush, rounding: Rounding) -> Result<Join, String> {
    let mut join = rounded(a, b, rounding)?;
    if let Some(size) = rounding.snap {
        snap(&mut join, size)?;
    }
    Ok(join)
}

/// Moves the vertices the join made onto a grid `size` apart, in the corner's plane only,
/// so the depth of the members stays as it was. A member's own corners away from the join
/// stay put. Neighbouring pieces share their vertices and snap them alike, so no gap
/// opens between them; an added piece left without volume is dropped and counted.
fn snap(join: &mut Join, size: f64) -> Result<(), String> {
    if !size.is_finite() || size <= 0.0 {
        return Err("Use a positive corner grid.".into());
    }
    // A T or a crossing only cuts along the other member's faces: no new corner to snap.
    if join.kind != Junction::Corner {
        return Ok(());
    }
    let [u, v] = join.axes;
    let place = |mut p: Vec3d| {
        p[u] = (p[u] / size).round() * size;
        p[v] = (p[v] / size).round() * size;
        p
    };
    for (after, before) in join.after.iter_mut().zip(&join.before) {
        let old = before.vertex_positions();
        let made = |p: Vec3d| !old.iter().any(|o| vec::squared_distance(*o, p) < EPS * EPS);
        *after = after
            .map_vertices(|p| if made(p) { place(p) } else { p })
            .map_err(|_| "The corner grid is too coarse for this corner.".to_string())?;
    }
    let count = join.added.len();
    join.added = std::mem::take(&mut join.added)
        .into_iter()
        .filter_map(|brush| brush.map_vertices(place).ok())
        .filter(|brush| brush.volume() > 1e-9)
        .collect();
    join.snapped_away = count - join.added.len();
    Ok(())
}

/// Adds a concave quarter-circle fillet as non-overlapping triangular prisms.
fn rounded(a: &Brush, b: &Brush, rounding: Rounding) -> Result<Join, String> {
    // Rounding shapes an L's corner; a T or a crossing is only cut flush.
    let plain = !rounding.wedges
        && !rounding.concentric
        && rounding.outer.is_none()
        && rounding.inner.is_none();
    if plain {
        return preview_junction(a, b);
    }
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
                        snap: None,
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
    fn snapping_puts_the_corner_on_its_grid_and_keeps_the_depth() {
        let a = cube(Vec3d::zero(), Vec3d::new(256.0, 32.0, 100.0), "a");
        let b = cube(Vec3d::zero(), Vec3d::new(32.0, 256.0, 100.0), "b");
        let rounding = Rounding {
            outer: Some((28.0, 8)),
            inner: Some((24.0, 8)),
            ..Rounding::default()
        };
        let volume = |join: &Join| -> f64 {
            join.after
                .iter()
                .chain(&join.added)
                .map(|brush| brush.volume())
                .sum()
        };
        let free = preview_with_rounding(&a, &b, rounding).unwrap();
        let snapped = preview_with_rounding(
            &a,
            &b,
            Rounding {
                snap: Some(1.0),
                ..rounding
            },
        )
        .unwrap();
        for brush in snapped.after.iter().chain(&snapped.added) {
            for p in brush.vertex_positions() {
                let on = |x: f64, step: f64| (x / step - (x / step).round()).abs() < 1e-6;
                assert!(on(p[0], 1.0) && on(p[1], 1.0), "off the grid: {p:?}");
                assert!(on(p[2], 100.0), "depth moved: {p:?}");
            }
        }
        // Near an arc's ends one coordinate moves less than half a step, so the first
        // pieces flatten into the wall even on a fine grid.
        assert_eq!(snapped.added.len() + snapped.snapped_away, free.added.len());
        assert!((volume(&snapped) - volume(&free)).abs() < volume(&free) * 0.01);

        // Arc points a coarse grid puts on top of each other leave pieces without volume.
        let coarse = preview_with_rounding(
            &a,
            &b,
            Rounding {
                snap: Some(32.0),
                ..rounding
            },
        )
        .unwrap();
        assert!(coarse.snapped_away > snapped.snapped_away);
        assert_eq!(coarse.added.len() + coarse.snapped_away, free.added.len());
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
                                    snap: None,
                                },
                                Rounding {
                                    outer: Some((1.0, 1)),
                                    inner: Some((48.0, 8)),
                                    concentric: true,
                                    wedges: false,
                                    preserve_thickness: false,
                                    snap: None,
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
                preserve_thickness: false,
                snap: None,
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
                    snap: None,
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
        // A T is not an L, but it is a junction, and so is an L of a thinner member
        // ending inside a deeper one: it butts against it. Touching and identical
        // members are not.
        for (min, max) in [
            (Vec3d::new(32.0, 0.0, 0.0), Vec3d::new(48.0, 96.0, 8.0)),
            (Vec3d::zero(), Vec3d::new(16.0, 96.0, 12.0)),
        ] {
            let join = preview_junction(&a, &cube(min, max, "b")).unwrap();
            assert_eq!(join.kind, Junction::Tee);
        }
        for (min, max) in [
            (Vec3d::new(0.0, 16.0, 0.0), Vec3d::new(16.0, 96.0, 8.0)),
            (Vec3d::zero(), Vec3d::new(128.0, 16.0, 8.0)),
        ] {
            assert!(preview_junction(&a, &cube(min, max, "b")).is_err());
        }
    }

    /// The gate frame from the owner's `join_corner.map`: a left post and the beam
    /// already joined with a mitre, and a plain right post overlapping the beam's other
    /// end.
    const GATE: &str = r#"{
"classname" "worldspawn"
{
( -128 16 128 ) ( -128 16 0 ) ( -128 32 0 ) a [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -112 32 112 ) ( -112 32 0 ) ( -112 16 0 ) a [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -128 16 0 ) ( -128 16 128 ) ( -112 16 112 ) a [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -128 32 128 ) ( -128 32 0 ) ( -112 32 0 ) a [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -112 32 0 ) ( -128 32 0 ) ( -128 16 0 ) a [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( -112 32 112 ) ( -112 16 112 ) ( -128 16 128 ) a [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
{
( 128 32 128 ) ( 128 32 112 ) ( 128 16 112 ) b [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -128 16 128 ) ( 128 16 128 ) ( 128 16 112 ) b [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -128 32 128 ) ( -112 32 112 ) ( 128 32 112 ) b [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 128 16 128 ) ( -128 16 128 ) ( -128 32 128 ) b [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( -112 16 112 ) ( 128 16 112 ) ( 128 32 112 ) b [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( -128 32 128 ) ( -128 16 128 ) ( -112 16 112 ) b [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
{
( 112 16 0 ) ( 112 17 0 ) ( 112 16 1 ) c [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 128 16 0 ) ( 128 16 1 ) ( 128 17 0 ) c [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 112 16 0 ) ( 112 16 1 ) ( 113 16 0 ) c [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 112 32 0 ) ( 113 32 0 ) ( 112 32 1 ) c [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 112 16 0 ) ( 113 16 0 ) ( 112 17 0 ) c [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 112 16 128 ) ( 112 17 128 ) ( 113 16 128 ) c [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
}
}
"#;

    fn gate() -> Vec<Brush> {
        let map = map_model::io::map_reader::read_map(GATE, None).unwrap();
        map.entities[0]
            .brushes
            .iter()
            .map(|b| {
                Brush::from_faces(&Bbox3d::from_scalars(-8192.0, 8192.0), b.faces.clone()).unwrap()
            })
            .collect()
    }

    /// The vertices of a brush, rounded and sorted, to compare shapes.
    fn corners(brush: &Brush) -> Vec<[i64; 3]> {
        let mut out: Vec<[i64; 3]> = brush
            .vertex_positions()
            .iter()
            .map(|v| {
                [
                    v.x().round() as i64,
                    v.y().round() as i64,
                    v.z().round() as i64,
                ]
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn a_member_mitred_at_its_far_end_joins_at_its_other_end() {
        let brushes = gate();
        // The beam has a mitre face at its left end, so it is not a box.
        assert!(brushes[1].face_planes().iter().any(|p| p
            .normal
            .v
            .iter()
            .filter(|n| n.abs() > EPS)
            .count()
            > 1));
        let join = preview(&brushes[1], &brushes[2]).unwrap();
        assert!((join.removed_volume - 16.0 * 16.0 * 16.0).abs() < 1e-6);
        // The right end comes out as the mirror image of the left one.
        let mirror = |corners: Vec<[i64; 3]>| {
            let mut out: Vec<[i64; 3]> = corners.into_iter().map(|[x, y, z]| [-x, y, z]).collect();
            out.sort();
            out
        };
        assert_eq!(corners(&join.after[1]), mirror(corners(&brushes[0])));
        let beam = corners(&join.after[0]);
        assert_eq!(beam, mirror(beam.clone()), "{beam:?}");
        // Either order of selection gives the same pieces.
        let swapped = preview(&brushes[2], &brushes[1]).unwrap();
        assert_eq!(corners(&swapped.after[0]), corners(&join.after[1]));
        assert_eq!(corners(&swapped.after[1]), corners(&join.after[0]));
    }

    #[test]
    fn joining_two_boxes_gives_the_same_mitre_as_before() {
        // The left pair of the gate before it was joined: the result is what the owner's
        // map holds for them.
        let post = cube(
            Vec3d::new(-128.0, 16.0, 0.0),
            Vec3d::new(-112.0, 32.0, 128.0),
            "a",
        );
        let beam = cube(
            Vec3d::new(-128.0, 16.0, 112.0),
            Vec3d::new(128.0, 32.0, 128.0),
            "b",
        );
        let join = preview(&post, &beam).unwrap();
        let brushes = gate();
        assert_eq!(corners(&join.after[0]), corners(&brushes[0]));
        let left_end =
            |b: &Brush| -> Vec<[i64; 3]> { corners(b).into_iter().filter(|c| c[0] < 0).collect() };
        assert_eq!(left_end(&join.after[1]), left_end(&brushes[1]));
    }

    #[test]
    fn slanted_faces_away_from_the_corner_are_allowed_and_inside_it_are_refused() {
        let post = cube(
            Vec3d::new(112.0, 16.0, 0.0),
            Vec3d::new(128.0, 32.0, 128.0),
            "c",
        );
        let beam = cube(
            Vec3d::new(-128.0, 16.0, 112.0),
            Vec3d::new(128.0, 32.0, 128.0),
            "b",
        );
        let cut = |brush: &Brush, anchor: Vec3d, normal: Vec3d| match brush.clip(
            &Plane3d::from_anchor_and_normal(anchor, vec::normalize(normal)),
        ) {
            ClipOutcome::Clipped(b) => b,
            _ => panic!("the cut changes the brush"),
        };
        // The post's foot bevelled, the beam's far end cut on a slant in plan.
        let post_bevelled = cut(
            &post,
            Vec3d::new(112.0, 16.0, 8.0),
            Vec3d::new(-1.0, 0.0, -1.0),
        );
        let beam_slanted = cut(
            &beam,
            Vec3d::new(-120.0, 16.0, 112.0),
            Vec3d::new(-1.0, -1.0, 0.0),
        );
        for (a, b) in [
            (&post_bevelled, &beam),
            (&post, &beam_slanted),
            (&post_bevelled, &beam_slanted),
        ] {
            let join = preview(a, b).unwrap();
            assert!((join.removed_volume - 16.0 * 16.0 * 16.0).abs() < 1e-6);
            let volume = a.volume() + b.volume() - join.removed_volume;
            assert!((join.after[0].volume() + join.after[1].volume() - volume).abs() < 1e-6);
        }
        // A bevel through the corner block itself leaves nothing to mitre.
        let corner_bevelled = cut(
            &beam,
            Vec3d::new(124.0, 16.0, 128.0),
            Vec3d::new(1.0, 0.0, 1.0),
        );
        let error = preview(&post, &corner_bevelled).err().unwrap();
        assert!(error.contains("rectangular block"), "{error}");
        // So does a member turned about the depth axis.
        use geometry_math::mat_ext::{rotation_matrix_about_axis, translation_matrix};
        let pivot = Vec3d::new(120.0, 24.0, 120.0);
        let turn = translation_matrix(pivot)
            * rotation_matrix_about_axis(Vec3d::new(0.0, 1.0, 0.0), 0.1)
            * translation_matrix(-pivot);
        let turned = beam
            .transform(&Bbox3d::from_scalars(-8192.0, 8192.0), &turn)
            .unwrap();
        let error = preview(&post, &turned).err().unwrap();
        assert!(error.contains("rectangular block"), "{error}");
    }

    /// The pieces of a join: each brush after it, then what it added.
    fn pieces(join: &Join) -> Vec<Brush> {
        join.after.iter().chain(&join.added).cloned().collect()
    }

    /// No two pieces share volume and together they hold what the members did, less
    /// the overlap once.
    fn tidy(join: &Join) {
        let all = pieces(join);
        for (i, x) in all.iter().enumerate() {
            for y in &all[i + 1..] {
                assert!(shared_volume(x, y) < 1e-6);
            }
        }
        let before: f64 = join.before.iter().map(Brush::volume).sum();
        let after: f64 = all.iter().map(Brush::volume).sum();
        assert!((before - join.removed_volume - after).abs() < 1e-6);
    }

    #[test]
    fn a_tee_cuts_the_stem_flush_with_the_bar_in_every_orientation() {
        let bounds = Bbox3d::from_scalars(-8192.0, 8192.0);
        // A bar along X, 16 deep, and stems of two widths meeting it from either side;
        // turned a quarter about Z the bar runs along Y.
        for turn in 0..4 {
            let rotation = geometry_math::mat_ext::rotation_matrix_about_axis(
                Vec3d::new(0.0, 0.0, 1.0),
                std::f64::consts::FRAC_PI_2 * f64::from(turn),
            );
            let turned = |b: Brush| b.transform(&bounds, &rotation).unwrap();
            for (width, from, to) in [(16.0, 8.0, 96.0), (32.0, -96.0, 12.0)] {
                let bar = turned(cube(
                    Vec3d::new(-128.0, 0.0, 0.0),
                    Vec3d::new(128.0, 16.0, 16.0),
                    "bar",
                ));
                let stem = turned(cube(
                    Vec3d::new(-width / 2.0, from, 0.0),
                    Vec3d::new(width / 2.0, to, 16.0),
                    "stem",
                ));
                for (first, second) in [(&bar, &stem), (&stem, &bar)] {
                    let join = preview_junction(first, second).unwrap();
                    assert_eq!(join.kind, Junction::Tee);
                    tidy(&join);
                    // The bar is left whole.
                    let whole = if std::ptr::eq(first, &bar) { 0 } else { 1 };
                    assert_eq!(join.after[whole].faces(), bar.faces());
                    assert!(join.added.is_empty());
                }
            }
        }
    }

    #[test]
    fn a_crossing_cuts_the_shorter_member_in_two_and_keeps_the_longer_whole() {
        let long = cube(
            Vec3d::new(-128.0, -8.0, 0.0),
            Vec3d::new(128.0, 8.0, 16.0),
            "long",
        );
        for width in [8.0, 16.0, 40.0] {
            let short = cube(
                Vec3d::new(-width / 2.0, -64.0, 0.0),
                Vec3d::new(width / 2.0, 64.0, 16.0),
                "short",
            );
            for (first, second, cut) in [(&long, &short, 1), (&short, &long, 0)] {
                let join = preview_junction(first, second).unwrap();
                assert_eq!(join.kind, Junction::Crossing);
                assert_eq!(join.split, Some(cut));
                assert_eq!(join.after[1 - cut].faces(), long.faces());
                assert_eq!(join.added.len(), 1);
                tidy(&join);
                // Each piece ends flush with one side of the long member.
                let ends: Vec<f64> = [&join.after[cut], &join.added[0]]
                    .iter()
                    .map(|b| {
                        let bb = b.bounds();
                        if bb.max.y() <= -8.0 + 1e-9 {
                            bb.max.y()
                        } else {
                            bb.min.y()
                        }
                    })
                    .collect();
                assert!(
                    ends.iter().any(|e| (e + 8.0).abs() < 1e-6)
                        && ends.iter().any(|e| (e - 8.0).abs() < 1e-6),
                    "{ends:?}"
                );
            }
        }
        // Two members of one size: the first selected stays whole, and the join says so.
        let across = cube(
            Vec3d::new(-8.0, -128.0, 0.0),
            Vec3d::new(8.0, 128.0, 16.0),
            "b",
        );
        let join = preview_junction(&long, &across).unwrap();
        assert_eq!(join.split, Some(1));
        assert!(join.note.is_some());
    }

    #[test]
    fn rounding_is_refused_for_a_tee_rather_than_guessed() {
        let bar = cube(
            Vec3d::new(-128.0, 0.0, 0.0),
            Vec3d::new(128.0, 16.0, 16.0),
            "bar",
        );
        let stem = cube(
            Vec3d::new(-8.0, 8.0, 0.0),
            Vec3d::new(8.0, 96.0, 16.0),
            "stem",
        );
        let rounding = Rounding {
            outer: Some((8.0, 4)),
            ..Default::default()
        };
        assert!(preview_with_rounding(&bar, &stem, rounding).is_err());
        let snapped = Rounding {
            snap: Some(16.0),
            ..Default::default()
        };
        assert_eq!(
            preview_with_rounding(&bar, &stem, snapped).unwrap().kind,
            Junction::Tee
        );

    }
}
