varying highp vec3 var_direction;
uniform lowp samplerCube sky;
void main(){gl_FragColor=vec4(textureCube(sky,var_direction).rgb,1.);}
