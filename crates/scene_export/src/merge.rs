//! Joins neighbouring brushes into single collision shapes.
//!
//! Only where the result is still convex, which is decided by volume: the solid
//! made from both plane sets is bigger than the two brushes together exactly
//! when their union has a dent. The mesh is left alone.

use rayon::prelude::*;

use crate::math::{Plane, Vec3};
use crate::overlap::{corners_of, corners_within};

/// How far apart two planes may be and still count as the same one, in map
/// units and in the cosine of the angle between them.
const SAME_PLANE_DISTANCE: f64 = 0.05;
const SAME_PLANE_ANGLE: f64 = 0.9995;
/// How much bigger the joined solid may be before the join is refused, as a
/// fraction of the two brushes together.
const ROOM_TO_SPARE: f64 = 0.001;

/// Joins what can be joined, and hands back the plane sets to build shapes from.
///
/// Greedy and in passes, so a row of blocks collapses one block at a time.
pub fn convex_groups(brushes: Vec<Vec<Plane>>, most_joins: usize) -> Vec<Vec<Plane>> {
    let mut planes = brushes;
    // One brush's corners have nothing to do with another's, so spread them over the
    // cores: a few hundred small clips each.
    let mut corners: Vec<Vec<Vec3>> = planes.par_iter().map(|p| corners_of(p)).collect();
    let mut volumes: Vec<f64> = planes
        .iter()
        .zip(&corners)
        .map(|(p, c)| volume_of(p, c))
        .collect();
    // Kept rather than recomputed: only the brush that just swallowed another
    // changes.
    let mut boxes: Vec<(Vec3, Vec3)> = corners.iter().map(|c| bounds(c)).collect();
    let mut alive = vec![true; planes.len()];

    // Sorted by where each brush starts, so the sweep can stop early.
    let mut order: Vec<usize> = (0..planes.len()).collect();
    order.sort_by(|a, b| {
        boxes[*a]
            .0
            .x
            .partial_cmp(&boxes[*b].0.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut joins = 0usize;
    let mut again = true;
    while again && joins < most_joins {
        again = false;
        for oa in 0..order.len() {
            let a = order[oa];
            if !alive[a] || corners[a].len() < 4 || joins >= most_joins {
                continue;
            }
            for &b in order.iter().skip(oa + 1) {
                if joins >= most_joins {
                    break;
                }
                if !alive[b] || corners[b].len() < 4 {
                    continue;
                }
                // Past the end of this one, and so is everything after it.
                if boxes[b].0.x > boxes[a].1.x + SAME_PLANE_DISTANCE {
                    break;
                }
                if !worth_trying(boxes[a], boxes[b]) {
                    continue;
                }
                // Two convex solids have a convex union only when one holds the other or
                // they are glued along a shared facet. Anything else has a dent, so skip
                // it before paying for the hull and its volume. On a map of rotated or
                // curved brushes this rejects nearly every pair for almost nothing.
                if !could_be_convex(&planes[a], &planes[b], &corners[a], &corners[b]) {
                    continue;
                }
                let both: Vec<Vec3> = corners[a].iter().chain(&corners[b]).copied().collect();
                let joined = holding_both(&planes[a], &planes[b], &both);
                let (lo, hi) = (boxes[a].0.min(boxes[b].0), boxes[a].1.max(boxes[b].1));
                let joined_corners = corners_within(&joined, lo, hi);
                if joined_corners.len() < 4 {
                    continue;
                }
                // The joined solid always holds both brushes. When it holds
                // nothing else, their union was convex and this is exactly it.
                let together = volumes[a] + volumes[b];
                let grown = volume_of(&joined, &joined_corners);
                if grown > together * (1.0 + ROOM_TO_SPARE) {
                    continue;
                }
                volumes[a] = grown;
                boxes[a] = bounds(&joined_corners);
                corners[a] = joined_corners;
                planes[a] = joined;
                alive[b] = false;
                joins += 1;
                // No restart. This brush carries on swallowing whatever else
                // it touches, and the sweep moves on when it runs out.
                again = true;
            }
        }
    }

    planes
        .into_iter()
        .enumerate()
        .filter(|(i, _)| alive[*i])
        .map(|(_, p)| p)
        .collect()
}

/// A necessary condition for the two brushes to have a convex union: one contains the
/// other, or they meet along a facet -- a face of one that the other sits flush against,
/// backed by an opposing face on the other side.
///
/// This is cheap (plane-vs-point tests, no clipping) and only has to not rule out a real
/// join. A pair it passes still goes through the full volume check.
fn could_be_convex(a: &[Plane], b: &[Plane], ca: &[Vec3], cb: &[Vec3]) -> bool {
    let inside = |planes: &[Plane], points: &[Vec3]| {
        points
            .iter()
            .all(|p| planes.iter().all(|pl| pl.distance_to(*p) <= SAME_PLANE_DISTANCE))
    };
    if inside(a, cb) || inside(b, ca) {
        return true;
    }

    // A seam: a plane of one that the other is entirely in front of, matched by an
    // opposing plane on the other that this one is entirely in front of.
    let in_front = |plane: &Plane, points: &[Vec3]| {
        points
            .iter()
            .all(|p| plane.distance_to(*p) >= -SAME_PLANE_DISTANCE)
    };
    for pa in a {
        if !in_front(pa, cb) {
            continue;
        }
        for pb in b {
            if pa.normal.dot(pb.normal) < -SAME_PLANE_ANGLE
                && (pa.d + pb.d).abs() < SAME_PLANE_DISTANCE
                && in_front(pb, ca)
            {
                return true;
            }
        }
    }
    false
}

fn worth_trying(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> bool {
    let ((a_min, a_max), (b_min, b_max)) = (a, b);
    let gap = SAME_PLANE_DISTANCE;
    a_min.x <= b_max.x + gap
        && b_min.x <= a_max.x + gap
        && a_min.y <= b_max.y + gap
        && b_min.y <= a_max.y + gap
        && a_min.z <= b_max.z + gap
        && b_min.z <= a_max.z + gap
}

fn bounds(points: &[Vec3]) -> (Vec3, Vec3) {
    let mut low = Vec3::splat(f64::INFINITY);
    let mut high = Vec3::splat(f64::NEG_INFINITY);
    for p in points {
        low = low.min(*p);
        high = high.max(*p);
    }
    (low, high)
}

/// The planes that hold both brushes at once.
///
/// Every plane of both would give what the two have in common, which for two
/// blocks side by side is nothing. The face they are glued along drops out,
/// since each brush is in front of the other's copy of it.
fn holding_both(a: &[Plane], b: &[Plane], points: &[Vec3]) -> Vec<Plane> {
    let mut out: Vec<Plane> = Vec::with_capacity(a.len() + b.len());
    for plane in a.iter().chain(b.iter()) {
        if points
            .iter()
            .any(|p| plane.distance_to(*p) > SAME_PLANE_DISTANCE)
        {
            continue;
        }
        if !out.iter().any(|p| same_plane(p, plane)) {
            out.push(*plane);
        }
    }
    out
}

fn same_plane(a: &Plane, b: &Plane) -> bool {
    a.normal.dot(b.normal) > SAME_PLANE_ANGLE && (a.d - b.d).abs() < SAME_PLANE_DISTANCE
}

/// Volume of a convex solid, by fanning every face into triangles and taking
/// the signed volume of each against the origin.
pub fn volume_of(planes: &[Plane], corners: &[Vec3]) -> f64 {
    if corners.len() < 4 {
        return 0.0;
    }
    let inside = corners.iter().fold(Vec3::ZERO, |a, b| a + *b) / corners.len() as f64;
    let mut total = 0.0;
    let mut face: Vec<Vec3> = Vec::new();
    for plane in planes {
        face.clear();
        face.extend(
            corners
                .iter()
                .copied()
                .filter(|p| plane.distance_to(*p).abs() < SAME_PLANE_DISTANCE),
        );
        if face.len() < 3 {
            continue;
        }
        crate::math::sort_winding(&mut face, plane.normal);
        // A cone from the middle of the solid out to each triangle of the face.
        for i in 1..face.len() - 1 {
            let a = face[0] - inside;
            let b = face[i] - inside;
            let c = face[i + 1] - inside;
            total += a.dot(b.cross(c)).abs() / 6.0;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_planes(min: Vec3, max: Vec3) -> Vec<Plane> {
        vec![
            Plane {
                normal: Vec3::X,
                d: max.x,
            },
            Plane {
                normal: -Vec3::X,
                d: -min.x,
            },
            Plane {
                normal: Vec3::Y,
                d: max.y,
            },
            Plane {
                normal: -Vec3::Y,
                d: -min.y,
            },
            Plane {
                normal: Vec3::Z,
                d: max.z,
            },
            Plane {
                normal: -Vec3::Z,
                d: -min.z,
            },
        ]
    }

    fn volume(planes: &[Plane]) -> f64 {
        volume_of(planes, &corners_of(planes))
    }

    #[test]
    fn the_volume_of_a_cube_is_the_cube_of_its_side() {
        let cube = box_planes(Vec3::ZERO, Vec3::splat(16.0));
        assert!((volume(&cube) - 4096.0).abs() < 0.5, "{}", volume(&cube));
    }

    #[test]
    fn two_cubes_side_by_side_become_one_box() {
        let brushes = vec![
            box_planes(Vec3::ZERO, Vec3::splat(16.0)),
            box_planes(Vec3::new(16.0, 0.0, 0.0), Vec3::new(32.0, 16.0, 16.0)),
        ];
        let joined = convex_groups(brushes, 8);
        assert_eq!(joined.len(), 1);
        assert!((volume(&joined[0]) - 8192.0).abs() < 1.0);
    }

    #[test]
    fn a_row_of_eight_becomes_one() {
        let brushes: Vec<Vec<Plane>> = (0..8)
            .map(|i| {
                let x = i as f64 * 16.0;
                box_planes(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 16.0, 16.0, 16.0))
            })
            .collect();
        let joined = convex_groups(brushes, 32);
        assert_eq!(joined.len(), 1);
        assert!((volume(&joined[0]) - 8.0 * 4096.0).abs() < 4.0);
    }

    #[test]
    fn an_l_shape_stays_two_shapes() {
        // Joining these would swallow the corner they leave out, which is
        // exactly the kind of quiet mistake a physics shape must never make.
        let brushes = vec![
            box_planes(Vec3::ZERO, Vec3::new(64.0, 16.0, 16.0)),
            box_planes(Vec3::ZERO, Vec3::new(16.0, 64.0, 16.0)),
        ];
        assert_eq!(convex_groups(brushes, 8).len(), 2);
    }

    #[test]
    fn a_step_stays_two_shapes() {
        // Side by side but different heights: the union has a corner missing.
        let brushes = vec![
            box_planes(Vec3::ZERO, Vec3::new(16.0, 16.0, 16.0)),
            box_planes(Vec3::new(16.0, 0.0, 0.0), Vec3::new(32.0, 16.0, 32.0)),
        ];
        assert_eq!(convex_groups(brushes, 8).len(), 2);
    }

    #[test]
    fn brushes_nowhere_near_each_other_are_left_alone() {
        let brushes = vec![
            box_planes(Vec3::ZERO, Vec3::splat(16.0)),
            box_planes(Vec3::splat(500.0), Vec3::splat(516.0)),
        ];
        assert_eq!(convex_groups(brushes, 8).len(), 2);
    }

    #[test]
    fn a_gap_between_them_is_left_alone() {
        // A unit apart is a doorway, not a wall.
        let brushes = vec![
            box_planes(Vec3::ZERO, Vec3::splat(16.0)),
            box_planes(Vec3::new(17.0, 0.0, 0.0), Vec3::new(33.0, 16.0, 16.0)),
        ];
        assert_eq!(convex_groups(brushes, 8).len(), 2);
    }

    #[test]
    fn a_floor_of_four_slabs_becomes_one() {
        let mut brushes = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let x = i as f64 * 32.0;
                let y = j as f64 * 32.0;
                brushes.push(box_planes(
                    Vec3::new(x, y, 0.0),
                    Vec3::new(x + 32.0, y + 32.0, 8.0),
                ));
            }
        }
        let joined = convex_groups(brushes, 32);
        assert_eq!(joined.len(), 1);
        assert!((volume(&joined[0]) - 64.0 * 64.0 * 8.0).abs() < 8.0);
    }
}
