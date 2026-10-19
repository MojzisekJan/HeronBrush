//! Tests for quat behavior.

#![allow(clippy::excessive_precision)]

use geometry_math::assert_approx_eq;
use geometry_math::quat::{self, Quatd, Quatf};
use geometry_math::scalar::{self, Float};
use geometry_math::vec::{self, Vec3d, Vec3f};

/// A rotation of 15 degrees about the positive Z axis.
fn q_15_deg_about_z() -> Quatf {
    Quatf::new(0.991444885, Vec3f::new(0.0, 0.0, 0.1305262))
}

/// A rotation of 15 degrees about the positive X axis.
fn q_15_deg_about_x() -> Quatf {
    Quatf::new(0.991444885, Vec3f::new(0.1305262, 0.0, 0.0))
}

#[test]
fn constructor_default() {
    let q = Quatf::default();
    assert_eq!(q.r, 0.0);
    assert!(vec::is_zero(q.v, f32::ALMOST_ZERO));
}

#[test]
fn constructor_with_rotation() {
    let angle = scalar::to_radians(15.0f32);
    let axis = vec::normalize(Vec3f::new(1.0, 2.0, 3.0));
    let q = Quatf::from_axis_angle(axis, angle);

    assert_approx_eq!(q.r, (angle / 2.0).cos());
    assert_approx_eq!(q.v, axis * (angle / 2.0).sin());
}

#[test]
fn constructor_with_vector_rotation() {
    let from = Vec3f::new(0.0, 1.0, 0.0);
    let to = Vec3f::new(1.0, 0.0, 0.0);
    let q = Quatf::from_to(from, to);
    assert_approx_eq!(q * from, to);
}

#[test]
fn constructor_with_opposite_vector_rotation() {
    for i in 0..3 {
        let mut from = Vec3d::zero();
        let mut to = Vec3d::zero();
        from[i] = 1.0;
        to[i] = -1.0;

        let q = Quatd::from_to(from, to);
        assert_approx_eq!(q * from, to);
        // The rotation axis must be perpendicular to both vectors.
        assert_approx_eq!(vec::dot(q.axis(), from), 0.0);
        assert_approx_eq!(vec::dot(q.axis(), to), 0.0);
    }
}

#[test]
fn constructor_with_equal_vector_rotation() {
    for i in 0..3 {
        let mut from = Vec3d::zero();
        from[i] = 1.0;
        let to = from;

        let q = Quatd::from_to(from, to);
        assert_approx_eq!(q * from, to);
    }
}

#[test]
fn angle() {
    let angle = scalar::to_radians(15.0f32);
    let q = Quatf::from_axis_angle(Vec3f::new(0.0, 0.0, 1.0), angle);
    assert_approx_eq!(q.angle(), angle, 0.001);
}

#[test]
fn axis() {
    assert_approx_eq!(Quatd::default().axis(), Vec3d::zero());
    assert_approx_eq!(
        Quatd::from_axis_angle(Vec3d::new(0.0, 0.0, 1.0), scalar::to_radians(45.0)).axis(),
        Vec3d::new(0.0, 0.0, 1.0)
    );
    let diagonal = vec::normalize(Vec3d::new(1.0, 1.0, 0.0));
    assert_approx_eq!(
        Quatd::from_axis_angle(diagonal, scalar::to_radians(25.0)).axis(),
        diagonal
    );
}

#[test]
fn conjugate() {
    let q = q_15_deg_about_z();
    assert_approx_eq!(q.conjugate().v, -q.v);
    assert_eq!(q.conjugate().r, q.r);
}

#[test]
fn is_equal() {
    assert!(quat::is_equal(&Quatd::default(), &Quatd::default(), 0.0));

    let q = q_15_deg_about_z();
    assert!(quat::is_equal(&q, &q, 0.0));
    // A quaternion and its negation describe the same rotation.
    assert!(!quat::is_equal(&q, &-q, 0.0));
    assert!(quat::same_rotation(&q, &-q, 0.0));
}

#[test]
fn equality_operators() {
    assert!(Quatd::default() == Quatd::default());

    let q = q_15_deg_about_z();
    let p = q_15_deg_about_x();

    assert!(q == q);
    assert!(q != -q);
    assert!(p == p);
    assert!(p != -p);
    assert!(q != p);
}

#[test]
fn operator_unary_minus_negates_all_components() {
    let q = q_15_deg_about_x();
    let nq = -q;
    assert_approx_eq!(nq.r, -q.r);
    assert_approx_eq!(nq.v, -q.v);
}

#[test]
fn operator_multiply_scalar_scales_all_components() {
    let q = q_15_deg_about_x();
    assert_approx_eq!((q * 2.0).r, q.r * 2.0);
    assert_approx_eq!((2.0 * q).r, q.r * 2.0);
    // Scalar multiplication applies to every component.
    assert_eq!((q * 2.0).v, q.v * 2.0);
    assert_eq!(2.0 * q, q * 2.0);
}

#[test]
fn operator_multiply_quaternions_composes_rotations() {
    let q1 = q_15_deg_about_z();
    // A rotation of 10 degrees about the positive Z axis.
    let q2 = Quatf::new(0.99619472, Vec3f::new(0.0, 0.0, 0.0871557369));
    let q = q1 * q2;

    let v = Vec3f::new(1.0, 0.0, 0.0);
    let w = q * v;

    let cos_a1_a2 = 0.906307756; // cos(15 deg + 10 deg)
    let sin_a1_a2 = 0.42261827; // sin(15 deg + 10 deg)
    assert_approx_eq!(w, Vec3f::new(cos_a1_a2, sin_a1_a2, 0.0));
}

#[test]
fn operator_multiply_vector_rotates_it() {
    let q = q_15_deg_about_z();
    let x = Vec3f::new(1.0, 0.0, 0.0);

    let cos_a = 0.965925812; // cos(15 deg)
    let sin_a = 0.258819044; // sin(15 deg)
    assert_approx_eq!(q * x, Vec3f::new(cos_a, sin_a, 0.0));
}

#[test]
fn a_rotation_and_its_conjugate_cancel() {
    let axis = vec::normalize(Vec3d::new(1.0, -2.0, 0.5));
    let q = Quatd::from_axis_angle(axis, scalar::to_radians(37.0));
    let v = Vec3d::new(3.0, 4.0, 5.0);

    assert_approx_eq!(q.conjugate() * (q * v), v);
}

#[test]
fn negation_preserves_rotation_and_conjugation_reverses_it() {
    let q = Quatd::from_axis_angle(Vec3d::new(0.0, 0.0, 1.0), 0.7);
    let point = Vec3d::new(1.0, 2.0, 3.0);
    assert_approx_eq!(q * point, (-q) * point);
    assert_approx_eq!(q.conjugate() * (q * point), point);
    assert!(quat::same_rotation(&q, &-q, 1e-12));
    assert!(!quat::same_rotation(&q, &q.conjugate(), 1e-12));
    assert_ne!(q, q.conjugate());
    assert_eq!(q * -1.0, -q);
}
