// Project-owned puzzle components, using exported data and the native C adapter.
// No player, FPS gameplay, door/portal system, renderer or prescribed schema.
#include "level.h"
#include "collision.h"
#include "raymath.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

typedef struct {int charge;bool armed;int sequence[3];int partner;} Puzzle;
static int failures;
static void check(bool ok,const char *message){if(!ok){fprintf(stderr,"FAIL %s\n",message);failures++;}}
static void configure(const HeronBrushLevel *level,Puzzle *puzzles){
    for(int i=0;i<level->entityCount;i++){
        const HeronBrushEntity *entity=&level->entities[i];
        const char *factory=HeronBrushGetProp(entity,"raylib_factory");if(!factory)continue;
        Puzzle *puzzle=&puzzles[i];puzzle->partner=-1;
        if(strcmp(factory,"panel")==0){
            puzzle->armed=strcmp(HeronBrushGetProp(entity,"armed"),"1")==0;
            check(sscanf(HeronBrushGetProp(entity,"sequence"),"%d %d %d",&puzzle->sequence[0],&puzzle->sequence[1],&puzzle->sequence[2])==3,"typed array");
            check(entity->targetCount==1,"resolved target count");
            const HeronBrushEntity *partner=HeronBrushFindEntity(level,entity->targets[0]);
            check(partner!=NULL,"forward target");if(partner)puzzle->partner=(int)(partner-level->entities);
        }else if(strcmp(factory,"beacon")==0)puzzle->charge=(int)HeronBrushPropFloat(entity,"charge",-1);
    }
}
int main(int argc,char **argv){
    if(argc!=2)return 2;
    static HeronBrushLevel first,second;static Puzzle puzzles[HERON_BRUSH_MAX_ENTITIES],other[HERON_BRUSH_MAX_ENTITIES];
    if(!HeronBrushLoadLevel(argv[1],&first)||!HeronBrushLoadLevel(argv[1],&second))return 2;
    configure(&first,puzzles);configure(&second,other);
    int panel=-1;
    for(int i=0;i<first.entityCount;i++)if(puzzles[i].armed)panel=i;
    check(panel>=0,"custom component factory");if(panel<0){HeronBrushFreeLevel(&first);HeronBrushFreeLevel(&second);return 1;}
    Puzzle *puzzle=&puzzles[panel];
    check(puzzle->sequence[0]==2&&puzzle->sequence[1]==3&&puzzle->sequence[2]==5,"native array values");
    check(puzzle->partner>=0&&puzzles[puzzle->partner].charge==7,"authored integer override");
    puzzles[puzzle->partner].charge++;
    check(puzzles[puzzle->partner].charge==8&&other[puzzle->partner].charge==7,"own action and isolated second map");
    HeronBrushCollider collider;HeronBrushColliderBuild(&collider,&first);
    check(fabsf(HeronBrushColliderRayHit(&collider,(Vector3){0,5,0},(Vector3){0,-1,0},10)-3)<.001f,"native convex brush hit");
    first.entities[panel].pos[0]+=3;
    HeronBrushColliderSetEntityTransform(&collider,panel,HeronBrushEntityMatrix(&first.entities[panel]));
    check(fabsf(HeronBrushColliderRayHit(&collider,(Vector3){0,5,0},(Vector3){0,-1,0},10)-5)<.001f,"old custom collider pose cleared");
    check(fabsf(HeronBrushColliderRayHit(&collider,(Vector3){3,5,0},(Vector3){0,-1,0},10)-3)<.001f,"project-owned collider movement");
    Vector3 centre={-3,4,3},velocity={0};bool grounded=false;
    for(int i=0;i<240;i++){
        velocity.y-=9.81f/60;centre=Vector3Add(centre,Vector3Scale(velocity,1.f/60));
        HeronBrushColliderResolveCapsule(&collider,&centre,.3f,.3f,&velocity,&grounded);
    }
    check(grounded&&fabsf(centre.y-.3f)<.02f,"project-owned falling sphere/floor collision");
    HeronBrushColliderFree(&collider);HeronBrushFreeLevel(&first);HeronBrushFreeLevel(&second);
    if(!failures)puts("HERON_RAYLIB_CUSTOM_GAME_OK: own components, typed properties, references, separate maps, moving convex collider and falling sphere");
    return failures?1:0;
}
