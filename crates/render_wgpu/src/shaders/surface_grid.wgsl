// World-space grid; the pixel footprint is evaluated independently on each axis.
fn surface_grid_axis(position: f32, footprint: f32) -> f32 {
    let step_size = frame.surface_grid.x;
    let major = max(64.0, step_size);
    let minimum = max(4.0 * footprint, 0.000001);
    var base = max(step_size, exp2(floor(log2(minimum * 4.0))));
    if (base >= major) { base = max(major, exp2(floor(log2(minimum)))); }
    let index = floor(position / base + 0.5);
    var level = base;
    var parity = index;
    for (var i = 0; i < 24; i += 1) {
        if (level >= max(major, minimum) || parity - 2.0 * floor(parity * 0.5) != 0.0) { break; }
        parity *= 0.5;
        level *= 2.0;
    }
    let coarse = level >= major;
    let threshold = select(minimum * 4.0, minimum, coarse);
    let fade = 1.0 - smoothstep(level, level * 2.0, threshold);
    let distance = abs(position - index * base) / max(footprint, 0.000001);
    return (1.0 - smoothstep(0.35, 1.15, distance)) * fade * select(0.45, 1.0, coarse);
}

fn surface_grid_coverage(position: vec3<f32>, normal: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> f32 {
    if (frame.surface_grid.x <= 0.0 || frame.surface_grid.y <= 0.0) { return 0.0; }
    let footprint = sqrt(dx * dx + dy * dy);
    let n = abs(normal);
    var p: vec2<f32>;
    var f: vec2<f32>;
    if (n.z >= n.x && n.z >= n.y) { p = position.xy; f = footprint.xy; }
    else if (n.x >= n.y) { p = position.yz; f = footprint.yz; }
    else { p = position.xz; f = footprint.xz; }
    let coverage = max(surface_grid_axis(p.x, f.x), surface_grid_axis(p.y, f.y));
    let fade = mix(1.0, 0.5, smoothstep(512.0, 1024.0, length(position - frame.eye.xyz)));
    // Keep the surface guide subordinate to material details.
    return coverage * fade * frame.surface_grid.y * 0.35;
}

fn surface_grid_color(color: vec3<f32>, coverage: f32) -> vec3<f32> {
    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    let ink = vec3<f32>(select(0.35, 0.015, luminance > 0.18));
    return mix(color, ink, coverage);
}
