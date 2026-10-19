// Mip-chain bloom (Jimenez, "Next Generation Post Processing in Call of Duty: Advanced
// Warfare", as Bevy does it): a 13-tap downsample from the HDR picture through a chain of
// half-size mips, the first with a soft threshold and Karis averaging against fireflies,
// then a 3x3 tent upsample added back up the chain. Prefixed with common.wgsl.

@group(1) @binding(0) var source: texture_2d<f32>;
@group(1) @binding(1) var source_sampler: sampler;

@vertex
fn vs_bloom(@builtin(vertex_index) index: u32) -> FullscreenOut {
    return fullscreen(index, 0.0);
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn karis_weight(c: vec3<f32>) -> f32 {
    return 1.0 / (1.0 + luminance(c));
}

// The GL renderer's soft knee: what passes the threshold fades in over `knee`.
fn threshold(c: vec3<f32>) -> vec3<f32> {
    return c * smoothstep(frame.post.z, frame.post.z + frame.post.w, luminance(c));
}

fn tap(uv: vec2<f32>, dx: f32, dy: f32, texel: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(source, source_sampler, uv + vec2<f32>(dx, dy) * texel, 0.0).rgb;
}

fn downsample(uv: vec2<f32>, first: bool) -> vec3<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(source));
    let a = tap(uv, -2.0, -2.0, texel);
    let b = tap(uv, 0.0, -2.0, texel);
    let c = tap(uv, 2.0, -2.0, texel);
    let d = tap(uv, -2.0, 0.0, texel);
    let e = tap(uv, 0.0, 0.0, texel);
    let f = tap(uv, 2.0, 0.0, texel);
    let g = tap(uv, -2.0, 2.0, texel);
    let h = tap(uv, 0.0, 2.0, texel);
    let i = tap(uv, 2.0, 2.0, texel);
    let j = tap(uv, -1.0, -1.0, texel);
    let k = tap(uv, 1.0, -1.0, texel);
    let l = tap(uv, -1.0, 1.0, texel);
    let m = tap(uv, 1.0, 1.0, texel);
    if (first) {
        // Five overlapping 2x2 boxes, each weighted by its brightness (Karis), so one hot
        // pixel cannot flicker into a large glow.
        var groups = array<vec3<f32>, 5>(
            (a + b + d + e) * 0.25, (b + c + e + f) * 0.25, (d + e + g + h) * 0.25,
            (e + f + h + i) * 0.25, (j + k + l + m) * 0.25);
        var total = vec3<f32>(0.0);
        var weights = 0.0;
        let box_weights = array<f32, 5>(0.125, 0.125, 0.125, 0.125, 0.5);
        for (var n = 0; n < 5; n += 1) {
            let value = threshold(groups[n]);
            let w = karis_weight(value) * box_weights[n];
            total += value * w;
            weights += w;
        }
        return total / max(weights, 1e-5);
    }
    return e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
}

@fragment
fn fs_downsample_first(in: FullscreenOut) -> @location(0) vec4<f32> {
    return vec4<f32>(downsample(in.uv, true), 1.0);
}

@fragment
fn fs_downsample(in: FullscreenOut) -> @location(0) vec4<f32> {
    return vec4<f32>(downsample(in.uv, false), 1.0);
}

// The 3x3 tent over the smaller mip, added (by the blend state) to the larger one.
@fragment
fn fs_upsample(in: FullscreenOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(source));
    let uv = in.uv;
    var total = tap(uv, 0.0, 0.0, texel) * 4.0;
    total += (tap(uv, -1.0, 0.0, texel) + tap(uv, 1.0, 0.0, texel) + tap(uv, 0.0, -1.0, texel) + tap(uv, 0.0, 1.0, texel)) * 2.0;
    total += tap(uv, -1.0, -1.0, texel) + tap(uv, 1.0, -1.0, texel) + tap(uv, -1.0, 1.0, texel) + tap(uv, 1.0, 1.0, texel);
    return vec4<f32>(total / 16.0, 1.0);
}
