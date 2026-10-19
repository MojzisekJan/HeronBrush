// Image-based lighting from the procedural sky: the sky drawn into a cube map with a box
// mip chain, the GGX-prefiltered specular cube, nine spherical-harmonic coefficients for
// diffuse, and the split-sum DFG table. All fragment passes (no compute), so the same
// code runs on the GLES path later. Prefixed with common.wgsl.

@group(1) @binding(0) var source: texture_cube<f32>;
@group(1) @binding(1) var source_sampler: sampler;

struct Params {
    // x cube face, y roughness, z the source's mip-0 size, w its mip count.
    face_roughness: vec4<f32>,
};
@group(1) @binding(2) var<uniform> params: Params;

@vertex
fn vs_ibl(@builtin(vertex_index) index: u32) -> FullscreenOut {
    return fullscreen(index, 0.0);
}

// The world direction through a cube face's texel, uv 0..1 with v down, in the usual
// cube-map convention (+X, -X, +Y, -Y, +Z, -Z).
fn cube_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let s = uv.x * 2.0 - 1.0;
    let t = uv.y * 2.0 - 1.0;
    var d: vec3<f32>;
    switch face {
        case 0u: { d = vec3<f32>(1.0, -t, -s); }
        case 1u: { d = vec3<f32>(-1.0, -t, s); }
        case 2u: { d = vec3<f32>(s, 1.0, t); }
        case 3u: { d = vec3<f32>(s, -1.0, -t); }
        case 4u: { d = vec3<f32>(s, -t, 1.0); }
        default: { d = vec3<f32>(-s, -t, -1.0); }
    }
    return normalize(d);
}

@fragment
fn fs_capture(in: FullscreenOut) -> @location(0) vec4<f32> {
    let direction = cube_direction(u32(params.face_roughness.x), in.uv);
    return vec4<f32>(sky_radiance(direction), 1.0);
}

// The next mip from the one above: a bilinear tap at the texel centre is the 2x2 average.
@fragment
fn fs_downsample(in: FullscreenOut) -> @location(0) vec4<f32> {
    let direction = cube_direction(u32(params.face_roughness.x), in.uv);
    return vec4<f32>(textureSampleLevel(source, source_sampler, direction, 0.0).rgb, 1.0);
}

fn radical_inverse(i: u32) -> f32 {
    return f32(reverseBits(i)) * 2.3283064365386963e-10;
}

fn hammersley(i: u32, count: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(count), radical_inverse(i));
}

// A GGX half vector around +Z for the sample `xi`.
fn importance_ggx(xi: vec2<f32>, roughness: f32) -> vec3<f32> {
    let a = roughness * roughness;
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    return vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
}

fn tangent_frame(n: vec3<f32>) -> mat3x3<f32> {
    let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(n.z) < 0.999);
    let t = normalize(cross(up, n));
    let b = cross(n, t);
    return mat3x3<f32>(t, b, n);
}

// The specular cube's mip for one roughness: the sky convolved with the GGX lobe, with
// N = V = R (the split-sum assumption). Filtered importance sampling (Krivanek and
// Colbert): each sample reads the source mip whose texel matches the sample's solid
// angle, so 64 samples come out smooth.
@fragment
fn fs_prefilter(in: FullscreenOut) -> @location(0) vec4<f32> {
    let n = cube_direction(u32(params.face_roughness.x), in.uv);
    let roughness = params.face_roughness.y;
    if (roughness <= 0.0) {
        return vec4<f32>(textureSampleLevel(source, source_sampler, n, 0.0).rgb, 1.0);
    }
    let frame_n = tangent_frame(n);
    let size = params.face_roughness.z;
    let texel_angle = 4.0 * PI / (6.0 * size * size);
    let count = 64u;
    var total = vec3<f32>(0.0);
    var weight = 0.0;
    for (var i = 0u; i < count; i += 1u) {
        let h = frame_n * importance_ggx(hammersley(i, count), roughness);
        let l = normalize(2.0 * dot(n, h) * h - n);
        let n_dot_l = dot(n, l);
        if (n_dot_l > 0.0) {
            let n_dot_h = max(dot(n, h), 0.0);
            let a2 = pow(roughness, 4.0);
            let dd = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
            let d = a2 / (PI * dd * dd);
            // pdf = D * NdotH / (4 VdotH), and V = N.
            let pdf = d * 0.25;
            let sample_angle = 1.0 / (f32(count) * pdf + 1e-4);
            let lod = clamp(0.5 * log2(sample_angle / texel_angle) + 1.0, 0.0, params.face_roughness.w - 1.0);
            total += textureSampleLevel(source, source_sampler, l, lod).rgb * n_dot_l;
            weight += n_dot_l;
        }
    }
    return vec4<f32>(total / max(weight, 1e-4), 1.0);
}

// One of the nine spherical-harmonic coefficients (the pixel's x) of the sky's radiance,
// integrated over a 16x16-per-face mip with each texel weighted by its solid angle.
@fragment
fn fs_sh(in: FullscreenOut) -> @location(0) vec4<f32> {
    let index = u32(in.position.x);
    let lod = params.face_roughness.w;
    let size = 16.0;
    var total = vec3<f32>(0.0);
    for (var face = 0u; face < 6u; face += 1u) {
        for (var y = 0; y < 16; y += 1) {
            for (var x = 0; x < 16; x += 1) {
                let uv = (vec2<f32>(f32(x), f32(y)) + 0.5) / size;
                let st = uv * 2.0 - 1.0;
                let solid_angle = 4.0 / (size * size) / pow(1.0 + dot(st, st), 1.5);
                let d = cube_direction(face, uv);
                let radiance = textureSampleLevel(source, source_sampler, d, lod).rgb;
                var basis: f32;
                switch index {
                    case 0u: { basis = 0.282095; }
                    case 1u: { basis = 0.488603 * d.y; }
                    case 2u: { basis = 0.488603 * d.z; }
                    case 3u: { basis = 0.488603 * d.x; }
                    case 4u: { basis = 1.092548 * d.x * d.y; }
                    case 5u: { basis = 1.092548 * d.y * d.z; }
                    case 6u: { basis = 0.315392 * (3.0 * d.z * d.z - 1.0); }
                    case 7u: { basis = 1.092548 * d.x * d.z; }
                    default: { basis = 0.546274 * (d.x * d.x - d.y * d.y); }
                }
                total += radiance * basis * solid_angle;
            }
        }
    }
    return vec4<f32>(total, 1.0);
}

// The split-sum DFG table (Karis 2013): x is N.V, y roughness; red is the scale and
// green the bias of F0 in the specular lobe's directional albedo.
@fragment
fn fs_dfg(in: FullscreenOut) -> @location(0) vec4<f32> {
    let n_dot_v = max(in.uv.x, 1e-3);
    let roughness = max(in.uv.y, 0.02);
    let v = vec3<f32>(sqrt(1.0 - n_dot_v * n_dot_v), 0.0, n_dot_v);
    let k = roughness * roughness / 2.0;
    var scale = 0.0;
    var bias = 0.0;
    let count = 512u;
    for (var i = 0u; i < count; i += 1u) {
        let h = importance_ggx(hammersley(i, count), roughness);
        let l = normalize(2.0 * dot(v, h) * h - v);
        let n_dot_l = max(l.z, 0.0);
        let n_dot_h = max(h.z, 0.0);
        let v_dot_h = max(dot(v, h), 0.0);
        if (n_dot_l > 0.0) {
            let g = (n_dot_v / (n_dot_v * (1.0 - k) + k)) * (n_dot_l / (n_dot_l * (1.0 - k) + k));
            let visibility = g * v_dot_h / max(n_dot_h * n_dot_v, 1e-5);
            let fc = pow(1.0 - v_dot_h, 5.0);
            scale += (1.0 - fc) * visibility;
            bias += fc * visibility;
        }
    }
    return vec4<f32>(scale / f32(count), bias / f32(count), 0.0, 1.0);
}
