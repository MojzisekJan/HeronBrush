#include "animated.h"
#include "player.h"

#include "raymath.h"

#include <math.h>
#include <string.h>

#ifndef HERON_BRUSH_ANIMATION_HEADER
#define HERON_BRUSH_ANIMATION_HEADER "../assets/maps/demo_animation.h"
#endif
#include HERON_BRUSH_ANIMATION_HEADER

// One more than there are, so a map without animations still has arrays.
#define HERON_BRUSH_ANIMATED_SLOTS (HB_ANIMATION_COUNT + 1)

static HbPlayer gPlayers[HERON_BRUSH_ANIMATED_SLOTS];
static HbTriggerState gTriggers[HB_TRIGGER_COUNT + 1];
static Matrix gRest[HERON_BRUSH_ANIMATED_SLOTS];     // each entity's exported transform
static Matrix gCurrent[HERON_BRUSH_ANIMATED_SLOTS];  // rest, then the animation, as last placed

// Where animation `a` puts its entity now: the player transforms world points of the
// entity at rest, and the composition is affine, so the images of the origin and the
// three unit points give the whole matrix.
static Matrix Posed(int a) {
    const Vector3 basis[4] = {{0, 0, 0}, {1, 0, 0}, {0, 1, 0}, {0, 0, 1}};
    Vector3 w[4];
    for (int i = 0; i < 4; i++) {
        Vector3 r = Vector3Transform(basis[i], gRest[a]);
        double in[3] = {r.x, r.y, r.z}, out[3];
        HbPoint(a, &gPlayers[a], in, out, 0);
        w[i] = (Vector3){(float)out[0], (float)out[1], (float)out[2]};
    }
    // raylib's Vector3Transform: x' = m0 x + m4 y + m8 z + m12, and so on.
    Matrix m = MatrixIdentity();
    m.m0 = w[1].x - w[0].x; m.m1 = w[1].y - w[0].y; m.m2 = w[1].z - w[0].z;
    m.m4 = w[2].x - w[0].x; m.m5 = w[2].y - w[0].y; m.m6 = w[2].z - w[0].z;
    m.m8 = w[3].x - w[0].x; m.m9 = w[3].y - w[0].y; m.m10 = w[3].z - w[0].z;
    m.m12 = w[0].x; m.m13 = w[0].y; m.m14 = w[0].z;
    return m;
}

bool HeronBrushAnimatedBuild(HeronBrushCollider *col, const HeronBrushLevel *level) {
    memset(gTriggers, 0, sizeof gTriggers);
    for (int a = 0; a < HB_ANIMATION_COUNT; a++) {
        int e = hb_animations[a].entity;
        if (e < 0 || e >= level->entityCount) return false;
        gRest[a] = HeronBrushEntityMatrix(&level->entities[e]);
        HbInit(&gPlayers[a], a);
        gCurrent[a] = Posed(a);
        HeronBrushColliderSetEntityTransform(col, e, gCurrent[a]);
    }
    return true;
}

bool HeronBrushAnimatedHas(int entIndex) {
    for (int a = 0; a < HB_ANIMATION_COUNT; a++)
        if (hb_animations[a].entity == entIndex) return true;
    return false;
}

// The animated entity whose top `feet` stands on, or -1: the highest one whose hulls'
// bounds hold the point across and whose top is within a step of it.
static int Under(const HeronBrushCollider *col, Vector3 feet) {
    int found = -1;
    float best = -INFINITY;
    for (int a = 0; a < HB_ANIMATION_COUNT; a++) {
        Vector3 mn, mx;
        if (!HeronBrushColliderEntityBounds(col, hb_animations[a].entity, &mn, &mx)) continue;
        bool across = feet.x >= mn.x - 0.05f && feet.x <= mx.x + 0.05f && feet.z >= mn.z - 0.05f &&
                      feet.z <= mx.z + 0.05f;
        bool onTop = feet.y >= mx.y - 0.25f && feet.y <= mx.y + 0.25f;
        if (across && onTop && mx.y > best) {
            found = a;
            best = mx.y;
        }
    }
    return found;
}

Vector3 HeronBrushAnimatedUpdate(HeronBrushCollider *col, Vector3 feet, bool grounded, float dt, double now) {
    if (HB_ANIMATION_COUNT == 0) return (Vector3){0, 0, 0};
    int under = grounded ? Under(col, feet) : -1;
    Matrix before = under >= 0 ? gCurrent[under] : MatrixIdentity();

    // From just above the feet to the top of the capsule: pads on the floor and
    // volumes that start higher up both catch the player.
    double point[3] = {feet.x, feet.y + 0.05, feet.z};
    HbCheckTriggersSpan(gTriggers, gPlayers, point, 2 * HERON_BRUSH_PLAYER_HALF - 0.05, now);
    for (int a = 0; a < HB_ANIMATION_COUNT; a++) {
        HbUpdate(&gPlayers[a], dt);
        gCurrent[a] = Posed(a);
        HeronBrushColliderSetEntityTransform(col, hb_animations[a].entity, gCurrent[a]);
    }
    if (under < 0) return (Vector3){0, 0, 0};
    // The point the feet stand on, carried with the entity: back into its frame as it
    // was, then out with the new pose, so a turning platform turns its rider too.
    Vector3 local = Vector3Transform(feet, MatrixInvert(before));
    return Vector3Subtract(Vector3Transform(local, gCurrent[under]), feet);
}

bool HeronBrushAnimatedMatrix(int entIndex, Matrix *out) {
    for (int a = 0; a < HB_ANIMATION_COUNT; a++) {
        if (hb_animations[a].entity == entIndex) {
            *out = gCurrent[a];
            return true;
        }
    }
    return false;
}

int HeronBrushAnimatedCount(void) { return HB_ANIMATION_COUNT; }

int HeronBrushAnimatedEntity(int animation) {
    return animation >= 0 && animation < HB_ANIMATION_COUNT ? hb_animations[animation].entity : -1;
}

const char *HeronBrushAnimatedName(int animation) {
    return animation >= 0 && animation < HB_ANIMATION_COUNT ? hb_animations[animation].name : "";
}

void HeronBrushAnimatedPlay(int animation) {
    if (animation >= 0 && animation < HB_ANIMATION_COUNT) HbPlay(&gPlayers[animation], animation, 0);
}

void HeronBrushAnimatedStop(int animation) {
    if (animation >= 0 && animation < HB_ANIMATION_COUNT) HbStop(&gPlayers[animation]);
}

bool HeronBrushAnimatedPlaying(int animation) {
    return animation >= 0 && animation < HB_ANIMATION_COUNT && gPlayers[animation].playing;
}
