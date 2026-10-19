// Keyframe animations made in the editor, played by the dependency-free C99 player the
// raylib export writes beside the level (`<map>_animation.h`).
//
// The player knows the clips, their pivots and the triggers that start them; this file
// applies what it says to the sample: each animated entity's mesh and its exact
// collision hulls take the animated transform every frame, the triggers fire from where
// the player's feet are, and whatever the player stands on carries them along, turning
// with it. The header is chosen when building (`make ANIM=path/to/map_animation.h`);
// the default is the demo map's, which has no animations.

#ifndef HERON_BRUSH_ANIMATED_H
#define HERON_BRUSH_ANIMATED_H

#include "collision.h"
#include "level.h"
#include "raylib.h"

#include <stdbool.h>

// Sets every animated entity at its first pose and moves its hulls there. Call after
// `HeronBrushColliderBuild`. False when the level and the compiled-in animations disagree
// (an animation names an entity the level does not have).
bool HeronBrushAnimatedBuild(HeronBrushCollider *col, const HeronBrushLevel *level);

// Whether entity `entIndex` is animated, so its triangles stay out of the static soup.
bool HeronBrushAnimatedHas(int entIndex);

// One step: fires the triggers the point `feet` is in, advances every playing clip
// by `dt`, moves the animated entities' hulls, and returns how far the animated entity
// under `feet` carried that point (zero when `grounded` is false or nothing animated is
// under it).
Vector3 HeronBrushAnimatedUpdate(HeronBrushCollider *col, Vector3 feet, bool grounded, float dt, double now);

// The world matrix entity `entIndex` is drawn and collides with, when it is animated.
bool HeronBrushAnimatedMatrix(int entIndex, Matrix *out);

// For checks: the number of animations, an animation's entity and name, and control
// of its playback.
int HeronBrushAnimatedCount(void);
int HeronBrushAnimatedEntity(int animation);
const char *HeronBrushAnimatedName(int animation);
void HeronBrushAnimatedPlay(int animation);
void HeronBrushAnimatedStop(int animation);
bool HeronBrushAnimatedPlaying(int animation);

#endif
