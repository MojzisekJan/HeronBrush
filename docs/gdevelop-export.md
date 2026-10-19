# GDevelop export

GDevelop is available in the Export window, MCP (`engine: "gdevelop"`), project
profiles and export-on-save. Export into the folder holding the project's
`game.json`. Without one, the export starts a project there. With one, it
replaces only the scene named after the level and the resources under
`<level>/`; other scenes, objects, events and resources remain.

GDevelop's frame is X right, Y down the screen, Z up, in pixels, at its physics
scale of 100 pixels per metre. A level position (X, Y, Z) in metres becomes
x = 100X, y = 100Z, elevation = 100Y. Rotations become GDevelop's angle,
rotationY and rotationX of the same rotation with Y and Z swapped.

| Written | GDevelop side |
| --- | --- |
| One `Scene3D::Model3DObject` per mesh | Rotated 90° about X and sized to the GLB's bounds including its origin, so the model stays where the level put it; `KeepOriginal` materials |
| `<level>/<entity>_collision.glb` + hidden object | `Physics3D` static `Mesh` body for world geometry, including collision the eye does not see |
| Movers | `Physics3D` kinematic `Box` fitted to the hull's bounds |
| `HeronPoint` | An invisible 3D box holding each point entity's place |
| Instance variables | Every typed property, plus `heron_name`, `heron_classname`, `heron_targets`, `heron_tags` |
| Layer effects | The sun as `DirectionalLight` with shadows, the sky as `Skybox` (six faces, environment light), AgX `ToneMapping`, and `ExponentialFog` when the map asks for fog |
| JavaScript event | Keyframe animation, trigger volumes and water, camera at `info_player_start` |

Anything with a true `HeronActivator` variable walks into trigger volumes.
Game code reaches the level through `runtimeScene.__heronLevels[name]`:
`play(entity, clip)`, `activate(trigger)`, the entities' objects, and water:
`water(point)`, `waterAt(object)` and `swim(...)` (see [water export](water-export.md)). Keep your
own events in another sheet or external events; re-export replaces the scene.

GDevelop 5.6 has no sensor or convex-hull body, and a mesh body is static only.
Trigger volumes are tested by the event, not by physics, and movers collide as
boxes. Static triangles are wound the other way round in the collision GLB:
the model's fit mirrors Y, and Jolt's triangles are one-sided.

## Native verification, 18 October 2026

GDevelop 5.6.284 (Homebrew cask) on macOS 26.5, Apple M4. The CLI loads and
exports a project without the GUI:

```sh
"/Applications/GDevelop 5.app/Contents/MacOS/GDevelop 5" /abs/game.json --run-command EXPORT_HTML5_EXTERNAL
```

`examples/engine-checks/gdevelop/` holds the export script, a probe that drops a
Physics3D ball and walks into a trigger, and the run script, which exports the
build and runs it in an isolated Microsoft Edge window.

| Map | Result |
| --- | --- |
| motion_lab | 12 objects; static mesh + 4 kinematic bodies. A ball settled at elevation 0 on the floor and 12 px on the 0.12 m start pad. Walking into `func_button_3` played `open`: the door ended at (662.5, 450, 200) px with angle −90°, the level's (6.625, 2, 4.5) m and 90° turn. |
| lantern_lane | Textures, imported models, sky and sun shadows from the player start; 67 objects, 14 bodies. |
| relay_yard | Gradient face paint and models; AgX keeps light surfaces from clipping. |
| sky | The sun's glow at camera angle 45°, the direction of the sun (+X, −Z). |

The GUI editor opened the lantern_lane project, its objects and typed instance
variables without warnings. The first native run found the collision winding
and the sky's Z mirror; both are fixed and covered above.
