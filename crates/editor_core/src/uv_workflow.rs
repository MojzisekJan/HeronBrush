//! UV transforms, face-plane distortion and texel density shared by UV controls.
use crate::document::Document;
use geometry_math::vec::{self, Vec2f, Vec3d};
use map_model::brush_face::BrushFace;

/// Which UV features a transform snaps to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SnapTarget {
    Grid,
    #[default]
    Vertex,
    Pixel,
}
/// How the editing grid is displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GridMode {
    #[default]
    Dynamic,
    Fixed,
    Pixel,
}
/// The anchor of a rotation or scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PivotMode {
    #[default]
    Center,
    Cursor,
    Origin,
    Individual,
}
/// UV display and transform options. These do not change the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub snap: bool,
    pub snap_target: SnapTarget,
    pub rotation_step: f32,
    pub grid: GridMode,
    pub show_grid: bool,
    pub show_outline: bool,
    pub opacity: f64,
    pub stretch: bool,
    pub area_stretch: bool,
    pub density: f64,
    pub pivot: PivotMode,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            snap: true,
            snap_target: SnapTarget::Vertex,
            rotation_step: 15.0,
            grid: GridMode::Dynamic,
            show_grid: true,
            show_outline: true,
            opacity: 0.9,
            stretch: false,
            area_stretch: false,
            density: 1.0,
            pivot: PivotMode::Center,
        }
    }
}
/// Interactive UV transform selected by G, R or S.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformKind {
    Move,
    Rotate,
    Scale,
}
/// A change to the texture mapping, expressed as a delta or scale ratio.
#[derive(Clone, Copy, Debug)]
pub enum Transform {
    Offset(Vec2f),
    Rotate(f32),
    Scale(Vec2f),
}
/// Measurements in texture pixels per world unit and relative directional stretch.
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub density: f64,
    pub stretch: f64,
}

/// Measures the mapping on the physical face plane, including free axes and shear.
pub fn metrics(face: &BrushFace) -> Option<Metrics> {
    let normal = face.normal();
    let (u, v) = face.uv_coord_system.axes(normal, &face.uv_attributes);
    let u = (u - normal * vec::dot(u, normal))
        / f64::from(map_model::uv::safe_scale(face.uv_attributes.scale.x()));
    let v = (v - normal * vec::dot(v, normal))
        / f64::from(map_model::uv::safe_scale(face.uv_attributes.scale.y()));
    let a = vec::dot(u, u);
    let b = vec::dot(u, v);
    let c = vec::dot(v, v);
    let determinant = a * c - b * b;
    if !determinant.is_finite() || determinant <= 1e-20 {
        return None;
    }
    let discriminant = ((a - c) * (a - c) + 4.0 * b * b).sqrt();
    let maximum = (a + c + discriminant) * 0.5;
    // Derive the smaller eigenvalue through the determinant to avoid cancellation.
    let minimum = determinant / maximum;
    Some(Metrics {
        density: determinant.sqrt().sqrt(),
        stretch: (maximum / minimum).sqrt().max(1.0),
    })
}
/// UV centroid of a physical face polygon.
pub fn center(face: &BrushFace, polygon: &[Vec3d]) -> Vec2f {
    if polygon.is_empty() {
        return Vec2f::zero();
    }
    polygon
        .iter()
        .map(|p| face.uv_coords(*p))
        .fold(Vec2f::zero(), |a, b| a + b)
        / polygon.len() as f32
}
/// Applies a transform to the selected faces, with an optional independent pivot.
pub fn transform(document: &mut Document, change: Transform, pivot: PivotMode, origin: Vec2f) {
    if let Transform::Offset(delta) = change {
        if delta.x().is_finite() && delta.y().is_finite() {
            document.edit_faces(|f| f.move_uv(delta));
        }
        return;
    }
    document.edit_faces_with_polygon(|face, polygon| {
        let anchor = if pivot == PivotMode::Individual {
            center(face, polygon)
        } else {
            origin
        };
        match change {
            Transform::Rotate(degrees) if degrees.is_finite() => {
                face.rotate_uv_about(anchor, degrees)
            }
            Transform::Scale(factor)
                if factor.x().is_finite()
                    && factor.y().is_finite()
                    && factor.x().abs() > 1e-6
                    && factor.y().abs() > 1e-6 =>
            {
                face.scale_uv_about(anchor, factor)
            }
            _ => {}
        }
    });
}
/// Gives all selected faces the requested density while preserving each face's center.
/// One call is one history edit, including mixed materials and brush selections.
pub fn set_density(document: &mut Document, target: f64) -> bool {
    if !target.is_finite() || !(1e-4..=1e4).contains(&target) {
        return false;
    }
    let before = document.revision();
    document.edit_faces_with_polygon(|face, polygon| {
        if let Some(current) = metrics(face) {
            if (current.density - target).abs() <= target * 1e-6 {
                return;
            }
            let factor = (current.density / target) as f32;
            if factor.is_finite() && factor > 1e-6 {
                face.scale_uv_about(center(face, polygon), Vec2f::new(factor, factor));
            }
        }
    });
    document.revision() != before
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Document {
        let mut d = Document::new(Default::default());
        for material in ["a", "b"] {
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
                material,
            )
            .unwrap();
            d.add_brushes(vec![brush]);
        }
        d.select_all();
        d.edit_faces(|f| {
            f.uv_attributes.scale = if f.material_name == "a" {
                Vec2f::new(-0.25, 0.5)
            } else {
                Vec2f::new(1.0, 2.0)
            };
            f.uv_attributes.offset = Vec2f::new(13.0, -29.0);
            f.uv_attributes.rotation = 23.0;
        });
        d
    }
    fn centers(d: &Document) -> Vec<Vec2f> {
        map_model::node::brushes_with_ids(d.tree_world())
            .into_iter()
            .flat_map(|(_, brush)| {
                (0..brush.geometry().face_count()).map(move |i| {
                    let polygon = brush
                        .geometry()
                        .face_positions(map_model::polyhedron::FaceId(i as u32));
                    center(
                        &brush.faces()[brush.source_face_index(i).unwrap()],
                        &polygon,
                    )
                })
            })
            .collect()
    }
    #[test]
    fn density_matches_mixed_faces_preserves_centers_mirroring_and_has_one_undo() {
        let mut d = document();
        let before = d.to_map_text();
        let anchors = centers(&d);
        assert!(set_density(&mut d, 2.0));
        for (_, brush) in map_model::node::brushes_with_ids(d.tree_world()) {
            for f in brush.faces() {
                assert!((metrics(f).unwrap().density - 2.0).abs() < 1e-5);
                assert!(
                    (f.uv_attributes.scale.x().abs() / f.uv_attributes.scale.y().abs() - 0.5).abs()
                        < 1e-5
                );
                assert_eq!(f.uv_attributes.scale.x() < 0.0, f.material_name == "a");
                assert_eq!(f.uv_attributes.rotation, 23.0);
            }
        }
        for (a, b) in anchors.iter().zip(centers(&d)) {
            assert!(vec::length(*a - b) < 0.001);
        }
        let after = d.to_map_text();
        assert!(!set_density(&mut d, 2.0));
        assert!(!set_density(&mut d, f64::NAN));
        assert!(!set_density(&mut d, 0.0));
        assert_eq!(d.to_map_text(), after);
        d.undo();
        assert_eq!(d.to_map_text(), before);
        d.redo();
        assert_eq!(d.to_map_text(), after);
    }
    #[test]
    fn individual_rotation_keeps_each_faces_own_pivot_and_undoes_atomically() {
        let mut d = document();
        let before = d.to_map_text();
        let anchors = centers(&d);
        transform(
            &mut d,
            Transform::Rotate(37.0),
            PivotMode::Individual,
            Vec2f::zero(),
        );
        for (a, b) in anchors.iter().zip(centers(&d)) {
            assert!(vec::length(*a - b) < 0.001);
        }
        assert_ne!(before, d.to_map_text());
        d.undo();
        assert_eq!(before, d.to_map_text());
        let id = d.selectable_brushes()[0].0;
        d.select_only(id);
        let untouched = d
            .selectable_brushes()
            .iter()
            .find(|(other, _)| *other != id)
            .map(|(id, _)| *id)
            .unwrap();
        let old = d.brush(untouched).unwrap().faces().to_vec();
        assert!(set_density(&mut d, 3.0));
        assert_eq!(d.brush(untouched).unwrap().faces(), old);
    }
}
