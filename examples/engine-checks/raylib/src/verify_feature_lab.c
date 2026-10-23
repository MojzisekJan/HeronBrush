// Feature lab check for the raylib export: the .level reader's collision lines, the
// generated animation, trigger and water headers, and raylib itself loading the GLBs
// and the sky. Build and run from the export folder (see README.md):
//
//   ./verify_feature_lab poses.txt [shot.png]
//
// poses.txt comes from examples/engine-checks/feature_lab_poses.py --text. With a
// shot path it also draws the level, with the sky and the water shader, into a PNG.
#include "collision.h"
#include "level.h"
#include "raylib.h"
#include "rlgl.h"

#include "feature_lab_animation.h"
#define HB_WATER
#include "feature_lab_water.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static HeronBrushLevel level;
static int failures;

static void report(const char *feature, int ok, const char *detail) {
    printf("%s: %s. %s\n", feature, ok ? "PASS" : "FAIL", detail);
    if (!ok) failures++;
}

// The entity whose targetname is `name`, as its index in the level.
static int station(const char *name) {
    for (int i = 0; i < level.entityCount; i++) {
        const char *t = HeronBrushGetProp(&level.entities[i], "targetname");
        if (t && !strcmp(t, name)) return i;
    }
    fprintf(stderr, "no entity %s\n", name);
    exit(1);
}
static int animation_of(int entity) {
    for (int a = 0; a < HB_ANIMATION_COUNT; a++)
        if (hb_animations[a].entity == entity) return a;
    return -1;
}
static float down(const HeronBrushCollider *col, int entity) {
    const float *p = level.entities[entity].pos;
    return 10 - HeronBrushColliderRayHit(col, (Vector3){p[0], 10, p[2]}, (Vector3){0, -1, 0}, 20);
}

static void check_collision(void) {
    char text[256];
    const HeronBrushEntity *skip = &level.entities[station("fl_skip")];
    const HeronBrushEntity *hidden = &level.entities[station("fl_hidden")];
    const HeronBrushEntity *solid = &level.entities[station("fl_solid")];
    const HeronBrushEntity *clip = &level.entities[station("fl_clip")];
    const HeronBrushEntity *trigger = &level.entities[station("fl_trigger")];
    const HeronBrushEntity *water = &level.entities[station("fl_water")];
    snprintf(text, sizeof text, "ctri: fl_skip %d triangles (5 sides), fl_hidden %d (6 sides); chull: fl_solid %d points, fl_clip %d points and no surface",
             skip->collision.triCount, hidden->collision.triCount,
             solid->collision.hullCount ? solid->collision.hulls[0].vertexCount : 0,
             clip->collision.hullCount ? clip->collision.hulls[0].vertexCount : 0);
    report("level reader", skip->collision.triCount == 10 && hidden->collision.triCount == 12 &&
           solid->collision.hullCount == 1 && solid->collision.hulls[0].vertexCount == 8 &&
           clip->collision.hullCount == 1 && clip->surfaceCount == 0, text);
    snprintf(text, sizeof text, "the trigger and the water are read as sensor %s volumes",
             trigger->collision.kind == HERON_BRUSH_COLLISION_CONVEX ? "convex" : "non-convex");
    report("sensor lines", trigger->collision.sensor && water->collision.sensor && !solid->collision.sensor, text);

    HeronBrushCollider col = {0};
    HeronBrushColliderBuild(&col, &level);
    float s = down(&col, station("fl_solid")), c = down(&col, station("fl_clip")), k = down(&col, station("fl_skip"));
    float h = down(&col, station("fl_hidden")), t = down(&col, station("fl_trigger"));
    // A capsule dropped onto fl_solid comes to rest on its top.
    Vector3 centre = {solid->pos[0], 6, solid->pos[2]}, velocity = {0, 0, 0};
    bool grounded = false;
    for (int i = 0; i < 400; i++) {
        centre.y -= 0.02f;
        HeronBrushColliderResolveCapsule(&col, &centre, 0.3f, 0.6f, &velocity, &grounded);
    }
    float feet = centre.y - 0.6f; // the half-height includes the radius
    snprintf(text, sizeof text, "rays down stop at fl_solid y=%.3f, fl_clip y=%.3f, fl_hidden y=%.3f; through fl_skip's top to y=%.3f and through the trigger to the floor y=%.3f; a capsule settles with its feet at y=%.3f",
             s, c, h, k, t, feet);
    report("collision", fabsf(s - 2.25f) < 1e-3f && fabsf(c - 3.25f) < 1e-3f && fabsf(h - 2.25f) < 1e-3f &&
           k < 2.2f && fabsf(t) < 1e-3f && fabsf(feet - 2.25f) < 0.05f, text);
    HeronBrushColliderFree(&col);
}

static void check_animation(const char *poses) {
    FILE *f = fopen(poses, "r");
    if (!f) { perror(poses); exit(1); }
    char name[64];
    double time, rest[3], posed[3], worst = 0;
    int samples = 0;
    while (fscanf(f, "%63s %lf %lf %lf %lf %lf %lf %lf", name, &time, &rest[0], &rest[1], &rest[2],
                  &posed[0], &posed[1], &posed[2]) == 8) {
        int a = animation_of(station(name));
        HbPlayer player;
        HbInit(&player, a);
        if (!strcmp(name, "fl_door")) HbPlay(&player, a, "open");
        HbSeek(&player, time);
        double out[3];
        HbPoint(a, &player, rest, out, 0);
        for (int i = 0; i < 3; i++) worst = fmax(worst, fabs(out[i] - posed[i]));
        samples++;
    }
    fclose(f);
    char text[160];
    snprintf(text, sizeof text, "HbPoint matches %d editor probe points of fl_mover and fl_door within %.1e m", samples, worst);
    report("animation", samples == 76 && worst < 1e-6, text);

    // A player standing in the raised trigger, feet on the floor, opens the door.
    HbPlayer players[HB_ANIMATION_COUNT + 1];
    for (int i = 0; i < HB_ANIMATION_COUNT; i++) HbInit(&players[i], i);
    HbTriggerState states[HB_TRIGGER_COUNT + 1] = {{0}};
    const float *at = level.entities[station("fl_trigger")].pos;
    double feet[3] = {at[0], 0.02, at[2]};
    int door = animation_of(station("fl_door"));
    HbCheckTriggers(states, players, feet, 0);
    int point = players[door].playing;
    HbTriggerState fresh[HB_TRIGGER_COUNT + 1] = {{0}};
    HbCheckTriggersSpan(fresh, players, feet, 1.8, 0);
    int span = players[door].playing && !strcmp(hb_clips[players[door].clip].name, "open");
    for (int i = 0; i < 90; i++) HbUpdate(&players[door], 1 / 60.0);
    snprintf(text, sizeof text, "HbCheckTriggersSpan with the feet on the floor plays %s to %.2f s; the feet point alone %s the 0.25 m raised volume",
             hb_clips[players[door].clip].name, players[door].time, point ? "is inside" : "is below");
    report("trigger opens door", span && !point && fabs(players[door].time - 1) < 1e-9, text);
}

static void check_water(void) {
    const float *at = level.entities[station("fl_water")].pos;
    double centre[3] = {at[0], at[1], at[2]}, above[3] = {at[0], at[1] + 1, at[2]}, depth = 0, dry = 0;
    int in = HbWaterFind(centre, &depth), out = HbWaterFind(above, &dry);
    char text[160];
    snprintf(text, sizeof text, "HbWaterFind: centre %.3f m deep (swim %.1f, priority %d), above the surface %d", depth,
             in >= 0 ? hb_waters[in].swim_speed : 0, in >= 0 ? hb_waters[in].priority : 0, out);
    report("water header", in == 0 && fabs(depth - 0.75) < 1e-9 && out == -1 && hb_waters[0].swim_speed == 2.5 &&
           hb_waters[0].priority == 3, text);
}

// raylib on the exported files: LoadModel on each GLB, the sky cube, and a picture.
static void check_gpu(const char *shot) {
    SetConfigFlags(FLAG_WINDOW_HIDDEN);
    SetTraceLogLevel(LOG_WARNING);
    InitWindow(1280, 720, "feature lab");
    if (!IsWindowReady()) { report("raylib window", 0, "no OpenGL context"); return; }
    Model models[HERON_BRUSH_MAX_ENTITIES] = {0};
    char text[256];
    int textured = 0, materials = 0;
    for (int i = 0; i < level.entityCount; i++) {
        if (!level.entities[i].mesh[0]) continue;
        models[i] = LoadModel(level.entities[i].mesh);
    }
    // Multi-material GLBs: every material of fl_textures has its own image.
    Model *t = &models[station("fl_textures")];
    for (int m = 1; m < t->materialCount; m++) {
        materials++;
        textured += t->materials[m].maps[MATERIAL_MAP_DIFFUSE].texture.id != rlGetTextureIdDefault() &&
                    t->materials[m].maps[MATERIAL_MAP_DIFFUSE].texture.width == 64;
    }
    snprintf(text, sizeof text, "LoadModel on fl_textures: %d meshes, %d materials, %d with their 64 x 64 image", t->meshCount, materials, textured);
    report("multi-material GLB", t->meshCount == 7 && materials == 7 && textured == 7, text);
    // The five opaque colours come back from the GPU exactly.
    static const unsigned char want[5][3] = {{220, 30, 40}, {30, 200, 60}, {40, 60, 230}, {240, 220, 30}, {255, 255, 255}};
    Model *c = &models[station("fl_colours")];
    int found = 0;
    for (int w = 0; w < 5; w++)
        for (int m = 1; m < c->materialCount; m++) {
            Texture2D tex = c->materials[m].maps[MATERIAL_MAP_DIFFUSE].texture;
            if (tex.width != 1) continue;
            Image image = LoadImageFromTexture(tex);
            Color p = GetImageColor(image, 0, 0);
            UnloadImage(image);
            if (p.r == want[w][0] && p.g == want[w][1] && p.b == want[w][2]) { found++; break; }
        }
    snprintf(text, sizeof text, "%d of 5 colours read back exactly from their 1 x 1 textures", found);
    report("colours", found == 5, text);
    Model *n = &models[station("fl_normal")];
    Texture2D normal = n->materials[1].maps[MATERIAL_MAP_NORMAL].texture;
    snprintf(text, sizeof text, "normal map %d x %d, mesh tangents %s; raylib's default shader does not use them", normal.width, normal.height,
             n->meshes[0].tangents ? "loaded" : "missing");
    report("normal map", normal.width == 512 && n->meshes[0].tangents, text);
    Model *e = &models[station("fl_surface")];
    int emissive = 0;
    for (int m = 1; m < e->materialCount; m++) emissive += e->materials[m].maps[MATERIAL_MAP_EMISSION].texture.id > 0;
    snprintf(text, sizeof text, "%d material with an emission map; raylib reads no emissive strength and its default shader draws no emission", emissive);
    report("emission", emissive == 1, text);
    Model *pbr = &models[station("fl_model_a")];
    int maps = 0;
    for (int m = 1; m < pbr->materialCount; m++)
        for (int slot = MATERIAL_MAP_METALNESS; slot <= MATERIAL_MAP_EMISSION; slot++)
            maps += pbr->materials[m].maps[slot].texture.id > 0;
    snprintf(text, sizeof text, "model: %d materials, %d metal-roughness, normal, emission and occlusion maps loaded", pbr->materialCount - 1, maps);
    report("model PBR", pbr->materialCount == 3 && maps >= 8, text);
    int drawn = 0;
    for (const char *name = "fl_clip"; name; name = strcmp(name, "fl_clip") ? NULL : "fl_trigger")
        drawn += models[station(name)].meshCount;
    snprintf(text, sizeof text, "the clip brush's and the trigger's collision-only GLBs give %d meshes to draw", drawn);
    report("nothing drawn", drawn == 0, text);

    Image strip = LoadImage("feature_lab/sky.png");
    TextureCubemap sky = LoadTextureCubemap(strip, CUBEMAP_LAYOUT_LINE_VERTICAL);
    Color top = GetImageColor(strip, strip.width / 2, strip.width * 2 + strip.width / 2);
    snprintf(text, sizeof text, "sky.png %d x %d makes a %d x %d cube; +Y face centre %d %d %d", strip.width, strip.height, sky.width, sky.height, top.r, top.g, top.b);
    report("sky cube", sky.id && sky.width == strip.width && strip.height == strip.width * 6 && top.b > top.r, text);
    UnloadImage(strip);

    if (shot) {
        Model cube = LoadModelFromMesh(GenMeshCube(1, 1, 1));
        cube.materials[0].shader = LoadShaderFromMemory(
            "#version 330\nin vec3 vertexPosition;uniform mat4 matProjection;uniform mat4 matView;out vec3 direction;\n"
            "void main(){direction=vertexPosition;gl_Position=matProjection*mat4(mat3(matView))*vec4(vertexPosition,1.0);}\n",
            "#version 330\nin vec3 direction;uniform samplerCube environmentMap;out vec4 finalColor;\n"
            "void main(){finalColor=vec4(texture(environmentMap,direction).rgb,1.0);}\n");
        int unit = MATERIAL_MAP_CUBEMAP;
        SetShaderValue(cube.materials[0].shader, GetShaderLocation(cube.materials[0].shader, "environmentMap"), &unit, SHADER_UNIFORM_INT);
        cube.materials[0].maps[MATERIAL_MAP_CUBEMAP].texture = sky;
        HbWaterShader water = HbWaterLoadShader();
        Camera3D camera = {{-20, 16, -8}, {2, 0, -9}, {0, 1, 0}, 55, CAMERA_PERSPECTIVE};
        RenderTexture2D target = LoadRenderTexture(1280, 720);
        BeginTextureMode(target);
        HbWaterView(&water, camera.position, 0);
        ClearBackground(DARKGRAY);
        BeginMode3D(camera);
        rlDisableBackfaceCulling();
        rlDisableDepthMask();
        DrawModel(cube, camera.position, 1, WHITE);
        rlEnableDepthMask();
        rlEnableBackfaceCulling();
        for (int pass = 0; pass < 2; pass++)
            for (int i = 0; i < level.entityCount; i++) {
                if (models[i].meshCount <= 0) continue;
                models[i].transform = HeronBrushEntityMatrix(&level.entities[i]);
                int w = HbWaterOf(i);
                if ((w >= 0 && hb_waters[w].surface_material) != (pass == 1)) continue;
                HbWaterDrawModel(&water, models[i], w);
            }
        EndMode3D();
        EndTextureMode();
        Image image = LoadImageFromTexture(target.texture);
        ImageFlipVertical(&image);
        ExportImage(image, shot);
        UnloadImage(image);
        printf("wrote %s\n", shot);
    }
    CloseWindow();
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s poses.txt [shot.png], from the export folder\n", argv[0]);
        return 2;
    }
    if (!HeronBrushLoadLevel("feature_lab.level", &level)) return 1;
    check_collision();
    check_animation(argv[1]);
    check_water();
    check_gpu(argc > 2 ? argv[2] : NULL);
    printf(failures ? "FEATURE_LAB raylib FAIL\n" : "FEATURE_LAB raylib PASS\n");
    return failures ? 1 : 0;
}
