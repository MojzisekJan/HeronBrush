//! Tests for vec behavior.

#![allow(clippy::excessive_precision)]

use std::cmp::Ordering;

use geometry_math::assert_approx_eq;
use geometry_math::scalar::{self, Float, Num};
use geometry_math::vec::{self, Vec2d, Vec2f, Vec3d, Vec3f, Vec4d, Vec4f, Vector};

/// Rotates `(1, 0, 0)` by the given number of degrees counter-clockwise about the
/// positive Z axis.
///
/// Computes the rotation directly from the axis and angle.
fn rotate_pos_x_by_degrees(degrees: f32) -> Vec3f {
    let r = scalar::to_radians(degrees);
    Vec3f::new(r.cos(), r.sin(), 0.0)
}

#[test]
fn default_constructor() {
    assert_eq!(Vec3f::default(), Vec3f::new(0.0, 0.0, 0.0));
}

#[test]
fn fill() {
    assert_eq!(Vec3f::fill(2.0), Vec3f::new(2.0, 2.0, 2.0));
    assert_eq!(Vec3f::fill(0.0), Vec3f::new(0.0, 0.0, 0.0));
    assert_eq!(Vec3f::fill(-2.0), Vec3f::new(-2.0, -2.0, -2.0));
}

#[test]
fn axis() {
    assert_eq!(Vec3f::axis(0), Vec3f::new(1.0, 0.0, 0.0));
    assert_eq!(Vec3f::axis(1), Vec3f::new(0.0, 1.0, 0.0));
    assert_eq!(Vec3f::axis(2), Vec3f::new(0.0, 0.0, 1.0));
}

#[test]
fn operator_subscript() {
    let v = Vec4f::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(v[0], 1.0);
    assert_eq!(v[1], 2.0);
    assert_eq!(v[2], 3.0);
    assert_eq!(v[3], 4.0);
}

#[test]
fn accessors() {
    let v = Vec4f::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(v.x(), v[0]);
    assert_eq!(v.y(), v[1]);
    assert_eq!(v.z(), v[2]);
    assert_eq!(v.w(), v[3]);
    assert_eq!(v.xy(), Vec2f::new(1.0, 2.0));
    assert_eq!(v.xyz(), Vec3f::new(1.0, 2.0, 3.0));
    assert_eq!(v.xyzw(), v);
}

#[test]
fn static_members() {
    assert_eq!(Vec3f::zero(), Vec3f::new(0.0, 0.0, 0.0));
    assert_eq!(Vec3f::one(), Vec3f::new(1.0, 1.0, 1.0));

    let nan = Vec3f::nan();
    let min = Vec3f::min_value();
    let max = Vec3f::max_value();
    for i in 0..3 {
        // The minimum vector uses the lowest finite component value.
        assert_eq!(min[i], f32::LOWEST);
        assert_eq!(max[i], f32::HIGHEST);
        assert!(nan[i].is_nan());
    }
}

#[test]
fn compare() {
    let cmp = |a: Vec3f, b: Vec3f| vec::compare(&a, &b, 0.0);
    let v = Vec3f::new;

    assert_eq!(cmp(v(0.0, 0.0, 0.0), v(0.0, 0.0, 0.0)), Ordering::Equal);
    assert_eq!(cmp(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0)), Ordering::Less);
    assert_eq!(cmp(v(1.0, 1.0, 1.0), v(2.0, 1.0, 1.0)), Ordering::Less);
    assert_eq!(cmp(v(1.0, 1.0, 1.0), v(1.0, 2.0, 1.0)), Ordering::Less);
    assert_eq!(cmp(v(1.0, 1.0, 1.0), v(1.0, 1.0, 2.0)), Ordering::Less);
    assert_eq!(cmp(v(1.0, 1.0, 1.0), v(2.0, 0.0, 0.0)), Ordering::Less);
    assert_eq!(cmp(v(1.0, 1.0, 1.0), v(1.0, 2.0, 0.0)), Ordering::Less);

    assert_eq!(cmp(v(1.0, 1.0, 1.0), v(0.0, 0.0, 0.0)), Ordering::Greater);
    assert_eq!(cmp(v(2.0, 1.0, 1.0), v(1.0, 1.0, 1.0)), Ordering::Greater);
    assert_eq!(cmp(v(1.0, 2.0, 1.0), v(1.0, 1.0, 1.0)), Ordering::Greater);
    assert_eq!(cmp(v(1.0, 1.0, 2.0), v(1.0, 1.0, 1.0)), Ordering::Greater);
    assert_eq!(cmp(v(2.0, 0.0, 0.0), v(1.0, 1.0, 1.0)), Ordering::Greater);
    assert_eq!(cmp(v(1.0, 2.0, 0.0), v(1.0, 1.0, 1.0)), Ordering::Greater);

    assert_ne!(cmp(v(1.0, 2.0, 0.0), Vec3f::nan()), Ordering::Equal);
    assert_ne!(cmp(Vec3f::nan(), v(1.0, 2.0, 0.0)), Ordering::Equal);
    // Inconsistent with `==` on two NaN floats, but consistent with IEEE 754-2008
    // totalOrder, and it is what makes `Ord` sound here.
    assert_eq!(cmp(Vec3f::nan(), Vec3f::nan()), Ordering::Equal);
}

#[test]
fn compare_ranges() {
    let v = Vec3f::new;
    let r1 = [v(1.0, 2.0, 3.0), v(1.0, 2.0, 3.0)];
    let r2 = [v(1.0, 2.0, 3.0), v(2.0, 2.0, 3.0)];
    let r3 = [v(1.0, 2.0, 3.0)];

    assert_eq!(vec::compare_all(&r1, &r1, 0.0), Ordering::Equal);
    assert_eq!(vec::compare_all(&r1, &r2, 0.0), Ordering::Less);
    assert_eq!(vec::compare_all(&r2, &r1, 0.0), Ordering::Greater);
    // A prefix is less than the longer sequence it prefixes.
    assert_eq!(vec::compare_all(&r3, &r1, 0.0), Ordering::Less);
    assert_eq!(vec::compare_all(&r1, &r3, 0.0), Ordering::Greater);
}

#[test]
fn is_equal() {
    let v = Vec3f::new;
    assert!(vec::is_equal(&v(0.0, 0.0, 0.0), &v(0.0, 0.0, 0.0), 0.0));
    assert!(vec::is_equal(&v(0.0, 0.0, 0.0), &v(0.1, 0.1, 0.1), 0.11));
    assert!(!vec::is_equal(&v(0.0, 0.0, 0.0), &v(0.1, 0.1, 0.1), 0.09));
}

#[test]
fn comparison_operators() {
    let v = Vec3f::new;
    assert!(v(1.0, 1.0, 1.0) == v(1.0, 1.0, 1.0));
    assert!(v(1.0, 1.0, 1.0) != v(1.0, 1.0, 2.0));
    assert!(v(1.0, 1.0, 1.0) < v(1.0, 1.0, 2.0));
    assert!(v(1.0, 1.0, 1.0) <= v(1.0, 1.0, 1.0));
    assert!(v(1.0, 1.0, 2.0) > v(1.0, 1.0, 1.0));
    assert!(v(1.0, 1.0, 1.0) >= v(1.0, 1.0, 1.0));
}

#[test]
fn slice() {
    let v = Vec4d::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(v.slice::<2>(0), Vec2d::new(1.0, 2.0));
    assert_eq!(v.slice::<2>(1), Vec2d::new(2.0, 3.0));
    assert_eq!(v.slice::<2>(2), Vec2d::new(3.0, 4.0));
}

#[test]
fn find_max_component() {
    let v = Vec3f::new;
    assert_eq!(v(1.0, 0.0, 0.0).find_max_component(0), 0);
    assert_ne!(v(-1.0, 0.0, 0.0).find_max_component(0), 0);
    assert_eq!(v(0.0, 1.0, 0.0).find_max_component(0), 1);
    assert_ne!(v(0.0, -1.0, 0.0).find_max_component(0), 1);
    assert_eq!(v(0.0, 0.0, 1.0).find_max_component(0), 2);
    assert_ne!(v(0.0, 0.0, -1.0).find_max_component(0), 2);

    assert_eq!(v(3.0, 1.0, -2.0).find_max_component(0), 0);
    assert_eq!(v(3.0, 1.0, -2.0).find_max_component(1), 1);
    assert_eq!(v(3.0, 1.0, -2.0).find_max_component(2), 2);
    assert_eq!(vec::normalize(v(1.0, 2.0, -3.0)).find_max_component(0), 1);
}

#[test]
fn find_abs_max_component() {
    let v = Vec3f::new;
    assert_eq!(v(1.0, 0.0, 0.0).find_abs_max_component(0), 0);
    assert_eq!(v(-1.0, 0.0, 0.0).find_abs_max_component(0), 0);
    assert_eq!(v(0.0, 1.0, 0.0).find_abs_max_component(0), 1);
    assert_eq!(v(0.0, -1.0, 0.0).find_abs_max_component(0), 1);
    assert_eq!(v(0.0, 0.0, 1.0).find_abs_max_component(0), 2);
    assert_eq!(v(0.0, 0.0, -1.0).find_abs_max_component(0), 2);

    assert_eq!(v(3.0, 1.0, -2.0).find_abs_max_component(0), 0);
    assert_eq!(v(3.0, 1.0, -2.0).find_abs_max_component(1), 2);
    assert_eq!(v(3.0, 1.0, -2.0).find_abs_max_component(2), 1);
    assert_eq!(
        vec::normalize(v(1.0, 2.0, -3.0)).find_abs_max_component(0),
        2
    );
}

#[test]
fn get_abs_max_component_axis() {
    let v = Vec3f::new;
    assert_eq!(v(1.0, 0.0, 0.0).get_abs_max_component_axis(0), v(1.0, 0.0, 0.0));
    assert_eq!(v(-1.0, 0.0, 0.0).get_abs_max_component_axis(0), v(-1.0, 0.0, 0.0));
    assert_eq!(v(0.0, 1.0, 0.0).get_abs_max_component_axis(0), v(0.0, 1.0, 0.0));
    assert_eq!(v(0.0, -1.0, 0.0).get_abs_max_component_axis(0), v(0.0, -1.0, 0.0));
    assert_eq!(v(0.0, 0.0, 1.0).get_abs_max_component_axis(0), v(0.0, 0.0, 1.0));
    assert_eq!(v(0.0, 0.0, -1.0).get_abs_max_component_axis(0), v(0.0, 0.0, -1.0));

    assert_eq!(v(3.0, -1.0, 2.0).get_abs_max_component_axis(0), v(1.0, 0.0, 0.0));
    assert_eq!(v(3.0, -1.0, 2.0).get_abs_max_component_axis(1), v(0.0, 0.0, 1.0));
    assert_eq!(v(3.0, -1.0, 2.0).get_abs_max_component_axis(2), v(0.0, -1.0, 0.0));
}

#[test]
fn get_max_component() {
    let v = Vec3f::new;
    assert_eq!(v(1.0, 0.0, 0.0).get_max_component(0), 1.0);
    assert_eq!(v(-1.0, 0.0, 0.0).get_max_component(0), 0.0);
    assert_eq!(v(0.0, 1.0, 0.0).get_max_component(0), 1.0);
    assert_eq!(v(0.0, -1.0, 0.0).get_max_component(0), 0.0);
    assert_eq!(v(0.0, 0.0, 1.0).get_max_component(0), 1.0);
    assert_eq!(v(0.0, 0.0, -1.0).get_max_component(0), 0.0);

    assert_eq!(v(3.0, 1.0, -2.0).get_max_component(0), 3.0);
    assert_eq!(v(3.0, 1.0, -2.0).get_max_component(1), 1.0);
    assert_eq!(v(3.0, 1.0, -2.0).get_max_component(2), -2.0);
}

#[test]
fn get_abs_max_component() {
    let v = Vec3f::new;
    assert_eq!(v(1.0, 0.0, 0.0).get_abs_max_component(0), 1.0);
    assert_eq!(v(-1.0, 0.0, 0.0).get_abs_max_component(0), -1.0);
    assert_eq!(v(0.0, 1.0, 0.0).get_abs_max_component(0), 1.0);
    assert_eq!(v(0.0, -1.0, 0.0).get_abs_max_component(0), -1.0);
    assert_eq!(v(0.0, 0.0, 1.0).get_abs_max_component(0), 1.0);
    assert_eq!(v(0.0, 0.0, -1.0).get_abs_max_component(0), -1.0);

    assert_eq!(v(3.0, 1.0, -2.0).get_abs_max_component(0), 3.0);
    assert_eq!(v(3.0, 1.0, -2.0).get_abs_max_component(1), -2.0);
    assert_eq!(v(3.0, 1.0, -2.0).get_abs_max_component(2), 1.0);
}

#[test]
fn arithmetic_operators() {
    let v = Vec3f::new;
    assert_eq!(-v(1.0, -2.0, 3.0), v(-1.0, 2.0, -3.0));
    assert_eq!(v(1.0, 2.0, 3.0) + v(3.0, 2.0, 1.0), v(4.0, 4.0, 4.0));
    assert_eq!(v(2.0, 3.0, 1.0) - v(1.0, 2.0, 2.0), v(1.0, 1.0, -1.0));
    assert_eq!(v(2.0, 3.0, -1.0) * v(1.0, 2.0, 2.0), v(2.0, 6.0, -2.0));
    assert_eq!(v(2.0, 3.0, 1.0) * 3.0, v(6.0, 9.0, 3.0));
    assert_eq!(3.0f32 * v(2.0, 3.0, 1.0), v(6.0, 9.0, 3.0));
    assert_eq!(v(2.0, 12.0, 2.0) / v(1.0, 2.0, -1.0), v(2.0, 6.0, -2.0));
    assert_eq!(v(2.0, 36.0, 4.0) / 2.0, v(1.0, 18.0, 2.0));
    assert_eq!(8.0f32 / v(2.0, 8.0, -4.0), v(4.0, 1.0, -2.0));
}

#[test]
fn min() {
    let v = Vec3f::new;
    assert_eq!(vec::min(v(2.0, 2.0, 2.0), v(3.0, 3.0, 3.0)), v(2.0, 2.0, 2.0));
    assert_eq!(vec::min(v(-2.0, -2.0, -2.0), v(-1.0, -1.0, -1.0)), v(-2.0, -2.0, -2.0));
    assert_eq!(vec::min(v(2.0, 2.0, 2.0), v(1.0, 3.0, 1.0)), v(1.0, 2.0, 1.0));
    assert_eq!(vec::min(v(-2.0, -2.0, -2.0), v(-1.0, -3.0, -1.0)), v(-2.0, -3.0, -2.0));
    assert_eq!(
        vec::min(vec::min(v(-2.0, -2.0, -2.0), v(-1.0, -3.0, -1.0)), v(-1.0, -3.0, -4.0)),
        v(-2.0, -3.0, -4.0)
    );
}

#[test]
fn max() {
    let v = Vec3f::new;
    assert_eq!(vec::max(v(2.0, 2.0, 2.0), v(3.0, 3.0, 3.0)), v(3.0, 3.0, 3.0));
    assert_eq!(vec::max(v(-2.0, -2.0, -2.0), v(-1.0, -1.0, -1.0)), v(-1.0, -1.0, -1.0));
    assert_eq!(vec::max(v(2.0, 2.0, 2.0), v(1.0, 3.0, 1.0)), v(2.0, 3.0, 2.0));
    assert_eq!(vec::max(v(-2.0, -2.0, -2.0), v(-1.0, -3.0, -1.0)), v(-1.0, -2.0, -1.0));
    assert_eq!(
        vec::max(vec::max(v(-2.0, -2.0, -2.0), v(-1.0, -3.0, -1.0)), v(4.0, -4.0, 1.0)),
        v(4.0, -2.0, 1.0)
    );
}

#[test]
fn abs_min() {
    let v = Vec3f::new;
    assert_eq!(vec::abs_min(v(2.0, 2.0, 2.0), v(3.0, 3.0, 3.0)), v(2.0, 2.0, 2.0));
    assert_eq!(vec::abs_min(v(-2.0, -2.0, -2.0), v(-1.0, -1.0, -1.0)), v(-1.0, -1.0, -1.0));
    assert_eq!(vec::abs_min(v(2.0, 2.0, 2.0), v(1.0, 3.0, 1.0)), v(1.0, 2.0, 1.0));
    assert_eq!(vec::abs_min(v(-2.0, -2.0, -2.0), v(-1.0, -3.0, -1.0)), v(-1.0, -2.0, -1.0));
    assert_eq!(
        vec::abs_min(
            vec::abs_min(v(-2.0, -2.0, -2.0), v(-1.0, -3.0, -1.0)),
            v(0.0, 1.0, -4.0)
        ),
        v(0.0, 1.0, -1.0)
    );
}

#[test]
fn abs_max() {
    let v = Vec3f::new;
    assert_eq!(vec::abs_max(v(2.0, 2.0, 2.0), v(3.0, 3.0, 3.0)), v(3.0, 3.0, 3.0));
    assert_eq!(vec::abs_max(v(-2.0, -2.0, -2.0), v(-1.0, -1.0, -1.0)), v(-2.0, -2.0, -2.0));
    assert_eq!(vec::abs_max(v(2.0, 2.0, 2.0), v(1.0, 3.0, 1.0)), v(2.0, 3.0, 2.0));
    assert_eq!(vec::abs_max(v(-2.0, -2.0, -2.0), v(-1.0, -3.0, -1.0)), v(-2.0, -3.0, -2.0));
    assert_eq!(
        vec::abs_max(
            vec::abs_max(v(-2.0, -2.0, -2.0), v(-1.0, -3.0, -1.0)),
            v(4.0, -1.0, 0.0)
        ),
        v(4.0, -3.0, -2.0)
    );
}

#[test]
fn abs() {
    let v = Vec3f::new;
    assert_eq!(vec::abs(v(1.0, -2.0, -3.0)), v(1.0, 2.0, 3.0));
    assert_eq!(vec::abs(v(0.0, -2.0, -3.0)), v(0.0, 2.0, 3.0));
}

#[test]
fn sign() {
    let v = Vec3d::new;
    assert_eq!(vec::sign(v(1.0, 1.0, 1.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::sign(v(0.0, 0.0, 0.0)), v(0.0, 0.0, 0.0));
    assert_eq!(vec::sign(-v(1.0, 1.0, 1.0)), v(-1.0, -1.0, -1.0));
}

#[test]
fn step() {
    let v = Vec3d::new;
    let one = v(1.0, 1.0, 1.0);
    assert_eq!(vec::step(one, v(0.0, 0.0, 0.0)), v(0.0, 0.0, 0.0));
    assert_eq!(vec::step(one, v(1.0, 1.0, 1.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::step(one, v(-1.0, 0.0, 1.0)), v(0.0, 0.0, 1.0));
    assert_eq!(vec::step(-one, v(-1.0, 0.0, 1.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::step(-one, v(-2.0, 0.0, 1.0)), v(0.0, 1.0, 1.0));
}

#[test]
fn smoothstep() {
    let v = Vec3d::new;
    let e0 = v(0.0, 0.0, 0.0);
    let e1 = v(1.0, 1.0, 1.0);
    assert_eq!(vec::smoothstep(e0, e1, v(-1.0, -1.0, -1.0)), v(0.0, 0.0, 0.0));
    assert_eq!(vec::smoothstep(e0, e1, v(0.0, 0.0, 0.0)), v(0.0, 0.0, 0.0));
    assert_eq!(vec::smoothstep(e0, e1, v(1.0, 1.0, 1.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::smoothstep(e0, e1, v(2.0, 2.0, 2.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::smoothstep(e0, e1, v(-1.0, 0.0, 2.0)), v(0.0, 0.0, 1.0));
    assert_eq!(vec::smoothstep(e0, e1, v(0.0, 0.5, 1.0)), v(0.0, 0.5, 1.0));
    assert_eq!(
        vec::smoothstep(e0, e1, v(0.25, 0.5, 0.75)),
        v(0.15625, 0.5, 0.84375)
    );
}

#[test]
fn dot() {
    assert_approx_eq!(
        vec::dot(Vec3f::new(2.3, 8.7878, -2323.0), Vec3f::new(4.333, -2.0, 322.0)),
        -748013.6097f32,
        1.0
    );
    assert_approx_eq!(
        vec::dot(Vec3f::new(2.3, 8.7878, -2323.0), Vec3f::zero()),
        0.0f32
    );

    assert_eq!(vec::dot(Vec2f::new(1.0, 0.0), Vec2f::new(1.0, 0.0)), 1.0);
    assert_eq!(vec::dot(Vec2f::new(0.0, 1.0), Vec2f::new(1.0, 0.0)), 0.0);
    assert_eq!(vec::dot(Vec2f::new(-1.0, 0.0), Vec2f::new(1.0, 0.0)), -1.0);
    assert_eq!(vec::dot(Vec2f::new(0.0, -1.0), Vec2f::new(1.0, 0.0)), 0.0);
}

#[test]
fn cross() {
    let v = Vec3f::new;
    assert_eq!(vec::cross(Vec3f::zero(), Vec3f::zero()), Vec3f::zero());
    assert_eq!(
        vec::cross(Vec3f::zero(), v(2.0, 34.233, -10003.0002)),
        Vec3f::zero()
    );
    assert_eq!(vec::cross(v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0)), v(0.0, 0.0, 1.0));
    assert_approx_eq!(
        vec::cross(v(12.302, -0.0017, 79898.3), v(2.0, 34.233, -10003.0002)),
        v(-2735141.499, 282853.508, 421.138),
        1.0
    );

    // Normalizing before or after the cross product gives the same direction.
    let t1 = v(7.0, 4.0, 0.0);
    let t2 = v(-2.0, 22.0, 0.0);
    assert_approx_eq!(
        vec::normalize(vec::cross(vec::normalize(t1), vec::normalize(t2))),
        vec::normalize(vec::cross(t1, t2))
    );
}

#[test]
fn squared_length() {
    assert_approx_eq!(vec::squared_length(Vec3f::zero()), 0.0f32);
    assert_approx_eq!(vec::squared_length(Vec3f::new(1.0, 0.0, 0.0)), 1.0f32);
    assert_approx_eq!(
        vec::squared_length(Vec3f::new(2.3, 8.7878, -2323.0)),
        5396411.51542884f32,
        1.0
    );
}

#[test]
fn length() {
    assert_approx_eq!(vec::length(Vec3f::zero()), 0.0f32);
    assert_approx_eq!(vec::length(Vec3f::new(1.0, 0.0, 0.0)), 1.0f32);
    assert_approx_eq!(
        vec::length(Vec3f::new(2.3, 8.7878, -2323.0)),
        5396411.51542884f32.sqrt()
    );
}

#[test]
fn normalize() {
    assert_eq!(
        vec::normalize(Vec3f::new(1.0, 0.0, 0.0)),
        Vec3f::new(1.0, 0.0, 0.0)
    );
    assert_eq!(
        vec::normalize(Vec3f::new(-1.0, 0.0, 0.0)),
        Vec3f::new(-1.0, 0.0, 0.0)
    );

    let v1 = Vec3f::new(2.3, 8.7878, -2323.0);
    let v2 = Vec3f::new(4.333, -2.0, 322.0);
    assert_approx_eq!(vec::normalize(v1), v1 / vec::length(v1));
    assert_approx_eq!(vec::normalize(v2), v2 / vec::length(v2));
}

#[test]
fn swizzle() {
    let v = Vec3d::new;
    assert_eq!(vec::swizzle(v(1.0, 2.0, 3.0), 0), v(2.0, 3.0, 1.0));
    assert_eq!(vec::swizzle(v(1.0, 2.0, 3.0), 1), v(3.0, 1.0, 2.0));
    assert_eq!(vec::swizzle(v(1.0, 2.0, 3.0), 2), v(1.0, 2.0, 3.0));
}

#[test]
fn unswizzle() {
    let v = Vec3d::new(1.0, 2.0, 3.0);
    for axis in 0..3 {
        assert_eq!(vec::unswizzle(vec::swizzle(v, axis), axis), v);
    }
}

#[test]
fn is_unit() {
    let eps = f32::ALMOST_ZERO;
    let v = Vec3f::new;
    assert!(vec::is_unit(v(1.0, 0.0, 0.0), eps));
    assert!(vec::is_unit(v(0.0, 1.0, 0.0), eps));
    assert!(vec::is_unit(v(0.0, 0.0, 1.0), eps));
    assert!(vec::is_unit(v(-1.0, 0.0, 0.0), eps));
    assert!(vec::is_unit(v(0.0, -1.0, 0.0), eps));
    assert!(vec::is_unit(v(0.0, 0.0, -1.0), eps));
    assert!(vec::is_unit(vec::normalize(v(1.0, 1.0, 1.0)), eps));
    assert!(!vec::is_unit(v(1.0, 1.0, 1.0), eps));
    assert!(!vec::is_unit(Vec3f::zero(), eps));
}

#[test]
fn is_zero() {
    assert!(vec::is_zero(Vec3f::zero(), f32::ALMOST_ZERO));
    assert!(!vec::is_zero(Vec3f::new(1.0, 0.0, 0.0), f32::ALMOST_ZERO));
}

#[test]
fn is_nan() {
    assert!(vec::is_nan(Vec3f::nan()));
    assert!(!vec::is_nan(Vec3f::new(1.0, 0.0, 0.0)));
}

#[test]
fn is_integral() {
    let v = Vec3f::new;
    assert!(vec::is_integral(v(1.0, 0.0, 0.0), 0.0));
    assert!(vec::is_integral(v(0.0, 1.0, 0.0), 0.0));
    assert!(vec::is_integral(v(0.0, 0.0, 1.0), 0.0));
    assert!(vec::is_integral(v(-1.0, 0.0, 0.0), 0.0));
    assert!(vec::is_integral(v(0.0, -1.0, 0.0), 0.0));
    assert!(vec::is_integral(v(0.0, 0.0, -1.0), 0.0));
    assert!(vec::is_integral(v(1.0, 1.0, 1.0), 0.0));
    assert!(vec::is_integral(Vec3f::zero(), 0.0));
    assert!(!vec::is_integral(vec::normalize(v(1.0, 1.0, 1.0)), 0.0));
}

#[test]
fn mix() {
    let zero = Vec3d::zero();
    let one = Vec3d::one();
    assert_eq!(vec::mix(zero, one, zero), zero);
    assert_eq!(vec::mix(zero, one, one), one);
    assert_eq!(vec::mix(zero, one, one / 2.0), one / 2.0);
}

#[test]
fn clamp() {
    let v = Vec3d::new;
    assert_eq!(
        vec::clamp(v(1.0, 1.0, 1.0), v(0.0, 0.0, 0.0), v(2.0, 2.0, 2.0)),
        v(1.0, 1.0, 1.0)
    );
    assert_eq!(
        vec::clamp(v(1.0, 1.0, 1.0), v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0)),
        v(1.0, 1.0, 1.0)
    );
    assert_eq!(
        vec::clamp(v(0.0, 0.0, 0.0), v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0)),
        v(0.0, 0.0, 0.0)
    );
    assert_eq!(
        vec::clamp(v(2.0, 0.0, -1.0), v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0)),
        v(1.0, 0.0, 0.0)
    );
    assert_eq!(
        vec::clamp(v(2.0, 0.0, -1.0), v(1.0, 0.0, -2.0), v(3.0, 1.0, 1.0)),
        v(2.0, 0.0, -1.0)
    );
}

#[test]
fn fract() {
    let v = Vec3d::new;
    assert_approx_eq!(vec::fract(v(0.0, 0.0, 0.0)), v(0.0, 0.0, 0.0));
    assert_approx_eq!(vec::fract(v(0.1, 0.7, 0.99999)), v(0.1, 0.7, 0.99999));
    assert_approx_eq!(vec::fract(v(-0.1, 0.7, -0.99999)), v(-0.1, 0.7, -0.99999));
    assert_approx_eq!(vec::fract(v(-1.3, 0.7, 1.99999)), v(-0.3, 0.7, 0.99999));
}

#[test]
fn mod_() {
    let v = Vec3d::new;
    assert_approx_eq!(vec::mod_(v(1.0, 1.0, 1.0), v(1.0, 1.0, 1.0)), v(0.0, 0.0, 0.0));
    assert_approx_eq!(vec::mod_(v(2.0, -1.0, 0.0), v(1.0, 1.0, 1.0)), v(0.0, 0.0, 0.0));
    assert_approx_eq!(
        vec::mod_(v(6.5, -6.5, 6.5), v(2.0, 2.0, -2.0)),
        v(0.5, -0.5, 0.5)
    );
}

#[test]
fn squared_distance() {
    let v1 = Vec3f::new(2.3, 8.7878, -2323.0);
    let v2 = Vec3f::new(4.333, -2.0, 322.0);

    assert_approx_eq!(vec::squared_distance(v1, v1), 0.0f32);
    assert_approx_eq!(
        vec::squared_distance(v1, Vec3f::zero()),
        vec::squared_length(v1)
    );
    assert_approx_eq!(
        vec::squared_distance(v1, v2),
        vec::squared_length(v1 - v2)
    );
}

#[test]
fn distance() {
    let v1 = Vec3f::new(2.3, 8.7878, -2323.0);
    let v2 = Vec3f::new(4.333, -2.0, 322.0);

    assert_approx_eq!(vec::distance(v1, v1), 0.0f32);
    assert_approx_eq!(vec::distance(v1, Vec3f::zero()), vec::length(v1));
    assert_approx_eq!(vec::distance(v1, v2), vec::length(v1 - v2));
}

#[test]
fn to_homogeneous_coords() {
    assert_eq!(
        Vec3f::new(1.0, 2.0, 3.0).to_homogeneous_coords(),
        Vec4f::new(1.0, 2.0, 3.0, 1.0)
    );
}

#[test]
fn to_cartesian_coords() {
    assert_eq!(
        Vec4f::new(2.0, 4.0, 8.0, 2.0).to_cartesian_coords(),
        Vec3f::new(1.0, 2.0, 4.0)
    );
}

#[test]
fn is_colinear() {
    let v = Vec3d::new;
    let eps = f64::COLINEAR_EPSILON;
    assert!(vec::is_colinear(Vec3d::zero(), Vec3d::zero(), Vec3d::zero(), eps));
    assert!(vec::is_colinear(Vec3d::one(), Vec3d::one(), Vec3d::one(), eps));
    assert!(vec::is_colinear(
        v(0.0, 0.0, 0.0),
        v(0.0, 0.0, 1.0),
        v(0.0, 0.0, 2.0),
        eps
    ));
    assert!(!vec::is_colinear(
        v(0.0, 0.0, 0.0),
        v(1.0, 0.0, 0.0),
        v(0.0, 1.0, 0.0),
        eps
    ));
    assert!(!vec::is_colinear(
        v(0.0, 0.0, 0.0),
        v(10.0, 0.0, 0.0),
        v(0.0, 1.0, 0.0),
        eps
    ));
}

#[test]
fn is_parallel() {
    let v = Vec3f::new;
    let eps = f32::COLINEAR_EPSILON;
    assert!(vec::is_parallel(v(1.0, 0.0, 0.0), v(1.0, 0.0, 0.0), eps));
    assert!(vec::is_parallel(v(1.0, 0.0, 0.0), v(-1.0, 0.0, 0.0), eps));
    assert!(vec::is_parallel(v(1.0, 1.0, 1.0), v(1.0, 1.0, 1.0), eps));
    assert!(vec::is_parallel(
        v(1.0, 1.0, 1.0),
        vec::normalize(v(1.0, 1.0, 1.0)),
        eps
    ));
}

#[test]
fn floor() {
    let v = Vec3f::new;
    assert_eq!(vec::floor(v(1.0, 0.0, 0.0)), v(1.0, 0.0, 0.0));
    assert_eq!(vec::floor(v(1.0, 1.0, 1.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::floor(Vec3f::zero()), Vec3f::zero());
    assert_eq!(vec::floor(vec::normalize(v(1.0, 1.0, 1.0))), Vec3f::zero());
    assert_eq!(vec::floor(v(0.4, 0.4, 0.4)), Vec3f::zero());
    assert_eq!(vec::floor(v(0.4, 0.5, 0.4)), Vec3f::zero());
    assert_eq!(vec::floor(v(-0.4, -0.5, -0.4)), v(-1.0, -1.0, -1.0));
}

#[test]
fn ceil() {
    let v = Vec3f::new;
    assert_eq!(vec::ceil(v(1.0, 0.0, 0.0)), v(1.0, 0.0, 0.0));
    assert_eq!(vec::ceil(v(1.0, 1.0, 1.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::ceil(Vec3f::zero()), Vec3f::zero());
    assert_eq!(vec::ceil(vec::normalize(v(1.0, 1.0, 1.0))), Vec3f::one());
    assert_eq!(vec::ceil(v(0.4, 0.4, 0.4)), Vec3f::one());
    assert_eq!(vec::ceil(v(0.4, 0.5, 0.4)), Vec3f::one());
    assert_eq!(vec::ceil(v(-0.4, -0.5, -0.4)), Vec3f::zero());
    assert_eq!(vec::ceil(v(-1.4, -1.5, -1.4)), v(-1.0, -1.0, -1.0));
}

#[test]
fn trunc() {
    let v = Vec3f::new;
    assert_eq!(vec::trunc(v(1.0, 0.0, 0.0)), v(1.0, 0.0, 0.0));
    assert_eq!(vec::trunc(v(1.0, 1.0, 1.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::trunc(Vec3f::zero()), Vec3f::zero());
    assert_eq!(vec::trunc(vec::normalize(v(1.0, 1.0, 1.0))), Vec3f::zero());
    assert_eq!(vec::trunc(vec::normalize(-v(1.0, 1.0, 1.0))), Vec3f::zero());
    assert_eq!(vec::trunc(v(0.4, 0.4, 0.4)), Vec3f::zero());
    assert_eq!(vec::trunc(v(0.4, 0.5, 0.4)), Vec3f::zero());
    assert_eq!(vec::trunc(v(-0.4, -0.5, -0.4)), Vec3f::zero());
    assert_eq!(vec::trunc(v(-1.4, -1.5, -1.4)), v(-1.0, -1.0, -1.0));
}

#[test]
fn round() {
    let v = Vec3f::new;
    assert_eq!(vec::round(v(1.0, 0.0, 0.0)), v(1.0, 0.0, 0.0));
    assert_eq!(vec::round(v(1.0, 1.0, 1.0)), v(1.0, 1.0, 1.0));
    assert_eq!(vec::round(Vec3f::zero()), Vec3f::zero());
    assert_eq!(vec::round(vec::normalize(v(1.0, 1.0, 1.0))), Vec3f::one());
    assert_eq!(vec::round(v(0.4, 0.4, 0.4)), Vec3f::zero());
    assert_eq!(vec::round(v(0.4, 0.5, 0.4)), v(0.0, 1.0, 0.0));
    assert_eq!(vec::round(v(-0.4, -0.5, -0.4)), v(0.0, -1.0, 0.0));
}

#[test]
fn snap_down() {
    let v = Vec3f::new;
    let one = Vec3f::one();
    assert_eq!(vec::snap_down(Vec3f::zero(), one), Vec3f::zero());
    assert_eq!(vec::snap_down(v(0.4, 0.5, 0.6), one), Vec3f::zero());
    assert_eq!(vec::snap_down(v(-0.4, -0.5, -0.6), one), Vec3f::zero());
    assert_eq!(vec::snap_down(v(1.4, 1.5, 1.6), one), one);
    assert_eq!(vec::snap_down(v(-1.4, -1.5, -1.6), one), -one);
    assert_eq!(vec::snap_down(v(1.4, 1.5, 1.6), v(2.0, 2.0, 2.0)), Vec3f::zero());
    assert_eq!(vec::snap_down(v(-1.4, -1.5, -1.6), v(2.0, 2.0, 2.0)), Vec3f::zero());
    assert_eq!(vec::snap_down(v(1.4, 1.5, 1.6), v(2.0, 1.0, 1.0)), v(0.0, 1.0, 1.0));
    assert_eq!(
        vec::snap_down(v(-1.4, -1.5, -1.6), v(2.0, 1.0, 1.0)),
        v(0.0, -1.0, -1.0)
    );
}

#[test]
fn snap_up() {
    let v = Vec3f::new;
    let one = Vec3f::one();
    assert_eq!(vec::snap_up(Vec3f::zero(), one), Vec3f::zero());
    assert_eq!(vec::snap_up(v(0.4, 0.5, 0.6), one), one);
    assert_eq!(vec::snap_up(v(-0.4, -0.5, -0.6), one), -one);
    assert_eq!(vec::snap_up(v(1.4, 1.5, 1.6), one), v(2.0, 2.0, 2.0));
    assert_eq!(vec::snap_up(v(-1.4, -1.5, -1.6), one), v(-2.0, -2.0, -2.0));
    assert_eq!(vec::snap_up(v(1.4, 1.5, 1.6), v(3.0, 3.0, 3.0)), v(3.0, 3.0, 3.0));
    assert_eq!(
        vec::snap_up(v(-1.4, -1.5, -1.6), v(3.0, 3.0, 3.0)),
        v(-3.0, -3.0, -3.0)
    );
    assert_eq!(vec::snap_up(v(1.4, 1.5, 1.6), v(3.0, 1.0, 1.0)), v(3.0, 2.0, 2.0));
    assert_eq!(
        vec::snap_up(v(-1.4, -1.5, -1.6), v(3.0, 1.0, 1.0)),
        v(-3.0, -2.0, -2.0)
    );
}

#[test]
fn snap() {
    assert_eq!(
        vec::snap(Vec2f::new(7.0, -3.0), Vec2f::new(4.0, 12.0)),
        Vec2f::new(8.0, 0.0)
    );
    assert_eq!(
        vec::snap(Vec2f::new(7.0, -5.0), Vec2f::new(-4.0, -2.0)),
        Vec2f::new(8.0, -6.0)
    );
    assert_eq!(
        vec::snap(Vec2f::new(-7.0, 5.0), Vec2f::new(-4.0, -2.0)),
        Vec2f::new(-8.0, 6.0)
    );
}

#[test]
fn correct() {
    let v = Vec3f::new(1.1, 2.2, 3.3);
    assert_eq!(vec::correct(v, 0, f32::CORRECT_EPSILON), v);
    assert_eq!(vec::correct(v, 0, 0.4), Vec3f::new(1.0, 2.0, 3.0));
    assert_eq!(vec::correct(v, 1, 0.4), v);
}

#[test]
fn is_between() {
    let v = Vec3f::new;
    assert!(vec::is_between(v(1.0, 0.0, 0.0), v(0.0, 0.0, 0.0), v(2.0, 0.0, 0.0)));
    assert!(vec::is_between(v(1.0, 0.0, 0.0), v(2.0, 0.0, 0.0), v(0.0, 0.0, 0.0)));
    assert!(vec::is_between(v(1.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 0.0, 0.0)));
    assert!(vec::is_between(v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 0.0, 0.0)));
    assert!(!vec::is_between(v(2.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 0.0, 0.0)));
}

#[test]
fn average() {
    let v = Vec3f::new;
    let vecs = [v(1.0, 1.0, 1.0), v(1.0, 1.0, 1.0), v(2.0, 2.0, 2.0)];
    let expected = 4.0f32 / 3.0;
    assert_eq!(vec::average(vecs), Some(v(expected, expected, expected)));
    assert_eq!(vec::average(std::iter::empty::<Vec3f>()), None);
}

#[test]
fn measure_angle() {
    let v = Vec3f::new;
    let x = v(1.0, 0.0, 0.0);
    let up = v(0.0, 0.0, 1.0);

    assert_approx_eq!(vec::measure_angle_3d(v(1.0, 0.0, 0.0), x, up), 0.0f32);
    assert_approx_eq!(vec::measure_angle_3d(v(0.0, 1.0, 0.0), x, up), f32::HALF_PI);
    assert_approx_eq!(vec::measure_angle_3d(v(-1.0, 0.0, 0.0), x, up), f32::PI);
    assert_approx_eq!(
        vec::measure_angle_3d(v(0.0, -1.0, 0.0), x, up),
        3.0 * f32::HALF_PI
    );

    for degrees in [0.13f32, 15.13, 359.95] {
        assert_approx_eq!(
            scalar::to_degrees(vec::measure_angle_3d(
                rotate_pos_x_by_degrees(degrees),
                x,
                up
            )),
            degrees,
            0.002
        );
    }
}

#[test]
fn vector_generic_over_size() {
    // Vector operations compose across const-generic dimensions.
    let a: Vector<f64, 5> = Vector::from_array([1.0, 2.0, 3.0, 4.0, 5.0]);
    let b: Vector<f64, 5> = Vector::fill(1.0);
    assert_eq!(vec::dot(a, b), 15.0);
    assert_eq!((a - b).v, [0.0, 1.0, 2.0, 3.0, 4.0]);
}
