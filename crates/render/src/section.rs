//! A temporary axis-aligned viewport section, independent of map geometry.
use geometry_math::{abstract_line::AbstractLine, ray::Ray3d, vec::Vec3d};

/// The retained half-space of a working 3D view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section {
    /// Whether clipping is active.
    pub enabled: bool,
    /// World axis: X=0, Y=1, Z=2.
    pub axis: usize,
    /// Position in map units.
    pub position: f64,
    /// Keep coordinates above the plane instead of below it.
    pub flipped: bool,
    /// Whether the first enable has placed the section in this map.
    pub initialized: bool,
}
impl Default for Section {
    fn default() -> Self {
        Self {
            enabled: false,
            axis: 2,
            position: 0.0,
            flipped: false,
            initialized: false,
        }
    }
}
/// The portion of an original pick ray that lies in the displayed half-space.
pub struct SectionRay {
    /// Ray starting at the visible region.
    pub ray: Ray3d,
    /// Start distance measured along the original ray.
    pub offset: f64,
    /// Remaining visible length, possibly infinite.
    pub length: f64,
}
impl Section {
    /// Initialize the first plane through the map, preserving later adjustments.
    pub fn set_enabled(&mut self, enabled: bool, center: Vec3d) {
        if enabled && !self.initialized {
            self.position = center[self.axis.min(2)];
            self.initialized = true;
        }
        self.enabled = enabled;
    }
    /// Plane equation for shaders. A zero vector disables clipping.
    pub fn plane(self) -> [f32; 4] {
        if !self.enabled || !self.position.is_finite() {
            return [0.0; 4];
        }
        let mut p = [0.0; 4];
        let sign = if self.flipped { -1.0 } else { 1.0 };
        p[self.axis.min(2)] = sign;
        p[3] = -sign * (self.position.clamp(-1e7, 1e7) as f32);
        p
    }
    /// Whether an annotation anchored at this point belongs to the visible half-space.
    pub fn visible_point(self, point: Vec3d) -> bool {
        let p = self.plane();
        (point.x() * f64::from(p[0])
            + point.y() * f64::from(p[1])
            + point.z() * f64::from(p[2])
            + f64::from(p[3]))
            <= 0.0001
    }
    /// Restrict a 3D pick ray to the visible half-space. Its returned hit distances
    /// need `offset` added back, so tools still locate points on the original ray.
    pub fn clip_ray(self, ray: &Ray3d) -> Option<SectionRay> {
        let mut start: f64 = 0.0;
        let mut end = f64::INFINITY;
        if self.enabled && self.position.is_finite() {
            let axis = self.axis.min(2);
            let sign = if self.flipped { -1.0 } else { 1.0 };
            let origin = sign * (ray.origin[axis] - self.position.clamp(-1e7, 1e7)) - 0.0001;
            let direction = sign * ray.direction[axis];
            if direction.abs() < 1e-12 {
                if origin > 0.0 {
                    return None;
                }
            } else if direction < 0.0 {
                start = start.max(-origin / direction);
            } else {
                end = -origin / direction;
            }
            if end < start {
                return None;
            }
        }
        Some(SectionRay {
            ray: Ray3d::new(ray.point_at_distance(start), ray.direction),
            offset: start,
            length: end - start,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pick_interval_handles_both_directions_and_parallel_rays() {
        let mut s = Section::default();
        s.axis = 0;
        s.position = 4.0;
        s.enabled = true;
        let entering = Ray3d::new(Vec3d::new(10., 0., 0.), Vec3d::new(-1., 0., 0.));
        let r = s.clip_ray(&entering).unwrap();
        assert!((r.offset - 6.).abs() < 0.001);
        assert!(r.length.is_infinite());
        let leaving = Ray3d::new(Vec3d::zero(), Vec3d::new(1., 0., 0.));
        assert!((s.clip_ray(&leaving).unwrap().length - 4.).abs() < 0.001);
        let parallel = Ray3d::new(Vec3d::new(10., 0., 0.), Vec3d::new(0., 1., 0.));
        assert!(s.clip_ray(&parallel).is_none());
        s.flipped = true;
        assert!(s.clip_ray(&parallel).is_some());
        s.enabled = false;
        let full = s.clip_ray(&entering).unwrap();
        assert_eq!(full.offset, 0.);
        assert!(full.length.is_infinite());
    }
}
