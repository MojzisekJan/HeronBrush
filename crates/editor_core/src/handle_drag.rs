//! Where a handle may go while it is being dragged in the 3D view.
//!
//! A 2D view answers "where is the pointer in the world" by itself: it looks down one
//! axis, so the other two are read straight off the screen. The 3D view cannot -- a
//! pointer there is a ray, and a ray meets a handle nowhere in particular. So a drag in
//! 3D first decides on a surface for the handle to slide along, and then asks where the
//! ray meets it. Which surface that is, is what the modifier keys change: a plane by
//! default, the vertical line through the handle with Alt, and the dominant axis of the
//! drag so far with Shift.

use geometry_math::abstract_line::AbstractLine;
use geometry_math::intersection;
use geometry_math::line::Line3d;
use geometry_math::plane::Plane3d;
use geometry_math::ray::Ray3d;
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec3d};

use crate::camera3d::Camera3d;

/// The surface a dragged handle slides along.
///
/// The supported geometric constraints for picking a dragged handle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandlePicker {
    /// The handle runs along a line: one axis of movement.
    Line { line: Line3d, offset: Vec3d },
    /// The handle slides on a plane: two axes of movement.
    Plane { plane: Plane3d, offset: Vec3d },
    /// The handle runs around a circle, which is how a rotation ring is dragged.
    Circle {
        center: Vec3d,
        normal: Vec3d,
        radius: f64,
        offset: Vec3d,
    },
    /// The handle slides on a plane, but only the part of that movement along one
    /// direction counts.
    ///
    /// `makeCanonicalHandlePicker`, which is how a face is extruded in a flat view: the
    /// pointer moves anywhere on the screen, and what it means is how far along the face's
    /// own normal it has gone.
    Canonical {
        plane: Plane3d,
        offset: Vec3d,
        initial: Vec3d,
        direction: Vec3d,
    },
}

impl HandlePicker {
    /// `makeLineHandlePicker`.
    ///
    /// The offset is where on the handle the press landed. Shifting the line back by it
    /// and the answer forward again is what stops the handle jumping to sit under the
    /// pointer the moment a drag begins.
    pub fn line(line: Line3d, offset: Vec3d) -> Self {
        HandlePicker::Line {
            line: Line3d {
                point: line.point - offset,
                direction: line.direction,
            },
            offset,
        }
    }

    /// `makePlaneHandlePicker`.
    pub fn plane(plane: Plane3d, offset: Vec3d) -> Self {
        HandlePicker::Plane {
            plane: Plane3d::from_anchor_and_normal(plane.anchor() - offset, plane.normal),
            offset,
        }
    }

    /// `makeCanonicalHandlePicker`.
    pub fn canonical(plane: Plane3d, initial: Vec3d, direction: Vec3d, offset: Vec3d) -> Self {
        HandlePicker::Canonical {
            plane: Plane3d::from_anchor_and_normal(plane.anchor() - offset, plane.normal),
            offset,
            initial,
            direction,
        }
    }

    /// `makeCircleHandlePicker`.
    pub fn circle(center: Vec3d, normal: Vec3d, radius: f64, offset: Vec3d) -> Self {
        HandlePicker::Circle {
            center: center - offset,
            normal,
            radius,
            offset,
        }
    }

    /// Where the handle would go for a pointer casting the given ray.
    ///
    /// Nothing comes back when the ray tells us nothing: parallel to the line, or missing
    /// the plane.
    pub fn pick(&self, ray: &Ray3d) -> Option<Vec3d> {
        match self {
            HandlePicker::Line { line, offset } => {
                let distance = geometry_math::distance::distance_ray_line(ray, line);
                if distance.parallel {
                    return None;
                }
                Some(line.point + line.direction * distance.position2 + *offset)
            }

            HandlePicker::Plane { plane, offset } => {
                // Rejects rays nearly parallel to the drag plane, whose intersections would be
                // unstable and far from the cursor. The handle retains its previous position.
                if vec::dot(ray.direction, plane.normal).abs() < GRAZING {
                    return None;
                }
                let distance = intersection::intersect_ray_plane(ray, plane)?;
                Some(ray.point_at_distance(distance) + *offset)
            }

            HandlePicker::Canonical {
                plane,
                offset,
                initial,
                direction,
            } => {
                let distance = intersection::intersect_ray_plane(ray, plane)?;
                let on_plane = ray.point_at_distance(distance) + *offset;
                // Only the part of the movement along the direction counts; the rest of
                // where the pointer went says nothing about how far the face should go.
                let along = vec::dot(on_plane - *initial, *direction);
                Some(*initial + *direction * along)
            }

            HandlePicker::Circle {
                center,
                normal,
                radius,
                offset,
            } => {
                let plane = Plane3d::from_anchor_and_normal(*center, *normal);
                let distance = intersection::intersect_ray_plane(ray, &plane)?;
                let hit = ray.point_at_distance(distance);
                let direction = hit - *center;
                let length = vec::length(direction);
                if length <= f64::ALMOST_ZERO {
                    return None;
                }
                Some(*center + direction / length * *radius + *offset)
            }
        }
    }
}

/// How a move drag is constrained.
///
/// `MoveHandleDragDelegate::MoveMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MoveMode {
    /// Across the horizontal plane through the handle.
    #[default]
    Default,
    /// Straight up and down.
    Vertical,
    /// Along whichever axis the drag has gone furthest in so far.
    Constricted,
}

/// Which mode a drag is in, from the modifier keys and how far it has come.
///
/// `MoveHandleDragDelegate::moveMode`. Shift only constricts once the drag has a dominant
/// axis: while the two largest components are the same one, there is nothing to constrict
/// to.
pub fn move_mode(alt: bool, shift: bool, initial: Vec3d, current: Vec3d) -> MoveMode {
    if alt {
        return MoveMode::Vertical;
    }

    if shift {
        // `get_abs_max_component(delta, 0) != get_abs_max_component(delta, 1)`: the
        // largest component and the second largest, compared as values. While they are
        // the same -- both zero at the start of a drag -- there is no axis to constrict
        // to and the drag stays free.
        let delta = current - initial;
        let largest = delta[delta.find_abs_max_component(0)];
        let second = delta[delta.find_abs_max_component(1)];
        if largest != second {
            return MoveMode::Constricted;
        }
    }

    MoveMode::Default
}

/// The surface a move drag slides along, for the given mode.
///
/// `MoveHandleDragDelegate::makeDragHandlePicker`.
pub fn picker_for_move(
    mode: MoveMode,
    initial: Vec3d,
    current: Vec3d,
    offset: Vec3d,
) -> HandlePicker {
    let up = Vec3d::new(0.0, 0.0, 1.0);
    match mode {
        MoveMode::Vertical => HandlePicker::line(
            Line3d {
                point: current,
                direction: up,
            },
            offset,
        ),
        MoveMode::Constricted => {
            let delta = current - initial;
            HandlePicker::line(
                Line3d {
                    point: initial,
                    direction: delta.get_abs_max_component_axis(0),
                },
                offset,
            )
        }
        // A perspective camera moves things across the ground by default, which is what a
        // level is mostly built on.
        MoveMode::Default => {
            HandlePicker::plane(Plane3d::from_anchor_and_normal(current, up), offset)
        }
    }
}

/// How nearly along a plane a ray may run before it says nothing about where it meets it.
///
/// About three degrees: past that the intersection is thousands of units away and moves by
/// hundreds for every pixel the pointer travels.
const GRAZING: f64 = 0.05;

/// How much larger a handle has to be in the world to keep its size on screen.
///
/// `PerspectiveCamera::perspectiveScalingFactor`: how far the point is ahead of the
/// camera, over how far the viewport would have to be for one world unit to be one pixel.
pub fn perspective_scaling_factor(camera: &Camera3d, point: Vec3d, viewport_height: f64) -> f64 {
    camera.world_units_per_pixel(point, viewport_height)
}

/// How far along the ray a handle is, if the ray reaches it at all.
///
/// `Camera::pickPointHandle`: the handle is a sphere whose world radius grows with
/// distance, so what is really being tested is a fixed number of pixels on screen.
pub fn pick_point_handle(
    camera: &Camera3d,
    viewport_height: f64,
    ray: &Ray3d,
    handle: Vec3d,
    handle_radius: f64,
) -> Option<f64> {
    let scaling = perspective_scaling_factor(camera, handle, viewport_height);
    if scaling <= 0.0 {
        // Behind the camera, where nothing can be picked.
        return None;
    }
    intersection::intersect_ray_sphere(ray, handle, 2.0 * handle_radius * scaling)
}

/// The handle nearest along the ray, of those the ray reaches.
///
/// Every visible handle remains pickable regardless of its distance from the camera.
pub fn nearest_handle(
    camera: &Camera3d,
    viewport_height: f64,
    ray: &Ray3d,
    handles: impl IntoIterator<Item = Vec3d>,
    handle_radius: f64,
) -> Option<Vec3d> {
    handles
        .into_iter()
        .filter_map(|handle| {
            pick_point_handle(camera, viewport_height, ray, handle, handle_radius)
                .map(|distance| (handle, distance))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(handle, _)| handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ray(origin: Vec3d, direction: Vec3d) -> Ray3d {
        Ray3d {
            origin,
            direction: vec::normalize(direction),
        }
    }

    fn looking_down_from_above() -> Camera3d {
        Camera3d {
            position: Vec3d::new(0.0, 0.0, 256.0),
            yaw: 0.0,
            pitch: -std::f64::consts::FRAC_PI_2 + 0.01,
            ..Default::default()
        }
    }

    #[test]
    fn a_plane_picker_follows_the_ray_onto_the_plane() {
        let picker = HandlePicker::plane(
            Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0)),
            Vec3d::zero(),
        );

        let picked = picker
            .pick(&ray(
                Vec3d::new(16.0, 24.0, 100.0),
                Vec3d::new(0.0, 0.0, -1.0),
            ))
            .expect("the ray meets the plane");
        geometry_math::assert_approx_eq!(picked, Vec3d::new(16.0, 24.0, 0.0), 1e-9);
    }

    #[test]
    fn a_ray_along_a_plane_picks_nothing() {
        let picker = HandlePicker::plane(
            Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0)),
            Vec3d::zero(),
        );
        assert_eq!(
            picker.pick(&ray(Vec3d::new(0.0, 0.0, 32.0), Vec3d::new(1.0, 0.0, 0.0))),
            None
        );
    }

    #[test]
    fn a_line_picker_stays_on_its_line() {
        // The vertical line through the origin, seen from the side.
        let picker = HandlePicker::line(
            Line3d {
                point: Vec3d::zero(),
                direction: Vec3d::new(0.0, 0.0, 1.0),
            },
            Vec3d::zero(),
        );

        let picked = picker
            .pick(&ray(
                Vec3d::new(100.0, 0.0, 48.0),
                Vec3d::new(-1.0, 0.0, 0.0),
            ))
            .expect("the ray is not parallel to the line");
        geometry_math::assert_approx_eq!(picked, Vec3d::new(0.0, 0.0, 48.0), 1e-9);
    }

    #[test]
    fn a_ray_along_a_line_picks_nothing() {
        let picker = HandlePicker::line(
            Line3d {
                point: Vec3d::zero(),
                direction: Vec3d::new(0.0, 0.0, 1.0),
            },
            Vec3d::zero(),
        );
        assert_eq!(
            picker.pick(&ray(
                Vec3d::new(0.0, 0.0, 100.0),
                Vec3d::new(0.0, 0.0, -1.0)
            )),
            None
        );
    }

    #[test]
    fn a_circle_picker_stays_on_its_circle() {
        let picker = HandlePicker::circle(
            Vec3d::zero(),
            Vec3d::new(0.0, 0.0, 1.0),
            32.0,
            Vec3d::zero(),
        );

        // Straight down onto a point well outside the circle: the answer is on the circle,
        // in that direction.
        let picked = picker
            .pick(&ray(
                Vec3d::new(100.0, 0.0, 64.0),
                Vec3d::new(0.0, 0.0, -1.0),
            ))
            .expect("the ray meets the circle's plane");
        geometry_math::assert_approx_eq!(picked, Vec3d::new(32.0, 0.0, 0.0), 1e-9);
        geometry_math::assert_approx_eq!(vec::length(picked), 32.0, 1e-9);
    }

    #[test]
    fn a_ray_through_the_centre_of_a_circle_picks_nothing() {
        // There is no direction to put the handle in.
        let picker = HandlePicker::circle(
            Vec3d::zero(),
            Vec3d::new(0.0, 0.0, 1.0),
            32.0,
            Vec3d::zero(),
        );
        assert_eq!(
            picker.pick(&ray(Vec3d::new(0.0, 0.0, 64.0), Vec3d::new(0.0, 0.0, -1.0))),
            None
        );
    }

    #[test]
    fn the_offset_keeps_the_handle_from_jumping_under_the_pointer() {
        // The press landed 8 units from the handle's own position. Dragging without moving
        // the pointer must leave the handle exactly where it was.
        let handle = Vec3d::new(0.0, 0.0, 0.0);
        let hit = Vec3d::new(8.0, 0.0, 0.0);
        let offset = handle - hit;

        let picker = HandlePicker::plane(
            Plane3d::from_anchor_and_normal(handle, Vec3d::new(0.0, 0.0, 1.0)),
            offset,
        );

        let picked = picker
            .pick(&ray(
                Vec3d::new(8.0, 0.0, 100.0),
                Vec3d::new(0.0, 0.0, -1.0),
            ))
            .expect("the ray meets the plane");
        geometry_math::assert_approx_eq!(picked, handle, 1e-9);
    }

    #[test]
    fn a_canonical_picker_keeps_only_the_movement_along_its_direction() {
        // The plane faces the viewer, as it does in a flat view; the direction is the
        // face's normal.
        let initial = Vec3d::zero();
        let towards_viewer = Vec3d::new(0.0, -1.0, 0.0);
        let picker = HandlePicker::canonical(
            Plane3d::from_anchor_and_normal(initial, towards_viewer),
            initial,
            Vec3d::new(0.0, 0.0, 1.0),
            Vec3d::zero(),
        );

        // The pointer has gone up and to the right; only the up counts.
        let picked = picker
            .pick(&ray(
                Vec3d::new(48.0, -100.0, 24.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ))
            .expect("the ray meets the plane");
        geometry_math::assert_approx_eq!(picked, Vec3d::new(0.0, 0.0, 24.0), 1e-9);
    }

    #[test]
    fn a_canonical_picker_gives_nothing_where_the_ray_misses_its_plane() {
        let picker = HandlePicker::canonical(
            Plane3d::from_anchor_and_normal(Vec3d::zero(), Vec3d::new(0.0, -1.0, 0.0)),
            Vec3d::zero(),
            Vec3d::new(0.0, 0.0, 1.0),
            Vec3d::zero(),
        );
        assert_eq!(
            picker.pick(&ray(
                Vec3d::new(0.0, -100.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0)
            )),
            None
        );
    }

    #[test]
    fn alt_moves_a_handle_straight_up_and_down() {
        assert_eq!(
            move_mode(true, false, Vec3d::zero(), Vec3d::new(32.0, 0.0, 0.0)),
            MoveMode::Vertical
        );

        let picker = picker_for_move(
            MoveMode::Vertical,
            Vec3d::zero(),
            Vec3d::new(16.0, 16.0, 0.0),
            Vec3d::zero(),
        );
        let HandlePicker::Line { line, .. } = picker else {
            panic!("vertical moves run along a line");
        };
        geometry_math::assert_approx_eq!(line.direction, Vec3d::new(0.0, 0.0, 1.0), 1e-9);
        geometry_math::assert_approx_eq!(line.point, Vec3d::new(16.0, 16.0, 0.0), 1e-9);
    }

    #[test]
    fn shift_constricts_to_the_axis_the_drag_has_gone_furthest_in() {
        let initial = Vec3d::zero();
        let current = Vec3d::new(8.0, 40.0, 2.0);
        assert_eq!(
            move_mode(false, true, initial, current),
            MoveMode::Constricted
        );

        let picker = picker_for_move(MoveMode::Constricted, initial, current, Vec3d::zero());
        let HandlePicker::Line { line, .. } = picker else {
            panic!("constricted moves run along a line");
        };
        // Y is the furthest, so that is the axis it runs along, from where the drag began.
        geometry_math::assert_approx_eq!(line.direction, Vec3d::new(0.0, 1.0, 0.0), 1e-9);
        geometry_math::assert_approx_eq!(line.point, initial, 1e-9);
    }

    #[test]
    fn shift_does_nothing_until_the_drag_has_a_dominant_axis() {
        // The two largest components are the same one, so there is nothing to constrict to.
        assert_eq!(
            move_mode(false, true, Vec3d::zero(), Vec3d::zero()),
            MoveMode::Default
        );
    }

    #[test]
    fn a_plain_drag_moves_a_handle_across_the_ground() {
        assert_eq!(
            move_mode(false, false, Vec3d::zero(), Vec3d::new(32.0, 0.0, 0.0)),
            MoveMode::Default
        );

        let picker = picker_for_move(
            MoveMode::Default,
            Vec3d::zero(),
            Vec3d::new(0.0, 0.0, 64.0),
            Vec3d::zero(),
        );
        let HandlePicker::Plane { plane, .. } = picker else {
            panic!("plain moves run across a plane");
        };
        geometry_math::assert_approx_eq!(plane.normal, Vec3d::new(0.0, 0.0, 1.0), 1e-9);
        // Through where the handle is now, not where it began.
        geometry_math::assert_approx_eq!(plane.anchor(), Vec3d::new(0.0, 0.0, 64.0), 1e-9);
    }

    #[test]
    fn a_handle_keeps_its_size_on_screen_however_far_away_it_is() {
        let camera = looking_down_from_above();
        let near = perspective_scaling_factor(&camera, Vec3d::new(0.0, 0.0, 128.0), 800.0);
        let far = perspective_scaling_factor(&camera, Vec3d::new(0.0, 0.0, -128.0), 800.0);

        // Twice as far away means twice the world radius for the same pixels.
        assert!(far > near);
        geometry_math::assert_approx_eq!(far / near, 3.0, 1e-6);
    }

    #[test]
    fn a_ray_through_a_handle_reaches_it_and_one_beside_it_does_not() {
        let camera = looking_down_from_above();
        let handle = Vec3d::new(0.0, 0.0, 0.0);
        let down = ray(Vec3d::new(0.0, 0.0, 256.0), Vec3d::new(0.0, 0.0, -1.0));

        assert!(pick_point_handle(&camera, 800.0, &down, handle, 3.0).is_some());

        let wide = ray(Vec3d::new(400.0, 0.0, 256.0), Vec3d::new(0.0, 0.0, -1.0));
        assert_eq!(pick_point_handle(&camera, 800.0, &wide, handle, 3.0), None);
    }

    #[test]
    fn a_handle_behind_the_camera_cannot_be_picked() {
        let camera = looking_down_from_above();
        // Above the camera, which is looking down.
        let behind = Vec3d::new(0.0, 0.0, 512.0);
        let up = ray(Vec3d::new(0.0, 0.0, 256.0), Vec3d::new(0.0, 0.0, 1.0));
        assert_eq!(pick_point_handle(&camera, 800.0, &up, behind, 3.0), None);
    }

    #[test]
    fn the_nearest_handle_along_the_ray_is_the_one_picked() {
        let camera = looking_down_from_above();
        let down = ray(Vec3d::new(0.0, 0.0, 256.0), Vec3d::new(0.0, 0.0, -1.0));

        let near = Vec3d::new(0.0, 0.0, 128.0);
        let far = Vec3d::new(0.0, 0.0, -128.0);
        assert_eq!(
            nearest_handle(&camera, 800.0, &down, [far, near], 3.0),
            Some(near)
        );
    }

    #[test]
    fn nothing_is_picked_where_the_ray_reaches_no_handle() {
        let camera = looking_down_from_above();
        let down = ray(Vec3d::new(400.0, 400.0, 256.0), Vec3d::new(0.0, 0.0, -1.0));
        assert_eq!(
            nearest_handle(&camera, 800.0, &down, [Vec3d::zero()], 3.0),
            None
        );
    }
}
#[cfg(test)]
mod grazing_tests {
    use super::*;
    use geometry_math::vec::Vec3d;

    #[test]
    fn a_ray_running_along_the_plane_says_nothing_about_where_it_meets_it() {
        let picker = HandlePicker::plane(
            geometry_math::plane::Plane3d::from_anchor_and_normal(
                Vec3d::zero(),
                Vec3d::new(0.0, 0.0, 1.0),
            ),
            Vec3d::zero(),
        );

        // Straight down at the plane: an answer.
        let down = Ray3d {
            origin: Vec3d::new(0.0, 0.0, 64.0),
            direction: Vec3d::new(0.0, 0.0, -1.0),
        };
        assert_eq!(picker.pick(&down), Some(Vec3d::zero()));

        // Along it, a degree off: no answer, rather than one thousands of units away.
        let along = Ray3d {
            origin: Vec3d::new(0.0, 0.0, 64.0),
            direction: geometry_math::vec::normalize(Vec3d::new(1.0, 0.0, -0.017)),
        };
        assert_eq!(picker.pick(&along), None);
    }
}
