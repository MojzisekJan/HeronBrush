//! Core glTF metallic-roughness material data, retained independently of preview images.
use crate::entity_model::Skin;
use std::hash::{Hash, Hasher};

/// A static model's linear factors and original decoded texture images.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPbr {
    /// Explicit source transparency mode; independent of image pixel values.
    pub alpha_mode: crate::material::Alpha,
    /// Linear alpha threshold for masked materials.
    pub alpha_cutoff: f32,
    /// Render both sides of source triangles.
    pub double_sided: bool,
    /// Linear roughness multiplier.
    pub roughness: f32,
    /// Linear metallic multiplier.
    pub metallic: f32,
    /// Linear emissive RGB multiplier.
    pub emissive_factor: [f32; 3],
    /// Multiplier of tangent-space normal X and Y.
    pub normal_scale: f32,
    /// Strength of baked occlusion on indirect lighting.
    pub occlusion_strength: f32,
    /// Linear tangent-space normal image.
    pub normal: Option<std::sync::Arc<Skin>>,
    /// Linear roughness G and metallic B image.
    pub metallic_roughness: Option<std::sync::Arc<Skin>>,
    /// Linear occlusion R image.
    pub occlusion: Option<std::sync::Arc<Skin>>,
    /// sRGB emissive image.
    pub emissive: Option<std::sync::Arc<Skin>>,
}
impl Default for ModelPbr {
    fn default() -> Self {
        Self {
            alpha_mode: crate::material::Alpha::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
            roughness: 1.,
            metallic: 1.,
            emissive_factor: [0.; 3],
            normal_scale: 1.,
            occlusion_strength: 1.,
            normal: None,
            metallic_roughness: None,
            occlusion: None,
            emissive: None,
        }
    }
}
impl Hash for ModelPbr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.alpha_mode.hash(state);
        self.alpha_cutoff.to_bits().hash(state);
        self.double_sided.hash(state);
        for f in [
            self.roughness,
            self.metallic,
            self.normal_scale,
            self.occlusion_strength,
        ]
        .into_iter()
        .chain(self.emissive_factor)
        {
            f.to_bits().hash(state);
        }
        for map in [
            &self.normal,
            &self.metallic_roughness,
            &self.occlusion,
            &self.emissive,
        ] {
            map.hash(state);
        }
    }
}
impl ModelPbr {
    /// Packed preview ORM, sampling both source maps at their own resolution.
    /// Exports retain the original images rather than this preview packing.
    pub fn preview_orm(&self) -> Skin {
        let width = self
            .metallic_roughness
            .iter()
            .chain(self.occlusion.iter())
            .map(|s| s.width)
            .max()
            .unwrap_or(1)
            .clamp(1, 4096);
        let height = self
            .metallic_roughness
            .iter()
            .chain(self.occlusion.iter())
            .map(|s| s.height)
            .max()
            .unwrap_or(1)
            .clamp(1, 4096);
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height {
            for x in 0..width {
                let sample = |map: &Option<std::sync::Arc<Skin>>, channel: usize| {
                    map.as_ref().map_or(255, |s| {
                        let sx = x as u64 * s.width as u64 / width as u64;
                        let sy = y as u64 * s.height as u64 / height as u64;
                        s.pixels[((sy * s.width as u64 + sx) * 4) as usize + channel]
                    })
                };
                pixels.extend([
                    sample(&self.occlusion, 0),
                    sample(&self.metallic_roughness, 1),
                    sample(&self.metallic_roughness, 2),
                    255,
                ]);
            }
        }
        Skin {
            width,
            height,
            pixels,
        }
    }
}
