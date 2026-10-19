//! A plane in Hesse normal form.
//!
//! A plane is a unit normal and the signed distance from the origin along that normal.
//! The normal is expected to be normalized; nothing enforces it, but the distance
//! computations are only in world units if it is.

use crate::mat::Matrix;
use crate::mat_ext;
use crate::scalar::{self, Float, Num};
use crate::util::{Axis, PlaneStatus};
use crate::vec::{self, Vector};

/// A plane, described by its normal and its signed distance from the origin.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "T: serde::Serialize",
        deserialize = "T: serde::Deserialize<'de> + Copy + Default"
    ))
)]
pub struct Plane<T, const S: usize> {
    /// The signed distance from the origin along the normal.
    pub distance: T,
    /// The plane's unit normal.
    pub normal: Vector<T, S>,
}

impl<T: Num, const S: usize> Default for Plane<T, S> {
    fn default() -> Self {
        Self {
            distance: T::ZERO,
            normal: Vector::zero(),
        }
    }
}

impl<T: Num, const S: usize> Plane<T, S> {
    /// Creates a plane from its distance from the origin and its normal.
    #[inline]
    pub const fn new(distance: T, normal: Vector<T, S>) -> Self {
        Self { distance, normal }
    }

    /// Creates the plane through the given anchor point with the given normal.
    #[inline]
    pub fn from_anchor_and_normal(anchor: Vector<T, S>, normal: Vector<T, S>) -> Self {
        Self {
            distance: vec::dot(anchor, normal),
            normal,
        }
    }

    /// Returns the point on the plane closest to the origin.
    #[inline]
    pub fn anchor(&self) -> Vector<T, S> {
        self.normal * self.distance
    }

    /// Returns the signed distance from the given point to this plane.
    ///
    /// The result is positive in front of the plane and negative behind it.
    #[inline]
    pub fn point_distance(&self, point: Vector<T, S>) -> T {
        vec::dot(point, self.normal) - self.distance
    }

    /// Returns this plane facing the other way.
    ///
    /// The distance is negated along with the normal, so the plane stays where it is.
    #[inline]
    pub fn flip(&self) -> Self {
        Self::new(-self.distance, -self.normal)
    }

    /// Returns the orthogonal projection of the given point onto this plane.
    #[inline]
    pub fn project_point(&self, point: Vector<T, S>) -> Vector<T, S> {
        point - self.normal * vec::dot(point, self.normal) + self.normal * self.distance
    }

    /// Returns the given vector with its component along the normal removed.
    #[inline]
    pub fn project_vector(&self, vector: Vector<T, S>) -> Vector<T, S> {
        self.project_point(self.anchor() + vector) - self.anchor()
    }
}

impl<T: Float, const S: usize> Plane<T, S> {
    /// Returns whether the given point is in front of, behind, or on this plane.
    pub fn point_status(&self, point: Vector<T, S>, epsilon: T) -> PlaneStatus {
        let distance = self.point_distance(point);
        if distance > epsilon {
            PlaneStatus::Above
        } else if distance < -epsilon {
            PlaneStatus::Below
        } else {
            PlaneStatus::Inside
        }
    }

    /// Returns whether the given point is in front of, behind, or on this plane, using
    /// the default epsilon.
    #[inline]
    pub fn point_status_default(&self, point: Vector<T, S>) -> PlaneStatus {
        self.point_status(point, T::POINT_STATUS_EPSILON)
    }

    /// Projects the given point onto this plane along the given direction.
    ///
    /// Returns `None` when the direction is parallel to the plane, in which case there is
    /// no intersection to project onto.
    pub fn project_point_along(
        &self,
        point: Vector<T, S>,
        direction: Vector<T, S>,
    ) -> Option<Vector<T, S>> {
        let cos = vec::dot(direction, self.normal);
        if scalar::is_zero(cos, T::ALMOST_ZERO) {
            return None;
        }
        let d = vec::dot(self.normal * self.distance - point, self.normal) / cos;
        Some(point + direction * d)
    }

    /// Projects the given vector onto this plane along the given direction.
    ///
    /// Returns `None` when the direction is parallel to the plane.
    pub fn project_vector_along(
        &self,
        vector: Vector<T, S>,
        direction: Vector<T, S>,
    ) -> Option<Vector<T, S>> {
        self.project_point_along(self.anchor() + vector, direction)
            .map(|point| point - self.anchor())
    }
}

impl<T: Float> Plane<T, 3> {
    /// Returns the coordinate along `axis` of the point on this plane whose other two
    /// coordinates are the components of `point`, in order.
    ///
    /// Returns 0 when the plane is parallel to the axis, since there is then either no
    /// such point or infinitely many.
    pub fn at(&self, point: Vector<T, 2>, axis: Axis) -> T {
        let axis = axis.index();
        if self.normal[axis] == T::ZERO {
            return T::ZERO;
        }

        let mut t = T::ZERO;
        let mut index = 0;
        for i in 0..3 {
            if i != axis {
                t += self.normal[i] * point[index];
                index += 1;
            }
        }
        (self.distance - t) / self.normal[axis]
    }

    /// Returns the x coordinate of the point on this plane at the given y and z.
    #[inline]
    pub fn x_at(&self, point: Vector<T, 2>) -> T {
        self.at(point, Axis::X)
    }

    /// Returns the y coordinate of the point on this plane at the given x and z.
    #[inline]
    pub fn y_at(&self, point: Vector<T, 2>) -> T {
        self.at(point, Axis::Y)
    }

    /// Returns the z coordinate of the point on this plane at the given x and y.
    #[inline]
    pub fn z_at(&self, point: Vector<T, 2>) -> T {
        self.at(point, Axis::Z)
    }

    /// Returns this plane transformed by the given 4x4 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 4, 4>) -> Self {
        Self::from_anchor_and_normal(
            *transform * self.anchor(),
            vec::normalize(mat_ext::strip_translation(transform) * self.normal),
        )
    }
}

/// Returns whether the given planes have the same normal and distance up to `epsilon`.
#[inline]
pub fn is_equal<T: Num, const S: usize>(
    lhs: &Plane<T, S>,
    rhs: &Plane<T, S>,
    epsilon: T,
) -> bool {
    scalar::is_equal(lhs.distance, rhs.distance, epsilon)
        && vec::is_equal(&lhs.normal, &rhs.normal, epsilon)
}

impl<T: Num, const S: usize> PartialEq for Plane<T, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.normal == other.normal
    }
}

/// Returns the unit normal of the plane through the three given points, or `None` if they
/// are colinear or coincident.
///
/// The `epsilon` bounds `sin(theta)` between the two edge vectors: rearranging
/// `|a x b| = |a| |b| sin(theta)` gives that ratio directly, which stays meaningful for
/// very short edges where the cross product's magnitude alone would not.
pub fn plane_normal<T: Float>(
    p1: Vector<T, 3>,
    p2: Vector<T, 3>,
    p3: Vector<T, 3>,
    epsilon: T,
) -> Option<Vector<T, 3>> {
    let v1 = p3 - p1;
    let v2 = p2 - p1;
    let normal = vec::cross(v1, v2);

    let sin_theta =
        scalar::abs(vec::length(normal) / (vec::length(v1) * vec::length(v2)));
    if sin_theta.is_nan() || sin_theta.is_infinite() || sin_theta < epsilon {
        return None;
    }
    Some(vec::normalize(normal))
}

/// Returns the plane through the three given points, or `None` if they are colinear or
/// coincident.
pub fn from_points<T: Float>(
    p1: Vector<T, 3>,
    p2: Vector<T, 3>,
    p3: Vector<T, 3>,
) -> Option<Plane<T, 3>> {
    plane_normal(p1, p2, p3, T::ANGLE_EPSILON)
        .map(|normal| Plane::from_anchor_and_normal(p1, normal))
}

/// Returns the plane through the first three of the given points, or `None` if there are
/// fewer than three or the first three do not define a plane.
pub fn from_point_sequence<T: Float, I: IntoIterator<Item = Vector<T, 3>>>(
    points: I,
) -> Option<Plane<T, 3>> {
    let mut iter = points.into_iter();
    let p1 = iter.next()?;
    let p2 = iter.next()?;
    let p3 = iter.next()?;
    from_points(p1, p2, p3)
}

/// Returns the horizontal plane through the given position, i.e. the one whose normal is
/// the positive Z axis.
#[inline]
pub fn horizontal_plane<T: Float>(position: Vector<T, 3>) -> Plane<T, 3> {
    Plane::from_anchor_and_normal(position, Vector::<T, 3>::new(T::ZERO, T::ZERO, T::ONE))
}

/// Returns the plane through the given position perpendicular to the given direction.
#[inline]
pub fn orthogonal_plane<T: Float>(
    position: Vector<T, 3>,
    direction: Vector<T, 3>,
) -> Plane<T, 3> {
    Plane::from_anchor_and_normal(position, vec::normalize(direction))
}

/// Returns the plane through the given position whose normal is the coordinate axis
/// closest to the given direction.
#[inline]
pub fn aligned_orthogonal_plane<T: Float>(
    position: Vector<T, 3>,
    direction: Vector<T, 3>,
) -> Plane<T, 3> {
    Plane::from_anchor_and_normal(position, direction.get_abs_max_component_axis(0))
}

/// A 3D single precision plane.
pub type Plane3f = Plane<f32, 3>;
/// A 3D double precision plane.
pub type Plane3d = Plane<f64, 3>;
