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
    /// Constant KHR_materials_clearcoat strength.
    pub clearcoat: f32,
    /// Constant top-coat perceptual roughness.
    pub coat_roughness: f32,
    /// Constant KHR_materials_anisotropy strength.
    pub anisotropy: f32,
    /// Anisotropy direction rotation in radians.
    pub anisotropy_rotation: f32,
    /// Constant linear RGB of the glTF sheen layer.
    pub sheen_color: [f32; 3],
    /// Perceptual roughness of the sheen layer.
    pub sheen_roughness: f32,
    /// sRGB sheen tint texture.
    pub sheen_color_texture: Option<std::sync::Arc<Skin>>,
    /// Linear sheen roughness stored in alpha.
    pub sheen_roughness_texture: Option<std::sync::Arc<Skin>>,
    /// Fraction of light transmitted through the dielectric surface.
    pub transmission: f32,
    /// Index of refraction for transmission.
    pub ior: f32,
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
            clearcoat: 0.0,
            coat_roughness: 0.0,
            anisotropy: 0.0,
            anisotropy_rotation: 0.0,
            sheen_color: [0.; 3],
            sheen_roughness: 0.,
            sheen_color_texture: None,
            sheen_roughness_texture: None,
            transmission: 0.,
            ior: 1.5,
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
            self.clearcoat,
            self.coat_roughness,
            self.anisotropy,
            self.anisotropy_rotation,
            self.sheen_roughness,
            self.transmission,
            self.ior,
            self.normal_scale,
            self.occlusion_strength,
        ]
        .into_iter()
        .chain(self.sheen_color)
        .chain(self.emissive_factor)
        {
            f.to_bits().hash(state);
        }
        for map in [
            &self.normal,
            &self.metallic_roughness,
            &self.occlusion,
            &self.emissive,
            &self.sheen_color_texture,
            &self.sheen_roughness_texture,
        ] {
            map.hash(state);
        }
    }
}
impl ModelPbr {
    /// Packs sRGB tint and linear roughness into one preview texture.
    pub fn preview_sheen(&self) -> Option<Skin> {
        let color = self.sheen_color_texture.as_deref();
        let roughness = self.sheen_roughness_texture.as_deref();
        let source = color.or(roughness)?;
        let width = color
            .iter()
            .chain(roughness.iter())
            .map(|skin| skin.width)
            .max()
            .unwrap_or(source.width)
            .clamp(1, 4096);
        let height = color
            .iter()
            .chain(roughness.iter())
            .map(|skin| skin.height)
            .max()
            .unwrap_or(source.height)
            .clamp(1, 4096);
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height {
            for x in 0..width {
                let sample = |skin: &Skin| {
                    let sx = u64::from(x) * u64::from(skin.width) / u64::from(width);
                    let sy = u64::from(y) * u64::from(skin.height) / u64::from(height);
                    let offset = ((sy * u64::from(skin.width) + sx) * 4) as usize;
                    <[u8; 4]>::try_from(&skin.pixels[offset..offset + 4]).unwrap()
                };
                let [r, g, b, _] = color.map_or([255; 4], sample);
                let alpha = roughness.map_or(255, |skin| sample(skin)[3]);
                pixels.extend([r, g, b, alpha]);
            }
        }
        Some(Skin {
            width,
            height,
            pixels,
        })
    }

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
