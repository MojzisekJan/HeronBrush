//! Intersection tests between the geometric primitives.
//!
//! Every `intersect_ray_*` function returns the distance along the ray to the nearest
//! hit, so the caller recovers the point with
//! [`AbstractLine::point_at_distance`].
//! `None` means no hit.
//!
//! Accepts point slices; callers can collect projected coordinates when needed.

use crate::abstract_line::AbstractLine;
use crate::bbox::Bbox;
use crate::line::Line;
use crate::mat::{self, Matrix};
use crate::plane::{self, Plane};
use crate::ray::Ray;
use crate::scalar::{self, Float};
use crate::segment::Segment;
use crate::util::Axis;
use crate::vec::{self, Vector};

/// What one polygon edge contributes to the crossing count in
/// [`polygon_contains_point_on_axis`].
enum EdgeCrossing {
    /// The point lies exactly on this edge or vertex; the answer is immediately "yes".
    OnEdge,
    /// The edge crosses the positive X axis.
    Crosses,
    /// The edge does not cross the positive X axis.
    DoesNotCross,
}

/// Classifies one polygon edge against the positive X axis, for the ray casting test in
/// [`polygon_contains_point_on_axis`].
///
/// Both vertices are given relative to the point being tested, so the test point is at
/// the origin and the ray is the positive X axis.
fn handle_polygon_edge_intersection<T: Float>(
    v0: Vector<T, 3>,
    v1: Vector<T, 3>,
) -> EdgeCrossing {
    if vec::is_zero(v0, T::ALMOST_ZERO) {
        // The point is a polygon vertex.
        return EdgeCrossing::OnEdge;
    }

    // Does the Y range this edge covers reach the X axis at all?
    if (scalar::is_zero(v0.y(), T::ALMOST_ZERO) && scalar::is_zero(v1.y(), T::ALMOST_ZERO))
        || (v0.y() > T::ZERO && v1.y() > T::ZERO)
        || (v0.y() < T::ZERO && v1.y() < T::ZERO)
    {
        return EdgeCrossing::DoesNotCross;
    }

    // Entirely on one side of the origin?
    if v0.x() > T::ZERO && v1.x() > T::ZERO {
        return EdgeCrossing::Crosses;
    }
    if v0.x() < T::ZERO && v1.x() < T::ZERO {
        return EdgeCrossing::DoesNotCross;
    }

    // Where does the edge cross the X axis?
    let x = -v0.y() * (v1.x() - v0.x()) / (v1.y() - v0.y()) + v0.x();
    if scalar::is_zero(x, T::ALMOST_ZERO) {
        return EdgeCrossing::OnEdge;
    }
    if x > T::ZERO {
        EdgeCrossing::Crosses
    } else {
        EdgeCrossing::DoesNotCross
    }
}

/// Returns whether the given polygon contains the given point, projecting both down the
/// given axis.
///
/// This is a ray casting test: it counts how often the positive X axis of the projected
/// frame crosses the polygon boundary, and the point is inside when that count is odd.
/// The axis should be the one the polygon's normal is most aligned with, so that the
/// projection does not collapse the polygon; [`polygon_contains_point`] picks it.
pub fn polygon_contains_point_on_axis<T: Float>(
    point: Vector<T, 3>,
    axis: Axis,
    vertices: &[Vector<T, 3>],
) -> bool {
    if vertices.is_empty() {
        return false;
    }

    let axis = axis.index();
    let origin = vec::swizzle(point, axis);
    let project = |v: Vector<T, 3>| vec::swizzle(v, axis) - origin;

    let first = project(vertices[0]);
    let mut previous = first;
    let mut crossings = 0usize;

    for vertex in &vertices[1..] {
        let current = project(*vertex);
        match handle_polygon_edge_intersection(previous, current) {
            EdgeCrossing::OnEdge => return true,
            EdgeCrossing::Crosses => crossings += 1,
            EdgeCrossing::DoesNotCross => {}
        }
        previous = current;
    }

    // Close the loop with the edge from the last vertex back to the first.
    match handle_polygon_edge_intersection(previous, first) {
        EdgeCrossing::OnEdge => return true,
        EdgeCrossing::Crosses => crossings += 1,
        EdgeCrossing::DoesNotCross => {}
    }

    !crossings.is_multiple_of(2)
}

/// Returns whether the polygon with the given normal contains the given point.
#[inline]
pub fn polygon_contains_point_with_normal<T: Float>(
    point: Vector<T, 3>,
    normal: Vector<T, 3>,
    vertices: &[Vector<T, 3>],
) -> bool {
    let axis = Axis::from_index(normal.find_abs_max_component(0))
        .expect("a 3D vector's largest component is one of the three axes");
    polygon_contains_point_on_axis(point, axis, vertices)
}

/// Returns whether the given polygon contains the given point.
///
/// The polygon's plane is derived from its first three vertices; returns false if they do
/// not define one, or if there are fewer than three.
pub fn polygon_contains_point<T: Float>(
    point: Vector<T, 3>,
    vertices: &[Vector<T, 3>],
) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    match plane::plane_normal(vertices[0], vertices[1], vertices[2], T::ANGLE_EPSILON) {
        Some(normal) => polygon_contains_point_with_normal(point, normal, vertices),
        None => false,
    }
}

/* ========== ray intersections ========== */

/// Returns the distance along the ray to its intersection with the given plane, or `None`
/// if the ray misses it or points away from it.
pub fn intersect_ray_plane<T: Float, const S: usize>(
    r: &Ray<T, S>,
    p: &Plane<T, S>,
) -> Option<T> {
    let d = vec::dot(r.direction, p.normal);
    if scalar::is_zero(d, T::ALMOST_ZERO) {
        return None;
    }
    let s = vec::dot(p.anchor() - r.origin, p.normal) / d;
    (s >= -T::ALMOST_ZERO).then_some(s)
}

/// Returns the distance along the ray to its intersection with the given triangle, or
/// `None` if it misses.
///
/// Uses the Möller-Trumbore algorithm.
pub fn intersect_ray_triangle<T: Float>(
    r: &Ray<T, 3>,
    p1: Vector<T, 3>,
    p2: Vector<T, 3>,
    p3: Vector<T, 3>,
) -> Option<T> {
    let e1 = p2 - p1;
    let e2 = p3 - p1;
    let p = vec::cross(r.direction, e2);
    let a = vec::dot(p, e1);
    if scalar::is_zero(a, T::ALMOST_ZERO) {
        // The ray is parallel to the triangle's plane.
        return None;
    }

    let t = r.origin - p1;
    let q = vec::cross(t, e1);

    let u = vec::dot(q, e2) / a;
    if u < -T::ALMOST_ZERO {
        return None;
    }
    let v = vec::dot(p, t) / a;
    if v < -T::ALMOST_ZERO {
        return None;
    }
    let w = vec::dot(q, r.direction) / a;
    if w < -T::ALMOST_ZERO {
        return None;
    }
    // The barycentric coordinates must sum to at most 1 to be inside the triangle.
    if v + w - T::ONE > T::ALMOST_ZERO {
        return None;
    }
    Some(u)
}

/// Returns the distance along the ray to its intersection with the polygon lying in the
/// given plane, or `None` if it misses.
pub fn intersect_ray_polygon_in_plane<T: Float>(
    r: &Ray<T, 3>,
    p: &Plane<T, 3>,
    vertices: &[Vector<T, 3>],
) -> Option<T> {
    let distance = intersect_ray_plane(r, p)?;
    let point = r.point_at_distance(distance);
    polygon_contains_point_with_normal(point, p.normal, vertices).then_some(distance)
}

/// Returns the distance along the ray to its intersection with the given polygon, or
/// `None` if it misses or the vertices do not define a plane.
pub fn intersect_ray_polygon<T: Float>(
    r: &Ray<T, 3>,
    vertices: &[Vector<T, 3>],
) -> Option<T> {
    let p = plane::from_point_sequence(vertices.iter().copied())?;
    intersect_ray_polygon_in_plane(r, &p, vertices)
}

/// Returns the distance along the ray to its entry into the given box, or `None` if it
/// misses.
pub fn intersect_ray_bbox<T: Float, const S: usize>(
    r: &Ray<T, S>,
    b: &Bbox<T, S>,
) -> Option<T> {
    // Pick the candidate plane on each axis: the near face if the origin is outside the
    // box on that axis, otherwise the face the ray is heading towards.
    let mut origins = [T::ZERO; S];
    let mut inside = [true; S];
    let mut all_inside = true;

    for i in 0..S {
        if r.origin[i] < b.min[i] {
            origins[i] = b.min[i];
            inside[i] = false;
            all_inside = false;
        } else if r.origin[i] > b.max[i] {
            origins[i] = b.max[i];
            inside[i] = false;
            all_inside = false;
        } else {
            origins[i] = if r.direction[i] < T::ZERO {
                b.min[i]
            } else {
                b.max[i]
            };
        }
    }

    let mut distances = [T::ZERO; S];
    for i in 0..S {
        distances[i] = if r.direction[i] != T::ZERO {
            (origins[i] - r.origin[i]) / r.direction[i]
        } else {
            -T::ONE
        };
    }

    let best = if all_inside {
        // Leaving the box: the first face reached wins.
        let mut best = 0;
        for i in 1..S {
            if distances[i] > T::ZERO && distances[i] < distances[best] {
                best = i;
            }
        }
        best
    } else {
        // Entering the box: the last face reached wins.
        let mut best = (0..S).find(|&i| !inside[i]).unwrap_or(0);
        for i in best + 1..S {
            if !inside[i] && distances[i] > distances[best] {
                best = i;
            }
        }
        best
    };

    if distances[best] < T::ZERO {
        return None;
    }

    // The hit must fall within the box on every other axis.
    for i in 0..S {
        if i != best {
            let coord = r.origin[i] + distances[best] * r.direction[i];
            if coord < b.min[i] || coord > b.max[i] {
                return None;
            }
        }
    }

    Some(distances[best])
}

/// Returns the distance along the ray to its nearest intersection with the given sphere,
/// or `None` if it misses.
pub fn intersect_ray_sphere<T: Float, const S: usize>(
    r: &Ray<T, S>,
    position: Vector<T, S>,
    radius: T,
) -> Option<T> {
    let diff = r.origin - position;
    let p = T::TWO * vec::dot(diff, r.direction);
    let q = vec::squared_length(diff) - radius * radius;
    let d = p * p - T::from_i32(4) * q;
    if d < T::ZERO {
        return None;
    }

    let s = d.sqrt();
    let t0 = (-p + s) / T::TWO;
    let t1 = (-p - s) / T::TWO;

    if t0 < T::ZERO && t1 < T::ZERO {
        // The sphere is entirely behind the ray.
        None
    } else if t0 > T::ZERO && t1 > T::ZERO {
        // Both hits are ahead; take the nearer one.
        Some(scalar::min(t0, t1))
    } else {
        // The origin is inside the sphere; take the one ahead.
        Some(scalar::max(t0, t1))
    }
}

/// Returns the distance along the ray to its nearest intersection with the given torus,
/// or `None` if it misses.
///
/// The torus lies in the XY plane around `position`. See
/// <https://marcin-chwedczuk.github.io/ray-tracing-torus>.
pub fn intersect_ray_torus<T: Float>(
    r: &Ray<T, 3>,
    position: Vector<T, 3>,
    major_radius: T,
    minor_radius: T,
) -> Option<T> {
    // Translating the ray rather than the torus leaves the hit distance unchanged.
    let origin = r.origin - position;

    let dd = vec::dot(r.direction, r.direction);
    let od = vec::dot(origin, r.direction);
    let oo = vec::dot(origin, origin);
    let mm_major = major_radius * major_radius;
    let mm_minor = minor_radius * minor_radius;
    let dz = r.direction.z();
    let oz = origin.z();
    let om = oo - mm_minor - mm_major;

    let (two, four, eight) = (T::TWO, T::from_i32(4), T::from_i32(8));
    let a = dd * dd;
    let b = four * dd * od;
    let c = two * dd * om + four * (od * od + mm_major * dz * dz);
    let d = four * od * om + eight * mm_major * oz * dz;
    let e = om * om - four * mm_major * (mm_minor - oz * oz);

    let roots = scalar::solve_quartic(a, b, c, d, e, T::ALMOST_ZERO);
    scalar::safe_min(
        roots
            .as_slice()
            .iter()
            .map(|&t| (t > T::ZERO).then_some(t)),
    )
}

/* ========== line and plane intersections ========== */

/// Returns the distance along `l1` to its intersection with `l2`, or `None` if the lines
/// are parallel.
pub fn intersect_line_line<T: Float>(l1: &Line<T, 2>, l2: &Line<T, 2>) -> Option<T> {
    let m = Matrix::<T, 2, 2>::from_rows([
        [l1.direction.x(), -l2.direction.x()],
        [l1.direction.y(), -l2.direction.y()],
    ]);
    let inverse = mat::invert(m)?;
    Some((inverse * (l2.point - l1.point)).x())
}

/// Returns the distance along the line to its intersection with the given plane, or
/// `None` if the line is parallel to the plane.
pub fn intersect_line_plane<T: Float>(l: &Line<T, 3>, p: &Plane<T, 3>) -> Option<T> {
    let f = vec::dot(l.direction, p.normal);
    if scalar::is_zero(f, T::ALMOST_ZERO) {
        return None;
    }
    Some(vec::dot(p.normal * p.distance - l.point, p.normal) / f)
}

/// Returns the line where the two given planes meet, or `None` if they are parallel.
pub fn intersect_plane_plane<T: Float>(
    p1: &Plane<T, 3>,
    p2: &Plane<T, 3>,
) -> Option<Line<T, 3>> {
    let line_direction = vec::normalize(vec::cross(p1.normal, p2.normal));
    if vec::is_nan(line_direction) {
        // The normals are parallel, so the planes are too.
        return None;
    }

    // Find a point on both planes. From <http://geomalgorithms.com/a05-_intersect-1.html>:
    // projecting the other plane's normal into this plane gives a direction from this
    // plane's anchor that is guaranteed to reach the other plane.
    let to_p2_direction = p1.project_vector(p2.normal);
    if to_p2_direction == Vector::zero() {
        return None;
    }

    let to_p2 = Line::new(p1.anchor(), vec::normalize(to_p2_direction));
    let distance = intersect_line_plane(&to_p2, p2)?;
    Some(Line::new(to_p2.point_at_distance(distance), line_direction))
}

/// Returns whether the two given segments overlap along a shared line.
///
/// Segments that merely cross do not overlap; they must be colinear and share more than
/// `epsilon` of length.
pub fn segments_overlap<T: Float, const S: usize>(
    lhs: &Segment<T, S>,
    rhs: &Segment<T, S>,
    epsilon: T,
) -> bool {
    let axis = lhs.end() - lhs.start();
    let length = vec::length(axis);
    if length < epsilon {
        return false;
    }
    let direction = axis / length;

    let project = |p: Vector<T, S>| vec::dot(p - lhs.start(), direction);
    let on_line = |p: Vector<T, S>, t: T| {
        vec::is_equal(&p, &(lhs.start() + direction * t), epsilon)
    };

    let t0 = project(rhs.start());
    let t1 = project(rhs.end());
    if !on_line(rhs.start(), t0) || !on_line(rhs.end(), t1) {
        return false;
    }

    let overlap = scalar::min(length, scalar::max(t0, t1))
        - scalar::max(T::ZERO, scalar::min(t0, t1));
    overlap > epsilon
}

/* ========== polygon clipping ========== */

/// The epsilon used by [`polygon_clip_by_plane`] to classify a vertex relative to a plane.
const CLIP_EPSILON: f64 = 0.0001;

/// Clips the given convex polygon against the given plane, keeping the part behind it.
///
/// Returns an empty vector when no vertex is behind the plane, i.e. when the plane does
/// not clip the polygon at all.
pub fn polygon_clip_by_plane<T: Float>(
    p: &Plane<T, 3>,
    vertices: &[Vector<T, 3>],
) -> Vec<Vector<T, 3>> {
    let mut result = Vec::new();
    polygon_clip_by_plane_into(p, vertices, &mut result);
    result
}

/// [`polygon_clip_by_plane`] into a buffer the caller keeps (cleared first), so the clip
/// loops that cut every face of every brush do not allocate a polygon per pass.
pub fn polygon_clip_by_plane_into<T: Float>(
    p: &Plane<T, 3>,
    vertices: &[Vector<T, 3>],
    result: &mut Vec<Vector<T, 3>>,
) {
    let epsilon = T::from_f64(CLIP_EPSILON);
    result.clear();

    let behind = vertices
        .iter()
        .filter(|v| p.point_distance(**v) < -epsilon)
        .count();
    if behind == 0 {
        return;
    }

    // Snapping near-zero distances to exactly zero keeps a vertex that sits on the plane
    // from being emitted twice, once as itself and once as a crossing point.
    let corrected_distance = |point: Vector<T, 3>| {
        let d = p.point_distance(point);
        if scalar::abs(d) > epsilon {
            d
        } else {
            T::ZERO
        }
    };

    result.reserve(behind);
    for (i, start) in vertices.iter().enumerate() {
        let end = vertices[scalar::succ(i, vertices.len(), 1)];
        let sd = corrected_distance(*start);
        let ed = corrected_distance(end);

        if sd <= T::ZERO {
            result.push(*start);
        }
        if (sd < T::ZERO && ed > T::ZERO) || (sd > T::ZERO && ed < T::ZERO) {
            let t = sd / (sd - ed);
            result.push(*start * (T::ONE - t) + end * t);
        }
    }
}

/// Returns whether the two given convex polygons, both lying in a plane with the given
/// normal, overlap.
///
/// Clips `rhs` against each of `lhs`'s edge planes in turn; anything left over is the
/// shared area.
pub fn polygons_overlap<T: Float>(
    lhs: &[Vector<T, 3>],
    rhs: &[Vector<T, 3>],
    normal: Vector<T, 3>,
) -> bool {
    if lhs.len() < 3 || rhs.len() < 3 {
        return false;
    }

    let center = match vec::average(lhs.iter().copied()) {
        Some(center) => center,
        None => return false,
    };

    let mut clipped = rhs.to_vec();
    for i in 0..lhs.len() {
        let v0 = lhs[i];
        let v1 = lhs[scalar::succ(i, lhs.len(), 1)];

        // Orient each edge plane so that the polygon's interior is behind it.
        let mut edge_normal = vec::cross(normal, v1 - v0);
        if vec::dot(edge_normal, center - v0) > T::ZERO {
            edge_normal = -edge_normal;
        }
        let edge_plane = Plane::from_anchor_and_normal(v0, vec::normalize(edge_normal));

        clipped = polygon_clip_by_plane(&edge_plane, &clipped);
        // A convex region can only shrink as further half-planes are intersected, so once
        // fewer than three vertices remain the shared area is provably zero.
        if clipped.len() < 3 {
            return false;
        }
    }
    true
}

/// Returns whether the given box and convex polygon intersect.
///
/// Polygon edges already lie in their polygon plane, so intersecting them with that plane cannot
/// add crossing points.
pub fn intersect_bbox_polygon<T: Float>(b: &Bbox<T, 3>, vertices: &[Vector<T, 3>]) -> bool {
    if vertices.len() < 3 {
        return false;
    }

    // Any polygon vertex inside the box settles it.
    if vertices.iter().any(|v| b.contains_point(*v)) {
        return true;
    }

    let Some(polygon_plane) = plane::from_points(vertices[0], vertices[1], vertices[2])
    else {
        return false;
    };

    // Otherwise the polygon must cut through the box, which means one of the box's edges
    // crosses the polygon.
    b.edges().iter().any(|(start, end)| {
        let l = Line::new(*start, *end - *start);
        match intersect_line_plane(&l, &polygon_plane) {
            // The direction here is the full edge, so the hit is on the edge exactly when
            // the distance is in [0, 1].
            Some(d) if d >= T::ZERO && d <= T::ONE => {
                let point = polygon_plane.project_point(l.point_at_distance(d));
                polygon_contains_point(point, vertices)
            }
            _ => false,
        }
    })
}
