//! An `R` by `C` matrix with elements of type `T`.
//!
//! Elements are stored column-major: `m[c][r]` selects row `r` of column `c`. [`Matrix::v`]
//! contains `C` columns of [`Vector<T, R>`], matching the renderer matrix layout.
//!
//! ## Construction and dimensions
//!
//! [`Matrix::from_rows`] and [`Matrix::from_columns`] take nested arrays that make layout explicit.
//! Vector products preserve their mathematical result dimensions. [`Determinant`] provides
//! determinants and adjugates for square matrices up to 4x4.

use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::scalar::Num;
use crate::vec::{self, Vector};

/// An `R` by `C` matrix, stored as `C` columns of `R` elements each.
#[derive(Debug, Clone, Copy)]
pub struct Matrix<T, const R: usize, const C: usize> {
    /// The columns of the matrix.
    pub v: [Vector<T, R>; C],
}

/* ========== construction ========== */

impl<T: Num, const R: usize, const C: usize> Default for Matrix<T, R, C> {
    /// Returns the identity matrix.
    fn default() -> Self {
        Self::identity()
    }
}

impl<T: Num, const R: usize, const C: usize> Matrix<T, R, C> {
    /// The number of rows.
    pub const ROWS: usize = R;
    /// The number of columns.
    pub const COLS: usize = C;

    /// Creates a matrix from its columns.
    #[inline]
    pub const fn from_column_vectors(v: [Vector<T, R>; C]) -> Self {
        Self { v }
    }

    /// Creates a matrix from a nested array of columns, each listing its `R` elements
    /// top to bottom.
    #[inline]
    pub fn from_columns(columns: [[T; R]; C]) -> Self {
        Self {
            v: columns.map(Vector::from_array),
        }
    }

    /// Creates a matrix from a nested array of rows, each listing its `C` elements left
    /// to right.
    ///
    /// Arguments follow the row-major order used when writing a matrix on paper.
    #[inline]
    pub fn from_rows(rows: [[T; C]; R]) -> Self {
        let mut result = Self::zero();
        for (r, row) in rows.iter().enumerate() {
            for (c, value) in row.iter().enumerate() {
                result.v[c].v[r] = *value;
            }
        }
        result
    }

    /// Creates a matrix with every element set to the given value.
    #[inline]
    pub fn fill(value: T) -> Self {
        Self {
            v: [Vector::fill(value); C],
        }
    }

    /// Creates a matrix with every element set to 0.
    #[inline]
    pub fn zero() -> Self {
        Self::fill(T::ZERO)
    }

    /// Creates an identity matrix, i.e. ones on the main diagonal and zeros elsewhere.
    #[inline]
    pub fn identity() -> Self {
        let mut result = Self::zero();
        for c in 0..C {
            for r in 0..R {
                if c == r {
                    result.v[c].v[r] = T::ONE;
                }
            }
        }
        result
    }

    /// Returns the element in row `r` of column `c`.
    #[inline]
    pub fn at(&self, r: usize, c: usize) -> T {
        self.v[c].v[r]
    }

    /// Returns this matrix with the element in row `r` of column `c` set to `value`.
    #[inline]
    pub fn set(mut self, r: usize, c: usize, value: T) -> Self {
        assert!(r < R && c < C, "element index out of range");
        self.v[c].v[r] = value;
        self
    }

    /// Returns the row at the given index.
    #[inline]
    pub fn row(&self, r: usize) -> Vector<T, C> {
        let mut result = Vector::zero();
        for c in 0..C {
            result.v[c] = self.v[c].v[r];
        }
        result
    }

    /// Returns the transpose of this matrix.
    #[inline]
    pub fn transpose(&self) -> Matrix<T, C, R> {
        let mut result = Matrix::<T, C, R>::zero();
        for c in 0..C {
            for r in 0..R {
                result.v[r].v[c] = self.v[c].v[r];
            }
        }
        result
    }

    /// Returns the `RR` by `CC` sub-matrix whose top left element is at row `r`,
    /// column `c`.
    ///
    /// # Panics
    ///
    /// Panics if the slice would extend past the edge of the matrix.
    pub fn slice<const RR: usize, const CC: usize>(
        &self,
        r: usize,
        c: usize,
    ) -> Matrix<T, RR, CC> {
        assert!(RR <= R && CC <= C, "slice must not exceed matrix");
        assert!(r <= R - RR && c <= C - CC, "slice offset out of range");
        let mut result = Matrix::<T, RR, CC>::zero();
        for row in 0..RR {
            for col in 0..CC {
                result.v[col].v[row] = self.v[col + c].v[row + r];
            }
        }
        result
    }
}

impl<T: Num, const S: usize> Matrix<T, S, S> {
    /// Returns a matrix that zeroes out component `e` of any vector it multiplies.
    ///
    /// # Panics
    ///
    /// Panics if `e` is out of range.
    #[inline]
    pub fn zero_out(e: usize) -> Self {
        assert!(e < S, "element index out of range");
        Self::identity().set(e, e, T::ZERO)
    }
}

/// Common 4x4 rotation and reflection matrices.
///
/// They are only defined for 4x4 matrices there, and only 4x4 is useful here: the
/// editor's transforms are all affine, so they carry a translation row.
impl<T: Num> Matrix<T, 4, 4> {
    /// A matrix that rotates by 90 degrees about the X axis, clockwise.
    pub fn rot_90_x_cw() -> Self {
        Self::from_rows([
            [T::ONE, T::ZERO, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, T::ONE, T::ZERO],
            [T::ZERO, -T::ONE, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, T::ZERO, T::ONE],
        ])
    }

    /// A matrix that rotates by 90 degrees about the Y axis, clockwise.
    pub fn rot_90_y_cw() -> Self {
        Self::from_rows([
            [T::ZERO, T::ZERO, -T::ONE, T::ZERO],
            [T::ZERO, T::ONE, T::ZERO, T::ZERO],
            [T::ONE, T::ZERO, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, T::ZERO, T::ONE],
        ])
    }

    /// A matrix that rotates by 90 degrees about the Z axis, clockwise.
    pub fn rot_90_z_cw() -> Self {
        Self::from_rows([
            [T::ZERO, T::ONE, T::ZERO, T::ZERO],
            [-T::ONE, T::ZERO, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, T::ONE, T::ZERO],
            [T::ZERO, T::ZERO, T::ZERO, T::ONE],
        ])
    }

    /// A matrix that rotates by 90 degrees about the X axis, counter-clockwise.
    pub fn rot_90_x_ccw() -> Self {
        Self::from_rows([
            [T::ONE, T::ZERO, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, -T::ONE, T::ZERO],
            [T::ZERO, T::ONE, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, T::ZERO, T::ONE],
        ])
    }

    /// A matrix that rotates by 90 degrees about the Y axis, counter-clockwise.
    pub fn rot_90_y_ccw() -> Self {
        Self::from_rows([
            [T::ZERO, T::ZERO, T::ONE, T::ZERO],
            [T::ZERO, T::ONE, T::ZERO, T::ZERO],
            [-T::ONE, T::ZERO, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, T::ZERO, T::ONE],
        ])
    }

    /// A matrix that rotates by 90 degrees about the Z axis, counter-clockwise.
    pub fn rot_90_z_ccw() -> Self {
        Self::from_rows([
            [T::ZERO, -T::ONE, T::ZERO, T::ZERO],
            [T::ONE, T::ZERO, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, T::ONE, T::ZERO],
            [T::ZERO, T::ZERO, T::ZERO, T::ONE],
        ])
    }

    /// A matrix that rotates by 180 degrees about the X axis.
    pub fn rot_180_x() -> Self {
        Self::diagonal_signs(T::ONE, -T::ONE, -T::ONE)
    }

    /// A matrix that rotates by 180 degrees about the Y axis.
    pub fn rot_180_y() -> Self {
        Self::diagonal_signs(-T::ONE, T::ONE, -T::ONE)
    }

    /// A matrix that rotates by 180 degrees about the Z axis.
    pub fn rot_180_z() -> Self {
        Self::diagonal_signs(-T::ONE, -T::ONE, T::ONE)
    }

    /// A matrix that mirrors along the X axis.
    pub fn mirror_x() -> Self {
        Self::diagonal_signs(-T::ONE, T::ONE, T::ONE)
    }

    /// A matrix that mirrors along the Y axis.
    pub fn mirror_y() -> Self {
        Self::diagonal_signs(T::ONE, -T::ONE, T::ONE)
    }

    /// A matrix that mirrors along the Z axis.
    pub fn mirror_z() -> Self {
        Self::diagonal_signs(T::ONE, T::ONE, -T::ONE)
    }

    /// Builds a diagonal matrix from the three axis signs, with a 1 in the w slot.
    ///
    /// The 180 degree rotations and the mirrors are all of this shape; naming it keeps
    /// six near-identical 4x4 literals out of the file.
    fn diagonal_signs(x: T, y: T, z: T) -> Self {
        Self::from_rows([
            [x, T::ZERO, T::ZERO, T::ZERO],
            [T::ZERO, y, T::ZERO, T::ZERO],
            [T::ZERO, T::ZERO, z, T::ZERO],
            [T::ZERO, T::ZERO, T::ZERO, T::ONE],
        ])
    }
}

/* ========== indexing ========== */

impl<T, const R: usize, const C: usize> Index<usize> for Matrix<T, R, C> {
    type Output = Vector<T, R>;

    /// Returns column `c`, so `m[c][r]` selects row `r` in that column.
    #[inline]
    fn index(&self, c: usize) -> &Vector<T, R> {
        &self.v[c]
    }
}

impl<T, const R: usize, const C: usize> IndexMut<usize> for Matrix<T, R, C> {
    #[inline]
    fn index_mut(&mut self, c: usize) -> &mut Vector<T, R> {
        &mut self.v[c]
    }
}

/* ========== comparison ========== */

/// Compares the given matrices column by column, treating elements that differ by at most
/// `epsilon` as equal.
pub fn compare<T: Num, const R: usize, const C: usize>(
    lhs: &Matrix<T, R, C>,
    rhs: &Matrix<T, R, C>,
    epsilon: T,
) -> Ordering {
    for c in 0..C {
        match vec::compare(&lhs.v[c], &rhs.v[c], epsilon) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

/// Returns whether the given matrices are element-wise equal up to `epsilon`.
#[inline]
pub fn is_equal<T: Num, const R: usize, const C: usize>(
    lhs: &Matrix<T, R, C>,
    rhs: &Matrix<T, R, C>,
    epsilon: T,
) -> bool {
    compare(lhs, rhs, epsilon) == Ordering::Equal
}

/// Returns whether every element of the given matrix is within `epsilon` of 0.
#[inline]
pub fn is_zero<T: Num, const R: usize, const C: usize>(
    m: &Matrix<T, R, C>,
    epsilon: T,
) -> bool {
    (0..C).all(|c| vec::is_zero(m.v[c], epsilon))
}

impl<T: Num, const R: usize, const C: usize> PartialEq for Matrix<T, R, C> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        compare(self, other, T::ZERO) == Ordering::Equal
    }
}

impl<T: Num, const R: usize, const C: usize> Eq for Matrix<T, R, C> {}

impl<T: Num, const R: usize, const C: usize> PartialOrd for Matrix<T, R, C> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Num, const R: usize, const C: usize> Ord for Matrix<T, R, C> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        compare(self, other, T::ZERO)
    }
}

/* ========== arithmetic ========== */

impl<T: Num, const R: usize, const C: usize> Neg for Matrix<T, R, C> {
    type Output = Self;

    #[inline]
    fn neg(mut self) -> Self {
        for column in &mut self.v {
            *column = -*column;
        }
        self
    }
}

macro_rules! impl_matrix_binop {
    ($trait:ident, $method:ident, $assign_trait:ident, $assign_method:ident, $op:tt) => {
        impl<T: Num, const R: usize, const C: usize> $trait for Matrix<T, R, C> {
            type Output = Self;

            #[inline]
            fn $method(mut self, rhs: Self) -> Self {
                for c in 0..C {
                    self.v[c] $op rhs.v[c];
                }
                self
            }
        }

        impl<T: Num, const R: usize, const C: usize> $assign_trait for Matrix<T, R, C> {
            #[inline]
            fn $assign_method(&mut self, rhs: Self) {
                for c in 0..C {
                    self.v[c] $op rhs.v[c];
                }
            }
        }
    };
}

impl_matrix_binop!(Add, add, AddAssign, add_assign, +=);
impl_matrix_binop!(Sub, sub, SubAssign, sub_assign, -=);

/// The matrix product.
impl<T: Num, const R1: usize, const C1R2: usize, const C2: usize> Mul<Matrix<T, C1R2, C2>>
    for Matrix<T, R1, C1R2>
{
    type Output = Matrix<T, R1, C2>;

    #[inline]
    fn mul(self, rhs: Matrix<T, C1R2, C2>) -> Matrix<T, R1, C2> {
        let mut result = Matrix::<T, R1, C2>::zero();
        for c in 0..C2 {
            for r in 0..R1 {
                for i in 0..C1R2 {
                    result.v[c].v[r] += self.v[i].v[r] * rhs.v[c].v[i];
                }
            }
        }
        result
    }
}

/// Multiplies a column vector by this matrix, i.e. `m * v`.
impl<T: Num, const R: usize, const C: usize> Mul<Vector<T, C>> for Matrix<T, R, C> {
    type Output = Vector<T, R>;

    #[inline]
    fn mul(self, rhs: Vector<T, C>) -> Vector<T, R> {
        let mut result = Vector::<T, R>::zero();
        for r in 0..R {
            for c in 0..C {
                result.v[r] += self.v[c].v[r] * rhs.v[c];
            }
        }
        result
    }
}

// Scalar multiplication and division are generated for each supported element type
// rather than written generically over `T`: a blanket `Mul<T> for Matrix<T, R, C>` would
// have to be proven not to overlap the matrix and vector products above.
macro_rules! impl_matrix_scalar {
    ($($t:ty),*) => {$(
        impl<const R: usize, const C: usize> Mul<$t> for Matrix<$t, R, C> {
            type Output = Self;

            #[inline]
            fn mul(mut self, rhs: $t) -> Self {
                for column in &mut self.v { *column *= rhs; }
                self
            }
        }

        impl<const R: usize, const C: usize> MulAssign<$t> for Matrix<$t, R, C> {
            #[inline]
            fn mul_assign(&mut self, rhs: $t) {
                for column in &mut self.v { *column *= rhs; }
            }
        }

        impl<const R: usize, const C: usize> Mul<Matrix<$t, R, C>> for $t {
            type Output = Matrix<$t, R, C>;

            #[inline]
            fn mul(self, mut rhs: Matrix<$t, R, C>) -> Matrix<$t, R, C> {
                // Multiplication is commutative, so the operand order does not matter.
                for column in &mut rhs.v { *column *= self; }
                rhs
            }
        }

        impl<const R: usize, const C: usize> Div<$t> for Matrix<$t, R, C> {
            type Output = Self;

            #[inline]
            fn div(mut self, rhs: $t) -> Self {
                for column in &mut self.v { *column /= rhs; }
                self
            }
        }

        impl<const R: usize, const C: usize> DivAssign<$t> for Matrix<$t, R, C> {
            #[inline]
            fn div_assign(&mut self, rhs: $t) {
                for column in &mut self.v { *column /= rhs; }
            }
        }
    )*};
}

impl_matrix_scalar!(f32, f64, i32, i64);

/// Multiplies a row vector by a matrix, i.e. `v * m`.
///
/// The result has one element per matrix column.
pub fn row_vector_mul<T: Num, const R: usize, const C: usize>(
    lhs: Vector<T, R>,
    rhs: Matrix<T, R, C>,
) -> Vector<T, C> {
    let mut result = Vector::<T, C>::zero();
    for c in 0..C {
        result.v[c] = vec::dot(lhs, rhs.v[c]);
    }
    result
}

/// Applies a 4x4 affine transform to a 3D point, going through homogeneous coordinates.
impl<T: Num> Mul<Vector<T, 3>> for Matrix<T, 4, 4> {
    type Output = Vector<T, 3>;

    #[inline]
    fn mul(self, rhs: Vector<T, 3>) -> Vector<T, 3> {
        (self * rhs.to_homogeneous_coords()).to_cartesian_coords()
    }
}

/// Applies a 3x3 affine transform to a 2D point, going through homogeneous coordinates.
impl<T: Num> Mul<Vector<T, 2>> for Matrix<T, 3, 3> {
    type Output = Vector<T, 2>;

    #[inline]
    fn mul(self, rhs: Vector<T, 2>) -> Vector<T, 2> {
        let h = self * rhs.with_z(T::ONE);
        let w = h.z();
        Vector::<T, 2>::new(h.x() / w, h.y() / w)
    }
}

/* ========== minors, determinants and inverses ========== */

/// Returns the matrix obtained by striking out the given row and column.
///
/// The result's dimensions must be given explicitly as `RM` and `CM`; on stable Rust a
/// function cannot return a `Matrix<T, R - 1, C - 1>` computed from its input's
/// dimensions.
///
/// # Panics
///
/// Panics if `row` or `col` is out of range, or if `RM` and `CM` are not one less than
/// the input's dimensions.
pub fn extract_minor<
    T: Num,
    const R: usize,
    const C: usize,
    const RM: usize,
    const CM: usize,
>(
    m: &Matrix<T, R, C>,
    row: usize,
    col: usize,
) -> Matrix<T, RM, CM> {
    assert!(row < R && col < C, "index out of range");
    assert!(RM + 1 == R && CM + 1 == C, "a minor is one smaller in each dimension");

    let mut result = Matrix::<T, RM, CM>::zero();
    let mut min_c = 0;
    for c in 0..C {
        if c == col {
            continue;
        }
        let mut min_r = 0;
        for r in 0..R {
            if r == row {
                continue;
            }
            result.v[min_c].v[min_r] = m.v[c].v[r];
            min_r += 1;
        }
        min_c += 1;
    }
    result
}

/// Square matrices whose determinant and adjugate can be computed.
///
/// Implemented for square matrices from 1x1 through 4x4, with size-specific minor expansion.
pub trait Determinant<T>: Sized {
    /// Returns the determinant of this matrix.
    fn determinant(&self) -> T;

    /// Returns the adjugate of this matrix, i.e. the transpose of its cofactor matrix.
    fn adjugate(&self) -> Self;
}

impl<T: Num> Determinant<T> for Matrix<T, 1, 1> {
    #[inline]
    fn determinant(&self) -> T {
        self.v[0].v[0]
    }

    #[inline]
    fn adjugate(&self) -> Self {
        Matrix::from_rows([[T::ONE]])
    }
}

impl<T: Num> Determinant<T> for Matrix<T, 2, 2> {
    #[inline]
    fn determinant(&self) -> T {
        self.at(0, 0) * self.at(1, 1) - self.at(1, 0) * self.at(0, 1)
    }

    #[inline]
    fn adjugate(&self) -> Self {
        Matrix::from_rows([
            [self.at(1, 1), -self.at(0, 1)],
            [-self.at(1, 0), self.at(0, 0)],
        ])
    }
}

impl<T: Num> Determinant<T> for Matrix<T, 3, 3> {
    #[inline]
    fn determinant(&self) -> T {
        self.at(0, 0) * self.at(1, 1) * self.at(2, 2)
            + self.at(0, 1) * self.at(1, 2) * self.at(2, 0)
            + self.at(0, 2) * self.at(1, 0) * self.at(2, 1)
            - self.at(0, 2) * self.at(1, 1) * self.at(2, 0)
            - self.at(0, 1) * self.at(1, 0) * self.at(2, 2)
            - self.at(0, 0) * self.at(1, 2) * self.at(2, 1)
    }

    fn adjugate(&self) -> Self {
        adjugate_via_minors::<T, 3, 2>(self)
    }
}

impl<T: Num> Determinant<T> for Matrix<T, 4, 4> {
    /// Laplace expansion along the first column.
    fn determinant(&self) -> T {
        let mut result = T::ZERO;
        for r in 0..4 {
            let sign = if r % 2 == 0 { T::ONE } else { -T::ONE };
            let minor: Matrix<T, 3, 3> = extract_minor(self, r, 0);
            result += sign * self.at(r, 0) * minor.determinant();
        }
        result
    }

    fn adjugate(&self) -> Self {
        adjugate_via_minors::<T, 4, 3>(self)
    }
}

/// Builds the adjugate of an `S` by `S` matrix from its `M` by `M` minors, where `M` is
/// `S - 1`.
///
/// Writing the cofactor into `result[r][c]` rather than `result[c][r]` transposes the
/// cofactor matrix on the fly, which is what makes the result the adjugate rather than
/// the cofactor matrix.
fn adjugate_via_minors<T: Num, const S: usize, const M: usize>(
    m: &Matrix<T, S, S>,
) -> Matrix<T, S, S>
where
    Matrix<T, M, M>: Determinant<T>,
{
    let mut result = Matrix::<T, S, S>::zero();
    for c in 0..S {
        for r in 0..S {
            let sign = if (c + r) % 2 == 0 { T::ONE } else { -T::ONE };
            let minor: Matrix<T, M, M> = extract_minor(m, r, c);
            // Row and column are swapped here: the cofactor for (r, c) is stored at
            // (c, r), which transposes the cofactor matrix on the fly and is what makes
            // the result the adjugate.
            result.v[r].v[c] = sign * minor.determinant();
        }
    }
    result
}

/// The pivot magnitude below which [`lup_decompose`] declares a matrix singular.
///
/// Uses a fixed threshold for detecting singularity.
const LUP_SINGULAR_THRESHOLD: f64 = 1.0e-15;

/// An LUP decomposition of a square matrix: `P * A = L * U`.
///
/// `L` and `U` are packed into a single matrix, with `L`'s implicit unit diagonal left
/// out, and the permutation `P` is compressed to one row index per row.
#[derive(Debug, Clone, Copy)]
pub struct Lup<T, const S: usize> {
    /// `L` and `U` packed into one matrix.
    pub lu: Matrix<T, S, S>,
    /// The permutation: `pi[i]` is the index of the column set to 1 in row `i` of `P`.
    pub pi: [usize; S],
}

/// Finds an LUP decomposition of the given matrix, or `None` if it is singular.
///
/// From "LUP-Decomposition", *Introduction to Algorithms* by Cormen et al., 2nd ed. p752.
pub fn lup_decompose<T: Num, const S: usize>(mut a: Matrix<T, S, S>) -> Option<Lup<T, S>> {
    let mut pi = [0usize; S];
    for (i, slot) in pi.iter_mut().enumerate() {
        *slot = i;
    }

    for k in 0..S {
        let mut p = T::ZERO;
        let mut k_prime = 0;
        for i in k..S {
            let candidate = crate::scalar::abs(a.v[k].v[i]);
            if candidate > p {
                p = candidate;
                k_prime = i;
            }
        }
        if p.to_f64() < LUP_SINGULAR_THRESHOLD {
            return None;
        }

        pi.swap(k, k_prime);
        // Swap rows k and k_prime across every column.
        for i in 0..S {
            a.v[i].v.swap(k, k_prime);
        }

        for i in k + 1..S {
            a.v[k].v[i] /= a.v[k].v[k];
            for j in k + 1..S {
                a.v[j].v[i] -= a.v[k].v[i] * a.v[j].v[k];
            }
        }
    }

    Some(Lup { lu: a, pi })
}

/// Solves `a * x = b` given an already computed decomposition of `a`.
///
/// From "LUP-Solve", *Introduction to Algorithms* by Cormen et al., 2nd ed. p745.
fn lup_solve_with<T: Num, const S: usize>(lup: &Lup<T, S>, b: Vector<T, S>) -> Vector<T, S> {
    let mut x = Vector::<T, S>::zero();
    let mut y = Vector::<T, S>::zero();

    // Forward substitution through L.
    for i in 0..S {
        let mut sum = T::ZERO;
        for j in 0..i {
            sum += lup.lu.v[j].v[i] * y.v[j];
        }
        y.v[i] = b.v[lup.pi[i]] - sum;
    }

    // Back substitution through U.
    for i in (0..S).rev() {
        let mut sum = T::ZERO;
        for j in i + 1..S {
            sum += lup.lu.v[j].v[i] * x.v[j];
        }
        x.v[i] = (y.v[i] - sum) / lup.lu.v[i].v[i];
    }

    x
}

/// Solves the system `a * x = b` using LU factorization with pivoting.
///
/// Returns `None` if no solution could be found.
pub fn lup_solve<T: Num, const S: usize>(
    a: Matrix<T, S, S>,
    b: Vector<T, S>,
) -> Option<Vector<T, S>> {
    lup_decompose(a).map(|lup| lup_solve_with(&lup, b))
}

/// Inverts the given square matrix, or returns `None` if it is not invertible.
///
/// From "Computing a matrix inverse from an LUP decomposition", *Introduction to
/// Algorithms* by Cormen et al., 2nd ed. p755.
pub fn invert<T: Num, const S: usize>(m: Matrix<T, S, S>) -> Option<Matrix<T, S, S>> {
    let lup = lup_decompose(m)?;

    let mut result = Matrix::<T, S, S>::zero();
    for i in 0..S {
        // Solve against the ith column of the identity matrix.
        let mut target = Vector::<T, S>::zero();
        target.v[i] = T::ONE;
        result.v[i] = lup_solve_with(&lup, target);
    }
    Some(result)
}

/* ========== dimension aliases ========== */

macro_rules! aliases {
    ($($name:ident = $t:ty, $s:literal;)*) => {$(
        #[doc = concat!("A ", stringify!($s), "x", stringify!($s), " `", stringify!($t), "` matrix.")]
        pub type $name = Matrix<$t, $s, $s>;
    )*};
}

aliases! {
    Mat2x2f = f32, 2; Mat3x3f = f32, 3; Mat4x4f = f32, 4;
    Mat2x2d = f64, 2; Mat3x3d = f64, 3; Mat4x4d = f64, 4;
}
