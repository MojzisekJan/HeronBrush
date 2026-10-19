# Three.js export check

Two ways to view the shared demo map in a browser:

- `index.html` loads `assets/demo.json`, textures and GLBs through
  `src/heron_brushlevel.js`. This is the integration example for your own game.
- `assets/demo.html` is the editor's generated self-contained viewer with a
  walking controller.

## Export and run

From the repository root:

```sh
python3 examples/engine-checks/threejs/export_demo.py
python3 examples/engine-checks/raylib/export_demo.py
python3 examples/engine-checks/threejs/verify_export.py
python3 -m http.server 8768 --bind 127.0.0.1 --directory examples/engine-checks/threejs
```

Open `http://127.0.0.1:8768/index.html` or `http://127.0.0.1:8768/assets/demo.html`.
Three.js loads from jsDelivr; change the import map to use local copies. WASD
moves, Space jumps, Shift runs.

`verify_export.py` compares the Three.js and raylib exports. The `verify_*.cjs`
and `verify_*.mjs` scripts cover animation, motion lab, water, custom games and
the web playtest; see [target parity](../../../docs/target-parity.md) and
[web](../../../docs/web.md) for their commands.

## Loader API

`loadLevel(url, {factories, blocks})` returns the scene, entity records, collision,
`animations`, `triggers` and `advance(dt, feet)`. `blocks(entity)` picks which
entities collide. Factories are chosen by `three_factory` or classname and may
return a cleanup function that `dispose()` calls.

Not covered: portals, enemies and FPS doors (game code), normal and roughness
maps, and TGA/WebP textures inside GLB.

Credits: loader and controller are MIT code from the earlier tbx project, see
`LICENSE-TBX`. Textures are covered by `LICENSE-KENNEY`.
