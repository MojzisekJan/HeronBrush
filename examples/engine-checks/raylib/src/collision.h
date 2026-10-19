// Collision for the FPS sample: a world-space triangle soup for the static
// world, plus exact convex hulls for every convex solid (movers included),
// and the small amount of maths an FPS controller needs against them: push a
// capsule out, measure the drop to the floor, cast a ray.
//
// The exporter already did the expensive part. `ctri` lines are the reduced
// concave collision mesh (buried faces gone, neighbours joined); `chull`
// records are the exact vertex cloud of each convex piece. Both are
// entity-local and go through the entity's full transform (scale, then roll,
// pitch, yaw in YXZ order, then translation), the same matrix the renderer
// draws the entity with. `cbox` fallback boxes are used only for an entity
// that has no hulls (an export from before hulls existed). The arena is a few
// hundred triangles and a few dozen hulls, so tests are brute force.

#ifndef HERON_BRUSH_COLLISION_H
#define HERON_BRUSH_COLLISION_H

#include "level.h"
#include "raylib.h"

#include <stdbool.h>

typedef struct {
    Vector3 a, b, c;     // vertices, world space
    Vector3 normal;      // unit face normal
    Vector3 bmin, bmax;  // triangle bounding box, for the broad-phase reject
} HeronBrushTri;

// One convex piece. The face list is built once from the local points; the
// world-space copy is refreshed whenever the owning entity's transform moves.
typedef struct {
    int entIndex;
    int vertCount;
    Vector3 *local;      // entity-local points
    Vector3 *world;      // the same points, world space
    int faceCount;
    int *faceVerts;      // 3 vertex indices per triangular face
    Vector4 *planes;     // world-space face planes: xyz outward unit normal, w = offset
    Vector3 bmin, bmax;  // world bounds
} HeronBrushHullShape;

typedef struct {
    HeronBrushTri *tris;        // static concave geometry
    int count;
    int cap;
    HeronBrushHullShape *hulls; // every convex piece, static and moving
    int hullCount;
    int *entHullFirst;   // per entity: first hull, and how many
    int *entHullCount;
    Vector3 min, max;    // whole-world bounds
} HeronBrushCollider;

// The entity's full transform: scale, then rotation `rot` = [pitch, yaw, roll]
// applied in YXZ order (roll about Z first, then pitch about X, then yaw about
// Y), then translation. The same matrix places the mesh and the collision.
Matrix HeronBrushEntityMatrix(const HeronBrushEntity *e);

// Builds the collision world. Every solid, non-sensor entity contributes: its
// `ctri` triangles to the static soup unless it is a mover, and each exact
// hull (or, lacking hulls, each `cbox`) as a convex piece. Movers then update
// their pieces with `HeronBrushColliderSetEntityTransform`.
void HeronBrushColliderBuild(HeronBrushCollider *col, const HeronBrushLevel *level);
void HeronBrushColliderFree(HeronBrushCollider *col);

// Moves every convex piece of entity `entIndex` to the world transform `m`.
void HeronBrushColliderSetEntityTransform(HeronBrushCollider *col, int entIndex, Matrix m);

// World-space bounds of entity `entIndex`'s convex pieces as last placed.
// False when it has none.
bool HeronBrushColliderEntityBounds(const HeronBrushCollider *col, int entIndex, Vector3 *min,
                             Vector3 *max);

// World-space bounding box of an entity's collision geometry (`ctri`, hulls,
// `cbox`) at its exported transform. False when the entity has none.
bool HeronBrushEntityWorldAABB(const HeronBrushEntity *e, Vector3 *min, Vector3 *max);

// Pushes a vertical capsule (radius `r`, half-height `h`, centred at `*centre`)
// out of anything it overlaps. The component of `*vel` heading into a surface
// is removed, so the caller slides along walls for free. `*grounded` is set
// when the capsule rests on something close to level.
void HeronBrushColliderResolveCapsule(const HeronBrushCollider *col, Vector3 *centre, float r,
                               float h, Vector3 *vel, bool *grounded);

// Distance straight down from `from` to the nearest upward-facing surface
// within `maxDist`, or -1 when there is none. For ground snapping and steps.
float HeronBrushColliderFloorBelow(const HeronBrushCollider *col, Vector3 from, float maxDist);

// Distance along `dir` (need not be unit) from `origin` to the nearest
// surface within `maxDist` metres, or -1 when the ray hits nothing. For
// line-of-sight and for stopping shots at walls.
float HeronBrushColliderRayHit(const HeronBrushCollider *col, Vector3 origin, Vector3 dir,
                        float maxDist);

// Draws the static triangles and the hull edges as lines, inside BeginMode3D.
void HeronBrushColliderDrawWires(const HeronBrushCollider *col);

#endif
