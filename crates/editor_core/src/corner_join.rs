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
    /// For each of `added`, the member it is a part of, when it is one: the parts of a
    /// taller member above and below the depth both share, left as they were. Shorter
    /// than `added` (or empty) means the rest belong to neither alone.
    pub owners: Vec<Option<usize>>,
}

impl Join {
    /// The member `added[index]` is a part of, if it is one.
    pub fn owner_of(&self, index: usize) -> Option<usize> {
        self.owners.get(index).copied().flatten()
    }
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
        owners: Vec::new(),
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
                owners: Vec::new(),
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
    /// Carry textures across the joints with [`continue_textures`].
    pub textures: bool,
}

/// The joined corner with `rounding`, its new vertices snapped to `rounding.snap`, and
/// every face the join made that another piece covers whole given the skip material.
pub fn preview_with_rounding(a: &Brush, b: &Brush, rounding: Rounding) -> Result<Join, String> {
    match join_once(a, b, rounding) {
        // An L of a lower wall against a higher one reads as a T, the lower one butting
        // against the higher: a mitre over the height they share is what was meant.
        Ok(join) if join.kind != Junction::Corner => match by_shared_depth(a, b, rounding) {
            Some(Ok(shared)) if shared.kind == Junction::Corner => Ok(shared),
            _ => Ok(join),
        },
        Ok(join) => Ok(join),
        Err(why) => by_shared_depth(a, b, rounding).unwrap_or(Err(why)),
    }
}

/// Members of different heights (or depths): the part of each within the depth both
/// share is joined as members of one height are, and what a taller member has above or
/// below it is split off and left as it was, so a 128 high wall meeting a 96 high one is
/// mitred from 0 to 96 and keeps its full end above. The upright axis is tried first.
/// The faces where a member was split get skip where the pieces on the other side cover
/// them whole. `None` when the members share all their depth on every axis or no
/// volume at all, so the plain refusal stands.
fn by_shared_depth(a: &Brush, b: &Brush, rounding: Rounding) -> Option<Result<Join, String>> {
    if shared_volume(a, b) <= EPS {
        return None;
    }
    let (aa, bb) = (a.bounds(), b.bounds());
    let mut first_error = None;
    for d in [2, 0, 1] {
        let (lo, hi) = (aa.min[d].max(bb.min[d]), aa.max[d].min(bb.max[d]));
        if hi - lo <= EPS
            || ((aa.min[d] - bb.min[d]).abs() <= EPS && (aa.max[d] - bb.max[d]).abs() <= EPS)
        {
            continue;
        }
        let mut up = Vec3d::zero();
        up[d] = 1.0;
        let mut at_hi = Vec3d::zero();
        at_hi[d] = hi;
        let mut at_lo = Vec3d::zero();
        at_lo[d] = lo;
        let top = Plane3d::from_anchor_and_normal(at_hi, up);
        let bottom = Plane3d::from_anchor_and_normal(at_lo, -up);
        // The member within [lo, hi], and what it has beyond.
        let slab = |brush: &Brush| -> Option<(Brush, Vec<Brush>)> {
            let bounds = brush.bounds();
            let mut middle = brush.clone();
            let mut rest = Vec::new();
            if bounds.max[d] > hi + EPS {
                rest.push(in_front_of(brush, &top)?);
                middle = match middle.clip(&top) {
                    ClipOutcome::Clipped(piece) => piece,
                    ClipOutcome::Unchanged => middle,
                    ClipOutcome::Empty => return None,
                };
            }
            if bounds.min[d] < lo - EPS {
                rest.push(in_front_of(brush, &bottom)?);
                middle = match middle.clip(&bottom) {
                    ClipOutcome::Clipped(piece) => piece,
                    ClipOutcome::Unchanged => middle,
                    ClipOutcome::Empty => return None,
                };
            }
            Some((middle, rest))
        };
        let (Some((a_mid, a_rest)), Some((b_mid, b_rest))) = (slab(a), slab(b)) else {
            continue;
        };
        let mut join = match join_once(&a_mid, &b_mid, rounding) {
            Ok(join) => join,
            Err(why) => {
                first_error.get_or_insert(why);
                continue;
            }
        };
        join.before = [a.clone(), b.clone()];
        join.owners = vec![None; join.added.len()];
        for (member, rest) in [(0, a_rest), (1, b_rest)] {
            for piece in rest {
                join.added.push(piece);
                join.owners.push(Some(member));
            }
        }
        hide_split_faces(&mut join, &[top, bottom]);
        return Some(Ok(join));
    }
    first_error.map(Err)
}

/// Skip on the faces lying in a split plane (either way round) that the pieces on the
/// other side of it cover whole, together: the top of a taller wall's lower part and
/// the bottom of its upper part, which rests on that and on the shorter wall.
fn hide_split_faces(join: &mut Join, planes: &[Plane3d]) {
    let pieces: Vec<Brush> = join.after.iter().chain(&join.added).cloned().collect();
    let in_split = |plane: &Plane3d| {
        planes.iter().any(|split| {
            vec::length(split.normal - plane.normal) < 1e-6
                && (split.distance - plane.distance).abs() < 1e-4
                || vec::length(split.normal + plane.normal) < 1e-6
                    && (split.distance + plane.distance).abs() < 1e-4
        })
    };
    let mut updated = pieces.clone();
    for (index, piece) in pieces.iter().enumerate() {
        let mut faces = piece.faces().to_vec();
        let mut changed = false;
        for (face_index, face) in piece.faces().iter().enumerate() {
            if !in_split(&face.plane) || face.material_name.as_str() == SKIP {
                continue;
            }
            let Some(polygon) = piece.face_polygon(face_index) else {
                continue;
            };
            let area = overlap_area(&polygon, &polygon, face.plane.normal);
            let covered: f64 = pieces
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .flat_map(|(_, brush)| {
                    (0..brush.faces().len()).filter_map(move |i| {
                        let other = &brush.faces()[i].plane;
                        (vec::length(other.normal + face.plane.normal) < 1e-6
                            && (other.distance + face.plane.distance).abs() < 1e-4)
                            .then(|| brush.face_polygon(i))
                            .flatten()
                    })
                })
                .map(|outline| overlap_area(&polygon, &outline, face.plane.normal))
                .sum();
            if covered >= area - 1e-3 {
                faces[face_index].material_name = SKIP.into();
                changed = true;
            }
        }
        if changed {
            if let Some(brush) = piece.with_exact_face_attributes(&faces) {
                updated[index] = brush;
            }
        }
    }
    let mut updated = updated.into_iter();
    join.after = [updated.next().expect("two"), updated.next().expect("two")];
    join.added = updated.collect();
}

/// A straight wall seen along `depth`: an upright prism (two faces facing along `depth`,
/// the rest across it) whose outline is a four-sided figure with two long parallel
/// sides. Returns the two long sides' lines, each as a point and a unit direction in
/// the plane of the other two axes, and the outline's centre there.
/// A line in a plane, as a point on it and a unit direction.
type Line2 = ([f64; 2], [f64; 2]);

fn wall_lines(brush: &Brush, [u, v]: [usize; 2]) -> Option<([Line2; 2], [f64; 2])> {
    let depth = 3 - u - v;
    let mut cap = None;
    for (index, face) in brush.faces().iter().enumerate() {
        let n = face.plane.normal;
        if n[depth].abs() > 1.0 - 1e-9 {
            if n[depth] > 0.0 {
                cap = brush.face_polygon(index);
            }
        } else if n[depth].abs() > 1e-9 {
            return None;
        }
    }
    let outline: Vec<[f64; 2]> = cap?.iter().map(|p| [p[u], p[v]]).collect();
    if outline.len() != 4 {
        return None;
    }
    let side = |k: usize| {
        let (p, q) = (outline[k], outline[(k + 1) % 4]);
        let d = [q[0] - p[0], q[1] - p[1]];
        let length = d[0].hypot(d[1]);
        (p, [d[0] / length, d[1] / length], length)
    };
    // The longer pair of opposite sides, which must be parallel.
    let pair = if side(0).2 + side(2).2 >= side(1).2 + side(3).2 {
        0
    } else {
        1
    };
    let (first, second) = (side(pair), side(pair + 2));
    let cross = first.1[0] * second.1[1] - first.1[1] * second.1[0];
    if cross.abs() > 1e-9 {
        return None;
    }
    let centre = outline
        .iter()
        .fold([0.0, 0.0], |c, p| [c[0] + p[0] / 4.0, c[1] + p[1] / 4.0]);
    Some(([(first.0, first.1), (second.0, first.1)], centre))
}

/// Where two lines, each a point and a direction, cross.
fn crossing_point(a: Line2, b: Line2) -> Option<[f64; 2]> {
    let (p, r) = a;
    let (q, s) = b;
    let denominator = r[0] * s[1] - r[1] * s[0];
    if denominator.abs() < 1e-9 {
        return None;
    }
    let t = ((q[0] - p[0]) * s[1] - (q[1] - p[1]) * s[0]) / denominator;
    Some([p[0] + r[0] * t, p[1] + r[1] * t])
}

/// Two straight walls meeting at an angle other than 90 degrees (rotated boxes): each is
/// cut by the mitre, the upright plane through the meeting of their outer faces and of
/// their inner faces, which halves the angle between walls of one thickness and runs
/// from the outer corner to the inner one otherwise. What either wall had beyond the
/// mitre goes, so ends drawn past the outer corner are trimmed. Both walls must share
/// their depth, and each one's end must reach right across the mitre.
fn angled(a: &Brush, b: &Brush) -> Result<Join, String> {
    let refused = || {
        "Select two straight walls that overlap at their ends and share their depth, at any angle."
            .to_string()
    };
    let shared = shared_volume(a, b);
    if shared <= EPS {
        return Err(refused());
    }
    let (aa, bb) = (a.bounds(), b.bounds());
    for depth in [2, 0, 1] {
        if (aa.min[depth] - bb.min[depth]).abs() > EPS
            || (aa.max[depth] - bb.max[depth]).abs() > EPS
        {
            continue;
        }
        let axes = match depth {
            0 => [1, 2],
            1 => [0, 2],
            _ => [0, 1],
        };
        let (Some((lines_a, centre_a)), Some((lines_b, centre_b))) =
            (wall_lines(a, axes), wall_lines(b, axes))
        else {
            continue;
        };
        let mut corners = Vec::with_capacity(4);
        for la in lines_a {
            for lb in lines_b {
                corners.push(crossing_point(la, lb).ok_or_else(refused)?);
            }
        }
        let middle = corners
            .iter()
            .fold([0.0, 0.0], |c, p| [c[0] + p[0] / 4.0, c[1] + p[1] / 4.0]);
        // Each wall's run, pointing from the corner along the wall.
        let run = |lines: [Line2; 2], centre: [f64; 2]| {
            let d = lines[0].1;
            let along = (centre[0] - middle[0]) * d[0] + (centre[1] - middle[1]) * d[1];
            if along < 0.0 {
                [-d[0], -d[1]]
            } else {
                d
            }
        };
        let (run_a, run_b) = (run(lines_a, centre_a), run(lines_b, centre_b));
        let towards = [run_a[0] + run_b[0], run_a[1] + run_b[1]];
        let score =
            |p: &[f64; 2]| (p[0] - middle[0]) * towards[0] + (p[1] - middle[1]) * towards[1];
        let inner = *corners
            .iter()
            .max_by(|p, q| score(p).total_cmp(&score(q)))
            .unwrap();
        let outer = *corners
            .iter()
            .min_by(|p, q| score(p).total_cmp(&score(q)))
            .unwrap();
        // Both walls' ends reach across the mitre: its two ends, nudged a hair inwards
        // and taken half way up, lie in both.
        let point = |p: [f64; 2]| {
            let mut out = Vec3d::zero();
            out[axes[0]] = p[0] + (middle[0] - p[0]) * 1e-4;
            out[axes[1]] = p[1] + (middle[1] - p[1]) * 1e-4;
            out[depth] = (aa.min[depth] + aa.max[depth]) / 2.0;
            out
        };
        if ![outer, inner]
            .iter()
            .all(|p| a.contains_point(point(*p)) && b.contains_point(point(*p)))
        {
            return Err(
                "Each wall's end has to reach right across the corner to be mitred at this angle."
                    .into(),
            );
        }
        // The mitre, facing away from the first wall.
        let along = [inner[0] - outer[0], inner[1] - outer[1]];
        let mut normal = Vec3d::zero();
        normal[axes[0]] = along[1];
        normal[axes[1]] = -along[0];
        normal = normal / vec::length(normal);
        let mut anchor = Vec3d::zero();
        anchor[axes[0]] = outer[0];
        anchor[axes[1]] = outer[1];
        anchor[depth] = aa.min[depth];
        let to_a =
            (centre_a[0] - outer[0]) * normal[axes[0]] + (centre_a[1] - outer[1]) * normal[axes[1]];
        if to_a > 0.0 {
            normal = -normal;
        }
        let plane = Plane3d::from_anchor_and_normal(anchor, normal);
        let opposite = Plane3d::from_anchor_and_normal(anchor, -normal);
        let (ClipOutcome::Clipped(first), ClipOutcome::Clipped(second)) =
            (a.clip(&plane), b.clip(&opposite))
        else {
            return Err("The corner could not be cut reliably at this scale.".into());
        };
        if first.volume() <= EPS || second.volume() <= EPS || shared_volume(&first, &second) > EPS {
            return Err("Corner validation failed; the original brushes were preserved.".into());
        }
        // An L, not a T: what either wall loses besides the shared part is only the tip
        // a rotated box's end has past the outer corner (a few times the shared part at
        // most), not a bar running on.
        let tip_a = a.volume() - first.volume() - shared;
        let tip_b = b.volume() - second.volume() - shared;
        if tip_a > 4.0 * shared + EPS || tip_b > 4.0 * shared + EPS {
            return Err(refused());
        }
        return Ok(Join {
            before: [a.clone(), b.clone()],
            after: [first, second],
            added: Vec::new(),
            axes,
            plane,
            removed_volume: shared,
            snapped_away: 0,
            kind: Junction::Corner,
            split: None,
            note: None,
            owners: Vec::new(),
        });
    }
    Err(refused())
}

/// The part two brushes share, if any.
pub(crate) fn shared_part(a: &Brush, b: &Brush) -> Option<Brush> {
    let mut inside = a.clone();
    for plane in b.face_planes() {
        match inside.clip(&plane) {
            ClipOutcome::Clipped(rest) => inside = rest,
            ClipOutcome::Unchanged => {}
            ClipOutcome::Empty => return None,
        }
    }
    (inside.volume() > EPS).then_some(inside)
}

/// One join of two members that share their depth: see [`preview_with_rounding`].
fn join_once(a: &Brush, b: &Brush, rounding: Rounding) -> Result<Join, String> {
    let square = |brush: &Brush| {
        brush.faces().iter().all(|face| {
            let n = face.plane.normal;
            (0..3).filter(|axis| n[*axis].abs() > 1e-9).count() == 1
        })
    };
    let mut join = match rounded(a, b, rounding) {
        // Rotated walls whose ends overlap can also pass for a T or a crossing; the
        // corner they make is what was meant. Square brushes keep what they had.
        Ok(join) if join.kind != Junction::Corner && !(square(a) && square(b)) => {
            match angled(a, b) {
                Ok(corner)
                    if rounding.outer.is_none()
                        && rounding.inner.is_none()
                        && !rounding.wedges
                        && !rounding.concentric =>
                {
                    corner
                }
                Ok(_) => return Err("Rounding is not supported for corners at angles other than 90 degrees; join this corner without rounding.".into()),
                Err(_) => join,
            }
        }
        Ok(join) => join,
        // Square brushes keep the refusal they had.
        Err(why) if square(a) && square(b) => return Err(why),
        // Not axis-aligned where they meet: walls at another angle than 90 degrees.
        Err(why) => match angled(a, b) {
            Ok(join)
                if rounding.outer.is_none()
                    && rounding.inner.is_none()
                    && !rounding.wedges
                    && !rounding.concentric =>
            {
                join
            }
            Ok(_) => {
                return Err("Rounding is not supported for corners at angles other than 90 degrees; join this corner without rounding.".into())
            }
            Err(angle_why) if angle_why.starts_with("Each wall") => return Err(angle_why),
            Err(_) => return Err(why),
        },
    };
    if let Some(size) = rounding.snap {
        snap(&mut join, size)?;
    }
    hide_covered_faces(&mut join);
    Ok(join)
}

/// What [`continue_textures`] did: faces it carried a neighbour's texture onto, and
/// meetings it left as they were because the two faces differ in material or scale, or
/// a face with fixed (paraxial) axes cannot turn the corner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextureSeams {
    pub aligned: usize,
    pub kept: usize,
}

/// Carries textures across the joints between `brushes`, so a pattern runs on without a
/// jump: a face lying in the same plane as its neighbour takes the neighbour's axes and
/// offset, world-continuous; a face turning a corner from it takes the neighbour's
/// mapping rotated about the shared edge, as if the texture were wrapped round the wall.
/// Scale and rotation stay the neighbour's, which is the user's. Paraxial faces (the
/// Standard format) cannot turn their axes, so they only take the neighbour's offset,
/// where that makes the two agree along the whole edge. Only visible faces that
/// meet along an edge count, and only when both carry the same material at the same
/// scale. Each connected run keeps its first face as it was, in the order of `brushes`,
/// a paraxial face before others;
/// a run that closes on itself, such as the outside of a room, keeps one seam where it
/// closes, since a loop's length is rarely a whole number of texture repeats.
pub fn continue_textures(brushes: &mut [Brush]) -> TextureSeams {
    use map_model::uv::UvCoordSystem;
    let mut seams = TextureSeams::default();
    // Every visible face with its outline.
    struct Face {
        brush: usize,
        index: usize,
        polygon: Vec<Vec3d>,
    }
    let mut faces: Vec<Face> = Vec::new();
    for (brush, b) in brushes.iter().enumerate() {
        for (index, face) in b.faces().iter().enumerate() {
            if face.material_name.as_str() == SKIP {
                continue;
            }
            if let Some(polygon) = b.face_polygon(index) {
                faces.push(Face {
                    brush,
                    index,
                    polygon,
                });
            }
        }
    }
    let bounds: Vec<Bbox3d> = brushes.iter().map(Brush::bounds).collect();
    let on_face = |b: &Brush, index: usize, polygon: &[Vec3d], p: Vec3d| {
        let plane = b.faces()[index].plane;
        plane.point_distance(p).abs() < 1e-4 && polygon_within(&[p], polygon, plane.normal)
    };
    // Whether the face is in the open just beside `at`, a point on its outline: a step
    // into the face across the outline and a hair out along its normal is in no brush.
    let open_beside = |f: &Face, at: Vec3d, along: Vec3d| -> bool {
        let normal = brushes[f.brush].faces()[f.index].plane.normal;
        let across = vec::normalize(vec::cross(normal, along));
        [across, -across].iter().any(|step| {
            let inside = at + *step * 0.05;
            polygon_within(&[inside], &f.polygon, normal)
                && !brushes
                    .iter()
                    .any(|b| b.contains_point(inside + normal * 0.01))
        })
    };
    // Which faces meet which: a stretch of one's outline lies on the other, and both
    // are in the open along it.
    let mut links: Vec<Vec<(usize, Vec3d, Vec3d)>> = vec![Vec::new(); faces.len()];
    for i in 0..faces.len() {
        for j in i + 1..faces.len() {
            let (f, g) = (&faces[i], &faces[j]);
            if f.brush == g.brush || !bounds[f.brush].expand(1e-3).intersects(&bounds[g.brush]) {
                continue;
            }
            let (nf, ng) = (
                brushes[f.brush].faces()[f.index].plane.normal,
                brushes[g.brush].faces()[g.index].plane.normal,
            );
            if vec::dot(nf, ng) < -0.5 {
                continue;
            }
            let shared = |from: &Face, onto: &Face| -> Option<(Vec3d, Vec3d)> {
                let n = from.polygon.len();
                (0..n).find_map(|k| {
                    let (p, q) = (from.polygon[k], from.polygon[(k + 1) % n]);
                    (vec::length(q - p) > 1e-3
                        && on_face(&brushes[onto.brush], onto.index, &onto.polygon, p)
                        && on_face(&brushes[onto.brush], onto.index, &onto.polygon, q))
                    .then(|| ((p + q) * 0.5, vec::normalize(q - p)))
                })
            };
            let Some((at, along)) = shared(g, f).or_else(|| shared(f, g)) else {
                continue;
            };
            if !open_beside(f, at, along) || !open_beside(g, at, along) {
                continue;
            }
            let (a, b) = (
                &brushes[f.brush].faces()[f.index],
                &brushes[g.brush].faces()[g.index],
            );
            if a.material_name != b.material_name || a.uv_attributes.scale != b.uv_attributes.scale
            {
                seams.kept += 1;
                continue;
            }
            links[i].push((j, at, along));
            links[j].push((i, at, along));
        }
    }
    // Out from the first face of each run, each face once.
    let mut updated: Vec<Vec<map_model::brush_face::BrushFace>> =
        brushes.iter().map(|b| b.faces().to_vec()).collect();
    let mut seen = vec![false; faces.len()];
    // Faces with fixed (paraxial) axes lead: they can only follow a neighbour by their
    // offset, a face with free axes can follow anything.
    let mut roots: Vec<usize> = (0..faces.len()).collect();
    roots.sort_by_key(|f| {
        matches!(
            brushes[faces[*f].brush].faces()[faces[*f].index].uv_coord_system,
            UvCoordSystem::Parallel { .. }
        )
    });
    for root in roots {
        if seen[root] || links[root].is_empty() {
            continue;
        }
        seen[root] = true;
        let mut queue = std::collections::VecDeque::from([root]);
        while let Some(current) = queue.pop_front() {
            for &(next, at, along) in &links[current] {
                if seen[next] {
                    continue;
                }
                let source = updated[faces[current].brush][faces[current].index].clone();
                let target = &mut updated[faces[next].brush][faces[next].index];
                let before = target.clone();
                if matches!(target.uv_coord_system, UvCoordSystem::Parallel { .. }) {
                    // From a paraxial neighbour too: its axes as they fall on its face.
                    target.wrap_uv_from(&source, at);
                } else {
                    // Paraxial axes follow the face's own world plane and cannot turn:
                    // the neighbour's offset and rotation, moved so the two agree on
                    // the edge, which holds along the edge only when both faces step
                    // the same way along it, as on a wall's upright corner.
                    target.uv_attributes = source.uv_attributes;
                    target.uv_attributes.offset += source.uv_coords(at) - target.uv_coords(at);
                    let further = at + along * 16.0;
                    let gap = source.uv_coords(further) - target.uv_coords(further);
                    if gap.x().abs() > 1e-3 || gap.y().abs() > 1e-3 {
                        *target = before;
                        seams.kept += 1;
                        continue;
                    }
                }
                seen[next] = true;
                if target.uv_coord_system != before.uv_coord_system
                    || target.uv_attributes != before.uv_attributes
                {
                    seams.aligned += 1;
                }
                queue.push_back(next);
            }
        }
    }
    for (brush, faces) in brushes.iter_mut().zip(updated) {
        *brush = brush.with_face_attributes_from(&faces);
    }
    seams
}

/// The area two convex outlines in one plane (seen along `normal` or against it) share.
pub(crate) fn overlap_area(a: &[Vec3d], b: &[Vec3d], normal: Vec3d) -> f64 {
    if a.len() < 3 || b.len() < 3 {
        return 0.0;
    }
    let centre = b.iter().fold(Vec3d::zero(), |sum, p| sum + *p) / b.len() as f64;
    let mut clipped: Vec<Vec3d> = a.to_vec();
    for i in 0..b.len() {
        let (from, to) = (b[i], b[(i + 1) % b.len()]);
        let mut inward = vec::cross(normal, to - from);
        if vec::dot(inward, centre - from) < 0.0 {
            inward = -inward;
        }
        let side = |p: Vec3d| vec::dot(inward, p - from);
        let mut next = Vec::with_capacity(clipped.len() + 1);
        for k in 0..clipped.len() {
            let (p, q) = (clipped[k], clipped[(k + 1) % clipped.len()]);
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                next.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                next.push(p + (q - p) * (sp / (sp - sq)));
            }
        }
        clipped = next;
        if clipped.len() < 3 {
            return 0.0;
        }
    }
    let mut twice = Vec3d::zero();
    for k in 0..clipped.len() {
        twice += vec::cross(clipped[k], clipped[(k + 1) % clipped.len()]);
    }
    vec::dot(twice, vec::normalize(normal)).abs() / 2.0
}

/// The material a face nobody can see gets, as the rounded corner, the sweep and the
/// exporters' default `skip_texture` name it.
const SKIP: &str = "skip";

/// Whether every corner of `inner` lies within the convex outline `outer`, both in the
/// plane with normal `normal`.
pub(crate) fn polygon_within(inner: &[Vec3d], outer: &[Vec3d], normal: Vec3d) -> bool {
    if outer.len() < 3 {
        return false;
    }
    let centre = outer.iter().fold(Vec3d::zero(), |sum, p| sum + *p) / outer.len() as f64;
    (0..outer.len()).all(|i| {
        let (from, to) = (outer[i], outer[(i + 1) % outer.len()]);
        let mut across = vec::cross(to - from, normal);
        let length = vec::length(across);
        if length < EPS {
            return true;
        }
        across /= length;
        if vec::dot(centre - from, across) > 0.0 {
            across = -across;
        }
        inner.iter().all(|p| vec::dot(*p - from, across) <= 1e-4)
    })
}

/// Settles the material of every face the join made (one in no plane of the brush it
/// came from; every face of a piece a rounding added): skip when it lies wholly on a face
/// of another piece, facing it, as the two sides of a mitre, a T's cut stem end and a
/// crossing's cut ends do; otherwise a visible material. A made face takes its material
/// from the face of the members that points most nearly its way, and that can be a skip
/// face an earlier join left, such as the far mitre of a beam joined at both ends; a
/// visible face is given the nearest face that is not skip instead. Faces the brushes had
/// before keep their material.
fn hide_covered_faces(join: &mut Join) {
    let mut pieces: Vec<(Brush, Option<usize>)> = join
        .after
        .iter()
        .enumerate()
        .map(|(i, b)| (b.clone(), Some(i)))
        .collect();
    pieces.extend(join.added.iter().map(|b| (b.clone(), join.split)));
    let plane_close = |a: &Plane3d, b: &Plane3d| {
        vec::length(a.normal - b.normal) < 1e-6 && (a.distance - b.distance).abs() < 1e-4
    };
    // The members' visible faces, for a made face that shows to take its material from.
    let visible: Vec<&map_model::brush_face::BrushFace> = join
        .before
        .iter()
        .flat_map(|b| b.faces())
        .filter(|f| f.material_name.as_str() != SKIP)
        .collect();
    let mut updated: Vec<Option<Brush>> = vec![None; pieces.len()];
    for (index, (piece, origin)) in pieces.iter().enumerate() {
        // A piece a rounding added has no faces of its own from before.
        let old_planes = if index < 2 {
            origin.map_or_else(Vec::new, |o| join.before[o].face_planes())
        } else {
            Vec::new()
        };
        let mut faces = piece.faces().to_vec();
        let mut changed = false;
        for (face_index, face) in piece.faces().iter().enumerate() {
            if old_planes.iter().any(|p| plane_close(p, &face.plane)) {
                continue;
            }
            let Some(polygon) = piece.face_polygon(face_index) else {
                continue;
            };
            let facing = Plane3d {
                normal: -face.plane.normal,
                distance: -face.plane.distance,
            };
            let covered = pieces.iter().enumerate().any(|(other, (brush, _))| {
                other != index
                    && (0..brush.faces().len()).any(|i| {
                        plane_close(&brush.faces()[i].plane, &facing)
                            && brush.face_polygon(i).is_some_and(|outer| {
                                polygon_within(&polygon, &outer, facing.normal)
                            })
                    })
            });
            let is_skip = face.material_name.as_str() == SKIP;
            if covered && !is_skip {
                faces[face_index].material_name = SKIP.into();
                changed = true;
            } else if !covered && is_skip {
                if let Some(source) = visible.iter().max_by(|a, b| {
                    vec::dot(a.plane.normal, face.plane.normal)
                        .total_cmp(&vec::dot(b.plane.normal, face.plane.normal))
                }) {
                    faces[face_index].material_name = source.material_name.clone();
                    changed = true;
                }
            }
        }
        if changed {
            updated[index] = piece.with_exact_face_attributes(&faces);
        }
    }
    for (index, brush) in updated.into_iter().enumerate() {
        let Some(brush) = brush else { continue };
        if index < 2 {
            join.after[index] = brush;
        } else {
            join.added[index - 2] = brush;
        }
    }
}

/// Checks the skip rule on a set of finished brushes by looking: a face is hidden when
/// points just outside it, at its centre and near each corner, all lie inside other
/// brushes. A face any of those points of which is in the open must not be skip, and a
/// hidden face the join made must be. `kept` are planes of faces from before the join,
/// which keep whatever material they had.
#[cfg(test)]
pub(crate) fn assert_skip_only_where_hidden(brushes: &[Brush], kept: &[Plane3d]) {
    for (index, brush) in brushes.iter().enumerate() {
        for (face_index, face) in brush.faces().iter().enumerate() {
            let Some(polygon) = brush.face_polygon(face_index) else {
                continue;
            };
            let centre =
                polygon.iter().fold(Vec3d::zero(), |sum, p| sum + *p) / polygon.len() as f64;
            let outside = face.plane.normal * 0.01;
            let mut samples = std::iter::once(centre)
                .chain(polygon.iter().map(|p| *p + (centre - *p) * 0.1))
                .map(|p| p + outside);
            let hidden = samples.all(|p| {
                brushes
                    .iter()
                    .enumerate()
                    .any(|(other, b)| other != index && b.contains_point(p))
            });
            let skip = face.material_name.as_str() == SKIP;
            let old = kept.iter().any(|p| {
                vec::length(p.normal - face.plane.normal) < 1e-6
                    && (p.distance - face.plane.distance).abs() < 1e-4
            });
            assert!(
                !skip || hidden,
                "a visible face is skip: brush {index}, normal {:?}",
                face.plane.normal
            );
            assert!(
                skip || !hidden || old,
                "a hidden face is not skip: brush {index}, normal {:?}",
                face.plane.normal
            );
        }
    }
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
                        textures: false,
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
                                    textures: false,
                                },
                                Rounding {
                                    outer: Some((1.0, 1)),
                                    inner: Some((48.0, 8)),
                                    concentric: true,
                                    wedges: false,
                                    preserve_thickness: false,
                                    snap: None,
                                    textures: false,
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
                textures: false,
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
                    textures: false,
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

    /// The faces of a brush with the skip material, as their planes' normals.
    fn skipped(brush: &Brush) -> Vec<Vec3d> {
        brush
            .faces()
            .iter()
            .filter(|f| f.material_name.as_str() == SKIP)
            .map(|f| f.plane.normal)
            .collect()
    }

    #[test]
    fn the_faces_a_join_hides_get_the_skip_material_and_no_others() {
        let plain = Rounding::default();
        // An L: each member's mitre face, and nothing else.
        let a = cube(Vec3d::zero(), Vec3d::new(128.0, 16.0, 8.0), "a");
        let b = cube(Vec3d::zero(), Vec3d::new(16.0, 96.0, 8.0), "b");
        let join = preview_with_rounding(&a, &b, plain).unwrap();
        for piece in &join.after {
            let hidden = skipped(piece);
            assert_eq!(hidden.len(), 1, "one mitre face");
            assert!(hidden[0].x().abs() > 0.1 && hidden[0].y().abs() > 0.1);
        }
        // A T: the stem's cut end only; the bar is untouched.
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
        let join = preview_with_rounding(&bar, &stem, plain).unwrap();
        assert!(skipped(&join.after[0]).is_empty());
        assert_eq!(skipped(&join.after[1]), vec![Vec3d::new(0.0, -1.0, 0.0)]);
        // A crossing: both cut ends.
        let long = cube(
            Vec3d::new(-128.0, -8.0, 0.0),
            Vec3d::new(128.0, 8.0, 16.0),
            "long",
        );
        let short = cube(
            Vec3d::new(-8.0, -64.0, 0.0),
            Vec3d::new(8.0, 64.0, 16.0),
            "short",
        );
        let join = preview_with_rounding(&long, &short, plain).unwrap();
        assert!(skipped(&join.after[0]).is_empty());
        assert_eq!(skipped(&join.after[1]).len(), 1);
        assert_eq!(skipped(&join.added[0]).len(), 1);
        // A rounded outside corner: the arc faces show and keep their material.
        let rounded = preview_with_rounding(
            &a,
            &b,
            Rounding {
                outer: Some((8.0, 4)),
                ..Default::default()
            },
        )
        .unwrap();
        for piece in &rounded.after {
            assert_eq!(skipped(piece).len(), 1, "only the mitre is hidden");
        }
        // A face that was skip before stays skip; the others keep their material.
        for piece in join.after.iter().chain(&join.added) {
            assert!(piece
                .faces()
                .iter()
                .all(|f| f.material_name.as_str() == SKIP
                    || f.material_name.as_str() == "long"
                    || f.material_name.as_str() == "short"));
        }
    }

    #[test]
    fn the_owners_gate_gets_skip_on_exactly_its_hidden_faces() {
        let brushes = gate();
        let join = preview_with_rounding(&brushes[1], &brushes[2], Rounding::default()).unwrap();
        // The beam's right mitre and the post's top mitre; the beam's left mitre from
        // the earlier join kept its material, as it was there before.
        assert_eq!(skipped(&join.after[0]).len(), 1);
        assert_eq!(skipped(&join.after[1]).len(), 1);
        assert!(skipped(&join.after[0])[0].x() > 0.0);
    }

    #[test]
    fn skip_lands_only_on_hidden_faces_in_every_kind_of_join() {
        let a = cube(Vec3d::zero(), Vec3d::new(128.0, 16.0, 16.0), "a");
        let b = cube(Vec3d::zero(), Vec3d::new(16.0, 96.0, 16.0), "b");
        let wide = cube(Vec3d::zero(), Vec3d::new(24.0, 96.0, 16.0), "b");
        let rounding = |outer, inner, concentric, wedges| Rounding {
            outer,
            inner,
            concentric,
            wedges,
            ..Default::default()
        };
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
        let long = cube(
            Vec3d::new(-128.0, -8.0, 0.0),
            Vec3d::new(128.0, 8.0, 16.0),
            "long",
        );
        let short = cube(
            Vec3d::new(-8.0, -64.0, 0.0),
            Vec3d::new(8.0, 64.0, 16.0),
            "short",
        );
        let gate = gate();
        let cases: Vec<(&Brush, &Brush, Rounding)> = vec![
            (&a, &b, Rounding::default()),
            (&a, &wide, Rounding::default()),
            (&a, &b, rounding(Some((8.0, 4)), None, false, false)),
            (&a, &b, rounding(None, Some((8.0, 8)), false, false)),
            (
                &a,
                &b,
                rounding(Some((8.0, 4)), Some((8.0, 8)), false, false),
            ),
            (
                &a,
                &b,
                rounding(Some((8.0, 8)), Some((8.0, 8)), true, false),
            ),
            (
                &a,
                &b,
                rounding(Some((8.0, 8)), Some((8.0, 8)), false, true),
            ),
            (&bar, &stem, Rounding::default()),
            (&long, &short, Rounding::default()),
            (&gate[1], &gate[2], Rounding::default()),
            (
                &gate[1],
                &gate[2],
                rounding(None, Some((8.0, 8)), false, false),
            ),
        ];
        let mut checked = 0;
        for (first, second, rounding) in cases {
            let Ok(join) = preview_with_rounding(first, second, rounding) else {
                continue;
            };
            let kept: Vec<Plane3d> = join.before.iter().flat_map(Brush::face_planes).collect();
            assert_skip_only_where_hidden(&pieces(&join), &kept);
            checked += 1;
        }
        assert!(checked >= 9, "{checked}");
    }
}
