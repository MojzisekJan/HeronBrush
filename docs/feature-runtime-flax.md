# Feature lab in Flax

The [feature matrix](feature-matrix.md) checks the Flax export of
`demos/feature_lab/feature_lab.map` in the files. This page is the engine run: the
generated C# importer builds the level in the Flax editor, and a check plugin
tests it feature by feature with PhysX and the imported assets.

Run on 23 October 2026 on Linux with Flax 1.12.6912 (PhysX 5.1.3, .NET 10.0.12),
from the extracted build in `~/Downloads/flax`. After one importer fix, all 77
checks passed both headless and on the GPU. Flax drew the level with Vulkan.

## Rerun

```sh
cargo build -p heron-brush
python3 examples/engine-checks/flax/run_feature_lab.py --root /tmp/feature-runs
python3 examples/engine-checks/flax/run_feature_lab.py --root /tmp/feature-runs --no-export --gpu
```

The script exports the feature lab for Flax with `export_feature_lab.py` and writes
the editor's poses of `fl_mover` with the editor_core test
`writes_feature_lab_mover_poses_for_engine_checks`. It then makes a project at
`<root>/heron-flax-feature-project`: the export under `Content`, the generated
scripts and `examples/engine-checks/flax/feature_lab/FeatureLabCheck.cs` under
`Source/Game`. The editor starts with `-headless -null -heronfeaturelab`. The
plugin calls `HeronBrushLevel.ImportModels()` and `HeronBrushLevel.Build()`, runs
the checks, logs `FEATURE_LAB PASS` or `FAIL` lines and exits. The log is
`<root>/flax-feature-runtime.txt`.

With `--gpu` the editor opens on the display instead. After the same checks it
points the editor viewport at 14 views and saves each with `Screenshot.Capture`
to `<root>/flax-feature-shots/`. The editor runs with `XDG_CONFIG_HOME` in the run
folder, so it leaves the person's `~/.config/Flax` alone. `FLAX_EDITOR` picks
another editor binary.

## Result

| Feature | Result | Evidence |
| --- | --- | --- |
| Colliders from inline hulls | PASS | `fl_solid`, `fl_clip`, the mover and the door have one `ConvexMesh` collider each; rays stop on the tops at 2.25 and 3.25 m exactly |
| Colliders from triangles | PASS | The worldspawn and the `func_detail` stations have `TriangleMesh` colliders; `fl_hidden`'s hidden face still collides at its plane |
| Rigid bodies on those colliders | PASS after the fix | A dropped 40 cm box rests on the clip (3.45 m), the solid, `fl_hidden` and the door, also after the level root is turned off and on |
| Sensors | PASS after the fix | `fl_trigger`, `fl_button` and `fl_water` are trigger colliders; each reports one `TriggerEnter` and one `TriggerExit` for a body passing through; rays ignore them |
| Skip | PASS | `fl_skip`'s model has 10 triangles and its collision has no top: a ray from above passes through to the floor, and its sides still collide |
| Hidden face absent | PASS | `fl_hidden`'s imported model has 10 triangles, `fl_solid`'s 12; the screenshot from the hidden side shows the station behind |
| Animation component | PASS | `HeronBrushAnimation` on the mover: 240 frames of playback within 2.4e-7, 11 seeks within 1.4e-7 |
| Door from trigger and button | PASS | `HeronBrushAnimationTrigger` with a player inside `fl_trigger` opens the door a quarter turn; the button closes it and opens it again; its collider moves with it |
| Water component | PASS | `HeronBrushWater` samples 75 cm depth and swim speed 2.5 inside, nothing above, priority 3 |
| Textures, colours and gradients through the GLB import | PASS | All 27 material slots sample an imported texture; the 35 images Flax extracted equal the export's source images pixel for pixel |
| Normal maps | PASS | `brick_wall` samples its colour and its normal map; the imported model samples its normal map |
| PBR maps of the imported model | Partly | The material samples the colour, metallic-roughness, normal and emissive maps. Flax's import drops the occlusion map |
| Properties | PASS | `charge` int 7, `armed` bool, `sequence` an array of three ints, the string with a comma, float and int types on the light, camera and water |
| Links | PASS | The probe, both waypoints, the button and the trigger name actors that exist at their map positions; both triggers hold `fl_door` as target |
| Shared model | PASS | Both `heron_model` actors use one imported `Model` |

### What was checked and how

**Collision.** The check reads each `MeshCollider`'s `CollisionData.Options.Type`.
Rays use `Physics.RayCast` with triggers off. Flax's PhysX setup has no visible
margin: the hits land on the brush faces to the centimetre. The dropped boxes and
the sensor visitors are real rigid bodies stepped with `Physics.Simulate`.

**Door.** The importer's trigger script tests the player position against the
exported volume planes rather than waiting for PhysX events. The check drives it
the way `OnFixedUpdate` does, calling `Advance` with a player position each 1/60 s.
It sends no animation calls to the door itself.

**Animation.** The mover plays from rest and advances 1/60 s at a time. Its
position and three axes match the editor's pose at every frame. Two frames needed
the pose a nanosecond earlier: the summed clock reads 2.9999999999999942 at frame
180, just short of the held key at 3 s, and the editor at exactly 3 s is past it.
The pose file carries both, so the check accepts either.

**Materials.** A Flax material made by the glTF import keeps its textures as graph
nodes, not parameters. The check lists the assets each material references. The
emissive paint face of `fl_surface` references its texture twice, as colour and as
emission. The roughness, metallic and emission factors sit in the material graph
and were not read.

**Rendering.** On the GPU run the editor compiled every material without an
error. The views show the six textures, the five paints, the four gradients, the
bricks with their normal map, the translucent water surface and both model copies.
An earlier attempt that exited right after building logged
`Unknown shader source file './Flax/Common.hlsl'` while the editor shut down. With
the editor left open for the views, no such error appeared. That is probably the
failure [target parity](target-parity.md) put down to the shader include setup.

## Fix

- `bf8eaea` Spawn the Flax level root active so its colliders simulate. The
  importer spawned the level root inactive and turned it on at the end. In Flax
  1.12 a collider spawned under an inactive parent answers raycasts but takes no
  part in the simulation. Before the fix a dropped box fell through every station
  to the floor, and no sensor reported `TriggerEnter`. A collider spawned the same
  way under an active parent worked, which pinned it down. The motion lab check
  missed it because Flax's `CharacterController` moves by scene queries. Test:
  `colliders_are_spawned_under_an_active_root`.

## Open

- Flax's glTF import leaves out the occlusion map. The GLB carries it; Flax does
  not extract the image.
- The material factors (roughness, metallic, emission strength 3 from
  `KHR_materials_emissive_strength`) were not checked. Reading them needs the
  material graph or a pixel comparison under controlled light.
- The headless run uses the Null renderer, so the material checks there are about
  references, not pixels. The `--gpu` run covers what is drawn, by eye.
- The runner now points the editor's config at the run folder. Before that, the
  first runs of this check wrote the check project's path into
  `~/.config/Flax/LastProject.txt`, replacing what was there.
