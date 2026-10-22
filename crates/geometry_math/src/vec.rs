//! An `S`-dimensional vector with components of type `T`.
//!
//! [`Vector<T, S>`] uses a const-generic dimension and numeric component type. Aliases such as
//! [`Vec3d`] cover common combinations.
//!

use std::cmp::Ordering;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use crate::scalar::{self, Float, Num};

/// An `S`-dimensional vector with components of type `T`.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Vector<T, const S: usize> {
    /// The vector components.
    pub v: [T; S],
}

/* ========== construction ========== */

impl<T: Num, const S: usize> Default for Vector<T, S> {
    fn default() -> Self {
        Self::fill(T::ZERO)
    }
}

impl<T: Num, const S: usize> Vector<T, S> {
    /// The number of components.
    pub const SIZE: usize = S;

    /// Creates a vector from an array of components.
    #[inline]
    pub const fn from_array(v: [T; S]) -> Self {
        Self { v }
    }

    /// Creates a vector with every component set to the given value.
    #[inline]
    pub fn fill(value: T) -> Self {
        Self { v: [value; S] }
    }

    /// Creates a vector with all components set to 0.
    #[inline]
    pub fn zero() -> Self {
        Self::fill(T::ZERO)
    }

    /// Creates a vector with all components set to 1.
    #[inline]
    pub fn one() -> Self {
        Self::fill(T::ONE)
    }

    /// Creates a unit vector along the given axis, i.e. with component `index` set to 1
    /// and every other component set to 0.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[inline]
    pub fn axis(index: usize) -> Self {
        assert!(
            index < S,
            "axis index {index} out of range for {S} components"
        );
        let mut result = Self::zero();
        result.v[index] = T::ONE;
        result
    }

    /// Creates a vector from another vector of a possibly different size, truncating or
    /// zero-padding as needed.
    #[inline]
    pub fn from_vec<const V: usize>(other: Vector<T, V>) -> Self {
        let mut result = Self::zero();
        for i in 0..S.min(V) {
            result.v[i] = other.v[i];
        }
        result
    }

    /// Returns the sub-vector of `RS` components starting at `offset`.
    ///
    /// # Panics
    ///
    /// Panics if the slice would extend past the end of the vector.
    #[inline]
    pub fn slice<const RS: usize>(&self, offset: usize) -> Vector<T, RS> {
        assert!(RS <= S, "slice must not exceed vector");
        assert!(offset <= S - RS, "slice offset out of range");
        let mut result = Vector::<T, RS>::zero();
        result.v[..RS].copy_from_slice(&self.v[offset..offset + RS]);
        result
    }
}

impl<T: Float, const S: usize> Vector<T, S> {
    /// Creates a vector with all components set to NaN.
    #[inline]
    pub fn nan() -> Self {
        Self::fill(T::nan())
    }
}

impl<T: Num, const S: usize> Vector<T, S> {
    /// Creates a vector with all components set to the most negative representable
    /// value.
    ///
    /// Uses [`Num::LOWEST`] as the lower bound, including negative values for floating-point
    /// components.
    #[inline]
    pub fn min_value() -> Self {
        Self::fill(T::LOWEST)
    }

    /// Creates a vector with all components set to the largest representable value.
    #[inline]
    pub fn max_value() -> Self {
        Self::fill(T::HIGHEST)
    }
}

/* ========== indexing ========== */

impl<T, const S: usize> Index<usize> for Vector<T, S> {
    type Output = T;

    #[inline]
    fn index(&self, i: usize) -> &T {
        &self.v[i]
    }
}

impl<T, const S: usize> IndexMut<usize> for Vector<T, S> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.v[i]
    }
}

/* ========== named component accessors ========== */

macro_rules! impl_accessors {
    ($size:literal; $($name:ident => $idx:literal),*) => {
        impl<T: Num> Vector<T, $size> {
            $(
                #[doc = concat!("Returns the `", stringify!($name), "` component.")]
                #[inline]
                pub fn $name(&self) -> T { self.v[$idx] }
            )*
        }
    };
}

macro_rules! impl_new {
    ($size:literal; $($arg:ident),*) => {
        impl<T: Num> Vector<T, $size> {
            #[doc = concat!("Creates a ", stringify!($size), "-dimensional vector from its components.")]
            #[inline]
            pub const fn new($($arg: T),*) -> Self {
                Self { v: [$($arg),*] }
            }
        }
    };
}

impl_new!(1; x);
impl_new!(2; x, y);
impl_new!(3; x, y, z);
impl_new!(4; x, y, z, w);

impl_accessors!(1; x => 0);
impl_accessors!(2; x => 0, y => 1);
impl_accessors!(3; x => 0, y => 1, z => 2);
impl_accessors!(4; x => 0, y => 1, z => 2, w => 3);

macro_rules! impl_swizzles {
    ($($size:literal),*) => {$(
        impl<T: Num> Vector<T, $size> {
            /// Returns the x and y components as a 2D vector.
            #[inline]
            pub fn xy(&self) -> Vector<T, 2> { Vector::from_array([self.v[0], self.v[1]]) }
            /// Returns the x and z components as a 2D vector.
            #[inline]
            pub fn xz(&self) -> Vector<T, 2> { Vector::from_array([self.v[0], self.v[2]]) }
            /// Returns the y and z components as a 2D vector.
            #[inline]
            pub fn yz(&self) -> Vector<T, 2> { Vector::from_array([self.v[1], self.v[2]]) }
            /// Returns the x, y and z components as a 3D vector.
            #[inline]
            pub fn xyz(&self) -> Vector<T, 3> {
                Vector::from_array([self.v[0], self.v[1], self.v[2]])
            }
        }
    )*};
}

impl_swizzles!(3, 4);

impl<T: Num> Vector<T, 2> {
    /// Returns this 2D vector extended to 3D with the given z component.
    #[inline]
    pub fn with_z(&self, z: T) -> Vector<T, 3> {
        Vector::from_array([self.v[0], self.v[1], z])
    }
}

impl<T: Num> Vector<T, 3> {
    /// Returns this 3D vector extended to 4D with the given w component.
    #[inline]
    pub fn with_w(&self, w: T) -> Vector<T, 4> {
        Vector::from_array([self.v[0], self.v[1], self.v[2], w])
    }

    /// Returns this point in homogeneous coordinates, i.e. extended with a w of 1.
    #[inline]
    pub fn to_homogeneous_coords(&self) -> Vector<T, 4> {
        self.with_w(T::ONE)
    }
}

impl<T: Num> Vector<T, 4> {
    /// Returns the x, y, z and w components as a 4D vector, i.e. a copy.
    #[inline]
    pub fn xyzw(&self) -> Vector<T, 4> {
        *self
    }

    /// Converts this point from homogeneous to cartesian coordinates by dividing the
    /// x, y and z components by w.
    #[inline]
    pub fn to_cartesian_coords(&self) -> Vector<T, 3> {
        let w = self.v[3];
        Vector::from_array([self.v[0] / w, self.v[1] / w, self.v[2] / w])
    }
}

/* ========== comparison ========== */

/// Lexicographically compares the given vectors component by component, treating
/// components that differ by at most `epsilon` as equal.
///
/// NaNs sort after all other components and compare equal to one another. With zero epsilon, this
/// defines the ordering used by [`Ord`] and [`Eq`].
pub fn compare<T: Num, const S: usize>(
    lhs: &Vector<T, S>,
    rhs: &Vector<T, S>,
    epsilon: T,
) -> Ordering {
    for i in 0..S {
        // A value is NaN exactly when it is not equal to itself. Testing it this way
        // rather than via `Float::is_nan` keeps this usable for integer components too.
        #[allow(clippy::eq_op)]
        let lhs_is_nan = lhs.v[i] != lhs.v[i];
        #[allow(clippy::eq_op)]
        let rhs_is_nan = rhs.v[i] != rhs.v[i];
        match (lhs_is_nan, rhs_is_nan) {
            (false, true) => return Ordering::Less,
            (true, false) => return Ordering::Greater,
            (true, true) => {}
            (false, false) => {
                if lhs.v[i] < rhs.v[i] - epsilon {
                    return Ordering::Less;
                }
                if lhs.v[i] > rhs.v[i] + epsilon {
                    return Ordering::Greater;
                }
            }
        }
    }
    Ordering::Equal
}

/// Lexicographically compares two sequences of vectors.
///
/// The sequences are walked in parallel and their elements compared with [`compare`]. If
/// one sequence runs out first it is considered the lesser one.
pub fn compare_all<'a, T, const S: usize, L, R>(lhs: L, rhs: R, epsilon: T) -> Ordering
where
    T: Num + 'a,
    L: IntoIterator<Item = &'a Vector<T, S>>,
    R: IntoIterator<Item = &'a Vector<T, S>>,
{
    let mut lhs = lhs.into_iter();
    let mut rhs = rhs.into_iter();
    loop {
        match (lhs.next(), rhs.next()) {
            (Some(l), Some(r)) => match compare(l, r, epsilon) {
                Ordering::Equal => {}
                other => return other,
            },
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (None, None) => return Ordering::Equal,
        }
    }
}

/// Returns whether the given vectors are component-wise equal up to `epsilon`.
///
/// Unlike `==`, this takes an epsilon into account.
#[inline]
pub fn is_equal<T: Num, const S: usize>(
    lhs: &Vector<T, S>,
    rhs: &Vector<T, S>,
    epsilon: T,
) -> bool {
    compare(lhs, rhs, epsilon) == Ordering::Equal
}

impl<T: Num, const S: usize> PartialEq for Vector<T, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        compare(self, other, T::ZERO) == Ordering::Equal
    }
}

impl<T: Num, const S: usize> Eq for Vector<T, S> {}

impl<T: Num, const S: usize> PartialOrd for Vector<T, S> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Num, const S: usize> Ord for Vector<T, S> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        compare(self, other, T::ZERO)
    }
}

/* ========== component search ========== */

impl<T: Num, const S: usize> Vector<T, S> {
    /// Returns the index of the k-largest component.
    ///
    /// If several components tie, the index of any of them may be returned.
    ///
    /// # Panics
    ///
    /// Panics if `k >= S`.
    pub fn find_max_component(&self, k: usize) -> usize {
        self.find_max_component_by(k, |v| v)
    }

    /// Returns the index of the k-largest component by absolute value.
    ///
    /// # Panics
    ///
    /// Panics if `k >= S`.
    pub fn find_abs_max_component(&self, k: usize) -> usize {
        self.find_max_component_by(k, scalar::abs)
    }

    fn find_max_component_by(&self, k: usize, key: impl Fn(T) -> T) -> usize {
        assert!(k < S, "k out of range");
        let mut indices: [usize; S] = [0; S];
        for (i, slot) in indices.iter_mut().enumerate() {
            *slot = i;
        }
        // Sort ascending, then index from the end to visit the largest components first.
        indices.sort_by(|&a, &b| {
            key(self.v[a])
                .partial_cmp(&key(self.v[b]))
                .unwrap_or(Ordering::Equal)
        });
        indices[S - k - 1]
    }

    /// Returns the coordinate system axis matching the k-largest absolute component,
    /// negated if that component is negative.
    ///
    /// For example, `(-1, 3, -5)` with `k = 0` returns the negative Z axis, and with
    /// `k = 1` returns the positive Y axis.
    pub fn get_abs_max_component_axis(&self, k: usize) -> Self {
        let index = self.find_abs_max_component(k);
        let result = Self::axis(index);
        if self.v[index] < T::ZERO {
            -result
        } else {
            result
        }
    }

    /// Returns the value of the k-largest component.
    ///
    /// # Panics
    ///
    /// Panics if `k >= S`.
    pub fn get_max_component(&self, k: usize) -> T {
        self.v[self.find_max_component(k)]
    }

    /// Returns the value of the k-largest component by absolute value.
    ///
    /// Note that this returns the component itself, not its absolute value.
    ///
    /// # Panics
    ///
    /// Panics if `k >= S`.
    pub fn get_abs_max_component(&self, k: usize) -> T {
        self.v[self.find_abs_max_component(k)]
    }
}

/* ========== arithmetic operators ========== */

impl<T: Num, const S: usize> Neg for Vector<T, S> {
    type Output = Self;

    #[inline]
    fn neg(mut self) -> Self {
        for c in &mut self.v {
            *c = -*c;
        }
        self
    }
}

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $assign_trait:ident, $assign_method:ident, $op:tt) => {
        impl<T: Num, const S: usize> $trait for Vector<T, S> {
            type Output = Self;

            #[inline]
            fn $method(mut self, rhs: Self) -> Self {
                for i in 0..S {
                    self.v[i] $op rhs.v[i];
                }
                self
            }
        }

        impl<T: Num, const S: usize> $trait<T> for Vector<T, S> {
            type Output = Self;

            #[inline]
            fn $method(mut self, rhs: T) -> Self {
                for c in &mut self.v {
                    *c $op rhs;
                }
                self
            }
        }

        impl<T: Num, const S: usize> $assign_trait for Vector<T, S> {
            #[inline]
            fn $assign_method(&mut self, rhs: Self) {
                for i in 0..S {
                    self.v[i] $op rhs.v[i];
                }
            }
        }

        impl<T: Num, const S: usize> $assign_trait<T> for Vector<T, S> {
            #[inline]
            fn $assign_method(&mut self, rhs: T) {
                for c in &mut self.v {
                    *c $op rhs;
                }
            }
        }
    };
}

impl_binop!(Add, add, AddAssign, add_assign, +=);
impl_binop!(Sub, sub, SubAssign, sub_assign, -=);
impl_binop!(Mul, mul, MulAssign, mul_assign, *=);
impl_binop!(Div, div, DivAssign, div_assign, /=);

// `scalar * vector` and `scalar / vector` cannot be written generically: the orphan rules
// forbid implementing a foreign trait for a bare type parameter. They are generated for
// each supported component type instead.
macro_rules! impl_scalar_lhs {
    ($($t:ty),*) => {$(
        impl<const S: usize> Mul<Vector<$t, S>> for $t {
            type Output = Vector<$t, S>;

            #[inline]
            fn mul(self, mut rhs: Vector<$t, S>) -> Vector<$t, S> {
                // Multiplication is commutative, so the operand order does not matter here.
                for c in &mut rhs.v { *c *= self; }
                rhs
            }
        }

        impl<const S: usize> Div<Vector<$t, S>> for $t {
            type Output = Vector<$t, S>;

            #[inline]
            fn div(self, mut rhs: Vector<$t, S>) -> Vector<$t, S> {
                // Division is not commutative: the scalar stays on the left.
                for c in &mut rhs.v { *c = self / *c; }
                rhs
            }
        }
    )*};
}

impl_scalar_lhs!(f32, f64, i32, i64);

/* ========== component-wise functions ========== */

macro_rules! componentwise {
    ($(#[$meta:meta])* $name:ident, $bound:ident, |$a:ident| $body:expr) => {
        $(#[$meta])*
        #[inline]
        pub fn $name<T: $bound, const S: usize>(mut v: Vector<T, S>) -> Vector<T, S> {
            for c in &mut v.v {
                let $a = *c;
                *c = $body;
            }
            v
        }
    };
}

macro_rules! componentwise2 {
    ($(#[$meta:meta])* $name:ident, $bound:ident, |$a:ident, $b:ident| $body:expr) => {
        $(#[$meta])*
        #[inline]
        pub fn $name<T: $bound, const S: usize>(
            mut lhs: Vector<T, S>,
            rhs: Vector<T, S>,
        ) -> Vector<T, S> {
            for i in 0..S {
                let $a = lhs.v[i];
                let $b = rhs.v[i];
                lhs.v[i] = $body;
            }
            lhs
        }
    };
}

componentwise2!(
    /// Returns the component-wise minimum of the given vectors.
    min, Num, |a, b| scalar::min(a, b)
);
componentwise2!(
    /// Returns the component-wise maximum of the given vectors.
    max, Num, |a, b| scalar::max(a, b)
);
componentwise2!(
    /// Returns the component-wise minimum by absolute value.
    ///
    /// Note that each result component is the component itself, not its absolute value.
    abs_min, Num, |a, b| scalar::abs_min(a, b)
);
componentwise2!(
    /// Returns the component-wise maximum by absolute value.
    ///
    /// Note that each result component is the component itself, not its absolute value.
    abs_max, Num, |a, b| scalar::abs_max(a, b)
);
componentwise2!(
    /// Returns the component-wise remainder of `v / f`.
    mod_, Float, |a, b| scalar::mod_(a, b)
);

componentwise!(
    /// Returns the component-wise absolute value.
    abs, Num, |a| scalar::abs(a)
);
componentwise!(
    /// Returns the component-wise sign, i.e. -1, 0 or +1 per component.
    sign, Num, |a| scalar::sign(a)
);
componentwise!(
    /// Returns the component-wise fractional part.
    fract, Float, |a| scalar::fract(a)
);
componentwise!(
    /// Returns the component-wise largest integer not greater than each component.
    floor, Float, |a| scalar::floor(a)
);
componentwise!(
    /// Returns the component-wise smallest integer not less than each component.
    ceil, Float, |a| scalar::ceil(a)
);
componentwise!(
    /// Returns the component-wise value rounded towards 0.
    trunc, Float, |a| scalar::trunc(a)
);
componentwise!(
    /// Returns the component-wise value rounded to the nearest integer.
    round, Float, |a| scalar::round(a)
);

/// Clamps each component of `v` to the interval given by the corresponding components of
/// `min_val` and `max_val`.
#[inline]
pub fn clamp<T: Num, const S: usize>(
    mut v: Vector<T, S>,
    min_val: Vector<T, S>,
    max_val: Vector<T, S>,
) -> Vector<T, S> {
    for i in 0..S {
        v.v[i] = scalar::clamp(v.v[i], min_val.v[i], max_val.v[i]);
    }
    v
}

/// Returns 0 or 1 per component depending on whether the component of `v` is less than
/// the corresponding edge component of `e`.
#[inline]
pub fn step<T: Num, const S: usize>(e: Vector<T, S>, mut v: Vector<T, S>) -> Vector<T, S> {
    for i in 0..S {
        v.v[i] = scalar::step(e.v[i], v.v[i]);
    }
    v
}

/// Performs component-wise smooth Hermite interpolation between 0 and 1.
#[inline]
pub fn smoothstep<T: Float, const S: usize>(
    e0: Vector<T, S>,
    e1: Vector<T, S>,
    mut v: Vector<T, S>,
) -> Vector<T, S> {
    for i in 0..S {
        v.v[i] = scalar::smoothstep(e0.v[i], e1.v[i], v.v[i]);
    }
    v
}

/// Component-wise linear interpolation between `lhs` and `rhs` using the weights `f`.
#[inline]
pub fn mix<T: Num, const S: usize>(
    mut lhs: Vector<T, S>,
    rhs: Vector<T, S>,
    f: Vector<T, S>,
) -> Vector<T, S> {
    for i in 0..S {
        lhs.v[i] = scalar::mix(lhs.v[i], rhs.v[i], f.v[i]);
    }
    lhs
}

/// Rounds each component towards 0 to the nearest multiple of the corresponding
/// component of `m`.
#[inline]
pub fn snap_down<T: Float, const S: usize>(mut v: Vector<T, S>, m: Vector<T, S>) -> Vector<T, S> {
    for i in 0..S {
        v.v[i] = scalar::snap_down(v.v[i], m.v[i]);
    }
    v
}

/// Rounds each component away from 0 to the nearest multiple of the corresponding
/// component of `m`.
#[inline]
pub fn snap_up<T: Float, const S: usize>(mut v: Vector<T, S>, m: Vector<T, S>) -> Vector<T, S> {
    for i in 0..S {
        v.v[i] = scalar::snap_up(v.v[i], m.v[i]);
    }
    v
}

/// Rounds each component to the nearest multiple of the corresponding component of `m`.
#[inline]
pub fn snap<T: Float, const S: usize>(mut v: Vector<T, S>, m: Vector<T, S>) -> Vector<T, S> {
    for i in 0..S {
        v.v[i] = scalar::snap(v.v[i], m.v[i]);
    }
    v
}

/// Rounds each component to the nearest integer if it is within `epsilon` of it,
/// retaining at most `decimals` decimal places.
#[inline]
pub fn correct<T: Float, const S: usize>(
    mut v: Vector<T, S>,
    decimals: u32,
    epsilon: T,
) -> Vector<T, S> {
    for c in &mut v.v {
        *c = scalar::correct(*c, decimals, epsilon);
    }
    v
}

/* ========== products and lengths ========== */

/// Returns the dot product of the given vectors.
#[inline]
pub fn dot<T: Num, const S: usize>(lhs: Vector<T, S>, rhs: Vector<T, S>) -> T {
    let mut result = T::ZERO;
    for i in 0..S {
        result += lhs.v[i] * rhs.v[i];
    }
    result
}

/// Returns the cross product of the given 3D vectors.
#[inline]
pub fn cross<T: Num>(lhs: Vector<T, 3>, rhs: Vector<T, 3>) -> Vector<T, 3> {
    Vector::from_array([
        lhs.v[1] * rhs.v[2] - lhs.v[2] * rhs.v[1],
        lhs.v[2] * rhs.v[0] - lhs.v[0] * rhs.v[2],
        lhs.v[0] * rhs.v[1] - lhs.v[1] * rhs.v[0],
    ])
}

/// Returns the squared length of the given vector.
///
/// Prefer this over [`length`] where only relative magnitudes matter — it avoids a square
/// root and is exact for integer components.
#[inline]
pub fn squared_length<T: Num, const S: usize>(v: Vector<T, S>) -> T {
    dot(v, v)
}

/// Returns the length of the given vector.
#[inline]
pub fn length<T: Float, const S: usize>(v: Vector<T, S>) -> T {
    squared_length(v).sqrt()
}

/// Returns the given vector scaled to unit length.
#[inline]
pub fn normalize<T: Float, const S: usize>(v: Vector<T, S>) -> Vector<T, S> {
    v / length(v)
}

/// Returns the squared distance between the given points.
#[inline]
pub fn squared_distance<T: Num, const S: usize>(lhs: Vector<T, S>, rhs: Vector<T, S>) -> T {
    squared_length(lhs - rhs)
}

/// Returns the distance between the given points.
#[inline]
pub fn distance<T: Float, const S: usize>(lhs: Vector<T, S>, rhs: Vector<T, S>) -> T {
    length(lhs - rhs)
}

/* ========== predicates ========== */

/// Returns whether the given vector has unit length up to `epsilon`.
#[inline]
pub fn is_unit<T: Float, const S: usize>(v: Vector<T, S>, epsilon: T) -> bool {
    scalar::is_equal(length(v), T::ONE, epsilon)
}

/// Returns whether every component of the given vector is within `epsilon` of 0.
#[inline]
pub fn is_zero<T: Num, const S: usize>(v: Vector<T, S>, epsilon: T) -> bool {
    (0..S).all(|i| scalar::is_zero(v.v[i], epsilon))
}

/// Returns whether any component of the given vector is NaN.
#[inline]
pub fn is_nan<T: Float, const S: usize>(v: Vector<T, S>) -> bool {
    (0..S).any(|i| v.v[i].is_nan())
}

/// Returns whether every component is within `epsilon` of an integer value.
#[inline]
pub fn is_integral<T: Float, const S: usize>(v: Vector<T, S>, epsilon: T) -> bool {
    (0..S).all(|i| scalar::abs(scalar::round(v.v[i]) - v.v[i]) <= epsilon)
}

/// Returns whether the three given points are colinear.
///
/// Uses the identity from <http://math.stackexchange.com/a/1778739>: the points are
/// colinear exactly when the Cauchy-Schwarz inequality is tight for the vectors between
/// them.
pub fn is_colinear<T: Num, const S: usize>(
    a: Vector<T, S>,
    b: Vector<T, S>,
    c: Vector<T, S>,
    epsilon: T,
) -> bool {
    let mut j = T::ZERO;
    let mut k = T::ZERO;
    let mut l = T::ZERO;
    for i in 0..S {
        let ac = a.v[i] - c.v[i];
        let ba = b.v[i] - a.v[i];
        j += ac * ba;
        k += ac * ac;
        l += ba * ba;
    }
    scalar::is_zero(j * j - k * l, epsilon)
}

/// Returns whether the given vectors point in the same or in opposite directions.
#[inline]
pub fn is_parallel<T: Float, const S: usize>(
    lhs: Vector<T, S>,
    rhs: Vector<T, S>,
    epsilon: T,
) -> bool {
    let cos = dot(normalize(lhs), normalize(rhs));
    scalar::is_equal(scalar::abs(cos), T::ONE, epsilon)
}

/// Returns whether the point `p` lies on the segment from `start` to `end`.
///
/// The three points are expected to be colinear; the result is undefined otherwise.
pub fn is_between<T: Float, const S: usize>(
    p: Vector<T, S>,
    start: Vector<T, S>,
    end: Vector<T, S>,
) -> bool {
    debug_assert!(is_colinear(p, start, end, T::COLINEAR_EPSILON));

    if p == start || p == end {
        return true;
    }
    let to_start = start - p;
    let to_end = end - p;
    dot(to_end, normalize(to_start)) < T::ZERO
}

/* ========== misc ========== */

/// Rearranges the components of a 3D vector:
///
/// - `0`: `x y z -> y z x`
/// - `1`: `x y z -> z x y`
/// - anything else: unchanged
#[inline]
pub fn swizzle<T: Num>(point: Vector<T, 3>, axis: usize) -> Vector<T, 3> {
    match axis {
        0 => Vector::from_array([point.v[1], point.v[2], point.v[0]]),
        1 => Vector::from_array([point.v[2], point.v[0], point.v[1]]),
        _ => point,
    }
}

/// Undoes the effect of [`swizzle`] for the same axis.
#[inline]
pub fn unswizzle<T: Num>(point: Vector<T, 3>, axis: usize) -> Vector<T, 3> {
    match axis {
        0 => Vector::from_array([point.v[2], point.v[0], point.v[1]]),
        1 => Vector::from_array([point.v[1], point.v[2], point.v[0]]),
        _ => point,
    }
}

/// Returns the average of the given vectors, or `None` if the iterator is empty.
///
/// Returns `None` for an empty input.
pub fn average<T: Float, const S: usize, I: IntoIterator<Item = Vector<T, S>>>(
    values: I,
) -> Option<Vector<T, S>> {
    let mut iter = values.into_iter();
    let mut result = iter.next()?;
    let mut count = T::ONE;
    for v in iter {
        result += v;
        count += T::ONE;
    }
    Some(result / count)
}

/// Computes the counter-clockwise angle from `axis` to `v` about the given `up` vector.
///
/// All three vectors are expected to be normalized. The result is the angle by which
/// `axis` must be rotated counter-clockwise about `up` to become `v`, in `[0, 2*PI)`.
pub fn measure_angle_3d<T: Float>(v: Vector<T, 3>, axis: Vector<T, 3>, up: Vector<T, 3>) -> T {
    let cos_angle = scalar::clamp(dot(v, axis), -T::ONE, T::ONE);
    let angle = cos_angle.acos();

    let perp = cross(axis, v);
    if dot(perp, up) >= T::ZERO {
        angle
    } else {
        T::TWO_PI - angle
    }
}

/// Computes the counter-clockwise angle from `axis` to `v` in the plane.
///
/// Both vectors are expected to be normalized.
pub fn measure_angle_2d<T: Float>(v: Vector<T, 2>, axis: Vector<T, 2>) -> T {
    measure_angle_3d(
        v.with_z(T::ZERO),
        axis.with_z(T::ZERO),
        Vector::from_array([T::ZERO, T::ZERO, T::ONE]),
    )
}

/* ========== dimension aliases ========== */

macro_rules! aliases {
    ($($name:ident = $t:ty, $s:literal;)*) => {$(
        #[doc = concat!("A ", stringify!($s), "-dimensional `", stringify!($t), "` vector.")]
        pub type $name = Vector<$t, $s>;
    )*};
}

aliases! {
    Vec1f = f32, 1; Vec1d = f64, 1; Vec1i = i32, 1; Vec1l = i64, 1;
    Vec2f = f32, 2; Vec2d = f64, 2; Vec2i = i32, 2; Vec2l = i64, 2;
    Vec3f = f32, 3; Vec3d = f64, 3; Vec3i = i32, 3; Vec3l = i64, 3;
    Vec4f = f32, 4; Vec4d = f64, 4; Vec4i = i32, 4; Vec4l = i64, 4;
}

/// Serialize a vector as a fixed-length sequence of its components.
///
/// A hand-rolled impl because `serde`'s derive does not cover a const-generic `[T; S]`.
#[cfg(feature = "serde")]
impl<T: serde::Serialize, const S: usize> serde::Serialize for Vector<T, S> {
    fn serialize<Ser: serde::Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        use serde::ser::SerializeTuple;
        let mut tuple = serializer.serialize_tuple(S)?;
        for component in &self.v {
            tuple.serialize_element(component)?;
        }
        tuple.end()
    }
}

#[cfg(feature = "serde")]
impl<'de, T, const S: usize> serde::Deserialize<'de> for Vector<T, S>
where
    T: serde::Deserialize<'de> + Copy + Default,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<T, const S: usize>(core::marker::PhantomData<T>);

        impl<'de, T, const S: usize> serde::de::Visitor<'de> for Visitor<T, S>
        where
            T: serde::Deserialize<'de> + Copy + Default,
        {
            type Value = Vector<T, S>;

            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(f, "a sequence of {S} components")
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut v = [T::default(); S];
                for (i, slot) in v.iter_mut().enumerate() {
                    *slot = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(Vector { v })
            }
        }

        deserializer.deserialize_tuple(S, Visitor(core::marker::PhantomData))
    }
}
