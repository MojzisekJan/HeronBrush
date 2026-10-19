//! The temporal filter's state and history targets (the pass is `shaders/taa.wgsl`).
//!
//! Every view frame is drawn with the projection jittered by a Halton (2, 3) sequence and
//! blended into a history reprojected by the camera's motion. How much of the new frame
//! goes in is 1 / (samples in the history + 1):
//!
//! - a moving camera keeps the history at nine samples, a weight of 0.1, the usual TAA
//!   balance between smoothing and lag (with antialiasing off, a moving camera draws
//!   plain unjittered frames instead and the history restarts);
//! - a still camera keeps adding samples, so the history becomes the exact running
//!   average of jittered frames: the GL renderer's accumulation, now with every effect
//!   in it, and with the noise of contact shadows, reflections and ambient occlusion
//!   averaged away. After the *Accumulation samples* preference's count (or a few frames
//!   without accumulation) it has converged and the host stops drawing;
//! - a change drawn with a still camera (a selection click, an edit) keeps up to four
//!   samples: the neighbourhood clip throws out what changed, the rest stays smooth. It
//!   is drawn with the last frame's jitter, so the renderer can keep the last frame's
//!   depth, normals and occlusion when only the shading changed (the click is cheap);
//!   refining frames after it move on through the sequence;
//! - objects moving under a still camera (the animation preview, a drag) keep up to
//!   sixteen samples and move on through the jitter sequence every frame, so the noise
//!   of the effects keeps averaging out. The prepass writes each surface's motion, and
//!   the filter reprojects every pixel by it as well as by the camera's motion; a
//!   per-pixel history (sample count and depth) lets surfaces that were hidden last frame
//!   start afresh, filtered spatially until they have gathered a few samples, while the
//!   rest keeps its history.

/// Frames a still view takes to settle when accumulation is off.
pub(crate) const STEADY_FRAMES: u32 = 8;
/// The history a moving camera keeps (a weight of one tenth).
const MOVING_SAMPLES: u32 = 9;
/// The history kept through a change drawn with a still camera.
const CHANGED_SAMPLES: u32 = 4;
/// The history kept while objects move under a still camera: the motion vectors keep
/// it valid, so it can be longer than a moving camera's.
const ANIMATED_SAMPLES: u32 = 16;
/// The Halton sequence repeats after this many jitters.
const JITTER_PERIOD: u32 = 64;

/// What happened to the scene since the last frame, besides the camera.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneChange {
    /// Nothing the host did not say (see `continuing`).
    None,
    /// Geometry changed without motion vectors (an edit, an object appeared).
    Changed,
    /// Objects moved, and the prepass has their motion.
    Moving,
}

/// What one frame of the filter does.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TaaFrame {
    /// The history to read and the one to write.
    pub read: usize,
    pub write: usize,
    /// The current frame's weight; 1 ignores the history.
    pub weight: f32,
    /// How many standard deviations the history may stray from the neighbourhood.
    pub spread: f32,
    /// The Halton index of this frame's jitter; `None` for a plain frame.
    pub jitter: Option<u32>,
    /// Varies per-pixel noise from frame to frame.
    pub noise: u32,
    /// Last frame's unjittered view-projection.
    pub previous: [[f32; 4]; 4],
    view_proj: [[f32; 4]; 4],
}

/// The filter's memory between frames.
#[derive(Default)]
pub(crate) struct TaaState {
    /// The history holding the latest picture.
    current: usize,
    /// Samples in it; 0 when it holds nothing usable.
    samples: u32,
    /// What a still view converges at; 0 while the filter is off.
    target: u32,
    /// The unjittered view-projection the history was drawn with.
    view_proj: [[f32; 4]; 4],
    /// Counts frames, for the jitter and the noise.
    frame: u32,
    /// The last frame repeated the one before's jitter.
    repeated: bool,
}

impl TaaState {
    /// Forgets the history (new targets, the filter turned off, a wireframe frame).
    pub fn reset(&mut self) {
        self.samples = 0;
        self.target = 0;
    }

    /// Plans a frame with the camera's unjittered `view_proj`. `continuing` says nothing
    /// changed but the frame count; `antialiasing` whether a moving camera is filtered;
    /// `target` the samples a still view converges at.
    pub fn begin(&mut self, view_proj: &[[f32; 4]; 4], continuing: bool, antialiasing: bool, target: u32, scene: SceneChange) -> TaaFrame {
        let moved = self.samples > 0 && self.view_proj != *view_proj;
        let mut plain = false;
        if moved {
            if antialiasing {
                self.samples = self.samples.min(MOVING_SAMPLES);
            } else {
                self.samples = 0;
                plain = true;
            }
        } else if scene == SceneChange::Moving {
            self.samples = self.samples.min(ANIMATED_SAMPLES);
        } else if !continuing || scene == SceneChange::Changed {
            self.samples = self.samples.min(CHANGED_SAMPLES);
        }
        self.target = target;
        let previous = if self.samples > 0 { self.view_proj } else { *view_proj };
        let still = self.samples > 0 && !moved;
        // A change of shading alone under a still camera repeats the last jitter (see the
        // module notes). Never when geometry changed: the prepass is drawn anyway, and
        // repeating every frame of an animation would freeze its noise in place.
        let repeat = still && !continuing && scene == SceneChange::None && self.frame > 0;
        let index = if repeat { self.frame - 1 } else { self.frame };
        self.repeated = repeat;
        TaaFrame {
            read: self.current,
            write: 1 - self.current,
            weight: 1.0 / (self.samples + 1) as f32,
            // A still camera's samples differ only by the jitter: a wide box lets them
            // average to the supersampled picture; a moving one clips tightly.
            spread: if still && continuing && scene == SceneChange::None { 2.5 } else { 1.25 },
            jitter: (!plain).then_some(index % JITTER_PERIOD + 1),
            noise: index % JITTER_PERIOD,
            previous,
            view_proj: *view_proj,
        }
    }

    /// Records that `frame` was drawn.
    pub fn finish(&mut self, frame: TaaFrame) {
        self.current = frame.write;
        self.samples = (self.samples + 1).min(1 << 16);
        self.view_proj = frame.view_proj;
        if !self.repeated {
            self.frame = self.frame.wrapping_add(1);
        }
    }

    /// Whether a still view has gathered all it will.
    pub fn converged(&self) -> bool {
        self.target == 0 || self.samples >= self.target
    }
}

/// The per-pixel history beside the picture: the view depth of the surface the history
/// holds, and how many samples it has gathered.
pub(crate) const META_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// The views the filter reads and the post passes that read its output.
pub(crate) struct Sources<'a> {
    pub hdr: &'a wgpu::TextureView,
    pub ssr: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub motion: &'a wgpu::TextureView,
    pub ids: &'a wgpu::TextureView,
    pub bloom: &'a wgpu::TextureView,
    pub tony: &'a wgpu::TextureView,
    pub sampler: &'a wgpu::Sampler,
}

/// Two history pictures (each read while the other is written) and the bind groups
/// that use them.
pub(crate) struct TaaTargets {
    pub views: [wgpu::TextureView; 2],
    /// The per-pixel history ([`META_FORMAT`]) beside each picture.
    pub meta: [wgpu::TextureView; 2],
    /// The filter reading history `i` (and writing the other).
    pub taa_groups: [wgpu::BindGroup; 2],
    /// The resolve and the bloom reading history `i` as the scene's light.
    pub resolve_groups: [wgpu::BindGroup; 2],
    pub bloom_groups: [wgpu::BindGroup; 2],
}

impl TaaTargets {
    pub fn new(device: &wgpu::Device, (width, height): (u32, u32), taa_layout: &wgpu::BindGroupLayout, resolve_layout: &wgpu::BindGroupLayout, bloom_layout: &wgpu::BindGroupLayout, s: Sources<'_>) -> Self {
        let make = |label: &str, format: wgpu::TextureFormat| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let views = [0, 1].map(|_| make("render_wgpu taa history", wgpu::TextureFormat::Rgba16Float));
        let meta = [0, 1].map(|_| make("render_wgpu taa history depth and samples", META_FORMAT));
        use wgpu::BindingResource::{Sampler, TextureView};
        let group = |label: &str, layout: &wgpu::BindGroupLayout, entries: &[(u32, wgpu::BindingResource<'_>)]| {
            let entries: Vec<wgpu::BindGroupEntry<'_>> = entries.iter().map(|(binding, resource)| wgpu::BindGroupEntry { binding: *binding, resource: resource.clone() }).collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some(label), layout, entries: &entries })
        };
        let taa_groups = [0, 1].map(|i| {
            group(
                "render_wgpu taa",
                taa_layout,
                &[(0, TextureView(s.hdr)), (1, TextureView(s.ssr)), (2, TextureView(s.depth)), (3, TextureView(&views[i])), (4, Sampler(s.sampler)), (5, TextureView(s.motion)), (6, TextureView(&meta[i]))],
            )
        });
        let resolve_groups = [0, 1].map(|i| {
            group(
                "render_wgpu resolve (taa)",
                resolve_layout,
                &[(0, TextureView(&views[i])), (1, TextureView(s.ids)), (2, TextureView(s.depth)), (3, TextureView(s.bloom)), (4, TextureView(s.tony)), (5, Sampler(s.sampler)), (9, TextureView(s.ssr))],
            )
        });
        let bloom_groups = [0, 1].map(|i| group("render_wgpu bloom source (taa)", bloom_layout, &[(0, TextureView(&views[i])), (1, Sampler(s.sampler))]));
        Self { views, meta, taa_groups, resolve_groups, bloom_groups }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(x: f32) -> [[f32; 4]; 4] {
        [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [x, 0.0, 0.0, 1.0]]
    }

    #[test]
    fn a_still_view_averages_until_it_converges() {
        let mut taa = TaaState::default();
        let first = taa.begin(&matrix(0.0), false, true, 16, SceneChange::None);
        assert_eq!(first.weight, 1.0);
        taa.finish(first);
        for n in 1..16 {
            assert!(!taa.converged());
            let frame = taa.begin(&matrix(0.0), true, true, 16, SceneChange::None);
            assert!((frame.weight - 1.0 / (n + 1) as f32).abs() < 1e-6);
            assert!(frame.jitter.is_some());
            taa.finish(frame);
        }
        assert!(taa.converged());
    }

    #[test]
    fn moving_keeps_a_tenth_and_a_change_keeps_a_little_history() {
        let mut taa = TaaState::default();
        for _ in 0..20 {
            let frame = taa.begin(&matrix(0.0), true, true, 16, SceneChange::None);
            taa.finish(frame);
        }
        let moved = taa.begin(&matrix(1.0), false, true, 16, SceneChange::None);
        assert!((moved.weight - 0.1).abs() < 1e-6);
        assert_eq!(moved.previous, matrix(0.0));
        taa.finish(moved);
        let changed = taa.begin(&matrix(1.0), false, true, 16, SceneChange::None);
        assert!((changed.weight - 0.2).abs() < 1e-6);
        // Drawn with the moved frame's jitter; the next refining frame moves on.
        assert_eq!(changed.jitter, moved.jitter);
        taa.finish(changed);
        let refining = taa.begin(&matrix(1.0), true, true, 16, SceneChange::None);
        assert_ne!(refining.jitter, moved.jitter);
        taa.finish(refining);
        // Without antialiasing a moving camera draws plain frames.
        let plain = taa.begin(&matrix(2.0), false, false, 16, SceneChange::None);
        assert_eq!(plain.weight, 1.0);
        assert!(plain.jitter.is_none());
    }

    #[test]
    fn moving_objects_keep_sixteen_samples_and_turn_the_jitter() {
        let mut taa = TaaState::default();
        for _ in 0..40 {
            let frame = taa.begin(&matrix(0.0), true, true, 64, SceneChange::None);
            taa.finish(frame);
        }
        let mut jitters = Vec::new();
        for _ in 0..20 {
            // As the host draws an animation: nothing it knows of changed, objects moved.
            let frame = taa.begin(&matrix(0.0), false, true, 64, SceneChange::Moving);
            assert!(frame.weight >= 1.0 / 17.0 - 1e-6 && frame.weight < 0.1, "{}", frame.weight);
            jitters.push(frame.jitter.expect("jittered"));
            taa.finish(frame);
            assert!(!taa.converged());
        }
        jitters.dedup();
        assert_eq!(jitters.len(), 20, "an animation froze the jitter: {jitters:?}");
        // An edit without motion vectors keeps four samples but does not repeat the jitter.
        let changed = taa.begin(&matrix(0.0), false, true, 64, SceneChange::Changed);
        assert!((changed.weight - 0.2).abs() < 1e-6);
        assert_ne!(changed.jitter, Some(*jitters.last().unwrap()));
    }
}
