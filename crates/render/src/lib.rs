//! The 3D renderer.
//!
//! The viewport supports physically based shading, shadowed sunlight, ambient occlusion and sky
//! illumination.
//!
//! Four passes, in order:
//!
//! 1. **Shadow.** Depth only, from the sun, into a square depth texture covering the
//!    scene.
//! 2. **Normals.** View-space normals and depth for the whole viewport, which is what
//!    ambient occlusion needs and the main pass would otherwise have to be split to give.
//! 3. **Occlusion.** Screen-space ambient occlusion from that, then a blur to take the
//!    sampling noise back out.
//! 4. **Scene.** Cook-Torrance shading, reading the shadow map and the occlusion, tone
//!    mapped at the end.
//!
//! Each of the first three can fail on its own -- a driver that will not give a
//! framebuffer, a texture format it does not have -- and each failure only costs its own
//! effect. A viewport that renders without shadows is worth much more than one that
//! refuses to render.

pub mod camera3d;
pub mod rotation_gizmo;
mod shaders;

use std::collections::HashMap;

use glow::HasContext;

use geometry_math::bbox::Bbox3d;
use geometry_math::mat::{self, Mat4x4d};
use geometry_math::mat_ext;
use geometry_math::vec::{self, Vec2f, Vec3d};

use camera3d::Camera3d;

/// Reserved editor material for translucent skip hatching, outside texture indices.
pub const SKIP_PREVIEW_MATERIAL: usize = usize::MAX;

/// One triangle of the scene's surface, as the frontend hands it to [`Renderer::upload`].
///
/// Lived in the document; moved here so the renderer depends on nothing above `map_model`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    /// The triangle's three corners, wound clockwise as seen from outside.
    pub positions: [Vec3d; 3],
    /// The outward normal of the face this triangle is part of.
    pub normal: Vec3d,
    /// Supplied model normals, when shading varies across the triangle.
    pub corner_normals: Option<[Vec3d; 3]>,
    /// Where each corner falls on the material, in texels.
    ///
    /// Texels rather than a 0-to-1 range, because that is what the face's UV coordinate
    /// system produces and what the `.map` file's offsets and scales are expressed in. The
    /// renderer divides by the material's size, which is the only place the size is known.
    pub uv: [Vec2f; 3],
    /// Which material the face wears, as an index into the document's collection.
    ///
    /// `None` for a face whose material the collection does not have, which is the normal
    /// state of a map opened without its materials.
    pub material: Option<usize>,
    /// A colour per corner, for a surface whose colour is not the same all over.
    ///
    /// `None` for everything but a face wearing a gradient, which is the one thing that
    /// cannot be said with a single colour.
    pub corner_colors: Option<[[f32; 3]; 3]>,
    /// The colour the surface is drawn in, before any material.
    ///
    /// Brushes are a neutral grey so the material does the talking; an entity drawn as a
    /// box is the colour its class was given, which is what tells one from another; an
    /// entity drawn as its model is white, so its own skin is what shows.
    pub color: [f32; 3],
    /// How opaque the surface is, 0 clear to 1 solid. Below 1 for a face painted a
    /// see-through colour; the material's own alpha channel is handled separately.
    pub alpha: f32,
    /// Optional per-face PBR settings; absent uses renderer defaults.
    pub surface_material: Option<map_model::brush_face::SurfaceMaterial>,
    /// Whether the surface is on a locked layer, and so cannot be edited.
    ///
    /// Drawn tinted towards the settings' `locked_face_color`, so what a drag will not
    /// touch looks different from what it will.
    pub locked: bool,
    /// Whether the brush it belongs to is selected.
    pub selected: bool,
    /// Drawn in its own paint, so never tinted as selected however its object is: what
    /// lets a retained renderer keep that per vertex rather than per selection.
    pub painted: bool,
}

/// One vertex as the shaders see it: position, normal, colour with alpha, and where it
/// falls on its material.
const FLOATS_PER_VERTEX: usize = 12;

/// The attribute layout, bound to these indices in every program so that one vertex array
/// works with all of them.
///
/// Without this, each program would assign its own locations and the same buffer would
/// need a vertex array per program.
const ATTRIBUTES: [(&str, i32, i32); 4] = [
    ("position", 3, 0),
    ("normal", 3, 3),
    ("color", 4, 6),
    ("uv", 2, 10),
];

/// Turns a colour a screen shows into the light it stands for.
///
/// The scene pass works in linear light; a colour picked in the preferences is in the
/// space a screen displays, which is roughly that light raised to one over 2.2.
fn to_linear(colour: [f32; 3]) -> [f32; 3] {
    colour.map(|channel| channel.powf(2.2))
}

/// How faintly the axes are drawn, so they read as a reference rather than as geometry.
const AXIS_ALPHA: f32 = 0.7;

/// The `index`th value of the Halton sequence in the given base.
///
/// A low-discrepancy sequence: every prefix of it is spread evenly over the unit
/// interval, which is what makes the accumulated picture look settled at ten samples
/// rather than only at a hundred.
fn halton(index: u32, base: u32) -> f64 {
    let mut result = 0.0;
    let mut fraction = 1.0;
    let mut i = index;
    while i > 0 {
        fraction /= f64::from(base);
        result += fraction * f64::from(i % base);
        i /= base;
    }
    result
}

/// The fragment outputs, in the order the framebuffers attach them.
///
/// `fragment` is the picture; `reflectivity` is what the reflection pass reads. A program
/// that has only the first is told about the second all the same, which does nothing.
const FRAGMENT_OUTPUTS: [&str; 2] = ["fragment", "reflectivity"];

/// An opaque colour with the given alpha put on it.
fn with_alpha(color: [f32; 3], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], alpha]
}

/// The axis colours, from `RenderSettings`' `x_axis_color` and its siblings.
fn axis_colors(settings: &RenderSettings) -> [[f32; 4]; 3] {
    [
        with_alpha(settings.x_axis_color, AXIS_ALPHA),
        with_alpha(settings.y_axis_color, AXIS_ALPHA),
        with_alpha(settings.z_axis_color, AXIS_ALPHA),
    ]
}

/// How large the shadow map is, on a side.
///
/// Rounded to a power of two so a store edited by hand cannot ask for a size the driver
/// will refuse, and kept inside what every OpenGL 3.2 implementation has to support.
fn shadow_size(settings: &RenderSettings) -> i32 {
    let asked = settings.shadow_map_size.clamp(256, 8192);
    (asked.next_power_of_two() as i32).min(8192)
}

/// Where the sun is, as the direction its light travels.
///
/// From above and off to one side, which is what makes a box read as a box: with the light
/// straight overhead its four walls all shade the same. Which side is the user's, since
/// where the shadows fall is half of what a viewport shows.
fn sun_direction(settings: &RenderSettings) -> Vec3d {
    let elevation = f64::from(settings.sun_elevation).to_radians();
    let azimuth = f64::from(settings.sun_azimuth).to_radians();
    // The direction to the sun, then reversed: the shader wants the way the light
    // travels, not the way to look for it.
    let to_sun = Vec3d::new(
        elevation.cos() * azimuth.cos(),
        elevation.cos() * azimuth.sin(),
        elevation.sin(),
    );
    vec::normalize(-to_sun)
}

/// How far a selected surface is pulled towards `Preferences::SelectedFaceColor`.
const SELECTION_TINT: f32 = 0.35;

/// A colour pulled towards the tint, if there is one.
///
/// A tint rather than a colour of its own, so a selected face still reads as what it
/// wears.
fn tinted(color: [f32; 3], tint: Option<[f32; 3]>) -> [f32; 3] {
    let Some(tint) = tint else {
        return color;
    };
    let mut result = [0.0f32; 3];
    for channel in 0..3 {
        result[channel] = color[channel] * (1.0 - SELECTION_TINT) + tint[channel] * SELECTION_TINT;
    }
    result
}

/// How large a tool handle is drawn, in pixels.
///
/// A point is drawn about its centre and a radius reaches out from one, so the point size
/// is twice the radius, plus a pixel so an odd number of pixels straddles the centre.
fn handle_point_size(settings: &RenderSettings) -> f32 {
    // The disc is drawn at the radius a press is picked by -- the preference plus two
    // pixels -- so that what is seen is what can be clicked.
    (settings.handle_radius + 2.0) * 2.0
}

/// How gradually a surface starts to bloom once it passes the threshold.
///
/// A soft knee rather than a hard cut, so a surface drifting past the threshold fades in
/// instead of appearing.
const BLOOM_KNEE: f32 = 0.7;

/// How far the shadow taps spread at `Preferences`'s default softness, in texels of the
/// shadow map.
const SHADOW_TAP_SPREAD: f32 = 1.6;

/// The bias `Preferences::ShadowBias`'s default stands for, slope-scaled in the shader.
const SHADOW_BIAS_SCALE: f32 = 1.0;

/// One stretch of the geometry buffer, all wearing the same material.
struct Batch {
    surface_material: Option<map_model::brush_face::SurfaceMaterial>,
    material: Option<usize>,
    first: i32,
    count: i32,
    /// What the material's alpha channel means, so the draw knows whether to mask, blend,
    /// or neither. Batches are ordered so the blended ones come last.
    alpha: map_model::material::Alpha,
}

/// Screen-space depth of field, in camera-space map units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthOfField {
    /// Distance along the camera forward axis to the sharp plane.
    pub focus_distance: f32,
    /// Full depth interval around the focus plane kept sharp.
    pub focus_range: f32,
    /// Maximum blur radius in pixels at 1080 pixels image height.
    pub radius: f32,
}

/// Everything the renderer draws by that a person can set: colours, the sun, the
/// shadow, occlusion, bloom and reflection knobs. The frontend fills it -- the desktop
/// reads its preference store -- and hands it over with [`Renderer::set_settings`];
/// this crate knows nothing of where the values come from (docs/android.md, steps 2
/// and 3).
#[derive(Debug, Clone, PartialEq)]
pub struct RenderSettings {
    /// Optional depth-aware blur for render cameras.
    pub depth_of_field: Option<DepthOfField>,
    /// Draw the world axes at the origin.
    pub show_axes: bool,
    /// What the view clears to.
    pub background_color: [f32; 3],
    /// Draw the procedural environment behind geometry in shaded renders.
    pub procedural_sky: bool,
    /// The X axis line.
    pub x_axis_color: [f32; 3],
    /// The Y axis line.
    pub y_axis_color: [f32; 3],
    /// The Z axis line.
    pub z_axis_color: [f32; 3],
    /// The tint a selected face is pulled towards.
    pub selected_face_color: [f32; 3],
    /// The tint a locked face is pulled towards.
    pub locked_face_color: [f32; 3],
    /// How large a tool handle is picked and drawn, in pixels of radius.
    pub handle_radius: f32,
    /// Physically based shading, or flat face colour with a little light.
    pub realistic_shading: bool,
    /// A shadowed sun.
    pub shadows: bool,
    /// The shadow map's side, in texels; rounded to a power of two.
    pub shadow_map_size: u32,
    /// How far a surface is pushed off its own shadow.
    pub shadow_bias: f32,
    /// How wide the shadow taps spread.
    pub shadow_softness: f32,
    /// Short screen-space shadows where surfaces meet.
    pub contact_shadows: bool,
    /// How far a contact shadow reaches, in map units.
    pub contact_shadow_distance: f32,
    /// How many steps a contact-shadow ray takes.
    pub contact_shadow_samples: f32,
    /// Screen-space ambient occlusion.
    pub ambient_occlusion: bool,
    /// How far occlusion looks around a point, in map units.
    pub ao_radius: f32,
    /// How dark the occlusion goes.
    pub ao_strength: f32,
    /// Screen-space reflections.
    pub reflections: bool,
    /// How far a reflection ray marches, in map units.
    pub reflection_distance: f32,
    /// How many steps that march takes.
    pub reflection_samples: f32,
    /// How strong the reflections mix in.
    pub reflection_strength: f32,
    /// Glow around what shades brighter than the threshold.
    pub bloom: bool,
    /// The brightness a surface blooms past.
    pub bloom_threshold: f32,
    /// How strong the bloom mixes in.
    pub bloom_intensity: f32,
    /// Refine a still view over several frames.
    pub accumulate: bool,
    /// How many frames the refinement runs.
    pub accumulation_samples: u32,
    /// FXAA over the finished picture.
    pub antialiasing: bool,
    /// The tone-mapping exposure.
    pub exposure: f32,
    /// The sky overhead.
    pub sky_color: [f32; 3],
    /// The sky at the horizon.
    pub horizon_color: [f32; 3],
    /// The sky below the horizon.
    pub ground_color: [f32; 3],
    /// How high the sun stands, in degrees.
    pub sun_elevation: f32,
    /// Which way the sun stands, in degrees.
    pub sun_azimuth: f32,
    /// The sun's light.
    pub sun_color: [f32; 3],
    /// How strong that light is.
    pub sun_intensity: f32,
    /// How strong the ambient (sky) light is.
    pub ambient_intensity: f32,
    /// Roughness for a surface with no material saying otherwise.
    pub default_roughness: f32,
    /// Metalness for the same.
    pub default_metalness: f32,
}

/// A point entity's sprite: where it is, how big to draw it, and which material holds its
/// picture.
#[derive(Debug, Clone, Copy, PartialEq)]

pub struct SpriteInstance {
    /// The entity's origin.
    pub position: Vec3d,
    /// Half the quad's width and height in world units.
    pub half_width: f32,
    /// ..
    pub half_height: f32,
    /// The material index the sprite's RGBA was loaded into.
    pub material: usize,
}

/// The renderer, and everything it owns on the GPU.
pub struct Renderer {
    /// Shared with the frontend, which draws its framebuffer with the same context: in
    /// WebGL one glow context cannot use objects another made.
    gl: std::rc::Rc<glow::Context>,
    /// What the frontend last handed [`Renderer::set_settings`].
    settings: RenderSettings,

    scene: Program,
    depth: Program,
    material_depth: Program,
    normals: Program,
    ssao: Program,
    blur: Program,
    bright: Program,
    gaussian: Program,
    composite: Program,
    ssr: Program,
    accumulate: Program,
    fxaa: Program,

    geometry: Mesh,
    /// The geometry's triangles as lines, on GLES only, which has no polygon line mode
    /// for [`Renderer::draw_wireframe`].
    wire: Mesh,
    /// Which stretch of the geometry wears which material.
    batches: Vec<Batch>,
    /// One texture per material, in the collection's own order; the white one until the
    /// material is first drawn.
    material_textures: Vec<glow::Texture>,
    /// The materials not made into textures yet. Their images are decoded the first time
    /// geometry or a sprite uses them, so a collection of thousands costs only what the
    /// map shows.
    pending_materials: Vec<Option<map_model::material::Material>>,
    normal_textures: Vec<Option<(glow::Texture, bool)>>,
    pbr_textures: Vec<Option<(glow::Texture, glow::Texture)>>,
    model_pbrs: Vec<Option<std::sync::Arc<map_model::model_pbr::ModelPbr>>>,
    /// The size of each material, which is what turns texels into a lookup.
    material_sizes: Vec<(f32, f32)>,
    /// What each material's alpha channel means, in the collection's own order.
    material_alpha: Vec<map_model::material::Alpha>,
    lines: Mesh,
    /// The tools' handles, drawn as points on top of the scene.
    handles: Mesh,
    /// The lines the tools draw: a clip plane, a rotation ring, a bounding box.
    guides: Mesh,
    /// Draws a point entity's `iconsprite` as a camera-facing quad.
    sprite_program: Program,
    /// One quad's worth of vertices, rebuilt each frame from the camera.
    sprite_mesh: Mesh,
    /// Where each sprite sits and how big it is, in world space -- camera-independent, so
    /// uploaded when the map changes rather than per frame.
    sprite_instances: Vec<SpriteInstance>,
    /// An empty vertex array, since a core profile refuses to draw without one even when
    /// the vertices come from `gl_VertexID`.
    fullscreen: glow::VertexArray,
    sky_background: Program,

    shadow: Option<ShadowTarget>,
    screen: Option<ScreenTargets>,
    post: Option<PostTargets>,
    noise: Option<glow::Texture>,
    /// A single white pixel, bound wherever a pass that could not be set up would have
    /// left a texture. Sampling an incomplete texture is undefined, and the value that
    /// makes its absence harmless is 1.0: nothing shadowed, nothing occluded.
    white: Option<glow::Texture>,
    kernel: Vec<f32>,

    /// How many frames of the same still view have been drawn into the accumulation
    /// buffer.
    ///
    /// Zero means the view has just changed and this frame is the first: it goes into the
    /// buffer whole rather than being averaged with what a different view left there.
    sample: u32,

    /// The world bounds the axes were uploaded for, and the colours they were drawn in.
    ///
    /// A colour changed in the preferences has to reach geometry that is otherwise sent
    /// once and never again.
    axes: Option<(Bbox3d, [[f32; 4]; 3])>,

    /// Whether the first frame has been checked for GL errors.
    ///
    /// A GL error does not stop anything; it makes the call that caused it do nothing, so
    /// the symptom is a picture that is quietly wrong. Checking the first frame catches a
    /// pass that is not doing what it looks like it is doing.
    checked_first_frame: bool,

    /// Whether the probe has reported the picture once it stopped accumulating.
    ///
    /// The first frame is the one drawn while the view is still moving, which is not the
    /// picture anyone looks at; the settled one is.
    probed_settled: bool,

    /// The framebuffer that was bound when the frame began.
    ///
    /// The host may render into a framebuffer of its own, not into 0 (egui's paint
    /// callback can), so the passes here put back what they found rather than binding
    /// the default.
    target: Option<glow::Framebuffer>,

    /// What the geometry spans, so the shadow map can be fitted to it.
    scene_bounds: Option<Bbox3d>,
}

impl Renderer {
    /// Sets up everything in the current GL context, drawing by `settings` until
    /// [`Renderer::set_settings`] hands over fresher ones.
    /// The frontend supplies a loaded OpenGL context and keeps its native context
    /// current on this thread for construction, drawing and destruction. It also
    /// owns the window/surface lifecycle and the libraries behind the function pointers.
    /// On GLES 3.0 and WebGL 2 the shaders are rewritten as GLSL ES and the calls GLES
    /// lacks are left out ([`for_context`]): the wireframe is drawn as lines rather than
    /// in polygon line mode, and the shadow map has no border.
    /// On context loss, discard the renderer without GL calls and recreate it, then
    /// re-upload CPU-side scene data into the new context.
    pub fn new(gl: glow::Context, settings: RenderSettings) -> Result<Self, String> {
        // SAFETY: The caller keeps `gl`'s native context current on this thread, as documented
        // above. Every call acts on that context and on objects it has just made.
        unsafe {
            // Which GL the driver actually gave, since what the shaders may use depends on
            // it and a mismatch shows up as a blank view rather than as an error.
            eprintln!(
                "3D view: OpenGL {} ({})",
                gl.get_parameter_string(glow::VERSION),
                gl.get_parameter_string(glow::RENDERER)
            );

            let scene = Program::new(
                &gl,
                shaders::SCENE_VERTEX,
                &shaders::normal_mapping(shaders::SCENE_FRAGMENT),
            )?;
            let depth = Program::new(&gl, shaders::DEPTH_VERTEX, shaders::DEPTH_FRAGMENT)?;
            let material_depth = Program::new(
                &gl,
                shaders::MATERIAL_DEPTH_VERTEX,
                shaders::MATERIAL_DEPTH_FRAGMENT,
            )?;
            let normals = Program::new(
                &gl,
                shaders::NORMAL_VERTEX,
                &shaders::normal_mapping(shaders::NORMAL_FRAGMENT),
            )?;
            let ssao = Program::new(&gl, shaders::FULLSCREEN_VERTEX, shaders::SSAO_FRAGMENT)?;
            let blur = Program::new(&gl, shaders::FULLSCREEN_VERTEX, shaders::BLUR_FRAGMENT)?;
            let bright = Program::new(&gl, shaders::FULLSCREEN_VERTEX, shaders::BRIGHT_FRAGMENT)?;
            let gaussian =
                Program::new(&gl, shaders::FULLSCREEN_VERTEX, shaders::GAUSSIAN_FRAGMENT)?;
            let composite =
                Program::new(&gl, shaders::FULLSCREEN_VERTEX, shaders::COMPOSITE_FRAGMENT)?;
            let accumulate = Program::new(
                &gl,
                shaders::FULLSCREEN_VERTEX,
                shaders::ACCUMULATE_FRAGMENT,
            )?;
            let ssr = Program::new(&gl, shaders::FULLSCREEN_VERTEX, shaders::SSR_FRAGMENT)?;
            let fxaa = Program::new(&gl, shaders::FULLSCREEN_VERTEX, shaders::FXAA_FRAGMENT)?;
            let sprite_program =
                Program::new(&gl, shaders::SPRITE_VERTEX, shaders::SPRITE_FRAGMENT)?;

            let sky_background =
                Program::new(&gl, shaders::FULLSCREEN_VERTEX, shaders::SKY_FRAGMENT)?;
            let geometry = Mesh::new(&gl)?;
            let wire = Mesh::new(&gl)?;
            let lines = Mesh::new(&gl)?;
            let handles = Mesh::new(&gl)?;
            let guides = Mesh::new(&gl)?;
            let sprite_mesh = Mesh::new(&gl)?;
            let fullscreen = gl
                .create_vertex_array()
                .map_err(|e| format!("could not create a vertex array: {e}"))?;

            // Each of these is an improvement on the picture, not a requirement for it, so
            // a failure to set one up is reported and stepped over.
            let shadow = ShadowTarget::new(&gl, shadow_size(&settings))
                .inspect_err(|error| eprintln!("3D view: no shadows: {error}"))
                .ok();
            let noise = make_noise_texture(&gl)
                .inspect_err(|error| eprintln!("3D view: no occlusion noise: {error}"))
                .ok();
            let white = make_white_texture(&gl)
                .inspect_err(|error| eprintln!("3D view: no fallback texture: {error}"))
                .ok();

            Ok(Self {
                gl: std::rc::Rc::new(gl),
                settings,
                scene,
                depth,
                material_depth,
                normals,
                ssao,
                blur,
                bright,
                gaussian,
                composite,
                ssr,
                accumulate,
                fxaa,
                geometry,
                wire,
                batches: Vec::new(),
                material_textures: Vec::new(),
                pending_materials: Vec::new(),
                normal_textures: Vec::new(),
                pbr_textures: Vec::new(),
                model_pbrs: Vec::new(),
                material_sizes: Vec::new(),
                material_alpha: Vec::new(),
                lines,
                handles,
                guides,
                sprite_program,
                sprite_mesh,
                sprite_instances: Vec::new(),
                fullscreen,
                sky_background,
                shadow,
                screen: None,
                post: None,
                noise,
                white,
                kernel: make_kernel(),
                sample: 0,
                axes: None,
                checked_first_frame: false,
                probed_settled: false,
                target: None,
                scene_bounds: None,
            })
        }
    }

    /// Replaces the geometry the view draws.
    pub fn upload(&mut self, triangles: &[Triangle]) {
        if self.pending_materials.iter().any(Option::is_some) {
            let mut used = vec![false; self.pending_materials.len()];
            for index in triangles.iter().filter_map(|t| t.material) {
                if let Some(slot) = used.get_mut(index) {
                    *slot = true;
                }
            }
            self.make_pending_textures(
                used.iter()
                    .enumerate()
                    .filter(|(_, u)| **u)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>(),
            );
        }
        let mut data: Vec<f32> = Vec::with_capacity(triangles.len() * 3 * FLOATS_PER_VERTEX);
        let mut bounds: Option<Bbox3d> = None;

        // What a triangle's material asks for when it is drawn: nothing, a cut-out mask,
        // or blending. A triangle whose material the collection does not have, or that
        // has not loaded yet, is treated as solid.
        let alpha_of = |triangle: &Triangle| {
            triangle
                .material
                .and_then(|index| self.material_alpha.get(index).copied())
                .unwrap_or(map_model::material::Alpha::Opaque)
        };
        // A surface is drawn blended if its material's alpha channel says so, or if it is
        // a face painted a see-through colour.
        let blended = |triangle: &Triangle| {
            alpha_of(triangle) == map_model::material::Alpha::Blended || triangle.alpha < 0.999
        };
        // What state a batch of this triangle needs: a cut-out mask still wins over
        // blending, since a masked material with a see-through paint on it makes no sense.
        let batch_alpha = |triangle: &Triangle| match alpha_of(triangle) {
            map_model::material::Alpha::Masked => map_model::material::Alpha::Masked,
            _ if blended(triangle) => map_model::material::Alpha::Blended,
            other => other,
        };

        // Grouped by material, so the whole scene is drawn in one pass per material
        // rather than one per surface. A level uses a few dozen materials and has
        // thousands of faces, so which of those two it is matters. Blended surfaces sort
        // to the end, so everything solid is already in the depth buffer to be seen
        // through when they are drawn.
        let mut order: Vec<usize> = (0..triangles.len()).collect();
        order.sort_by_key(|index| {
            (
                blended(&triangles[*index]),
                triangles[*index].material,
                triangles[*index].surface_material,
            )
        });

        self.batches.clear();
        let mut batch: Option<Batch> = None;

        for index in order {
            let triangle = &triangles[index];
            // The surface's own colour, pulled towards the selection colour: a tint
            // rather than a colour of its own, so a selected face still reads as what it
            // wears.
            // A locked surface is tinted too, so what a drag will not touch looks
            // different from what it will; being selected wins, since a selected surface
            // on a locked layer is still the one being talked about.
            let tint = if triangle.selected {
                Some(self.settings.selected_face_color)
            } else if triangle.locked {
                Some(self.settings.locked_face_color)
            } else {
                None
            };
            let color = tinted(triangle.color, tint);

            let this_alpha = batch_alpha(triangle);
            match &mut batch {
                Some(open)
                    if open.material == triangle.material
                        && open.alpha == this_alpha
                        && open.surface_material == triangle.surface_material =>
                {
                    open.count += 3
                }
                _ => {
                    if let Some(finished) = batch.take() {
                        self.batches.push(finished);
                    }
                    batch = Some(Batch {
                        surface_material: triangle.surface_material,
                        material: triangle.material,
                        first: (data.len() / FLOATS_PER_VERTEX) as i32,
                        count: 3,
                        alpha: this_alpha,
                    });
                }
            }

            for (index, (corner, uv)) in triangle.positions.iter().zip(triangle.uv).enumerate() {
                bounds = Some(match bounds {
                    Some(bounds) => bounds.merged_with_point(*corner),
                    None => Bbox3d::new(*corner, *corner),
                });
                // A surface with a colour per corner -- a face wearing a gradient -- is
                // the one thing the single colour above cannot say. The tint for being
                // selected or locked still applies, since the point of it is that the
                // selection is readable whatever the surface wears.
                let corner_color = match triangle.corner_colors {
                    Some(corners) => tinted(corners[index], tint),
                    None => color,
                };
                data.extend([corner.x() as f32, corner.y() as f32, corner.z() as f32]);
                let normal = triangle
                    .corner_normals
                    .map_or(triangle.normal, |n| n[index]);
                data.extend([normal.x() as f32, normal.y() as f32, normal.z() as f32]);
                data.extend([
                    corner_color[0],
                    corner_color[1],
                    corner_color[2],
                    triangle.alpha,
                ]);
                data.extend([uv.x(), uv.y()]);
            }
        }
        if let Some(finished) = batch {
            self.batches.push(finished);
        }

        self.scene_bounds = bounds;
        self.geometry.upload(&self.gl, &data, glow::DYNAMIC_DRAW);
        if embedded(&self.gl) {
            // Each triangle's three sides as lines: the corners `ab`, `bc` and `ca`.
            let mut lines: Vec<f32> = Vec::with_capacity(data.len() * 2);
            for triangle in data.as_chunks::<{ 3 * FLOATS_PER_VERTEX }>().0 {
                let corner = |index: usize| {
                    &triangle[index * FLOATS_PER_VERTEX..(index + 1) * FLOATS_PER_VERTEX]
                };
                for (from, to) in [(0, 1), (1, 2), (2, 0)] {
                    lines.extend_from_slice(corner(from));
                    lines.extend_from_slice(corner(to));
                }
            }
            self.wire.upload(&self.gl, &lines, glow::DYNAMIC_DRAW);
        }
    }

    /// Replaces the materials the scene can be drawn with.
    ///
    /// One texture each, indexed the way the document's collection is, so a triangle's
    /// material index means the same thing on both sides.
    pub fn upload_materials(&mut self, materials: &[map_model::material::Material]) {
        // SAFETY: The frontend keeps this renderer's context current (see `Renderer::new`). The
        // textures were made by it, the shared white one is skipped, and draining the lists
        // means none is deleted twice or used again.
        unsafe {
            for texture in self.material_textures.drain(..) {
                if Some(texture) != self.white {
                    self.gl.delete_texture(texture);
                }
            }
            for (texture, _) in self.normal_textures.drain(..).flatten() {
                self.gl.delete_texture(texture);
            }
            for (orm, emissive) in self.pbr_textures.drain(..).flatten() {
                self.gl.delete_texture(orm);
                self.gl.delete_texture(emissive);
            }
        }
        self.material_sizes = materials
            .iter()
            .map(|m| (m.width as f32, m.height as f32))
            .collect();
        self.material_alpha = materials.iter().map(|m| m.alpha()).collect();
        self.normal_textures = vec![None; materials.len()];
        self.pbr_textures = vec![None; materials.len()];
        self.model_pbrs = materials.iter().map(|m| m.model_pbr.clone()).collect();
        // Every slot holds something from the start, or a material after a missing one
        // would be drawn with the wrong texture.
        self.material_textures = self
            .white
            .map(|white| vec![white; materials.len()])
            .unwrap_or_default();
        self.pending_materials = materials.iter().cloned().map(Some).collect();
    }

    /// Makes the textures of the materials `used` names that are not textures yet,
    /// decoding their images (on this thread: the GL context is here).
    fn make_pending_textures(&mut self, used: impl IntoIterator<Item = usize>) {
        for index in used {
            let Some(material) = self.pending_materials.get_mut(index).and_then(Option::take)
            else {
                continue;
            };
            // SAFETY: The context is current (see `Renderer::new`). `make_rgba_texture` checks each
            // image's size against its pixel count before GL reads the slice.
            unsafe {
                let normal = material.normal_map.as_ref().and_then(|map| {
                    let pixels = map.pixels()?;
                    make_rgba_texture(
                        &self.gl,
                        map.width,
                        map.height,
                        &pixels[..(map.width * map.height * 4) as usize],
                    )
                    .ok()
                    .map(|texture| (texture, map.flip_y))
                });
                self.normal_textures[index] = normal;
                if let Some(pbr) = &material.model_pbr {
                    let orm = pbr.preview_orm();
                    let white = map_model::entity_model::Skin {
                        width: 1,
                        height: 1,
                        pixels: vec![255; 4],
                    };
                    let emission = pbr.emissive.as_deref().unwrap_or(&white);
                    if let Ok(orm) = make_rgba_texture(&self.gl, orm.width, orm.height, &orm.pixels)
                    {
                        match make_rgba_texture(
                            &self.gl,
                            emission.width,
                            emission.height,
                            &emission.pixels,
                        ) {
                            Ok(emission) => self.pbr_textures[index] = Some((orm, emission)),
                            Err(_) => self.gl.delete_texture(orm),
                        }
                    }
                }
                match make_material_texture(&self.gl, &material) {
                    Ok(texture) => self.material_textures[index] = texture,
                    Err(error) => eprintln!("3D view: {}: {error}", material.name),
                }
            }
            // The decode settled what the alpha channel means.
            self.material_alpha[index] = material.alpha();
        }
    }

    /// Uploads the coordinate axes, which never change.
    ///
    /// `RenderService::renderCoordinateSystem` draws one line per axis, spanning the world
    /// bounds and passing through their centre, in the axis colours.
    pub fn upload_axes(&mut self, world_bounds: &Bbox3d) {
        // Kept so a colour changed in the preferences can be noticed and the axes sent
        // again; they are uploaded once otherwise, since they never move.
        self.axes = Some((*world_bounds, axis_colors(&self.settings)));
        let center = world_bounds.center();
        let mut data: Vec<f32> = Vec::with_capacity(6 * FLOATS_PER_VERTEX);

        for (axis, color) in axis_colors(&self.settings).iter().enumerate() {
            // `coordinateSystemVerticesX` and its siblings: the axis's own extent, with the
            // other two components taken from the centre.
            let mut start = center;
            let mut end = center;
            start[axis] = world_bounds.min[axis];
            end[axis] = world_bounds.max[axis];

            for endpoint in [start, end] {
                data.extend([
                    endpoint.x() as f32,
                    endpoint.y() as f32,
                    endpoint.z() as f32,
                ]);
                data.extend([0.0f32, 0.0, 0.0]);
                data.extend(color);
                // A line samples no material, but the vertex still has to be
                // `FLOATS_PER_VERTEX` long or every one after it is read from the wrong
                // place -- which is why the axes were not drawn at all.
                data.extend([0.0f32, 0.0]);
            }
        }

        self.lines.upload(&self.gl, &data, glow::STATIC_DRAW);
    }

    /// Replaces the tool overlay: the handles to draw as points, and the lines to join.
    ///
    /// The overlay is whatever the active tool wants shown -- vertex handles, a bounding
    /// box, a rotation ring, a clip plane. It is drawn twice, once against the depth
    /// buffer and once through it, so a handle behind a wall is still visible without
    /// being mistaken for one in front of it.
    pub fn upload_overlay(&mut self, handles: &[(Vec3d, [f32; 4])], guides: &[(Vec3d, [f32; 4])]) {
        let to_vertices = |points: &[(Vec3d, [f32; 4])]| {
            let mut data = Vec::with_capacity(points.len() * FLOATS_PER_VERTEX);
            for (position, color) in points {
                data.extend([
                    position.x() as f32,
                    position.y() as f32,
                    position.z() as f32,
                ]);
                data.extend([0.0f32, 0.0, 0.0]);
                data.extend(color);
                // The `uv` attribute the material pass added: overlay geometry never
                // samples a material, but the vertex still has to be `FLOATS_PER_VERTEX`
                // wide or every vertex after the first reads into the next one's data.
                data.extend([0.0f32, 0.0]);
            }
            data
        };

        let handle_data = to_vertices(handles);
        let guide_data = to_vertices(guides);
        self.handles
            .upload(&self.gl, &handle_data, glow::DYNAMIC_DRAW);
        self.guides
            .upload(&self.gl, &guide_data, glow::DYNAMIC_DRAW);
    }

    /// Replaces the point-entity sprites the view draws. Camera-independent, so this is
    /// only called when the map changes; the billboard is built from the camera each
    /// frame in `Self::render_sprites`.
    pub fn upload_sprites(&mut self, sprites: Vec<SpriteInstance>) {
        self.make_pending_textures(
            sprites
                .iter()
                .map(|sprite| sprite.material)
                .collect::<Vec<_>>(),
        );
        self.sprite_instances = sprites;
    }

    /// Draws each sprite as a quad facing the camera, into the finished picture and
    /// depth-tested against the scene, so a sprite behind a wall is hidden. Grouped by
    /// material, one small buffer upload per group.
    fn render_sprites(
        &mut self,
        camera: &Camera3d,
        into: Option<glow::Framebuffer>,
        width: i32,
        height: i32,
    ) {
        if self.sprite_instances.is_empty() {
            return;
        }
        let right = camera.right();
        let up = camera.up();
        let view_projection = self.projection(camera, width, height) * camera.view_matrix();

        let materials: Vec<usize> = {
            let mut seen: Vec<usize> = self
                .sprite_instances
                .iter()
                .map(|instance| instance.material)
                .collect();
            seen.sort_unstable();
            seen.dedup();
            seen
        };

        // SAFETY: The context is current (see `Renderer::new`), and `into` and the sprite program
        // belong to it.
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, into);
            self.gl.viewport(0, 0, width, height);
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LESS);
            // A sprite does not occlude anything, so it tests depth without writing it.
            self.gl.depth_mask(false);
            self.gl.disable(glow::CULL_FACE);
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.gl.use_program(Some(self.sprite_program.program));
        }
        self.sprite_program
            .set_matrix(&self.gl, "view_projection", &view_projection);
        self.sprite_program.set_i32(&self.gl, "sprite", 2);

        for material in materials {
            let Some(&texture) = self.material_textures.get(material) else {
                continue;
            };
            bind_texture(&self.gl, 2, texture);

            let mut data: Vec<f32> = Vec::new();
            for instance in self
                .sprite_instances
                .iter()
                .filter(|instance| instance.material == material)
            {
                let hw = f64::from(instance.half_width);
                let hh = f64::from(instance.half_height);
                let bottom_left = instance.position - right * hw - up * hh;
                let bottom_right = instance.position + right * hw - up * hh;
                let top_right = instance.position + right * hw + up * hh;
                let top_left = instance.position - right * hw + up * hh;
                let mut vertex = |point: Vec3d, u: f32, v: f32| {
                    data.extend([
                        point.x() as f32,
                        point.y() as f32,
                        point.z() as f32,
                        0.0,
                        0.0,
                        0.0, // normal, unused
                        0.0,
                        0.0,
                        0.0,
                        0.0, // colour, unused
                        u,
                        v,
                    ]);
                };
                vertex(bottom_left, 0.0, 1.0);
                vertex(bottom_right, 1.0, 1.0);
                vertex(top_right, 1.0, 0.0);
                vertex(bottom_left, 0.0, 1.0);
                vertex(top_right, 1.0, 0.0);
                vertex(top_left, 0.0, 0.0);
            }

            self.sprite_mesh.upload(&self.gl, &data, glow::DYNAMIC_DRAW);
            // SAFETY: The context is current, and `Mesh::draw` reads only the vertices the upload
            // above put in the buffer.
            unsafe {
                self.sprite_mesh.draw(&self.gl, glow::TRIANGLES);
            }
        }

        // SAFETY: The context is current (see `Renderer::new`); this only resets state.
        unsafe {
            self.gl.depth_mask(true);
            self.gl.disable(glow::BLEND);
        }
    }

    /// The GL context it draws with, for a frontend's own framebuffers and blits.
    pub fn gl(&self) -> &std::rc::Rc<glow::Context> {
        &self.gl
    }

    /// Forgets what has been accumulated, because the view or the map has changed.
    ///
    /// Every frame after this one is averaged with the ones before it, so a frame of a
    /// different scene left in the buffer would smear across the new one.
    /// Hands the renderer what to draw by. The frontend calls this whenever the
    /// settings may have changed -- once per frame is cheap, the struct is plain data --
    /// and a change that affects the accumulated picture starts the accumulation over.
    pub fn set_settings(&mut self, settings: RenderSettings) {
        if self.settings != settings {
            self.settings = settings;
            self.restart_accumulating();
        }
    }

    /// Starts the still-view refinement over, e.g. after the camera or scene moved.
    pub fn restart_accumulating(&mut self) {
        self.sample = 0;
    }

    /// Whether another sample would still improve the picture.
    pub fn wants_another_sample(&self) -> bool {
        self.settings.accumulate
            && self.post.is_some()
            && self.sample < self.settings.accumulation_samples.max(1)
    }

    /// Draw triangle topology directly into the caller's framebuffer, without materials
    /// or post-processing. Polygon mode is restored before tools and subsequent frames.
    pub fn draw_wireframe(&mut self, camera: &Camera3d, width: i32, height: i32) {
        let width = width.max(1);
        let height = height.max(1);
        self.ensure_axis_colors();
        // SAFETY: The context is current (see `Renderer::new`). Each mesh draws only the vertices
        // its last upload holds.
        unsafe {
            let target = self
                .gl
                .get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING);
            self.gl.viewport(0, 0, width, height);
            let [r, g, b] = self.settings.background_color;
            self.gl.clear_color(r, g, b, 1.0);
            self.gl.depth_mask(true);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.disable(glow::CULL_FACE);
            self.gl.disable(glow::BLEND);
            self.gl.use_program(Some(self.scene.program));
            self.scene.set_matrix(
                &self.gl,
                "view_projection",
                &(camera.projection_matrix(width, height) * camera.view_matrix()),
            );
            self.scene.set_f32(&self.gl, "lit", -1.0);
            let contrast = if 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.5 {
                0.12
            } else {
                0.85
            };
            self.scene
                .set_vec3(&self.gl, "wire_color", contrast, contrast, contrast);
            self.scene.set_f32(&self.gl, "tonemap_here", 0.0);
            self.scene.set_f32(&self.gl, "alpha_scale", 1.0);
            self.scene.set_f32(&self.gl, "round_points", 0.0);
            // GLES has no line mode for polygons: there the triangles' sides are drawn
            // as lines instead.
            if embedded(&self.gl) {
                self.wire.draw(&self.gl, glow::LINES);
            } else {
                self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
                self.geometry.draw(&self.gl, glow::TRIANGLES);
                self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            }
            self.render_overlay(target, width, height);
        }
    }

    /// Draws one shaded frame.
    pub fn draw(&mut self, camera: &Camera3d, width: i32, height: i32) {
        let width = width.max(1);
        let height = height.max(1);

        let sun = sun_direction(&self.settings);
        let light_matrix = self.light_matrix(sun);

        // Whatever the host is drawing into, which is where the last pass has to end up.
        // SAFETY: The context is current (see `Renderer::new`); this only reads a binding.
        self.target = unsafe {
            self.gl
                .get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING)
        };

        self.ensure_post_targets(width, height);
        self.ensure_shadow_target();
        self.ensure_axis_colors();

        self.render_shadow_map(&light_matrix);
        self.render_occlusion(camera, width, height);
        self.render_scene(camera, width, height, sun, &light_matrix);

        match self.post.as_ref().map(|post| post.ldr_framebuffer) {
            Some(composite) => {
                self.render_reflections(camera, width, height);
                self.render_bloom_and_composite(camera);
                self.render_sprites(camera, Some(composite), width, height);
                self.render_overlay(Some(composite), width, height);
                let finished = self.render_accumulation(width, height);
                self.render_anti_aliasing(finished, width, height);
            }
            None => {
                self.render_sprites(camera, self.target, width, height);
                self.render_overlay(self.target, width, height);
            }
        }

        // This view has now been sampled once more. A view that has changed since had its
        // count put back to zero before the frame began.
        if self.wants_another_sample() {
            self.sample += 1;
        }

        // Once the accumulation has finished, report what settled: the first frame is the
        // one drawn while the view is still moving, and the settled one is what is
        // actually looked at.
        if std::env::var_os("TBR_RENDER_PROBE").is_some()
            && !self.probed_settled
            && self.checked_first_frame
            && self.settings.accumulate
            && self.sample + 1 >= self.settings.accumulation_samples.max(1)
        {
            self.probed_settled = true;
            eprintln!("3D view: settled after {} samples", self.sample + 1);
            self.probe_frame(width, height);
        }

        if !self.checked_first_frame {
            self.checked_first_frame = true;
            // SAFETY: The context is current (see `Renderer::new`); this only reads the error flag.
            let error = unsafe { self.gl.get_error() };
            if error != glow::NO_ERROR {
                eprintln!("3D view: the first frame raised GL error {error:#x}");
            }
            if std::env::var_os("TBR_RENDER_PROBE").is_some() {
                // What the passes were actually asked to do. A setting that is not
                // reaching the shader looks exactly like one that is doing nothing, and
                // the difference matters when reading the numbers below.
                eprintln!(
                    "3D view: shadows {} ({}px, contact {}), occlusion {}, bloom {}, \
                     antialiasing {}, exposure {}",
                    self.settings.shadows,
                    shadow_size(&self.settings),
                    if self.settings.contact_shadows {
                        format!("{} units", self.settings.contact_shadow_distance)
                    } else {
                        "off".to_string()
                    },
                    self.settings.ambient_occlusion,
                    self.settings.bloom,
                    self.settings.antialiasing,
                    self.settings.exposure,
                );
                self.probe_frame(width, height);
            }
        }
    }

    /// Reports what the first frame actually contains.
    ///
    /// A renderer can raise no GL error and still draw nothing -- a framebuffer bound to
    /// the wrong place, a matrix that puts everything behind the camera. Reading the
    /// picture back says whether there is one. Behind an environment variable because it
    /// stalls the pipeline, and it exists because whoever is changing the passes usually
    /// cannot see the window they are changing.
    fn probe_frame(&self, width: i32, height: i32) {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        // SAFETY: The context is current (see `Renderer::new`). `read_pixels` writes
        // `width * height` RGBA bytes, which is the length of `pixels`: RGBA rows need no
        // padding at any pack alignment GL allows up to 4, and nothing here binds a pack buffer.
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, self.target);
            self.gl.read_pixels(
                0,
                0,
                width,
                height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }

        let mut lowest = [255u8; 3];
        let mut highest = [0u8; 3];
        let mut totals = [0u64; 3];
        let mut counts: std::collections::HashMap<[u8; 3], u64> = std::collections::HashMap::new();

        for pixel in pixels.as_chunks::<4>().0 {
            for channel in 0..3 {
                lowest[channel] = lowest[channel].min(pixel[channel]);
                highest[channel] = highest[channel].max(pixel[channel]);
                totals[channel] += u64::from(pixel[channel]);
            }
            *counts.entry([pixel[0], pixel[1], pixel[2]]).or_default() += 1;
        }

        let count = (width * height) as u64;
        let mean = totals.map(|total| total / count.max(1));
        eprintln!(
            "3D view: first frame {width}x{height}, mean {mean:?}, range {lowest:?}..{highest:?}, {} distinct colours",
            counts.len()
        );

        // The commonest few, which is what says whether a picture is one flat colour with
        // a fringe or a scene with shading in it.
        let mut common: Vec<([u8; 3], u64)> = counts.into_iter().collect();
        common.sort_by_key(|(_, seen)| std::cmp::Reverse(*seen));
        let listed: Vec<String> = common
            .iter()
            .take(8)
            .map(|(colour, seen)| {
                // A count as well as a share: an effect that touches a few hundred
                // pixels rounds to zero percent and looks like an effect that does
                // nothing, which is the difference worth being able to see.
                format!("{colour:?} {seen} ({}%)", seen * 100 / count.max(1))
            })
            .collect();
        eprintln!("3D view: commonest {}", listed.join(", "));
    }

    /// Render the uploaded scene into a separate RGBA target, independent of window size.
    /// Returns bottom-to-top rows, as OpenGL reads them. The caller must own a current GL context.
    pub fn capture_rgba(
        &mut self,
        camera: &Camera3d,
        width: i32,
        height: i32,
        samples: u32,
    ) -> Result<Vec<u8>, String> {
        let result = self.capture_batch(camera, width, height, samples, true, None);
        self.restart_accumulating();
        result
    }

    /// Capture one accumulating sample and display it in the caller's framebuffer.
    /// Keep scene/settings/resolution unchanged between steps; set first after cancellation.
    pub fn capture_step(
        &mut self,
        camera: &Camera3d,
        width: i32,
        height: i32,
        first: bool,
        preview: (i32, i32),
    ) -> Result<Vec<u8>, String> {
        self.capture_batch(camera, width, height, 1, first, Some(preview))
    }

    fn capture_batch(
        &mut self,
        camera: &Camera3d,
        width: i32,
        height: i32,
        samples: u32,
        first: bool,
        preview: Option<(i32, i32)>,
    ) -> Result<Vec<u8>, String> {
        if !(16..=4096).contains(&width) || !(16..=4096).contains(&height) {
            return Err("Render dimensions must be between 16 and 4096 pixels".into());
        }
        // SAFETY: The caller keeps the context current, as `capture_rgba` says. The size is
        // checked above, so `pixels` is exactly the RGBA bytes `read_pixels` writes for the
        // target. Every object made here is deleted before returning, on the error paths too.
        unsafe {
            let gl = &self.gl;
            let max = gl
                .get_parameter_i32(glow::MAX_TEXTURE_SIZE)
                .min(gl.get_parameter_i32(glow::MAX_RENDERBUFFER_SIZE));
            if width > max || height > max {
                return Err(format!("GPU render limit is {max} pixels"));
            }
            let old_draw = gl.get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING);
            let old_read = gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);
            let target = gl.create_framebuffer()?;
            let color = match gl.create_renderbuffer() {
                Ok(v) => v,
                Err(e) => {
                    gl.delete_framebuffer(target);
                    return Err(e);
                }
            };
            let depth = match gl.create_renderbuffer() {
                Ok(v) => v,
                Err(e) => {
                    gl.delete_renderbuffer(color);
                    gl.delete_framebuffer(target);
                    return Err(e);
                }
            };
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target));
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
            let result = if complete {
                if first {
                    self.restart_accumulating();
                }
                for _ in 0..samples.clamp(1, 64) {
                    self.draw(camera, width, height);
                }
                let mut pixels = vec![0; width as usize * height as usize * 4];
                self.gl
                    .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(target));
                self.gl.read_pixels(
                    0,
                    0,
                    width,
                    height,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(Some(&mut pixels)),
                );
                if let Some((pw, ph)) = preview {
                    self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, old_draw);
                    self.gl.blit_framebuffer(
                        0,
                        0,
                        width,
                        height,
                        0,
                        0,
                        pw.max(1),
                        ph.max(1),
                        glow::COLOR_BUFFER_BIT,
                        glow::LINEAR,
                    );
                }
                Ok(pixels)
            } else {
                Err("The GPU could not allocate the render target; try a smaller resolution".into())
            };
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, old_draw);
            self.gl.bind_framebuffer(glow::READ_FRAMEBUFFER, old_read);
            self.gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            self.gl.delete_renderbuffer(color);
            self.gl.delete_renderbuffer(depth);
            self.gl.delete_framebuffer(target);
            self.target = old_draw;
            result
        }
    }

    /// Releases everything on the GPU.
    pub fn destroy(self) {
        // SAFETY: The context is current (see `Renderer::new`). `self` is consumed, so nothing
        // is used after it is deleted, and the white texture the material slots share is
        // deleted once, at the end.
        unsafe {
            self.scene.destroy(&self.gl);
            self.sky_background.destroy(&self.gl);
            self.depth.destroy(&self.gl);
            self.material_depth.destroy(&self.gl);
            self.normals.destroy(&self.gl);
            self.ssao.destroy(&self.gl);
            self.blur.destroy(&self.gl);
            self.bright.destroy(&self.gl);
            self.gaussian.destroy(&self.gl);
            self.composite.destroy(&self.gl);
            self.ssr.destroy(&self.gl);
            self.accumulate.destroy(&self.gl);
            self.fxaa.destroy(&self.gl);
            self.geometry.destroy(&self.gl);
            self.wire.destroy(&self.gl);
            for texture in &self.material_textures {
                // The white fallback is deleted with the rest below, not here.
                if Some(*texture) != self.white {
                    self.gl.delete_texture(*texture);
                }
            }
            for (texture, _) in self.normal_textures.iter().flatten() {
                self.gl.delete_texture(*texture);
            }
            for (orm, emissive) in self.pbr_textures.iter().flatten() {
                self.gl.delete_texture(*orm);
                self.gl.delete_texture(*emissive);
            }
            self.lines.destroy(&self.gl);
            self.handles.destroy(&self.gl);
            self.guides.destroy(&self.gl);
            self.sprite_mesh.destroy(&self.gl);
            self.gl.delete_vertex_array(self.fullscreen);
            if let Some(shadow) = self.shadow {
                shadow.destroy(&self.gl);
            }
            if let Some(screen) = self.screen {
                screen.destroy(&self.gl);
            }
            if let Some(post) = self.post {
                post.destroy(&self.gl);
            }
            for texture in [self.noise, self.white].into_iter().flatten() {
                self.gl.delete_texture(texture);
            }
        }
    }

    /* ========== the passes ========== */

    /// The matrix taking the world into the sun's clip space.
    ///
    /// Fitted to the geometry rather than to the world: the world is tens of thousands of
    /// units across and a shadow map stretched over that has nothing left for a doorway.
    fn light_matrix(&self, sun: Vec3d) -> Mat4x4d {
        let bounds = self
            .scene_bounds
            .unwrap_or_else(|| Bbox3d::from_scalars(-512.0, 512.0));
        let center = bounds.center();
        let radius = (vec::length(bounds.size()) / 2.0).max(64.0);

        // Stand the sun far enough back that the whole scene is in front of it.
        let eye = center - sun * radius * 2.0;
        // Any up vector that is not the sun's own direction will do.
        let up = if sun.z().abs() > 0.95 {
            Vec3d::new(0.0, 1.0, 0.0)
        } else {
            Vec3d::new(0.0, 0.0, 1.0)
        };

        let view = mat_ext::view_matrix(sun, up) * mat_ext::translation_matrix(-eye);
        let projection = mat_ext::ortho_matrix(0.1, radius * 4.0, -radius, radius, radius, -radius);
        projection * view
    }

    /// Sends the axes again if their colours have been changed in the preferences.
    fn ensure_axis_colors(&mut self) {
        if let Some((bounds, colors)) = self.axes {
            if colors != axis_colors(&self.settings) {
                self.upload_axes(&bounds);
            }
        }
    }

    /// Remakes the shadow map if the preferences now ask for a different size.
    fn ensure_shadow_target(&mut self) {
        let wanted = shadow_size(&self.settings);
        if self
            .shadow
            .as_ref()
            .is_some_and(|shadow| shadow.size == wanted)
        {
            return;
        }
        // SAFETY: The context is current (see `Renderer::new`), and the old target is taken out
        // before it is destroyed.
        unsafe {
            if let Some(shadow) = self.shadow.take() {
                shadow.destroy(&self.gl);
            }
            self.shadow = ShadowTarget::new(&self.gl, wanted)
                .inspect_err(|error| eprintln!("3D view: no shadows: {error}"))
                .ok();
        }
    }

    fn render_shadow_map(&mut self, light_matrix: &Mat4x4d) {
        let Some(shadow) = &self.shadow else {
            return;
        };

        // SAFETY: The context is current (see `Renderer::new`). Each batch's range lies inside
        // the geometry buffer, since `upload` builds both from the same vertices.
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(shadow.framebuffer));
            self.gl.viewport(0, 0, shadow.size, shadow.size);
            self.gl.clear(glow::DEPTH_BUFFER_BIT);

            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LESS);
            // Drawing back faces into the shadow map moves the self-shadowing bias problem
            // to the side of the surface the camera cannot see.
            self.gl.enable(glow::CULL_FACE);
            self.gl.front_face(glow::CW);
            self.gl.cull_face(glow::FRONT);

            self.gl.use_program(Some(self.depth.program));
            self.depth
                .set_matrix(&self.gl, "view_projection", light_matrix);

            let ordinary_opaque = self.batches.iter().all(|batch| {
                batch.alpha == map_model::material::Alpha::Opaque
                    && batch.material != Some(SKIP_PREVIEW_MATERIAL)
                    && batch
                        .material
                        .and_then(|m| self.model_pbrs.get(m))
                        .is_none_or(|p| p.is_none())
            });
            if ordinary_opaque {
                self.geometry.draw(&self.gl, glow::TRIANGLES);
            } else {
                self.gl.use_program(Some(self.material_depth.program));
                self.material_depth
                    .set_matrix(&self.gl, "view_projection", light_matrix);
                self.material_depth.set_i32(&self.gl, "material", 2);
                self.gl.bind_vertex_array(Some(self.geometry.array));
                for batch in &self.batches {
                    if batch.alpha == map_model::material::Alpha::Blended
                        || batch.material == Some(SKIP_PREVIEW_MATERIAL)
                    {
                        continue;
                    }
                    let pbr = batch
                        .material
                        .and_then(|m| self.model_pbrs.get(m))
                        .and_then(|p| p.as_ref());
                    if pbr.is_some_and(|p| p.double_sided)
                        || batch.alpha == map_model::material::Alpha::Masked
                    {
                        self.gl.disable(glow::CULL_FACE);
                    } else {
                        self.gl.enable(glow::CULL_FACE);
                        self.gl.cull_face(if pbr.is_some() {
                            glow::BACK
                        } else {
                            glow::FRONT
                        });
                    }
                    self.material_depth.set_f32(
                        &self.gl,
                        "material_face_mode",
                        pbr.map_or(0., |p| if p.double_sided { 2. } else { 1. }),
                    );
                    self.material_depth.set_f32(
                        &self.gl,
                        "material_alpha_mode",
                        if batch.alpha == map_model::material::Alpha::Masked {
                            1.
                        } else {
                            0.
                        },
                    );
                    self.material_depth.set_f32(
                        &self.gl,
                        "material_alpha_cutoff",
                        pbr.map_or(0.5, |p| p.alpha_cutoff),
                    );
                    let (w, h) = batch
                        .material
                        .and_then(|m| self.material_sizes.get(m))
                        .copied()
                        .unwrap_or((1., 1.));
                    self.material_depth
                        .set_vec2(&self.gl, "material_size", w, h);
                    if let Some(texture) = batch
                        .material
                        .and_then(|m| self.material_textures.get(m))
                        .copied()
                        .or(self.white)
                    {
                        bind_texture(&self.gl, 2, texture);
                    }
                    self.gl
                        .draw_arrays(glow::TRIANGLES, batch.first, batch.count);
                }
                self.gl.enable(glow::CULL_FACE);
                self.gl.cull_face(glow::FRONT);
            }
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, self.target);
        }
    }

    fn render_occlusion(&mut self, camera: &Camera3d, width: i32, height: i32) {
        self.ensure_screen_targets(width, height);
        let (Some(screen), Some(noise)) = (&self.screen, self.noise) else {
            return;
        };

        let view = camera.view_matrix();
        let projection = self.projection(camera, width, height);
        let Some(inverse_projection) = mat::invert(projection) else {
            return;
        };

        // SAFETY: The context is current (see `Renderer::new`). Each batch's range lies inside
        // the geometry buffer, since `upload` builds both from the same vertices.
        unsafe {
            // The normals and depth the occlusion pass reads.
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(screen.normal_framebuffer));
            self.gl.viewport(0, 0, width, height);
            self.gl.clear_color(0.5, 0.5, 1.0, 1.0);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LESS);
            // Not culled: a patch is a surface with no inside, and the depth test settles
            // a closed brush's far side anyway.
            self.gl.disable(glow::CULL_FACE);

            self.gl.use_program(Some(self.normals.program));
            self.normals.set_matrix(&self.gl, "view", &view);
            self.normals.set_matrix(&self.gl, "projection", &projection);
            self.gl.bind_vertex_array(Some(self.geometry.array));
            for batch in &self.batches {
                self.bind_normal_map(&self.normals, batch.material);
                self.gl
                    .draw_arrays(glow::TRIANGLES, batch.first, batch.count);
            }
            self.gl.bind_vertex_array(None);

            // The occlusion itself.
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(screen.ao_framebuffer));
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.use_program(Some(self.ssao.program));
            self.ssao.set_matrix(&self.gl, "projection", &projection);
            self.ssao
                .set_matrix(&self.gl, "inverse_projection", &inverse_projection);
            self.ssao.set_vec3_slice(&self.gl, "kernel", &self.kernel);
            self.ssao.set_vec2(
                &self.gl,
                "noise_scale",
                width as f32 / 4.0,
                height as f32 / 4.0,
            );
            self.ssao
                .set_f32(&self.gl, "radius", self.settings.ao_radius);
            self.ssao.set_f32(&self.gl, "bias", 0.5);
            self.ssao.set_i32(&self.gl, "depth_texture", 0);
            self.ssao.set_i32(&self.gl, "normal_texture", 1);
            self.ssao.set_i32(&self.gl, "noise_texture", 2);
            bind_texture(&self.gl, 0, screen.depth_texture);
            bind_texture(&self.gl, 1, screen.normal_texture);
            bind_texture(&self.gl, 2, noise);
            self.draw_fullscreen();

            // And the blur that takes the sampling noise back out.
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(screen.blur_framebuffer));
            self.gl.use_program(Some(self.blur.program));
            self.blur.set_i32(&self.gl, "source", 0);
            self.blur.set_i32(&self.gl, "depth_texture", 1);
            self.blur.set_i32(&self.gl, "normal_texture", 2);
            self.blur
                .set_matrix(&self.gl, "inverse_projection", &inverse_projection);
            self.blur
                .set_f32(&self.gl, "radius", self.settings.ao_radius);
            self.blur
                .set_vec2(&self.gl, "texel", 1.0 / width as f32, 1.0 / height as f32);
            bind_texture(&self.gl, 0, screen.ao_texture);
            bind_texture(&self.gl, 1, screen.depth_texture);
            bind_texture(&self.gl, 2, screen.normal_texture);
            self.draw_fullscreen();

            self.gl.bind_framebuffer(glow::FRAMEBUFFER, self.target);
        }
    }

    fn render_scene(
        &mut self,
        camera: &Camera3d,
        width: i32,
        height: i32,
        sun: Vec3d,
        light_matrix: &Mat4x4d,
    ) {
        let view_projection = self.projection(camera, width, height) * camera.view_matrix();
        // The map the scene samples, which may have been remade at a different size.
        let texel_side = self
            .shadow
            .as_ref()
            .map_or(1.0, |shadow| shadow.size as f32);

        // Into the floating-point buffer when there is one, so the bright parts survive to
        // be bloomed; straight to the screen when there is not.
        let into_post = self.post.as_ref().map(|post| post.hdr_framebuffer);

        // SAFETY: The context is current (see `Renderer::new`), and every framebuffer, texture
        // and program used here is a live object of the renderer's.
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, into_post.or(self.target));
            self.gl.viewport(0, 0, width, height);
            // The background the map sits on. The preference is in the colour a screen
            // shows; the scene pass works in linear light, so it is converted.
            let [r, g, b] = to_linear(self.settings.background_color);
            self.gl.clear_color(r, g, b, 1.0);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            if self.settings.procedural_sky {
                if let Some(inverse) = mat::invert(view_projection) {
                    self.gl.disable(glow::DEPTH_TEST);
                    self.gl.disable(glow::BLEND);
                    self.gl.disable(glow::CULL_FACE);
                    self.gl.depth_mask(false);
                    self.gl.use_program(Some(self.sky_background.program));
                    let program = &self.sky_background;
                    program.set_matrix(&self.gl, "inverse_view_projection", &inverse);
                    program.set_vec3(
                        &self.gl,
                        "camera_position",
                        camera.position.x() as f32,
                        camera.position.y() as f32,
                        camera.position.z() as f32,
                    );
                    for (name, color) in [
                        ("sky_color", self.settings.sky_color),
                        ("horizon_color", self.settings.horizon_color),
                        ("ground_color", self.settings.ground_color),
                    ] {
                        program.set_vec3(&self.gl, name, color[0], color[1], color[2]);
                    }
                    let color = self.settings.sun_color;
                    let energy = self.settings.sun_intensity;
                    program.set_vec3(
                        &self.gl,
                        "sun_color",
                        color[0] * energy,
                        color[1] * energy,
                        color[2] * energy,
                    );
                    program.set_vec3(
                        &self.gl,
                        "to_sun",
                        -sun.x() as f32,
                        -sun.y() as f32,
                        -sun.z() as f32,
                    );
                    program.set_f32(&self.gl, "exposure", self.settings.exposure);
                    program.set_f32(
                        &self.gl,
                        "tonemap_here",
                        if into_post.is_some() { 0.0 } else { 1.0 },
                    );
                    // The sky writes colour only, and the post-processing target also takes
                    // reflectivity: GLES and WebGL refuse to draw while a buffer the shader
                    // has no output for is active, so only the colour is for this pass.
                    if into_post.is_some() {
                        self.gl.draw_buffers(&[glow::COLOR_ATTACHMENT0, glow::NONE]);
                    }
                    self.draw_fullscreen();
                    if into_post.is_some() {
                        self.gl
                            .draw_buffers(&[glow::COLOR_ATTACHMENT0, glow::COLOR_ATTACHMENT1]);
                    }
                    self.gl.depth_mask(true);
                }
            }
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LESS);
            // Faces are wound clockwise as seen from outside, which is the opposite of
            // OpenGL's default; the shader uses that to tell which side of a patch is
            // being looked at.
            self.gl.front_face(glow::CW);
            // Not culled, for the same reason as the normal pass.
            self.gl.disable(glow::CULL_FACE);

            self.gl.use_program(Some(self.scene.program));
            self.scene
                .set_matrix(&self.gl, "view_projection", &view_projection);
            self.scene
                .set_matrix(&self.gl, "light_view_projection", light_matrix);
            self.scene.set_vec3(
                &self.gl,
                "camera_position",
                camera.position.x() as f32,
                camera.position.y() as f32,
                camera.position.z() as f32,
            );
            self.scene.set_vec3(
                &self.gl,
                "light_direction",
                sun.x() as f32,
                sun.y() as f32,
                sun.z() as f32,
            );
            // Warm sun, cool sky, dark ground: the standard three that make a shape read.
            // Each colour carries its own brightness, so the two are multiplied here
            // rather than being separate uniforms.
            let sun_color = self.settings.sun_color;
            let sun_intensity = self.settings.sun_intensity;
            self.scene.set_vec3(
                &self.gl,
                "light_color",
                sun_color[0] * sun_intensity,
                sun_color[1] * sun_intensity,
                sun_color[2] * sun_intensity,
            );
            let [sky_r, sky_g, sky_b] = self.settings.sky_color;
            self.scene
                .set_vec3(&self.gl, "sky_color", sky_r, sky_g, sky_b);
            let [haze_r, haze_g, haze_b] = self.settings.horizon_color;
            self.scene
                .set_vec3(&self.gl, "horizon_color", haze_r, haze_g, haze_b);
            let [ground_r, ground_g, ground_b] = self.settings.ground_color;
            self.scene
                .set_vec3(&self.gl, "ground_color", ground_r, ground_g, ground_b);
            self.scene
                .set_f32(&self.gl, "roughness_value", self.settings.default_roughness);
            self.scene
                .set_f32(&self.gl, "metallic_value", self.settings.default_metalness);
            self.scene.set_f32(
                &self.gl,
                "ambient_strength",
                self.settings.ambient_intensity,
            );
            self.scene
                .set_f32(&self.gl, "exposure", self.settings.exposure);
            self.scene.set_f32(
                &self.gl,
                "flat_shading",
                if self.settings.realistic_shading {
                    0.0
                } else {
                    1.0
                },
            );
            self.scene
                .set_vec2(&self.gl, "viewport", width as f32, height as f32);
            self.scene
                .set_vec2(&self.gl, "shadow_texel", 1.0 / texel_side, 1.0 / texel_side);
            self.scene.set_i32(&self.gl, "shadow_map", 0);
            self.scene.set_i32(&self.gl, "occlusion", 1);

            // The prepass depth, for the contact shadows. Without the prepass there is
            // nothing to march, so the march is turned off rather than reading whatever
            // is bound.
            let depth = self.screen.as_ref().map(|screen| screen.depth_texture);
            self.scene.set_i32(&self.gl, "scene_depth", 3);
            if let Some(depth) = depth {
                bind_texture(&self.gl, 3, depth);
            }
            self.scene
                .set_matrix(&self.gl, "view", &camera.view_matrix());
            self.scene.set_matrix(
                &self.gl,
                "projection",
                &self.projection(camera, width, height),
            );
            self.scene.set_f32(
                &self.gl,
                "contact_shadow_distance",
                if depth.is_some() && self.settings.contact_shadows && self.settings.shadows {
                    self.settings.contact_shadow_distance
                } else {
                    0.0
                },
            );
            self.scene.set_f32(
                &self.gl,
                "contact_shadow_samples",
                self.settings.contact_shadow_samples,
            );
            // A quarter of a unit off the surface: enough that a flat wall does not
            // shadow itself, small enough that the line where two surfaces meet is not
            // stepped over.
            self.scene
                .set_f32(&self.gl, "contact_shadow_thickness", 0.25);
            self.scene.set_f32(
                &self.gl,
                "shadow_softness",
                SHADOW_TAP_SPREAD * self.settings.shadow_softness,
            );
            self.scene.set_f32(
                &self.gl,
                "shadow_bias",
                SHADOW_BIAS_SCALE * self.settings.shadow_bias,
            );
            // Turning shadows off is the shader taking every surface as lit rather than
            // the pass being skipped: the map would otherwise hold whatever was in it.
            self.scene.set_f32(
                &self.gl,
                "shadow_strength",
                if self.settings.shadows { 1.0 } else { 0.0 },
            );
            self.scene.set_f32(
                &self.gl,
                "occlusion_strength",
                if self.settings.ambient_occlusion {
                    self.settings.ao_strength
                } else {
                    0.0
                },
            );
            // Without a composite pass there is nothing else to map the light, so the
            // scene shader does it itself.
            self.scene.set_f32(
                &self.gl,
                "tonemap_here",
                if into_post.is_some() { 0.0 } else { 1.0 },
            );

            // Where a pass could not be set up, the texture it would have filled is
            // replaced by the white one, which says "nothing is shadowed" and "nothing is
            // occluded".
            let shadow_texture = self
                .shadow
                .as_ref()
                .map(|shadow| shadow.depth_texture)
                .or(self.white);
            let occlusion_texture = self
                .screen
                .as_ref()
                .map(|screen| screen.blur_texture)
                .or(self.white);
            if let Some(texture) = shadow_texture {
                bind_texture(&self.gl, 0, texture);
            }
            if let Some(texture) = occlusion_texture {
                bind_texture(&self.gl, 1, texture);
            }

            self.scene.set_f32(&self.gl, "lit", 1.0);
            self.scene.set_i32(&self.gl, "material", 2);
            self.draw_batches();
        }
    }

    /// The projection for this frame, nudged by less than a pixel.
    ///
    /// Antialiasing by accumulation: the same still view is drawn again and again with
    /// the sample point moved about inside the pixel, and the average of those is a
    /// picture with no stair-stepped edges at all. This is what a Blender viewport does
    /// while nothing is moving, and it is worth much more than a post-process guess at
    /// where the edges were. While the view is moving there is nothing to accumulate, so
    /// the offset is zero and FXAA does the work instead.
    fn projection(&self, camera: &Camera3d, width: i32, height: i32) -> Mat4x4d {
        let projection = camera.projection_matrix(width, height);
        if self.sample == 0 {
            return projection;
        }

        // A Halton sequence rather than random offsets: it fills the pixel evenly at
        // every count, so ten samples look like ten samples rather than like nine and a
        // clump.
        let (x, y) = (halton(self.sample, 2) - 0.5, halton(self.sample, 3) - 0.5);
        // Clip space is two units across, so a pixel is two over the side.
        let offset = Vec3d::new(
            x * 2.0 / f64::from(width.max(1)),
            y * 2.0 / f64::from(height.max(1)),
            0.0,
        );
        mat_ext::translation_matrix(offset) * projection
    }

    /// Gathers the reflections the composite adds.
    ///
    /// Runs between the scene and the composite: it needs the scene's colour to reflect
    /// and the weights the scene wrote alongside it. What it writes is the difference
    /// from the sky already reflected, so the composite adds it rather than blending.
    fn render_reflections(&self, camera: &Camera3d, width: i32, height: i32) {
        let (Some(post), Some(screen)) = (self.post.as_ref(), self.screen.as_ref()) else {
            return;
        };

        let projection = self.projection(camera, width, height);
        let Some(inverse_projection) = mat::invert(projection) else {
            return;
        };

        // SAFETY: The context is current (see `Renderer::new`), and every framebuffer, texture
        // and program used here is a live object of the renderer's.
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(post.ssr_framebuffer));
            self.gl.viewport(0, 0, width, height);
            // Nothing found means nothing added, so the buffer starts at nothing.
            self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            self.gl.disable(glow::DEPTH_TEST);

            self.gl.use_program(Some(self.ssr.program));
            self.ssr.set_matrix(&self.gl, "projection", &projection);
            self.ssr
                .set_matrix(&self.gl, "inverse_projection", &inverse_projection);
            self.ssr.set_matrix(&self.gl, "view", &camera.view_matrix());

            let [sky_r, sky_g, sky_b] = self.settings.sky_color;
            self.ssr
                .set_vec3(&self.gl, "sky_color", sky_r, sky_g, sky_b);
            let [haze_r, haze_g, haze_b] = self.settings.horizon_color;
            self.ssr
                .set_vec3(&self.gl, "horizon_color", haze_r, haze_g, haze_b);
            let [ground_r, ground_g, ground_b] = self.settings.ground_color;
            self.ssr
                .set_vec3(&self.gl, "ground_color", ground_r, ground_g, ground_b);
            let sun = sun_direction(&self.settings);
            self.ssr.set_vec3(
                &self.gl,
                "light_direction",
                sun.x() as f32,
                sun.y() as f32,
                sun.z() as f32,
            );
            let sun_color = self.settings.sun_color;
            let sun_intensity = self.settings.sun_intensity;
            self.ssr.set_vec3(
                &self.gl,
                "light_color",
                sun_color[0] * sun_intensity,
                sun_color[1] * sun_intensity,
                sun_color[2] * sun_intensity,
            );

            self.ssr
                .set_f32(&self.gl, "ssr_distance", self.settings.reflection_distance);
            self.ssr
                .set_f32(&self.gl, "ssr_samples", self.settings.reflection_samples);
            self.ssr.set_f32(&self.gl, "ssr_thickness", 0.5);
            self.ssr.set_f32(
                &self.gl,
                "ssr_strength",
                if self.settings.reflections {
                    self.settings.reflection_strength
                } else {
                    0.0
                },
            );

            self.ssr.set_i32(&self.gl, "scene_color", 0);
            self.ssr.set_i32(&self.gl, "scene_depth", 1);
            self.ssr.set_i32(&self.gl, "scene_normal", 2);
            self.ssr.set_i32(&self.gl, "reflectivity", 3);
            bind_texture(&self.gl, 0, post.hdr_texture);
            bind_texture(&self.gl, 1, screen.depth_texture);
            bind_texture(&self.gl, 2, screen.normal_texture);
            bind_texture(&self.gl, 3, post.reflect_texture);
            self.draw_fullscreen();
        }
    }

    /// Draws the axes and the tools' handles, in the colours they were chosen in.
    ///
    /// After the composite rather than with the scene: the scene is in linear light and
    /// gets tone mapped, which would shift colours picked to be looked at. This pass draws
    /// into the finished picture, but against the same depth the scene was drawn with, so
    /// what is behind a wall is still behind it.
    fn render_overlay(&self, into: Option<glow::Framebuffer>, width: i32, height: i32) {
        // SAFETY: The context is current (see `Renderer::new`). The meshes draw only the vertices
        // their last upload holds.
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, into);
            self.gl.viewport(0, 0, width, height);

            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LESS);
            self.gl.disable(glow::CULL_FACE);
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            self.gl.use_program(Some(self.scene.program));
            self.scene.set_f32(&self.gl, "lit", 0.0);
            self.scene.set_f32(&self.gl, "tonemap_here", 0.0);
            self.scene.set_f32(&self.gl, "alpha_scale", 1.0);
            // `MapViewBase::renderCoordinateSystem` draws the axes only when the
            // preference asks for them.
            if self.settings.show_axes {
                self.lines.draw(&self.gl, glow::LINES);
            }

            // The tools' handles, drawn against the depth buffer and then faintly through
            // it, so a handle round a corner is still findable.
            // GLES always takes the point size from the shader, and has no switch for it.
            if !embedded(&self.gl) {
                self.gl.enable(glow::PROGRAM_POINT_SIZE);
            }
            self.scene
                .set_f32(&self.gl, "point_size", handle_point_size(&self.settings));
            for (test_depth, alpha) in [(true, 1.0f32), (false, 0.35)] {
                if test_depth {
                    self.gl.enable(glow::DEPTH_TEST);
                } else {
                    self.gl.disable(glow::DEPTH_TEST);
                }
                self.scene.set_f32(&self.gl, "alpha_scale", alpha);
                self.scene.set_f32(&self.gl, "round_points", 0.0);
                self.guides.draw(&self.gl, glow::LINES);
                // Only the points are discs; a line has no point coordinate to cut.
                self.scene.set_f32(&self.gl, "round_points", 1.0);
                self.handles.draw(&self.gl, glow::POINTS);
                self.scene.set_f32(&self.gl, "round_points", 0.0);
            }
            if !embedded(&self.gl) {
                self.gl.disable(glow::PROGRAM_POINT_SIZE);
            }
            self.scene.set_f32(&self.gl, "point_size", 1.0);

            self.gl.disable(glow::BLEND);
            self.gl.enable(glow::DEPTH_TEST);
        }
    }

    /// Shared normal sampling for the lit scene and screen-space normal pass.
    fn bind_normal_map(&self, program: &Program, material: Option<usize>) {
        let normal = material
            .and_then(|i| self.normal_textures.get(i))
            .copied()
            .flatten();
        let pbr = material
            .and_then(|i| self.model_pbrs.get(i))
            .and_then(|p| p.as_ref());
        program.set_f32(
            &self.gl,
            "material_face_mode",
            pbr.map_or(0.0, |p| if p.double_sided { 2.0 } else { 1.0 }),
        );
        program.set_i32(&self.gl, "normal_map", 4);
        program.set_f32(
            &self.gl,
            "has_normal_map",
            if normal.is_some() { 1.0 } else { 0.0 },
        );
        program.set_f32(
            &self.gl,
            "normal_scale",
            material
                .and_then(|i| self.model_pbrs.get(i))
                .and_then(|p| p.as_ref())
                .map_or(1., |p| p.normal_scale),
        );
        program.set_f32(
            &self.gl,
            "normal_y_sign",
            if normal.is_some_and(|(_, flip)| flip) {
                -1.0
            } else {
                1.0
            },
        );
        if let Some((texture, _)) = normal {
            bind_texture(&self.gl, 4, texture);
        }
        let (width, height) = material
            .and_then(|i| self.material_sizes.get(i))
            .copied()
            .unwrap_or((1.0, 1.0));
        program.set_vec2(&self.gl, "material_size", width, height);
    }

    /// Draws the geometry, one pass per material.
    ///
    /// # Safety
    /// The context must be current on this thread.
    unsafe fn draw_batches(&self) {
        // SAFETY: The caller keeps the context current. Each batch's range lies inside the
        // geometry buffer, since `upload` builds both from the same vertices.
        unsafe {
            self.gl.bind_vertex_array(Some(self.geometry.array));
            let mut blending = false;
            for batch in &self.batches {
                self.scene.set_i32(
                    &self.gl,
                    "skip_hatch",
                    i32::from(batch.material == Some(SKIP_PREVIEW_MATERIAL)),
                );
                let pbr = batch
                    .material
                    .and_then(|i| self.model_pbrs.get(i))
                    .and_then(|p| p.as_ref());
                let maps = batch
                    .material
                    .and_then(|i| self.pbr_textures.get(i))
                    .copied()
                    .flatten();
                self.scene.set_i32(&self.gl, "model_orm", 5);
                self.scene.set_i32(&self.gl, "model_emissive", 6);
                if let Some(white) = self.white {
                    bind_texture(&self.gl, 5, maps.map_or(white, |m| m.0));
                    bind_texture(&self.gl, 6, maps.map_or(white, |m| m.1));
                }
                let e = pbr.map_or([0.; 3], |p| p.emissive_factor);
                self.scene
                    .set_vec3(&self.gl, "model_emissive_factor", e[0], e[1], e[2]);
                self.scene.set_f32(
                    &self.gl,
                    "model_ao_strength",
                    pbr.map_or(0., |p| p.occlusion_strength),
                );
                let [roughness, metallic, emission] = batch
                    .surface_material
                    .map(|s| s.factors())
                    .unwrap_or_else(|| {
                        pbr.map_or(
                            [
                                self.settings.default_roughness,
                                self.settings.default_metalness,
                                0.0,
                            ],
                            |p| [p.roughness, p.metallic, 0.],
                        )
                    });
                self.scene.set_f32(&self.gl, "roughness_value", roughness);
                self.scene.set_f32(&self.gl, "metallic_value", metallic);
                self.scene.set_f32(&self.gl, "emission_value", emission);
                let texture = batch
                    .material
                    .and_then(|index| self.material_textures.get(index).copied())
                    .or(self.white);
                let Some(texture) = texture else {
                    continue;
                };
                bind_texture(&self.gl, 2, texture);
                self.bind_normal_map(&self.scene, batch.material);

                // A surface with no material gets the white pixel and a size of one, which
                // leaves the vertex colour showing through unchanged.
                let (width, height) = batch
                    .material
                    .and_then(|index| self.material_sizes.get(index).copied())
                    .unwrap_or((1.0, 1.0));
                self.scene
                    .set_vec2(&self.gl, "material_size", width, height);

                let mode = match batch.alpha {
                    map_model::material::Alpha::Opaque => 0.0,
                    map_model::material::Alpha::Masked => 1.0,
                    map_model::material::Alpha::Blended => 2.0,
                };
                self.scene.set_f32(&self.gl, "material_alpha_mode", mode);
                self.scene.set_f32(
                    &self.gl,
                    "material_alpha_cutoff",
                    pbr.map_or(0.5, |p| p.alpha_cutoff),
                );

                // The blended batches are last. Turn on blending and stop writing depth
                // once the first one comes round: a pane of glass should not hide what is
                // behind another pane, and two panes read better unsorted this way than
                // with one punching a hole in the other.
                if batch.alpha == map_model::material::Alpha::Blended && !blending {
                    self.gl.enable(glow::BLEND);
                    self.gl
                        .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    self.gl.depth_mask(false);
                    blending = true;
                }

                self.gl
                    .draw_arrays(glow::TRIANGLES, batch.first, batch.count);
            }
            if blending {
                self.gl.depth_mask(true);
                self.gl.disable(glow::BLEND);
            }
            self.scene.set_f32(&self.gl, "material_alpha_mode", 0.0);
            self.scene.set_i32(&self.gl, "skip_hatch", 0);
            self.gl.bind_vertex_array(None);
        }
    }

    /// Draws one triangle over the whole target, its corners made from `gl_VertexID`.
    ///
    /// # Safety
    /// The context must be current on this thread.
    unsafe fn draw_fullscreen(&self) {
        // SAFETY: The caller keeps the context current. The vertex array has no attribute
        // enabled, so the draw reads no buffer.
        unsafe {
            self.gl.bind_vertex_array(Some(self.fullscreen));
            self.gl.draw_arrays(glow::TRIANGLES, 0, 3);
            self.gl.bind_vertex_array(None);
        }
    }

    /// The post-processing chain: bloom, the tone map, and anti-aliasing.
    ///
    /// The scene is drawn in linear light with no upper bound, which is the only way the
    /// bright parts can be found and made to bloom; this is where it becomes a picture.
    /// With no targets to do it in, the scene shader has already tone mapped and there is
    /// nothing left to do.
    fn render_bloom_and_composite(&mut self, camera: &Camera3d) {
        let Some(post) = &self.post else {
            return;
        };
        let (width, height) = (post.width, post.height);

        // SAFETY: The context is current (see `Renderer::new`), and every framebuffer, texture
        // and program used here is a live object of the renderer's.
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.disable(glow::CULL_FACE);

            // What is bright enough to bloom, at half size: bloom is a wide, soft glow, so
            // the resolution it is built at is not what decides how it looks.
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(post.bloom_framebuffers[0]));
            self.gl.viewport(0, 0, post.bloom_width, post.bloom_height);
            self.gl.use_program(Some(self.bright.program));
            self.bright.set_i32(&self.gl, "source", 0);
            self.bright
                .set_f32(&self.gl, "threshold", self.settings.bloom_threshold);
            self.bright.set_f32(&self.gl, "knee", BLOOM_KNEE);
            bind_texture(&self.gl, 0, post.hdr_texture);
            self.draw_fullscreen();

            // Two passes of a separable blur, across and then down.
            self.gl.use_program(Some(self.gaussian.program));
            self.gaussian.set_i32(&self.gl, "source", 0);
            for (pass, direction) in [
                (1usize, (1.0 / post.bloom_width as f32, 0.0)),
                (0usize, (0.0, 1.0 / post.bloom_height as f32)),
            ] {
                self.gl
                    .bind_framebuffer(glow::FRAMEBUFFER, Some(post.bloom_framebuffers[pass]));
                self.gaussian
                    .set_vec2(&self.gl, "direction", direction.0, direction.1);
                bind_texture(&self.gl, 0, post.bloom_textures[1 - pass]);
                self.draw_fullscreen();
            }

            // The scene and its bloom, mapped and encoded, at full size.
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(post.ldr_framebuffer));
            self.gl.viewport(0, 0, width, height);
            self.gl.use_program(Some(self.composite.program));
            self.composite.set_i32(&self.gl, "scene_depth", 3);
            if let Some(screen) = &self.screen {
                bind_texture(&self.gl, 3, screen.depth_texture);
            }
            let dof = self
                .settings
                .depth_of_field
                .filter(|_| self.screen.is_some());
            self.composite.set_vec2(
                &self.gl,
                "clip_planes",
                camera.near as f32,
                camera.far as f32,
            );
            self.composite
                .set_vec2(&self.gl, "image_size", width as f32, height as f32);
            self.composite.set_vec3(
                &self.gl,
                "dof",
                dof.map_or(1., |d| d.focus_distance.max(1.)),
                dof.map_or(0., |d| d.focus_range.max(0.)),
                dof.map_or(0., |d| d.radius.clamp(0., 24.)),
            );
            self.composite.set_i32(&self.gl, "scene", 0);
            self.composite.set_i32(&self.gl, "bloom", 1);
            self.composite.set_i32(&self.gl, "reflections", 2);
            bind_texture(&self.gl, 2, post.ssr_texture);
            self.composite.set_f32(
                &self.gl,
                "bloom_intensity",
                if self.settings.bloom {
                    self.settings.bloom_intensity
                } else {
                    0.0
                },
            );
            self.composite
                .set_f32(&self.gl, "exposure", self.settings.exposure);
            bind_texture(&self.gl, 0, post.hdr_texture);
            bind_texture(&self.gl, 1, post.bloom_textures[0]);
            self.draw_fullscreen();

            self.gl.enable(glow::DEPTH_TEST);
        }
    }

    /// Softens the edges of the finished picture, onto whatever the host is drawing into.
    /// Averages this frame into what the still view has gathered, and returns what the
    /// last pass should put on the screen.
    ///
    /// The two buffers swap each frame, since a shader cannot read the one it writes.
    /// Which one holds the answer is decided by the count, so the next frame reads the
    /// one this frame wrote.
    fn render_accumulation(&self, width: i32, height: i32) -> Option<glow::Texture> {
        let post = self.post.as_ref()?;
        if !self.settings.accumulate {
            return None;
        }

        let write = (self.sample % 2) as usize;
        let read = 1 - write;

        // SAFETY: The context is current (see `Renderer::new`), and every framebuffer, texture
        // and program used here is a live object of the renderer's.
        unsafe {
            self.gl.bind_framebuffer(
                glow::FRAMEBUFFER,
                Some(post.accumulation_framebuffers[write]),
            );
            self.gl.viewport(0, 0, width, height);
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.use_program(Some(self.accumulate.program));
            self.accumulate.set_i32(&self.gl, "current", 0);
            self.accumulate.set_i32(&self.gl, "history", 1);
            // A running average rather than a fade, so an early frame counts as much as a
            // late one.
            self.accumulate
                .set_f32(&self.gl, "weight", 1.0 / (self.sample as f32 + 1.0));
            bind_texture(&self.gl, 0, post.ldr_texture);
            bind_texture(&self.gl, 1, post.accumulation_textures[read]);
            self.draw_fullscreen();
            self.gl.enable(glow::DEPTH_TEST);
        }

        Some(post.accumulation_textures[write])
    }

    fn render_anti_aliasing(&self, source: Option<glow::Texture>, width: i32, height: i32) {
        let Some(post) = &self.post else {
            return;
        };

        // SAFETY: The context is current (see `Renderer::new`), and every framebuffer, texture
        // and program used here is a live object of the renderer's.
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, self.target);
            self.gl.viewport(0, 0, width, height);
            self.gl.use_program(Some(self.fxaa.program));
            self.fxaa.set_i32(&self.gl, "source", 0);
            self.fxaa
                .set_vec2(&self.gl, "texel", 1.0 / width as f32, 1.0 / height as f32);
            // Turning it off is the shader passing the picture through rather than the
            // pass being skipped: the picture is in the low dynamic range buffer, and
            // this is what puts it on the screen.
            //
            // An accumulated picture is already smooth, and guessing at edges in it would
            // only soften what is right; so the guess is only made on the first frame of
            // a view, which is the one seen while the camera is moving.
            let guessing = self.settings.antialiasing && self.sample == 0;
            self.fxaa
                .set_f32(&self.gl, "amount", if guessing { 1.0 } else { 0.0 });
            bind_texture(&self.gl, 0, source.unwrap_or(post.ldr_texture));
            self.draw_fullscreen();
            self.gl.enable(glow::DEPTH_TEST);
        }
    }

    /// Makes sure the post-processing targets match the viewport.
    fn ensure_post_targets(&mut self, width: i32, height: i32) {
        if self
            .post
            .as_ref()
            .is_some_and(|post| post.width == width && post.height == height)
        {
            return;
        }

        if let Some(old) = self.post.take() {
            // SAFETY: The context is current (see `Renderer::new`), and `old` was taken out, so
            // nothing uses it again.
            unsafe { old.destroy(&self.gl) };
        }
        // SAFETY: The context is current (see `Renderer::new`).
        self.post = unsafe { PostTargets::new(&self.gl, width, height) }
            .inspect_err(|error| eprintln!("3D view: no bloom or anti-aliasing: {error}"))
            .ok();
    }

    /// Makes sure the screen-sized targets match the viewport.
    fn ensure_screen_targets(&mut self, width: i32, height: i32) {
        if self
            .screen
            .as_ref()
            .is_some_and(|screen| screen.width == width && screen.height == height)
        {
            return;
        }

        if let Some(old) = self.screen.take() {
            // SAFETY: The context is current (see `Renderer::new`), and `old` was taken out, so
            // nothing uses it again.
            unsafe { old.destroy(&self.gl) };
        }
        // SAFETY: The context is current (see `Renderer::new`).
        self.screen = unsafe { ScreenTargets::new(&self.gl, width, height) }
            .inspect_err(|error| eprintln!("3D view: no ambient occlusion: {error}"))
            .ok();
    }
}

/* ========== a shader program ========== */

struct Program {
    program: glow::Program,
    uniforms: HashMap<String, glow::UniformLocation>,
}

impl Program {
    /// Compiles and links a program from the two sources, rewritten for GLES if need be.
    ///
    /// # Safety
    /// `gl`'s context must be current on this thread.
    unsafe fn new(
        gl: &glow::Context,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<Self, String> {
        // SAFETY: The caller keeps `gl`'s context current, and every object used here was made
        // in it just above.
        unsafe {
            let program = gl
                .create_program()
                .map_err(|e| format!("could not create a shader program: {e}"))?;

            let mut shaders = Vec::new();
            for (kind, source) in [
                (glow::VERTEX_SHADER, vertex_source),
                (glow::FRAGMENT_SHADER, fragment_source),
            ] {
                let shader = gl
                    .create_shader(kind)
                    .map_err(|e| format!("could not create a shader: {e}"))?;
                gl.shader_source(shader, &for_context(gl, source));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    return Err(format!(
                        "the shader did not compile: {}",
                        gl.get_shader_info_log(shader)
                    ));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            // Fixing the attribute indices before linking is what lets one vertex array
            // feed every program.
            for (index, (name, _, _)) in ATTRIBUTES.iter().enumerate() {
                gl.bind_attrib_location(program, index as u32, name);
            }

            // And the outputs, for the same reason and with a worse failure. GLSL 1.50
            // has no `layout(location = ...)`, so a shader with two outputs gets whatever
            // the linker chose unless it is told; the second one then writes into nothing
            // and reads back as whatever the buffer was cleared to. A name the shader does
            // not have is ignored, so every program can be told the same list.
            // GLSL ES places them with `layout` instead (`for_context`), and WebGL has no
            // call for it at all.
            if !embedded(gl) {
                for (index, name) in FRAGMENT_OUTPUTS.iter().enumerate() {
                    gl.bind_frag_data_location(program, index as u32, name);
                }
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(format!(
                    "the shader program did not link: {}",
                    gl.get_program_info_log(program)
                ));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            // The program is asked what uniforms it has rather than being told: a name
            // left out of a hand-written list would be set into nothing, and a uniform
            // that silently stays zero is a black screen with no error to go on.
            //
            // A uniform the compiler found no use for is optimized away and is not among
            // these; that is not an error, it just has nothing to set.
            let mut uniforms = HashMap::new();
            for index in 0..gl.get_active_uniforms(program) {
                let Some(uniform) = gl.get_active_uniform(program, index) else {
                    continue;
                };
                // An array comes back as `kernel[0]`; the name to look it up by is the
                // one without the subscript.
                let name = uniform
                    .name
                    .split_once('[')
                    .map_or(uniform.name.as_str(), |(base, _)| base);
                if let Some(location) = gl.get_uniform_location(program, name) {
                    uniforms.insert(name.to_string(), location);
                }
            }

            Ok(Self { program, uniforms })
        }
    }

    fn set_matrix(&self, gl: &glow::Context, name: &str, matrix: &Mat4x4d) {
        let Some(location) = self.uniforms.get(name) else {
            return;
        };
        // GL wants the elements column by column, which is how `Matrix` already stores
        // them, so this is a copy rather than a transpose.
        let mut elements = [0f32; 16];
        for column in 0..4 {
            for row in 0..4 {
                elements[column * 4 + row] = matrix[column][row] as f32;
            }
        }
        // SAFETY: `gl` is the renderer's context, current while it draws (see `Renderer::new`),
        // and `location` came from this program. GL reads the 16 elements of `elements`.
        unsafe { gl.uniform_matrix_4_f32_slice(Some(location), false, &elements) };
    }

    fn set_vec3(&self, gl: &glow::Context, name: &str, x: f32, y: f32, z: f32) {
        if let Some(location) = self.uniforms.get(name) {
            // SAFETY: `gl` is the renderer's context, current while it draws (see `Renderer::new`),
            // and `location` came from this program.
            unsafe { gl.uniform_3_f32(Some(location), x, y, z) };
        }
    }

    fn set_vec3_slice(&self, gl: &glow::Context, name: &str, values: &[f32]) {
        if let Some(location) = self.uniforms.get(name) {
            // SAFETY: `gl` is the renderer's context, current while it draws (see `Renderer::new`),
            // and `location` came from this program. glow passes GL the slice's own length.
            unsafe { gl.uniform_3_f32_slice(Some(location), values) };
        }
    }

    fn set_vec2(&self, gl: &glow::Context, name: &str, x: f32, y: f32) {
        if let Some(location) = self.uniforms.get(name) {
            // SAFETY: `gl` is the renderer's context, current while it draws (see `Renderer::new`),
            // and `location` came from this program.
            unsafe { gl.uniform_2_f32(Some(location), x, y) };
        }
    }

    fn set_f32(&self, gl: &glow::Context, name: &str, value: f32) {
        if let Some(location) = self.uniforms.get(name) {
            // SAFETY: `gl` is the renderer's context, current while it draws (see `Renderer::new`),
            // and `location` came from this program.
            unsafe { gl.uniform_1_f32(Some(location), value) };
        }
    }

    fn set_i32(&self, gl: &glow::Context, name: &str, value: i32) {
        if let Some(location) = self.uniforms.get(name) {
            // SAFETY: `gl` is the renderer's context, current while it draws (see `Renderer::new`),
            // and `location` came from this program.
            unsafe { gl.uniform_1_i32(Some(location), value) };
        }
    }

    /// Deletes the program.
    ///
    /// # Safety
    /// `gl` must be the context that made the program, current on this thread, and the
    /// program must not be used afterwards.
    unsafe fn destroy(&self, gl: &glow::Context) {
        // SAFETY: The caller keeps `gl`'s context current and stops using the program.
        unsafe { gl.delete_program(self.program) };
    }
}

/* ========== geometry ========== */

struct Mesh {
    array: glow::VertexArray,
    buffer: glow::Buffer,
    count: i32,
}

impl Mesh {
    /// A vertex array and an empty buffer laid out as [`ATTRIBUTES`] says.
    ///
    /// # Safety
    /// `gl`'s context must be current on this thread.
    unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        // SAFETY: The caller keeps `gl`'s context current. The attribute offsets are into the
        // bound buffer, not into memory of ours, and stay inside one vertex (the vertex layout
        // test checks that they add up).
        unsafe {
            let array = gl
                .create_vertex_array()
                .map_err(|e| format!("could not create a vertex array: {e}"))?;
            let buffer = gl
                .create_buffer()
                .map_err(|e| format!("could not create a buffer: {e}"))?;

            gl.bind_vertex_array(Some(array));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));

            let stride = (FLOATS_PER_VERTEX * std::mem::size_of::<f32>()) as i32;
            for (index, (_, components, offset)) in ATTRIBUTES.iter().enumerate() {
                gl.enable_vertex_attrib_array(index as u32);
                gl.vertex_attrib_pointer_f32(
                    index as u32,
                    *components,
                    glow::FLOAT,
                    false,
                    stride,
                    offset * std::mem::size_of::<f32>() as i32,
                );
            }
            gl.bind_vertex_array(None);

            Ok(Self {
                array,
                buffer,
                count: 0,
            })
        }
    }

    fn upload(&mut self, gl: &glow::Context, data: &[f32], usage: u32) {
        self.count = (data.len() / FLOATS_PER_VERTEX) as i32;
        // SAFETY: `gl` is the renderer's context, current while it works (see `Renderer::new`).
        // `bytes_of` covers exactly `data`, so GL copies that and no more, and `count` above
        // matches what the buffer now holds.
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes_of(data), usage);
        }
    }

    /// Draws every vertex of the last upload.
    ///
    /// # Safety
    /// `gl` must be the context that made the mesh, current on this thread.
    unsafe fn draw(&self, gl: &glow::Context, mode: u32) {
        if self.count == 0 {
            return;
        }
        // SAFETY: The caller keeps `gl`'s context current. `count` comes from the last upload,
        // so the draw reads only vertices the buffer holds.
        unsafe {
            gl.bind_vertex_array(Some(self.array));
            gl.draw_arrays(mode, 0, self.count);
            gl.bind_vertex_array(None);
        }
    }

    /// Deletes the vertex array and the buffer.
    ///
    /// # Safety
    /// `gl` must be the context that made the mesh, current on this thread, and the mesh
    /// must not be used afterwards.
    unsafe fn destroy(&self, gl: &glow::Context) {
        // SAFETY: The caller keeps `gl`'s context current and stops using the mesh.
        unsafe {
            gl.delete_vertex_array(self.array);
            gl.delete_buffer(self.buffer);
        }
    }
}

/* ========== render targets ========== */

struct ShadowTarget {
    framebuffer: glow::Framebuffer,
    depth_texture: glow::Texture,
    /// The side the map was made at, so a change in the preferences can remake it.
    size: i32,
}

impl ShadowTarget {
    /// A depth texture `size` texels square, with a framebuffer to draw into it.
    ///
    /// # Safety
    /// `gl`'s context must be current on this thread.
    unsafe fn new(gl: &glow::Context, size: i32) -> Result<Self, String> {
        // SAFETY: The caller keeps `gl`'s context current, and every object used here was made
        // in it just above. No call reads memory of ours except the four-float border colour.
        unsafe {
            let depth_texture = make_texture(
                gl,
                glow::DEPTH_COMPONENT24 as i32,
                size,
                size,
                glow::DEPTH_COMPONENT,
                // GLES takes a 24-bit depth texture only as unsigned integers.
                glow::UNSIGNED_INT,
                // Not every driver will filter a depth texture, and the nine taps below
                // are the filtering anyway.
                glow::NEAREST,
            )?;
            // Outside the map, nothing is in shadow, so the border reads as fully lit.
            // GLES 3.0 has no border: there the map's edge stretches out instead.
            let wrap = if embedded(gl) {
                glow::CLAMP_TO_EDGE
            } else {
                glow::CLAMP_TO_BORDER
            };
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap as i32);
            if !embedded(gl) {
                gl.tex_parameter_f32_slice(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_BORDER_COLOR,
                    &[1.0, 1.0, 1.0, 1.0],
                );
            }

            let framebuffer = gl
                .create_framebuffer()
                .map_err(|e| format!("could not create a framebuffer: {e}"))?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::TEXTURE_2D,
                Some(depth_texture),
                0,
            );
            // Depth only, so there is no colour buffer to write to or read from.
            gl.draw_buffers(&[glow::NONE]);
            gl.read_buffer(glow::NONE);

            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!(
                    "the shadow framebuffer is not usable ({status:#x})"
                ));
            }

            Ok(Self {
                framebuffer,
                depth_texture,
                size,
            })
        }
    }

    /// Deletes the framebuffer and the texture.
    ///
    /// # Safety
    /// `gl` must be the context that made the target, current on this thread, and the
    /// target must not be used afterwards.
    unsafe fn destroy(&self, gl: &glow::Context) {
        // SAFETY: The caller keeps `gl`'s context current and stops using the target.
        unsafe {
            gl.delete_framebuffer(self.framebuffer);
            gl.delete_texture(self.depth_texture);
        }
    }
}

struct ScreenTargets {
    width: i32,
    height: i32,
    normal_framebuffer: glow::Framebuffer,
    normal_texture: glow::Texture,
    depth_texture: glow::Texture,
    ao_framebuffer: glow::Framebuffer,
    ao_texture: glow::Texture,
    blur_framebuffer: glow::Framebuffer,
    blur_texture: glow::Texture,
}

impl ScreenTargets {
    /// The normal, occlusion and blur targets at the viewport's size.
    ///
    /// # Safety
    /// `gl`'s context must be current on this thread.
    unsafe fn new(gl: &glow::Context, width: i32, height: i32) -> Result<Self, String> {
        // SAFETY: The caller keeps `gl`'s context current, and every object used here was made
        // in it just above.
        unsafe {
            let normal_texture = make_texture(
                gl,
                glow::RGBA8 as i32,
                width,
                height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::NEAREST,
            )?;
            let depth_texture = make_texture(
                gl,
                glow::DEPTH_COMPONENT24 as i32,
                width,
                height,
                glow::DEPTH_COMPONENT,
                // GLES takes a 24-bit depth texture only as unsigned integers.
                glow::UNSIGNED_INT,
                glow::NEAREST,
            )?;

            let normal_framebuffer = gl
                .create_framebuffer()
                .map_err(|e| format!("could not create a framebuffer: {e}"))?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(normal_framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(normal_texture),
                0,
            );
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::TEXTURE_2D,
                Some(depth_texture),
                0,
            );
            gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
            check_framebuffer(gl, "the normal framebuffer")?;

            let (ao_framebuffer, ao_texture) = make_single_channel_target(gl, width, height)?;
            let (blur_framebuffer, blur_texture) = make_single_channel_target(gl, width, height)?;

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            Ok(Self {
                width,
                height,
                normal_framebuffer,
                normal_texture,
                depth_texture,
                ao_framebuffer,
                ao_texture,
                blur_framebuffer,
                blur_texture,
            })
        }
    }

    /// Deletes the framebuffers and their textures.
    ///
    /// # Safety
    /// `gl` must be the context that made the targets, current on this thread, and the
    /// targets must not be used afterwards.
    unsafe fn destroy(&self, gl: &glow::Context) {
        // SAFETY: The caller keeps `gl`'s context current and stops using the targets.
        unsafe {
            gl.delete_framebuffer(self.normal_framebuffer);
            gl.delete_framebuffer(self.ao_framebuffer);
            gl.delete_framebuffer(self.blur_framebuffer);
            gl.delete_texture(self.normal_texture);
            gl.delete_texture(self.depth_texture);
            gl.delete_texture(self.ao_texture);
            gl.delete_texture(self.blur_texture);
        }
    }
}

/// Where the scene is drawn before it becomes a picture.
///
/// The scene goes into a floating-point buffer with no upper bound, so the bright parts
/// stay bright enough to find; the bloom is built at half that size, since a wide soft
/// glow does not need the resolution; and the finished picture goes into an ordinary one
/// for the anti-aliasing to read.
struct PostTargets {
    width: i32,
    height: i32,
    bloom_width: i32,
    bloom_height: i32,
    hdr_framebuffer: glow::Framebuffer,
    hdr_texture: glow::Texture,
    /// How strongly each pixel reflects, and how roughly.
    ///
    /// The scene pass writes it alongside the colour, so the reflection pass knows what
    /// weight to give a hit and how far to blur it. Without it a reflection could only be
    /// added on top of the sky already reflected there, which would count the light
    /// twice.
    reflect_texture: glow::Texture,
    /// Where the reflections are gathered, before the composite adds them.
    ssr_framebuffer: glow::Framebuffer,
    ssr_texture: glow::Texture,
    /// The two buffers the accumulated picture is passed between: one holds what has been
    /// gathered so far, the other takes the new average. They swap each frame, since a
    /// shader cannot read the buffer it is writing.
    accumulation_framebuffers: [glow::Framebuffer; 2],
    accumulation_textures: [glow::Texture; 2],
    depth: glow::Renderbuffer,
    ldr_framebuffer: glow::Framebuffer,
    ldr_texture: glow::Texture,
    bloom_framebuffers: [glow::Framebuffer; 2],
    bloom_textures: [glow::Texture; 2],
}

impl PostTargets {
    /// The post-processing targets at the viewport's size.
    ///
    /// # Safety
    /// `gl`'s context must be current on this thread.
    unsafe fn new(gl: &glow::Context, width: i32, height: i32) -> Result<Self, String> {
        // SAFETY: The caller keeps `gl`'s context current, and every object used here was made
        // in it just above.
        unsafe {
            let hdr_texture = make_texture(
                gl,
                glow::RGBA16F as i32,
                width,
                height,
                glow::RGBA,
                glow::FLOAT,
                glow::LINEAR,
            )?;
            let depth = gl
                .create_renderbuffer()
                .map_err(|e| format!("could not create a renderbuffer: {e}"))?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, width, height);

            let hdr_framebuffer = gl
                .create_framebuffer()
                .map_err(|e| format!("could not create a framebuffer: {e}"))?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(hdr_framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(hdr_texture),
                0,
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth),
            );
            let reflect_texture = make_texture(
                gl,
                glow::RGBA16F as i32,
                width,
                height,
                glow::RGBA,
                glow::FLOAT,
                glow::LINEAR,
            )?;
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT1,
                glow::TEXTURE_2D,
                Some(reflect_texture),
                0,
            );
            gl.draw_buffers(&[glow::COLOR_ATTACHMENT0, glow::COLOR_ATTACHMENT1]);
            check_framebuffer(gl, "the scene framebuffer")?;

            let (ssr_framebuffer, ssr_texture) =
                make_color_target(gl, glow::RGBA16F as i32, width, height)?;
            let first_accumulation = make_color_target(gl, glow::RGBA8 as i32, width, height)?;
            let second_accumulation = make_color_target(gl, glow::RGBA8 as i32, width, height)?;

            let (ldr_framebuffer, ldr_texture) =
                make_color_target(gl, glow::RGBA8 as i32, width, height)?;
            // The same depth the scene was drawn with, so the overlay drawn into this
            // framebuffer afterwards is occluded by the geometry the same way.
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(ldr_framebuffer));
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth),
            );
            check_framebuffer(gl, "the composite framebuffer")?;

            let bloom_width = (width / 2).max(1);
            let bloom_height = (height / 2).max(1);
            let first = make_color_target(gl, glow::RGBA16F as i32, bloom_width, bloom_height)?;
            let second = make_color_target(gl, glow::RGBA16F as i32, bloom_width, bloom_height)?;

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            Ok(Self {
                width,
                height,
                bloom_width,
                bloom_height,
                hdr_framebuffer,
                hdr_texture,
                reflect_texture,
                ssr_framebuffer,
                ssr_texture,
                accumulation_framebuffers: [first_accumulation.0, second_accumulation.0],
                accumulation_textures: [first_accumulation.1, second_accumulation.1],
                depth,
                ldr_framebuffer,
                ldr_texture,
                bloom_framebuffers: [first.0, second.0],
                bloom_textures: [first.1, second.1],
            })
        }
    }

    /// Deletes the framebuffers, textures and the depth buffer.
    ///
    /// # Safety
    /// `gl` must be the context that made the targets, current on this thread, and the
    /// targets must not be used afterwards.
    unsafe fn destroy(&self, gl: &glow::Context) {
        // SAFETY: The caller keeps `gl`'s context current and stops using the targets.
        unsafe {
            gl.delete_framebuffer(self.hdr_framebuffer);
            gl.delete_framebuffer(self.ldr_framebuffer);
            gl.delete_framebuffer(self.ssr_framebuffer);
            gl.delete_texture(self.hdr_texture);
            gl.delete_texture(self.ldr_texture);
            gl.delete_texture(self.reflect_texture);
            gl.delete_texture(self.ssr_texture);
            for framebuffer in self.accumulation_framebuffers {
                gl.delete_framebuffer(framebuffer);
            }
            for texture in self.accumulation_textures {
                gl.delete_texture(texture);
            }
            gl.delete_renderbuffer(self.depth);
            for framebuffer in self.bloom_framebuffers {
                gl.delete_framebuffer(framebuffer);
            }
            for texture in self.bloom_textures {
                gl.delete_texture(texture);
            }
        }
    }
}

/// A framebuffer with one colour attachment of the given format.
///
/// # Safety
/// `gl`'s context must be current on this thread.
unsafe fn make_color_target(
    gl: &glow::Context,
    internal_format: i32,
    width: i32,
    height: i32,
) -> Result<(glow::Framebuffer, glow::Texture), String> {
    // SAFETY: The caller keeps `gl`'s context current, and the texture attached is the one
    // made just above.
    unsafe {
        // The type has to suit the internal format on GLES: bytes for RGBA8, floats for the
        // half-float targets. The desktop accepts either.
        let kind = if internal_format == glow::RGBA8 as i32 {
            glow::UNSIGNED_BYTE
        } else {
            glow::FLOAT
        };
        let texture = make_texture(
            gl,
            internal_format,
            width,
            height,
            glow::RGBA,
            kind,
            glow::LINEAR,
        )?;
        let framebuffer = gl
            .create_framebuffer()
            .map_err(|e| format!("could not create a framebuffer: {e}"))?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
        check_framebuffer(gl, "a post-processing framebuffer")?;
        Ok((framebuffer, texture))
    }
}

/* ========== small GL helpers ========== */

/// An empty texture of the given format, clamped at its edges.
///
/// # Safety
/// `gl`'s context must be current on this thread.
unsafe fn make_texture(
    gl: &glow::Context,
    internal_format: i32,
    width: i32,
    height: i32,
    format: u32,
    kind: u32,
    filter: u32,
) -> Result<glow::Texture, String> {
    // SAFETY: The caller keeps `gl`'s context current. `tex_image_2d` gets no pixels, so it
    // only allocates the storage and reads no memory of ours.
    unsafe {
        let texture = gl
            .create_texture()
            .map_err(|e| format!("could not create a texture: {e}"))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            internal_format,
            width,
            height,
            0,
            format,
            kind,
            glow::PixelUnpackData::Slice(None),
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter as i32);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        Ok(texture)
    }
}

/// A framebuffer holding one channel, which is all an occlusion factor needs.
///
/// # Safety
/// `gl`'s context must be current on this thread.
unsafe fn make_single_channel_target(
    gl: &glow::Context,
    width: i32,
    height: i32,
) -> Result<(glow::Framebuffer, glow::Texture), String> {
    // SAFETY: The caller keeps `gl`'s context current, and the texture attached is the one
    // made just above.
    unsafe {
        let texture = make_texture(
            gl,
            glow::R8 as i32,
            width,
            height,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::LINEAR,
        )?;
        let framebuffer = gl
            .create_framebuffer()
            .map_err(|e| format!("could not create a framebuffer: {e}"))?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
        check_framebuffer(gl, "an occlusion framebuffer")?;
        Ok((framebuffer, texture))
    }
}

/// Whether the bound framebuffer can be drawn into, as an error naming `what` if not.
///
/// # Safety
/// `gl`'s context must be current on this thread.
unsafe fn check_framebuffer(gl: &glow::Context, what: &str) -> Result<(), String> {
    // SAFETY: The caller keeps `gl`'s context current; this only reads a status.
    let status = unsafe { gl.check_framebuffer_status(glow::FRAMEBUFFER) };
    if status == glow::FRAMEBUFFER_COMPLETE {
        Ok(())
    } else {
        Err(format!("{what} is not usable ({status:#x})"))
    }
}

fn bind_texture(gl: &glow::Context, unit: u32, texture: glow::Texture) {
    // SAFETY: `gl` is the renderer's context, current while it draws (see `Renderer::new`),
    // and `texture` is one it made.
    unsafe {
        gl.active_texture(glow::TEXTURE0 + unit);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    }
}

/// The directions ambient occlusion samples in, in a hemisphere about the normal.
///
/// Weighted towards the centre, so most samples land near the pixel and pick up the
/// creases, with a few further out for the wider shape.
fn make_kernel() -> Vec<f32> {
    let mut kernel = Vec::with_capacity(shaders::SSAO_KERNEL_SIZE * 3);
    let mut random = Random::new(0x5eed_1234_9abc_def0);

    for i in 0..shaders::SSAO_KERNEL_SIZE {
        let mut sample = [
            random.next_f32() * 2.0 - 1.0,
            random.next_f32() * 2.0 - 1.0,
            random.next_f32(),
        ];
        let length = (sample[0] * sample[0] + sample[1] * sample[1] + sample[2] * sample[2]).sqrt();
        if length > 0.0 {
            for component in &mut sample {
                *component /= length;
            }
        }

        let t = i as f32 / shaders::SSAO_KERNEL_SIZE as f32;
        let scale = 0.1 + 0.9 * t * t;
        for component in sample {
            kernel.push(component * random.next_f32() * scale);
        }
    }
    kernel
}

/// One material as a GL texture.
///
/// Mipmapped and filtered: a level is full of surfaces seen at a glancing angle, and an
/// unmipmapped texture on one of those is a field of aliasing rather than a wall.
///
/// # Safety
/// `gl`'s context must be current on this thread.
unsafe fn make_material_texture(
    gl: &glow::Context,
    material: &map_model::material::Material,
) -> Result<glow::Texture, String> {
    let pixels = material
        .pixels()
        .ok_or_else(|| "the image did not decode".to_string())?;
    // SAFETY: The caller keeps `gl`'s context current. The slice is bounds-checked here, and
    // `make_rgba_texture` checks its length against the size again.
    unsafe {
        make_rgba_texture(
            gl,
            material.width,
            material.height,
            &pixels[..(material.width * material.height * 4) as usize],
        )
    }
}

/// A mipmapped, repeating RGBA texture from `width * height` pixels of four bytes.
///
/// # Safety
/// `gl`'s context must be current on this thread. The pixel count is checked here.
unsafe fn make_rgba_texture(
    gl: &glow::Context,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<glow::Texture, String> {
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) * 4 != pixels.len() as u64
    {
        return Err("the image has no pixels".to_string());
    }

    // SAFETY: The caller keeps `gl`'s context current. `pixels` holds exactly
    // `width * height` RGBA bytes, checked above, and RGBA rows need no padding at any
    // unpack alignment up to 4 (egui sets 1, `make_noise_texture` puts back 4).
    unsafe {
        let texture = gl
            .create_texture()
            .map_err(|e| format!("could not create a texture: {e}"))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            width as i32,
            height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(pixels)),
        );
        gl.generate_mipmap(glow::TEXTURE_2D);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        // A material tiles across a face, which is the whole point of the texel
        // coordinates the .map format stores.
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        Ok(texture)
    }
}

/// One white pixel, for wherever a missing pass would otherwise leave nothing bound.
///
/// # Safety
/// `gl`'s context must be current on this thread.
unsafe fn make_white_texture(gl: &glow::Context) -> Result<glow::Texture, String> {
    // SAFETY: The caller keeps `gl`'s context current, and the slice is the one RGBA pixel
    // a 1 by 1 texture reads.
    unsafe {
        let texture = gl
            .create_texture()
            .map_err(|e| format!("could not create a texture: {e}"))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            1,
            1,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&[255, 255, 255, 255])),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        Ok(texture)
    }
}

/// A small tiled texture of random directions, to rotate the kernel per pixel.
///
/// # Safety
/// `gl`'s context must be current on this thread.
unsafe fn make_noise_texture(gl: &glow::Context) -> Result<glow::Texture, String> {
    let mut random = Random::new(0xabcd_ef01_2345_6789);
    let mut pixels = Vec::with_capacity(4 * 4 * 3);
    for _ in 0..16 {
        // Rotations about the normal, so the third component is not used.
        pixels.push((random.next_f32() * 255.0) as u8);
        pixels.push((random.next_f32() * 255.0) as u8);
        pixels.push(128);
    }

    // SAFETY: The caller keeps `gl`'s context current. `pixels` is 4 by 4 RGB bytes, and
    // with the unpack alignment at 1 for the upload GL reads those 48 bytes and no more.
    unsafe {
        let texture = gl
            .create_texture()
            .map_err(|e| format!("could not create a texture: {e}"))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        // Three bytes per pixel, which is not the four GL assumes by default.
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB8 as i32,
            4,
            4,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&pixels)),
        );
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        Ok(texture)
    }
}

/// A small deterministic generator, so the sampling pattern is the same every run.
///
/// Nothing here needs randomness that anyone could rely on; it needs a spread of
/// directions that does not repeat, and a run that looks the same twice.
struct Random(u64);

impl Random {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_f32(&mut self) -> f32 {
        // xorshift64*, which is short and has no visible structure at this size.
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let value = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (value >> 40) as f32 / (1u32 << 24) as f32
    }
}

/// Reinterprets a slice of floats as the bytes GL wants.
fn bytes_of(data: &[f32]) -> &[u8] {
    // SAFETY: Every byte of an `f32` is a valid `u8`, a `u8` needs no alignment, and
    // `size_of_val` is the length of `data` in bytes. The result borrows `data`, so it
    // cannot outlive it.
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data)) }
}

/// Whether the context is GLES (or WebGL 2): no polygon line mode, no texture border, no
/// point size switch, and shaders in GLSL ES.
fn embedded(gl: &glow::Context) -> bool {
    gl.version().is_embedded
}

/// A shader written as GLSL 1.50 in the version GLES 3.0 and WebGL 2 read: `#version 300
/// es` with the precisions GLSL ES wants said. The bodies are written to be valid in both,
/// without the implicit int-to-float conversions GLSL 1.50 allows and GLSL ES does not.
fn for_context<'a>(gl: &glow::Context, source: &'a str) -> std::borrow::Cow<'a, str> {
    const DESKTOP: &str = "#version 150 core";
    match source.strip_prefix(DESKTOP) {
        Some(body) if embedded(gl) => {
            // GLSL ES wants every output of a shader with several placed by `layout`, where
            // the desktop binds them by name before linking (FRAGMENT_OUTPUTS).
            let body: String = body
                .lines()
                .map(|line| {
                    let output = line
                        .trim()
                        .strip_prefix("out vec4 ")
                        .and_then(|rest| rest.strip_suffix(';'));
                    match output.and_then(|name| {
                        FRAGMENT_OUTPUTS
                            .iter()
                            .position(|wanted| *wanted == name.trim())
                    }) {
                        Some(location) => {
                            format!("layout(location = {location}) {}\n", line.trim())
                        }
                        None => format!("{line}\n"),
                    }
                })
                .collect();
            format!(
                "#version 300 es\nprecision highp float;\nprecision highp int;\nprecision highp sampler2D;\nprecision highp sampler2DShadow;\nprecision highp samplerCube;{body}"
            )
            .into()
        }
        _ => source.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_occlusion_kernel_stays_in_its_hemisphere() {
        let kernel = make_kernel();
        assert_eq!(kernel.len(), shaders::SSAO_KERNEL_SIZE * 3);

        for sample in kernel.chunks(3) {
            // Every direction points along the normal, which is the third component, or
            // the sample would look for occluders behind the surface.
            assert!(sample[2] >= 0.0, "{sample:?}");
            // And none reaches further than the radius it is scaled by.
            let length =
                (sample[0] * sample[0] + sample[1] * sample[1] + sample[2] * sample[2]).sqrt();
            assert!(length <= 1.0, "{length}");
        }
    }

    #[test]
    fn the_occlusion_kernel_favours_the_near_samples() {
        // The scale grows quadratically along the kernel, so the first samples sit much
        // closer to the pixel than the last: that is what makes creases shade before whole
        // rooms do.
        let kernel = make_kernel();
        let length = |i: usize| {
            let s = &kernel[i * 3..i * 3 + 3];
            (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
        };

        let first: f32 = (0..4).map(length).sum::<f32>() / 4.0;
        let last: f32 = (shaders::SSAO_KERNEL_SIZE - 4..shaders::SSAO_KERNEL_SIZE)
            .map(length)
            .sum::<f32>()
            / 4.0;
        assert!(last > first * 2.0, "first {first}, last {last}");
    }

    #[test]
    fn the_generator_repeats_itself_run_to_run() {
        // The sampling pattern is baked into the picture, so it must not change between
        // runs; two generators from the same seed give the same sequence.
        let mut a = Random::new(7);
        let mut b = Random::new(7);
        for _ in 0..32 {
            assert_eq!(a.next_f32(), b.next_f32());
        }
    }

    #[test]
    fn the_generator_stays_between_zero_and_one() {
        let mut random = Random::new(11);
        for _ in 0..4096 {
            let value = random.next_f32();
            assert!((0.0..1.0).contains(&value), "{value}");
        }
    }

    #[test]
    fn the_vertex_layout_adds_up() {
        let total: i32 = ATTRIBUTES.iter().map(|(_, components, _)| components).sum();
        assert_eq!(total as usize, FLOATS_PER_VERTEX);

        // Each attribute starts where the last one ended.
        let mut expected = 0;
        for (_, components, offset) in ATTRIBUTES {
            assert_eq!(offset, expected);
            expected += components;
        }
    }
}
