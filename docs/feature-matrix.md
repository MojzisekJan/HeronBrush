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

Checked on 23 October 2026, on Linux, with the editor at commit `68b9294`.

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

| Feature | Godot | Defold | three.js | Bevy | raylib | Unity | Unreal | Flax | Stride | Cocos | GDevelop | OBJ | glTF | GLB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Textures and UVs | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Face colours | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Gradients | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Surface materials | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Brush normal maps | PASS | N/A | PASS | PASS | PASS | N/A | PASS | PASS | N/A | PASS | PASS | N/A | PASS | PASS |
| Solid collision | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Clip brushes | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Skip faces | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Export-hidden faces | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Trigger volumes | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Sky and fog | PASS | PASS | PASS | PASS | PASS | N/A | N/A | N/A | PASS | PASS | PASS | N/A | N/A | N/A |
| Keyframe animation | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Door, button and trigger links | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Typed properties | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS |
| Target links | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS |
| Lights | PASS | N/A | N/A | N/A | N/A | PASS | PASS | N/A | N/A | N/A | N/A | N/A | N/A | N/A |
| Water | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | PASS | PASS | N/A | N/A | N/A |
| Imported models, shared | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| Model PBR maps | PASS | N/A | PASS | PASS | PASS | PASS | PASS | PASS | N/A | PASS | PASS | N/A | PASS | PASS |

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
  and cone.
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
| Sky and fog | Unity, Unreal, Flax, OBJ, glTF, GLB | `Target::exports_sky` is false; [sky export](sky-export.md) says so |
| Keyframe animation, door links | Unreal, OBJ, glTF, GLB | The export warns that there is no playback controller |
| Water | Unreal, OBJ, glTF, GLB | Not in [water export](water-export.md)'s adapter list |
| Lights | all but Godot, Unity and Unreal | [Common entities](common-entities.md) lists native lights for those three; the others keep colour, energy and range in the entity data, which the check confirms |
| Brush normal maps | Unity, Defold, Stride, OBJ | Unity's OBJ path has no slot for them; Defold's `.model` binds one colour sampler; [Stride export](stride-export.md) lists them as not transferred; OBJ writes none |
| Model PBR maps | Defold, Stride, OBJ | As above; [model PBR](model-pbr.md) says the OBJ writer does not carry them |
| Collision rows | Unreal | The importer spawns meshes and lights only; collision stays in the GLBs and JSON |
| Collision rows | OBJ, glTF, GLB | Geometry formats; clip and skip faces are left out of the mesh as required |
| Properties, links | OBJ | Geometry only |

Defold and raylib keep the surface factors in the GLB and level JSON, and the
export warns that their default shaders ignore them. Stride writes roughness and
metalness into its materials but not emission, as its page says.

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

## Open

These pass or are N/A in the table, but are worth a decision:

- OBJ, glTF and GLB draw the trigger volume as a visible box and all six faces
  of the water volume. The portable writers do not consult the class's
  `visuals=false`, although glTF's `extras.heron.build` carries it. Leaving
  gameplay volumes out of a portable scene is a design choice, not a contained
  fix.
- Unity has no normal maps for brush textures. Its OBJ and MTL path would need a
  normal-map entry and the importer's material adapter would need to read it.
- Godot finds a brush texture's normal map only as `<texture>_normal` in the
  project's texture folder. The editor also pairs `_normal_gl`, `_n` and
  `_normal_dx`, and flips the green channel of the last; Godot's companion
  lookup does neither.
- Unreal's importer sets up no collision, triggers or animation. Its collision
  data is in each GLB's unreferenced second mesh and in the level JSON.
- The portable glTF names painted materials after the editor's internal key,
  such as `paint:Rgb { r: 220, g: 30, b: 40 }:None`. Harmless, but not a name a
  person would pick.

## What still needs an engine run

File checks cannot show these. For each installed engine:

- **Godot 4.7.1** (`~/.local/bin/godot`). The import of the OBJ meshes and of the
  scene's relative `textures/` resource paths. Clamped paint and the normal,
  roughness, metallic and emission settings as drawn. The ConcavePolygonShape3D
  without the skip face, and the trigger's Area3D `body_entered`. AnimationPlayer
  playback of the bezier and held tracks, and the button and trigger opening the
  door through `HeronAnimationTrigger`. The water shader and view script. The
  panorama sky and fog. The direction of the spot light at angles -60 45 0. The
  shared mesh resource of the two model copies.
- **Defold** (`~/.local/opt/Defold`). A Bob build of the `.model`, `.collisionobject`
  and generated scripts. Static hulls, trigger messages and the kinematic door.
  The baked animation script in `fixed_update`. The sky pass in `feature_lab.render`
  and the water pass. That `/main/model.material` with `tex0` draws every
  painted and textured surface.
- **Unity 6000.3.24f1** (`~/Unity/Hub/Editor`). The Build Level importer on this
  export: OBJ and MTL import with the paint PNGs, convex and triangle
  MeshColliders, trigger colliders, `HeronBrushAnimation` and its triggers, the
  Point and Spot lights' orientation, `ApplySurface` and `ApplyPbr` materials
  and the water shader.
- **Flax** (`~/Downloads/flax`). The generated importer building actors from the
  GLBs and `feature_lab.level.json`: colliders from the inline hulls and
  triangles, sensors, animation and water components, and whether its GLB import
  keeps the normal and PBR maps.
- **three.js** (through `~/.local/bin/chromium`). GLTFLoader on the GLBs: embedded
  textures, clamped paint samplers, normal maps with tangents and
  `KHR_materials_emissive_strength`. The Octree built from collision meshes and
  inline hulls, with the trigger as a sensor that opens the door. Animation poses
  against the editor's. `sky.hdr` through HDRLoader, FogExp2 and the water.
- **Bevy** (cargo, `demos/bevy_fps`). GLB loading with the normal map and emissive
  strength. Avian colliders from each GLB's collision mesh, sensors, and
  `heron_animation.rs` playback driven by `HeronAnimationTrigger`. The KTX2 sky
  from `heron_sky.rs` and the water adapters. Lights are entity data the game
  has to turn into Bevy lights.
- **raylib** (`~/.cache/heron-brush/raylib-6.0`). `LoadModel` on multi-material
  GLBs with their embedded images. The `.level` reader's `ctri`, `chull` and
  `sensor` lines, the C animation header against editor poses, the `sky.png`
  cube and `feature_lab_water.h`.

Stride, Cocos Creator and GDevelop have their own native checks on macOS (see
their pages); Unreal has none on this machine.
