//! Transform matrices built from higher level descriptions.
//!
//! Projection, rotation, translation and scaling matrices.
//!
//! Affine transform implementations cover the concrete dimensions used by the editor without
//! requiring generic dimension arithmetic.

use crate::bbox::Bbox;
use crate::mat::{self, Determinant, Matrix};
use crate::quat::Quat;
use crate::scalar::{self, Float};
use crate::util::Axis;
use crate::vec::{self, Vector};

/* ========== projections ========== */

/// How much of the tangent of half the field of view the frustum actually spans.
///
/// Perspective projection uses three quarters of the tangent of half the configured field of view.
/// Picking and screen projection must use the same factor as rendering.
pub const FRUSTUM_SCALE: f64 = 0.75;

/// Returns a perspective projection matrix.
///
/// `fov` is the vertical field of view in degrees, and `width` and `height` give the
/// viewport's aspect ratio.
pub fn perspective_matrix<T: Float>(
    fov: T,
    near_plane: T,
    far_plane: T,
    width: i32,
    height: i32,
) -> Matrix<T, 4, 4> {
    let v_frustum =
        (scalar::to_radians(fov) / T::TWO).tan() * T::from_f64(FRUSTUM_SCALE) * near_plane;
    let h_frustum = v_frustum * T::from_i32(width) / T::from_i32(height);
    let depth = far_plane - near_plane;

    let z = T::ZERO;
    Matrix::from_rows([
        [near_plane / h_frustum, z, z, z],
        [z, near_plane / v_frustum, z, z],
        [
            z,
            z,
            -(far_plane + near_plane) / depth,
            -T::TWO * far_plane * near_plane / depth,
        ],
        [z, z, -T::ONE, z],
    ])
}

/// Returns an orthographic projection matrix for the given clipping volume.
pub fn ortho_matrix<T: Float>(
    near_plane: T,
    far_plane: T,
    left: T,
    top: T,
    right: T,
    bottom: T,
) -> Matrix<T, 4, 4> {
    let width = right - left;
    let height = top - bottom;
    let depth = far_plane - near_plane;

    let z = T::ZERO;
    Matrix::from_rows([
        [T::TWO / width, z, z, -(left + right) / width],
        [z, T::TWO / height, z, -(top + bottom) / height],
        [z, z, -T::TWO / depth, -(far_plane + near_plane) / depth],
        [z, z, z, T::ONE],
    ])
}

/// Returns a view matrix looking along `direction` with the given `up` vector.
///
/// Both vectors are expected to be normalized and perpendicular.
pub fn view_matrix<T: Float>(direction: Vector<T, 3>, up: Vector<T, 3>) -> Matrix<T, 4, 4> {
    let f = direction;
    let s = vec::cross(f, up);
    let u = vec::cross(s, f);

    let z = T::ZERO;
    Matrix::from_rows([
        [s[0], s[1], s[2], z],
        [u[0], u[1], u[2], z],
        [-f[0], -f[1], -f[2], z],
        [z, z, z, T::ONE],
    ])
}

/* ========== rotations ========== */

/// Returns a rotation matrix for the given roll, pitch and yaw, in radians.
///
/// The rotations are applied roll first, then pitch, then yaw.
pub fn rotation_matrix_from_euler<T: Float>(roll: T, pitch: T, yaw: T) -> Matrix<T, 4, 4> {
    let (o, i) = (T::ZERO, T::ONE);

    let (cr, sr) = (roll.cos(), roll.sin());
    let r = Matrix::from_rows([
        [i, o, o, o],
        [o, cr, -sr, o],
        [o, sr, cr, o],
        [o, o, o, i],
    ]);

    let (cp, sp) = (pitch.cos(), pitch.sin());
    let p = Matrix::from_rows([
        [cp, o, sp, o],
        [o, i, o, o],
        [-sp, o, cp, o],
        [o, o, o, i],
    ]);

    let (cy, sy) = (yaw.cos(), yaw.sin());
    let y = Matrix::from_rows([
        [cy, -sy, o, o],
        [sy, cy, o, o],
        [o, o, i, o],
        [o, o, o, i],
    ]);

    y * p * r
}

/// Recovers the roll, pitch and yaw from a rotation matrix, returned in that order.
///
/// From <http://www.gregslabaugh.net/publications/euler.pdf>. That paper is written for
/// row-major matrices with 1-based indices; the indexing here is column-major and
/// 0-based, as everywhere else in this crate.
pub fn rotation_matrix_to_euler_angles<T: Float>(rot: &Matrix<T, 4, 4>) -> Vector<T, 3> {
    let (theta, psi, phi);

    if scalar::abs(rot[0][2]) != T::ONE {
        theta = -rot[0][2].asin();
        let cos_theta = theta.cos();
        psi = (rot[1][2] / cos_theta).atan2(rot[2][2] / cos_theta);
        phi = (rot[0][1] / cos_theta).atan2(rot[0][0] / cos_theta);
    } else {
        // Gimbal lock: pitch is straight up or straight down, and roll and yaw are no
        // longer independent, so yaw is pinned to zero.
        phi = T::ZERO;
        if rot[0][2] == -T::ONE {
            theta = T::HALF_PI;
            psi = rot[1][0].atan2(rot[2][0]);
        } else {
            theta = -T::HALF_PI;
            psi = (-rot[1][0]).atan2(-rot[2][0]);
        }
    }

    Vector::<T, 3>::new(psi, theta, phi)
}

/// Returns a matrix rotating by `angle` radians about the given axis.
///
/// The axis is expected to be normalized.
pub fn rotation_matrix_about_axis<T: Float>(
    axis: Vector<T, 3>,
    angle: T,
) -> Matrix<T, 4, 4> {
    let s = (-angle).sin();
    let c = (-angle).cos();
    let i = T::ONE - c;

    let ix = i * axis[0];
    let ix2 = ix * axis[0];
    let ixy = ix * axis[1];
    let ixz = ix * axis[2];
    let iy = i * axis[1];
    let iy2 = iy * axis[1];
    let iyz = iy * axis[2];
    let iz2 = i * axis[2] * axis[2];
    let sx = s * axis[0];
    let sy = s * axis[1];
    let sz = s * axis[2];

    let z = T::ZERO;
    Matrix::from_rows([
        [ix2 + c, ixy + sz, ixz - sy, z],
        [ixy - sz, iy2 + c, iyz + sx, z],
        [ixz + sy, iyz - sx, iz2 + c, z],
        [z, z, z, T::ONE],
    ])
}

/// Returns the rotation matrix equivalent to the given quaternion.
pub fn rotation_matrix_from_quat<T: Float>(q: &Quat<T>) -> Matrix<T, 4, 4> {
    let (x, y, z) = (q.v[0], q.v[1], q.v[2]);
    let w = q.r;
    let (x2, y2, z2) = (x * x, y * y, z * z);
    let (one, two, o) = (T::ONE, T::TWO, T::ZERO);

    Matrix::from_rows([
        [one - two * (y2 + z2), two * (x * y - z * w), two * (x * z + y * w), o],
        [two * (x * y + z * w), one - two * (x2 + z2), two * (y * z - x * w), o],
        [two * (x * z - y * w), two * (y * z + x * w), one - two * (x2 + y2), o],
        [o, o, o, one],
    ])
}

/// Returns the matrix for the shortest rotation taking `from` to `to`.
///
/// Both vectors are expected to be normalized.
pub fn rotation_matrix_from_to<T: Float>(
    from: Vector<T, 3>,
    to: Vector<T, 3>,
) -> Matrix<T, 4, 4> {
    rotation_matrix_from_quat(&Quat::from_to(from, to))
}

/* ========== translation and scaling ========== */

/// Returns a 4x4 matrix translating by the given 3D delta.
pub fn translation_matrix<T: Float>(delta: Vector<T, 3>) -> Matrix<T, 4, 4> {
    let mut result = Matrix::identity();
    for i in 0..3 {
        result.v[3].v[i] = delta[i];
    }
    result
}

/// How far a matrix moves things, if moving them is all it does.
///
/// `None` for anything that also turns, scales or mirrors. What it is for: a translation
/// takes a convex solid to the same solid somewhere else, so a caller that has one
/// already built can move it rather than work it out again.
pub fn translation_of<T: Float>(matrix: &Matrix<T, 4, 4>) -> Option<Vector<T, 3>> {
    let identity: Matrix<T, 4, 4> = Matrix::identity();
    for column in 0..3 {
        for row in 0..4 {
            if !scalar::is_equal(
                matrix.v[column].v[row],
                identity.v[column].v[row],
                T::ALMOST_ZERO,
            ) {
                return None;
            }
        }
    }
    // The last column is the translation, and its w must still be 1.
    if !scalar::is_equal(matrix.v[3].v[3], T::ONE, T::ALMOST_ZERO) {
        return None;
    }
    Some(Vector::<T, 3>::new(matrix.v[3].v[0], matrix.v[3].v[1], matrix.v[3].v[2]))
}

/// Returns a 3x3 matrix translating by the given 2D delta.
pub fn translation_matrix_2d<T: Float>(delta: Vector<T, 2>) -> Matrix<T, 3, 3> {
    let mut result = Matrix::identity();
    for i in 0..2 {
        result.v[2].v[i] = delta[i];
    }
    result
}

/// Returns the translation part of the given transform, i.e. the transform with its
/// rotation and scaling stripped out.
pub fn translation_part<T: Float, const S: usize>(m: &Matrix<T, S, S>) -> Matrix<T, S, S> {
    let mut result = Matrix::identity();
    for i in 0..S - 1 {
        result.v[S - 1].v[i] = m.v[S - 1].v[i];
    }
    result
}

/// Returns the given transform with its translation removed.
pub fn strip_translation<T: Float, const S: usize>(m: &Matrix<T, S, S>) -> Matrix<T, S, S> {
    let mut result = *m;
    for i in 0..S - 1 {
        result.v[S - 1].v[i] = T::ZERO;
    }
    result
}

/// Returns a 4x4 matrix scaling by the given per-axis factors.
pub fn scaling_matrix<T: Float>(factors: Vector<T, 3>) -> Matrix<T, 4, 4> {
    let mut result = Matrix::identity();
    for i in 0..3 {
        result.v[i].v[i] = factors[i];
    }
    result
}

/// Returns a 3x3 matrix scaling by the given per-axis factors.
pub fn scaling_matrix_2d<T: Float>(factors: Vector<T, 2>) -> Matrix<T, 3, 3> {
    let mut result = Matrix::identity();
    for i in 0..2 {
        result.v[i].v[i] = factors[i];
    }
    result
}

/// Returns a matrix mirroring along the given axis.
pub fn mirror_matrix<T: Float>(axis: Axis) -> Matrix<T, 4, 4> {
    match axis {
        Axis::X => Matrix::mirror_x(),
        Axis::Y => Matrix::mirror_y(),
        Axis::Z => Matrix::mirror_z(),
    }
}

/// Returns a shear matrix with the given six shear factors.
pub fn shear_matrix<T: Float>(sxy: T, sxz: T, syx: T, syz: T, szx: T, szy: T) -> Matrix<T, 4, 4> {
    let (o, i) = (T::ZERO, T::ONE);
    Matrix::from_rows([
        [i, syx, szx, o],
        [sxy, i, szy, o],
        [sxz, syz, i, o],
        [o, o, o, i],
    ])
}

/// Returns whether the given transform preserves orientation, i.e. does not mirror.
///
/// This looks at the determinant of the transform's linear part, ignoring translation.
pub fn is_orientation_preserving_transform<T: Float>(transform: &Matrix<T, 4, 4>) -> bool {
    let linear: Matrix<T, 3, 3> = mat::extract_minor(transform, 3, 3);
    linear.determinant() > T::ZERO
}

/// Returns the matrix taking world coordinates into the coordinate system with the given
/// axes and origin.
///
/// Returns `None` when the axes are linearly dependent and do not define a coordinate system.
pub fn coordinate_system_matrix<T: Float>(
    x: Vector<T, 3>,
    y: Vector<T, 3>,
    z: Vector<T, 3>,
    o: Vector<T, 3>,
) -> Option<Matrix<T, 4, 4>> {
    mat::invert(Matrix::from_rows([
        [x[0], y[0], z[0], o[0]],
        [x[1], y[1], z[1], o[1]],
        [x[2], y[2], z[2], o[2]],
        [T::ZERO, T::ZERO, T::ZERO, T::ONE],
    ]))
}

/* ========== plane projection ========== */

/// Returns a matrix projecting onto the given plane along the given direction.
///
/// The result maps world space into a coordinate system whose X and Y axes lie in the
/// plane and whose Z axis is the projection direction, so dropping the Z component of a
/// transformed point gives its projection.
///
/// Returns `None` if the three axes turn out to be linearly dependent, which happens when
/// `direction` lies in the plane.
pub fn plane_projection_matrix_along<T: Float>(
    distance: T,
    normal: Vector<T, 3>,
    direction: Vector<T, 3>,
) -> Option<Matrix<T, 4, 4>> {
    // Cross the normal with whichever cardinal axis it is least aligned with, so the
    // result cannot be degenerate.
    let reference = if normal.find_abs_max_component(0) == 0 {
        Vector::<T, 3>::new(T::ZERO, T::ZERO, T::ONE)
    } else {
        Vector::<T, 3>::new(T::ONE, T::ZERO, T::ZERO)
    };
    let x_axis = vec::normalize(vec::cross(normal, reference));
    let y_axis = vec::normalize(vec::cross(normal, x_axis));

    debug_assert!(vec::is_unit(x_axis, T::ALMOST_ZERO));
    debug_assert!(vec::is_unit(y_axis, T::ALMOST_ZERO));
    debug_assert!(vec::is_unit(direction, T::ALMOST_ZERO));

    coordinate_system_matrix(x_axis, y_axis, direction, normal * distance)
}

/// Returns a matrix projecting orthogonally onto the given plane.
#[inline]
pub fn plane_projection_matrix<T: Float>(
    distance: T,
    normal: Vector<T, 3>,
) -> Option<Matrix<T, 4, 4>> {
    plane_projection_matrix_along(distance, normal, normal)
}

/* ========== bounding box transforms ========== */

/// Returns the transform taking `old_box` onto `new_box`.
pub fn scale_bbox_matrix<T: Float>(
    old_box: &Bbox<T, 3>,
    new_box: &Bbox<T, 3>,
) -> Matrix<T, 4, 4> {
    let scale_factors = new_box.size() / old_box.size();
    translation_matrix(new_box.min)
        * scaling_matrix(scale_factors)
        * translation_matrix(-old_box.min)
}

/// Returns the transform resizing `old_box` to `new_size` while keeping `anchor_point`
/// fixed.
pub fn scale_bbox_matrix_with_anchor<T: Float>(
    old_box: &Bbox<T, 3>,
    new_size: Vector<T, 3>,
    anchor_point: Vector<T, 3>,
) -> Matrix<T, 4, 4> {
    let scale_factors = new_size / old_box.size();
    translation_matrix(anchor_point)
        * scaling_matrix(scale_factors)
        * translation_matrix(-anchor_point)
}

/// Returns the transform shearing the given box's face by `delta`, holding the opposite
/// face in place.
///
/// `side_to_shear` names the face by its outward normal, which must be one of the six
/// axis directions; anything else returns `None`.
pub fn shear_bbox_matrix<T: Float>(
    box_: &Bbox<T, 3>,
    side_to_shear: Vector<T, 3>,
    delta: Vector<T, 3>,
) -> Option<Matrix<T, 4, 4>> {
    let size = box_.size();
    let o = T::ZERO;

    // The delta is expressed relative to the box's extent along the sheared axis, so the
    // shear is proportional to how far across the box a point is.
    let shear = match (side_to_shear.x(), side_to_shear.y(), side_to_shear.z()) {
        (x, y, z) if x == T::ONE && y == o && z == o => {
            let d = delta / size.x();
            shear_matrix(d.y(), d.z(), o, o, o, o)
        }
        (x, y, z) if x == -T::ONE && y == o && z == o => {
            let d = delta / size.x();
            shear_matrix(-d.y(), -d.z(), o, o, o, o)
        }
        (x, y, z) if x == o && y == T::ONE && z == o => {
            let d = delta / size.y();
            shear_matrix(o, o, d.x(), d.z(), o, o)
        }
        (x, y, z) if x == o && y == -T::ONE && z == o => {
            let d = delta / size.y();
            shear_matrix(o, o, -d.x(), -d.z(), o, o)
        }
        (x, y, z) if x == o && y == o && z == T::ONE => {
            let d = delta / size.z();
            shear_matrix(o, o, o, o, d.x(), d.y())
        }
        (x, y, z) if x == o && y == o && z == -T::ONE => {
            let d = delta / size.z();
            shear_matrix(o, o, o, o, -d.x(), -d.y())
        }
        // Rejects an invalid shear argument instead of silently returning the identity matrix.
        _ => return None,
    };

    // Shear about a vertex of the opposite face, so that face stays put.
    let opposite = -side_to_shear;
    let anchor = box_
        .faces()
        .iter()
        .find(|f| f.normal == opposite)
        .map(|f| f.vertices[0])?;

    Some(translation_matrix(anchor) * shear * translation_matrix(-anchor))
}

/* ========== transform from point correspondences ========== */

/// Returns the affine transform taking the four given input points onto the four given
/// output points.
///
/// The first three points of each set are expected to be coplanar and the fourth off that
/// plane, so that the four together pin down an affine map. Returns `None` if they do not
/// — if the input points are coplanar or colinear, the system has no unique solution.
#[allow(clippy::too_many_arguments)]
pub fn points_transformation_matrix<T: Float>(
    on_plane0_in: Vector<T, 3>,
    on_plane1_in: Vector<T, 3>,
    on_plane2_in: Vector<T, 3>,
    off_plane_in: Vector<T, 3>,
    on_plane0_out: Vector<T, 3>,
    on_plane1_out: Vector<T, 3>,
    on_plane2_out: Vector<T, 3>,
    off_plane_out: Vector<T, 3>,
) -> Option<Matrix<T, 4, 4>> {
    // Translating both sets so their first point sits at the origin removes the
    // translation from the problem, leaving a pure 3x3 linear map to solve for. The
    // translation is put back at the end.
    let v0_in = on_plane1_in - on_plane0_in;
    let v1_in = on_plane2_in - on_plane0_in;
    let v2_in = off_plane_in - on_plane0_in;
    let v0_out = on_plane1_out - on_plane0_out;
    let v1_out = on_plane2_out - on_plane0_out;
    let v2_out = off_plane_out - on_plane0_out;

    // Nine unknowns — the 3x3 matrix, read row by row — and nine equations, three per
    // point correspondence.
    let b = Vector::<T, 9>::from_array([
        v0_out.x(), v0_out.y(), v0_out.z(),
        v1_out.x(), v1_out.y(), v1_out.z(),
        v2_out.x(), v2_out.y(), v2_out.z(),
    ]);

    let o = T::ZERO;
    let row = |v: Vector<T, 3>, block: usize| {
        let mut r = [o; 9];
        r[block * 3] = v.x();
        r[block * 3 + 1] = v.y();
        r[block * 3 + 2] = v.z();
        r
    };
    let a = Matrix::<T, 9, 9>::from_rows([
        row(v0_in, 0), row(v0_in, 1), row(v0_in, 2),
        row(v1_in, 0), row(v1_in, 1), row(v1_in, 2),
        row(v2_in, 0), row(v2_in, 1), row(v2_in, 2),
    ]);

    let x = mat::lup_solve(a, b)?;
    let linear = Matrix::<T, 4, 4>::from_rows([
        [x[0], x[1], x[2], o],
        [x[3], x[4], x[5], o],
        [x[6], x[7], x[8], o],
        [o, o, o, T::ONE],
    ]);

    Some(translation_matrix(on_plane0_out) * linear * translation_matrix(-on_plane0_in))
}

/// Returns the affine transform taking three input points onto three output points,
/// preserving handedness.
///
/// The off-plane point each set needs is synthesized from the triangle's own normal, so
/// the transform is the one that also maps the input triangle's facing onto the output's.
pub fn points_transformation_matrix_3<T: Float>(
    on_plane0_in: Vector<T, 3>,
    on_plane1_in: Vector<T, 3>,
    on_plane2_in: Vector<T, 3>,
    on_plane0_out: Vector<T, 3>,
    on_plane1_out: Vector<T, 3>,
    on_plane2_out: Vector<T, 3>,
) -> Option<Matrix<T, 4, 4>> {
    let off_plane_in = on_plane0_in
        + vec::normalize(vec::cross(
            on_plane1_in - on_plane0_in,
            on_plane2_in - on_plane0_in,
        ));
    let off_plane_out = on_plane0_out
        + vec::normalize(vec::cross(
            on_plane1_out - on_plane0_out,
            on_plane2_out - on_plane0_out,
        ));

    points_transformation_matrix(
        on_plane0_in,
        on_plane1_in,
        on_plane2_in,
        off_plane_in,
        on_plane0_out,
        on_plane1_out,
        on_plane2_out,
        off_plane_out,
    )
}
