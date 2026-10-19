//! A line segment between two points.
//!
//! A segment stores its endpoints in sorted order, so that two segments describing the
//! same pair of points compare equal whichever way round they were built. That is why the
//! endpoints are private and reached through [`Segment::start`] and [`Segment::end`].

use std::cmp::Ordering;

use crate::abstract_line::AbstractLine;
use crate::mat::Matrix;
use crate::scalar::{Float, Num};
use crate::vec::{self, Vector};

/// A line segment between two points.
#[derive(Debug, Clone, Copy)]
pub struct Segment<T, const S: usize> {
    start: Vector<T, S>,
    end: Vector<T, S>,
}

impl<T: Num, const S: usize> Default for Segment<T, S> {
    fn default() -> Self {
        Self {
            start: Vector::zero(),
            end: Vector::zero(),
        }
    }
}

impl<T: Num, const S: usize> Segment<T, S> {
    /// Creates a segment between the given points.
    ///
    /// The endpoints are stored in sorted order, so `Segment::new(a, b)` and
    /// `Segment::new(b, a)` are the same segment.
    #[inline]
    pub fn new(p1: Vector<T, S>, p2: Vector<T, S>) -> Self {
        if p1 < p2 {
            Self { start: p1, end: p2 }
        } else {
            Self { start: p2, end: p1 }
        }
    }

    /// Returns the lesser of the two endpoints.
    #[inline]
    pub fn start(&self) -> Vector<T, S> {
        self.start
    }

    /// Returns the greater of the two endpoints.
    #[inline]
    pub fn end(&self) -> Vector<T, S> {
        self.end
    }

    /// Returns the midpoint of this segment.
    #[inline]
    pub fn center(&self) -> Vector<T, S> {
        (self.start + self.end) / T::TWO
    }

    /// Returns the squared length of this segment.
    #[inline]
    pub fn squared_length(&self) -> T {
        vec::squared_length(self.end - self.start)
    }

    /// Returns this segment translated by the given delta.
    #[inline]
    pub fn translate(&self, delta: Vector<T, S>) -> Self {
        Self::new(self.start + delta, self.end + delta)
    }
}

impl<T: Float, const S: usize> Segment<T, S> {
    /// Returns the length of this segment.
    #[inline]
    pub fn length(&self) -> T {
        vec::length(self.end - self.start)
    }

    /// Returns the normalized direction from [`Segment::start`] to [`Segment::end`].
    #[inline]
    pub fn direction_normalized(&self) -> Vector<T, S> {
        vec::normalize(self.end - self.start)
    }

    /// Returns whether the given point lies within `max_distance` of this segment.
    pub fn contains(&self, point: Vector<T, S>, max_distance: T) -> bool {
        let f = self.distance_to_projected_point(point);
        // Reject points whose projection falls off either end before doing the more
        // expensive perpendicular distance check.
        if f < -max_distance || f * f > self.squared_length() + max_distance * max_distance
        {
            return false;
        }
        let projected = self.point_at_distance(f);
        vec::squared_distance(projected, point) <= max_distance * max_distance
    }
}

impl<T: Float> Segment<T, 3> {
    /// Returns this segment transformed by the given 4x4 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 4, 4>) -> Self {
        Self::new(*transform * self.start, *transform * self.end)
    }
}

impl<T: Float> Segment<T, 2> {
    /// Returns this segment transformed by the given 3x3 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 3, 3>) -> Self {
        Self::new(*transform * self.start, *transform * self.end)
    }
}

impl<T: Float, const S: usize> AbstractLine<T, S> for Segment<T, S> {
    #[inline]
    fn origin(&self) -> Vector<T, S> {
        self.start
    }

    #[inline]
    fn direction(&self) -> Vector<T, S> {
        self.direction_normalized()
    }
}

/// Compares the given segments by their endpoints, treating components that differ by at
/// most `epsilon` as equal.
pub fn compare<T: Num, const S: usize>(
    lhs: &Segment<T, S>,
    rhs: &Segment<T, S>,
    epsilon: T,
) -> Ordering {
    match vec::compare(&lhs.start, &rhs.start, epsilon) {
        Ordering::Equal => vec::compare(&lhs.end, &rhs.end, epsilon),
        other => other,
    }
}

/// Returns whether the given segments have the same endpoints up to `epsilon`.
#[inline]
pub fn is_equal<T: Num, const S: usize>(
    lhs: &Segment<T, S>,
    rhs: &Segment<T, S>,
    epsilon: T,
) -> bool {
    compare(lhs, rhs, epsilon) == Ordering::Equal
}

impl<T: Num, const S: usize> PartialEq for Segment<T, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        compare(self, other, T::ZERO) == Ordering::Equal
    }
}

impl<T: Num, const S: usize> Eq for Segment<T, S> {}

impl<T: Num, const S: usize> PartialOrd for Segment<T, S> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Num, const S: usize> Ord for Segment<T, S> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        compare(self, other, T::ZERO)
    }
}

/// Returns the vertices of the given segments, two per segment.
pub fn vertices<'a, T: Num + 'a, const S: usize, I>(segments: I) -> Vec<Vector<T, S>>
where
    I: IntoIterator<Item = &'a Segment<T, S>>,
{
    segments
        .into_iter()
        .flat_map(|s| [s.start(), s.end()])
        .collect()
}

/// A 2D single precision segment.
pub type Segment2f = Segment<f32, 2>;
/// A 2D double precision segment.
pub type Segment2d = Segment<f64, 2>;
/// A 3D single precision segment.
pub type Segment3f = Segment<f32, 3>;
/// A 3D double precision segment.
pub type Segment3d = Segment<f64, 3>;
