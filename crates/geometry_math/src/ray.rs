//! A ray: a half-line starting at an origin and extending in one direction.
//!

use crate::abstract_line::AbstractLine;
use crate::mat::Matrix;
use crate::mat_ext;
use crate::scalar::{Float, Num};
use crate::util::PlaneStatus;
use crate::vec::{self, Vector};

/// A ray, described by its origin and direction.
///
/// The direction is expected to be normalized.
#[derive(Debug, Clone, Copy)]
pub struct Ray<T, const S: usize> {
    /// The point the ray starts at.
    pub origin: Vector<T, S>,
    /// The direction the ray extends in.
    pub direction: Vector<T, S>,
}

impl<T: Num, const S: usize> Default for Ray<T, S> {
    fn default() -> Self {
        Self {
            origin: Vector::zero(),
            direction: Vector::zero(),
        }
    }
}

impl<T: Num, const S: usize> Ray<T, S> {
    /// Creates a ray from the given origin in the given direction.
    #[inline]
    pub const fn new(origin: Vector<T, S>, direction: Vector<T, S>) -> Self {
        Self { origin, direction }
    }
}

impl<T: Float, const S: usize> Ray<T, S> {
    /// Returns whether the given point lies ahead of, behind, or level with the ray's
    /// origin along its direction.
    ///
    /// This classifies against the plane through the origin perpendicular to the ray, so
    /// [`PlaneStatus::Above`] means the point is in front of the ray's start.
    pub fn point_status(&self, point: Vector<T, S>) -> PlaneStatus {
        let scale = vec::dot(self.direction, point - self.origin);
        if scale > T::POINT_STATUS_EPSILON {
            PlaneStatus::Above
        } else if scale < -T::POINT_STATUS_EPSILON {
            PlaneStatus::Below
        } else {
            PlaneStatus::Inside
        }
    }
}

impl<T: Float> Ray<T, 3> {
    /// Returns this ray transformed by the given 4x4 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 4, 4>) -> Self {
        Self::new(
            *transform * self.origin,
            vec::normalize(mat_ext::strip_translation(transform) * self.direction),
        )
    }
}

impl<T: Float> Ray<T, 2> {
    /// Returns this ray transformed by the given 3x3 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 3, 3>) -> Self {
        Self::new(
            *transform * self.origin,
            vec::normalize(mat_ext::strip_translation(transform) * self.direction),
        )
    }
}

impl<T: Num, const S: usize> AbstractLine<T, S> for Ray<T, S> {
    #[inline]
    fn origin(&self) -> Vector<T, S> {
        self.origin
    }

    #[inline]
    fn direction(&self) -> Vector<T, S> {
        self.direction
    }
}

/// Returns whether the given rays have the same origin and direction up to `epsilon`.
#[inline]
pub fn is_equal<T: Num, const S: usize>(lhs: &Ray<T, S>, rhs: &Ray<T, S>, epsilon: T) -> bool {
    vec::is_equal(&lhs.origin, &rhs.origin, epsilon)
        && vec::is_equal(&lhs.direction, &rhs.direction, epsilon)
}

impl<T: Num, const S: usize> PartialEq for Ray<T, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin && self.direction == other.direction
    }
}

/// A 3D single precision ray.
pub type Ray3f = Ray<f32, 3>;
/// A 3D double precision ray.
pub type Ray3d = Ray<f64, 3>;
/// A 2D single precision ray.
pub type Ray2f = Ray<f32, 2>;
/// A 2D double precision ray.
pub type Ray2d = Ray<f64, 2>;
