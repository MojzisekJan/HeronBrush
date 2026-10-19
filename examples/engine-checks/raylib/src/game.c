#include "game.h"

#include "raymath.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

// Rounded rectangles for the HUD, with analytical anti-aliasing after
// blog.frost.kiwi/analytical-anti-aliasing: the fragment's coverage is worked
// out from its signed distance to the shape's edge, one pixel of gradient
// across it, rather than from MSAA samples. Sharp at any size.
//
// The corners are a squircle, not a quarter circle: the positive part of the
// distance uses an L4 norm instead of L2, which is the continuous-curvature
// rounding of an iOS icon rather than a plain CSS border-radius.
static const char *ROUND_FS =
    "#version 330\n"
    "in vec2 fragTexCoord;\n"
    "in vec4 fragColor;\n"
    "uniform sampler2D texture0;\n"
    "uniform vec4 colDiffuse;\n"
    "uniform vec2 uSize;\n"
    "uniform float uRadius;\n"
    "uniform float uGloss;\n" // 0 flat, 1 a light sheen down from the top
    "out vec4 finalColor;\n"
    "float lp4(vec2 v) {\n"
    "  v = v * v;\n"
    "  v = v * v;\n"
    "  return pow(v.x + v.y, 0.25);\n"
    "}\n"
    "float sdSquircle(vec2 p, vec2 b, float r) {\n"
    "  vec2 q = abs(p) - b + r;\n"
    "  return min(max(q.x, q.y), 0.0) + lp4(max(q, vec2(0.0))) - r;\n"
    "}\n"
    "void main() {\n"
    "  vec2 p = (fragTexCoord - 0.5) * uSize;\n"
    "  float d = sdSquircle(p, uSize * 0.5, uRadius);\n"
    "  float aa = max(length(vec2(dFdx(d), dFdy(d))), 1e-5);\n"
    "  float cov = clamp(0.5 - d / aa, 0.0, 1.0);\n"
    "  vec4 c = colDiffuse * fragColor * texture(texture0, fragTexCoord);\n"
    "  float top = smoothstep(0.55, 0.0, fragTexCoord.y);\n"
    "  c.rgb = mix(c.rgb, min(c.rgb * 1.4 + 0.18, vec3(1.0)), uGloss * top);\n"
    "  finalColor = vec4(c.rgb, c.a * cov);\n"
    "}\n";

static Shader gRound;
static int gRoundSizeLoc, gRoundRadLoc, gRoundGlossLoc;
static Texture2D gWhite;
static Font gFont;
static bool gFontLoaded;

// A portal disc: a swirl, concentric pulses and a bright rim, on a
// camera-facing quad. `uColor` is the portal's own colour from the map.
static const char *PORTAL_FS =
    "#version 330\n"
    "in vec2 fragTexCoord;\n"
    "uniform float uTime;\n"
    "uniform vec3 uColor;\n"
    "out vec4 finalColor;\n"
    "void main() {\n"
    "  vec2 uv = fragTexCoord * 2.0 - 1.0;\n"
    "  float r = length(uv);\n"
    "  if (r > 1.0) discard;\n"
    "  float ang = atan(uv.y, uv.x);\n"
    "  float swirl = sin(ang * 3.0 + uTime * 2.2 - r * 9.0) * 0.5 + 0.5;\n"
    "  float rings = sin(r * 20.0 - uTime * 5.0) * 0.5 + 0.5;\n"
    "  float glow  = smoothstep(1.0, 0.15, r);\n"
    "  float rim   = smoothstep(1.0, 0.86, r) * smoothstep(0.80, 0.95, r) * 2.0;\n"
    "  vec3  col   = uColor * (0.35 + swirl * 0.65) * (0.5 + rings * 0.5)\n"
    "              + vec3(rim);\n"
    "  float a = glow * (0.5 + rings * 0.3) + rim;\n"
    "  finalColor = vec4(col, clamp(a, 0.0, 0.95));\n"
    "}\n";

static Shader gPortal;
static int gPortalTimeLoc, gPortalColLoc;

static Sound gSnd[HERON_BRUSH_SND_COUNT];
static bool gAudio;
static float gVolume = 0.7f;
static bool gMuted;
static bool gSliderGrab; // the volume knob is being dragged

static void apply_volume(void) {
    SetMasterVolume(gMuted ? 0.0f : gVolume);
}

void HeronBrushGameSound(HeronBrushSound id) {
    if (gAudio && id >= 0 && id < HERON_BRUSH_SND_COUNT && gSnd[id].frameCount > 0)
        PlaySound(gSnd[id]);
}

void HeronBrushGameLoadAssets(void) {
    gRound = LoadShaderFromMemory(NULL, ROUND_FS);
    gRoundSizeLoc = GetShaderLocation(gRound, "uSize");
    gRoundRadLoc = GetShaderLocation(gRound, "uRadius");
    gRoundGlossLoc = GetShaderLocation(gRound, "uGloss");
    Image w = GenImageColor(2, 2, WHITE);
    gWhite = LoadTextureFromImage(w);
    UnloadImage(w);

    if (FileExists("assets/font.ttf")) {
        gFont = LoadFontEx("assets/font.ttf", 64, NULL, 0);
        SetTextureFilter(gFont.texture, TEXTURE_FILTER_BILINEAR);
        gFontLoaded = gFont.texture.id != 0;
    }

    gPortal = LoadShaderFromMemory(NULL, PORTAL_FS);
    gPortalTimeLoc = GetShaderLocation(gPortal, "uTime");
    gPortalColLoc = GetShaderLocation(gPortal, "uColor");

    if (IsAudioDeviceReady()) {
        gAudio = true;
        const char *files[HERON_BRUSH_SND_COUNT] = {
            "assets/sounds/blaster.ogg", "assets/sounds/jump_a.ogg",
            "assets/sounds/land.ogg",    "assets/sounds/pickup.wav",
            "assets/sounds/enemy_hurt.ogg", "assets/sounds/enemy_destroy.ogg",
        };
        for (int i = 0; i < HERON_BRUSH_SND_COUNT; i++)
            if (FileExists(files[i])) gSnd[i] = LoadSound(files[i]);
        apply_volume();
    }
}

void HeronBrushGameUnloadAssets(void) {
    UnloadShader(gRound);
    UnloadShader(gPortal);
    UnloadTexture(gWhite);
    if (gFontLoaded) UnloadFont(gFont);
    for (int i = 0; i < HERON_BRUSH_SND_COUNT; i++)
        if (gSnd[i].frameCount > 0) UnloadSound(gSnd[i]);
}

// Text through the loaded font (the kit's Lilita One), or raylib's built-in
// one if it is missing. A soft drop shadow keeps it readable over the map.
static void tx(const char *s, float x, float y, float sz, Color c) {
    float off = fmaxf(1.0f, sz * 0.04f);
    Color shadow = {0, 0, 0, (unsigned char)(c.a * 0.33f)};
    if (gFontLoaded) {
        DrawTextEx(gFont, s, (Vector2){x + off, y + off}, sz, sz * 0.06f, shadow);
        DrawTextEx(gFont, s, (Vector2){x, y}, sz, sz * 0.06f, c);
    } else {
        DrawText(s, (int)(x + off), (int)(y + off), (int)sz, shadow);
        DrawText(s, (int)x, (int)y, (int)sz, c);
    }
}
static float txw(const char *s, float sz) {
    if (gFontLoaded) return MeasureTextEx(gFont, s, sz, sz * 0.06f).x;
    return (float)MeasureText(s, (int)sz);
}
static void txmid(const char *s, float cx, float y, float sz, Color c) {
    tx(s, cx - txw(s, sz) * 0.5f, y, sz, c);
}

// A rounded rectangle, `r` the corner radius and `gloss` 0..1 a light sheen
// running down from the top edge.
static void rrect_g(float x, float y, float w, float h, float r, Color c,
                    float gloss) {
    if (w <= 0.0f || h <= 0.0f) return;
    r = fminf(r, fminf(w, h) * 0.5f);
    float size[2] = {w, h};
    SetShaderValue(gRound, gRoundSizeLoc, size, SHADER_UNIFORM_VEC2);
    SetShaderValue(gRound, gRoundRadLoc, &r, SHADER_UNIFORM_FLOAT);
    SetShaderValue(gRound, gRoundGlossLoc, &gloss, SHADER_UNIFORM_FLOAT);
    BeginShaderMode(gRound);
    DrawTexturePro(gWhite, (Rectangle){0, 0, 2, 2}, (Rectangle){x, y, w, h},
                   (Vector2){0, 0}, 0.0f, c);
    EndShaderMode();
}
static void rrect(float x, float y, float w, float h, float r, Color c) {
    rrect_g(x, y, w, h, r, c, 0.0f);
}

// Numbers from the kit (examples/fps-kit). Metres and seconds.
#define PLAYER_HP      100.0f
#define LIVES          3
#define REGEN_DELAY    4.0f
#define REGEN_RATE     14.0f
#define ENEMY_HP       100.0f
#define ENEMY_SHOT     4.0f
#define ENEMY_RELOAD   1.7f
#define ENEMY_RANGE    22.0f
#define ENEMY_KEEP     6.0f  // metres it tries to hold from the player
#define ENEMY_SPEED    2.6f
#define ENEMY_R        0.45f
#define WALKER_SPEED   2.9f
#define WALKER_HH      0.90f // half the block's height, same as the player capsule
#define WALKER_HW      0.35f // half its width
#define BARREL_HP      30.0f
#define BARREL_DMG     60.0f
#define BARREL_RADIUS  4.0f
#define BARREL_R       0.5f
#define ITEM_REACH     0.95f

// Bolts in flight. The enemy's is slow enough to read and side-step; the
// player's is quick but still takes a moment to cross the arena.
#define ENEMY_BOLT_SPEED 15.0f
#define PLAYER_BOLT_SPEED 78.0f
#define REPEATER_BOLT_SPEED 96.0f
#define BOLT_LIFE      3.0f
#define BOLT_R         0.13f

static Vector3 v3(float x, float y, float z) { return (Vector3){x, y, z}; }
static Vector3 add3(Vector3 a, Vector3 b) { return Vector3Add(a, b); }
static Vector3 sub3(Vector3 a, Vector3 b) { return Vector3Subtract(a, b); }
static float dist3(Vector3 a, Vector3 b) { return Vector3Distance(a, b); }
static Vector3 entPos(const HeronBrushEntity *e) { return v3(e->pos[0], e->pos[1], e->pos[2]); }

static void toast(HeronBrushGame *g, const char *msg) {
    strncpy(g->toast, msg, sizeof(g->toast) - 1);
    g->toast[sizeof(g->toast) - 1] = '\0';
    g->toastTime = 2.6f;
}

static void hurt_player(HeronBrushGame *g, float amount) {
    if (g->state != HERON_BRUSH_PLAYING) return;
    g->hp -= amount;
    g->sinceHurt = 0.0f;
    HeronBrushGameSound(HERON_BRUSH_SND_HURT);
    if (g->hp <= 0.0f) {
        g->hp = 0.0f;
        g->state = HERON_BRUSH_DEAD;
        g->deadTimer = 1.4f;
        g->lives--;
    }
}

// Ray vs a sphere at `c` radius `r`. Distance along `d` (unit) or -1.
static float ray_sphere(Vector3 o, Vector3 d, Vector3 c, float r) {
    Vector3 oc = sub3(o, c);
    float b = Vector3DotProduct(oc, d);
    float cc = Vector3DotProduct(oc, oc) - r * r;
    float disc = b * b - cc;
    if (disc < 0.0f) return -1.0f;
    float t = -b - sqrtf(disc);
    return t >= 0.0f ? t : -1.0f;
}

// Ray against an enemy body: one sphere for a flying drone, a short stack of
// them up the tall walker box. Nearest hit distance along `d`, or -1.
static float ray_enemy(Vector3 o, Vector3 d, const HeronBrushEnemy *en, float pad) {
    if (!en->walker) return ray_sphere(o, d, en->pos, ENEMY_R + pad);
    float best = -1.0f;
    for (int k = -1; k <= 1; k++) {
        Vector3 c = {en->pos.x, en->pos.y + (float)k * WALKER_HH * 0.6f, en->pos.z};
        float t = ray_sphere(o, d, c, WALKER_HW + 0.12f + pad);
        if (t >= 0.0f && (best < 0.0f || t < best)) best = t;
    }
    return best;
}

static void spawn_proj(HeronBrushGame *g, Vector3 pos, Vector3 dir, float speed,
                       float damage, bool fromPlayer) {
    if (g->projCount >= HERON_BRUSH_MAX_PROJECTILES) return;
    float dl = Vector3Length(dir);
    if (dl < 1e-4f) return;
    HeronBrushProjectile *pr = &g->projectiles[g->projCount++];
    pr->pos = pos;
    pr->vel = Vector3Scale(dir, speed / dl);
    pr->life = BOLT_LIFE;
    pr->damage = damage;
    pr->radius = BOLT_R;
    pr->fromPlayer = fromPlayer;
    pr->alive = true;
}

// -------------------------------------------------------------------------

void HeronBrushGameInit(HeronBrushGame *g, const HeronBrushLevel *level, Vector3 spawnFeet, float spawnYaw) {
    memset(g, 0, sizeof(*g));
    g->state = HERON_BRUSH_MENU;
    g->hp = g->maxHp = PLAYER_HP;
    g->lives = LIVES;
    g->spawnFeet = spawnFeet;
    g->spawnYaw = spawnYaw;
    g->lastCheckpoint = -1;

    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        const char *c = e->classname;

        if ((strcmp(c, "fps_enemy") == 0 || strcmp(c, "fps_walker") == 0) &&
            g->enemyCount < HERON_BRUSH_MAX_ENEMIES) {
            HeronBrushEnemy *en = &g->enemies[g->enemyCount++];
            en->entIndex = i;
            en->home = en->pos = entPos(e);
            en->hp = ENEMY_HP;
            en->alive = true;
            en->walker = strcmp(c, "fps_walker") == 0;
        } else if ((strcmp(c, "fps_pickup") == 0 || strcmp(c, "fps_key") == 0 ||
                    strcmp(c, "fps_weapon_pickup") == 0) &&
                   g->itemCount < HERON_BRUSH_MAX_ITEMS) {
            g->items[g->itemCount++] = (HeronBrushItem){.entIndex = i};
        } else if (strcmp(c, "fps_barrel") == 0 && g->barrelCount < HERON_BRUSH_MAX_BARRELS) {
            HeronBrushBarrel *b = &g->barrels[g->barrelCount++];
            b->entIndex = i;
            b->hp = HeronBrushPropFloat(e, "health", BARREL_HP);
            b->alive = true;
        } else if (strcmp(c, "fps_spawner") == 0 && g->spawnerCount < HERON_BRUSH_MAX_SPAWNERS) {
            g->spawners[g->spawnerCount++] = (HeronBrushSpawner){.entIndex = i};
        }
    }
}

void HeronBrushGameRespawn(HeronBrushGame *g, HeronBrushPlayer *player) {
    HeronBrushPlayerInit(player, g->spawnFeet, g->spawnYaw);
    g->hp = g->maxHp;
    g->sinceHurt = REGEN_DELAY;
    g->projCount = 0; // clear any bolts still in the air
    g->state = HERON_BRUSH_PLAYING;
}

const char *const *HeronBrushGameKeys(const HeronBrushGame *g, int *count) {
    static const char *ptrs[HERON_BRUSH_MAX_KEYS];
    for (int i = 0; i < g->keyCount; i++) ptrs[i] = g->keys[i];
    *count = g->keyCount;
    return ptrs;
}

bool HeronBrushGameAimHint(const HeronBrushGame *g, Vector3 eye, float *yaw, float *pitch,
                    float *range) {
    float best = 1e30f;
    Vector3 target = {0};
    for (int i = 0; i < g->enemyCount; i++) {
        if (!g->enemies[i].alive) continue;
        float d = dist3(eye, g->enemies[i].pos);
        if (d < best) best = d, target = g->enemies[i].pos;
    }
    if (best > 1e29f) return false;
    Vector3 d = Vector3Normalize(sub3(target, eye));
    *yaw = atan2f(-d.x, -d.z);
    *pitch = asinf(Clamp(d.y, -1.0f, 1.0f));
    *range = best;
    return true;
}

bool HeronBrushGameOwnsClass(const char *c) {
    return strcmp(c, "fps_enemy") == 0 || strcmp(c, "fps_walker") == 0 ||
           strcmp(c, "fps_pickup") == 0 ||
           strcmp(c, "fps_key") == 0 || strcmp(c, "fps_weapon_pickup") == 0 ||
           strcmp(c, "fps_barrel") == 0 || strcmp(c, "fps_portal") == 0 ||
           strcmp(c, "fps_spawner") == 0;
}

// -------------------------------------------------------------------------

static void explode_barrel(HeronBrushGame *g, const HeronBrushLevel *level, HeronBrushBarrel *b,
                           HeronBrushPlayer *player) {
    b->alive = false;
    Vector3 at = entPos(&level->entities[b->entIndex]);

    float pd = dist3(at, player->pos);
    if (pd < BARREL_RADIUS) {
        float share = 1.0f - pd / BARREL_RADIUS;
        hurt_player(g, BARREL_DMG * share);
        Vector3 away = Vector3Normalize(sub3(player->pos, at));
        player->vel = add3(player->vel, Vector3Scale(away, 6.0f * share));
        player->vel.y += 3.0f * share;
    }
    for (int i = 0; i < g->enemyCount; i++) {
        if (!g->enemies[i].alive) continue;
        float d = dist3(at, g->enemies[i].pos);
        if (d < BARREL_RADIUS) g->enemies[i].hp -= BARREL_DMG * (1.0f - d / BARREL_RADIUS);
    }
    for (int i = 0; i < g->barrelCount; i++) {
        HeronBrushBarrel *o = &g->barrels[i];
        if (o == b || !o->alive) continue;
        if (dist3(at, entPos(&level->entities[o->entIndex])) < BARREL_RADIUS) o->lit = true, o->fuse = 0.15f;
    }
    toast(g, "boom");
}

void HeronBrushGameShoot(HeronBrushGame *g, const HeronBrushLevel *level, const HeronBrushCollider *col,
                  Camera3D camera) {
    (void)level;
    (void)col;
    if (g->state != HERON_BRUSH_PLAYING || g->shootTimer > 0.0f) return;

    bool repeater = g->weapon == 1;
    g->shootTimer = repeater ? 0.09f : 0.22f;
    g->muzzle = 0.05f;

    Vector3 fwd = Vector3Normalize(sub3(camera.target, camera.position));
    Vector3 muzzle = add3(camera.position, Vector3Scale(fwd, 0.4f));

    int bolts = repeater ? 1 : 4;
    float perBolt = repeater ? 12.0f : 7.0f;
    float spread = repeater ? 0.006f : 0.02f;
    float speed = repeater ? REPEATER_BOLT_SPEED : PLAYER_BOLT_SPEED;

    for (int p = 0; p < bolts; p++) {
        Vector3 d = fwd;
        d.x += ((float)GetRandomValue(-1000, 1000) / 1000.0f) * spread;
        d.y += ((float)GetRandomValue(-1000, 1000) / 1000.0f) * spread;
        d.z += ((float)GetRandomValue(-1000, 1000) / 1000.0f) * spread;
        spawn_proj(g, muzzle, d, speed, perBolt, true);
    }

    HeronBrushGameSound(HERON_BRUSH_SND_SHOOT);
}

// -------------------------------------------------------------------------

static void update_enemies(HeronBrushGame *g, const HeronBrushCollider *col, HeronBrushPlayer *player,
                           float dt) {
    Vector3 eye = {player->pos.x, player->pos.y + HERON_BRUSH_PLAYER_EYE, player->pos.z};

    for (int i = 0; i < g->enemyCount; i++) {
        HeronBrushEnemy *en = &g->enemies[i];
        if (!en->alive) continue;
        en->t += dt;

        Vector3 flat = sub3(player->pos, en->home);
        flat.y = 0.0f;
        float d = Vector3Length(flat);

        if (en->walker) {
            // March straight in, stopping just short, and fall onto whatever
            // the player would stand on: a capsule with gravity, resolved the
            // same way as the player's.
            Vector3 step = {0.0f, 0.0f, 0.0f};
            if (d > 1.6f && d < ENEMY_RANGE + 8.0f) {
                Vector3 toP = Vector3Scale(flat, 1.0f / d);
                step = Vector3Scale(toP, WALKER_SPEED * dt);
            }
            en->vel.y -= 20.0f * dt;
            en->pos.x += step.x;
            en->pos.z += step.z;
            en->pos.y += en->vel.y * dt;
            bool grounded = false;
            HeronBrushColliderResolveCapsule(col, &en->pos, WALKER_HW, WALKER_HH,
                                      &en->vel, &grounded);
            if (grounded && en->vel.y < 0.0f) en->vel.y = 0.0f;
            en->home = en->pos;
        } else {
            // Flyer: hold roughly ENEMY_KEEP from the player, close in from far
            // off, back away if crowded, drift sideways otherwise so it is not
            // a stationary target.
            if (d > 0.1f) {
                Vector3 toP = Vector3Scale(flat, 1.0f / d);
                Vector3 side = {toP.z, 0.0f, -toP.x};
                Vector3 move = {0};
                if (d > ENEMY_KEEP + 1.0f && d < ENEMY_RANGE + 6.0f)
                    move = toP;
                else if (d < ENEMY_KEEP - 1.0f)
                    move = Vector3Scale(toP, -1.0f);
                else
                    move = Vector3Scale(side, sinf(en->t * 0.7f + i));
                en->home = add3(en->home, Vector3Scale(move, ENEMY_SPEED * dt));
            }
            en->pos = en->home;
            en->pos.y = en->home.y + sinf(en->t * 2.5f) * 0.3f;
        }

        en->fireTimer -= dt;
        float pd = dist3(en->pos, eye);
        if (en->fireTimer <= 0.0f && pd < ENEMY_RANGE) {
            Vector3 dir = sub3(eye, en->pos);
            float wall = HeronBrushColliderRayHit(col, en->pos, dir, pd - 0.3f);
            if (wall < 0.0f) {
                // Lead the player a touch, then loose a bolt they can outrun
                // sideways.
                Vector3 aim = add3(eye, Vector3Scale(player->vel, 0.12f));
                Vector3 from = add3(en->pos, Vector3Scale(
                    Vector3Normalize(sub3(aim, en->pos)), ENEMY_R + 0.15f));
                spawn_proj(g, from, sub3(aim, from), ENEMY_BOLT_SPEED, ENEMY_SHOT,
                           false);
                en->fireTimer = ENEMY_RELOAD;
            } else {
                en->fireTimer = 0.3f; // blocked, look again soon
            }
        }
    }
}

static void update_items(HeronBrushGame *g, const HeronBrushLevel *level, HeronBrushPlayer *player) {
    for (int i = 0; i < g->itemCount; i++) {
        HeronBrushItem *it = &g->items[i];
        if (it->taken) continue;
        const HeronBrushEntity *e = &level->entities[it->entIndex];
        Vector3 d = sub3(entPos(e), player->pos);
        if (sqrtf(d.x * d.x + d.z * d.z) > ITEM_REACH || fabsf(d.y) > 1.5f) continue;

        it->taken = true;
        HeronBrushGameSound(HERON_BRUSH_SND_PICKUP);
        const char *c = e->classname;
        if (strcmp(c, "fps_key") == 0) {
            const char *k = HeronBrushGetProp(e, "key");
            if (k && g->keyCount < HERON_BRUSH_MAX_KEYS) {
                strncpy(g->keys[g->keyCount], k, 31);
                g->keys[g->keyCount][31] = '\0';
                g->keyCount++;
            }
            char m[64];
            snprintf(m, sizeof(m), "picked up the %s key", k ? k : "?");
            toast(g, m);
        } else if (strcmp(c, "fps_weapon_pickup") == 0) {
            const char *w = HeronBrushGetProp(e, "weapon");
            g->weapon = (w && strcmp(w, "blaster-repeater") == 0) ? 1 : 0;
            toast(g, g->weapon ? "Blaster Repeater" : "Blaster");
        } else {
            int value = (int)HeronBrushPropFloat(e, "value", 1.0f);
            g->score += value * 25;
            if (g->hp < g->maxHp) {
                g->hp = fminf(g->maxHp, g->hp + 8.0f);
            }
            toast(g, "+25");
        }
    }
}

// A sensor entity whose world AABB contains `p`.
static const HeronBrushEntity *sensor_at(const HeronBrushLevel *level, const char *classname,
                                  Vector3 p, float pad) {
    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        if (strcmp(e->classname, classname) != 0) continue;
        Vector3 mn, mx;
        if (!HeronBrushEntityWorldAABB(e, &mn, &mx)) continue;
        if (p.x >= mn.x - pad && p.x <= mx.x + pad && p.y >= mn.y - pad &&
            p.y <= mx.y + pad && p.z >= mn.z - pad && p.z <= mx.z + pad)
            return e;
    }
    return NULL;
}

static void update_volumes(HeronBrushGame *g, const HeronBrushLevel *level, HeronBrushPlayer *player,
                           float dt) {
    Vector3 p = player->pos;

    // fps_checkpoint: remember where to come back, patch up.
    const HeronBrushEntity *cp = sensor_at(level, "fps_checkpoint", p, 0.0f);
    if (cp) {
        int idx = (int)(cp - level->entities);
        if (idx != g->lastCheckpoint) {
            g->lastCheckpoint = idx;
            g->spawnFeet = v3(p.x, p.y - HERON_BRUSH_PLAYER_HALF, p.z);
            g->spawnYaw = player->yaw;
            g->hp = g->maxHp;
            toast(g, "checkpoint");
        }
    }

    // fps_hurt: bites of damage.
    const HeronBrushEntity *hurt = sensor_at(level, "fps_hurt", p, 0.0f);
    if (hurt) {
        float interval = fmaxf(HeronBrushPropFloat(hurt, "interval", 0.5f), 0.05f);
        g->hurtTimer -= dt;
        if (g->hurtTimer <= 0.0f) {
            g->hurtTimer = interval;
            if ((int)HeronBrushPropFloat(hurt, "instant", 0.0f) != 0)
                hurt_player(g, g->maxHp + 1.0f);
            else
                hurt_player(g, HeronBrushPropFloat(hurt, "damage", 20.0f) * interval);
        }
    } else {
        g->hurtTimer = 0.0f;
    }

    // fps_exit: the way out.
    const HeronBrushEntity *exit = sensor_at(level, "fps_exit", p, 0.0f);
    if (exit) {
        const char *needs = HeronBrushGetProp(exit, "needs");
        bool ok = !needs || needs[0] == '\0';
        for (int i = 0; i < g->keyCount && !ok; i++)
            if (strcmp(g->keys[i], needs) == 0) ok = true;
        if (ok)
            g->state = HERON_BRUSH_WON;
        else
            toast(g, "the way out is locked");
    }

    // Fell out of the world.
    if (p.y < -25.0f) hurt_player(g, g->maxHp + 1.0f);
}

static void update_portals(HeronBrushGame *g, const HeronBrushLevel *level, HeronBrushPlayer *player,
                           float dt) {
    if (g->portalCooldown > 0.0f) {
        g->portalCooldown -= dt;
        return;
    }
    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        if (strcmp(e->classname, "fps_portal") != 0 || e->targetCount == 0) continue;
        Vector3 mouth = entPos(e);
        mouth.y += 1.0f;
        if (dist3(mouth, player->pos) > 1.3f) continue;

        const HeronBrushEntity *dst = HeronBrushFindEntity(level, e->targets[0]);
        if (!dst) continue;
        float yaw = dst->rot[1];
        Vector3 fwd = {-sinf(yaw), 0.0f, -cosf(yaw)};
        Vector3 out = add3(entPos(dst), Vector3Scale(fwd, 1.6f));
        player->pos = v3(out.x, out.y + HERON_BRUSH_PLAYER_HALF, out.z);
        player->yaw = yaw;
        g->portalCooldown = 0.8f;
        g->portalFlash = 0.4f;
        toast(g, "whoosh");
        return;
    }
}

static void update_spawners(HeronBrushGame *g, const HeronBrushLevel *level, HeronBrushPlayer *player,
                            float dt) {
    for (int s = 0; s < g->spawnerCount; s++) {
        HeronBrushSpawner *sp = &g->spawners[s];
        const HeronBrushEntity *e = &level->entities[sp->entIndex];

        if (sp->left == 0 && !sp->done) {
            // A sensor that targets this spawner and holds the player fires it.
            for (int i = 0; i < level->entityCount; i++) {
                const HeronBrushEntity *t = &level->entities[i];
                if (!t->collision.sensor) continue;
                bool aimed = false;
                for (int k = 0; k < t->targetCount; k++)
                    if (strcmp(t->targets[k], e->name) == 0) aimed = true;
                if (!aimed) continue;
                Vector3 mn, mx;
                if (HeronBrushEntityWorldAABB(t, &mn, &mx) && player->pos.x > mn.x &&
                    player->pos.x < mx.x && player->pos.z > mn.z && player->pos.z < mx.z) {
                    sp->left = (int)HeronBrushPropFloat(e, "count", 3.0f);
                    sp->timer = 0.0f;
                    toast(g, "here they come");
                }
            }
        }

        if (sp->left > 0) {
            sp->timer -= dt;
            if (sp->timer <= 0.0f && g->enemyCount < HERON_BRUSH_MAX_ENEMIES) {
                sp->timer = HeronBrushPropFloat(e, "interval", 0.8f);
                float spread = HeronBrushPropFloat(e, "spread", 2.0f);
                int n = (int)HeronBrushPropFloat(e, "count", 3.0f);
                float ang = 6.2831853f * (float)(n - sp->left) / (float)(n > 0 ? n : 1);
                HeronBrushEnemy *en = &g->enemies[g->enemyCount++];
                memset(en, 0, sizeof(*en));
                en->entIndex = sp->entIndex;
                en->home = en->pos = add3(entPos(e), v3(cosf(ang) * spread, 1.0f, sinf(ang) * spread));
                en->hp = ENEMY_HP;
                en->alive = true;
                sp->left--;
                if (sp->left == 0) sp->done = true;
            }
        }
    }
}

// Step every bolt: one move against the level and its targets, then drop it if
// it hit anything or timed out. Compacted in place so the array does not
// silently fill.
static void update_projectiles(HeronBrushGame *g, const HeronBrushLevel *level,
                               const HeronBrushCollider *col, HeronBrushPlayer *player,
                               float dt) {
    int w = 0;
    for (int i = 0; i < g->projCount; i++) {
        HeronBrushProjectile pr = g->projectiles[i];
        if (!pr.alive) continue;

        float speed = Vector3Length(pr.vel);
        Vector3 dir = speed > 1e-4f ? Vector3Scale(pr.vel, 1.0f / speed)
                                    : v3(0.0f, 0.0f, 1.0f);
        float step = speed * dt;
        float hitT = step;
        bool spent = false;

        float wall = HeronBrushColliderRayHit(col, pr.pos, dir, step + pr.radius);
        if (wall >= 0.0f && wall < hitT) hitT = wall, spent = true;

        if (pr.fromPlayer) {
            int he = -1, hb = -1;
            for (int e = 0; e < g->enemyCount; e++) {
                if (!g->enemies[e].alive) continue;
                float t = ray_enemy(pr.pos, dir, &g->enemies[e], pr.radius);
                if (t >= 0.0f && t < hitT) hitT = t, he = e, hb = -1;
            }
            for (int b = 0; b < g->barrelCount; b++) {
                if (!g->barrels[b].alive) continue;
                Vector3 bp = entPos(&level->entities[g->barrels[b].entIndex]);
                bp.y += 0.6f;
                float t = ray_sphere(pr.pos, dir, bp, BARREL_R + pr.radius);
                if (t >= 0.0f && t < hitT) hitT = t, hb = b, he = -1;
            }
            if (he >= 0) {
                g->enemies[he].hp -= pr.damage;
                g->hitMarker = 0.12f;
                HeronBrushGameSound(HERON_BRUSH_SND_HURT);
                spent = true;
            } else if (hb >= 0) {
                g->barrels[hb].hp -= pr.damage;
                g->hitMarker = 0.12f;
                spent = true;
            }
        } else {
            // The player as a capsule: a few points along the step against the
            // body axis. Cheap, and the enemy bolt is slow.
            float rr = HERON_BRUSH_PLAYER_RADIUS + pr.radius;
            for (int s = 0; s <= 4; s++) {
                Vector3 at = add3(pr.pos, Vector3Scale(dir, hitT * (float)s / 4.0f));
                float ay = Clamp(at.y, player->pos.y - HERON_BRUSH_PLAYER_HALF,
                                 player->pos.y + HERON_BRUSH_PLAYER_HALF);
                if (dist3(at, v3(player->pos.x, ay, player->pos.z)) <= rr) {
                    hurt_player(g, pr.damage);
                    spent = true;
                    break;
                }
            }
        }

        pr.pos = add3(pr.pos, Vector3Scale(dir, hitT));
        pr.life -= dt;
        if (spent || pr.life <= 0.0f) continue;
        g->projectiles[w++] = pr;
    }
    g->projCount = w;
}

void HeronBrushGameUpdate(HeronBrushGame *g, const HeronBrushLevel *level, const HeronBrushCollider *col,
                   HeronBrushPlayer *player, HeronBrushMovers *movers, float dt) {
    (void)movers;
    if (g->state != HERON_BRUSH_PLAYING) return;
    g->started = true;

    if (g->shootTimer > 0.0f) g->shootTimer -= dt;
    if (g->hitMarker > 0.0f) g->hitMarker -= dt;
    if (g->muzzle > 0.0f) g->muzzle -= dt;
    if (g->toastTime > 0.0f) g->toastTime -= dt;
    if (g->portalFlash > 0.0f) g->portalFlash -= dt;

    update_enemies(g, col, player, dt);
    update_projectiles(g, level, col, player, dt);
    update_items(g, level, player);
    update_volumes(g, level, player, dt);
    update_portals(g, level, player, dt);
    update_spawners(g, level, player, dt);

    for (int i = 0; i < g->enemyCount; i++) {
        if (g->enemies[i].alive && g->enemies[i].hp <= 0.0f) {
            g->enemies[i].alive = false;
            g->score += 100;
            HeronBrushGameSound(HERON_BRUSH_SND_DIE);
        }
    }

    for (int i = 0; i < g->barrelCount; i++) {
        HeronBrushBarrel *b = &g->barrels[i];
        if (!b->alive) continue;
        if (b->hp <= 0.0f && !b->lit) b->lit = true, b->fuse = HeronBrushPropFloat(&level->entities[b->entIndex], "fuse", 0.0f);
        if (b->lit) {
            b->fuse -= dt;
            if (b->fuse <= 0.0f) explode_barrel(g, level, b, player);
        }
    }

    // Health regen, the way the kit does not but a lone player needs.
    g->sinceHurt += dt;
    if (g->sinceHurt > REGEN_DELAY && g->hp < g->maxHp)
        g->hp = fminf(g->maxHp, g->hp + REGEN_RATE * dt);
}

// -------------------------------------------------------------------------

void HeronBrushGameDrawWorld(const HeronBrushGame *g, const HeronBrushLevel *level, Camera3D camera) {
    for (int i = 0; i < g->enemyCount; i++) {
        const HeronBrushEnemy *en = &g->enemies[i];
        if (!en->alive) continue;
        Vector3 p = en->pos;
        if (dist3(p, camera.position) < ENEMY_R + 0.25f) continue; // inside the camera
        Vector3 look = Vector3Normalize(sub3(camera.position, p));
        if (en->walker) {
            Vector3 sz = {WALKER_HW * 2.0f, WALKER_HH * 2.0f, WALKER_HW * 2.0f};
            DrawCubeV(p, sz, (Color){40, 42, 52, 255});
            DrawCubeWiresV(p, sz, (Color){14, 15, 22, 255});
            // a red eye near the top, facing the player
            Vector3 head = {p.x, p.y + WALKER_HH * 0.7f, p.z};
            DrawSphere(add3(head, Vector3Scale(look, WALKER_HW + 0.04f)), 0.1f,
                       (Color){210, 60, 50, 255});
            continue;
        }
        DrawSphere(p, ENEMY_R, (Color){220, 60, 50, 255});
        DrawSphereWires(p, ENEMY_R, 6, 8, (Color){120, 20, 20, 255});
        // an "eye" toward the player
        DrawSphere(add3(p, Vector3Scale(look, ENEMY_R * 0.8f)), 0.13f, (Color){255, 230, 120, 255});
        // The health bar is drawn later, in 2D (HeronBrushGameDrawEnemyBars).
    }

    for (int i = 0; i < g->itemCount; i++) {
        const HeronBrushItem *it = &g->items[i];
        if (it->taken) continue;
        const HeronBrushEntity *e = &level->entities[it->entIndex];
        Vector3 p = entPos(e);
        p.y += 0.25f + sinf((float)GetTime() * 3.0f + i) * 0.1f;
        const char *c = e->classname;
        Color col = strcmp(c, "fps_key") == 0          ? (Color){235, 70, 70, 255}
                    : strcmp(c, "fps_weapon_pickup") == 0 ? (Color){90, 210, 255, 255}
                                                          : (Color){255, 210, 60, 255};
        DrawSphere(p, 0.18f, col);
        DrawSphereWires(p, 0.24f, 5, 6, Fade(col, 0.6f));
    }

    for (int i = 0; i < g->barrelCount; i++) {
        const HeronBrushBarrel *b = &g->barrels[i];
        if (!b->alive) continue;
        Vector3 p = entPos(&level->entities[b->entIndex]);
        Color c = b->lit ? (Color){255, 180, 60, 255} : (Color){210, 110, 30, 255};
        DrawCylinder(v3(p.x, p.y - 0.6f, p.z), 0.4f, 0.4f, 1.2f, 10, c);
        DrawCylinderWires(v3(p.x, p.y - 0.6f, p.z), 0.4f, 0.4f, 1.2f, 10, (Color){90, 50, 20, 255});
    }

    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        if (strcmp(e->classname, "fps_portal") != 0) continue;
        Vector3 p = entPos(e);
        p.y += 1.1f;

        float rgb[3] = {0.35f, 0.74f, 1.0f};
        const char *cs = HeronBrushGetProp(e, "color");
        if (cs) sscanf(cs, "%f %f %f", &rgb[0], &rgb[1], &rgb[2]);
        float t = (float)GetTime();
        SetShaderValue(gPortal, gPortalTimeLoc, &t, SHADER_UNIFORM_FLOAT);
        SetShaderValue(gPortal, gPortalColLoc, rgb, SHADER_UNIFORM_VEC3);

        BeginShaderMode(gPortal);
        DrawBillboardRec(camera, gWhite, (Rectangle){0, 0, 2, 2}, p,
                         (Vector2){2.6f, 3.0f}, WHITE);
        EndShaderMode();
    }

    for (int i = 0; i < g->projCount; i++) {
        const HeronBrushProjectile *pr = &g->projectiles[i];
        if (!pr->alive) continue;
        Color core = pr->fromPlayer ? (Color){140, 225, 255, 255}
                                    : (Color){255, 110, 55, 255};
        Vector3 back = sub3(pr->pos, Vector3Scale(Vector3Normalize(pr->vel), 0.6f));
        DrawLine3D(back, pr->pos, Fade(core, 0.45f));
        DrawSphere(pr->pos, pr->radius, core);
        DrawSphere(pr->pos, pr->radius * 1.9f, Fade(core, 0.22f));
    }
    if (g->muzzle > 0.0f) {
        Vector3 m = add3(camera.position,
                         Vector3Scale(Vector3Normalize(sub3(camera.target, camera.position)), 0.4f));
        DrawSphere(m, 0.06f, (Color){255, 240, 150, 255});
    }
}

// Full health is green, empty is red, and it slides through amber on the way.
static Color hp_color(float frac) {
    return ColorFromHSV(Clamp(frac, 0.0f, 1.0f) * 125.0f, 0.72f, 0.88f);
}

static void bar(int x, int y, int w, int h, float frac, Color fg) {
    float rad = h * 0.5f;
    rrect(x - 2, y - 2, w + 4, h + 4, rad + 2, (Color){0, 0, 0, 170});
    rrect(x, y, w, h, rad, (Color){30, 32, 38, 220});
    rrect_g(x, y, w * Clamp(frac, 0.0f, 1.0f), h, rad, fg, 1.0f);
}

// A health bar above each hurt enemy, projected to the screen so it always
// faces the camera and gets the same rounded, anti-aliased edges as the HUD.
void HeronBrushGameDrawEnemyBars(const HeronBrushGame *g, Camera3D camera, int sw, int sh) {
    Vector3 fwd = Vector3Normalize(sub3(camera.target, camera.position));
    for (int i = 0; i < g->enemyCount; i++) {
        const HeronBrushEnemy *en = &g->enemies[i];
        if (!en->alive || en->hp >= ENEMY_HP) continue;
        float up = en->walker ? WALKER_HH + 0.30f : ENEMY_R + 0.32f;
        Vector3 top = add3(en->pos, v3(0.0f, up, 0.0f));
        if (Vector3DotProduct(sub3(top, camera.position), fwd) <= 0.0f) continue;
        Vector2 s = GetWorldToScreen(top, camera);
        if (s.x < -60.0f || s.x > sw + 60.0f || s.y < -40.0f || s.y > sh + 40.0f)
            continue;
        float f = en->hp / ENEMY_HP;
        float bw = 46.0f, bh = 7.0f;
        float x = s.x - bw * 0.5f, y = s.y - bh;
        rrect(x - 1.5f, y - 1.5f, bw + 3.0f, bh + 3.0f, (bh + 3.0f) * 0.5f,
              (Color){0, 0, 0, 190});
        rrect(x, y, bw, bh, bh * 0.5f, (Color){35, 38, 44, 230});
        rrect_g(x, y, bw * f, bh, bh * 0.5f, hp_color(f), 1.0f);
    }
}

// The two menu buttons, `which` 0 = play, 1 = quit.
static Rectangle menu_button(int sw, int sh, int which) {
    float bw = 260.0f, bh = 58.0f, gap = 16.0f;
    return (Rectangle){sw / 2.0f - bw / 2.0f, sh / 2.0f - 6.0f + which * (bh + gap),
                       bw, bh};
}

HeronBrushMenuHit HeronBrushGameMenuHit(int sw, int sh) {
    Vector2 m = GetMousePosition();
    if (CheckCollisionPointRec(m, menu_button(sw, sh, 0))) return HERON_BRUSH_MENU_PLAY;
    if (CheckCollisionPointRec(m, menu_button(sw, sh, 1))) return HERON_BRUSH_MENU_QUIT;
    return HERON_BRUSH_MENU_NONE;
}

static void menu_btn(int sw, int sh, int which, const char *label, bool hot) {
    Rectangle r = menu_button(sw, sh, which);
    Color fill = hot ? (Color){255, 220, 120, 240} : (Color){40, 48, 62, 230};
    Color text = hot ? (Color){20, 24, 32, 255} : (Color){225, 232, 245, 255};
    rrect(r.x, r.y, r.width, r.height, 14.0f, fill);
    txmid(label, r.x + r.width / 2.0f, r.y + r.height / 2.0f - 15.0f, 30.0f, text);
}

// The sound row, drawn low on the menu and pause screens: a "sound" checkbox
// on the left, then a volume slider you can click or drag by its knob.
static Rectangle opt_check(int sw, int sh) {
    return (Rectangle){sw / 2.0f - 200.0f, (float)sh - 112.0f, 20.0f, 20.0f};
}
static Rectangle opt_bar(int sw, int sh) {
    return (Rectangle){sw / 2.0f - 96.0f, (float)sh - 110.0f, 180.0f, 15.0f};
}
// The grab area around the bar: generous, so the knob is easy to catch and a
// drag keeps working when the pointer strays off the track.
static Rectangle opt_track(int sw, int sh) {
    Rectangle b = opt_bar(sw, sh);
    return (Rectangle){b.x - 10.0f, b.y - 13.0f, b.width + 20.0f, b.height + 26.0f};
}
static Vector2 opt_knob(int sw, int sh) {
    Rectangle b = opt_bar(sw, sh);
    float f = gMuted ? 0.0f : Clamp(gVolume, 0.0f, 1.0f);
    return (Vector2){b.x + b.width * f, b.y + b.height * 0.5f};
}

void HeronBrushGameDrawOptions(int sw, int sh) {
    Rectangle c = opt_check(sw, sh), b = opt_bar(sw, sh);
    Vector2 mp = GetMousePosition();
    bool hotC = CheckCollisionPointRec(mp, c);
    bool hotBar = gSliderGrab || CheckCollisionPointRec(mp, opt_track(sw, sh));
    bool on = !gMuted;

    rrect(c.x, c.y, c.width, c.height, 5.0f,
          hotC ? (Color){255, 220, 120, 240} : (Color){40, 48, 62, 235});
    if (on) {
        Color tick = hotC ? (Color){20, 24, 32, 255} : (Color){120, 200, 255, 255};
        DrawLineEx((Vector2){c.x + c.width * 0.22f, c.y + c.height * 0.52f},
                   (Vector2){c.x + c.width * 0.42f, c.y + c.height * 0.74f}, 3.0f, tick);
        DrawLineEx((Vector2){c.x + c.width * 0.42f, c.y + c.height * 0.74f},
                   (Vector2){c.x + c.width * 0.80f, c.y + c.height * 0.26f}, 3.0f, tick);
    }
    tx("sound", c.x + c.width + 10.0f, c.y + 1.0f, 18.0f,
       (Color){170, 185, 210, 255});

    rrect(b.x, b.y, b.width, b.height, b.height * 0.5f, (Color){28, 30, 36, 220});
    if (on) {
        rrect_g(b.x, b.y, b.width * Clamp(gVolume, 0.0f, 1.0f), b.height,
                b.height * 0.5f, (Color){120, 200, 255, 255}, 1.0f);
        // The drag knob, a little bigger when the pointer is on the track.
        Vector2 k = opt_knob(sw, sh);
        float kr = hotBar ? 11.0f : 9.0f;
        rrect(k.x - kr - 1.5f, k.y - kr - 1.5f, (kr + 1.5f) * 2.0f,
              (kr + 1.5f) * 2.0f, kr + 1.5f, (Color){0, 0, 0, 150});
        rrect_g(k.x - kr, k.y - kr, kr * 2.0f, kr * 2.0f, kr,
                (Color){235, 245, 255, 255}, 1.0f);
        rrect(k.x - kr * 0.45f, k.y - kr * 0.45f, kr * 0.9f, kr * 0.9f, kr * 0.45f,
              (Color){90, 170, 235, 255});
    }
    tx(TextFormat("%d%%", on ? (int)(gVolume * 100.0f + 0.5f) : 0),
       b.x + b.width + 16.0f, b.y - 4.0f, 20.0f, (Color){200, 210, 230, 255});
}

// One frame of the sound row's mouse handling. The checkbox toggles on a
// click; the slider sets its level on a click anywhere on the track and then
// follows the pointer until the button comes up. Returns true while the
// pointer is over or dragging the row, so the caller does not also read the
// click as a menu press.
bool HeronBrushGameOptionsInput(int sw, int sh) {
    Vector2 mp = GetMousePosition();
    Rectangle b = opt_bar(sw, sh);

    if (IsMouseButtonReleased(MOUSE_BUTTON_LEFT)) gSliderGrab = false;

    if (IsMouseButtonPressed(MOUSE_BUTTON_LEFT)) {
        if (CheckCollisionPointRec(mp, opt_check(sw, sh))) {
            gMuted = !gMuted;
            apply_volume();
            return true;
        }
        if (CheckCollisionPointRec(mp, opt_track(sw, sh))) gSliderGrab = true;
    }

    if (gSliderGrab && IsMouseButtonDown(MOUSE_BUTTON_LEFT)) {
        gVolume = Clamp((mp.x - b.x) / b.width, 0.0f, 1.0f);
        gMuted = false;
        apply_volume();
        return true;
    }

    return gSliderGrab || CheckCollisionPointRec(mp, opt_check(sw, sh)) ||
           CheckCollisionPointRec(mp, opt_track(sw, sh));
}

void HeronBrushGameDrawHUD(const HeronBrushGame *g, int sw, int sh) {
    float cx = sw / 2.0f, cy = sh / 2.0f;

    // A bright wash for a beat after stepping through a portal.
    if (g->portalFlash > 0.0f) {
        int a = (int)(Clamp(g->portalFlash / 0.4f, 0.0f, 1.0f) * 200.0f);
        DrawRectangle(0, 0, sw, sh, (Color){180, 220, 255, a});
    }

    // Crosshair.
    Color cc = g->hitMarker > 0.0f ? (Color){255, 90, 90, 255} : (Color){255, 255, 255, 220};
    int gap = 5, len = 11, x = sw / 2, y = sh / 2;
    DrawLine(x - gap - len, y, x - gap, y, cc);
    DrawLine(x + gap, y, x + gap + len, y, cc);
    DrawLine(x, y - gap - len, x, y - gap, cc);
    DrawLine(x, y + gap, x, y + gap + len, cc);
    if (g->hitMarker > 0.0f) {
        DrawLine(x - 7, y - 7, x + 7, y + 7, cc);
        DrawLine(x - 7, y + 7, x + 7, y - 7, cc);
    }

    if (g->state == HERON_BRUSH_PLAYING || g->state == HERON_BRUSH_DEAD) {
        float hpf = g->hp / g->maxHp;
        bar(22, sh - 46, 224, 22, hpf, hp_color(hpf));
        tx(TextFormat("%d", (int)ceilf(g->hp)), 32, sh - 44, 18, RAYWHITE);
        tx(TextFormat("lives  %d", g->lives < 0 ? 0 : g->lives), 22, sh - 74, 20, RAYWHITE);

        const char *sc = TextFormat("SCORE  %ld", g->score);
        tx(sc, sw - 24 - txw(sc, 24), 18, 24, RAYWHITE);
        int live = 0;
        for (int i = 0; i < g->enemyCount; i++)
            if (g->enemies[i].alive) live++;
        if (live > 0) {
            const char *en = TextFormat("enemies  %d", live);
            tx(en, sw - 24 - txw(en, 18), 48, 18, (Color){255, 150, 140, 255});
        }

        const char *wn = g->weapon ? "Blaster Repeater" : "Blaster";
        tx(wn, sw - 24 - txw(wn, 20), sh - 36, 20, (Color){200, 220, 255, 255});

        for (int i = 0; i < g->keyCount; i++)
            tx(TextFormat("[%s]", g->keys[i]), 22 + i * 80, sh - 102, 18,
               (Color){235, 90, 90, 255});

        if (g->toastTime > 0.0f) {
            int a = (int)(Clamp(g->toastTime / 0.6f, 0.0f, 1.0f) * 255);
            txmid(g->toast, cx, cy - 96, 24, (Color){255, 255, 255, a});
        }
    }

    if (g->state == HERON_BRUSH_DEAD) {
        DrawRectangle(0, 0, sw, sh, (Color){120, 0, 0, 90});
        txmid("You died", cx, cy - 64, 52, RAYWHITE);
        const char *s = g->lives >= 0 ? TextFormat("%d live%s left  -  press Enter", g->lives,
                                                   g->lives == 1 ? "" : "s")
                                      : "out of lives  -  press Enter for the menu";
        txmid(s, cx, cy + 12, 22, RAYWHITE);
    }

    if (g->state == HERON_BRUSH_MENU) {
        // The same screen serves as the pause menu once play has started: Esc
        // from the game comes back here, it never closes the game.
        DrawRectangle(0, 0, sw, sh, (Color){10, 14, 22, g->started ? 165 : 205});
        txmid(g->started ? "Paused" : "heron_brush  arena", cx, cy - 150, 62, RAYWHITE);
        if (g->started)
            txmid(TextFormat("score  %ld", g->score), cx, cy - 92, 20,
                  (Color){150, 165, 190, 255});
        else
            txmid("a brush map, exported flat, played in raylib", cx, cy - 92,
                  20, (Color){150, 165, 190, 255});
        HeronBrushMenuHit hot = HeronBrushGameMenuHit(sw, sh);
        menu_btn(sw, sh, 0, g->started ? "Resume" : "Play", hot == HERON_BRUSH_MENU_PLAY);
        menu_btn(sw, sh, 1, "Quit", hot == HERON_BRUSH_MENU_QUIT);
        HeronBrushGameDrawOptions(sw, sh);
        txmid(g->started ? "Esc or Enter to resume"
                         : "WASD move    space jump    shift run    mouse fire    V noclip",
              cx, sh - 54, 18, (Color){150, 165, 190, 255});
    }

    if (g->state == HERON_BRUSH_WON) {
        DrawRectangle(0, 0, sw, sh, (Color){8, 20, 12, 190});
        txmid("Arena cleared", cx, cy - 64, 56, RAYWHITE);
        txmid(TextFormat("score  %ld       Enter for the menu", g->score), cx, cy + 14,
              22, (Color){200, 230, 210, 255});
    }
}
