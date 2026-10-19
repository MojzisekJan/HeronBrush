// Read the exported file through the same C parser used at runtime.
#include "level.h"
#include <assert.h>
#include <stdio.h>
static HeronBrushLevel level;
int main(void) {
 assert(HeronBrushLoadLevel("assets/maps/demo.level", &level));
 int hulls=0, vertices=0, targets=0, properties=0;
 for(int i=0;i<level.entityCount;i++) {
  HeronBrushEntity *e=&level.entities[i];
  targets+=e->targetCount;properties+=e->propCount;
  for(int h=0;h<e->collision.hullCount;h++) {
   hulls++;vertices+=e->collision.hulls[h].vertexCount;
   assert(e->collision.hulls[h].vertexCount>=4);
  }
 }
 assert(level.entityCount==48 && hulls==19 && targets==8 && properties==362);
 printf("C reader PASS: entities=%d hulls=%d vertices=%d targets=%d properties=%d\n",
  level.entityCount,hulls,vertices,targets,properties);
 HeronBrushFreeLevel(&level);
 return 0;
}
