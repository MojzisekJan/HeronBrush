//! Quadratic Bézier surfaces.
//!

use crate::scalar::Float;
use crate::vec::Vector;

/// Evaluates a quadratic Bézier surface at the given surface coordinates.
///
/// The surface is defined by a 3x3 grid of control points; `u` and `v` run from 0 to 1
/// along the grid's two directions. The result is found by interpolating each row along
/// `u`, then interpolating the three results along `v`.
pub fn evaluate_quadratic_bezier_surface<T: Float, const C: usize>(
    control_points: &[[Vector<T, C>; 3]; 3],
    u: T,
    v: T,
) -> Vector<T, C> {
    let rows = [
        interpolate(u, &control_points[0]),
        interpolate(u, &control_points[1]),
        interpolate(u, &control_points[2]),
    ];
    interpolate(v, &rows)
}

/// Blends three control points with the quadratic Bernstein basis at `x`.
fn interpolate<T: Float, const C: usize>(x: T, p: &[Vector<T, C>; 3]) -> Vector<T, C> {
    let x2 = x * x;
    let b0 = T::ONE - T::TWO * x + x2;
    let b1 = T::TWO * (x - x2);
    let b2 = x2;
    p[0] * b0 + p[1] * b1 + p[2] * b2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_approx_eq;
    use crate::vec::Vec3d;

    /// A flat 3x3 grid spanning the unit square in the XY plane.
    fn flat_grid() -> [[Vec3d; 3]; 3] {
        [
            [
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(0.5, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
            ],
            [
                Vec3d::new(0.0, 0.5, 0.0),
                Vec3d::new(0.5, 0.5, 0.0),
                Vec3d::new(1.0, 0.5, 0.0),
            ],
            [
                Vec3d::new(0.0, 1.0, 0.0),
                Vec3d::new(0.5, 1.0, 0.0),
                Vec3d::new(1.0, 1.0, 0.0),
            ],
        ]
    }

    #[test]
    fn the_corners_interpolate_the_corner_control_points() {
        let grid = flat_grid();
        assert_approx_eq!(
            evaluate_quadratic_bezier_surface(&grid, 0.0, 0.0),
            Vec3d::zero()
        );
        assert_approx_eq!(
            evaluate_quadratic_bezier_surface(&grid, 1.0, 1.0),
            Vec3d::new(1.0, 1.0, 0.0)
        );
        assert_approx_eq!(
            evaluate_quadratic_bezier_surface(&grid, 1.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0)
        );
    }

    #[test]
    fn an_evenly_spaced_flat_grid_stays_flat_and_evenly_spaced() {
        let grid = flat_grid();
        assert_approx_eq!(
            evaluate_quadratic_bezier_surface(&grid, 0.5, 0.5),
            Vec3d::new(0.5, 0.5, 0.0)
        );
    }

    #[test]
    fn a_raised_middle_control_point_lifts_the_surface() {
        let mut grid = flat_grid();
        grid[1][1] = Vec3d::new(0.5, 0.5, 1.0);

        let center = evaluate_quadratic_bezier_surface(&grid, 0.5, 0.5);
        // The surface is pulled towards the raised point but does not reach it: the
        // middle basis function peaks at 0.5, so the centre rises by 0.5 * 0.5.
        assert_approx_eq!(center, Vec3d::new(0.5, 0.5, 0.25));
        // The corners are unaffected.
        assert_approx_eq!(
            evaluate_quadratic_bezier_surface(&grid, 0.0, 0.0),
            Vec3d::zero()
        );
    }
}
