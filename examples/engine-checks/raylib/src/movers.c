#include "movers.h"

#include "raymath.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Quake axes to engine axes, the same swap heron_brush applies to geometry:
// (qx, qy, qz) -> (qy, qz, qx).
static Vector3 quake_to_engine(float qx, float qy, float qz) {
    return (Vector3){qy, qz, qx};
}

void HeronBrushMoversBuild(HeronBrushMovers *m, HeronBrushCollider *col, const HeronBrushLevel *level) {
    int n = 0;
    for (int i = 0; i < level->entityCount; i++)
        if (HeronBrushIsMoverClass(level->entities[i].classname)) n++;

    m->items = n ? calloc((size_t)n, sizeof(HeronBrushMover)) : NULL;
    m->count = 0;
    if (!m->items) return;

    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        if (!HeronBrushIsMoverClass(e->classname)) continue;

        HeronBrushMover *mv = &m->items[m->count++];
        mv->entIndex = i;
        mv->rest = HeronBrushEntityMatrix(e);
        if (!HeronBrushColliderEntityBounds(col, i, &mv->restMin, &mv->restMax) &&
            !HeronBrushEntityWorldAABB(e, &mv->restMin, &mv->restMax))
            mv->restMin = mv->restMax = (Vector3){e->pos[0], e->pos[1], e->pos[2]};

        float travelUnits = HeronBrushPropFloat(e, "travel", 64.0f);
        float speedUnits = HeronBrushPropFloat(e, "speed", 100.0f);
        mv->openDist = travelUnits / (level->scale > 0 ? level->scale : 32.0f);
        float speed = speedUnits / (level->scale > 0 ? level->scale : 32.0f);
        mv->rate = mv->openDist > 0.01f ? speed / mv->openDist : 1.0f;
        mv->wait = HeronBrushPropFloat(e, "wait", 3.0f);
        mv->isLift = strcmp(e->classname, "fps_lift") == 0;
        mv->requiredKey[0] = '\0';
        if (strcmp(e->classname, "fps_key_door") == 0) {
            const char *k = HeronBrushGetProp(e, "key");
            if (k) {
                strncpy(mv->requiredKey, k, sizeof(mv->requiredKey) - 1);
                mv->requiredKey[sizeof(mv->requiredKey) - 1] = '\0';
            }
        }

        Vector3 dir = {1, 0, 0};
        const char *od = HeronBrushGetProp(e, "open_direction");
        if (od) {
            float x = 0, y = 0, z = 0;
            if (sscanf(od, "%f %f %f", &x, &y, &z) == 3)
                dir = quake_to_engine(x, y, z);
        }
        float dl = Vector3Length(dir);
        mv->openDir = dl > 1e-4f ? Vector3Scale(dir, 1.0f / dl) : (Vector3){1, 0, 0};
    }
}

void HeronBrushMoversFree(HeronBrushMovers *m) {
    free(m->items);
    m->items = NULL;
    m->count = 0;
}

static bool point_in_box(Vector3 p, Vector3 mn, Vector3 mx, float pad) {
    return p.x >= mn.x - pad && p.x <= mx.x + pad && p.y >= mn.y - pad &&
           p.y <= mx.y + pad && p.z >= mn.z - pad && p.z <= mx.z + pad;
}

// Does the player set off this mover? Its trigger overlaps the player, or it is
// a door and the player is close to it.
static bool activated(const HeronBrushMover *mv, const HeronBrushLevel *level,
                      Vector3 playerCentre, float radius) {
    const char *want = level->entities[mv->entIndex].name;

    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *t = &level->entities[i];
        if (!t->collision.sensor) continue;
        bool targetsUs = false;
        for (int k = 0; k < t->targetCount; k++)
            if (strcmp(t->targets[k], want) == 0) targetsUs = true;
        if (!targetsUs) continue;

        Vector3 mn, mx;
        if (HeronBrushEntityWorldAABB(t, &mn, &mx) &&
            point_in_box(playerCentre, mn, mx, radius))
            return true;
    }

    if (!mv->isLift && point_in_box(playerCentre, mv->restMin, mv->restMax, 1.3f)) return true;
    return false;
}

static bool has_key(const char *want, const char *const *held, int n) {
    if (!want || want[0] == '\0') return true;
    for (int i = 0; i < n; i++)
        if (held[i] && strcmp(held[i], want) == 0) return true;
    return false;
}

Vector3 HeronBrushMoversUpdate(HeronBrushMovers *m, HeronBrushCollider *col, const HeronBrushLevel *level,
                        Vector3 playerCentre, Vector3 playerFeet, float radius,
                        const char *const *heldKeys, int heldCount, float dt) {
    Vector3 ride = {0, 0, 0};

    for (int i = 0; i < m->count; i++) {
        HeronBrushMover *mv = &m->items[i];

        bool locked = !has_key(mv->requiredKey, heldKeys, heldCount);
        bool active = !locked && activated(mv, level, playerCentre, radius);
        if (active) mv->hold = mv->wait;
        else if (mv->hold > 0.0f) mv->hold -= dt;

        float target = active || mv->hold > 0.0f ? 1.0f : 0.0f;
        float prev = mv->amount;
        float delta = mv->rate * dt;
        if (mv->amount < target) mv->amount = fminf(target, mv->amount + delta);
        else if (mv->amount > target) mv->amount = fmaxf(target, mv->amount - delta);

        Vector3 off = Vector3Scale(mv->openDir, mv->amount * mv->openDist);
        HeronBrushColliderSetEntityTransform(col, mv->entIndex,
                                      MatrixMultiply(mv->rest, MatrixTranslate(off.x, off.y, off.z)));
        Vector3 mn = Vector3Add(mv->restMin, off);
        Vector3 mx = Vector3Add(mv->restMax, off);

        mv->shift = Vector3Scale(mv->openDir, (mv->amount - prev) * mv->openDist);

        // Carrying a rider: feet just above the top face, centred over it.
        float topY = mx.y;
        bool over = playerCentre.x >= mn.x - radius && playerCentre.x <= mx.x + radius &&
                    playerCentre.z >= mn.z - radius && playerCentre.z <= mx.z + radius;
        if (over && playerFeet.y >= topY - 0.06f && playerFeet.y <= topY + 0.45f)
            ride = Vector3Add(ride, mv->shift);
    }
    return ride;
}

Vector3 HeronBrushMoverDrawOffset(const HeronBrushMovers *m, int entIndex) {
    for (int i = 0; i < m->count; i++) {
        if (m->items[i].entIndex == entIndex) {
            const HeronBrushMover *mv = &m->items[i];
            return Vector3Scale(mv->openDir, mv->amount * mv->openDist);
        }
    }
    return (Vector3){0, 0, 0};
}
