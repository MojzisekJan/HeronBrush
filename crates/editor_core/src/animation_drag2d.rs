//! Frontend-neutral pieces of direct pose manipulation in orthographic views: picking the
//! animated brushes, snapping, and multi-turn rotation. Rest brushes stay untouched.
use crate::{
    animation_curves::Clip,
    animation_pose::Pose,
    camera2d::{Camera2d, ViewPlane},
    document::Document,
    grid::Grid,
};
use geometry_math::vec::{Vec2d, Vec3d};

/// Within this many screen pixels of the pose centre a press grabs the pivot.
pub const PIVOT_GRAB: f64 = 10.0;
/// A press must travel this many screen pixels before a drag authors anything.
pub const DRAG_THRESHOLD: f64 = 3.0;

fn triangle_hit(p: Vec2d, a: Vec2d, b: Vec2d, c: Vec2d) -> bool {
    let cross = |a: Vec2d, b: Vec2d| a.x() * b.y() - a.y() * b.x();
    if cross(b - a, c - a).abs() < 1e-8 {
        return false;
    }
    let s = [cross(b - a, p - a), cross(c - b, p - b), cross(a - c, p - c)];
    s.iter().all(|x| *x >= -1e-8) || s.iter().all(|x| *x <= 1e-8)
}

/// Whether plane point `p` lies on one of the entity's brushes in their animated pose.
pub fn hit(d: &Document, pose: &Pose, camera: Camera2d, p: Vec2d) -> bool {
    let Ok(track) =
        crate::keyframe_animation::Track::from_properties("", "once", Some(&pose.clip.json()))
    else {
        return false;
    };
    map_model::node::contents_of(d.tree_world(), pose.entity)
        .into_iter()
        .any(|id| {
            let Some(brush) = d.brush(id) else {
                return false;
            };
            brush.geometry_faces().any(|(_, points)| {
                let v: Vec<_> = points
                    .into_iter()
                    .map(|v| {
                        camera
                            .plane
                            .project(track.transform_clamped(v, pose.pivot, pose.time, false))
                    })
                    .collect();
                (1..v.len().saturating_sub(1)).any(|i| triangle_hit(p, v[0], v[i], v[i + 1]))
            })
        })
}

/// The pose's clip with the position keyed `delta` further along each moved axis.
pub fn translated(pose: &Pose, delta: Vec3d) -> Result<Clip, String> {
    let mut draft = pose.clone();
    for axis in 0..3 {
        if delta[axis].abs() > 1e-10 {
            draft.clip = draft.keyed(axis, delta[axis])?;
        }
    }
    Ok(draft.clip)
}

/// Snaps the visible world coordinates, preserving the hidden axis even off grid.
pub fn snapped_position(start: Vec3d, plane: ViewPlane, delta: Vec2d, grid: &Grid) -> Vec3d {
    let mut result = start + plane.unproject(delta, 0.0);
    for axis in 0..3 {
        if axis != plane.depth_axis() {
            result[axis] = grid.snap_scalar(result[axis]);
        }
    }
    result
}

/// The pivot moved so the pose centre lands on the grid of the visible axes.
pub fn snapped_pivot(pose: &Pose, plane: ViewPlane, delta: Vec2d, grid: &Grid) -> Vec3d {
    pose.pivot + snapped_position(pose.centre(), plane, delta, grid) - pose.centre()
}

/// Accumulates the angle swept about the centre, so a drag can turn more than once.
#[derive(Clone, Copy, Debug)]
pub struct Turn {
    pub start: Vec2d,
    previous: f64,
    angle: f64,
}
impl Turn {
    /// A turn starting at `start`, the press relative to the centre in plane units.
    pub fn new(start: Vec2d) -> Self {
        Self { start, previous: start.y().atan2(start.x()), angle: 0.0 }
    }
    /// The total angle in radians after the pointer moved `delta` from the press.
    pub fn update(&mut self, delta: Vec2d) -> Option<f64> {
        let v = self.start + delta;
        if v.x() * v.x() + v.y() * v.y() < 1e-10 {
            return None;
        }
        let now = v.y().atan2(v.x());
        let step = (now - self.previous + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        self.angle += step;
        self.previous = now;
        Some(self.angle)
    }
}

/// The sign that turns a counter-clockwise screen angle into the depth axis rotation.
pub fn rotation_sign(plane: ViewPlane) -> f64 {
    if plane == ViewPlane::Front {
        -1.0
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Document {
        let mut d = Document::new(Default::default());
        for x in [0.0, 128.0] {
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &geometry_math::bbox::Bbox3d::new(
                    Vec3d::new(x - 32.0, -32.0, -32.0),
                    Vec3d::new(x + 32.0, 32.0, 32.0),
                ),
                "stone",
            )
            .unwrap();
            d.add_brushes(vec![brush]);
        }
        d.select_all();
        d.group_selection("Platform").unwrap();
        let id = d.create_brush_entity("func_animate").unwrap();
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 0 0 0 0 0 0");
        d.set_entity_property(id, "animation_pivot", "0 0 0");
        d
    }
    #[test]
    fn pivot_snaps_visible_world_axes_and_preserves_hidden_depth() {
        let d = document();
        let mut pose = Pose::selected(&d, 1.0).unwrap();
        pose.pivot = Vec3d::new(3.0, 5.0, 7.0);
        pose.clip = pose.keyed(0, 11.0).unwrap();
        for plane in [ViewPlane::Top, ViewPlane::Front, ViewPlane::Side] {
            let grid = Grid::default();
            let result = snapped_pivot(&pose, plane, Vec2d::new(19.0, -27.0), &grid);
            assert_eq!(result[plane.depth_axis()], pose.pivot[plane.depth_axis()]);
            let centre = result + pose.centre() - pose.pivot;
            for axis in 0..3 {
                if axis != plane.depth_axis() {
                    assert_eq!(centre[axis] % 16.0, 0.0);
                }
            }
        }
    }
    #[test]
    fn translation_uses_visible_axes_and_picks_the_animated_brush() {
        let d = document();
        let pose = Pose::selected(&d, 1.0).unwrap();
        for plane in [ViewPlane::Top, ViewPlane::Front, ViewPlane::Side] {
            let camera = Camera2d::new(plane, 8192.0);
            assert!(hit(&d, &pose, camera, Vec2d::zero()));
            assert!(!hit(&d, &pose, camera, Vec2d::new(500.0, 500.0)));
            let delta = plane.unproject(Vec2d::new(64.0, 96.0), 0.0);
            let clip = translated(&pose, delta).unwrap();
            for axis in 0..3 {
                assert_eq!(clip.sample(axis, 1.0), delta[axis]);
                assert_eq!(clip.sample(axis + 3, 1.0), 0.0);
            }
        }
    }
    #[test]
    fn a_turn_accumulates_past_half_a_revolution() {
        let mut turn = Turn::new(Vec2d::new(24.0, 0.0));
        for n in 1..=8 {
            let angle = n as f64 * std::f64::consts::FRAC_PI_2;
            let delta = Vec2d::new(24.0 * angle.cos() - 24.0, 24.0 * angle.sin());
            assert!((turn.update(delta).unwrap() - angle).abs() < 1e-8);
        }
    }
}
