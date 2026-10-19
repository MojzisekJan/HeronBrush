shader_type spatial;
// Sits right in front of the camera and covers it. No depth test, because it
// has to draw over everything; no culling, because which way it faces is not
// something worth thinking about.
render_mode blend_mix, unshaded, cull_disabled, depth_test_disabled, depth_draw_never, shadows_disabled, fog_disabled;

// The other half of the water. The surface shader draws the boundary; this
// draws being on the wrong side of it. Without it, going under looks like
// walking around in air with a plane floating overhead.

/// Height of the water surface in world space. The player sets this from
/// whichever volume it is in.
uniform float water_level = 0.0;
/// What everything fades towards as it gets further away.
uniform vec3 underwater_colour : source_color = vec3(0.03, 0.20, 0.30);
/// How fast that happens, per metre.
uniform float absorption : hint_range(0.0, 2.0) = 0.28;
/// How much the colour takes over even close up.
uniform float tint : hint_range(0.0, 1.0) = 0.45;
/// How far the view swims about, as a fraction of the screen.
uniform float wobble : hint_range(0.0, 0.03) = 0.004;
uniform float wobble_speed : hint_range(0.0, 3.0) = 0.6;
/// How dark the corners go.
uniform float vignette : hint_range(0.0, 1.0) = 0.35;

uniform sampler2D screen_texture : hint_screen_texture, filter_linear_mipmap;
uniform sampler2D depth_texture : hint_depth_texture, filter_linear;

float hash12(vec2 p) {
	uvec2 q = uvec2(ivec2(p)) * uvec2(1597334677u, 3812015801u);
	uint n = (q.x ^ q.y) * 1597334677u;
	return float(n) * (1.0 / 4294967295.0);
}

float value_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return -1.0 + 2.0 * mix(
		mix(hash12(i + vec2(0.0, 0.0)), hash12(i + vec2(1.0, 0.0)), u.x),
		mix(hash12(i + vec2(0.0, 1.0)), hash12(i + vec2(1.0, 1.0)), u.x), u.y);
}

void fragment() {
	vec2 drift = vec2(
		value_noise(SCREEN_UV * 12.0 + TIME * wobble_speed * vec2(1.0, 0.4)),
		value_noise(SCREEN_UV * 12.0 + TIME * wobble_speed * vec2(0.6, 1.0))
	);
	vec2 uv = SCREEN_UV + drift * wobble;

	vec3 scene = texture(screen_texture, uv).rgb;

	// Where the scene behind this pixel is, in front of the camera.
	float raw = texture(depth_texture, uv).x;
	vec3 ndc = vec3(uv * 2.0 - 1.0, raw);
#if CURRENT_RENDERER == RENDERER_COMPATIBILITY
	ndc.z = raw * 2.0 - 1.0;
#endif
	vec4 view = INV_PROJECTION_MATRIX * vec4(ndc, 1.0);
	vec3 view_pos = view.xyz / view.w;
	float away = -view_pos.z;
	float along = length(view_pos);

	// Whether this pixel is being looked at through water.
	//
	// The eye first: this draws being underwater, so an eye above the surface
	// has nothing to say, whatever the body is doing. Looking down into a pool
	// from the bank is the surface shader's business, not this one's.
	vec3 eye = INV_VIEW_MATRIX[3].xyz;
	if (eye.y >= water_level) {
		discard;
	}
	// Then the ray. Looking up from under the surface, it leaves the water at
	// some distance; anything further away than that is out of the water and is
	// left alone. Worked out from the ray rather than from the quad, because
	// the quad tilts with the view and its own height would put a straight edge
	// across the picture.
	vec3 ray = normalize(mat3(INV_VIEW_MATRIX) * normalize(view_pos));
	if (ray.y > 0.0001) {
		float crossing = (water_level - eye.y) / ray.y;
		if (crossing < along) {
			discard;
		}
	}

	vec3 colour = mix(scene, underwater_colour, tint);
	colour = mix(underwater_colour, colour, exp(-away * absorption));

	// Corners darker, which is most of what makes a screen effect read as
	// being inside something rather than as a colour filter.
	float corner = distance(SCREEN_UV, vec2(0.5));
	colour *= 1.0 - smoothstep(0.35, 0.85, corner) * vignette;

	ALBEDO = colour;
	ALPHA = 1.0;
}
