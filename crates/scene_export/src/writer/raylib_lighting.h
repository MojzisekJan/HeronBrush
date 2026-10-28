/* Optional raylib 5/6 forward lighting. Include after raylib.h and raymath.h.
 * Authored point/spot/sun lights, diffuse/specular response and normal textures.
 * This is a baseline shader, without shadow maps or full glTF PBR extensions. */
#ifndef HERON_RAYLIB_LIGHTING_H
#define HERON_RAYLIB_LIGHTING_H
#include <stdio.h>
typedef struct { int kind; float position[3],direction[3],color[3],energy,range,cutoff; } HbRenderLight;
__LIGHT_DATA__
static const char *hb_light_vs =
"#version 330\nin vec3 vertexPosition;in vec2 vertexTexCoord;in vec3 vertexNormal;in vec4 vertexColor;\n"
"uniform mat4 mvp,matModel,matNormal;out vec3 worldPosition,worldNormal;out vec2 uv;out vec4 tint;\n"
"void main(){worldPosition=(matModel*vec4(vertexPosition,1)).xyz;worldNormal=normalize((matNormal*vec4(vertexNormal,0)).xyz);uv=vertexTexCoord;tint=vertexColor;gl_Position=mvp*vec4(vertexPosition,1);}\n";
static const char *hb_light_fs =
"#version 330\nin vec3 worldPosition,worldNormal;in vec2 uv;in vec4 tint;out vec4 finalColor;\n"
"uniform sampler2D texture0,texture2;uniform vec4 colDiffuse;uniform vec3 cameraPosition;uniform int hasNormal;uniform float roughness,metallic;\n"
"struct Light{int kind;vec3 position,direction,color;float energy,range,cutoff;};\n"
"uniform Light lights[__LIGHT_CAPACITY__];uniform int lightCount;\n"
"void main(){vec4 base=texture(texture0,uv)*colDiffuse*tint;vec3 albedo=pow(max(base.rgb,vec3(0)),vec3(2.2));\n"
"vec3 n=normalize(worldNormal);if(hasNormal!=0){vec3 q1=dFdx(worldPosition),q2=dFdy(worldPosition);vec2 st1=dFdx(uv),st2=dFdy(uv);vec3 t=q1*st2.y-q2*st1.y;float len=length(t);if(len>1e-8){t=normalize(t-n*dot(n,t));vec3 b=normalize(cross(n,t))*sign(st1.x*st2.y-st1.y*st2.x);n=normalize(mat3(t,b,n)*(texture(texture2,uv).xyz*2-1));}}\n"
"vec3 v=normalize(cameraPosition-worldPosition),color=albedo*0.2;float gloss=max(2.0,2.0/pow(max(roughness,0.08),4)-2.0);\n"
"for(int i=0;i<lightCount;i++){Light l=lights[i];vec3 delta=l.position-worldPosition;float distance=length(delta);vec3 d=l.kind==2?-l.direction:delta/max(distance,0.0001);float attenuation=1.0;\n"
"if(l.kind!=2){float edge=max(0.0,1.0-pow(distance/max(l.range,0.001),4));attenuation=edge*edge/(1.0+distance*distance);if(l.kind==1)attenuation*=smoothstep(l.cutoff,min(1.0,l.cutoff+0.08),dot(-d,l.direction));}\n"
"float diffuse=max(dot(n,d),0.0);vec3 h=normalize(d+v),specular=mix(vec3(0.04),albedo,clamp(metallic,0,1));float highlight=pow(max(dot(n,h),0.0),min(gloss,1000.0));\n"
"color+=(albedo*(1-clamp(metallic,0,1))*diffuse+specular*highlight)*pow(max(l.color,vec3(0)),vec3(2.2))*l.energy*attenuation;}\n"
"finalColor=vec4(pow(max(color,vec3(0)),vec3(1.0/2.2)),base.a);}\n";
/* raylib's glTF reader loads images without the glTF mip filtering policy. */
static inline void HbLightingPrepare(Model *model) {
    for(int m=0;m<model->materialCount;m++)for(int i=0;i<=MATERIAL_MAP_EMISSION;i++){
        Texture2D *texture=&model->materials[m].maps[i].texture;
        if(texture->id&&texture->width>1&&texture->height>1){GenTextureMipmaps(texture);SetTextureFilter(*texture,TEXTURE_FILTER_TRILINEAR);}
    }
}
static inline Shader HbLightingLoad(void) {
    Shader s=LoadShaderFromMemory(hb_light_vs,hb_light_fs);int count=HB_RENDER_LIGHT_COUNT;
    SetShaderValue(s,GetShaderLocation(s,"lightCount"),&count,SHADER_UNIFORM_INT);
    for(int i=0;i<count;i++){
        char name[80];const HbRenderLight *l=&hb_render_lights[i];
#define HB_LIGHT_FIELD(field,type) snprintf(name,sizeof(name),"lights[%d].%s",i,#field);SetShaderValue(s,GetShaderLocation(s,name),&l->field,type)
        HB_LIGHT_FIELD(kind,SHADER_UNIFORM_INT);HB_LIGHT_FIELD(position,SHADER_UNIFORM_VEC3);HB_LIGHT_FIELD(direction,SHADER_UNIFORM_VEC3);HB_LIGHT_FIELD(color,SHADER_UNIFORM_VEC3);HB_LIGHT_FIELD(energy,SHADER_UNIFORM_FLOAT);HB_LIGHT_FIELD(range,SHADER_UNIFORM_FLOAT);HB_LIGHT_FIELD(cutoff,SHADER_UNIFORM_FLOAT);
#undef HB_LIGHT_FIELD
    }
    return s;
}
static inline void HbLightingDraw(Shader shader,Model model,Vector3 camera) {
    SetShaderValue(shader,GetShaderLocation(shader,"cameraPosition"),&camera,SHADER_UNIFORM_VEC3);
    for(int i=0;i<model.meshCount;i++){
        Material material=model.materials[model.meshMaterial[i]];material.shader=shader;
        int normal=material.maps[MATERIAL_MAP_NORMAL].texture.id!=0;
        SetShaderValue(shader,GetShaderLocation(shader,"hasNormal"),&normal,SHADER_UNIFORM_INT);
        SetShaderValue(shader,GetShaderLocation(shader,"roughness"),&material.maps[MATERIAL_MAP_ROUGHNESS].value,SHADER_UNIFORM_FLOAT);
        SetShaderValue(shader,GetShaderLocation(shader,"metallic"),&material.maps[MATERIAL_MAP_METALNESS].value,SHADER_UNIFORM_FLOAT);
        DrawMesh(model.meshes[i],material,model.transform);
    }
}
#endif
