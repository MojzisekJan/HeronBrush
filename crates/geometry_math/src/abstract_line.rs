//! Operations shared by every line-like type.
//!
//! Implement [`AbstractLine`] to provide an origin and direction for the shared line operations.

use crate::scalar::Num;
use crate::vec::{self, Vector};

/// A type described by an origin and a direction: a [line](crate::line::Line),
/// a [ray](crate::ray::Ray) or a [segment](crate::segment::Segment).
///
/// The direction is expected to be normalized, so that distances along the line are in
/// world units.
pub trait AbstractLine<T: Num, const S: usize> {
    /// Returns the point the line is anchored at.
    fn origin(&self) -> Vector<T, S>;

    /// Returns the line's direction.
    fn direction(&self) -> Vector<T, S>;

    /// Returns the distance from the origin to the given point's projection onto this
    /// line.
    ///
    /// The result is negative when the projection falls behind the origin.
    #[inline]
    fn distance_to_projected_point(&self, point: Vector<T, S>) -> T {
        vec::dot(point - self.origin(), self.direction())
    }

    /// Returns the point at the given distance from the origin along this line.
    #[inline]
    fn point_at_distance(&self, distance: T) -> Vector<T, S> {
        self.origin() + self.direction() * distance
    }

    /// Returns the orthogonal projection of the given point onto this line.
    #[inline]
    fn project_point(&self, point: Vector<T, S>) -> Vector<T, S> {
        self.point_at_distance(self.distance_to_projected_point(point))
    }
}
