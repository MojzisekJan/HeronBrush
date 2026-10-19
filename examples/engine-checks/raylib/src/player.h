// A small first-person controller: capsule against the level's collision
// soup, gravity, jump, and a step-up so stairs and kerbs are walkable.
//
// Input comes in as a struct rather than being read here, so the same update
// runs from the keyboard or from a scripted walk (--demo).

#ifndef HERON_BRUSH_PLAYER_H
#define HERON_BRUSH_PLAYER_H

#include "collision.h"
#include "raylib.h"

#include <stdbool.h>

// Capsule and movement, all in metres and metres/second.
#define HERON_BRUSH_PLAYER_RADIUS 0.35f
#define HERON_BRUSH_PLAYER_HALF   0.90f // half the capsule height
#define HERON_BRUSH_PLAYER_EYE    0.68f // eye height above the capsule centre
#define HERON_BRUSH_PLAYER_STEP   0.55f // tallest step that can be walked up
// Air actions after leaving the ground, shared by the double jump and the
// wall jump: jump + wall, wall + wall, or wall + jump.
#define HERON_BRUSH_PLAYER_AIRJUMPS 2

typedef struct {
    float forward;  // -1..1, W/S
    float strafe;   // -1..1, D/A
    float fly;      // -1..1, E/Q; noclip, and up/down while swimming
    Vector2 look;   // mouse delta in pixels
    bool jump;      // pressed this frame
    bool jumpHeld;  // held (swim up)
    bool run;       // held (swim down)
} HeronBrushInput;

typedef struct {
    Vector3 pos; // capsule centre
    Vector3 vel;
    float yaw, pitch; // radians
    bool grounded;
    bool noclip;    // free-fly: no gravity, no collision
    int airJumps;   // air actions left before touching ground again
    float wallCooldown; // stops a wall jump chaining off one contact

    // View feel, all eased: a lean into strafes, a walk bob, a dip on landing.
    float roll, bobPhase, bobAmount, landDip;
    bool prevGrounded;

    // Stair smoothing: the capsule snaps up each step, the eye trails behind
    // by `stepLag` (<= 0) and eases back to it, so the view glides up.
    float stepLag, prevPosY;

    // Set for the one frame the thing happens, for the caller's sound effects.
    bool didJump, didWallJump, didLand;
} HeronBrushPlayer;

// Places the capsule with its feet at `feet`, looking along `yaw`.
void HeronBrushPlayerInit(HeronBrushPlayer *p, Vector3 feet, float yaw);

// One step. `inWater` softens gravity and turns jump into a swim stroke.
void HeronBrushPlayerUpdate(HeronBrushPlayer *p, const HeronBrushCollider *col, HeronBrushInput in,
                     bool inWater, float dt);

// The camera at the player's eye.
Camera3D HeronBrushPlayerCamera(const HeronBrushPlayer *p);

#endif
