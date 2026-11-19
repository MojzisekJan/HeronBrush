//! Built-in material captures shared by the native and WebGL viewports.
//! The atlas is baked once from camera-space spheres, then sampled by surface normal.

/// A viewport-only material capture. Never stored in map or engine export data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum MatCap {
    /// Use the map materials.
    #[default]
    Off = 0,
    /// Warm diffuse clay capture.
    Clay = 1,
    /// Polished studio metal capture.
    Chrome = 2,
    /// Camera-space surface normals.
    Normals = 3,
}
impl MatCap {
    /// Presets in display order.
    pub const ALL: [Self; 4] = [Self::Off, Self::Clay, Self::Chrome, Self::Normals];
    /// Label in the preset chooser.
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Clay => "Clay",
            Self::Chrome => "Chrome",
            Self::Normals => "Normals",
        }
    }
    /// Whether the viewport replaces material shading.
    pub fn enabled(self) -> bool {
        self != Self::Off
    }
}

/// Three 128-pixel material captures side by side, in sRGB RGBA8.
/// Rim texels extend to the equator so bilinear filtering never samples a background.
pub fn atlas() -> (u32, u32, Vec<u8>) {
    // Shared by additional viewports and offline renderers; keep ownership of the
    // returned pixels independent while avoiding repeated sphere shading.
    static ATLAS: std::sync::OnceLock<(u32, u32, Vec<u8>)> = std::sync::OnceLock::new();
    ATLAS.get_or_init(bake_atlas).clone()
}

fn bake_atlas() -> (u32, u32, Vec<u8>) {
    let side = 128u32;
    let mut pixels = Vec::with_capacity((side * side * 3 * 4) as usize);
    let normalize = |v: [f32; 3]| {
        let l = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
        v.map(|x| x / l)
    };
    let dot = |a: [f32; 3], b: [f32; 3]| a.into_iter().zip(b).map(|(a, b)| a * b).sum::<f32>();
    let key = normalize([-0.5, 0.7, 1.0]);
    let fill = normalize([0.8, 0.2, 0.5]);
    for y in 0..side {
        for preset in 0..3 {
            for x in 0..side {
                let nx = ((x as f32 + 0.5) / side as f32) * 2.0 - 1.0;
                let ny = ((y as f32 + 0.5) / side as f32) * 2.0 - 1.0;
                let n = normalize([nx, ny, (1.0 - nx * nx - ny * ny).max(0.0).sqrt()]);
                let color = match preset {
                    0 => {
                        let diffuse =
                            0.18 + 0.65 * dot(n, key).max(0.0) + 0.15 * dot(n, fill).max(0.0);
                        let highlight =
                            dot(n, normalize([-0.3, 0.4, 1.0])).max(0.0).powf(28.0) * 0.08;
                        [0.68, 0.39, 0.25].map(|c| c * diffuse + highlight)
                    }
                    1 => {
                        let reflection = [
                            2.0 * n[2] * n[0],
                            2.0 * n[2] * n[1],
                            2.0 * n[2] * n[2] - 1.0,
                        ];
                        let band = (-(reflection[1] - 0.45).powi(2) * 18.0).exp();
                        let softbox = (-(reflection[0] + 0.5).powi(2) * 24.0
                            - (reflection[1] - 0.3).powi(2) * 7.0)
                            .exp();
                        let rim = (1.0 - n[2]).powf(3.0) * 0.24;
                        let base = 0.025
                            + 0.13 * (reflection[1] * 0.5 + 0.5)
                            + 0.65 * band
                            + 0.8 * softbox
                            + rim;
                        [base * 0.9, base * 0.95, base]
                    }
                    _ => n.map(|c| ((c + 1.0) * 0.5).powf(2.2)),
                };
                pixels.extend(
                    color.map(|c| (c.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0).round() as u8),
                );
                pixels.push(255);
            }
        }
    }
    (side * 3, side, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_atlas_preserves_pixels_and_independent_ownership() {
        let expected = bake_atlas();
        let mut first = atlas();
        assert_eq!(first, expected);
        first.2.fill(0);
        assert_eq!(atlas(), expected);
    }

    #[test]
    fn builtins_are_opaque_distinct_and_have_directional_detail() {
        let (w, h, p) = atlas();
        assert_eq!(p.len(), (w * h * 4) as usize);
        assert!(p.chunks_exact(4).all(|c| c[3] == 255));
        let pixel = |x: u32, y: u32| &p[((y * w + x) * 4) as usize..((y * w + x) * 4 + 3) as usize];
        assert_ne!(pixel(64, 64), pixel(192, 64));
        assert_ne!(pixel(192, 64), pixel(320, 64));
        assert_ne!(pixel(32, 96), pixel(96, 32));
        assert!(pixel(320, 64)[2] > 240);
    }
}
