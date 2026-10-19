// Fullscreen passes after the scene: bloom + tone map + selection outline + dithering,
// then FXAA. Prefixed with common.wgsl at build time.

@group(1) @binding(0) var hdr: texture_2d<f32>;
@group(1) @binding(1) var ids: texture_2d<u32>;
@group(1) @binding(2) var depth: texture_depth_2d;
@group(1) @binding(3) var bloom: texture_2d<f32>;
@group(1) @binding(4) var tony_lut: texture_3d<f32>;
@group(1) @binding(5) var post_sampler: sampler;
// Screen-space reflections at half resolution: what to add to the scene's light.
@group(1) @binding(9) var reflections: texture_2d<f32>;

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> FullscreenOut {
    return fullscreen(index, 0.0);
}

fn is_selected(id: u32) -> bool {
    return id != 0u && (objects[id - 1u].flags & OBJECT_SELECTED) != 0u;
}

// AgX's look, matched by eye against the GL renderer's ACES on the bundled maps: plain
// AgX puts mid-grey at 0.21 of display white where that (unscaled) ACES fit puts it at
// 0.27, and its gentle contrast made lit stone read grey. Most of a stop more light
// (x1.8) and a mild version of Blender's "Punchy" look (contrast as a power on the
// sigmoid's output, a little saturation) keep maps lit for GL about as bright (mean
// luminance within 5 % on the tower) and as crisp, while highlights still roll off
// without hue skews.
const AGX_EXPOSURE: f32 = 1.8;
const AGX_POWER: f32 = 1.2;
const AGX_SATURATION: f32 = 1.1;

// AgX (Troy Sobotka's, as in Blender 4), in the polynomial fit of Benjamin Wrensch's
// "minimal AgX": inset into a wider working space, log2 encoding over 16.5 stops, the
// sigmoid as a sixth-order polynomial, the look, outset back. Returns linear display
// light.
fn agx(color: vec3<f32>) -> vec3<f32> {
    let inset = mat3x3<f32>(
        vec3<f32>(0.842479062253094, 0.0423282422610123, 0.0423756549057051),
        vec3<f32>(0.0784335999999992, 0.878468636469772, 0.0784336),
        vec3<f32>(0.0792237451477643, 0.0791661274605434, 0.879142973793104));
    let outset = mat3x3<f32>(
        vec3<f32>(1.19687900512017, -0.0528968517574562, -0.0529716355144438),
        vec3<f32>(-0.0980208811401368, 1.15190312990417, -0.0980434501171241),
        vec3<f32>(-0.0990297440797205, -0.0989611768448433, 1.15107367264116));
    let min_ev = -12.47393;
    let max_ev = 4.026069;
    var x = inset * max(color, vec3<f32>(1e-10));
    x = clamp((log2(x) - min_ev) / (max_ev - min_ev), vec3<f32>(0.0), vec3<f32>(1.0));
    let x2 = x * x;
    let x4 = x2 * x2;
    x = 15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x + 0.4298 * x2 + 0.1191 * x - 0.00232;
    x = pow(max(x, vec3<f32>(0.0)), vec3<f32>(AGX_POWER));
    let luma_agx = dot(x, vec3<f32>(0.2126, 0.7152, 0.0722));
    x = luma_agx + AGX_SATURATION * (x - luma_agx);
    x = outset * x;
    return pow(max(x, vec3<f32>(0.0)), vec3<f32>(2.2));
}

// Khronos PBR Neutral (2024): keeps base colours as authored up to 0.8, compresses
// highlights towards white with a little desaturation.
fn khronos_neutral(color_in: vec3<f32>) -> vec3<f32> {
    var color = color_in;
    let start_compression = 0.8 - 0.04;
    let desaturation = 0.15;
    let x = min(color.r, min(color.g, color.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    color -= offset;
    let peak = max(color.r, max(color.g, color.b));
    if (peak < start_compression) {
        return color;
    }
    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    color *= new_peak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, vec3<f32>(new_peak), g);
}

// Tomasz Stachowiak's TonyMcMapface, a 48^3 LUT indexed by the stimulus over itself
// plus one (MIT / Apache-2.0, bundled from its repository).
fn tony_mc_mapface(color: vec3<f32>) -> vec3<f32> {
    let encoded = color / (color + 1.0);
    let uv = encoded * (47.0 / 48.0) + 0.5 / 48.0;
    return textureSampleLevel(tony_lut, post_sampler, uv, 0.0).rgb;
}

fn tonemap(light: vec3<f32>) -> vec3<f32> {
    let exposed = max(light, vec3<f32>(0.0)) * frame.params0.w;
    let mapper = u32(frame.post.x);
    if (mapper == 1u) {
        return agx(exposed * AGX_EXPOSURE);
    } else if (mapper == 2u) {
        return khronos_neutral(exposed);
    } else if (mapper == 3u) {
        return tony_mc_mapface(exposed);
    }
    return aces(exposed);
}

// Triangular noise of one 8-bit step, added before quantising, so smooth gradients (the
// sky, a lit wall) do not band.
fn dither(p: vec2<f32>) -> vec3<f32> {
    let a = interleaved_gradient_noise(p);
    let b = interleaved_gradient_noise(p + vec2<f32>(19.0, 47.0));
    return vec3<f32>((a - b) / 255.0);
}

fn scene_light(p: vec2<i32>, uv: vec2<f32>) -> vec4<f32> {
    var light = textureLoad(hdr, p, 0);
    // Unless the temporal filter's output, which this reads then, has them already.
    if (frame.reflection.x > 0.0 && frame.taa.z < 0.5) {
        light = vec4<f32>(light.rgb + textureSampleLevel(reflections, post_sampler, uv, 0.0).rgb, light.a);
    }
    if (frame.post.y > 0.0) {
        light = vec4<f32>(light.rgb + textureSampleLevel(bloom, post_sampler, uv, 0.0).rgb * frame.post.y, light.a);
    }
    return light;
}

fn view_depth_at(p: vec2<i32>) -> f32 {
    let d = textureLoad(depth, p, 0);
    if (d <= 0.0) {
        return 1e9;
    }
    return -view_position((vec2<f32>(p) + 0.5) * frame.viewport.zw, d).z;
}

// The blur radius in pixels for something `z` units away, as the GL renderer's depth of
// field has it: zero inside the sharp range, growing with the distance past it.
fn blur_radius(z: f32) -> f32 {
    let error = max(abs(z - frame.dof.x) - frame.dof.y * 0.5, 0.0);
    return frame.dof.z * clamp(error / max(z, 1.0), 0.0, 1.0) * frame.viewport.y / 1080.0;
}

// A 32-tap disc gather in linear HDR light, before the tone map; a sharper foreground
// sample is kept out of a blurred background (the GL renderer's rule). The disc turns
// with each accumulation sample, so an offline render converges to a smooth bokeh.
fn depth_of_field(p: vec2<i32>, uv: vec2<f32>, light: vec4<f32>) -> vec4<f32> {
    let z = view_depth_at(p);
    let radius = blur_radius(z);
    if (radius <= 0.05) {
        return light;
    }
    let size = vec2<i32>(textureDimensions(hdr));
    var sum = light.rgb;
    var weight = 1.0;
    let turn = frame.misc.z * 2.39996323;
    for (var i = 0; i < 32; i += 1) {
        let r = sqrt((f32(i) + 0.5) / 32.0);
        let angle = f32(i) * 2.39996323 + turn;
        let offset = vec2<f32>(cos(angle), sin(angle)) * r * radius;
        let q = clamp(p + vec2<i32>(round(offset)), vec2<i32>(0), size - 1);
        let sample_depth = view_depth_at(q);
        var accept = 1.0;
        if (sample_depth < z * 0.98) {
            accept = smoothstep(r * radius - 0.5, r * radius + 0.5, blur_radius(sample_depth));
        }
        sum += scene_light(q, (vec2<f32>(q) + 0.5) * frame.viewport.zw).rgb * accept;
        weight += accept;
    }
    return vec4<f32>(sum / weight, light.a);
}

// HDR to display: bloom, exposure, the chosen tone map and the sRGB curve; the
// background colour where nothing was drawn and there is no sky; a screen-space outline
// round the selected objects from the object-id target; dithering.
@fragment
fn fs_resolve(in: FullscreenOut) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(hdr));
    let p = clamp(vec2<i32>(in.position.xy), vec2<i32>(0), size - 1);
    var light = scene_light(p, in.uv);
    if (frame.dof.z > 0.0) {
        light = depth_of_field(p, in.uv, light);
    }
    var color: vec3<f32>;
    let d = textureLoad(depth, p, 0);
    if (d <= 0.0 && frame.params1.y < 0.5) {
        color = frame.background.rgb;
        // Blended surfaces (and bloom) over the background: their light is in the HDR
        // target.
        if (light.a > 0.0) {
            color = mix(color, linear_to_srgb(tonemap(light.rgb)), light.a);
        }
    } else {
        color = linear_to_srgb(tonemap(light.rgb));
    }

    let width = i32(frame.params1.w);
    if (width > 0) {
        let center = textureLoad(ids, p, 0).r;
        if (!is_selected(center)) {
            var hit = false;
            for (var y = -width; y <= width; y += 1) {
                for (var x = -width; x <= width; x += 1) {
                    if (x * x + y * y > width * width) {
                        continue;
                    }
                    let q = clamp(p + vec2<i32>(x, y), vec2<i32>(0), size - 1);
                    // Only where the selection is in front: an unselected surface
                    // nearer the camera (reverse-Z: larger depth) hides the outline.
                    if (is_selected(textureLoad(ids, q, 0).r) && d <= textureLoad(depth, q, 0) + 1e-6) {
                        hit = true;
                    }
                }
            }
            if (hit) {
                color = frame.selected_color.rgb;
            }
        }
    }
    return vec4<f32>(color + dither(in.position.xy), 1.0);
}

@group(1) @binding(6) var ldr: texture_2d<f32>;
@group(1) @binding(7) var linear_sampler: sampler;

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@group(1) @binding(8) var accumulated: texture_2d<f32>;

// One sample of an offline render, back to linear light, blended into the running
// average by the blend constant (1 / samples so far).
@fragment
fn fs_accumulate(in: FullscreenOut) -> @location(0) vec4<f32> {
    let c = textureLoad(ldr, vec2<i32>(in.position.xy), 0).rgb;
    return vec4<f32>(srgb_to_linear(c), 1.0);
}

// The running average back to the display curve, dithered, into the output.
@fragment
fn fs_accumulated(in: FullscreenOut) -> @location(0) vec4<f32> {
    let c = textureLoad(accumulated, vec2<i32>(in.position.xy), 0).rgb;
    return vec4<f32>(linear_to_srgb(c) + dither(in.position.xy), 1.0);
}

// The GL renderer's FXAA, for a view drawn without the temporal filter (kept for the
// GLES path, P5, where its history may cost too much; the view uses the filter whenever
// antialiasing or accumulation is on); a plain copy for an offline render's samples.
@fragment
fn fs_fxaa(in: FullscreenOut) -> @location(0) vec4<f32> {
    let texel = frame.viewport.zw;
    let uv = in.uv;
    let middle = textureSampleLevel(ldr, linear_sampler, uv, 0.0).rgb;
    if (frame.misc.x < 0.5) {
        return vec4<f32>(middle, 1.0);
    }
    let luma_middle = luma(middle);
    let luma_north = luma(textureSampleLevel(ldr, linear_sampler, uv + vec2<f32>(0.0, -texel.y), 0.0).rgb);
    let luma_south = luma(textureSampleLevel(ldr, linear_sampler, uv + vec2<f32>(0.0, texel.y), 0.0).rgb);
    let luma_west = luma(textureSampleLevel(ldr, linear_sampler, uv + vec2<f32>(-texel.x, 0.0), 0.0).rgb);
    let luma_east = luma(textureSampleLevel(ldr, linear_sampler, uv + vec2<f32>(texel.x, 0.0), 0.0).rgb);
    let lowest = min(luma_middle, min(min(luma_north, luma_south), min(luma_west, luma_east)));
    let highest = max(luma_middle, max(max(luma_north, luma_south), max(luma_west, luma_east)));
    let range = highest - lowest;
    if (range < max(0.0312, highest * 0.125)) {
        return vec4<f32>(middle, 1.0);
    }
    let horizontal = abs(luma_west + luma_east - 2.0 * luma_middle);
    let vertical = abs(luma_north + luma_south - 2.0 * luma_middle);
    let along = select(vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vertical >= horizontal);
    let average = (luma_north + luma_south + luma_west + luma_east) * 0.25;
    let blend = clamp(abs(average - luma_middle) / range, 0.0, 1.0) * 0.75;
    let blurred = 0.5 * (textureSampleLevel(ldr, linear_sampler, uv + along * texel, 0.0).rgb
        + textureSampleLevel(ldr, linear_sampler, uv - along * texel, 0.0).rgb);
    return vec4<f32>(mix(middle, 0.5 * (middle + blurred), blend), 1.0);
}
