//! Orient texture mapping using an explicitly chosen geometric edge.
use crate::document::{Document, FaceRef};
use geometry_math::vec;
use map_model::polyhedron::FaceId;

/// The texture coordinate along which an edge should run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeAxis {
    Horizontal,
    Vertical,
}

/// Aligns the chosen edge in texture space, preserving its midpoint and the UV scale.
/// The active face determines the turn; every selected face receives that same turn.
/// Returns false for a stale selection, missing edge or degenerate mapping.
pub fn align(document: &mut Document, reference: FaceRef, edge: usize, axis: EdgeAxis) -> bool {
    if document.face_to_edit().map(|(face, _)| face) != Some(reference) {
        return false;
    }
    let Some(brush) = document.brush(reference.node) else {
        return false;
    };
    let points = brush
        .geometry()
        .face_positions(FaceId(reference.face as u32));
    if edge >= points.len() || points.len() < 2 {
        return false;
    }
    let (start, end) = (points[edge], points[(edge + 1) % points.len()]);
    if vec::length(end - start) < 1e-6 {
        return false;
    }
    let Some(face) = document.face_at(reference) else {
        return false;
    };
    let component = match axis {
        EdgeAxis::Horizontal => 1,
        EdgeAxis::Vertical => 0,
    };
    let initial = face.uv_coords(end) - face.uv_coords(start);
    let mut quarter_turn = face.clone();
    quarter_turn.rotate_uv(90.0);
    let quarter = quarter_turn.uv_coords(end) - quarter_turn.uv_coords(start);
    let (cosine, sine) = (initial[component], quarter[component]);
    if !cosine.is_finite() || !sine.is_finite() || cosine.hypot(sine) < 1e-6 {
        return false;
    }
    // Both roots differ by 180 degrees. Choose the nearest orientation to avoid a flip.
    let degrees = ((-cosine).atan2(sine).to_degrees() + 90.0).rem_euclid(180.0) - 90.0;
    let midpoint = (start + end) * 0.5;
    document.edit_faces(|face| {
        let anchor = face.uv_coords(midpoint);
        face.rotate_uv_about(anchor, degrees);
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec2f;
    fn document() -> (Document, FaceRef) {
        let mut document = Document::new(WorldBounds::default());
        document.add_brushes(vec![
            map_model::shapes::create_cuboid(
                &document.world_bounds(),
                &Bbox3d::from_scalars(-32.0, 32.0),
                "stone",
            )
            .expect("test cube"),
        ]);
        document.select_all();
        let reference = document.face_to_edit().expect("selected face").0;
        document.select_face_only(reference);
        (document, reference)
    }
    #[test]
    fn aligns_both_axes_with_nonuniform_and_mirrored_scales_and_atomic_undo() {
        for parallel in [false, true] {
            for axis in [EdgeAxis::Horizontal, EdgeAxis::Vertical] {
                for scale in [Vec2f::new(0.5, 2.0), Vec2f::new(-0.5, 2.0)] {
                    let (mut document, reference) = document();
                    document.edit_faces(|face| {
                        if parallel {
                            let (u_axis, v_axis) = face
                                .uv_coord_system
                                .axes(face.normal(), &face.uv_attributes);
                            face.uv_coord_system =
                                map_model::uv::UvCoordSystem::Parallel { u_axis, v_axis };
                        }
                        face.rotate_uv(27.0);
                        face.uv_attributes.scale = scale;
                    });
                    let points = document
                        .brush(reference.node)
                        .expect("cube")
                        .geometry()
                        .face_positions(FaceId(reference.face as u32));
                    let midpoint = (points[0] + points[1]) * 0.5;
                    let before = document.to_map_text();
                    let anchor = document
                        .face_at(reference)
                        .expect("face")
                        .uv_coords(midpoint);
                    assert!(align(&mut document, reference, 0, axis));
                    let face = document.face_at(reference).expect("face");
                    let delta = face.uv_coords(points[1]) - face.uv_coords(points[0]);
                    let component = if axis == EdgeAxis::Horizontal { 1 } else { 0 };
                    assert!(
                        delta[component].abs() < 0.001,
                        "edge must follow {axis:?}: {delta:?}"
                    );
                    assert_eq!(face.uv_attributes.scale, scale);
                    assert!(
                        vec::length(face.uv_coords(midpoint) - anchor) < 0.001,
                        "edge midpoint stays fixed"
                    );
                    document.undo();
                    assert_eq!(
                        document.to_map_text(),
                        before,
                        "one undo restores the mapping"
                    );
                }
            }
        }
    }
    #[test]
    fn stale_or_invalid_edges_do_not_edit_the_document() {
        let (mut document, reference) = document();
        let before = document.to_map_text();
        assert!(!align(
            &mut document,
            reference,
            usize::MAX,
            EdgeAxis::Horizontal
        ));
        document.clear_selection();
        assert!(!align(&mut document, reference, 0, EdgeAxis::Horizontal));
        assert_eq!(document.to_map_text(), before);
    }
}
