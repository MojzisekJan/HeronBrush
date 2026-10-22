//! An axis aligned bounding box.
//!
//! A box is a min and a max point subject to the invariant that `min[i] <= max[i]` for
//! every component. [`Bbox::new`] panics if that does not hold; [`Bbox::new_repaired`]
//! swaps the offending components instead.
//!
//! Face, edge and vertex accessors return fixed-size arrays that can be iterated, indexed or
//! destructured.

use crate::mat::Matrix;
use crate::scalar::{Float, Num};
use crate::vec::{self, Vector};

/// Which end of an axis a corner sits at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Corner {
    /// The low end of the axis.
    Min,
    /// The high end of the axis.
    Max,
}

/// Where a coordinate falls relative to a box along one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Range {
    /// Below the box's minimum.
    Less,
    /// Inside the box.
    Within,
    /// Above the box's maximum.
    Greater,
}

/// An axis aligned bounding box.
#[derive(Debug, Clone, Copy)]
pub struct Bbox<T, const S: usize> {
    /// The corner with the smallest coordinate on every axis.
    pub min: Vector<T, S>,
    /// The corner with the largest coordinate on every axis.
    pub max: Vector<T, S>,
}

impl<T: Num, const S: usize> PartialEq for Bbox<T, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

impl<T: Num, const S: usize> Eq for Bbox<T, S> {}

impl<T: Num, const S: usize> Default for Bbox<T, S> {
    /// Returns the degenerate box at the origin.
    fn default() -> Self {
        Self {
            min: Vector::zero(),
            max: Vector::zero(),
        }
    }
}

impl<T: Num, const S: usize> Bbox<T, S> {
    /// Returns whether the given points satisfy the box invariant.
    pub fn is_valid_pair(min: Vector<T, S>, max: Vector<T, S>) -> bool {
        (0..S).all(|i| min[i] <= max[i])
    }

    /// Creates a box spanning the given corners.
    ///
    /// # Panics
    ///
    /// Panics unless `min[i] <= max[i]` for every component. Use
    /// [`Bbox::new_repaired`] when the ordering is not known in advance.
    #[inline]
    pub fn new(min: Vector<T, S>, max: Vector<T, S>) -> Self {
        assert!(
            Self::is_valid_pair(min, max),
            "a bounding box needs min <= max on every axis"
        );
        Self { min, max }
    }

    /// Creates a box spanning the given corners, swapping any components that are the
    /// wrong way round.
    #[inline]
    pub fn new_repaired(a: Vector<T, S>, b: Vector<T, S>) -> Self {
        Self {
            min: vec::min(a, b),
            max: vec::max(a, b),
        }
    }

    /// Creates a box with every min component set to `min` and every max component to
    /// `max`.
    ///
    /// # Panics
    ///
    /// Panics if `min > max`.
    #[inline]
    pub fn from_scalars(min: T, max: T) -> Self {
        Self::new(Vector::fill(min), Vector::fill(max))
    }

    /// Creates the degenerate box containing exactly the given point.
    #[inline]
    pub fn from_point(point: Vector<T, S>) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    /// Returns the smallest box containing all the given points, or `None` if there are
    /// none.
    pub fn merge_all<I: IntoIterator<Item = Vector<T, S>>>(points: I) -> Option<Self> {
        let mut iter = points.into_iter();
        let mut result = Self::from_point(iter.next()?);
        for point in iter {
            result = result.merged_with_point(point);
        }
        Some(result)
    }

    /// Returns whether this box satisfies its invariant.
    ///
    /// This can only be false for a box built by mutating [`Bbox::min`] or
    /// [`Bbox::max`] directly.
    #[inline]
    pub fn is_valid(&self) -> bool {
        Self::is_valid_pair(self.min, self.max)
    }

    /// Returns whether this box has zero extent along any axis.
    #[inline]
    pub fn is_empty(&self) -> bool {
        debug_assert!(self.is_valid());
        (0..S).any(|i| self.min[i] >= self.max[i])
    }

    /// Returns the box's centre.
    #[inline]
    pub fn center(&self) -> Vector<T, S> {
        debug_assert!(self.is_valid());
        (self.min + self.max) / T::TWO
    }

    /// Returns the box's extent along each axis.
    #[inline]
    pub fn size(&self) -> Vector<T, S> {
        debug_assert!(self.is_valid());
        self.max - self.min
    }

    /// Returns the product of the box's extents.
    pub fn volume(&self) -> T {
        debug_assert!(self.is_valid());
        let size = self.size();
        let mut result = size[0];
        for i in 1..S {
            result *= size[i];
        }
        result
    }

    /// Returns whether the given point lies inside this box, boundary included.
    pub fn contains_point(&self, point: Vector<T, S>) -> bool {
        debug_assert!(self.is_valid());
        (0..S).all(|i| point[i] >= self.min[i] && point[i] <= self.max[i])
    }

    /// Returns whether the given box lies inside this one, boundary included.
    pub fn contains(&self, other: &Self) -> bool {
        debug_assert!(self.is_valid());
        (0..S).all(|i| other.min[i] >= self.min[i] && other.max[i] <= self.max[i])
    }

    /// Returns whether the given box lies strictly inside this one, touching no boundary.
    pub fn encloses(&self, other: &Self) -> bool {
        debug_assert!(self.is_valid());
        (0..S).all(|i| other.min[i] > self.min[i] && other.max[i] < self.max[i])
    }

    /// Returns whether this box and the given one overlap, touching included.
    pub fn intersects(&self, other: &Self) -> bool {
        (0..S).all(|i| other.max[i] >= self.min[i] && other.min[i] <= self.max[i])
    }

    /// Returns the point in this box closest to the given point.
    #[inline]
    pub fn constrain(&self, point: Vector<T, S>) -> Vector<T, S> {
        debug_assert!(self.is_valid());
        vec::max(self.min, vec::min(self.max, point))
    }

    /// Returns the corner at the given end of each axis.
    pub fn corner_position(&self, corners: [Corner; S]) -> Vector<T, S> {
        debug_assert!(self.is_valid());
        let mut result = Vector::zero();
        for i in 0..S {
            result[i] = match corners[i] {
                Corner::Min => self.min[i],
                Corner::Max => self.max[i],
            };
        }
        result
    }

    /// Returns where the given point falls relative to this box on each axis.
    pub fn relative_position(&self, point: Vector<T, S>) -> [Range; S] {
        debug_assert!(self.is_valid());
        let mut result = [Range::Within; S];
        for i in 0..S {
            result[i] = if point[i] < self.min[i] {
                Range::Less
            } else if point[i] > self.max[i] {
                Range::Greater
            } else {
                Range::Within
            };
        }
        result
    }

    /// Returns this box grown by `f` on every side.
    ///
    /// A negative `f` shrinks the box and can invalidate it when the shrink exceeds half an extent.
    #[inline]
    pub fn expand(&self, f: T) -> Self {
        debug_assert!(self.is_valid());
        Self {
            min: self.min - Vector::fill(f),
            max: self.max + Vector::fill(f),
        }
    }

    /// Returns this box moved by the given delta.
    #[inline]
    pub fn translate(&self, delta: Vector<T, S>) -> Self {
        debug_assert!(self.is_valid());
        Self {
            min: self.min + delta,
            max: self.max + delta,
        }
    }

    /// Returns the smallest box containing this box and the given one.
    #[inline]
    pub fn merged(&self, other: &Self) -> Self {
        Self {
            min: vec::min(self.min, other.min),
            max: vec::max(self.max, other.max),
        }
    }

    /// Returns the smallest box containing this box and the given point.
    #[inline]
    pub fn merged_with_point(&self, point: Vector<T, S>) -> Self {
        Self {
            min: vec::min(self.min, point),
            max: vec::max(self.max, point),
        }
    }

    /// Returns the overlap of this box and the given one, or `None` if they do not
    /// overlap.
    ///
    /// Returns `None` when the boxes do not intersect.
    pub fn intersected(&self, other: &Self) -> Option<Self> {
        let min = vec::max(self.min, other.min);
        let max = vec::min(self.max, other.max);
        Self::is_valid_pair(min, max).then_some(Self { min, max })
    }
}

impl<T: Num> Bbox<T, 3> {
    /// Returns the box's eight vertices.
    ///
    /// Vertices are ordered as the top four clockwise from above, then the bottom four clockwise
    /// from below.
    pub fn vertices(&self) -> [Vector<T, 3>; 8] {
        let size = self.size();
        let x = Vector::<T, 3>::new(size.x(), T::ZERO, T::ZERO);
        let y = Vector::<T, 3>::new(T::ZERO, size.y(), T::ZERO);
        let z = Vector::<T, 3>::new(T::ZERO, T::ZERO, size.z());
        let (min, max) = (self.min, self.max);

        [
            max,
            max - y,
            min + z,
            max - x,
            min,
            min + x,
            max - z,
            min + y,
        ]
    }

    /// Returns the box's twelve edges as pairs of endpoints.
    ///
    /// Edges are ordered as the four top edges, four bottom edges, then four side edges.
    pub fn edges(&self) -> [(Vector<T, 3>, Vector<T, 3>); 12] {
        let size = self.size();
        let x = Vector::<T, 3>::new(size.x(), T::ZERO, T::ZERO);
        let y = Vector::<T, 3>::new(T::ZERO, size.y(), T::ZERO);
        let z = Vector::<T, 3>::new(T::ZERO, T::ZERO, size.z());
        let (min, max) = (self.min, self.max);

        [
            (max, max - y),
            (max - y, max - y - x),
            (max - y - x, max - x),
            (max - x, max),
            (min, min + x),
            (min + x, min + x + y),
            (min + x + y, min + y),
            (min + y, min),
            (min, min + z),
            (min + y, min + y + z),
            (min + x + y, min + x + y + z),
            (min + x, min + x + z),
        ]
    }

    /// Returns the box's six faces, each as four vertices and an outward normal.
    ///
    /// Faces are ordered top, bottom, front, back, left and right.
    pub fn faces(&self) -> [Face<T>; 6] {
        let size = self.size();
        let x = Vector::<T, 3>::new(size.x(), T::ZERO, T::ZERO);
        let y = Vector::<T, 3>::new(T::ZERO, size.y(), T::ZERO);
        let z = Vector::<T, 3>::new(T::ZERO, T::ZERO, size.z());
        let (min, max) = (self.min, self.max);
        let (o, i) = (T::ZERO, T::ONE);

        [
            Face::new(
                [max, max - y, max - y - x, max - x],
                Vector::<T, 3>::new(o, o, i),
            ),
            Face::new(
                [min, min + x, min + x + y, min + y],
                Vector::<T, 3>::new(o, o, -i),
            ),
            Face::new(
                [min, min + z, min + z + x, min + x],
                Vector::<T, 3>::new(o, -i, o),
            ),
            Face::new(
                [max, max - x, max - x - z, max - z],
                Vector::<T, 3>::new(o, i, o),
            ),
            Face::new(
                [min, min + y, min + y + z, min + z],
                Vector::<T, 3>::new(-i, o, o),
            ),
            Face::new(
                [max, max - z, max - z - y, max - y],
                Vector::<T, 3>::new(i, o, o),
            ),
        ]
    }

    /// Returns the corner at the given end of each of the three axes.
    #[inline]
    pub fn corner(&self, x: Corner, y: Corner, z: Corner) -> Vector<T, 3> {
        self.corner_position([x, y, z])
    }
}

/// One face of a 3D bounding box.
#[derive(Debug, Clone, Copy)]
pub struct Face<T> {
    /// The face's four vertices, wound consistently.
    pub vertices: [Vector<T, 3>; 4],
    /// The face's outward normal.
    pub normal: Vector<T, 3>,
}

impl<T: Num> PartialEq for Face<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices && self.normal == other.normal
    }
}

impl<T: Num> Eq for Face<T> {}

impl<T: Num> Face<T> {
    /// Creates a face from its vertices and normal.
    #[inline]
    pub const fn new(vertices: [Vector<T, 3>; 4], normal: Vector<T, 3>) -> Self {
        Self { vertices, normal }
    }
}

impl<T: Float> Bbox<T, 3> {
    /// Returns the smallest axis aligned box containing this box transformed by the given
    /// transform.
    ///
    /// The result is generally larger than the transformed box, since a rotated box is no
    /// longer axis aligned.
    pub fn transform(&self, transform: &Matrix<T, 4, 4>) -> Self {
        let mut vertices = self.vertices().into_iter().map(|v| *transform * v);
        let first = vertices.next().expect("a box always has eight vertices");
        vertices.fold(Self::from_point(first), |acc, v| acc.merged_with_point(v))
    }
}

/// Returns whether the given boxes have the same corners up to `epsilon`.
#[inline]
pub fn is_equal<T: Num, const S: usize>(lhs: &Bbox<T, S>, rhs: &Bbox<T, S>, epsilon: T) -> bool {
    vec::is_equal(&lhs.min, &rhs.min, epsilon) && vec::is_equal(&lhs.max, &rhs.max, epsilon)
}

/// Incrementally builds the smallest box containing everything added to it.
///
/// The builder starts empty, so its first point defines the initial box without implicitly
/// including the origin.
#[derive(Debug, Clone, Copy)]
pub struct Builder<T, const S: usize> {
    bounds: Option<Bbox<T, S>>,
}

impl<T, const S: usize> Default for Builder<T, S> {
    fn default() -> Self {
        Self { bounds: None }
    }
}

impl<T: Num, const S: usize> Builder<T, S> {
    /// Creates an empty builder.
    #[inline]
    pub fn new() -> Self {
        Self { bounds: None }
    }

    /// Returns whether anything has been added yet.
    #[inline]
    pub fn initialized(&self) -> bool {
        self.bounds.is_some()
    }

    /// Returns the box built so far, or `None` if nothing has been added.
    #[inline]
    pub fn bounds(&self) -> Option<Bbox<T, S>> {
        self.bounds
    }

    /// Grows the box to contain the given point.
    pub fn add_point(&mut self, point: Vector<T, S>) {
        self.bounds = Some(match self.bounds {
            Some(bounds) => bounds.merged_with_point(point),
            None => Bbox::from_point(point),
        });
    }

    /// Grows the box to contain the given box.
    pub fn add_bbox(&mut self, other: &Bbox<T, S>) {
        self.bounds = Some(match self.bounds {
            Some(bounds) => bounds.merged(other),
            None => *other,
        });
    }

    /// Grows the box to contain all the given points.
    pub fn add_points<I: IntoIterator<Item = Vector<T, S>>>(&mut self, points: I) {
        for point in points {
            self.add_point(point);
        }
    }
}

/// A 1D single precision bounding box.
pub type Bbox1f = Bbox<f32, 1>;
/// A 1D double precision bounding box.
pub type Bbox1d = Bbox<f64, 1>;
/// A 2D single precision bounding box.
pub type Bbox2f = Bbox<f32, 2>;
/// A 2D double precision bounding box.
pub type Bbox2d = Bbox<f64, 2>;
/// A 3D single precision bounding box.
pub type Bbox3f = Bbox<f32, 3>;
/// A 3D double precision bounding box.
pub type Bbox3d = Bbox<f64, 3>;
