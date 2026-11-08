// Generated defines select only the samplers present in this material.
varying highp vec3 var_position;
varying mediump vec3 var_normal;
varying mediump vec2 var_texcoord0;
#ifdef HB_ALBEDO
uniform lowp sampler2D tex0;
#endif
#ifdef HB_NORMAL
uniform lowp sampler2D tex_normal;
#endif
#ifdef HB_MR
uniform lowp sampler2D tex_mr;
#endif
#ifdef HB_ORM
uniform lowp sampler2D tex_orm;
#endif
#ifdef HB_ROUGHNESS
uniform lowp sampler2D tex_roughness;
#endif
#ifdef HB_METALLIC
uniform lowp sampler2D tex_metallic;
#endif
#ifdef HB_AO
uniform lowp sampler2D tex_ao;
#endif
#ifdef HB_EMISSIVE
uniform lowp sampler2D tex_emissive;
#endif
uniform mediump vec4 surface_params; // roughness, metallic, albedo-coloured emission, normal scale
uniform mediump vec4 emission_params; // linear emission colour, alpha mode: opaque/mask/blend
uniform mediump vec4 extra_params; // occlusion strength, alpha cutoff, double sided
uniform highp vec4 camera_position;
uniform mediump vec4 sun_direction;
uniform mediump vec4 sun_color;
uniform mediump vec4 ambient_color;
uniform highp vec4 light_positions[8];
uniform mediump vec4 light_colors[8];
uniform mediump vec4 light_directions[8];
const float PI=3.14159265359;
vec3 brdf(vec3 N,vec3 V,vec3 L,vec3 base,float rough,float metal) {
    float nl=clamp(dot(N,L),0.0,1.0), nv=clamp(dot(N,V),0.001,1.0);
    vec3 sum=V+L;vec3 H=sum*inversesqrt(max(dot(sum,sum),0.0001));
    float nh=clamp(dot(N,H),0.0,1.0), vh=clamp(dot(V,H),0.0,1.0);
    float a=rough*rough, a2=a*a;
    float d=nh*nh*(a2-1.0)+1.0;
    float D=a2/max(PI*d*d,0.00001);
    float k=(rough+1.0)*(rough+1.0)/8.0;
    float G=(nl/(nl*(1.0-k)+k))*(nv/(nv*(1.0-k)+k));
    vec3 F=mix(vec3(0.04),base,metal);
    F=F+(1.0-F)*pow(1.0-vh,5.0);
    vec3 spec=D*G*F/max(4.0*nl*nv,0.001);
    return ((1.0-F)*(1.0-metal)*base/PI+spec)*nl;
}
void main() {
    if(!gl_FrontFacing && extra_params.z<0.5) discard;
    vec4 albedo=vec4(1.0);
#ifdef HB_ALBEDO
    albedo=texture2D(tex0,var_texcoord0);
    // Default Defold texture profiles store premultiplied RGB. Light straight colour.
    albedo.rgb=mix(albedo.rgb,albedo.rgb/max(albedo.a,0.0001),extra_params.w);
#endif
    if(emission_params.w>0.5 && emission_params.w<1.5 && albedo.a<extra_params.y) discard;
    vec3 base=pow(max(albedo.rgb,vec3(0.0)),vec3(2.2));
    float rough=surface_params.x,metal=surface_params.y,ao=1.0;
#ifdef HB_MR
    vec4 mr=texture2D(tex_mr,var_texcoord0);rough*=mr.g;metal*=mr.b;
#endif
#ifdef HB_ORM
    vec4 orm=texture2D(tex_orm,var_texcoord0);ao=orm.r;rough*=orm.g;metal*=orm.b;
#endif
#ifdef HB_ROUGHNESS
    rough*=texture2D(tex_roughness,var_texcoord0).r;
#endif
#ifdef HB_METALLIC
    metal*=texture2D(tex_metallic,var_texcoord0).r;
#endif
#ifdef HB_AO
    ao=texture2D(tex_ao,var_texcoord0).r;
#endif
    rough=clamp(rough,0.045,1.0);metal=clamp(metal,0.0,1.0);
    ao=mix(1.0,ao,extra_params.x);
    vec3 N=normalize(var_normal);if(!gl_FrontFacing) N=-N;
#ifdef HB_NORMAL
    vec3 dp1=dFdx(var_position),dp2=dFdy(var_position);
    vec2 duv1=dFdx(var_texcoord0),duv2=dFdy(var_texcoord0);
    vec3 T=cross(dp2,N)*duv1.x+cross(N,dp1)*duv2.x;
    vec3 B=cross(dp2,N)*duv1.y+cross(N,dp1)*duv2.y;
    float inv=inversesqrt(max(max(dot(T,T),dot(B,B)),0.000001));
    vec3 mapped=texture2D(tex_normal,var_texcoord0).xyz*2.0-1.0;
    mapped.xy*=surface_params.w;
    N=normalize(mat3(T*inv,B*inv,N)*mapped);
#endif
    vec3 V=normalize(camera_position.xyz-var_position+vec3(0.000001));
    vec3 lit=base*ambient_color.rgb*ao*(1.0-metal*0.85);
    if(dot(sun_color.rgb,vec3(1.0))>0.0) lit+=brdf(N,V,normalize(sun_direction.xyz),base,rough,metal)*sun_color.rgb;
    for(int i=0;i<8;i++) {
        if(light_colors[i].w<=0.0 || dot(light_colors[i].rgb,vec3(1.0))<=0.0) continue;
        vec3 delta=light_positions[i].xyz-var_position;
        float distance_to_light=length(delta);
        vec3 L=delta/max(distance_to_light,0.0001);
        float attenuation=1.0;
        if(light_positions[i].w<=0.0) { L=-light_directions[i].xyz; }
        else {
            float range=light_positions[i].w;
            float fade=clamp(1.0-pow(distance_to_light/range,4.0),0.0,1.0);
            attenuation=fade*fade/max(distance_to_light*distance_to_light,1.0);
            if(light_directions[i].w>0.0) {
                float cosine=dot(-L,light_directions[i].xyz);
                attenuation*=smoothstep(light_directions[i].w,min(1.0,light_directions[i].w+0.05),cosine);
            }
        }
        lit+=brdf(N,V,L,base,rough,metal)*light_colors[i].rgb*attenuation;
    }
    vec3 emission=emission_params.rgb;
#ifdef HB_EMISSIVE
    emission*=pow(texture2D(tex_emissive,var_texcoord0).rgb,vec3(2.2));
#endif
    lit+=emission+base*surface_params.z;
    lit=lit/(vec3(1.0)+lit);
    gl_FragColor=vec4(pow(max(lit,vec3(0.0)),vec3(1.0/2.2)),emission_params.w>1.5?albedo.a:1.0);
}
