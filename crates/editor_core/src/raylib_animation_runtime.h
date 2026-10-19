/* Generated animation runtime: C99, no JSON parser or raylib dependency. */
#include <math.h>
#include <string.h>
typedef struct { double time,value,incoming,outgoing; int interpolation; } HbKey;
typedef struct { const HbKey *keys; int count; } HbChannel;
typedef struct { const char *name; double duration; int mode; HbChannel channels[6]; } HbClip;
typedef struct { const char *name; int entity,first,count,selected,autoplay; double pivot[3],scale; } HbAnimation;
typedef struct { int clip,playing,posed; double time; } HbPlayer;
