//! An infinite line through a point in a given direction.
//!

use crate::abstract_line::AbstractLine;
use crate::mat::Matrix;
use crate::mat_ext;
use crate::scalar::{Float, Num};
use crate::vec::{self, Vector};

/// An infinite line, described by a point on it and a direction.
///
/// The direction is expected to be normalized.
#[derive(Debug, Clone, Copy)]
pub struct Line<T, const S: usize> {
    /// A point on the line.
    pub point: Vector<T, S>,
    /// The line's direction.
    pub direction: Vector<T, S>,
}

impl<T: Num, const S: usize> Default for Line<T, S> {
    fn default() -> Self {
        Self {
            point: Vector::zero(),
            direction: Vector::zero(),
        }
    }
}

impl<T: Num, const S: usize> Line<T, S> {
    /// Creates a line through the given point in the given direction.
    #[inline]
    pub const fn new(point: Vector<T, S>, direction: Vector<T, S>) -> Self {
        Self { point, direction }
    }

    /// Returns an equivalent line in canonical form.
    ///
    /// Two lines are the same set of points exactly when their canonical forms are
    /// identical, which is what makes `==` meaningful. The canonical form anchors the
    /// line at the point closest to the origin and orients the direction so its first
    /// non-zero component is positive.
    pub fn make_canonical(&self) -> Self {
        let distance = vec::dot(self.point, self.direction);
        let new_point = self.point - self.direction * distance;

        let mut new_direction = self.direction;
        for i in 0..S {
            if self.direction[i] != T::ZERO {
                if self.direction[i] < T::ZERO {
                    new_direction = -new_direction;
                }
                break;
            }
        }

        Self::new(new_point, new_direction)
    }
}

impl<T: Float> Line<T, 3> {
    /// Returns this line transformed by the given 4x4 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 4, 4>) -> Self {
        Self::new(
            *transform * self.point,
            vec::normalize(mat_ext::strip_translation(transform) * self.direction),
        )
    }
}

impl<T: Float> Line<T, 2> {
    /// Returns this line transformed by the given 3x3 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 3, 3>) -> Self {
        Self::new(
            *transform * self.point,
            vec::normalize(mat_ext::strip_translation(transform) * self.direction),
        )
    }
}

impl<T: Num, const S: usize> AbstractLine<T, S> for Line<T, S> {
    #[inline]
    fn origin(&self) -> Vector<T, S> {
        self.point
    }

    #[inline]
    fn direction(&self) -> Vector<T, S> {
        self.direction
    }
}

/// Returns whether the given lines have the same point and direction up to `epsilon`.
///
/// This compares the representations, not the sets of points: two lines describing the
/// same infinite line but anchored at different points are not equal here. Use
/// [`Line::make_canonical`] on both first to compare the lines themselves.
#[inline]
pub fn is_equal<T: Num, const S: usize>(lhs: &Line<T, S>, rhs: &Line<T, S>, epsilon: T) -> bool {
    vec::is_equal(&lhs.point, &rhs.point, epsilon)
        && vec::is_equal(&lhs.direction, &rhs.direction, epsilon)
}

/// Two lines are equal when they describe the same set of points.
impl<T: Num, const S: usize> PartialEq for Line<T, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let lhs = self.make_canonical();
        let rhs = other.make_canonical();
        lhs.point == rhs.point && lhs.direction == rhs.direction
    }
}

/// A 2D single precision line.
pub type Line2f = Line<f32, 2>;
/// A 2D double precision line.
pub type Line2d = Line<f64, 2>;
/// A 3D single precision line.
pub type Line3f = Line<f32, 3>;
/// A 3D double precision line.
pub type Line3d = Line<f64, 3>;
