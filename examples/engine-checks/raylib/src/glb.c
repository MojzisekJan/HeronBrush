#include "glb.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// A JSON token. Tokens are stored in document order, so the subtree of token i
// is i + 1 .. next - 1, and an object's keys and values alternate inside it.
enum { J_OBJ, J_ARR, J_STR, J_PRIM };
typedef struct {
    int type, start, end, next;
} Tok;

typedef struct {
    const char *js;
    int len, pos;
    Tok *t;
    int n, cap;
} Parser;

static void ws(Parser *p) {
    while (p->pos < p->len && (p->js[p->pos] == ' ' || p->js[p->pos] == '\t' ||
                               p->js[p->pos] == '\n' || p->js[p->pos] == '\r'))
        p->pos++;
}

static int value(Parser *p, int depth) {
    ws(p);
    if (p->pos >= p->len || p->n >= p->cap || depth > 64) return -1;
    int at = p->n++;
    Tok *t = &p->t[at];
    char c = p->js[p->pos];
    t->start = p->pos;
    if (c == '{' || c == '[') {
        char close = c == '{' ? '}' : ']';
        t->type = c == '{' ? J_OBJ : J_ARR;
        p->pos++;
        ws(p);
        if (p->pos < p->len && p->js[p->pos] == close) {
            p->pos++;
        } else {
            for (;;) {
                if (t->type == J_OBJ) {
                    int key = value(p, depth + 1);
                    if (key < 0 || p->t[key].type != J_STR) return -1;
                    ws(p);
                    if (p->pos >= p->len || p->js[p->pos] != ':') return -1;
                    p->pos++;
                }
                if (value(p, depth + 1) < 0) return -1;
                ws(p);
                if (p->pos >= p->len) return -1;
                if (p->js[p->pos] == ',') { p->pos++; continue; }
                if (p->js[p->pos] == close) { p->pos++; break; }
                return -1;
            }
        }
        t = &p->t[at];
    } else if (c == '"') {
        t->type = J_STR;
        t->start = ++p->pos;
        while (p->pos < p->len && p->js[p->pos] != '"') p->pos += p->js[p->pos] == '\\' ? 2 : 1;
        if (p->pos >= p->len) return -1;
        t->end = p->pos++;
        t->next = p->n;
        return at;
    } else {
        t->type = J_PRIM;
        while (p->pos < p->len && !strchr(",}] \t\r\n", p->js[p->pos])) p->pos++;
    }
    t->end = p->pos;
    t->next = p->n;
    return at;
}

static bool is(const Parser *p, int tok, const char *s) {
    int n = p->t[tok].end - p->t[tok].start;
    return (int)strlen(s) == n && strncmp(p->js + p->t[tok].start, s, (size_t)n) == 0;
}

// The value of `key` in object `obj`, or -1.
static int get(const Parser *p, int obj, const char *key) {
    if (obj < 0 || p->t[obj].type != J_OBJ) return -1;
    for (int k = obj + 1; k < p->t[obj].next; k = p->t[k + 1].next)
        if (is(p, k, key)) return k + 1;
    return -1;
}

// Element `i` of array `arr`, or -1.
static int at(const Parser *p, int arr, int i) {
    if (arr < 0 || p->t[arr].type != J_ARR) return -1;
    for (int k = arr + 1; k < p->t[arr].next; k = p->t[k].next)
        if (i-- == 0) return k;
    return -1;
}

static double num(const Parser *p, int tok, double fallback) {
    if (tok < 0 || p->t[tok].type != J_PRIM) return fallback;
    char buf[64];
    int n = p->t[tok].end - p->t[tok].start;
    if (n <= 0 || n >= (int)sizeof(buf)) return fallback;
    memcpy(buf, p->js + p->t[tok].start, (size_t)n);
    buf[n] = '\0';
    return atof(buf);
}

static void nums(const Parser *p, int arr, float *out, int n) {
    for (int i = 0; i < n; i++) out[i] = (float)num(p, at(p, arr, i), out[i]);
}

// Whether texture `index` samples with CLAMP_TO_EDGE (33071) on S.
static bool clamps(const Parser *p, int root, int textureInfo) {
    int texture = at(p, get(p, root, "textures"), (int)num(p, get(p, textureInfo, "index"), -1));
    int sampler = at(p, get(p, root, "samplers"), (int)num(p, get(p, texture, "sampler"), -1));
    return (int)num(p, get(p, sampler, "wrapS"), 10497) == 33071;
}

bool HeronBrushReadGlbInfo(const char *path, HeronBrushGlbInfo *out) {
    memset(out, 0, sizeof(*out));
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "glb: cannot open %s\n", path);
        return false;
    }
    uint32_t header[5] = {0};
    bool ok = fread(header, sizeof(uint32_t), 5, f) == 5 && header[0] == 0x46546C67u &&
              header[4] == 0x4E4F534Au; // "glTF", first chunk "JSON"
    char *js = ok ? malloc(header[3] + 1) : NULL;
    ok = js && fread(js, 1, header[3], f) == header[3];
    fclose(f);
    if (!ok) {
        fprintf(stderr, "glb: %s is not a binary glTF\n", path);
        free(js);
        return false;
    }

    Parser p = {.js = js, .len = (int)header[3], .cap = (int)header[3] + 1};
    p.t = malloc(sizeof(Tok) * (size_t)p.cap);
    int root = p.t ? value(&p, 0) : -1;
    if (root < 0) {
        fprintf(stderr, "glb: %s has unreadable JSON\n", path);
        free(p.t);
        free(js);
        return false;
    }

    int materials = get(&p, root, "materials");
    for (int i = 0; i < HERON_BRUSH_GLB_MAX_MATERIALS; i++) {
        int m = at(&p, materials, i);
        if (m < 0) break;
        HeronBrushGlbMaterial *mat = &out->materials[out->materialCount++];
        *mat = (HeronBrushGlbMaterial){
            .baseColor = {1, 1, 1, 1}, .metallic = 1, .roughness = 1,
            .alphaCutoff = 0.5f, .normalScale = 1,
        };
        int name = get(&p, m, "name");
        if (name >= 0) {
            int n = p.t[name].end - p.t[name].start;
            if (n >= (int)sizeof(mat->name)) n = (int)sizeof(mat->name) - 1;
            memcpy(mat->name, js + p.t[name].start, (size_t)n);
        }
        int pbr = get(&p, m, "pbrMetallicRoughness");
        nums(&p, get(&p, pbr, "baseColorFactor"), mat->baseColor, 4);
        mat->metallic = (float)num(&p, get(&p, pbr, "metallicFactor"), 1.0);
        mat->roughness = (float)num(&p, get(&p, pbr, "roughnessFactor"), 1.0);
        int base = get(&p, pbr, "baseColorTexture");
        mat->hasBaseTexture = base >= 0;
        mat->baseClamp = base >= 0 && clamps(&p, root, base);
        nums(&p, get(&p, m, "emissiveFactor"), mat->emissive, 3);
        float strength = (float)num(&p, get(&p, get(&p, get(&p, m, "extensions"),
            "KHR_materials_emissive_strength"), "emissiveStrength"), 1.0);
        for (int c = 0; c < 3; c++) mat->emissive[c] *= strength;
        mat->hasEmissiveTexture = get(&p, m, "emissiveTexture") >= 0;
        int normal = get(&p, m, "normalTexture");
        mat->hasNormalTexture = normal >= 0;
        mat->normalScale = (float)num(&p, get(&p, normal, "scale"), 1.0);
        int mode = get(&p, m, "alphaMode");
        if (mode >= 0 && is(&p, mode, "BLEND")) mat->alphaMode = HERON_BRUSH_ALPHA_BLEND;
        else if (mode >= 0 && is(&p, mode, "MASK")) mat->alphaMode = HERON_BRUSH_ALPHA_MASK;
        mat->alphaCutoff = (float)num(&p, get(&p, m, "alphaCutoff"), 0.5);
        int dbl = get(&p, m, "doubleSided");
        mat->doubleSided = dbl >= 0 && is(&p, dbl, "true");
    }

    int prims = get(&p, at(&p, get(&p, root, "meshes"), 0), "primitives");
    for (int i = 0; i < HERON_BRUSH_GLB_MAX_MATERIALS * 4; i++) {
        int prim = at(&p, prims, i);
        if (prim < 0) break;
        int mat = (int)num(&p, get(&p, prim, "material"), -1);
        out->primitiveMaterial[out->primitiveCount] = mat < out->materialCount ? mat : -1;
        out->primitiveHasTangent[out->primitiveCount] =
            get(&p, get(&p, prim, "attributes"), "TANGENT") >= 0;
        out->primitiveCount++;
    }

    free(p.t);
    free(js);
    return true;
}
