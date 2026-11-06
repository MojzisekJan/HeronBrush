//! Camera display transforms; scene-linear image exports bypass these controls.
/// Supported HDR display transforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToneMap {
    /// Fitted ACES curve.
    Aces,
    /// AgX with a restrained contrast look.
    #[default]
    Agx,
    /// Khronos PBR Neutral.
    KhronosNeutral,
    /// TonyMcMapface lookup table.
    TonyMcMapface,
}
/// Camera-specific display controls, neutral by default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorGrading {
    /// Optional override of the editor's display transform.
    pub tone_map: Option<ToneMap>,
    /// White balance illuminant in kelvin.
    pub temperature: f32,
    /// Green to magenta adjustment, -1 to 1.
    pub tint: f32,
    /// Contrast about scene-linear middle grey (0.18).
    pub contrast: f32,
    /// Luminance-preserving saturation multiplier.
    pub saturation: f32,
}
impl Default for ColorGrading {
    fn default() -> Self {
        Self {
            tone_map: None,
            temperature: 6500.0,
            tint: 0.0,
            contrast: 1.0,
            saturation: 1.0,
        }
    }
}
impl ColorGrading {
    /// Diagonal linear-RGB white balance relative to the neutral 6500 K illuminant.
    pub fn white_balance(self) -> [f32; 3] {
        fn illuminant(k: f32) -> [f32; 3] {
            let k = k.clamp(2000.0, 12000.0);
            let x = if k <= 4000.0 {
                -0.2661239e9 / k.powi(3) - 2.343_58e5 / k.powi(2) + 0.8776956e3 / k + 0.179910
            } else {
                -3.0258469e9 / k.powi(3) + 2.107_038e6 / k.powi(2) + 0.2226347e3 / k + 0.240390
            };
            let y = if k <= 2222.0 {
                -1.1063814 * x.powi(3) - 1.3481102 * x * x + 2.1855583 * x - 0.20219683
            } else if k <= 4000.0 {
                -0.9549476 * x.powi(3) - 1.3741859 * x * x + 2.091_37 * x - 0.16748867
            } else {
                3.081758 * x.powi(3) - 5.873_387 * x * x + 3.7511299 * x - 0.37001483
            };
            let (x, z) = (x / y, (1.0 - x - y) / y);
            [
                3.2404542 * x - 1.5371385 - 0.4985314 * z,
                -0.969266 * x + 1.8760108 + 0.041556 * z,
                0.0556434 * x - 0.2040259 + 1.0572252 * z,
            ]
        }
        let source = illuminant(self.temperature);
        let reference = illuminant(6500.0);
        let mut gain =
            std::array::from_fn(|i| (reference[i] / source[i].max(0.02)).clamp(0.05, 20.0));
        let green = gain[1];
        for value in &mut gain {
            *value /= green;
        }
        gain[1] *= 2.0f32.powf(-self.tint.clamp(-1.0, 1.0));
        gain
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn neutral_and_temperature_balance_are_finite() {
        assert_eq!(ColorGrading::default().white_balance(), [1.0; 3]);
        for temperature in [2000.0, 3200.0, 6500.0, 9000.0, 12000.0] {
            assert!(ColorGrading {
                temperature,
                ..Default::default()
            }
            .white_balance()
            .iter()
            .all(|v| v.is_finite() && *v > 0.0));
        }
        let warm = ColorGrading {
            temperature: 9000.0,
            ..Default::default()
        }
        .white_balance();
        let cool = ColorGrading {
            temperature: 3200.0,
            ..Default::default()
        }
        .white_balance();
        assert!(warm[0] / warm[2] > cool[0] / cool[2]);
    }
}
