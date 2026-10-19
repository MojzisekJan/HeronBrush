//! Rotation ring picking and drag math shared by desktop and future touch frontends.
use geometry_math::vec::{self, Vec3d};

fn to_unit(v: Vec3d) -> Option<Vec3d> {
    let length = vec::length(v);
    (length.is_finite() && length > 1e-9).then(|| v / length)
}

/// A rotation gesture with a fixed projection and axis for its entire lifetime.
#[derive(Debug, Clone, Copy)]
pub struct RingDrag {
    /// Unit world-space axis selected at pointer-down.
    pub axis: Vec3d,
    /// Fixed pivot for this gesture.
    pub center: Vec3d,
    /// Unit radial direction at pointer-down, also used by the angle overlay.
    pub reference: Vec3d,
    camera: crate::camera3d::Camera3d,
    height: f64,
    start: geometry_math::vec::Vec2d,
    mode: RingMotion,
}
#[derive(Debug, Clone, Copy)]
enum RingMotion {
    Plane { reference: Vec3d },
    Tangent { pixels_per_radian: geometry_math::vec::Vec2d },
}
impl RingDrag {
    fn screen(
        camera: &crate::camera3d::Camera3d,
        height: f64,
        ray: &geometry_math::ray::Ray3d,
    ) -> Option<geometry_math::vec::Vec2d> {
        // Width only translates projected coordinates, so a square virtual viewport
        // works for both pointer rays and ring points without knowing the real width.
        camera.project(ray.origin + ray.direction * 100.0, height, height)
    }
    fn on_plane(center: Vec3d, axis: Vec3d, ray: &geometry_math::ray::Ray3d) -> Option<Vec3d> {
        let denominator = vec::dot(ray.direction, axis);
        if denominator.abs() < 1e-6 {
            return None;
        }
        let distance = vec::dot(center - ray.origin, axis) / denominator;
        if distance <= 0.0 {
            return None;
        }
        to_unit(ray.origin + ray.direction * distance - center)
    }
    /// Pick the visible polyline closest to the pointer, using depth only for ties.
    pub fn pick(
        camera: &crate::camera3d::Camera3d,
        height: f64,
        ray: &geometry_math::ray::Ray3d,
        center: Vec3d,
        radius: f64,
        tolerance: f64,
    ) -> Option<Self> {
        use geometry_math::vec::Vec2d;
        if radius <= 0.0 || !radius.is_finite() {
            return None;
        }
        let pointer = Self::screen(camera, height, ray)?;
        let mut best: Option<(f64, f64, Vec3d, Vec3d)> = None;
        for axis_index in 0..3 {
            let (u, v) = match axis_index {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            let mut axis = Vec3d::zero();
            axis[axis_index] = 1.0;
            for i in 0..64 {
                let point = |step: usize| {
                    let angle = step as f64 / 64.0 * std::f64::consts::TAU;
                    let mut p = center;
                    p[u] += radius * angle.cos();
                    p[v] += radius * angle.sin();
                    p
                };
                let a = point(i);
                let b = point(i + 1);
                let (Some(sa), Some(sb)) = (
                    camera.project(a, height, height),
                    camera.project(b, height, height),
                ) else {
                    continue;
                };
                let edge = sb - sa;
                let length = vec::dot(edge, edge);
                if length < 1e-10 {
                    continue;
                }
                let t = (vec::dot(pointer - sa, edge) / length).clamp(0.0, 1.0);
                let distance = vec::length(pointer - (sa + edge * t));
                if distance > tolerance {
                    continue;
                }
                // Perspective-correct interpolation along the displayed segment.
                let za = vec::dot(a - camera.position, camera.direction());
                let zb = vec::dot(b - camera.position, camera.direction());
                let world_t = (t / zb) / ((1.0 - t) / za + t / zb);
                let caught = a + (b - a) * world_t;
                let depth = vec::dot(caught - camera.position, camera.direction());
                if best.as_ref().is_none_or(|(old, z, _, _)| {
                    distance < *old - 0.5 || ((distance - *old).abs() <= 0.5 && depth < *z)
                }) {
                    best = Some((distance, depth, axis, caught));
                }
            }
        }
        let (_, _, axis, caught) = best?;
        let facing = vec::dot(to_unit(center - camera.position)?, axis).abs();
        let mode = if facing >= 0.2 {
            RingMotion::Plane {
                reference: Self::on_plane(center, axis, ray)?,
            }
        } else {
            let radial = to_unit(caught - center)? * radius;
            let tangent = vec::cross(axis, radial);
            let at = camera.project(center + radial, height, height)?;
            let near = camera.project(center + radial + tangent * 0.01, height, height)?;
            let mut direction = (near - at) / 0.01;
            let pixels = radius / camera.world_units_per_pixel(center, height);
            if vec::length(direction) < pixels * 0.15 {
                // At an edge-on silhouette tip the projected tangent vanishes.
                // Use the ring's visible direction instead of amplifying noise.
                let side = vec::cross(axis, camera.direction());
                direction =
                    Vec2d::new(vec::dot(side, camera.right()), -vec::dot(side, camera.up()));
            }
            let length = vec::length(direction);
            if length < 1e-8 {
                return None;
            }
            RingMotion::Tangent {
                pixels_per_radian: direction / length * pixels,
            }
        };
        Some(Self {
            axis,
            center,
            reference: match mode { RingMotion::Plane { reference } => reference, _ => to_unit(caught - center)? },
            camera: *camera,
            height,
            start: pointer,
            mode,
        })
    }
    /// Signed angle from the press, never switching modes during a gesture.
    pub fn angle(&self, ray: &geometry_math::ray::Ray3d) -> Option<f64> {
        match self.mode {
            RingMotion::Plane { reference } => {
                let current = Self::on_plane(self.center, self.axis, ray)?;
                Some(
                    vec::dot(self.axis, vec::cross(reference, current))
                        .atan2(vec::dot(reference, current)),
                )
            }
            RingMotion::Tangent { pixels_per_radian } => {
                let delta = Self::screen(&self.camera, self.height, ray)? - self.start;
                Some(
                    vec::dot(delta, pixels_per_radian)
                        / vec::dot(pixels_per_radian, pixels_per_radian),
                )
            }
        }
    }
}

#[cfg(test)]
mod ring_tests {
    use super::*;
    use crate::camera3d::Camera3d;
    use geometry_math::ray::Ray3d;
    fn ray(camera: &Camera3d, point: Vec3d) -> Ray3d {
        let screen = camera.project(point, 800.0, 800.0).unwrap();
        camera.ray_through(screen.x(), screen.y(), 800.0, 800.0)
    }
    #[test]
    fn each_axis_tracks_positive_and_negative_quarter_turns_from_both_sides() {
        for index in 0..3 {
            for side in [-1.0, 1.0] {
                let mut axis = Vec3d::zero();
                axis[index] = 1.0;
                let mut camera = Camera3d { position: axis * 500.0 * side, ..Default::default() };
                camera.look_along(-camera.position);
                let mut from = Vec3d::zero();
                from[(index + 1) % 3] = 48.0;
                from[(index + 2) % 3] = 40.0;
                let radius = vec::length(from);
                let drag = RingDrag::pick(
                    &camera,
                    800.0,
                    &ray(&camera, from),
                    Vec3d::zero(),
                    radius,
                    6.0,
                )
                .unwrap();
                assert_eq!(drag.axis, axis);
                assert!(drag.angle(&ray(&camera, from)).unwrap().abs() < 1e-9);
                for sign in [-1.0, 1.0] {
                    let to = vec::cross(axis, from) * sign;
                    let measured = drag.angle(&ray(&camera, to)).unwrap();
                    assert!(
                        (measured - sign * std::f64::consts::FRAC_PI_2).abs() < 1e-8,
                        "axis {index}, side {side}: {measured}"
                    );
                }
            }
        }
    }
    #[test]
    fn edge_on_ring_is_pickable_and_moves_smoothly_in_both_directions() {
        let mut camera = Camera3d { position: Vec3d::new(0.0, -500.0, 0.0), ..Default::default() };
        camera.look_along(-camera.position);
        let start = Vec3d::new(32.0, -(64.0_f64.powi(2) - 32.0_f64.powi(2)).sqrt(), 0.0);
        let press = ray(&camera, start);
        let drag = RingDrag::pick(&camera, 800.0, &press, Vec3d::zero(), 64.0, 6.0).unwrap();
        assert_eq!(drag.axis, Vec3d::new(0.0, 0.0, 1.0));
        assert!(matches!(drag.mode, RingMotion::Tangent { .. }));
        let screen = camera.project(start, 800.0, 800.0).unwrap();
        let angle = |pixels| {
            drag.angle(&camera.ray_through(screen.x() + pixels, screen.y(), 800.0, 800.0))
                .unwrap()
        };
        assert!(angle(20.0) > 0.0);
        assert!(angle(-20.0) < 0.0);
        assert!((angle(40.0) - 2.0 * angle(20.0)).abs() < 1e-9);
        assert!(angle(0.0).abs() < 1e-9);
    }
    #[test]
    fn screen_tolerance_is_constant_with_distance_and_rejects_empty_space() {
        for distance in [256.0, 1024.0, 8192.0] {
            let mut camera = Camera3d { position: Vec3d::new(0.0, 0.0, distance), ..Default::default() };
            camera.look_along(-camera.position);
            let radius = 64.0 * camera.world_units_per_pixel(Vec3d::zero(), 800.0);
            let point = Vec3d::new(0.8, 0.6, 0.0) * radius;
            let screen = camera.project(point, 800.0, 800.0).unwrap();
            let drag = RingDrag::pick(
                &camera,
                800.0,
                &camera.ray_through(screen.x() + 2.0, screen.y(), 800.0, 800.0),
                Vec3d::zero(),
                radius,
                6.0,
            )
            .unwrap();
            assert_eq!(drag.axis, Vec3d::new(0.0, 0.0, 1.0));
            assert!(RingDrag::pick(
                &camera,
                800.0,
                &camera.ray_through(0.0, 0.0, 800.0, 800.0),
                Vec3d::zero(),
                radius,
                6.0
            )
            .is_none());
        }
    }
}
