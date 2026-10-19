//! A polygon, stored as an ordered list of vertices.
//!
//! Vertices are kept rotated so that the smallest one comes first. That makes two
//! polygons with the same vertices in the same cyclic order compare equal regardless of
//! where their vertex lists happened to start, which is why the vertices are private and
//! reached through [`Polygon::vertices`].

use std::cmp::Ordering;

use crate::mat::Matrix;
use crate::scalar::{Float, Num};
use crate::vec::{self, Vector};

/// A polygon in `S` dimensions.
#[derive(Debug, Clone)]
pub struct Polygon<T, const S: usize> {
    vertices: Vec<Vector<T, S>>,
}

impl<T, const S: usize> Default for Polygon<T, S> {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }
}

impl<T: Num, const S: usize> Polygon<T, S> {
    /// Creates a polygon from the given vertices, rotating the smallest one to the front.
    pub fn new(mut vertices: Vec<Vector<T, S>>) -> Self {
        if let Some(min) = vertices
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(i, _)| i)
        {
            vertices.rotate_left(min);
        }
        Self { vertices }
    }

    /// Returns the polygon's vertices, starting with the smallest.
    #[inline]
    pub fn vertices(&self) -> &[Vector<T, S>] {
        &self.vertices
    }

    /// Returns the number of vertices.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns whether the polygon has no vertices.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Returns whether the given point is one of this polygon's vertices.
    #[inline]
    pub fn has_vertex(&self, vertex: Vector<T, S>) -> bool {
        self.vertices.contains(&vertex)
    }

    /// Returns the average of the polygon's vertices, or `None` if it has none.
    pub fn center(&self) -> Option<Vector<T, S>>
    where
        T: Float,
    {
        vec::average(self.vertices.iter().copied())
    }

    /// Returns this polygon with its winding reversed.
    ///
    /// The first vertex stays put and the rest are reversed, which keeps the polygon
    /// anchored at the same vertex while flipping which way round its edges run.
    pub fn invert(&self) -> Self {
        let mut vertices = self.vertices.clone();
        if vertices.len() > 1 {
            vertices[1..].reverse();
        }
        Self::new(vertices)
    }

    /// Returns this polygon moved by the given offset.
    pub fn translate(&self, offset: Vector<T, S>) -> Self {
        Self::new(self.vertices.iter().map(|v| *v + offset).collect())
    }
}

impl<T: Float> Polygon<T, 3> {
    /// Returns this polygon transformed by the given 4x4 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 4, 4>) -> Self {
        Self::new(self.vertices.iter().map(|v| *transform * *v).collect())
    }
}

impl<T: Float> Polygon<T, 2> {
    /// Returns this polygon transformed by the given 3x3 affine transform.
    pub fn transform(&self, transform: &Matrix<T, 3, 3>) -> Self {
        Self::new(self.vertices.iter().map(|v| *transform * *v).collect())
    }
}

/// Compares the given polygons vertex by vertex, treating components that differ by at
/// most `epsilon` as equal.
///
/// Fewer vertices sorts first. When the first vertices match but the second do not, the
/// comparison first tries matching the remaining vertices in reverse: a polygon and its
/// inversion have the same vertices in opposite order, and this makes them compare equal
/// rather than being ordered by an accident of winding.
pub fn compare<T: Num, const S: usize>(
    lhs: &Polygon<T, S>,
    rhs: &Polygon<T, S>,
    epsilon: T,
) -> Ordering {
    let lhs_verts = lhs.vertices();
    let rhs_verts = rhs.vertices();

    match lhs_verts.len().cmp(&rhs_verts.len()) {
        Ordering::Equal => {}
        other => return other,
    }

    let count = lhs_verts.len();
    if count == 0 {
        return Ordering::Equal;
    }

    match vec::compare(&lhs_verts[0], &rhs_verts[0], epsilon) {
        Ordering::Equal => {}
        other => return other,
    }
    if count == 1 {
        return Ordering::Equal;
    }

    let forward = || vec::compare_all(&lhs_verts[2..], &rhs_verts[2..], epsilon);

    if vec::compare(&lhs_verts[1], &rhs_verts[1], epsilon) == Ordering::Equal {
        return forward();
    }

    // The second vertices differ, so try matching the rest in reverse order.
    let reversed_matches = lhs_verts[1..]
        .iter()
        .enumerate()
        .all(|(k, v)| vec::compare(v, &rhs_verts[count - (k + 1)], epsilon) == Ordering::Equal);
    if reversed_matches {
        Ordering::Equal
    } else {
        forward()
    }
}

/// Returns whether the given polygons are equal up to `epsilon`.
#[inline]
pub fn is_equal<T: Num, const S: usize>(
    lhs: &Polygon<T, S>,
    rhs: &Polygon<T, S>,
    epsilon: T,
) -> bool {
    compare(lhs, rhs, epsilon) == Ordering::Equal
}

impl<T: Num, const S: usize> PartialEq for Polygon<T, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        compare(self, other, T::ZERO) == Ordering::Equal
    }
}

impl<T: Num, const S: usize> Eq for Polygon<T, S> {}

impl<T: Num, const S: usize> PartialOrd for Polygon<T, S> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Num, const S: usize> Ord for Polygon<T, S> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        compare(self, other, T::ZERO)
    }
}

/// Returns the vertices of all the given polygons, in order.
pub fn vertices<'a, T: Num + 'a, const S: usize, I>(polygons: I) -> Vec<Vector<T, S>>
where
    I: IntoIterator<Item = &'a Polygon<T, S>>,
{
    polygons
        .into_iter()
        .flat_map(|p| p.vertices().iter().copied())
        .collect()
}

/// A 2D single precision polygon.
pub type Polygon2f = Polygon<f32, 2>;
/// A 2D double precision polygon.
pub type Polygon2d = Polygon<f64, 2>;
/// A 3D single precision polygon.
pub type Polygon3f = Polygon<f32, 3>;
/// A 3D double precision polygon.
pub type Polygon3d = Polygon<f64, 3>;
