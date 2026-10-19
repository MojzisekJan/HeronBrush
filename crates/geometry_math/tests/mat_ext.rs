//! Tests for mat ext behavior.


#![allow(clippy::excessive_precision)]

use geometry_math::assert_approx_eq;
use geometry_math::bbox::Bbox3d;
use geometry_math::mat::Mat4x4d;
use geometry_math::mat_ext::*;
use geometry_math::quat::Quatd;
use geometry_math::scalar::{self, Float};
use geometry_math::util::Axis;
use geometry_math::vec::{self, Vec3d, Vec4d};

#[test]
fn rotation_matrix_with_euler_angles() {
    let deg90 = scalar::to_radians(90.0);
    assert_approx_eq!(
        rotation_matrix_from_euler(deg90, 0.0, 0.0),
        Mat4x4d::rot_90_x_ccw()
    );
    assert_approx_eq!(
        rotation_matrix_from_euler(0.0, deg90, 0.0),
        Mat4x4d::rot_90_y_ccw()
    );
    assert_approx_eq!(
        rotation_matrix_from_euler(0.0, 0.0, deg90),
        Mat4x4d::rot_90_z_ccw()
    );
}

#[test]
fn rotation_matrix_to_euler_angles_for_90_degree_rotations() {
    let deg90 = scalar::to_radians(90.0);
    assert_approx_eq!(
        rotation_matrix_to_euler_angles(&Mat4x4d::rot_90_x_ccw()),
        Vec3d::new(deg90, 0.0, 0.0)
    );
    assert_approx_eq!(
        rotation_matrix_to_euler_angles(&Mat4x4d::rot_90_y_ccw()),
        Vec3d::new(0.0, deg90, 0.0)
    );
    assert_approx_eq!(
        rotation_matrix_to_euler_angles(&Mat4x4d::rot_90_z_ccw()),
        Vec3d::new(0.0, 0.0, deg90)
    );
}

#[test]
fn rotation_matrix_to_euler_angles_round_trips() {
    let roll = scalar::to_radians(12.0);
    let pitch = scalar::to_radians(13.0);
    let yaw = scalar::to_radians(14.0);

    let rot = rotation_matrix_from_euler(roll, pitch, yaw);
    let recovered = rotation_matrix_to_euler_angles(&rot);

    assert_approx_eq!(recovered.x(), roll);
    assert_approx_eq!(recovered.y(), pitch);
    assert_approx_eq!(recovered.z(), yaw);
}

#[test]
fn rotation_matrix_with_axis_and_angle() {
    let deg90 = scalar::to_radians(90.0);
    assert_approx_eq!(
        rotation_matrix_about_axis(Vec3d::new(1.0, 0.0, 0.0), deg90),
        Mat4x4d::rot_90_x_ccw()
    );
    assert_approx_eq!(
        rotation_matrix_about_axis(Vec3d::new(0.0, 1.0, 0.0), deg90),
        Mat4x4d::rot_90_y_ccw()
    );
    assert_approx_eq!(
        rotation_matrix_about_axis(Vec3d::new(0.0, 0.0, 1.0), deg90),
        Mat4x4d::rot_90_z_ccw()
    );
    assert_approx_eq!(
        rotation_matrix_about_axis(Vec3d::new(0.0, 0.0, 1.0), deg90) * Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0)
    );
}

#[test]
fn rotation_matrix_with_quaternion() {
    let deg90 = scalar::to_radians(90.0);
    for (axis, expected) in [
        (Vec3d::new(1.0, 0.0, 0.0), Mat4x4d::rot_90_x_ccw()),
        (Vec3d::new(0.0, 1.0, 0.0), Mat4x4d::rot_90_y_ccw()),
        (Vec3d::new(0.0, 0.0, 1.0), Mat4x4d::rot_90_z_ccw()),
    ] {
        assert_approx_eq!(
            rotation_matrix_from_quat(&Quatd::from_axis_angle(axis, deg90)),
            expected
        );
    }
}

#[test]
fn the_quaternion_and_axis_angle_rotations_agree() {
    // A fixed spread of axes and angles makes the test reproducible.
    let axes = [
        Vec3d::new(1.0, 2.0, 3.0),
        Vec3d::new(-1.0, 0.5, 2.0),
        Vec3d::new(0.1, -0.9, 0.4),
        Vec3d::new(1.0, 1.0, 1.0),
        Vec3d::new(-3.0, -1.0, 2.0),
    ];
    let angles = [0.0, 0.3, 1.0, 2.5, f64::PI, 5.9];

    for axis in axes {
        let axis = vec::normalize(axis);
        for angle in angles {
            assert_approx_eq!(
                rotation_matrix_from_quat(&Quatd::from_axis_angle(axis, angle)),
                rotation_matrix_about_axis(axis, angle)
            );
        }
    }
}

#[test]
fn rotation_matrix_from_to_rotates_one_onto_the_other() {
    let from = vec::normalize(Vec3d::new(0.0, 1.0, 0.0));
    let to = vec::normalize(Vec3d::new(1.0, 0.0, 0.0));
    assert_approx_eq!(rotation_matrix_from_to(from, to) * from, to);
}

#[test]
fn translation_matrix_translates() {
    let v = Vec3d::new(2.0, 3.0, 4.0);
    let t = translation_matrix(v);

    assert_approx_eq!(t[0], Vec4d::new(1.0, 0.0, 0.0, 0.0));
    assert_approx_eq!(t[1], Vec4d::new(0.0, 1.0, 0.0, 0.0));
    assert_approx_eq!(t[2], Vec4d::new(0.0, 0.0, 1.0, 0.0));
    assert_approx_eq!(t[3], v.with_w(1.0));

    assert_approx_eq!(t * Vec3d::new(1.0, 1.0, 1.0), Vec3d::new(3.0, 4.0, 5.0));
}

#[test]
fn translation_part_and_strip_translation() {
    let v = Vec3d::new(2.0, 3.0, 4.0);
    let t = translation_matrix(v);
    let s = scaling_matrix(Vec3d::new(2.0, 3.0, 4.0));

    assert_approx_eq!(strip_translation(&(s * t)), s);
    assert_approx_eq!(strip_translation(&(t * s)), s);
    assert_approx_eq!(translation_part(&(s * t)), translation_matrix(s * v));
}

#[test]
fn scaling_matrix_scales_each_axis() {
    assert_eq!(
        scaling_matrix(Vec3d::new(2.0, 3.0, 4.0)),
        Mat4x4d::from_rows([
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    );
}

#[test]
fn mirror_matrix_flips_one_axis() {
    let x = Vec3d::new(1.0, 0.0, 0.0);
    let y = Vec3d::new(0.0, 1.0, 0.0);
    let z = Vec3d::new(0.0, 0.0, 1.0);

    let mir_x = mirror_matrix::<f64>(Axis::X);
    assert_eq!(mir_x * x, -x);
    assert_eq!(mir_x * y, y);
    assert_eq!(mir_x * z, z);

    let mir_y = mirror_matrix::<f64>(Axis::Y);
    assert_eq!(mir_y * x, x);
    assert_eq!(mir_y * y, -y);
    assert_eq!(mir_y * z, z);

    let mir_z = mirror_matrix::<f64>(Axis::Z);
    assert_eq!(mir_z * x, x);
    assert_eq!(mir_z * y, y);
    assert_eq!(mir_z * z, -z);
}

#[test]
fn orientation_preserving_transforms_are_recognised() {
    let preserving = |m: Mat4x4d| is_orientation_preserving_transform(&m);

    // A mirror flips orientation.
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        assert!(!preserving(mirror_matrix::<f64>(axis)));
    }

    assert!(preserving(translation_matrix(Vec3d::new(1.0, 2.0, 3.0))));
    assert!(preserving(scaling_matrix(Vec3d::new(1.0, 2.0, 3.0))));
    // A negative scale factor is a mirror in disguise.
    assert!(!preserving(scaling_matrix(Vec3d::new(-1.0, 2.0, 3.0))));
    assert!(preserving(rotation_matrix_about_axis(
        Vec3d::new(0.0, 0.0, 1.0),
        scalar::to_radians(23.0)
    )));
}

#[test]
fn coordinate_system_matrix_maps_into_the_system() {
    let o = Vec3d::new(1.0, 1.0, 1.0);
    let m = coordinate_system_matrix(
        Vec3d::new(-1.0, 0.0, 0.0),
        Vec3d::new(0.0, -1.0, 0.0),
        Vec3d::new(0.0, 0.0, -1.0),
        o,
    )
    .expect("the three axes are independent");

    assert_eq!(m * Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(-1.0, 0.0, 0.0) + o);
    assert_eq!(m * Vec3d::new(0.0, 1.0, 0.0), Vec3d::new(0.0, -1.0, 0.0) + o);
    assert_eq!(m * Vec3d::new(0.0, 0.0, 1.0), Vec3d::new(0.0, 0.0, -1.0) + o);
}

#[test]
fn coordinate_system_matrix_reports_dependent_axes() {
    let x = Vec3d::new(1.0, 0.0, 0.0);
    assert_eq!(coordinate_system_matrix(x, x, x, Vec3d::zero()), None);
}

#[test]
fn shear_matrix_shears_one_axis_by_another() {
    // The parameters are named for what they shear *by*: `sxy` and `sxz` scale the
    // contribution of x to y and to z, and so on. Each triple below moves one input axis
    // and leaves the origin fixed.
    let by_z = shear_matrix(0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
    assert_eq!(by_z * Vec3d::new(0.0, 0.0, 1.0), Vec3d::new(1.0, 1.0, 1.0));
    assert_eq!(by_z * Vec3d::zero(), Vec3d::zero());

    let by_y = shear_matrix(0.0, 0.0, 1.0, 1.0, 0.0, 0.0);
    assert_eq!(by_y * Vec3d::new(0.0, 1.0, 0.0), Vec3d::new(1.0, 1.0, 1.0));
    assert_eq!(by_y * Vec3d::zero(), Vec3d::zero());

    let by_x = shear_matrix(1.0, 1.0, 0.0, 0.0, 0.0, 0.0);
    assert_eq!(by_x * Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(1.0, 1.0, 1.0));
    assert_eq!(by_x * Vec3d::zero(), Vec3d::zero());
}

#[test]
fn perspective_and_ortho_matrices_map_the_near_plane() {
    // An orthographic volume maps its own corners onto the clip cube.
    let m = ortho_matrix(1.0, 100.0, -10.0, 10.0, 10.0, -10.0);
    assert_approx_eq!(m * Vec3d::new(-10.0, -10.0, -1.0), Vec3d::new(-1.0, -1.0, -1.0));
    assert_approx_eq!(m * Vec3d::new(10.0, 10.0, -100.0), Vec3d::new(1.0, 1.0, 1.0));

    // A perspective matrix keeps the view direction on the axis.
    let p = perspective_matrix(90.0, 1.0, 100.0, 800, 600);
    let on_axis = p * Vec3d::new(0.0, 0.0, -10.0);
    assert_approx_eq!(on_axis.x(), 0.0);
    assert_approx_eq!(on_axis.y(), 0.0);
}

#[test]
fn the_view_matrix_maps_the_view_direction_onto_negative_z() {
    let direction = Vec3d::new(0.0, 1.0, 0.0);
    let up = Vec3d::new(0.0, 0.0, 1.0);
    let m = view_matrix(direction, up);

    assert_approx_eq!(m * direction, Vec3d::new(0.0, 0.0, -1.0));
    assert_approx_eq!(m * up, Vec3d::new(0.0, 1.0, 0.0));
}

#[test]
fn a_matrix_applies_to_a_sequence_of_points_through_map() {
    // Applies a matrix to each point through an iterator.
    let m = translation_matrix(Vec3d::new(1.0, 2.0, 3.0));
    let points = [Vec3d::zero(), Vec3d::new(1.0, 1.0, 1.0)];

    let transformed: Vec<Vec3d> = points.iter().map(|p| m * *p).collect();
    assert_eq!(
        transformed,
        vec![Vec3d::new(1.0, 2.0, 3.0), Vec3d::new(2.0, 3.0, 4.0)]
    );
}

/* ========== the functions that needed plane and bbox ========== */

#[test]
fn plane_projection_matrix_flattens_onto_the_plane() {
    // The plane at x = 160, expressed with a normal pointing the other way.
    let m = plane_projection_matrix(-160.0, Vec3d::new(-1.0, 0.0, 0.0))
        .expect("the axes are independent");

    // A point on that plane lands at z = 0; its x and y may be anything.
    assert_approx_eq!((m * Vec3d::new(160.0, 1.0, 2.0)).z(), 0.0);

    // A point one unit off the plane lands one unit off z = 0.
    let off = (m * Vec3d::new(161.0, 1.0, 2.0)).z();
    assert_approx_eq!(scalar::abs(off), 1.0);
}

#[test]
fn plane_projection_along_a_direction_in_the_plane_is_rejected() {
    let normal = Vec3d::new(0.0, 0.0, 1.0);
    // A direction lying in the plane gives a degenerate coordinate system.
    assert_eq!(
        plane_projection_matrix_along(0.0, normal, Vec3d::new(1.0, 0.0, 0.0)),
        None
    );
}

#[test]
fn scale_bbox_matrix_maps_one_box_onto_another() {
    let old_box = Bbox3d::from_scalars(-1.0, 1.0);
    let new_box = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(4.0, 6.0, 8.0));

    let m = scale_bbox_matrix(&old_box, &new_box);
    assert_approx_eq!(m * old_box.min, new_box.min);
    assert_approx_eq!(m * old_box.max, new_box.max);
    assert_approx_eq!(m * old_box.center(), new_box.center());
}

#[test]
fn scale_bbox_matrix_with_anchor_keeps_the_anchor_fixed() {
    let old_box = Bbox3d::from_scalars(-1.0, 1.0);
    let anchor = old_box.min;

    let m = scale_bbox_matrix_with_anchor(&old_box, Vec3d::new(4.0, 4.0, 4.0), anchor);
    assert_approx_eq!(m * anchor, anchor);
    // The box has doubled in each direction away from the anchor.
    assert_approx_eq!(m * old_box.max, Vec3d::new(3.0, 3.0, 3.0));
}

#[test]
fn shear_bbox_matrix_holds_the_opposite_face_still() {
    let b = Bbox3d::from_scalars(-1.0, 1.0);
    let top = Vec3d::new(0.0, 0.0, 1.0);
    let delta = Vec3d::new(2.0, 0.0, 0.0);

    let m = shear_bbox_matrix(&b, top, delta).expect("+Z is one of the six faces");

    // The bottom face is unmoved.
    assert_approx_eq!(
        m * Vec3d::new(-1.0, -1.0, -1.0),
        Vec3d::new(-1.0, -1.0, -1.0)
    );
    // The top face has slid along X by the full delta.
    assert_approx_eq!(m * Vec3d::new(-1.0, -1.0, 1.0), Vec3d::new(1.0, -1.0, 1.0));
}

#[test]
fn shear_bbox_matrix_rejects_a_side_that_is_not_a_face() {
    let b = Bbox3d::from_scalars(-1.0, 1.0);
    assert_eq!(
        shear_bbox_matrix(&b, Vec3d::new(1.0, 1.0, 0.0), Vec3d::one()),
        None
    );
}

#[test]
fn points_transformation_matrix_recovers_a_known_transform() {
    let input = [
        Vec3d::new(2.0, 0.0, 0.0),
        Vec3d::new(4.0, 0.0, 0.0),
        Vec3d::new(2.0, 2.0, 0.0),
    ];

    let m = translation_matrix(Vec3d::new(100.0, 100.0, 100.0))
        * scaling_matrix(Vec3d::new(2.0, 2.0, 2.0))
        * rotation_matrix_about_axis(Vec3d::new(0.0, 0.0, 1.0), scalar::to_radians(90.0));
    let output: Vec<Vec3d> = input.iter().map(|p| m * *p).collect();

    let recovered =
        points_transformation_matrix_3(input[0], input[1], input[2], output[0], output[1], output[2])
            .expect("the three points are not colinear");

    for (i, p) in input.iter().enumerate() {
        assert_approx_eq!(recovered * *p, output[i]);
    }
}

#[test]
fn points_transformation_matrix_reports_a_degenerate_input() {
    // Three colinear points cannot pin down a transform.
    let a = Vec3d::zero();
    let b = Vec3d::new(1.0, 0.0, 0.0);
    let c = Vec3d::new(2.0, 0.0, 0.0);
    assert_eq!(points_transformation_matrix_3(a, b, c, a, b, c), None);
}
