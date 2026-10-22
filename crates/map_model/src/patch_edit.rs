//! Changing a Bézier patch's control-point grid.
//!
//! A patch's row and column counts are always odd, because the surface is a strip of
//! quadratic pieces that overlap by one point (3 points is one piece, 5 is two, 7 is
//! three). Adding detail splits every piece in two -- 3 becomes 5, 5 becomes 9 -- at the
//! midpoint of each Bézier segment, so the surface keeps its exact shape and only the
//! grid gets finer. Removing detail drops every other line, which does change the shape,
//! the way decimating any curve does.
//!

use geometry_math::vec::Vec2f;

use crate::io::map_reader::{Patch, PatchControlPoint};

/// The most control points a patch may have along one axis after adding detail.
pub const MAX_SIZE: usize = 17;

/// Whether the patch can take another round of detail on its columns.
pub fn can_add_columns(patch: &Patch) -> bool {
    patch.columns >= 3 && patch.columns * 2 - 1 <= MAX_SIZE
}

/// Whether the patch can take another round of detail on its rows.
pub fn can_add_rows(patch: &Patch) -> bool {
    patch.rows >= 3 && patch.rows * 2 - 1 <= MAX_SIZE
}

/// Whether the patch has columns to spare.
pub fn can_remove_columns(patch: &Patch) -> bool {
    patch.columns >= 5 && patch.columns % 2 == 1
}

/// Whether the patch has rows to spare.
pub fn can_remove_rows(patch: &Patch) -> bool {
    patch.rows >= 5 && patch.rows % 2 == 1
}

/// Splits every Bézier segment along the columns, keeping the surface exactly.
pub fn add_columns(patch: &Patch) -> Option<Patch> {
    if !can_add_columns(patch) {
        return None;
    }
    let grid = rows_of(patch);
    let refined: Vec<Vec<PatchControlPoint>> = grid.iter().map(|row| subdivide(row)).collect();
    Some(from_rows(patch, refined))
}

/// Splits every Bézier segment along the rows, keeping the surface exactly.
pub fn add_rows(patch: &Patch) -> Option<Patch> {
    if !can_add_rows(patch) {
        return None;
    }
    let columns = columns_of(patch);
    let refined: Vec<Vec<PatchControlPoint>> = columns.iter().map(|col| subdivide(col)).collect();
    Some(from_columns(patch, refined))
}

/// Drops every other column.
pub fn remove_columns(patch: &Patch) -> Option<Patch> {
    if !can_remove_columns(patch) {
        return None;
    }
    let refined: Vec<Vec<PatchControlPoint>> = rows_of(patch)
        .iter()
        .map(|row| row.iter().step_by(2).copied().collect())
        .collect();
    Some(from_rows(patch, refined))
}

/// Drops every other row.
pub fn remove_rows(patch: &Patch) -> Option<Patch> {
    if !can_remove_rows(patch) {
        return None;
    }
    let refined: Vec<Vec<PatchControlPoint>> = columns_of(patch)
        .iter()
        .map(|col| col.iter().step_by(2).copied().collect())
        .collect();
    Some(from_columns(patch, refined))
}

/// The control points laid out row by row.
fn rows_of(patch: &Patch) -> Vec<Vec<PatchControlPoint>> {
    patch
        .control_points
        .chunks(patch.columns)
        .map(<[_]>::to_vec)
        .collect()
}

/// The control points laid out column by column.
fn columns_of(patch: &Patch) -> Vec<Vec<PatchControlPoint>> {
    (0..patch.columns)
        .map(|c| {
            (0..patch.rows)
                .map(|r| patch.control_points[r * patch.columns + c])
                .collect()
        })
        .collect()
}

/// Rebuilds a patch from rows of control points.
fn from_rows(patch: &Patch, rows: Vec<Vec<PatchControlPoint>>) -> Patch {
    let columns = rows.first().map_or(0, Vec::len);
    Patch {
        material_name: patch.material_name.clone(),
        rows: rows.len(),
        columns,
        control_points: rows.into_iter().flatten().collect(),
        // Still the same patch, with a different grid: it keeps who it is.
        id: patch.id,
        line: patch.line,
    }
}

/// Rebuilds a patch from columns of control points.
fn from_columns(patch: &Patch, columns: Vec<Vec<PatchControlPoint>>) -> Patch {
    let rows = columns.first().map_or(0, Vec::len);
    let column_count = columns.len();
    let mut control_points = Vec::with_capacity(rows * column_count);
    for r in 0..rows {
        for column in &columns {
            control_points.push(column[r]);
        }
    }
    Patch {
        material_name: patch.material_name.clone(),
        rows,
        columns: column_count,
        control_points,
        id: patch.id,
        line: patch.line,
    }
}

/// Splits every quadratic Bézier segment of a strip at its midpoint.
///
/// A strip is `2n+1` points -- `n` segments `[0,1,2] [2,3,4] …` sharing endpoints. Each
/// segment `[a, b, c]` becomes `[a, (a+b)/2, (a+2b+c)/4, (b+c)/2, c]`, which is the same
/// curve with a point added in the middle. The result is `4n+1` points.
fn subdivide(strip: &[PatchControlPoint]) -> Vec<PatchControlPoint> {
    if strip.len() < 3 {
        return strip.to_vec();
    }
    let mut out = Vec::with_capacity(strip.len() * 2 - 1);
    let mut i = 0;
    while i + 2 < strip.len() {
        let (a, b, c) = (strip[i], strip[i + 1], strip[i + 2]);
        let ab = midpoint(a, b);
        let bc = midpoint(b, c);
        let mid = midpoint(ab, bc);
        if i == 0 {
            out.push(a);
        }
        out.push(ab);
        out.push(mid);
        out.push(bc);
        out.push(c);
        i += 2;
    }
    out
}

/// The point halfway between two control points, position and UV both.
fn midpoint(a: PatchControlPoint, b: PatchControlPoint) -> PatchControlPoint {
    PatchControlPoint {
        position: (a.position + b.position) * 0.5,
        uv: Vec2f::new((a.uv.x() + b.uv.x()) * 0.5, (a.uv.y() + b.uv.y()) * 0.5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::vec::Vec3d;

    /// A flat 3x3 patch across a 64-unit square, its UV running 0..1.
    fn flat_patch() -> Patch {
        let mut control_points = Vec::new();
        for r in 0..3 {
            for c in 0..3 {
                control_points.push(PatchControlPoint {
                    position: Vec3d::new(c as f64 * 32.0, r as f64 * 32.0, 0.0),
                    uv: Vec2f::new(c as f32 * 0.5, r as f32 * 0.5),
                });
            }
        }
        Patch {
            // A patch built for a test, which nothing merges.
            id: None,
            material_name: "stone".into(),
            rows: 3,
            columns: 3,
            control_points,
            line: 1,
        }
    }

    /// A patch bent into an arch, so subdividing has a curve to preserve.
    fn arch_patch() -> Patch {
        let mut patch = flat_patch();
        // Lift the middle column.
        for r in 0..3 {
            patch.control_points[r * 3 + 1].position.v[2] = 48.0;
        }
        patch
    }

    #[test]
    fn adding_columns_makes_the_grid_five_wide() {
        let wider = add_columns(&flat_patch()).expect("it can grow");
        assert_eq!((wider.rows, wider.columns), (3, 5));
        assert_eq!(wider.control_points.len(), 15);
    }

    #[test]
    fn adding_rows_makes_the_grid_five_tall() {
        let taller = add_rows(&flat_patch()).expect("it can grow");
        assert_eq!((taller.rows, taller.columns), (5, 3));
    }

    #[test]
    fn subdividing_keeps_the_surface_where_it_was() {
        let patch = arch_patch();
        let refined = add_columns(&patch).expect("it grows");

        // Both describe the same Bézier surface, so a point sampled off each matches.
        for (u, v) in [(0.25, 0.5), (0.5, 0.5), (0.75, 0.25)] {
            let a = crate::patch_mesh::tessellate(&patch, 8);
            let b = crate::patch_mesh::tessellate(&refined, 8);
            // The tessellations are of the same shape; their bounds agree.
            let bounds = |tris: &[crate::patch_mesh::PatchTriangle]| {
                let mut lo = Vec3d::new(f64::MAX, f64::MAX, f64::MAX);
                let mut hi = Vec3d::new(f64::MIN, f64::MIN, f64::MIN);
                for t in tris {
                    for p in t.positions {
                        for i in 0..3 {
                            lo.v[i] = lo.v[i].min(p.v[i]);
                            hi.v[i] = hi.v[i].max(p.v[i]);
                        }
                    }
                }
                (lo, hi)
            };
            let (alo, ahi) = bounds(&a);
            let (blo, bhi) = bounds(&b);
            for i in 0..3 {
                assert!((alo.v[i] - blo.v[i]).abs() < 0.01, "{u},{v}");
                assert!((ahi.v[i] - bhi.v[i]).abs() < 0.01, "{u},{v}");
            }
        }
    }

    #[test]
    fn removing_a_column_pair_reverses_the_count() {
        let wide = add_columns(&add_columns(&flat_patch()).unwrap()).unwrap();
        assert_eq!(wide.columns, 9);
        let back = remove_columns(&wide).expect("it shrinks");
        assert_eq!(back.columns, 5);
    }

    #[test]
    fn a_patch_will_not_grow_past_the_cap_or_shrink_past_three() {
        let mut patch = flat_patch();
        while let Some(bigger) = add_columns(&patch) {
            patch = bigger;
        }
        assert!(patch.columns <= MAX_SIZE);

        let mut patch = flat_patch();
        assert!(remove_columns(&patch).is_none(), "3 columns is the floor");
        patch = add_columns(&patch).unwrap();
        while let Some(smaller) = remove_columns(&patch) {
            patch = smaller;
        }
        assert_eq!(patch.columns, 3);
    }
}
