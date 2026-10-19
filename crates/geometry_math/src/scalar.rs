//! Scalar numeric traits, constants and free functions.
//!
//! [`Num`] defines common numeric operations and [`Float`] adds floating-point operations and
//! associated tolerance constants.

use std::fmt::Debug;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, Sub, SubAssign};

/// A signed numeric type: the component type of a [`crate::vec::Vector`].
///
/// Implemented for `f32`, `f64`, `i32` and `i64`.
pub trait Num:
    Copy
    + Debug
    + Default
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Rem<Output = Self>
    + Neg<Output = Self>
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
{
    /// The additive identity.
    const ZERO: Self;
    /// The multiplicative identity.
    const ONE: Self;
    /// `ONE + ONE`, used often enough to be worth naming.
    const TWO: Self;
    /// The smallest representable value (`std::numeric_limits<T>::lowest()`).
    const LOWEST: Self;
    /// The largest representable value.
    const HIGHEST: Self;

    /// Converts an `i32` into this type, the equivalent of `static_cast<T>(v)`.
    fn from_i32(v: i32) -> Self;

    /// Converts this value to `f64`, for interoperating with floating point code.
    fn to_f64(self) -> f64;

    /// Converts an `f64` into this type, the equivalent of `static_cast<T>(v)`.
    fn from_f64(v: f64) -> Self;
}

/// A floating point type, carrying the epsilon values and transcendental functions the
/// geometry code needs.
///
/// Associated constants define numeric and geometric tolerances.
pub trait Float: Num {
    /// An epsilon value to use for comparisons.
    const ALMOST_ZERO: Self;
    /// An epsilon for determining the position of a point relative to a plane.
    ///
    /// This is the value tyrbsp uses.
    const POINT_STATUS_EPSILON: Self;
    /// An epsilon for correcting a value that is very close to an integral value.
    ///
    /// This is the value QBSP uses.
    const CORRECT_EPSILON: Self;
    /// An epsilon to use when testing colinearity.
    ///
    /// This value hits a sweet spot in relation to [`Float::POINT_STATUS_EPSILON`].
    const COLINEAR_EPSILON: Self;
    /// An epsilon to use when comparing angles in radians.
    ///
    /// If the absolute sine of the angle between two vectors is less than this, they are
    /// considered parallel or opposite.
    const ANGLE_EPSILON: Self;

    /// The ratio of a circle's circumference to its diameter.
    const PI: Self;
    /// `2 * PI`.
    const TWO_PI: Self;
    /// `PI / 2`.
    const HALF_PI: Self;
    /// `PI / 4`.
    const QUARTER_PI: Self;
    /// `3 * PI / 2`.
    const THREE_HALF_PI: Self;
    /// Euler's constant.
    const E: Self;

    /// Positive infinity.
    const INFINITY: Self;
    /// Negative infinity.
    const NEG_INFINITY: Self;
    /// The difference between 1.0 and the next larger representable value.
    const MACHINE_EPSILON: Self;

    /// Returns a NaN value.
    fn nan() -> Self;
    /// Returns whether this value is NaN.
    fn is_nan(self) -> bool;
    /// Returns whether this value is positive or negative infinity.
    fn is_infinite(self) -> bool;

    /// Returns the square root of this value.
    fn sqrt(self) -> Self;
    /// Returns the sine of this value, which is interpreted as an angle in radians.
    fn sin(self) -> Self;
    /// Returns the cosine of this value, which is interpreted as an angle in radians.
    fn cos(self) -> Self;
    /// Returns the tangent of this value, which is interpreted as an angle in radians.
    fn tan(self) -> Self;
    /// Returns the arcsine of this value in radians.
    fn asin(self) -> Self;
    /// Returns the arccosine of this value in radians.
    fn acos(self) -> Self;
    /// Returns the four quadrant arctangent of `self / rhs` in radians.
    fn atan2(self, rhs: Self) -> Self;
    /// Returns `self` raised to the power of `n`.
    fn powi(self, n: i32) -> Self;
    /// Returns the cube root of this value.
    fn cbrt(self) -> Self;

    /// Returns the smallest floating point value strictly greater than this one, or
    /// infinity if no such value exists.
    fn next_greater(self) -> Self;
}

macro_rules! impl_num_int {
    ($($t:ty),*) => {$(
        impl Num for $t {
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const TWO: Self = 2;
            const LOWEST: Self = <$t>::MIN;
            const HIGHEST: Self = <$t>::MAX;

            #[inline]
            fn from_i32(v: i32) -> Self { v as $t }
            #[inline]
            fn to_f64(self) -> f64 { self as f64 }
            #[inline]
            fn from_f64(v: f64) -> Self { v as $t }
        }
    )*};
}

macro_rules! impl_num_float {
    ($($t:ty => $consts:path),*) => {$(
        impl Num for $t {
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            const TWO: Self = 2.0;
            // The lowest finite value is the most negative representable value.
            const LOWEST: Self = <$t>::MIN;
            const HIGHEST: Self = <$t>::MAX;

            #[inline]
            fn from_i32(v: i32) -> Self { v as $t }
            #[inline]
            fn to_f64(self) -> f64 { self as f64 }
            #[inline]
            fn from_f64(v: f64) -> Self { v as $t }
        }

        impl Float for $t {
            const ALMOST_ZERO: Self = 0.001;
            const POINT_STATUS_EPSILON: Self = 0.0001;
            const CORRECT_EPSILON: Self = 0.001;
            const COLINEAR_EPSILON: Self = 0.00001;
            const ANGLE_EPSILON: Self = 0.00000001;

            const PI: Self = { use $consts as c; c::PI };
            const TWO_PI: Self = 2.0 * <$t as Float>::PI;
            const HALF_PI: Self = <$t as Float>::PI / 2.0;
            const QUARTER_PI: Self = <$t as Float>::PI / 4.0;
            const THREE_HALF_PI: Self = 3.0 * <$t as Float>::PI / 2.0;
            const E: Self = { use $consts as c; c::E };

            const INFINITY: Self = <$t>::INFINITY;
            const NEG_INFINITY: Self = <$t>::NEG_INFINITY;
            const MACHINE_EPSILON: Self = <$t>::EPSILON;

            #[inline]
            fn nan() -> Self { <$t>::NAN }
            #[inline]
            fn is_nan(self) -> bool { <$t>::is_nan(self) }
            #[inline]
            fn is_infinite(self) -> bool { <$t>::is_infinite(self) }

            #[inline]
            fn sqrt(self) -> Self { <$t>::sqrt(self) }
            #[inline]
            fn sin(self) -> Self { <$t>::sin(self) }
            #[inline]
            fn cos(self) -> Self { <$t>::cos(self) }
            #[inline]
            fn tan(self) -> Self { <$t>::tan(self) }
            #[inline]
            fn asin(self) -> Self { <$t>::asin(self) }
            #[inline]
            fn acos(self) -> Self { <$t>::acos(self) }
            #[inline]
            fn atan2(self, rhs: Self) -> Self { <$t>::atan2(self, rhs) }
            #[inline]
            fn powi(self, n: i32) -> Self { <$t>::powi(self, n) }
            #[inline]
            fn cbrt(self) -> Self { <$t>::cbrt(self) }

            #[inline]
            fn next_greater(self) -> Self {
                // `nextafter(value, infinity)`
                <$t>::next_up(self)
            }
        }
    )*};
}

impl_num_int!(i32, i64);
impl_num_float!(f32 => core::f32::consts, f64 => core::f64::consts);

/* ========== selection ========== */

/// Returns the smaller of the two values.
#[inline]
pub fn min<T: Num>(lhs: T, rhs: T) -> T {
    if lhs < rhs {
        lhs
    } else {
        rhs
    }
}

/// Returns the larger of the two values.
#[inline]
pub fn max<T: Num>(lhs: T, rhs: T) -> T {
    if lhs > rhs {
        lhs
    } else {
        rhs
    }
}

/// Returns the absolute value of the given value.
#[inline]
pub fn abs<T: Num>(v: T) -> T {
    if v < T::ZERO {
        -v
    } else {
        v
    }
}

/// Returns the value with the smaller absolute value.
///
/// Note that this returns the value itself, not its absolute value.
#[inline]
pub fn abs_min<T: Num>(lhs: T, rhs: T) -> T {
    if abs(lhs) < abs(rhs) {
        lhs
    } else {
        rhs
    }
}

/// Returns the value with the larger absolute value.
///
/// Note that this returns the value itself, not its absolute value.
#[inline]
pub fn abs_max<T: Num>(lhs: T, rhs: T) -> T {
    if abs(lhs) > abs(rhs) {
        lhs
    } else {
        rhs
    }
}

/// Returns the minimum of the given values, ignoring any that are `None`.
///
/// Returns `None` only when every input is `None`.
#[inline]
pub fn safe_min<T: Num, I: IntoIterator<Item = Option<T>>>(values: I) -> Option<T> {
    values
        .into_iter()
        .flatten()
        .fold(None, |acc: Option<T>, v| Some(match acc {
            Some(a) if a < v => a,
            _ => v,
        }))
}

/// Returns the maximum of the given values, ignoring any that are NaN.
///
/// Returns NaN for an empty input or when every value is NaN.
#[inline]
pub fn safe_max<T: Float, I: IntoIterator<Item = T>>(values: I) -> T {
    values
        .into_iter()
        .filter(|v| !v.is_nan())
        .fold(T::nan(), |acc, v| if acc.is_nan() || v > acc { v } else { acc })
}

/// Returns the absolute difference of the given values.
///
/// For signed values, computes the difference of magnitudes, `abs(abs(lhs) - abs(rhs))`, rather
/// than `abs(lhs - rhs)`.
#[inline]
pub fn abs_difference<T: Num>(lhs: T, rhs: T) -> T {
    abs(abs(lhs) - abs(rhs))
}

/// Clamps the given value to the interval `[min_val, max_val]`.
#[inline]
pub fn clamp<T: Num>(v: T, min_val: T, max_val: T) -> T {
    max(min(v, max_val), min_val)
}

/// Returns `-1`, `0` or `+1` depending on the sign of the given value.
#[inline]
pub fn sign<T: Num>(v: T) -> T {
    if v < T::ZERO {
        -T::ONE
    } else if v > T::ZERO {
        T::ONE
    } else {
        T::ZERO
    }
}

/// Returns `0` if `v` is less than the edge value `e`, and `1` otherwise.
#[inline]
pub fn step<T: Num>(e: T, v: T) -> T {
    if v < e {
        T::ZERO
    } else {
        T::ONE
    }
}

/// Performs smooth Hermite interpolation between 0 and 1 when `e0 < v < e1`.
#[inline]
pub fn smoothstep<T: Float>(e0: T, e1: T, v: T) -> T {
    let t = clamp((v - e0) / (e1 - e0), T::ZERO, T::ONE);
    t * t * (T::from_i32(3) - T::TWO * t)
}

/* ========== rounding ========== */

/// Returns the nearest integer value not greater in magnitude than the given value, i.e.
/// the given value rounded towards 0.
#[inline]
pub fn trunc<T: Float>(v: T) -> T {
    if v.is_nan() || v.is_infinite() {
        return v;
    }

    let min_v = T::from_f64(i64::MIN as f64);
    let max_exclusive_v = -min_v;
    if v < min_v || v >= max_exclusive_v {
        return v;
    }

    // Within this range Rust's `f64::trunc` matches `static_cast<int64_t>`.
    T::from_f64(v.to_f64().trunc())
}

/// Returns the floating point remainder of `x / y`.
#[inline]
pub fn mod_<T: Float>(x: T, y: T) -> T {
    x - y * trunc(x / y)
}

/// Computes the largest integer value not greater than the given value.
#[inline]
pub fn floor<T: Float>(v: T) -> T {
    let r = trunc(v);
    if v >= T::ZERO {
        r
    } else if r == v {
        v
    } else {
        r - T::ONE
    }
}

/// Computes the smallest integer value not less than the given value.
#[inline]
pub fn ceil<T: Float>(v: T) -> T {
    let r = trunc(v);
    if v < T::ZERO || r == v {
        r
    } else {
        r + T::ONE
    }
}

/// Linearly interpolates between `x` and `y` using the weight `a`, returning
/// `(1 - a) * x + a * y`.
#[inline]
pub fn mix<T: Num>(x: T, y: T, a: T) -> T {
    (T::ONE - a) * x + a * y
}

/// Returns the fractional part of the given value.
#[inline]
pub fn fract<T: Float>(v: T) -> T {
    if v > T::ZERO {
        v - floor(v)
    } else {
        v - ceil(v)
    }
}

/// Rounds the given value to the nearest integer value, halfway cases away from zero.
#[inline]
pub fn round<T: Float>(v: T) -> T {
    let half = T::ONE / T::TWO;
    if v > T::ZERO {
        floor(v + half)
    } else {
        ceil(v - half)
    }
}

/// Rounds the given value away from 0.
#[inline]
pub fn round_up<T: Float>(v: T) -> T {
    if v < T::ZERO {
        floor(v)
    } else {
        ceil(v)
    }
}

/// Rounds the given value towards 0.
///
/// This is equivalent to [`trunc`]; it exists for symmetry with [`round_up`].
#[inline]
pub fn round_down<T: Float>(v: T) -> T {
    if v > T::ZERO {
        floor(v)
    } else {
        ceil(v)
    }
}

/// Returns the nearest integer strictly farther from 0 than the given value, or 0 if the
/// given value is 0.
#[inline]
pub fn next_integer<T: Float>(x: T) -> T {
    if x > T::ZERO {
        floor(x) + T::ONE
    } else if x < T::ZERO {
        ceil(x) - T::ONE
    } else {
        T::ZERO
    }
}

/// Returns the nearest integer strictly closer to 0 than the given value, or 0 if the
/// given value is 0.
#[inline]
pub fn prev_integer<T: Float>(x: T) -> T {
    if x > T::ZERO {
        ceil(x) - T::ONE
    } else if x < T::ZERO {
        floor(x) + T::ONE
    } else {
        T::ZERO
    }
}

/// Rounds the given value to the nearest multiple of the given grid size.
///
/// # Panics
///
/// Panics if `grid` is zero.
#[inline]
pub fn snap<T: Float>(v: T, grid: T) -> T {
    assert!(grid != T::ZERO, "grid size must not be 0");
    grid * round(v / grid)
}

/// Rounds the given value away from 0 to the nearest multiple of the given grid size.
///
/// # Panics
///
/// Panics if `grid` is not positive.
#[inline]
pub fn snap_up<T: Float>(v: T, grid: T) -> T {
    assert!(grid > T::ZERO, "grid size must be positive");
    grid * round_up(v / grid)
}

/// Rounds the given value towards 0 to the nearest multiple of the given grid size.
///
/// # Panics
///
/// Panics if `grid` is not positive.
#[inline]
pub fn snap_down<T: Float>(v: T, grid: T) -> T {
    assert!(grid > T::ZERO, "grid size must be positive");
    grid * round_down(v / grid)
}

/// Rounds the given value to the nearest integer if its distance to that integer is less
/// than `epsilon`, retaining at most `decimals` decimal places.
#[inline]
pub fn correct<T: Float>(v: T, decimals: u32, epsilon: T) -> T {
    let m = T::from_i32(1i32 << decimals);
    let r = round(v * m);
    if abs(v - r) < epsilon {
        r / m
    } else {
        v
    }
}

/* ========== comparison ========== */

/// Returns whether the distance between the given values is at most `epsilon`.
#[inline]
pub fn is_equal<T: Num>(lhs: T, rhs: T, epsilon: T) -> bool {
    abs(lhs - rhs) <= epsilon
}

/// Returns whether the given value is within `epsilon` of 0.
#[inline]
pub fn is_zero<T: Num>(v: T, epsilon: T) -> bool {
    abs(v) <= epsilon
}

/// Returns whether `v` lies in the closed interval `[s, e]`.
#[inline]
pub fn contains<T: Num>(v: T, s: T, e: T) -> bool {
    if s < e {
        v >= s && v <= e
    } else {
        v >= e && v <= s
    }
}

/* ========== statistics ========== */

/// Returns the arithmetic mean of the given values, or `None` if there are none.
#[inline]
pub fn mean<T: Num, I: IntoIterator<Item = T>>(values: I) -> Option<T> {
    let mut sum = T::ZERO;
    let mut count = T::ZERO;
    for v in values {
        sum += v;
        count += T::ONE;
    }
    if count == T::ZERO {
        None
    } else {
        Some(sum / count)
    }
}

/// Returns the sum of the squared deviations of the given values from their mean, or
/// `None` if there are none.
///
/// Computes the total sum of squared deviations without dividing by the sample count.
pub fn variance<T: Num, I: IntoIterator<Item = T>>(values: I) -> Option<T>
where
    I::IntoIter: Clone,
{
    let iter = values.into_iter();
    let m = mean(iter.clone())?;
    Some(iter.fold(T::ZERO, |acc, v| acc + (v - m) * (v - m)))
}

/* ========== angles ========== */

/// Converts the given angle from degrees to radians.
#[inline]
pub fn to_radians<T: Float>(d: T) -> T {
    d * T::PI / T::from_i32(180)
}

/// Converts the given angle from radians to degrees.
#[inline]
pub fn to_degrees<T: Float>(r: T) -> T {
    r * T::from_i32(180) / T::PI
}

/// Normalizes the given angle in radians to the interval `[0, 2*PI)`.
#[inline]
pub fn normalize_radians<T: Float>(mut angle: T) -> T {
    while angle < T::ZERO {
        angle += T::TWO_PI;
    }
    mod_(angle, T::TWO_PI)
}

/// Normalizes the given angle in degrees to the interval `[0, 360)`.
#[inline]
pub fn normalize_degrees<T: Float>(mut angle: T) -> T {
    let full = T::from_i32(360);
    while angle < T::ZERO {
        angle += full;
    }
    mod_(angle, full)
}

/* ========== index arithmetic ========== */

/// Returns `(index + stride) % count`.
#[inline]
pub fn succ(index: usize, count: usize, stride: usize) -> usize {
    (index + stride) % count
}

/// Returns `(index + count - stride % count) % count`.
#[inline]
pub fn pred(index: usize, count: usize, stride: usize) -> usize {
    ((index + count) - (stride % count)) % count
}

/* ========== polynomial solvers ========== */

/// The real roots of a polynomial, at most `N` of them.
///
/// Fixed-size, allocation-free root storage. [`Roots::as_slice`] exposes only valid entries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Roots<T, const N: usize> {
    count: usize,
    values: [T; N],
}

impl<T: Num, const N: usize> Roots<T, N> {
    /// Creates an empty set of roots.
    #[inline]
    pub fn none() -> Self {
        Self {
            count: 0,
            values: [T::ZERO; N],
        }
    }

    /// Returns the roots that were actually found.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.values[..self.count]
    }

    /// Returns the number of roots that were found.
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns whether no roots were found.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Solves `a*x^2 + b*x + c = 0` and returns its real roots.
///
/// Algorithm reference: <https://github.com/erich666/GraphicsGems/blob/master/gems/Roots3And4.c>.
pub fn solve_quadratic<T: Float>(a: T, b: T, c: T, epsilon: T) -> Roots<T, 2> {
    // normal form: x^2 + px + q = 0, where p is actually p/2
    let p = b / (T::TWO * a);
    let q = c / a;
    let d = p * p - q;

    if is_zero(d, epsilon) {
        Roots {
            count: 1,
            values: [-p, T::ZERO],
        }
    } else if d < T::ZERO {
        Roots::none()
    } else {
        let d2 = d.sqrt();
        Roots {
            count: 2,
            values: [d2 - p, -d2 - p],
        }
    }
}

/// Solves `a*x^3 + b*x^2 + c*x + d = 0` and returns its real roots.
pub fn solve_cubic<T: Float>(a: T, b: T, c: T, d: T, epsilon: T) -> Roots<T, 3> {
    let third = T::ONE / T::from_i32(3);
    let half = T::ONE / T::TWO;

    // normal form: x^3 + A*x^2 + B*x + C = 0
    let ca = b / a;
    let cb = c / a;
    let cc = d / a;

    // substitute x = y - A/3 to eliminate the quadratic term: x^3 + px + q = 0
    let p = third * (-third * ca * ca + cb);
    let q = half * (T::TWO / T::from_i32(27) * ca * ca * ca - third * ca * cb + cc);

    // Cardano's formula
    let p3 = p * p * p;
    let disc = q * q + p3;

    let count;
    let mut values = [T::ZERO; 3];

    if is_zero(disc, epsilon) {
        if is_zero(q, epsilon) {
            // one triple solution
            count = 1;
            values[0] = T::ZERO;
        } else {
            // one single and one double solution
            let u = (-q).cbrt();
            count = 2;
            values[0] = u * T::TWO;
            values[1] = -u;
        }
    } else if disc < T::ZERO {
        // casus irreducibilis: three real solutions
        let phi = third * (-q / (-p3).sqrt()).acos();
        let t = T::TWO * (-p).sqrt();
        count = 3;
        values[0] = t * phi.cos();
        values[1] = -t * (phi + T::PI / T::from_i32(3)).cos();
        values[2] = -t * (phi - T::PI / T::from_i32(3)).cos();
    } else {
        // one real solution
        let d2 = disc.sqrt();
        let u = (d2 - q).cbrt();
        let v = -(d2 + q).cbrt();
        count = 1;
        values[0] = u + v;
    }

    // resubstitute
    let sub = third * ca;
    for value in values.iter_mut().take(count) {
        *value -= sub;
    }

    Roots { count, values }
}

/// Solves `a*x^4 + b*x^3 + c*x^2 + d*x + e = 0` and returns its real roots.
pub fn solve_quartic<T: Float>(a: T, b: T, c: T, d: T, e: T, epsilon: T) -> Roots<T, 4> {
    let half = T::ONE / T::TWO;
    let quarter = T::ONE / T::from_i32(4);

    // normal form: x^4 + A*x^3 + B*x^2 + C*x + D = 0
    let ca = b / a;
    let cb = c / a;
    let cc = d / a;
    let cd = e / a;

    // substitute x = y - A/4 to eliminate the cubic term: x^4 + px^2 + qx + r = 0
    let p = -T::from_f64(3.0 / 8.0) * ca * ca + cb;
    let q = T::from_f64(1.0 / 8.0) * ca * ca * ca - half * ca * cb + cc;
    let r = -T::from_f64(3.0 / 256.0) * ca * ca * ca * ca
        + T::from_f64(1.0 / 16.0) * ca * ca * cb
        - quarter * ca * cc
        + cd;

    let mut count;
    let mut values = [T::ZERO; 4];

    if is_zero(r, epsilon) {
        // no absolute term: y * (y^3 + py + q) = 0
        let cubic = solve_cubic(T::ONE, T::ZERO, p, q, epsilon);
        count = cubic.len();
        values[..count].copy_from_slice(cubic.as_slice());
        values[count] = T::ZERO;
        count += 1;
    } else {
        // solve the resolvent cubic ...
        let resolvent = solve_cubic(
            T::ONE,
            -half * p,
            -r,
            half * r * p - T::from_f64(1.0 / 8.0) * q * q,
            epsilon,
        );
        debug_assert!(!resolvent.is_empty());
        let z = resolvent.as_slice()[0];

        // ... and use its one real solution z to build two quadratic equations
        let mut u = z * z - r;
        let mut v = T::TWO * z - p;

        if is_zero(u, epsilon) {
            u = T::ZERO;
        } else if u > T::ZERO {
            u = u.sqrt();
        } else {
            return Roots::none();
        }

        if is_zero(v, epsilon) {
            v = T::ZERO;
        } else if v > T::ZERO {
            v = v.sqrt();
        } else {
            return Roots::none();
        }

        let first = solve_quadratic(T::ONE, if q < T::ZERO { -v } else { v }, z - u, epsilon);
        let n1 = first.len();
        values[..n1].copy_from_slice(first.as_slice());

        let second = solve_quadratic(T::ONE, if q < T::ZERO { v } else { -v }, z + u, epsilon);
        let n2 = second.len();
        values[n1..n1 + n2].copy_from_slice(second.as_slice());

        count = n1 + n2;
    }

    // resubstitute
    let sub = quarter * ca;
    for value in values.iter_mut().take(count) {
        *value -= sub;
    }

    Roots { count, values }
}
