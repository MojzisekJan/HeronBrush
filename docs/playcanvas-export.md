# PlayCanvas export

Choose **PlayCanvas** in the export dialog and export to a folder. This is the
open-source PlayCanvas engine loaded as one script, not the PlayCanvas cloud editor.
The export writes:

| File | What it is |
| --- | --- |
| `<level>.html` | A playtest page. Open it in a desktop browser and click **Play**. |
| `<level>.json` | The level: the same file as the Babylon.js target's, the three.js level with a `light` block on each light entity |
| `<level>/*.glb` | One mesh per solid entity and imported model, with its textures |
| `<level>/sky.hdr` | The map's sky as an equirectangular panorama, laid out for PlayCanvas |
| `heron_playcanvas.js` | An ES module that loads the level into your own PlayCanvas app |

The page holds the meshes, a 256 by 128 copy of the sky and the runtime, so it needs
no local server. It loads PlayCanvas 2.23.1 from jsDelivr's npm mirror and the
ammo.js build the engine ships, from the engine's repository at tag `v2.23.1`
(`examples/assets/wasm/ammo/`), so the browser needs internet access.

## The playtest page

- WASD moves; the mouse looks around.
- Space jumps; Shift runs.
- E presses the button you look at, within three metres.
- R returns to the start; Esc pauses and releases the mouse.
- If mouse capture is unavailable, **Play with drag-to-look** keeps keyboard
  movement and lets you drag the mouse to turn.

PlayCanvas has no character controller, so the page makes one: an ammo.js capsule
body 56 map units tall and 16 wide, with the eyes at 50, scaled by the export's
units per metre. The page sets its velocity every frame. Five rays down from the
capsule's middle find the floor, and the capsule closes the gap to it, so the feet
rest on the floor rather than on a contact margin, and the player steps up onto
anything up to 18 map units high, as in Quake. The floor's own motion over the last
step is added, so the player rides moving platforms. Walls stop the capsule
through ammo.js contacts; it has no friction, so it slides along them. Place an
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

`heron_playcanvas.js` needs PlayCanvas 2, and ammo.js for collision. It takes the
`pc` namespace as an argument, so the UMD build and the `playcanvas` package both
work. Load ammo.js before the app, or the rigidbody system has no world; `loadLevel`
also starts it if `Ammo` is ready by then.

```js
import {loadLevel} from './heron_playcanvas.js';

globalThis.Ammo = await Ammo({locateFile: file => ammoFolder + file});
const app = new pc.Application(canvas);
const level = await loadLevel(pc, app, 'arena.json', {
  factories: {
    fl_probe(record, level) {
      // record.values.charge is 7, record.targets are entity records
    },
  },
});
app.on('update', dt => {
  const feet = player.getPosition();  // your player
  level.update(dt, [['player', feet], ['player', feet.clone().add(new pc.Vec3(0, 1.75, 0))]]);
});
app.start();
```

`loadLevel(pc, app, source, options)` takes the level's URL, or
`{level, meshes, sky}` with the parsed level, its GLBs by mesh path (bytes or URLs)
and the sky panorama (a URL or the `.hdr`'s bytes). PlayCanvas is right-handed with
Y up and -Z forward, which is the level's own frame, in metres at 32 map units a
metre. Nothing is converted.

Options:

- `parent`: an entity to put the level under. Place it before loading: bodies take
  their place when they are made.
- `factories`: callbacks by the `playcanvas_factory` property or by classname,
  called with `(record, level)` once every entity exists and every target is
  resolved, so forward references work. A callback may return a cleanup function.
- `lights: false` leaves out the entities' light components; `sky: false` leaves out
  the sky, sun, fog and tone mapping.

The returned level has:

- `root`, an `Entity` with one child per entity at its transform.
- `entities`, one record per entity: `name`, `classname`, `targetname`, `node` (the
  entity), `meshes` (its mesh instances), `properties` (typed, as in the level file),
  `values` (the same as plain JSON), `targets` (entity records), and `body`, `light`,
  `animation`, `trigger` and `water` where the entity has them. `node.heron` is the
  record too.
- `byName`, `animations` (by entity name), `triggers`, `waters`, `lights`, `bodies`.
- `update(dt, actors)` advances the triggers, animations and water waves. `actors`
  are `[id, point]` pairs in world space; one id may give several points.
- `water(point)` gives `{data, depth}` for the water at a point, or null.
- `raycast(from, to, triggers)`: the first body a ray meets, passing trigger volumes
  unless `triggers` is true.
- `dispose()`.

### What becomes what

| Level data | PlayCanvas |
| --- | --- |
| Entity transform | `Entity`: position, a quaternion from `rotation_yxz` (Y, then X, then Z), scale |
| `mesh` | The GLB as a `container` asset, instantiated with `instantiateRenderEntity` under the entity. Copies of one GLB share meshes and materials |
| Materials | The GLB's `StandardMaterial`s: sRGB diffuse maps, repeating, and clamped baked paint and gradients; metalness and roughness (`gloss` with `glossInvert`); `KHR_materials_emissive_strength` as `emissiveIntensity`; normal maps with the GLB's tangents; an imported model's occlusion and emissive maps |
| `collision.triangles` | A `mesh` collision shape (a Bullet triangle mesh) |
| `collision.hulls` | A `mesh` collision with `convexHull`, one hull per brush, with no margin |
| Triangles and hulls together | A `compound` collision with one child entity for each |
| `collision.sensor` | A collision component with no rigidbody, which PlayCanvas makes a trigger volume: rays and bodies pass, and `triggerenter` fires when a body enters |
| Collision of an animated entity | A `kinematic` rigidbody, which ammo.js moves with the entity every physics step |
| Other collision | A `static` rigidbody |
| `_heron_animation` | `HeronAnimation`: linear, hold and bezier keys, pivots, once, loop and ping-pong, named clips |
| `_heron_trigger` | `HeronAnimationTrigger`: play, open, close, toggle, pause, resume and stop, the wait and once-only rules; `activate()` for a button press |
| `_heron_water` | `HeronWater`: depth queries; the surface's material becomes blended, two-sided and waving through its `transformVS` shader chunk |
| `light` | An `omni`, `spot` (outer cone the half-angle, no inner cone) or `directional` light component on a child entity turned so that it shines down the entity's -Z, with inverse-square falloff and `range` |
| `environment` | `sky.hdr` through `EnvLighting` as the skybox and environment atlas, a directional light from `to_sun`, ACES tone mapping on the app's cameras at the map's exposure, `FOG_EXP2` fog |

PlayCanvas lights shine down their entity's -Y, so each light sits on a child entity
turned 90 degrees about X. Mesh and convex collision follow the entity's scale, so
the shapes take the level's points as they are.

PlayCanvas gives a convex hull a 1 cm margin, and Bullet adds a margin outside the
hull's points, so a 2 m brush would collide as 2.02 m. The runtime sets the margin of
the hulls it makes to zero.

PlayCanvas has no strength for image-based light. The map's ambient intensity dims
the light and not the sky, so the runtime makes the environment atlas from a copy of
the panorama scaled by it, and the skybox from the panorama itself.

Light energy is passed on as PlayCanvas's `intensity` without calibration, as the
Unity, Unreal and Babylon.js targets do. The sun is the map's sun intensity over pi,
which is how the editor's view and three.js light a diffuse surface. PlayCanvas has
no AgX tone mapping, so the sky is tone mapped with ACES and looks a little more
contrasted than in the editor.

`<level>/sky.hdr` is not the same image as the three.js or Babylon.js target's:
PlayCanvas reads a panorama half a turn round from Babylon.js, and the PlayCanvas
export bakes it that way.

## Not covered

- Entity scripts, enemies and other gameplay are your game's.
- Shadows. The lights' `shadows` flag is in the level data; the runtime turns no
  shadows on.
- A spot light's softness: the cone fades from its axis to its edge.
- The waves move every vertex of the water's surface. The baseline surface is the
  volume's upward faces only, so this is the same as the other targets; a custom
  water mesh with sides would wave its sides too.
- The water shader chunk is GLSL. Under WebGPU the water is still see-through but
  flat.
- The playtest page is not the editor's live playtest. That one is the three.js page.

Run-time checks of every feature are in
[feature runtime checks for PlayCanvas](feature-runtime-playcanvas.md).
