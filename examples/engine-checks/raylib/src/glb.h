// The glTF material data raylib's loader leaves behind.
//
// raylib 5.5 and 6.0 load the base colour, normal and emissive textures of a
// .glb, but they read the metallic and roughness factors only when there is a
// metallic-roughness texture, the emissive factor only when there is an
// emissive texture, and never alphaMode, doubleSided, the texture samplers or
// KHR_materials_emissive_strength. The exporter writes factors without
// textures, so this reads the JSON chunk of the .glb itself, with a small
// tokenizer and no raylib dependency.

#ifndef HERON_BRUSH_GLB_H
#define HERON_BRUSH_GLB_H

#include <stdbool.h>

#define HERON_BRUSH_GLB_MAX_MATERIALS 64

typedef enum { HERON_BRUSH_ALPHA_OPAQUE, HERON_BRUSH_ALPHA_MASK, HERON_BRUSH_ALPHA_BLEND } HeronBrushAlphaMode;

typedef struct {
    char name[96];
    float baseColor[4];  // linear RGBA factor, default 1 1 1 1
    float metallic;      // default 1, as glTF specifies
    float roughness;     // default 1
    float emissive[3];   // linear, already multiplied by emissiveStrength
    HeronBrushAlphaMode alphaMode;
    float alphaCutoff;   // default 0.5, used by MASK
    bool doubleSided;
    bool hasBaseTexture;
    bool baseClamp;      // the base colour sampler clamps (generated face paint)
    bool hasEmissiveTexture;
    bool hasNormalTexture;
    float normalScale;   // default 1
} HeronBrushGlbMaterial;

typedef struct {
    HeronBrushGlbMaterial materials[HERON_BRUSH_GLB_MAX_MATERIALS];
    int materialCount;
    // Primitive p of glTF mesh 0 wears materials[primitiveMaterial[p]], or -1.
    int primitiveMaterial[HERON_BRUSH_GLB_MAX_MATERIALS * 4];
    int primitiveCount;
    bool primitiveHasTangent[HERON_BRUSH_GLB_MAX_MATERIALS * 4];
} HeronBrushGlbInfo;

// Reads the material table and mesh 0's primitive list of the .glb at `path`.
// Returns false (and prints to stderr) when the file is not a readable .glb.
bool HeronBrushReadGlbInfo(const char *path, HeronBrushGlbInfo *out);

#endif
