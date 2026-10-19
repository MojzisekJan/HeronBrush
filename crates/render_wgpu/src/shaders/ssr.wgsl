// Screen-space reflections, after the GL renderer's: each reflective pixel marches its
// reflection through the depth buffer and, where it lands on geometry, works out how
// much brighter or darker that is than the environment the forward pass already
// reflected there. The resolve adds the difference, so a reflection replaces the sky
// rather than piling on top of it. Rough surfaces scatter their rays by a GGX sample
// when a temporal filter will average them (glossy reflections); without one the ray
// is the mirror direction and roughness only fades it. At half resolution: reflections
// are soft, and the resolve upsamples them bilinearly. Prefixed with common.wgsl.

@group(1) @binding(0) var scene_light: texture_2d<f32>;
@group(1) @binding(1) var scene_depth: texture_depth_2d;
@group(1) @binding(2) var scene_normals: texture_2d<f32>;
@group(1) @binding(3) var reflectivity: texture_2d<f32>;
@group(1) @binding(4) var specular_env: texture_cube<f32>;
@group(1) @binding(5) var env_sampler: sampler;

@vertex
fn vs_ssr(@builtin(vertex_index) index: u32) -> FullscreenOut {
    return fullscreen(index, 0.0);
}

fn view_at(q: vec2<i32>, size: vec2<i32>) -> vec3<f32> {
    let d = textureLoad(scene_depth, q, 0);
    return view_position((vec2<f32>(q) + 0.5) / vec2<f32>(size), d);
}

fn screen_of(p: vec3<f32>) -> vec2<f32> {
    let clip = frame.proj * vec4<f32>(p, 1.0);
    let ndc = clip.xy / clip.w;
    return vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
}

@fragment
fn fs_ssr(in: FullscreenOut) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(scene_depth));
    // The full-resolution texel this half-resolution pixel stands for.
    let p = min(vec2<i32>(in.position.xy) * 2, size - 1);
    let surface = textureLoad(reflectivity, p, 0);
    let roughness = surface.a;
    // Nothing here reflects enough to be worth marching for.
    let fade_rough = 1.0 - smoothstep(0.55, 0.9, roughness);
    if (max(surface.r, max(surface.g, surface.b)) * fade_rough < 0.002) {
        return vec4<f32>(0.0);
    }
    let d = textureLoad(scene_depth, p, 0);
    if (d <= 0.0) {
        return vec4<f32>(0.0);
    }
    let origin = view_at(p, size);
    let world_normal = octahedral_decode(textureLoad(scene_normals, p, 0).xy);
    let normal = normalize((frame.view * vec4<f32>(world_normal, 0.0)).xyz);
    let v = normalize(-origin);
    var h = normal;
    if (frame.reflection.w > 0.5 && roughness > 0.05) {
        // A GGX half vector round the normal, from per-pixel noise that changes each frame.
        let xi = vec2<f32>(
            interleaved_gradient_noise(in.position.xy + vec2<f32>(frame.misc.w * 5.0, 3.0)),
            interleaved_gradient_noise(in.position.xy + vec2<f32>(11.0, frame.misc.w * 17.0)),
        );
        let a = roughness * roughness * 0.5;
        let phi = 2.0 * PI * xi.x;
        let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
        let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
        let helper = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(normal.z) > 0.9);
        let tangent = normalize(cross(helper, normal));
        let bitangent = cross(normal, tangent);
        h = normalize(tangent * (cos(phi) * sin_theta) + bitangent * (sin(phi) * sin_theta) + normal * cos_theta);
    }
    let direction = reflect(-v, h);
    // A reflection heading towards the camera has nothing on screen to show.
    if (direction.z > 0.0 || dot(direction, normal) <= 0.0) {
        return vec4<f32>(0.0);
    }
    let steps = max(u32(frame.reflection.z), 1u);
    let step_length = frame.reflection.y / f32(steps);
    let thickness = max(0.5, -origin.z * 0.004);
    let jitter = interleaved_gradient_noise(in.position.xy + vec2<f32>(frame.misc.w * 3.0, 29.0));
    var at = origin + normal * thickness + direction * step_length * (0.5 + jitter);
    var hit = vec2<f32>(-1.0);
    for (var i = 0u; i < steps; i += 1u) {
        let uv = screen_of(at);
        if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || at.z > -0.01) {
            break;
        }
        let q = min(vec2<i32>(uv * vec2<f32>(size)), size - 1);
        let stored = view_at(q, size).z;
        let difference = stored - at.z;
        if (difference > 0.0 && difference < thickness + step_length * 2.0 && textureLoad(scene_depth, q, 0) > 0.0) {
            // Halve back to where the ray crossed the surface.
            var before = at - direction * step_length;
            var after = at;
            for (var j = 0u; j < 4u; j += 1u) {
                let middle = (before + after) * 0.5;
                let mq = min(vec2<i32>(screen_of(middle) * vec2<f32>(size)), size - 1);
                if (view_at(mq, size).z - middle.z > 0.0) {
                    after = middle;
                } else {
                    before = middle;
                }
            }
            hit = screen_of(after);
            break;
        }
        at += direction * step_length;
    }
    if (hit.x < 0.0) {
        return vec4<f32>(0.0);
    }
    // What the forward pass already reflected here: the environment along the world
    // reflection at this roughness, at the sky's strength.
    let world_direction = (transpose(frame.view) * vec4<f32>(direction, 0.0)).xyz;
    let replaced = textureSampleLevel(specular_env, env_sampler, world_direction, roughness * (frame.effects.w - 1.0)).rgb * frame.params0.z;
    let found = textureLoad(scene_light, min(vec2<i32>(hit * vec2<f32>(size)), size - 1), 0).rgb;
    // Fade towards the window's edges, where the march runs out of picture.
    let edge = smoothstep(vec2<f32>(0.0), vec2<f32>(0.1), hit) * smoothstep(vec2<f32>(0.0), vec2<f32>(0.1), 1.0 - hit);
    let confidence = edge.x * edge.y * fade_rough * frame.reflection.x;
    return vec4<f32>((found - replaced) * surface.rgb * confidence, 1.0);
}
