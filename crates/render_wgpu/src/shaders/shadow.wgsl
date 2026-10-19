// Shadow-map depth, honoring imported face rules and mask cutoff.
// Prefixed with common.wgsl.

@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;

struct Cascade {
    view_proj: mat4x4<f32>,
};
@group(2) @binding(0) var<uniform> cascade: Cascade;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(4) object: u32,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material: u32,
};

@vertex
fn vs_shadow(v: VertexIn, @builtin(instance_index) material: u32) -> VertexOut {
    var out: VertexOut;
    let object = objects[v.object & 0x00ffffffu];
    out.uv = v.uv;
    out.material = material;
    if ((object.flags & OBJECT_HIDDEN) != 0u) {
        out.clip = vec4<f32>(0.0, 0.0, -2.0, 1.0);
    } else {
        out.clip = cascade.view_proj * vec4<f32>(v.position, 1.0);
        // Casters behind the map's near edge are flattened onto it rather than clipped,
        // so they still shadow ("pancaking").
        out.clip.z = max(out.clip.z, 0.0);
    }
    return out;
}

// A face of a light's shadow cube: as above, but clipped at the near plane rather than
// pancaked, since what is nearer than the cube's near plane is the light's own fitting.
@vertex
fn vs_shadow_cube(v: VertexIn, @builtin(instance_index) material: u32) -> VertexOut {
    var out: VertexOut;
    let object = objects[v.object & 0x00ffffffu];
    out.uv = v.uv;
    out.material = material;
    if ((object.flags & OBJECT_HIDDEN) != 0u) {
        out.clip = vec4<f32>(0.0, 0.0, -2.0, 1.0);
    } else {
        out.clip = cascade.view_proj * vec4<f32>(v.position, 1.0);
    }
    return out;
}

@fragment
fn fs_shadow_masked(in: VertexOut, @builtin(front_facing) front: bool) {
    let m = materials[in.material];
    let sampled = textureSample(albedo, albedo_sampler, in.uv / max(m.size, vec2<f32>(1.0)));
    if ((m.model_params.w == 1.0 && !front) || sampled.a < m.model_params.z) {
        discard;
    }
}

@fragment
fn fs_shadow_opaque(in: VertexOut, @builtin(front_facing) front: bool) {
    if (materials[in.material].model_params.w == 1.0 && !front) { discard; }
}
