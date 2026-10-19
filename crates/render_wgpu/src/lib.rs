//! A retained, wgpu-based renderer for the 3D view, designed after Bevy's: the host owns
//! the device and queue, the renderer keeps a render world fed by deltas from
//! `editor_core` (see [`editor_core::render_scene`]), and draws through a fixed set of
//! passes into targets of its own. See `docs/renderer-wgpu.md` for the architecture and
//! the phase plan; this is phase 3.
//!
//! Passes, in order (material mode; solid mode skips 1, 2, 3, 5, 7 and 9):
//! 1. image-based lighting, only when the sky or sun changed: sky cube, GGX prefilter,
//!    spherical harmonics ([`ibl`]);
//! 2. cascaded sun shadow maps, each cascade only when it changed ([`shadows`]);
//! 3. cube shadow maps of the most important shadowed light entities, each face only
//!    when it changed; the lights binned into clusters on the CPU ([`lights`]);
//! 4. depth + object-id + normal + motion prepass (opaque and masked; reverse-Z float
//!    depth; each surface's motion since last frame, from a moving chunk's copy of its
//!    vertices at their previous positions), kept from the last frame when only shading
//!    changed;
//! 5. GTAO (full resolution up to 2.5 megapixels), then a depth-aware denoise;
//! 6. sky where the prepass left the far plane, forward PBR (depth-equal, each pixel
//!    shaded once: sun with cascades and contact shadows, clustered light entities,
//!    image-based light), then blended surfaces; also the reflectivity for SSR;
//! 7. screen-space reflections at half resolution;
//! 8. temporal antialiasing into a history reprojected by camera and object motion,
//!    with a per-pixel depth and sample count against disocclusion ([`taa`]);
//! 9. mip-chain bloom;
//! 10. resolve: reflections, bloom, exposure, tone map (ACES, AgX, Khronos PBR Neutral
//!     or TonyMcMapface), sRGB, background, selection outline, dithering;
//! 11. point-entity sprites, then the overlay: guides and handles against the scene
//!     depth, then faintly through it;
//! 12. FXAA, only without the temporal filter (and a copy for an offline sample).
//!
//! Wireframe skips all but the overlay: edges straight into the output.
//!
//! Nothing is drawn when nothing changed: the host keeps the last picture (see
//! `heron-brush`'s view) once the temporal filter has converged, and inside a frame the
//! environment and the shadow maps are cached across frames.

#![allow(clippy::too_many_arguments)]

mod ibl;
mod lights;
pub mod present;
pub mod scene;
mod shadows;
mod taa;
mod textures;
mod timer;

use std::hash::{Hash, Hasher};
use web_time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use geometry_math::bbox::Bbox3d;
use geometry_math::mat::{invert, Mat4x4d};
use geometry_math::vec::Vec3d;
use render::camera3d::Camera3d;
use render::RenderSettings;

pub use scene::{Phase, RenderScene, TextureStats, UploadStats, DEFAULT_TEXTURE_BUDGET};

/// The format of [`Renderer::output`]: display-encoded (the sRGB curve) 8-bit RGBA.
pub const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const ID_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Uint;
const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
/// Each surface's motion since last frame, in world units (zero where nothing moved).
const MOTION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Occlusion and view depth.
const AO_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
/// What screen-space reflections may replace: specular weight and roughness.
const REFLECTIVITY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// At most this many bloom mips.
const BLOOM_LEVELS: u32 = 6;

/// The TonyMcMapface LUT (48³, RGB9E5), from <https://github.com/h3r2tic/tony-mc-mapface>,
/// MIT or Apache-2.0; see `assets/README.md`.
const TONY_MC_MAPFACE: &[u8] = include_bytes!("../assets/tony_mc_mapface_48.rgb9e5");

/// How the scene is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    /// Triangle edges in one colour, no depth.
    Wireframe,
    /// Flat: material colour with a gentle directional term, no lighting.
    Solid,
    /// Physically based, lit by the sun and the sky.
    Material,
}

/// The curve that maps HDR light to the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tonemapper {
    /// The fitted ACES curve the GL renderer uses.
    Aces,
    /// Troy Sobotka's AgX, as Blender has it: graceful highlights, no hue skews.
    #[default]
    Agx,
    /// Khronos PBR Neutral: base colours as authored, highlights compressed.
    KhronosNeutral,
    /// Tomasz Stachowiak's TonyMcMapface (a LUT).
    TonyMcMapface,
}

/// What one frame shows.
pub struct ViewParams<'a> {
    /// The camera.
    pub camera: &'a Camera3d,
    /// The output size in device pixels.
    pub size: (u32, u32),
    /// Colours, sun, sky, exposure, and which effects are on.
    pub settings: &'a RenderSettings,
    /// Wireframe, solid or material.
    pub mode: DisplayMode,
    /// The tone map.
    pub tonemapper: Tonemapper,
    /// Tool handles.
    pub points: &'a [(Vec3d, [f32; 4])],
    /// Guide lines, as pairs.
    pub guides: &'a [(Vec3d, [f32; 4])],
    /// Draws the world axes across these bounds when the settings ask for axes.
    pub world_bounds: Option<&'a Bbox3d>,
    /// The width of the selection outline in pixels; 0 for none.
    pub outline_width: u32,
    /// For an offline render, which sample this is: the projection is jittered within
    /// the pixel (Halton 2, 3) and FXAA is left to the averaging. `None` for the view.
    pub sample: Option<u32>,
    /// Light entities light the scene (point and spot lights, material mode only).
    pub lights: bool,
    /// Nothing changed since the last frame: this one only refines the temporal filter's
    /// picture (see [`Renderer::converged`]). False after any change the host drew for.
    pub continuing: bool,
}

/// What the last frame did.
#[derive(Debug, Default, Clone, Copy)]
pub struct FrameStats {
    /// Draw calls issued (scene and shadow passes).
    pub draws: u32,
    /// Geometry draws into sun shadow cascades. Included in `draws`.
    pub sun_shadow_draws: u32,
    /// Geometry draws into local-light shadow cube faces. Included in `draws`.
    pub light_shadow_draws: u32,
    /// Geometry draws into the depth/normal/id prepass. Included in `draws`.
    pub prepass_draws: u32,
    /// Chunks drawn.
    pub chunks_drawn: u32,
    /// Chunks skipped by frustum culling.
    pub chunks_culled: u32,
    /// Shadow cascades drawn this frame (the others were cached).
    pub cascades_drawn: u32,
    /// Light entities in view (in the clusters).
    pub lights: u32,
    /// Cube faces of light shadows drawn this frame (the others were cached).
    pub light_faces_drawn: u32,
    /// The depth, ids, normals and ambient occlusion of the last frame were still right
    /// (only object states or shading changed), so the prepass and GTAO were skipped.
    pub prepass_cached: bool,
    /// The environment lighting was rebuilt this frame.
    pub environment_rebuilt: bool,
    /// CPU time to record and submit the frame.
    pub encode: Duration,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FrameUniform {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    inv_proj: [[f32; 4]; 4],
    cascades: [[[f32; 4]; 4]; 4],
    eye: [f32; 4],
    light_dir: [f32; 4],
    light_color: [f32; 4],
    sky: [f32; 4],
    horizon: [f32; 4],
    ground: [f32; 4],
    selected_color: [f32; 4],
    locked_color: [f32; 4],
    background: [f32; 4],
    wire_color: [f32; 4],
    params0: [f32; 4],
    params1: [f32; 4],
    viewport: [f32; 4],
    sun_color: [f32; 4],
    forward: [f32; 4],
    cascade_splits: [f32; 4],
    cascade_texel: [f32; 4],
    cascade_bias: [f32; 4],
    shadow: [f32; 4],
    effects: [f32; 4],
    post: [f32; 4],
    misc: [f32; 4],
    dof: [f32; 4],
    lights: [f32; 4],
    cluster: [f32; 4],
    point_shadow: [f32; 4],
    contact: [f32; 4],
    reflection: [f32; 4],
    taa: [f32; 4],
    view_proj_unjittered: [[f32; 4]; 4],
    prev_view_proj: [[f32; 4]; 4],
    ao: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OverlayVertex {
    position: [f32; 3],
    color: [f32; 4],
}

/// Everything sized to the view.
struct Targets {
    size: (u32, u32),
    depth_texture: wgpu::Texture,
    depth: wgpu::TextureView,
    /// In the browser, a copy of the prepass depth for the lighting to read, so the
    /// forward pass can attach the depth itself writable: Safari's WebGPU (26) loses the
    /// prepass depth in a pass that attaches it read-only, the forward pass then tests
    /// against nothing, and the view stays black.
    depth_copy: Option<(wgpu::Texture, wgpu::TextureView)>,
    ids: wgpu::TextureView,
    normals: wgpu::TextureView,
    motion: wgpu::TextureView,
    hdr: wgpu::TextureView,
    reflectivity: wgpu::TextureView,
    /// Screen-space reflections, half resolution, and the group that draws them.
    ssr: wgpu::TextureView,
    ssr_group: wgpu::BindGroup,
    ldr: wgpu::TextureView,
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
    resolve_group: wgpu::BindGroup,
    fxaa_group: wgpu::BindGroup,
    ao_raw: wgpu::TextureView,
    ao: wgpu::TextureView,
    gtao_group: wgpu::BindGroup,
    denoise_group: wgpu::BindGroup,
    /// One view per bloom mip, and per mip a group sampling it; the HDR picture's group.
    bloom_mips: Vec<wgpu::TextureView>,
    bloom_groups: Vec<wgpu::BindGroup>,
    bloom_hdr_group: wgpu::BindGroup,
    /// The temporal filter's history, made the first time it is used.
    taa: Option<taa::TaaTargets>,
    /// Bumped on every reallocation, so groups that hold these views are rebuilt.
    generation: u64,
}

struct Pipelines {
    prepass: wgpu::RenderPipeline,
    forward: wgpu::RenderPipeline,
    blended: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
    wire: wgpu::RenderPipeline,
    resolve: wgpu::RenderPipeline,
    fxaa: wgpu::RenderPipeline,
    lines: [wgpu::RenderPipeline; 2],
    points: [wgpu::RenderPipeline; 2],
    sprites: wgpu::RenderPipeline,
    shadow_opaque: wgpu::RenderPipeline,
    shadow_single: wgpu::RenderPipeline,
    shadow_masked: wgpu::RenderPipeline,
    cube_opaque: wgpu::RenderPipeline,
    cube_single: wgpu::RenderPipeline,
    cube_masked: wgpu::RenderPipeline,
    gtao: wgpu::RenderPipeline,
    denoise: wgpu::RenderPipeline,
    ssr: wgpu::RenderPipeline,
    ssr_layout: wgpu::BindGroupLayout,
    taa: wgpu::RenderPipeline,
    taa_layout: wgpu::BindGroupLayout,
    bloom_first: wgpu::RenderPipeline,
    bloom_down: wgpu::RenderPipeline,
    bloom_up: wgpu::RenderPipeline,
    resolve_layout: wgpu::BindGroupLayout,
    fxaa_layout: wgpu::BindGroupLayout,
    ao_layout: wgpu::BindGroupLayout,
    bloom_layout: wgpu::BindGroupLayout,
    lighting_layout: wgpu::BindGroupLayout,
    cascade_layout: wgpu::BindGroupLayout,
    fade_groups: [wgpu::BindGroup; 2],
    _fade_buffers: [wgpu::Buffer; 2],
    accumulate: wgpu::RenderPipeline,
    accumulated: wgpu::RenderPipeline,
    accumulate_layout: wgpu::BindGroupLayout,
}

/// The running average of an offline render, at the output's size.
struct Accumulation {
    size: (u32, u32),
    view: wgpu::TextureView,
    /// Reads the sample (the picture before FXAA) into the average.
    from_sample: wgpu::BindGroup,
    /// Reads the average into the output.
    from_average: wgpu::BindGroup,
    samples: u32,
}

/// The renderer: a retained scene, pipelines and per-size targets on the host's device.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// The render world. Feed it through [`editor_core::render_scene::SceneTracker`].
    pub scene: RenderScene,
    frame_buffer: wgpu::Buffer,
    frame_layout: wgpu::BindGroupLayout,
    frame_group: Option<(u64, wgpu::BindGroup)>,
    pipelines: Pipelines,
    targets: Option<Targets>,
    target_generation: u64,
    overlay_lines: Option<wgpu::Buffer>,
    overlay_points: Option<wgpu::Buffer>,
    /// The overlay's vertices, rebuilt every frame in vectors kept across frames.
    overlay_scratch: (Vec<OverlayVertex>, Vec<OverlayVertex>),
    linear_sampler: wgpu::Sampler,
    shadow_sampler: wgpu::Sampler,
    shadows: Option<shadows::Shadows>,
    /// Stands in for the shadow maps while shadows are off.
    no_shadows: wgpu::TextureView,
    shadow_generation: u64,
    ibl: ibl::Ibl,
    _tony: wgpu::Texture,
    tony_view: wgpu::TextureView,
    /// Light entities: table, clusters, cube shadows.
    lights: lights::Lights,
    /// The forward pass's lighting group and the (target, shadow, lights) generations it
    /// is for.
    lighting_group: Option<((u64, u64, u64), wgpu::BindGroup)>,
    timer: Option<timer::GpuTimer>,
    profiling: bool,
    accumulation: Option<Accumulation>,
    /// The temporal filter's state across frames.
    taa: taa::TaaState,
    /// What the depth, id, normal and occlusion targets hold (see `prepass_key`).
    prepass_key: Option<u64>,
    /// The scene's sync epoch and geometry generation at the last frame: whether it was
    /// synced, and whether its geometry changed, since.
    rendered_epoch: u64,
    rendered_generation: u64,
    /// The base materials the last frame's visible draws used: what the next frame's
    /// streaming step asks for.
    used_materials: Vec<usize>,
    /// A texture became resident or was evicted in the last frame, so the picture is not
    /// final however converged the temporal filter is.
    textures_changed: bool,
    /// What the last frame did.
    pub stats: FrameStats,
}

const COMMON: &str = include_str!("shaders/common.wgsl");

fn module(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(format!("{COMMON}\n{source}").into()),
    })
}

fn texture_entry(binding: u32, sample_type: wgpu::TextureSampleType) -> wgpu::BindGroupLayoutEntry {
    texture_entry_dim(binding, sample_type, wgpu::TextureViewDimension::D2)
}

fn texture_entry_dim(binding: u32, sample_type: wgpu::TextureSampleType, view_dimension: wgpu::TextureViewDimension) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type, view_dimension, multisampled: false }, count: None }
}

fn sampler_entry(binding: u32, kind: wgpu::SamplerBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(kind), count: None }
}

fn depth_state(compare: wgpu::CompareFunction, write: bool) -> Option<wgpu::DepthStencilState> {
    Some(wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(write),
        depth_compare: Some(compare),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    })
}

struct PipelineSpec<'a> {
    label: &'a str,
    layout: &'a wgpu::PipelineLayout,
    module: &'a wgpu::ShaderModule,
    vs: &'a str,
    fs: Option<&'a str>,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    topology: wgpu::PrimitiveTopology,
    targets: &'a [Option<wgpu::ColorTargetState>],
    depth: Option<wgpu::DepthStencilState>,
}

fn pipeline(device: &wgpu::Device, spec: PipelineSpec<'_>) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: Some(spec.layout),
        vertex: wgpu::VertexState {
            module: spec.module,
            entry_point: Some(spec.vs),
            compilation_options: Default::default(),
            buffers: spec.buffers,
        },
        primitive: wgpu::PrimitiveState { topology: spec.topology, cull_mode: None, ..Default::default() },
        depth_stencil: spec.depth,
        multisample: wgpu::MultisampleState::default(),
        fragment: spec.fs.map(|fs| wgpu::FragmentState { module: spec.module, entry_point: Some(fs), compilation_options: Default::default(), targets: spec.targets }),
        multiview_mask: None,
        cache: None,
    })
}

fn target(format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>) -> Option<wgpu::ColorTargetState> {
    Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })
}

fn to_f32(m: &Mat4x4d) -> [[f32; 4]; 4] {
    let mut out = [[0f32; 4]; 4];
    for (column, out_column) in out.iter_mut().enumerate() {
        for (row, value) in out_column.iter_mut().enumerate() {
            *value = m[column][row] as f32;
        }
    }
    out
}

/// The camera's projection for wgpu: the GL projection remapped to a reversed 0..1 depth
/// range (near 1, far 0), which spreads float depth precision evenly.
pub fn projection(camera: &Camera3d, width: u32, height: u32) -> Mat4x4d {
    let remap = Mat4x4d::from_rows([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, -0.5, 0.5], [0.0, 0.0, 0.0, 1.0]]);
    remap * camera.projection_matrix(width as i32, height.max(1) as i32)
}

/// The camera's view-projection for wgpu (see [`projection`]).
pub fn view_projection(camera: &Camera3d, width: u32, height: u32) -> Mat4x4d {
    projection(camera, width, height) * camera.view_matrix()
}

/// The six frustum planes of a view-projection (column-major), each `ax + by + cz + d`,
/// positive inside: the side planes, then `z >= 0` and `z <= w`. Holds for the reversed
/// camera depth and for the shadow cascades' 0..1 depth alike.
fn frustum_planes(m: &[[f32; 4]; 4]) -> [[f32; 4]; 6] {
    let row = |r: usize| [m[0][r], m[1][r], m[2][r], m[3][r]];
    let (r0, r1, r2, r3) = (row(0), row(1), row(2), row(3));
    let add = |a: [f32; 4], b: [f32; 4]| [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]];
    let sub = |a: [f32; 4], b: [f32; 4]| [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]];
    [add(r3, r0), sub(r3, r0), add(r3, r1), sub(r3, r1), r2, sub(r3, r2)]
}

fn outside(planes: &[[f32; 4]; 6], (lo, hi): ([f32; 3], [f32; 3])) -> bool {
    planes.iter().any(|p| {
        // The box corner furthest along the plane's normal.
        let x = if p[0] >= 0.0 { hi[0] } else { lo[0] };
        let y = if p[1] >= 0.0 { hi[1] } else { lo[1] };
        let z = if p[2] >= 0.0 { hi[2] } else { lo[2] };
        p[0] * x + p[1] * y + p[2] * z + p[3] < 0.0
    })
}

fn sun_direction(settings: &RenderSettings) -> [f32; 3] {
    let elevation = settings.sun_elevation.to_radians();
    let azimuth = settings.sun_azimuth.to_radians();
    let to_sun = [elevation.cos() * azimuth.cos(), elevation.cos() * azimuth.sin(), elevation.sin()];
    let length = (to_sun[0] * to_sun[0] + to_sun[1] * to_sun[1] + to_sun[2] * to_sun[2]).sqrt().max(1e-6);
    [-to_sun[0] / length, -to_sun[1] / length, -to_sun[2] / length]
}

/// The side of a shadow cascade: half the preference's (rounded to a power of two), so
/// the four cascades hold as many texels, and as much memory, as the GL renderer's one
/// map over the whole level (16 MB at the default 2048) -- and spend them where the
/// camera looks.
fn shadow_size(settings: &RenderSettings) -> u32 {
    (settings.shadow_map_size.clamp(512, 8192).next_power_of_two() / 2).max(256)
}

/// The side of a light's shadow cube face: a quarter of a sun cascade's (256 at the
/// default map size), so eight shadowed lights cost about as much memory as one cascade.
fn light_shadow_size(settings: &RenderSettings) -> u32 {
    (shadow_size(settings) / 4).clamp(128, 1024)
}

/// Ambient occlusion at full resolution up to this many pixels (a 4K view goes to half):
/// at half resolution creases a pixel or two wide, the corners of trim and the gaps of
/// quoins, fall between occlusion texels and the picture loses the definition GL's
/// full-resolution SSAO gives it.
const AO_FULL_RESOLUTION_PIXELS: u32 = 2_500_000;

fn ao_full_resolution((width, height): (u32, u32)) -> bool {
    width * height <= AO_FULL_RESOLUTION_PIXELS
}

/// How gradually a surface starts to bloom past the threshold (the GL renderer's knee).
const BLOOM_KNEE: f32 = 0.7;
/// The shadow filter radius in texels at the default softness (the GL renderer's).
const SHADOW_TAP_SPREAD: f32 = 1.6;
/// The bias preference's default, which stands for one unit of the normal offset.
const DEFAULT_SHADOW_BIAS: f32 = 0.0015;

fn v4(c: [f32; 3]) -> [f32; 4] {
    [c[0], c[1], c[2], 1.0]
}

impl Renderer {
    /// Sets the renderer up on the host's device. Needs nothing beyond WebGPU's defaults
    /// except storage buffers in the vertex stage (every native backend). Uses timestamp
    /// queries for profiling when the device has them.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        use wgpu::TextureSampleType as T;
        let scene = RenderScene::new(device, queue);
        let storage = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        };
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_wgpu frame"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                storage(1),
                storage(2),
            ],
        });
        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render_wgpu frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let fragment_storage = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        };
        let layout_of = |label: &str, entries: &[wgpu::BindGroupLayoutEntry]| device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some(label), entries });
        let filterable = T::Float { filterable: true };
        let unfilterable = T::Float { filterable: false };
        let resolve_layout = layout_of(
            "render_wgpu resolve",
            &[
                texture_entry(0, unfilterable),
                texture_entry(1, T::Uint),
                texture_entry(2, T::Depth),
                texture_entry(3, filterable),
                texture_entry_dim(4, filterable, wgpu::TextureViewDimension::D3),
                sampler_entry(5, wgpu::SamplerBindingType::Filtering),
                texture_entry(9, filterable),
            ],
        );
        let taa_layout = layout_of(
            "render_wgpu taa",
            &[
                texture_entry(0, unfilterable),
                texture_entry(1, filterable),
                texture_entry(2, T::Depth),
                texture_entry(3, filterable),
                sampler_entry(4, wgpu::SamplerBindingType::Filtering),
                texture_entry(5, unfilterable),
                texture_entry(6, unfilterable),
            ],
        );
        let ssr_layout = layout_of(
            "render_wgpu ssr",
            &[
                texture_entry(0, unfilterable),
                texture_entry(1, T::Depth),
                texture_entry(2, unfilterable),
                texture_entry(3, unfilterable),
                texture_entry_dim(4, filterable, wgpu::TextureViewDimension::Cube),
                sampler_entry(5, wgpu::SamplerBindingType::Filtering),
            ],
        );
        let fxaa_layout = layout_of("render_wgpu fxaa", &[texture_entry(6, filterable), sampler_entry(7, wgpu::SamplerBindingType::Filtering)]);
        let accumulate_layout = layout_of("render_wgpu accumulate", &[texture_entry(6, unfilterable), texture_entry(8, unfilterable)]);
        let ao_layout = layout_of("render_wgpu ao", &[texture_entry(0, T::Depth), texture_entry(1, unfilterable), texture_entry(2, unfilterable)]);
        let bloom_layout = layout_of("render_wgpu bloom", &[texture_entry(0, filterable), sampler_entry(1, wgpu::SamplerBindingType::Filtering)]);
        let lighting_layout = layout_of(
            "render_wgpu lighting",
            &[
                texture_entry_dim(0, T::Depth, wgpu::TextureViewDimension::D2Array),
                sampler_entry(1, wgpu::SamplerBindingType::Comparison),
                texture_entry_dim(2, filterable, wgpu::TextureViewDimension::Cube),
                texture_entry(3, unfilterable),
                texture_entry(4, filterable),
                sampler_entry(5, wgpu::SamplerBindingType::Filtering),
                texture_entry(6, unfilterable),
                fragment_storage(7),
                fragment_storage(8),
                fragment_storage(9),
                texture_entry_dim(10, T::Depth, wgpu::TextureViewDimension::CubeArray),
                texture_entry(11, T::Depth),
            ],
        );
        let cascade_layout = shadows::Shadows::layout(device);
        let fade_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_wgpu overlay fade"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let layout = |label: &str, groups: &[&wgpu::BindGroupLayout]| {
            let groups: Vec<Option<&wgpu::BindGroupLayout>> = std::iter::once(&frame_layout).chain(groups.iter().copied()).map(Some).collect();
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some(label), bind_group_layouts: &groups, immediate_size: 0 })
        };
        let scene_layout = layout("render_wgpu scene", &[&scene.texture_layout]);
        let forward_layout = layout("render_wgpu forward", &[&scene.texture_layout, &lighting_layout]);
        let shadow_layout = layout("render_wgpu shadow", &[&scene.texture_layout, &cascade_layout]);
        let resolve_pipeline_layout = layout("render_wgpu resolve", &[&resolve_layout]);
        let fxaa_pipeline_layout = layout("render_wgpu fxaa", &[&fxaa_layout]);
        let accumulate_pipeline_layout = layout("render_wgpu accumulate", &[&accumulate_layout]);
        let overlay_layout = layout("render_wgpu overlay", &[&fade_layout]);
        let ao_pipeline_layout = layout("render_wgpu ao", &[&ao_layout]);
        let bloom_pipeline_layout = layout("render_wgpu bloom", &[&bloom_layout]);
        let ssr_pipeline_layout = layout("render_wgpu ssr", &[&ssr_layout]);
        let taa_pipeline_layout = layout("render_wgpu taa", &[&taa_layout]);

        let scene_module = module(device, "render_wgpu scene", include_str!("shaders/scene.wgsl"));
        let shadow_module = module(device, "render_wgpu shadow", include_str!("shaders/shadow.wgsl"));
        let post_module = module(device, "render_wgpu post", include_str!("shaders/post.wgsl"));
        let overlay_module = module(device, "render_wgpu overlay", include_str!("shaders/overlay.wgsl"));
        let sprite_module = module(device, "render_wgpu sprite", include_str!("shaders/sprite.wgsl"));
        let ao_module = module(device, "render_wgpu ao", include_str!("shaders/ao.wgsl"));
        let bloom_module = module(device, "render_wgpu bloom", include_str!("shaders/bloom.wgsl"));
        let ibl_module = module(device, "render_wgpu ibl", include_str!("shaders/ibl.wgsl"));
        let ssr_module = module(device, "render_wgpu ssr", include_str!("shaders/ssr.wgsl"));
        let taa_module = module(device, "render_wgpu taa", include_str!("shaders/taa.wgsl"));

        let vertex_layout = [Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<scene::Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &scene::Vertex::ATTRIBUTES,
        })];
        // The prepass's second vertex buffer: last frame's positions, read from a copy of
        // a moving chunk's vertices or from the chunk's own buffer.
        let previous_attributes = wgpu::vertex_attr_array![5 => Float32x3];
        let prepass_layout = [
            vertex_layout[0].clone(),
            Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<scene::Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &previous_attributes }),
        ];
        let overlay_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        let line_layout = [Some(wgpu::VertexBufferLayout { array_stride: 28, step_mode: wgpu::VertexStepMode::Vertex, attributes: &overlay_attributes })];
        let point_layout = [Some(wgpu::VertexBufferLayout { array_stride: 28, step_mode: wgpu::VertexStepMode::Instance, attributes: &overlay_attributes })];
        let sprite_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
        let sprite_layout = [Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<scene::SpriteGpu>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &sprite_attributes })];
        let tri = wgpu::PrimitiveTopology::TriangleList;
        let alpha = Some(wgpu::BlendState::ALPHA_BLENDING);
        let additive = Some(wgpu::BlendState { color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add }, alpha: wgpu::BlendComponent::REPLACE });
        use wgpu::CompareFunction as C;

        let spec = |label, layout, module, vs, fs, buffers, topology, targets, depth| PipelineSpec { label, layout, module, vs, fs: Some(fs), buffers, topology, targets, depth };
        let output = [target(OUTPUT_FORMAT, None)];
        let output_alpha = [target(OUTPUT_FORMAT, alpha)];
        let hdr = [target(HDR_FORMAT, None)];
        let ao_target = [target(AO_FORMAT, None)];
        let prepass_targets = [target(ID_FORMAT, None), target(NORMAL_FORMAT, None), target(MOTION_FORMAT, None)];
        let prepass = pipeline(device, spec("prepass", &scene_layout, &scene_module, "vs_prepass", "fs_prepass", &prepass_layout, tri, &prepass_targets, depth_state(C::Greater, true)));
        // The forward passes also write the reflectivity for SSR; surfaces seen through
        // leave it alone (their own reflection is already blended in).
        let lit = [target(HDR_FORMAT, None), target(REFLECTIVITY_FORMAT, None)];
        let forward = pipeline(device, spec("forward", &forward_layout, &scene_module, "vs_main", "fs_forward", &vertex_layout, tri, &lit, depth_state(C::GreaterEqual, false)));
        let lit_alpha = [target(HDR_FORMAT, alpha), Some(wgpu::ColorTargetState { format: REFLECTIVITY_FORMAT, blend: None, write_mask: wgpu::ColorWrites::empty() })];
        let blended = pipeline(device, spec("blended", &forward_layout, &scene_module, "vs_main", "fs_forward", &vertex_layout, tri, &lit_alpha, depth_state(C::GreaterEqual, false)));
        let sky = pipeline(device, spec("sky", &scene_layout, &scene_module, "vs_sky", "fs_sky", &[], tri, &lit, depth_state(C::Equal, false)));
        let wire = pipeline(device, spec("wire", &scene_layout, &scene_module, "vs_main", "fs_wire", &vertex_layout, wgpu::PrimitiveTopology::LineList, &output, depth_state(C::Always, false)));
        let resolve = pipeline(device, spec("resolve", &resolve_pipeline_layout, &post_module, "vs_fullscreen", "fs_resolve", &[], tri, &output, None));
        let fxaa = pipeline(device, spec("fxaa", &fxaa_pipeline_layout, &post_module, "vs_fullscreen", "fs_fxaa", &[], tri, &output, None));
        // A running average: new = sample * c + old * (1 - c), c = 1 / samples so far.
        let average = Some(wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Constant, dst_factor: wgpu::BlendFactor::OneMinusConstant, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent::REPLACE,
        });
        let accumulate_target = [target(HDR_FORMAT, average)];
        let accumulate = pipeline(device, spec("accumulate", &accumulate_pipeline_layout, &post_module, "vs_fullscreen", "fs_accumulate", &[], tri, &accumulate_target, None));
        let accumulated = pipeline(device, spec("accumulated", &accumulate_pipeline_layout, &post_module, "vs_fullscreen", "fs_accumulated", &[], tri, &output, None));
        let lines = [C::GreaterEqual, C::Always].map(|c| pipeline(device, spec("overlay lines", &overlay_layout, &overlay_module, "vs_line", "fs_line", &line_layout, wgpu::PrimitiveTopology::LineList, &output_alpha, depth_state(c, false))));
        let points = [C::GreaterEqual, C::Always].map(|c| pipeline(device, spec("overlay points", &overlay_layout, &overlay_module, "vs_point", "fs_point", &point_layout, tri, &output_alpha, depth_state(c, false))));
        let sprites = pipeline(device, spec("sprites", &scene_layout, &sprite_module, "vs_sprite", "fs_sprite", &sprite_layout, tri, &output_alpha, depth_state(C::GreaterEqual, false)));
        let gtao = pipeline(device, spec("gtao", &ao_pipeline_layout, &ao_module, "vs_ao", "fs_gtao", &[], tri, &ao_target, None));
        let denoise = pipeline(device, spec("ao denoise", &ao_pipeline_layout, &ao_module, "vs_ao", "fs_denoise", &[], tri, &ao_target, None));
        let ssr = pipeline(device, spec("ssr", &ssr_pipeline_layout, &ssr_module, "vs_ssr", "fs_ssr", &[], tri, &hdr, None));
        let taa_targets = [target(HDR_FORMAT, None), target(taa::META_FORMAT, None)];
        let taa = pipeline(device, spec("taa", &taa_pipeline_layout, &taa_module, "vs_taa", "fs_taa", &[], tri, &taa_targets, None));
        let hdr_additive = [target(HDR_FORMAT, additive)];
        let bloom_first = pipeline(device, spec("bloom first", &bloom_pipeline_layout, &bloom_module, "vs_bloom", "fs_downsample_first", &[], tri, &hdr, None));
        let bloom_down = pipeline(device, spec("bloom down", &bloom_pipeline_layout, &bloom_module, "vs_bloom", "fs_downsample", &[], tri, &hdr, None));
        let bloom_up = pipeline(device, spec("bloom up", &bloom_pipeline_layout, &bloom_module, "vs_bloom", "fs_upsample", &[], tri, &hdr_additive, None));
        // Shadow casters: normal depth (near 0), slope-scaled bias against acne on
        // surfaces the sun grazes; the normal offset in the lookup does the rest.
        let shadow_depth = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(C::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState { constant: 0, slope_scale: 1.5, clamp: 0.0 },
        });
        let shadow_opaque = pipeline(device, PipelineSpec { label: "shadow opaque", layout: &shadow_layout, module: &shadow_module, vs: "vs_shadow", fs: None, buffers: &vertex_layout, topology: tri, targets: &[], depth: shadow_depth.clone() });
        let shadow_single = pipeline(device, PipelineSpec { label: "shadow single-sided", layout: &shadow_layout, module: &shadow_module, vs: "vs_shadow", fs: Some("fs_shadow_opaque"), buffers: &vertex_layout, topology: tri, targets: &[], depth: shadow_depth.clone() });
        let shadow_masked = pipeline(device, PipelineSpec { label: "shadow masked", layout: &shadow_layout, module: &shadow_module, vs: "vs_shadow", fs: Some("fs_shadow_masked"), buffers: &vertex_layout, topology: tri, targets: &[], depth: shadow_depth.clone() });
        // Light cubes: the same casters without pancaking, which would flatten the
        // light's own marker (inside the near plane) onto the cube and black it out.
        let cube_opaque = pipeline(device, PipelineSpec { label: "light shadow opaque", layout: &shadow_layout, module: &shadow_module, vs: "vs_shadow_cube", fs: None, buffers: &vertex_layout, topology: tri, targets: &[], depth: shadow_depth.clone() });
        let cube_single = pipeline(device, PipelineSpec { label: "light shadow single-sided", layout: &shadow_layout, module: &shadow_module, vs: "vs_shadow_cube", fs: Some("fs_shadow_opaque"), buffers: &vertex_layout, topology: tri, targets: &[], depth: shadow_depth.clone() });
        let cube_masked = pipeline(device, PipelineSpec { label: "light shadow masked", layout: &shadow_layout, module: &shadow_module, vs: "vs_shadow_cube", fs: Some("fs_shadow_masked"), buffers: &vertex_layout, topology: tri, targets: &[], depth: shadow_depth });

        let fade_buffers = [1.0f32, 0.35].map(|a| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("overlay fade"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&[a, 0.0, 0.0, 0.0]));
            buffer
        });
        let fade_groups = [0, 1].map(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("overlay fade"),
                layout: &fade_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: fade_buffers[i].as_entire_binding() }],
            })
        });
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("render_wgpu linear clamp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("render_wgpu shadow compare"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(C::LessEqual),
            ..Default::default()
        });
        let no_shadows = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("render_wgpu no shadows"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let tony = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_wgpu TonyMcMapface"),
            size: wgpu::Extent3d { width: 48, height: 48, depth_or_array_layers: 48 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgb9e5Ufloat,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tony, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            TONY_MC_MAPFACE,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(48 * 4), rows_per_image: Some(48) },
            wgpu::Extent3d { width: 48, height: 48, depth_or_array_layers: 48 },
        );
        let tony_view = tony.create_view(&wgpu::TextureViewDescriptor::default());
        let ibl = ibl::Ibl::new(device, queue, &frame_layout, &ibl_module);
        Self {
            device: device.clone(),
            queue: queue.clone(),
            scene,
            frame_buffer,
            frame_layout,
            frame_group: None,
            pipelines: Pipelines {
                prepass,
                forward,
                blended,
                sky,
                wire,
                resolve,
                fxaa,
                lines,
                points,
                sprites,
                shadow_opaque,
                shadow_single,
                shadow_masked,
                cube_opaque,
                cube_single,
                cube_masked,
                gtao,
                denoise,
                ssr,
                ssr_layout,
                taa,
                taa_layout,
                bloom_first,
                bloom_down,
                bloom_up,
                resolve_layout,
                fxaa_layout,
                ao_layout,
                bloom_layout,
                lighting_layout,
                cascade_layout,
                fade_groups,
                _fade_buffers: fade_buffers,
                accumulate,
                accumulated,
                accumulate_layout,
            },
            targets: None,
            target_generation: 0,
            overlay_lines: None,
            overlay_points: None,
            overlay_scratch: (Vec::new(), Vec::new()),
            linear_sampler,
            shadow_sampler,
            shadows: None,
            no_shadows,
            shadow_generation: 0,
            ibl,
            _tony: tony,
            tony_view,
            lights: lights::Lights::new(device),
            lighting_group: None,
            timer: None,
            profiling: false,
            accumulation: None,
            taa: taa::TaaState::default(),
            prepass_key: None,
            rendered_epoch: 0,
            rendered_generation: 0,
            used_materials: Vec::new(),
            textures_changed: false,
            stats: FrameStats::default(),
        }
    }

    /// Times each pass group on the GPU from now on (when the device has timestamp
    /// queries); read the result with [`Renderer::gpu_times`] after [`Renderer::wait_idle`].
    pub fn set_profiling(&mut self, on: bool) {
        self.profiling = on;
        if on && self.timer.is_none() {
            self.timer = timer::GpuTimer::new(&self.device, &self.queue);
        }
    }

    /// The last frame's GPU time per pass group, in milliseconds (profiling only).
    pub fn gpu_times(&mut self) -> &[(&'static str, f64)] {
        match &mut self.timer {
            Some(timer) if self.profiling => {
                timer.read(&self.device);
                &timer.last
            }
            _ => &[],
        }
    }

    /// The finished picture of the last [`Renderer::render`], in [`OUTPUT_FORMAT`].
    pub fn output(&self) -> Option<&wgpu::Texture> {
        self.targets.as_ref().map(|t| &t.output)
    }

    /// A view of [`Renderer::output`].
    pub fn output_view(&self) -> Option<&wgpu::TextureView> {
        self.targets.as_ref().map(|t| &t.output_view)
    }

    fn ensure_targets(&mut self, size: (u32, u32)) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let device = &self.device;
        let make = |label: &str, format, (width, height): (u32, u32), mips: u32, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let attach = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        let separate_depth = cfg!(target_arch = "wasm32");
        let copied = if separate_depth { wgpu::TextureUsages::COPY_SRC } else { wgpu::TextureUsages::empty() };
        let depth_texture = make("render_wgpu depth", DEPTH_FORMAT, size, 1, attach | copied);
        let depth = view(&depth_texture);
        let depth_copy = separate_depth.then(|| {
            let texture = make("render_wgpu depth copy", DEPTH_FORMAT, size, 1, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
            let copy_view = view(&texture);
            (texture, copy_view)
        });
        let ids = view(&make("render_wgpu ids", ID_FORMAT, size, 1, attach));
        let normals = view(&make("render_wgpu normals", NORMAL_FORMAT, size, 1, attach));
        let motion = view(&make("render_wgpu motion", MOTION_FORMAT, size, 1, attach));
        let hdr = view(&make("render_wgpu hdr", HDR_FORMAT, size, 1, attach));
        let reflectivity = view(&make("render_wgpu reflectivity", REFLECTIVITY_FORMAT, size, 1, attach));
        let ldr = view(&make("render_wgpu ldr", OUTPUT_FORMAT, size, 1, attach));
        let output = make("render_wgpu output", OUTPUT_FORMAT, size, 1, attach | wgpu::TextureUsages::COPY_SRC);
        let output_view = view(&output);
        let half = (size.0.div_ceil(2).max(1), size.1.div_ceil(2).max(1));
        let ao_size = if ao_full_resolution(size) { size } else { half };
        let ao_raw = view(&make("render_wgpu ao raw", AO_FORMAT, ao_size, 1, attach));
        let ao = view(&make("render_wgpu ao", AO_FORMAT, ao_size, 1, attach));
        let ssr = view(&make("render_wgpu ssr", HDR_FORMAT, half, 1, attach));
        // The last mip about 8 to 16 texels across: as wide a glow as the picture needs,
        // without passes over a few texels (each costs a pass's fixed price).
        let levels = (32 - half.0.min(half.1).max(1).leading_zeros()).saturating_sub(3).clamp(1, BLOOM_LEVELS);
        let bloom = make("render_wgpu bloom", HDR_FORMAT, half, levels, attach);
        let bloom_mips: Vec<wgpu::TextureView> = (0..levels)
            .map(|mip| bloom.create_view(&wgpu::TextureViewDescriptor { base_mip_level: mip, mip_level_count: Some(1), ..Default::default() }))
            .collect();
        let group = |label: &str, layout: &wgpu::BindGroupLayout, resources: &[wgpu::BindingResource<'_>], first: u32| {
            let entries: Vec<wgpu::BindGroupEntry<'_>> = resources.iter().enumerate().map(|(i, r)| wgpu::BindGroupEntry { binding: first + i as u32, resource: r.clone() }).collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some(label), layout, entries: &entries })
        };
        use wgpu::BindingResource::{Sampler, TextureView};
        let p = &self.pipelines;
        // The resolve's reflections sit at binding 9, after the FXAA and accumulation
        // bindings the same shader module declares.
        let resolve_group = {
            let entries = [
                wgpu::BindGroupEntry { binding: 0, resource: TextureView(&hdr) },
                wgpu::BindGroupEntry { binding: 1, resource: TextureView(&ids) },
                wgpu::BindGroupEntry { binding: 2, resource: TextureView(&depth) },
                wgpu::BindGroupEntry { binding: 3, resource: TextureView(&bloom_mips[0]) },
                wgpu::BindGroupEntry { binding: 4, resource: TextureView(&self.tony_view) },
                wgpu::BindGroupEntry { binding: 5, resource: Sampler(&self.linear_sampler) },
                wgpu::BindGroupEntry { binding: 9, resource: TextureView(&ssr) },
            ];
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("render_wgpu resolve"), layout: &p.resolve_layout, entries: &entries })
        };
        let ssr_group = group(
            "render_wgpu ssr",
            &p.ssr_layout,
            &[TextureView(&hdr), TextureView(&depth), TextureView(&normals), TextureView(&reflectivity), TextureView(&self.ibl.specular_view), Sampler(&self.linear_sampler)],
            0,
        );
        let fxaa_group = group("render_wgpu fxaa", &p.fxaa_layout, &[TextureView(&ldr), Sampler(&self.linear_sampler)], 6);
        let gtao_group = group("render_wgpu gtao", &p.ao_layout, &[TextureView(&depth), TextureView(&normals), TextureView(&ao)], 0);
        let denoise_group = group("render_wgpu ao denoise", &p.ao_layout, &[TextureView(&depth), TextureView(&normals), TextureView(&ao_raw)], 0);
        let bloom_groups = bloom_mips.iter().map(|mip| group("render_wgpu bloom", &p.bloom_layout, &[TextureView(mip), Sampler(&self.linear_sampler)], 0)).collect();
        let bloom_hdr_group = group("render_wgpu bloom source", &p.bloom_layout, &[TextureView(&hdr), Sampler(&self.linear_sampler)], 0);
        self.target_generation += 1;
        self.targets = Some(Targets {
            size,
            depth_texture,
            depth,
            depth_copy,
            ids,
            normals,
            motion,
            hdr,
            reflectivity,
            ssr,
            ssr_group,
            ldr,
            output,
            output_view,
            resolve_group,
            fxaa_group,
            ao_raw,
            ao,
            gtao_group,
            denoise_group,
            bloom_mips,
            bloom_groups,
            bloom_hdr_group,
            taa: None,
            generation: self.target_generation,
        });
    }

    /// Forgets the temporal filter's history: the next frame starts it afresh.
    pub fn reset_history(&mut self) {
        self.taa.reset();
    }

    /// Whether the temporal filter has converged: a still view needs no more frames until
    /// something changes. Always true without the filter.
    pub fn converged(&self) -> bool {
        self.taa.converged() && !self.streaming()
    }

    /// Whether textures the view draws are still being decoded or uploaded: the host
    /// should keep drawing frames until they are in.
    pub fn streaming(&self) -> bool {
        self.textures_changed || self.scene.streaming() || self.scene.any_missing(&self.used_materials)
    }

    /// Sets how much GPU memory material textures may take (bytes) before the least
    /// recently drawn are dropped.
    pub fn set_texture_budget(&mut self, bytes: u64) {
        self.scene.texture_budget = bytes.max(16 << 20);
    }

    /// Makes the temporal filter's history for the current targets, if not made yet.
    fn ensure_taa(&mut self) {
        let Some(targets) = self.targets.as_mut() else { return };
        if targets.taa.is_some() {
            return;
        }
        targets.taa = Some(taa::TaaTargets::new(
            &self.device,
            targets.size,
            &self.pipelines.taa_layout,
            &self.pipelines.resolve_layout,
            &self.pipelines.bloom_layout,
            taa::Sources { hdr: &targets.hdr, ssr: &targets.ssr, depth: &targets.depth, motion: &targets.motion, ids: &targets.ids, bloom: &targets.bloom_mips[0], tony: &self.tony_view, sampler: &self.linear_sampler },
        ));
        // A new history holds nothing yet.
        self.taa.reset();
    }

    /// Makes (or remakes, for a new size) the shadow maps.
    fn ensure_shadows(&mut self, size: u32) {
        if self.shadows.as_ref().is_some_and(|s| s.size == size) {
            return;
        }
        self.shadows = Some(shadows::Shadows::new(&self.device, &self.pipelines.cascade_layout, size));
        self.shadow_generation += 1;
    }

    fn frame_group(&mut self) -> wgpu::BindGroup {
        let generation = self.scene.buffers_generation;
        if let Some((g, group)) = &self.frame_group {
            if *g == generation {
                return group.clone();
            }
        }
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_wgpu frame"),
            layout: &self.frame_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.frame_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: self.scene.object_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: self.scene.material_buffer.as_entire_binding() },
            ],
        });
        self.frame_group = Some((generation, group.clone()));
        group
    }

    /// The forward pass's lighting group, rebuilt only when a view it holds was replaced.
    fn lighting_group(&mut self) -> wgpu::BindGroup {
        let targets = self.targets.as_ref().expect("made first");
        let key = (targets.generation, self.shadow_generation * 2 + u64::from(self.shadows.is_some()), self.lights.generation);
        if let Some((k, group)) = &self.lighting_group {
            if *k == key {
                return group.clone();
            }
        }
        use wgpu::BindingResource::{Sampler, TextureView};
        let shadow_view = self.shadows.as_ref().map_or(&self.no_shadows, |s| &s.array_view);
        let resources = [
            TextureView(shadow_view),
            Sampler(&self.shadow_sampler),
            TextureView(&self.ibl.specular_view),
            TextureView(&self.ibl.sh_view),
            TextureView(&self.ibl.dfg_view),
            Sampler(&self.linear_sampler),
            TextureView(&targets.ao),
            self.lights.light_buffer.as_entire_binding(),
            self.lights.cluster_buffer.as_entire_binding(),
            self.lights.index_buffer.as_entire_binding(),
            TextureView(self.lights.shadow_view()),
            TextureView(targets.depth_copy.as_ref().map_or(&targets.depth, |(_, copy)| copy)),
        ];
        let entries: Vec<wgpu::BindGroupEntry<'_>> = resources.into_iter().enumerate().map(|(i, resource)| wgpu::BindGroupEntry { binding: i as u32, resource }).collect();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("render_wgpu lighting"), layout: &self.pipelines.lighting_layout, entries: &entries });
        self.lighting_group = Some((key, group.clone()));
        group
    }

    fn write_overlay(&mut self, view: &ViewParams<'_>) -> (u32, u32, u32) {
        let (mut lines, mut points) = std::mem::take(&mut self.overlay_scratch);
        lines.clear();
        points.clear();
        if view.settings.show_axes {
            if let Some(bounds) = view.world_bounds {
                let center = bounds.center();
                let colors = [view.settings.x_axis_color, view.settings.y_axis_color, view.settings.z_axis_color];
                for (axis, color) in colors.iter().enumerate() {
                    let mut from = [center.x() as f32, center.y() as f32, center.z() as f32];
                    let mut to = from;
                    let (lo, hi) = (bounds.min, bounds.max);
                    from[axis] = [lo.x(), lo.y(), lo.z()][axis] as f32;
                    to[axis] = [hi.x(), hi.y(), hi.z()][axis] as f32;
                    lines.push(OverlayVertex { position: from, color: v4(*color) });
                    lines.push(OverlayVertex { position: to, color: v4(*color) });
                }
            }
        }
        let axes = lines.len() as u32;
        let to_vertex = |(p, c): &(Vec3d, [f32; 4])| OverlayVertex { position: [p.x() as f32, p.y() as f32, p.z() as f32], color: *c };
        lines.extend(view.guides.iter().map(to_vertex));
        points.extend(view.points.iter().map(to_vertex));
        let usage = wgpu::BufferUsages::VERTEX;
        write_vertices(&self.device, &self.queue, &mut self.overlay_lines, bytemuck::cast_slice(&lines), usage);
        write_vertices(&self.device, &self.queue, &mut self.overlay_points, bytemuck::cast_slice(&points), usage);
        let counts = (axes, lines.len() as u32, points.len() as u32);
        self.overlay_scratch = (lines, points);
        counts
    }

    /// The bounds of everything in the scene, if anything.
    fn scene_bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut chunks = self.scene.chunks.values().filter(|c| c.vertex_buffer.is_some());
        let first = chunks.next()?.bounds;
        Some(chunks.fold(first, |(lo, hi), c| {
            let (a, b) = c.bounds;
            ([lo[0].min(a[0]), lo[1].min(a[1]), lo[2].min(a[2])], [hi[0].max(b[0]), hi[1].max(b[1]), hi[2].max(b[2])])
        }))
    }

    /// Draws a frame into [`Renderer::output`] and submits it. Sync the scene first.
    pub fn render(&mut self, view: &ViewParams<'_>) {
        let started = Instant::now();
        // Textures for what the last frame drew: queued, uploaded within the budget,
        // evicted when over it. One frame late, which the placeholders cover.
        let used = std::mem::take(&mut self.used_materials);
        self.textures_changed = view.mode == DisplayMode::Material && self.scene.stream(&used);
        let size = (view.size.0.max(1), view.size.1.max(1));
        self.ensure_targets(size);
        let s = view.settings;
        let shaded = view.mode == DisplayMode::Material;
        // No sync since the last frame: what moved then has stopped, and its motion
        // vectors go back to zero.
        if self.scene.sync_epoch == self.rendered_epoch {
            self.scene.settle();
        }
        let scene_change = if self.scene.moving() {
            taa::SceneChange::Moving
        } else if self.scene.geometry_generation != self.rendered_generation {
            taa::SceneChange::Changed
        } else {
            taa::SceneChange::None
        };
        let bounds = self.scene_bounds();
        let shadows_on = shaded && s.shadows && bounds.is_some();
        let ao_on = shaded && s.ambient_occlusion;
        let bloom_on = shaded && s.bloom && s.bloom_intensity > 0.0;
        let ssr_on = shaded && s.reflections && s.reflection_strength > 0.0 && s.reflection_samples >= 1.0;
        // While shadows are off the maps are kept as they are: nothing draws into them,
        // so their cache keys still say what they hold when shadows come back on.
        if shadows_on {
            self.ensure_shadows(shadow_size(s));
        }

        let mut projection = projection(view.camera, size.0, size.1);
        let unjittered = to_f32(&(projection * view.camera.view_matrix()));
        // The temporal filter: on for the view when antialiasing or accumulation asks for
        // it, in the shaded modes; how much history to keep, and which jitter to draw.
        let wants_taa = view.sample.is_none() && view.mode != DisplayMode::Wireframe && (s.antialiasing || s.accumulate);
        let taa_frame = if wants_taa {
            self.ensure_taa();
            let target = if s.accumulate { s.accumulation_samples.max(1) } else { taa::STEADY_FRAMES };
            Some(self.taa.begin(&unjittered, view.continuing, s.antialiasing, target, scene_change))
        } else {
            self.taa.reset();
            None
        };
        let jitter_sample = view.sample.or(taa_frame.and_then(|f| f.jitter));
        if let Some(sample) = jitter_sample {
            // A sub-pixel offset in clip space (scaled by w, so it is the same in pixels
            // at every depth); the first sample is the pixel centre.
            let (jx, jy) = if sample == 0 { (0.0, 0.0) } else { (halton(sample, 2) - 0.5, halton(sample, 3) - 0.5) };
            let jitter = Mat4x4d::from_rows([
                [1.0, 0.0, 0.0, 2.0 * jx / f64::from(size.0)],
                [0.0, 1.0, 0.0, -2.0 * jy / f64::from(size.1)],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]);
            projection = jitter * projection;
        }
        let view_proj = projection * view.camera.view_matrix();
        let inverse = invert(view_proj).unwrap_or(view_proj);
        let vp = to_f32(&view_proj);
        let light = sun_direction(s);
        let fit = match (shadows_on, bounds) {
            (true, Some(bounds)) => {
                let light64 = light.map(f64::from);
                shadows::fit(view.camera, &view.camera.projection_matrix(size.0 as i32, size.1 as i32), light64, bounds, shadow_size(s), f64::from(s.shadow_bias / DEFAULT_SHADOW_BIAS))
            }
            _ => shadows::Fit::default(),
        };
        let planes = frustum_planes(&vp);
        let view_matrix = to_f32(&view.camera.view_matrix());
        let proj_f32 = to_f32(&projection);
        let light_frame = self.lights.update(
            &self.device,
            &self.queue,
            &self.pipelines.cascade_layout,
            &self.scene.lights,
            self.scene.lights_generation,
            [view.camera.position.x() as f32, view.camera.position.y() as f32, view.camera.position.z() as f32],
            &view_matrix,
            &proj_f32,
            &planes,
            size,
            light_shadow_size(s),
            shaded && view.lights,
        );
        // FXAA only for a view without the temporal filter (the offline samples average
        // instead).
        let fxaa = s.antialiasing && view.sample.is_none() && taa_frame.is_none();
        let background = s.background_color;
        let contrast = if 0.2126 * background[0] + 0.7152 * background[1] + 0.0722 * background[2] > 0.5 { 0.12 } else { 0.85 };
        let eye = view.camera.position;
        let forward = view.camera.direction();
        let bloom_levels = self.targets.as_ref().map_or(1, |t| t.bloom_mips.len()) as f32;
        let uniform = FrameUniform {
            view_proj: vp,
            inv_view_proj: to_f32(&inverse),
            view: view_matrix,
            proj: proj_f32,
            inv_proj: to_f32(&invert(projection).unwrap_or(projection)),
            cascades: fit.matrices,
            eye: [eye.x() as f32, eye.y() as f32, eye.z() as f32, 1.0],
            light_dir: [light[0], light[1], light[2], 0.0],
            light_color: v4(s.sun_color.map(|c| c * s.sun_intensity)),
            sky: v4(s.sky_color),
            horizon: v4(s.horizon_color),
            ground: v4(s.ground_color),
            selected_color: v4(s.selected_face_color),
            locked_color: v4(s.locked_face_color),
            background: v4(background),
            wire_color: [contrast, contrast, contrast, 1.0],
            params0: [s.default_roughness, s.default_metalness, s.ambient_intensity, s.exposure],
            params1: [f32::from(u8::from(view.mode == DisplayMode::Solid)), f32::from(u8::from(s.procedural_sky)), (s.handle_radius + 2.0) * 2.0, view.outline_width as f32],
            viewport: [size.0 as f32, size.1 as f32, 1.0 / size.0 as f32, 1.0 / size.1 as f32],
            sun_color: v4(s.sun_color),
            forward: [forward.x() as f32, forward.y() as f32, forward.z() as f32, 0.0],
            cascade_splits: fit.splits,
            cascade_texel: fit.texel,
            cascade_bias: fit.bias,
            shadow: [
                f32::from(u8::from(shadows_on)),
                (s.shadow_bias / DEFAULT_SHADOW_BIAS).max(0.0),
                (s.shadow_softness * SHADOW_TAP_SPREAD).max(0.5),
                1.0 / shadow_size(s) as f32,
            ],
            effects: [f32::from(u8::from(ao_on)), s.ao_strength, s.ao_radius.max(1.0), ibl::SPECULAR_MIPS as f32],
            post: [
                match view.tonemapper {
                    Tonemapper::Aces => 0.0,
                    Tonemapper::Agx => 1.0,
                    Tonemapper::KhronosNeutral => 2.0,
                    Tonemapper::TonyMcMapface => 3.0,
                },
                if bloom_on { s.bloom_intensity / bloom_levels } else { 0.0 },
                s.bloom_threshold,
                BLOOM_KNEE,
            ],
            misc: [f32::from(u8::from(fxaa)), 1.0, view.sample.unwrap_or(0) as f32, taa_frame.map_or(0.0, |f| f.noise as f32)],
            dof: match s.depth_of_field {
                Some(d) if d.radius > 0.0 => [d.focus_distance, d.focus_range, d.radius, 0.0],
                _ => [0.0; 4],
            },
            lights: light_frame.lights,
            cluster: light_frame.cluster,
            point_shadow: [light_frame.point_shadow[0], light_frame.point_shadow[1], light_frame.point_shadow[2], (s.shadow_bias / DEFAULT_SHADOW_BIAS).max(0.0)],
            // As GL: contact shadows only with the sun's shadows.
            contact: [f32::from(u8::from(shadows_on && s.contact_shadows && s.contact_shadow_distance > 0.0)), s.contact_shadow_distance, s.contact_shadow_samples.max(1.0), 0.0],
            reflection: [if ssr_on { s.reflection_strength } else { 0.0 }, s.reflection_distance.max(1.0), s.reflection_samples.max(1.0), f32::from(u8::from(taa_frame.is_some()))],
            taa: match taa_frame {
                Some(f) => [1.0, f.weight, f32::from(u8::from(ssr_on)), f.spread],
                None => [0.0, 1.0, 0.0, 1.0],
            },
            view_proj_unjittered: unjittered,
            prev_view_proj: taa_frame.map_or(unjittered, |f| f.previous),
            // Under the temporal filter each frame turns its slice, so one per frame adds
            // up to many as the history fills; alone, three keep the noise down.
            ao: [if ao_full_resolution(size) { 1.0 } else { 2.0 }, if taa_frame.is_some() { 1.0 } else { 3.0 }, 0.0, 0.0],
        };
        self.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&uniform));
        let (axis_count, line_count, point_count) = self.write_overlay(view);
        let frame_group = self.frame_group();
        let lighting_group = self.lighting_group();
        let mut stats = FrameStats { lights: light_frame.lights[0] as u32, ..FrameStats::default() };
        let mut timer = if self.profiling { self.timer.take() } else { None };
        if let Some(timer) = &mut timer {
            timer.begin();
        }
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("render_wgpu frame") });

        if shaded {
            stats.environment_rebuilt = self.ibl.update(&mut encoder, &frame_group, s);
        }

        // Shadow cascades whose matrix or casters changed.
        if shadows_on {
            let hidden = self.scene.hidden_generation;
            let materials = self.scene.materials_generation;
            let shadows = self.shadows.as_mut().expect("made above");
            for (i, matrix) in fit.matrices.iter().enumerate() {
                let cascade_planes = frustum_planes(matrix);
                // Walked twice (the key, then the draws if it changed) rather than collected:
                // most frames only hash.
                let casters = || self.scene.chunks.iter().filter(|(_, c)| c.vertex_buffer.is_some() && !outside(&cascade_planes, c.bounds));
                let key = shadows::cascade_key(matrix, casters().map(|(k, c)| (*k, c.generation)), hidden, materials);
                if !shadows.claim(&self.queue, i, key, matrix) {
                    continue;
                }
                stats.cascades_drawn += 1;
                let depth = wgpu::RenderPassDepthStencilAttachment { view: shadows.layer(i), depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None };
                let mut rp = pass(&mut encoder, "shadow cascade", &[], Some(depth), timer.as_mut().and_then(|t| t.writes("shadows")));
                rp.set_bind_group(0, &frame_group, &[]);
                rp.set_bind_group(2, &shadows.uniform_group, &[shadows::Shadows::offset(i)]);
                // Every opaque surface of a chunk in one call; masked ones per material,
                // as their holes come from the texture.
                rp.set_pipeline(&self.pipelines.shadow_opaque);
                rp.set_bind_group(1, self.scene.texture_bind_group(0), &[]);
                for (_, chunk) in casters() {
                    let (Some(vb), Some(ib)) = (&chunk.vertex_buffer, &chunk.index_buffer) else { continue };
                    rp.set_vertex_buffer(0, vb.slice(..));
                    rp.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    if chunk.single_sided_opaque {
                        for draw in chunk.draws.iter().filter(|draw| draw.phase == Phase::Opaque) {
                            rp.set_pipeline(if self.scene.single_sided(draw.material) {&self.pipelines.shadow_single} else {&self.pipelines.shadow_opaque});
                            rp.draw_indexed(draw.indices.clone(), 0, draw.material..draw.material + 1);
                            stats.draws += 1;
                        }
                    } else if !chunk.opaque.is_empty() {
                        rp.set_pipeline(&self.pipelines.shadow_opaque);
                        rp.draw_indexed(chunk.opaque.clone(), 0, 0..1);
                        stats.draws += 1;
                    }
                }
                rp.set_pipeline(&self.pipelines.shadow_masked);
                for (_, chunk) in casters() {
                    let (Some(vb), Some(ib)) = (&chunk.vertex_buffer, &chunk.index_buffer) else { continue };
                    let mut masked = chunk.draws.iter().filter(|d| d.phase == Phase::Masked).peekable();
                    if masked.peek().is_none() {
                        continue;
                    }
                    rp.set_vertex_buffer(0, vb.slice(..));
                    rp.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    for draw in masked {
                        rp.set_bind_group(1, self.scene.texture_bind_group(draw.material), &[]);
                        rp.draw_indexed(draw.indices.clone(), 0, draw.material..draw.material + 1);
                        stats.draws += 1;
                    }
                }
            }
        }

        stats.sun_shadow_draws = stats.draws;

        // Light cube faces whose light or casters changed.
        if let Some(cubes) = self.lights.shadows.as_mut() {
            let hidden = self.scene.hidden_generation;
            let materials = self.scene.materials_generation;
            for &(slot, index) in &self.lights.shadowed {
                let light = &self.scene.lights[index];
                let position = [light.position.x() as f32, light.position.y() as f32, light.position.z() as f32];
                for face in 0..6 {
                    if !lights::face_reached(face, light.spot) {
                        continue;
                    }
                    let matrix = lights::face_matrix(face, position, light.range);
                    let face_planes = frustum_planes(&matrix);
                    let casters = || self.scene.chunks.iter().filter(|(_, c)| c.vertex_buffer.is_some() && !outside(&face_planes, c.bounds));
                    let key = lights::face_key(light, face, casters().map(|(k, c)| (*k, c.generation)), hidden, materials);
                    if !cubes.claim(&self.queue, slot, face, key, &matrix) {
                        continue;
                    }
                    stats.light_faces_drawn += 1;
                    let depth = wgpu::RenderPassDepthStencilAttachment { view: cubes.face(slot, face), depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None };
                    let mut rp = pass(&mut encoder, "light shadow", &[], Some(depth), timer.as_mut().and_then(|t| t.writes("light shadows")));
                    rp.set_bind_group(0, &frame_group, &[]);
                    rp.set_bind_group(2, &cubes.uniform_group, &[lights::CubeShadows::offset(slot, face)]);
                    for (_, chunk) in casters() {
                        let (Some(vb), Some(ib)) = (&chunk.vertex_buffer, &chunk.index_buffer) else { continue };
                        rp.set_vertex_buffer(0, vb.slice(..));
                        rp.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        if !chunk.opaque.is_empty() {
                            rp.set_pipeline(&self.pipelines.cube_opaque);
                            rp.set_bind_group(1, self.scene.texture_bind_group(0), &[]);
                            if chunk.single_sided_opaque {
                                for draw in chunk.draws.iter().filter(|draw| draw.phase == Phase::Opaque) {
                                    rp.set_pipeline(if self.scene.single_sided(draw.material) {&self.pipelines.cube_single} else {&self.pipelines.cube_opaque});
                                    rp.draw_indexed(draw.indices.clone(), 0, draw.material..draw.material + 1);
                                    stats.draws += 1;
                                }
                            } else {
                                rp.draw_indexed(chunk.opaque.clone(), 0, 0..1);
                                stats.draws += 1;
                            }
                        }
                        for draw in chunk.draws.iter().filter(|d| d.phase == Phase::Masked) {
                            rp.set_pipeline(&self.pipelines.cube_masked);
                            rp.set_bind_group(1, self.scene.texture_bind_group(draw.material), &[]);
                            rp.draw_indexed(draw.indices.clone(), 0, draw.material..draw.material + 1);
                            stats.draws += 1;
                        }
                    }
                }
            }
        }

        stats.light_shadow_draws = stats.draws - stats.sun_shadow_draws;

        let visible: Vec<&scene::Chunk> = self
            .scene
            .chunks
            .values()
            .filter(|c| {
                let out = outside(&planes, c.bounds);
                if out {
                    stats.chunks_culled += 1;
                }
                !out && c.vertex_buffer.is_some()
            })
            .collect();
        stats.chunks_drawn = visible.len() as u32;
        if view.mode == DisplayMode::Material {
            self.used_materials = self.scene.materials_of(visible.iter().copied());
        }
        // A selection click, a hover or a light edit with a still camera changes only
        // the shading: the prepass targets and the occlusion still hold, and are kept.
        let prepass_key = (view.mode != DisplayMode::Wireframe).then(|| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            for v in vp.iter().flatten().chain(uniform.ao.iter()).chain([uniform.misc[3], s.ao_radius].iter()) {
                v.to_bits().hash(&mut hasher);
            }
            (size, self.target_generation, self.scene.hidden_generation, self.scene.materials_generation, self.scene.textures_generation, ao_on).hash(&mut hasher);
            // Chunk generations are unique, so their sum names the set drawn.
            visible.iter().fold(0u64, |sum, c| sum.wrapping_add(c.generation.wrapping_mul(0x9e37_79b9_7f4a_7c15))).hash(&mut hasher);
            hasher.finish()
        });
        let reuse_prepass = prepass_key.is_some() && prepass_key == self.prepass_key;
        stats.prepass_cached = reuse_prepass;

        let targets = self.targets.as_ref().expect("made above");
        let p = &self.pipelines;
        let clear_depth = |view| wgpu::RenderPassDepthStencilAttachment { view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }), stencil_ops: None };
        let keep_depth = |view| wgpu::RenderPassDepthStencilAttachment { view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }), stencil_ops: None };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        // `depth_only`: the prepass needs a material for masked holes, normal maps and
        // imported face rules; ordinary opaque surfaces are one call. It also reads last
        // frame's positions (second vertex buffer) for the motion target.
        let draw_phase = |pass: &mut wgpu::RenderPass<'static>, phases: &[Phase], wire: bool, depth_only: bool, stats: &mut FrameStats| {
            for chunk in &visible {
                let (Some(vb), Some(ib)) = (&chunk.vertex_buffer, &chunk.index_buffer) else { continue };
                pass.set_vertex_buffer(0, vb.slice(..));
                if depth_only {
                    // Last frame's positions: the chunk's own where nothing moved.
                    pass.set_vertex_buffer(1, chunk.previous_buffer.as_ref().unwrap_or(vb).slice(..));
                }
                if wire {
                    let Some(wb) = &chunk.wire_buffer else { continue };
                    pass.set_bind_group(1, self.scene.texture_bind_group(0), &[]);
                    pass.set_index_buffer(wb.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..chunk.wire_count, 0, 0..1);
                    stats.draws += 1;
                    continue;
                }
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                if depth_only && !chunk.plain.is_empty() {
                    pass.set_bind_group(1, self.scene.texture_bind_group(0), &[]);
                    pass.draw_indexed(chunk.plain.clone(), 0, 0..1);
                    stats.draws += 1;
                }
                let merged = |d: &&scene::Draw| depth_only && d.phase == Phase::Opaque && !d.prepass_material;
                for draw in chunk.draws.iter().filter(|d| phases.contains(&d.phase)).filter(|d| !merged(d)) {
                    pass.set_bind_group(1, self.scene.texture_bind_group(draw.material), &[]);
                    pass.draw_indexed(draw.indices.clone(), 0, draw.material..draw.material + 1);
                    stats.draws += 1;
                }
            }
        };

        // The overlay goes into the picture before FXAA in the shaded modes, so its lines
        // are smoothed as the GL renderer's are; straight into the output in wireframe.
        let draw_overlay = |encoder: &mut wgpu::CommandEncoder, into: &wgpu::TextureView| {
            if line_count + point_count == 0 {
                return;
            }
            let mut rp = pass(encoder, "overlay", &[color(into, wgpu::LoadOp::Load)], Some(keep_depth(&targets.depth)), None);
            rp.set_bind_group(0, &frame_group, &[]);
            for pass_index in 0..2 {
                rp.set_bind_group(1, &p.fade_groups[pass_index], &[]);
                // The axes only against the depth, as the GL renderer draws them.
                let first = if pass_index == 0 { 0 } else { axis_count };
                if let (Some(buffer), true) = (&self.overlay_lines, line_count > first) {
                    rp.set_pipeline(&p.lines[pass_index]);
                    rp.set_vertex_buffer(0, buffer.slice(..));
                    rp.draw(first..line_count, 0..1);
                }
                if let (Some(buffer), true) = (&self.overlay_points, point_count > 0) {
                    rp.set_pipeline(&p.points[pass_index]);
                    rp.set_vertex_buffer(0, buffer.slice(..));
                    rp.draw(0..6, 0..point_count);
                }
            }
        };
        let fullscreen = |encoder: &mut wgpu::CommandEncoder, label: &str, into: &wgpu::TextureView, load: wgpu::LoadOp<wgpu::Color>, pipeline: &wgpu::RenderPipeline, group: &wgpu::BindGroup, writes: Option<wgpu::RenderPassTimestampWrites<'_>>| {
            draw_fullscreen(encoder, label, into, load, pipeline, &frame_group, group, writes);
        };

        if view.mode == DisplayMode::Wireframe {
            let bg = wgpu::Color { r: f64::from(background[0]), g: f64::from(background[1]), b: f64::from(background[2]), a: 1.0 };
            let mut rp = pass(&mut encoder, "wireframe", &[color(&targets.output_view, wgpu::LoadOp::Clear(bg))], Some(clear_depth(&targets.depth)), timer.as_mut().and_then(|t| t.writes("wireframe")));
            rp.set_pipeline(&p.wire);
            rp.set_bind_group(0, &frame_group, &[]);
            draw_phase(&mut rp, &[], true, false, &mut stats);
            drop(rp);
            draw_overlay(&mut encoder, &targets.output_view);
        } else {
            if !reuse_prepass {
                let colors = [color(&targets.ids, clear), color(&targets.normals, clear), color(&targets.motion, clear)];
                let mut rp = pass(&mut encoder, "prepass", &colors, Some(clear_depth(&targets.depth)), timer.as_mut().and_then(|t| t.writes("prepass")));
                rp.set_pipeline(&p.prepass);
                rp.set_bind_group(0, &frame_group, &[]);
                let before = stats.draws;
                draw_phase(&mut rp, &[Phase::Opaque, Phase::Masked], false, true, &mut stats);
                stats.prepass_draws = stats.draws - before;
            }
            if ao_on && !reuse_prepass {
                fullscreen(&mut encoder, "gtao", &targets.ao_raw, clear, &p.gtao, &targets.gtao_group, timer.as_mut().and_then(|t| t.writes("ao")));
                fullscreen(&mut encoder, "ao denoise", &targets.ao, clear, &p.denoise, &targets.denoise_group, timer.as_mut().and_then(|t| t.writes("ao")));
            }
            {
                // The depth is read-only here, so contact shadows can sample it (in the
                // browser they sample a copy; see `Targets::depth_copy`).
                let read_depth = wgpu::RenderPassDepthStencilAttachment { view: &targets.depth, depth_ops: None, stencil_ops: None };
                let depth = match &targets.depth_copy {
                    Some((copy, _)) => {
                        let extent = wgpu::Extent3d { width: targets.size.0, height: targets.size.1, depth_or_array_layers: 1 };
                        encoder.copy_texture_to_texture(targets.depth_texture.as_image_copy(), copy.as_image_copy(), extent);
                        keep_depth(&targets.depth)
                    }
                    None => read_depth,
                };
                let mut rp = pass(&mut encoder, "forward", &[color(&targets.hdr, clear), color(&targets.reflectivity, clear)], Some(depth), timer.as_mut().and_then(|t| t.writes("forward")));
                rp.set_bind_group(0, &frame_group, &[]);
                if s.procedural_sky {
                    rp.set_pipeline(&p.sky);
                    rp.set_bind_group(1, self.scene.texture_bind_group(0), &[]);
                    rp.draw(0..3, 0..1);
                }
                rp.set_pipeline(&p.forward);
                rp.set_bind_group(2, &lighting_group, &[]);
                draw_phase(&mut rp, &[Phase::Opaque, Phase::Masked], false, false, &mut stats);
                rp.set_pipeline(&p.blended);
                draw_phase(&mut rp, &[Phase::Blended], false, false, &mut stats);
            }
            if ssr_on {
                fullscreen(&mut encoder, "ssr", &targets.ssr, clear, &p.ssr, &targets.ssr_group, timer.as_mut().and_then(|t| t.writes("ssr")));
            }
            // The temporal filter blends this frame into its history; what follows reads
            // the new history instead of the frame.
            let (resolve_group, bloom_source) = match (taa_frame, &targets.taa) {
                (Some(frame), Some(history)) => {
                    let mut rp = pass(
                        &mut encoder,
                        "taa",
                        &[color(&history.views[frame.write], clear), color(&history.meta[frame.write], clear)],
                        None,
                        timer.as_mut().and_then(|t| t.writes("taa")),
                    );
                    rp.set_pipeline(&p.taa);
                    rp.set_bind_group(0, &frame_group, &[]);
                    rp.set_bind_group(1, &history.taa_groups[frame.read], &[]);
                    rp.draw(0..3, 0..1);
                    drop(rp);
                    (&history.resolve_groups[frame.write], &history.bloom_groups[frame.write])
                }
                _ => (&targets.resolve_group, &targets.bloom_hdr_group),
            };
            if bloom_on {
                let mips = &targets.bloom_mips;
                fullscreen(&mut encoder, "bloom", &mips[0], clear, &p.bloom_first, bloom_source, timer.as_mut().and_then(|t| t.writes("bloom")));
                // Down the chain, each mip from the one above it; then back up, each adding
                // the tent-filtered mip below it.
                for (mip, source) in mips.iter().skip(1).zip(&targets.bloom_groups) {
                    fullscreen(&mut encoder, "bloom", mip, clear, &p.bloom_down, source, timer.as_mut().and_then(|t| t.writes("bloom")));
                }
                for (mip, source) in mips.iter().zip(targets.bloom_groups.iter().skip(1)).rev() {
                    fullscreen(&mut encoder, "bloom", mip, wgpu::LoadOp::Load, &p.bloom_up, source, timer.as_mut().and_then(|t| t.writes("bloom")));
                }
            }
            // Straight into the output unless FXAA or an offline render's averaging reads
            // the picture afterwards.
            let direct = !fxaa && view.sample.is_none();
            let picture = if direct { &targets.output_view } else { &targets.ldr };
            fullscreen(&mut encoder, "resolve", picture, wgpu::LoadOp::Clear(wgpu::Color::BLACK), &p.resolve, resolve_group, timer.as_mut().and_then(|t| t.writes("post")));
            if !self.scene.sprite_draws.is_empty() {
                if let Some(buffer) = &self.scene.sprite_buffer {
                    let mut rp = pass(&mut encoder, "sprites", &[color(picture, wgpu::LoadOp::Load)], Some(keep_depth(&targets.depth)), None);
                    rp.set_pipeline(&p.sprites);
                    rp.set_bind_group(0, &frame_group, &[]);
                    rp.set_vertex_buffer(0, buffer.slice(..));
                    for (material, range) in &self.scene.sprite_draws {
                        if let Some(group) = self.scene.base_texture_bind_group(*material) {
                            rp.set_bind_group(1, group, &[]);
                            rp.draw(0..6, range.clone());
                            stats.draws += 1;
                        }
                    }
                }
            }
            draw_overlay(&mut encoder, picture);
            if !direct {
                fullscreen(&mut encoder, "fxaa", &targets.output_view, wgpu::LoadOp::Clear(wgpu::Color::BLACK), &p.fxaa, &targets.fxaa_group, timer.as_mut().and_then(|t| t.writes("post")));
            }
        }
        if let Some(frame) = taa_frame {
            self.taa.finish(frame);
        }
        self.prepass_key = prepass_key;
        self.rendered_epoch = self.scene.sync_epoch;
        self.rendered_generation = self.scene.geometry_generation;
        if let Some(timer) = &timer {
            timer.finish(&mut encoder);
        }
        self.queue.submit([encoder.finish()]);
        if timer.is_some() {
            self.timer = timer;
        }
        stats.encode = started.elapsed();
        self.stats = stats;
    }

    /// Draws one more sample of an offline render (`view.sample` says which; 0 starts
    /// over) and leaves the average so far in [`Renderer::output`]. Each sample is a full
    /// frame with a sub-pixel jitter and a turned depth-of-field disc, averaged in linear
    /// light, as the GL renderer's accumulation does with all effects on.
    pub fn accumulate(&mut self, view: &ViewParams<'_>) {
        let sample = view.sample.unwrap_or(0);
        self.render(view);
        let targets = self.targets.as_ref().expect("rendered");
        if self.accumulation.as_ref().is_none_or(|a| a.size != targets.size) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("render_wgpu accumulation"),
                size: wgpu::Extent3d { width: targets.size.0, height: targets.size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let group = |sample_view: &wgpu::TextureView, average: &wgpu::TextureView| {
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("render_wgpu accumulate"),
                    layout: &self.pipelines.accumulate_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(sample_view) },
                        wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(average) },
                    ],
                })
            };
            // The sample is read from the picture before FXAA; the average is read with
            // the (unused here) sample slot pointing at the HDR target, which no pass of
            // the copy writes.
            let from_sample = group(&targets.ldr, &targets.hdr);
            let from_average = group(&targets.hdr, &view);
            self.accumulation = Some(Accumulation { size: targets.size, view, from_sample, from_average, samples: 0 });
        }
        let accumulation = self.accumulation.as_mut().expect("made above");
        accumulation.samples = if sample == 0 { 1 } else { accumulation.samples + 1 };
        let frame_group = self.frame_group.as_ref().expect("rendered").1.clone();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("render_wgpu accumulate") });
        {
            let load = if accumulation.samples == 1 { wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT) } else { wgpu::LoadOp::Load };
            let mut rp = pass(&mut encoder, "accumulate", &[color(&accumulation.view, load)], None, None);
            rp.set_pipeline(&self.pipelines.accumulate);
            rp.set_bind_group(0, &frame_group, &[]);
            rp.set_bind_group(1, &accumulation.from_sample, &[]);
            let weight = 1.0 / f64::from(accumulation.samples);
            rp.set_blend_constant(wgpu::Color { r: weight, g: weight, b: weight, a: weight });
            rp.draw(0..3, 0..1);
        }
        draw_fullscreen(&mut encoder, "accumulated", &targets.output_view, wgpu::LoadOp::Clear(wgpu::Color::BLACK), &self.pipelines.accumulated, &frame_group, &accumulation.from_average, None);
        self.queue.submit([encoder.finish()]);
    }

    /// Blocks until the GPU has finished everything submitted, for profiling.
    pub fn wait_idle(&self) {
        let _ = self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
    }

    /// Reads [`Renderer::output`] back as tightly packed RGBA8 rows, top to bottom.
    /// Blocking; for tests and image export.
    pub fn read_output(&self) -> Option<(u32, u32, Vec<u8>)> {
        let targets = self.targets.as_ref()?;
        let (width, height) = targets.size;
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render_wgpu readback"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("render_wgpu readback") });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &targets.output, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) } },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.wait_idle();
        let data = slice.get_mapped_range().ok()?;
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let start = (y * row) as usize;
            out.extend_from_slice(&data[start..start + (width * 4) as usize]);
        }
        drop(data);
        buffer.unmap();
        Some((width, height, out))
    }
}

/// The `index`th value of the Halton sequence in the given base: a low-discrepancy
/// sequence, so every prefix of the jitter covers the pixel evenly (as GL's).
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

fn write_vertices(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &mut Option<wgpu::Buffer>, data: &[u8], usage: wgpu::BufferUsages) {
    if data.is_empty() {
        return;
    }
    let needed = data.len() as u64;
    if buffer.as_ref().is_none_or(|b| b.size() < needed) {
        *buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render_wgpu overlay"),
            size: needed.next_power_of_two().max(1024),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
    queue.write_buffer(buffer.as_ref().expect("just made"), 0, data);
}

fn color(view: &wgpu::TextureView, load: wgpu::LoadOp<wgpu::Color>) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    Some(wgpu::RenderPassColorAttachment { view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load, store: wgpu::StoreOp::Store } })
}

/// One fullscreen triangle into `into` with a pass of its own.
fn draw_fullscreen(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    into: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
    pipeline: &wgpu::RenderPipeline,
    frame_group: &wgpu::BindGroup,
    group: &wgpu::BindGroup,
    writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
) {
    let mut rp = pass(encoder, label, &[color(into, load)], None, writes);
    rp.set_pipeline(pipeline);
    rp.set_bind_group(0, frame_group, &[]);
    rp.set_bind_group(1, group, &[]);
    rp.draw(0..3, 0..1);
}

/// Begins a render pass, detached from the encoder's borrow.
fn pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    colors: &[Option<wgpu::RenderPassColorAttachment<'_>>],
    depth: Option<wgpu::RenderPassDepthStencilAttachment<'_>>,
    timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
) -> wgpu::RenderPass<'static> {
    encoder
        .begin_render_pass(&wgpu::RenderPassDescriptor { label: Some(label), color_attachments: colors, depth_stencil_attachment: depth, timestamp_writes: timestamps, occlusion_query_set: None, multiview_mask: None })
        .forget_lifetime()
}
