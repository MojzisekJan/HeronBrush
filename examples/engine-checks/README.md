# Engine export checks

Small projects that take a Heron Brush export into a real engine and check that it
loads, renders and behaves. Most use the same map,
[`demos/bevy_fps/maps/demo.map`](../../demos/bevy_fps/maps/demo.map) (32 map units
per metre), exported through the editor's MCP commands with the FPS kit FGD.

The FPS gameplay in these projects is example game code. The exporter only writes
geometry, collision, materials and entity data; see
[the integration contract](../../docs/engine-export-contract.md).

| Folder | What it checks |
| --- | --- |
| [`godot/`](godot/README.md) | Native scene import, portals, doors, enemy damage |
| [`raylib/`](raylib/README.md) | Level reader, collision, animation riding, a playable C sample |
| [`threejs/`](threejs/README.md) | JSON loader and generated HTML viewer in a browser |
| [`babylon/`](babylon/README.md) | The feature lab through `heron_babylon.js` and Havok, and the generated playtest page, in Chromium |
| [`defold/`](defold/README.md) | Bob build, Bullet physics, game-owned doors and portals |
| [`flax/`](flax/README.md) | C# importer, native colliders, scene reload, animation |
| [`unity/`](unity/README.md) | Final and live import, collision, the feature lab in the engine, optional gameplay kit |
| `stride/`, `gdevelop/`, `cocos/` | Export scripts and run helpers, see [Stride](../../docs/stride-export.md), [GDevelop](../../docs/gdevelop-export.md), [Cocos](../../docs/cocos-export.md) |
| Bevy | Lives in [`demos/bevy_fps`](../../demos/bevy_fps/README.md) |

`export_motion_lab.py`, `prepare_motion_lab.py` and `prepare_water.py` export the
animation and water test maps; [target parity](../../docs/target-parity.md) lists
their commands. `export_feature_lab.py` exports the feature lab to all 15 targets
and `check_feature_lab.py` checks the files feature by feature; see
[the feature matrix](../../docs/feature-matrix.md). `feature_lab_poses.py` writes the
editor's own animation poses for the run-time checks of three.js, Bevy, raylib and the
portable glTF in [feature runtime checks](../../docs/feature-runtime-web.md), and of
Babylon.js in [its own page](../../docs/feature-runtime-babylon.md).

## Running

Build the editor first (`cargo build -p heron-brush`), then follow the folder's
README. Editor-side tests:

```sh
cargo test -p scene_export
cargo test -p editor_core --lib export::tests
cargo test -p editor_core --lib export_schema_tests
```

Not covered: a rendered map does not prove gameplay, and data checks do not prove
native physics. Each engine needs its own run on each platform you ship.
