# Export feature matrix

Which export target carries which map feature, checked in the exported files.
The fixture is `demos/feature_lab/feature_lab.map`: one small station per
feature, each an entity whose `targetname` starts with `fl_`. The checker reads
every target's output with the Python standard library and reports PASS, FAIL or
N/A per feature. N/A means the target's own documentation or code says it does
not carry the feature; the reason names where.

A PASS here means the data is in the files and right. It does not mean an engine
imports it, draws it or simulates it. The last section lists what still needs an
engine run.

Checked on 23 October 2026, on Linux, with the editor at commit `68b9294`. The
Babylon.js column was added on the same day, from `493d040` on branch `target-babylon`,
and the PlayCanvas column from `d54d95d` on branch `target-playcanvas`.

## Rerun

```sh
cargo build -p heron-brush
python3 examples/engine-checks/export_feature_lab.py
python3 examples/engine-checks/check_feature_lab.py --markdown /tmp/feature-matrix.md
```

The export goes through the real editor binary over MCP, one folder per target
under `/tmp/heron-feature-<target>` (`--root` picks another parent). Each folder
is emptied first, and the fixture's textures are copied into it as a game project
would hold them. The editor runs with scratch XDG folders. Both scripts take
target names to run only some, for example `godot bevy`. The checker exits with
status 1 on any FAIL.

`python3 demos/feature_lab/build.py` writes the map again from its description,
through the MCP animation and trigger tools.

## Result

Updated from fresh file exports on 28 October 2026. These results inspect written
assets; native evidence and limits are in [the local audit](export-audit-2026-10-28.md).

| Feature | Godot | Defold | three.js | Babylon.js | PlayCanvas | Bevy | raylib | Unity | Unreal | Flax | Stride | Cocos | GDevelop | OBJ | glTF | GLB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Textures and UVs | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Face colours | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Gradients | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Surface materials | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Brush normal maps | PASS | N/A | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Solid collision | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Clip brushes | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Skip faces | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Export-hidden faces | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Trigger volumes | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Sky and fog | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Keyframe animation | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Door, button and trigger links | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Typed properties | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS |
| Target links | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS |
| Lights | PASS | N/A | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | N/A | N/A | N/A | N/A | N/A |
| Water | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Imported models, shared | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Model PBR maps | PASS | N/A | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS |

Before the fixes below, the first run had 18 failures: face colours on all 14
targets, the GDevelop clip brush, the Unreal clip brush and trigger, and the
imported models in glTF and GLB.

The editor has no `nodraw` texture. Skip is its no-draw face: not drawn, and
left out of triangle collision. That is the skip row.

## What each check reads

Each target is read the way its engine reads it: Godot's `.tscn` and the OBJ
meshes it names, Defold's `.model` textures rather than the GLB's own, Stride's
`.sdmat` factors, raylib's `.level` text and C header, GDevelop's `game.json`
and the data its generated event runs, the Unity and Unreal importer scripts.
The other targets read the level JSON and the GLBs beside it.

- **Textures and UVs.** `fl_textures` has six textures on one cube. Each surface
  must use the source image's exact pixels and repeat. The UV span over each
  64-unit face must match the face's scale and rotation (1, 1/2 by 2, the square
  root of 2 for 45 degrees). The offset face must sit 16 by 8 texels off its
  plain neighbour, and the face turned 90 degrees must grow U along the other
  axis than the plain grass top.
- **Face colours.** Five opaque colours must be exact, in an image or a factor.
  The translucent face must be the wood texture blended with the paint at
  128/255, within 4/255 per channel over the whole image. Paint images clamp.
- **Gradients.** The two-colour, three-stop, radial and black-to-white gradients
  must be baked into images holding all their stop colours. The two-colour one
  must run along a single image axis.
- **Surface materials.** The four overrides, roughness, metallic and emission,
  must be in the material factors or the level JSON's `surface` entries.
- **Brush normal maps.** `brick_wall` has `brick_wall_normal` beside it. The
  material must carry a 512 by 512 normal map, and GLB primitives tangents.
- **Solid collision.** `fl_solid` must have a 2 m solid shape, not a sensor, and
  the worldspawn floor collision.
- **Clip brushes.** `fl_clip` must draw nothing and have a solid 2 by 3 by 2 m
  shape.
- **Skip faces.** `fl_skip`'s top is `skip`. The render mesh must cover five
  sides of the box. A triangle collision mesh must cover five as well. A convex
  hull covers the whole brush, as a hull must.
- **Export-hidden faces.** One face of `fl_hidden` is marked hidden. The render
  mesh must cover five sides and the triangle collision all six.
- **Trigger volumes.** `fl_trigger` must draw nothing and have a sensor shape.
  GDevelop has no sensor body; its generated event must carry the volume.
- **Sky and fog.** The sky image the target uses must exist and be referenced,
  and the fog must have density 0.02 a metre and colour 180 190 200.
- **Keyframe animation.** `fl_mover` loops for 4 s, with linear X keys 0, 128, 0
  at 0, 2 and 4 s, held Z keys 0, 32, 0 at 0, 1 and 3 s and a bezier Z turn from
  0 to 90 degrees, and plays on start. Godot's native AnimationPlayer tracks,
  raylib's C tables, Defold's baked script and the shared `_heron_animation`
  payload are compared key by key. The target's runtime file must be there.
- **Door, button and trigger links.** `fl_door` has `open` and `close` clips and
  does not play on start. The button toggles it, the trigger opens it with a
  1.5 s wait, and both have an activation volume.
- **Typed properties.** `fl_probe` is a project class with defaults from the
  export's class binding: `charge` 9 overridden to the int 7 in the map, the bool
  `armed`, the array `sequence` and two strings, one with a comma. The light's
  float energy and colour, the camera's float and int and the water's float and
  int must keep their types.
- **Target links.** Four links must resolve to the right entities: the
  waypoints, the probe and the button and trigger aimed at the door.
- **Lights.** A native point and spot light with the map's colour, energy, range
  and cone. For Babylon.js and PlayCanvas, the level JSON's `light` blocks that
  `heron_babylon.js` and `heron_playcanvas.js` turn into lights.
- **Water.** The `_heron_water` payload with the volume and `swim_speed` 2.5, a
  sensor, a surface of upward faces only, and the target's water adapter file.
- **Imported models, shared.** Two copies of `models/pbr.glb` must share one
  mesh, each with its own scale and turn, and no file may name the author's
  source folder.
- **Model PBR maps.** The model's material must carry its metallic-roughness,
  normal, occlusion and emissive maps.

## Not claimed

| Feature | Targets | Why |
| --- | --- | --- |
| Sky and fog | Unreal, OBJ, glTF, GLB | `Target::exports_sky` is false; [sky export](sky-export.md) says so |
| Keyframe animation, door links | Unreal, OBJ, glTF, GLB | The export warns that there is no playback controller |
| Water | Unreal, OBJ, glTF, GLB | Not in [water export](water-export.md)'s adapter list |
| Lights | Defold, Stride, Cocos, GDevelop, OBJ, glTF, GLB | These default adapters retain light properties without generating common-light rendering. Three.js, Bevy, Flax and raylib now have explicit adapters; see [common entities](common-entities.md). |
| Brush normal maps | Defold default material | Project shaders can opt into `defold_sampler_normal`; the default sample uses albedo. Unity, Stride and OBJ now transfer companion maps. |
| Model PBR maps | Defold default material, OBJ | Defold project samplers are opt-in. OBJ carries several PBR maps but has no full glTF/AO material equivalent; see [model PBR](model-pbr.md). |
| Collision rows | Unreal | The importer spawns meshes and lights only; collision stays in the GLBs and JSON |
| Collision rows | OBJ, glTF, GLB | Geometry formats; clip and skip faces are left out of the mesh as required |
| Properties, links | OBJ | Geometry only |

Defold and raylib keep the surface factors in the GLB and level JSON, and the
export warns that the default shaders do not implement full PBR. Stride writes roughness and
metalness, emission, normal maps, metallic/roughness maps and AO into native
materials. The Stride PBR additions passed writer regressions and native YAML
loading on 28 October 2026; the table above includes the new asset checks. See [Stride export](stride-export.md).

## Fixed

- `367d7b8` Keep the last texel row and column under translucent face paint. The
  bake's last row and column sat on the face's far edge and took the texel of
  the next tile, so a 64-unit face over a 64-texel texture lost its last texel
  column and row on every target. Test:
  `translucent_paint_keeps_every_texel_of_a_face_one_tile_wide`.
- `4b36bab` Give GDevelop clip brushes a collision body. A convex solid got its
  box body only on the object that draws it, so a clip brush became a bare
  point and did not block. Test:
  `a_clip_brush_that_draws_nothing_still_gets_a_hidden_box_body`.
- `5913462` Spawn no Unreal mesh actor for a brush that draws nothing. The import
  script imported the clip brush's and the trigger's collision-only GLBs as
  static mesh actors. Test: `a_collision_only_glb_is_not_imported_as_a_mesh`.
- `68b9294` Name imported model materials in glTF by file, not by full path. The
  portable glTF and GLB named the model's materials after its absolute source
  path. Test: `imported_model_materials_are_named_without_the_source_folder`.

## Remaining native and rendering work

- Unreal has no installed supported editor here; its collision/gameplay and sky
  adapters still need implementation and native verification.
- Defold's default sample shader does not become full PBR. Explicit project
  sampler bindings are supported and checked with Bob.
- Raylib's forward preview shader has no shadow maps or complete PBR/IBL path.
- Flax needs a rendered check, native imported-model AO mapping and a homogeneous
  fog adapter. Its sky,
  common lights and the rest of the feature lab passed the native Null renderer.
- Stride needs a native rendered project on this machine. Its material assets
  passed actual Stride YAML loading, not shader compilation or rendering.
- Cocos Creator and GDevelop have no supported native installation found here.
  Earlier macOS evidence remains documented on their target pages.

Godot, Unity, Defold, Three.js, Babylon.js, PlayCanvas, Bevy, raylib and Blender
were exercised using their installed runtimes. The audit distinguishes full
feature-lab runs, component state checks, asset loading and rendered smoke checks.
Reproduction commands remain in [engine checks](../examples/engine-checks/README.md)
and [the local audit](export-audit-2026-10-28.md).
