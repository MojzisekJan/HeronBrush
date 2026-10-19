//! The convex hull of a set of coplanar 3D points.
//!
//! Uses Graham's scan; see <http://geomalgorithms.com/a10-_hull-1.html>. Coplanar points are
//! projected onto their dominant coordinate plane, hulled there, then projected back.

use std::cmp::Ordering;

use crate::scalar::{self, Float};
use crate::vec::{self, Vector};

/// Which side of the line through `p1` and `p2` the point `p3` lies on, in the XY plane.
///
/// Returns 1 for left, -1 for right and 0 when the three are colinear.
fn is_left<T: Float>(p1: Vector<T, 3>, p2: Vector<T, 3>, p3: Vector<T, 3>) -> i32 {
    let result = (p2.x() - p1.x()) * (p3.y() - p1.y())
        - (p3.x() - p1.x()) * (p2.y() - p1.y());
    if result < T::ZERO {
        -1
    } else if result > T::ZERO {
        1
    } else {
        0
    }
}

/// Orders two points by the angle they subtend at the anchor, breaking ties by putting
/// the point further from the anchor first.
///
/// Keeping the furthest colinear point first is what lets the duplicate-removal pass drop
/// the ones behind it without losing a hull vertex.
fn less_than_by_angle<T: Float>(
    anchor: Vector<T, 3>,
    lhs: Vector<T, 3>,
    rhs: Vector<T, 3>,
) -> Ordering {
    match is_left(anchor, lhs, rhs) {
        s if s > 0 => Ordering::Less,
        s if s < 0 => Ordering::Greater,
        _ => {
            let dxl = scalar::abs(lhs.x() - anchor.x());
            let dxr = scalar::abs(rhs.x() - anchor.x());
            if dxl == dxr {
                let dyl = scalar::abs(lhs.y() - anchor.y());
                let dyr = scalar::abs(rhs.y() - anchor.y());
                if dyl > dyr {
                    Ordering::Less
                } else if dyl < dyr {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            } else if dxl > dxr {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
    }
}

/// Returns the convex hull of the given coplanar points, in order around the hull.
///
/// Returns an empty vector for fewer than three points, collinear points or a degenerate hull.
pub fn convex_hull<T: Float>(points: &[Vector<T, 3>]) -> Vec<Vector<T, 3>> {
    if points.len() <= 2 {
        return Vec::new();
    }

    let mut points = points.to_vec();

    // Find a point not on the line through the first two, so that the set spans a plane.
    let third = (2..points.len())
        .find(|&i| !vec::is_colinear(points[0], points[1], points[i], T::COLINEAR_EPSILON));
    let Some(third) = third else {
        return Vec::new();
    };

    // Work in the coordinate plane the points' own plane is most aligned with, so the
    // projection cannot collapse them onto a line.
    let normal = vec::cross(points[third] - points[0], points[1] - points[0]);
    let axis = normal.find_abs_max_component(0);
    for p in &mut points {
        *p = vec::swizzle(*p, axis);
    }

    move_anchor_to_front(&mut points);
    sort_and_dedup_by_angle(&mut points);

    if points.len() <= 2 {
        return Vec::new();
    }

    let mut hull = build_hull(&points);
    for p in &mut hull {
        *p = vec::unswizzle(*p, axis);
    }
    hull
}

/// Swaps the lowest point — and among equally low ones, the rightmost — to the front.
///
/// Every other point is then to the left of it, which is what makes the angular sort a
/// total order.
fn move_anchor_to_front<T: Float>(points: &mut [Vector<T, 3>]) {
    let mut anchor = 0;
    for i in 1..points.len() {
        if points[i].y() < points[anchor].y()
            || (points[i].y() == points[anchor].y() && points[i].x() > points[anchor].x())
        {
            anchor = i;
        }
    }
    points.swap(0, anchor);
}

/// Sorts the points after the anchor by angle, then drops all but the furthest of each
/// run of colinear points.
fn sort_and_dedup_by_angle<T: Float>(points: &mut Vec<Vector<T, 3>>) {
    let anchor = points[0];
    points[1..].sort_by(|a, b| less_than_by_angle(anchor, *a, *b));

    let mut kept = vec![points[0]];
    let mut i = 1;
    while i < points.len() {
        let p1 = points[i];
        kept.push(p1);
        i += 1;
        // Everything colinear with the anchor and `p1` sorts after it and is nearer, so
        // it is inside the hull.
        while i < points.len() && is_left(anchor, p1, points[i]) == 0 {
            i += 1;
        }
    }
    *points = kept;
}

/// Runs Graham's scan over the angularly sorted points.
fn build_hull<T: Float>(points: &[Vector<T, 3>]) -> Vec<Vector<T, 3>> {
    let mut stack = Vec::with_capacity(points.len());
    stack.push(points[0]);
    stack.push(points[1]);

    for &p in &points[2..] {
        // Drop points that a right turn has made interior.
        while stack.len() > 1 {
            let t1 = stack[stack.len() - 2];
            let t2 = stack[stack.len() - 1];
            if is_left(t1, t2, p) < 0 {
                stack.pop();
            } else {
                break;
            }
        }
        stack.push(p);
    }

    stack
}
