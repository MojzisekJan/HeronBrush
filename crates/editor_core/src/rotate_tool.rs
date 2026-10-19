//! Rotating the selection by dragging a ring around it.
//!
//! The tool keeps one thing of its own: where the rotation turns about. Everything else
//! comes from the drag -- the angle is measured from where the drag started, about the
//! axis the view looks down, and each step rolls the selection back and rotates it afresh
//! from the original so the result depends only on where the pointer is now.

use geometry_math::vec::{self, Vec3d};

use crate::document::Document;
use crate::grid::Grid;

/// The rotate tool's state.
#[derive(Debug, Clone, Default)]
pub struct RotateTool {
    center: Option<Vec3d>,
    angle: f64,
}

impl RotateTool {
    /// Creates a tool with no centre yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns what the rotation turns about.
    pub fn center(&self) -> Option<Vec3d> {
        self.center
    }

    /// Moves what the rotation turns about.
    ///
    /// `RotateTool::setRotationCenter`.
    pub fn set_center(&mut self, position: Vec3d) {
        self.center = Some(position);
    }

    /// Returns the angle of the drag in progress, in radians.
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// Puts the centre back where the selection says it should be.
    ///
    /// `RotateTool::resetRotationCenter`: one entity turns about its own origin, and
    /// anything else about the snapped middle of what is selected. Called when the tool is
    /// entered, so it starts somewhere sensible rather than wherever it was left.
    pub fn reset_center(&mut self, document: &Document, grid: &Grid) {
        if document.selection_count() == 1 {
            if let Some(origin) = document
                .selected_entity()
                .and_then(|(_, entity)| entity.origin())
            {
                self.center = Some(origin);
                return;
            }
        }

        self.center = document
            .selection_bounds()
            .map(|bounds| grid.snap_point(bounds.center()));
    }

    /// Forgets the centre and the angle, which is what leaving the tool does.
    pub fn clear(&mut self) {
        self.center = None;
        self.angle = 0.0;
    }

    /// Measures the angle a drag has turned through, and remembers it.
    ///
    /// The ring drag's `update`: the angle from where the drag started to where the
    /// pointer is now, both measured from the centre, about the ring's axis. Snapping is
    /// applied to the angle rather than to the pointer, which is what makes a rotation
    /// land on a whole number of degrees however far out the pointer is.
    pub fn measure(&mut self, grid: &Grid, axis: Vec3d, from: Vec3d, to: Vec3d) -> Option<f64> {
        let center = self.center?;
        let reference = to_unit(from - center)?;
        let current = to_unit(to - center)?;

        self.angle = grid.snap_angle(vec::measure_angle_3d(current, reference, axis));
        Some(self.angle)
    }

    /// Apply angle snapping to a 3D gesture while retaining the usual 0..TAU range.
    pub fn set_drag_angle(&mut self, grid: &Grid, angle: f64) -> f64 {
        self.angle = grid.snap_angle(angle).rem_euclid(std::f64::consts::TAU);
        self.angle
    }

    /// The angle to show, which reads better as -5 degrees than as 355.
    ///
    /// The ring drag's `render` does the same before drawing the indicator.
    pub fn display_angle(&self) -> f64 {
        if self.angle > std::f64::consts::PI {
            self.angle - std::f64::consts::TAU
        } else {
            self.angle
        }
    }
}

/// Normalizes a vector, or returns nothing if it has no direction to give.
fn to_unit(v: Vec3d) -> Option<Vec3d> {
    let length = vec::length(v);
    (length > 0.0).then(|| v / length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    fn grid(snap: bool) -> Grid {
        let mut grid = Grid::default();
        grid.set_snap(snap);
        grid
    }

    #[test]
    fn a_quarter_turn_measures_a_quarter_turn() {
        let mut tool = RotateTool::new();
        tool.set_center(Vec3d::zero());

        // Looking down Z, from +X round to +Y.
        let angle = tool
            .measure(
                &grid(false),
                Vec3d::new(0.0, 0.0, 1.0),
                Vec3d::new(32.0, 0.0, 0.0),
                Vec3d::new(0.0, 32.0, 0.0),
            )
            .expect("an angle");

        geometry_math::assert_approx_eq!(angle, FRAC_PI_2, 1e-9);
    }

    #[test]
    fn snapping_rounds_the_angle_to_fifteen_degrees() {
        let mut tool = RotateTool::new();
        tool.set_center(Vec3d::zero());

        // Twenty degrees rounds to fifteen; the pointer's distance does not matter.
        let twenty = 20.0f64.to_radians();
        let angle = tool
            .measure(
                &grid(true),
                Vec3d::new(0.0, 0.0, 1.0),
                Vec3d::new(100.0, 0.0, 0.0),
                Vec3d::new(twenty.cos(), twenty.sin(), 0.0) * 7.0,
            )
            .expect("an angle");

        geometry_math::assert_approx_eq!(angle.to_degrees(), 15.0, 1e-9);
    }

    #[test]
    fn without_snapping_the_angle_is_whatever_it_is() {
        let mut tool = RotateTool::new();
        tool.set_center(Vec3d::zero());

        let twenty = 20.0f64.to_radians();
        let angle = tool
            .measure(
                &grid(false),
                Vec3d::new(0.0, 0.0, 1.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(twenty.cos(), twenty.sin(), 0.0),
            )
            .expect("an angle");

        geometry_math::assert_approx_eq!(angle.to_degrees(), 20.0, 1e-9);
    }

    #[test]
    fn a_drag_that_has_not_left_the_centre_measures_nothing() {
        let mut tool = RotateTool::new();
        tool.set_center(Vec3d::zero());

        assert_eq!(
            tool.measure(
                &grid(false),
                Vec3d::new(0.0, 0.0, 1.0),
                Vec3d::zero(),
                Vec3d::new(32.0, 0.0, 0.0),
            ),
            None
        );
    }

    #[test]
    fn there_is_no_angle_without_a_centre() {
        let mut tool = RotateTool::new();
        assert_eq!(
            tool.measure(
                &grid(false),
                Vec3d::new(0.0, 0.0, 1.0),
                Vec3d::new(32.0, 0.0, 0.0),
                Vec3d::new(0.0, 32.0, 0.0),
            ),
            None
        );
    }

    #[test]
    fn the_angle_shown_takes_the_short_way_round() {
        let mut tool = RotateTool::new();
        tool.set_center(Vec3d::zero());

        // Five degrees short of a full turn reads as five degrees back.
        let nearly_round = TAU - 5.0f64.to_radians();
        tool.measure(
            &grid(false),
            Vec3d::new(0.0, 0.0, 1.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(nearly_round.cos(), nearly_round.sin(), 0.0),
        );

        geometry_math::assert_approx_eq!(tool.display_angle().to_degrees(), -5.0, 1e-9);
        assert!(tool.angle() > PI);
    }

    #[test]
    fn leaving_the_tool_forgets_where_it_was_turning_about() {
        let mut tool = RotateTool::new();
        tool.set_center(Vec3d::new(16.0, 16.0, 16.0));
        assert_eq!(tool.center(), Some(Vec3d::new(16.0, 16.0, 16.0)));

        tool.clear();
        assert_eq!(tool.center(), None);
        assert_eq!(tool.angle(), 0.0);
    }
}
