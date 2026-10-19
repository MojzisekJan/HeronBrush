#include "lighting.h"

#include "raymath.h"
#include "rlgl.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

// The editor's viewport defaults (Preferences: Sun elevation 46, Sun azimuth
// 233 counter-clockwise from map +X, sun colour and intensity 3.2, sky,
// horizon and ground at ambient intensity 0.45, exposure 1).
#define SUN_ELEVATION 46.0f
#define SUN_AZIMUTH 233.0f
#define SUN_INTENSITY 3.2f
#define AMBIENT_INTENSITY 0.45f

static Vector3 parse_rgb(const char *s, Vector3 fallback) {
    if (!s) return fallback;
    Vector3 v = fallback;
    // Both "0..1 0..1 0..1" and "0..255 0..255 0..255" turn up in maps.
    if (sscanf(s, "%f %f %f", &v.x, &v.y, &v.z) != 3) return fallback;
    if (v.x > 1.5f || v.y > 1.5f || v.z > 1.5f) v = Vector3Scale(v, 1.0f / 255.0f);
    return v;
}

// The sun's travel direction from a light_sun's `rot` (Godot order, radians):
// the entity's forward, flipped down if it points at the sky.
static Vector3 sun_from_rot(const float rot[3]) {
    float yaw = rot[1], pitch = rot[0];
    Vector3 f = {-sinf(yaw) * cosf(pitch), sinf(pitch), -cosf(yaw) * cosf(pitch)};
    if (f.y > 0.0f) f = Vector3Negate(f);
    float len = Vector3Length(f);
    return len > 1e-4f ? Vector3Scale(f, 1.0f / len)
                       : (Vector3){-0.35f, -0.85f, -0.4f};
}

static void parse_lights(HeronBrushLighting *lt, const HeronBrushLevel *level, bool useMapSun) {
    // Map axes (x, y, z) are engine axes (z, x, y), so the editor's direction
    // towards the sun, (cos e cos a, cos e sin a, sin e) in the map, is this.
    float e = SUN_ELEVATION * DEG2RAD, a = SUN_AZIMUTH * DEG2RAD;
    Vector3 toSun = {cosf(e) * sinf(a), sinf(e), cosf(e) * cosf(a)};
    lt->sunDir = Vector3Negate(Vector3Normalize(toSun));
    lt->sunColor = Vector3Scale((Vector3){1.0f, 0.9375f, 0.84375f}, SUN_INTENSITY);
    lt->sky = Vector3Scale((Vector3){0.10980392f, 0.44313726f, 0.84705883f}, AMBIENT_INTENSITY);
    lt->horizon = Vector3Scale((Vector3){0.5254902f, 0.36862746f, 0.23529412f}, AMBIENT_INTENSITY);
    lt->ground = Vector3Scale((Vector3){0.10f, 0.09f, 0.08f}, AMBIENT_INTENSITY);
    lt->exposure = 1.0f;
    lt->pointCount = 0;

    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *ent = &level->entities[i];
        const char *c = ent->classname;

        if (useMapSun && strcmp(c, "worldspawn") == 0 && HeronBrushGetProp(ent, "ambient_light_color")) {
            Vector3 amb = parse_rgb(HeronBrushGetProp(ent, "ambient_light_color"), lt->sky);
            amb = Vector3Scale(amb, HeronBrushPropFloat(ent, "ambient_light_energy", 1.0f) * AMBIENT_INTENSITY);
            lt->sky = lt->horizon = amb;
            lt->ground = Vector3Scale(amb, 0.25f);
        } else if (useMapSun && strcmp(c, "light_sun") == 0) {
            lt->sunDir = sun_from_rot(ent->rot);
            Vector3 col = parse_rgb(HeronBrushGetProp(ent, "light_color"), (Vector3){1, 1, 1});
            lt->sunColor = Vector3Scale(col, HeronBrushPropFloat(ent, "light_energy", 1.0f) * SUN_INTENSITY);
        } else if (strcmp(c, "light") == 0 && lt->pointCount < HERON_BRUSH_MAX_POINT_LIGHTS) {
            int k = lt->pointCount++;
            lt->pointPos[k] = (Vector3){ent->pos[0], ent->pos[1], ent->pos[2]};
            Vector3 col = parse_rgb(HeronBrushGetProp(ent, "light_color"),
                                    (Vector3){1.0f, 0.95f, 0.85f});
            // Watts-ish energy to a radiance that reads beside the sun.
            lt->pointColor[k] = Vector3Scale(col, HeronBrushPropFloat(ent, "light_energy", 1.0f) * 4.0f);
            lt->pointRange[k] = HeronBrushPropFloat(ent, "range", HeronBrushPropFloat(ent, "light", 8.0f));
        }
    }
}

static RenderTexture2D make_shadowmap(int res) {
    RenderTexture2D t = {0};
    t.id = rlLoadFramebuffer();
    t.texture.width = res;
    t.texture.height = res;
    if (t.id == 0) return t;

    rlEnableFramebuffer(t.id);
    t.depth.id = rlLoadTextureDepth(res, res, false);
    t.depth.width = res;
    t.depth.height = res;
    t.depth.format = 19; // DEPTH_COMPONENT_24BIT, matches raylib's own helper
    t.depth.mipmaps = 1;
    rlFramebufferAttach(t.id, t.depth.id, RL_ATTACHMENT_DEPTH, RL_ATTACHMENT_TEXTURE2D, 0);
    bool ok = rlFramebufferComplete(t.id);
    rlDisableFramebuffer();
    if (!ok) {
        rlUnloadFramebuffer(t.id);
        t.id = 0;
    }
    return t;
}

bool HeronBrushLightingInit(HeronBrushLighting *lt, const HeronBrushLevel *level, Vector3 sceneMin,
                     Vector3 sceneMax, bool useMapSun) {
    memset(lt, 0, sizeof(*lt));
    parse_lights(lt, level, useMapSun);

    lt->sceneCenter = Vector3Scale(Vector3Add(sceneMin, sceneMax), 0.5f);
    lt->sceneRadius = fmaxf(Vector3Length(Vector3Subtract(sceneMax, sceneMin)) * 0.5f, 4.0f);

    lt->shader = LoadShader("assets/shaders/" HERON_BRUSH_GLSL "/pbr.vs",
                            "assets/shaders/" HERON_BRUSH_GLSL "/pbr.fs");
    if (!IsShaderValid(lt->shader) || lt->shader.id == rlGetShaderIdDefault()) {
        TraceLog(LOG_ERROR, "PBR shader did not compile");
        return false;
    }
    Shader *s = &lt->shader;
    // raylib binds MATERIAL_MAP_ALBEDO to the location in locs[SHADER_LOC_MAP_ALBEDO]
    // ("texture0" by default); point the normal and emission slots at ours.
    s->locs[SHADER_LOC_MAP_NORMAL] = GetShaderLocation(*s, "normalMap");
    s->locs[SHADER_LOC_MAP_EMISSION] = GetShaderLocation(*s, "emissiveMap");
    s->locs[SHADER_LOC_MAP_METALNESS] = -1; // texture1: not used by this shader
    lt->locViewPos = GetShaderLocation(*s, "uViewPos");
    lt->locSunDir = GetShaderLocation(*s, "uSunDir");
    lt->locSunColor = GetShaderLocation(*s, "uSunColor");
    lt->locSky = GetShaderLocation(*s, "uSky");
    lt->locHorizon = GetShaderLocation(*s, "uHorizon");
    lt->locGround = GetShaderLocation(*s, "uGround");
    lt->locExposure = GetShaderLocation(*s, "uExposure");
    lt->locLightVP = GetShaderLocation(*s, "uLightVP");
    lt->locShadowMap = GetShaderLocation(*s, "uShadowMap");
    lt->locShadowRes = GetShaderLocation(*s, "uShadowRes");
    lt->locHasShadow = GetShaderLocation(*s, "uHasShadow");
    lt->locPointCount = GetShaderLocation(*s, "uPointCount");
    lt->locPointPos = GetShaderLocation(*s, "uPointPos");
    lt->locPointColor = GetShaderLocation(*s, "uPointColor");
    lt->locPointRange = GetShaderLocation(*s, "uPointRange");
    lt->locBaseColor = GetShaderLocation(*s, "uBaseColor");
    lt->locMetalRough = GetShaderLocation(*s, "uMetalRough");
    lt->locEmissive = GetShaderLocation(*s, "uEmissive");
    lt->locMaterial = GetShaderLocation(*s, "uMaterial");

    // The point lights never move, so upload them once.
    SetShaderValue(*s, lt->locPointCount, &lt->pointCount, SHADER_UNIFORM_INT);
    if (lt->pointCount > 0) {
        SetShaderValueV(*s, lt->locPointPos, lt->pointPos, SHADER_UNIFORM_VEC3, lt->pointCount);
        SetShaderValueV(*s, lt->locPointColor, lt->pointColor, SHADER_UNIFORM_VEC3, lt->pointCount);
        SetShaderValueV(*s, lt->locPointRange, lt->pointRange, SHADER_UNIFORM_FLOAT, lt->pointCount);
    }

    lt->shadowRes = 2048;
    lt->shadowMap = make_shadowmap(lt->shadowRes);
    lt->shadows = lt->shadowMap.id != 0;
    if (lt->shadows) {
        SetTextureWrap(lt->shadowMap.depth, TEXTURE_WRAP_CLAMP);
        float res = (float)lt->shadowRes;
        SetShaderValue(*s, lt->locShadowRes, &res, SHADER_UNIFORM_FLOAT);
    }
    int has = lt->shadows ? 1 : 0;
    SetShaderValue(*s, lt->locHasShadow, &has, SHADER_UNIFORM_INT);
    return true;
}

void HeronBrushLightingFree(HeronBrushLighting *lt) {
    UnloadShader(lt->shader);
    if (lt->shadowMap.id != 0) {
        rlUnloadTexture(lt->shadowMap.depth.id);
        rlUnloadFramebuffer(lt->shadowMap.id);
    }
}

void HeronBrushLightingBeginShadowPass(HeronBrushLighting *lt) {
    if (!lt->shadows) return;

    Vector3 up = fabsf(lt->sunDir.y) > 0.98f ? (Vector3){0.0f, 0.0f, 1.0f}
                                             : (Vector3){0.0f, 1.0f, 0.0f};
    Camera3D lc = {0};
    lc.position =
        Vector3Subtract(lt->sceneCenter, Vector3Scale(lt->sunDir, lt->sceneRadius * 1.6f));
    lc.target = lt->sceneCenter;
    lc.up = up;
    lc.projection = CAMERA_ORTHOGRAPHIC;
    lc.fovy = lt->sceneRadius * 2.0f;

    BeginTextureMode(lt->shadowMap);
    ClearBackground(WHITE);
    BeginMode3D(lc);

    // A tight ortho box around the scene sphere: better depth precision than
    // raylib's default 0.01..1000 range, so the bias can stay small.
    float r = lt->sceneRadius * 1.12f;
    Matrix proj = MatrixOrtho(-r, r, -r, r, 0.05f, lt->sceneRadius * 3.4f);
    rlSetMatrixProjection(proj);
    lt->lightVP = MatrixMultiply(rlGetMatrixModelview(), proj);

    // Render back faces into the depth map so the acne lands on faces that are
    // in shadow anyway.
    rlSetCullFace(RL_CULL_FACE_FRONT);
}

void HeronBrushLightingEndShadowPass(HeronBrushLighting *lt) {
    if (!lt->shadows) return;
    rlSetCullFace(RL_CULL_FACE_BACK);
    EndMode3D();
    EndTextureMode();
}

void HeronBrushLightingApply(HeronBrushLighting *lt, Vector3 viewPos) {
    Shader s = lt->shader;
    SetShaderValue(s, lt->locViewPos, &viewPos, SHADER_UNIFORM_VEC3);
    SetShaderValue(s, lt->locSunDir, &lt->sunDir, SHADER_UNIFORM_VEC3);
    SetShaderValue(s, lt->locSunColor, &lt->sunColor, SHADER_UNIFORM_VEC3);
    SetShaderValue(s, lt->locSky, &lt->sky, SHADER_UNIFORM_VEC3);
    SetShaderValue(s, lt->locHorizon, &lt->horizon, SHADER_UNIFORM_VEC3);
    SetShaderValue(s, lt->locGround, &lt->ground, SHADER_UNIFORM_VEC3);
    SetShaderValue(s, lt->locExposure, &lt->exposure, SHADER_UNIFORM_FLOAT);

    if (!lt->shadows) return;
    SetShaderValueMatrix(s, lt->locLightVP, lt->lightVP);

    // Bind the depth texture to a high slot the material maps do not use.
    int slot = 10;
    rlEnableShader(s.id);
    rlActiveTextureSlot(slot);
    rlEnableTexture(lt->shadowMap.depth.id);
    rlSetUniform(lt->locShadowMap, &slot, SHADER_UNIFORM_INT, 1);
    rlDisableShader();
}

void HeronBrushLightingSetMaterial(const HeronBrushLighting *lt, const HeronBrushGlbMaterial *m,
                            bool hasNormal, bool hasEmissive) {
    static const HeronBrushGlbMaterial fallback = {
        .baseColor = {1, 1, 1, 1}, .metallic = 0, .roughness = 1, .alphaCutoff = 0.5f,
    };
    if (!m) m = &fallback;
    float metalRough[2] = {m->metallic, m->roughness};
    float material[4] = {
        hasNormal && m->hasNormalTexture ? fmaxf(m->normalScale, 1e-3f) : 0.0f,
        (float)m->alphaMode, m->alphaCutoff,
        hasEmissive && m->hasEmissiveTexture ? 1.0f : 0.0f,
    };
    SetShaderValue(lt->shader, lt->locBaseColor, m->baseColor, SHADER_UNIFORM_VEC4);
    SetShaderValue(lt->shader, lt->locMetalRough, metalRough, SHADER_UNIFORM_VEC2);
    SetShaderValue(lt->shader, lt->locEmissive, m->emissive, SHADER_UNIFORM_VEC3);
    SetShaderValue(lt->shader, lt->locMaterial, material, SHADER_UNIFORM_VEC4);
}
