//! The "X: 64" labels along the selection's bounding box.
//!
//! Selection size labels show axis extents beside bounding-box edges. Flat views label their two
//! visible axes; 3D labels use edges facing the camera.
//!
//! This only says where a label goes and what it reads. The views draw it, since the
//! flat views and the 3D view's overlay project a point differently.

use geometry_math::bbox::{Bbox3d, Range};
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec3d};

/// Which side of the anchor holds the label. `Top` aligns the label top edge with the anchor so the
/// text hangs below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Top,
    Bottom,
    /// Used by optional minimum and maximum coordinate labels.
    #[allow(dead_code, reason = "alignment variants describe all four anchor sides")]
    Left,
    Right,
}

/// `SizeTextAnchor::extraOffsets`: how far the label is pushed off its anchor.
pub const GAP: f64 = 8.0;

/// One label: what it says and where it hangs.
#[derive(Debug, Clone, PartialEq)]
pub struct SizeLabel {
    pub text: String,
    /// The point in the world the label is placed against.
    pub base: Vec3d,
    pub alignment: Alignment,
}

/// The labels a flat view looking along `direction` shows for `bounds`.
///
/// `renderSize2D`: an axis is labelled when the view does not look along it, at the
/// middle of the box's minimum edge on that axis, `SizeTextAnchor2D::basePosition`.
pub fn labels_2d(bounds: &Bbox3d, direction: Vec3d) -> Vec<SizeLabel> {
    let size = vec::correct(bounds.size(), 0, f64::CORRECT_EPSILON);
    let half = size / 2.0;
    (0..3)
        .filter(|axis| direction[*axis] == 0.0)
        .map(|axis| {
            let mut base = bounds.min;
            base[axis] += half[axis];
            let alignment = if axis == 0 || (axis == 1 && direction.x() != 0.0) {
                Alignment::Top
            } else {
                Alignment::Right
            };
            SizeLabel { text: text(axis, size[axis]), base, alignment }
        })
        .collect()
}

/// The labels the 3D view shows for `bounds`, from a camera at `position` looking along
/// `direction`.
///
/// `renderSize3D`: every axis is labelled, on the edge nearest the camera,
/// `SizeTextAnchor3D::basePosition`. The Z label goes to the left of a vertical edge;
/// the X and Y labels below the box when the camera is under it, above it otherwise.
pub fn labels_3d(bounds: &Bbox3d, position: Vec3d, direction: Vec3d) -> Vec<SizeLabel> {
    let size = vec::correct(bounds.size(), 0, f64::CORRECT_EPSILON);
    (0..3)
        .map(|axis| SizeLabel {
            text: text(axis, size[axis]),
            base: base_3d(bounds, axis, position, direction),
            alignment: alignment_3d(bounds, axis, position),
        })
        .collect()
}

fn text(axis: usize, size: f64) -> String {
    format!("{}: {}", ["X", "Y", "Z"][axis], size)
}

fn alignment_3d(bounds: &Bbox3d, axis: usize, position: Vec3d) -> Alignment {
    if axis == 2 {
        return Alignment::Right;
    }
    let at = bounds.relative_position(position);
    if at[2] == Range::Less {
        Alignment::Top
    } else {
        Alignment::Bottom
    }
}

/// `SizeTextAnchor3D::basePosition`, case for case.
fn base_3d(bounds: &Bbox3d, axis: usize, position: Vec3d, direction: Vec3d) -> Vec3d {
    use Range::{Greater, Less, Within};
    let at = bounds.relative_position(position);
    let half = bounds.size() / 2.0;
    let (min, max) = (bounds.min, bounds.max);
    let mut pos = Vec3d::zero();

    if axis == 2 {
        match (at[0], at[1]) {
            (Less, Less) | (Less, Within) => {
                pos[0] = min.x();
                pos[1] = max.y();
            }
            (Less, Greater) | (Within, Greater) => {
                pos[0] = max.x();
                pos[1] = max.y();
            }
            (Greater, Greater) | (Greater, Within) => {
                pos[0] = max.x();
                pos[1] = min.y();
            }
            (Within, Less) | (Greater, Less) => {
                pos[0] = min.x();
                pos[1] = min.y();
            }
            (Within, Within) => {
                pos[0] = if direction.x() <= 0.0 { min.x() } else { max.x() };
                pos[1] = if direction.y() <= 0.0 { min.y() } else { max.y() };
            }
        }
        pos[2] = min.z() + half.z();
        return pos;
    }

    let inside_z = at[2] == Within;
    if axis == 0 {
        pos[0] = min.x() + half.x();
        pos[1] = match (at[0], at[1]) {
            (Less, Less) => pick(inside_z, min.y(), max.y()),
            (Less, Within) => max.y(),
            (Less, Greater) => pick(inside_z, max.y(), min.y()),
            (Within, Less) => pick(inside_z, min.y(), max.y()),
            (Within, Within) => {
                if direction.y() <= 0.0 {
                    min.y()
                } else {
                    max.y()
                }
            }
            (Within, Greater) => pick(inside_z, max.y(), min.y()),
            (Greater, Less) => pick(inside_z, min.y(), max.y()),
            (Greater, Within) => min.y(),
            (Greater, Greater) => pick(inside_z, max.y(), min.y()),
        };
    } else {
        pos[1] = min.y() + half.y();
        pos[0] = match (at[0], at[1]) {
            (Less, Less) | (Less, Within) | (Less, Greater) => pick(inside_z, min.x(), max.x()),
            (Within, Less) => min.x(),
            (Within, Within) => {
                if direction.x() <= 0.0 {
                    min.x()
                } else {
                    max.x()
                }
            }
            (Within, Greater) => max.x(),
            (Greater, Less) | (Greater, Within) | (Greater, Greater) => {
                pick(inside_z, max.x(), min.x())
            }
        };
    }
    pos[2] = if at[2] == Less { min.z() } else { max.z() };
    pos
}

fn pick(condition: bool, when_true: f64, when_false: f64) -> f64 {
    if condition {
        when_true
    } else {
        when_false
    }
}

/// Formats three space-separated coordinates, rounding floating-point noise consistently with size
/// labels.
pub fn point_text(position: Vec3d) -> String {
    let position = vec::correct(position, 0, f64::CORRECT_EPSILON);
    // Rounding a coordinate that was a hair below zero gives negative zero, which prints
    // as "-0" and reads as a different place from "0".
    let plain = |value: f64| if value == 0.0 { 0.0 } else { value };
    format!(
        "{} {} {}",
        plain(position.x()),
        plain(position.y()),
        plain(position.z())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> Bbox3d {
        Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(64.0, 32.0, 16.0))
    }

    #[test]
    fn a_handle_reads_as_its_three_coordinates() {
        assert_eq!(point_text(Vec3d::new(16.0, -8.0, 4.0)), "16 -8 4");
        // A drag leaves a coordinate a hair off a whole number; it still reads whole.
        assert_eq!(point_text(Vec3d::new(64.00000000000006, 0.0, -0.5)), "64 0 -0.5");
        // A coordinate a hair below zero is zero, not "-0".
        assert_eq!(point_text(Vec3d::new(-1e-9, 0.0, 0.0)), "0 0 0");
    }

    #[test]
    fn a_top_view_labels_x_below_and_y_to_the_left() {
        let labels = labels_2d(&cube(), Vec3d::new(0.0, 0.0, -1.0));
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].text, "X: 64");
        assert_eq!(labels[0].base, Vec3d::new(32.0, 0.0, 0.0));
        assert_eq!(labels[0].alignment, Alignment::Top);
        assert_eq!(labels[1].text, "Y: 32");
        assert_eq!(labels[1].base, Vec3d::new(0.0, 16.0, 0.0));
        assert_eq!(labels[1].alignment, Alignment::Right);
    }

    #[test]
    fn a_side_view_puts_y_below_since_it_runs_across() {
        let labels = labels_2d(&cube(), Vec3d::new(-1.0, 0.0, 0.0));
        let y = labels.iter().find(|l| l.text.starts_with("Y")).unwrap();
        assert_eq!(y.alignment, Alignment::Top);
        let z = labels.iter().find(|l| l.text.starts_with("Z")).unwrap();
        assert_eq!(z.alignment, Alignment::Right);
        assert_eq!(z.base, Vec3d::new(0.0, 0.0, 8.0));
    }

    #[test]
    fn a_size_a_hair_off_a_whole_number_reads_as_the_whole_number() {
        let bounds = Bbox3d::new(Vec3d::zero(), Vec3d::new(96.00000000000065, 1.0, 1.0));
        let labels = labels_2d(&bounds, Vec3d::new(0.0, 0.0, -1.0));
        assert_eq!(labels[0].text, "X: 96");
    }

    #[test]
    fn a_size_that_is_not_whole_keeps_its_decimals() {
        let bounds = Bbox3d::new(Vec3d::zero(), Vec3d::new(10.5, 1.0, 1.0));
        let labels = labels_2d(&bounds, Vec3d::new(0.0, 0.0, -1.0));
        assert_eq!(labels[0].text, "X: 10.5");
    }

    #[test]
    fn in_3d_the_labels_sit_on_the_edges_facing_the_camera() {
        // A camera below and before the box on both axes, looking up at it.
        let labels = labels_3d(
            &cube(),
            Vec3d::new(-100.0, -100.0, -100.0),
            vec::normalize(Vec3d::new(1.0, 1.0, 1.0)),
        );
        assert_eq!(labels.len(), 3);
        // X along the bottom, on the edge with the smaller y, hanging below.
        assert_eq!(labels[0].base, Vec3d::new(32.0, 32.0, 0.0));
        assert_eq!(labels[0].alignment, Alignment::Top);
        // Z on the corner nearest the camera, to the left of it.
        assert_eq!(labels[2].base, Vec3d::new(0.0, 32.0, 8.0));
        assert_eq!(labels[2].alignment, Alignment::Right);
    }

    #[test]
    fn a_camera_above_the_box_gets_the_labels_above() {
        let labels = labels_3d(
            &cube(),
            Vec3d::new(32.0, 16.0, 200.0),
            Vec3d::new(0.0, 0.0, -1.0),
        );
        assert_eq!(labels[0].alignment, Alignment::Bottom);
        assert_eq!(labels[0].base.z(), 16.0);
        assert_eq!(labels[1].alignment, Alignment::Bottom);
    }
}
