//! Scaling the selection by dragging its bounding box.
//!
//! Bounding-box handles and the transformations produced by dragging them.
//!
//! A box has three kinds of handle. A **side** handle moves one face and scales along one
//! axis; an **edge** handle moves two faces and scales along two; a **corner** handle
//! moves three. Each is named by a point on the unit box about the origin -- a normal for
//! a side, a corner of `(±1, ±1, ±1)` for a corner, two of those for an edge -- so the
//! same handle means the same thing whatever the selection's actual size.
//!
//! Whatever the handle, the drag ends in one new box, and the transform taking the old
//! box to the new one is what the selection is scaled by.

use geometry_math::bbox::Bbox3d;
use geometry_math::line::Line3d;
use geometry_math::vec::{self, Vec3d};

/// What stays put while the box is resized.
///
/// `AnchorPos`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnchorPos {
    /// The far side of the box stays put, so only the dragged side moves.
    #[default]
    Opposite,
    /// The middle stays put, so both sides move and the box grows symmetrically.
    Center,
}

/// Which axes follow the one being dragged.
///
/// `ProportionalAxes`. Scaling one axis proportionally scales the others by the same
/// ratio, which is what keeps a shape's proportions while it is resized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProportionalAxes {
    axes: [bool; 3],
}

impl ProportionalAxes {
    /// Sets each axis in turn.
    pub fn new(x: bool, y: bool, z: bool) -> Self {
        Self { axes: [x, y, z] }
    }

    /// Every axis follows.
    pub fn all() -> Self {
        Self::new(true, true, true)
    }

    /// No axis follows.
    pub fn none() -> Self {
        Self::default()
    }

    /// Returns whether the given axis follows.
    pub fn is_axis_proportional(&self, axis: usize) -> bool {
        self.axes[axis]
    }
}

/// A handle on a bounding box.
///
/// Handle variants identify the side, edge or corner and the axes affected by a drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BBoxHandle {
    /// A face, named by its outward normal: one of `±x`, `±y`, `±z`.
    Side(Vec3d),
    /// An edge, named by the two corners it runs between.
    Edge(Vec3d, Vec3d),
    /// A corner, named by its `(±1, ±1, ±1)` position on the unit box.
    Corner(Vec3d),
}

/// The six side normals, in the order `allSides` produces them.
pub fn all_sides() -> Vec<Vec3d> {
    let mut sides = Vec::with_capacity(6);
    for axis in 0..3 {
        for sign in [-1.0, 1.0] {
            let mut normal = Vec3d::zero();
            normal[axis] = sign;
            sides.push(normal);
        }
    }
    sides
}

/// The eight corners of the unit box about the origin, as `allCorners` gives them.
pub fn all_corners() -> Vec<Vec3d> {
    let mut corners = Vec::with_capacity(8);
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                corners.push(Vec3d::new(x, y, z));
            }
        }
    }
    corners
}

/// The twelve edges of the unit box, as `allEdges` gives them.
///
/// Two corners share an edge when they differ along exactly one axis.
pub fn all_edges() -> Vec<(Vec3d, Vec3d)> {
    let corners = all_corners();
    let mut edges = Vec::with_capacity(12);
    for (i, first) in corners.iter().enumerate() {
        for second in &corners[i + 1..] {
            let differing = (0..3).filter(|axis| first[*axis] != second[*axis]).count();
            if differing == 1 {
                edges.push((*first, *second));
            }
        }
    }
    edges
}

/// Every handle a box offers, sides first, then edges, then corners.
pub fn all_handles() -> Vec<BBoxHandle> {
    let mut handles: Vec<BBoxHandle> = all_sides().into_iter().map(BBoxHandle::Side).collect();
    handles.extend(
        all_edges()
            .into_iter()
            .map(|(a, b)| BBoxHandle::Edge(a, b)),
    );
    handles.extend(all_corners().into_iter().map(BBoxHandle::Corner));
    handles
}

/// The point on `box_` a unit-box corner names.
///
/// `pointForBBoxCorner`.
pub fn point_for_bbox_corner(box_: &Bbox3d, corner: Vec3d) -> Vec3d {
    let mut result = Vec3d::zero();
    for axis in 0..3 {
        result[axis] = if corner[axis] == 1.0 {
            box_.max[axis]
        } else {
            box_.min[axis]
        };
    }
    result
}

/// The middle of the face `normal` names.
///
/// `centerForBBoxSide`.
pub fn center_for_bbox_side(box_: &Bbox3d, normal: Vec3d) -> Vec3d {
    let axis = normal.find_abs_max_component(0);
    let mut center = box_.center();
    center[axis] = if normal[axis] > 0.0 {
        box_.max[axis]
    } else {
        box_.min[axis]
    };
    center
}

/// The two ends of the edge two unit-box corners name.
pub fn points_for_bbox_edge(box_: &Bbox3d, first: Vec3d, second: Vec3d) -> (Vec3d, Vec3d) {
    (
        point_for_bbox_corner(box_, first),
        point_for_bbox_corner(box_, second),
    )
}

impl BBoxHandle {
    /// Where the handle sits on the given box.
    pub fn position(&self, box_: &Bbox3d) -> Vec3d {
        match self {
            BBoxHandle::Side(normal) => center_for_bbox_side(box_, *normal),
            BBoxHandle::Edge(first, second) => {
                let (a, b) = points_for_bbox_edge(box_, *first, *second);
                (a + b) / 2.0
            }
            BBoxHandle::Corner(corner) => point_for_bbox_corner(box_, *corner),
        }
    }

    /// The handle across the box from this one, which is what stays put during a drag.
    pub fn opposite(&self) -> Self {
        match self {
            BBoxHandle::Side(normal) => BBoxHandle::Side(-*normal),
            BBoxHandle::Edge(first, second) => BBoxHandle::Edge(-*first, -*second),
            BBoxHandle::Corner(corner) => BBoxHandle::Corner(-*corner),
        }
    }

    /// The line this handle moves along.
    ///
    /// `handleLineForHit`: from the handle across the box from this one, through this
    /// one. A side runs along its own normal, which is the same line. The anchor does not
    /// come into it -- every one of these lines passes through the middle of the box, so
    /// anchoring the middle instead of the far side gives the same line.
    pub fn drag_line(&self, box_: &Bbox3d) -> Line3d {
        match self {
            BBoxHandle::Side(normal) => Line3d {
                point: center_for_bbox_side(box_, *normal),
                direction: *normal,
            },
            _ => {
                let from = self.opposite().position(box_);
                let to = self.position(box_);
                Line3d {
                    point: from,
                    direction: vec::normalize(to - from),
                }
            }
        }
    }

    /// The box that results from dragging this handle by `delta`.
    ///
    /// Returns `None` if scaling would invert or flatten the box.
    pub fn drag(
        &self,
        box_: &Bbox3d,
        delta: Vec3d,
        proportional: ProportionalAxes,
        anchor: AnchorPos,
    ) -> Option<Bbox3d> {
        match self {
            BBoxHandle::Side(normal) => move_bbox_side(box_, *normal, delta, proportional, anchor),
            BBoxHandle::Corner(corner) => move_bbox_corner(box_, *corner, delta, anchor),
            BBoxHandle::Edge(first, second) => {
                move_bbox_edge(box_, *first, *second, delta, proportional, anchor)
            }
        }
    }
}

/// `moveBBoxSide`.
pub fn move_bbox_side(
    box_: &Bbox3d,
    normal: Vec3d,
    delta: Vec3d,
    proportional: ProportionalAxes,
    anchor_type: AnchorPos,
) -> Option<Bbox3d> {
    let mut side_length_delta = vec::dot(normal, delta);

    // When using a center anchor, we're stretching both sides at once, so multiply the
    // delta by 2.
    if anchor_type == AnchorPos::Center {
        side_length_delta *= 2.0;
    }

    let axis = normal.find_abs_max_component(0);
    let in_side_length = box_.max[axis] - box_.min[axis];
    let side_length = in_side_length + side_length_delta;

    if side_length <= 0.0 {
        return None;
    }

    let axis1 = normal.find_abs_max_component(0);
    let axis2 = normal.find_abs_max_component(1);
    let axis3 = normal.find_abs_max_component(2);

    let mut new_size = box_.size();
    new_size[axis1] = side_length;

    // Optionally apply proportional scaling to axis2/axis3.
    let ratio = side_length / box_.size()[axis1];
    if proportional.is_axis_proportional(axis2) {
        new_size[axis2] *= ratio;
    }
    if proportional.is_axis_proportional(axis3) {
        new_size[axis3] *= ratio;
    }

    let anchor = if anchor_type == AnchorPos::Center {
        box_.center()
    } else {
        center_for_bbox_side(box_, -normal)
    };

    let matrix = geometry_math::mat_ext::scale_bbox_matrix_with_anchor(box_, new_size, anchor);
    Some(Bbox3d::new_repaired(matrix * box_.min, matrix * box_.max))
}

/// `moveBBoxCorner`.
pub fn move_bbox_corner(
    box_: &Bbox3d,
    corner: Vec3d,
    delta: Vec3d,
    anchor_type: AnchorPos,
) -> Option<Bbox3d> {
    let opposite_point = point_for_bbox_corner(box_, -corner);
    let anchor = if anchor_type == AnchorPos::Center {
        box_.center()
    } else {
        opposite_point
    };
    let old_corner = point_for_bbox_corner(box_, corner);
    let new_corner = old_corner + delta;

    // Check for inverting the box.
    for axis in 0..3 {
        if new_corner[axis] == anchor[axis] {
            return None;
        }
        let old_positive = old_corner[axis] > anchor[axis];
        let new_positive = new_corner[axis] > anchor[axis];
        if old_positive != new_positive {
            return None;
        }
    }

    let points = if anchor_type == AnchorPos::Center {
        [anchor - (new_corner - anchor), new_corner]
    } else {
        [opposite_point, new_corner]
    };
    Bbox3d::merge_all(points)
}

/// `moveBBoxEdge`.
pub fn move_bbox_edge(
    box_: &Bbox3d,
    first: Vec3d,
    second: Vec3d,
    delta: Vec3d,
    proportional: ProportionalAxes,
    anchor_type: AnchorPos,
) -> Option<Bbox3d> {
    let (a, b) = points_for_bbox_edge(box_, first, second);
    let edge_mid = (a + b) / 2.0;
    let (oa, ob) = points_for_bbox_edge(box_, -first, -second);
    let opposite_edge_mid = (oa + ob) / 2.0;

    let anchor = if anchor_type == AnchorPos::Center {
        box_.center()
    } else {
        opposite_edge_mid
    };

    let old_anchor_dist = edge_mid - anchor;
    let new_anchor_dist = old_anchor_dist + delta;

    // Check for crossing over the anchor.
    for axis in 0..3 {
        if old_anchor_dist[axis] > 0.0 && new_anchor_dist[axis] < 0.0 {
            return None;
        }
        if old_anchor_dist[axis] < 0.0 && new_anchor_dist[axis] > 0.0 {
            return None;
        }
    }

    let non_moving_axis = old_anchor_dist.find_abs_max_component(2);

    let corner1 = if anchor_type == AnchorPos::Center {
        anchor - new_anchor_dist
    } else {
        anchor
    };
    let corner2 = anchor + new_anchor_dist;

    let mut p1 = vec::min(corner1, corner2);
    let mut p2 = vec::max(corner1, corner2);

    // The only type of proportional scaling we support is optionally scaling the
    // nonMovingAxis.
    if proportional.is_axis_proportional(non_moving_axis) {
        let axis1 = old_anchor_dist.find_abs_max_component(0);
        let ratio = (p2 - p1)[axis1] / box_.size()[axis1];

        p1[non_moving_axis] = anchor[non_moving_axis] - (box_.size()[non_moving_axis] * ratio * 0.5);
        p2[non_moving_axis] = anchor[non_moving_axis] + (box_.size()[non_moving_axis] * ratio * 0.5);
    } else {
        p1[non_moving_axis] = box_.min[non_moving_axis];
        p2[non_moving_axis] = box_.max[non_moving_axis];
    }

    let result = Bbox3d::new(vec::min(p1, p2), vec::max(p1, p2));

    // Check for zero size.
    (result.size() != Vec3d::zero()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> Bbox3d {
        Bbox3d::from_scalars(-32.0, 32.0)
    }

    #[test]
    fn a_box_has_six_sides_twelve_edges_and_eight_corners() {
        assert_eq!(all_sides().len(), 6);
        assert_eq!(all_edges().len(), 12);
        assert_eq!(all_corners().len(), 8);
        assert_eq!(all_handles().len(), 26);
    }

    #[test]
    fn a_handle_sits_where_the_unit_box_says_it_does() {
        let box_ = cube();

        geometry_math::assert_approx_eq!(
            BBoxHandle::Side(Vec3d::new(0.0, 0.0, 1.0)).position(&box_),
            Vec3d::new(0.0, 0.0, 32.0),
            1e-9
        );
        geometry_math::assert_approx_eq!(
            BBoxHandle::Corner(Vec3d::new(1.0, -1.0, 1.0)).position(&box_),
            Vec3d::new(32.0, -32.0, 32.0),
            1e-9
        );
        geometry_math::assert_approx_eq!(
            BBoxHandle::Edge(Vec3d::new(1.0, 1.0, 1.0), Vec3d::new(1.0, 1.0, -1.0))
                .position(&box_),
            Vec3d::new(32.0, 32.0, 0.0),
            1e-9
        );
    }

    #[test]
    fn dragging_a_side_moves_only_that_side() {
        let dragged = move_bbox_side(
            &cube(),
            Vec3d::new(0.0, 0.0, 1.0),
            Vec3d::new(0.0, 0.0, 16.0),
            ProportionalAxes::none(),
            AnchorPos::Opposite,
        )
        .expect("a box");

        geometry_math::assert_approx_eq!(dragged.min, Vec3d::fill(-32.0), 1e-9);
        geometry_math::assert_approx_eq!(dragged.max, Vec3d::new(32.0, 32.0, 48.0), 1e-9);
    }

    #[test]
    fn an_anchored_centre_moves_both_sides() {
        let dragged = move_bbox_side(
            &cube(),
            Vec3d::new(0.0, 0.0, 1.0),
            Vec3d::new(0.0, 0.0, 16.0),
            ProportionalAxes::none(),
            AnchorPos::Center,
        )
        .expect("a box");

        // The delta counts twice, once for each side, and the middle stays put.
        geometry_math::assert_approx_eq!(dragged.min, Vec3d::new(-32.0, -32.0, -48.0), 1e-9);
        geometry_math::assert_approx_eq!(dragged.max, Vec3d::new(32.0, 32.0, 48.0), 1e-9);
    }

    #[test]
    fn a_proportional_axis_follows_the_one_being_dragged() {
        let proportional = ProportionalAxes::new(true, false, false);

        // The top goes from 64 to 96 units, half again, so X follows and Y does not.
        let dragged = move_bbox_side(
            &cube(),
            Vec3d::new(0.0, 0.0, 1.0),
            Vec3d::new(0.0, 0.0, 32.0),
            proportional,
            AnchorPos::Opposite,
        )
        .expect("a box");

        geometry_math::assert_approx_eq!(dragged.size(), Vec3d::new(96.0, 64.0, 96.0), 1e-9);
    }

    #[test]
    fn a_side_dragged_through_the_box_is_refused() {
        assert_eq!(
            move_bbox_side(
                &cube(),
                Vec3d::new(0.0, 0.0, 1.0),
                Vec3d::new(0.0, 0.0, -64.0),
                ProportionalAxes::none(),
                AnchorPos::Opposite,
            ),
            None
        );
    }

    #[test]
    fn dragging_a_corner_moves_three_sides() {
        let corner = Vec3d::new(1.0, 1.0, 1.0);
        let dragged = move_bbox_corner(
            &cube(),
            corner,
            Vec3d::new(16.0, 16.0, 16.0),
            AnchorPos::Opposite,
        )
        .expect("a box");

        geometry_math::assert_approx_eq!(dragged.min, Vec3d::fill(-32.0), 1e-9);
        geometry_math::assert_approx_eq!(dragged.max, Vec3d::fill(48.0), 1e-9);
    }

    #[test]
    fn a_corner_dragged_past_its_anchor_is_refused() {
        assert_eq!(
            move_bbox_corner(
                &cube(),
                Vec3d::new(1.0, 1.0, 1.0),
                Vec3d::new(-96.0, 0.0, 0.0),
                AnchorPos::Opposite,
            ),
            None
        );
        // Landing exactly on the anchor would flatten the box.
        assert_eq!(
            move_bbox_corner(
                &cube(),
                Vec3d::new(1.0, 1.0, 1.0),
                Vec3d::new(-64.0, 0.0, 0.0),
                AnchorPos::Opposite,
            ),
            None
        );
    }

    #[test]
    fn dragging_an_edge_moves_two_sides_and_leaves_the_third_axis_alone() {
        // The edge along Z at +X +Y.
        let dragged = move_bbox_edge(
            &cube(),
            Vec3d::new(1.0, 1.0, 1.0),
            Vec3d::new(1.0, 1.0, -1.0),
            Vec3d::new(16.0, 16.0, 0.0),
            ProportionalAxes::none(),
            AnchorPos::Opposite,
        )
        .expect("a box");

        geometry_math::assert_approx_eq!(dragged.min, Vec3d::fill(-32.0), 1e-9);
        geometry_math::assert_approx_eq!(dragged.max, Vec3d::new(48.0, 48.0, 32.0), 1e-9);
    }

    #[test]
    fn every_handle_has_one_across_the_box_from_it() {
        let box_ = cube();
        for handle in all_handles() {
            let opposite = handle.opposite();
            // Mirroring twice is where it started.
            geometry_math::assert_approx_eq!(
                opposite.opposite().position(&box_),
                handle.position(&box_),
                1e-9
            );
            // And the two are on opposite sides of the middle.
            let from_centre = handle.position(&box_) - box_.center();
            let other = opposite.position(&box_) - box_.center();
            geometry_math::assert_approx_eq!(other, -from_centre, 1e-9);
        }
    }

    #[test]
    fn a_handle_moves_along_the_line_from_the_one_across_the_box() {
        let box_ = cube();

        // A side runs along its own normal, from the middle of the far side.
        let side = BBoxHandle::Side(Vec3d::new(0.0, 0.0, 1.0));
        let line = side.drag_line(&box_);
        geometry_math::assert_approx_eq!(line.direction, Vec3d::new(0.0, 0.0, 1.0), 1e-9);
        geometry_math::assert_approx_eq!(line.point, Vec3d::new(0.0, 0.0, 32.0), 1e-9);

        // A corner runs along the box's diagonal.
        let corner = BBoxHandle::Corner(Vec3d::new(1.0, 1.0, 1.0));
        let line = corner.drag_line(&box_);
        geometry_math::assert_approx_eq!(line.point, Vec3d::fill(-32.0), 1e-9);
        geometry_math::assert_approx_eq!(
            line.direction,
            vec::normalize(Vec3d::fill(1.0)),
            1e-9
        );

        // And every line passes through the middle of the box.
        for handle in all_handles() {
            let line = handle.drag_line(&box_);
            let to_centre = box_.center() - line.point;
            let along = vec::dot(to_centre, line.direction);
            geometry_math::assert_approx_eq!(line.point + line.direction * along, box_.center(), 1e-9);
        }
    }

    #[test]
    fn every_handle_can_be_dragged_outwards() {
        let box_ = cube();
        for handle in all_handles() {
            // Away from the middle, so no handle is pushed through the box.
            let outwards = (handle.position(&box_) - box_.center()) * 0.25;
            let dragged = handle
                .drag(
                    &box_,
                    outwards,
                    ProportionalAxes::none(),
                    AnchorPos::Opposite,
                )
                .unwrap_or_else(|| panic!("{handle:?} could not be dragged"));
            assert!(dragged.size().x() > 0.0);
            assert!(dragged.size().y() > 0.0);
            assert!(dragged.size().z() > 0.0);
        }
    }
}
