/* Run from the exported directory. Free camera, no game-specific physics. */
#ifdef HB_SKY
#include "rlgl.h"
/* The map's sky, baked as the editor's view shows it, on a cube round the camera. */
static const char *hb_sky_vs="#version 330\nin vec3 vertexPosition;uniform mat4 matProjection;uniform mat4 matView;out vec3 direction;\n"
    "void main(){direction=vertexPosition;gl_Position=matProjection*mat4(mat3(matView))*vec4(vertexPosition,1.0);}\n";
static const char *hb_sky_fs="#version 330\nin vec3 direction;uniform samplerCube environmentMap;out vec4 finalColor;\n"
    "void main(){finalColor=vec4(texture(environmentMap,direction).rgb,1.0);}\n";
#endif
static void RestPoint(int entity,const double p[3],double out[3],int normal) {
    const PreviewEntity *e=&entities[entity];double v[3]={p[0]*e->scale[0],p[1]*e->scale[1],p[2]*e->scale[2]};
    const int order[3]={2,0,1};
    for(int n=0;n<3;n++){int a=order[n],j=(a+1)%3,k=(a+2)%3;double s=sin(e->rot[a]),c=cos(e->rot[a]),x=v[j],y=v[k];v[j]=c*x-s*y;v[k]=s*x+c*y;}
    for(int i=0;i<3;i++)out[i]=v[i]+(normal?0:e->pos[i]);
}
static Matrix At(int entity,int animation,const HbPlayer *player) {
    double m[16]={0};m[15]=1;
    for(int i=0;i<4;i++){double v[3]={0},rest[3],p[3];if(i<3)v[i]=1;RestPoint(entity,v,rest,i<3);HbPoint(animation,player,rest,p,i<3);for(int j=0;j<3;j++)m[i*4+j]=p[j];}
    return (Matrix){(float)m[0],(float)m[4],(float)m[8],(float)m[12],(float)m[1],(float)m[5],(float)m[9],(float)m[13],(float)m[2],(float)m[6],(float)m[10],(float)m[14],(float)m[3],(float)m[7],(float)m[11],(float)m[15]};
}
int main(int argc,char **argv) {
    SetConfigFlags(FLAG_WINDOW_RESIZABLE);
    InitWindow(1280,720,"Heron Brush - raylib animation preview");SetTargetFPS(60);
#ifdef HB_SKY
    Model sky=LoadModelFromMesh(GenMeshCube(1,1,1));
    sky.materials[0].shader=LoadShaderFromMemory(hb_sky_vs,hb_sky_fs);
    int cubemap=MATERIAL_MAP_CUBEMAP;SetShaderValue(sky.materials[0].shader,GetShaderLocation(sky.materials[0].shader,"environmentMap"),&cubemap,SHADER_UNIFORM_INT);
    Image strip=LoadImage(HB_SKY);sky.materials[0].maps[MATERIAL_MAP_CUBEMAP].texture=LoadTextureCubemap(strip,CUBEMAP_LAYOUT_LINE_VERTICAL);UnloadImage(strip);
#endif
#ifdef HB_LIGHTING
    Shader lighting=HbLightingLoad();
#endif
    Model models[ENTITY_COUNT+1]={0};int animation[ENTITY_COUNT+1];
    HbPlayer players[HB_ANIMATION_COUNT+1];
    for(int i=0;i<HB_ANIMATION_COUNT;i++)HbInit(&players[i],i);
    Vector3 lo={1e20f,1e20f,1e20f},hi={-1e20f,-1e20f,-1e20f};int loaded=0;
    for(int i=0;i<ENTITY_COUNT;i++){
        animation[i]=-1;for(int a=0;a<HB_ANIMATION_COUNT;a++)if(hb_animations[a].entity==i)animation[i]=a;
        if(!entities[i].mesh[0])continue;
        models[i]=LoadModel(entities[i].mesh);if(models[i].meshCount<=0)continue;loaded++;
#ifdef HB_LIGHTING
        HbLightingPrepare(&models[i]);
#endif
        BoundingBox b=GetModelBoundingBox(models[i]);
        for(int n=0;n<8;n++){double p[3]={n&1?b.max.x:b.min.x,n&2?b.max.y:b.min.y,n&4?b.max.z:b.min.z},r[3];RestPoint(i,p,r,0);
            lo.x=fminf(lo.x,r[0]);lo.y=fminf(lo.y,r[1]);lo.z=fminf(lo.z,r[2]);hi.x=fmaxf(hi.x,r[0]);hi.y=fmaxf(hi.y,r[1]);hi.z=fmaxf(hi.z,r[2]);
            int a=animation[i];if(a>=0)for(int c=hb_animations[a].first;c<hb_animations[a].first+hb_animations[a].count;c++)for(int step=0;step<=32;step++){
                HbPlayer sample={c,0,1,hb_clips[c].duration*step/32.0};double q[3];HbPoint(a,&sample,r,q,0);
                lo.x=fminf(lo.x,q[0]);lo.y=fminf(lo.y,q[1]);lo.z=fminf(lo.z,q[2]);hi.x=fmaxf(hi.x,q[0]);hi.y=fmaxf(hi.y,q[1]);hi.z=fmaxf(hi.z,q[2]);
            }
        }
    }
    if(!loaded){lo=(Vector3){-5,-5,-5};hi=(Vector3){5,5,5};}
    Vector3 center={(lo.x+hi.x)/2,(lo.y+hi.y)/2,(lo.z+hi.z)/2};
    float span=fmaxf(5,fmaxf(hi.x-lo.x,fmaxf(hi.y-lo.y,hi.z-lo.z)));
    Camera3D camera={{center.x+span,center.y+span*.7f,center.z+span},center,{0,1,0},60,CAMERA_PERSPECTIVE};
#ifdef HB_WATER
    HbWaterShader water=HbWaterLoadShader();
#endif
    HbTriggerState triggers[HB_TRIGGER_COUNT+1]={{0}};
    int paused=0,verifyFrame=0;
    while(!WindowShouldClose()){
        if(IsMouseButtonDown(MOUSE_BUTTON_RIGHT))UpdateCamera(&camera,CAMERA_FREE);
        if(IsKeyPressed(KEY_SPACE))paused=!paused;
        for(int a=0;a<HB_ANIMATION_COUNT;a++){
            if(IsKeyPressed(KEY_R))HbPlay(&players[a],a,0);
            if(IsKeyPressed(KEY_N)){int next=hb_animations[a].first+(players[a].clip-hb_animations[a].first+1)%hb_animations[a].count;HbPlay(&players[a],a,hb_clips[next].name);}
            if(!paused)HbUpdate(&players[a],GetFrameTime());
        }
        double actor[3]={camera.position.x,camera.position.y,camera.position.z};
        if(!paused)HbCheckTriggers(triggers,players,actor,GetTime());
        int underwater=-1;
#ifdef HB_WATER
        underwater=HbWaterView(&water,camera.position,(float)GetTime());
        BeginDrawing();ClearBackground(underwater>=0?water.fog:(Color){28,30,36,255});BeginMode3D(camera);
#else
        BeginDrawing();ClearBackground((Color){28,30,36,255});BeginMode3D(camera);
#endif
#ifdef HB_SKY
        /* Under water the fog hides the sky. */
        if(underwater<0){rlDisableBackfaceCulling();rlDisableDepthMask();DrawModel(sky,camera.position,1,WHITE);rlEnableBackfaceCulling();rlEnableDepthMask();}
#endif
        /* Water surfaces are see-through, so they go after everything else. */
        for(int pass=0;pass<2;pass++)for(int i=0;i<ENTITY_COUNT;i++)if(models[i].meshCount>0){
            int a=animation[i];HbPlayer rest={0};models[i].transform=At(i,a,a<0?&rest:&players[a]);
#ifdef HB_WATER
            int w=HbWaterOf(i);if((w>=0&&hb_waters[w].surface_material)!=(pass==1))continue;
#ifdef HB_LIGHTING
            if(underwater<0&&!(w>=0&&hb_waters[w].surface_material))HbLightingDraw(lighting,models[i],camera.position);
            else
#endif
            HbWaterDrawModel(&water,models[i],w);
#else
            if(pass==0){
#ifdef HB_LIGHTING
                HbLightingDraw(lighting,models[i],camera.position);
#else
                DrawModel(models[i],(Vector3){0,0,0},1,WHITE);
#endif
            }
#endif
        }
        EndMode3D();DrawText("RMB + mouse/WASD camera | SPACE pause | R play/restart | N next named clips",16,16,20,RAYWHITE);
        int y=44;for(int a=0;a<HB_ANIMATION_COUNT&&y<GetScreenHeight()-24;a++,y+=22)DrawText(TextFormat("%s: %s  %.2fs",hb_animations[a].name,hb_clips[players[a].clip].name,players[a].time),16,y,18,ORANGE);
        EndDrawing();
        if(argc>1&&++verifyFrame==30){Image shot=LoadImageFromScreen();ExportImage(shot,argv[1]);UnloadImage(shot);break;}
    }
    for(int i=0;i<ENTITY_COUNT;i++){if(models[i].meshCount>0)UnloadModel(models[i]);}
#ifdef HB_LIGHTING
    UnloadShader(lighting);
#endif
    CloseWindow();return 0;
}
