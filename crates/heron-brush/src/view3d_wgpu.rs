//! The 3D view on wgpu (the `wgpu` feature): the core's [`Scene3d`] synced into
//! `render_wgpu`'s retained scene through a [`SceneTracker`], drawn into the renderer's
//! own target, and shown in egui's render pass by a paint callback that blits it. The
//! overlay (handles, guides, axes) is drawn by the renderer; the HUD by egui, as on GL.

use std::sync::Arc;

use editor_core::render_scene::SceneTracker;
use editor_core::view3d::Scene3d;
use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use render_wgpu::present::Blitter;
use render_wgpu::{DisplayMode, Renderer, ViewParams};

/// The wgpu side of a view.
pub struct WgpuView {
    device: wgpu::Device,
    renderer: Renderer,
    tracker: SceneTracker,
    blitter: Arc<Blitter>,
    /// The blit bind group for the renderer's current output, and the size it is for.
    picture: Option<((u32, u32), wgpu::BindGroup)>,
    /// The picture the output holds ([`editor_core::view3d::picture_key`]).
    drawn: Option<u64>,
}

/// Blits a picture into egui's pass, inside the viewport egui set for the rect.
pub struct Blit {
    blitter: Arc<Blitter>,
    picture: wgpu::BindGroup,
}

impl Blit {
    /// Shows `picture` (a bind group from `blitter`).
    pub fn new(blitter: Arc<Blitter>, picture: wgpu::BindGroup) -> Self {
        Self { blitter, picture }
    }
}

impl egui_wgpu::CallbackTrait for Blit {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        _resources: &egui_wgpu::CallbackResources,
    ) {
        self.blitter.draw(render_pass, &self.picture);
    }
}

/// Why render_wgpu cannot draw on this adapter, if it cannot: it keeps its object and
/// material tables in storage buffers read by the vertex stage, which GLES 3.0 (wgpu's GL
/// backend, a phone without Vulkan) lacks. docs/renderer-wgpu.md, *Not done
/// yet*, plans the downlevel path.
pub fn unsupported(state: &egui_wgpu::RenderState) -> Option<String> {
    let info = state.adapter.get_info();
    let flags = state.adapter.get_downlevel_capabilities().flags;
    let limits = state.adapter.limits();
    let missing = !flags.contains(wgpu::DownlevelFlags::VERTEX_STORAGE)
        || limits.max_storage_buffers_per_shader_stage < 4;
    (info.backend == wgpu::Backend::Gl || missing).then(|| {
        format!(
            "The 3D view needs Vulkan, Metal or Direct3D 12; {} offers {:?}. The 2D views work.",
            info.name, info.backend
        )
    })
}

impl WgpuView {
    /// A view on eframe's device.
    pub fn new(state: &egui_wgpu::RenderState) -> Self {
        let mut renderer = Renderer::new(&state.device, &state.queue);
        renderer.set_profiling(crate::profile::enabled());
        Self {
            device: state.device.clone(),
            renderer,
            tracker: SceneTracker::new(),
            blitter: Arc::new(Blitter::new(&state.device, state.target_format)),
            picture: None,
            drawn: None,
        }
    }

    /// Syncs, draws when the picture changed, and shows it in `rect`. Returns whether it
    /// drew a new picture.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        scene: &Scene3d,
        size: (f64, f64),
        scale: i32,
        profiling: bool,
    ) -> bool {
        let pixels = (
            (f64::from(rect.width()) * f64::from(scale))
                .round()
                .max(1.0) as u32,
            (f64::from(rect.height()) * f64::from(scale))
                .round()
                .max(1.0) as u32,
        );
        self.renderer.scene.stats = Default::default();
        self.renderer
            .set_texture_budget(editor_core::render_settings::texture_memory_bytes());
        let frame = scene.prepare_frame(
            &mut self.tracker,
            &mut self.renderer.scene,
            size.0,
            size.1,
            scale,
        );
        let resized = self.picture.as_ref().map(|(s, _)| *s) != Some(pixels);
        // The tone map is not in the core's render settings (GL has only ACES), so the
        // picture key alone would keep a picture drawn with another one.
        let tonemapper = crate::render_page::tone_mapper();
        let lights = editor_core::render_settings::entity_lights();
        let key = frame.picture_key
            ^ (tonemapper as u64 * 2 + u64::from(lights)).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        // The same picture again while the temporal filter still refines a still view;
        // once it has converged, nothing is drawn until something changes.
        let refining =
            !resized && !frame.playing && self.drawn == Some(key) && !self.renderer.converged();
        let drew = resized || frame.playing || self.drawn != Some(key) || refining;
        // Nothing the host knows of changed: a refining frame, or a frame of an animation
        // (which moves objects without moving the key; the renderer sees their motion and
        // filters accordingly).
        let continuing = !resized && self.drawn == Some(key);
        if drew {
            let mode = if frame.wireframe {
                DisplayMode::Wireframe
            } else if frame.settings.realistic_shading {
                DisplayMode::Material
            } else {
                DisplayMode::Solid
            };
            let outline = if editor_core::preferences::pref(|p| p.selection_outline) {
                2 * scale.max(1) as u32
            } else {
                0
            };
            let camera = *scene.camera.borrow();
            self.renderer.render(&ViewParams {
                camera: &camera,
                size: pixels,
                settings: &frame.settings,
                mode,
                tonemapper,
                points: &frame.overlay.points,
                guides: &frame.overlay.guides,
                world_bounds: Some(&frame.world_bounds),
                outline_width: outline,
                sample: None,
                lights,
                continuing,
            });
            self.drawn = Some(key);
            if resized {
                let view = self.renderer.output_view().expect("rendered");
                self.picture = Some((pixels, self.blitter.bind(&self.device, view)));
            }
            if profiling {
                self.renderer.wait_idle();
                let (s, u) = (self.renderer.stats, self.renderer.scene.stats);
                let times = self.renderer.gpu_times().to_vec();
                crate::profile::wgpu_passes(&times);
                let gpu: Vec<String> = times
                    .iter()
                    .map(|(label, ms)| format!("{label} {ms:.2}"))
                    .collect();
                if u.chunks > 0
                    || self.tracker.stats.upserted > 0
                    || self.tracker.stats.states > 0
                    || s.environment_rebuilt
                {
                    eprintln!(
                        "[profile] wgpu sync: {:?}; upload: {} chunks, {} KiB, {} object slots; draw: {} calls, {} chunks ({} culled), {} shadow cascades, {} lights in view, {} light shadow faces, prepass {}, environment {}, encode {:.2}ms; gpu ms: {}",
                        self.tracker.stats,
                        u.chunks,
                        u.geometry_bytes / 1024,
                        u.object_slots,
                        s.draws,
                        s.chunks_drawn,
                        s.chunks_culled,
                        s.cascades_drawn,
                        s.lights,
                        s.light_faces_drawn,
                        if s.prepass_cached { "cached" } else { "drawn" },
                        if s.environment_rebuilt { "rebuilt" } else { "cached" },
                        s.encode.as_secs_f64() * 1000.0,
                        gpu.join(", ")
                    );
                }
            }
        }
        if let Some(path) = crate::script::take_view_shot() {
            if let Some((w, h, pixels)) = self.renderer.read_output() {
                match editor_core::assets::encode_png(w, h, &pixels)
                    .and_then(|png| std::fs::write(&path, png).map_err(|e| e.to_string()))
                {
                    Ok(()) => println!("script: {}", path.display()),
                    Err(e) => eprintln!("script: {e}"),
                }
            }
        }
        if frame.playing || !self.renderer.converged() {
            ui.ctx().request_repaint();
        }
        if let Some((_, picture)) = &self.picture {
            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                rect,
                Blit {
                    blitter: Arc::clone(&self.blitter),
                    picture: picture.clone(),
                },
            ));
        }
        drew
    }
}
