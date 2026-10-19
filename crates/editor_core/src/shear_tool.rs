//! Shearing the selection by dragging a face of its bounding box sideways.
//!
//! Where the scale tool moves a face of the bounding box along its own normal, the shear
//! tool slides it *across* it, holding the opposite face still, which is how a wall is
//! given a lean. The handle is a face, not a corner, so there are six of them.
//!
//! Which way a face may slide is the whole of the tool's behaviour, and it depends on
//! which face: the top and bottom slide freely across themselves, and the four walls slide
//! either along the ground or straight up, since a wall that could go both ways at once
//! would be scaled rather than sheared.

use geometry_math::bbox::Bbox3d;
use geometry_math::line::Line3d;
use geometry_math::plane::Plane3d;
use geometry_math::vec::{self, Vec3d};

use crate::handle_drag::HandlePicker;
use crate::scale_tool;

/// The six faces of a box, as their outward normals and the middle of each.
pub fn handles(bounds: &Bbox3d) -> Vec<(Vec3d, Vec3d)> {
    scale_tool::all_sides()
        .into_iter()
        .map(|normal| (normal, scale_tool::center_for_bbox_side(bounds, normal)))
        .collect()
}

/// Which way the face with the given normal may slide.
///
/// `ShearToolController::makeHandlePositionProposer`. A horizontal face slides across
/// itself in two directions at once; a vertical one slides along a single line, either up
/// or sideways, since sliding it both ways at once is a scale rather than a shear.
///
/// Returns `None` for an edge-on face because it has no visible screen-space direction for a shear
/// drag.
pub fn picker(
    side_normal: Vec3d,
    side_center: Vec3d,
    offset: Vec3d,
    perspective: bool,
    camera_direction: Vec3d,
    vertical: bool,
) -> Option<HandlePicker> {
    let up = Vec3d::new(0.0, 0.0, 1.0);

    if perspective {
        // The top and the bottom: the only faces whose own plane is the ground, so the
        // only ones that can slide in two directions without leaving it.
        if vec::abs(side_normal) == up {
            return Some(HandlePicker::plane(
                Plane3d::from_anchor_and_normal(side_center, side_normal),
                offset,
            ));
        }

        if vertical {
            return Some(HandlePicker::line(
                Line3d {
                    point: side_center,
                    direction: up,
                },
                offset,
            ));
        }

        return along(side_center, vec::cross(side_normal, up), offset);
    }

    // A 2D view: the face slides across the screen, which is the direction perpendicular
    // to both its normal and the way the view looks.
    along(side_center, vec::cross(side_normal, camera_direction), offset)
}

/// A line through the given point, or nothing if the direction has no length.
fn along(point: Vec3d, direction: Vec3d, offset: Vec3d) -> Option<HandlePicker> {
    let length = vec::length(direction);
    (length > geometry_math::scalar::Float::ALMOST_ZERO).then(|| {
        HandlePicker::line(
            Line3d {
                point,
                direction: direction / length,
            },
            offset,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> Bbox3d {
        Bbox3d::from_scalars(-32.0, 32.0)
    }

    #[test]
    fn a_box_has_one_handle_per_face() {
        let handles = handles(&cube());
        assert_eq!(handles.len(), 6);

        for (normal, center) in handles {
            // Each handle sits at the middle of the face its normal names.
            let axis = normal.find_abs_max_component(0);
            geometry_math::assert_approx_eq!(center[axis], 32.0 * normal[axis], 1e-9);
            // And the other two components are the middle of the box.
            for other in 0..3 {
                if other != axis {
                    geometry_math::assert_approx_eq!(center[other], 0.0, 1e-9);
                }
            }
        }
    }

    #[test]
    fn a_horizontal_face_slides_across_itself() {
        let picker = picker(
            Vec3d::new(0.0, 0.0, 1.0),
            Vec3d::new(0.0, 0.0, 32.0),
            Vec3d::zero(),
            true,
            Vec3d::new(0.0, 1.0, 0.0),
            false,
        );

        let Some(HandlePicker::Plane { plane, .. }) = picker else {
            panic!("the top slides across a plane");
        };
        geometry_math::assert_approx_eq!(plane.normal, Vec3d::new(0.0, 0.0, 1.0), 1e-9);
        geometry_math::assert_approx_eq!(plane.anchor(), Vec3d::new(0.0, 0.0, 32.0), 1e-9);
    }

    #[test]
    fn a_wall_slides_along_the_ground_by_default() {
        // The +X wall, which slides along Y.
        let picker = picker(
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(32.0, 0.0, 0.0),
            Vec3d::zero(),
            true,
            Vec3d::new(0.0, 1.0, 0.0),
            false,
        );

        let Some(HandlePicker::Line { line, .. }) = picker else {
            panic!("a wall slides along a line");
        };
        // Along Y; which way round makes no difference to a line.
        geometry_math::assert_approx_eq!(vec::abs(line.direction), Vec3d::new(0.0, 1.0, 0.0), 1e-9);
    }

    #[test]
    fn alt_makes_a_wall_slide_straight_up() {
        let picker = picker(
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(32.0, 0.0, 0.0),
            Vec3d::zero(),
            true,
            Vec3d::new(0.0, 1.0, 0.0),
            true,
        );

        let Some(HandlePicker::Line { line, .. }) = picker else {
            panic!("a wall slides along a line");
        };
        geometry_math::assert_approx_eq!(line.direction, Vec3d::new(0.0, 0.0, 1.0), 1e-9);
    }

    #[test]
    fn in_a_flat_view_a_face_slides_across_the_screen() {
        // Looking down Z, at the +X wall: it can only slide along Y.
        let picker = picker(
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(32.0, 0.0, 0.0),
            Vec3d::zero(),
            false,
            Vec3d::new(0.0, 0.0, -1.0),
            false,
        );

        let Some(HandlePicker::Line { line, .. }) = picker else {
            panic!("a face in a flat view slides along a line");
        };
        geometry_math::assert_approx_eq!(vec::abs(line.direction), Vec3d::new(0.0, 1.0, 0.0), 1e-9);
    }

    #[test]
    fn every_face_has_somewhere_to_slide_in_every_view() {
        for perspective in [true, false] {
            for vertical in [true, false] {
                for (normal, center) in handles(&cube()) {
                    let picker = picker(
                        normal,
                        center,
                        Vec3d::zero(),
                        perspective,
                        Vec3d::new(0.0, 1.0, 0.0),
                        vertical,
                    );
                    // Either the face has a direction with a length, or it has none at
                    // all -- never a direction that is really a zero vector.
                    match picker {
                        Some(HandlePicker::Line { line, .. }) => {
                            geometry_math::assert_approx_eq!(vec::length(line.direction), 1.0, 1e-9);
                        }
                        Some(HandlePicker::Plane { plane, .. }) => {
                            geometry_math::assert_approx_eq!(vec::length(plane.normal), 1.0, 1e-9);
                        }
                        Some(HandlePicker::Circle { .. })
                        | Some(HandlePicker::Canonical { .. }) => {
                            panic!("shearing uses only lines and planes")
                        }
                        // Only a face seen exactly edge-on in a flat view.
                        None => {
                            assert!(!perspective);
                            geometry_math::assert_approx_eq!(
                                vec::abs(normal),
                                Vec3d::new(0.0, 1.0, 0.0),
                                1e-9
                            );
                        }
                    }
                }
            }
        }
    }
}
