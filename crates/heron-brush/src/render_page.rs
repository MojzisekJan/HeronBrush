//! The render window: the camera and its
//! settings down the left, a live preview at the output's aspect ratio on the right that
//! redraws as they change and shows a render filling in sample by sample, and Render PNG
//! and Render GIF under it. A GIF is the map's animations played from the start, each
//! frame a whole render. The camera and its settings live in the map, as a render camera entity, and
//! the window follows edits made to it elsewhere (the gizmo, undo, the Entity tab).
//!
//! The preview is drawn in [`tick`] by the renderer eframe runs on: on GL by the shared
//! renderer into a framebuffer of its own, with the main window's GL context current; on
//! wgpu (the `wgpu` feature) by a `render_wgpu` renderer of its own, which also does the
//! offline accumulation. Either is shown in the window as the 3D view is (`view3d.rs`).
//!
//! [`tick`]: RenderPage::tick
use std::sync::Arc;
use web_time::{Duration, Instant};

use crate::{
    colour_picker, dialog, editor::Editor, file_dialog::FileDialog, form, theme, view3d::Gpu,
    view3d::ProcAddress, widgets,
};
use editor_core::{
    camera3d::Camera3d,
    i18n::{tr, translation_key},
    render_camera::{self, Job, Lighting, Navigation, Settings},
};
use eframe::{egui, egui_glow};
use glow::HasContext;
use map_model::node::NodeId;

/// The output presets: a name, a width and a height.
pub const OUTPUT_PRESETS: [(&str, i32, i32); 3] = [
    (translation_key("Full HD"), 1920, 1080),
    (translation_key("Square"), 1080, 1080),
    (translation_key("Portrait"), 1080, 1920),
];

/// The lighting presets, in the order [`Lighting::preset`] numbers them.
const LIGHTING_PRESETS: [&str; 3] = [
    translation_key("Daylight"),
    translation_key("Sunset"),
    translation_key("Overcast"),
];

/// The window's numbers, in the order they are shown: a label, the lowest and highest value, and the
/// step. 0–5 are the camera, 6–9 the output and lens, 10–16 the lighting, 17–19 the depth
/// of field, 20–21 the animation.
pub const NUMBERS: [(&str, f64, f64, f64); 22] = [
    (translation_key("X (map units)"), -1e6, 1e6, 1.),
    (translation_key("Y (map units)"), -1e6, 1e6, 1.),
    (translation_key("Z (map units)"), -1e6, 1e6, 1.),
    (translation_key("Yaw (degrees)"), -360., 360., 1.),
    (translation_key("Pitch (degrees)"), -89., 89., 1.),
    (translation_key("Far clip"), 64., 1_000_000., 32.),
    (translation_key("Width (px)"), 16., 4096., 1.),
    (translation_key("Height (px)"), 16., 4096., 1.),
    (translation_key("Samples"), 1., 64., 1.),
    (translation_key("Vertical field of view"), 10., 120., 1.),
    (translation_key("Exposure"), 0.1, 5., 0.1),
    (translation_key("Sun elevation"), 0., 90., 1.),
    (translation_key("Sun azimuth"), 0., 360., 1.),
    (translation_key("Sun intensity"), 0., 10., 0.1),
    (translation_key("Ambient intensity"), 0., 3., 0.05),
    (translation_key("Shadow softness"), 0., 4., 0.1),
    (translation_key("AO strength"), 0., 3., 0.1),
    (
        translation_key("Focus distance (units)"),
        1.,
        1_000_000.,
        16.,
    ),
    (translation_key("Sharp range (units)"), 0., 1_000_000., 16.),
    (translation_key("Blur radius (1080p px)"), 0., 24., 0.5),
    (translation_key("Length (seconds)"), 0.1, 120., 0.5),
    (
        translation_key("Frames per second"),
        1.,
        render_camera::MAX_VIDEO_FPS as f64,
        1.,
    ),
];

/// The colour rows of the Lighting section.
const COLOURS: [&str; 4] = [
    translation_key("Sky colour"),
    translation_key("Horizon colour"),
    translation_key("Ground colour"),
    translation_key("Sun colour"),
];

/// How often the window looks for edits made to its camera elsewhere.
const POLL: Duration = Duration::from_millis(40);

/// Sets a render's output size to one of [`OUTPUT_PRESETS`].
pub fn apply_output_preset(settings: &mut Settings, index: usize) {
    let (_, width, height) = OUTPUT_PRESETS[index % OUTPUT_PRESETS.len()];
    settings.width = width;
    settings.height = height;
}

/// The settings with their lighting spelled out, as the window edits them.
fn with_lighting(mut settings: Settings) -> Settings {
    if settings.lighting.is_none() {
        settings.lighting = Some(Lighting::from_render(&settings.render()));
    }
    settings
}

/// The framebuffer the preview is drawn into.
struct Target {
    framebuffer: glow::Framebuffer,
    color: glow::Renderbuffer,
    depth: glow::Renderbuffer,
    size: (i32, i32),
}

/// One saved camera as last seen in the map: its node, its pose and its settings.
type Seen = (NodeId, [f64; 5], Settings);

pub struct RenderPage {
    pub open: bool,
    /// The map the window was opened on; another one disables it.
    generation: Option<u64>,
    navigation: Navigation,
    frame: bool,
    camera: Camera3d,
    settings: Settings,
    /// The camera in the map as last loaded into the form.
    last_seen: Option<Seen>,
    polled: Option<Instant>,
    job: Option<Job>,
    /// The renderer's own context, shared: in WebGL one glow context cannot use objects
    /// another made.
    gl: Option<std::rc::Rc<glow::Context>>,
    renderer: Option<render::Renderer>,
    preview: Option<Target>,
    /// The preview's size in pixels, as the window last laid it out.
    wanted: Option<(i32, i32)>,
    /// The map content last uploaded to the renderer.
    uploaded: Option<(u64, u64)>,
    /// What the preview last drew, so a still form costs nothing.
    drawn: Option<String>,
    status: String,
    progress: (f32, String),
    /// The wgpu renderer and its preview, when eframe runs on wgpu.
    #[cfg(feature = "wgpu")]
    wgpu: Option<WgpuPreview>,
}

/// The page's own `render_wgpu` renderer: the scene as a clean render (no editor
/// helpers), drawn at the preview's size or, during a render, at the output's size one
/// sample at a time.
#[cfg(feature = "wgpu")]
struct WgpuPreview {
    device: eframe::egui_wgpu::wgpu::Device,
    renderer: render_wgpu::Renderer,
    blitter: Arc<render_wgpu::present::Blitter>,
    /// The blit bind group for the renderer's output, and the size it is for.
    picture: Option<((u32, u32), eframe::egui_wgpu::wgpu::BindGroup)>,
}

#[cfg(feature = "wgpu")]
impl WgpuPreview {
    fn new(state: &eframe::egui_wgpu::RenderState) -> Self {
        Self {
            device: state.device.clone(),
            renderer: render_wgpu::Renderer::new(&state.device, &state.queue),
            blitter: Arc::new(render_wgpu::present::Blitter::new(
                &state.device,
                state.target_format,
            )),
            picture: None,
        }
    }

    /// Replaces the scene with the document's clean render triangles, as one object, posed
    /// `time` seconds into the animation when given.
    fn upload(&mut self, document: &editor_core::document::Document, time: Option<f64>) {
        use editor_core::render_scene::SceneSink;
        let scene = &mut self.renderer.scene;
        scene.set_materials(document.materials().materials());
        scene.clear();
        scene.upsert(0, &document.render_triangles_at(time), Default::default());
        scene.set_lights(&document.scene_lights());
        scene.finish();
        // An offline render wants every texture from its first sample, not placeholders.
        scene.load_textures_now();
    }

    /// Keeps the blit bind group pointing at the current output.
    fn track_output(&mut self) {
        let Some(texture) = self.renderer.output() else {
            return;
        };
        let size = (texture.width(), texture.height());
        if self.picture.as_ref().map(|(s, _)| *s) != Some(size) {
            let view = self.renderer.output_view().expect("rendered");
            self.picture = Some((size, self.blitter.bind(&self.device, view)));
        }
    }
}

/// What the view settings' tone map preference asks of `render_wgpu`.
#[cfg(feature = "wgpu")]
pub fn tone_mapper() -> render_wgpu::Tonemapper {
    use editor_core::preferences::ToneMapper;
    match editor_core::render_settings::tone_mapper() {
        ToneMapper::Aces => render_wgpu::Tonemapper::Aces,
        ToneMapper::Agx => render_wgpu::Tonemapper::Agx,
        ToneMapper::KhronosNeutral => render_wgpu::Tonemapper::KhronosNeutral,
        ToneMapper::TonyMcMapface => render_wgpu::Tonemapper::TonyMcMapface,
    }
}

impl Default for RenderPage {
    fn default() -> Self {
        Self {
            open: false,
            generation: None,
            navigation: Navigation::default(),
            frame: false,
            camera: Camera3d::default(),
            settings: with_lighting(Settings::default()),
            last_seen: None,
            polled: None,
            job: None,
            gl: None,
            renderer: None,
            preview: None,
            wanted: None,
            uploaded: None,
            drawn: None,
            status: String::new(),
            progress: (0.0, String::new()),
            #[cfg(feature = "wgpu")]
            wgpu: None,
        }
    }
}

impl RenderPage {
    /// Opens the window on the selected render camera, or the first saved one, or the 3D
    /// view's camera when the map has none. Already open on this map, it stays as it is.
    pub fn open(&mut self, editor: &Editor) {
        let document = editor.document.borrow();
        if self.open && self.generation == Some(document.generation()) {
            return;
        }
        self.generation = Some(document.generation());
        match render_camera::active(&document) {
            Some((id, camera, settings)) => {
                self.load(camera, &settings);
                self.last_seen = Some((id, render_camera::pose(camera), settings));
            }
            None => {
                let view = *editor.camera.borrow();
                self.load(view, &Settings::default());
                self.view(view);
                self.last_seen = None;
            }
        }
        self.job = None;
        self.navigation = Navigation::default();
        self.frame = false;
        self.drawn = None;
        self.status =
            tr("Position the 3D view, then place a camera. Settings are saved with the map.")
                .to_string();
        self.progress = (0.0, tr("Ready").to_string());
        self.open = true;
    }

    /// Puts a saved camera and its settings in the form.
    fn load(&mut self, camera: Camera3d, settings: &Settings) {
        self.settings = with_lighting(settings.clone());
        self.camera = Camera3d {
            position: camera.position,
            yaw: camera.yaw,
            pitch: camera.pitch,
            ..Camera3d::default()
        };
        self.settings.far = camera.far;
    }

    /// Puts a view's position, direction, lens and far clip in the form.
    fn view(&mut self, camera: Camera3d) {
        self.camera.position = camera.position;
        self.camera.yaw = camera.yaw;
        self.camera.pitch = camera.pitch;
        self.settings.fov = camera.fov_degrees;
        self.settings.far = camera.far;
    }

    /// The camera and settings the form says, held to their ranges.
    fn read(&self) -> (Camera3d, Settings) {
        let settings = self.settings.clone().validated();
        let camera = Camera3d {
            position: self.camera.position,
            yaw: self.camera.yaw,
            pitch: self
                .camera
                .pitch
                .clamp(-89f64.to_radians(), 89f64.to_radians()),
            far: settings.far,
            fov_degrees: settings.fov,
            ..Camera3d::default()
        };
        (camera, settings)
    }

    /// Whether the map in the editor is no longer the one the window was opened on.
    fn stale(&self, editor: &Editor) -> bool {
        self.generation != Some(editor.document.borrow().generation())
    }

    /// The saved camera the window works on: the selected one, the one it last loaded, or
    /// the first.
    fn chosen(&self, editor: &Editor) -> Option<Seen> {
        render_camera::chosen(
            &editor.document.borrow(),
            self.last_seen.as_ref().map(|seen| seen.0),
        )
        .map(|(id, camera, settings)| (id, render_camera::pose(camera), settings))
    }

    /// Frees the renderer and the preview. The GL context must be current.
    pub fn destroy(&mut self) {
        if let Some(renderer) = self.renderer.take() {
            renderer.destroy();
        }
        if let (Some(gl), Some(target)) = (&self.gl, self.preview.take()) {
            // SAFETY: The caller keeps the GL context current, as the doc says (`tick` calls this
            // with the main window's context current). The names were made by `gl`, and `take`
            // removes them, so they are not used again.
            unsafe {
                gl.delete_framebuffer(target.framebuffer);
                gl.delete_renderbuffer(target.color);
                gl.delete_renderbuffer(target.depth);
            }
        }
        #[cfg(feature = "wgpu")]
        {
            self.wgpu = None;
        }
        self.uploaded = None;
        self.drawn = None;
        self.job = None;
    }

    /// Called every frame with the main window's GL context current, outside child
    /// viewport callbacks: follows the map and the 3D view, and draws the preview or the
    /// next sample of a render.
    pub fn tick(&mut self, ctx: &egui::Context, editor: &Editor, gpu: &Gpu) {
        if !self.open {
            #[cfg(feature = "wgpu")]
            let wgpu_held = self.wgpu.is_some();
            #[cfg(not(feature = "wgpu"))]
            let wgpu_held = false;
            if self.renderer.is_some() || self.preview.is_some() || wgpu_held {
                self.destroy();
            }
            return;
        }
        if self.stale(editor) {
            if self.job.take().is_some() || self.navigation.target.is_some() || self.frame {
                self.navigation = Navigation::default();
                self.frame = false;
                render_camera::set_frame(&editor.camera, None);
            }
            return;
        }
        if self.job.is_none() && self.polled.is_none_or(|at| at.elapsed() >= POLL) {
            self.polled = Some(Instant::now());
            self.follow(editor);
        }
        if self.navigation.target.is_some() {
            ctx.request_repaint_after(POLL);
        }
        let Some(wanted) = self.wanted else { return };
        match gpu {
            Gpu::Gl(proc_address) => self.tick_gl(ctx, editor, proc_address, wanted),
            #[cfg(feature = "wgpu")]
            Gpu::Wgpu(state) => match crate::view3d_wgpu::unsupported(state) {
                Some(why) => {
                    self.status = why;
                    self.job = None;
                }
                None => self.tick_wgpu(ctx, editor, state, wanted),
            },
        }
    }

    /// The preview or the next render sample, on wgpu.
    #[cfg(feature = "wgpu")]
    fn tick_wgpu(
        &mut self,
        ctx: &egui::Context,
        editor: &Editor,
        state: &eframe::egui_wgpu::RenderState,
        wanted: (i32, i32),
    ) {
        let content = {
            let document = editor.document.borrow();
            (document.generation(), document.content_revision())
        };
        let key = self.preview_key(content);
        let (camera, settings) = self.read();
        let preview = self.wgpu.get_or_insert_with(|| WgpuPreview::new(state));
        let document = editor.document.borrow();
        let view = |camera, size, settings, sample| render_wgpu::ViewParams {
            camera,
            size,
            settings,
            mode: render_wgpu::DisplayMode::Material,
            tonemapper: tone_mapper(),
            points: &[],
            guides: &[],
            world_bounds: None,
            outline_width: 0,
            sample,
            lights: editor_core::render_settings::entity_lights(),
            continuing: false,
        };
        if let Some(job) = &mut self.job {
            if job.completed == 0 {
                preview.upload(
                    &document,
                    job.video.as_ref().map(render_camera::Video::time),
                );
                // A posed scene is not the one the preview shows.
                self.uploaded = job.video.is_none().then_some(content);
            }
            let size = (
                job.settings.width.max(16) as u32,
                job.settings.height.max(16) as u32,
            );
            let render_settings = job.settings.render();
            preview.renderer.accumulate(&view(
                &job.camera,
                size,
                &render_settings,
                Some(job.completed),
            ));
            preview.track_output();
            let finished = job.advance();
            self.progress = progress(job);
            if finished {
                // `write_png` and `Video::push` take rows bottom first, as GL reads them.
                let pixels = preview.renderer.read_output().map(|(width, _, pixels)| {
                    let row = width as usize * 4;
                    pixels
                        .chunks_exact(row)
                        .rev()
                        .flatten()
                        .copied()
                        .collect::<Vec<u8>>()
                });
                let done = match pixels {
                    Some(pixels) => finish_frame(job, &pixels),
                    None => Some(tr("Render failed").to_string()),
                };
                if let Some(status) = done {
                    self.status = status;
                    // The finished render stays in the preview until the form changes.
                    self.drawn = Some(key);
                    self.job = None;
                    self.uploaded = None;
                }
            }
            ctx.request_repaint();
        } else if self.drawn.as_ref() != Some(&key) {
            if self.uploaded != Some(content) {
                preview.upload(&document, None);
                self.uploaded = Some(content);
            }
            let mut render_settings = settings.render();
            render_settings.accumulate = false;
            preview.renderer.render(&view(
                &camera,
                (wanted.0.max(1) as u32, wanted.1.max(1) as u32),
                &render_settings,
                None,
            ));
            preview.track_output();
            self.drawn = Some(key);
            ctx.request_repaint();
        }
    }

    /// The preview or the next render sample, on GL.
    fn tick_gl(
        &mut self,
        ctx: &egui::Context,
        editor: &Editor,
        proc_address: &ProcAddress,
        wanted: (i32, i32),
    ) {
        if self.renderer.is_none() {
            // SAFETY: `tick` runs with the main window's GL context current on this thread, and
            // `proc_address` is eframe's loader for it (`Gpu::from_creation_context`).
            let make = || unsafe { crate::gl_glue::context(proc_address) };
            match render::Renderer::new(make(), editor_core::render_settings::from_preferences()) {
                Ok(renderer) => {
                    self.gl = Some(std::rc::Rc::clone(renderer.gl()));
                    self.renderer = Some(renderer);
                }
                Err(error) => {
                    self.status = error;
                    self.wanted = None;
                    return;
                }
            }
        }
        if self.preview.as_ref().map(|target| target.size) != Some(wanted) {
            self.resize(wanted);
            self.drawn = None;
        }
        let content = {
            let document = editor.document.borrow();
            (document.generation(), document.content_revision())
        };
        let key = self.preview_key(content);
        let (camera, settings) = self.read();
        let (Some(gl), Some(target), Some(renderer)) =
            (&self.gl, &self.preview, &mut self.renderer)
        else {
            return;
        };
        let document = editor.document.borrow();
        // In WebGL the bound framebuffer may be egui's, which this context cannot name;
        // egui draws to the canvas there.
        #[cfg(not(target_arch = "wasm32"))]
        // SAFETY: `tick` runs with the main window's GL context current, which `gl` wraps.
        let previous = unsafe { gl.get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING) };
        #[cfg(target_arch = "wasm32")]
        let previous = None;
        // SAFETY: Same context, and `target.framebuffer` is the live preview `resize` made.
        unsafe { gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.framebuffer)) };
        if let Some(job) = &mut self.job {
            if job.completed == 0 {
                renderer.upload_materials(document.materials().materials());
                renderer.upload(
                    &document
                        .render_triangles_at(job.video.as_ref().map(render_camera::Video::time)),
                );
                renderer.set_settings(job.settings.render());
                // A posed scene is not the one the preview shows.
                self.uploaded = job.video.is_none().then_some(content);
            }
            let result = renderer.capture_step(
                &job.camera,
                job.settings.width,
                job.settings.height,
                job.completed == 0,
                target.size,
            );
            let done = match result {
                Ok(pixels) => {
                    let finished = job.advance();
                    self.progress = progress(job);
                    match finished.then(|| finish_frame(job, &pixels)).flatten() {
                        Some(status) => {
                            self.status = status;
                            // The finished render stays in the preview until the form changes.
                            self.drawn = Some(key);
                            self.uploaded = None;
                            true
                        }
                        None => false,
                    }
                }
                Err(error) => {
                    self.status = format!("{}: {error}", tr("Render failed"));
                    self.drawn = None;
                    true
                }
            };
            if done {
                self.job = None;
            }
            ctx.request_repaint();
        } else if self.drawn.as_ref() != Some(&key) {
            if self.uploaded != Some(content) {
                renderer.upload_materials(document.materials().materials());
                renderer.upload(&document.render_triangles());
                self.uploaded = Some(content);
            }
            let mut preview = settings.render();
            preview.accumulate = false;
            renderer.set_settings(preview);
            renderer.draw(&camera, target.size.0, target.size.1);
            self.drawn = Some(key);
            ctx.request_repaint();
        }
        // SAFETY: Same context, and `previous` is the binding read above.
        unsafe { gl.bind_framebuffer(glow::FRAMEBUFFER, previous) };
    }

    /// What the preview depends on.
    fn preview_key(&self, content: (u64, u64)) -> String {
        let (camera, settings) = self.read();
        // The wgpu-only view settings are not in the render settings.
        let extra = (
            editor_core::render_settings::tone_mapper(),
            editor_core::render_settings::entity_lights(),
        );
        format!(
            "{:?}{settings:?}{:?}{content:?}{extra:?}",
            render_camera::pose(camera),
            self.wanted
        )
    }

    fn resize(&mut self, (width, height): (i32, i32)) {
        let Some(gl) = &self.gl else { return };
        if let Some(old) = self.preview.take() {
            // SAFETY: Only `tick_gl` calls this, with the context current. `old` was taken out of
            // `self.preview`, so its names are not used again.
            unsafe {
                gl.delete_framebuffer(old.framebuffer);
                gl.delete_renderbuffer(old.color);
                gl.delete_renderbuffer(old.depth);
            }
        }
        // SAFETY: Same context. The objects are made here, the previous draw binding is restored,
        // and an incomplete framebuffer is deleted rather than kept.
        unsafe {
            let (Ok(framebuffer), Ok(color), Ok(depth)) = (
                gl.create_framebuffer(),
                gl.create_renderbuffer(),
                gl.create_renderbuffer(),
            ) else {
                self.status = tr("Render failed").to_string();
                return;
            };
            #[cfg(not(target_arch = "wasm32"))]
            let previous = gl.get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING);
            #[cfg(target_arch = "wasm32")]
            let previous = None;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(color));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, width, height);
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(color),
            );
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, width, height);
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth),
            );
            let complete =
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, previous);
            if complete {
                self.preview = Some(Target {
                    framebuffer,
                    color,
                    depth,
                    size: (width, height),
                });
            } else {
                gl.delete_framebuffer(framebuffer);
                gl.delete_renderbuffer(color);
                gl.delete_renderbuffer(depth);
            }
        }
    }

    /// Follows the map: while the 3D view drives the camera, writes its moves to the map
    /// once it rests; otherwise loads the saved camera into the form whenever an edit
    /// elsewhere changes it.
    fn follow(&mut self, editor: &Editor) {
        if self.navigation.target.is_some() {
            let id = self.navigation.target.expect("checked");
            let saved = render_camera::cameras(&editor.document.borrow())
                .into_iter()
                .find(|camera| camera.0 == id);
            let Some((id, saved_camera, saved_settings)) = saved else {
                self.navigation = Navigation::default();
                self.frame = false;
                render_camera::set_frame(&editor.camera, None);
                self.status = tr("Camera removed. Navigation link stopped.").to_string();
                return;
            };
            let key = (
                id,
                render_camera::pose(saved_camera),
                saved_settings.clone(),
            );
            if !self.navigation.dirty && self.last_seen.as_ref() != Some(&key) {
                *editor.camera.borrow_mut() = saved_camera;
                self.load(saved_camera, &saved_settings);
                self.navigation.pose = Some(render_camera::pose(saved_camera));
                self.last_seen = Some(key);
            }
            let (_, settings) = self.read();
            let mut camera = *editor.camera.borrow();
            camera.fov_degrees = settings.fov;
            camera.far = settings.far;
            let pose = render_camera::pose(camera);
            if self.navigation.pose != Some(pose) {
                self.navigation.pose = Some(pose);
                self.navigation.last_move = Some(Instant::now());
                self.navigation.dirty = true;
                self.camera.position = camera.position;
                self.camera.yaw = camera.yaw;
                self.camera.pitch = camera.pitch;
            }
            let rested = self
                .navigation
                .last_move
                .is_some_and(|at| at.elapsed() >= Duration::from_millis(350));
            if self.navigation.dirty && rested && !editor.document.borrow().in_transaction() {
                render_camera::save_target(
                    &mut editor.document.borrow_mut(),
                    Some(id),
                    camera,
                    &settings,
                );
                self.navigation.dirty = false;
                self.last_seen = Some((id, pose, settings.clone()));
            }
            if self.frame {
                render_camera::set_frame(&editor.camera, Some(&settings));
            }
        } else {
            let saved = render_camera::chosen(
                &editor.document.borrow(),
                self.last_seen.as_ref().map(|seen| seen.0),
            );
            let key = saved.as_ref().map(|(id, camera, settings)| {
                (*id, render_camera::pose(*camera), settings.clone())
            });
            if self.last_seen != key {
                if let Some((_, camera, settings)) = saved {
                    self.load(camera, &settings);
                    if self.frame {
                        *editor.camera.borrow_mut() = camera;
                    }
                }
                self.last_seen = key;
            }
            if self.frame {
                render_camera::set_frame(&editor.camera, Some(&self.read().1));
            }
        }
    }

    /// Stops the 3D view driving the camera, writing where it was left to the map first.
    pub fn finish_navigation(&mut self, editor: &Editor) {
        let Some(id) = self.navigation.target else {
            return;
        };
        let view = *editor.camera.borrow();
        let moved =
            self.navigation.dirty || self.navigation.pose != Some(render_camera::pose(view));
        if moved
            && render_camera::exists(&editor.document.borrow(), id)
            && !editor.document.borrow().in_transaction()
        {
            let (mut camera, settings) = self.read();
            camera.position = view.position;
            camera.yaw = view.yaw;
            camera.pitch = view.pitch;
            render_camera::save_target(
                &mut editor.document.borrow_mut(),
                Some(id),
                camera,
                &settings,
            );
            self.last_seen = Some((id, render_camera::pose(camera), settings));
        }
        self.navigation = Navigation::default();
    }

    /// Writes the form's camera and settings to the map, as Save camera and settings does.
    fn save(&mut self, editor: &Editor) -> NodeId {
        let (camera, settings) = self.read();
        let target = self.chosen(editor).map(|seen| seen.0);
        let id = render_camera::save_target(
            &mut editor.document.borrow_mut(),
            target,
            camera,
            &settings,
        );
        self.last_seen = self.last_seen.take().filter(|seen| seen.0 == id).or(Some((
            id,
            render_camera::pose(camera),
            settings,
        )));
        id
    }

    pub fn show(&mut self, ctx: &egui::Context, editor: &Editor) {
        if !self.open {
            return;
        }
        let embedded = ctx.embed_viewports();
        let mut close = false;
        self.open = dialog::show(
            ctx,
            "render-camera",
            tr("Render image"),
            [1040.0, 700.0],
            |ui, part| {
                if part == dialog::Part::Actions {
                    close = dialog::button(ui, tr("Close"), true).clicked();
                    return;
                }
                let full = form::body_rect(ui, embedded);
                let left = egui::Rect::from_min_size(
                    full.min,
                    egui::vec2(370.0_f32.min(full.width() * 0.45), full.height()),
                );
                let right =
                    egui::Rect::from_min_max(egui::pos2(left.right() + 16.0, full.top()), full.max);
                let mut controls = form::region(ui, left);
                egui::ScrollArea::vertical()
                    .id_salt("render-controls")
                    .auto_shrink([false, false])
                    .show(&mut controls, |ui| {
                        ui.set_max_width(ui.available_width() - 10.0);
                        ui.add_enabled_ui(self.job.is_none() && !self.stale(editor), |ui| {
                            self.controls(ui, editor)
                        });
                    });
                let mut output = form::region(ui, right);
                self.output(&mut output, editor);
            },
        ) && !close;
        if !self.open {
            self.job = None;
            self.finish_navigation(editor);
            self.frame = false;
            render_camera::set_frame(&editor.camera, None);
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let width = ui.available_width();
        let button = |ui: &mut egui::Ui, label: &str| theme::button(ui, label, width, true);
        if button(ui, tr("Place camera from 3D view"))
            .on_hover_text("Ctrl+Alt+Num0 / Ctrl+Alt+0")
            .clicked()
        {
            self.view(*editor.camera.borrow());
            self.save(editor);
            self.status =
                tr("Camera saved in the map. Save the map to keep it on disk.").to_string();
        }
        if button(ui, tr("Save camera and settings")).clicked() {
            self.save(editor);
            self.status =
                tr("Camera saved in the map. Save the map to keep it on disk.").to_string();
        }
        if button(ui, tr("Select camera in map")).clicked() {
            if let Some((id, ..)) = self.chosen(editor) {
                editor.document.borrow_mut().select_only(id);
            }
        }
        if button(ui, tr("Move 3D view to camera")).clicked() {
            *editor.camera.borrow_mut() = self.read().0;
        }
        ui.add_space(4.0);
        let mut frame = self.frame;
        if form::switch_row(ui, tr("Camera frame in 3D view"), None, &mut frame) {
            self.frame = frame;
            if frame {
                let (camera, settings) = self.read();
                *editor.camera.borrow_mut() = camera;
                render_camera::set_frame(&editor.camera, Some(&settings));
            } else {
                self.finish_navigation(editor);
                render_camera::set_frame(&editor.camera, None);
            }
        }
        let mut linked = self.navigation.target.is_some();
        if form::switch_row(ui, tr("Control camera from 3D view"), None, &mut linked) {
            if linked {
                let (camera, settings) = self.read();
                let id = self.save(editor);
                self.navigation = Navigation {
                    target: Some(id),
                    pose: Some(render_camera::pose(camera)),
                    ..Default::default()
                };
                self.last_seen = Some((id, render_camera::pose(camera), settings.clone()));
                *editor.camera.borrow_mut() = camera;
                self.frame = true;
                render_camera::set_frame(&editor.camera, Some(&settings));
            } else {
                self.finish_navigation(editor);
            }
        }
        ui.add_space(8.0);
        widgets::card(ui, tr("Presets"), None, |ui| {
            let names: Vec<&str> = OUTPUT_PRESETS.iter().map(|(name, ..)| tr(name)).collect();
            if let Some(index) = form::button_strip(ui, &names, true) {
                apply_output_preset(&mut self.settings, index);
            }
            let names: Vec<&str> = LIGHTING_PRESETS.iter().map(|name| tr(name)).collect();
            if let Some(index) = form::button_strip(ui, &names, true) {
                let (lighting, elevation, azimuth) = Lighting::preset(index);
                self.settings.lighting = Some(lighting);
                self.settings.sun_elevation = elevation as f32;
                self.settings.sun_azimuth = azimuth as f32;
            }
        });
        ui.add_space(8.0);
        let sections = [
            translation_key("Camera position"),
            translation_key("Output and lens"),
            translation_key("Lighting"),
            translation_key("Depth of field"),
            translation_key("Animation"),
        ];
        for (index, title) in sections.into_iter().enumerate() {
            widgets::card(ui, "", None, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                form::fold(ui, title, tr(title), index == 1, |ui| {
                    self.section(ui, index, editor)
                });
            });
            ui.add_space(6.0);
        }
        if theme::button(ui, tr("Load camera from map"), width, true).clicked() {
            if let Some((id, camera, settings)) = render_camera::chosen(
                &editor.document.borrow(),
                self.last_seen.as_ref().map(|seen| seen.0),
            ) {
                self.load(camera, &settings);
                self.last_seen = Some((id, render_camera::pose(camera), settings));
            }
        }
    }

    /// One of the five folding sections.
    fn section(&mut self, ui: &mut egui::Ui, index: usize, editor: &Editor) {
        let numbers = |page: &mut Self, ui: &mut egui::Ui, range: std::ops::Range<usize>| {
            for number in range {
                page.number_row(ui, number);
            }
        };
        match index {
            0 => numbers(self, ui, 0..6),
            1 => numbers(self, ui, 6..10),
            2 => {
                numbers(self, ui, 10..17);
                let lighting = self.settings.lighting.get_or_insert_with(|| {
                    Lighting::from_render(&editor_core::render_settings::from_preferences())
                });
                for (label, colour) in COLOURS.iter().zip([
                    &mut lighting.sky,
                    &mut lighting.horizon,
                    &mut lighting.ground,
                    &mut lighting.sun,
                ]) {
                    widgets::row(ui, tr(label), None, |ui| {
                        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                        let mut rgb = [byte(colour[0]), byte(colour[1]), byte(colour[2])];
                        if colour_picker::button(ui, ui.scope_id().with(*label), &mut rgb, None)
                            .changed()
                        {
                            *colour = rgb.map(|c| f32::from(c) / 255.0);
                        }
                        ui.label(
                            egui::RichText::new(colour_picker::hex(rgb, None))
                                .font(egui::FontId::monospace(theme::text_size() * 0.92))
                                .color(theme::dim()),
                        );
                    });
                }
                form::switch_row(
                    ui,
                    tr("Procedural sky"),
                    None,
                    &mut self.settings.procedural_sky,
                );
                form::switch_row(ui, tr("Shadows"), None, &mut self.settings.shadows);
                form::switch_row(ui, tr("Ambient occlusion"), None, &mut self.settings.ao);
                form::switch_row(ui, tr("Bloom"), None, &mut self.settings.bloom);
            }
            3 => {
                form::switch_row(
                    ui,
                    tr("Enable depth of field"),
                    None,
                    &mut self.settings.depth_of_field,
                );
                numbers(self, ui, 17..20);
                let width = ui.available_width();
                if theme::button(ui, tr("Focus on selection"), width, true).clicked() {
                    self.focus_on_selection(editor);
                }
            }
            _ => {
                numbers(self, ui, 20..22);
                let width = ui.available_width();
                if theme::button(ui, tr("Length from animation"), width, true).clicked() {
                    match render_camera::animation_length(&editor.document.borrow()) {
                        Some(seconds) => {
                            self.settings.video_seconds = seconds.clamp(0.1, 120.);
                            self.status =
                                tr("Length set to the longest animation in the map.").to_string();
                        }
                        None => {
                            self.status = tr("Nothing in the map plays on its own.").to_string()
                        }
                    }
                }
            }
        }
    }

    /// One of [`NUMBERS`]' rows.
    fn number_row(&mut self, ui: &mut egui::Ui, index: usize) {
        let (label, min, max, step) = NUMBERS[index];
        let decimals = if step < 0.1 {
            2
        } else if step < 1.0 {
            1
        } else {
            0
        };
        let mut value = self.number(index).clamp(min, max);
        widgets::row(ui, tr(label), None, |ui| {
            let width = ui.available_width().min(150.0);
            if widgets::spin_width(ui, &mut value, min..=max, step, decimals, width).changed() {
                self.set_number(index, value);
            }
        });
    }

    fn number(&self, index: usize) -> f64 {
        let s = &self.settings;
        let lighting = s
            .lighting
            .clone()
            .unwrap_or_else(|| Lighting::from_render(&s.render()));
        match index {
            0 => self.camera.position.x(),
            1 => self.camera.position.y(),
            2 => self.camera.position.z(),
            3 => self.camera.yaw.to_degrees(),
            4 => self.camera.pitch.to_degrees(),
            5 => s.far,
            6 => f64::from(s.width),
            7 => f64::from(s.height),
            8 => f64::from(s.samples),
            9 => s.fov,
            10 => f64::from(s.exposure),
            11 => f64::from(s.sun_elevation),
            12 => f64::from(s.sun_azimuth),
            13 => f64::from(lighting.sun_intensity),
            14 => f64::from(lighting.ambient_intensity),
            15 => f64::from(lighting.shadow_softness),
            16 => f64::from(lighting.ao_strength),
            17 => f64::from(s.focus_distance),
            18 => f64::from(s.focus_range),
            19 => f64::from(s.blur_radius),
            20 => s.video_seconds,
            _ => f64::from(s.video_fps),
        }
    }

    fn set_number(&mut self, index: usize, value: f64) {
        let s = &mut self.settings;
        let p = self.camera.position;
        match index {
            0 => self.camera.position = geometry_math::vec::Vec3d::new(value, p.y(), p.z()),
            1 => self.camera.position = geometry_math::vec::Vec3d::new(p.x(), value, p.z()),
            2 => self.camera.position = geometry_math::vec::Vec3d::new(p.x(), p.y(), value),
            3 => self.camera.yaw = value.to_radians(),
            4 => self.camera.pitch = value.to_radians(),
            5 => s.far = value,
            6 => s.width = value.round() as i32,
            7 => s.height = value.round() as i32,
            8 => s.samples = value.round() as u32,
            9 => s.fov = value,
            10 => s.exposure = value as f32,
            11 => s.sun_elevation = value as f32,
            12 => s.sun_azimuth = value as f32,
            13..=16 => {
                let render = s.render();
                let lighting = s
                    .lighting
                    .get_or_insert_with(|| Lighting::from_render(&render));
                let slot = match index {
                    13 => &mut lighting.sun_intensity,
                    14 => &mut lighting.ambient_intensity,
                    15 => &mut lighting.shadow_softness,
                    _ => &mut lighting.ao_strength,
                };
                *slot = value as f32;
            }
            17 => s.focus_distance = value as f32,
            18 => s.focus_range = value as f32,
            19 => s.blur_radius = value as f32,
            20 => s.video_seconds = value,
            _ => s.video_fps = value.round() as u32,
        }
    }

    /// Focuses the lens on the centre of the selection, and says how that went.
    fn focus_on_selection(&mut self, editor: &Editor) {
        let (camera, _) = self.read();
        let Some(bounds) = editor.document.borrow().selection_bounds() else {
            self.status = tr("Select an object in the map to focus on it.").to_string();
            return;
        };
        let distance =
            geometry_math::vec::dot(bounds.center() - camera.position, camera.direction());
        if distance > camera.near {
            self.settings.focus_distance = distance as f32;
            self.settings.depth_of_field = true;
            self.status = tr("Focus set to the centre of the selection.").to_string();
        } else {
            self.status = tr("The selection is behind the camera.").to_string();
        }
    }

    /// The preview, the status line and the render controls.
    fn output(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let stale = self.stale(editor);
        let area = ui.max_rect();
        let below = 4.0 * theme::CONTROL_HEIGHT + 22.0 + 5.0 * 8.0 + 36.0;
        let space = egui::Rect::from_min_size(
            area.min,
            egui::vec2(area.width(), (area.height() - below).max(120.0)),
        );
        let aspect =
            (f64::from(self.settings.width.max(1)) / f64::from(self.settings.height.max(1))) as f32;
        let size = if space.width() / space.height() > aspect {
            egui::vec2(space.height() * aspect, space.height())
        } else {
            egui::vec2(space.width(), space.width() / aspect)
        };
        let rect = egui::Rect::from_center_size(
            egui::pos2(space.center().x, space.top() + size.y / 2.0),
            size,
        );
        ui.allocate_rect(
            egui::Rect::from_min_max(space.min, egui::pos2(space.max.x, rect.max.y)),
            egui::Sense::hover(),
        );
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::ZERO, theme::base());
        let ppp = ui.ctx().pixels_per_point();
        let pixels = (
            (rect.width() * ppp).round().max(16.0) as i32,
            (rect.height() * ppp).round().max(16.0) as i32,
        );
        if self.wanted != Some(pixels) {
            self.wanted = Some(pixels);
            ui.ctx().request_repaint();
        }
        // In the browser the preview is blitted with the renderer's own context, found
        // through the registry (`gl_glue::publish`).
        #[cfg(target_arch = "wasm32")]
        if let (Some(target), Some(gl)) = (&self.preview, &self.gl) {
            let key = std::rc::Rc::as_ptr(gl) as usize as u64;
            crate::gl_glue::publish(key, gl, target.framebuffer);
            let (width, height) = target.size;
            let callback = egui_glow::CallbackFn::new(move |info, _| {
                crate::gl_glue::blit(key, width, height, info.viewport_in_pixels())
            });
            ui.painter().add(egui::PaintCallback {
                rect,
                callback: Arc::new(callback),
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(target) = &self.preview {
            let source = crate::gl_glue::name(target.framebuffer);
            let (width, height) = target.size;
            let callback = egui_glow::CallbackFn::new(move |info, painter| {
                use eframe::glow::HasContext as _;
                let gl = painter.gl();
                let viewport = info.viewport_in_pixels();
                // SAFETY: egui_glow runs paint callbacks with its context current, and
                // eframe's windows share one native context, so `source` names the
                // preview's framebuffer here too. A name deleted since only raises a GL
                // error. The read binding found on entry is restored.
                unsafe {
                    let previous = gl.get_parameter_i32(eframe::glow::READ_FRAMEBUFFER_BINDING);
                    let source = crate::gl_glue::eframe_framebuffer(source)
                        .expect("framebuffer names are non-zero");
                    gl.bind_framebuffer(eframe::glow::READ_FRAMEBUFFER, Some(source));
                    gl.blit_framebuffer(
                        0,
                        0,
                        width,
                        height,
                        viewport.left_px,
                        viewport.from_bottom_px,
                        viewport.left_px + viewport.width_px,
                        viewport.from_bottom_px + viewport.height_px,
                        eframe::glow::COLOR_BUFFER_BIT,
                        eframe::glow::LINEAR,
                    );
                    gl.bind_framebuffer(
                        eframe::glow::READ_FRAMEBUFFER,
                        crate::gl_glue::eframe_framebuffer(previous),
                    );
                }
            });
            ui.painter().add(egui::PaintCallback {
                rect,
                callback: Arc::new(callback),
            });
        }
        #[cfg(feature = "wgpu")]
        if let Some(preview) = &self.wgpu {
            if let Some((_, picture)) = &preview.picture {
                ui.painter()
                    .add(eframe::egui_wgpu::Callback::new_paint_callback(
                        rect,
                        crate::view3d_wgpu::Blit::new(
                            Arc::clone(&preview.blitter),
                            picture.clone(),
                        ),
                    ));
            }
        }
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::ZERO,
            egui::Stroke::new(1.0, theme::border()),
            egui::StrokeKind::Outside,
        );
        ui.add_space(10.0);
        let status = if stale {
            tr("The map changed. Close and reopen this render window.")
        } else {
            self.status.as_str()
        };
        form::text(ui, status);
        ui.add_space(8.0);
        let width = ui.available_width();
        let png =
            form::accent_button_width(ui, tr("Render PNG"), width, self.job.is_none() && !stale)
                .clicked();
        ui.add_space(4.0);
        let gif = theme::button(ui, tr("Render GIF"), width, self.job.is_none() && !stale)
            .on_hover_text(tr(
                "Plays the map's animations from the start, as the 3D view's preview does.",
            ))
            .clicked();
        self.render(editor, png, gif);
        ui.add_space(4.0);
        form::progress(ui, self.progress.0, &self.progress.1);
        ui.add_space(4.0);
        if theme::button(ui, tr("Cancel render"), width, self.job.is_some()).clicked() {
            self.job = None;
            self.drawn = None;
            self.status = tr("Render cancelled. No image was written.").to_string();
            self.uploaded = None;
            self.progress.1 = tr("Cancelled").to_string();
        }
    }

    /// Asks where to write the PNG or the GIF, saves the camera and its settings in the
    /// map, and starts the render; [`tick`](Self::tick) draws it a sample at a time.
    /// `png` and `gif` ask where; the render starts on the frame the answer comes, so this
    /// is called every frame the window shows.
    fn render(&mut self, editor: &Editor, png: bool, gif: bool) {
        let still = FileDialog::new()
            .id("render-png")
            .set_title(tr("Render PNG"))
            .add_filter("PNG image", &["png"])
            .set_file_name("render.png")
            .save_file(png);
        let animated = FileDialog::new()
            .id("render-gif")
            .set_title(tr("Render GIF"))
            .add_filter("GIF image", &["gif"])
            .set_file_name("render.gif")
            .save_file(gif);
        let (path, animated) = match (still, animated) {
            (Some(path), _) => (path, false),
            (None, Some(path)) => (path, true),
            (None, None) => return,
        };
        if self.stale(editor) {
            return;
        }
        self.finish_navigation(editor);
        let (camera, settings) = self.read();
        let video = if animated {
            match render_camera::Video::new(&settings) {
                Ok(video) => Some(video),
                Err(error) => {
                    self.status = format!("{}: {error}", tr("Render failed"));
                    return;
                }
            }
        } else {
            None
        };
        self.save(editor);
        self.status = tr("Rendering").to_string();
        self.progress = (0.0, tr("Rendering").to_string());
        self.job = Some(Job {
            path,
            camera,
            settings,
            completed: 0,
            video,
        });
    }
}

/// How far `job` has got: samples for a still, frames for an animation.
fn progress(job: &Job) -> (f32, String) {
    match &job.video {
        Some(video) => {
            let samples = video.frames * job.settings.samples;
            let done = video.frame * job.settings.samples + job.completed;
            (
                done as f32 / samples as f32,
                format!(
                    "{} {} / {}",
                    tr("Frame"),
                    (video.frame + 1).min(video.frames),
                    video.frames
                ),
            )
        }
        None => (
            job.completed as f32 / job.settings.samples as f32,
            format!("{} / {}", job.completed, job.settings.samples),
        ),
    }
}

/// Takes a finished frame's `pixels`, bottom row first: writes a still out, or adds the
/// frame to the animation and starts the next. Returns the status line once the job is
/// over, `None` while more frames are to come.
fn finish_frame(job: &mut Job, pixels: &[u8]) -> Option<String> {
    let (width, height) = (job.settings.width, job.settings.height);
    let saved = |path: &std::path::Path| {
        format!(
            "{}: {} ({} × {})",
            tr("Saved"),
            path.display(),
            width,
            height
        )
    };
    let Some(video) = &mut job.video else {
        return Some(
            match render_camera::write_png(&job.path, width, height, pixels) {
                Ok(()) => saved(&job.path),
                Err(error) => format!("{}: {error}", tr("Render failed")),
            },
        );
    };
    match video.push(pixels) {
        Ok(false) => {
            job.completed = 0;
            None
        }
        Ok(true) => {
            let video = job.video.take().expect("checked");
            Some(match video.finish(&job.path) {
                Ok(()) => saved(&job.path),
                Err(error) => format!("{}: {error}", tr("Render failed")),
            })
        }
        Err(error) => Some(format!("{}: {error}", tr("Render failed"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The output presets set their sizes; the lighting presets and number rows keep
    /// their order and ranges.
    #[test]
    fn presets_and_ranges_keep_their_values() {
        let mut settings = Settings::default();
        apply_output_preset(&mut settings, 2);
        assert_eq!((settings.width, settings.height), (1080, 1920));
        apply_output_preset(&mut settings, 1);
        assert_eq!((settings.width, settings.height), (1080, 1080));
        apply_output_preset(&mut settings, 0);
        assert_eq!((settings.width, settings.height), (1920, 1080));
        let range = |label: &str| {
            NUMBERS
                .iter()
                .find(|(name, ..)| *name == label)
                .map(|(_, min, max, _)| (*min, *max))
                .unwrap()
        };
        assert_eq!(range("Vertical field of view"), (10., 120.));
        assert_eq!(range("Exposure"), (0.1, 5.));
        assert_eq!(range("Sun elevation"), (0., 90.));
        assert_eq!(range("Focus distance (units)").0, 1.);
        // Every range is one the stored settings keep, so nothing typed is lost on save.
        let mut page = RenderPage::default();
        // The azimuth wraps round, so 360 reads back as 0.
        for (index, (_, min, max, _)) in NUMBERS
            .iter()
            .enumerate()
            .skip(5)
            .filter(|(index, _)| *index != 12)
        {
            for value in [*min, *max] {
                page.set_number(index, value);
                let (_, saved) = page.read();
                let read = RenderPage {
                    settings: saved,
                    ..RenderPage::default()
                };
                assert!(
                    (read.number(index) - value).abs() < 1e-3,
                    "{} at {value}",
                    NUMBERS[index].0
                );
            }
        }
    }

    /// Placing the camera writes it to the map at once, and an edit made to it elsewhere
    /// comes back into the form.
    #[test]
    fn the_form_follows_the_map() {
        let ctx = egui::Context::default();
        let editor = Editor::open(&ctx, None);
        let mut page = RenderPage::default();
        page.open(&editor);
        editor.camera.borrow_mut().position = geometry_math::vec::Vec3d::new(64., 32., 16.);
        page.view(*editor.camera.borrow());
        let id = page.save(&editor);
        let (_, saved, _) = render_camera::cameras(&editor.document.borrow())
            .into_iter()
            .find(|c| c.0 == id)
            .unwrap();
        assert_eq!(saved.position.x(), 64.);
        // Moved by something else: the window picks it up.
        let mut moved = saved;
        moved.position = geometry_math::vec::Vec3d::new(128., 32., 16.);
        let (_, settings) = page.read();
        render_camera::save_target(
            &mut editor.document.borrow_mut(),
            Some(id),
            moved,
            &settings,
        );
        page.follow(&editor);
        assert_eq!(page.camera.position.x(), 128.);
        assert!(!page.stale(&editor));
    }
}
