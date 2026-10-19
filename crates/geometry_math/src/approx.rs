//! Epsilon-tolerant comparison for the math types.
//!
//! [`ApproxEq`] and `assert_approx_eq!` compare values with a default epsilon of
//! [`crate::scalar::Float::ALMOST_ZERO`].

use crate::mat::{self, Matrix};
use crate::scalar::{self, Float};
use crate::vec::{self, Vector};

/// Comparison up to an epsilon.
pub trait ApproxEq {
    /// The epsilon type used to compare values of this type.
    type Epsilon: Copy;

    /// The epsilon used by [`ApproxEq::approx_eq`].
    const DEFAULT_EPSILON: Self::Epsilon;

    /// Returns whether the two values are equal up to the given epsilon.
    fn approx_eq_eps(&self, other: &Self, epsilon: Self::Epsilon) -> bool;

    /// Returns whether the two values are equal up to [`ApproxEq::DEFAULT_EPSILON`].
    #[inline]
    fn approx_eq(&self, other: &Self) -> bool {
        self.approx_eq_eps(other, Self::DEFAULT_EPSILON)
    }
}

macro_rules! impl_approx_scalar {
    ($($t:ty),*) => {$(
        impl ApproxEq for $t {
            type Epsilon = $t;

            const DEFAULT_EPSILON: $t = <$t as Float>::ALMOST_ZERO;

            #[inline]
            fn approx_eq_eps(&self, other: &Self, epsilon: $t) -> bool {
                scalar::is_equal(*self, *other, epsilon)
            }
        }
    )*};
}

impl_approx_scalar!(f32, f64);

impl<T: Float, const S: usize> ApproxEq for Vector<T, S>
where
    T: ApproxEq<Epsilon = T>,
{
    type Epsilon = T;

    const DEFAULT_EPSILON: T = T::ALMOST_ZERO;

    #[inline]
    fn approx_eq_eps(&self, other: &Self, epsilon: T) -> bool {
        vec::is_equal(self, other, epsilon)
    }
}

impl<T: Float, const R: usize, const C: usize> ApproxEq for Matrix<T, R, C>
where
    T: ApproxEq<Epsilon = T>,
{
    type Epsilon = T;

    const DEFAULT_EPSILON: T = T::ALMOST_ZERO;

    #[inline]
    fn approx_eq_eps(&self, other: &Self, epsilon: T) -> bool {
        mat::is_equal(self, other, epsilon)
    }
}

impl<A: ApproxEq> ApproxEq for Option<A> {
    type Epsilon = A::Epsilon;

    const DEFAULT_EPSILON: A::Epsilon = A::DEFAULT_EPSILON;

    #[inline]
    fn approx_eq_eps(&self, other: &Self, epsilon: A::Epsilon) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.approx_eq_eps(b, epsilon),
            (None, None) => true,
            _ => false,
        }
    }
}

/// Asserts that two values are equal up to an epsilon.
///
/// With two arguments the epsilon is [`ApproxEq::DEFAULT_EPSILON`]; a third argument
/// overrides it.
#[macro_export]
macro_rules! assert_approx_eq {
    ($lhs:expr, $rhs:expr $(,)?) => {{
        let lhs = $lhs;
        let rhs = $rhs;
        assert!(
            $crate::approx::ApproxEq::approx_eq(&lhs, &rhs),
            "expected {:?} ~= {:?}",
            lhs,
            rhs
        );
    }};
    ($lhs:expr, $rhs:expr, $epsilon:expr $(,)?) => {{
        let lhs = $lhs;
        let rhs = $rhs;
        let epsilon = $epsilon;
        assert!(
            $crate::approx::ApproxEq::approx_eq_eps(&lhs, &rhs, epsilon),
            "expected {:?} ~= {:?} (epsilon {:?})",
            lhs,
            rhs,
            epsilon
        );
    }};
}
