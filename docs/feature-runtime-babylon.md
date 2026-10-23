# Feature lab at run time: Babylon.js

[The feature matrix](feature-matrix.md) checks the feature lab's export in the
files. This page runs the Babylon.js export in Chromium, the way a game reads it and
the way the generated page plays it. The animation checks compare against poses the
editor itself computes.

Checked on 23 October 2026 on Linux (x86-64, AMD Radeon 610M, Mesa), with the editor
on branch `target-babylon` from `493d040`. Babylon.js 9.29.0 and Havok 1.3.14, from
jsDelivr. Chromium is Playwright's Chrome for Testing 156 with SwiftShader WebGL 2.

## Rerun

```sh
cargo build -p heron-brush
python3 examples/engine-checks/export_feature_lab.py --root /tmp/fl babylon
python3 examples/engine-checks/feature_lab_poses.py /tmp/fl/poses.json
HERON_CDN_CACHE=/tmp/fl/cdn node examples/engine-checks/babylon/verify_feature_lab.cjs /tmp/fl /tmp/fl/poses.json /tmp/fl/out
```

The check needs Node with `playwright`; see
[the check's README](../examples/engine-checks/babylon/README.md) for the Node that
Python's Playwright package carries.

## Summary

| Feature | Babylon.js |
| --- | --- |
| Textures and UVs | PASS |
| Face colours, clamped paint | PASS |
| Gradients | PASS |
| Surface materials, emissive strength | PASS |
| Brush normal maps with tangents | PASS |
| Collision: hulls, triangles, clip, skip, hidden | PASS |
| Trigger as a sensor that opens the door | PASS |
| Animation against editor poses | PASS |
| Sky and fog | PASS (fixed) |
| Water | PASS |
| Lights | PASS |
| Imported model, PBR maps, shared mesh | PASS |
| Typed properties and target links | PASS |
| Playtest page: standing, walls, raised trigger, button, riding the mover, swimming | PASS (fixed) |

"(fixed)" marks failures found by this run and fixed before the first commit of the
target.

## The level through heron_babylon.js

`feature_lab_checks.js` makes a right-handed scene with Havok and calls `loadLevel`
from the exported `heron_babylon.js` on `feature_lab.json`. Results:

- Textures and UVs: PASS. The six textures on `fl_textures` come through the glTF
  loader as `PBRMaterial` albedo textures that repeat. Babylon.js decodes them as sRGB
  in the texture format, not in the shader. Read back from the GPU, they hold the
  source PNG's exact pixels, and each face has its UV span: 1 by 1, 2 by 1/2, the
  square root of 2 for the 45 degree face, 1/2 for the face scaled by 2.
- Face colours: PASS. Five exact colours, read back from the GPU, and the translucent
  face equals the wood texture blended with the paint at 128/255 within 4/255. Every
  paint texture clamps.
- Gradients: PASS. All stops of the four gradients are in their images, every sampler
  clamps, and three faces have UVs past the image's ends, which only a clamping
  sampler draws right.
- Surface materials: PASS. Roughness and metallic match, and
  `KHR_materials_emissive_strength` gives an emission of 3 with the paint as the
  emissive texture.
- Normal maps: PASS. The bump texture is 512 by 512 and not an sRGB texture, the mesh
  has the GLB's `TANGENT`, and the effect compiled for the render defines `TANGENT`.
- Collision: PASS. All through Havok rays and bodies:
  - `fl_solid` is a static body. A ray stops on its hull top at y 2.25, and a sphere
    dropped on it settles at 2.25.
  - The clip brush draws nothing and stops a ray at 3.25.
  - A ray from above passes `fl_skip`'s skip top and stops on the inside of its bottom
    face at 0.25 (Havok's triangles are two-sided; three.js's Octree culls the back
    face and reaches the floor). Its four walls and its bottom stop rays.
  - `fl_hidden` draws 10 triangles, five sides, and blocks on all six.
  - The trigger is a Havok trigger shape and draws nothing. A ray passes it to the
    floor, a ray with `shouldHitTriggers` stops at its top (3.25), and a dropped
    sphere falls through it to the floor while the plugin reports `TRIGGER_ENTERED`.
- Animation: PASS. `HeronAnimation` matches 12 editor poses of `fl_mover` (linear,
  hold and bezier keys) within 6.9e-7 m, and 7 poses of the door's `open` clip
  within 1e-4 m. The mover's body is `ANIMATED`: after a pose and a physics step, a ray at
  the mover's new place hits it.
- Door, button and trigger: PASS. Through `level.update`, a point inside the trigger
  fires it, the door reaches the editor's open pose after 1.2 s, re-entering inside
  the 1.5 s wait does not fire again, and the button toggles the door to `close`.
- Sky and fog: PASS after a fix. `HDRCubeTexture` reads `sky.hdr` as the environment
  and the skybox. With only the sky drawn, looking along `to_sun` gives 154 161 176,
  the opposite way 66 92 132 and straight up a blue 50 86 134. Fog is
  `FOGMODE_EXP2` with density 0.02 and colour 180 190 200, the editor's linear values
  as three.js reads them.
- Water: PASS. `HeronWater` finds the volume's centre 0.75 m under the surface, with
  swim speed 2.5 and priority 3, and nothing above it. The GLB draws two upward
  triangles, alpha-blended at 0.55, and the effect is compiled with the wave plugin.
  The volume is a Havok trigger.
- Lights: PASS. `fl_light` is a `PointLight` (255 200 120, intensity 2.5, range 8)
  and `fl_spot` a `SpotLight` (a 60 degree cone, range 10), each under its entity.
  The spot points down its entity's -Z turned by `rotation_yxz`, within 1e-4. A white
  card 2 m from each light on its axis is brighter with the light on: the sum of the
  pixel's channels goes from 406 to 471 for the point light and from 161 to 311 for
  the spot. The feature lab's spot points 60 degrees up, as every target exports it.
- Imported models: PASS. Both copies share one geometry; the material has albedo,
  bump (level 0.35), metallic-roughness, occlusion (0.25) and emissive textures.
- Properties and links: PASS. `fl_probe`'s record keeps int `charge` 7 over the
  binding's default 9, bool `armed`, the array [2, 3, 5] and both strings;
  `light_color` stays a colour. `node.metadata.heron` is the record. The four links
  resolve to entity records.

## The generated playtest page

`verify_feature_lab.cjs` opens `feature_lab.html?verify=1` and drives its Havok
character controller one step per simulated frame, as the page steps once a frame.

- Standing: PASS. The feet rest at y 0.014 on the floor and 2.276 on `fl_solid`.
- Walls: PASS. Walking at `fl_hidden` stops the player's middle at x -0.277, the wall
  at 0 less the 0.25 m radius.
- Raised trigger: PASS. Standing in `fl_trigger` with the feet at 0.02, under the
  volume, which starts at 0.25, the middle is inside and the door opens to the end
  of its clip.
- Button: PASS. E aimed at `fl_button` activates it and the door toggles to `close`.
- Riding: PASS after a fix. Standing on `fl_mover`, the player moves 1.385 m in
  0.7 s while the platform moves 1.400 m.
- Swimming: PASS. In the pool the player's middle reads a depth of 0.46 m and swim
  speed 2.5.
- Clamped paint: PASS. The page draws all six `fl_colours` paint textures clamped.

Pictures, in the check's output folder: `babylon-lab.png` is the three.js check's
view (from -20 16 -8), fogged; `babylon-stations.png` is closer, along the stations,
without fog; `babylon-playtest.png` is the generated page. They show the textured
cube, the colour, gradient, glowing surface and normal-mapped brick cubes, the water
as a see-through surface, the models, and no trigger or clip box.

### Fixed

These were found by this check while the target was being written:

- The sky's glow sat away from the sun. Babylon.js's `HDRCubeTexture` reads an
  equirectangular panorama a quarter turn round from three.js's layout. Drawing two
  panoramas with one bright spot each and finding the spots in the skybox gave the
  layout; the export now bakes `sky.hdr` with `babylon_panorama_direction`. Test:
  `babylon_reads_three_s_panorama_a_quarter_turn_on`.
- The player hovered 11 cm over the floor: the character controller's default keep
  distance and contact tolerance are 5 and 10 cm. The page sets them to 0.3 and 0.6
  map units.
- A player on the mover was not carried. The controller measures a moving floor's
  velocity between rendered frames, and the page took several physics steps a
  frame, so most steps saw the floor standing still. The page now takes one step a
  frame.

## Open

- Shadows: the lights' `shadows` flag is in the level data, and the runtime makes no
  shadow generators.
- The trigger shapes are Havok triggers, which report bodies. The character
  controller is not a body Havok reports, so the page tests the player against the
  `_heron_trigger` volumes itself, as the other runtimes do.
- Babylon.js has no AgX tone mapping. The runtime uses ACES at the map's exposure, so
  the colours differ slightly from the editor's view and from three.js.
- Havok's two-sided triangles stop a ray inside a box whose top is skip, where
  three.js's Octree lets it through to the floor. Both are right for a solid box; a
  game that casts rays from inside geometry should know.
- The motion lab is not run through Babylon.js yet; the feature lab covers each
  animation mode, the door and riding.
