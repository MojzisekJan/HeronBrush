//! Turning a brush face into flat Bézier patches.
//!
//! Converts triangles and quads to patches. Larger polygons are tiled with diagonal-sharing quads
//! and an optional final triangular patch, without gaps or overlaps.

use geometry_math::vec::{self, Vec2f, Vec3d};

use crate::brush_face::BrushFace;
use crate::io::map_reader::{Patch, PatchControlPoint};

/// The default control-point grid size.
pub const DEFAULT_POINTS: usize = 3;

/// Builds the patches covering `polygon`, a face's convex outline wound as the geometry
/// gives it, wearing `face`'s material and UV mapping.
///
/// `point_rows` and `point_columns` are the control-point counts along each axis; both
/// must be odd and at least 3, as a Bézier patch's grid always is. A polygon with fewer
/// than three points, or bad grid counts, makes nothing.
///
/// `texture_size` is in pixels. Brush-face UVs use texels and must be divided by it to produce
/// texture-relative patch UVs.
pub fn create_patches(
    face: &BrushFace,
    polygon: &[Vec3d],
    point_rows: usize,
    point_columns: usize,
    texture_size: Vec2f,
) -> Vec<Patch> {
    let n = polygon.len();
    if n < 3
        || point_rows < 3
        || point_rows.is_multiple_of(2)
        || point_columns < 3
        || point_columns.is_multiple_of(2)
    {
        return Vec::new();
    }

    let v = select_start_vertex(polygon, face.normal());
    let mut patches = Vec::new();

    if n == 3 {
        patches.push(make_patch(face, point_rows, point_columns, [v[1], v[2], v[0], v[0]], texture_size));
        return patches;
    }

    patches.push(make_patch(face, point_rows, point_columns, [v[0], v[1], v[2], v[3]], texture_size));

    // Extend the quad outward by one vertex on each side of its non-boundary diagonal
    // until fewer than two are left.
    let (mut lo, mut hi) = (0usize, 3usize);
    let mut remaining = n - 4;
    while remaining >= 2 {
        let new_lo = (lo + n - 1) % n;
        let new_hi = (hi + 1) % n;
        patches.push(make_patch(
            face,
            point_rows,
            point_columns,
            [v[new_lo], v[lo], v[hi], v[new_hi]],
            texture_size,
        ));
        lo = new_lo;
        hi = new_hi;
        remaining -= 2;
    }

    if remaining == 1 {
        let last = (hi + 1) % n;
        patches.push(make_patch(
            face,
            point_rows,
            point_columns,
            [v[hi], v[last], v[lo], v[lo]],
            texture_size,
        ));
    }

    patches
}

/// One patch spanning the quad `c0 c1 c2 c3` (row 0 col 0, row 0 col last, row last col
/// last, row last col 0), its control points a bilinear grid, each wearing the UV the
/// face's mapping gives its position.
fn make_patch(
    face: &BrushFace,
    rows: usize,
    columns: usize,
    corners: [Vec3d; 4],
    texture_size: Vec2f,
) -> Patch {
    let [c0, c1, c2, c3] = corners;
    let mut control_points = Vec::with_capacity(rows * columns);
    for i in 0..rows {
        let s = i as f64 / (rows - 1) as f64;
        for j in 0..columns {
            let t = j as f64 / (columns - 1) as f64;
            let position = c0 * ((1.0 - s) * (1.0 - t))
                + c1 * ((1.0 - s) * t)
                + c2 * (s * t)
                + c3 * (s * (1.0 - t));
            let uv = face.uv_coords(position);
            control_points.push(PatchControlPoint {
                position,
                uv: Vec2f::new(
                    uv.x() / texture_size.x().max(1.0),
                    uv.y() / texture_size.y().max(1.0),
                ),
            });
        }
    }

    Patch {
        material_name: face.material_name.to_string(),
        // A patch made here is new, so it is given its identity when it reaches the tree.
        id: None,
        rows,
        columns,
        control_points,
        line: 0,
    }
}

/// Reverses the polygon and, for a quad or larger, rotates it so the most symmetric
/// consecutive quad comes first -- `selectStartVertex`.
fn select_start_vertex(polygon: &[Vec3d], normal: Vec3d) -> Vec<Vec3d> {
    let mut vertices: Vec<Vec3d> = polygon.iter().rev().copied().collect();
    let n = vertices.len();
    if n >= 4 {
        let best = (0..n)
            .min_by(|&a, &b| {
                quad_score(&vertices, a, normal)
                    .partial_cmp(&quad_score(&vertices, b, normal))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(0);
        vertices.rotate_left(best);
    }
    vertices
}

/// Scores a candidate quad by angle regularity, then edge regularity; lower scores are more square.
fn quad_score(vertices: &[Vec3d], i: usize, normal: Vec3d) -> (f64, f64) {
    let n = vertices.len();
    let q = [
        vertices[i % n],
        vertices[(i + 1) % n],
        vertices[(i + 2) % n],
        vertices[(i + 3) % n],
    ];

    let angle_variance = geometry_math::scalar::variance([
        corner_angle(q[3], q[0], q[1], normal),
        corner_angle(q[0], q[1], q[2], normal),
        corner_angle(q[1], q[2], q[3], normal),
        corner_angle(q[2], q[3], q[0], normal),
    ])
    .unwrap_or(0.0);

    let edge_variance = geometry_math::scalar::variance([
        vec::length(q[1] - q[0]),
        vec::length(q[2] - q[1]),
        vec::length(q[3] - q[2]),
        vec::length(q[0] - q[3]),
    ])
    .unwrap_or(0.0);

    (angle_variance, edge_variance)
}

/// The interior angle at `b` between the edges to `a` and to `c`, about `normal`.
fn corner_angle(a: Vec3d, b: Vec3d, c: Vec3d, normal: Vec3d) -> f64 {
    let to_a = a - b;
    let to_c = c - b;
    if vec::squared_length(to_a) < f64::EPSILON || vec::squared_length(to_c) < f64::EPSILON {
        return 0.0;
    }
    vec::measure_angle_3d(vec::normalize(to_a), vec::normalize(to_c), normal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes;
    use geometry_math::bbox::Bbox3d;

    /// The top face of a unit cube and its outline.
    fn cube_top() -> (BrushFace, Vec<Vec3d>) {
        let world = Bbox3d::from_scalars(-16384.0, 16384.0);
        let brush = shapes::create_cuboid(
            &world,
            &Bbox3d::new(Vec3d::new(-32.0, -32.0, -32.0), Vec3d::new(32.0, 32.0, 32.0)),
            "stone",
        )
        .expect("a cube");
        let (face, polygon) = brush
            .geometry_faces()
            .find(|(face, _)| face.normal().z() > 0.9)
            .expect("a top face");
        (face.clone(), polygon)
    }

    /// The size the shapes give a face's material, so a UV coordinate can be checked.
    fn size() -> Vec2f {
        Vec2f::new(64.0, 64.0)
    }

    #[test]
    fn a_quad_face_becomes_one_flat_patch() {
        let (face, polygon) = cube_top();
        let patches = create_patches(&face, &polygon, 3, 3, size());

        assert_eq!(patches.len(), 1);
        let patch = &patches[0];
        assert_eq!(patch.rows, 3);
        assert_eq!(patch.columns, 3);
        assert_eq!(patch.control_points.len(), 9);
        assert_eq!(patch.material_name, "stone");
        // Every control point sits on the face's plane (z = 32).
        for point in &patch.control_points {
            assert!((point.position.z() - 32.0).abs() < 1e-6, "{:?}", point.position);
        }
        // The centre control point is the average of the four corners.
        let centre = patch.control_points[4].position;
        assert!(centre.x().abs() < 1e-6 && centre.y().abs() < 1e-6);
    }

    #[test]
    fn a_finer_grid_gives_more_control_points_on_the_same_plane() {
        let (face, polygon) = cube_top();
        let patches = create_patches(&face, &polygon, 5, 5, size());
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].control_points.len(), 25);
        for point in &patches[0].control_points {
            assert!((point.position.z() - 32.0).abs() < 1e-6);
        }
    }

    #[test]
    fn bad_grid_counts_and_short_polygons_make_nothing() {
        let (face, polygon) = cube_top();
        assert!(create_patches(&face, &polygon, 2, 3, size()).is_empty());
        assert!(create_patches(&face, &polygon, 4, 3, size()).is_empty());
        assert!(create_patches(&face, &polygon[..2], 3, 3, size()).is_empty());
    }

    #[test]
    fn the_uv_coordinates_count_copies_of_the_material_not_texels() {
        // The cube's top is 64 units across and the material 64 pixels, so the face wears
        // exactly one copy of it: the patch should span one unit of UV, not sixty-four.
        let (face, polygon) = cube_top();
        let patches = create_patches(&face, &polygon, 3, 3, size());

        let mut widest = 0.0f32;
        for point in &patches[0].control_points {
            widest = widest.max(point.uv.x().abs());
        }
        assert!((widest - 0.5).abs() < 1e-4, "the outer edge sits at 0.5: {widest}");
    }
}
