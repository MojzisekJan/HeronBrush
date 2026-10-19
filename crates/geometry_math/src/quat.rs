//! A quaternion, used to represent rotations.
//!
//! A quaternion is a real part `r` and a vector part `v`. For a rotation of `angle`
//! radians about a unit `axis`, `r` is `cos(angle / 2)` and `v` is
//! `axis * sin(angle / 2)`.

use std::ops::{Mul, Neg};

use crate::scalar::{self, Float};
use crate::vec::{self, Vector};

/// A quaternion.
#[derive(Debug, Clone, Copy)]
pub struct Quat<T> {
    /// The real part.
    pub r: T,
    /// The vector part.
    pub v: Vector<T, 3>,
}

impl<T: Float> Default for Quat<T> {
    /// Returns the zero quaternion.
    ///
    /// Note that this is not the identity rotation, which is `Quat::new(1, zero)`.
    fn default() -> Self {
        Self {
            r: T::ZERO,
            v: Vector::zero(),
        }
    }
}

impl<T: Float> Quat<T> {
    /// Creates a quaternion from its real and vector parts.
    #[inline]
    pub const fn new(r: T, v: Vector<T, 3>) -> Self {
        Self { r, v }
    }

    /// Creates a quaternion representing a rotation of `angle` radians about `axis`.
    ///
    /// The axis is expected to be normalized.
    pub fn from_axis_angle(axis: Vector<T, 3>, angle: T) -> Self {
        debug_assert!(
            vec::is_unit(axis, T::ALMOST_ZERO),
            "the axis must be a unit vector"
        );
        let half = angle / T::TWO;
        Self {
            r: half.cos(),
            v: axis * half.sin(),
        }
    }

    /// Creates a quaternion representing the shortest rotation taking `from` to `to`.
    ///
    /// Both vectors are expected to be normalized.
    pub fn from_to(from: Vector<T, 3>, to: Vector<T, 3>) -> Self {
        debug_assert!(
            vec::is_unit(from, T::ALMOST_ZERO),
            "`from` must be a unit vector"
        );
        debug_assert!(
            vec::is_unit(to, T::ALMOST_ZERO),
            "`to` must be a unit vector"
        );

        let cos = vec::dot(from, to);
        let z_axis = Vector::<T, 3>::new(T::ZERO, T::ZERO, T::ONE);

        if scalar::is_equal(cos, T::ONE, T::ALMOST_ZERO) {
            // The vectors are equal, so there is nothing to rotate.
            Self::from_axis_angle(z_axis, T::ZERO)
        } else if scalar::is_equal(-cos, T::ONE, T::ALMOST_ZERO) {
            // The vectors are opposite, so any perpendicular axis will do. Crossing with
            // Z finds one unless `from` is itself parallel to Z, in which case X works.
            let mut axis = vec::cross(from, z_axis);
            if scalar::is_zero(vec::squared_length(axis), T::ALMOST_ZERO) {
                axis = vec::cross(from, Vector::<T, 3>::new(T::ONE, T::ZERO, T::ZERO));
            }
            Self::from_axis_angle(vec::normalize(axis), scalar::to_radians(T::from_i32(180)))
        } else {
            Self::from_axis_angle(vec::normalize(vec::cross(from, to)), cos.acos())
        }
    }

    /// Returns the rotation angle in radians.
    #[inline]
    pub fn angle(&self) -> T {
        T::TWO * self.r.acos()
    }

    /// Returns the rotation axis.
    ///
    /// Returns the vector part unchanged when it is zero, i.e. when there is no rotation
    /// to take an axis from.
    pub fn axis(&self) -> Vector<T, 3> {
        if vec::is_zero(self.v, T::ALMOST_ZERO) {
            self.v
        } else {
            self.v / self.r.acos().sin()
        }
    }

    /// Returns the conjugate, i.e. this rotation reversed.
    #[inline]
    pub fn conjugate(&self) -> Self {
        Self {
            r: self.r,
            v: -self.v,
        }
    }
}

/// Returns whether the given quaternions are equal up to `epsilon`.
///
/// Compares corresponding components. Use [`same_rotation`] to compare orientations.
#[inline]
pub fn is_equal<T: Float>(lhs: &Quat<T>, rhs: &Quat<T>, epsilon: T) -> bool {
    scalar::is_equal(lhs.r, rhs.r, epsilon) && vec::is_equal(&lhs.v, &rhs.v, epsilon)
}

impl<T: Float> PartialEq for Quat<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.r == other.r && self.v == other.v
    }
}

/// Whether two unit quaternions represent the same rotation within `epsilon`.
/// Both `q` and `-q` encode the same orientation; a conjugate encodes its inverse.
pub fn same_rotation<T: Float>(lhs: &Quat<T>, rhs: &Quat<T>, epsilon: T) -> bool {
    is_equal(lhs, rhs, epsilon) || is_equal(lhs, &-*rhs, epsilon)
}

/// Negates every component. This preserves the orientation of a unit quaternion.
impl<T: Float> Neg for Quat<T> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self {
            r: -self.r,
            v: -self.v,
        }
    }
}

/// The Hamilton product, i.e. the composition of two rotations.
impl<T: Float> Mul for Quat<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let nr = self.r * rhs.r - vec::dot(self.v, rhs.v);
        let nx = self.r * rhs.v.x() + self.v.x() * rhs.r + self.v.y() * rhs.v.z()
            - self.v.z() * rhs.v.y();
        let ny = self.r * rhs.v.y() + self.v.y() * rhs.r + self.v.z() * rhs.v.x()
            - self.v.x() * rhs.v.z();
        let nz = self.r * rhs.v.z() + self.v.z() * rhs.r + self.v.x() * rhs.v.y()
            - self.v.y() * rhs.v.x();
        Self::new(nr, Vector::<T, 3>::new(nx, ny, nz))
    }
}

/// Rotates the given vector by this quaternion.
impl<T: Float> Mul<Vector<T, 3>> for Quat<T> {
    type Output = Vector<T, 3>;

    #[inline]
    fn mul(self, rhs: Vector<T, 3>) -> Vector<T, 3> {
        (self * Quat::new(T::ZERO, rhs) * self.conjugate()).v
    }
}

// Component-wise scalar multiplication in either operand order.
macro_rules! impl_quat_scalar {
    ($($t:ty),*) => {$(
        impl Mul<$t> for Quat<$t> {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: $t) -> Self {
                Self { r: self.r * rhs, v: self.v * rhs }
            }
        }

        impl Mul<Quat<$t>> for $t {
            type Output = Quat<$t>;

            #[inline]
            fn mul(self, rhs: Quat<$t>) -> Quat<$t> {
                Quat { r: self * rhs.r, v: rhs.v * self }
            }
        }
    )*};
}

impl_quat_scalar!(f32, f64);

/// A single precision quaternion.
pub type Quatf = Quat<f32>;
/// A double precision quaternion.
pub type Quatd = Quat<f64>;
