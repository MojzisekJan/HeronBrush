// The baseline water surface: the volume's upward faces, tinted, see-through from
// both sides and rocked by a small wave. Collision does not follow the wave.
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled;

// Linear colour; alpha is the surface's opacity.
uniform vec4 tint = vec4(0.05, 0.38, 0.48, 0.55);
uniform sampler2D albedo_texture : source_color, hint_default_white, filter_linear_mipmap, repeat_enable;
uniform float wave_height = 0.035;
uniform float wave_speed = 1.5;

void vertex() {
	VERTEX.y += smoothstep(0.5, 0.9, NORMAL.y) * wave_height
		* sin(VERTEX.x * 1.7 + TIME * wave_speed) * cos(VERTEX.z * 1.3 - TIME * wave_speed * 0.8);
}

void fragment() {
	vec4 color = texture(albedo_texture, UV) * tint;
	ALBEDO = color.rgb;
	ALPHA = color.a;
	ROUGHNESS = 0.14;
	METALLIC = 0.05;
}
