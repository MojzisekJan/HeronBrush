// Shows the finished picture in a host's render pass (egui's), into whatever viewport the
// host set. The picture is display-encoded (the sRGB curve); an sRGB target wants it
// linear, and encodes it again on write, so the bytes arrive unchanged.

@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var picture_sampler: sampler;

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blit(@builtin(vertex_index) index: u32) -> Out {
    var out: Out;
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_blit_gamma(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(picture, picture_sampler, in.uv).rgb, 1.0);
}

@fragment
fn fs_blit_linear(in: Out) -> @location(0) vec4<f32> {
    let c = textureSample(picture, picture_sampler, in.uv).rgb;
    return vec4<f32>(select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045)), 1.0);
}
