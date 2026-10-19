#include "level.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Pulls the next whitespace-delimited token off `*cursor`, advancing it and
// terminating the token in place. NULL at end of line.
static char *word(char **cursor) {
    if (!*cursor) return NULL;
    while (**cursor == ' ' || **cursor == '\t') (*cursor)++;
    if (**cursor == '\0') return NULL;
    char *start = *cursor;
    while (**cursor && **cursor != ' ' && **cursor != '\t') (*cursor)++;
    if (**cursor) {
        **cursor = '\0';
        (*cursor)++;
    }
    return start;
}

// Whatever is left, leading blank trimmed, trailing newline already gone.
static const char *rest(char *cursor) {
    if (!cursor) return "";
    while (*cursor == ' ' || *cursor == '\t') cursor++;
    return cursor;
}

static void copy(char *dst, const char *src, size_t cap) {
    if (!src) src = "";
    strncpy(dst, src, cap - 1);
    dst[cap - 1] = '\0';
}

static void readn(char **cursor, float *out, int n) {
    for (int i = 0; i < n; i++) {
        char *w = word(cursor);
        out[i] = w ? (float)atof(w) : 0.0f;
    }
}

static void read3(char **cursor, float out[3]) { readn(cursor, out, 3); }

// Appends `n` floats to a growing malloc'd buffer. *count is the number of
// groups of `n` already stored. Grows the allocation to fit exactly; a level
// loads once so the extra copies do not matter.
static void append_floats(float **buf, int *count, char **cursor, int n) {
    float *grown = realloc(*buf, (size_t)(*count + 1) * n * sizeof(float));
    if (!grown) return;
    *buf = grown;
    readn(cursor, grown + (size_t)(*count) * n, n);
    (*count)++;
}

bool HeronBrushLoadLevel(const char *path, HeronBrushLevel *level) {
    memset(level, 0, sizeof(*level));

    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "heron_brush: cannot open %s\n", path);
        return false;
    }

    char line[1024];
    HeronBrushEntity *e = NULL; // the open record, or NULL in the header

    while (fgets(line, sizeof(line), f)) {
        line[strcspn(line, "\r\n")] = '\0';
        char *cursor = line;
        char *key = word(&cursor);
        if (!key || key[0] == '#') continue;

        if (strcmp(key, "end") == 0) {
            e = NULL;
            continue;
        }

        if (strcmp(key, "entity") == 0) {
            if (level->entityCount >= HERON_BRUSH_MAX_ENTITIES) {
                fprintf(stderr, "heron_brush: more than %d entities, raise HERON_BRUSH_MAX_ENTITIES\n",
                        HERON_BRUSH_MAX_ENTITIES);
                break;
            }
            e = &level->entities[level->entityCount++];
            memset(e, 0, sizeof(*e));
            e->scale[0] = e->scale[1] = e->scale[2] = 1.0f;
            copy(e->classname, word(&cursor), HERON_BRUSH_MAX_NAME);
            copy(e->name, word(&cursor), HERON_BRUSH_MAX_NAME);
            continue;
        }

        if (!e) {
            // Header keys.
            if (strcmp(key, "format") == 0) level->format = atoi(rest(cursor));
            else if (strcmp(key, "name") == 0) copy(level->name, rest(cursor), HERON_BRUSH_MAX_NAME);
            else if (strcmp(key, "scale") == 0) level->scale = (float)atof(rest(cursor));
            else if (strcmp(key, "texture") == 0 && level->textureCount < HERON_BRUSH_MAX_TEXTURES) {
                HeronBrushTexture *t = &level->textures[level->textureCount++];
                copy(t->name, word(&cursor), HERON_BRUSH_MAX_NAME);
                const char *img = word(&cursor);
                copy(t->image, (img && strcmp(img, "-") != 0) ? img : "", HERON_BRUSH_MAX_VALUE);
                char *w = word(&cursor);
                t->width = w ? (float)atof(w) : 0.0f;
                w = word(&cursor);
                t->height = w ? (float)atof(w) : 0.0f;
            }
            continue;
        }

        // Entity keys.
        if (strcmp(key, "solid") == 0) e->solid = atoi(rest(cursor)) != 0;
        else if (strcmp(key, "mesh") == 0) copy(e->mesh, rest(cursor), HERON_BRUSH_MAX_VALUE);
        else if (strcmp(key, "pos") == 0) read3(&cursor, e->pos);
        else if (strcmp(key, "rot") == 0) read3(&cursor, e->rot);
        else if (strcmp(key, "scale") == 0) read3(&cursor, e->scale);
        else if (strcmp(key, "surface") == 0 && e->surfaceCount < HERON_BRUSH_MAX_SURFACES) {
            HeronBrushSurface *s = &e->surfaces[e->surfaceCount++];
            char *w = word(&cursor);
            s->primitive = w ? atoi(w) : 0;
            copy(s->texture, word(&cursor), HERON_BRUSH_MAX_NAME);
        } else if (strcmp(key, "collision") == 0) {
            const char *kind = word(&cursor);
            if (kind && strcmp(kind, "trimesh") == 0) e->collision.kind = HERON_BRUSH_COLLISION_TRIMESH;
            else if (kind && strcmp(kind, "convex") == 0) e->collision.kind = HERON_BRUSH_COLLISION_CONVEX;
            char *w = word(&cursor);
            e->collision.mesh = w ? atoi(w) : 0;
            w = word(&cursor);
            e->collision.parts = w ? atoi(w) : 0;
            w = word(&cursor);
            e->collision.sensor = w && strcmp(w, "sensor") == 0;
        } else if (strcmp(key, "ctri") == 0) {
            append_floats(&e->collision.tris, &e->collision.triCount, &cursor, 9);
        } else if (strcmp(key, "chull") == 0) {
            HeronBrushHull *grown = realloc(e->collision.hulls,
                (size_t)(e->collision.hullCount + 1) * sizeof(HeronBrushHull));
            if (!grown) { fclose(f); HeronBrushFreeLevel(level); return false; }
            e->collision.hulls = grown;
            HeronBrushHull *h = &grown[e->collision.hullCount++];
            memset(h, 0, sizeof(*h));
            h->expectedCount = atoi(rest(cursor));
        } else if (strcmp(key, "chv") == 0 && e->collision.hullCount > 0) {
            HeronBrushHull *h = &e->collision.hulls[e->collision.hullCount - 1];
            append_floats(&h->vertices, &h->vertexCount, &cursor, 3);
        } else if (strcmp(key, "chend") == 0 && e->collision.hullCount > 0) {
            HeronBrushHull *h = &e->collision.hulls[e->collision.hullCount - 1];
            if (h->vertexCount != h->expectedCount) {
                fprintf(stderr, "heron_brush: incomplete convex hull\n");
                fclose(f); HeronBrushFreeLevel(level); return false;
            }
        } else if (strcmp(key, "cbox") == 0) {
            append_floats(&e->collision.boxes, &e->collision.boxCount, &cursor, 6);
        } else if (strcmp(key, "tag") == 0 && e->tagCount < HERON_BRUSH_MAX_TAGS) {
            copy(e->tags[e->tagCount++], rest(cursor), HERON_BRUSH_MAX_NAME);
        } else if (strcmp(key, "target") == 0 && e->targetCount < HERON_BRUSH_MAX_TARGETS) {
            copy(e->targets[e->targetCount++], rest(cursor), HERON_BRUSH_MAX_NAME);
        } else if (strcmp(key, "prop") == 0 && e->propCount < HERON_BRUSH_MAX_PROPS) {
            HeronBrushProp *p = &e->props[e->propCount++];
            copy(p->type, word(&cursor), sizeof(p->type));
            copy(p->key, word(&cursor), HERON_BRUSH_MAX_NAME);
            copy(p->value, rest(cursor), HERON_BRUSH_MAX_VALUE);
        }
        // group, scene: parsed past, unused by this viewer.
    }

    fclose(f);

    if (level->format == 0) {
        fprintf(stderr, "heron_brush: %s has no format line, is it a level file?\n", path);
        return false;
    }
    if (level->format > 2) {
        fprintf(stderr, "heron_brush: %s is format %d, this reader knows 2\n", path, level->format);
        return false;
    }
    return true;
}

void HeronBrushFreeLevel(HeronBrushLevel *level) {
    for (int i = 0; i < level->entityCount; i++) {
        free(level->entities[i].collision.tris);
        free(level->entities[i].collision.boxes);
        for (int h = 0; h < level->entities[i].collision.hullCount; h++)
            free(level->entities[i].collision.hulls[h].vertices);
        free(level->entities[i].collision.hulls);
        level->entities[i].collision.hulls = NULL;
        level->entities[i].collision.hullCount = 0;
        level->entities[i].collision.tris = NULL;
        level->entities[i].collision.boxes = NULL;
    }
}

const char *HeronBrushGetProp(const HeronBrushEntity *entity, const char *key) {
    for (int i = 0; i < entity->propCount; i++) {
        if (strcmp(entity->props[i].key, key) == 0) return entity->props[i].value;
    }
    return NULL;
}

float HeronBrushPropFloat(const HeronBrushEntity *entity, const char *key, float fallback) {
    const char *v = HeronBrushGetProp(entity, key);
    return v && v[0] ? (float)atof(v) : fallback;
}

bool HeronBrushHasTag(const HeronBrushEntity *entity, const char *tag) {
    for (int i = 0; i < entity->tagCount; i++) {
        if (strcmp(entity->tags[i], tag) == 0) return true;
    }
    return false;
}

const HeronBrushEntity *HeronBrushFindEntity(const HeronBrushLevel *level, const char *name) {
    for (int i = 0; i < level->entityCount; i++) {
        if (strcmp(level->entities[i].name, name) == 0) return &level->entities[i];
    }
    return NULL;
}

bool HeronBrushIsMoverClass(const char *classname) {
    return strcmp(classname, "func_door") == 0 ||
           strcmp(classname, "fps_key_door") == 0 ||
           strcmp(classname, "fps_lift") == 0;
}
