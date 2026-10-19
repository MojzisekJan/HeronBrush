//! Tests for mat behavior.

#![allow(clippy::excessive_precision)]

use std::cmp::Ordering;

use geometry_math::assert_approx_eq;
use geometry_math::mat::{self, Determinant, Mat2x2d, Mat3x3d, Mat4x4d, Mat4x4f, Matrix};
use geometry_math::vec::{Vec3d, Vec4d};

/// The 4x4 matrix containing 1 through 16 in row-major order.
fn m1234() -> Mat4x4d {
    Mat4x4d::from_rows([
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 8.0],
        [9.0, 10.0, 11.0, 12.0],
        [13.0, 14.0, 15.0, 16.0],
    ])
}

#[test]
fn constructor_default_is_the_identity() {
    assert_eq!(Mat4x4d::default(), Mat4x4d::identity());
}

#[test]
fn from_rows_fills_in_row_major_order() {
    let m = Mat3x3d::from_rows([
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0],
        [7.0, 8.0, 9.0],
    ]);
    // `m[c][r]` selects column c, row r.
    assert_eq!(m[0][0], 1.0);
    assert_eq!(m[1][0], 2.0);
    assert_eq!(m[2][0], 3.0);
    assert_eq!(m[0][1], 4.0);
    assert_eq!(m[1][1], 5.0);
    assert_eq!(m[2][1], 6.0);
    assert_eq!(m[0][2], 7.0);
    assert_eq!(m[1][2], 8.0);
    assert_eq!(m[2][2], 9.0);

    let m: Matrix<f64, 4, 3> = Matrix::from_rows([
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0],
        [7.0, 8.0, 9.0],
        [10.0, 11.0, 12.0],
    ]);
    assert_eq!(m[0][3], 10.0);
    assert_eq!(m[1][3], 11.0);
    assert_eq!(m[2][3], 12.0);
}

#[test]
fn from_columns_fills_in_column_major_order() {
    let by_rows = Mat3x3d::from_rows([
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0],
        [7.0, 8.0, 9.0],
    ]);
    let by_columns = Mat3x3d::from_columns([
        [1.0, 4.0, 7.0],
        [2.0, 5.0, 8.0],
        [3.0, 6.0, 9.0],
    ]);
    assert_eq!(by_rows, by_columns);
}

#[test]
fn fill() {
    assert_eq!(Mat4x4f::fill(1.0), Mat4x4f::from_rows([[1.0; 4]; 4]));
    assert_eq!(Mat4x4f::fill(-2.0), Mat4x4f::from_rows([[-2.0; 4]; 4]));
}

#[test]
fn identity() {
    assert_eq!(
        Mat4x4f::identity(),
        Mat4x4f::from_rows([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    );
}

#[test]
fn zero() {
    assert_eq!(Mat4x4f::zero(), Mat4x4f::from_rows([[0.0; 4]; 4]));
}

#[test]
fn rotations_by_90_degrees() {
    let x = Vec4d::new(1.0, 0.0, 0.0, 0.0);
    let y = Vec4d::new(0.0, 1.0, 0.0, 0.0);

    assert_approx_eq!(Mat4x4d::rot_90_x_cw() * y, Vec4d::new(0.0, 0.0, -1.0, 0.0));
    assert_approx_eq!(Mat4x4d::rot_90_y_cw() * x, Vec4d::new(0.0, 0.0, 1.0, 0.0));
    assert_approx_eq!(Mat4x4d::rot_90_z_cw() * y, Vec4d::new(1.0, 0.0, 0.0, 0.0));

    assert_approx_eq!(Mat4x4d::rot_90_x_ccw() * y, Vec4d::new(0.0, 0.0, 1.0, 0.0));
    assert_approx_eq!(Mat4x4d::rot_90_y_ccw() * x, Vec4d::new(0.0, 0.0, -1.0, 0.0));
    assert_approx_eq!(Mat4x4d::rot_90_z_ccw() * x, Vec4d::new(0.0, 1.0, 0.0, 0.0));
}

#[test]
fn rotations_by_180_degrees() {
    let x = Vec4d::new(1.0, 0.0, 0.0, 0.0);
    let y = Vec4d::new(0.0, 1.0, 0.0, 0.0);

    assert_approx_eq!(Mat4x4d::rot_180_x() * y, Vec4d::new(0.0, -1.0, 0.0, 0.0));
    assert_approx_eq!(Mat4x4d::rot_180_y() * x, Vec4d::new(-1.0, 0.0, 0.0, 0.0));
    assert_approx_eq!(Mat4x4d::rot_180_z() * y, Vec4d::new(0.0, -1.0, 0.0, 0.0));
}

#[test]
fn mirrors() {
    let v = Vec4d::new(1.0, 1.0, 1.0, 0.0);
    assert_approx_eq!(Mat4x4d::mirror_x() * v, Vec4d::new(-1.0, 1.0, 1.0, 0.0));
    assert_approx_eq!(Mat4x4d::mirror_y() * v, Vec4d::new(1.0, -1.0, 1.0, 0.0));
    assert_approx_eq!(Mat4x4d::mirror_z() * v, Vec4d::new(1.0, 1.0, -1.0, 0.0));
}

#[test]
fn zero_out() {
    let v = Vec4d::new(1.0, 1.0, 1.0, 1.0);
    assert_approx_eq!(Mat4x4d::zero_out(0) * v, Vec4d::new(0.0, 1.0, 1.0, 1.0));
    assert_approx_eq!(Mat4x4d::zero_out(1) * v, Vec4d::new(1.0, 0.0, 1.0, 1.0));
    assert_approx_eq!(Mat4x4d::zero_out(2) * v, Vec4d::new(1.0, 1.0, 0.0, 1.0));
}

#[test]
fn operator_subscript_returns_columns() {
    let m = m1234();
    for c in 0..4 {
        for r in 0..4 {
            assert_eq!(m[c][r], m.v[c].v[r]);
            assert_eq!(m.at(r, c), m.v[c].v[r]);
        }
    }
}

#[test]
fn row_returns_a_row() {
    assert_eq!(m1234().row(1), geometry_math::Vector::from_array([5.0, 6.0, 7.0, 8.0]));
}

#[test]
fn compare() {
    let a = Mat4x4d::from_rows([[1.0, 2.0, 3.0, 4.0]; 4]);
    let b = Mat4x4d::from_rows([[1.0, 2.0, 3.0, 4.0]; 4]);
    assert_eq!(mat::compare(&a, &b, 0.0), Ordering::Equal);

    let greater = a.set(0, 0, 2.0);
    assert_eq!(mat::compare(&a, &greater, 0.0), Ordering::Less);
    assert_eq!(mat::compare(&greater, &a, 0.0), Ordering::Greater);

    // Differences within epsilon compare equal.
    assert_eq!(mat::compare(&a, &a.set(0, 0, 1.05), 0.1), Ordering::Equal);
}

#[test]
fn is_equal() {
    let a = m1234();
    assert!(mat::is_equal(&a, &a, 0.0));
    assert!(mat::is_equal(&a, &a.set(2, 3, 12.05), 0.1));
    assert!(!mat::is_equal(&a, &a.set(2, 3, 12.5), 0.1));
}

#[test]
fn is_zero() {
    assert!(mat::is_zero(&Mat4x4d::zero(), 0.0));
    assert!(!mat::is_zero(&Mat4x4d::identity(), 0.0));
    assert!(mat::is_zero(&Mat4x4d::fill(0.05), 0.1));
}

#[test]
fn equality_operators() {
    assert!(m1234() == m1234());
    assert!(m1234() != m1234().set(0, 0, 0.0));
}

#[test]
fn operator_unary_minus() {
    assert_eq!(-m1234(), m1234() * -1.0);
}

#[test]
fn operator_binary_plus_and_minus() {
    let m = m1234();
    assert_eq!(m + m, m * 2.0);
    assert_eq!(m - m, Mat4x4d::zero());
}

#[test]
fn operator_multiply_matrix() {
    let lhs = m1234();
    let rhs = Mat4x4d::from_rows([
        [2.0, 2.0, 3.0, 4.0],
        [5.0, 8.0, 7.0, 8.0],
        [9.0, 11.0, 11.0, 12.0],
        [13.0, 14.0, 15.0, 16.0],
    ]);
    let expected = Mat4x4d::from_rows([
        [91.0, 107.0, 110.0, 120.0],
        [207.0, 247.0, 254.0, 280.0],
        [323.0, 387.0, 398.0, 440.0],
        [439.0, 527.0, 542.0, 600.0],
    ]);
    assert_eq!(lhs * rhs, expected);

    // A non-square product, checking the dimensions line up.
    let lhs: Matrix<f64, 4, 3> = Matrix::from_rows([
        [1.0, 2.0, 3.0],
        [5.0, 6.0, 7.0],
        [9.0, 10.0, 11.0],
        [13.0, 14.0, 15.0],
    ]);
    let rhs: Matrix<f64, 3, 2> =
        Matrix::from_rows([[2.0, 2.0], [5.0, 8.0], [9.0, 11.0]]);
    let expected: Matrix<f64, 4, 2> = Matrix::from_rows([
        [39.0, 51.0],
        [103.0, 135.0],
        [167.0, 219.0],
        [231.0, 303.0],
    ]);
    assert_eq!(lhs * rhs, expected);
}

#[test]
fn operator_multiply_scalar() {
    let tripled = Mat4x4d::from_rows([
        [3.0, 6.0, 9.0, 12.0],
        [15.0, 18.0, 21.0, 24.0],
        [27.0, 30.0, 33.0, 36.0],
        [39.0, 42.0, 45.0, 48.0],
    ]);
    assert_eq!(m1234() * 3.0, tripled);
    assert_eq!(3.0 * m1234(), tripled);
    assert_eq!(tripled / 3.0, m1234());
}

#[test]
fn operator_multiply_vector_right() {
    let v = Vec4d::new(1.0, 2.0, 3.0, 1.0);
    assert_approx_eq!(Mat4x4d::identity() * v, v);

    let expected = Vec4d::new(18.0, 46.0, 74.0, 102.0);
    assert_eq!(m1234() * v, expected);

    // A 4x4 times a 3D point goes through homogeneous coordinates.
    assert_eq!(m1234() * v.xyz(), expected.to_cartesian_coords());
}

#[test]
fn operator_multiply_vector_left() {
    let v = Vec4d::new(1.0, 2.0, 3.0, 1.0);
    assert_approx_eq!(mat::row_vector_mul(v, Mat4x4d::identity()), v);

    let expected = Vec4d::new(51.0, 58.0, 65.0, 72.0);
    assert_eq!(mat::row_vector_mul(v, m1234()), expected);
}

#[test]
fn set() {
    assert_eq!(m1234().set(0, 0, 0.0).at(0, 0), 0.0);
    assert_eq!(m1234().set(0, 2, 0.0).at(0, 2), 0.0);
    assert_eq!(m1234().set(3, 2, 0.0).at(3, 2), 0.0);
    // Every other element is untouched.
    assert_eq!(m1234().set(3, 2, 0.0).at(3, 3), 16.0);
}

#[test]
fn transpose() {
    assert_eq!(
        m1234().transpose(),
        Mat4x4d::from_rows([
            [1.0, 5.0, 9.0, 13.0],
            [2.0, 6.0, 10.0, 14.0],
            [3.0, 7.0, 11.0, 15.0],
            [4.0, 8.0, 12.0, 16.0],
        ])
    );
    assert_eq!(m1234().transpose().transpose(), m1234());
}

#[test]
fn slice() {
    let m = m1234();
    assert_eq!(
        m.slice::<2, 2>(1, 1),
        Mat2x2d::from_rows([[6.0, 7.0], [10.0, 11.0]])
    );
    assert_eq!(
        m.slice::<2, 2>(2, 1),
        Mat2x2d::from_rows([[10.0, 11.0], [14.0, 15.0]])
    );
    assert_eq!(
        m.slice::<2, 2>(2, 2),
        Mat2x2d::from_rows([[11.0, 12.0], [15.0, 16.0]])
    );
}

#[test]
fn extract_minor() {
    let m = m1234();
    let minor = |r, c| -> Mat3x3d { mat::extract_minor(&m, r, c) };

    assert_approx_eq!(
        minor(0, 0),
        Mat3x3d::from_rows([[6.0, 7.0, 8.0], [10.0, 11.0, 12.0], [14.0, 15.0, 16.0]])
    );
    assert_approx_eq!(
        minor(3, 3),
        Mat3x3d::from_rows([[1.0, 2.0, 3.0], [5.0, 6.0, 7.0], [9.0, 10.0, 11.0]])
    );
    assert_approx_eq!(
        minor(1, 2),
        Mat3x3d::from_rows([[1.0, 2.0, 4.0], [9.0, 10.0, 12.0], [13.0, 14.0, 16.0]])
    );
    assert_approx_eq!(
        minor(2, 1),
        Mat3x3d::from_rows([[1.0, 3.0, 4.0], [5.0, 7.0, 8.0], [13.0, 15.0, 16.0]])
    );
}

#[test]
fn determinant() {
    let m2 = Mat4x4d::from_rows([
        [65.0, 12.0, -3.0, -5.0],
        [-5.0, 1.0, 0.0, 0.0],
        [19.0, 10.0, 11.0, 8.0],
        [0.0, 1.0, -8.0, 3.0],
    ]);
    let m3 = Mat4x4d::from_rows([
        [3.0, 2.0, -1.0, 4.0],
        [2.0, 1.0, 5.0, 7.0],
        [0.0, 5.0, 2.0, -6.0],
        [-1.0, 2.0, 1.0, 0.0],
    ]);

    assert_approx_eq!(Mat4x4d::zero().determinant(), 0.0);
    assert_approx_eq!(Mat4x4d::identity().determinant(), 1.0);
    assert_approx_eq!(m1234().determinant(), 0.0);
    assert_approx_eq!(m2.determinant(), 15661.0);
    assert_approx_eq!(m3.determinant(), -418.0);
}

#[test]
fn determinant_of_smaller_matrices() {
    assert_eq!(Matrix::<f64, 1, 1>::from_rows([[7.0]]).determinant(), 7.0);
    assert_eq!(
        Mat2x2d::from_rows([[1.0, 2.0], [3.0, 4.0]]).determinant(),
        -2.0
    );
    assert_eq!(
        Mat3x3d::from_rows([[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]])
            .determinant(),
        24.0
    );
}

#[test]
fn adjugate() {
    let m2 = Mat4x4d::from_rows([
        [65.0, 12.0, -3.0, -5.0],
        [-5.0, 1.0, 0.0, 0.0],
        [19.0, 10.0, 11.0, 8.0],
        [0.0, 1.0, -8.0, 3.0],
    ]);
    let r2 = Mat4x4d::from_rows([
        [97.0, -1685.0, 49.0, 31.0],
        [485.0, 7236.0, 245.0, 155.0],
        [-167.0, -651.0, 400.0, -1345.0],
        [-607.0, -4148.0, 985.0, 1582.0],
    ]);
    let m3 = Mat4x4d::from_rows([
        [3.0, 2.0, -1.0, 4.0],
        [2.0, 1.0, 5.0, 7.0],
        [0.0, 5.0, 2.0, -6.0],
        [-1.0, 2.0, 1.0, 0.0],
    ]);
    let r3 = Mat4x4d::from_rows([
        [-47.0, -28.0, -64.0, 221.0],
        [-56.0, 20.0, -14.0, -128.0],
        [65.0, -68.0, -36.0, 59.0],
        [-25.0, -6.0, 46.0, -87.0],
    ]);

    assert_approx_eq!(Mat4x4d::identity().adjugate(), Mat4x4d::identity());
    assert_approx_eq!(Mat4x4d::zero().adjugate(), Mat4x4d::zero());
    assert_approx_eq!(m1234().adjugate(), Mat4x4d::zero());
    assert_approx_eq!(m2.adjugate(), r2);
    assert_approx_eq!(m3.adjugate(), r3);
}

#[test]
fn invert() {
    let m2 = Mat4x4d::from_rows([
        [65.0, 12.0, -3.0, -5.0],
        [-5.0, 1.0, 0.0, 0.0],
        [19.0, 10.0, 11.0, 8.0],
        [0.0, 1.0, -8.0, 3.0],
    ]);
    let r2 = Mat4x4d::from_rows([
        [0.0061937296468936, -0.10759210778367, 0.0031287912649256, 0.0019794393716876],
        [0.030968648234468, 0.46203946108167, 0.015643956324628, 0.0098971968584382],
        [-0.01066343145393, -0.04156822680544, 0.025541153183066, -0.08588212757806],
        [-0.038758699955303, -0.2648617585084, 0.062895089713301, 0.10101526083903],
    ]);
    let m3 = Mat4x4d::from_rows([
        [0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 128.0],
        [-1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let m4 = Mat4x4d::from_rows([
        [0.0, 0.0, -1.0, 0.0],
        [-1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, -128.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    assert_eq!(mat::invert(Mat4x4d::identity()), Some(Mat4x4d::identity()));
    assert_approx_eq!(mat::invert(m2), Some(r2));
    assert_approx_eq!(mat::invert(m3), Some(m4));
    assert_eq!(mat::invert(Mat4x4d::zero()), None);
    assert_eq!(mat::invert(m1234()), None);
}

#[test]
fn invert_round_trips() {
    let m = Mat4x4d::from_rows([
        [3.0, 2.0, -1.0, 4.0],
        [2.0, 1.0, 5.0, 7.0],
        [0.0, 5.0, 2.0, -6.0],
        [-1.0, 2.0, 1.0, 0.0],
    ]);
    let inverse = mat::invert(m).expect("the matrix is invertible");
    assert_approx_eq!(m * inverse, Mat4x4d::identity());
    assert_approx_eq!(inverse * m, Mat4x4d::identity());
}

#[test]
fn lup_solve() {
    let a = Mat4x4d::from_rows([
        [0.93629336358419923, -0.27509584731824366, 0.21835066314633442, 87.954817941228995],
        [0.28962947762551555, 0.95642508584923236, -0.03695701352462509, 120.90975499501228],
        [-0.19866933079506122, -0.09784339500725571, 0.97517032720181584, 87.434439141401043],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let x = Vec4d::new(20.0, -60.0, 32.0, 1.0);
    let b = a * x;

    let solved = mat::lup_solve(a, b).expect("the system has a solution");
    assert_approx_eq!(solved, x);
    assert_approx_eq!(a * solved, b);
}

#[test]
fn lup_solve_reports_a_singular_system() {
    assert_eq!(mat::lup_solve(Mat4x4d::zero(), Vec4d::one()), None);
}

#[test]
fn a_transform_and_its_inverse_round_trip_a_point() {
    // Translate by (10, 20, 30) as an affine 4x4, the shape every editor transform takes.
    let translate = Mat4x4d::identity()
        .set(0, 3, 10.0)
        .set(1, 3, 20.0)
        .set(2, 3, 30.0);
    let point = Vec3d::new(1.0, 2.0, 3.0);

    assert_approx_eq!(translate * point, Vec3d::new(11.0, 22.0, 33.0));

    let inverse = mat::invert(translate).expect("a translation is invertible");
    assert_approx_eq!(inverse * (translate * point), point);
}
