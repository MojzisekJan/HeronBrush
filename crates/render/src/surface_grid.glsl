uniform vec4 surface_grid;

// Each axis thins independently, preserving lines viewed at grazing angles.
float surface_grid_axis(float position, float footprint) {
    float stepSize = surface_grid.x;
    float major = max(64.0, stepSize);
    float minimum = max(4.0 * footprint, 0.000001);
    float base = max(stepSize, exp2(floor(log2(minimum * 4.0))));
    if (base >= major) base = max(major, exp2(floor(log2(minimum))));
    float index = floor(position / base + 0.5);
    float level = base;
    float parity = index;
    for (int i = 0; i < 24; ++i) {
        if (level >= max(major, minimum) || mod(parity, 2.0) != 0.0) break;
        parity *= 0.5;
        level *= 2.0;
    }
    bool coarse = level >= major;
    float threshold = coarse ? minimum : minimum * 4.0;
    float fade = 1.0 - smoothstep(level, level * 2.0, threshold);
    float distance = abs(position - index * base) / max(footprint, 0.000001);
    return (1.0 - smoothstep(0.35, 1.15, distance)) * fade * (coarse ? 1.0 : 0.45);
}

float surface_grid_coverage(vec3 position, vec3 normal, vec3 dx, vec3 dy, vec3 eye) {
    if (surface_grid.x <= 0.0 || surface_grid.y <= 0.0) return 0.0;
    vec3 footprint = sqrt(dx * dx + dy * dy);
    vec3 n = abs(normal);
    vec2 p, f;
    if (n.z >= n.x && n.z >= n.y) { p = position.xy; f = footprint.xy; }
    else if (n.x >= n.y) { p = position.yz; f = footprint.yz; }
    else { p = position.xz; f = footprint.xz; }
    float coverage = max(surface_grid_axis(p.x, f.x), surface_grid_axis(p.y, f.y));
    float fade = mix(1.0, 0.5, smoothstep(512.0, 1024.0, length(position - eye)));
    // Keep the surface guide subordinate to material details.
    return coverage * fade * surface_grid.y * 0.35;
}

vec3 surface_grid_color(vec3 color, float coverage) {
    float luminance = dot(color, vec3(0.2126, 0.7152, 0.0722));
    vec3 ink = vec3(luminance > 0.18 ? 0.015 : 0.35);
    return mix(color, ink, coverage);
}
