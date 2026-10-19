// Reader for the flat-text level file `heron_brush export --engine raylib` writes.
//
// The format is line oriented: one keyword per line, whitespace separated,
// `#` and blank lines ignored. An `entity` line opens a record and `end`
// closes it. See docs/level-format.md for what the fields mean; this reader
// keeps them as written and leaves the meaning to main.c.

#ifndef HERON_BRUSH_LEVEL_H
#define HERON_BRUSH_LEVEL_H

#include <stdbool.h>

// Fixed caps, so the whole level is one flat allocation. Raise them if a
// bigger map needs it; a `HeronBrushLevel` is a few megabytes at these sizes and
// belongs in static or heap storage, not on the stack.
#define HERON_BRUSH_MAX_TEXTURES 192
#define HERON_BRUSH_MAX_ENTITIES 512
#define HERON_BRUSH_MAX_SURFACES 32
#define HERON_BRUSH_MAX_PROPS    48
#define HERON_BRUSH_MAX_TAGS     12
#define HERON_BRUSH_MAX_TARGETS  16
#define HERON_BRUSH_MAX_NAME     96
#define HERON_BRUSH_MAX_VALUE    192

typedef struct {
    char name[HERON_BRUSH_MAX_NAME];
    char image[HERON_BRUSH_MAX_VALUE]; // "" when the level wrote "-" (no image on disk)
    float width, height;
} HeronBrushTexture;

typedef struct {
    int primitive;
    char texture[HERON_BRUSH_MAX_NAME];
} HeronBrushSurface;

typedef enum { HERON_BRUSH_COLLISION_NONE, HERON_BRUSH_COLLISION_TRIMESH, HERON_BRUSH_COLLISION_CONVEX } HeronBrushCollisionKind;

typedef struct {
    float *vertices; // exact entity-local xyz point cloud
    int vertexCount;
    int expectedCount;
} HeronBrushHull;

typedef struct {
    HeronBrushCollisionKind kind;
    int mesh;  // glTF mesh index inside the .glb
    int parts; // primitives the shape spreads over
    bool sensor; // a trigger volume: reports overlaps, does not block movement

    // Inline geometry from the `ctri` and `cbox` lines, entity-local metres.
    // Add the entity's `pos` for world space. malloc'd, freed by HeronBrushFreeLevel.
    float *tris;   // 9 floats per triangle (3 points)
    int triCount;
    float *boxes;  // 6 floats per box (min xyz, max xyz)
    int boxCount;
    HeronBrushHull *hulls;
    int hullCount;
    // Physics engines supporting convex point clouds should use hulls instead
    // of the matching boxes. This FPS sample explicitly uses only boxes.
} HeronBrushCollision;

typedef struct {
    char type[16]; // bool int float string vec2 vec3 color array
    char key[HERON_BRUSH_MAX_NAME];
    char value[HERON_BRUSH_MAX_VALUE]; // the rest of the line, verbatim
} HeronBrushProp;

typedef struct {
    char classname[HERON_BRUSH_MAX_NAME];
    char name[HERON_BRUSH_MAX_NAME];
    bool solid;
    char mesh[HERON_BRUSH_MAX_VALUE]; // "" when the entity draws nothing

    float pos[3];
    float rot[3];   // radians, applied Y then X then Z
    float scale[3];

    HeronBrushSurface surfaces[HERON_BRUSH_MAX_SURFACES];
    int surfaceCount;

    HeronBrushCollision collision;

    char tags[HERON_BRUSH_MAX_TAGS][HERON_BRUSH_MAX_NAME];
    int tagCount;

    // Resolved names of the entities this one's `target` points at.
    char targets[HERON_BRUSH_MAX_TARGETS][HERON_BRUSH_MAX_NAME];
    int targetCount;

    HeronBrushProp props[HERON_BRUSH_MAX_PROPS];
    int propCount;
} HeronBrushEntity;

typedef struct {
    int format;
    char name[HERON_BRUSH_MAX_NAME];
    float scale; // map units per metre, already applied to positions

    HeronBrushTexture textures[HERON_BRUSH_MAX_TEXTURES];
    int textureCount;

    HeronBrushEntity entities[HERON_BRUSH_MAX_ENTITIES];
    int entityCount;

} HeronBrushLevel;

// Reads the file at `path` into `out` (a few megabytes: use static or heap
// storage). Returns false and prints a line to stderr on failure.
bool HeronBrushLoadLevel(const char *path, HeronBrushLevel *out);

// Frees the collision geometry HeronBrushLoadLevel malloc'd. The HeronBrushLevel itself is
// caller-owned storage and is not freed.
void HeronBrushFreeLevel(HeronBrushLevel *level);

// The value of `key` on `entity`, or NULL if it has no such property.
const char *HeronBrushGetProp(const HeronBrushEntity *entity, const char *key);

// The value of `key` parsed as a float, or `fallback` if absent or unparsable.
float HeronBrushPropFloat(const HeronBrushEntity *entity, const char *key, float fallback);

// True when `entity` carries the tag `tag`.
bool HeronBrushHasTag(const HeronBrushEntity *entity, const char *tag);

// The entity named `name`, or NULL. Used to follow `target` links.
const HeronBrushEntity *HeronBrushFindEntity(const HeronBrushLevel *level, const char *name);

// True for classnames the demo animates: func_door, fps_key_door, fps_lift.
bool HeronBrushIsMoverClass(const char *classname);

#endif
