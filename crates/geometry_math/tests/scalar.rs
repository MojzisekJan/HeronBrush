//! Tests for scalar behavior.

#![allow(clippy::excessive_precision)]

use geometry_math::assert_approx_eq;
use geometry_math::scalar::{self, Float};

/// The epsilon `checkSolution` uses for the polynomial solver tests.
const SOLUTION_EPSILON: f64 = 0.00000001;

fn check_solution(expected: &[f64], actual: &[f64]) {
    assert_eq!(actual.len(), expected.len(), "wrong number of solutions");
    for (a, e) in actual.iter().zip(expected) {
        assert_approx_eq!(*a, *e, SOLUTION_EPSILON);
    }
}

#[test]
fn is_nan() {
    assert!(f64::nan().is_nan());
    assert!(f32::nan().is_nan());
    assert!(!(1.0f64).is_nan());
    assert!(!(1.0f32).is_nan());
}

#[test]
fn is_inf() {
    assert!(f64::INFINITY.is_infinite());
    assert!((-f64::INFINITY).is_infinite());
    assert!(f32::INFINITY.is_infinite());
    assert!((-f32::INFINITY).is_infinite());
    assert!(!(0.0f64).is_infinite());
    assert!(!(0.0f32).is_infinite());
}

#[test]
fn min() {
    assert_eq!(scalar::min(1.0, 1.0), 1.0);
    assert_eq!(scalar::min(1.0, 2.0), 1.0);
    assert_eq!(scalar::min(2.0, 1.0), 1.0);
    assert_eq!(scalar::min(-1.0, 2.0), -1.0);
    assert_eq!(scalar::min(1.0, -2.0), -2.0);
    assert_eq!(scalar::min(-1.0, -2.0), -2.0);
    // Nested calls combine multiple inputs.
    assert_eq!(scalar::min(scalar::min(-1.0, -2.0), -3.0), -3.0);
    assert_eq!(scalar::min(scalar::min(-3.0, -1.0), -2.0), -3.0);
}

#[test]
fn max() {
    assert_eq!(scalar::max(1.0, 1.0), 1.0);
    assert_eq!(scalar::max(1.0, 2.0), 2.0);
    assert_eq!(scalar::max(2.0, 1.0), 2.0);
    assert_eq!(scalar::max(-1.0, 2.0), 2.0);
    assert_eq!(scalar::max(1.0, -2.0), 1.0);
    assert_eq!(scalar::max(-1.0, -2.0), -1.0);
    assert_eq!(scalar::max(scalar::max(-1.0, -2.0), -3.0), -1.0);
    assert_eq!(scalar::max(scalar::max(-3.0, -2.0), -1.0), -1.0);
}

#[test]
fn abs_min() {
    assert_eq!(scalar::abs_min(1.0, 1.0), 1.0);
    assert_eq!(scalar::abs_min(1.0, 2.0), 1.0);
    assert_eq!(scalar::abs_min(2.0, 1.0), 1.0);
    assert_eq!(scalar::abs_min(-1.0, 2.0), -1.0);
    assert_eq!(scalar::abs_min(1.0, -2.0), 1.0);
    assert_eq!(scalar::abs_min(-1.0, -2.0), -1.0);
    assert_eq!(scalar::abs_min(scalar::abs_min(1.0, -2.0), 3.0), 1.0);
}

#[test]
fn abs_max() {
    assert_eq!(scalar::abs_max(1.0, 1.0), 1.0);
    assert_eq!(scalar::abs_max(1.0, 2.0), 2.0);
    assert_eq!(scalar::abs_max(2.0, 1.0), 2.0);
    assert_eq!(scalar::abs_max(-1.0, 2.0), 2.0);
    assert_eq!(scalar::abs_max(1.0, -2.0), -2.0);
    assert_eq!(scalar::abs_max(-1.0, -2.0), -2.0);
    assert_eq!(scalar::abs_max(scalar::abs_max(-1.0, -2.0), -3.0), -3.0);
}

#[test]
fn safe_min() {
    assert_eq!(scalar::safe_min([Some(1.0), Some(1.0)]), Some(1.0));
    assert_eq!(scalar::safe_min([Some(1.0), Some(2.0)]), Some(1.0));
    assert_eq!(scalar::safe_min([Some(2.0), Some(1.0)]), Some(1.0));
    assert_eq!(scalar::safe_min([Some(-1.0), Some(2.0)]), Some(-1.0));
    assert_eq!(scalar::safe_min([Some(1.0), Some(-2.0)]), Some(-2.0));
    assert_eq!(scalar::safe_min([Some(-1.0), Some(-2.0)]), Some(-2.0));
    assert_eq!(
        scalar::safe_min([Some(-1.0), Some(-2.0), Some(-3.0)]),
        Some(-3.0)
    );

    assert_eq!(scalar::safe_min([Some(1.0), None]), Some(1.0));
    assert_eq!(scalar::safe_min([None, Some(-1.0)]), Some(-1.0));
    assert_eq!(scalar::safe_min([None::<f64>, None]), None);

    assert_eq!(scalar::safe_min([None, Some(1.0), Some(-2.0)]), Some(-2.0));
    assert_eq!(scalar::safe_min([Some(1.0), None, Some(-2.0)]), Some(-2.0));
    assert_eq!(scalar::safe_min([Some(1.0), Some(-2.0), None]), Some(-2.0));
    assert_eq!(scalar::safe_min([Some(1.0), None, None]), Some(1.0));
    assert_eq!(scalar::safe_min([None::<f64>, None, None]), None);
}

#[test]
fn safe_max() {
    assert_eq!(scalar::safe_max([1.0, 1.0]), 1.0);
    assert_eq!(scalar::safe_max([1.0, 2.0]), 2.0);
    assert_eq!(scalar::safe_max([2.0, 1.0]), 2.0);
    assert_eq!(scalar::safe_max([-1.0, 2.0]), 2.0);
    assert_eq!(scalar::safe_max([1.0, -2.0]), 1.0);
    assert_eq!(scalar::safe_max([-1.0, -2.0]), -1.0);

    assert_eq!(scalar::safe_max([1.0, f64::nan()]), 1.0);
    assert_eq!(scalar::safe_max([f64::nan(), -1.0]), -1.0);
    assert!(scalar::safe_max([f64::nan(), f64::nan()]).is_nan());

    assert_eq!(scalar::safe_max([f64::nan(), 1.0, -2.0]), 1.0);
    assert_eq!(scalar::safe_max([1.0, f64::nan(), -2.0]), 1.0);
    assert_eq!(scalar::safe_max([1.0, -2.0, f64::nan()]), 1.0);
    assert_eq!(scalar::safe_max([1.0, f64::nan(), f64::nan()]), 1.0);
    assert!(scalar::safe_max([f64::nan(), f64::nan(), f64::nan()]).is_nan());
}

#[test]
fn abs_difference() {
    assert_eq!(scalar::abs_difference(4, 7), 3);
    assert_eq!(scalar::abs_difference(7, 4), 3);
    assert_eq!(scalar::abs_difference(7, -1), 6);
    assert_eq!(scalar::abs_difference(-7, 1), 6);
    assert_eq!(scalar::abs_difference(-7, -1), 6);
}

#[test]
fn clamp() {
    assert_eq!(scalar::clamp(0.0, 0.0, 1.0), 0.0);
    assert_eq!(scalar::clamp(1.0, 0.0, 1.0), 1.0);
    assert_eq!(scalar::clamp(-1.0, 0.0, 1.0), 0.0);
    assert_eq!(scalar::clamp(2.0, 0.0, 1.0), 1.0);
    assert_eq!(scalar::clamp(0.5, 0.0, 1.0), 0.5);

    assert_eq!(scalar::clamp(0.0, -1.0, 0.0), 0.0);
    assert_eq!(scalar::clamp(-1.0, -1.0, 0.0), -1.0);
    assert_eq!(scalar::clamp(1.0, -1.0, 0.0), 0.0);
    assert_eq!(scalar::clamp(-2.0, -1.0, 0.0), -1.0);
    assert_eq!(scalar::clamp(-0.5, -1.0, 0.0), -0.5);

    assert_eq!(scalar::clamp(0.0, -1.0, 1.0), 0.0);
    assert_eq!(scalar::clamp(-1.0, -1.0, 1.0), -1.0);
    assert_eq!(scalar::clamp(1.0, -1.0, 1.0), 1.0);
    assert_eq!(scalar::clamp(-2.0, -1.0, 1.0), -1.0);
    assert_eq!(scalar::clamp(2.0, -1.0, 1.0), 1.0);
}

#[test]
fn sign() {
    assert_eq!(scalar::sign(-2), -1);
    assert_eq!(scalar::sign(-1), -1);
    assert_eq!(scalar::sign(0), 0);
    assert_eq!(scalar::sign(1), 1);
    assert_eq!(scalar::sign(2), 1);
}

#[test]
fn step() {
    assert_eq!(scalar::step(1, -1), 0);
    assert_eq!(scalar::step(1, 0), 0);
    assert_eq!(scalar::step(1, 1), 1);
    assert_eq!(scalar::step(1, 2), 1);
}

#[test]
fn smoothstep() {
    assert_approx_eq!(scalar::smoothstep(0.0, 1.0, -1.0), 0.0);
    assert_approx_eq!(scalar::smoothstep(0.0, 1.0, 0.0), 0.0);
    assert_approx_eq!(scalar::smoothstep(0.0, 1.0, 0.25), 0.15625);
    assert_approx_eq!(scalar::smoothstep(0.0, 1.0, 0.5), 0.5);
    assert_approx_eq!(scalar::smoothstep(0.0, 1.0, 0.75), 0.84375);
    assert_approx_eq!(scalar::smoothstep(0.0, 1.0, 1.0), 1.0);
    assert_approx_eq!(scalar::smoothstep(0.0, 1.0, 2.0), 1.0);
}

#[test]
fn mod_() {
    assert_approx_eq!(scalar::mod_(4.0, 2.0), 0.0);
    assert_approx_eq!(scalar::mod_(5.0, 2.0), 1.0);
    assert_approx_eq!(scalar::mod_(-5.0, 2.0), -1.0);
    assert_approx_eq!(scalar::mod_(5.0, -2.0), 1.0);
    assert_approx_eq!(scalar::mod_(-5.0, -2.0), -1.0);
    assert_approx_eq!(scalar::mod_(5.5, 2.0), 1.5);
}

#[test]
fn floor() {
    assert_approx_eq!(scalar::floor(-0.7), -1.0);
    assert_approx_eq!(scalar::floor(-0.5), -1.0);
    assert_approx_eq!(scalar::floor(-0.4), -1.0);
    assert_approx_eq!(scalar::floor(0.0), 0.0);
    assert_approx_eq!(scalar::floor(0.4), 0.0);
    assert_approx_eq!(scalar::floor(0.6), 0.0);
    assert_approx_eq!(scalar::floor(1.0), 1.0);
}

#[test]
fn ceil() {
    assert_approx_eq!(scalar::ceil(-1.1), -1.0);
    assert_approx_eq!(scalar::ceil(-0.7), 0.0);
    assert_approx_eq!(scalar::ceil(-0.5), 0.0);
    assert_approx_eq!(scalar::ceil(-0.4), 0.0);
    assert_approx_eq!(scalar::ceil(0.0), 0.0);
    assert_approx_eq!(scalar::ceil(0.4), 1.0);
    assert_approx_eq!(scalar::ceil(0.6), 1.0);
    assert_approx_eq!(scalar::ceil(1.0), 1.0);
    assert_approx_eq!(scalar::ceil(1.1), 2.0);
}

#[test]
fn trunc() {
    assert_approx_eq!(scalar::trunc(-1.1), -1.0);
    assert_approx_eq!(scalar::trunc(-0.7), 0.0);
    assert_approx_eq!(scalar::trunc(-0.5), 0.0);
    assert_approx_eq!(scalar::trunc(-0.4), 0.0);
    assert_approx_eq!(scalar::trunc(0.0), 0.0);
    assert_approx_eq!(scalar::trunc(0.4), 0.0);
    assert_approx_eq!(scalar::trunc(0.6), 0.0);
    assert_approx_eq!(scalar::trunc(1.0), 1.0);
    assert_approx_eq!(scalar::trunc(1.1), 1.0);

    // Values beyond the i64 range are returned unchanged.
    let huge = 1.05513e154;
    assert_eq!(scalar::trunc(huge), huge);
    assert_eq!(scalar::trunc(-huge), -huge);

    let int64_limit = 9223372036854775808.0;
    assert_eq!(scalar::trunc(int64_limit), int64_limit);
    assert_eq!(scalar::trunc(-int64_limit), -int64_limit);

    assert!(scalar::trunc(f64::INFINITY).is_infinite());
    assert!(scalar::trunc(-f64::INFINITY).is_infinite());
    assert!(scalar::trunc(f64::nan()).is_nan());
}

#[test]
fn mix() {
    assert_approx_eq!(scalar::mix(1.0, 2.0, 0.0), 1.0);
    assert_approx_eq!(scalar::mix(1.0, 2.0, 1.0), 2.0);
    assert_approx_eq!(scalar::mix(1.0, 2.0, 0.5), 1.5);

    assert_approx_eq!(scalar::mix(-1.0, 2.0, 0.0), -1.0);
    assert_approx_eq!(scalar::mix(-1.0, 2.0, 1.0), 2.0);
    assert_approx_eq!(scalar::mix(-1.0, 2.0, 0.5), 0.5);

    assert_approx_eq!(scalar::mix(-1.0, -2.0, 0.0), -1.0);
    assert_approx_eq!(scalar::mix(-1.0, -2.0, 1.0), -2.0);
    assert_approx_eq!(scalar::mix(-1.0, -2.0, 0.5), -1.5);
}

#[test]
fn fract() {
    assert_approx_eq!(scalar::fract(-1.2), -0.2);
    assert_approx_eq!(scalar::fract(-1.0), 0.0);
    assert_approx_eq!(scalar::fract(-0.7), -0.7);
    assert_approx_eq!(scalar::fract(0.0), 0.0);
    assert_approx_eq!(scalar::fract(0.7), 0.7);
    assert_approx_eq!(scalar::fract(1.0), 0.0);
    assert_approx_eq!(scalar::fract(1.2), 0.2);
}

#[test]
fn round() {
    assert_approx_eq!(scalar::round(-1.1), -1.0);
    assert_approx_eq!(scalar::round(-0.7), -1.0);
    assert_approx_eq!(scalar::round(-0.5), -1.0);
    assert_approx_eq!(scalar::round(-0.4), 0.0);
    assert_approx_eq!(scalar::round(0.0), 0.0);
    assert_approx_eq!(scalar::round(0.4), 0.0);
    assert_approx_eq!(scalar::round(0.6), 1.0);
    assert_approx_eq!(scalar::round(1.0), 1.0);
    assert_approx_eq!(scalar::round(1.1), 1.0);
}

#[test]
fn round_up() {
    assert_approx_eq!(scalar::round_up(-1.1), -2.0);
    assert_approx_eq!(scalar::round_up(-0.7), -1.0);
    assert_approx_eq!(scalar::round_up(-0.5), -1.0);
    assert_approx_eq!(scalar::round_up(-0.4), -1.0);
    assert_approx_eq!(scalar::round_up(0.0), 0.0);
    assert_approx_eq!(scalar::round_up(0.4), 1.0);
    assert_approx_eq!(scalar::round_up(0.6), 1.0);
    assert_approx_eq!(scalar::round_up(1.0), 1.0);
    assert_approx_eq!(scalar::round_up(1.1), 2.0);
}

#[test]
fn round_down() {
    assert_approx_eq!(scalar::round_down(-1.1), -1.0);
    assert_approx_eq!(scalar::round_down(-0.7), 0.0);
    assert_approx_eq!(scalar::round_down(-0.5), 0.0);
    assert_approx_eq!(scalar::round_down(-0.4), 0.0);
    assert_approx_eq!(scalar::round_down(0.0), 0.0);
    assert_approx_eq!(scalar::round_down(0.4), 0.0);
    assert_approx_eq!(scalar::round_down(0.6), 0.0);
    assert_approx_eq!(scalar::round_down(1.0), 1.0);
    assert_approx_eq!(scalar::round_down(1.1), 1.0);
}

#[test]
fn next_integer() {
    assert_approx_eq!(scalar::next_integer(-1.1), -2.0);
    assert_approx_eq!(scalar::next_integer(-1.0), -2.0);
    assert_approx_eq!(scalar::next_integer(-0.7), -1.0);
    assert_approx_eq!(scalar::next_integer(-0.5), -1.0);
    assert_approx_eq!(scalar::next_integer(-0.4), -1.0);
    assert_approx_eq!(scalar::next_integer(0.0), 0.0);
    assert_approx_eq!(scalar::next_integer(0.4), 1.0);
    assert_approx_eq!(scalar::next_integer(0.6), 1.0);
    assert_approx_eq!(scalar::next_integer(1.0), 2.0);
    assert_approx_eq!(scalar::next_integer(1.1), 2.0);
}

#[test]
fn prev_integer() {
    assert_approx_eq!(scalar::prev_integer(-1.1), -1.0);
    assert_approx_eq!(scalar::prev_integer(-1.0), 0.0);
    assert_approx_eq!(scalar::prev_integer(-0.7), 0.0);
    assert_approx_eq!(scalar::prev_integer(-0.5), 0.0);
    assert_approx_eq!(scalar::prev_integer(-0.4), 0.0);
    assert_approx_eq!(scalar::prev_integer(0.0), 0.0);
    assert_approx_eq!(scalar::prev_integer(0.4), 0.0);
    assert_approx_eq!(scalar::prev_integer(0.6), 0.0);
    assert_approx_eq!(scalar::prev_integer(1.0), 0.0);
    assert_approx_eq!(scalar::prev_integer(1.1), 1.0);
}

#[test]
fn snap() {
    assert_approx_eq!(scalar::snap(0.0, 1.0), 0.0);
    assert_approx_eq!(scalar::snap(0.4, 1.0), 0.0);
    assert_approx_eq!(scalar::snap(0.5, 1.0), 1.0);
    assert_approx_eq!(scalar::snap(0.6, 1.0), 1.0);
    assert_approx_eq!(scalar::snap(-0.4, 1.0), 0.0);
    assert_approx_eq!(scalar::snap(-0.5, 1.0), -1.0);
    assert_approx_eq!(scalar::snap(-0.6, 1.0), -1.0);

    assert_approx_eq!(scalar::snap(1.4, 1.0), 1.0);
    assert_approx_eq!(scalar::snap(1.5, 1.0), 2.0);
    assert_approx_eq!(scalar::snap(1.6, 1.0), 2.0);
    assert_approx_eq!(scalar::snap(-1.4, 1.0), -1.0);
    assert_approx_eq!(scalar::snap(-1.5, 1.0), -2.0);
    assert_approx_eq!(scalar::snap(-1.6, 1.0), -2.0);

    assert_approx_eq!(scalar::snap(0.0, 2.0), 0.0);
    assert_approx_eq!(scalar::snap(0.4, 2.0), 0.0);
    assert_approx_eq!(scalar::snap(0.5, 2.0), 0.0);
    assert_approx_eq!(scalar::snap(0.6, 2.0), 0.0);
    assert_approx_eq!(scalar::snap(-0.4, 2.0), 0.0);
    assert_approx_eq!(scalar::snap(-0.5, 2.0), 0.0);
    assert_approx_eq!(scalar::snap(-0.6, 2.0), 0.0);

    assert_approx_eq!(scalar::snap(1.4, 2.0), 2.0);
    assert_approx_eq!(scalar::snap(1.5, 2.0), 2.0);
    assert_approx_eq!(scalar::snap(1.6, 2.0), 2.0);
    assert_approx_eq!(scalar::snap(-1.4, 2.0), -2.0);
    assert_approx_eq!(scalar::snap(-1.5, 2.0), -2.0);
    assert_approx_eq!(scalar::snap(-1.6, 2.0), -2.0);
}

#[test]
fn snap_up() {
    assert_approx_eq!(scalar::snap_up(0.0, 1.0), 0.0);
    assert_approx_eq!(scalar::snap_up(0.4, 1.0), 1.0);
    assert_approx_eq!(scalar::snap_up(0.5, 1.0), 1.0);
    assert_approx_eq!(scalar::snap_up(0.6, 1.0), 1.0);
    assert_approx_eq!(scalar::snap_up(-0.4, 1.0), -1.0);
    assert_approx_eq!(scalar::snap_up(-0.5, 1.0), -1.0);
    assert_approx_eq!(scalar::snap_up(-0.6, 1.0), -1.0);

    assert_approx_eq!(scalar::snap_up(1.4, 1.0), 2.0);
    assert_approx_eq!(scalar::snap_up(1.5, 1.0), 2.0);
    assert_approx_eq!(scalar::snap_up(1.6, 1.0), 2.0);
    assert_approx_eq!(scalar::snap_up(-1.4, 1.0), -2.0);
    assert_approx_eq!(scalar::snap_up(-1.5, 1.0), -2.0);
    assert_approx_eq!(scalar::snap_up(-1.6, 1.0), -2.0);

    assert_approx_eq!(scalar::snap_up(0.0, 2.0), 0.0);
    assert_approx_eq!(scalar::snap_up(0.4, 2.0), 2.0);
    assert_approx_eq!(scalar::snap_up(0.5, 2.0), 2.0);
    assert_approx_eq!(scalar::snap_up(0.6, 2.0), 2.0);
    assert_approx_eq!(scalar::snap_up(-0.4, 2.0), -2.0);
    assert_approx_eq!(scalar::snap_up(-0.5, 2.0), -2.0);
    assert_approx_eq!(scalar::snap_up(-0.6, 2.0), -2.0);

    assert_approx_eq!(scalar::snap_up(1.4, 2.0), 2.0);
    assert_approx_eq!(scalar::snap_up(1.5, 2.0), 2.0);
    assert_approx_eq!(scalar::snap_up(1.6, 2.0), 2.0);
    assert_approx_eq!(scalar::snap_up(-1.4, 2.0), -2.0);
    assert_approx_eq!(scalar::snap_up(-1.5, 2.0), -2.0);
    assert_approx_eq!(scalar::snap_up(-1.6, 2.0), -2.0);
}

#[test]
fn snap_down() {
    assert_approx_eq!(scalar::snap_down(0.0, 1.0), 0.0);
    assert_approx_eq!(scalar::snap_down(0.4, 1.0), 0.0);
    assert_approx_eq!(scalar::snap_down(0.5, 1.0), 0.0);
    assert_approx_eq!(scalar::snap_down(0.6, 1.0), 0.0);
    assert_approx_eq!(scalar::snap_down(-0.4, 1.0), 0.0);
    assert_approx_eq!(scalar::snap_down(-0.5, 1.0), 0.0);
    assert_approx_eq!(scalar::snap_down(-0.6, 1.0), 0.0);

    assert_approx_eq!(scalar::snap_down(1.4, 1.0), 1.0);
    assert_approx_eq!(scalar::snap_down(1.5, 1.0), 1.0);
    assert_approx_eq!(scalar::snap_down(1.6, 1.0), 1.0);
    assert_approx_eq!(scalar::snap_down(-1.4, 1.0), -1.0);
    assert_approx_eq!(scalar::snap_down(-1.5, 1.0), -1.0);
    assert_approx_eq!(scalar::snap_down(-1.6, 1.0), -1.0);

    assert_approx_eq!(scalar::snap_down(0.0, 2.0), 0.0);
    assert_approx_eq!(scalar::snap_down(0.4, 2.0), 0.0);
    assert_approx_eq!(scalar::snap_down(1.6, 2.0), 0.0);
    assert_approx_eq!(scalar::snap_down(-1.6, 2.0), 0.0);
}

#[test]
fn correct() {
    assert_approx_eq!(scalar::correct(1.1, 0, f64::CORRECT_EPSILON), 1.1);

    assert_approx_eq!(scalar::correct(1.1, 0, 0.4), 1.0);
    assert_approx_eq!(scalar::correct(-1.1, 0, 0.4), -1.0);
    assert_approx_eq!(scalar::correct(1.3, 0, 0.4), 1.0);
    assert_approx_eq!(scalar::correct(1.4, 0, 0.3), 1.4);

    assert_approx_eq!(scalar::correct(1.1, 1, 0.4), 1.1);
    assert_approx_eq!(scalar::correct(-1.1, 1, 0.4), -1.1);
    assert_approx_eq!(scalar::correct(1.3, 1, 0.4), 1.3);
    assert_approx_eq!(scalar::correct(1.4, 1, 0.3), 1.4);
}

#[test]
fn is_equal() {
    assert!(scalar::is_equal(1.0, 1.0, 0.0));
    assert!(scalar::is_equal(-1.0, -1.0, 0.0));
    assert!(scalar::is_equal(-1.001, -1.001, 0.0));
    assert!(scalar::is_equal(1.0, 1.001, 0.1));
    assert!(scalar::is_equal(1.0, 1.0999, 0.1));

    assert!(!scalar::is_equal(1.0, 1.11, 0.1));
    assert!(!scalar::is_equal(1.0, 1.1, 0.09));
    assert!(!scalar::is_equal(-1.0, 1.11, 0.1));
    assert!(!scalar::is_equal(1.0, 1.1, 0.0));
}

#[test]
fn is_zero() {
    assert!(scalar::is_zero(0.0, 0.0));
    assert!(scalar::is_zero(0.0, 0.1));
    assert!(scalar::is_zero(0.099, 0.1));
    assert!(scalar::is_zero(-0.099, 0.1));
    assert!(!scalar::is_zero(0.099, 0.0));
    assert!(!scalar::is_zero(-1.0, 0.0));
}

#[test]
fn contains() {
    assert!(scalar::contains(0.0, 0.0, 1.0));
    assert!(scalar::contains(1.0, 0.0, 1.0));
    assert!(scalar::contains(0.0, 1.0, 0.0));
    assert!(scalar::contains(1.0, 1.0, 0.0));

    assert!(!scalar::contains(1.1, 0.0, 1.0));
    assert!(!scalar::contains(1.1, 1.0, 0.0));
    assert!(!scalar::contains(-0.1, 0.0, 1.0));
    assert!(!scalar::contains(-0.1, 1.0, 0.0));
}

#[test]
fn mean() {
    assert_eq!(scalar::mean([2.0]), Some(2.0));
    assert_eq!(scalar::mean([2.0, 4.0]), Some(3.0));
    assert_eq!(scalar::mean([1.0, 2.0, 3.0]), Some(2.0));
    assert_eq!(scalar::mean([2.0, 4.0, 6.0, 8.0]), Some(5.0));
    assert_eq!(scalar::mean(std::iter::empty::<f64>()), None);
}

#[test]
fn variance() {
    assert_eq!(scalar::variance([2.0]), Some(0.0));
    assert_eq!(scalar::variance([2.0, 4.0]), Some(2.0));
    assert_eq!(scalar::variance([2.0, 2.0, 2.0]), Some(0.0));
    assert_eq!(scalar::variance([2.0, 4.0, 6.0, 8.0]), Some(20.0));
}

#[test]
fn to_radians() {
    assert_eq!(scalar::to_radians(0.0), 0.0);
    assert_eq!(scalar::to_radians(90.0), f64::HALF_PI);
    assert_eq!(scalar::to_radians(180.0), f64::PI);
    assert_eq!(scalar::to_radians(360.0), f64::TWO_PI);
    assert_eq!(scalar::to_radians(-180.0), -f64::PI);
    assert_eq!(scalar::to_radians(-360.0), -f64::TWO_PI);
}

#[test]
fn to_degrees() {
    assert_eq!(scalar::to_degrees(0.0), 0.0);
    assert_eq!(scalar::to_degrees(f64::HALF_PI), 90.0);
    assert_eq!(scalar::to_degrees(f64::PI), 180.0);
    assert_eq!(scalar::to_degrees(f64::TWO_PI), 360.0);
    assert_eq!(scalar::to_degrees(-f64::PI), -180.0);
    assert_eq!(scalar::to_degrees(-f64::TWO_PI), -360.0);
}

#[test]
fn normalize_radians() {
    assert_eq!(scalar::normalize_radians(f64::TWO_PI), 0.0);
    assert_eq!(scalar::normalize_radians(f64::HALF_PI), f64::HALF_PI);
    assert_eq!(scalar::normalize_radians(-f64::HALF_PI), f64::THREE_HALF_PI);
    assert_eq!(
        scalar::normalize_radians(f64::HALF_PI + f64::TWO_PI),
        f64::HALF_PI
    );
}

#[test]
fn normalize_degrees() {
    assert_eq!(scalar::normalize_degrees(0.0), 0.0);
    assert_eq!(scalar::normalize_degrees(360.0), 0.0);
    assert_eq!(scalar::normalize_degrees(90.0), 90.0);
    assert_eq!(scalar::normalize_degrees(-90.0), 270.0);
    assert_eq!(scalar::normalize_degrees(360.0 + 90.0), 90.0);
}

#[test]
fn succ() {
    assert_eq!(scalar::succ(0, 1, 1), 0);
    assert_eq!(scalar::succ(0, 2, 1), 1);
    assert_eq!(scalar::succ(1, 2, 1), 0);
    assert_eq!(scalar::succ(0, 3, 2), 2);
    assert_eq!(scalar::succ(2, 3, 2), 1);
}

#[test]
fn pred() {
    assert_eq!(scalar::pred(0, 1, 1), 0);
    assert_eq!(scalar::pred(0, 2, 1), 1);
    assert_eq!(scalar::pred(1, 2, 1), 0);
    assert_eq!(scalar::pred(0, 3, 2), 1);
    assert_eq!(scalar::pred(2, 3, 2), 0);
}

#[test]
fn next_greater() {
    assert!(1.0 < f64::next_greater(1.0));
    assert!(-1.0 < f64::next_greater(-1.0));
}

#[test]
fn solve_quadratic() {
    let eps = f64::ALMOST_ZERO;

    check_solution(
        &[2.0, -8.0],
        scalar::solve_quadratic(1.0, 6.0, -16.0, eps).as_slice(),
    );
    check_solution(
        &[-1.0, -9.0],
        scalar::solve_quadratic(1.0, 10.0, 9.0, eps).as_slice(),
    );
    check_solution(
        &[7.0, -4.0],
        scalar::solve_quadratic(0.5, -1.5, -14.0, eps).as_slice(),
    );
    check_solution(
        &[2.0],
        scalar::solve_quadratic(1.0, -4.0, 4.0, eps).as_slice(),
    );
    check_solution(
        &[],
        scalar::solve_quadratic(1.0, 12.0, 37.0, eps).as_slice(),
    );
}

#[test]
fn solve_cubic() {
    let eps = f64::ALMOST_ZERO;

    check_solution(
        &[-2.0],
        scalar::solve_cubic(1.0, 0.0, -2.0, 4.0, eps).as_slice(),
    );
    check_solution(
        &[7.0 / 9.0],
        scalar::solve_cubic(9.0, -43.0, 145.0, -91.0, eps).as_slice(),
    );
    check_solution(
        &[4.464101615, 2.0, -2.464101615],
        scalar::solve_cubic(1.0, -4.0, -7.0, 22.0, eps).as_slice(),
    );

    // casus irreducibilis
    check_solution(
        &[-2.0, 1.0],
        scalar::solve_cubic(1.0, 0.0, -3.0, 2.0, eps).as_slice(),
    );
    check_solution(
        &[4.0 / 3.0, 1.0 / 3.0, -10.0 / 6.0],
        scalar::solve_cubic(1.0, 0.0, -7.0 / 3.0, 20.0 / 27.0, eps).as_slice(),
    );
}

#[test]
fn solve_quartic() {
    let eps = f64::ALMOST_ZERO;

    check_solution(
        &[],
        scalar::solve_quartic(1.0, 1.0, 1.0, 1.0, 1.0, eps).as_slice(),
    );
    check_solution(
        &[],
        scalar::solve_quartic(1.0, -1.0, 1.0, -1.0, 1.0, eps).as_slice(),
    );
    check_solution(
        &[
            -0.203258341626567109,
            -4.91984728399109344,
            2.76090563295441601,
            0.362199992663244539,
        ],
        scalar::solve_quartic(1.0, 2.0, -14.0, 2.0, 1.0, eps).as_slice(),
    );
    check_solution(
        &[1.5986745079, -1.0],
        scalar::solve_quartic(1.0, 3.0, 0.0, -8.0, -6.0, eps).as_slice(),
    );
    check_solution(
        &[-1.0, -1.0],
        scalar::solve_quartic(1.0, 4.0, 6.0, 4.0, 1.0, eps).as_slice(),
    );
    check_solution(
        &[-3.0, 2.0],
        scalar::solve_quartic(1.0, 2.0, -11.0, -12.0, 36.0, eps).as_slice(),
    );
    check_solution(
        &[
            -1.0 - 6.0f64.sqrt(),
            -1.0 - 11.0f64.sqrt(),
            11.0f64.sqrt() - 1.0,
            6.0f64.sqrt() - 1.0,
        ],
        scalar::solve_quartic(1.0, 4.0, -11.0, -30.0, 50.0, eps).as_slice(),
    );
}
