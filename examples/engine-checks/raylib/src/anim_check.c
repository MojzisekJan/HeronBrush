// `--anim-check`: the editor's animations and the sample's collision together, on
// demos/motion_lab. The same checks, with the same limits, as the Godot project's
// demos/motion_lab/godot/verify.gd: ride the L-path shuttle, the lift and the rotor and
// stay on them, walk on the turning rotor, jump off, open the door from its pad and
// walk through, be stopped by the closed door, start the lift from its pad. A fixed
// 1/60 s step and the sample's own player and collider; nothing is drawn.

#include "anim_check.h"

#include "animated.h"
#include "player.h"
#include "raymath.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static const float DT = 1.0f / 60.0f;

static HeronBrushCollider *gCol;
static const HeronBrushLevel *gLevel;
static double gTime;
static int gErrors;

static void expect(bool ok, const char *what) {
    if (!ok) {
        gErrors++;
        printf("FAIL: %s\n", what);
    }
}

// The animation of the entity named `name` (as the level writes it), or -1.
static int animation_of(const char *name) {
    for (int a = 0; a < HeronBrushAnimatedCount(); a++) {
        int e = HeronBrushAnimatedEntity(a);
        if (e >= 0 && strcmp(gLevel->entities[e].name, name) == 0) return a;
    }
    return -1;
}

static Matrix pose(int a) {
    Matrix m = MatrixIdentity();
    HeronBrushAnimatedMatrix(HeronBrushAnimatedEntity(a), &m);
    return m;
}

static Vector3 origin(int a) {
    Matrix m = pose(a);
    return (Vector3){m.m12, m.m13, m.m14};
}

// The turn about the vertical axis, from where the entity's x axis points.
static float yaw_of(int a) {
    Matrix m = pose(a);
    return atan2f(-m.m2, m.m0);
}

static Vector3 local(int a, Vector3 p) { return Vector3Transform(p, MatrixInvert(pose(a))); }

// One frame, in the order the sample runs it: the animations (and what they carry),
// then the player.
static void step(HeronBrushPlayer *p, HeronBrushInput in) {
    Vector3 feet = {p->pos.x, p->pos.y - HERON_BRUSH_PLAYER_HALF, p->pos.z};
    p->pos = Vector3Add(p->pos, HeronBrushAnimatedUpdate(gCol, feet, p->grounded, DT, gTime));
    HeronBrushPlayerUpdate(p, gCol, in, false, DT);
    gTime += DT;
}

static void frames(HeronBrushPlayer *p, int n) {
    for (int i = 0; i < n; i++) step(p, (HeronBrushInput){0});
}

// Facing that makes `forward` walk along the flat direction (dx, dz).
static float facing(float dx, float dz) { return atan2f(-dx, -dz); }

static void reset_all(HeronBrushPlayer *p) {
    for (int a = 0; a < HeronBrushAnimatedCount(); a++) HeronBrushAnimatedStop(a);
    HeronBrushPlayerInit(p, (Vector3){16, 1, 12}, 0);
    frames(p, 8);
}

static void ride(HeronBrushPlayer *p, const char *label, int a, float seconds, Vector3 offset) {
    char what[160];
    reset_all(p);
    HeronBrushPlayerInit(p, Vector3Add(origin(a), offset), 0);
    frames(p, 30);
    snprintf(what, sizeof what, "%s: player did not land", label);
    expect(p->grounded, what);
    Vector3 anchor = local(a, p->pos);
    HeronBrushAnimatedPlay(a);
    float drift = 0, vertical = 0;
    int airborne = 0;
    Vector3 lo = origin(a), hi = lo;
    bool turned = false, cut = false;
    for (int i = 0; i < (int)(seconds * 60); i++) {
        step(p, (HeronBrushInput){0});
        Vector3 o = origin(a);
        lo = Vector3Min(lo, o);
        hi = Vector3Max(hi, o);
        turned = turned || fabsf(yaw_of(a)) > 1.0f;
        if (strcmp(label, "shuttle") == 0 && o.x < -4.04f && o.z > -5.96f) cut = true;
        Vector3 now = local(a, p->pos);
        drift = fmaxf(drift, hypotf(now.x - anchor.x, now.z - anchor.z));
        vertical = fmaxf(vertical, fabsf(now.y - anchor.y));
        if (!p->grounded) airborne++;
    }
    printf("%s: horizontal_drift %.4f m, vertical_drift %.4f m, airborne_frames %d, travel %.2f %.2f %.2f m\n",
           label, drift, vertical, airborne, hi.x - lo.x, hi.y - lo.y, hi.z - lo.z);
    if (strcmp(label, "shuttle") == 0) {
        expect(hi.x - lo.x > 5.9f && hi.z - lo.z > 5.9f, "Shuttle did not traverse both segments");
        expect(!cut, "Shuttle cut diagonally across its L path");
    }
    if (strcmp(label, "lift") == 0) expect(hi.y - lo.y > 5.9f, "Lift did not reach both floors");
    if (strcmp(label, "rotor") == 0) {
        expect(turned, "Rotating platform remained stationary");
        expect(drift < 0.01f, "Idle rotation correction regressed");
    }
    snprintf(what, sizeof what, "%s: horizontal drift %.4f m", label, drift);
    expect(drift < 0.16f, what);
    snprintf(what, sizeof what, "%s: vertical drift %.4f m", label, vertical);
    expect(vertical < 0.12f, what);
    snprintf(what, sizeof what, "%s: lost floor contact for %d frames", label, airborne);
    expect(airborne <= 2, what);
    if (strcmp(label, "rotor") == 0) {
        Vector3 before = local(a, p->pos);
        Vector3 o = origin(a);
        p->yaw = facing(o.x - p->pos.x, o.z - p->pos.z);
        for (int i = 0; i < 10; i++) step(p, (HeronBrushInput){.forward = 1});
        expect(Vector3Distance(local(a, p->pos), before) > 0.4f, "Player cannot walk on rotating platform");
    }
    // Jumping must detach instead of being glued to the moving body.
    step(p, (HeronBrushInput){.jump = true});
    frames(p, 11);
    snprintf(what, sizeof what, "%s: jump failed to detach", label);
    expect(!p->grounded, what);
}

int HeronBrushAnimCheck(HeronBrushCollider *col, const HeronBrushLevel *level) {
    gCol = col;
    gLevel = level;
    gTime = 0;
    gErrors = 0;
    int shuttle = animation_of("func_animate"), lift = animation_of("func_animate_1");
    int rotor = animation_of("func_animate_2"), door = animation_of("func_animate_3");
    if (shuttle < 0 || lift < 0 || rotor < 0 || door < 0) {
        printf("FAIL: this is not motion_lab's level and animation player "
               "(make ANIM=.../motion_lab_animation.h, --level .../motion_lab.level)\n");
        return 1;
    }
    HeronBrushPlayer p;
    ride(&p, "shuttle", shuttle, 17.0f, (Vector3){0, 0.4f, 0});
    ride(&p, "lift", lift, 15.0f, (Vector3){0, 0.4f, 0});
    ride(&p, "rotor", rotor, 120.0f, (Vector3){1, 0.4f, 0});

    // The door's pad, walked onto, starts it; it swings a quarter turn about its hinge.
    reset_all(&p);
    HeronBrushPlayerInit(&p, (Vector3){8, 0.02f, 3}, 0);
    frames(&p, 35);
    expect(HeronBrushAnimatedPlaying(door), "Door pad did not start animation");
    expect(fabsf(yaw_of(door)) > 0.05f, "Door did not rotate");
    frames(&p, 100);
    printf("door: turned %.4f rad when open\n", yaw_of(door));
    expect(fabsf(fabsf(yaw_of(door)) - PI / 2) < 0.02f, "Door did not fully open");
    // The open doorway lets the player through; the closed door stops them.
    HeronBrushPlayerInit(&p, (Vector3){8, 0.1f, 4.5f}, facing(0, 1));
    for (int i = 0; i < 35; i++) step(&p, (HeronBrushInput){.forward = 1});
    printf("door: open doorway, walked to z %.2f\n", p.pos.z);
    expect(p.pos.z > 6.5f, "Open doorway blocked player");
    HeronBrushAnimatedStop(door);
    frames(&p, 5);
    HeronBrushPlayerInit(&p, (Vector3){8, 0.1f, 4.5f}, facing(0, 1));
    for (int i = 0; i < 35; i++) step(&p, (HeronBrushInput){.forward = 1});
    printf("door: closed door, walked to z %.2f\n", p.pos.z);
    expect(p.pos.z < 5.8f, "Closed door collision failed");

    // The lift starts only through its own pad.
    HeronBrushAnimatedStop(lift);
    HeronBrushPlayerInit(&p, (Vector3){6, 0.02f, -3}, 0);
    frames(&p, 10);
    expect(HeronBrushAnimatedPlaying(lift), "Lift pad did not start animation");

    if (gErrors == 0) printf("HERON_MOTION_LAB_RAYLIB_OK\n");
    return gErrors == 0 ? 0 : 1;
}
