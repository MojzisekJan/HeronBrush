# Godot export check

A copy of the Kenney Starter Kit FPS that plays a Heron Brush export of the shared
demo map. FGD annotations pick the scenes, scripts, groups and signal connections;
the project's own scripts implement doors, enemies and portals.

## Export and run

From the repository root, with a built `target/debug/heron-brush`:

```sh
python3 examples/engine-checks/godot/export_demo.py
GODOT=/path/to/godot python3 examples/engine-checks/godot/verify.py
```

`verify.py` imports the project and runs five suites (`verify_contract.gd`,
`verify_demo.gd`, `verify_portal.gd`, `verify_doors.gd`, `verify_damage.gd`), with
logs in `evidence/`.

`verify_feature_lab.py` runs the feature lab's own export in Godot, feature by
feature, and with `--shots` saves pictures of each station; see
[the Godot feature lab run](../../../docs/feature-runtime-godot.md).

Play the map, or save a screenshot to `evidence/demo.png`:

```sh
godot --path examples/engine-checks/godot res://scenes/play.tscn -- --level=res://maps/demo.tscn
godot --path examples/engine-checks/godot --rendering-method gl_compatibility --max-fps 60 --script capture_demo.gd
```

The kit scripts assume 32 map units per metre. Use `res://` paths for textures
inside the project.

Not covered: Windows, Forward+ rendering, and other games' scripts. Godot may
report resource leaks at shutdown; they do not affect the checks.

Credits: Kenney kit (`README-Kenney.md`, `LICENSE.md`), textures
(`textures/License.txt`) and font (`fonts/license.txt`). Other copied code is MIT
code from the earlier tbx project, see `LICENSE-TBX`.
