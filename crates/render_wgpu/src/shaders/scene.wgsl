// The scene geometry: depth/id/normal prepass, forward PBR pass, wireframe, and the sky.
// Prefixed with common.wgsl at build time.

@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;
@group(1) @binding(2) var normal_map: texture_2d<f32>;
@group(1) @binding(3) var model_orm: texture_2d<f32>;
@group(1) @binding(4) var model_emissive: texture_2d<f32>;

// What the forward pass is lit by (group 2 is used by fs_forward only).
@group(2) @binding(0) var shadow_map: texture_depth_2d_array;
@group(2) @binding(1) var shadow_sampler: sampler_comparison;
@group(2) @binding(2) var specular_env: texture_cube<f32>;
@group(2) @binding(3) var sh_coefficients: texture_2d<f32>;
@group(2) @binding(4) var dfg_lut: texture_2d<f32>;
@group(2) @binding(5) var env_sampler: sampler;
@group(2) @binding(6) var ambient_occlusion: texture_2d<f32>;

// Light entities (see lights.rs): the table, each cluster's (offset, count) into the
// index list, the index list, and the shadow cubes.
struct Light {
    // xyz position, w reach.
    position_range: vec4<f32>,
    // rgb colour times energy, w the falloff exponent in metres.
    color_decay: vec4<f32>,
    // xyz spot direction, w cosine of the half-angle (-2 for a point light).
    direction_cutoff: vec4<f32>,
    // x the cone's rim exponent, y shadow cube slot (-1 none), z map units per metre.
    params: vec4<f32>,
};
@group(2) @binding(7) var<storage, read> lights: array<Light>;
@group(2) @binding(8) var<storage, read> light_clusters: array<vec2<u32>>;
@group(2) @binding(9) var<storage, read> light_indices: array<u32>;
@group(2) @binding(10) var light_shadows: texture_depth_cube_array;
// The prepass depth, for contact shadows (the pass holds it as a read-only attachment).
@group(2) @binding(11) var scene_depth: texture_depth_2d;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) object: u32,
};

struct VertexOut {
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    // object slot + 1 (0 is "nothing"), vertex flags, material slot.
    @location(4) @interpolate(flat) ids: vec3<u32>,
};

@vertex
fn vs_main(v: VertexIn, @builtin(instance_index) material: u32) -> VertexOut {
    return scene_vertex(v, material);
}

fn scene_vertex(v: VertexIn, material: u32) -> VertexOut {
    var out: VertexOut;
    let slot = v.object & 0x00ffffffu;
    let vflags = v.object >> 24u;
    let object = objects[slot];
    out.world = v.position;
    out.normal = v.normal.xyz;
    out.uv = v.uv;
    // The selection and lock tints, on the vertex colour as the GL renderer bakes them.
    var color = v.color.rgb;
    let selected = ((object.flags & OBJECT_SELECTED) != 0u && (vflags & NO_TINT) == 0u) || (vflags & FACE_SELECTED) != 0u;
    if (selected) {
        color = mix(color, frame.selected_color.rgb, 0.35);
    } else if ((object.flags & OBJECT_LOCKED) != 0u) {
        color = mix(color, frame.locked_color.rgb, 0.35);
    }
    out.color = vec4<f32>(color, v.color.a);
    out.ids = vec3<u32>(slot + 1u, vflags, material);
    if ((object.flags & OBJECT_HIDDEN) != 0u) {
        out.clip = vec4<f32>(0.0, 0.0, -2.0, 1.0);
    } else {
        out.clip = frame.view_proj * vec4<f32>(v.position, 1.0);
    }
    return out;
}

// What the albedo texture gives at a texel, or -- while the texture is still being
// streamed in -- the material's average colour (sRGB bytes packed in factors.w).
fn albedo_texel(m: Material, sampled: vec4<f32>) -> vec4<f32> {
    if ((m.flags & MATERIAL_PENDING) == 0u) {
        return sampled;
    }
    let average = unpack4x8unorm(bitcast<u32>(m.factors.w));
    let c = average.rgb;
    let linear = select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
    return vec4<f32>(linear, average.a);
}

fn material_uv(m: Material, uv: vec2<f32>) -> vec2<f32> {
    return uv / max(m.size, vec2<f32>(1.0));
}

// The surface normal facing the viewer (patches have no inside; decided by the view
// direction rather than by winding, which the APIs disagree on), then bent by the normal
// map with a tangent frame derived from the face's UVs as the GL renderer does, which
// handles mirrored mappings. `sampled` is the normal-map texel, `dp*`/`duv*` the screen
// derivatives of the position and UV, taken by the caller in uniform control flow.
fn surface_normal(in: VertexOut, m: Material, sampled: vec3<f32>, dpx: vec3<f32>, dpy: vec3<f32>, duvx: vec2<f32>, duvy: vec2<f32>, front: bool) -> vec3<f32> {
    var n = normalize(in.normal);
    let face_sign = select(1.0, -1.0, m.model_params.w == 2.0 && !front);
    if (m.model_params.w == 0.0 && dot(n, frame.eye.xyz - in.world) < 0.0) {
        n = -n;
    }
    if ((m.flags & MATERIAL_NORMAL_MAP) == 0u) {
        return n * face_sign;
    }
    let determinant = duvx.x * duvy.y - duvx.y * duvy.x;
    if (abs(determinant) < 1e-12) {
        return n * face_sign;
    }
    var t = (dpx * duvy.y - dpy * duvx.y) / determinant;
    let b = (dpy * duvx.x - dpx * duvy.x) / determinant;
    t -= n * dot(n, t);
    if (dot(t, t) < 1e-12) {
        return n * face_sign;
    }
    t = normalize(t);
    let handed = select(1.0, -1.0, dot(cross(n, t), b) < 0.0);
    let bitangent = normalize(cross(n, t)) * handed;
    var s = sampled * 2.0 - 1.0;
    s = vec3<f32>(s.xy * m.model_params.x,s.z);
    if ((m.flags & MATERIAL_NORMAL_FLIP_Y) != 0u) {
        s.y = -s.y;
    }
    let result = mat3x3<f32>(t, bitangent, n) * s;
    return select(n, normalize(result), dot(result, result) > 1e-8) * face_sign;
}

// The prepass reads each vertex twice: where it is, and (a second vertex buffer)
// where it was last frame, which is the same buffer unless the object moved.
struct PrepassVertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) object: u32,
    @location(5) previous: vec3<f32>,
};

struct PrepassVertexOut {
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) @interpolate(flat) ids: vec3<u32>,
    // Where this point of the surface was last frame, minus where it is (world units).
    @location(5) motion: vec3<f32>,
};

@vertex
fn vs_prepass(v: PrepassVertexIn, @builtin(instance_index) material: u32) -> PrepassVertexOut {
    let out = scene_vertex(VertexIn(v.position, v.normal, v.uv, v.color, v.object), material);
    return PrepassVertexOut(out.clip, out.world, out.normal, out.uv, out.color, out.ids, v.previous - v.position);
}

struct PrepassOut {
    @location(0) id: u32,
    @location(1) normal: vec2<f32>,
    // The surface's motion since last frame in world space (zero where nothing moved):
    // with the camera's two view-projections it gives the temporal filter the exact
    // place a pixel's surface was, whatever moved.
    @location(2) motion: vec4<f32>,
};

// Depth, object id, the shading normal (octahedral, world space) for ambient occlusion,
// and the object motion for the temporal filter. Masked materials drop their clear
// texels here too, so a fence does not occlude what is behind its holes.
@fragment
fn fs_prepass(prepass: PrepassVertexOut, @builtin(front_facing) front: bool) -> PrepassOut {
    let in = VertexOut(prepass.clip, prepass.world, prepass.normal, prepass.uv, prepass.color, prepass.ids);
    let m = materials[in.ids.z];
    let uv = material_uv(m, in.uv);
    let sampled = textureSample(albedo, albedo_sampler, uv);
    let bumps = textureSample(normal_map, albedo_sampler, uv).xyz;
    let n = surface_normal(in, m, bumps, dpdx(in.world), dpdy(in.world), dpdx(uv), dpdy(uv), front);
    if (m.model_params.w == 1.0 && !front) { discard; }
    if (m.alpha_mode == 1u && sampled.a < m.model_params.z) {
        discard;
    }
    return PrepassOut(in.ids.x, octahedral_encode(n), vec4<f32>(prepass.motion, 0.0));
}

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / max(PI * d * d, 1e-6);
}

fn geometry_schlick(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// A 16-tap Poisson disc; each tap is itself a 2x2 hardware PCF lookup, so the edge is
// smooth without per-pixel noise (there is no temporal filter yet to hide noise).
var<private> SHADOW_DISC: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>(-0.94201624, -0.39906216), vec2<f32>(0.94558609, -0.76890725),
    vec2<f32>(-0.09418410, -0.92938870), vec2<f32>(0.34495938, 0.29387760),
    vec2<f32>(-0.91588581, 0.45771432), vec2<f32>(-0.81544232, -0.87912464),
    vec2<f32>(-0.38277543, 0.27676845), vec2<f32>(0.97484398, 0.75648379),
    vec2<f32>(0.44323325, -0.97511554), vec2<f32>(0.53742981, -0.47373420),
    vec2<f32>(-0.26496911, -0.41893023), vec2<f32>(0.79197514, 0.19090188),
    vec2<f32>(-0.24188840, 0.99706507), vec2<f32>(-0.81409955, 0.91437590),
    vec2<f32>(0.19984126, 0.78641367), vec2<f32>(0.14383161, -0.14100790));

fn cascade_lookup(world: vec3<f32>, n: vec3<f32>, n_dot_l: f32, cascade: u32) -> f32 {
    // Normal offset: push the lookup off the surface by about a texel, more when the
    // light grazes it, so a surface does not shadow itself in stripes.
    let texel = frame.cascade_texel[cascade];
    let offset = n * texel * frame.shadow.y * (0.6 + 1.4 * (1.0 - n_dot_l));
    let p = frame.cascades[cascade] * vec4<f32>(world + offset, 1.0);
    let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || p.z > 1.0) {
        return 1.0;
    }
    let reference = p.z - frame.cascade_bias[cascade];
    let radius = frame.shadow.z * frame.shadow.w;
    var lit = 0.0;
    for (var i = 0u; i < 16u; i += 1u) {
        lit += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + SHADOW_DISC[i] * radius, i32(cascade), reference);
    }
    return lit / 16.0;
}

// The sun's visibility from the cascaded shadow maps: the first cascade whose range
// holds the point, blended into the next over the last tenth of its range so the
// change of resolution has no seam; the last cascade fades out to lit.
fn sun_visibility(world: vec3<f32>, n: vec3<f32>, n_dot_l: f32) -> f32 {
    if (frame.shadow.x < 0.5 || n_dot_l <= 0.0) {
        return 1.0;
    }
    let depth = dot(world - frame.eye.xyz, frame.forward.xyz);
    var cascade = 0u;
    while (cascade < 4u && depth > frame.cascade_splits[cascade]) {
        cascade += 1u;
    }
    if (cascade >= 4u) {
        return 1.0;
    }
    var visibility = cascade_lookup(world, n, n_dot_l, cascade);
    let far = frame.cascade_splits[cascade];
    let near = select(0.0, frame.cascade_splits[max(cascade, 1u) - 1u], cascade > 0u);
    let band = (far - near) * 0.1;
    let t = (depth - (far - band)) / max(band, 1e-4);
    if (t > 0.0) {
        var next = 1.0;
        if (cascade < 3u) {
            next = cascade_lookup(world, n, n_dot_l, cascade + 1u);
        }
        visibility = mix(visibility, next, clamp(t, 0.0, 1.0));
    }
    return visibility;
}

// Short screen-space shadows where surfaces meet, which the cascades are too coarse for:
// a ray from the surface towards the sun, marched through the prepass depth in view
// space. A sample counts as blocked when the depth buffer holds a surface in front of
// it, but not so far in front that it is another surface altogether (GL's rule); the
// thickness grows with distance as depth precision falls. The start is jittered per
// pixel, which a temporal filter turns into a soft edge.
fn contact_shadow(world: vec3<f32>, n: vec3<f32>, pixel: vec2<f32>) -> f32 {
    let steps = max(u32(frame.contact.z), 1u);
    let origin_view = (frame.view * vec4<f32>(world, 1.0)).xyz;
    let thickness = max(0.25, -origin_view.z * 0.002);
    let normal_view = normalize((frame.view * vec4<f32>(n, 0.0)).xyz);
    let direction = normalize((frame.view * vec4<f32>(-frame.light_dir.xyz, 0.0)).xyz);
    let origin = origin_view + normal_view * thickness;
    let step_length = frame.contact.y / f32(steps);
    let jitter = interleaved_gradient_noise(pixel + vec2<f32>(frame.misc.w * 7.0, frame.misc.w * 13.0));
    let size = vec2<i32>(textureDimensions(scene_depth));
    for (var i = 0u; i < steps; i += 1u) {
        let at = origin + direction * (step_length * (f32(i) + jitter));
        if (at.z > -0.01) {
            break;
        }
        let clip = frame.proj * vec4<f32>(at, 1.0);
        let ndc = clip.xy / clip.w;
        let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
            break;
        }
        let q = min(vec2<i32>(uv * vec2<f32>(size)), size - 1);
        let d = textureLoad(scene_depth, q, 0);
        if (d <= 0.0) {
            continue;
        }
        let stored = view_position((vec2<f32>(q) + 0.5) / vec2<f32>(size), d).z;
        let difference = stored - at.z;
        if (difference > thickness && difference < thickness * 40.0) {
            return 0.0;
        }
    }
    return 1.0;
}

// The cluster a fragment at `pixel` and view depth `depth` falls in.
fn light_cluster(pixel: vec2<f32>, depth: f32) -> u32 {
    let grid = vec3<u32>(frame.lights.yzw);
    let x = min(u32(max(pixel.x, 0.0) / frame.cluster.x), grid.x - 1u);
    let y = min(u32(max(pixel.y, 0.0) / frame.cluster.y), grid.y - 1u);
    let slice = floor(log2(max(depth, 1e-3)) * frame.cluster.z - frame.cluster.w);
    let z = u32(clamp(slice, 0.0, f32(grid.z) - 1.0));
    return (z * grid.y + y) * grid.x + x;
}

// How much of a light reaches `world` past what its shadow cube holds: the depth the
// cube's face stores for this direction against this point's, offset off the surface by
// about a texel and filtered with eight hardware 2x2 PCF taps round the direction.
fn light_visibility(slot: i32, light_position: vec3<f32>, range: f32, world: vec3<f32>, n: vec3<f32>) -> f32 {
    let from_light = world - light_position;
    let axis = max(abs(from_light.x), max(abs(from_light.y), abs(from_light.z)));
    // A texel of the face at this distance, in world units.
    let texel = 2.0 * axis * frame.point_shadow.y;
    let d = from_light + n * texel * 1.5 * max(frame.point_shadow.w, 0.25);
    let major = max(abs(d.x), max(abs(d.y), abs(d.z)));
    let near = frame.point_shadow.z;
    let far = max(range, near * 2.0);
    let reference = far / (far - near) - far * near / ((far - near) * major) - 1e-5;
    // Two axes across the direction, for the filter taps.
    let dir = normalize(d);
    let helper = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(dir.z) > 0.9);
    let across = normalize(cross(dir, helper));
    let up = cross(dir, across);
    let radius = 1.5 * frame.point_shadow.y * 2.0;
    var lit = 0.0;
    for (var i = 0u; i < 8u; i += 1u) {
        let tap = SHADOW_DISC[i * 2u] * radius;
        lit += textureSampleCompareLevel(light_shadows, shadow_sampler, dir + across * tap.x + up * tap.y, slot, reference);
    }
    return lit / 8.0;
}

// The light entities' direct light on a surface: Cook-Torrance as for the sun, with
// Godot's falloff (the light's decay over distance in metres, windowed to zero at its
// reach) and spot cone, and the light's shadow cube where it has one.
fn entity_lights(pixel: vec2<f32>, depth: f32, world: vec3<f32>, n: vec3<f32>, geometric: vec3<f32>, v: vec3<f32>, f0: vec3<f32>, diffuse_color: vec3<f32>, roughness: f32, compensation: vec3<f32>) -> vec3<f32> {
    var total = vec3<f32>(0.0);
    let cluster = light_clusters[light_cluster(pixel, depth)];
    let n_dot_v = max(dot(n, v), 1e-4);
    for (var i = 0u; i < cluster.y; i += 1u) {
        let light = lights[light_indices[cluster.x + i]];
        let to_light = light.position_range.xyz - world;
        let distance = length(to_light);
        let range = light.position_range.w;
        if (distance >= range) {
            continue;
        }
        let l = to_light / max(distance, 1e-4);
        let n_dot_l = dot(n, l);
        if (n_dot_l <= 0.0) {
            continue;
        }
        let nd = pow(distance / range, 4.0);
        let window = max(1.0 - nd, 0.0);
        var attenuation = window * window * pow(max(distance / light.params.z, 0.1), -light.color_decay.w);
        let cutoff = light.direction_cutoff.w;
        if (cutoff > -1.5) {
            let s = max(dot(-l, light.direction_cutoff.xyz), cutoff);
            let rim = max(1e-4, (1.0 - s) / max(1.0 - cutoff, 1e-4));
            attenuation *= 1.0 - pow(rim, light.params.x);
        }
        if (attenuation <= 1e-5) {
            continue;
        }
        if (light.params.y >= 0.0) {
            attenuation *= light_visibility(i32(light.params.y), light.position_range.xyz, range, world, geometric);
        }
        let h = normalize(v + l);
        let d = distribution_ggx(max(dot(n, h), 0.0), roughness);
        let g = geometry_schlick(n_dot_v, roughness) * geometry_schlick(n_dot_l, roughness);
        let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
        let specular = (d * g * f) / max(4.0 * n_dot_v * n_dot_l, 1e-4) * compensation;
        let diffuse = (vec3<f32>(1.0) - f) * diffuse_color / PI;
        total += (diffuse + specular) * light.color_decay.rgb * n_dot_l * attenuation;
    }
    return total;
}

// Diffuse irradiance from the sky's nine spherical-harmonic coefficients, already
// convolved with the cosine lobe and divided by pi: the radiance a white Lambertian
// surface facing `n` reflects.
fn sh_irradiance(n: vec3<f32>) -> vec3<f32> {
    let c0 = textureLoad(sh_coefficients, vec2<i32>(0, 0), 0).rgb;
    let c1 = textureLoad(sh_coefficients, vec2<i32>(1, 0), 0).rgb;
    let c2 = textureLoad(sh_coefficients, vec2<i32>(2, 0), 0).rgb;
    let c3 = textureLoad(sh_coefficients, vec2<i32>(3, 0), 0).rgb;
    let c4 = textureLoad(sh_coefficients, vec2<i32>(4, 0), 0).rgb;
    let c5 = textureLoad(sh_coefficients, vec2<i32>(5, 0), 0).rgb;
    let c6 = textureLoad(sh_coefficients, vec2<i32>(6, 0), 0).rgb;
    let c7 = textureLoad(sh_coefficients, vec2<i32>(7, 0), 0).rgb;
    let c8 = textureLoad(sh_coefficients, vec2<i32>(8, 0), 0).rgb;
    // Band factors of the clamped cosine (pi, 2pi/3, pi/4), over pi.
    let a1 = 2.0 / 3.0;
    let a2 = 0.25;
    var e = c0 * 0.282095;
    e += a1 * 0.488603 * (c1 * n.y + c2 * n.z + c3 * n.x);
    e += a2 * (1.092548 * (c4 * n.x * n.y + c5 * n.y * n.z + c7 * n.x * n.z)
        + 0.315392 * c6 * (3.0 * n.z * n.z - 1.0) + 0.546274 * c8 * (n.x * n.x - n.y * n.y));
    return max(e, vec3<f32>(0.0));
}

// Ambient occlusion from the GTAO target (upsampled, when it is at half resolution, with
// weights that favour texels at this pixel's depth so occlusion does not bleed across
// silhouettes).
fn sample_occlusion(pixel: vec2<f32>, depth: f32) -> f32 {
    let size = vec2<i32>(textureDimensions(ambient_occlusion));
    if (frame.ao.x < 1.5) {
        return textureLoad(ambient_occlusion, min(vec2<i32>(pixel), size - 1), 0).r;
    }
    let half_position = pixel * 0.5 - 0.5;
    let base = vec2<i32>(floor(half_position));
    let f = half_position - floor(half_position);
    var total = 0.0;
    var weights = 0.0;
    for (var y = 0; y < 2; y += 1) {
        for (var x = 0; x < 2; x += 1) {
            let q = clamp(base + vec2<i32>(x, y), vec2<i32>(0), size - 1);
            let texel = textureLoad(ambient_occlusion, q, 0);
            let bilinear = select(1.0 - f.x, f.x, x == 1) * select(1.0 - f.y, f.y, y == 1);
            let similar = 1.0 / (1e-3 + abs(texel.g - depth) / max(depth, 1.0) * 20.0);
            let w = bilinear * similar + 1e-5;
            total += texel.r * w;
            weights += w;
        }
    }
    return total / weights;
}

struct ForwardOut {
    @location(0) light: vec4<f32>,
    // What screen-space reflections may replace: the weight the environment's specular
    // was added at, before the sky strength (rgb), and the roughness it was blurred by
    // (a). Zero for surfaces seen through (their pipeline does not write it) and unlit
    // ones.
    @location(1) reflectivity: vec4<f32>,
};

fn forward_out(light: vec4<f32>) -> ForwardOut {
    return ForwardOut(light, vec4<f32>(0.0));
}

// Forward shading in linear HDR: Cook-Torrance GGX for the sun with cascaded shadows;
// image-based lighting from the sky (SH irradiance, prefiltered GGX specular, split-sum
// DFG with multi-scattering energy compensation); ambient and specular occlusion.
@fragment
fn fs_forward(in: VertexOut, @builtin(front_facing) front: bool) -> ForwardOut {
    let m = materials[in.ids.z];
    let uv = material_uv(m, in.uv);
    // Everything that needs derivatives first, in uniform control flow.
    let sampled = albedo_texel(m, textureSample(albedo, albedo_sampler, uv));
    let orm = textureSample(model_orm,albedo_sampler,uv).rgb;
    let emissive = textureSample(model_emissive,albedo_sampler,uv).rgb;
    let bumps = textureSample(normal_map, albedo_sampler, uv).xyz;
    let dpx = dpdx(in.world);
    let dpy = dpdy(in.world);
    let duvx = dpdx(uv);
    let duvy = dpdy(uv);
    let coordinate = (in.uv.x + in.uv.y) / 8.0;
    let feather = max(fwidth(coordinate), 0.001);

    if ((in.ids.y & SKIP) != 0u) {
        let distance_to_band = abs(fract(coordinate) - 0.5);
        let band = 1.0 - smoothstep(0.12 - feather, 0.12 + feather, distance_to_band);
        let ink = mix(vec3<f32>(0.32, 0.13, 0.40), vec3<f32>(0.85, 0.42, 0.92), band);
        // Linear, so the tone map brings it back near the colours GL draws it in.
        return forward_out(vec4<f32>(pow(ink * in.color.rgb, vec3<f32>(2.2)), mix(0.10, 0.70, band) * in.color.a));
    }
    if (m.model_params.w == 1.0 && !front) { discard; }
    var surface_alpha = in.color.a;
    if (m.alpha_mode == 2u) {
        surface_alpha *= sampled.a;
    } else if (m.alpha_mode == 1u && sampled.a < m.model_params.z) {
        discard;
    }
    // The texture is sRGB and sampled linear; the vertex colour is sRGB.
    let albedo_linear = pow(in.color.rgb, vec3<f32>(2.2)) * sampled.rgb;
    let ambient_strength = frame.params0.z;

    if (frame.params1.x > 0.5) {
        var flat_n = normalize(in.normal);
        if (dot(flat_n, frame.eye.xyz - in.world) < 0.0) {
            flat_n = -flat_n;
        }
        let shade = 0.45 + 0.55 * clamp(0.5 + 0.5 * dot(flat_n, normalize(vec3<f32>(0.35, 0.25, 0.9))), 0.0, 1.0);
        return forward_out(vec4<f32>(albedo_linear * shade * ambient_strength, surface_alpha));
    }

    var geometric = normalize(in.normal);
    if ((m.model_params.w == 0.0 && dot(geometric, frame.eye.xyz - in.world) < 0.0) || (m.model_params.w == 2.0 && !front)) {
        geometric = -geometric;
    }
    let n = surface_normal(in, m, bumps, dpx, dpy, duvx, duvy, front);

    var roughness = frame.params0.x;
    var metallic = frame.params0.y;
    var emission = 0.0;
    if ((m.flags & MATERIAL_OWN_FACTORS) != 0u) {
        roughness = m.factors.x;
        metallic = m.factors.y;
        emission = m.factors.z;
    }
    if ((m.flags & 32u) != 0u) {roughness *= orm.g; metallic *= orm.b;}
    roughness = clamp(roughness, 0.05, 1.0);
    metallic = clamp(metallic, 0.0, 1.0);

    let v = normalize(frame.eye.xyz - in.world);
    let l = normalize(-frame.light_dir.xyz);
    let h = normalize(v + l);
    let f0 = mix(vec3<f32>(0.04), albedo_linear, metallic);
    let diffuse_color = albedo_linear * (1.0 - metallic);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 1e-4);

    // Split-sum terms: scale and bias of F0 for the directional albedo of the specular
    // lobe, and from them the multiple-scattering compensation (Fdez-Aguera 2019).
    let dfg = textureSampleLevel(dfg_lut, env_sampler, vec2<f32>(n_dot_v, roughness), 0.0).rg;
    let single = f0 * dfg.x + dfg.y;
    let energy = max(dfg.x + dfg.y, 1e-3);
    let compensation = 1.0 + f0 * (1.0 / energy - 1.0);

    // The sun.
    let d = distribution_ggx(max(dot(n, h), 0.0), roughness);
    let g = geometry_schlick(n_dot_v, roughness) * geometry_schlick(n_dot_l, roughness);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
    let specular = (d * g * f) / max(4.0 * n_dot_v * n_dot_l, 1e-4) * compensation;
    let diffuse = (vec3<f32>(1.0) - f) * diffuse_color / PI;
    // Shadowed by the geometric normal: a normal map does not move the shadow map.
    var visibility = sun_visibility(in.world, geometric, max(dot(geometric, l), 0.0));
    if (frame.contact.x > 0.5 && visibility > 0.0 && n_dot_l > 0.0) {
        visibility *= contact_shadow(in.world, geometric, in.clip.xy);
    }
    var direct = (diffuse + specular) * frame.light_color.rgb * n_dot_l * visibility;

    // Occlusion: GTAO for the diffuse ambient, with Jimenez's multi-bounce fit so bright
    // surfaces do not go grey in creases; Lagarde's approximation for the specular.
    let depth = dot(in.world - frame.eye.xyz, frame.forward.xyz);
    if (frame.lights.x > 0.5) {
        direct += entity_lights(in.clip.xy, depth, in.world, n, geometric, v, f0, diffuse_color, roughness, compensation);
    }
    var ao = 1.0;
    if (frame.effects.x > 0.5) {
        ao = mix(1.0, sample_occlusion(in.clip.xy, depth), clamp(frame.effects.y, 0.0, 1.0));
    }
    if ((m.flags & 32u) != 0u) {ao *= mix(1.0,orm.r,m.model_params.y);}
    let a = 2.0404 * albedo_linear - 0.3324;
    let b = -4.7951 * albedo_linear + 0.6417;
    let c = 2.7552 * albedo_linear + 0.6903;
    let diffuse_ao = max(vec3<f32>(ao), ((a * ao + b) * ao + c) * ao);
    let specular_ao = clamp(pow(n_dot_v + ao, exp2(-16.0 * roughness - 1.0)) - 1.0 + ao, 0.0, 1.0);

    var irradiance: vec3<f32>;
    var radiance: vec3<f32>;
    let r = reflect(-v, n);
    if (frame.misc.y > 0.5) {
        irradiance = sh_irradiance(n);
        let lod = roughness * (frame.effects.w - 1.0);
        radiance = textureSampleLevel(specular_env, env_sampler, r, lod).rgb;
    } else {
        irradiance = sky_light(n, 1.0, 0.0);
        radiance = sky_light(r, roughness, 0.0);
    }
    // Multiple scattering: the energy single scattering loses comes back, tinted by the
    // average Fresnel, and diffuse takes what the specular leaves.
    let missing = 1.0 - energy;
    let f_average = f0 + (1.0 - f0) / 21.0;
    let multiple = single * f_average / (1.0 - missing * f_average);
    let diffuse_weight = diffuse_color * (1.0 - single - multiple * missing);
    let ambient = (diffuse_weight * irradiance * diffuse_ao + (single * radiance + multiple * missing * irradiance) * specular_ao) * ambient_strength;
    let light = direct + ambient + albedo_linear * emission + emissive * m.model_emissive.rgb;
    return ForwardOut(vec4<f32>(light, surface_alpha), vec4<f32>(single * specular_ao, roughness));
}

// Wireframe: the triangles' edges in one colour, written straight to the display target.
@fragment
fn fs_wire(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(frame.wire_color.rgb, 1.0);
}

// The procedural sky behind everything, drawn where the prepass left the far plane.
@vertex
fn vs_sky(@builtin(vertex_index) index: u32) -> FullscreenOut {
    return fullscreen(index, 0.0);
}

@fragment
fn fs_sky(in: FullscreenOut) -> ForwardOut {
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let world = frame.inv_view_proj * vec4<f32>(ndc, 0.5, 1.0);
    let direction = normalize(world.xyz / world.w - frame.eye.xyz);
    let up = clamp(direction.z, -1.0, 1.0);
    let sun = normalize(-frame.light_dir.xyz);
    let sun_height = sun.z;
    var color = sky_radiance(direction);
    // The disc itself, antialiased by its screen-space width; the environment map
    // leaves it out, since the direct light already accounts for the sun.
    let daylight = smoothstep(-0.12, 0.08, sun_height);
    let sunset = (1.0 - smoothstep(0.04, 0.4, abs(sun_height))) * daylight;
    let angle = atan2(length(cross(direction, sun)), dot(direction, sun));
    let radius = radians(0.75);
    let edge = max(fwidth(angle), 0.00002);
    let disc = 1.0 - smoothstep(radius - edge, radius + edge, angle);
    let limb = sqrt(max(0.0, 1.0 - pow(angle / radius, 2.0)));
    let warmth = mix(vec3<f32>(1.0), vec3<f32>(1.0, 0.48, 0.16), sunset);
    let horizon_edge = max(fwidth(up), 0.0001);
    let visible = smoothstep(-horizon_edge, horizon_edge, up);
    color += frame.sun_color.rgb * warmth * disc * (9.0 + 3.0 * limb) * visible * smoothstep(-0.06, 0.0, sun_height + radius);
    return forward_out(vec4<f32>(color, 1.0));
}
