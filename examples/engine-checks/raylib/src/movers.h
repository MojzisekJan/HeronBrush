// The level's moving parts: doors and lifts, opened by the triggers that
// `target` them (and doors also by the player walking up to them).
//
// heron_brush exported the wiring. Each `trigger_*` volume carries the resolved names
// of what it fires (`target` lines), each `func_door` / `fps_lift` carries the
// distance, direction, speed and wait the mapper set. This turns that into
// motion: slide the mesh and its exact collision hulls, carry a rider on a lift.

#ifndef HERON_BRUSH_MOVERS_H
#define HERON_BRUSH_MOVERS_H

#include "collision.h"
#include "level.h"
#include "raylib.h"

typedef struct {
    int entIndex;   // into level->entities
    Matrix rest;    // the entity's exported transform (scale, YXZ rotation, position)
    Vector3 restMin, restMax;  // world bounds of its hulls when shut
    Vector3 openDir;           // unit, engine space
    float openDist;            // metres from shut to open
    float rate;                // open fraction per second
    float wait;                // seconds to stay open after last trigger
    bool isLift;               // lifts move only on a trigger, doors also on touch
    char requiredKey[32];      // fps_key_door: the key that unlocks it, or ""

    float amount;   // 0 shut .. 1 open
    float hold;     // seconds left before it starts closing
    Vector3 shift;  // world delta applied this step, to carry a rider
} HeronBrushMover;

typedef struct {
    HeronBrushMover *items;
    int count;
} HeronBrushMovers;

// Finds the movers and fills the list. Call right after `HeronBrushColliderBuild`,
// which already holds each mover's hulls at the shut position.
void HeronBrushMoversBuild(HeronBrushMovers *m, HeronBrushCollider *col, const HeronBrushLevel *level);
void HeronBrushMoversFree(HeronBrushMovers *m);

// One step: open movers whose trigger `playerCentre` is inside (or, for
// doors, that `playerCentre` is close to), close the rest after their wait,
// move them, and move their collision hulls with them. Returns the world delta of
// the mover the player is standing on, so the caller can carry them up a lift.
Vector3 HeronBrushMoversUpdate(HeronBrushMovers *m, HeronBrushCollider *col, const HeronBrushLevel *level,
                        Vector3 playerCentre, Vector3 playerFeet, float radius,
                        const char *const *heldKeys, int heldCount, float dt);

// The world offset of entity `entIndex` from its exported transform, {0,0,0}
// for anything that is not a mover. Draw it with HeronBrushEntityMatrix(e) followed
// by a translation by this.
Vector3 HeronBrushMoverDrawOffset(const HeronBrushMovers *m, int entIndex);

#endif
