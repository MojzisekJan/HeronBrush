# Raylib animation export

Build with raylib 5+ installed:

```sh
cc -std=c99 -O2 "demo_preview.c" -o preview $(pkg-config --cflags --libs raylib) -lm
./preview
```

Run from this exported directory. SPACE pauses, R plays/restarts all default clips (including disabled autoplay), N selects the next named clip. This is a free-camera visual preview, without gameplay or physics.

Include `demo_animation.h` in your game. HbInit initializes each HbPlayer and respects start_on. HbUpdate advances seconds. HbPlay selects a named clip; HbSeek, HbPause, HbResume, HbStop control playback. HbStop restores rest geometry; once holds its final pose. hb_animations[a].entity is the entity index in the .level/.json. HbMatrix yields the animation transform to apply AFTER the entity rest transform. HbPoint transforms WORLD engine-space vertices; use the same transform for collision vertices/hulls and update your physics body's pose/broadphase each frame. Never transform the previous frame's vertices again.

Linear, Hold and Bezier curves, pivots, once/loop/ping-pong, and named clips are included. Group members in one animated brush entity share playback. Exported func_button, trigger_once and trigger_multiple targets are included. Keep HbTriggerState states[HB_TRIGGER_COUNT+1] = {0} and call HbCheckTriggers(states, players, player_position, seconds) each frame. The preview uses its camera position. HbActivateTrigger handles explicit button presses, wait cooldowns and trigger_once. HbActivateAnimation toggles named open/close clips or restarts the selected clip.
