//! The clip tool: cutting the selected brushes along a plane placed by hand.
//!
//! Three points define the cutting plane, but two are usually enough: the third is worked
//! out from the *help vectors* recorded with each point — in a 2D view the direction the
//! camera looks, in the 3D view the axis each face under the click faces along. Which
//! axis those vectors mostly agree on is the direction the plane runs away in. That is
//! what makes clipping in a 2D view work at all, since a 2D view can only ever place
//! points on one plane.
//!
//! Both halves are always computed. Which of them survives is the *clip side*, cycled
//! with a key, so the same two points can keep the front, the back, or both.

use geometry_math::bbox::Bbox3d;
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec3d};
use map_model::brush::{Brush, ClipOutcome};

use crate::i18n::translation_key;

/// Which side of the plane is kept.
///
/// `ClipTool::ClipSide`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClipSide {
    /// Keep what is in front of the plane.
    #[default]
    Front,
    /// Keep both halves, as separate brushes.
    Both,
    /// Keep what is behind the plane.
    Back,
}

impl ClipSide {
    /// What the status line says when this side is chosen, as a key for [`crate::i18n::tr`].
    pub fn kept(self) -> &'static str {
        match self {
            ClipSide::Front => translation_key("The clip keeps the front part"),
            ClipSide::Both => translation_key("The clip keeps both parts"),
            ClipSide::Back => translation_key("The clip keeps the back part"),
        }
    }

    /// `ClipTool::toggleSide`: front, then both, then back, then round again.
    fn next(self) -> Self {
        match self {
            ClipSide::Front => ClipSide::Both,
            ClipSide::Both => ClipSide::Back,
            ClipSide::Back => ClipSide::Front,
        }
    }

    fn keeps_front(self) -> bool {
        self != ClipSide::Back
    }

    fn keeps_back(self) -> bool {
        self != ClipSide::Front
    }
}

/// A point of the clip plane, and what the click that placed it suggests about the
/// direction the plane runs away in.
#[derive(Debug, Clone, PartialEq)]
struct ClipPoint {
    point: Vec3d,
    help_vectors: Vec<Vec3d>,
}

/// How the clip plane is being described.
///
/// The supported clipping strategies.
#[derive(Debug, Clone, PartialEq)]
enum Strategy {
    /// Points placed by clicking, up to three.
    Points(Vec<ClipPoint>),
    /// The plane of a brush face that was clicked.
    Face([Vec3d; 3]),
}

/// The clip tool's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClipTool {
    clip_side: ClipSide,
    strategy: Option<Strategy>,
    /// The index of the point being dragged, and what it was before the drag began.
    drag: Option<(usize, ClipPoint)>,
}

/// How far along the help vector the worked-out third point is placed.
///
/// The distance does not affect the plane; 128 keeps the auxiliary point close to the others.
const THIRD_POINT_DISTANCE: f64 = 128.0;

impl ClipTool {
    /// Creates a tool with nothing placed yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns which side of the plane is kept.
    pub fn clip_side(&self) -> ClipSide {
        self.clip_side
    }

    /// Moves on to the next clip side, if there is a plane for it to mean anything about.
    pub fn toggle_side(&mut self) -> bool {
        if !self.can_clip() {
            return false;
        }
        self.clip_side = self.clip_side.next();
        true
    }

    /// Returns the points placed so far, for drawing them.
    pub fn points(&self) -> Vec<Vec3d> {
        match &self.strategy {
            Some(Strategy::Points(points)) => points.iter().map(|p| p.point).collect(),
            Some(Strategy::Face(points)) => points.to_vec(),
            None => Vec::new(),
        }
    }

    /// Returns whether any point has been placed.
    pub fn has_points(&self) -> bool {
        match &self.strategy {
            Some(Strategy::Points(points)) => !points.is_empty(),
            // A face's points were not placed by hand, so there is nothing to take back.
            Some(Strategy::Face(_)) | None => false,
        }
    }

    /// The three points of the cutting plane, worked-out third included.
    ///
    /// `ClipStrategy::getPoints`.
    fn plane_points(&self) -> Option<[Vec3d; 3]> {
        match &self.strategy {
            Some(Strategy::Face(points)) => Some(*points),
            Some(Strategy::Points(points)) => match points.len() {
                3 => Some([points[0].point, points[1].point, points[2].point]),
                2 => self
                    .compute_third_point()
                    .map(|third| [points[0].point, points[1].point, third]),
                _ => None,
            },
            None => None,
        }
    }

    /// The third point, worked out from the two placed and the help vectors.
    ///
    /// `PointClipStrategy::computeThirdPoint`.
    fn compute_third_point(&self) -> Option<Vec3d> {
        let Some(Strategy::Points(points)) = &self.strategy else {
            return None;
        };
        if points.len() != 2 {
            return None;
        }

        let point = points[1].point + self.compute_help_vector() * THIRD_POINT_DISTANCE;
        (!vec::is_colinear(points[0].point, points[1].point, point, f64::ALMOST_ZERO))
            .then_some(point)
    }

    /// The axis the help vectors mostly agree on.
    ///
    /// `PointClipStrategy::computeHelpVector`. The six counts are the three axes in each
    /// direction; the winner's axis is the answer, and the tie-breaking prefers Z, then X,
    /// because a clip plane that runs vertically is by far the most common.
    fn compute_help_vector(&self) -> Vec3d {
        let Some(Strategy::Points(points)) = &self.strategy else {
            return Vec3d::new(0.0, 0.0, 1.0);
        };

        let mut counts = [0usize; 6];
        for help_vector in points.iter().flat_map(|point| &point.help_vectors) {
            let axis = help_vector.find_abs_max_component(0);
            let index = if help_vector[axis] > 0.0 {
                axis
            } else {
                axis + 3
            };
            counts[index] += 1;
        }

        let first_index = max_index(&counts);
        let next_index = max_index_after(&counts, first_index);

        match next_index {
            None => axis_vector(first_index % 3),
            Some(next_index) if counts[first_index] > counts[next_index] => {
                axis_vector(first_index % 3)
            }
            Some(next_index) => {
                // Two counts are equal.
                if first_index % 3 == 2 || next_index % 3 == 2 {
                    // Prefer the Z axis if possible.
                    Vec3d::new(0.0, 0.0, 1.0)
                } else {
                    // Z cannot win, so X and Y are a tie; prefer X.
                    Vec3d::new(1.0, 0.0, 0.0)
                }
            }
        }
    }

    /// Returns whether there is a plane to clip along.
    pub fn can_clip(&self) -> bool {
        self.plane_points().is_some()
    }

    /// Returns whether a point may be placed here.
    ///
    /// `PointClipStrategy::canAddPoint`: no more than three, no two the same, and the
    /// third not on the line through the first two.
    pub fn can_add_point(&self, point: Vec3d) -> bool {
        let Some(Strategy::Points(points)) = &self.strategy else {
            // With no strategy yet, or a face's plane, the first point starts one afresh.
            return true;
        };

        let room = points.len() < 2
            || (points.len() == 2
                && !vec::is_colinear(points[0].point, points[1].point, point, f64::ALMOST_ZERO));

        room && !points
            .iter()
            .any(|p| vec::is_equal(&p.point, &point, f64::ALMOST_ZERO))
    }

    /// Places a point, with what the click suggests about the plane's direction.
    pub fn add_point(&mut self, point: Vec3d, help_vectors: Vec<Vec3d>) -> bool {
        if !self.can_add_point(point) {
            return false;
        }

        let points = match &mut self.strategy {
            Some(Strategy::Points(points)) => points,
            _ => {
                self.strategy = Some(Strategy::Points(Vec::new()));
                match &mut self.strategy {
                    Some(Strategy::Points(points)) => points,
                    _ => unreachable!("just set"),
                }
            }
        };

        points.push(ClipPoint {
            point,
            help_vectors,
        });
        true
    }

    /// Takes back the last point placed.
    pub fn remove_last_point(&mut self) -> bool {
        let Some(Strategy::Points(points)) = &mut self.strategy else {
            return false;
        };
        if points.pop().is_none() {
            return false;
        }
        if points.is_empty() {
            self.strategy = None;
        }
        true
    }

    /// Cuts along the plane of a face instead of along placed points.
    ///
    /// `ClipTool::setFace`.
    pub fn set_face(&mut self, points: [Vec3d; 3]) {
        self.strategy = Some(Strategy::Face(points));
    }

    /// Forgets the plane, keeping the tool active.
    ///
    /// `ClipTool::reset`. Returns whether there was anything to forget, which is what
    /// decides whether Escape leaves the tool as well.
    pub fn reset(&mut self) -> bool {
        let had_strategy = self.strategy.is_some();
        self.strategy = None;
        self.drag = None;
        had_strategy
    }

    /// Starts dragging the point nearest the given position, within `radius`.
    ///
    /// `ClipTool::beginDragPoint`, with the pick result replaced by a distance test,
    /// since a 2D view has no pick result to consult.
    pub fn begin_drag_point(&mut self, near: Vec3d, radius: f64) -> Option<Vec3d> {
        let Some(Strategy::Points(points)) = &self.strategy else {
            return None;
        };

        let (index, found) = points
            .iter()
            .enumerate()
            .filter(|(_, p)| vec::distance(p.point, near) <= radius)
            .min_by(|(_, a), (_, b)| {
                vec::squared_distance(a.point, near)
                    .partial_cmp(&vec::squared_distance(b.point, near))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?;

        let position = found.point;
        self.drag = Some((index, found.clone()));
        Some(position)
    }

    /// Moves the point being dragged.
    ///
    /// `PointClipStrategy::dragPoint`: a point may not be dragged onto another, nor onto
    /// the line through the other two.
    pub fn drag_point(&mut self, new_position: Vec3d, help_vectors: Vec<Vec3d>) -> bool {
        let Some((index, _)) = self.drag.clone() else {
            return false;
        };
        let Some(Strategy::Points(points)) = &mut self.strategy else {
            return false;
        };

        for (i, other) in points.iter().enumerate() {
            if i != index && vec::is_equal(&other.point, &new_position, f64::ALMOST_ZERO) {
                return false;
            }
        }

        if points.len() == 3 {
            let first = (index + 1) % 3;
            let second = (index + 2) % 3;
            if vec::is_colinear(
                points[first].point,
                points[second].point,
                new_position,
                f64::ALMOST_ZERO,
            ) {
                return false;
            }
        }

        points[index] = ClipPoint {
            point: new_position,
            help_vectors,
        };
        true
    }

    /// Ends the drag, keeping where the point was left.
    pub fn end_drag_point(&mut self) {
        self.drag = None;
    }

    /// Ends the drag, putting the point back where it started.
    pub fn cancel_drag_point(&mut self) {
        if let Some((index, original)) = self.drag.take() {
            if let Some(Strategy::Points(points)) = &mut self.strategy {
                if index < points.len() {
                    points[index] = original;
                }
            }
        }
    }

    /// Returns whether a drag is under way.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Cuts one brush, returning what survives on the chosen side.
    ///
    /// `ClipTool::updateBrushes`, which clips the same brush twice: once along the plane
    /// as given, once along it reversed. Both halves are wanted whenever the side is
    /// `Both`, and computing both regardless is what lets the side be changed without
    /// having to place the points again.
    ///
    /// Clipping keeps what is *behind* a plane, since that is how a brush is defined --
    /// the part behind every one of its faces. So the front half is what lies behind the
    /// plane through the points in the order they were placed, and reversing two of them
    /// gives the other half.
    pub fn clip_brush(&self, _world_bounds: &Bbox3d, brush: &Brush) -> Vec<Brush> {
        let Some([p1, p2, p3]) = self.plane_points() else {
            return vec![brush.clone()];
        };
        let mut result = Vec::new();
        if self.clip_side.keeps_front() {
            result.extend(Self::half(brush, p1, p2, p3));
        }
        // The same plane the other way round, which is how the far half is kept.
        if self.clip_side.keeps_back() {
            result.extend(Self::half(brush, p1, p3, p2));
        }
        result
    }

    /// The halves the chosen side would throw away, for drawing them faded.
    ///
    /// `ClipTool::updateBrushes` keeps these too and renders them ghosted, so it is clear
    /// what a clip on the current side removes. Nothing when the side is `Both`, since
    /// then nothing is removed.
    pub fn clipped_away(&self, brush: &Brush) -> Vec<Brush> {
        let Some([p1, p2, p3]) = self.plane_points() else {
            return Vec::new();
        };
        let mut result = Vec::new();
        if !self.clip_side.keeps_front() {
            result.extend(Self::half(brush, p1, p2, p3));
        }
        if !self.clip_side.keeps_back() {
            result.extend(Self::half(brush, p1, p3, p2));
        }
        result
    }

    /// One side of `brush`: the part behind the plane through `a`, `b`, `c`.
    ///
    /// A plane that does not cut the brush leaves it whole on one side and takes it away
    /// entirely on the other, which is what `Unchanged` and `Empty` mean here.
    fn half(brush: &Brush, a: Vec3d, b: Vec3d, c: Vec3d) -> Option<Brush> {
        geometry_math::plane::from_points(a, b, c).and_then(|plane| match brush.clip(&plane) {
            ClipOutcome::Clipped(half) => Some(half),
            ClipOutcome::Unchanged => Some(brush.clone()),
            ClipOutcome::Empty => None,
        })
    }
}

/// The index of the largest count, the first if several are equal.
fn max_index(counts: &[usize; 6]) -> usize {
    let mut best = 0;
    for (index, count) in counts.iter().enumerate() {
        if *count > counts[best] {
            best = index;
        }
    }
    best
}

/// The index of the largest count after `after`, if there is anything after it.
fn max_index_after(counts: &[usize; 6], after: usize) -> Option<usize> {
    let mut best: Option<usize> = None;
    for index in (after + 1)..counts.len() {
        if best.is_none_or(|current| counts[index] > counts[current]) {
            best = Some(index);
        }
    }
    best
}

fn axis_vector(axis: usize) -> Vec3d {
    let mut v = Vec3d::zero();
    v[axis] = 1.0;
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_model::shapes;

    fn world() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    fn cube() -> Brush {
        shapes::create_cuboid(&world(), &Bbox3d::from_scalars(-32.0, 32.0), "stone")
            .expect("a cube")
    }

    /// Looking down Z, as the top view does.
    fn looking_down() -> Vec<Vec3d> {
        vec![Vec3d::new(0.0, 0.0, -1.0)]
    }

    #[test]
    fn a_new_tool_has_nothing_to_clip_with() {
        let tool = ClipTool::new();
        assert!(!tool.can_clip());
        assert!(!tool.has_points());
        assert_eq!(tool.points(), Vec::<Vec3d>::new());
        assert_eq!(tool.clip_side(), ClipSide::Front);
    }

    #[test]
    fn two_points_and_a_help_vector_are_enough_to_clip() {
        // This is the whole point of the help vectors: a 2D view can only place points on
        // one plane, so the third has to come from the direction the view looks.
        let mut tool = ClipTool::new();
        assert!(tool.add_point(Vec3d::new(-32.0, 0.0, 0.0), looking_down()));
        assert!(!tool.can_clip());
        assert!(tool.add_point(Vec3d::new(32.0, 0.0, 0.0), looking_down()));
        assert!(tool.can_clip());

        // The plane runs away along Z, the axis the view looks down.
        // The plane runs along Z, so both halves keep the cube's full height.
        let halves = tool.clip_brush(&world(), &cube());
        geometry_math::assert_approx_eq!(halves[0].bounds().size().z(), 64.0, 1e-6);
    }

    #[test]
    fn a_point_may_not_land_on_another() {
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(0.0, 0.0, 0.0), looking_down());

        assert!(!tool.can_add_point(Vec3d::new(0.0, 0.0, 0.0)));
        assert!(!tool.add_point(Vec3d::new(0.0, 0.0, 0.0), looking_down()));
        assert_eq!(tool.points().len(), 1);
    }

    #[test]
    fn a_third_point_may_not_land_on_the_line_through_the_first_two() {
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(-32.0, 0.0, 0.0), looking_down());
        tool.add_point(Vec3d::new(32.0, 0.0, 0.0), looking_down());

        assert!(!tool.can_add_point(Vec3d::new(64.0, 0.0, 0.0)));
        assert!(tool.can_add_point(Vec3d::new(0.0, 32.0, 0.0)));
    }

    #[test]
    fn points_can_be_taken_back_one_at_a_time() {
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(-32.0, 0.0, 0.0), looking_down());
        tool.add_point(Vec3d::new(32.0, 0.0, 0.0), looking_down());

        assert!(tool.remove_last_point());
        assert_eq!(tool.points().len(), 1);
        assert!(tool.remove_last_point());
        assert!(!tool.has_points());
        assert!(!tool.remove_last_point());
    }

    #[test]
    fn the_help_vector_follows_whichever_axis_the_points_agree_on() {
        // Two points placed in the front view, which looks along Y.
        let mut tool = ClipTool::new();
        let along_y = vec![Vec3d::new(0.0, 1.0, 0.0)];
        tool.add_point(Vec3d::new(-32.0, 0.0, -32.0), along_y.clone());
        tool.add_point(Vec3d::new(32.0, 0.0, 32.0), along_y);

        geometry_math::assert_approx_eq!(
            tool.compute_help_vector(),
            Vec3d::new(0.0, 1.0, 0.0),
            1e-9
        );
    }

    #[test]
    fn a_tie_between_two_axes_prefers_z_and_then_x() {
        let mut tool = ClipTool::new();
        // One vote for X and one for Y: neither is Z, so X wins.
        tool.add_point(Vec3d::new(0.0, 0.0, 0.0), vec![Vec3d::new(1.0, 0.0, 0.0)]);
        tool.add_point(Vec3d::new(0.0, 32.0, 0.0), vec![Vec3d::new(0.0, 1.0, 0.0)]);
        geometry_math::assert_approx_eq!(
            tool.compute_help_vector(),
            Vec3d::new(1.0, 0.0, 0.0),
            1e-9
        );

        // One vote for X and one for Z: Z wins.
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(0.0, 0.0, 0.0), vec![Vec3d::new(1.0, 0.0, 0.0)]);
        tool.add_point(Vec3d::new(0.0, 32.0, 0.0), vec![Vec3d::new(0.0, 0.0, 1.0)]);
        geometry_math::assert_approx_eq!(
            tool.compute_help_vector(),
            Vec3d::new(0.0, 0.0, 1.0),
            1e-9
        );
    }

    #[test]
    fn the_clip_side_cycles_only_once_there_is_a_plane() {
        let mut tool = ClipTool::new();
        assert!(!tool.toggle_side());
        assert_eq!(tool.clip_side(), ClipSide::Front);

        tool.add_point(Vec3d::new(-32.0, 0.0, 0.0), looking_down());
        tool.add_point(Vec3d::new(32.0, 0.0, 0.0), looking_down());

        assert!(tool.toggle_side());
        assert_eq!(tool.clip_side(), ClipSide::Both);
        assert!(tool.toggle_side());
        assert_eq!(tool.clip_side(), ClipSide::Back);
        assert!(tool.toggle_side());
        assert_eq!(tool.clip_side(), ClipSide::Front);
    }

    #[test]
    fn clipping_a_cube_in_half_keeps_the_side_that_was_chosen() {
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(0.0, -32.0, 0.0), looking_down());
        tool.add_point(Vec3d::new(0.0, 32.0, 0.0), looking_down());

        let front = tool.clip_brush(&world(), &cube());
        assert_eq!(front.len(), 1);
        // Half a cube either way, so half the volume of its bounds.
        geometry_math::assert_approx_eq!(front[0].bounds().size().x(), 32.0, 1e-6);

        tool.toggle_side();
        assert_eq!(tool.clip_side(), ClipSide::Both);
        let both = tool.clip_brush(&world(), &cube());
        assert_eq!(both.len(), 2);
        geometry_math::assert_approx_eq!(both[0].bounds().size().x(), 32.0, 1e-6);
        geometry_math::assert_approx_eq!(both[1].bounds().size().x(), 32.0, 1e-6);
        // The two halves are on opposite sides and together fill the original.
        let merged = both[0].bounds().merged(&both[1].bounds());
        geometry_math::assert_approx_eq!(merged.size().x(), 64.0, 1e-6);

        tool.toggle_side();
        assert_eq!(tool.clip_side(), ClipSide::Back);
        let back = tool.clip_brush(&world(), &cube());
        assert_eq!(back.len(), 1);
        // The other half from the one the front kept.
        assert!(back[0].bounds().min.x() != front[0].bounds().min.x());
    }

    #[test]
    fn the_ghost_half_is_the_opposite_of_the_kept_one() {
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(0.0, -32.0, 0.0), looking_down());
        tool.add_point(Vec3d::new(0.0, 32.0, 0.0), looking_down());

        // Keeping the front, the ghost is the back, and the two meet nowhere.
        let kept = tool.clip_brush(&world(), &cube());
        let ghost = tool.clipped_away(&cube());
        assert_eq!(ghost.len(), 1);
        assert!(ghost[0].bounds().min.x() != kept[0].bounds().min.x());

        // Keeping both halves throws nothing away.
        tool.toggle_side();
        assert_eq!(tool.clip_side(), ClipSide::Both);
        assert!(tool.clipped_away(&cube()).is_empty());

        // Keeping the back, the ghost is the front.
        tool.toggle_side();
        assert_eq!(tool.clip_side(), ClipSide::Back);
        let back = tool.clip_brush(&world(), &cube());
        let ghost = tool.clipped_away(&cube());
        assert_eq!(ghost.len(), 1);
        assert!(ghost[0].bounds().min.x() != back[0].bounds().min.x());
    }

    #[test]
    fn nothing_to_clip_along_means_no_ghost() {
        // No points placed, so there is no plane and nothing to show either way.
        assert!(ClipTool::new().clipped_away(&cube()).is_empty());
    }

    #[test]
    fn a_plane_that_misses_a_brush_leaves_it_whole_on_one_side_and_gone_on_the_other() {
        // The plane is well clear of the cube, so one half is the whole cube and the
        // other is nothing at all.
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(0.0, -128.0, 0.0), looking_down());
        tool.add_point(Vec3d::new(64.0, -128.0, 0.0), looking_down());

        assert_eq!(tool.clip_brush(&world(), &cube()).len(), 0);

        tool.toggle_side();
        tool.toggle_side();
        assert_eq!(tool.clip_side(), ClipSide::Back);
        let back = tool.clip_brush(&world(), &cube());
        assert_eq!(back.len(), 1);
        geometry_math::assert_approx_eq!(
            back[0].bounds().size(),
            Vec3d::new(64.0, 64.0, 64.0),
            1e-6
        );
    }

    #[test]
    fn a_face_can_stand_in_for_the_points() {
        let mut tool = ClipTool::new();
        tool.set_face([
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.0, 32.0, 0.0),
            Vec3d::new(0.0, 0.0, 32.0),
        ]);

        assert!(tool.can_clip());
        // A face's points were not placed by hand, so there is nothing to take back.
        assert!(!tool.has_points());
        assert!(!tool.remove_last_point());
        assert_eq!(tool.clip_brush(&world(), &cube()).len(), 1);
    }

    #[test]
    fn dragging_a_point_moves_it_but_not_onto_another() {
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(-32.0, 0.0, 0.0), looking_down());
        tool.add_point(Vec3d::new(32.0, 0.0, 0.0), looking_down());

        assert_eq!(
            tool.begin_drag_point(Vec3d::new(-30.0, 0.0, 0.0), 8.0),
            Some(Vec3d::new(-32.0, 0.0, 0.0))
        );
        assert!(tool.is_dragging());

        assert!(tool.drag_point(Vec3d::new(-64.0, 0.0, 0.0), looking_down()));
        assert_eq!(tool.points()[0], Vec3d::new(-64.0, 0.0, 0.0));

        // Onto the other point is refused, and leaves it where it was.
        assert!(!tool.drag_point(Vec3d::new(32.0, 0.0, 0.0), looking_down()));
        assert_eq!(tool.points()[0], Vec3d::new(-64.0, 0.0, 0.0));

        tool.end_drag_point();
        assert!(!tool.is_dragging());
    }

    #[test]
    fn a_cancelled_drag_puts_the_point_back() {
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(-32.0, 0.0, 0.0), looking_down());

        tool.begin_drag_point(Vec3d::new(-32.0, 0.0, 0.0), 8.0);
        tool.drag_point(Vec3d::new(-64.0, 0.0, 0.0), looking_down());
        tool.cancel_drag_point();

        assert_eq!(tool.points()[0], Vec3d::new(-32.0, 0.0, 0.0));
        assert!(!tool.is_dragging());
    }

    #[test]
    fn a_drag_reaches_only_a_point_within_the_radius() {
        let mut tool = ClipTool::new();
        tool.add_point(Vec3d::new(0.0, 0.0, 0.0), looking_down());

        assert_eq!(tool.begin_drag_point(Vec3d::new(64.0, 0.0, 0.0), 8.0), None);
        assert!(!tool.is_dragging());
    }

    #[test]
    fn resetting_forgets_the_plane() {
        let mut tool = ClipTool::new();
        assert!(!tool.reset());

        tool.add_point(Vec3d::new(0.0, 0.0, 0.0), looking_down());
        assert!(tool.reset());
        assert!(!tool.has_points());
        assert!(!tool.can_clip());
    }
}
