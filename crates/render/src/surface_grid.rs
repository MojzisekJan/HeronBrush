//! World-space surface grid used by editor viewports. Image renders leave it disabled.

/// Grid spacing and opacity, independent of material UV coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SurfaceGrid {
    /// Cell size in map units. Zero disables the overlay.
    pub spacing: f32,
    /// Overall line opacity in the range zero to one.
    pub opacity: f32,
}

impl SurfaceGrid {
    /// Validated parameters shared by the GL and WGPU shaders.
    pub fn parameters(self) -> [f32; 4] {
        if !self.spacing.is_finite() || self.spacing <= 0.0 || !self.opacity.is_finite() {
            return [0.0; 4];
        }
        [self.spacing, self.opacity.clamp(0.0, 1.0), 0.0, 0.0]
    }
}

#[cfg(test)]
mod tests {
    use super::SurfaceGrid;

    #[test]
    fn invalid_grid_settings_cannot_reach_shader_arithmetic() {
        for spacing in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                SurfaceGrid {
                    spacing,
                    opacity: 0.5
                }
                .parameters(),
                [0.0; 4]
            );
        }
        assert_eq!(
            SurfaceGrid {
                spacing: 16.0,
                opacity: f32::NAN
            }
            .parameters(),
            [0.0; 4]
        );
        assert_eq!(
            SurfaceGrid {
                spacing: 0.125,
                opacity: 2.0
            }
            .parameters(),
            [0.125, 1.0, 0.0, 0.0]
        );
        assert_eq!(SurfaceGrid::default().parameters(), [0.0; 4]);
    }
}
