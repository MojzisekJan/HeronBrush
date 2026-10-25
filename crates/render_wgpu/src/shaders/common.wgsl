// Shared by every pass: the per-frame uniform, the object and material tables, and the
// small helpers (sky, encodings, noise) more than one pass needs.

struct Frame {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    // The projection (reverse-Z) and its inverse, for view-space reconstruction.
    proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    // World to shadow-map space per cascade: xy in -1..1, z in 0..1 (not reversed).
    cascades: array<mat4x4<f32>, 4>,
    eye: vec4<f32>,
    // The way the sunlight travels (from the sun), xyz.
    light_dir: vec4<f32>,
    // Sun colour times intensity.
    light_color: vec4<f32>,
    sky: vec4<f32>,
    horizon: vec4<f32>,
    ground: vec4<f32>,
    selected_color: vec4<f32>,
    locked_color: vec4<f32>,
    background: vec4<f32>,
    wire_color: vec4<f32>,
    // x default roughness, y default metallic, z ambient strength, w exposure.
    params0: vec4<f32>,
    // x flat shading, y procedural sky, z handle size in pixels, w outline width.
    params1: vec4<f32>,
    // width, height, 1/width, 1/height in pixels.
    viewport: vec4<f32>,
    // The sun's own colour, for the disc in the sky.
    sun_color: vec4<f32>,
    // The camera's forward axis, xyz.
    forward: vec4<f32>,
    // The far end of each cascade, as distance along the view axis.
    cascade_splits: vec4<f32>,
    // The world size of one shadow texel, per cascade.
    cascade_texel: vec4<f32>,
    // The depth bias per cascade, in shadow-map depth units.
    cascade_bias: vec4<f32>,
    // x shadows on, y normal-offset scale, z filter radius in texels, w 1 / map size.
    shadow: vec4<f32>,
    // x ambient occlusion on, y its strength, z its radius in world units, w the
    // specular environment's mip count.
    effects: vec4<f32>,
    // x tone mapper (0 ACES, 1 AgX, 2 Khronos PBR Neutral, 3 TonyMcMapface),
    // y bloom intensity (0 for none), z bloom threshold, w bloom knee.
    post: vec4<f32>,
    // x FXAA on, y image-based lighting ready, z the accumulation sample (rotates the
    // depth-of-field disc), w the temporal filter's frame (varies per-pixel noise).
    misc: vec4<f32>,
    // Depth of field: focus distance, sharp range (map units), the largest blur radius in
    // pixels of a 1080-pixel-high image (0 for none), unused.
    dof: vec4<f32>,
    // Light entities: x how many are in view (0 skips them), yzw the cluster grid.
    lights: vec4<f32>,
    // x, y a cluster tile in pixels; a view depth z falls in slice log2(z) * z_ - w_.
    cluster: vec4<f32>,
    // Light shadow cubes: x side, y 1 / side, z near plane, w normal-offset scale.
    point_shadow: vec4<f32>,
    // Contact shadows: x on, y how far the ray goes (map units), z its steps, w unused.
    contact: vec4<f32>,
    // Screen-space reflections: x strength (0 off), y how far the ray goes, z its steps,
    // w 1 to scatter rays by roughness (a temporal filter averages them).
    reflection: vec4<f32>,
    // Temporal antialiasing: x on, y the current frame's weight in the blend (1: no
    // history), z the reflections are in its output (the resolve does not add them
    // again), w how many standard deviations the history may stray from the
    // neighbourhood.
    taa: vec4<f32>,
    // The view-projection without the sub-pixel jitter (the overlay, sprites, and TAA's
    // reprojection), and last frame's.
    view_proj_unjittered: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    // Ambient occlusion: x full-resolution pixels per occlusion texel (1 or 2), y slices
    // per pixel, zw unused.
    ao: vec4<f32>,
};

struct Object {
    // bit 0 selected, bit 1 locked, bit 2 hidden.
    flags: u32,
    // RGBA8 tint, alpha is the strength. Unused so far.
    tint: u32,
    pad0: u32,
    pad1: u32,
};

struct Material {
    // The texture's size in texels: the vertex UVs are in texels.
    size: vec2<f32>,
    // 0 opaque, 1 masked, 2 blended.
    alpha_mode: u32,
    // bit 0 has its own PBR factors (else the frame defaults), bit 1 skip hatching,
    // bit 2 has a normal map, bit 3 the normal map's green channel points down, bit 4
    // the texture is not resident yet, bit 5 has imported PBR maps.
    flags: u32,
    // roughness, metallic, emission; w holds the average colour (sRGB RGBA8, as bits)
    // a pending texture is drawn in.
    factors: vec4<f32>,
    // normal scale, AO strength, alpha mask cutoff, face rule (0 brush, 1 single, 2 double)
    model_params: vec4<f32>,
    model_emissive: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> objects: array<Object>;
@group(0) @binding(2) var<storage, read> materials: array<Material>;

const OBJECT_SELECTED: u32 = 1u;
const OBJECT_LOCKED: u32 = 2u;
const OBJECT_HIDDEN: u32 = 4u;
// Drawn but casts no shadow: an editor-only symbol such as a light's bulb.
const OBJECT_SHADOWLESS: u32 = 8u;

const MATERIAL_OWN_FACTORS: u32 = 1u;
const MATERIAL_NORMAL_MAP: u32 = 4u;
const MATERIAL_NORMAL_FLIP_Y: u32 = 8u;
const MATERIAL_PENDING: u32 = 16u;

// Per-vertex flags, in the top byte of the object word (see editor_core::render_scene).
const FACE_SELECTED: u32 = 1u;
const NO_TINT: u32 = 2u;
const SKIP: u32 = 4u;

const PI: f32 = 3.14159265359;

fn aces(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Linear light to the sRGB transfer curve, what an 8-bit display target stores.
fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let x = max(c, vec3<f32>(0.0));
    return select(1.055 * pow(x, vec3<f32>(1.0 / 2.4)) - 0.055, x * 12.92, x <= vec3<f32>(0.0031308));
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

// Octahedral unit-vector encoding into two numbers in -1..1, for the normal target.
fn octahedral_encode(n: vec3<f32>) -> vec2<f32> {
    let p = n.xy / (abs(n.x) + abs(n.y) + abs(n.z));
    if (n.z >= 0.0) {
        return p;
    }
    return (1.0 - abs(p.yx)) * select(vec2<f32>(-1.0), vec2<f32>(1.0), p >= vec2<f32>(0.0));
}

fn octahedral_decode(e: vec2<f32>) -> vec3<f32> {
    var n = vec3<f32>(e, 1.0 - abs(e.x) - abs(e.y));
    let t = max(-n.z, 0.0);
    n.x += select(t, -t, n.x >= 0.0);
    n.y += select(t, -t, n.y >= 0.0);
    return normalize(n);
}

// Jorge Jimenez's interleaved gradient noise: cheap per-pixel noise that a small blur
// turns into a smooth result.
fn interleaved_gradient_noise(p: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(p, vec2<f32>(0.06711056, 0.00583715))));
}

// Three-band sky, as the GL renderer's `sky()`: ground, horizon haze, zenith, plus the
// sun's glint when `sun_weight` > 0.
fn sky_light(direction: vec3<f32>, roughness: f32, sun_weight: f32) -> vec3<f32> {
    let up = clamp(direction.z, -1.0, 1.0);
    let above = mix(frame.horizon.rgb, frame.sky.rgb, pow(clamp(up, 0.0, 1.0), 0.45));
    let below = mix(frame.horizon.rgb, frame.ground.rgb, pow(clamp(-up, 0.0, 1.0), 0.35));
    var result = select(below, above, up >= 0.0);
    if (sun_weight > 0.0) {
        let to_sun = normalize(-frame.light_dir.xyz);
        let alignment = clamp(dot(direction, to_sun), 0.0, 1.0);
        let sharpness = mix(2000.0, 8.0, clamp(roughness, 0.0, 1.0));
        result += frame.light_color.rgb * pow(alignment, sharpness) * (1.0 - roughness * 0.6) * sun_weight;
    }
    return result;
}

// The procedural sky as seen, without the sun's disc: zenith, horizon haze warmed
// towards the sun at sunset, ground, dimmed at night, and the glow round the sun. What
// the view shows behind the geometry and what the environment map is made from.
fn sky_radiance(direction: vec3<f32>) -> vec3<f32> {
    let up = clamp(direction.z, -1.0, 1.0);
    let sun = normalize(-frame.light_dir.xyz);
    let sun_height = sun.z;
    let daylight = smoothstep(-0.12, 0.08, sun_height);
    let sunset = (1.0 - smoothstep(0.04, 0.4, abs(sun_height))) * daylight;
    let facing = pow(max(dot(direction.xy, sun.xy), 0.0), 3.0);
    let horizon = mix(frame.horizon.rgb, vec3<f32>(1.0, 0.40, 0.12), sunset * facing * 0.55);
    let zenith = mix(frame.sky.rgb, frame.sky.rgb * vec3<f32>(0.65, 0.55, 0.85), sunset * 0.3);
    let above = mix(horizon, zenith, pow(max(up, 0.0), 0.38));
    let below = mix(horizon, frame.ground.rgb, pow(max(-up, 0.0), 0.35));
    var color = select(below, above, up >= 0.0) * mix(0.055, 1.0, daylight);
    let angle = acos(clamp(dot(direction, sun), -1.0, 1.0));
    let halo = 0.65 * exp(-angle * 18.0) + 0.12 * exp(-angle * 3.0);
    let warmth = mix(vec3<f32>(1.0), vec3<f32>(1.0, 0.48, 0.16), sunset);
    color += frame.sun_color.rgb * warmth * halo * smoothstep(-0.12, 0.06, up) * smoothstep(-0.06, 0.0, sun_height + radians(0.75));
    return color;
}

struct FullscreenOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One triangle that covers the screen; uv (0,0) top-left. `z` is the depth it is drawn at.
fn fullscreen(index: u32, z: f32) -> FullscreenOut {
    var out: FullscreenOut;
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, z, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

// The view-space position of a pixel (uv 0..1, top-left origin) at reverse-Z depth `d`.
fn view_position(uv: vec2<f32>, d: f32) -> vec3<f32> {
    let clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, d, 1.0);
    let v = frame.inv_proj * clip;
    return v.xyz / v.w;
}
