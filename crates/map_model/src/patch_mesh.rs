//! Turning a Bézier patch into triangles.
//!
//! A patch is a grid of control points, and the surface it describes is made of
//! overlapping 3x3 quadratic Bézier pieces: a 3x3 patch is one piece, a 5x3 is two side by
//! side sharing their middle column, and so on. That is why the format only ever writes
//! odd row and column counts.
//!
//! Nothing here edits a patch. This exists so that a map holding one can be looked at:
//! before it, a patch was read, kept and written back without ever appearing on screen,
//! which makes a map full of them look empty.

use geometry_math::bezier_surface::evaluate_quadratic_bezier_surface;
use geometry_math::vec::{self, Vec2f, Vec3d};

use crate::io::map_reader::Patch;

/// One triangle of a tessellated patch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatchTriangle {
    /// The corners.
    pub positions: [Vec3d; 3],
    /// Where each corner falls on the material, in texels.
    pub uv: [Vec2f; 3],
    /// The way the triangle faces.
    pub normal: Vec3d,
}

/// How finely each 3x3 piece is cut up, by default.
///
/// A patch is a curve; too few steps and it reads as the flat quadrilaterals it is drawn
/// with rather than as the curve it is.
pub const DEFAULT_SUBDIVISIONS: usize = 6;

/// Turns a patch into triangles.
///
/// Each 3x3 piece is cut into `subdivisions` steps along both directions. Each triangle is
/// emitted once, wound the way the control grid runs. A patch is a surface rather than a
/// solid and has to be seen from both sides, but emitting it twice is not the way: a
/// shadow map and a screen-space normal both assume geometry with an inside, and a
/// surface that faces both ways at once shadows and occludes itself everywhere. The
/// renderer draws it without culling and turns the normal towards the viewer instead.
pub fn tessellate(patch: &Patch, subdivisions: usize) -> Vec<PatchTriangle> {
    let steps = subdivisions.max(1);
    let mut result = Vec::new();

    // The format writes odd counts because the pieces overlap by one row and column; a
    // count that is even or too small describes no piece at all.
    if patch.rows < 3 || patch.columns < 3 {
        return result;
    }
    let expected = patch.rows * patch.columns;
    if patch.control_points.len() < expected {
        return result;
    }

    for piece_row in 0..(patch.rows - 1) / 2 {
        for piece_column in 0..(patch.columns - 1) / 2 {
            let at = |r: usize, c: usize| {
                patch.control_points[(piece_row * 2 + r) * patch.columns + piece_column * 2 + c]
            };

            let positions = [
                [at(0, 0).position, at(0, 1).position, at(0, 2).position],
                [at(1, 0).position, at(1, 1).position, at(1, 2).position],
                [at(2, 0).position, at(2, 1).position, at(2, 2).position],
            ];
            // The UV coordinates curve with the surface, so they are interpolated the same
            // way rather than being spread evenly across the piece.
            let uvs = [
                [at(0, 0).uv, at(0, 1).uv, at(0, 2).uv],
                [at(1, 0).uv, at(1, 1).uv, at(1, 2).uv],
                [at(2, 0).uv, at(2, 1).uv, at(2, 2).uv],
            ];

            let point = |u: f64, v: f64| evaluate_quadratic_bezier_surface(&positions, u, v);
            let coord = |u: f64, v: f64| {
                evaluate_quadratic_bezier_surface(&uvs, u as f32, v as f32)
            };

            for row in 0..steps {
                for column in 0..steps {
                    let (u0, u1) = (
                        column as f64 / steps as f64,
                        (column + 1) as f64 / steps as f64,
                    );
                    let (v0, v1) =
                        (row as f64 / steps as f64, (row + 1) as f64 / steps as f64);

                    let corners = [
                        (point(u0, v0), coord(u0, v0)),
                        (point(u1, v0), coord(u1, v0)),
                        (point(u1, v1), coord(u1, v1)),
                        (point(u0, v1), coord(u0, v1)),
                    ];

                    for [a, b, c] in [[0, 1, 2], [0, 2, 3]] {
                        push_triangle(
                            &mut result,
                            [corners[a].0, corners[b].0, corners[c].0],
                            [corners[a].1, corners[b].1, corners[c].1],
                        );
                    }
                }
            }
        }
    }

    result
}

/// Adds a triangle, skipping one with no area.
fn push_triangle(out: &mut Vec<PatchTriangle>, positions: [Vec3d; 3], uv: [Vec2f; 3]) {
    let normal = vec::cross(positions[1] - positions[0], positions[2] - positions[0]);
    let length = vec::length(normal);
    if length <= 0.0 {
        // A step of the grid that came out degenerate: at the tip of a cone, say, where
        // several control points meet.
        return;
    }
    let normal = normal / length;

    out.push(PatchTriangle {
        positions,
        uv,
        normal,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::map_reader::PatchControlPoint;

    /// A patch of the given size, flat in the XY plane.
    fn flat_patch(rows: usize, columns: usize) -> Patch {
        let mut control_points = Vec::new();
        for row in 0..rows {
            for column in 0..columns {
                control_points.push(PatchControlPoint {
                    position: Vec3d::new(column as f64 * 32.0, row as f64 * 32.0, 0.0),
                    uv: Vec2f::new(column as f32, row as f32),
                });
            }
        }
        Patch {
            // A patch built for a test, which nothing merges.
            id: None,
            material_name: "stone".into(),
            rows,
            columns,
            control_points,
            line: 0,
        }
    }

    /// A 3x3 patch bowed upwards in the middle.
    fn curved_patch() -> Patch {
        let mut patch = flat_patch(3, 3);
        patch.control_points[4].position = Vec3d::new(32.0, 32.0, 64.0);
        patch
    }

    #[test]
    fn a_flat_patch_comes_out_flat() {
        let triangles = tessellate(&flat_patch(3, 3), 4);
        assert!(!triangles.is_empty());
        for triangle in &triangles {
            for corner in triangle.positions {
                geometry_math::assert_approx_eq!(corner.z(), 0.0, 1e-9);
            }
        }
    }

    #[test]
    fn each_triangle_is_emitted_once() {
        // A patch is seen from both sides, but not by drawing it twice: a shadow map and
        // a screen-space normal both assume geometry with an inside, and a surface facing
        // both ways at once shadows and occludes itself everywhere.
        let steps = 2;
        let triangles = tessellate(&flat_patch(3, 3), steps);
        assert_eq!(triangles.len(), steps * steps * 2, "two per cell of the grid");

        // All of them face the same way, since the patch is flat.
        for triangle in &triangles {
            geometry_math::assert_approx_eq!(triangle.normal, triangles[0].normal, 1e-9);
        }
    }

    #[test]
    fn a_larger_patch_is_made_of_more_pieces() {
        // 5x3 is two 3x3 pieces sharing their middle column; 5x5 is four.
        let one = tessellate(&flat_patch(3, 3), 2).len();
        let two = tessellate(&flat_patch(3, 5), 2).len();
        let four = tessellate(&flat_patch(5, 5), 2).len();

        assert_eq!(two, one * 2);
        assert_eq!(four, one * 4);
    }

    #[test]
    fn subdividing_further_makes_more_triangles() {
        let coarse = tessellate(&flat_patch(3, 3), 2).len();
        let fine = tessellate(&flat_patch(3, 3), 4).len();
        // Twice the steps in both directions is four times the cells.
        assert_eq!(fine, coarse * 4);
    }

    #[test]
    fn a_curved_patch_bows_but_stays_inside_its_control_points() {
        let triangles = tessellate(&curved_patch(), 6);
        let highest = triangles
            .iter()
            .flat_map(|triangle| triangle.positions)
            .fold(f64::MIN, |highest, corner| highest.max(corner.z()));

        // It rises, but a quadratic Bézier does not reach its middle control point.
        assert!(highest > 1.0, "the patch did not bow: {highest}");
        assert!(highest < 64.0, "the patch overshot its control point: {highest}");
    }

    #[test]
    fn the_corners_of_a_patch_are_its_corner_control_points() {
        // The curve passes through the corners exactly, whatever it does in between.
        let patch = curved_patch();
        let triangles = tessellate(&patch, 4);

        for corner in [0usize, 2, 6, 8] {
            let wanted = patch.control_points[corner].position;
            assert!(
                triangles
                    .iter()
                    .flat_map(|triangle| triangle.positions)
                    .any(|point| vec::distance(point, wanted) < 1e-6),
                "the patch does not reach {wanted:?}"
            );
        }
    }

    #[test]
    fn uv_coordinates_come_along_with_the_surface() {
        let triangles = tessellate(&flat_patch(3, 3), 3);
        let widest = triangles
            .iter()
            .flat_map(|triangle| triangle.uv)
            .fold(0.0f32, |widest, uv| widest.max(uv.x()));
        // The control points run from 0 to 2 across, and the surface reaches both ends.
        geometry_math::assert_approx_eq!(widest, 2.0, 1e-5);
    }

    #[test]
    fn a_patch_too_small_to_describe_a_surface_makes_nothing() {
        assert!(tessellate(&flat_patch(1, 3), 4).is_empty());
        assert!(tessellate(&flat_patch(3, 1), 4).is_empty());
    }

    #[test]
    fn a_patch_with_missing_control_points_makes_nothing() {
        // A truncated file should not take the editor down with it.
        let mut patch = flat_patch(3, 3);
        patch.control_points.truncate(4);
        assert!(tessellate(&patch, 4).is_empty());
    }
}
