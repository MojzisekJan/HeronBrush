# Babylon.js export

Choose **Babylon.js** in the export dialog and export to a folder. It writes:

| File | What it is |
| --- | --- |
| `<level>.html` | A playtest page. Open it in a desktop browser and click **Play**. |
| `<level>.json` | The level: the three.js level file, with a `light` block on each light entity |
| `<level>/*.glb` | One mesh per solid entity and imported model, with its textures |
| `<level>/sky.hdr` | The map's sky as an equirectangular panorama, laid out for Babylon.js |
| `heron_babylon.js` | An ES module that loads the level into your own Babylon.js scene |

The page holds the meshes, a 256 by 128 copy of the sky and the runtime, so it needs
no local server. It loads Babylon.js 9.29.0, its glTF loader and Havok 1.3.14 from
jsDelivr, so the browser needs internet access.

## The playtest page

- WASD moves; the mouse looks around.
- Space jumps; Shift runs.
- E presses the button you look at, within three metres.
- R returns to the start; Esc pauses and releases the mouse.
- If mouse capture is unavailable, **Play with drag-to-look** keeps keyboard
  movement and lets you drag the mouse to turn.

The player is Havok's `PhysicsCharacterController`: a capsule 56 map units tall and
16 wide, with the eyes at 50, scaled by the export's units per metre. It stands on
static and moving collision, rides moving platforms and stops at walls. Place an
`info_player_start` (or `info_player_deathmatch`, `player_start`, `fps_player`) to
choose the start; without one the player starts above the middle of the level, on
the first surface below. With collision export off, the page flies: Space goes up
and left Ctrl down.

Doors, buttons and triggers work. A trigger fires when the player's feet, middle or
head enters it, so a trigger brush that starts above the floor still catches a
player standing in it. Keyframe animation plays as the editor shows it. In water
the player swims at the volume's speed, Space rises, Ctrl dives, and the view takes
the water's colour and fog.

## Using the level in your own game

`heron_babylon.js` needs Babylon.js 9 with the glTF loader, and Havok for collision.
It takes the `BABYLON` namespace as an argument, so the UMD build and the ES packages
both work.

```js
import {loadLevel} from './heron_babylon.js';

const engine = new BABYLON.Engine(canvas, true);
const scene = new BABYLON.Scene(engine);
scene.enablePhysics(new BABYLON.Vector3(0, -9.81, 0), new BABYLON.HavokPlugin(true, await HavokPhysics()));
const level = await loadLevel(BABYLON, scene, 'arena.json', {
  factories: {
    fl_probe(record, level) {
      // record.values.charge is 7, record.targets are entity records
    },
  },
});
scene.onBeforeRenderObservable.add(() => {
  const feet = player.position;  // your player
  level.update(engine.getDeltaTime() / 1000, [['player', feet], ['player', feet.add(new BABYLON.Vector3(0, 1.75, 0))]]);
});
```

`loadLevel(BABYLON, scene, source, options)` takes the level's URL, or
`{level, meshes, sky}` with the parsed level, its GLBs by mesh path (bytes or URLs)
and the sky's URL. It switches the scene to `useRightHandedSystem`, which is the
level's own frame: Y up, -Z forward, metres at 32 map units a metre. Nothing is
converted, and the glTF loader adds no mirroring root.

Options:

- `parent`: a node to put the level under. Place it before loading: static bodies
  take their place when they are made.
- `factories`: callbacks by the `babylon_factory` property or by classname, called
  with `(record, level)` once every entity exists and every target is resolved, so
  forward references work. A callback may return a cleanup function.
- `lights: false` leaves out the entities' native lights; `sky: false` leaves out
  the sky, sun, fog and tone mapping.

The returned level has:

- `root`, a `TransformNode` with one child per entity at its transform.
- `entities`, one record per entity: `name`, `classname`, `targetname`, `node`,
  `meshes`, `properties` (typed, as in the level file), `values` (the same as plain
  JSON), `targets` (entity records), and `body`, `light`, `animation`, `trigger` and
  `water` where the entity has them. `node.metadata.heron` is the record too.
- `byName`, `animations` (by entity name), `triggers`, `waters`, `lights`, `bodies`.
- `update(dt, actors)` advances the triggers, animations and water waves. `actors`
  are `[id, point]` pairs in world space; one id may give several points.
- `water(point)` gives `{data, depth}` for the water at a point, or null.
- `dispose()`.

### What becomes what

| Level data | Babylon.js |
| --- | --- |
| Entity transform | `TransformNode`: position, a quaternion from `rotation_yxz` (Y, then X, then Z), scaling |
| `mesh` | The GLB through the glTF loader, instantiated under the node. Copies of one GLB share geometry |
| Materials | The GLB's: albedo sRGB textures, repeating, and clamped baked paint and gradients; metallic-roughness; `KHR_materials_emissive_strength`; normal maps with the GLB's tangents; an imported model's occlusion and emissive maps |
| `collision.triangles` | `PhysicsShapeMesh` |
| `collision.hulls` | One `PhysicsShapeConvexHull` each, in a `PhysicsShapeContainer` when there are several |
| `collision.sensor` | `shape.isTrigger`: rays and bodies pass, and the plugin's `onTriggerCollisionObservable` reports bodies entering |
| Collision of an animated entity | An `ANIMATED` body that follows its node every physics step |
| Other collision | A `STATIC` body |
| `_heron_animation` | `HeronAnimation`: linear, hold and bezier keys, pivots, once, loop and ping-pong, named clips |
| `_heron_trigger` | `HeronAnimationTrigger`: play, open, close, toggle, pause, resume and stop, the wait and once-only rules; `activate()` for a button press |
| `_heron_water` | `HeronWater`: depth queries; the surface becomes alpha-blended, two-sided and waving through a material plugin |
| `light` | `PointLight`, `SpotLight` (cone twice the half-angle) or `DirectionalLight` under the node, shining down its local -Z, with the glTF falloff and `range` |
| `environment` | `sky.hdr` through `HDRCubeTexture` as the environment and skybox, a `DirectionalLight` from `to_sun`, ACES tone mapping at the map's exposure, `FOGMODE_EXP2` fog |

Havok bodies take a node's position and rotation, not its scale, so the runtime
bakes an entity's scale into its collision shapes. Brush entities have their
transform in their vertices already.

Light energy is passed on as Babylon's `intensity` without calibration, as the
Unity and Unreal importers do. The sun is the map's sun intensity over pi, which is
how the editor's view and three.js light a diffuse surface. Babylon.js has no AgX
tone mapping, so the sky is tone mapped with ACES and looks a little more contrasted
than in the editor.

`<level>/sky.hdr` is not the same image as the three.js target's: Babylon.js reads a
panorama a quarter turn round from three.js, and the Babylon.js export bakes it that
way.

## Not covered

- Entity scripts, enemies and other gameplay are your game's.
- Shadows. The lights' `shadows` flag is in the level data; the runtime makes no
  shadow generators.
- A spot light's softness: Babylon.js's glTF falloff fades from the axis to the cone's
  edge.
- The playtest page is not the editor's live playtest. That one is the three.js page.

Run-time checks of every feature are in
[feature runtime checks for Babylon.js](feature-runtime-babylon.md).
