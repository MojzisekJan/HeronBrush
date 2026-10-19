varying mediump vec2 var_texcoord0;
uniform lowp sampler2D tex0;
uniform lowp vec4 water_tint;
void main(){gl_FragColor=texture2D(tex0,var_texcoord0)*water_tint;}
