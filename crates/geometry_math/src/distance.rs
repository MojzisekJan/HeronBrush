//! Closest-approach distances between the line-like primitives and points.
//!
//! Every function comes in a squared and an unsquared form. Prefer the squared one when
//! only comparing distances: it avoids a square root, and the positions it reports are
//! unaffected.

use crate::abstract_line::AbstractLine;
use crate::line::Line;
use crate::ray::Ray;
use crate::scalar::{self, Float};
use crate::segment::Segment;
use crate::vec::{self, Vector};

/// The distance from a point to a line-like object, and where along it the closest
/// approach happens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointDistance<T> {
    /// The distance along the line-like object to the closest point.
    pub position: T,
    /// The distance from that point to the given point.
    pub distance: T,
}

impl<T: Float> PointDistance<T> {
    /// Creates a point distance.
    #[inline]
    pub const fn new(position: T, distance: T) -> Self {
        Self { position, distance }
    }

    /// Returns this result with a squared distance converted to an unsquared one.
    #[inline]
    fn sqrt(self) -> Self {
        Self {
            position: self.position,
            distance: self.distance.sqrt(),
        }
    }
}

/// The distance between two line-like objects, and where along each the closest approach
/// happens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineDistance<T> {
    /// Whether the two objects are parallel, in which case the positions describe an
    /// overlap rather than a unique closest pair.
    pub parallel: bool,
    /// The distance along the first object to its closest point.
    pub position1: T,
    /// The distance between the two closest points.
    pub distance: T,
    /// The distance along the second object to its closest point.
    pub position2: T,
}

impl<T: Float> LineDistance<T> {
    /// Creates a result for two parallel objects.
    #[inline]
    pub fn parallel(position1: T, distance: T, position2: T) -> Self {
        debug_assert!(!position1.is_nan() && !distance.is_nan() && !position2.is_nan());
        Self {
            parallel: true,
            position1,
            distance,
            position2,
        }
    }

    /// Creates a result for two non-parallel objects.
    #[inline]
    pub fn non_parallel(position1: T, distance: T, position2: T) -> Self {
        debug_assert!(!position1.is_nan() && !distance.is_nan() && !position2.is_nan());
        Self {
            parallel: false,
            position1,
            distance,
            position2,
        }
    }

    /// Returns whether the two objects lie on the same line, within `max_distance`.
    #[inline]
    pub fn is_colinear(&self, max_distance: T) -> bool {
        self.parallel && self.distance <= max_distance
    }

    /// Returns this result with a squared distance converted to an unsquared one.
    #[inline]
    fn sqrt(self) -> Self {
        Self {
            distance: self.distance.sqrt(),
            ..self
        }
    }
}

/* ========== point to ray and segment ========== */

/// Returns the squared distance from the given ray to the given point.
pub fn squared_distance_ray_point<T: Float, const S: usize>(
    r: &Ray<T, S>,
    p: Vector<T, S>,
) -> PointDistance<T> {
    let origin_to_point = p - r.origin;
    // Clamping at 0 keeps the closest point on the ray rather than on its backward
    // extension.
    let position = scalar::max(vec::dot(origin_to_point, r.direction), T::ZERO);
    if position == T::ZERO {
        PointDistance::new(position, vec::squared_length(origin_to_point))
    } else {
        PointDistance::new(
            position,
            vec::squared_length(r.point_at_distance(position) - p),
        )
    }
}

/// Returns the distance from the given ray to the given point.
#[inline]
pub fn distance_ray_point<T: Float, const S: usize>(
    r: &Ray<T, S>,
    p: Vector<T, S>,
) -> PointDistance<T> {
    squared_distance_ray_point(r, p).sqrt()
}

/// Returns the squared distance from the given segment to the given point.
pub fn squared_distance_segment_point<T: Float, const S: usize>(
    s: &Segment<T, S>,
    p: Vector<T, S>,
) -> PointDistance<T> {
    let vector = s.end() - s.start();
    let len = vec::length(vector);
    let dir = vector / len;

    let scale = vec::dot(p - s.start(), dir);
    // The closest point is clamped to the segment's own extent.
    let position = scalar::min(scalar::max(T::ZERO, scale), len);
    let distance = vec::squared_length(p - s.point_at_distance(position));
    PointDistance::new(position, distance)
}

/// Returns the distance from the given segment to the given point.
#[inline]
pub fn distance_segment_point<T: Float, const S: usize>(
    s: &Segment<T, S>,
    p: Vector<T, S>,
) -> PointDistance<T> {
    squared_distance_segment_point(s, p).sqrt()
}

/* ========== ray to segment, ray and line ========== */

/// Returns the squared distance between the given ray and segment.
pub fn squared_distance_ray_segment<T: Float, const S: usize>(
    r: &Ray<T, S>,
    s: &Segment<T, S>,
) -> LineDistance<T> {
    let p1 = s.start();
    let p2 = s.end();
    let mut u = p2 - p1;
    let mut v = r.direction;
    let mut w = p1 - r.origin;

    let a = vec::dot(u, u);
    let b = vec::dot(u, v);
    let c = vec::dot(v, v);
    let d = vec::dot(u, w);
    let e = vec::dot(v, w);
    let denominator = a * c - b * b;

    if scalar::is_zero(denominator, T::ALMOST_ZERO) {
        let p1_on_r = r.distance_to_projected_point(p1);
        let p2_on_r = r.distance_to_projected_point(p2);

        if p1_on_r < T::ZERO && p2_on_r < T::ZERO {
            // The segment lies entirely behind the ray's origin.
            return if p1_on_r > p2_on_r {
                LineDistance::parallel(T::ZERO, vec::squared_distance(r.origin, p1), T::ZERO)
            } else {
                LineDistance::parallel(
                    T::ZERO,
                    vec::squared_distance(r.origin, p2),
                    p2_on_r - p1_on_r,
                )
            };
        } else if p1_on_r > T::ZERO && p2_on_r > T::ZERO {
            // The segment lies entirely ahead of the ray's origin.
            let perpendicular = vec::squared_distance(r.point_at_distance(p1_on_r), p1);
            return if p1_on_r > p2_on_r {
                LineDistance::parallel(p2_on_r, perpendicular, p1_on_r - p2_on_r)
            } else {
                LineDistance::parallel(p1_on_r, perpendicular, T::ZERO)
            };
        }

        // The segment straddles the ray's origin.
        let perpendicular = vec::squared_distance(r.point_at_distance(p1_on_r), p1);
        let r_origin_on_s = s.distance_to_projected_point(r.origin);
        return LineDistance::parallel(T::ZERO, perpendicular, r_origin_on_s);
    }

    let s_denominator = denominator;
    let mut t_denominator = denominator;
    let mut s_numerator = b * e - c * d;
    let mut t_numerator = a * e - b * d;

    // Clamp the closest point to the segment's ends.
    if s_numerator < T::ZERO {
        s_numerator = T::ZERO;
        t_numerator = e;
        t_denominator = c;
    } else if s_numerator > s_denominator {
        s_numerator = s_denominator;
        t_numerator = e + b;
        t_denominator = c;
    }

    let sc = if scalar::is_zero(s_numerator, T::ALMOST_ZERO) {
        T::ZERO
    } else {
        s_numerator / s_denominator
    };
    let tc = scalar::max(
        if scalar::is_zero(t_numerator, T::ALMOST_ZERO) {
            T::ZERO
        } else {
            t_numerator / t_denominator
        },
        T::ZERO,
    );

    u *= sc;
    v *= tc;
    w += u;
    let delta = w - v;

    LineDistance::non_parallel(tc, vec::squared_length(delta), sc * a.sqrt())
}

/// Returns the distance between the given ray and segment.
#[inline]
pub fn distance_ray_segment<T: Float, const S: usize>(
    r: &Ray<T, S>,
    s: &Segment<T, S>,
) -> LineDistance<T> {
    squared_distance_ray_segment(r, s).sqrt()
}

/// Returns the squared distance between the two given rays.
pub fn squared_distance_ray_ray<T: Float, const S: usize>(
    lhs: &Ray<T, S>,
    rhs: &Ray<T, S>,
) -> LineDistance<T> {
    let mut u = rhs.direction;
    let mut v = lhs.direction;
    let mut w = rhs.origin - lhs.origin;

    let a = vec::dot(u, u);
    let b = vec::dot(u, v);
    let c = vec::dot(v, v);
    let d = vec::dot(u, w);
    let e = vec::dot(v, w);
    let denominator = a * c - b * b;

    if scalar::is_zero(denominator, T::ALMOST_ZERO) {
        let rhs_origin_on_lhs = lhs.distance_to_projected_point(rhs.origin);
        let lhs_origin_on_rhs = rhs.distance_to_projected_point(lhs.origin);
        let perpendicular = vec::squared_distance(lhs.project_point(rhs.origin), rhs.origin);
        return LineDistance::parallel(
            scalar::max(T::ZERO, rhs_origin_on_lhs),
            perpendicular,
            scalar::max(T::ZERO, lhs_origin_on_rhs),
        );
    }

    let s_denominator = denominator;
    let mut t_denominator = denominator;
    let mut s_numerator = b * e - c * d;
    let mut t_numerator = a * e - b * d;

    // Clamp to the second ray's origin: a ray does not extend backwards.
    if s_numerator < T::ZERO {
        s_numerator = T::ZERO;
        t_numerator = e;
        t_denominator = c;
    }

    let sc = if scalar::is_zero(s_numerator, T::ALMOST_ZERO) {
        T::ZERO
    } else {
        s_numerator / s_denominator
    };
    let tc = scalar::max(
        if scalar::is_zero(t_numerator, T::ALMOST_ZERO) {
            T::ZERO
        } else {
            t_numerator / t_denominator
        },
        T::ZERO,
    );

    u *= sc;
    v *= tc;
    w += u;
    let delta = w - v;

    LineDistance::non_parallel(tc, vec::squared_length(delta), sc)
}

/// Returns the distance between the two given rays.
#[inline]
pub fn distance_ray_ray<T: Float, const S: usize>(
    lhs: &Ray<T, S>,
    rhs: &Ray<T, S>,
) -> LineDistance<T> {
    squared_distance_ray_ray(lhs, rhs).sqrt()
}

/// Returns the squared distance between the given ray and line.
pub fn squared_distance_ray_line<T: Float, const S: usize>(
    r: &Ray<T, S>,
    l: &Line<T, S>,
) -> LineDistance<T> {
    let w0 = r.origin - l.point;
    let a = vec::dot(r.direction, r.direction);
    let b = vec::dot(r.direction, l.direction);
    let c = vec::dot(l.direction, l.direction);
    let d = vec::dot(r.direction, w0);
    let e = vec::dot(l.direction, w0);
    let denominator = a * c - b * b;

    if scalar::is_zero(denominator, T::ALMOST_ZERO) {
        let perpendicular = vec::squared_distance(r.project_point(l.point), l.point);
        // A line extends both ways, so this position may be negative.
        let r_origin_on_l = l.distance_to_projected_point(r.origin);
        return LineDistance::parallel(T::ZERO, perpendicular, r_origin_on_l);
    }

    // The ray is clamped at its origin; the line is not.
    let sc = scalar::max((b * e - c * d) / denominator, T::ZERO);
    let tc = (a * e - b * d) / denominator;

    let on_ray = r.origin + r.direction * sc;
    let on_line = l.point + l.direction * tc;
    LineDistance::non_parallel(sc, vec::squared_length(on_ray - on_line), tc)
}

/// Returns the distance between the given ray and line.
#[inline]
pub fn distance_ray_line<T: Float, const S: usize>(
    r: &Ray<T, S>,
    l: &Line<T, S>,
) -> LineDistance<T> {
    squared_distance_ray_line(r, l).sqrt()
}
