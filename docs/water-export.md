# Water volumes across engine targets

Godot, Bevy, Three.js, Babylon.js, Defold, Unity, Flax, Stride, Cocos Creator, GDevelop and
raylib export a common optional water adapter.
Create a brush entity `water_volume`, or add `water_enabled=1` to a custom brush
class. Existing `fps_water` and `func_water` names are recognized too. Explicit
`water_enabled=0` disables the adapter. A texture named water alone is not a
water volume. The built-in definition exposes its controls in the inspector;
Water Lab is included in the example chooser.

The generated `_heron_water` property contains version-1 JSON with entity-local
convex planes, one surface height per brush, optional surface geometry and the
parameters below. Geometry and speeds are in metres in this payload, including
when Flax placements use centimetres. Each enabled water collider is a sensor.
Visuals and collision stay separate: the baseline draws upward faces of the
brushes as a double-sided transparent surface, while swimming uses the complete
closed volume. Other faces can also be authored with skip as usual.

| Property | Default | Meaning |
| --- | --- | --- |
| `swim_speed` | 3 | Horizontal speed in m/s |
| `rise_speed` | 3.5 | Rise/dive speed in m/s |
| `sink_speed` | 0.8 | Idle sinking speed in m/s |
| `water_drag` | 8 | Exponential velocity response per second |
| `water_priority` | 0 | Higher priority wins in overlapping volumes |
| `water_color` | `0.05 0.38 0.48` | RGB components in 0..1 |
| `water_opacity` | 0.55 | Surface alpha |
| `water_fog_density` | 0.18 | Underwater effect density |
| `wave_height` | 0.035 | Cosmetic wave amplitude in metres |
| `wave_speed` | 1.5 | Cosmetic wave rate |
| `water_surface_material` | 1 | Use the baseline surface and material |

Invalid numeric values fall back to defaults; finite ranges are bounded. Queries
use the exact plane intersection, not a volume AABB. With equal priorities the
more deeply containing volume wins. Sampling every step also handles teleport,
exit, overlapping volumes and moved/scaled/rotated level parents without relying
on paired physics entry/exit callbacks. Movement samples the player body; camera
queries are separate so the underwater effect starts at the camera, not the feet.

Set `water_surface_material=0` to preserve the full authored visual mesh and game
material. Water queries, sensor geometry and swimming parameters remain usable.
Game-owned players can call the adapters directly; an FPS class or our optional
walker is not required. Cosmetic waves do not move the sensor boundary, simulate
fluids, float arbitrary rigid bodies or implement currents.

## Target adapters

| Target | Surface | Player and view integration |
| --- | --- | --- |
| Godot 4.5 | Upward faces with a generated `ShaderMaterial` (tint, the face's texture, waves; blended, both sides) | The volume becomes an `Area3D` with a convex shape per brush, whatever its class says, and a `heron_water` child in the `heron_water` group with `sample`, `velocity`, `priority`, `tint` and `fog_density`. `<level>/heron_water_view.gd` has `find` (priority, then depth, across volumes) and, put under a Camera3D, gives it the water's fog while it is under water and its own environment back afterwards. The entity's own script stays on the `Area3D`. |
| Stride 4.4 | The GLB's blended surface becomes a material of its own with `MaterialTransparencyBlendFeature` and no culling; no waves | `HeronBrushStrideWater` on each volume (`Sample`, `Velocity`, `Find`), `BuiltLevel.Waters` and `Water(point)`. `HeronBrushStrideWaterView` on the camera drives the compositor's `Fog` post effect and restores it; `HeronBrushStrideLevelLoader` adds it to the scene's first camera (`WaterView`). The collider is a Bullet trigger. |
| Cocos Creator 3.8 | Cocos ignores glTF blending, so `HeronBrushLevel` gives the surface a transparent, double-sided `builtin-standard` material that keeps its texture; no waves | `HeronWater` per volume (`sample`, `velocity`), `level.water(point)` and `level.waters`. With **View** set to the game camera, the scene fog and the camera's clear colour become the water's while it is under water, and go back afterwards. The collider is a trigger. |
| GDevelop 5.6 | `KeepOriginal` keeps the GLB's blending; the event adds the wave to the surface's three.js material and the surface casts no shadow | `runtimeScene.__heronLevels[name]`: `water(point)` and `swim(data, velocity, walk, rise, dive, dt)` in the level's frame and metres, `waterAt(object)`. While the camera is in water the three.js fog and background become the water's. No physics body, as with triggers. |
| raylib 5.5 | `<level>_water.h`'s shader: the surface waves and is drawn see-through from both sides after the opaque models | `<level>_water.h` holds the volumes as C data: `HbWaterFind`, `HbWaterSample` and `HbWaterVelocity` in plain C, for a loader without a JSON parser. After `raylib.h`, `HbWaterLoadShader`, `HbWaterView` and `HbWaterDrawModel` draw the level through one shader that also fogs it while the camera is under water, as the generated preview does. The `.level` collision line says `sensor`. |
| Bevy 0.19 | Transparent StandardMaterial and private animated mesh copies | `heron_water.rs` supplies queries/velocity without Avian; `heron_water_render.rs` adds waves and per-camera DistanceFog. The Bevy sample wires its Avian player and restores gravity on exit. |
| Three.js | MeshStandardMaterial with a wave vertex hook | Both generated HTML and the independent JSON importer swim with Space/Ctrl and apply underwater fog. HTML scene updates replace water adapters together with the staged scene. `heron_water.js` is exported for custom consumers. |
| Babylon.js 9 | The GLB's `PBRMaterial`, alpha-blended and two-sided, with a material plugin that waves upward faces in the vertex shader | `HeronWater` in `heron_babylon.js` (`sample`, `HeronWater.sample`), `level.water(point)`; the volume is a Havok trigger. The generated page swims with Space/Ctrl and gives the view the water's fog. |
| Defold | Generated shader and separate alpha-blended `heron_water` pass | Generated properties scripts register/unregister volumes. `<level>_water.lua` exposes `sample` and `velocity`. The sample player uses held Space/Ctrl and the optional GUI tint adapter. |
| Unity | `HeronBrushWater.shader`, transparent double-sided waves | Importer attaches `HeronBrushWater`. Optional MotionPlayer.Step adds a `dive` argument after `jump`; Water/Underwater state is exposed. Assign the level's water components to a HeronBrushWaterView on your camera for the baseline fog adapter. |
| Flax | Transparent imported material and a private animated virtual mesh | Importer attaches HeronBrushWater; queries convert cm/reflected Z correctly. Optional MotionPlayer.Step adds `dive`. Assign waters to HeronBrushWaterView on the camera for the screen tint adapter. |

For Bevy declare both exported modules, add HeronWaterPlugin from
`heron_water_render`, attach HeronWater to the entity root, and use a unique mesh
copy with HeronWaterSurface as the sample does. Mark cameras with
HeronWaterCamera. The query-only module does not depend on a renderer or physics
plugin. See `demos/bevy_fps/src/main.rs` and `physics.rs`.

Defold's custom render scripts must draw the `heron_water` predicate after opaque
models with alpha blending, depth test on, depth writes off and culling off.
The generated `<level>.render` is an optional minimal 3D/GUI renderer, which also draws the sky,, not a
replacement to apply blindly to a project's custom pipeline. A `defold_material`
override owns its own uniforms; generated scripts do not write baseline uniforms
to it. The GUI adapter accepts `heron_water_view {color=vector4(...)}` messages;
custom games can use the sample's camera query or supply their own effects.

Unity's baseline shader/fog is checked with the built-in render pipeline. URP,
HDRP, split-screen and custom post-processing should use project-owned materials
and camera effects through the same query API. Flax's baseline virtual mesh uses
the supplied local geometry rather than waiting for resident source vertex
buffers; its arrays are reused. Its simple tint is a single-view adapter. Custom
pipelines can replace it without changing exported volumes.

These are a usable common baseline, not identical water rendering across engines.
Godot Lantern Lane's depth-aware shader, foam, refraction and underwater shader
are separate demo assets; they have not been ported to the other pipelines. Surface
normals remain the authored normals during the small cosmetic displacement.

## raylib check, 18 October 2026

raylib 5.5 (Homebrew, OpenGL 3.3) on macOS 26.5, Apple M4, with `demos/water_lab`
exported through the editor's MCP: the generated preview and its water header
compile with `-Wall -Wextra` and no warnings. A check program built on the
preview's own code passed the same queries as the other targets (two volumes,
depth 2 m, overlap by priority, dry points, swim, rise and sink), a falling
point sinking through the volume to the floor and rising back, and drew the
see-through surface from above and the fogged view below. The contract test
compiles and runs the plain-C queries against a fresh export.

## Stride, Cocos Creator and GDevelop checks, 18 October 2026

`demos/water_lab` exported through the editor's MCP on macOS 26.5, Apple M4, in
Stride 4.4.1 (Vulkan through MoltenVK, Bullet), Cocos Creator 3.8.8 (web-desktop
build in Microsoft Edge, Bullet) and GDevelop 5.6.284 (HTML5 build in Edge). In
each: both enabled volumes found and the disabled one left out; depth 2 m at 2 m
down; the priority-10 volume winning the overlap; points above the surface and
just past a side plane dry; swim, rise and idle-sink velocities. In Stride and
Cocos a dynamic ball fell through the volume onto the floor (y = -3.75). The
camera was fogged under water and restored on leaving. Screenshots showed the
see-through surface from above and the fog below; in Cocos only after the
runtime material, since its importer had drawn the surface opaque.

## Godot check, 18 October 2026

Godot 4.5 (Forward+, Metal) on macOS 26.5, Apple M4, with the contract test's
fixture exported as a Godot scene: two enabled volumes found and the disabled one
left out; depth 2 m at 2 m under the surface; the priority-10 volume winning the
overlap; points above the surface and just past a side plane dry; swim, rise and
idle-sink velocities; a falling capsule reported by the `Area3D`, sinking through
it onto the floor and rising back to the surface; the camera fogged under water
and restored on leaving; the same depth with the level moved and turned 90
degrees. `demos/water_lab` exported through the editor's MCP rendered with the
transparent surface above water and the fog below, the surface visible from
underneath.

Exporting Lantern Lane again would give its `fps_water` the baseline surface in
place of the committed scene's material; set `water_surface_material=0` on it to
keep that.

## Checks on 15 October 2026

- Core/model/renderer/export regression: **1,459 passed, 14 ignored**,
  `/tmp/heron-water-regression-complete.log`; desktop viewer and WASM checks passed.
- Fresh export integration checks all five payloads, custom opt-in, explicit
  disable, overlap priority, complete convex sensors, top-only surface indices,
  custom-material opt-out and transparent double-sided GLB material flags.
- Bevy/Avian: native dynamic capsule rises/dives/swims, passes through water,
  restores gravity on exit and samples transformed roots. Existing headless
  regression: 7 passed, 3 ignored; water native check passed separately.
- Three.js: actual Chromium in-app browser, offline vendored libraries, both HTML
  and independent JSON entry points. Swim/rise/dive/sink/exit, overlap, parent
  transforms and the surface adapter passed. Surface and underwater views were
  visually inspected. Standalone Playwright reproduction requires an existing
  Chromium installation; no browser was downloaded for this work.
- Unity 6000.3.24f1: native built-in pipeline shader compilation, sensor colliders,
  movement, overlap, transformed parent and fog restoration passed. Nographics
  validates shader/material state and physics, not a rendered image.
- Flax 1.12.6912: native PhysX, overlap, movement, transformed parent, virtual
  surface creation/update and camera tint state passed with Null rendering.
  A Vulkan attempt reached the graphics device but the installed engine failed
  shader includes at `./Flax/Common.hlsl`, including its own editor materials.
  A rendered Flax water image remains unverified; no engine files were changed.
- Defold 1.13.2: actual resource/shader compiler and OpenGL/Bullet runtime passed
  the exported volume and sample player's rise/dive/sink/swim/exit checks.

Logs: `/tmp/heron-water-bevy-final.log`, `/tmp/heron-water-bevy-regression.log`,
`/tmp/heron-water-unity-final.log`, `/tmp/heron-water-flax-final.log`,
`/tmp/heron-water-defold-build-final.log`, `/tmp/heron-water-defold-final.log`.

Reproduce the fixture without installed engines:

```sh
cargo test -p editor_core --lib water_volume_contract --offline -- --nocapture
python3 examples/engine-checks/prepare_water.py
python3 examples/engine-checks/threejs/serve_water_check.py
```

The server prints HTML and independent importer URLs. Both display their actual
controller check result. Core exports and prepared projects stay under
`/tmp/heron-water-check`; `demos/water_lab/water_lab.map` is the bundled source example.

```sh
CARGO_BUILD_JOBS=2 cargo test --manifest-path examples/bevy-motion-check/Cargo.toml \
  --target-dir demos/bevy_fps/target exported_water_native --offline -- --ignored --nocapture
/path/to/Unity -batchmode -nographics -projectPath /tmp/heron-water-check/unity \
  -executeMethod VerifyWater.Run -logFile /tmp/heron-water-unity-final.log
/path/to/FlaxEditor -headless -null -project /tmp/heron-water-check/flax -exit -std -heronwater
```

Build Defold with the installed Bob compiler, then run its compiled project with
`--config=check.enabled=4`. The native checks never modify user maps or require
an online service. Engines, SDKs and packages were not downloaded or installed.
