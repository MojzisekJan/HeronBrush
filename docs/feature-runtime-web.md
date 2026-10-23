# Feature lab at run time: three.js, Bevy, raylib and portable glTF

[The feature matrix](feature-matrix.md) checks the feature lab's exports in the
files. This page runs them: three.js 0.186 in Chromium, Bevy 0.19 with Avian 0.7,
raylib 6.0, and the portable GLB and glTF through three.js's GLTFLoader. Each
check reads the export the way a game reads it, and the animation checks compare
against poses the editor itself computes.

Checked on 23 October 2026 on Linux (x86-64, AMD Radeon 610M, Mesa), with the
editor on branch `runtime-web` from `b6c63f1`. Chromium is Playwright's Chrome for
Testing 156 with SwiftShader WebGL.

## Rerun

```sh
cargo build -p heron-brush
python3 examples/engine-checks/export_feature_lab.py --root /tmp/fl three bevy raylib obj gltf glb
python3 examples/engine-checks/feature_lab_poses.py /tmp/fl/poses.json --text /tmp/fl/poses.txt

# three.js, the example loader, the playtest page, GLB and glTF
node examples/engine-checks/threejs/verify_feature_lab.cjs /tmp/fl /tmp/fl/poses.json /tmp/fl/out

# Bevy: Avian, triggers, animation and water without a window, then the glTF loader
HERON_BEVY_FEATURE=/tmp/fl/heron-feature-bevy HERON_FEATURE_POSES=/tmp/fl/poses.json \
  cargo test --manifest-path examples/bevy-motion-check/Cargo.toml exported_feature_lab -- --ignored --nocapture
HERON_BEVY_FEATURE=/tmp/fl/heron-feature-bevy \
  cargo test --manifest-path demos/bevy_fps/Cargo.toml exported_feature_lab_glbs -- --ignored --nocapture

# raylib: the .level reader and generated headers, then LoadModel, the sky and a picture
make -C examples/engine-checks/raylib feature-check RAYLIB=~/.cache/heron-brush/raylib-6.0 \
  FEATURE=/tmp/fl/heron-feature-raylib POSES=/tmp/fl/poses.txt SHOT=/tmp/fl/out/raylib.png
```

`feature_lab_poses.py` asks the editor for its poses through MCP `sample_animation`:
fl_mover's loop every third of a second and fl_door's `open` clip every sixth. For
each pose it moves four probe points (the pivot and 64 units along each axis) the
way the editor moves the brushes, in the export's frame. The engine checks move the
same rest points with their runtime's pose of the entity and compare.

The three.js check needs Node with `playwright`. This machine has no Node on the
path; Python's Playwright package carries one, see
[the three.js check's README](../examples/engine-checks/threejs/README.md).

## Summary

| Feature | three.js | Bevy | raylib | GLB and glTF |
| --- | --- | --- | --- | --- |
| Textures and UVs | PASS | Picture | PASS | PASS |
| Face colours, clamped paint | PASS (fixed) | PASS (sample fixed) | PASS | PASS (names fixed) |
| Gradients | PASS | Picture | Picture | PASS |
| Surface materials, emissive strength | PASS | PASS | Data only | PASS |
| Brush normal maps with tangents | PASS | PASS (sample fixed) | Data only | PASS |
| Collision: hulls, triangles, clip, skip, hidden | PASS | PASS | PASS | Not in the format |
| Trigger as a sensor that opens the door | PASS (fixed) | PASS (fixed) | PASS (fixed) | Volume in extras (new) |
| Animation against editor poses | PASS | PASS | PASS | Not in the format |
| Sky and fog | PASS | Picture | Sky PASS, fog numbers only | Not in the format |
| Water | PASS | PASS | PASS | Surface only, volume in extras (new) |
| Imported model, PBR maps, shared mesh | PASS | PASS | PASS | PASS |
| Typed properties | File check | File check | File check | PASS |

"File check" means only [the feature matrix](feature-matrix.md) covers it.
"Data only" means the engine loads the data but its default shader does not use
it. "Picture" means the rendered picture shows it and no check measured it.
"(fixed)" marks failures found by this run and fixed on this branch.

## three.js

`verify_feature_lab.cjs` serves the export and the repository's vendored
three.js 0.186 (HDRLoader, which the web build does not vendor, comes from
jsDelivr at the same version). `feature_lab_checks.js` assembles the level as a
game does: one group per entity at its transform, `GLTFLoader` for each GLB,
`heron_animation.js` and `heron_water.js` from the export, an `Octree` from the
level JSON's collision. It also runs the example loader in `src/` and the
generated `feature_lab.html` with `?verify`. Results:

- Textures and UVs: PASS. The six textures on `fl_textures` come through
  GLTFLoader with `RepeatWrapping`, sRGB, the source PNG's exact pixels and the
  face's UV span: 1 by 1, 2 by 1/2, the square root of 2 for the 45 degree face,
  1/2 for the face scaled by 2.
- Face colours: PASS. Five exact colours, and the translucent face equals the
  wood texture blended with the paint at 128/255 within 4/255. Every paint map
  clamps.
- Gradients: PASS. All stops of the four gradients are in their baked maps, which
  clamp.
- Surface materials: PASS. Roughness and metalness match;
  `KHR_materials_emissive_strength` gives `emissiveIntensity` 3 with the paint as
  the emissive map.
- Normal maps: PASS. `normalMap` is the 512 by 512 image, linear, the geometry
  has the GLB's `TANGENT`, and after a render the linked shader program reads the
  `tangent` attribute.
- Collision: PASS. Rays and a dropped capsule stop on `fl_solid`'s inline hull
  top (y 2.25). The clip brush draws nothing and stops a ray at 3.25. A ray from
  above goes through `fl_skip`'s skip top to the floor; its four walls and bottom
  stop rays. `fl_hidden` blocks on every side, the hidden face included. The
  trigger is not in the Octree.
- Animation: PASS. `HeronAnimation` matches 12 editor poses of fl_mover (linear,
  hold and bezier keys) within 4e-15 m and 7 poses of the door's `open` clip.
- Door, button and trigger: PASS. Through `HeronAnimationTrigger`, a point inside
  the trigger fires it, the door reaches the editor's open pose after 1.2 s,
  re-entering inside the 1.5 s wait does not fire again, and the button toggles
  the door to `close`. The playtest page and the example loader failed this with
  a player standing in the trigger; see below.
- Sky and fog: PASS. `HDRLoader` reads `sky.hdr` (1024 by 512), blue at the top
  and warm at the horizon. `FogExp2` has density 0.02 and colour 180 190 200.
  These are the editor's linear light values (`writer/sky.rs`), as Bevy reads
  them too.
- Water: PASS. `HeronWater` finds the volume's centre 0.75 m under the surface,
  with swim speed 2.5 and priority 3, and nothing above it. The GLB draws two
  upward triangles; the adapter makes them blended and see-through.
- Imported models: PASS. Both copies name one GLB; its materials carry base
  colour, normal (scale 0.35), metal-roughness, occlusion (strength 0.25) and
  emissive maps.

Pictures: `three-lab.png` (the JSON and GLB route, lit by `sky.hdr`) and
`three-playtest.png` (the generated page) show every station: the textured cube,
the colour, gradient, surface and brick cubes, the water as a see-through surface
and no trigger box.

### Fixed

- `59e216e` The playtest page tested only the player's feet against trigger
  volumes. The feature lab's trigger brush starts 0.25 m above the floor, so a
  player standing in it never opened the door. The page now tests the feet, the
  middle and the head. `14948f3` does the same for the example loader's
  `advance(dt, feet, height)`.
- `59e216e` The page replaced every GLB map with a texture set to repeat, so
  baked paint lost its clamp and could bleed at face edges. The level's
  `textures` entries now carry `"clamp": true` for baked paint, and the page and
  the example loader use it. The example loader used to clamp every `bbpaint_`
  image, which also clamped textures tinted for a surface material. Test:
  `baked_paint_is_marked_clamped_in_the_level_textures`, and
  `the_playtest_tests_triggers_along_the_whole_player`.

## Bevy

`demos/bevy_fps/src/feature_check.rs` runs in the headless
`examples/bevy-motion-check` crate: it spawns every entity of `feature_lab.json`
with an Avian collider built from its GLB's collision mesh, as the motion lab
check does, and the exported `heron_animation.rs`, `heron_entities.rs` and
`heron_water.rs`. A second test in `demos/bevy_fps/src/main.rs` loads the GLBs
through Bevy's own glTF loader with rendering switched off.

- Collision: PASS. Avian ray casts stop at `fl_solid` 2.25, `fl_clip` 3.25 and
  `fl_hidden` 2.25, pass `fl_skip`'s missing top (to its bottom face at 0.25) and
  pass the trigger to the floor. The trigger's and the water's colliders are
  `Sensor`s.
- Animation: PASS. `heron_animation.rs` matches the editor's fl_mover and door
  poses within 2.1e-6 m (f32).
- Trigger: PASS after a fix. Avian's sensor reports the player standing in the
  trigger. With the exported `HeronActivator` at the player's feet alone the door
  stayed closed, 2.8 m from its open pose. With the new `HeronActivatorHeight(2.0)`
  it opens to within 1.2e-7 m of the editor's open pose.
- Water: PASS. `HeronWater::sample` gives 0.75 m at the volume's centre and none
  above it.
- GLB materials through Bevy's glTF loader: PASS. With rendering switched off,
  `func_detail_4.glb` gives the 512 by 512 normal map as a linear image and a
  mesh with `ATTRIBUTE_TANGENT`; the emissive paint's `emissive` is 3 (the glTF
  factor times `KHR_materials_emissive_strength`) with its map; all six paint
  images on `fl_colours` sample with `ClampToEdge`; both model materials have
  normal, metal-roughness, occlusion and emissive maps.

`bevy.png` is the sample viewer (`--fly --at -20,16,-8 --look 22,-16,-1 --shot`)
on the export, with `heron_sky.rs` lighting it from `sky.ktx2` and the water
surface drawn by `heron_water_render.rs`. The stations, the brick's normal-mapped
relief, the water as a see-through surface and no trigger box are all there. Its
fog is heavier than three.js's at the same density: Bevy's `DistanceFog` uses
`FogFalloff::Exponential`, three.js's `FogExp2` the square of it.

### Fixed

- `18a6230` The generated `heron_animation.rs` tested a single activator point. The new
  optional `HeronActivatorHeight` component tests the feet, middle and top, and
  the sample puts it on its player.
- `18a6230` The sample viewer builds its materials from the level's `textures`. It did not
  load a brush texture's normal map (its comment said the GLBs carry no tangents,
  which is no longer true) and clamped every `bbpaint_` image by name. It now
  loads `maps`' `normal_texture` as a linear image and reads the new `clamp`
  flag.

## raylib

`examples/engine-checks/raylib/src/verify_feature_lab.c` (`make feature-check`)
compiles against the export's `feature_lab_animation.h` and `feature_lab_water.h`
and the sample's `src/level.c` and `src/collision.c`. It needs an OpenGL context
for the second half; X11 on this machine gave one, with a hidden window and an
off-screen render target.

- `.level` reader: PASS. `ctri`: 10 triangles for `fl_skip` (five sides) and 12
  for `fl_hidden` (six). `chull`: 8 points each for `fl_solid` and `fl_clip`; the
  clip brush has no `surface` line. The trigger and the water read as `sensor`.
- Collision: PASS. The sample's collider stops rays at 2.25, 3.25 and 2.25, lets
  one through `fl_skip`'s top and through the trigger to the floor, and a capsule
  settles on `fl_solid`.
- Animation: PASS. `HbPoint` matches all 76 editor probe points of fl_mover and
  the door exactly (0 m in doubles).
- Trigger: PASS after a fix. `HbCheckTriggers` with the feet on the floor misses
  the raised volume; the new `HbCheckTriggersSpan` with a 1.8 m player plays the
  door's `open` clip to 1 s.
- Water header: PASS. `HbWaterFind` gives 0.75 m at the centre (swim speed 2.5,
  priority 3) and -1 above the surface.
- `LoadModel`: PASS. `fl_textures` loads as 7 meshes with 7 materials, each with
  its 64 by 64 image. The five colours read back exactly from the GPU. The brick
  material has its 512 by 512 normal map and the mesh its tangents. The model has
  both materials with metal-roughness, normal, occlusion and emission maps. The
  clip brush's and trigger's collision-only GLBs give raylib no mesh to draw,
  because their meshes hang from no node.
- Normal maps and emission: data only. raylib's default shader uses neither, and
  raylib does not read `KHR_materials_emissive_strength`; the export already warns
  about this.
- Sky: PASS. `sky.png` (256 by 1536) becomes a 256 by 256 cube through
  `LoadTextureCubemap(CUBEMAP_LAYOUT_LINE_VERTICAL)`, blue at the top face.

`raylib.png` draws the level with the sky cube and the water header's shader: the
same stations as three.js, the water as a surface, no trigger box.

### Fixed

- `d2a6bb8` The generated header gains `HbCheckTriggersSpan`, which tests the
  feet, middle and head; the sample's `animated.c` uses it with its capsule's
  height. The committed demo and motion lab headers carry the same trigger code.
  Test: the C check in `raylib_exported_c_matches_editor_curves_pivots_modes_and_controls`.

## Portable GLB and glTF

`checkPortable` in `feature_lab_checks.js` loads `feature_lab.glb` and
`feature_lab.gltf` with GLTFLoader alone, as any glTF tool would.

- Textures: PASS. Every textured face has a repeating map.
- Colours: PASS. Five exact colour factors, now named `Paint #DC1E28` and so on
  (they were `paint:Rgb { r: 220, g: 30, b: 40 }:None`). The translucent face is a
  clamped baked map.
- Gradients, surface, normal maps: PASS. All stops present; roughness, metalness
  and emissive strength 3; a 512 by 512 normal map with tangents.
- Properties: PASS. GLTFLoader puts extras in `userData`: `heron.properties`
  keeps `charge` as int 7, `armed` true and `sequence` [2, 3, 5].
- Imported models: PASS. Both copies share one geometry and all PBR maps.
- Trigger: the trigger node has no mesh; its extras keep `target fl_door` and its
  six planes.
- Water: two upward triangles are drawn, and the volume is in the extras.

### Trigger and water volumes, changed

Before, OBJ, glTF and GLB drew the trigger as an opaque hazard-textured box and
the water as a closed six-sided cube. The portable writers ignored the class's
`visuals=false` and the water rules the engine targets follow. The change
(`ef263fe`), approved for this branch:

- A class whose build directives say `visuals=false` (the built-in
  `trigger_once` and `trigger_multiple`, and any project class with that binding)
  is left out of the OBJ. In glTF and GLB it becomes an empty node at the entity,
  with `extras.entity` and `extras.heron` as for any entity and
  `extras.heron.volume.brushes`: per brush, `planes` as `[x, y, z, d]` with
  outward normals (inside where `x*px + y*py + z*pz <= d`) and `vertices`, in the
  file's metres and Y-up axes. No `KHR_node_visibility`.
- A water volume draws only its upward faces with its own material, and its node
  carries the whole volume the same way. With `water_surface_material=0` it draws
  all its faces, as the engine targets do.
- No engine target reads the portable writers' output; they all go through the
  builder, which already handled both cases. `check_feature_lab.py` now fails a
  portable export that draws the trigger or more than the water's top.

Tests: `a_trigger_is_an_empty_node_with_its_volume_in_the_extras`,
`water_draws_only_its_top_and_keeps_its_volume_in_the_extras`,
`a_trigger_is_left_out_and_water_draws_only_its_top`.

### Also fixed

- `ef263fe` Every brush face in the OBJ was wound clockwise around its normal, so
  a viewer culling back faces showed the inside of each box. OBJ brush and patch
  faces now wind counter-clockwise. Test:
  `faces_wind_counter_clockwise_around_their_normals`.
- `ef263fe` Solid paint materials in glTF and GLB are named by colour, with the
  surface factors when there are any. Test: `solid_paint_materials_are_named_by_colour`.

## Open

- raylib draws neither normal maps nor emission with its default shader. A game
  needs its own lighting shader; `feature_lab_water.h`'s shader is a starting
  point for a single pass.
- The Bevy sample viewer still builds its own materials from the level's
  textures, so an imported model's PBR maps (in the GLB) are not drawn there. The
  level's `textures` entry for a model skin lists no maps. A game that takes the
  GLB's materials gets them, as the glTF loader test shows.
- The three.js and Bevy water GLBs keep all 24 corners of the volume in the
  vertex buffer while the index draws only the top two triangles. Harmless, a few
  hundred bytes.
- three.js's `FogExp2` falls off with the square of distance times density,
  Bevy's `Exponential` and Godot's fog without the square, so one map's fog is
  thinner in three.js. `FogFalloff::ExponentialSquared` would match three.js.
  The editor's 3D view draws no fog, so which falloff `fog_density` means is a
  decision for the format.
- The three.js and Bevy fog colours treat the map's 0..1 colour values as linear
  light, as the editor's view does (`writer/sky.rs`). Godot's `Color` reads the
  same numbers as sRGB. Whether `fog_light_color` means sRGB or linear is a
  decision for the format.
