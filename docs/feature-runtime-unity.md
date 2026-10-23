# Feature lab in Unity

The [feature matrix](feature-matrix.md) checks the Unity export of
`demos/feature_lab/feature_lab.map` in its files. This page is the engine run:
Unity 6000.3.24f1 on Linux, Built-in Render Pipeline, a fresh project, the
generated Build Level importer, then Play Mode. Every feature below passed after
the fixes listed further down.

Checked on 23 October 2026 with `examples/engine-checks/unity/VerifyFeatureLab.cs`.
The edit-mode and Play Mode checks ran with `-batchmode -nographics`. The captures
ran with the GPU (AMD Radeon 610M, OpenGL 4.6 core).

## Rerun

```sh
cargo build -p heron-brush
Unity -batchmode -nographics -quit -createProject /tmp/heron-unity-feature -logFile /tmp/unity-create.log
python3 examples/engine-checks/unity/prepare_feature_lab.py /tmp/heron-unity-feature
Unity -batchmode -nographics -projectPath /tmp/heron-unity-feature -executeMethod VerifyFeatureLab.Run -logFile /tmp/unity-feature.log
Unity -batchmode -projectPath /tmp/heron-unity-feature -executeMethod VerifyFeatureLab.Render -logFile /tmp/unity-feature-render.log
```

`Unity` is `~/Unity/Hub/Editor/6000.3.24f1/Editor/Unity`. The licence was already
activated. The prepare script exports the map through the editor's MCP into
`Assets/Levels/heron-feature-unity`, installs the check, and writes
`feature_lab_reference.json` from the ignored test
`portable_animation::tests::feature_lab_editor_reference`. That file is the
editor's side of the comparison:

- the mover's and the door's poses at sample times, from the editor's own
  preview transform (`Track::transform`), as four map points per pose;
- the colour the editor gives points inside every face of the texture, colour,
  gradient, surface and normal-map stations: the texel under the face's UV, the
  paint sampler's colour, or the two blended for translucent paint.

`Run` exits 0 and logs `HERON_FEATURE_LAB PASS` when every feature passes. The
lines below come from `feature-lab-unity.txt`, which it writes into the project.

## Result

| Feature | Result | Evidence from the engine |
| --- | --- | --- |
| Build Level importer | PASS | Builds `feature_lab` with all 24 entities; no warning or error logged during the build |
| OBJ and MTL import | PASS | 15 OBJ meshes; all 36 material slots carry the MTL's names, each with its `feature_lab_paint` PNG as albedo and a working shader |
| Textures and UVs | PASS | 7 surfaces hold the source pixels and repeat; UV spans 1 x 1, 1.414 x 1.414, 2 x 0.5 and 0.5 x 0.5; 111 points on 12 faces sample the same texel the editor shows |
| Face colours | PASS | 304 points on 6 faces match the editor, largest difference 0/255, the translucent face included; paint images clamp |
| Gradients | PASS | 360 points on 6 faces, sampled bilinear as Unity draws them, within 2/255 of the editor's continuous gradient |
| Surface materials | PASS | 4 `ApplySurface` materials on Standard: metallic, smoothness = 1 - roughness, emission 3 on the albedo map with `_EMISSION` on; albedo unchanged at 80 points |
| Brush normal maps | PASS | `_BumpMap` is the 512 x 512 `brick_wall` map imported as NormalMap, `_NORMALMAP` on, pixels equal to the OpenGL source |
| Solid collision | PASS | `fl_solid` has a convex 2 x 2 x 2 m MeshCollider and a ray down stops on its top at y = 2.25; the worldspawn floor has a triangle MeshCollider and stops a ray at y = 0 |
| Clip brushes | PASS | `fl_clip` has no renderer and a solid convex 2 x 3 x 2 m MeshCollider; a ray stops on its top at y = 3.25 |
| Skip faces | PASS | `fl_skip` draws 5 sides and its triangle MeshCollider covers the same 5; a ray down through the skipped top passes to the floor |
| Export-hidden faces | PASS | `fl_hidden` draws 5 sides; its triangle collision covers all 6 |
| Trigger volumes | PASS | `fl_trigger` draws nothing and has a convex trigger MeshCollider; in Play Mode a CharacterController walking in gets `OnTriggerEnter` from `trigger_multiple` |
| Keyframe animation | PASS | 16 poses of the mover (linear X, held Z, bezier turn, 4 s loop) and the door's open clip match the editor within 0.000002 m; in Play Mode the mover autoplays |
| Door, button and trigger | PASS | Entering the trigger plays `open` and the door stops at the editor's open pose; then entering the button toggles to `close` and the door returns to rest; both poses within 0.002 m of the editor's, measured 0 |
| Lights | PASS | Point light 2.5, range 8, colour 255 200 120; Spot light 3, range 10, cone 60 degrees, shining along (-0.35, 0.87, 0.35), the direction the editor gives angles `-60 45 0` |
| Water | PASS | `HeronBrushWater` reads swim_speed 2.5 and reports the centre 0.75 m deep; trigger collider; 2 upward surface triangles; in Play Mode the surface draws with `HeronBrush/Water`, and `HeronBrushWaterView` turns on exponential squared fog of density 0.18 in the water tint under the surface and restores it above |
| Typed properties | PASS | `HeronBrushEntity.entityJson` keeps `charge` int 7, `armed` bool, `sequence` [2, 3, 5], the comma string, the light's float energy and colour, the camera's float fov and int `current`, the water's float and int |
| Target links | PASS | `fl_wp_a` to `fl_wp_b`, `fl_probe` to `fl_wp_a`, trigger and button to `fl_door` name scene nodes; both trigger components hold the door's `HeronBrushAnimation` |
| Imported models, shared | PASS | Both `heron_model` copies draw the same mesh assets, at scales 32 and 48, the second turned 45 degrees |
| Model PBR maps | PASS | `ApplyPbr` gives the model materials metallic-smoothness, NormalMap-type normal, occlusion and emission maps |
| Sky and fog | N/A | `Target::exports_sky` is false for Unity, as before |

The render pass drew every station from two sides into `captures/`. They show the
textures, the brick normal map shading its mortar lines, the saturated emissive
face, the translucent water in its tint and the painted colours. `VerifyFeatureLab.Render` calls the water component's `Start` itself,
because edit mode skips it.

A second `Run` over the same project, with the generated materials and collision
assets already present, passed as well.

## Fixed

The first engine run failed three features. Each fix is in the generated importer
or the Unity export, with a unit test. Commit `9eb3012` on `runtime-unity`.

- **Surface emission switched itself off.** `ApplySurface` copied the OBJ
  material, which carries the GI flag EmissiveIsBlack, and enabled `_EMISSION`.
  Unity's Standard shader editor derives `_EMISSION` from that flag when it
  validates a material, so the saved material ended with the keyword off and the
  emissive face did not glow. In a fresh project this happened during the first
  build. The importer now sets the flag to BakedEmissive when emission is above
  zero, as `ApplyPbr` already did. Test:
  `generated_materials_get_normal_maps_clamped_paint_and_a_lasting_glow`.
- **Paint images repeated.** Unity imports every PNG with Repeat wrap. The
  contract says baked paint clamps, and a repeating bake blends its far edge
  into its near one in the smaller mip levels. Build Level now sets the importer
  of each generated paint image (not tiled textures) to Clamp. Same test.
- **No normal maps on brush textures.** The export now writes each generated
  material's companion normal map as `<material>_normal.png` beside its albedo,
  names it on the MTL's `norm` line, and has Build Level call `ApplyNormal`. That
  imports the image as a NormalMap texture and sets `_BumpMap` with
  `_NORMALMAP` on the material, reusing the `ApplySurface` material when there is
  one. The map comes from the same `normal_png` as the GLB targets, so every
  suffix the editor pairs (`_normal_gl`, `_normal`, `_n`, `_normal_dx`) arrives,
  and a `_normal_dx` map has its green channel inverted to OpenGL's +Y, which is
  Unity's convention. Tests:
  `unity_gets_brush_normal_maps_beside_the_albedo_in_the_opengl_convention`
  (a DirectX map, checked flipped in the written PNG) and the importer test
  above. The file checker no longer reports Unity's brush normal maps as N/A.

## Open

- **Live link.** The Unity live addon builds its materials from the same MTL
  but has no `ApplySurface`, `ApplyNormal` or clamping step, so surface
  overrides, brush normal maps and paint clamping reach only the final export.
  Its MTL leaves out the `norm` line, since it does not write the images.
- **Painted faces lose the normal map.** A face with translucent paint becomes
  a baked image without one, the same as on the GLB targets.
- **Render pipelines.** The run used the Built-in pipeline. `ApplyNormal` picks
  URP Lit under URP, as `ApplySurface` does, but URP and HDRP were not run.
- **Sky and fog** stay out of the Unity export.
- **Unity's OBJ importer** ignores the `norm` line: the imported material has no
  `_BumpMap` until Build Level runs. The line is there for people and other MTL
  readers.
