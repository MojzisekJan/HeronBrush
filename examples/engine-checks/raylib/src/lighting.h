// Physically based forward lighting for the world geometry: the exported glTF
// metallic-roughness materials under one directional sun with a shadow map, a
// sky/horizon/ground ambient and up to four point lights from the map's
// `light` entities.
//
// raylib's stock material is unlit, so the demo carries its own shader
// (assets/shaders/glsl330, glsl300es and glsl100, picked at build time like the
// raylib examples do) plus a depth pre-pass from the sun's point of view. The
// default sun, sky and exposure follow the editor's viewport defaults, so a map
// looks roughly the same in both; `useMapSun` takes the level's `light_sun` and
// worldspawn ambient instead.

#ifndef HERON_BRUSH_LIGHTING_H
#define HERON_BRUSH_LIGHTING_H

#include "glb.h"
#include "level.h"
#include "raylib.h"

#include <stdbool.h>

#define HERON_BRUSH_MAX_POINT_LIGHTS 4

// The shader dialect directory under assets/shaders, chosen like the raylib
// examples choose it: desktop GL 3.3 unless the build targets OpenGL ES 3.0 or
// 2.0 (define GRAPHICS_API_OPENGL_ES3 / ES2 as for raylib itself).
#if defined(GRAPHICS_API_OPENGL_ES3)
#define HERON_BRUSH_GLSL "glsl300es"
#elif defined(GRAPHICS_API_OPENGL_ES2)
#define HERON_BRUSH_GLSL "glsl100"
#else
#define HERON_BRUSH_GLSL "glsl330"
#endif

typedef struct {
    Shader shader; // assign this to every world material

    Vector3 sunDir;   // unit, the way the light travels
    Vector3 sunColor; // linear, intensity applied
    Vector3 sky, horizon, ground; // ambient radiance, linear, intensity applied
    float exposure;

    int pointCount;
    Vector3 pointPos[HERON_BRUSH_MAX_POINT_LIGHTS];
    Vector3 pointColor[HERON_BRUSH_MAX_POINT_LIGHTS];
    float pointRange[HERON_BRUSH_MAX_POINT_LIGHTS];

    RenderTexture2D shadowMap;
    bool shadows;
    int shadowRes;
    Matrix lightVP;

    Vector3 sceneCenter;
    float sceneRadius;

    int locViewPos, locSunDir, locSunColor, locSky, locHorizon, locGround, locExposure;
    int locLightVP, locShadowMap, locShadowRes, locHasShadow;
    int locPointCount, locPointPos, locPointColor, locPointRange;
    int locBaseColor, locMetalRough, locEmissive, locMaterial;
} HeronBrushLighting;

// Reads the lights from `level` and builds the shader and the shadow map.
// `sceneMin`/`sceneMax` bound the visible geometry, for framing the sun's
// depth pass. Returns false when the shader files cannot be compiled.
bool HeronBrushLightingInit(HeronBrushLighting *lt, const HeronBrushLevel *level, Vector3 sceneMin,
                     Vector3 sceneMax, bool useMapSun);
void HeronBrushLightingFree(HeronBrushLighting *lt);

// The sun's depth pre-pass. Between these two, draw the opaque shadow casters
// (the same meshes as the main pass, no blended surfaces, no markers). Does
// nothing at all when the shadow map could not be created.
void HeronBrushLightingBeginShadowPass(HeronBrushLighting *lt);
void HeronBrushLightingEndShadowPass(HeronBrushLighting *lt);

// Upload the per-frame uniforms and bind the shadow map, just before the main
// 3D pass.
void HeronBrushLightingApply(HeronBrushLighting *lt, Vector3 viewPos);

// Upload one glTF material's factors before drawing a primitive wearing it.
// `hasNormal` and `hasEmissive` say whether its raylib material actually holds
// those textures.
void HeronBrushLightingSetMaterial(const HeronBrushLighting *lt, const HeronBrushGlbMaterial *m,
                            bool hasNormal, bool hasEmissive);

#endif
