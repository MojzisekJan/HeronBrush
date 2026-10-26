//! The camera of the 3D map view.
//!
//! A first-person camera: the viewer stands somewhere and looks in a
//! direction, and moves by walking rather than by orbiting a focus point. That suits a
//! level editor, where the thing being judged is what a player will see.

use geometry_math::mat::Mat4x4d;
use geometry_math::mat_ext;
use geometry_math::ray::Ray3d;
use geometry_math::scalar;
use geometry_math::vec::{self, Vec2d, Vec3d};

/// The camera of the 3D view.
#[derive(Debug, Clone, Copy)]
pub struct Camera3d {
    /// Where the viewer stands.
    pub position: Vec3d,
    /// Which way the viewer faces, measured as a counter-clockwise angle in the XY plane
    /// from the positive X axis, in radians.
    pub yaw: f64,
    /// How far up or down the viewer looks, in radians. Positive is up.
    pub pitch: f64,
    /// The near clipping distance.
    pub near: f64,
    /// The far clipping distance.
    pub far: f64,
    /// The vertical field of view, in degrees. The frontend keeps it in step with
    /// whatever preference or control drives it, the way `RenderSettings` is filled.
    pub fov_degrees: f64,
}

/// How far from straight ahead the camera may look, up or down.
///
/// Stopping just short of straight up keeps the view direction from becoming parallel to
/// the up vector, which would leave the view matrix without a sideways axis.
const MAX_PITCH: f64 = std::f64::consts::FRAC_PI_2 - 0.01;

impl Default for Camera3d {
    fn default() -> Self {
        Self {
            position: Vec3d::new(-256.0, -256.0, 192.0),
            yaw: std::f64::consts::FRAC_PI_4,
            pitch: -0.4,
            near: 1.0,
            far: 32768.0,
            fov_degrees: 90.0,
        }
    }
}

impl Camera3d {
    /// World units per logical viewport pixel at a point's forward depth.
    /// Supply logical dimensions consistently for both picking and rendering;
    /// physical framebuffer pixels belong only to the GL viewport.
    pub fn world_units_per_pixel(&self, point: Vec3d, viewport_height: f64) -> f64 {
        let depth = vec::dot(point - self.position, self.direction());
        let focal = (viewport_height / 2.0) / (self.fov().to_radians() / 2.0).tan();
        if focal <= 0.0 {
            return 1.0;
        }
        depth / focal
    }

    /// The vertical field of view, in degrees.
    pub fn fov(&self) -> f64 {
        self.fov_degrees
    }

    /// Returns the direction the camera looks in.
    pub fn direction(&self) -> Vec3d {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3d::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch)
    }

    /// Returns the camera's up direction.
    pub fn up(&self) -> Vec3d {
        vec::normalize(vec::cross(self.right(), self.direction()))
    }

    /// Returns the camera's rightward direction.
    ///
    /// The world's Z axis is up, so right is always level with the horizon however far up
    /// or down the camera looks.
    pub fn right(&self) -> Vec3d {
        Vec3d::new(self.yaw.sin(), -self.yaw.cos(), 0.0)
    }

    /// Points the camera along a direction.
    ///
    /// `Camera::setDirection`, taken apart into the two angles this camera is kept as. A
    /// direction of nothing leaves it where it is, since there is no way to face nowhere.
    pub fn look_along(&mut self, direction: Vec3d) {
        let direction = vec::normalize(direction);
        if !direction.x().is_finite() {
            return;
        }
        self.yaw = direction.y().atan2(direction.x());
        let level = (direction.x() * direction.x() + direction.y() * direction.y()).sqrt();
        self.pitch = direction.z().atan2(level).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Turns the camera by the given angles, in radians.
    pub fn turn(&mut self, yaw: f64, pitch: f64) {
        self.yaw = scalar::normalize_radians(self.yaw + yaw);
        self.pitch = (self.pitch + pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Turns the camera about `center` by `yaw` and `pitch`, its position going round with
    /// its direction: `Camera::orbit`. Past straight up or down it stops turning that way.
    pub fn orbit(&mut self, center: Vec3d, yaw: f64, pitch: f64) {
        let mut offset = self.position - center;
        let (sin, cos) = yaw.sin_cos();
        offset = Vec3d::new(
            offset.x() * cos - offset.y() * sin,
            offset.x() * sin + offset.y() * cos,
            offset.z(),
        );
        self.yaw = scalar::normalize_radians(self.yaw + yaw);
        let wanted = (self.pitch + pitch).clamp(-MAX_PITCH, MAX_PITCH);
        let turned = wanted - self.pitch;
        // Rodrigues' rotation about the camera's right, which raises the view as the
        // pitch rises.
        let axis = self.right();
        let (sin, cos) = turned.sin_cos();
        offset = offset * cos
            + vec::cross(axis, offset) * sin
            + axis * (vec::dot(axis, offset) * (1.0 - cos));
        self.pitch = wanted;
        self.position = center + offset;
    }

    /// Moves the camera by the given distances along its own axes.
    ///
    /// Forward follows the view direction, including its pitch, so looking down and moving
    /// forward descends — which is how flying through a level is expected to work.
    pub fn fly(&mut self, forward: f64, right: f64, up: f64) {
        self.position += self.direction() * forward;
        self.position += self.right() * right;
        self.position += Vec3d::new(0.0, 0.0, 1.0) * up;
    }

    /// Slides the camera across its own view plane.
    ///
    /// `CameraTool3D`'s pan: right is the camera's own right and up is the camera's own
    /// up, so the world slides under the pointer whichever way the camera is facing.
    pub fn pan(&mut self, right: f64, up: f64) {
        self.position += self.right() * right;
        self.position += self.up() * up;
    }

    /// Returns the matrix taking world space into the camera's view.
    pub fn view_matrix(&self) -> Mat4x4d {
        mat_ext::view_matrix(self.direction(), self.up())
            * mat_ext::translation_matrix(-self.position)
    }

    /// Returns the matrix projecting the camera's view onto the viewport.
    pub fn projection_matrix(&self, width: i32, height: i32) -> Mat4x4d {
        mat_ext::perspective_matrix(self.fov(), self.near, self.far, width, height.max(1))
    }

    /// How far the viewport reaches at one unit ahead: half its width and half its height.
    ///
    /// Uses the same frustum as [`geometry_math::mat_ext::perspective_matrix`], including its three-quarter tangent
    /// factor, so picking rays match rendered screen positions.
    fn half_extents(&self, width: f64, height: f64) -> (f64, f64) {
        let half_height = scalar::to_radians(self.fov() / 2.0).tan() * mat_ext::FRUSTUM_SCALE;
        (half_height * (width / height.max(1.0)), half_height)
    }

    /// Where a point in the world lands on the viewport, or `None` if it is behind the
    /// camera.
    ///
    /// The inverse of [`Self::ray_through`], and what a lasso needs: whether an object is
    /// inside a rectangle on screen is a question about where its corners are drawn.
    pub fn project(&self, point: Vec3d, width: f64, height: f64) -> Option<Vec2d> {
        let width = width.max(1.0);
        let height = height.max(1.0);

        // Into the camera's own frame first: a point at or behind the near plane has no
        // place on the viewport, and dividing by its depth would put it there twice.
        let ahead = vec::dot(point - self.position, self.direction());
        if ahead <= self.near {
            return None;
        }

        let (half_width, half_height) = self.half_extents(width, height);
        let across = vec::dot(point - self.position, self.right()) / ahead;
        let up = vec::dot(point - self.position, self.up()) / ahead;

        Some(Vec2d::new(
            (across / half_width + 1.0) * 0.5 * width,
            (1.0 - up / half_height) * 0.5 * height,
        ))
    }

    /// Returns the ray a click at the given viewport position casts into the scene.
    ///
    /// The viewport's Y axis grows downward while the camera's up vector does not, so the
    /// vertical offset is negated on the way in.
    pub fn ray_through(&self, x: f64, y: f64, width: f64, height: f64) -> Ray3d {
        // Where the click falls on the near plane, as a fraction of its half-extents.
        let (half_width, half_height) = self.half_extents(width, height);

        let ndc_x = (2.0 * x / width.max(1.0)) - 1.0;
        let ndc_y = 1.0 - (2.0 * y / height.max(1.0));

        let direction = vec::normalize(
            self.direction()
                + self.right() * (ndc_x * half_width)
                + self.up() * (ndc_y * half_height),
        );
        Ray3d::new(self.position, direction)
    }

    /// Frames a box in a square viewport when no viewport dimensions are available.
    pub fn frame(&mut self, bounds: &geometry_math::bbox::Bbox3d) {
        self.frame_in_viewport(bounds, 1.0, 1.0);
    }

    /// Frames every box corner with a ten-percent border in the supplied viewport.
    pub fn frame_in_viewport(
        &mut self,
        bounds: &geometry_math::bbox::Bbox3d,
        width: f64,
        height: f64,
    ) {
        let center = bounds.center();
        let forward = self.direction();
        let right = self.right();
        let up = self.up();
        let (half_width, half_height) = self.half_extents(width.max(1.0), height.max(1.0));
        let mut distance: f64 = 64.0;
        for corner in bounds.vertices() {
            let offset = corner - center;
            let depth = vec::dot(offset, forward);
            // At distance d, this corner has forward depth d + depth.
            distance = distance.max(vec::dot(offset, right).abs() / (half_width * 0.8) - depth);
            distance = distance.max(vec::dot(offset, up).abs() / (half_height * 0.8) - depth);
            distance = distance.max(self.near + 1.0 - depth);
        }
        self.position = center - forward * distance;
    }
}

#[cfg(test)]
mod tests {

    /// `Camera::orbit`: the camera goes round the centre at the same distance, and a
    /// camera looking at it keeps looking at it.
    #[test]
    fn orbiting_keeps_the_centre_in_view_at_the_same_distance() {
        let mut camera = Camera3d::default();
        let center = camera.position + camera.direction() * 200.0;
        camera.orbit(center, 0.7, 0.3);
        let offset = center - camera.position;
        assert!((vec::length(offset) - 200.0).abs() < 1e-6);
        assert!(vec::length(vec::normalize(offset) - camera.direction()) < 1e-6);
    }

    use super::*;
    use geometry_math::assert_approx_eq;

    #[test]
    fn a_point_projects_back_to_the_place_a_ray_through_it_came_from() {
        // The projection is the inverse of the ray cast: a ray through a pixel, followed
        // a way out, lands on a point that projects back to that pixel.
        let camera = Camera3d::default();
        let (width, height) = (800.0, 600.0);
        for (x, y) in [(400.0, 300.0), (100.0, 500.0), (760.0, 40.0)] {
            let ray = camera.ray_through(x, y, width, height);
            let point = ray.origin + ray.direction * 512.0;
            let back = camera
                .project(point, width, height)
                .expect("it is in front");
            assert!(
                (back.x() - x).abs() < 1e-6 && (back.y() - y).abs() < 1e-6,
                "{x},{y} came back as {back:?}"
            );
        }
    }

    #[test]
    fn a_point_behind_the_camera_lands_nowhere() {
        let camera = Camera3d::default();
        let behind = camera.position - camera.direction() * 100.0;
        assert_eq!(camera.project(behind, 800.0, 600.0), None);
    }

    #[test]
    fn the_camera_axes_are_perpendicular_and_normalized() {
        let mut camera = Camera3d::default();
        for (yaw, pitch) in [(0.0, 0.0), (1.0, 0.5), (-2.0, -1.2), (3.0, 1.5)] {
            camera.yaw = yaw;
            camera.pitch = pitch;

            let (d, r, u) = (camera.direction(), camera.right(), camera.up());
            assert_approx_eq!(vec::length(d), 1.0, 1e-9);
            assert_approx_eq!(vec::length(r), 1.0, 1e-9);
            assert_approx_eq!(vec::length(u), 1.0, 1e-9);
            assert_approx_eq!(vec::dot(d, r), 0.0, 1e-9);
            assert_approx_eq!(vec::dot(d, u), 0.0, 1e-9);
            assert_approx_eq!(vec::dot(r, u), 0.0, 1e-9);
        }
    }

    #[test]
    fn yaw_zero_looks_down_the_positive_x_axis() {
        let camera = Camera3d {
            yaw: 0.0,
            pitch: 0.0,
            ..Camera3d::default()
        };
        assert_approx_eq!(camera.direction(), Vec3d::new(1.0, 0.0, 0.0), 1e-9);
    }

    #[test]
    fn pitch_is_clamped_short_of_straight_up() {
        let mut camera = Camera3d::default();
        camera.turn(0.0, 100.0);
        assert!(camera.pitch < std::f64::consts::FRAC_PI_2);
        camera.turn(0.0, -100.0);
        assert!(camera.pitch > -std::f64::consts::FRAC_PI_2);

        // Still a usable set of axes at the limit.
        assert_approx_eq!(vec::length(camera.up()), 1.0, 1e-9);
    }

    #[test]
    fn flying_forward_follows_the_view_direction() {
        let mut camera = Camera3d {
            position: Vec3d::zero(),
            yaw: 0.0,
            pitch: 0.0,
            ..Camera3d::default()
        };
        camera.fly(10.0, 0.0, 0.0);
        assert_approx_eq!(camera.position, Vec3d::new(10.0, 0.0, 0.0), 1e-9);

        // Up is the world's up, not the camera's, so it is unaffected by pitch.
        camera.pitch = 1.0;
        camera.fly(0.0, 0.0, 5.0);
        assert_approx_eq!(camera.position, Vec3d::new(10.0, 0.0, 5.0), 1e-9);
    }

    #[test]
    fn a_point_ahead_of_the_camera_projects_into_the_viewport() {
        let camera = Camera3d {
            position: Vec3d::zero(),
            yaw: 0.0,
            pitch: 0.0,
            ..Camera3d::default()
        };
        let matrix = camera.projection_matrix(800, 600) * camera.view_matrix();

        // 100 units straight ahead lands in the middle of the near-to-far range, on axis.
        let projected = matrix * Vec3d::new(100.0, 0.0, 0.0).to_homogeneous_coords();
        let ndc = projected.to_cartesian_coords();
        assert_approx_eq!(ndc.x(), 0.0, 1e-9);
        assert_approx_eq!(ndc.y(), 0.0, 1e-9);
        assert!(ndc.z() > -1.0 && ndc.z() < 1.0, "{}", ndc.z());
    }

    #[test]
    fn a_point_behind_the_camera_projects_outside_the_near_plane() {
        let camera = Camera3d {
            position: Vec3d::zero(),
            yaw: 0.0,
            pitch: 0.0,
            ..Camera3d::default()
        };
        let matrix = camera.projection_matrix(800, 600) * camera.view_matrix();

        let projected = matrix * Vec3d::new(-100.0, 0.0, 0.0).to_homogeneous_coords();
        // The w component is negative behind the camera, which is what clipping tests.
        assert!(projected.w() < 0.0, "{}", projected.w());
    }

    #[test]
    fn a_click_in_the_middle_looks_straight_ahead() {
        let camera = Camera3d {
            position: Vec3d::zero(),
            yaw: 0.0,
            pitch: 0.0,
            ..Camera3d::default()
        };
        let ray = camera.ray_through(400.0, 300.0, 800.0, 600.0);

        assert_approx_eq!(ray.origin, Vec3d::zero(), 1e-9);
        assert_approx_eq!(ray.direction, camera.direction(), 1e-9);
    }

    #[test]
    fn a_click_off_centre_aims_off_centre_the_same_way() {
        let camera = Camera3d {
            position: Vec3d::zero(),
            yaw: 0.0,
            pitch: 0.0,
            ..Camera3d::default()
        };

        // Right of centre aims right; below centre aims down.
        let right = camera.ray_through(700.0, 300.0, 800.0, 600.0);
        assert!(vec::dot(right.direction, camera.right()) > 0.0);
        assert_approx_eq!(vec::dot(right.direction, camera.up()), 0.0, 1e-9);

        let down = camera.ray_through(400.0, 500.0, 800.0, 600.0);
        assert!(vec::dot(down.direction, camera.up()) < 0.0);
    }

    #[test]
    fn every_pick_ray_is_normalized_and_points_forwards() {
        let camera = Camera3d::default();
        for (x, y) in [
            (0.0, 0.0),
            (799.0, 0.0),
            (0.0, 599.0),
            (799.0, 599.0),
            (400.0, 300.0),
        ] {
            let ray = camera.ray_through(x, y, 800.0, 600.0);
            assert_approx_eq!(vec::length(ray.direction), 1.0, 1e-9);
            // Every corner of the viewport is still in front of the camera.
            assert!(vec::dot(ray.direction, camera.direction()) > 0.0);
        }
    }

    #[test]
    fn framing_a_box_puts_it_in_front_of_the_camera() {
        use geometry_math::bbox::Bbox3d;

        let mut camera = Camera3d::default();
        let bounds = Bbox3d::new(
            Vec3d::new(-128.0, -128.0, 0.0),
            Vec3d::new(128.0, 128.0, 64.0),
        );
        camera.frame_in_viewport(&bounds, 800.0, 600.0);

        let to_center = bounds.center() - camera.position;
        assert!(
            vec::dot(vec::normalize(to_center), camera.direction()) > 0.99,
            "the camera should be looking at the box"
        );
    }

    #[test]
    fn framing_keeps_all_corners_inside_the_viewport_border() {
        use geometry_math::bbox::Bbox3d;
        for (width, height) in [(1600.0, 400.0), (300.0, 1200.0), (640.0, 480.0)] {
            for direction in [Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(1.0, 2.0, -3.0)] {
                for bounds in [
                    Bbox3d::new(
                        Vec3d::new(-900.0, -24.0, -32.0),
                        Vec3d::new(1400.0, 64.0, 250.0),
                    ),
                    Bbox3d::new(
                        Vec3d::new(500.0, -600.0, 0.0),
                        Vec3d::new(564.0, 600.0, 0.0),
                    ),
                    Bbox3d::from_point(Vec3d::new(32.0, 64.0, 128.0)),
                ] {
                    for fov in [45.0, 90.0, 120.0] {
                        let mut camera = Camera3d {
                            fov_degrees: fov,
                            near: 100.0,
                            ..Camera3d::default()
                        };
                        camera.look_along(direction);
                        camera.frame_in_viewport(&bounds, width, height);
                        for corner in bounds.vertices() {
                            let p = camera
                                .project(corner, width, height)
                                .expect("corner clears near plane");
                            assert!(
                                p.x() >= width * 0.1 - 1e-7 && p.x() <= width * 0.9 + 1e-7,
                                "{p:?}"
                            );
                            assert!(
                                p.y() >= height * 0.1 - 1e-7 && p.y() <= height * 0.9 + 1e-7,
                                "{p:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn looking_along_a_direction_and_reading_it_back_gives_the_same_one() {
        let mut camera = Camera3d::default();
        for direction in [
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, -1.0, 0.0),
            Vec3d::new(1.0, 1.0, 1.0),
            Vec3d::new(-3.0, 2.0, -1.0),
        ] {
            camera.look_along(direction);
            let read = camera.direction();
            let wanted = vec::normalize(direction);
            assert!(
                vec::length(read - wanted) < 1e-9,
                "looked along {direction:?} and got {read:?}"
            );
        }
    }
}
