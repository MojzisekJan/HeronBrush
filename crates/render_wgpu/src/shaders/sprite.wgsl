// Point-entity sprites: camera-facing quads of the entity's `iconsprite`, drawn into the
// finished (display-encoded) picture against the scene depth without writing it, as the
// GL renderer draws them. Prefixed with common.wgsl.

@group(1) @binding(0) var sprite: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct SpriteIn {
    @location(0) position: vec3<f32>,
    @location(1) half_size: vec2<f32>,
};

struct SpriteOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_sprite(s: SpriteIn, @builtin(vertex_index) index: u32) -> SpriteOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    let corner = corners[index];
    // The camera's right and up axes are the view matrix's first two rows.
    let right = vec3<f32>(frame.view[0][0], frame.view[1][0], frame.view[2][0]);
    let up = vec3<f32>(frame.view[0][1], frame.view[1][1], frame.view[2][1]);
    let world = s.position + right * corner.x * s.half_size.x + up * corner.y * s.half_size.y;
    var out: SpriteOut;
    out.clip = frame.view_proj_unjittered * vec4<f32>(world, 1.0);
    out.uv = vec2<f32>(corner.x * 0.5 + 0.5, 0.5 - corner.y * 0.5);
    return out;
}

@fragment
fn fs_sprite(in: SpriteOut) -> @location(0) vec4<f32> {
    let texel = textureSample(sprite, sprite_sampler, in.uv);
    // The texture is sampled linear; the picture is sRGB-encoded.
    return vec4<f32>(linear_to_srgb(texel.rgb), texel.a);
}
