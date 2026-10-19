// The tools' overlay: guide lines and round handles, drawn into the finished picture
// against the scene's depth, then faintly through it. Prefixed with common.wgsl.

struct Fade {
    alpha: vec4<f32>,
};
@group(1) @binding(0) var<uniform> fade: Fade;

struct LineIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct LineOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) corner: vec2<f32>,
};

// The overlay is drawn after the temporal filter with the unjittered projection, against
// a depth drawn jittered by up to half a pixel: on a surface it lies on (the axes on a
// floor at z = 0) the two would fight, dashing the line. Half a percent of the distance
// towards the camera (reverse-Z depth scales with 1 / distance) covers half a pixel on
// surfaces up to about 85 degrees from facing the camera.
fn overlay_clip(position: vec3<f32>) -> vec4<f32> {
    let clip = frame.view_proj_unjittered * vec4<f32>(position, 1.0);
    return vec4<f32>(clip.xy, clip.z * 1.005, clip.w);
}

@vertex
fn vs_line(v: LineIn) -> LineOut {
    var out: LineOut;
    out.clip = overlay_clip(v.position);
    out.color = v.color;
    out.corner = vec2<f32>(0.0);
    return out;
}

@fragment
fn fs_line(in: LineOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color.rgb, in.color.a * fade.alpha.x);
}

// One instance per handle, six vertices making a screen-aligned square.
@vertex
fn vs_point(v: LineIn, @builtin(vertex_index) index: u32) -> LineOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    let corner = corners[index];
    var out: LineOut;
    let center = overlay_clip(v.position);
    let half_size = frame.params1.z * 0.5;
    out.clip = center + vec4<f32>(corner * half_size * 2.0 * frame.viewport.zw * center.w, 0.0, 0.0);
    out.color = v.color;
    out.corner = corner;
    return out;
}

@fragment
fn fs_point(in: LineOut) -> @location(0) vec4<f32> {
    let radius = length(in.corner);
    if (radius > 1.0) {
        discard;
    }
    let edge = smoothstep(1.0, 1.0 - fwidth(radius) * 1.5, radius);
    return vec4<f32>(in.color.rgb, in.color.a * fade.alpha.x * edge);
}
