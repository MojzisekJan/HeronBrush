shader_type spatial;
// Not cull_back: the player swims inside this, and the surface has to be there
// when you look up at it from below.
render_mode blend_mix, depth_draw_never, cull_disabled, diffuse_burley, specular_schlick_ggx;

// Built along the lines of the stylized water shader at gameidea.org, fitted to
// brush level geometry rather than to a subdivided ocean plane. The
// one thing that could not come across is vertex displacement: a brush face has
// four corners, so there is nothing to displace and every wave lives in the
// normal.

group_uniforms colour;
/// The water's own colour where it is shallow.
uniform vec3 shallow_colour : source_color = vec3(0.44, 0.95, 0.98);
/// And where it is deep.
uniform vec3 deep_colour : source_color = vec3(0.0, 0.34, 0.29);
/// What is left when the water is deep enough to hide everything.
uniform vec3 fog_colour : source_color = vec3(0.0, 0.06, 0.11);
/// How fast each channel is swallowed, per metre of water. Red goes first in
/// the real thing, which is the whole reason deep water reads as blue, so keep
/// the red component the largest unless you want something other than water.
uniform vec3 absorption : source_color = vec3(0.45, 0.11, 0.05);
/// Metres of water before the colour reaches `deep_colour`.
uniform float max_depth : hint_range(0.1, 50.0) = 4.0;
/// Metres of water before the colour starts to shift at all.
uniform float fade_start : hint_range(0.0, 10.0) = 0.15;
/// How much the water's colour multiplies what is under it. At 1 the bottom is
/// stained the full colour of the water, which is what real water does and what
/// makes a dark floor come out nearly black. Lower it to keep the water bright
/// whatever it is standing on.
uniform float seabed_tint : hint_range(0.0, 1.0) = 1.0;
/// How much of its own colour the water lays over what is underneath, at full
/// depth. Real water leaves that to absorption, which over the two metres of a
/// small pool hardly does anything and leaves the bottom showing through as
/// grey. Painted water is simply opaque, and more so the deeper it gets.
uniform float opacity : hint_range(0.0, 1.0) = 0.0;
/// How much of that opacity is already there in the shallowest water. Zero
/// means shallow water is clear glass, which is right for a sandy edge and
/// wrong along a wall: the wall is close under the surface, so it counts as
/// shallows and shows through bright and busy in a band all the way round.
uniform float opacity_floor : hint_range(0.0, 1.0) = 0.0;
/// Number of steps to break the depth into. Zero is a smooth gradient. Three or
/// four gives the flat bands of painted water, which is most of what separates
/// a cartoon sea from a photograph of one.
uniform int colour_bands : hint_range(0, 12) = 0;

group_uniforms waves;
/// Height of the notional waves. Nothing moves, but this is what sets how far
/// the surface tilts.
uniform float wave_height : hint_range(0.0, 2.0) = 0.35;
/// Sharpness of the wave crests. High is choppy sea, low is a still pool.
uniform float wave_choppy : hint_range(0.0, 10.0) = 3.5;
uniform float wave_speed : hint_range(0.0, 5.0) = 0.6;
/// Waves per metre. Bigger number, smaller waves.
uniform float wave_freq : hint_range(0.0, 4.0) = 0.55;
uniform float wave_strength : hint_range(0.0, 4.0) = 1.0;
uniform int wave_octaves : hint_range(1, 6) = 4;
/// Where the ripples start being averaged away, in metres from the camera.
/// Without this, distant water shimmers as the detail fights for pixels.
uniform float smoothing_distance : hint_range(1.0, 200.0) = 18.0;

group_uniforms foam;
uniform vec3 foam_colour : source_color = vec3(1.0, 1.0, 1.0);
/// Darker line just off the foam, which is what gives it thickness.
uniform vec3 foam_edge_colour : source_color = vec3(0.0, 0.05, 0.12);
/// Depth at which foam starts, in metres. Foam fills in below this.
uniform float foam_depth : hint_range(0.0, 5.0) = 0.7;
/// Size of the foam clumps. Bigger number, smaller clumps.
uniform float foam_scale : hint_range(0.1, 20.0) = 2.2;
uniform float foam_speed : hint_range(0.0, 2.0) = 0.7;
/// How much of the clump pattern counts as foam. Higher leaves less.
uniform float foam_cutoff : hint_range(0.0, 1.0) = 0.62;
/// How much foam the wave crests carry, away from any shore. Pools want none.
uniform float foam_on_crests : hint_range(0.0, 10.0) = 0.0;
uniform vec2 foam_edge_offset = vec2(0.05, 0.05);
/// Size of the bubbles inside the foam.
uniform float bubble_scale : hint_range(0.0, 50.0) = 14.0;
uniform float bubble_strength : hint_range(0.0, 8.0) = 0.9;

group_uniforms caustics;
uniform sampler2D caustics_texture : filter_linear_mipmap, repeat_enable;
uniform float caustics_scale : hint_range(0.1, 10.0) = 1.6;
uniform float caustics_speed : hint_range(0.0, 1.0) = 0.06;
uniform float caustics_intensity : hint_range(0.0, 2.0) = 0.5;
/// How quickly the light on the bottom fades as the water deepens.
uniform float caustics_depth_fade : hint_range(0.0, 1.0) = 0.35;

group_uniforms surface;
uniform float refraction : hint_range(0.0, 2.0) = 0.6;
/// Refraction is eased off further away, where it would only be noise.
uniform float refraction_distance : hint_range(0.0, 100.0) = 6.0;
uniform float water_roughness : hint_range(0.0, 1.0) = 0.08;
uniform float water_specular : hint_range(0.0, 1.0) = 0.6;

uniform sampler2D screen_texture : hint_screen_texture, filter_linear_mipmap;
uniform sampler2D depth_texture : hint_depth_texture, filter_linear;

const mat2 OCTAVE_TURN = mat2(vec2(1.6, 1.2), vec2(-1.2, 1.6));

varying vec3 world_vertex;
varying vec3 flat_normal;

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

/// One layer of waves. The sine and cosine are mixed by their own value, which
/// is what puts sharp crests between round troughs instead of a plain ripple.
float wave_octave(vec2 uv, float choppy) {
	uv += value_noise(uv);
	vec2 wv = 1.0 - abs(sin(uv));
	vec2 swv = abs(cos(uv));
	wv = mix(wv, swv, wv);
	return pow(1.0 - pow(wv.x * wv.y, 0.65), choppy);
}

/// Height of the water surface at a world position. Only ever used to work out
/// which way the surface is facing.
float wave_height_at(vec2 world_xz, float time, int octaves) {
	float freq = wave_freq;
	float amp = wave_height;
	float choppy = wave_choppy;
	vec2 uv = world_xz;
	uv.x *= 0.75;
	float h = 0.0;
	for (int i = 0; i < octaves; i++) {
		h += (wave_octave((uv + time * wave_speed) * freq, choppy)
			+ wave_octave((uv - time * wave_speed) * freq, choppy)) * amp;
		uv *= OCTAVE_TURN;
		freq *= 1.9;
		amp *= 0.22;
		choppy = mix(choppy, 1.0, 0.2);
	}
	return h;
}

/// Which way the surface faces, from the slope between two points a step apart.
/// The step is the whole story: a large one averages the waves away, a small
/// one keeps every crease, which is what makes it useful for distance fading.
vec3 wave_normal(vec2 world_xz, float step_size, float time, int octaves) {
	float here = wave_height_at(world_xz, time, octaves);
	vec3 n;
	n.x = here - wave_height_at(world_xz + vec2(step_size, 0.0), time, octaves);
	n.z = here - wave_height_at(world_xz + vec2(0.0, step_size), time, octaves);
	n.y = step_size / max(wave_strength, 0.001);
	return normalize(n);
}

vec2 hash22(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.xx + p3.yz) * p3.zy);
}

/// Distance to the nearest of a set of drifting points, one per grid cell. The
/// cells are what foam is made of here: clumps with edges, not a smooth fog.
float voronoi(vec2 uv, float time) {
	vec2 cell = floor(uv);
	vec2 within = fract(uv);
	float nearest = 1.0;
	for (int j = -1; j <= 1; j++) {
		for (int i = -1; i <= 1; i++) {
			vec2 step_to = vec2(float(i), float(j));
			vec2 point = hash22(cell + step_to);
			point = 0.5 + 0.5 * sin(time * foam_speed + 6.2831 * point);
			vec2 away = step_to - within + point;
			nearest = min(nearest, dot(away, away));
		}
	}
	return nearest;
}

float voronoi_layers(vec2 uv, float time) {
	float total = 0.0;
	float amp = 0.5;
	mat2 turn = mat2(vec2(cos(0.5), sin(0.5)), vec2(-sin(0.5), cos(0.5)));
	for (int i = 0; i < 3; i++) {
		total += amp * pow(1.0 - voronoi(uv, time), 2.0);
		uv = turn * uv * 2.0 + vec2(100.0);
		amp *= 0.5;
	}
	return total;
}

/// How far away the first thing behind this pixel is, in metres.
float linear_depth(vec2 uv, mat4 inverse_projection) {
	float raw = texture(depth_texture, uv).x;
	vec3 ndc = vec3(uv * 2.0 - 1.0, raw);
#if CURRENT_RENDERER == RENDERER_COMPATIBILITY
	ndc.z = raw * 2.0 - 1.0;
#endif
	vec4 view = inverse_projection * vec4(ndc, 1.0);
	return -view.z / view.w;
}

/// Where in the world the thing behind this pixel is.
vec3 world_behind(vec2 uv, mat4 inverse_projection, mat4 inverse_view) {
	float raw = texture(depth_texture, uv).x;
	vec3 ndc = vec3(uv * 2.0 - 1.0, raw);
#if CURRENT_RENDERER == RENDERER_COMPATIBILITY
	ndc.z = raw * 2.0 - 1.0;
#endif
	vec4 world = inverse_view * inverse_projection * vec4(ndc, 1.0);
	return world.xyz / world.w;
}

void vertex() {
	world_vertex = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	flat_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	// The waves are laid out in world space, not in the brush's own UVs. A
	// brush face can be scaled and rotated any way at all, and letting
	// that decide the size of the waves means every pool ripples differently.
	vec2 world_xz = world_vertex.xz;
	float away = length(VERTEX);

	// Further off, the step between the two height samples grows, which
	// averages the small waves out. Keeping every crease at every distance is
	// what makes water shimmer and crawl.
	float step_size = max(0.01, away * 0.006);
	vec3 detailed = wave_normal(world_xz, step_size, TIME, wave_octaves);
	float flatten = clamp((away - smoothing_distance) / (smoothing_distance * 4.0), 0.0, 1.0);
	vec3 surface_normal = normalize(mix(detailed, flat_normal, flatten));
	NORMAL = (VIEW_MATRIX * vec4(surface_normal, 0.0)).xyz;

	float surface_distance = -VERTEX.z;
	float behind = linear_depth(SCREEN_UV, INV_PROJECTION_MATRIX);
	float thickness = max(0.0, behind - surface_distance);

	// What is underneath, bent by the waves. How far it bends eases off with
	// distance, and stops entirely where the water is too shallow to refract.
	vec3 view_flat = (VIEW_MATRIX * vec4(flat_normal, 0.0)).xyz;
	vec2 bend = (NORMAL.xy - view_flat.xy)
		* refraction
		* clamp(refraction_distance / max(0.1, away), 0.0, 1.0)
		* smoothstep(0.0, max_depth, thickness)
		* 0.05;
	vec2 bent_uv = SCREEN_UV + bend;

	// A bent sample that lands on something in front of the water is thrown
	// away, or a leg standing beside the pool smears across the surface.
	float bent_behind = linear_depth(bent_uv, INV_PROJECTION_MATRIX);
	if (bent_behind < surface_distance - 0.001) {
		bent_uv = SCREEN_UV;
		bent_behind = behind;
	}
	vec3 under = texture(screen_texture, bent_uv).rgb;
	thickness = max(0.0, bent_behind - surface_distance);

	// How deep the water is here, measured straight down rather than along the
	// line of sight. The two are very different at a low angle: sighting along
	// a pool puts metres of water in front of every pixel, so a colour taken
	// from that slides about as the camera turns and the band of foam at the
	// shore collapses. Straight down is a property of the pool, so the colours
	// follow the bottom and stay put.
	vec3 floor_pos = world_behind(bent_uv, INV_PROJECTION_MATRIX, INV_VIEW_MATRIX);
	float depth_here = clamp(world_vertex.y - floor_pos.y, 0.0, max_depth);

	// Seen from underneath, there is no bottom behind the surface, there is
	// sky. Measuring downwards then gives nought everywhere, the whole surface
	// counts as shallows, and it comes out covered in foam. So from below the
	// old measure along the line of sight is used instead, which is the
	// distance out through the far side of the pool and reads as deep water.
	bool from_below = !FRONT_FACING;
	float depth_used = from_below ? thickness : depth_here;
	float depth_ratio = clamp((depth_used - fade_start) / max(0.001, max_depth - fade_start), 0.0, 1.0);
	// Stepped rather than smooth, if asked for. The steps are taken before the
	// colour is worked out, so the bands land on the water itself and not on
	// whatever happens to be lying on the bottom.
	if (colour_bands > 0) {
		depth_ratio = floor(depth_ratio * float(colour_bands)) / float(colour_bands - 1);
		depth_ratio = clamp(depth_ratio, 0.0, 1.0);
	}

	// Light bent by the surface, gathered into bright lines on whatever the
	// water is standing on. Two layers drifting apart, multiplied, so the
	// pattern moves without ever repeating obviously.
	// The pattern is projected straight down, so on a wall every height along it
	// lands on the same spot in the texture and the light smears into vertical
	// stripes. Which way the surface behind faces comes from how the rebuilt
	// position shifts between neighbouring pixels, and the light is taken away
	// wherever that surface is not lying roughly flat.
	vec3 behind_normal = normalize(cross(dFdy(floor_pos), dFdx(floor_pos)));
	float lies_flat = abs(behind_normal.y);
	lies_flat *= lies_flat;
	if (!from_below && thickness > 0.0 && caustics_intensity > 0.0 && lies_flat > 0.01) {
		vec2 first = floor_pos.xz * caustics_scale + vec2(TIME * caustics_speed, 0.0);
		vec2 second = floor_pos.xz * caustics_scale * 0.7 - vec2(0.0, TIME * caustics_speed * 0.8);
		float light = texture(caustics_texture, first).r * texture(caustics_texture, second).r;
		under += light * caustics_intensity * lies_flat
			* (1.0 - clamp(thickness * caustics_depth_fade, 0.0, 1.0));
	}

	// Beer and Lambert: each channel is swallowed at its own rate, so the
	// colour of deep water is what is left rather than a colour chosen for it.
	vec3 through = exp(-thickness * absorption);
	vec3 volume = mix(shallow_colour, deep_colour, depth_ratio);
	ALBEDO = mix(fog_colour, under * mix(vec3(1.0), volume, seabed_tint), through);
	// Laid over the top rather than mixed into the absorption, so that turning
	// it up cannot make the water darker, only more its own colour.
	ALBEDO = mix(ALBEDO, volume, opacity * mix(opacity_floor, 1.0, depth_ratio));

	// Foam where the water runs out against something solid, and on crests if
	// this is meant to be a sea rather than a pool.
	// No foam from below either. Foam sits on top of water; from underneath
	// there is nothing to sit on and it only ever looked like white spots
	// drifting about on the ceiling.
	float shore = from_below ? 0.0 : smoothstep(foam_depth, 0.0, depth_here);
	float crest = from_below ? 0.0 : smoothstep(0.7, 0.6, surface_normal.y) * foam_on_crests;
	float foam_level = clamp(shore + crest, 0.0, 1.0);

	// The clumps are dragged about by a slow noise, so the foam looks carried
	// by the water instead of painted onto it.
	vec2 flow = world_xz * 0.5 + TIME * 0.05 * foam_speed;
	vec2 warp = vec2(value_noise(flow), value_noise(flow + vec2(5.2, 1.3))) * 0.5;

	float foam = 0.0;
	float foam_edge = 0.0;
	if (foam_level > 0.001) {
		float clumps = voronoi_layers(world_xz * foam_scale + warp, TIME) + foam_level;
		float shifted = voronoi_layers((world_xz + foam_edge_offset) * foam_scale + warp, TIME)
			+ foam_level;
		foam = smoothstep(foam_cutoff + 0.5, foam_cutoff + 0.6, clumps);
		// The same shape a step to the side, minus the foam itself, leaves a
		// line along one edge of every clump. That line is what stops the foam
		// looking like a flat white sticker.
		foam_edge = clamp(smoothstep(foam_cutoff + 0.5, foam_cutoff + 0.6, shifted) - foam, 0.0, 1.0);

		float bubbles = smoothstep(0.2, 0.8, 1.0 - sqrt(voronoi(world_xz * bubble_scale + warp, TIME)));
		foam *= 1.0 - clamp((1.0 - bubbles) * bubble_strength, 0.0, 1.0);
	}

	ALBEDO = mix(ALBEDO, foam_edge_colour, foam_edge);
	ALBEDO = mix(ALBEDO, foam_colour, foam);

	ROUGHNESS = mix(water_roughness, 0.8, foam + foam_edge);
	METALLIC = 0.0;
	SPECULAR = water_specular;
}
