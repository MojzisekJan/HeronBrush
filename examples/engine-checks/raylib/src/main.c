// A heron_brush map in raylib: the other half of `heron_brush export --engine raylib`.
//
// Reads assets/maps/demo.level and the .glb meshes next to it, draws them with
// their exported glTF materials under a small PBR shader, and plays the map: a
// first-person controller against the level's `ctri` triangles and exact
// `chull` convex hulls, plus the small
// game the kit's FGD describes (see game.c) — shooting, enemies, pickups,
// health, a menu and a pause.
//
//   mouse         look and fire (Esc pauses)
//   WASD          walk, space jump (again in mid-air for a double jump), shift run
//   1 / 2         blaster / repeater
//   V             toggle noclip free-fly (Q/E for down/up)
//   C             show the collision geometry
//   P             hide the point-entity markers
//   F12           screenshot
//
// Triggers open the doors and lifts they target, jump pads throw you, water
// floats you, spawners drop enemy waves. That wiring came out of the .map;
// see docs/how-it-fits.md.

#include "anim_check.h"
#include "animated.h"
#include "collision.h"
#include "game.h"
#include "glb.h"
#include "level.h"
#include "lighting.h"
#include "movers.h"
#include "player.h"

#include "raylib.h"
#include "raymath.h"
#include "rlgl.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ASSETS_DIR "assets"
#define LEVEL_FILE "assets/maps/demo.level"

static char gLevelDir[512] = "assets/maps"; // where the level's mesh paths start

// One drawn primitive of an entity's mesh, with what the renderer needs to
// know about its glTF material.
typedef struct {
    int prim;               // raylib mesh index (= glTF primitive of mesh 0)
    int gltfMaterial;       // into EntityMesh.info->materials, or -1
    bool blended;           // alphaMode BLEND, or water: drawn sorted, after the opaque world
    bool doubleSided;
    bool sharedTexture;     // the albedo is one of gTextures, not the model's own
    Vector3 centre;         // mesh-local centroid, for back-to-front sorting
} Surface;

typedef struct {
    Model model;
    HeronBrushGlbInfo *info;
    Surface surfaces[HERON_BRUSH_MAX_SURFACES];
    int surfaceCount;
    bool loaded;
    bool water;
} EntityMesh;

// A blended primitive queued for the sorted pass.
typedef struct {
    int ent, surface;
    float depth;
} Blended;

static Shader gWater;
static int gWaterTimeLoc, gWaterCamLoc, gWaterSunLoc;

static Texture2D gTextures[HERON_BRUSH_MAX_TEXTURES];
static Texture2D gFallback;
static EntityMesh gMeshes[HERON_BRUSH_MAX_ENTITIES];
static Blended gBlended[HERON_BRUSH_MAX_ENTITIES * 4];

// A stable colour per classname, so the same kind of entity is the same hue.
static Color ClassColor(const char *name) {
    unsigned h = 2166136261u;
    for (const char *c = name; *c; c++) h = (h ^ (unsigned char)*c) * 16777619u;
    return ColorFromHSV((float)(h % 360), 0.55f, 0.95f);
}

static Texture2D FindTexture(const HeronBrushLevel *level, const char *name) {
    for (int i = 0; i < level->textureCount; i++) {
        if (strcmp(level->textures[i].name, name) == 0 && gTextures[i].id != 0) {
            return gTextures[i];
        }
    }
    return gFallback;
}

// The external images the level names, used only where a .glb carries no
// embedded albedo (TGA/WebP sources, or an export made before embedding).
static void LoadTextures(const HeronBrushLevel *level) {
    for (int i = 0; i < level->textureCount; i++) {
        const HeronBrushTexture *t = &level->textures[i];
        if (t->image[0] == '\0') continue;
        char path[512];
        snprintf(path, sizeof(path), "%s/%s", ASSETS_DIR, t->image);
        if (!FileExists(path)) continue;
        gTextures[i] = LoadTexture(path);
        GenTextureMipmaps(&gTextures[i]);
        SetTextureFilter(gTextures[i], TEXTURE_FILTER_TRILINEAR);
        SetTextureWrap(gTextures[i], strncmp(t->name, "bbpaint_", 8) == 0 ? TEXTURE_WRAP_CLAMP : TEXTURE_WRAP_REPEAT);
    }
}

// Mipmaps and the glTF sampler's wrap mode for a texture the model owns.
static void PrepareTexture(Texture2D *tex, bool clamp) {
    if (tex->id == 0 || tex->id == rlGetTextureIdDefault()) return;
    if (tex->mipmaps <= 1) GenTextureMipmaps(tex);
    SetTextureFilter(*tex, TEXTURE_FILTER_TRILINEAR);
    SetTextureWrap(*tex, clamp ? TEXTURE_WRAP_CLAMP : TEXTURE_WRAP_REPEAT);
}

static bool HasTexture(const Material *m, int map) {
    unsigned id = m->maps[map].texture.id;
    return id != 0 && id != rlGetTextureIdDefault();
}

static void LoadMeshes(const HeronBrushLevel *level) {
    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        // No `surface` lines means the `.glb` holds only collision geometry, a
        // clip brush or a trigger volume. Nothing to draw, and raylib warns
        // when it loads a mesh with no material, so skip it here.
        if (e->mesh[0] == '\0' || e->surfaceCount == 0) continue;

        char path[512];
        snprintf(path, sizeof(path), "%s/%s", gLevelDir, e->mesh);
        if (!FileExists(path)) {
            TraceLog(LOG_WARNING, "missing mesh %s", path);
            continue;
        }

        Model model = LoadModel(path);
        if (model.meshCount == 0) {
            UnloadModel(model);
            continue;
        }
        EntityMesh *m = &gMeshes[i];
        m->model = model;
        // raylib drops the factors, alpha mode, samplers and emissive strength;
        // read them from the file itself.
        m->info = calloc(1, sizeof(HeronBrushGlbInfo));
        if (m->info && !HeronBrushReadGlbInfo(path, m->info)) {
            free(m->info);
            m->info = NULL;
        }

        // raylib loads every mesh in the .glb, the collision mesh included.
        // Only the primitives a `surface` line names get drawn.
        m->water = strcmp(e->classname, "fps_water") == 0;
        for (int s = 0; s < e->surfaceCount && m->surfaceCount < HERON_BRUSH_MAX_SURFACES; s++) {
            int prim = e->surfaces[s].primitive;
            if (prim < 0 || prim >= model.meshCount) continue;
            int mat = model.meshMaterial[prim];
            if (mat < 0 || mat >= model.materialCount) continue;
            Surface *sf = &m->surfaces[m->surfaceCount++];
            sf->prim = prim;
            sf->gltfMaterial = (m->info && prim < m->info->primitiveCount)
                                   ? m->info->primitiveMaterial[prim] : -1;
            const HeronBrushGlbMaterial *gm =
                sf->gltfMaterial >= 0 ? &m->info->materials[sf->gltfMaterial] : NULL;
            sf->blended = m->water || (gm && gm->alphaMode == HERON_BRUSH_ALPHA_BLEND);
            sf->doubleSided = gm && gm->doubleSided;

            Material *rm = &model.materials[mat];
            bool clamp = gm && gm->baseClamp;
            if (!HasTexture(rm, MATERIAL_MAP_ALBEDO)) {
                rm->maps[MATERIAL_MAP_ALBEDO].texture = FindTexture(level, e->surfaces[s].texture);
                sf->sharedTexture = true;
            } else {
                PrepareTexture(&rm->maps[MATERIAL_MAP_ALBEDO].texture, clamp);
            }
            PrepareTexture(&rm->maps[MATERIAL_MAP_NORMAL].texture, clamp);
            PrepareTexture(&rm->maps[MATERIAL_MAP_EMISSION].texture, clamp);
            rm->maps[MATERIAL_MAP_ALBEDO].color = WHITE;
            if (m->water) rm->shader = gWater; // the rest get the PBR shader once it exists

            const Mesh *mesh = &model.meshes[prim];
            Vector3 sum = {0};
            for (int v = 0; v < mesh->vertexCount; v++)
                sum = Vector3Add(sum, (Vector3){mesh->vertices[v * 3], mesh->vertices[v * 3 + 1],
                                                mesh->vertices[v * 3 + 2]});
            sf->centre = Vector3Scale(sum, 1.0f / (float)(mesh->vertexCount > 0 ? mesh->vertexCount : 1));
        }
        m->loaded = true;
    }
}

// Feet position and facing of the first `info_player_start`, or the middle of
// the world a couple of metres up if the map has none.
static void FindSpawn(const HeronBrushLevel *level, Vector3 *feet, float *yaw) {
    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        if (strcmp(e->classname, "info_player_start") != 0) continue;
        *feet = (Vector3){e->pos[0], e->pos[1], e->pos[2]};
        *yaw = e->rot[1];
        return;
    }
    *feet = (Vector3){0, 3, 0};
    *yaw = 0.0f;
}

// Where entity `i` is drawn: its exported transform (scale, YXZ rotation,
// position), then a mover's current offset. Collision uses the same matrix.
static Matrix EntityDrawMatrix(const HeronBrushLevel *level, const HeronBrushMovers *movers, int i) {
    Matrix animated;
    if (HeronBrushAnimatedMatrix(i, &animated)) return animated;
    Vector3 o = HeronBrushMoverDrawOffset(movers, i);
    return MatrixMultiply(HeronBrushEntityMatrix(&level->entities[i]), MatrixTranslate(o.x, o.y, o.z));
}

// Union of every drawn mesh's world-space bounding box, for framing the sun's
// shadow pass. Falls back to the collider bounds if nothing drew.
static void SceneBounds(const HeronBrushLevel *level, const HeronBrushCollider *col,
                        Vector3 *min, Vector3 *max) {
    bool any = false;
    for (int i = 0; i < level->entityCount; i++) {
        if (!gMeshes[i].loaded) continue;
        BoundingBox b = GetModelBoundingBox(gMeshes[i].model);
        Matrix m = HeronBrushEntityMatrix(&level->entities[i]);
        for (int c = 0; c < 8; c++) {
            Vector3 p = {(c & 1) ? b.max.x : b.min.x, (c & 2) ? b.max.y : b.min.y,
                         (c & 4) ? b.max.z : b.min.z};
            p = Vector3Transform(p, m);
            *min = any ? Vector3Min(*min, p) : p;
            *max = any ? Vector3Max(*max, p) : p;
            any = true;
        }
    }
    if (!any) {
        *min = col->min;
        *max = col->max;
    }
}

static void DrawSurface(const HeronBrushLighting *light, EntityMesh *m, const Surface *sf, Matrix at) {
    Material mat = m->model.materials[m->model.meshMaterial[sf->prim]];
    if (!m->water) {
        const HeronBrushGlbMaterial *gm = sf->gltfMaterial >= 0 ? &m->info->materials[sf->gltfMaterial] : NULL;
        HeronBrushLightingSetMaterial(light, gm, HasTexture(&mat, MATERIAL_MAP_NORMAL),
                               HasTexture(&mat, MATERIAL_MAP_EMISSION));
    }
    if (sf->doubleSided) rlDisableBackfaceCulling();
    DrawMesh(m->model.meshes[sf->prim], mat, at);
    if (sf->doubleSided) rlEnableBackfaceCulling();
}

// The opaque world: every primitive that is not blended. Also the shadow
// casters for the sun's depth pass.
static void DrawOpaque(const HeronBrushLevel *level, const HeronBrushMovers *movers, const HeronBrushLighting *light) {
    for (int i = 0; i < level->entityCount; i++) {
        EntityMesh *m = &gMeshes[i];
        if (!m->loaded) continue;
        Matrix at = EntityDrawMatrix(level, movers, i);
        for (int s = 0; s < m->surfaceCount; s++)
            if (!m->surfaces[s].blended) DrawSurface(light, m, &m->surfaces[s], at);
    }
}

static int FartherFirst(const void *a, const void *b) {
    float da = ((const Blended *)a)->depth, db = ((const Blended *)b)->depth;
    return (da < db) - (da > db);
}

// Blended primitives (alphaMode BLEND, water) after the opaque world: sorted
// back to front by the distance of each primitive's centroid from the camera,
// depth-tested but not depth-written, so they neither hide each other nor
// anything drawn after them.
static void DrawBlended(const HeronBrushLevel *level, const HeronBrushMovers *movers, const HeronBrushLighting *light,
                        Vector3 eye) {
    int n = 0;
    for (int i = 0; i < level->entityCount; i++) {
        EntityMesh *m = &gMeshes[i];
        if (!m->loaded) continue;
        Matrix at = EntityDrawMatrix(level, movers, i);
        for (int s = 0; s < m->surfaceCount; s++) {
            if (!m->surfaces[s].blended || n >= (int)(sizeof(gBlended) / sizeof(gBlended[0]))) continue;
            Vector3 c = Vector3Transform(m->surfaces[s].centre, at);
            gBlended[n++] = (Blended){i, s, Vector3DistanceSqr(c, eye)};
        }
    }
    if (n == 0) return;
    qsort(gBlended, (size_t)n, sizeof(Blended), FartherFirst);
    rlDrawRenderBatchActive();
    rlDisableDepthMask();
    for (int k = 0; k < n; k++) {
        EntityMesh *m = &gMeshes[gBlended[k].ent];
        DrawSurface(light, m, &m->surfaces[gBlended[k].surface],
                    EntityDrawMatrix(level, movers, gBlended[k].ent));
    }
    rlEnableDepthMask();
}

// The other half of the water: a 2D pass over the whole screen for being under
// the surface. No render texture, so it cannot bend the scene like the kit's
// shader; a blue-green wash, moving light bands, and a heavy vignette.
static void DrawUnderwater(int sw, int sh, float t) {
    DrawRectangle(0, 0, sw, sh, (Color){18, 74, 96, 120});

    for (int i = 0; i < 7; i++) {
        float base = (float)sh * (0.12f + 0.12f * i);
        float y = base + sinf(t * 0.9f + i * 1.7f) * 24.0f;
        DrawRectangle(0, (int)y, sw, 2, (Color){170, 230, 240, 26});
    }

    int v = (int)(sh * 0.28f);
    DrawRectangleGradientV(0, 0, sw, v, (Color){6, 34, 46, 150}, (Color){0, 0, 0, 0});
    DrawRectangleGradientV(0, sh - v, sw, v, (Color){0, 0, 0, 0}, (Color){6, 34, 46, 150});
    int hh = (int)(sw * 0.18f);
    DrawRectangleGradientH(0, 0, hh, sh, (Color){6, 34, 46, 120}, (Color){0, 0, 0, 0});
    DrawRectangleGradientH(sw - hh, 0, hh, sh, (Color){0, 0, 0, 0}, (Color){6, 34, 46, 120});
}

int main(int argc, char **argv) {
    // raylib's TakeScreenshot writes relative to the working directory, so
    // --shot takes a bare filename, not a path.
    const char *shot = NULL;
    int shotFrames = 90;
    bool demo = false, animCheck = false, play = false, startNoclip = false, startShowCollision = false, noAim = false;
    bool haveAt = false, haveYaw = false, mapSun = false;
    const char *levelFile = LEVEL_FILE;
    Vector3 atPos = {0};
    float atYaw = 0.0f;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--shot") == 0 && i + 1 < argc) shot = argv[++i];
        else if (strcmp(argv[i], "--frames") == 0 && i + 1 < argc) shotFrames = atoi(argv[++i]);
        else if (strcmp(argv[i], "--demo") == 0) demo = true;
        else if (strcmp(argv[i], "--anim-check") == 0) animCheck = true;
        else if (strcmp(argv[i], "--play") == 0) play = true;
        else if (strcmp(argv[i], "--noclip") == 0) startNoclip = true;
        else if (strcmp(argv[i], "--showcol") == 0) startShowCollision = true;
        else if (strcmp(argv[i], "--noaim") == 0) noAim = true;
        else if (strcmp(argv[i], "--map-sun") == 0) mapSun = true;
        else if (strcmp(argv[i], "--level") == 0 && i + 1 < argc) levelFile = argv[++i];
        else if (strcmp(argv[i], "--at") == 0 && i + 1 < argc) {
            haveAt = sscanf(argv[++i], "%f,%f,%f", &atPos.x, &atPos.y, &atPos.z) == 3;
        } else if (strcmp(argv[i], "--look") == 0 && i + 1 < argc) {
            atYaw = atof(argv[++i]) * 3.14159265f / 180.0f;
            haveYaw = true;
        }
    }

    SetTraceLogLevel(LOG_WARNING);
    SetConfigFlags(FLAG_MSAA_4X_HINT | FLAG_WINDOW_RESIZABLE);
    InitWindow(1280, 720, "heron_brush map in raylib");
    if (!IsWindowReady()) {
        fprintf(stderr, "Cannot initialize the raylib window; a working display and OpenGL context are required.\n");
        return 1;
    }
    SetExitKey(KEY_NULL); // Esc pauses to the menu; it must not close the window
    InitAudioDevice();
    SetTargetFPS(60);

    ChangeDirectory(GetApplicationDirectory());
    if (!FileExists(levelFile)) {
        // Running from the crate directory rather than next to the binary.
        ChangeDirectory("examples/raylib-demo");
    }

    static HeronBrushLevel level; // a few megabytes; not a stack object
    if (!HeronBrushLoadLevel(levelFile, &level)) {
        CloseWindow();
        return 1;
    }
    snprintf(gLevelDir, sizeof(gLevelDir), "%s", GetDirectoryPath(levelFile));

    Image checked = GenImageChecked(64, 64, 8, 8, (Color){90, 90, 100, 255},
                                    (Color){130, 130, 140, 255});
    gFallback = LoadTextureFromImage(checked);
    UnloadImage(checked);

    HeronBrushGameLoadAssets();
    // Water keeps its own stylized shader (assets/shaders/*/water.*).
    gWater = LoadShader("assets/shaders/" HERON_BRUSH_GLSL "/water.vs", "assets/shaders/" HERON_BRUSH_GLSL "/water.fs");
    gWaterTimeLoc = GetShaderLocation(gWater, "uTime");
    gWaterCamLoc = GetShaderLocation(gWater, "uCam");
    gWaterSunLoc = GetShaderLocation(gWater, "uSun");

    LoadTextures(&level);
    LoadMeshes(&level);

    HeronBrushCollider collider;
    HeronBrushColliderBuild(&collider, &level);

    // Light the world geometry with the PBR shader: the editor's default sun
    // and sky (or the map's own with --map-sun), the map's point lights and a
    // shadow map for the sun. Markers, enemies and bolts stay on the stock
    // shader.
    Vector3 sceneMin, sceneMax;
    SceneBounds(&level, &collider, &sceneMin, &sceneMax);
    HeronBrushLighting light;
    if (!HeronBrushLightingInit(&light, &level, sceneMin, sceneMax, mapSun)) {
        CloseWindow();
        return 1;
    }
    {
        float s[3] = {light.sunDir.x, light.sunDir.y, light.sunDir.z};
        SetShaderValue(gWater, gWaterSunLoc, s, SHADER_UNIFORM_VEC3);
    }
    for (int i = 0; i < level.entityCount; i++) {
        if (!gMeshes[i].loaded || gMeshes[i].water) continue;
        Model *mdl = &gMeshes[i].model;
        for (int m = 0; m < mdl->materialCount; m++) mdl->materials[m].shader = light.shader;
    }

    Vector3 feet;
    float yaw;
    FindSpawn(&level, &feet, &yaw);
    if (haveAt) feet = atPos;

    HeronBrushMovers movers;
    HeronBrushMoversBuild(&movers, &collider, &level);
    // The editor's keyframe animations, compiled in from the export (`make ANIM=`).
    if (!HeronBrushAnimatedBuild(&collider, &level))
        TraceLog(LOG_WARNING, "the compiled-in animations are for another map; rebuild with make -B ANIM=...");
    if (animCheck) {
        int code = HeronBrushAnimCheck(&collider, &level);
        HeronBrushMoversFree(&movers);
        HeronBrushColliderFree(&collider);
        CloseWindow();
        return code;
    }

    Vector3 startFeet = haveAt ? atPos : feet;
    float startYaw = haveYaw ? atYaw : yaw;

    HeronBrushPlayer player;
    HeronBrushPlayerInit(&player, startFeet, startYaw);
    player.noclip = startNoclip;

    static HeronBrushGame game;
    HeronBrushGameInit(&game, &level, startFeet, startYaw);
    // The scripted run skips the menu, and so does --play, which leaves the controls
    // alone (for a screenshot of the level as it moves).
    if (demo || play) game.state = HERON_BRUSH_PLAYING;

    bool showMarkers = true;
    bool showCollision = startShowCollision;
    bool grabbed = false;
    bool quit = false;

    int frame = 0;
    double animTime = 0; // the animations run on the simulated clock, not the wall clock
    bool submerged = false;
    while (!WindowShouldClose() && !quit) {
        frame++;
        float dt = GetFrameTime();
        if (dt > 0.05f) dt = 0.05f; // a long stall must not fling the player

        if (IsKeyPressed(KEY_F12)) TakeScreenshot("heron_brush-raylib.png");
        bool enter = IsKeyPressed(KEY_ENTER) || IsKeyPressed(KEY_KP_ENTER);

        switch (game.state) {
        case HERON_BRUSH_MENU:
            // This screen is also the pause menu. Esc never quits: the game
            // only closes on the Quit button or the window's own close. Esc or
            // Enter resumes a game that is already under way.
            if (enter || (game.started && IsKeyPressed(KEY_ESCAPE))) {
                game.state = HERON_BRUSH_PLAYING;
            } else {
                // Runs every frame so a slider drag is followed, not just the
                // press.
                bool onRow = HeronBrushGameOptionsInput(GetScreenWidth(), GetScreenHeight());
                if (!onRow && IsMouseButtonPressed(MOUSE_BUTTON_LEFT)) {
                    HeronBrushMenuHit h = HeronBrushGameMenuHit(GetScreenWidth(), GetScreenHeight());
                    if (h == HERON_BRUSH_MENU_PLAY) game.state = HERON_BRUSH_PLAYING;
                    else if (h == HERON_BRUSH_MENU_QUIT) quit = true;
                }
            }
            break;
        case HERON_BRUSH_PLAYING:
            if (IsKeyPressed(KEY_ESCAPE)) game.state = HERON_BRUSH_MENU; // pause
            if (IsKeyPressed(KEY_P)) showMarkers = !showMarkers;
            if (IsKeyPressed(KEY_C)) showCollision = !showCollision;
            if (IsKeyPressed(KEY_V)) {
                player.noclip = !player.noclip;
                player.vel = (Vector3){0};
            }
            if (IsKeyPressed(KEY_ONE)) game.weapon = 0;
            if (IsKeyPressed(KEY_TWO)) game.weapon = 1;
            if (IsMouseButtonDown(MOUSE_BUTTON_LEFT) || (demo && frame % 10 < 5))
                HeronBrushGameShoot(&game, &level, &collider, HeronBrushPlayerCamera(&player));
            break;
        case HERON_BRUSH_DEAD:
            game.deadTimer -= dt;
            if (game.deadTimer <= 0.0f || enter) {
                if (game.lives >= 0) {
                    HeronBrushGameRespawn(&game, &player);
                } else {
                    HeronBrushPlayerInit(&player, startFeet, startYaw);
                    HeronBrushGameInit(&game, &level, startFeet, startYaw);
                }
            }
            break;
        case HERON_BRUSH_WON:
            if (enter) {
                HeronBrushPlayerInit(&player, startFeet, startYaw);
                HeronBrushGameInit(&game, &level, startFeet, startYaw);
            }
            break;
        }

        bool wantGrab = game.state == HERON_BRUSH_PLAYING;
        if (wantGrab != grabbed) {
            grabbed = wantGrab;
            if (grabbed) DisableCursor();
            else EnableCursor();
        }

        Camera3D camera = HeronBrushPlayerCamera(&player);

        if (game.state == HERON_BRUSH_PLAYING) {
            HeronBrushInput in = {0};
            if (demo) {
                in.forward = 1.0f;
                in.jump = (frame % 150 == 90) || (frame % 150 == 99);
                // Normally hold jump so a fall in the pool swims back out;
                // --noaim instead dives (screenshots of the underwater view)
                // and weaves, so the strafe-lean shows.
                in.jumpHeld = !noAim;
                in.run = noAim;
                if (noAim) {
                    in.strafe = sinf(frame * 0.05f) * 0.5f;
                    in.jump = frame % 20 == 0; // hop often, for wall jumps
                }
                Camera3D c = HeronBrushPlayerCamera(&player);
                float wantYaw, wantPitch, range;
                if (!noAim &&
                    HeronBrushGameAimHint(&game, c.position, &wantYaw, &wantPitch, &range) &&
                    range < 16.0f) {
                    // A close enemy: turn to it and hold ground so the scripted
                    // run shows a fight. look.x/.y are mouse deltas; player.c
                    // negates them.
                    float dy = wantYaw - player.yaw;
                    while (dy > 3.14159f) dy -= 6.28318f;
                    while (dy < -3.14159f) dy += 6.28318f;
                    in.look.x = -dy / 0.0025f * fminf(dt * 4.0f, 1.0f);
                    in.look.y = -(wantPitch - player.pitch) / 0.0025f * fminf(dt * 4.0f, 1.0f);
                    in.forward = range > 8.0f ? 0.5f : 0.0f;
                } else if (!haveYaw) {
                    in.look.x = 42.0f * dt;
                }
            } else {
                in.forward = (float)(IsKeyDown(KEY_W) - IsKeyDown(KEY_S));
                in.strafe = (float)(IsKeyDown(KEY_D) - IsKeyDown(KEY_A));
                in.fly = (float)(IsKeyDown(KEY_E) - IsKeyDown(KEY_Q));
                in.jump = IsKeyPressed(KEY_SPACE);
                in.jumpHeld = IsKeyDown(KEY_SPACE);
                in.run = IsKeyDown(KEY_LEFT_SHIFT);
                in.look = GetMouseDelta();
            }

            // Doors and lifts, then a lift can carry the player up with it.
            Vector3 feetNow = {player.pos.x, player.pos.y - HERON_BRUSH_PLAYER_HALF, player.pos.z};
            int nkeys = 0;
            const char *const *keys = HeronBrushGameKeys(&game, &nkeys);
            Vector3 ride = HeronBrushMoversUpdate(&movers, &collider, &level, player.pos,
                                          feetNow, HERON_BRUSH_PLAYER_RADIUS, keys, nkeys, dt);
            player.pos = Vector3Add(player.pos, ride);

            // The editor's animations, then whatever animated thing the player stands
            // on carries them, turning with it.
            animTime += dt;
            Vector3 feetAnimated = {player.pos.x, player.pos.y - HERON_BRUSH_PLAYER_HALF, player.pos.z};
            player.pos = Vector3Add(player.pos,
                                    HeronBrushAnimatedUpdate(&collider, feetAnimated, player.grounded, dt, animTime));

            bool inWater = false;
            float waterTop = 0.0f;
            for (int i = 0; i < level.entityCount && !inWater; i++) {
                if (strcmp(level.entities[i].classname, "fps_water") != 0) continue;
                Vector3 mn, mx;
                if (HeronBrushEntityWorldAABB(&level.entities[i], &mn, &mx) &&
                    player.pos.x > mn.x && player.pos.x < mx.x &&
                    player.pos.z > mn.z && player.pos.z < mx.z &&
                    player.pos.y - HERON_BRUSH_PLAYER_HALF < mx.y &&
                    player.pos.y + HERON_BRUSH_PLAYER_HALF > mn.y) {
                    inWater = true;
                    waterTop = mx.y;
                }
            }
            submerged = inWater && player.pos.y + HERON_BRUSH_PLAYER_EYE < waterTop;

            HeronBrushPlayerUpdate(&player, &collider, in, inWater, dt);
            if (player.didJump || player.didWallJump) HeronBrushGameSound(HERON_BRUSH_SND_JUMP);
            if (player.didLand) HeronBrushGameSound(HERON_BRUSH_SND_LAND);

            // Jump pads: touch one and it throws you up.
            if (!player.noclip) {
                for (int i = 0; i < level.entityCount; i++) {
                    const HeronBrushEntity *e = &level.entities[i];
                    if (strcmp(e->classname, "fps_jump_pad") != 0) continue;
                    Vector3 mn, mx;
                    if (!HeronBrushEntityWorldAABB(e, &mn, &mx)) continue;
                    float feetY = player.pos.y - HERON_BRUSH_PLAYER_HALF;
                    if (player.pos.x > mn.x - 0.3f && player.pos.x < mx.x + 0.3f &&
                        player.pos.z > mn.z - 0.3f && player.pos.z < mx.z + 0.3f &&
                        feetY < mx.y + 0.4f && feetY > mn.y - 0.3f) {
                        float s = HeronBrushPropFloat(e, "strength", 14.0f);
                        player.vel.y = fmaxf(player.vel.y, s * 0.9f);
                        player.grounded = false;
                    }
                }
            }

            HeronBrushGameUpdate(&game, &level, &collider, &player, &movers, dt);
            camera = HeronBrushPlayerCamera(&player);
        }

        BeginDrawing();

        // The sun's depth pre-pass: the opaque world from the sun's point of
        // view, into the shadow map.
        HeronBrushLightingBeginShadowPass(&light);
        DrawOpaque(&level, &movers, &light);
        HeronBrushLightingEndShadowPass(&light);

        ClearBackground((Color){120, 150, 190, 255});

        float now = (float)GetTime();
        SetShaderValue(gWater, gWaterTimeLoc, &now, SHADER_UNIFORM_FLOAT);
        Vector3 camPos = camera.position;
        SetShaderValue(gWater, gWaterCamLoc, &camPos, SHADER_UNIFORM_VEC3);
        HeronBrushLightingApply(&light, camPos);

        BeginMode3D(camera);
        if (showCollision) DrawGrid(64, 1.0f);

        DrawOpaque(&level, &movers, &light); // lit opaque geometry
        HeronBrushGameDrawWorld(&game, &level, camera); // enemies, bolts, markers (unlit)
        for (int i = 0; i < level.entityCount; i++) {
            const HeronBrushEntity *e = &level.entities[i];
            if (gMeshes[i].loaded || e->solid || !showMarkers) continue;
            if (strcmp(e->classname, "info_player_start") == 0 ||
                strncmp(e->classname, "fps_", 4) == 0 ||
                HeronBrushGameOwnsClass(e->classname))
                continue;
            Vector3 p = {e->pos[0], e->pos[1], e->pos[2]};
            Color c = ClassColor(e->classname);
            DrawCube(p, 0.5f, 0.5f, 0.5f, c);
            DrawCubeWires(p, 0.5f, 0.5f, 0.5f, ColorBrightness(c, -0.4f));
        }
        DrawBlended(&level, &movers, &light, camPos); // blended and water, back to front, last

        if (showCollision) HeronBrushColliderDrawWires(&collider);
        EndMode3D();

        if (submerged) DrawUnderwater(GetScreenWidth(), GetScreenHeight(), now);

        if (game.state == HERON_BRUSH_PLAYING) {
            float sp = sqrtf(player.vel.x * player.vel.x + player.vel.z * player.vel.z);
            DrawText(TextFormat("%s  %.1f m/s  %s", player.noclip ? "noclip" : "walk", sp,
                                player.grounded ? "grounded" : "airborne"),
                     12, 12, 16, (Color){180, 190, 210, 220});
        }
        HeronBrushGameDrawEnemyBars(&game, camera, GetScreenWidth(), GetScreenHeight());
        HeronBrushGameDrawHUD(&game, GetScreenWidth(), GetScreenHeight());

        EndDrawing();

        if (demo && shot && frame % 30 == 0) {
            float hsp = sqrtf(player.vel.x * player.vel.x + player.vel.z * player.vel.z);
            fprintf(stderr, "f%-4d hp %.0f  score %ld  pos %.1f %.1f %.1f  speed %.2f\n",
                    frame, game.hp, game.score, player.pos.x, player.pos.y,
                    player.pos.z, hsp);
        }
        if (shot && frame >= shotFrames) {
            TakeScreenshot(shot);
            printf("player %.2f %.2f %.2f  hp %.0f  score %ld  state %d\n", player.pos.x,
                   player.pos.y, player.pos.z, game.hp, game.score, game.state);
            break;
        }
    }

    HeronBrushMoversFree(&movers);
    HeronBrushColliderFree(&collider);
    // The meshes share the shaders and any external textures held elsewhere, so
    // point those materials back at raylib's defaults before UnloadModel, or it
    // would free them out from under everyone. Embedded textures are the
    // model's own and go with it.
    for (int i = 0; i < level.entityCount; i++) {
        EntityMesh *em = &gMeshes[i];
        if (!em->loaded) continue;
        Model *mdl = &em->model;
        for (int s = 0; s < em->surfaceCount; s++)
            if (em->surfaces[s].sharedTexture)
                mdl->materials[mdl->meshMaterial[em->surfaces[s].prim]].maps[MATERIAL_MAP_ALBEDO].texture.id =
                    rlGetTextureIdDefault();
        for (int m = 0; m < mdl->materialCount; m++) mdl->materials[m].shader.id = rlGetShaderIdDefault();
        UnloadModel(*mdl);
        free(em->info);
    }
    for (int i = 0; i < level.textureCount; i++) {
        if (gTextures[i].id != 0) UnloadTexture(gTextures[i]);
    }
    UnloadShader(gWater);
    HeronBrushLightingFree(&light);
    HeronBrushGameUnloadAssets();
    CloseAudioDevice();
    UnloadTexture(gFallback);
    HeronBrushFreeLevel(&level);
    CloseWindow();
    return 0;
}
