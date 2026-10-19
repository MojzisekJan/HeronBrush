# Cocos Creator export

Written for Cocos Creator 3.8. Export into a folder inside a resources bundle, for
example `assets/resources/levels`, so the level loads by path at runtime.

- `<level>.json`: every entity with its transform, typed properties, targets, tags
  and collision.
- `<level>/*.glb`: the models. Cocos imports them with their materials and textures.
- `<level>/sky_*.png`: the editor's sky as six cube faces.
- `HeronBrushLevel.ts`: the loader component. It is the same for every level, so
  keep the levels in one folder.

Add **HeronBrushLevel** to a node and set **Level** to the level file's path in the
bundle without its extension, for example `levels/motion_lab`. When the scene starts
it builds the level under that node:

- models from the GLBs;
- colliders, with **Physics** on: world geometry as a static triangle mesh, movers as
  kinematic convex bodies that follow their animation, trigger volumes as triggers;
- the editor's keyframe animation, and trigger volumes that start it when one of the
  **Activators** (usually the player) walks in. `activate(name)` presses a button;
- with **Environment** on: the editor's sky as the scene's skybox and image-based
  light, the scene's directional light turned into the editor's sun, and the map's fog;
- water volumes: a see-through surface, `level.water(point)` for the volume a point
  is in and how deep, each `HeronWater`'s `velocity` for swimming, and with **View**
  set to the game camera, the water's fog while the camera is under water.

From code, `await level.ready`, then use `level.entities` (nodes by name),
`level.records` (typed properties) and `level.animations`.

Nothing is converted: Cocos Creator's frame is the level's, right-handed with Y up,
in metres.
