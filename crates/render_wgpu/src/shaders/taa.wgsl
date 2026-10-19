// Temporal antialiasing (after Karis 2014, Jimenez's filmic SMAA T2x, and Bevy's): each
// frame is drawn with the projection jittered inside the pixel and blended into a
// history reprojected by the camera's motion and each surface's own (the prepass's
// motion target); the history is clipped to the colour spread of the current pixel's
// neighbourhood, so what changed does not ghost, and dropped where it held another
// surface (its depth disagrees): what was hidden last frame starts afresh, spatially
// filtered until it has gathered a few samples. With a still camera the blend weight
// falls as 1/n and the history converges to a supersampled picture, which is the GL
// renderer's accumulation with every effect in it. Prefixed with common.wgsl.

@group(1) @binding(0) var current: texture_2d<f32>;
@group(1) @binding(1) var reflections: texture_2d<f32>;
@group(1) @binding(2) var depth_texture: texture_depth_2d;
@group(1) @binding(3) var history: texture_2d<f32>;
@group(1) @binding(4) var history_sampler: sampler;
// Each surface's motion since last frame in world units (the prepass), and the
// history's own depth and sample count per pixel (written beside it last frame).
@group(1) @binding(5) var motion_texture: texture_2d<f32>;
@group(1) @binding(6) var history_meta: texture_2d<f32>;

// What the per-pixel history stores as the depth of the sky (and of anything farther).
const FAR_DEPTH: f32 = 60000.0;

struct TaaOut {
    @location(0) color: vec4<f32>,
    // x the view depth of the surface this pixel holds, y the samples in its history.
    @location(1) record: vec4<f32>,
};

// The view depth at reverse-Z depth `d` (the projection's z row; the jitter does not
// touch it).
fn linear_depth(d: f32) -> f32 {
    if (d <= 0.0) {
        return FAR_DEPTH;
    }
    return min(frame.proj[3][2] / (d + frame.proj[2][2]), FAR_DEPTH);
}

@vertex
fn vs_taa(@builtin(vertex_index) index: u32) -> FullscreenOut {
    return fullscreen(index, 0.0);
}

// The scene's light at a pixel, without reflections.
fn scene_at(p: vec2<i32>, size: vec2<i32>) -> vec4<f32> {
    return max(textureLoad(current, clamp(p, vec2<i32>(0), size - 1), 0), vec4<f32>(0.0));
}

// HDR light squeezed into 0..1 (reversibly), so one bright texel does not dominate the
// neighbourhood statistics or the blend (Karis), then into YCoCg, where the clip box
// fits colour spreads more tightly than in RGB.
fn encode(c: vec3<f32>) -> vec3<f32> {
    let squeezed = c / (1.0 + max(c.r, max(c.g, c.b)));
    let y = dot(squeezed, vec3<f32>(0.25, 0.5, 0.25));
    let co = dot(squeezed, vec3<f32>(0.5, 0.0, -0.5));
    let cg = dot(squeezed, vec3<f32>(-0.25, 0.5, -0.25));
    return vec3<f32>(y, co, cg);
}

fn decode(e: vec3<f32>) -> vec3<f32> {
    let squeezed = max(vec3<f32>(e.x + e.y - e.z, e.x + e.z, e.x - e.y - e.z), vec3<f32>(0.0));
    return squeezed / max(1.0 - max(squeezed.r, max(squeezed.g, squeezed.b)), 1e-4);
}

// The history at `uv`, Catmull-Rom filtered with five bilinear taps: sharper than one
// bilinear tap, so a moving camera does not blur the picture frame by frame.
fn sample_history(uv: vec2<f32>, size: vec2<f32>) -> vec4<f32> {
    let position = uv * size;
    let center = floor(position - 0.5) + 0.5;
    let f = position - center;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let t0 = (center - 1.0) / size;
    let t3 = (center + 2.0) / size;
    let t12 = (center + w2 / w12) / size;
    var result = textureSampleLevel(history, history_sampler, vec2<f32>(t12.x, t0.y), 0.0) * (w12.x * w0.y);
    result += textureSampleLevel(history, history_sampler, vec2<f32>(t0.x, t12.y), 0.0) * (w0.x * w12.y);
    result += textureSampleLevel(history, history_sampler, vec2<f32>(t12.x, t12.y), 0.0) * (w12.x * w12.y);
    result += textureSampleLevel(history, history_sampler, vec2<f32>(t3.x, t12.y), 0.0) * (w3.x * w12.y);
    result += textureSampleLevel(history, history_sampler, vec2<f32>(t12.x, t3.y), 0.0) * (w12.x * w3.y);
    let total = w12.x * w0.y + w0.x * w12.y + w12.x * w12.y + w3.x * w12.y + w12.x * w3.y;
    return max(result / total, vec4<f32>(0.0));
}

// Clips `h` towards the box centre until it lies inside the box (rather than clamping
// each channel, which shifts its hue).
fn clip_to_box(h: vec3<f32>, center: vec3<f32>, extent: vec3<f32>) -> vec3<f32> {
    let offset = h - center;
    let units = abs(offset / max(extent, vec3<f32>(1e-5)));
    let largest = max(units.x, max(units.y, units.z));
    return select(h, center + offset / largest, largest > 1.0);
}

@fragment
fn fs_taa(in: FullscreenOut) -> TaaOut {
    let size = vec2<i32>(textureDimensions(current));
    let p = vec2<i32>(in.position.xy);
    var here = scene_at(p, size);
    // The reflections (half resolution, smooth) only at the centre: the neighbourhood's
    // spread comes from the scene's own edges, and nine filtered taps more per pixel
    // would cost more than they tell.
    var reflected = vec3<f32>(0.0);
    if (frame.reflection.x > 0.0) {
        reflected = textureSampleLevel(reflections, history_sampler, in.uv, 0.0).rgb;
        here = vec4<f32>(max(here.rgb + reflected, vec3<f32>(0.0)), here.a);
    }
    let depth_here = linear_depth(textureLoad(depth_texture, p, 0));
    let weight = frame.taa.y;
    if (weight >= 1.0) {
        return TaaOut(here, vec4<f32>(depth_here, 1.0, 0.0, 0.0));
    }
    // The neighbourhood's mean and spread, its tent-filtered light (for pixels without a
    // history), its nearest depth (reprojecting with the nearest depth keeps a
    // foreground edge's history attached to the foreground), and its range of depths.
    var m1 = vec3<f32>(0.0);
    var m2 = vec3<f32>(0.0);
    var blurred = vec3<f32>(0.0);
    var nearest = 0.0;
    var nearest_at = p;
    var farthest = 1.0;
    for (var y = -1; y <= 1; y += 1) {
        for (var x = -1; x <= 1; x += 1) {
            let q = p + vec2<i32>(x, y);
            let light = scene_at(q, size).rgb;
            let c = encode(light);
            m1 += c;
            m2 += c * c;
            blurred += light * f32((2 - abs(x)) * (2 - abs(y)));
            let d = textureLoad(depth_texture, clamp(q, vec2<i32>(0), size - 1), 0);
            if (d > nearest) {
                nearest = d;
                nearest_at = q;
            }
            farthest = min(farthest, d);
        }
    }
    blurred = blurred / 16.0 + reflected;
    // Reverse-Z depth falls monotonically with distance.
    let depth_range = linear_depth(farthest) - linear_depth(nearest);
    // The centre's reflection shifts the whole box, so a reflective surface's history is
    // not clipped towards its unreflected neighbours.
    let shift = encode(here.rgb) - encode(scene_at(p, size).rgb);
    let mean = m1 / 9.0 + shift;
    let sigma = sqrt(max(m2 / 9.0 - (mean - shift) * (mean - shift), vec3<f32>(0.0)));

    // Where this pixel's surface was in the last frame: unproject with this frame's
    // (jittered) matrices, move back by the surface's own motion, project with last
    // frame's and this frame's unjittered ones; zero for a still camera and scene.
    let uv = in.uv;
    let at_uv = (vec2<f32>(nearest_at) + 0.5) / vec2<f32>(size);
    let world_h = frame.inv_view_proj * vec4<f32>(at_uv.x * 2.0 - 1.0, 1.0 - at_uv.y * 2.0, nearest, 1.0);
    let world = world_h.xyz / world_h.w;
    let moved = textureLoad(motion_texture, nearest_at, 0).xyz;
    let before = frame.prev_view_proj * vec4<f32>(world + moved, 1.0);
    let now = frame.view_proj_unjittered * vec4<f32>(world, 1.0);
    let motion = (before.xy / before.w - now.xy / now.w) * vec2<f32>(0.5, -0.5);
    let history_uv = uv + motion;

    // The history is only this surface's if it held a surface at the depth this one had
    // last frame: of the four texels round the spot, the closest in depth decides, with
    // a tolerance of about a pixel's worth of the local depth slope (so jitter and
    // grazing floors pass). Otherwise the surface was hidden (behind an object that has
    // moved on, or off screen) and starts afresh.
    let cap = max(round(1.0 / weight - 1.0), 0.0);
    var samples = 0.0;
    var previous = vec4<f32>(0.0);
    if (all(history_uv >= vec2<f32>(0.0)) && all(history_uv <= vec2<f32>(1.0))) {
        let expected = select(min(before.w, FAR_DEPTH), FAR_DEPTH, nearest <= 0.0);
        let tolerance = 0.005 * expected + 0.75 * depth_range + 0.25;
        let corner = vec2<i32>(floor(history_uv * vec2<f32>(size) - 0.5));
        var closest = 1e30;
        var gathered = 0.0;
        for (var i = 0; i < 4; i += 1) {
            let q = clamp(corner + vec2<i32>(i & 1, i >> 1u), vec2<i32>(0), size - 1);
            let texel = textureLoad(history_meta, q, 0);
            let difference = abs(texel.x - expected);
            if (difference < closest) {
                closest = difference;
                gathered = texel.y;
            }
        }
        if (closest <= tolerance) {
            samples = min(gathered, cap);
            previous = sample_history(history_uv, vec2<f32>(size));
        }
    }
    let count = samples + 1.0;
    // A young history is mostly the current frame and its noise (one slice of
    // occlusion, one contact-shadow march): lean on the neighbourhood instead until a
    // few samples are in.
    var light = here.rgb;
    if (samples < 3.0) {
        light = mix(here.rgb, blurred, (3.0 - samples) / 3.0 * 0.8);
    }
    if (samples <= 0.0) {
        return TaaOut(vec4<f32>(light, here.a), vec4<f32>(depth_here, count, 0.0, 0.0));
    }
    let blend = 1.0 / count;
    let clipped = clip_to_box(encode(previous.rgb), mean, sigma * frame.taa.w);
    let blended = mix(clipped, encode(light), blend);
    return TaaOut(vec4<f32>(decode(blended), mix(previous.a, here.a, blend)), vec4<f32>(depth_here, count, 0.0, 0.0));
}
