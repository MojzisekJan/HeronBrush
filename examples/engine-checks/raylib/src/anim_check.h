// `--anim-check`: the editor's animations and the collision together, on motion_lab
// (anim_check.c). Returns the process exit code: 0 when every check passed.
#ifndef HERON_BRUSH_ANIM_CHECK_H
#define HERON_BRUSH_ANIM_CHECK_H

#include "collision.h"
#include "level.h"

int HeronBrushAnimCheck(HeronBrushCollider *col, const HeronBrushLevel *level);

#endif
