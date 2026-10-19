/* Water volumes: each is an entity's convex brushes, as planes in the entity's own
 * frame, in metres. A point is in a brush when it is behind every plane. The
 * collision in the .level marks the volume `sensor`: nothing here moves a body. A
 * player asks HbWaterFind where it is every step and, while it is in water, takes
 * its velocity from HbWaterVelocity. Points are world positions in the level's
 * frame with each volume's entity at its rest pose. */
static inline double HbWaterSample(int water,const double world[3]){
    const HbWater *w=&hb_waters[water];const double *m=w->to_local;double depth=-1;
    double p[3];for(int i=0;i<3;i++)p[i]=m[i*4]*world[0]+m[i*4+1]*world[1]+m[i*4+2]*world[2]+m[i*4+3];
    for(int v=0;v<w->count;v++){
        const HbWaterVolume *volume=&w->volumes[v];int inside=volume->count>=4;
        for(int i=0;inside&&i<volume->count;i++){const double *q=volume->planes[i];if(q[0]*p[0]+q[1]*p[1]+q[2]*p[2]>q[3]+1e-5)inside=0;}
        if(inside){double d=(volume->surface-p[1])*w->scale_y;if(d>depth)depth=d;}
    }
    return depth;
}
/* The volume `world` is in, or -1; `depth` gets how far below its surface. Where
 * volumes overlap the higher priority wins, then the deeper. */
static inline int HbWaterFind(const double world[3],double *depth){
    int best=-1;double best_depth=-1;
    for(int i=0;i<HB_WATER_COUNT;i++){
        double d=HbWaterSample(i,world);if(d<0)continue;
        if(best<0||hb_waters[i].priority>hb_waters[best].priority||(hb_waters[i].priority==hb_waters[best].priority&&d>best_depth)){best=i;best_depth=d;}
    }
    if(depth)*depth=best_depth;
    return best;
}
/* Swimming: `walk` steers in the horizontal plane (its length is capped at 1),
 * `rise` and `dive` move up and down, and with neither the body sinks slowly. The
 * current velocity eases towards that at the volume's drag. */
static inline void HbWaterVelocity(int water,const double current[3],const double walk[3],int rise,int dive,double dt,double out[3]){
    const HbWater *w=&hb_waters[water];double length=sqrt(walk[0]*walk[0]+walk[1]*walk[1]+walk[2]*walk[2]);
    double k=(length>1?1/length:1)*w->swim_speed;
    double wish[3]={walk[0]*k,(rise?w->rise_speed:0)-(dive?w->rise_speed:0)-(!rise&&!dive?w->sink_speed:0),walk[2]*k};
    double f=1-exp(-w->drag*dt);
    for(int i=0;i<3;i++)out[i]=current[i]+(wish[i]-current[i])*f;
}
#if defined(RAYLIB_H)
#include "rlgl.h"
/* One shader for the whole level: textures and material colours as raylib draws
 * them, a small wave on a water surface's upward faces, and fog while the camera is
 * under water (squared exponential, as three.js's FogExp2). */
static const char *hb_water_vs="#version 330\n"
    "in vec3 vertexPosition;in vec2 vertexTexCoord;in vec3 vertexNormal;in vec4 vertexColor;\n"
    "uniform mat4 mvp;uniform mat4 matModel;uniform float time;uniform float waveHeight;uniform float waveSpeed;\n"
    "out vec2 uv;out vec4 tint;out vec3 world;\n"
    "void main(){vec3 p=vertexPosition;p.y+=smoothstep(0.5,0.9,vertexNormal.y)*waveHeight*sin(p.x*1.7+time*waveSpeed)*cos(p.z*1.3-time*waveSpeed*0.8);\n"
    "uv=vertexTexCoord;tint=vertexColor;world=(matModel*vec4(p,1.0)).xyz;gl_Position=mvp*vec4(p,1.0);}\n";
static const char *hb_water_fs="#version 330\n"
    "in vec2 uv;in vec4 tint;in vec3 world;uniform sampler2D texture0;uniform vec4 colDiffuse;\n"
    "uniform vec3 viewPos;uniform vec3 fogColor;uniform float fogDensity;out vec4 finalColor;\n"
    "void main(){vec4 c=texture(texture0,uv)*colDiffuse*tint;float d=length(world-viewPos)*fogDensity;\n"
    "finalColor=vec4(mix(fogColor,c.rgb,exp(-d*d)),c.a);}\n";
typedef struct {Shader shader;int time,height,speed,view,fog_color,fog_density;int underwater;Color fog;} HbWaterShader;
static inline HbWaterShader HbWaterLoadShader(void){
    HbWaterShader s={0};s.shader=LoadShaderFromMemory(hb_water_vs,hb_water_fs);
    s.time=GetShaderLocation(s.shader,"time");s.height=GetShaderLocation(s.shader,"waveHeight");s.speed=GetShaderLocation(s.shader,"waveSpeed");
    s.view=GetShaderLocation(s.shader,"viewPos");s.fog_color=GetShaderLocation(s.shader,"fogColor");s.fog_density=GetShaderLocation(s.shader,"fogDensity");
    s.shader.locs[SHADER_LOC_MATRIX_MODEL]=GetShaderLocation(s.shader,"matModel");
    s.underwater=-1;return s;
}
/* Once a frame, before drawing: the water the camera is in (-1 for none) gives the
 * fog, and `fog` its colour for clearing the background, since fog does not reach
 * a sky. */
static inline int HbWaterView(HbWaterShader *s,Vector3 camera,float time){
    double at[3]={camera.x,camera.y,camera.z};int water=HbWaterFind(at,0);
    float view[3]={camera.x,camera.y,camera.z},color[3]={0,0,0},density=0;
    if(water>=0){for(int i=0;i<3;i++)color[i]=(float)hb_waters[water].color[i];density=(float)hb_waters[water].fog_density;}
    SetShaderValue(s->shader,s->time,&time,SHADER_UNIFORM_FLOAT);SetShaderValue(s->shader,s->view,view,SHADER_UNIFORM_VEC3);
    SetShaderValue(s->shader,s->fog_color,color,SHADER_UNIFORM_VEC3);SetShaderValue(s->shader,s->fog_density,&density,SHADER_UNIFORM_FLOAT);
    s->fog=(Color){(unsigned char)(color[0]*255),(unsigned char)(color[1]*255),(unsigned char)(color[2]*255),255};
    s->underwater=water;return water;
}
/* The water volume drawn by the entity `entity`, or -1. */
static inline int HbWaterOf(int entity){for(int i=0;i<HB_WATER_COUNT;i++)if(hb_waters[i].entity==entity)return i;return -1;}
/* Draws a model through the shader. A water volume's baseline surface
 * (surface_material) waves and is see-through from both sides; draw those after
 * everything opaque, a volume keeping its own material with the rest. */
static inline void HbWaterDrawModel(HbWaterShader *s,Model model,int water){
    for(int i=0;i<model.materialCount;i++)model.materials[i].shader=s->shader;
    int surface=water>=0&&hb_waters[water].surface_material;
    float height=surface?(float)hb_waters[water].wave_height:0,speed=surface?(float)hb_waters[water].wave_speed:0;
    SetShaderValue(s->shader,s->height,&height,SHADER_UNIFORM_FLOAT);SetShaderValue(s->shader,s->speed,&speed,SHADER_UNIFORM_FLOAT);
    if(surface){rlDrawRenderBatchActive();rlDisableBackfaceCulling();rlDisableDepthMask();BeginBlendMode(BLEND_ALPHA);}
    DrawModel(model,(Vector3){0,0,0},1,WHITE);
    if(surface){EndBlendMode();rlDrawRenderBatchActive();rlEnableDepthMask();rlEnableBackfaceCulling();}
}
#endif
