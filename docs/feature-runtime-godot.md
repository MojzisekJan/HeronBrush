# Feature lab in Godot

The [feature matrix](feature-matrix.md) checks the exported files. This page
checks the Godot export of the same fixture, `demos/feature_lab/feature_lab.map`,
inside the engine: Godot imports it, loads the scene, steps physics, plays the
animations and draws it.

Checked on 23 October 2026 on Linux with Godot 4.7.1 (official build), Forward+ on
Vulkan with an AMD Radeon 610M.

## Rerun

```sh
cargo build -p heron-brush
GODOT=~/.local/bin/godot python3 examples/engine-checks/godot/verify_feature_lab.py --shots
```

The script exports the map for Godot through `export_feature_lab.py` into
`/tmp/heron-feature-godot` (`--root` picks another parent, `--no-export` reuses
the last export), writes a `project.godot` there and runs
`godot --headless --editor --import`. Then it runs
`examples/engine-checks/godot/verify_feature_lab.gd` headless in that project.
The script prints one PASS or FAIL line per check and exits with status 1 on
any failure. `--shots` then opens a window and saves two pictures per station
into `/tmp/heron-feature-godot-shots`: one as exported and one with the map's
fog turned off. That run needs a display and a GPU.

## Result

All 36 checks pass after the three fixes below.

| Feature | Result | What the engine showed |
| --- | --- | --- |
| Import | PASS | The scene loads. All 15 `MeshInstance3D` nodes get their OBJ mesh. 35 textures load, 10 of them through the scene's relative `textures/` paths. The import log has no errors. |
| Textures and UVs | PASS | Each of the 7 faces on `fl_textures` shows its source image pixel for pixel, with repeat on. UV spans are 1, 2 by 1/2, the square root of 2 and 1/2 as the face scales say. V falls as Y rises on the unrotated walls, so no image is upside down. The crate's U runs at 45 degrees, the dirt top runs across the grass top, and metal sits a quarter and an eighth of a tile off wood. |
| Face colours | PASS | The five opaque paints are exact. The translucent face is wood under the paint at 128/255, within 4/255 on every texel. Paint clamps, its UVs stay inside the image, and the material is opaque. |
| Gradients | PASS | The four gradient images import unchanged and hold all their stop colours. They clamp. |
| Surface materials | PASS (after fix) | Roughness, metallic and emission 0.2/0.9/0, 0.9/0/0, 0.65/0/3 and 0.05/1/0. The emissive face now glows in its own orange (see the fixes). |
| Brush normal maps | PASS (after fix) | `brick_wall` uses a 512 by 512 normal map with the pixels of `brick_wall_normal.png`. The mesh has tangents. In the render the bricks show relief. |
| Export-hidden faces | PASS | The render mesh has five sides, without the hidden one (Godot +Z). Its collision has all six, and a ray at the hidden side stops on it. |
| Skip faces | PASS | The render mesh of `fl_skip` has no top. A ray down through its top passes the brush and lands on the floor. A ray from the side stops on it. |
| Solid collision | PASS | A ray from above stops on `fl_solid`'s top at y 2.25. The worldspawn floor stops a ray at y 0. |
| Clip brushes | PASS | `fl_clip` stops a ray at its top, y 3.25, and has no visual node at all. |
| Trigger volumes | PASS | A `CharacterBody3D` walked into `trigger_multiple` raises `body_entered`. |
| Keyframe animation | PASS | `fl_mover` plays `default` on start, looping over 4 s. Seeked to 11 times from 0 to 4 s, its corners sit within 2 mm of where the editor's formula puts them: linear X, held Z with the step at 1 and 3 s, and the bezier turn. In 5 s of real play it loops and stays within 2 mm of the editor's pose for the player's clock. |
| Door, button and trigger links | PASS | The door waits closed. The trigger starts `open`, and 1.3 s later the door has turned 90 degrees about its pivot. The button toggles it to `close` and back to rest, and a second press opens it again. |
| Water | PASS | The wave `ShaderMaterial` is on a mesh of upward faces only. The volume script reports depth 0.75 at the centre, -1 above it, and `swim_speed` 2.5. `heron_water_view.gd` loads, fogs a camera inside the water at density 0.18 and gives the old environment back when the camera leaves. |
| Sky and fog | PASS | The background is the `sky.hdr` panorama, 1024 by 512. Fog density is 0.02 and the colour is 180 190 200. |
| Lights | PASS | The `OmniLight3D` has range 8 and energy 2.5 in the right colour and place. The `SpotLight3D` has range 10 and angle 30, and shines the way the editor's `facing` rule reads `angles -60 45 0`. |
| Imported models, shared | PASS | Both copies hold the same `ArrayMesh` object, at a scale ratio of 1.5 and turned 45 degrees. |
| Model PBR maps | PASS | Normal, metallic-roughness, occlusion and emissive maps load on the model's materials. |
| Typed properties | PASS | `fl_probe` has the int 7, the bool `armed`, the array `[2, 3, 5]` and both strings, the comma kept. The light, camera and water values keep float, int and Color. |
| Target links | PASS | All 4 `target_paths` resolve to the entity their `target` names. |

## Pictures against the editor

The same 15 views were drawn by the editor's own renderer (`render_wgpu`, with
preferences from an empty configuration) and by Godot. With the fog off, every
station matches the editor's picture face for face: which image is on which
face, its turn, scale and offset, the paint colours and the gradient
directions. The clip, trigger and skip top are not drawn in Godot, as intended.

Some differences remain, and they are not the export's material data:

- The map's fog of 0.02 a metre makes the exported level look paler than the
  editor's view, which does not draw fog.
- Godot's ambient light is a little stronger, so shaded sides are lighter.
- The water draws only its tinted top, over the wood texture, where the editor
  draws the whole volume.
- The test model's surfaces catch a specular highlight in Godot that the editor
  does not show.

The spot light shines upwards in both, because pitch -60 in `angles` means up.
That comes from the fixture, not the export.

## Fixed

- Paint the emissive face in its own colour. The scene gave an emissive face
  paint a white `emission` with its image as `emission_texture`, and left
  Godot's emission operator at Add. That gave white plus orange at strength 3,
  which drew as white. The operator is now Multiply, as the editor draws it.
  Test: `a_glowing_face_paint_glows_in_its_own_colour` in `scene_export`.
- Light a map without a render camera as the 3D view does. `map_sky` used a
  default render camera's sun and exposure when the map had none: sun azimuth
  135 and elevation 45. The 3D view uses the preferences, which default to 233
  and 46, so the exported shadows fell about 100 degrees away from the editor's.
  [Sky export](sky-export.md) already said the preferences apply. This affects
  every target that writes the sky. Test:
  `a_map_without_a_render_camera_takes_the_sun_and_exposure_of_the_3d_view`.
- Give Godot the normal map the editor pairs. The editor pairs `_normal_gl`,
  `_normal`, `_n` and `_normal_dx` (`map_model::material`), and flips the green
  channel of a DirectX map. The GLB targets already got that image from
  `paint_export::normal_png`. Godot looked only for `<texture>_normal` in the
  project. Now the export hands Godot the editor's map: the file itself when it
  is in the project and in the OpenGL convention, otherwise the converted PNG
  in `<level>_paint/<texture>_normal.png`. Textured faces with only a surface
  material keep their normal map too. Tests:
  `godot_takes_the_normal_map_the_editor_paired_and_flips_a_directx_one` in
  `editor_core`, `the_editors_normal_map_replaces_the_one_found_by_name` in
  `scene_export`.

The feature lab loads its textures from `demos/feature_lab/textures`, outside
the Godot project, so its brick normal map now goes out as a copy in
`feature_lab_paint/`. A project whose editor textures sit inside the Godot
project keeps referencing the original file.

## Open

- An `AnimatableBody3D` follows the animation one physics step late. The
  `HeronAnimation` adapter sets the body's transform, and with
  `sync_to_physics` on, the node reports the new transform only after the next
  physics step. Over 4 m in 2 s that is 3 cm. It is how Godot moves kinematic
  bodies, so the checks allow it.
- `scene_export` used on its own, without the editor, still looks only for
  `<texture>_normal`. It has no image decoder to flip a DirectX map.
