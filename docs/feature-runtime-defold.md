# Feature lab in Defold

The [feature matrix](feature-matrix.md) checks the Defold export of
`demos/feature_lab/feature_lab.map` in the files. This page is the engine run:
Bob builds the export and dmengine plays it with a check script, feature by
feature.

Run on 23 October 2026 on Linux with Defold 1.13.2 (engine `20692b3`, Bullet
3.25, OpenGL), from the Defold editor package in `~/.local/opt/Defold`. Every
check passed: 57 in the engine and 7 on its screenshots.

## Rerun

```sh
cargo build -p heron-brush
python3 examples/engine-checks/defold/run_feature_lab.py --root /tmp/feature-runs
```

The script exports the feature lab for Defold with `export_feature_lab.py`, writes
the editor's poses of `fl_mover` with the editor_core test
`writes_feature_lab_mover_poses_for_engine_checks`, and adds the check project from
`examples/engine-checks/defold/feature_lab/` plus the sample's `/main/model.material`.
It then runs `bob.jar distclean build` and dmengine. Bob compiles what the
project reaches, so the exported `feature_lab.render` itself is not compiled; its
script is, inside the check's wrapper. dmengine runs inside a nested
Xephyr X server, so the screenshots hold the game window and nothing else on the
desktop. Output goes to `<root>/defold-feature-evidence/`: the Bob log, the engine
log and one PNG per camera view. `--no-run` stops after the build. The script
exits non-zero on any failure.

The check project's `main.collection` holds the export as a collection instance
`level`, a camera and a kinematic box in the `player` group. The box plays the
player: the check script moves it into volumes and reads what happens. The render
script is the exported `feature_lab.render_script` with a wrapper that counts, per
frame, which predicates it drew.

## Result

| Feature | Result | Evidence |
| --- | --- | --- |
| Bob builds `.model`, `.collisionobject` and scripts | PASS | Bob exits 0 and writes all 16 models, 15 collision objects and 24 scripts, the sky and water materials and the render script |
| Textures, colours and gradients through `/main/model.material` with `tex0` | PASS | Screenshots below; red, blue, green and yellow paint each cover 3.7 to 4.7% of their view |
| Hidden face absent | PASS | Seen from its hidden side, `fl_hidden` shows the station behind it; from above it shows its wood top |
| Static hulls for solid and clip | PASS | Rays stop on `fl_solid` at 2.29 m and on `fl_clip` at 3.29 m (tops 2.25 and 3.25 m); the clip side at x 2.04 m |
| Skip | PASS, with a note | The top is not drawn: from above the floor shows through. The collision is a convex hull of the whole brush, so a ray stops on the skip top at 2.29 m |
| Trigger messages | PASS | `trigger_response` from `fl_trigger` and `fl_button` reaches the player box; a ray through the trigger ignores it and reaches the floor |
| Kinematic door from trigger and button | PASS | The trigger opens `fl_door` 90 degrees; the button closes it, then opens it again; the door's collider leaves the doorway and blocks the open side |
| Baked animation against editor poses | PASS | 240 frames of playback within 1.9e-6 m, and 11 seeks within 2.4e-7 |
| Sky and water passes | PASS | The render script drew the `heron_sky`, `model` and `heron_water` predicates in all 390 frames counted; the sky and the water surface are in the screenshots |
| Properties | PASS | `charge` 7, `armed` true, the string with a comma, light energy 2.5, camera fov 62.5 and current 1, water speed 2.5 and priority 3; the level JSON keeps `sequence` as an array of three ints and each type |
| Links | PASS | `fl_probe`, both waypoints, the button and the trigger name game objects that exist, at their map positions |
| Water volume | PASS | The generated `feature_lab_water.lua` samples depth 0.75 m and swim speed 2.5 in the volume, and nothing above it |
| Imported models | PASS | Both `heron_model` objects draw the shared `heron_model.glb` with its base colour |

### What was checked and how

**Rendering.** The machine has a GPU and an X display. dmengine draws through the
exported render script into a 1280 by 720 Xephyr screen, and the script saves 17
views. Each painted, gradient and textured face draws with the right image: the
six textures of `fl_textures`, the five opaque paints of `fl_colours`, the four
gradients including the radial one, and the brick wall. The pixel checks look for
each opaque paint by its channel ratios, so lighting does not matter. The sky check
reads the top-left pixel, (171, 185, 207), not the clear colour (20, 31, 43).

**Hidden and skip faces.** Back faces are culled, so a box missing one face is
invisible from that side and shows what is behind it. The centre of the
`hidden_face` view is grey (96, 102, 110), the metal of `fl_skip` behind it; the
same box from above is wood (159, 115, 73). The `skip_top` view's centre is the
floor (177, 177, 179).

**Collision.** Bullet pads convex hulls with a collision margin, so every ray
stops about 0.04 m outside the brush face: 2.2906 for a 2.25 m top, 0.0418 for the
floor. The check allows 0.05 m. Every brush entity gets a convex hull, so the skip
top keeps its collision in Defold. The feature matrix says the same: a hull covers
the whole brush.

**Triggers and the door.** The probe enters `fl_trigger` at 0.05 s. By 2 s the
door's X axis is (0, 0, -1), a quarter turn. A ray across the doorway misses it,
and a ray across the open side hits it. The button then toggles it shut, axis
(1, 0, 0), and open again. The generated scripts did this from `trigger_response`
messages; the check sent no messages of its own to the door.

**Animation.** The fixed timestep is on, so the baked script advances in
`fixed_update`. After n fixed steps, the mover's position and three axes match the
editor's pose at frame n-1 for all 240 frames. The mover's script runs after the
check script within a step, so the offset is one step throughout. The editor poses
come from `Track::transform`, the function the editor's preview uses. The check
then pauses the mover and seeks it to 11 times, including 0.999 and 1.0 s on
either side of a held key and 5.0 s into the second loop.

## Fixes

None needed in the Defold writer. The check scripts and the editor pose writer
are on branch `runtime-defold-flax`.

## Open

- Bullet's 0.04 m hull margin makes every hull 4 cm larger than its brush. A
  player stands 4 cm above the floor and touches walls 4 cm early. Insetting the
  exported hull planes by the margin would cancel it. That changes the exported
  geometry, which the feature matrix checks as exact, so it is a decision rather
  than a fix.
- `fl_button`'s `wait` reaches Defold as `hash("0")`, a string, while
  `fl_trigger`'s `wait` is the number 1.5. The map writes both as text; the level
  JSON types the trigger's as a float and the button's as a string. The generated
  button script reads its own numeric copy in the trigger payload, so nothing
  breaks, but a game script reading `go.get(url, "wait")` gets a hash.
- The sample `/main/model.material` ignores the roughness, metallic and emission
  factors, as the export warns. The emissive face of `fl_surface` draws as plain
  orange.
- The screenshots ran in Xephyr, whose OpenGL here is Mesa's. An earlier run in a
  window on the real display, on the GPU, drew the same views.
