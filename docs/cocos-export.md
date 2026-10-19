# Cocos Creator export

Cocos Creator is available in the Export window, MCP (`engine: "cocos"`), project
profiles and export-on-save. Export into a folder inside a resources bundle, for
example `assets/resources/levels`. Cocos Creator's frame is the level's own, so
nothing is converted.

The export writes `<level>.json`, the GLB models, the sky as six PNG faces and
`HeronBrushLevel.ts`. The component is the same for every level; keep levels in
one folder so the project has one copy. Add it to a node and set **Level** to
the level file's bundle path without extension (`levels/motion_lab`). It builds:

- models from the GLBs (Cocos imports their materials and textures);
- colliders from the level's collision, made at runtime: world geometry as a
  static triangle `MeshCollider`, movers as kinematic convex bodies that follow
  their animation, trigger volumes as triggers;
- keyframe animation and trigger volumes, fired by the **Activators** nodes;
- with **Environment**, the sky as the scene's skybox and image-based light, the
  scene's directional light as the sun with shadows, and the map's fog;
- water volumes (`water(point)`, each `HeronWater`'s `velocity`), a see-through
  surface, and with **View** set to the camera, fog under water (see
  [water export](water-export.md)).

Two Cocos details the runtime handles: colliders are configured while their
nodes are inactive, and the script avoids spreading iterators, because Cocos
compiles project scripts to ES5, where `[...set]` makes a one-element array.

## Native verification, 18 October 2026

Cocos Creator 3.8.8 (`CocosCreator-v3.8.8-mac-121518.zip` from the official
download page) on macOS 26.5, Apple M4, physics `physics-ammo` (Bullet). A
project copied from the bundled `hello-3d-world` template was built for
web-desktop from the command line and run in Microsoft Edge:

```sh
CocosCreator.app/Contents/MacOS/CocosCreator --project /abs/project --build "platform=web-desktop;debug=true"
```

`examples/engine-checks/cocos/` holds the export script, the test bootstrap
(`HeronCheck.ts`, placed in `assets/check`) and the run script.

| Map | Result |
| --- | --- |
| motion_lab | 10 entities, 9 models, 9 colliders (4 triggers, 4 kinematic), 4 animations, 60 fps. A ball rested at y = 0.37 on the start pad; a ray hit the floor at 0. Walking into `func_button_3` played `open`, ending at (6.625, 2, 4.5) with a 90° Y turn, as in Stride and Three.js. |
| lantern_lane | Textures, models, water and sky; 64 entities, 66 colliders; ball on the floor at 0.5. |
| relay_yard | Gradient paint on walls and road; ball settled on the floor. |
| sky | Seamless cube; the sun's glow in the sun's direction. |

The Creator GUI was not opened, so this verifies the scripts, assets and runtime
in a web build, not the editor's inspector. Native mobile/desktop builds remain
unchecked.
