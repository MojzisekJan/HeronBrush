// Ground-truth ambient occlusion (Jimenez et al. 2016, after Intel's XeGTAO) at full
// resolution (half on very large views), then a depth-aware blur. Both write (occlusion, view depth) so the
// forward pass can upsample without bleeding across silhouettes. Prefixed with
// common.wgsl.

@group(1) @binding(0) var depth_texture: texture_depth_2d;
@group(1) @binding(1) var normal_texture: texture_2d<f32>;
@group(1) @binding(2) var occlusion_texture: texture_2d<f32>;

@vertex
fn vs_ao(@builtin(vertex_index) index: u32) -> FullscreenOut {
    return fullscreen(index, 0.0);
}

fn full_size() -> vec2<i32> {
    return vec2<i32>(textureDimensions(depth_texture));
}

fn view_at(p: vec2<i32>) -> vec3<f32> {
    let q = clamp(p, vec2<i32>(0), full_size() - 1);
    let d = textureLoad(depth_texture, q, 0);
    return view_position((vec2<f32>(q) + 0.5) * frame.viewport.zw, d);
}

const STEPS: u32 = 6u;

@fragment
fn fs_gtao(in: FullscreenOut) -> @location(0) vec4<f32> {
    // The full-resolution texel this occlusion pixel stands for.
    let half_pixel = vec2<i32>(in.position.xy);
    let p = min(half_pixel * i32(frame.ao.x), full_size() - 1);
    let slices = max(u32(frame.ao.y), 1u);
    let d = textureLoad(depth_texture, p, 0);
    if (d <= 0.0) {
        return vec4<f32>(1.0, 60000.0, 0.0, 1.0);
    }
    let position = view_at(p);
    let view_depth = -position.z;
    let world_normal = octahedral_decode(textureLoad(normal_texture, p, 0).xy);
    let normal = normalize((frame.view * vec4<f32>(world_normal, 0.0)).xyz);
    let v = normalize(-position);

    // The radius in full-resolution pixels at this depth.
    let radius = frame.effects.z;
    let pixels = radius * frame.proj[1][1] * 0.5 * frame.viewport.y / max(view_depth, 1e-3);
    if (pixels < 1.0) {
        return vec4<f32>(1.0, view_depth, 0.0, 1.0);
    }
    let reach = min(pixels, 128.0);
    // Falloff: full weight to 60 % of the radius, none past it.
    let falloff_range = 0.4 * radius;
    let falloff_from = radius - falloff_range;

    // Shifted every frame under the temporal filter, which then averages the slices
    // of several frames.
    let shift = vec2<f32>(5.588238) * frame.misc.w;
    let noise_angle = interleaved_gradient_noise(vec2<f32>(half_pixel) + shift);
    let noise_step = interleaved_gradient_noise(vec2<f32>(half_pixel) + vec2<f32>(37.0, 17.0) + shift);
    var visibility = 0.0;
    for (var slice = 0u; slice < slices; slice += 1u) {
        let phi = (f32(slice) + noise_angle) * PI / f32(slices);
        let direction = vec3<f32>(cos(phi), sin(phi), 0.0);
        // Screen y points down, view y up.
        let omega = vec2<f32>(direction.x, -direction.y);
        let ortho = direction - v * dot(direction, v);
        let axis = normalize(cross(direction, v));
        let projected = normal - axis * dot(normal, axis);
        let projected_length = length(projected);
        let sign_n = sign(dot(ortho, projected));
        let cos_n = clamp(dot(projected, v) / max(projected_length, 1e-4), 0.0, 1.0);
        let n = sign_n * acos(cos_n);
        let low0 = cos(n + PI * 0.5);
        let low1 = cos(n - PI * 0.5);
        var horizon0 = low0;
        var horizon1 = low1;
        for (var step = 0u; step < STEPS; step += 1u) {
            var s = (f32(step) + noise_step) / f32(STEPS);
            s = s * s;
            let offset = omega * max(s * reach, f32(step) + 1.0);
            let sample0 = view_at(p + vec2<i32>(round(offset)));
            let sample1 = view_at(p - vec2<i32>(round(offset)));
            let delta0 = sample0 - position;
            let delta1 = sample1 - position;
            let distance0 = length(delta0);
            let distance1 = length(delta1);
            let weight0 = clamp((falloff_from + falloff_range - distance0) / falloff_range, 0.0, 1.0);
            let weight1 = clamp((falloff_from + falloff_range - distance1) / falloff_range, 0.0, 1.0);
            let cos0 = mix(low0, dot(delta0 / max(distance0, 1e-4), v), weight0);
            let cos1 = mix(low1, dot(delta1 / max(distance1, 1e-4), v), weight1);
            horizon0 = max(horizon0, cos0);
            horizon1 = max(horizon1, cos1);
        }
        var h0 = -acos(clamp(horizon1, -1.0, 1.0));
        var h1 = acos(clamp(horizon0, -1.0, 1.0));
        h0 = n + clamp(h0 - n, -PI * 0.5, PI * 0.5);
        h1 = n + clamp(h1 - n, -PI * 0.5, PI * 0.5);
        let arc0 = (cos_n + 2.0 * h0 * sin(n) - cos(2.0 * h0 - n)) * 0.25;
        let arc1 = (cos_n + 2.0 * h1 * sin(n) - cos(2.0 * h1 - n)) * 0.25;
        visibility += projected_length * (arc0 + arc1);
    }
    return vec4<f32>(clamp(visibility / f32(slices), 0.0, 1.0), view_depth, 0.0, 1.0);
}

// A 4x4 depth-aware box over the half-resolution occlusion: the noise of the slice and
// step jitter is a 4x4 pattern, so this takes it out without spreading across edges.
@fragment
fn fs_denoise(in: FullscreenOut) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(occlusion_texture));
    let p = vec2<i32>(in.position.xy);
    let center = textureLoad(occlusion_texture, p, 0);
    if (center.g >= 50000.0) {
        return center;
    }
    var total = 0.0;
    var weights = 0.0;
    for (var y = -2; y <= 1; y += 1) {
        for (var x = -2; x <= 1; x += 1) {
            let q = clamp(p + vec2<i32>(x, y), vec2<i32>(0), size - 1);
            let texel = textureLoad(occlusion_texture, q, 0);
            let w = 1.0 / (1e-3 + abs(texel.g - center.g) / max(center.g, 1.0) * 40.0);
            total += texel.r * w;
            weights += w;
        }
    }
    return vec4<f32>(total / weights, center.g, 0.0, 1.0);
}
