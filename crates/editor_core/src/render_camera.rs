//! Saved render cameras: the camera entities a map keeps, their settings and lighting,
//! the frame a camera's aspect draws in the 3D view, and writing a render out. The
//! window that renders an image is the frontend's.
use crate::{
    camera3d::Camera3d,
    document::Document,
};
use map_model::node::{self, NodeId};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, path::PathBuf, rc::Rc};
/// Editor-only render camera entity class.
pub const CLASS: &str = crate::document::RENDER_CAMERA_CLASS;
pub const KEY: &str = "_tb_render_settings";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub width: i32,
    pub height: i32,
    pub samples: u32,
    pub fov: f64,
    pub far: f64,
    pub exposure: f32,
    pub sun_elevation: f32,
    pub sun_azimuth: f32,
    pub shadows: bool,
    pub ao: bool,
    pub bloom: bool,
    pub procedural_sky: bool,
    pub lighting: Option<Lighting>,
    pub depth_of_field: bool,
    pub focus_distance: f32,
    pub focus_range: f32,
    pub blur_radius: f32,
    /// How long an animation render runs, in seconds.
    pub video_seconds: f64,
    /// An animation render's frames per second.
    pub video_fps: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            samples: 16,
            fov: 55.,
            far: 32768.,
            exposure: 1.,
            sun_elevation: 45.,
            sun_azimuth: 135.,
            shadows: true,
            ao: true,
            bloom: false,
            procedural_sky: true,
            lighting: None,
            depth_of_field: false,
            focus_distance: 512.,
            focus_range: 128.,
            blur_radius: 10.,
            video_seconds: 4.,
            video_fps: 25,
        }
    }
}
impl Settings {
    pub fn validated(mut self) -> Self {
        self.lighting = self.lighting.map(Lighting::validated);
        self.focus_distance = self.focus_distance.clamp(1., 1000000.);
        self.focus_range = self.focus_range.clamp(0., 1000000.);
        self.blur_radius = self.blur_radius.clamp(0., 24.);
        self.width = self.width.clamp(16, 4096);
        self.height = self.height.clamp(16, 4096);
        self.far = self.far.clamp(64., 1000000.);
        self.samples = self.samples.clamp(1, 64);
        self.fov = self.fov.clamp(10., 120.);
        self.exposure = self.exposure.clamp(0.1, 5.);
        self.sun_elevation = self.sun_elevation.clamp(0., 90.);
        self.sun_azimuth = self.sun_azimuth.rem_euclid(360.);
        self.video_seconds = if self.video_seconds.is_finite() { self.video_seconds.clamp(0.1, 120.) } else { 4. };
        self.video_fps = self.video_fps.clamp(1, MAX_VIDEO_FPS);
        self
    }
    pub fn render(&self) -> render::RenderSettings {
        let mut s = crate::render_settings::from_preferences();
        if let Some(lighting) = &self.lighting {
            lighting.apply(&mut s);
        }
        s.depth_of_field = self.depth_of_field.then_some(render::DepthOfField {
            focus_distance: self.focus_distance,
            focus_range: self.focus_range,
            radius: self.blur_radius,
        });
        s.show_axes = false;
        s.realistic_shading = true;
        s.shadows = self.shadows;
        s.ambient_occlusion = self.ao;
        s.bloom = self.bloom;
        s.procedural_sky = self.procedural_sky;
        s.exposure = self.exposure;
        s.sun_elevation = self.sun_elevation;
        s.sun_azimuth = self.sun_azimuth;
        s.accumulate = true;
        s.accumulation_samples = self.samples;
        s.antialiasing = true;
        s
    }
}
/// Camera-local environment controls. Older maps inherit editor preferences until saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lighting {
    pub sky: [f32; 3],
    pub horizon: [f32; 3],
    pub ground: [f32; 3],
    pub sun: [f32; 3],
    pub sun_intensity: f32,
    pub ambient_intensity: f32,
    pub shadow_softness: f32,
    pub ao_strength: f32,
}
impl Lighting {
    pub fn from_render(s: &render::RenderSettings) -> Self {
        Self {
            sky: s.sky_color,
            horizon: s.horizon_color,
            ground: s.ground_color,
            sun: s.sun_color,
            sun_intensity: s.sun_intensity,
            ambient_intensity: s.ambient_intensity,
            shadow_softness: s.shadow_softness,
            ao_strength: s.ao_strength,
        }
    }
    pub fn validated(mut self) -> Self {
        for color in [
            &mut self.sky,
            &mut self.horizon,
            &mut self.ground,
            &mut self.sun,
        ] {
            for c in color {
                *c = if c.is_finite() { c.clamp(0., 1.) } else { 0. };
            }
        }
        for (value, max) in [
            (&mut self.sun_intensity, 10.),
            (&mut self.ambient_intensity, 3.),
            (&mut self.shadow_softness, 4.),
            (&mut self.ao_strength, 3.),
        ] {
            *value = if value.is_finite() {
                value.clamp(0., max)
            } else {
                1.
            };
        }
        self
    }
    pub fn apply(&self, s: &mut render::RenderSettings) {
        s.sky_color = self.sky;
        s.horizon_color = self.horizon;
        s.ground_color = self.ground;
        s.sun_color = self.sun;
        s.sun_intensity = self.sun_intensity;
        s.ambient_intensity = self.ambient_intensity;
        s.shadow_softness = self.shadow_softness;
        s.ao_strength = self.ao_strength;
    }
    pub fn preset(index: usize) -> (Self, f64, f64) {
        let day = Self {
            sky: [0.34, 0.42, 0.55],
            horizon: [0.62, 0.68, 0.78],
            ground: [0.10, 0.09, 0.08],
            sun: [1., 0.94, 0.84],
            sun_intensity: 3.2,
            ambient_intensity: 0.45,
            shadow_softness: 1.,
            ao_strength: 1.,
        };
        match index {
            1 => (
                Self {
                    sky: [0.24, 0.29, 0.48],
                    horizon: [0.85, 0.46, 0.26],
                    sun: [1., 0.55, 0.24],
                    sun_intensity: 3.8,
                    ambient_intensity: 0.55,
                    shadow_softness: 1.8,
                    ..day
                },
                12.,
                220.,
            ),
            2 => (
                Self {
                    sky: [0.65, 0.69, 0.76],
                    horizon: [0.75, 0.77, 0.80],
                    sun: [0.90, 0.94, 1.],
                    sun_intensity: 0.6,
                    ambient_intensity: 1.2,
                    shadow_softness: 3.,
                    ..day
                },
                60.,
                135.,
            ),
            _ => (day, 45., 135.),
        }
    }
}
// Frame state belongs to a particular viewport, so separate map windows stay independent.
thread_local! {
    static FRAMES: RefCell<std::collections::HashMap<usize,(f64,f64)>> = RefCell::new(Default::default());
}
pub fn view_key(view: &Rc<RefCell<Camera3d>>) -> usize {
    Rc::as_ptr(view) as usize
}
pub fn set_frame(view: &Rc<RefCell<Camera3d>>, settings: Option<&Settings>) -> bool {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        let old = frames.get(&view_key(view)).copied();
        let next = settings.map(|s| (s.fov, s.width as f64 / s.height as f64));
        if let Some(next) = next {
            frames.insert(view_key(view), next);
        } else {
            frames.remove(&view_key(view));
        }
        old != next
    })
}
/// Output aspect ratio requested for this editor viewport.
pub fn view_aspect(view: &Rc<RefCell<Camera3d>>) -> Option<f64> {
    FRAMES.with(|f| f.borrow().get(&view_key(view)).map(|v| v.1))
}
/// Fit the camera's vertical FOV inside a viewport of any shape without changing framing.
pub fn view_fov(view: &Rc<RefCell<Camera3d>>, width: f64, height: f64) -> Option<f64> {
    FRAMES.with(|f| {
        f.borrow()
            .get(&view_key(view))
            .map(|&(fov, aspect)| frame_fov(fov, aspect, width, height))
    })
}
pub fn frame_fov(fov: f64, aspect: f64, width: f64, height: f64) -> f64 {
    let (_, _, _, fh) = frame_rect(aspect, width, height);
    (2. * ((fov.to_radians() / 2.).tan() * height.max(1.) / fh.max(1.)).atan()).to_degrees()
}
pub fn frame_rect(aspect: f64, width: f64, height: f64) -> (f64, f64, f64, f64) {
    let w = width.min(height * aspect).max(1.);
    let h = (w / aspect).max(1.);
    ((width - w) / 2., (height - h) / 2., w, h)
}
/// Shade the area outside the output image and show a thin composition frame.
pub fn draw_frame(cr: &mut dyn crate::canvas::Canvas, aspect: f64, width: f64, height: f64) {
    let (x, y, w, h) = frame_rect(aspect, width, height);
    let _ = cr.save();
    cr.set_source_rgba(0., 0., 0., 0.62);
    cr.rectangle(0., 0., width, y);
    cr.rectangle(0., y + h, width, height - y - h);
    cr.rectangle(0., y, x, h);
    cr.rectangle(x + w, y, width - x - w, h);
    let _ = cr.fill();
    cr.set_source_rgba(0.96, 0.75, 0.32, 0.95);
    cr.set_line_width(1.5);
    cr.rectangle(x + 0.75, y + 0.75, w - 1.5, h - 1.5);
    let _ = cr.stroke();
    let _ = cr.restore();
}
pub struct Job {
    pub path: PathBuf,
    pub camera: Camera3d,
    pub settings: Settings,
    pub completed: u32,
    /// The animation being rendered, a whole job's samples per frame; `None` for a still.
    pub video: Option<Video>,
}
impl Job {
    pub fn advance(&mut self) -> bool {
        self.completed += 1;
        self.completed >= self.settings.samples
    }
}
#[derive(Default)]
pub struct Navigation {
    pub target: Option<NodeId>,
    pub pose: Option<[f64; 5]>,
    pub last_move: Option<web_time::Instant>,
    pub dirty: bool,
}
pub fn pose(c: Camera3d) -> [f64; 5] {
    [
        c.position.x(),
        c.position.y(),
        c.position.z(),
        c.yaw,
        c.pitch,
    ]
}
pub fn exists(doc: &Document, id: NodeId) -> bool {
    cameras(doc).iter().any(|v| v.0 == id)
}
pub fn cameras(doc: &Document) -> Vec<(NodeId, Camera3d, Settings)> {
    node::point_entities_by_layer(doc.tree_world())
        .into_iter()
        .filter_map(|(id, e, _)| {
            if e.classname() != Some(CLASS) {
                return None;
            }
            let p = e.origin()?;
            if ![p.x(), p.y(), p.z()].iter().all(|v| v.is_finite()) {
                return None;
            }
            let angles: Vec<f64> = e
                .property("angles")
                .unwrap_or("0 0 0")
                .split_whitespace()
                .filter_map(|v| v.parse().ok())
                .collect();
            let settings = serde_json::from_str::<Settings>(e.property(KEY).unwrap_or("{}"))
                .unwrap_or_default()
                .validated();
            let yaw = angles
                .get(1)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(0.)
                .to_radians();
            let pitch = -angles
                .first()
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(0.)
                .clamp(-89., 89.)
                .to_radians();
            Some((
                id,
                Camera3d {
                    position: p,
                    yaw,
                    pitch,
                    fov_degrees: settings.fov,
                    far: settings.far,
                    ..Camera3d::default()
                },
                settings,
            ))
        })
        .collect()
}
pub fn active(doc: &Document) -> Option<(NodeId, Camera3d, Settings)> {
    chosen(doc, None)
}
pub fn chosen(doc: &Document, preferred: Option<NodeId>) -> Option<(NodeId, Camera3d, Settings)> {
    let list = cameras(doc);
    list.iter()
        .find(|(id, _, _)| doc.selection_ids().contains(id))
        .or_else(|| list.iter().find(|v| Some(v.0) == preferred))
        .or(list.first())
        .cloned()
}
/// Set the selected (or first) render camera from the perspective view, creating it if needed.
/// Render output and lighting settings survive; a visible render frame keeps its lens FOV.
pub fn align_to_view(doc: &mut Document, view: &Rc<RefCell<Camera3d>>) -> NodeId {
    let camera = *view.borrow();
    let previous = active(doc);
    let mut settings = previous.as_ref().map(|v| v.2.clone()).unwrap_or_default();
    settings.fov = FRAMES.with(|frames| frames.borrow().get(&view_key(view)).map(|v| v.0))
        .unwrap_or(camera.fov_degrees);
    settings.far = camera.far;
    let settings = settings.validated();
    doc.run_as_one_edit(|doc| save_target(doc, previous.map(|v| v.0), camera, &settings))
}
#[cfg(any(test, feature = "test-support"))]
pub fn save(doc: &mut Document, camera: Camera3d, settings: &Settings) -> NodeId {
    save_target(doc, active(doc).map(|v| v.0), camera, settings)
}
pub fn save_target(
    doc: &mut Document,
    previous: Option<NodeId>,
    camera: Camera3d,
    settings: &Settings,
) -> NodeId {
    let current_layer = doc.current_layer();
    let layer = doc
        .layers()
        .iter()
        .find(|l| l.name == "Render cameras" && l.omit_from_export)
        .map(|l| l.index)
        .unwrap_or_else(|| {
            let n = doc.add_layer("Render cameras");
            doc.set_layer_omitted(n, true);
            n
        });
    let id = doc.run_as_one_edit(|doc| {
        let id = previous.unwrap_or_else(|| doc.create_point_entity(CLASS, camera.position));
        doc.set_entity_property(
            id,
            "origin",
            &format!(
                "{} {} {}",
                camera.position.x(),
                camera.position.y(),
                camera.position.z()
            ),
        );
        doc.set_entity_property(
            id,
            "angles",
            &format!(
                "{} {} 0",
                -camera.pitch.to_degrees(),
                camera.yaw.to_degrees()
            ),
        );
        doc.set_entity_property(
            id,
            KEY,
            &serde_json::to_string(settings).expect("finite render settings"),
        );
        if previous.is_none() {
            doc.select_only(id);
            doc.move_selection_to_layer(layer);
        }
        id
    });
    doc.set_current_layer(current_layer);
    id
}
/// Camera frusta drawn only as editing guides, never included in a render.
pub fn guides(doc: &Document, viewer: &Camera3d) -> Vec<(geometry_math::vec::Vec3d, [f32; 4])> {
    let mut lines = Vec::new();
    let visible: std::collections::HashSet<_> = doc.entity_boxes().iter().map(|e| e.node).collect();
    for (id, c, s) in cameras(doc) {
        if !visible.contains(&id) || geometry_math::vec::length(c.position - viewer.position) < 24. {
            continue;
        }

        let base = c.position + c.direction() * 32.;
        let h = (s.fov.to_radians() / 2.).tan() * 32. * geometry_math::mat_ext::FRUSTUM_SCALE;
        let w = h * s.width as f64 / s.height as f64;
        let corners = [
            base + c.right() * w + c.up() * h,
            base - c.right() * w + c.up() * h,
            base - c.right() * w - c.up() * h,
            base + c.right() * w - c.up() * h,
        ];
        for i in 0..4 {
            for p in [c.position, corners[i], corners[i], corners[(i + 1) % 4]] {
                lines.push((p, [0.95, 0.7, 0.2, 1.]));
            }
        }
    }
    lines
}
pub fn write_png(path: &std::path::Path, width: i32, height: i32, pixels: &[u8]) -> Result<(), String> {
    // The pixels come bottom row first, as OpenGL reads them, and are written opaque.
    let (width, height) = (width as usize, height as usize);
    let mut rgb = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        let row = (height - 1 - y) * width;
        for x in 0..width {
            rgb.extend_from_slice(&pixels[(row + x) * 4..(row + x) * 4 + 3]);
        }
    }
    // Encoded in memory and written through `files`, so the web build saves it to the
    // file the user picked or downloads it.
    let mut png = Vec::new();
    image::write_buffer_with_format(&mut std::io::Cursor::new(&mut png), &rgb, width as u32, height as u32, image::ColorType::Rgb8, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    crate::files::write(path, png).map_err(|e| e.to_string())
}

/// The highest animation frame rate: GIF frame delays are whole hundredths of a second,
/// and most viewers slow anything under two hundredths down.
pub const MAX_VIDEO_FPS: u32 = 50;

/// How long one cycle of the map's longest animation takes, in seconds: what an
/// animation render needs to show it all. `None` when nothing plays on its own.
pub fn animation_length(doc: &Document) -> Option<f64> {
    doc.preview_motions().values().filter_map(crate::motion_preview::Motion::cycle).reduce(f64::max)
}

/// An animated GIF filled in a frame at a time, as an animation render finishes them.
pub struct Video {
    /// How many frames the animation has.
    pub frames: u32,
    /// How many of them are encoded.
    pub frame: u32,
    fps: u32,
    size: (u16, u16),
    encoder: gif::Encoder<Vec<u8>>,
}
impl Video {
    /// An empty GIF for `settings`' output size, length and frame rate, looping forever.
    pub fn new(settings: &Settings) -> Result<Self, String> {
        let size = (settings.width as u16, settings.height as u16);
        let mut encoder = gif::Encoder::new(Vec::new(), size.0, size.1, &[]).map_err(|e| e.to_string())?;
        encoder.set_repeat(gif::Repeat::Infinite).map_err(|e| e.to_string())?;
        let frames = (settings.video_seconds * f64::from(settings.video_fps)).round().max(1.0) as u32;
        Ok(Self { frames, frame: 0, fps: settings.video_fps, size, encoder })
    }

    /// The animation time of the next frame, in seconds.
    pub fn time(&self) -> f64 {
        f64::from(self.frame) / f64::from(self.fps)
    }

    /// Encodes the next frame from `pixels`, RGBA with the bottom row first as OpenGL reads
    /// them, and says whether that was the last.
    pub fn push(&mut self, pixels: &[u8]) -> Result<bool, String> {
        let (width, height) = (usize::from(self.size.0), usize::from(self.size.1));
        if pixels.len() != width * height * 4 {
            return Err(format!("frame is {} bytes, not {width} × {height}", pixels.len()));
        }
        let rgb: Vec<u8> = pixels.chunks_exact(width * 4).rev().flat_map(|row| row.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]])).collect();
        // The palette is chosen per frame; speed 10 is the quantiser's usual trade-off.
        let mut frame = gif::Frame::from_rgb_speed(self.size.0, self.size.1, &rgb, 10);
        // Each frame ends where the frame rate says, so rounding errors do not add up.
        let at = |frame: u32| (f64::from(frame) * 100.0 / f64::from(self.fps)).round() as u16;
        frame.delay = at(self.frame + 1) - at(self.frame);
        self.encoder.write_frame(&frame).map_err(|e| e.to_string())?;
        self.frame += 1;
        Ok(self.frame >= self.frames)
    }

    /// Writes the GIF to `path`, through `files` as [`write_png`] does.
    pub fn finish(self, path: &std::path::Path) -> Result<(), String> {
        let gif = self.encoder.into_inner().map_err(|e| e.to_string())?;
        crate::files::write(path, gif).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An animation render has as many frames as its length and rate say, each shown for
    /// its share of the length, and loops.
    #[test]
    fn a_video_holds_every_frame_at_its_rate() {
        let settings = Settings { width: 32, height: 16, video_seconds: 1.0, video_fps: 30, ..Settings::default() }.validated();
        let mut video = Video::new(&settings).unwrap();
        assert_eq!(video.frames, 30);
        let mut last = false;
        while !last {
            assert_eq!(video.time(), f64::from(video.frame) / 30.0);
            let shade = (video.frame * 8) as u8;
            last = video.push(&[shade, 0, 255 - shade, 255].repeat(32 * 16)).unwrap();
        }
        assert!(video.push(&[0; 4]).is_err(), "a frame of the wrong size is refused");
        let path = std::env::temp_dir().join("heron-render-video-test.gif");
        video.finish(&path).unwrap();
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(std::fs::File::open(&path).unwrap()).unwrap();
        assert_eq!((decoder.width(), decoder.height()), (32, 16));
        assert_eq!(decoder.repeat(), gif::Repeat::Infinite);
        let mut delays = Vec::new();
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            delays.push(frame.delay);
        }
        assert_eq!(delays.len(), 30);
        assert_eq!(delays.iter().map(|d| u32::from(*d)).sum::<u32>(), 100);
        let _ = std::fs::remove_file(path);
    }

    /// The motion lab demo plays a 16 s ping-pong and a 12 s loop on its own: an animation
    /// render is 16 s long, and its frames move those brushes.
    #[test]
    fn an_animation_render_poses_the_map() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demos/motion_lab/motion_lab.map");
        let doc = Document::open(&path, crate::world::WorldBounds::default()).unwrap();
        assert_eq!(animation_length(&doc), Some(16.0));
        let corners = |triangles: Vec<render::Triangle>| triangles.iter().map(|t| format!("{:?}", t.positions)).collect::<Vec<_>>();
        let rest = corners(doc.render_triangles());
        assert_eq!(corners(doc.render_triangles_at(Some(0.0))), rest);
        let moved = corners(doc.render_triangles_at(Some(4.0)));
        assert_eq!(moved.len(), rest.len());
        assert_ne!(moved, rest);
    }

    /// One cycle of each kind of motion, and none for one that waits for a trigger.
    #[test]
    fn motions_report_their_cycle() {
        use crate::motion_preview::Motion;
        use geometry_math::vec::Vec3d;
        let rotate = Motion::Rotate { pivot: Vec3d::new(0., 0., 0.), axis: Vec3d::new(0., 0., 1.), speed: -90. };
        assert_eq!(rotate.cycle(), Some(4.0));
        let shuttle = Motion::Shuttle { direction: Vec3d::new(1., 0., 0.), travel: 128., speed: 64., wait: 0.5 };
        assert_eq!(shuttle.cycle(), Some(5.0));
        let track = std::sync::Arc::new(crate::keyframe_animation::Track::parse("0 0 0 0 0 0 0; 1.5 0 0 64 0 0 0", "pingpong").unwrap());
        let pivot = Vec3d::new(0., 0., 0.);
        assert_eq!(Motion::Keyframes { autoplay: true, pivot, track: track.clone() }.cycle(), Some(3.0));
        assert_eq!(Motion::Keyframes { autoplay: false, pivot, track }.cycle(), None);
    }
}
