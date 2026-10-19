# Native editor input checks

These fixtures drive actual egui pointer/keyboard input in the native editor. On Linux
it runs on an isolated Xephyr display. On macOS the runner opens an ordinary window,
visible while the check runs, and keeps the display awake with `caffeinate`: a sleeping
display never delivers the window screenshot. Desktop input is untouched on both,
and while a script runs the editor uses its in-app file choosers and questions,
because a system dialog would block the window. On macOS a script run keeps the
bundled Inter instead of the system's San Francisco, so script coordinates stay the
same on both systems. Existing Xephyr (Linux) and the
debug binary are required; no package download. The helper copies `cube.map` into the
output folder so tests cannot overwrite source maps, and uses isolated preferences.

```sh
cargo build -p heron-brush --offline
python3 examples/editor-checks/run_native.py examples/editor-checks/native-clip.script --shots /tmp/heron-clip
```

The runner now requires every declared JSON snapshot and PNG, removes stale
evidence before starting, and launches a separate instance with isolated config
and data. A missing artifact fails the run. A `saved` snapshot must be clean and
match the scene written to disk (the separate material-source header is omitted
from scene snapshots). Unit checks for these guarantees run with
`python3 -m unittest discover -s examples/editor-checks -p test_run_native.py`.

`native-basics.script` covers duplicate, movement, clipboard copy/paste/cut,
delete, Undo/Redo and save. `native-reopen.script` captures a fresh process
opening the resulting file. These scripts require the current development binary.

Repeat with `native-vertex`, `native-sweep`, `native-fillet`, `native-uv` and
`native-animation`. Every script serializes the map before/after its UI edit,
executes one Undo and asserts exact restoration. PNGs include the window and the
3D viewport. Animation also starts/stops the native timeline and captures its
intermediate pose. Scripts assume a fresh config, English, scale 1 and the default
1360×760 window, or 1360×900 where a script explicitly requests it; changing
layout/DPI requires new coordinates. They are fixture
checks, not a full usability/accessibility review or a complex-map stress test.

The material fixtures also exercise one-face assignment over an existing colour
with both Shift face selection and a Face-tool handle:

```sh
python3 examples/editor-checks/make_material_fixture.py /tmp/heron-material.map
python3 examples/editor-checks/run_native.py examples/editor-checks/native-material.script --map /tmp/heron-material.map --shots /tmp/heron-material
python3 examples/editor-checks/run_native.py examples/editor-checks/native-material-handles.script --map /tmp/heron-material.map --shots /tmp/heron-material-handles
```

The checker requires exactly one brick face and five unchanged stone faces, then
exact Undo/Redo and saved-scene equality. This uses only repository textures.
Script isolation retains asynchronous system clipboard responses so Copy/Paste
actually reaches the application. Synthetic pointer/key input remains isolated.

Current basic audit (16 October 2026): basics, fresh reopen, UV, Fillet, Clip,
Vertex and both material selection workflows passed in native Linux editor.
The reopen scene exactly matched the saved one and was clean. Fillet preview
kept the map and dirty flag unchanged until apply. UV changed exactly one face.
Full core tests: 825 passed / 13 ignored. Frontend: 127 passed / 5 ignored,
plus the subsequent clipboard-isolation regression and script parser checks.
The developer binary was rebuilt offline; no release installation was made.
Sweep/animation were not rerun in this series. These checks do not cover large
maps, performance or every combination of tools.

macOS run (18 October 2026, Apple Silicon, wgpu/Metal, debug build): all 22 scripts
pass: basics, reopen, UV, Fillet, Fillet selection, Clip, Vertex, Sweep, animation,
both material scripts, all four texture copy/transfer scripts, both paint drags,
middle-click material, diagnostics, both welcome scripts, and the four
`verify_model_import.py` workflows. Sweep, animation and the model-import scripts
needed new coordinates after the Geometry menu and centred dialogs. Sweep also
needed short waits after collapsing inspector sections: on the faster Mac renderer
clicks landed before the collapse animation ended. Diagnostics now saves before
`quit`; with unsaved changes the close question waited for an answer.

Historical run: all six passed on Linux on 14 October 2026. This does not
validate later editor changes or updated script coordinates. Clip had a split Undo/parent-loss bug
fixed with one structural batch. Shift face picking was blocked by preparing an
extrusion; plain Shift-click now selects a face while Shift-drag keeps extrusion.
Core regression checks cover both paths and clip brush-entity parenting.

The diagnostic fixture repairs a deliberately invalid UV scale, checks exact
one-step Undo and hides/restores the finding in the native issue panel:

```sh
python3 examples/editor-checks/run_native.py examples/editor-checks/native-diagnostics.script --map examples/editor-checks/bad-uv.map --shots /tmp/heron-diagnostics
```

This passed on Linux. A separate isolated preference file with Blender plus
`tool_brush=F5` and `save=F6` also passed native dispatch and showed F6 in File/Save.
The update dialog displayed `MojzisekJan/HeronBrush` with startup checking off.
It did not download or install a release.

Static model imports have a dedicated native workflow check:

```sh
cargo build -p heron-brush --offline
python3 examples/editor-checks/verify_model_import.py --root /tmp/heron-native-model-check
```

It generates tiny textured OBJ/MTL/PNG and embedded GLB fixtures locally. Four
GUI scripts exercise the actual in-app file dialog, import after hovering a 2D
view, texture preview, inspector rotation and scale, exact Undo/Redo and save/reload.
Assertions verify project asset copies, source-byte retention, loaded triangle and
skin data and transformed model bounds. PNGs support visual inspection.
`--case import-obj`, `import-glb`, `model-rotation` or `model-scale` selects one
workflow; repeat the option to select several. Add `--multi` to generate models
with two materials and supplied corner normals. Use a separate `--root` folder
for these fixtures. Snapshots then also verify both
material indices, both loaded skins and normals on both triangles. No downloads
or engine installs.

`run_native.py --cwd <folder>` adds that folder as a file-browser place, preserving
isolated preferences and the temporary map copy. Scripts retain the English,
scale-1 and 1360×760 coordinate assumptions above. This verifies basic static
model workflows; it does not cover OS drag/drop, complex assets or engine execution.

Shared exported models have an offline Godot check:

```sh
python3 examples/editor-checks/verify_instances.py
```

It creates a fresh temporary export, checks three nodes sharing one mesh resource,
and compares world vertices and transformed normals with editor data. Requires an
already installed Godot. See [scope and fallback rules](../../docs/model-instancing.md).

PBR import has a generated local GLB fixture and checks for preview, Godot and
Three.js. See [PBR support and reproduction](../../docs/model-pbr.md). The browser
check uses the vendored Three.js modules and a localhost server; no installations
or CDN downloads are needed.

The welcome layout can be inspected without opening a map:

```sh
python3 examples/editor-checks/run_native.py examples/editor-checks/native-welcome.script --welcome --shots /tmp/heron-welcome
```

Texture transfer has dedicated painted-target regressions. Generate the material
fixture above, then run `native-texture-copy.script` and
`native-texture-copy-handles.script` with that map. They use the actual system
clipboard and select the destination through Shift picking or a Face-tool handle.
`native-texture-transfer.script` and `native-texture-transfer-handles.script`
exercise Alt-click transfer. Each requires two textured faces, four unchanged
stone faces and four remaining paint annotations, plus exact Undo/Redo and save.
The source remains selected during an Alt transfer. The isolated Xephyr display
has no window manager; on a desktop where Alt-click moves windows, use clipboard
copy/paste instead. These checks do not change desktop mouse bindings.

Fillet source selection has a two-brush fixture:

```sh
python3 examples/editor-checks/run_native.py examples/editor-checks/native-fillet-selection.script --map examples/editor-checks/two-cubes.map --shots /tmp/heron-fillet-selection
```

It starts Fillet with no selection, selects/adds/removes brushes in both views,
pins and applies an edge preview, undoes the edit and checks empty-click behavior.
All selection-only snapshots must retain the exact map and active Fillet tool.

Paint dragging is covered by `native-paint-drag.script` and
`native-paint-drag-2d.script`. Generate a material fixture with
`make_material_fixture.py OUTPUT_MAP`, or pass `two-cubes.map` as its second
argument for the 2D case. The checks require two painted faces, one Undo for
the whole drag, exact Redo/save and unchanged selection. The 3D case also
checks Shift eyedropper without a map edit.

`native-middle-material.script` uses the single-cube material fixture. It copies
a selected face with Ctrl+C, pastes onto a pointed face with a middle click in
both views, and checks that selection stays unchanged. It also checks a focused
UV field, exact Undo, and that middle dragging still pans without painting.
