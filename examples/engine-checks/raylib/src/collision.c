#include "collision.h"

#include "animated.h"

#include "raymath.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

static Vector3 sub(Vector3 a, Vector3 b) { return Vector3Subtract(a, b); }
static Vector3 add(Vector3 a, Vector3 b) { return Vector3Add(a, b); }
static Vector3 scale(Vector3 a, float s) { return Vector3Scale(a, s); }
static float dot(Vector3 a, Vector3 b) { return Vector3DotProduct(a, b); }
static Vector3 cross(Vector3 a, Vector3 b) { return Vector3CrossProduct(a, b); }

Matrix HeronBrushEntityMatrix(const HeronBrushEntity *e) {
    // raylib composes left to right: the first matrix applies first.
    Matrix m = MatrixScale(e->scale[0], e->scale[1], e->scale[2]);
    m = MatrixMultiply(m, MatrixRotateZ(e->rot[2]));
    m = MatrixMultiply(m, MatrixRotateX(e->rot[0]));
    m = MatrixMultiply(m, MatrixRotateY(e->rot[1]));
    return MatrixMultiply(m, MatrixTranslate(e->pos[0], e->pos[1], e->pos[2]));
}

// Closest point on triangle abc to p. Ericson, Real-Time Collision Detection.
static Vector3 closest_on_tri(Vector3 p, Vector3 a, Vector3 b, Vector3 c) {
    Vector3 ab = sub(b, a), ac = sub(c, a), ap = sub(p, a);
    float d1 = dot(ab, ap), d2 = dot(ac, ap);
    if (d1 <= 0.0f && d2 <= 0.0f) return a;

    Vector3 bp = sub(p, b);
    float d3 = dot(ab, bp), d4 = dot(ac, bp);
    if (d3 >= 0.0f && d4 <= d3) return b;

    float vc = d1 * d4 - d3 * d2;
    if (vc <= 0.0f && d1 >= 0.0f && d3 <= 0.0f)
        return add(a, scale(ab, d1 / (d1 - d3)));

    Vector3 cp = sub(p, c);
    float d5 = dot(ab, cp), d6 = dot(ac, cp);
    if (d6 >= 0.0f && d5 <= d6) return c;

    float vb = d5 * d2 - d1 * d6;
    if (vb <= 0.0f && d2 >= 0.0f && d6 <= 0.0f)
        return add(a, scale(ac, d2 / (d2 - d6)));

    float va = d3 * d6 - d5 * d4;
    if (va <= 0.0f && (d4 - d3) >= 0.0f && (d5 - d6) >= 0.0f)
        return add(b, scale(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))));

    float denom = 1.0f / (va + vb + vc);
    return add(a, add(scale(ab, vb * denom), scale(ac, vc * denom)));
}

static Vector3 closest_on_seg(Vector3 p, Vector3 a, Vector3 b) {
    Vector3 ab = sub(b, a);
    float t = dot(sub(p, a), ab) / (dot(ab, ab) + 1e-9f);
    return add(a, scale(ab, Clamp(t, 0.0f, 1.0f)));
}

// Closest points between segment s0-s1 and triangle abc, refined by a few
// rounds of alternation. Good enough for walking around. Returns the distance.
static float seg_tri(Vector3 s0, Vector3 s1, Vector3 a, Vector3 b, Vector3 c,
                     Vector3 *onSeg, Vector3 *onTri) {
    Vector3 ps = scale(add(s0, s1), 0.5f);
    Vector3 pt = closest_on_tri(ps, a, b, c);
    for (int k = 0; k < 3; k++) {
        ps = closest_on_seg(pt, s0, s1);
        pt = closest_on_tri(ps, a, b, c);
    }
    *onSeg = ps;
    *onTri = pt;
    return Vector3Distance(ps, pt);
}

static void set_tri(HeronBrushTri *t, Vector3 a, Vector3 b, Vector3 c) {
    t->a = a;
    t->b = b;
    t->c = c;
    t->normal = Vector3Normalize(cross(sub(b, a), sub(c, a)));
    t->bmin = Vector3Min(a, Vector3Min(b, c));
    t->bmax = Vector3Max(a, Vector3Max(b, c));
}

// --- Convex hull of a point cloud --------------------------------------------
//
// Incremental: a starting tetrahedron, then each point outside the current hull
// replaces the faces it sees with a fan to the horizon. Brush hulls have a few
// dozen points, so the quadratic edge search does not matter. Points within a
// small tolerance of a face count as inside, which drops the coplanar and
// duplicate points brushes are full of.

typedef struct { int a, b, c; } Face;

static Vector3 face_normal(const Vector3 *p, Face f) {
    return cross(sub(p[f.b], p[f.a]), sub(p[f.c], p[f.a]));
}

static int build_hull(const Vector3 *p, int n, int **outVerts) {
    *outVerts = NULL;
    if (n < 4) return 0;
    Vector3 lo = p[0], hi = p[0];
    for (int i = 1; i < n; i++) { lo = Vector3Min(lo, p[i]); hi = Vector3Max(hi, p[i]); }
    float eps = fmaxf(Vector3Distance(lo, hi), 1e-3f) * 1e-5f;

    // A non-degenerate starting tetrahedron.
    int i0 = 0, i1 = -1, i2 = -1, i3 = -1;
    float best = eps;
    for (int i = 1; i < n; i++) {
        float d = Vector3Distance(p[i], p[i0]);
        if (d > best) { best = d; i1 = i; }
    }
    if (i1 < 0) return 0;
    best = eps * eps;
    for (int i = 0; i < n; i++) {
        float d = Vector3LengthSqr(cross(sub(p[i1], p[i0]), sub(p[i], p[i0])));
        if (d > best) { best = d; i2 = i; }
    }
    if (i2 < 0) return 0;
    Vector3 nrm = Vector3Normalize(cross(sub(p[i1], p[i0]), sub(p[i2], p[i0])));
    best = eps;
    for (int i = 0; i < n; i++) {
        float d = fabsf(dot(nrm, sub(p[i], p[i0])));
        if (d > best) { best = d; i3 = i; }
    }
    if (i3 < 0) return 0; // flat: no volume to collide with

    int cap = 4 + 2 * n * 3;
    Face *faces = malloc(sizeof(Face) * (size_t)cap);
    int (*horizon)[2] = malloc(sizeof(int[2]) * (size_t)cap * 3);
    bool *gone = malloc(sizeof(bool) * (size_t)cap);
    if (!faces || !horizon || !gone) { free(faces); free(horizon); free(gone); return 0; }
    int fc = 0;
    Vector3 inside = scale(add(add(p[i0], p[i1]), add(p[i2], p[i3])), 0.25f);
    Face start[4] = {{i0, i1, i2}, {i0, i1, i3}, {i0, i2, i3}, {i1, i2, i3}};
    for (int k = 0; k < 4; k++) {
        Face f = start[k];
        if (dot(face_normal(p, f), sub(inside, p[f.a])) > 0.0f) { int t = f.b; f.b = f.c; f.c = t; }
        faces[fc++] = f;
    }

    for (int i = 0; i < n; i++) {
        if (i == i0 || i == i1 || i == i2 || i == i3) continue;
        int visible = 0;
        for (int k = 0; k < fc; k++) {
            Vector3 fn = Vector3Normalize(face_normal(p, faces[k]));
            gone[k] = dot(fn, sub(p[i], p[faces[k].a])) > eps;
            visible += gone[k];
        }
        if (!visible) continue;
        // Edges of a visible face whose neighbour across that edge stays.
        int hc = 0;
        for (int k = 0; k < fc; k++) {
            if (!gone[k]) continue;
            int e[3][2] = {{faces[k].a, faces[k].b}, {faces[k].b, faces[k].c}, {faces[k].c, faces[k].a}};
            for (int j = 0; j < 3; j++) {
                bool shared = false;
                for (int q = 0; q < fc && !shared; q++) {
                    if (q == k || !gone[q]) continue;
                    int v[3] = {faces[q].a, faces[q].b, faces[q].c};
                    for (int s = 0; s < 3; s++)
                        if (v[s] == e[j][1] && v[(s + 1) % 3] == e[j][0]) shared = true;
                }
                if (!shared) { horizon[hc][0] = e[j][0]; horizon[hc][1] = e[j][1]; hc++; }
            }
        }
        int kept = 0;
        for (int k = 0; k < fc; k++) if (!gone[k]) faces[kept++] = faces[k];
        fc = kept;
        if (fc + hc > cap) break; // cannot happen for a valid cloud; stay safe
        for (int k = 0; k < hc; k++) faces[fc++] = (Face){horizon[k][0], horizon[k][1], i};
    }

    int *verts = malloc(sizeof(int) * 3 * (size_t)fc);
    if (verts) {
        for (int k = 0; k < fc; k++) {
            verts[k * 3] = faces[k].a;
            verts[k * 3 + 1] = faces[k].b;
            verts[k * 3 + 2] = faces[k].c;
        }
    }
    free(faces);
    free(horizon);
    free(gone);
    *outVerts = verts;
    return verts ? fc : 0;
}

static void hull_place(HeronBrushHullShape *h, Matrix m) {
    Vector3 centre = {0};
    for (int i = 0; i < h->vertCount; i++) {
        h->world[i] = Vector3Transform(h->local[i], m);
        centre = add(centre, h->world[i]);
        h->bmin = i ? Vector3Min(h->bmin, h->world[i]) : h->world[i];
        h->bmax = i ? Vector3Max(h->bmax, h->world[i]) : h->world[i];
    }
    centre = scale(centre, 1.0f / (float)h->vertCount);
    for (int f = 0; f < h->faceCount; f++) {
        Vector3 a = h->world[h->faceVerts[f * 3]];
        Vector3 nrm = Vector3Normalize(cross(sub(h->world[h->faceVerts[f * 3 + 1]], a),
                                             sub(h->world[h->faceVerts[f * 3 + 2]], a)));
        // A mirroring scale turns the winding inside out; keep normals outward.
        if (dot(nrm, sub(centre, a)) > 0.0f) nrm = Vector3Negate(nrm);
        h->planes[f] = (Vector4){nrm.x, nrm.y, nrm.z, dot(nrm, a)};
    }
}

static bool hull_init(HeronBrushHullShape *h, int entIndex, const float *xyz, int count, Matrix m) {
    memset(h, 0, sizeof(*h));
    h->entIndex = entIndex;
    h->local = malloc(sizeof(Vector3) * (size_t)count);
    h->world = malloc(sizeof(Vector3) * (size_t)count);
    if (!h->local || !h->world) return false;
    for (int i = 0; i < count; i++)
        h->local[i] = (Vector3){xyz[i * 3], xyz[i * 3 + 1], xyz[i * 3 + 2]};
    h->vertCount = count;
    h->faceCount = build_hull(h->local, count, &h->faceVerts);
    if (h->faceCount == 0) return false;
    h->planes = malloc(sizeof(Vector4) * (size_t)h->faceCount);
    if (!h->planes) return false;
    hull_place(h, m);
    return true;
}

static void hull_free(HeronBrushHullShape *h) {
    free(h->local);
    free(h->world);
    free(h->faceVerts);
    free(h->planes);
}

// --- Building ------------------------------------------------------------------

static bool blocks(const HeronBrushEntity *e) { return !e->collision.sensor; }

static Vector3 entity_point(const float *f, Matrix m) {
    return Vector3Transform((Vector3){f[0], f[1], f[2]}, m);
}

bool HeronBrushEntityWorldAABB(const HeronBrushEntity *e, Vector3 *mnOut, Vector3 *mxOut) {
    Matrix m = HeronBrushEntityMatrix(e);
    Vector3 mn = {1e30f, 1e30f, 1e30f}, mx = {-1e30f, -1e30f, -1e30f};
    bool any = false;
    const HeronBrushCollision *c = &e->collision;
    for (int k = 0; k < c->triCount * 3; k++) {
        Vector3 p = entity_point(&c->tris[k * 3], m);
        mn = Vector3Min(mn, p);
        mx = Vector3Max(mx, p);
        any = true;
    }
    for (int h = 0; h < c->hullCount; h++) {
        for (int k = 0; k < c->hulls[h].vertexCount; k++) {
            Vector3 p = entity_point(&c->hulls[h].vertices[k * 3], m);
            mn = Vector3Min(mn, p);
            mx = Vector3Max(mx, p);
            any = true;
        }
    }
    if (c->hullCount == 0) {
        for (int k = 0; k < c->boxCount; k++) {
            const float *f = &c->boxes[k * 6];
            for (int corner = 0; corner < 8; corner++) {
                float q[3] = {f[(corner & 1) ? 3 : 0], f[(corner & 2) ? 4 : 1], f[(corner & 4) ? 5 : 2]};
                Vector3 p = entity_point(q, m);
                mn = Vector3Min(mn, p);
                mx = Vector3Max(mx, p);
                any = true;
            }
        }
    }
    if (any) {
        *mnOut = mn;
        *mxOut = mx;
    }
    return any;
}

// Whether entity `i` moves, so its triangles stay out of the static soup and only its
// hulls, which follow it, collide: the sample's doors and lifts, and anything the
// editor animated (animated.c).
static bool moves(int i, const HeronBrushEntity *e) {
    return HeronBrushIsMoverClass(e->classname) || HeronBrushAnimatedHas(i);
}

void HeronBrushColliderBuild(HeronBrushCollider *col, const HeronBrushLevel *level) {
    memset(col, 0, sizeof(*col));
    int tris = 0, hulls = 0;
    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        if (!blocks(e)) continue;
        if (!moves(i, e)) tris += e->collision.triCount;
        hulls += e->collision.hullCount ? e->collision.hullCount : e->collision.boxCount;
    }
    col->tris = tris ? malloc(sizeof(HeronBrushTri) * (size_t)tris) : NULL;
    col->cap = col->tris ? tris : 0;
    col->hulls = hulls ? calloc((size_t)hulls, sizeof(HeronBrushHullShape)) : NULL;
    col->entHullFirst = calloc((size_t)level->entityCount + 1, sizeof(int));
    col->entHullCount = calloc((size_t)level->entityCount + 1, sizeof(int));

    for (int i = 0; i < level->entityCount; i++) {
        const HeronBrushEntity *e = &level->entities[i];
        col->entHullFirst[i] = col->hullCount;
        if (!blocks(e)) continue;
        Matrix m = HeronBrushEntityMatrix(e);
        const HeronBrushCollision *c = &e->collision;

        if (!moves(i, e) && col->tris) {
            for (int k = 0; k < c->triCount; k++) {
                const float *f = &c->tris[k * 9];
                set_tri(&col->tris[col->count++], entity_point(f, m), entity_point(f + 3, m),
                        entity_point(f + 6, m));
            }
        }
        if (!col->hulls) continue;
        if (c->hullCount > 0) {
            for (int h = 0; h < c->hullCount; h++) {
                HeronBrushHullShape *shape = &col->hulls[col->hullCount];
                if (hull_init(shape, i, c->hulls[h].vertices, c->hulls[h].vertexCount, m))
                    col->hullCount++;
                else
                    hull_free(shape);
            }
        } else {
            // An older export: only the fallback boxes, as eight-point hulls.
            for (int k = 0; k < c->boxCount; k++) {
                const float *f = &c->boxes[k * 6];
                float pts[24];
                for (int corner = 0; corner < 8; corner++) {
                    pts[corner * 3] = f[(corner & 1) ? 3 : 0];
                    pts[corner * 3 + 1] = f[(corner & 2) ? 4 : 1];
                    pts[corner * 3 + 2] = f[(corner & 4) ? 5 : 2];
                }
                HeronBrushHullShape *shape = &col->hulls[col->hullCount];
                if (hull_init(shape, i, pts, 8, m)) col->hullCount++;
                else hull_free(shape);
            }
        }
        col->entHullCount[i] = col->hullCount - col->entHullFirst[i];
    }

    bool any = false;
    for (int i = 0; i < col->count; i++) {
        col->min = any ? Vector3Min(col->min, col->tris[i].bmin) : col->tris[i].bmin;
        col->max = any ? Vector3Max(col->max, col->tris[i].bmax) : col->tris[i].bmax;
        any = true;
    }
    for (int i = 0; i < col->hullCount; i++) {
        col->min = any ? Vector3Min(col->min, col->hulls[i].bmin) : col->hulls[i].bmin;
        col->max = any ? Vector3Max(col->max, col->hulls[i].bmax) : col->hulls[i].bmax;
        any = true;
    }
}

void HeronBrushColliderFree(HeronBrushCollider *col) {
    for (int i = 0; i < col->hullCount; i++) hull_free(&col->hulls[i]);
    free(col->hulls);
    free(col->tris);
    free(col->entHullFirst);
    free(col->entHullCount);
    memset(col, 0, sizeof(*col));
}

void HeronBrushColliderSetEntityTransform(HeronBrushCollider *col, int entIndex, Matrix m) {
    if (!col->entHullCount) return;
    for (int k = 0; k < col->entHullCount[entIndex]; k++)
        hull_place(&col->hulls[col->entHullFirst[entIndex] + k], m);
}

bool HeronBrushColliderEntityBounds(const HeronBrushCollider *col, int entIndex, Vector3 *mn, Vector3 *mx) {
    if (!col->entHullCount || col->entHullCount[entIndex] == 0) return false;
    for (int k = 0; k < col->entHullCount[entIndex]; k++) {
        const HeronBrushHullShape *h = &col->hulls[col->entHullFirst[entIndex] + k];
        *mn = k ? Vector3Min(*mn, h->bmin) : h->bmin;
        *mx = k ? Vector3Max(*mx, h->bmax) : h->bmax;
    }
    return true;
}

// --- Queries -------------------------------------------------------------------

static float signed_dist(Vector4 pl, Vector3 p) { return pl.x * p.x + pl.y * p.y + pl.z * p.z - pl.w; }

// How far, and which way, a capsule around segment s0-s1 with radius r must
// move to stop overlapping hull h. False when it does not overlap.
static bool capsule_hull(const HeronBrushHullShape *h, Vector3 s0, Vector3 s1, float r,
                         Vector3 *nOut, float *depthOut) {
    Vector3 lo = Vector3Min(s0, s1), hi = Vector3Max(s0, s1);
    if (h->bmax.x < lo.x - r || h->bmin.x > hi.x + r || h->bmax.y < lo.y - r ||
        h->bmin.y > hi.y + r || h->bmax.z < lo.z - r || h->bmin.z > hi.z + r)
        return false;

    // Does the segment itself pass through the solid? Clip it by every plane.
    float t0 = 0.0f, t1 = 1.0f;
    bool through = true;
    for (int f = 0; f < h->faceCount && through; f++) {
        float d0 = signed_dist(h->planes[f], s0), d1 = signed_dist(h->planes[f], s1);
        if (d0 > 0.0f && d1 > 0.0f) through = false;
        else if (d0 > 0.0f) t0 = fmaxf(t0, d0 / (d0 - d1));
        else if (d1 > 0.0f) t1 = fminf(t1, d0 / (d0 - d1));
        if (t0 > t1) through = false;
    }
    if (through) {
        // Deep: leave through the face that needs the shortest move.
        float best = 1e30f;
        for (int f = 0; f < h->faceCount; f++) {
            float d = fminf(signed_dist(h->planes[f], s0), signed_dist(h->planes[f], s1));
            float push = r - d;
            if (push < best) {
                best = push;
                *nOut = (Vector3){h->planes[f].x, h->planes[f].y, h->planes[f].z};
            }
        }
        *depthOut = best;
        return best > 0.0f;
    }

    // Outside: the nearest surface point decides, over every face.
    float best = r;
    Vector3 ns = {0}, nt = {0};
    for (int f = 0; f < h->faceCount; f++) {
        Vector3 ps, pt;
        float d = seg_tri(s0, s1, h->world[h->faceVerts[f * 3]], h->world[h->faceVerts[f * 3 + 1]],
                          h->world[h->faceVerts[f * 3 + 2]], &ps, &pt);
        if (d < best) { best = d; ns = ps; nt = pt; }
    }
    if (best >= r) return false;
    if (best > 1e-5f) {
        *nOut = scale(sub(ns, nt), 1.0f / best);
    } else {
        // Touching exactly: the face it touches.
        float least = 1e30f;
        for (int f = 0; f < h->faceCount; f++) {
            float d = fabsf(signed_dist(h->planes[f], nt));
            if (d < least) { least = d; *nOut = (Vector3){h->planes[f].x, h->planes[f].y, h->planes[f].z}; }
        }
    }
    *depthOut = r - best;
    return true;
}

static void apply_push(Vector3 *centre, Vector3 *s0, Vector3 *s1, Vector3 n, float push,
                       Vector3 *vel, bool *grounded) {
    Vector3 d = scale(n, push + 0.001f);
    *centre = add(*centre, d);
    *s0 = add(*s0, d);
    *s1 = add(*s1, d);
    float vn = dot(*vel, n);
    if (vn < 0.0f) *vel = sub(*vel, scale(n, vn));
    if (n.y > 0.5f) *grounded = true;
}

void HeronBrushColliderResolveCapsule(const HeronBrushCollider *col, Vector3 *centre, float r,
                               float h, Vector3 *vel, bool *grounded) {
    *grounded = false;
    float half = fmaxf(h - r, 0.0f);

    for (int iter = 0; iter < 5; iter++) {
        Vector3 s0 = {centre->x, centre->y - half, centre->z};
        Vector3 s1 = {centre->x, centre->y + half, centre->z};
        bool moved = false;

        for (int i = 0; i < col->count; i++) {
            const HeronBrushTri *t = &col->tris[i];
            if (t->bmax.x < centre->x - r || t->bmin.x > centre->x + r ||
                t->bmax.y < s0.y - r || t->bmin.y > s1.y + r ||
                t->bmax.z < centre->z - r || t->bmin.z > centre->z + r)
                continue;
            Vector3 ps, pt;
            float dist = seg_tri(s0, s1, t->a, t->b, t->c, &ps, &pt);
            if (dist >= r) continue;
            Vector3 n = dist > 1e-4f ? scale(sub(ps, pt), 1.0f / dist) : t->normal;
            apply_push(centre, &s0, &s1, n, r - (dist > 1e-4f ? dist : 0.0f), vel, grounded);
            moved = true;
        }
        for (int i = 0; i < col->hullCount; i++) {
            Vector3 n;
            float depth;
            if (!capsule_hull(&col->hulls[i], s0, s1, r, &n, &depth)) continue;
            apply_push(centre, &s0, &s1, n, depth, vel, grounded);
            moved = true;
        }
        if (!moved) break;
    }
}

// Möller-Trumbore. Distance along unit `d`, or -1.
static float ray_tri(const HeronBrushTri *t, Vector3 o, Vector3 d) {
    Vector3 e1 = sub(t->b, t->a), e2 = sub(t->c, t->a);
    Vector3 pvec = cross(d, e2);
    float det = dot(e1, pvec);
    if (fabsf(det) < 1e-7f) return -1.0f;
    float inv = 1.0f / det;
    Vector3 tvec = sub(o, t->a);
    float u = dot(tvec, pvec) * inv;
    if (u < -1e-4f || u > 1.0001f) return -1.0f;
    Vector3 qvec = cross(tvec, e1);
    float v = dot(d, qvec) * inv;
    if (v < -1e-4f || u + v > 1.0001f) return -1.0f;
    return dot(e2, qvec) * inv;
}

// Where a ray enters convex hull h (Cyrus-Beck), and the normal of the face it
// enters through; the entry is negative when the ray starts inside. False on a miss.
static bool ray_hull(const HeronBrushHullShape *h, Vector3 o, Vector3 d, float *tOut, Vector3 *nOut) {
    float enter = -1e30f, leave = 1e30f;
    Vector3 n = {0};
    for (int f = 0; f < h->faceCount; f++) {
        float denom = h->planes[f].x * d.x + h->planes[f].y * d.y + h->planes[f].z * d.z;
        float dist = signed_dist(h->planes[f], o);
        if (fabsf(denom) < 1e-9f) {
            if (dist > 0.0f) return false;
            continue;
        }
        float t = -dist / denom;
        if (denom < 0.0f) {
            if (t > enter) { enter = t; n = (Vector3){h->planes[f].x, h->planes[f].y, h->planes[f].z}; }
        } else if (t < leave) {
            leave = t;
        }
        if (enter > leave) return false;
    }
    if (leave < 0.0f) return false;
    *tOut = enter;
    *nOut = n;
    return true;
}

float HeronBrushColliderFloorBelow(const HeronBrushCollider *col, Vector3 from, float maxDist) {
    Vector3 dir = {0.0f, -1.0f, 0.0f};
    float best = -1.0f;

    for (int i = 0; i < col->count; i++) {
        const HeronBrushTri *t = &col->tris[i];
        // Near-horizontal, either winding: a downward ray only meets floors.
        if (fabsf(t->normal.y) <= 0.5f) continue;
        if (from.x < t->bmin.x || from.x > t->bmax.x || from.z < t->bmin.z ||
            from.z > t->bmax.z)
            continue;
        float d = ray_tri(t, from, dir);
        if (d >= -0.02f && d <= maxDist && (best < 0.0f || d < best)) best = d;
    }
    for (int i = 0; i < col->hullCount; i++) {
        const HeronBrushHullShape *h = &col->hulls[i];
        if (from.x < h->bmin.x || from.x > h->bmax.x || from.z < h->bmin.z ||
            from.z > h->bmax.z || from.y < h->bmin.y - 0.02f)
            continue;
        float d;
        Vector3 n;
        if (!ray_hull(h, from, dir, &d, &n) || n.y <= 0.5f) continue;
        if (d >= -0.02f && d <= maxDist && (best < 0.0f || d < best)) best = d;
    }
    return best;
}

float HeronBrushColliderRayHit(const HeronBrushCollider *col, Vector3 origin, Vector3 dir,
                        float maxDist) {
    float len = Vector3Length(dir);
    if (len < 1e-6f) return -1.0f;
    Vector3 d = scale(dir, 1.0f / len);
    float best = -1.0f;

    for (int i = 0; i < col->count; i++) {
        float hit = ray_tri(&col->tris[i], origin, d);
        if (hit > 0.01f && hit <= maxDist && (best < 0.0f || hit < best)) best = hit;
    }
    for (int i = 0; i < col->hullCount; i++) {
        float hit;
        Vector3 n;
        if (!ray_hull(&col->hulls[i], origin, d, &hit, &n)) continue;
        if (hit > 0.01f && hit <= maxDist && (best < 0.0f || hit < best)) best = hit;
    }
    return best;
}

void HeronBrushColliderDrawWires(const HeronBrushCollider *col) {
    for (int i = 0; i < col->count; i++) {
        const HeronBrushTri *t = &col->tris[i];
        Color c = t->normal.y > 0.5f ? (Color){80, 220, 120, 90} : (Color){120, 160, 255, 70};
        DrawLine3D(t->a, t->b, c);
        DrawLine3D(t->b, t->c, c);
        DrawLine3D(t->c, t->a, c);
    }
    for (int i = 0; i < col->hullCount; i++) {
        const HeronBrushHullShape *h = &col->hulls[i];
        for (int f = 0; f < h->faceCount; f++) {
            Vector3 a = h->world[h->faceVerts[f * 3]], b = h->world[h->faceVerts[f * 3 + 1]],
                    c = h->world[h->faceVerts[f * 3 + 2]];
            Color k = {255, 190, 60, 160};
            DrawLine3D(a, b, k);
            DrawLine3D(b, c, k);
            DrawLine3D(c, a, k);
        }
    }
}
