// The game on top of the walker: a menu, pause, shooting, enemies, pickups,
// health and lives. The behaviour and the numbers are lifted from the kit's
// GDScript (examples/fps-kit/scripts, objects) so a map plays about the same
// in either engine. Everything keys off entity classnames from the level.

#ifndef HERON_BRUSH_GAME_H
#define HERON_BRUSH_GAME_H

#include "collision.h"
#include "level.h"
#include "movers.h"
#include "player.h"
#include "raylib.h"

typedef enum {
    HERON_BRUSH_MENU, // also the pause screen: Esc from play comes back here
    HERON_BRUSH_PLAYING,
    HERON_BRUSH_DEAD,
    HERON_BRUSH_WON,
} HeronBrushGameState;

typedef struct {
    int entIndex;
    Vector3 pos, home;
    Vector3 vel;  // walkers only: gravity + capsule resolve, like the player
    float hp;
    float fireTimer;
    float t;
    bool alive;
    bool walker; // fps_walker: a dark block on the ground, not a flyer
} HeronBrushEnemy;

typedef struct {
    int entIndex;
    bool taken;
    float t;
} HeronBrushItem;

typedef struct {
    int entIndex;
    float hp;
    bool alive;
    float fuse;
    bool lit;
} HeronBrushBarrel;

typedef struct {
    int entIndex;
    int left;      // enemies still to spawn in the current wave
    float timer;
    bool done;
} HeronBrushSpawner;

// A shot in flight: a real body with a position and a velocity, so it can be
// seen coming and side-stepped. Player bolts and enemy bolts share the type;
// `fromPlayer` picks what they can hit.
typedef struct {
    Vector3 pos, vel;
    float life;    // seconds before it fizzles out
    float damage;
    float radius;
    bool fromPlayer;
    bool alive;
} HeronBrushProjectile;

#define HERON_BRUSH_MAX_ENEMIES     96
#define HERON_BRUSH_MAX_ITEMS       160
#define HERON_BRUSH_MAX_BARRELS     32
#define HERON_BRUSH_MAX_SPAWNERS    16
#define HERON_BRUSH_MAX_KEYS        8
#define HERON_BRUSH_MAX_PROJECTILES 256

typedef struct {
    HeronBrushGameState state;
    bool started; // play has begun at least once, so the menu is a pause screen

    float hp, maxHp;
    int lives;
    long score;
    float sinceHurt; // seconds since last damage, for regen

    int weapon; // 0 blaster (slow, hard), 1 repeater (fast, soft)
    float shootTimer;
    float hitMarker; // countdown while the hit cross shows
    float muzzle;    // countdown while the muzzle flash shows

    char keys[HERON_BRUSH_MAX_KEYS][32];
    int keyCount;

    Vector3 spawnFeet;
    float spawnYaw;

    char toast[128];
    float toastTime;

    float portalCooldown;
    float portalFlash; // full-screen flash right after a teleport
    float deadTimer;
    int lastCheckpoint;
    float hurtTimer;

    HeronBrushEnemy enemies[HERON_BRUSH_MAX_ENEMIES];
    int enemyCount;
    HeronBrushItem items[HERON_BRUSH_MAX_ITEMS];
    int itemCount;
    HeronBrushBarrel barrels[HERON_BRUSH_MAX_BARRELS];
    int barrelCount;
    HeronBrushSpawner spawners[HERON_BRUSH_MAX_SPAWNERS];
    int spawnerCount;
    HeronBrushProjectile projectiles[HERON_BRUSH_MAX_PROJECTILES];
    int projCount;
} HeronBrushGame;

void HeronBrushGameInit(HeronBrushGame *g, const HeronBrushLevel *level, Vector3 spawnFeet, float spawnYaw);

// One PLAYING step: enemies, pickups, hurt volumes, portals, spawners, regen,
// and the win/lose checks. Moves `player` for portals and jump pads.
void HeronBrushGameUpdate(HeronBrushGame *g, const HeronBrushLevel *level, const HeronBrushCollider *col,
                   HeronBrushPlayer *player, HeronBrushMovers *movers, float dt);

// Fire the weapon: a hitscan ray from the camera.
void HeronBrushGameShoot(HeronBrushGame *g, const HeronBrushLevel *level, const HeronBrushCollider *col,
                  Camera3D camera);

// Names of the keys the player is carrying, for gating key doors.
const char *const *HeronBrushGameKeys(const HeronBrushGame *g, int *count);

// Yaw, pitch and range to the nearest live enemy from `eye`, for the scripted
// --demo. False when there is nothing to aim at.
bool HeronBrushGameAimHint(const HeronBrushGame *g, Vector3 eye, float *yaw, float *pitch,
                    float *range);

// Respawn at the last checkpoint (or the start) after a death, keeping score.
void HeronBrushGameRespawn(HeronBrushGame *g, HeronBrushPlayer *player);

// True for classnames the game draws itself, so main.c leaves them alone.
bool HeronBrushGameOwnsClass(const char *classname);

// GL resources for the HUD (a rounded-rectangle shader). Call once after the
// window is up, and free before it closes.
void HeronBrushGameLoadAssets(void);
void HeronBrushGameUnloadAssets(void);

void HeronBrushGameDrawWorld(const HeronBrushGame *g, const HeronBrushLevel *level, Camera3D camera);
void HeronBrushGameDrawEnemyBars(const HeronBrushGame *g, Camera3D camera, int screenW, int screenH);
void HeronBrushGameDrawHUD(const HeronBrushGame *g, int screenW, int screenH);

// Which menu button the mouse is over, for click handling in main.c.
typedef enum { HERON_BRUSH_MENU_NONE, HERON_BRUSH_MENU_PLAY, HERON_BRUSH_MENU_QUIT } HeronBrushMenuHit;
HeronBrushMenuHit HeronBrushGameMenuHit(int screenW, int screenH);

// Sound effects, played by id. `HeronBrushGameLoadAssets` opens the files; the audio
// device must already be up (main.c calls InitAudioDevice first).
typedef enum {
    HERON_BRUSH_SND_SHOOT,
    HERON_BRUSH_SND_JUMP,
    HERON_BRUSH_SND_LAND,
    HERON_BRUSH_SND_PICKUP,
    HERON_BRUSH_SND_HURT,
    HERON_BRUSH_SND_DIE,
    HERON_BRUSH_SND_COUNT,
} HeronBrushSound;
void HeronBrushGameSound(HeronBrushSound id);

// One frame of the sound row's mouse handling (checkbox click, slider click
// and drag). Call it every frame the menu is up. Returns true while the
// pointer is over or working the row, so main.c does not also treat the click
// as a menu press.
bool HeronBrushGameOptionsInput(int screenW, int screenH);
void HeronBrushGameDrawOptions(int screenW, int screenH);

#endif
