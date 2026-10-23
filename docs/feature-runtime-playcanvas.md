# Feature lab at run time: PlayCanvas

[The feature matrix](feature-matrix.md) checks the feature lab's export in the
files. This page runs the PlayCanvas export in Chromium, the way a game reads it and
the way the generated page plays it. The animation checks compare against poses the
editor itself computes.

Checked on 23 October 2026 on Linux (x86-64, AMD Radeon 610M, Mesa), with the editor
on branch `target-playcanvas` from `4c11486`. PlayCanvas 2.23.1 from jsDelivr, and
the ammo.js build in the engine's repository at tag `v2.23.1`. Chromium is
Playwright's Chrome for Testing 156 with SwiftShader WebGL 2.

## Rerun

```sh
cargo build -p heron-brush
python3 examples/engine-checks/export_feature_lab.py --root /tmp/fl playcanvas
python3 examples/engine-checks/feature_lab_poses.py /tmp/fl/poses.json
HERON_CDN_CACHE=/tmp/fl/cdn node examples/engine-checks/playcanvas/verify_feature_lab.cjs /tmp/fl /tmp/fl/poses.json /tmp/fl/out
```

The check needs Node with `playwright`; see
[the check's README](../examples/engine-checks/playcanvas/README.md) for the Node
that Python's Playwright package carries.

## Summary

| Feature | PlayCanvas |
| --- | --- |
| Textures and UVs | PASS |
| Face colours, clamped paint | PASS |
| Gradients | PASS |
| Surface materials, emissive strength | PASS |
| Brush normal maps with tangents | PASS |
| Collision: hulls, triangles, clip, skip, hidden | PASS (fixed) |
| Trigger as a trigger volume that opens the door | PASS |
| Animation against editor poses, kinematic body | PASS |
| Sky, environment light and fog | PASS (fixed) |
| Water | PASS |
| Lights | PASS |
| Imported model, PBR maps, shared mesh | PASS |
| Typed properties and target links | PASS |
| Playtest page: standing, walls, raised trigger, button, riding the mover, swimming | PASS (fixed) |

"(fixed)" marks failures found by this run and fixed before the first commit of the
target.

## The level through heron_playcanvas.js

`feature_lab_checks.js` loads ammo.js, makes a `pc.Application` and calls `loadLevel`
from the exported `heron_playcanvas.js` on `feature_lab.json`. Results:

- Textures and UVs: PASS. The six textures on `fl_textures` come from the GLB's
  container asset as `StandardMaterial` diffuse maps that repeat and are sRGB
  textures. Read back from the GPU with `Texture.read`, they hold the source PNG's
  exact pixels, and each face has its UV span: 1 by 1, 2 by 1/2, the square root of
  2 for the 45 degree face, 1/2 for the face scaled by 2.
- Face colours: PASS. Five exact colours, read back from the GPU, and the translucent
  face equals the wood texture blended with the paint at 128/255 within 4/255. Every
  paint texture clamps.
- Gradients: PASS. All stops of the four gradients are in their images, every sampler
  clamps, and three faces have UVs past the image's ends, which only a clamping
  sampler draws right.
- Surface materials: PASS. Roughness (PlayCanvas's `gloss` with `glossInvert`) and
  metalness match, and `KHR_materials_emissive_strength` gives an emission of 3
  (`emissive` times `emissiveIntensity`) with the paint as the emissive map.
- Normal maps: PASS. The normal map is 512 by 512 and not an sRGB texture, the mesh
  has the GLB's `TANGENT`, and the vertex shader the render compiled reads
  `vertex_tangent`.
- Collision: PASS after a fix. All through ammo.js rays and bodies:
  - `fl_solid` is a static rigidbody with a convex hull. A ray stops on its top at
    y 2.250, and a sphere dropped on it settles at 2.250.
  - The clip brush draws nothing and stops a ray at 3.250.
  - `fl_skip` is a static triangle mesh. A ray from above passes its skip top and
    stops on the inside of its bottom face at 0.25 (Bullet's triangles are two-sided
    to rays, as Havok's are). Its four walls and its bottom stop rays.
  - `fl_hidden` draws 10 triangles, five sides, and blocks on all six.
  - The trigger is a collision component with no rigidbody, which PlayCanvas makes a
    trigger volume, and draws nothing. A ray passes it to the floor, a ray that takes
    triggers stops at its top (3.25), and a dropped sphere falls through it to the
    floor while the volume fires `triggerenter` for it.
- Animation: PASS. `HeronAnimation` matches 12 editor poses of `fl_mover` (linear,
  hold and bezier keys) within 6.9e-7 m, and 7 poses of the door's `open` clip
  within 1e-4 m. The mover's rigidbody is kinematic: after a pose and a physics
  step, a ray at the mover's new place hits it.
- Door, button and trigger: PASS. Through `level.update`, a point inside the trigger
  fires it, the door reaches the editor's open pose after 1.2 s, re-entering inside
  the 1.5 s wait does not fire again, and the button toggles the door to `close`.
- Sky, environment light and fog: PASS after a fix. `EnvLighting` turns `sky.hdr`
  into the skybox and the environment atlas. With only the sky drawn, looking along
  `to_sun` gives 230 233 238, the opposite way 162 193 223, the direction mirrored in
  Z 164 195 223, and straight up a blue 136 187 224. A white matte card lit by the
  environment alone, untoned, is 0.445 as bright as under an atlas made from the
  undimmed sky; the map's ambient intensity is 0.45. Fog is `FOG_EXP2` with density
  0.02 and colour 180 190 200.
- Water: PASS. `HeronWater` finds the volume's centre 0.75 m under the surface, with
  swim speed 2.5 and priority 3, and nothing above it. The GLB draws two upward
  triangles, blended at opacity 0.55, and the vertex shader the render compiled has
  the wave. The volume is a trigger volume.
- Lights: PASS. `fl_light` is an omni light component (255 200 120, intensity 2.5,
  range 8) and `fl_spot` a spot (outer cone 30 degrees from the axis, range 10), each
  on a child of its entity. The spot points down its entity's -Z turned by
  `rotation_yxz`, within 1e-4. A white card 2 m from each light on its axis is
  brighter with the light on: the sum of the pixel's channels goes from 623 to 745
  for the omni light and from 228 to 739 for the spot. The feature lab's spot points
  60 degrees up, as every target exports it.
- Imported models: PASS. Both copies share one mesh; the material has diffuse,
  normal (bumpiness 0.35), metalness and gloss, occlusion (`aoIntensity` 0.25) and
  emissive maps.
- Properties and links: PASS. `fl_probe`'s record keeps int `charge` 7 over the
  binding's default 9, bool `armed`, the array [2, 3, 5] and both strings;
  `light_color` stays a colour. `entity.heron` is the record. The four links resolve
  to entity records.

## The generated playtest page

`verify_feature_lab.cjs` opens `feature_lab.html?verify=1` and drives its capsule
player one 1/60 s step at a time, as the page steps once a frame.

- Standing: PASS. The feet rest at y 0.000 on the floor and 2.250 on `fl_solid`.
- Walls: PASS. Walking at `fl_hidden` stops the player's middle at x -0.250, the wall
  at 0 less the 0.25 m radius.
- Raised trigger: PASS. Standing in `fl_trigger` with the feet at 0.00, under the
  volume, which starts at 0.25, the middle is inside and the door opens to the end
  of its clip.
- Button: PASS. E aimed at `fl_button` activates it and the door toggles to `close`.
- Riding: PASS after a fix. Standing on `fl_mover`, the player moves 1.392 m in
  0.7 s while the platform moves 1.400 m.
- Swimming: PASS. In the pool the player's middle reads a depth of 0.46 m and swim
  speed 2.5.
- Clamped paint: PASS. The page draws all six `fl_colours` paint maps clamped.

Pictures, in the check's output folder: `playcanvas-lab.png` is the three.js check's
view (from -20 16 -8), fogged; `playcanvas-stations.png` is closer, along the
stations, without fog; `playcanvas-playtest.png` is the generated page. They show
the textured cube, the colour, gradient, glowing surface and normal-mapped brick
cubes, the point light's pool of light on the floor, the water as a see-through
surface, the models, and no trigger or clip box.

### Measured before writing

PlayCanvas's skybox and environment atlas sample an equirectangular panorama with
`dir.x` negated. Four panoramas with one bright spot each, found in the skybox by
turning a narrow camera, put u = 0.1 on the horizon at (0.60, 0, -0.80) and
u = 0.35 at (0.80, 0, 0.60): half a turn on from Babylon.js's layout. The export
bakes `sky.hdr` with `playcanvas_panorama_direction`. Test:
`playcanvas_reads_babylon_s_panorama_half_a_turn_on`.

### Fixed

These were found by this check while the target was being written:

- Rays stopped on `fl_solid` and the clip brush 1 cm high, and a dropped sphere
  rested 1 cm over them. PlayCanvas gives the convex hulls it makes a 1 cm margin,
  and Bullet adds the margin outside the points. The runtime sets the margin of its
  hulls to zero.
- A player on the mover rode 1.07 m while the platform moved 1.40 m. The page took
  the body's vertical speed after a physics step as a sign of a jump, and contacts
  with the moving platform gave it one now and then, so the player kept leaving
  "the ground" and slowing in the air. The page now keeps its own jump state.
- The map's ambient intensity of 0.45 was ignored: PlayCanvas's image-based light
  has no strength, and `skyboxIntensity` dims the sky too. The runtime makes the
  environment atlas from a copy of the panorama scaled by the ambient intensity.

## Open

- Shadows: the lights' `shadows` flag is in the level data, and the runtime turns no
  shadows on.
- The trigger volumes report bodies entering. The page's capsule is a body, but the
  page tests the player against the `_heron_trigger` volumes itself, at feet, middle
  and head, as the other runtimes do.
- PlayCanvas has no AgX tone mapping. The runtime uses ACES at the map's exposure,
  so the colours differ slightly from the editor's view and from three.js.
- The water's wave is a GLSL shader chunk; under WebGPU the water is flat. It moves
  every vertex of the surface, which is the volume's upward faces.
- The page's player steps up anything up to 18 map units, as Quake's does, found
  by its rays; the step is not swept, so a step under a low ceiling can push the
  capsule into it. The feature lab has no stairs, so this is untested.
- The motion lab is not run through PlayCanvas yet; the feature lab covers each
  animation mode, the door and riding.
