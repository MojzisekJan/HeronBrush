// Textured, with a cheap fake sun: a face pointing up is full bright, a face
// pointing down sits at the ambient floor. No real lights yet.
varying mediump vec3 var_normal;
varying mediump vec2 var_texcoord0;

uniform lowp sampler2D tex0;

void main() {
    vec4 color = texture2D(tex0, var_texcoord0.xy);
    if (color.a < 0.5) discard;

    float ambient = 0.55;
    float lit = ambient + (1.0 - ambient) * clamp(var_normal.y * 0.5 + 0.5, 0.0, 1.0);
    gl_FragColor = vec4(color.rgb * lit, 1.0);
}
