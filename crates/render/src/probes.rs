//! Immutable, baked local reflection and diffuse-light volumes.
use std::sync::atomic::{AtomicU64, Ordering};
/// One box-shaped influence volume in world coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Volume {
    /// Capture origin, in map units.
    pub position: [f32; 3],
    /// Half size of the box, in map units.
    pub half_size: [f32; 3],
    /// Fraction of each half extent used to blend into the global environment.
    pub blend: f32,
    /// Linear radiance multiplier.
    pub intensity: f32,
    /// Use local radiance for reflections.
    pub reflections: bool,
    /// Use local irradiance for diffuse light.
    pub diffuse: bool,
}
/// A captured local environment, prefiltered for perceptual roughness.
#[derive(Clone)]
pub struct BakedProbe {
    /// Influence and parallax-correction box.
    pub volume: Volume,
    /// Base cube face size.
    pub size: u32,
    /// Cube levels; each stores six faces in +X,-X,+Y,-Y,+Z,-Z order.
    pub mips: Vec<Vec<[f32; 4]>>,
    /// Radiance spherical harmonics; convolution is performed at shading time.
    pub sh: [[f32; 3]; 9],
}
/// A completed bake. Identity keeps settings comparisons and debug hashes cheap.
pub struct Lighting {
    id: u64,
    probes: Vec<BakedProbe>,
}
impl Lighting {
    /// Maximum simultaneously sampled local volumes.
    pub const MAX: usize = 8;
    /// Validates immutable probe pixels before publishing a bake to the render thread.
    pub fn new(probes: Vec<BakedProbe>) -> Result<Self, String> {
        if probes.len() > Self::MAX {
            return Err("At most eight light probes are supported".into());
        }
        for p in &probes {
            if ![16, 32, 64].contains(&p.size) || p.mips.len() != p.size.ilog2() as usize + 1 {
                return Err("Invalid probe cube dimensions".into());
            }
            if !p.volume.position.iter().all(|v| v.is_finite())
                || !p.volume.half_size.iter().all(|v| v.is_finite() && *v > 0.0)
                || !p.volume.intensity.is_finite()
                || p.volume.intensity < 0.0
                || !p.volume.blend.is_finite()
                || !(0.01..=1.0).contains(&p.volume.blend)
            {
                return Err("Invalid probe volume".into());
            }
            for (level, pixels) in p.mips.iter().enumerate() {
                let side = (p.size >> level).max(1);
                if pixels.len() != (side * side * 6) as usize
                    || pixels
                        .iter()
                        .flatten()
                        .any(|v| !v.is_finite() || *v < 0.0 || *v > 65504.0)
                {
                    return Err("Invalid probe radiance".into());
                }
            }
            if p.sh.iter().flatten().any(|v| !v.is_finite()) {
                return Err("Invalid probe irradiance".into());
            }
        }
        static ID: AtomicU64 = AtomicU64::new(1);
        Ok(Self {
            id: ID.fetch_add(1, Ordering::Relaxed),
            probes,
        })
    }
    /// Immutable captures.
    pub fn probes(&self) -> &[BakedProbe] {
        &self.probes
    }
    /// Runtime identity of this bake.
    pub fn id(&self) -> u64 {
        self.id
    }
}
impl PartialEq for Lighting {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Lighting {}
impl std::fmt::Debug for Lighting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProbeLighting")
            .field("id", &self.id)
            .field("count", &self.probes.len())
            .finish()
    }
}
