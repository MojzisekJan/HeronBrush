//! Screen-space orientation gizmo shared by the 2D and 3D views.
//! Thin axis lines and small labels keep the map visible underneath.
//! Axis colours follow preferences. This is an orientation indicator.

use geometry_math::vec::{self, Vec2d, Vec3d};

use crate::canvas::{Canvas, LineCap};

use crate::preferences::{pref, Color};

const ARM: f64 = 15.0;
const LABEL_DISTANCE: f64 = 22.0;
const EXTENT: f64 = 27.0;
const MARGIN: f64 = 10.0;

/// Where the gizmo is anchored in logical pixels, independent of display scale.
pub fn center(viewport: (f64, f64)) -> Vec2d {
    let inset = EXTENT + MARGIN;
    Vec2d::new(inset, viewport.1 - inset)
}

/// One arrow, projected: where it runs on the screen relative to the gizmo's centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrow {
    /// The world axis this is, 0 to 2.
    pub axis: usize,
    /// The screen direction the axis points in, in pixels per world unit. Shorter than
    /// one when the axis is foreshortened, down to nothing when it points at the viewer.
    pub direction: Vec2d,
    /// The axis's component along the camera's line of sight: positive points away from
    /// the viewer, negative towards them.
    pub depth: f64,
}

/// Projects the three world axes the way the camera sees them, `cameraRotationMatrix`.
///
/// `right`, `up` and `direction` are the camera's own frame, unit length. The arrows come
/// back sorted far to near, which is the order to draw them in so a nearer one covers a
/// farther one. `hidden` leaves an axis out: `Compass2D` does not draw the axis a flat
/// view looks along, there being nothing to show of it.
pub fn arrows(right: Vec3d, up: Vec3d, direction: Vec3d, hidden: Option<usize>) -> Vec<Arrow> {
    let mut arrows: Vec<Arrow> = (0..3)
        .filter(|axis| Some(*axis) != hidden)
        .map(|axis| {
            let mut unit = Vec3d::zero();
            unit[axis] = 1.0;
            Arrow {
                axis,
                // Screen y grows downward, so the camera's up is negated.
                direction: Vec2d::new(vec::dot(unit, right), -vec::dot(unit, up)),
                depth: vec::dot(unit, direction),
            }
        })
        .collect();
    arrows.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    arrows
}

/// The colour an axis is drawn in, `Preferences::axisColor`.
pub fn axis_color(axis: usize) -> Color {
    match axis {
        0 => pref(|p| p.x_axis_color),
        1 => pref(|p| p.y_axis_color),
        _ => pref(|p| p.z_axis_color),
    }
}

/// A compact, unboxed triad. Small text halos keep the labels legible on map
/// geometry without covering the scene with a background panel.
pub fn draw(cr: &mut dyn Canvas, viewport: (f64, f64), arrows: &[Arrow]) {
    cr.save();
    let at = center(viewport);
    cr.set_line_cap(LineCap::Round);
    for axis in arrows {
        let (r, g, b, _) = axis_color(axis.axis).rgba_f64();
        let start = at - axis.direction * 5.0;
        let end = at + axis.direction * ARM;
        cr.new_path();
        cr.move_to(start.x(), start.y());
        cr.line_to(end.x(), end.y());
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.35);
        cr.set_line_width(2.5);
        cr.stroke_preserve();
        cr.set_source_rgba(r, g, b, 0.8);
        cr.set_line_width(1.25);
        cr.stroke();
    }
    cr.set_font_bold(true);
    for axis in arrows {
        let end = at + axis.direction * LABEL_DISTANCE;
        let label = ["X", "Y", "Z"][axis.axis];
        let (r, g, b, _) = axis_color(axis.axis).rgba_f64();
        let extents = cr.text_extents(label, 9.0);
        cr.new_path();
        cr.move_to(end.x() - extents.width / 2.0 - extents.x_bearing,
            end.y() - extents.height / 2.0 - extents.y_bearing);
        cr.set_source_rgba(r, g, b, 0.95);
        cr.show_text_outlined(label, 9.0, [0.0, 0.0, 0.0, 0.65], 2.0);
    }
    cr.set_font_bold(false);
    cr.restore();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_camera_looking_along_y_sees_x_to_the_right_and_z_up() {
        let arrows = arrows(
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 0.0, 1.0),
            Vec3d::new(0.0, 1.0, 0.0),
            None,
        );
        let x = arrows.iter().find(|a| a.axis == 0).unwrap();
        let z = arrows.iter().find(|a| a.axis == 2).unwrap();
        let y = arrows.iter().find(|a| a.axis == 1).unwrap();
        assert_eq!(x.direction, Vec2d::new(1.0, 0.0));
        assert_eq!(z.direction, Vec2d::new(0.0, -1.0));
        assert_eq!(y.direction, Vec2d::new(0.0, 0.0));
        assert_eq!(y.depth, 1.0);
    }

    #[test]
    fn the_farthest_arrow_comes_first() {
        // Looking down the diagonal, so every axis has some depth.
        let direction = vec::normalize(Vec3d::new(1.0, 2.0, 3.0));
        let right = vec::normalize(vec::cross(direction, Vec3d::new(0.0, 0.0, 1.0)));
        let up = vec::cross(right, direction);
        let arrows = arrows(right, up, direction, None);
        assert_eq!(arrows.len(), 3);
        assert!(arrows[0].depth >= arrows[1].depth && arrows[1].depth >= arrows[2].depth);
        assert_eq!(arrows[0].axis, 2, "Z has the largest share of the look direction");
    }

    #[test]
    fn a_flat_view_leaves_its_depth_axis_out() {
        let arrows = arrows(
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(0.0, 0.0, -1.0),
            Some(2),
        );
        assert_eq!(arrows.len(), 2);
        assert!(arrows.iter().all(|a| a.axis != 2));
    }

}
