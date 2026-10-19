#include "player.h"

#include "raymath.h"

#include <math.h>

#define R HERON_BRUSH_PLAYER_RADIUS
#define H HERON_BRUSH_PLAYER_HALF
#define JUMP_VEL 7.0f
#define GRAVITY  20.0f

static Vector3 forward_flat(float yaw) {
    return (Vector3){-sinf(yaw), 0.0f, -cosf(yaw)};
}
static Vector3 right_flat(float yaw) {
    return (Vector3){cosf(yaw), 0.0f, -sinf(yaw)};
}
static float toward(float a, float b, float step) {
    float d = b - a;
    return fabsf(d) <= step ? b : a + (d > 0.0f ? step : -step);
}
static float horiz_len(Vector3 v) { return sqrtf(v.x * v.x + v.z * v.z); }

// A wall the capsule is up against: probe outward every 30 degrees at mid
// height, take the nearest hit whose face is near-vertical. `*normal` points
// away from the wall.
static bool wall_normal(const HeronBrushCollider *col, Vector3 centre, Vector3 *normal) {
    float best = R + 0.28f;
    bool found = false;
    for (int i = 0; i < 12; i++) {
        float a = (float)i * (3.14159265f / 6.0f);
        Vector3 d = {cosf(a), 0.0f, sinf(a)};
        float hit = HeronBrushColliderRayHit(col, centre, d, R + 0.28f);
        if (hit >= 0.0f && hit < best) {
            best = hit;
            *normal = (Vector3){-d.x, 0.0f, -d.z};
            found = true;
        }
    }
    return found;
}

static float smooth(float a, float b, float rate, float dt) {
    return a + (b - a) * Clamp(rate * dt, 0.0f, 1.0f);
}

// A small horizontal kick on a jump, along the way you are already going (or
// the way you are steering when nearly stopped), capped so hops do not
// compound into a launch.
static void hop_boost(HeronBrushPlayer *p, Vector3 wish, float add, float cap) {
    Vector3 h = {p->vel.x, 0.0f, p->vel.z};
    float sp = horiz_len(h);
    Vector3 dir;
    if (sp > 0.5f) {
        dir = Vector3Scale(h, 1.0f / sp);
    } else {
        float wl = horiz_len(wish);
        if (wl < 0.1f) return;
        dir = Vector3Scale(wish, 1.0f / wl);
    }
    float ns = fminf(sp + add, cap);
    p->vel.x = dir.x * ns;
    p->vel.z = dir.z * ns;
}

// Ease the view-feel state toward a gentle strafe lean, a walk bob, and a dip
// on the frame the feet touch down. All soft: the lean takes about a third of
// a second to reach full.
static void view_feel(HeronBrushPlayer *p, float strafe, bool walking, float speed,
                      float dt) {
    float wantRoll = (walking || speed > 0.3f) ? -strafe * 0.035f : 0.0f;
    p->roll = smooth(p->roll, wantRoll, 6.0f, dt);
    p->bobAmount = smooth(p->bobAmount, walking ? 1.0f : 0.0f, 5.0f, dt);
    if (walking) p->bobPhase += dt * (6.5f + speed * 0.7f);
    p->landDip = smooth(p->landDip, 0.0f, 9.0f, dt);
    if (p->grounded && !p->prevGrounded) {
        p->landDip = -0.075f;
        p->didLand = true;
    }
    p->prevGrounded = p->grounded;

    // Stair smoothing: the capsule snaps up (or down) a step in one frame; the
    // eye holds still and eases the gap out, so a flight of tiny steps reads
    // as a ramp instead of a stutter. Only for a real step, not a jump.
    float dy = p->pos.y - p->prevPosY;
    if (p->grounded && fabsf(dy) > 0.02f && fabsf(dy) < HERON_BRUSH_PLAYER_STEP + 0.15f &&
        fabsf(p->vel.y) < 3.5f) {
        p->stepLag -= dy;
    }
    p->stepLag = smooth(p->stepLag, 0.0f, 14.0f, dt);
    p->stepLag = Clamp(p->stepLag, -0.55f, 0.55f);
    p->prevPosY = p->pos.y;
}

void HeronBrushPlayerInit(HeronBrushPlayer *p, Vector3 feet, float yaw) {
    *p = (HeronBrushPlayer){0};
    p->pos = (Vector3){feet.x, feet.y + H, feet.z};
    p->prevPosY = p->pos.y;
    p->yaw = yaw;
}

Camera3D HeronBrushPlayerCamera(const HeronBrushPlayer *p) {
    Camera3D cam = {0};
    cam.fovy = 70.0f;
    cam.projection = CAMERA_PERSPECTIVE;

    float bobY = sinf(p->bobPhase * 2.0f) * 0.028f * p->bobAmount;
    cam.position = (Vector3){
        p->pos.x,
        p->pos.y + HERON_BRUSH_PLAYER_EYE + bobY + p->landDip + p->stepLag,
        p->pos.z};

    Vector3 dir = {-sinf(p->yaw) * cosf(p->pitch), sinf(p->pitch),
                   -cosf(p->yaw) * cosf(p->pitch)};
    cam.target = Vector3Add(cam.position, dir);

    // A lean into strafes plus a touch of it from the walk bob, applied as a
    // roll of the up vector around the view direction.
    float roll = p->roll + sinf(p->bobPhase) * 0.012f * p->bobAmount;
    cam.up = Vector3RotateByAxisAngle((Vector3){0, 1, 0}, dir, roll);
    return cam;
}

// Horizontal move with step climbing. While grounded the whole capsule is
// lifted by the step height for the move, so anything shorter than a step is
// glided straight over instead of stopping the capsule's lower sphere. A real
// wall, taller than the step, still blocks. After the move the feet are
// planted back down onto whatever is now underneath.
static void move_horizontal(HeronBrushPlayer *p, const HeronBrushCollider *col, Vector3 flat,
                            bool grounded, bool *touchedGround) {
    float lift = grounded ? HERON_BRUSH_PLAYER_STEP : 0.0f;

    Vector3 pos = {p->pos.x + flat.x, p->pos.y + lift, p->pos.z + flat.z};
    bool g = false;
    HeronBrushColliderResolveCapsule(col, &pos, R, H, &p->vel, &g);

    if (grounded) {
        Vector3 foot = {pos.x, pos.y - H + 0.05f, pos.z};
        float drop = HeronBrushColliderFloorBelow(col, foot, lift + 0.45f);
        if (drop >= 0.0f) {
            pos.y += 0.05f - drop; // settle onto the step or floor
            g = true;
        } else {
            pos.y -= lift; // walked off an edge: let go, gravity takes over
        }
    }

    p->pos = pos;
    *touchedGround = g;
}

void HeronBrushPlayerUpdate(HeronBrushPlayer *p, const HeronBrushCollider *col, HeronBrushInput in,
                     bool inWater, float dt) {
    p->didJump = p->didWallJump = p->didLand = false;
    p->yaw -= in.look.x * 0.0025f;
    p->pitch = Clamp(p->pitch - in.look.y * 0.0025f, -1.54f, 1.54f);

    Vector3 wish = Vector3Add(Vector3Scale(forward_flat(p->yaw), in.forward),
                              Vector3Scale(right_flat(p->yaw), in.strafe));
    float wl = Vector3Length(wish);
    if (wl > 1e-4f) wish = Vector3Scale(wish, 1.0f / wl);

    if (p->noclip) {
        float sp = in.run ? 20.0f : 9.0f;
        Vector3 dir = {-sinf(p->yaw) * cosf(p->pitch), sinf(p->pitch),
                       -cosf(p->yaw) * cosf(p->pitch)};
        Vector3 move = Vector3Add(Vector3Scale(dir, in.forward),
                                  Vector3Scale(right_flat(p->yaw), in.strafe));
        move.y += in.fly;
        p->pos = Vector3Add(p->pos, Vector3Scale(move, sp * dt));
        p->vel = (Vector3){0};
        p->grounded = false;
        view_feel(p, 0.0f, false, 0.0f, dt);
        return;
    }

    // Horizontal velocity eases toward the wished-for speed. Less grip in air.
    float topSpeed = in.run ? 8.5f : 5.0f;
    if (inWater) topSpeed = 3.5f;
    Vector3 hv = {p->vel.x, 0.0f, p->vel.z};
    // Full grip on the ground, little in the air; a jump off the ground keeps
    // the air grip for that frame so a hop carries its momentum through the
    // touchdown instead of being scrubbed.
    float grip = inWater ? 4.0f : ((p->grounded && !in.jump) ? 12.0f : 2.5f);
    hv = Vector3Lerp(hv, Vector3Scale(wish, topSpeed), Clamp(grip * dt, 0.0f, 1.0f));
    p->vel.x = hv.x;
    p->vel.z = hv.z;

    if (inWater) {
        // Swimming (kit scripts/water.gd): the look direction carries you,
        // jump rises, run sinks, and you drift down slowly doing nothing.
        // vel.y eases toward that wished-for rate rather than snapping.
        float rise = in.forward * sinf(p->pitch) * 2.5f + in.fly * 2.5f;
        if (in.jumpHeld) rise += 3.5f;
        if (in.run) rise -= 3.5f;
        if (fabsf(rise) < 0.05f) rise = -0.8f;
        rise = Clamp(rise, -3.5f, 3.5f);
        p->vel.y = toward(p->vel.y, rise, 9.0f * dt);
    } else {
        p->vel.y -= GRAVITY * dt;
        if (p->wallCooldown > 0.0f) p->wallCooldown -= dt;

        if (p->grounded) {
            if (p->vel.y < 0.0f) p->vel.y = -3.0f; // hug the ground on slopes
            if (in.jump) {
                p->vel.y = JUMP_VEL;
                p->grounded = false;
                p->didJump = true;
                // A nudge for a hop with momentum, but only up to a hair over
                // top speed, so holding jump does not build a launch.
                hop_boost(p, wish, 0.35f, topSpeed + 1.5f);
            }
        } else if (in.jump && p->airJumps > 0) {
            Vector3 wn;
            if (p->wallCooldown <= 0.0f && wall_normal(col, p->pos, &wn)) {
                // Kick off the wall: up, away from it, plus a little of where
                // you are steering. Costs an air action, same as a double jump.
                p->vel.y = JUMP_VEL;
                p->vel.x = wn.x * 5.5f + wish.x * 2.5f;
                p->vel.z = wn.z * 5.5f + wish.z * 2.5f;
                hop_boost(p, wish, 0.6f, topSpeed + 3.0f);
                p->wallCooldown = 0.25f;
                p->didWallJump = true;
            } else {
                p->vel.y = JUMP_VEL; // plain double jump
                hop_boost(p, wish, 0.35f, topSpeed + 1.5f);
                p->didJump = true;
            }
            p->airJumps--;
        }
    }

    Vector3 delta = Vector3Scale(p->vel, dt);

    if (inWater) {
        // No stairs, no ground clamp, no floor snap under water: one move, one
        // resolve, so a swim stroke off the pool floor is not scrubbed off.
        bool g = false;
        p->pos = Vector3Add(p->pos, delta);
        HeronBrushColliderResolveCapsule(col, &p->pos, R, H, &p->vel, &g);
        p->grounded = false;
        p->airJumps = HERON_BRUSH_PLAYER_AIRJUMPS;
        view_feel(p, 0.0f, false, 0.0f, dt);
        return;
    }

    bool wasGrounded = p->grounded;
    bool gh = false;

    move_horizontal(p, col, (Vector3){delta.x, 0.0f, delta.z}, wasGrounded, &gh);

    if (wasGrounded && gh && !in.jump) {
        // move_horizontal planted the feet; a full vertical resolve here would
        // fight the step it just climbed. Just keep the fall speed in check.
        p->grounded = true;
        if (p->vel.y > 0.0f) p->vel.y = 0.0f;
    } else {
        bool gv = false;
        p->pos.y += delta.y;
        HeronBrushColliderResolveCapsule(col, &p->pos, R, H, &p->vel, &gv);
        p->grounded = gv;

        // Stepped off a ledge onto a lower surface within a step: snap to it
        // rather than starting a little fall.
        if (!p->grounded && !inWater && wasGrounded && p->vel.y <= 1.0f) {
            Vector3 foot = {p->pos.x, p->pos.y - H + 0.05f, p->pos.z};
            float drop = HeronBrushColliderFloorBelow(col, foot, HERON_BRUSH_PLAYER_STEP + 0.1f);
            if (drop >= 0.0f) {
                p->pos.y += 0.05f - drop;
                p->vel.y = 0.0f;
                p->grounded = true;
            }
        }
    }

    if (p->grounded) p->airJumps = HERON_BRUSH_PLAYER_AIRJUMPS;

    float speed = horiz_len(p->vel);
    view_feel(p, in.strafe, p->grounded && speed > 0.6f, speed, dt);
}
