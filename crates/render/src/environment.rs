//! Shared scene-linear environment data, independent of graphics backends.
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Immutable, scene-linear latitude-longitude panorama.
#[derive(PartialEq)]
pub struct EnvironmentImage {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
    fingerprint: u64,
}
impl std::fmt::Debug for EnvironmentImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvironmentImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}
// Construction excludes NaN, and fields are immutable outside this module.
impl Eq for EnvironmentImage {}
impl EnvironmentImage {
    /// Panorama width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }
    /// Panorama height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }
    /// Sanitized linear RGBA pixels in row-major order.
    pub fn pixels(&self) -> &[[f32; 4]] {
        &self.pixels
    }
    /// Stable content key for GPU resource reuse.
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    /// Validates a 2:1 latitude-longitude panorama and sanitizes values for HDR targets.
    pub fn new(width: u32, height: u32, mut pixels: Vec<[f32; 4]>) -> Result<Self, String> {
        if height == 0
            || width != height.saturating_mul(2)
            || width > 8192
            || pixels.len() as u64 != u64::from(width) * u64::from(height)
        {
            return Err("Expected a 2:1 equirectangular panorama up to 8192 pixels wide".into());
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        width.hash(&mut hash);
        height.hash(&mut hash);
        for pixel in &mut pixels {
            for value in &mut pixel[..3] {
                *value = if value.is_finite() {
                    value.clamp(0.0, 65504.0)
                } else {
                    0.0
                };
                value.to_bits().hash(&mut hash);
            }
            pixel[3] = 1.0;
        }
        Ok(Self {
            width,
            height,
            pixels,
            fingerprint: hash.finish(),
        })
    }
    /// Bilinear sampling wraps longitude and clamps latitude, including both poles.
    pub fn sample(&self, direction: [f32; 3], rotation_degrees: f32) -> [f32; 3] {
        let angle = direction[1].atan2(direction[0]) - rotation_degrees.to_radians();
        let u = (angle / std::f32::consts::TAU + 0.5).rem_euclid(1.0);
        let v = direction[2].clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
        let x = u * self.width as f32 - 0.5;
        let y = (v * self.height as f32 - 0.5).clamp(0.0, self.height as f32 - 1.0);
        let (left, top) = (x.floor() as i32, y.floor() as u32);
        let at = |dx: i32, dy: u32| {
            self.pixels[((top + dy).min(self.height - 1) * self.width
                + (left + dx).rem_euclid(self.width as i32) as u32)
                as usize]
        };
        let (a, b, c, d) = (at(0, 0), at(1, 0), at(0, 1), at(1, 1));
        let (fx, fy) = (x - x.floor(), y - y.floor());
        std::array::from_fn(|k| {
            (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy
        })
    }
}

/// Environment lighting and independently controlled visible background.
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    /// Shared panorama pixels.
    pub image: Arc<EnvironmentImage>,
    /// Rotation around the world Z axis in degrees.
    pub rotation_degrees: f32,
    /// Scene-linear radiance multiplier.
    pub intensity: f32,
    /// Whether the panorama replaces the flat camera background.
    pub show_background: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_and_sanitizes_hdr_pixels() {
        assert!(EnvironmentImage::new(0, 0, vec![]).is_err());
        assert!(EnvironmentImage::new(4, 1, vec![[0.0; 4]; 4]).is_err());
        assert!(EnvironmentImage::new(2, 1, vec![]).is_err());
        let image = EnvironmentImage::new(
            2,
            1,
            vec![
                [f32::NAN, -1.0, f32::INFINITY, 0.0],
                [2.0, 90000.0, 0.5, 0.0],
            ],
        )
        .unwrap();
        assert_eq!(image.pixels()[0], [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(image.pixels()[1], [2.0, 65504.0, 0.5, 1.0]);
        assert!(format!("{image:?}").len() < 150);
    }
    #[test]
    fn sampling_wraps_the_seam_and_rotation_and_clamps_poles() {
        let image = EnvironmentImage::new(
            4,
            2,
            vec![
                [1.0, 0.0, 0.0, 1.0],
                [2.0, 0.0, 0.0, 1.0],
                [3.0, 0.0, 0.0, 1.0],
                [4.0, 0.0, 0.0, 1.0],
                [5.0, 0.0, 0.0, 1.0],
                [6.0, 0.0, 0.0, 1.0],
                [7.0, 0.0, 0.0, 1.0],
                [8.0, 0.0, 0.0, 1.0],
            ],
        )
        .unwrap();
        assert_eq!(image.sample([-1.0, 0.0, 0.0], 0.0), [4.5, 0.0, 0.0]);
        assert_eq!(image.sample([0.0, 0.0, 1.0], 0.0), [2.5, 0.0, 0.0]);
        assert_eq!(image.sample([0.0, 0.0, -1.0], 0.0), [6.5, 0.0, 0.0]);
        for angle in [0.0, 90.0, 180.0, 270.0] {
            let a = image.sample([1.0, 0.0, 0.0], angle);
            let b = image.sample([1.0, 0.0, 0.0], angle + 360.0);
            assert!((a[0] - b[0]).abs() < 0.00001);
        }
        assert_eq!(
            image.sample([0.0, 1.0, 0.0], 90.0),
            image.sample([1.0, 0.0, 0.0], 0.0)
        );
    }
}
