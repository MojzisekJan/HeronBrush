//! The GLSL the renderer runs.
//!
//! GLSL 1.50, which is OpenGL 3.2 core -- available on every platform this runs on, and
//! on macOS the newest one that still exists.

/// Draws a triangle covering the viewport from nothing but the vertex index.
///
/// One oversized triangle rather than two triangles of a quad: the diagonal where two
/// would meet makes the rasterizer shade that seam's pixels twice, and a full-screen pass
/// is exactly where that costs the most.
pub const FULLSCREEN_VERTEX: &str = r#"#version 150 core
out vec2 v_uv;

void main() {
    vec2 corner = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    v_uv = corner;
    gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}
"#;

/// Derive the tangent frame from the actual face UVs, including mirrored UVs.
pub const NORMAL_MAPPING: &str = r#"
uniform float material_face_mode;
uniform sampler2D normal_map;
uniform float has_normal_map;
uniform float normal_y_sign;
uniform float normal_scale;
uniform vec2 material_size;
vec3 mapped_normal(vec3 n, vec3 position, vec2 texels) {
    if (has_normal_map < 0.5) return n;
    vec2 uv = texels / max(material_size, vec2(1.0));
    vec3 px = dFdx(position), py = dFdy(position);
    vec2 tx = dFdx(uv), ty = dFdy(uv);
    float determinant = tx.x * ty.y - tx.y * ty.x;
    if (abs(determinant) < 1e-12) return n;
    vec3 t = (px * ty.y - py * tx.y) / determinant;
    vec3 b = (py * tx.x - px * ty.x) / determinant;
    t -= n * dot(n,t);
    if (dot(t,t) < 1e-12) return n;
    t = normalize(t);
    b = normalize(cross(n,t)) * (dot(cross(n,t),b) < 0.0 ? -1.0 : 1.0);
    vec3 sample_normal = texture(normal_map,uv).xyz * 2.0 - 1.0;
    sample_normal.xy *= normal_scale;
    sample_normal.y *= normal_y_sign;
    vec3 result = mat3(t,b,n) * sample_normal;
    return dot(result,result) > 1e-8 ? normalize(result) : n;
}
"#;

pub fn normal_mapping(source: &str) -> String {
    source.replace("// NORMAL_MAPPING",NORMAL_MAPPING)
}

/// Writes depth and nothing else, for the shadow map.
pub const DEPTH_VERTEX: &str = r#"#version 150 core
in vec3 position;
uniform mat4 view_projection;

void main() {
    gl_Position = view_projection * vec4(position, 1.0);
}
"#;

pub const DEPTH_FRAGMENT: &str = r#"#version 150 core
void main() {}
"#;

/// Material-aware shadow depth, used only for masks/imported face rules.
pub const MATERIAL_DEPTH_VERTEX: &str = r#"#version 150 core
in vec3 position;
in vec2 uv;
uniform mat4 view_projection;
out vec2 v_uv;
void main() {
    v_uv = uv;
    gl_Position = view_projection * vec4(position, 1.0);
}
"#;
pub const MATERIAL_DEPTH_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
uniform sampler2D material;
uniform vec2 material_size;
uniform float material_alpha_mode;
uniform float material_alpha_cutoff;
uniform float material_face_mode;
void main() {
    if (material_alpha_mode > 1.5) discard;
    if (!gl_FrontFacing && material_face_mode > 0.5 && material_face_mode < 1.5) discard;
    if (material_alpha_mode > 0.5 && texture(material, v_uv / max(material_size, vec2(1.0))).a < material_alpha_cutoff) discard;
}
"#;

/// Writes view-space normals, for screen-space ambient occlusion to read.
pub const NORMAL_VERTEX: &str = r#"#version 150 core
in vec3 position;
in vec3 normal;

uniform mat4 view;
uniform mat4 projection;

out vec3 v_view_normal;
out vec3 v_view_position;
in vec2 uv;
out vec2 v_uv;

void main() {
    v_uv = uv;
    v_view_position = (view * vec4(position,1.0)).xyz;
    v_view_normal = mat3(view) * normal;
    gl_Position = projection * view * vec4(position, 1.0);
}
"#;

pub const NORMAL_FRAGMENT: &str = r#"#version 150 core
in vec3 v_view_normal;
in vec3 v_view_position;
in vec2 v_uv;
// NORMAL_MAPPING
out vec4 fragment;

void main() {
    // A surface seen from behind -- a patch, which has no inside -- would otherwise write
    // a normal pointing away from the camera, and ambient occlusion would sample a
    // hemisphere into the surface and find it entirely occluded.
    vec3 normal = normalize(v_view_normal);
    if (!gl_FrontFacing && material_face_mode < 0.5) {
        normal = -normal;
    }
    normal = mapped_normal(normal,v_view_position,v_uv);
    if (!gl_FrontFacing && material_face_mode > 0.5 && material_face_mode < 1.5) discard;
    if (!gl_FrontFacing && material_face_mode > 1.5) normal = -normal;
    // Packed into an unsigned texture, so the range is halved and shifted.
    fragment = vec4(normal * 0.5 + 0.5, 1.0);
}
"#;

/// How many directions ambient occlusion samples per pixel.
pub const SSAO_KERNEL_SIZE: usize = 24;

/// Screen-space ambient occlusion.
///
/// For each pixel, points are sampled in a hemisphere around it, oriented along its
/// normal; a point that the depth buffer says is behind some surface is occluded. This is
/// what gives creases and corners their contact shading, and it is most of what makes a
/// flat-shaded room read as a room.
pub const SSAO_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;

uniform sampler2D depth_texture;
uniform sampler2D normal_texture;
uniform sampler2D noise_texture;

uniform mat4 projection;
uniform mat4 inverse_projection;
uniform vec3 kernel[24];
uniform vec2 noise_scale;
uniform float radius;
uniform float bias;

vec3 view_position(vec2 uv) {
    float depth = texture(depth_texture, uv).r;
    vec4 clip = vec4(uv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
    vec4 view = inverse_projection * clip;
    return view.xyz / view.w;
}

void main() {
    float depth = texture(depth_texture, v_uv).r;
    // Nothing was drawn here, so there is nothing to occlude.
    if (depth >= 1.0) {
        fragment = vec4(1.0);
        return;
    }

    vec3 origin = view_position(v_uv);
    vec3 normal = normalize(texture(normal_texture, v_uv).xyz * 2.0 - 1.0);

    // A per-pixel rotation about the normal, from a small tiled noise texture. Without it
    // the same 24 directions are used everywhere and the result bands visibly; with it the
    // error turns into noise that the blur pass takes out.
    vec3 random = vec3(texture(noise_texture, v_uv * noise_scale).xy * 2.0 - 1.0, 0.0);
    vec3 tangent = normalize(random - normal * dot(random, normal));
    vec3 bitangent = cross(normal, tangent);
    mat3 to_view = mat3(tangent, bitangent, normal);

    float occlusion = 0.0;
    for (int i = 0; i < 24; ++i) {
        vec3 sample_position = origin + to_view * kernel[i] * radius;

        vec4 offset = projection * vec4(sample_position, 1.0);
        offset.xyz /= offset.w;
        offset.xyz = offset.xyz * 0.5 + 0.5;
        if (offset.x < 0.0 || offset.x > 1.0 || offset.y < 0.0 || offset.y > 1.0) {
            continue;
        }

        float sample_depth = view_position(offset.xy).z;
        // A surface far in front of this pixel is a different surface, not a crease, so
        // its contribution is faded out rather than counted.
        float range = smoothstep(0.0, 1.0, radius / max(abs(origin.z - sample_depth), 1e-4));
        occlusion += (sample_depth >= sample_position.z + bias ? 1.0 : 0.0) * range;
    }

    fragment = vec4(1.0 - occlusion / 24.0);
}
"#;

/// Removes SSAO noise without carrying dark pixels across silhouettes or corners.
pub const BLUR_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;

uniform sampler2D source;
uniform sampler2D depth_texture;
uniform sampler2D normal_texture;
uniform mat4 inverse_projection;
uniform float radius;
uniform vec2 texel;

vec3 view_position(vec2 uv, float depth) {
    vec4 view = inverse_projection * vec4(uv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
    return view.xyz / view.w;
}

void main() {
    float depth = texture(depth_texture, v_uv).r;
    if (depth >= 1.0) {
        fragment = vec4(1.0);
        return;
    }
    vec3 origin = view_position(v_uv, depth);
    vec3 normal = normalize(texture(normal_texture, v_uv).xyz * 2.0 - 1.0);
    float total = 0.0;
    float weights = 0.0;
    float tolerance = max(radius * 0.1, 0.001);
    // Symmetric integer taps keep the result centred on the current pixel.
    for (int x = -2; x <= 2; ++x) {
        for (int y = -2; y <= 2; ++y) {
            vec2 uv = v_uv + vec2(x, y) * texel;
            if (any(lessThan(uv, vec2(0.0))) || any(greaterThan(uv, vec2(1.0)))) {
                continue;
            }
            float sample_depth = texture(depth_texture, uv).r;
            if (sample_depth >= 1.0) {
                continue;
            }
            vec3 sample_normal = normalize(texture(normal_texture, uv).xyz * 2.0 - 1.0);
            vec3 delta = view_position(uv, sample_depth) - origin;
            // Distance to the tangent plane preserves a continuous sloping floor.
            float separation = max(abs(dot(delta, normal)), abs(dot(delta, sample_normal)));
            float spatial = exp(-float(x * x + y * y) / 4.0);
            float weight = spatial * exp(-separation * separation / (tolerance * tolerance))
                * pow(max(dot(normal, sample_normal), 0.0), 8.0);
            total += texture(source, uv).r * weight;
            weights += weight;
        }
    }
    fragment = vec4(total / max(weights, 0.0001));
}
"#;

/// The scene, shaded.
pub const SCENE_VERTEX: &str = r#"#version 150 core
in vec3 position;
in vec3 normal;
in vec4 color;
in vec2 uv;

uniform mat4 view_projection;
uniform mat4 light_view_projection;
// How large a handle is drawn, in pixels. Ignored for anything but points.
uniform float point_size;

out vec3 v_world_position;
out vec3 v_normal;
out vec4 v_color;
out vec2 v_uv;
out vec4 v_light_position;

void main() {
    v_world_position = position;
    v_normal = normal;
    v_color = color;
    v_uv = uv;
    v_light_position = light_view_projection * vec4(position, 1.0);
    gl_Position = view_projection * vec4(position, 1.0);
    gl_PointSize = point_size;
}
"#;

/// Physically based shading: one directional light with a shadow map, plus a sky.
///
/// The reflectance is Cook-Torrance -- GGX for the distribution of microfacets, Smith for
/// how they shadow each other, Schlick for Fresnel -- which is what everything from
/// Blender's EEVEE to a game engine uses, and what makes a surface's roughness read as
/// roughness rather than as a change of colour.
///
/// The sky stands in for image-based lighting. A level is mostly interiors, so a full
/// environment probe would be mostly wasted; a two-colour gradient gives the fill light
/// that keeps unlit faces from going black, and the direction-dependence that keeps them
/// from going flat.
pub const SCENE_FRAGMENT: &str = r#"#version 150 core
in vec3 v_world_position;
in vec3 v_normal;
in vec4 v_color;
in vec2 v_uv;
in vec4 v_light_position;

out vec4 fragment;
// How strongly this pixel reflects, and how roughly, for the reflection pass. The colour
// already has the sky reflected in it at this weight, so a reflection pass that finds
// real geometry can put the difference in rather than adding light that is already there.
out vec4 reflectivity;

uniform vec3 camera_position;
uniform vec3 light_direction;
uniform vec3 light_color;
uniform vec3 sky_color;
// The haze just above the horizon, which is the brightest part of a real sky.
uniform vec3 horizon_color;
uniform vec3 ground_color;
uniform float roughness_value;
uniform float metallic_value;
uniform float emission_value;
uniform sampler2D model_orm;
uniform sampler2D model_emissive;
uniform vec3 model_emissive_factor;
uniform float model_ao_strength;
uniform float ambient_strength;
// How strongly the occlusion buffer darkens a crease. Zero leaves it flat.
uniform float occlusion_strength;
// How much light reaches the film, applied before the tone map.
uniform float exposure;

uniform sampler2D shadow_map;
uniform vec2 shadow_texel;
// How far the shadow taps spread, in texels of the shadow map.
uniform float shadow_softness;
// How far a surface is pushed out of its own shadow, and how far again when it is nearly
// edge-on to the light. Zero for both turns shadowing off, since nothing is then behind
// anything.
uniform float shadow_bias;
uniform float shadow_strength;
uniform sampler2D occlusion;
uniform vec2 viewport;

// Samples the prepass depth buffer for short-range contact shadows that a level-wide shadow map
// cannot resolve.
uniform sampler2D scene_depth;
uniform mat4 view;
uniform mat4 projection;
// How far the march reaches, in world units. Zero turns it off.
uniform float contact_shadow_distance;
// How many samples it takes on the way.
uniform float contact_shadow_samples;
// How far behind a surface a sample has to be to count as blocked, in view units.
uniform float contact_shadow_thickness;
// The material on the surface, and its size in texels. A surface with no material is
// given a single white pixel and a size of one, which comes out as the vertex colour.
uniform sampler2D material;
// NORMAL_MAPPING
// 1.0 when the scene is drawn straight to the screen, with no composite pass to map it.
uniform float tonemap_here;
// 1.0 for surfaces; 0.0 for lines and handles, which have no normal and are drawn as
// they are.
uniform float lit;
uniform vec3 wire_color;
// 1.0 draws every face flat -- its material's colour with only a little directional
// shading -- and skips the shadow, occlusion and reflection reads.
uniform float flat_shading;
// Fades the unlit overlay, so a handle behind geometry can be shown without being
// mistaken for one in front of it.
uniform float alpha_scale;
// Whether a point is drawn as a disc rather than as the square OpenGL hands it.
uniform float round_points;
// What the material's alpha channel means: 0 solid, 1 a cut-out mask (a texel is
// either kept whole or dropped), 2 blended (the texel's alpha is the surface's).
uniform float material_alpha_mode;
uniform float material_alpha_cutoff;
uniform bool skip_hatch;

const float PI = 3.14159265359;

float distribution_ggx(vec3 n, vec3 h, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float n_dot_h = max(dot(n, h), 0.0);
    float d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / max(PI * d * d, 1e-6);
}

float geometry_schlick(float n_dot_v, float roughness) {
    float r = roughness + 1.0;
    float k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

float geometry_smith(vec3 n, vec3 v, vec3 l, float roughness) {
    return geometry_schlick(max(dot(n, v), 0.0), roughness)
         * geometry_schlick(max(dot(n, l), 0.0), roughness);
}

vec3 fresnel_schlick(float cos_theta, vec3 f0) {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

vec3 fresnel_schlick_roughness(float cos_theta, vec3 f0, float roughness) {
    return f0 + (max(vec3(1.0 - roughness), f0) - f0)
              * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// The light coming from the sky in a direction. The world is Z up, so the horizon is at
// z == 0.
//
// Three bands rather than one gradient: the ground below, a bright haze at the horizon,
// and the sky above. A real sky is brightest just above the horizon and a single lerp
// from ground to sky misses that, which is much of what makes a viewport look flat --
// every surface facing sideways gets the same middling grey.
//
// `roughness` widens the sun in it, since that is what a rough surface does to a
// reflection, and `sun_weight` is zero for the diffuse lookup: the sun's own light is
// already in the direct term, and adding it here would count it twice.
vec3 sky(vec3 direction, float roughness, float sun_weight) {
    float up = clamp(direction.z, -1.0, 1.0);

    // The power concentrates the change near the horizon, where a real sky puts it.
    vec3 above = mix(horizon_color, sky_color, pow(clamp(up, 0.0, 1.0), 0.45));
    vec3 below = mix(horizon_color, ground_color, pow(clamp(-up, 0.0, 1.0), 0.35));
    vec3 result = up >= 0.0 ? above : below;

    if (sun_weight > 0.0) {
        vec3 to_sun = normalize(-light_direction);
        float alignment = clamp(dot(direction, to_sun), 0.0, 1.0);
        // A mirror sees a disc, a rough surface a broad glow.
        float sharpness = mix(2000.0, 8.0, clamp(roughness, 0.0, 1.0));
        result += light_color * pow(alignment, sharpness)
                * (1.0 - roughness * 0.6) * sun_weight;
    }

    return result;
}

float shadow_factor(vec3 n, vec3 l) {
    vec3 projected = v_light_position.xyz / v_light_position.w;
    projected = projected * 0.5 + 0.5;

    // Outside what the shadow map covers, nothing is known, so nothing is shadowed.
    if (projected.z > 1.0 || projected.x < 0.0 || projected.x > 1.0
        || projected.y < 0.0 || projected.y > 1.0) {
        return 1.0;
    }

    // A surface nearly edge-on to the light needs a larger bias, or its own depth reads as
    // being in front of itself and it shadows itself in stripes.
    float bias = max(shadow_bias * (1.0 - dot(n, l)), shadow_bias * 0.27);

    // A Poisson disk rather than a square of neighbours: the same number of taps spread
    // unevenly over a wider area, which reads as a soft edge instead of as nine steps.
    const vec2 disk[12] = vec2[12](
        vec2(-0.326, -0.406), vec2(-0.840, -0.074), vec2(-0.696,  0.457),
        vec2(-0.203,  0.621), vec2( 0.962, -0.195), vec2( 0.473, -0.480),
        vec2( 0.519,  0.767), vec2( 0.185, -0.893), vec2( 0.507,  0.064),
        vec2( 0.896,  0.412), vec2(-0.322, -0.933), vec2(-0.792, -0.598)
    );

        if (shadow_strength <= 0.0) {
        return 1.0;
    }

    float lit_fraction = 0.0;
    for (int i = 0; i < 12; ++i) {
        vec2 at = projected.xy + disk[i] * shadow_texel * shadow_softness;
        float depth = texture(shadow_map, at).r;
        lit_fraction += projected.z - bias > depth ? 0.0 : 1.0;
    }
    return lit_fraction / 12.0;
}

/// How much of the light reaches this point once the geometry just in front of it is
/// taken into account.
///
/// Marches from the surface towards the light in view space and asks the prepass depth
/// whether anything is in the way. It only sees what is on screen and only over a short
/// distance, which is the point: it fills in what the shadow map is too coarse to hold
/// and leaves the rest to the shadow map.
float contact_shadow(vec3 world_position, vec3 n, vec3 l) {
    if (contact_shadow_distance <= 0.0) {
        return 1.0;
    }

    vec3 origin = (view * vec4(world_position, 1.0)).xyz;
    vec3 direction = normalize((view * vec4(l, 0.0)).xyz);

    // Start a little off the surface, along its own normal, or every point shadows
    // itself.
    vec3 view_normal = normalize((view * vec4(n, 0.0)).xyz);
    origin += view_normal * contact_shadow_thickness;

    int steps = int(max(contact_shadow_samples, 1.0));
    float step_length = contact_shadow_distance / float(steps);

    for (int i = 1; i <= steps; ++i) {
        vec3 at = origin + direction * (step_length * float(i));
        // Behind the camera has no depth to read.
        if (at.z > -0.001) {
            break;
        }

        vec4 clip = projection * vec4(at, 1.0);
        vec2 uv = (clip.xy / clip.w) * 0.5 + 0.5;
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
            break;
        }

        // The prepass depth, back into view space. The projection is a perspective one,
        // so this undoes it rather than reading the value straight.
        float stored = texture(scene_depth, uv).r * 2.0 - 1.0;
        vec4 unprojected = inverse(projection) * vec4(clip.xy / clip.w, stored, 1.0);
        float stored_z = unprojected.z / unprojected.w;

        float difference = stored_z - at.z;
        // In front of the sample, but not so far in front that it is a different
        // surface altogether -- that would shadow through a wall.
        if (difference > contact_shadow_thickness
            && difference < contact_shadow_thickness * 40.0) {
            return 0.0;
        }
    }

    return 1.0;
}

vec3 tonemap(vec3 color) {
    // The ACES curve, in the fitted form everyone uses. Without a tone map the bright end
    // clips to white and takes the shape with it.
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), 0.0, 1.0);
}

void main() {
    if (lit < -0.5) {
        fragment = vec4(wire_color, 1.0);
        reflectivity = vec4(0.0);
        return;
    }
    if (lit < 0.5) {
        // Cuts square point sprites into discs. `gl_PointCoord` is used only when drawing points,
        // never lines.
        if (round_points > 0.5) {
            vec2 from_middle = gl_PointCoord - vec2(0.5);
            float radius = length(from_middle);
            if (radius > 0.5) {
                discard;
            }
            // A pixel of softness at the rim, or the disc reads as a cog at this size.
            float edge = smoothstep(0.5, 0.5 - fwidth(radius) * 1.5, radius);
            fragment = vec4(v_color.rgb, v_color.a * alpha_scale * edge);
            reflectivity = vec4(0.0);
            return;
        }
        fragment = vec4(v_color.rgb, v_color.a * alpha_scale);
        reflectivity = vec4(0.0);
        return;
    }

    if (skip_hatch) {
        // Antialiased diagonal bands repeat every eight map UV units.
        float coordinate = (v_uv.x + v_uv.y) / 8.0;
        float distance_to_band = abs(fract(coordinate) - 0.5);
        float feather = max(fwidth(coordinate), 0.001);
        float band = 1.0 - smoothstep(0.12 - feather, 0.12 + feather, distance_to_band);
        vec3 ink = mix(vec3(0.32, 0.13, 0.40), vec3(0.85, 0.42, 0.92), band);
        fragment = vec4(ink * v_color.rgb, mix(0.10, 0.70, band) * v_color.a * alpha_scale);
        reflectivity = vec4(0.0);
        return;
    }

    // The UV coordinates are in texels, as the .map format expresses them, so the size of
    // the material is what turns them into a lookup.
    vec4 sampled = texture(material, v_uv / max(material_size, vec2(1.0)));
    vec3 texel = sampled.rgb;
    // A masked material drops the see-through texels outright, the way a fence or a leaf
    // is drawn; a blended one carries its alpha through to the surface, on top of any the
    // face's own colour set.
    if (!gl_FrontFacing && material_face_mode > 0.5 && material_face_mode < 1.5) discard;
    float surface_alpha = v_color.a;
    if (material_alpha_mode > 1.5) {
        surface_alpha *= sampled.a;
    } else if (material_alpha_mode > 0.5 && sampled.a < material_alpha_cutoff) {
        discard;
    }
    // sRGB to linear: the shading below is in linear light, and skipping this is what
    // makes a textured surface look washed out and flat.
    vec3 albedo = pow(v_color.rgb, vec3(2.2)) * pow(texel, vec3(2.2));
    vec3 orm = texture(model_orm, v_uv / max(material_size,vec2(1.0))).rgb;
    float roughness = clamp(roughness_value * orm.g, 0.05, 1.0);
    float metallic = clamp(metallic_value * orm.b, 0.0, 1.0);

    vec3 n = normalize(v_normal);
    // Same as the normal pass: a surface with no inside is lit by whichever side is being
    // looked at, not by the side its winding happens to name.
    float face_sign = (!gl_FrontFacing && material_face_mode > 1.5) ? -1.0 : 1.0;
    if (!gl_FrontFacing && material_face_mode < 0.5) {
        n = -n;
    }

    // Flat shading: the material's colour with a gentle three-quarter directional term so
    // faces facing different ways still read apart, and nothing else -- no shadows, no
    // occlusion, no reflection.
    if (flat_shading > 0.5) {
        float shade = 0.45 + 0.55 * clamp(
            0.5 + 0.5 * dot(n * face_sign, normalize(vec3(0.35, 0.25, 0.9))), 0.0, 1.0);
        vec3 flat_light = albedo * shade * ambient_strength;
        vec3 flat_exposed = flat_light * exposure;
        fragment = vec4(mix(flat_light, tonemap(flat_exposed), tonemap_here), surface_alpha);
        reflectivity = vec4(0.0);
        return;
    }

    n = mapped_normal(n, v_world_position, v_uv) * face_sign;
    vec3 v = normalize(camera_position - v_world_position);
    vec3 l = normalize(-light_direction);
    vec3 h = normalize(v + l);

    vec3 f0 = mix(vec3(0.04), albedo, metallic);

    // The direct light.
    float n_dot_l = max(dot(n, l), 0.0);
    float d = distribution_ggx(n, h, roughness);
    float g = geometry_smith(n, v, l, roughness);
    vec3 f = fresnel_schlick(max(dot(h, v), 0.0), f0);

    vec3 specular = (d * g * f)
                  / max(4.0 * max(dot(n, v), 0.0) * n_dot_l, 1e-4);
    vec3 diffuse = (vec3(1.0) - f) * (1.0 - metallic) * albedo / PI;

    float lit_fraction = shadow_factor(n, l);
    if (lit_fraction > 0.0 && shadow_strength > 0.0) {
        lit_fraction *= contact_shadow(v_world_position, n, l);
    }
    vec3 direct = (diffuse + specular) * light_color * n_dot_l * lit_fraction;

    // The sky, as ambient light, cut back where the geometry occludes itself.
    float ao = mix(1.0, texture(occlusion, gl_FragCoord.xy / viewport).r,
                   occlusion_strength);
    ao *= mix(1.0,orm.r,model_ao_strength);
    // The diffuse lookup is the sky as a rough surface sees it, with no sun in it.
    vec3 irradiance = sky(n, 1.0, 0.0);
    // The specular lookup is along the reflection, sharpened by how smooth the surface
    // is, and it does see the sun.
    vec3 reflected = sky(reflect(-v, n), roughness, 1.0);
    vec3 fr = fresnel_schlick_roughness(max(dot(n, v), 0.0), f0, roughness);
    // What the sky reflection was multiplied by on its way in, which is what a real
    // reflection has to be multiplied by to take its place.
    vec3 specular_weight = fr * (1.0 - roughness * 0.4) * ambient_strength * ao;
    vec3 ambient = (vec3(1.0) - fr) * (1.0 - metallic) * albedo * irradiance
                     * ambient_strength * ao
                 + reflected * specular_weight;

    // No tone map here: the light is left in linear, high dynamic range, for the bloom
    // pass to find the bright parts of and the composite to map. Where there is no such
    // pass, `tonemap_here` puts it back.
    // The exposure is applied here only when the tone map is: otherwise the composite
    // pass does both, and doing it twice would double it.
    vec3 model_emission = pow(texture(model_emissive,v_uv / max(material_size,vec2(1.0))).rgb,vec3(2.2)) * model_emissive_factor;
    vec3 light = direct + ambient + albedo * emission_value + model_emission;
    vec3 exposed = light * exposure;
    fragment = vec4(mix(light, tonemap(exposed), tonemap_here), surface_alpha);
    // The weight the sky reflection was added at, and the roughness it was blurred by. A
    // surface being seen through is left out of the reflection pass: it has already been
    // blended over what reflects, and feeding it back in would double it.
    reflectivity = surface_alpha < 0.999 ? vec4(0.0) : vec4(specular_weight, roughness);
}
"#;

/// Keeps only what is bright enough to bloom.
///
/// A soft knee rather than a hard cut: with a hard one, a surface drifting past the
/// threshold pops into glowing, and in an editor the camera is always drifting.
pub const BRIGHT_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;

uniform sampler2D source;
uniform float threshold;
uniform float knee;

void main() {
    vec3 color = texture(source, v_uv).rgb;
    float luma = dot(color, vec3(0.2126, 0.7152, 0.0722));
    fragment = vec4(color * smoothstep(threshold, threshold + knee, luma), 1.0);
}
"#;

/// One half of a separable Gaussian blur.
///
/// Separable because a 9x9 blur done in two passes is 18 taps rather than 81, and at
/// bloom's scale that is the difference between free and not.
pub const GAUSSIAN_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;

uniform sampler2D source;
uniform vec2 direction;

void main() {
    // The nine-tap kernel, folded to five by sampling between texels: the hardware's own
    // bilinear filter does half the work.
    const float offsets[3] = float[3](0.0, 1.3846153846, 3.2307692308);
    const float weights[3] = float[3](0.2270270270, 0.3162162162, 0.0702702703);

    vec3 total = texture(source, v_uv).rgb * weights[0];
    for (int i = 1; i < 3; ++i) {
        total += texture(source, v_uv + direction * offsets[i]).rgb * weights[i];
        total += texture(source, v_uv - direction * offsets[i]).rgb * weights[i];
    }
    fragment = vec4(total, 1.0);
}
"#;

/// Adds the bloom, maps the result into what a screen can show, and encodes it.
pub const COMPOSITE_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;

uniform sampler2D scene;
uniform sampler2D bloom;
// What the reflection pass found, as the difference from the sky already reflected.
uniform sampler2D reflections;
uniform float bloom_intensity;
// How much light reaches the film, applied before the tone map.
uniform float exposure;
uniform sampler2D scene_depth;
uniform vec2 clip_planes;
uniform vec2 image_size;
// Focus distance, full sharp range, maximum radius at 1080p.
uniform vec3 dof;
float view_depth(vec2 uv) {
    float z = texture(scene_depth, uv).r * 2.0 - 1.0;
    float n = clip_planes.x, f = clip_planes.y;
    return 2.0 * n * f / max(f + n - z * (f - n), 0.0001);
}
vec3 lighting(vec2 uv) {
    return texture(scene, uv).rgb + texture(reflections, uv).rgb
        + texture(bloom, uv).rgb * bloom_intensity;
}
float blur_radius(float z) {
    float error = max(abs(z - dof.x) - dof.y * 0.5, 0.0);
    return dof.z * clamp(error / max(z, 1.0), 0.0, 1.0);
}

vec3 tonemap(vec3 color) {
    // The ACES curve, in the fitted form everyone uses.
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), 0.0, 1.0);
}

void main() {
    vec3 light = lighting(v_uv);
    if (dof.z > 0.0) {
        float depth = view_depth(v_uv);
        float radius = blur_radius(depth);
        if (radius > 0.05) {
            vec3 sum = light;
            float weight = 1.0;
            // A deterministic disk in linear HDR light, before tone mapping.
            for (int i = 0; i < 32; ++i) {
                float r = sqrt((float(i) + 0.5) / 32.0);
                float angle = float(i) * 2.39996323;
                vec2 offset = vec2(cos(angle), sin(angle)) * r * radius
                    * (image_size.y / 1080.0) / image_size;
                vec2 uv = clamp(v_uv + offset, 0.5 / image_size, 1.0 - 0.5 / image_size);
                float sample_depth = view_depth(uv);
                // Keep a focused foreground object out of a blurred background gather.
                float accept = sample_depth < depth * 0.98
                    ? smoothstep(r * radius - 0.5, r * radius + 0.5, blur_radius(sample_depth)) : 1.0;
                sum += lighting(uv) * accept;
                weight += accept;
            }
            light = sum / weight;
        }
    }
    vec3 mapped = tonemap(light * exposure);
    // Back to what a screen expects. Everything above this line is in linear light, and
    // skipping this is what makes the whole picture come out too dark.
    fragment = vec4(pow(mapped, vec3(1.0 / 2.2)), 1.0);
}
"#;


/// Screen-space reflections.
///
/// Marches the reflection of each pixel through the depth buffer and, where it lands on
/// geometry, works out how much brighter or darker that is than the sky the scene pass
/// already reflected there. The composite adds the difference, so a reflection replaces
/// the sky rather than being piled on top of it -- adding would count the same light
/// twice and make every smooth surface glow.
///
/// It only sees what is on screen. A reflection of something behind the camera, or off
/// the edge of the window, is not there to be found; the confidence fades towards the
/// edges so that what it does find does not end in a hard line.
pub const SSR_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;

uniform sampler2D scene_color;
uniform sampler2D scene_depth;
uniform sampler2D scene_normal;
uniform sampler2D reflectivity;

uniform mat4 projection;
uniform mat4 inverse_projection;
uniform mat4 view;

// The sky, so the pass can work out what it is replacing. The same three bands the scene
// shader uses; kept in step with it by hand, since a shader cannot include another.
uniform vec3 sky_color;
uniform vec3 horizon_color;
uniform vec3 ground_color;
uniform vec3 light_direction;
uniform vec3 light_color;

// How far the march goes, in view units, and how many steps it takes.
uniform float ssr_distance;
uniform float ssr_samples;
// How far behind a surface a sample has to be to count as a hit.
uniform float ssr_thickness;
// Zero turns the pass off, which leaves the sky reflection the scene pass put there.
uniform float ssr_strength;

vec3 view_position(vec2 uv) {
    float depth = texture(scene_depth, uv).r;
    vec4 clip = vec4(uv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
    vec4 position = inverse_projection * clip;
    return position.xyz / position.w;
}

vec3 sky(vec3 direction, float roughness) {
    float up = clamp(direction.z, -1.0, 1.0);
    vec3 above = mix(horizon_color, sky_color, pow(clamp(up, 0.0, 1.0), 0.45));
    vec3 below = mix(horizon_color, ground_color, pow(clamp(-up, 0.0, 1.0), 0.35));
    vec3 result = up >= 0.0 ? above : below;

    vec3 to_sun = normalize(-light_direction);
    float alignment = clamp(dot(direction, to_sun), 0.0, 1.0);
    float sharpness = mix(2000.0, 8.0, clamp(roughness, 0.0, 1.0));
    result += light_color * pow(alignment, sharpness) * (1.0 - roughness * 0.6);
    return result;
}

void main() {
    fragment = vec4(0.0);
    if (ssr_strength <= 0.0) {
        return;
    }

    vec4 surface = texture(reflectivity, v_uv);
    vec3 weight = surface.rgb;
    float roughness = surface.a;
    // Nothing here reflects enough to be worth marching for.
    if (max(weight.r, max(weight.g, weight.b)) < 0.002) {
        return;
    }

    float depth = texture(scene_depth, v_uv).r;
    if (depth >= 1.0) {
        return;
    }

    vec3 origin = view_position(v_uv);
    vec3 normal = normalize(texture(scene_normal, v_uv).xyz * 2.0 - 1.0);
    vec3 direction = normalize(reflect(normalize(origin), normal));

    // A reflection heading towards the camera has nothing on screen to show.
    if (direction.z > 0.0) {
        return;
    }

    int steps = int(max(ssr_samples, 1.0));
    float step_length = ssr_distance / float(steps);

    // Start off the surface, or the first sample reads the surface itself.
    vec3 at = origin + normal * ssr_thickness + direction * step_length;
    vec2 hit_uv = vec2(-1.0);

    for (int i = 0; i < steps; ++i) {
        vec4 clip = projection * vec4(at, 1.0);
        vec2 uv = (clip.xy / clip.w) * 0.5 + 0.5;
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
            break;
        }

        float stored_z = view_position(uv).z;
        float difference = stored_z - at.z;
        if (difference > ssr_thickness && difference < ssr_thickness + step_length * 2.0) {
            hit_uv = uv;
            break;
        }

        at += direction * step_length;
    }

    if (hit_uv.x < 0.0) {
        return;
    }

    // The sky this pixel already has reflected in it, in world space, so the difference
    // is what the geometry adds or takes away.
    mat3 to_world = transpose(mat3(view));
    vec3 world_direction = to_world * direction;
    vec3 replaced = sky(world_direction, roughness);
    vec3 found = texture(scene_color, hit_uv).rgb;

    // Fade towards the edges of the window, where the march runs out of picture, and with
    // roughness, since a rough surface reflects too widely for one ray to stand for.
    vec2 fade = smoothstep(vec2(0.0), vec2(0.12), hit_uv)
              * smoothstep(vec2(0.0), vec2(0.12), 1.0 - hit_uv);
    float confidence = fade.x * fade.y * (1.0 - roughness) * ssr_strength;

    fragment = vec4((found - replaced) * weight * confidence, 1.0);
}
"#;

/// Averages this frame into what the still view has gathered so far.
///
/// Antialiasing by accumulation. Each frame is drawn with the sample point moved about
/// inside the pixel, so averaging them gives an edge that is genuinely smooth rather than
/// one a post-process guessed the shape of. The weight is one over the count, which makes
/// the running average exact rather than an exponential fade -- an early frame counts as
/// much as a late one, which is what stops the picture settling on the first thing it saw.
pub const ACCUMULATE_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;

uniform sampler2D current;
uniform sampler2D history;
// One over the number of frames gathered including this one. At one the history is
// ignored, which is what makes the first frame of a new view replace the old one.
uniform float weight;

void main() {
    vec3 now = texture(current, v_uv).rgb;
    if (weight >= 1.0) {
        fragment = vec4(now, 1.0);
        return;
    }
    vec3 before = texture(history, v_uv).rgb;
    fragment = vec4(mix(before, now, weight), 1.0);
}
"#;

/// Fast approximate anti-aliasing.
///
/// Works on the finished picture rather than on the geometry, which is what lets it run
/// after the tone map and cost one pass instead of a multisampled framebuffer. It finds
/// edges by their change in brightness and blurs along them.
pub const FXAA_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;

uniform sampler2D source;
uniform vec2 texel;
// How much of the softened result is kept. Zero passes the picture straight through.
uniform float amount;

const float EDGE_THRESHOLD = 0.125;
const float EDGE_THRESHOLD_MIN = 0.0312;
const float SUBPIXEL = 0.75;

float luma(vec3 color) {
    return dot(color, vec3(0.299, 0.587, 0.114));
}

void main() {
    vec3 middle = texture(source, v_uv).rgb;
    if (amount <= 0.0) {
        fragment = vec4(middle, 1.0);
        return;
    }

    float luma_middle = luma(middle);
    float luma_north = luma(texture(source, v_uv + vec2(0.0, -texel.y)).rgb);
    float luma_south = luma(texture(source, v_uv + vec2(0.0, texel.y)).rgb);
    float luma_west = luma(texture(source, v_uv + vec2(-texel.x, 0.0)).rgb);
    float luma_east = luma(texture(source, v_uv + vec2(texel.x, 0.0)).rgb);

    float lowest = min(luma_middle, min(min(luma_north, luma_south), min(luma_west, luma_east)));
    float highest = max(luma_middle, max(max(luma_north, luma_south), max(luma_west, luma_east)));
    float range = highest - lowest;

    // Flat enough that there is no edge here to soften.
    if (range < max(EDGE_THRESHOLD_MIN, highest * EDGE_THRESHOLD)) {
        fragment = vec4(middle, 1.0);
        return;
    }

    // Which way the edge runs: the direction brightness changes least along.
    float horizontal = abs(luma_west + luma_east - 2.0 * luma_middle);
    float vertical = abs(luma_north + luma_south - 2.0 * luma_middle);
    vec2 along = vertical >= horizontal ? vec2(1.0, 0.0) : vec2(0.0, 1.0);

    // How far off the middle the neighbourhood's average sits, which is how much of a
    // blur this pixel wants.
    float average = (luma_north + luma_south + luma_west + luma_east) * 0.25;
    float blend = clamp(abs(average - luma_middle) / range, 0.0, 1.0) * SUBPIXEL;

    vec3 blurred = 0.5 * (texture(source, v_uv + along * texel).rgb
                        + texture(source, v_uv - along * texel).rgb);
    fragment = vec4(mix(middle, 0.5 * (middle + blurred), blend), 1.0);
}
"#;

/// A camera-facing sprite: the quad is built in world space each frame, so the vertex
/// shader only projects it. Drawn after the composite, in sRGB, depth-tested against the
/// scene so a sprite behind a wall is hidden.
pub const SPRITE_VERTEX: &str = r#"#version 150 core
in vec3 position;
in vec2 uv;

uniform mat4 view_projection;

out vec2 v_uv;

void main() {
    v_uv = uv;
    gl_Position = view_projection * vec4(position, 1.0);
}
"#;

pub const SPRITE_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;

uniform sampler2D sprite;

out vec4 fragment;

void main() {
    vec4 texel = texture(sprite, v_uv);
    // A sprite's transparent pixels carry no colour worth blending.
    if (texel.a < 0.02) {
        discard;
    }
    fragment = texel;
}
"#;

/// Directional sky at infinity. Uses the same environment colours and sun as the
/// scene's ambient lighting, with a soft solar disc and horizon haze.
pub const SKY_FRAGMENT: &str = r#"#version 150 core
in vec2 v_uv;
out vec4 fragment;
uniform mat4 inverse_view_projection;
uniform vec3 camera_position;
uniform vec3 sky_color;
uniform vec3 horizon_color;
uniform vec3 ground_color;
uniform vec3 sun_color;
uniform vec3 to_sun;
uniform float exposure;
uniform float tonemap_here;
vec3 tonemap(vec3 c) {
    c = max(c, vec3(0.0));
    c = clamp((c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14), 0.0, 1.0);
    return pow(c, vec3(1.0 / 2.2));
}
void main() {
    vec4 world = inverse_view_projection * vec4(v_uv * 2.0 - 1.0, 0.5, 1.0);
    vec3 direction = normalize(world.xyz / world.w - camera_position);
    float up = clamp(direction.z, -1.0, 1.0);
    vec3 sun = normalize(to_sun);
    float sun_height = sun.z;
    // A gently darkening twilight, with warmer haze towards the low sun.
    float daylight = smoothstep(-0.12, 0.08, sun_height);
    float sunset = (1.0 - smoothstep(0.04, 0.4, abs(sun_height))) * daylight;
    float facing = pow(max(dot(direction.xy, sun.xy), 0.0), 3.0);
    vec3 horizon = mix(horizon_color, vec3(1.0, 0.40, 0.12), sunset * facing * 0.55);
    vec3 zenith = mix(sky_color, sky_color * vec3(0.65, 0.55, 0.85), sunset * 0.3);
    vec3 above = mix(horizon, zenith, pow(max(up, 0.0), 0.38));
    vec3 below = mix(horizon, ground_color, pow(max(-up, 0.0), 0.35));
    vec3 color = (up >= 0.0 ? above : below) * mix(0.055, 1.0, daylight);

    // Use angular distance rather than an almost-one dot threshold. This keeps
    // the disc edge stable, including pixels looking straight at its centre.
    float angle = atan(length(cross(direction, sun)), dot(direction, sun));
    float radius = radians(0.75); // Readable, stylized 1.5-degree solar disc.
    float edge = max(fwidth(angle), 0.00002);
    float disc = 1.0 - smoothstep(radius - edge, radius + edge, angle);
    float limb = sqrt(max(0.0, 1.0 - pow(angle / radius, 2.0)));
    float halo = 0.65 * exp(-angle * 18.0) + 0.12 * exp(-angle * 3.0);
    vec3 warmth = mix(vec3(1.0), vec3(1.0, 0.48, 0.16), sunset);
    // Clip at the horizon, so a setting sun disappears from the bottom up.
    float horizon_edge = max(fwidth(up), 0.0001);
    float visible = smoothstep(-horizon_edge, horizon_edge, up);
    color += sun_color * warmth * (disc * (9.0 + 3.0 * limb) * visible
        + halo * smoothstep(-0.12, 0.06, up))
        * smoothstep(-0.06, 0.0, sun_height + radius);
    fragment = vec4(mix(color, tonemap(color * exposure), tonemap_here), 1.0);
}
"#;
