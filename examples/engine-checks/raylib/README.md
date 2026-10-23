# raylib export check

A small C game that loads the shared demo map from the raylib export. `src/level.c`
reads the general `.level` format (properties, targets, transforms, materials,
triangle and convex hull collision). The FPS code in `src/game.c` and
`src/movers.c` is an example, not part of the exporter. Export also writes a
lossless `.json` sidecar for data the flat format cannot hold.

## Export and run

From the repository root, after building `target/debug/heron-brush`:

```sh
python3 examples/engine-checks/raylib/export_demo.py
python3 examples/engine-checks/threejs/export_demo.py
python3 examples/engine-checks/threejs/verify_export.py
cd examples/engine-checks/raylib
cc -std=c99 -Wall -Wextra src/verify_level.c src/level.c -o /tmp/heron-raylib-verify
/tmp/heron-raylib-verify
make -j2 RAYLIB=/path/to/raylib   # or plain `make` with a pkg-config install
./heron_brush-raylib-demo --demo --shot runtime.png --frames 140
```

Needs raylib 5.5 or 6.0 and OpenGL. Run without flags to play: Enter starts, WASD
moves, mouse aims and fires, Space jumps, C shows collision, Esc pauses.

Other checks:

- `make game-check RAYLIB=...`: portals and doors through the real game update, no window.
- `make anim-check RAYLIB=...`: motion lab animations with player carrying
  (export first with `export_motion_lab.py`). `make ANIM=assets/maps/<map>_animation.h`
  builds the game with another map's animations; `--play` skips the scripted input.
- `make feature-check RAYLIB=... FEATURE=... POSES=... SHOT=...`: the feature lab
  export through `src/level.c`, the generated animation, trigger and water headers,
  `LoadModel` and the sky cube, with a picture; see
  [feature runtime checks](../../../docs/feature-runtime-web.md).
- `prepare_custom_game.py` with `src/verify_custom_game.c`: a separate puzzle game
  without the FPS code.

Not covered: portal momentum and look rotation, TGA/WebP textures inside GLB, and
long strings beyond the fixed caps in `src/level.h`.

Credits: game sources are MIT code from the earlier tbx project, see `LICENSE-TBX`.
Textures, sounds and font are covered by `LICENSE-KENNEY` and the licence files
next to them in `assets/`.
