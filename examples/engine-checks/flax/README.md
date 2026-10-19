# Flax export check

A Flax project (tested with 1.12) that imports the shared demo map with the
generated C# importer. The check builds the scene, checks every actor's metadata
and target links, and creates native mesh colliders. Gameplay is left to the
game: the generated `Build(configure)` callback runs after all actors exist.

## Export and run

From the repository root, with the .NET 8 SDK installed:

```sh
cargo build -p heron-brush
python3 examples/engine-checks/flax/export.py
/path/to/FlaxEditor -headless -null -project "$PWD/examples/engine-checks/flax" -exit -std -heroncheck -heronphysics
/path/to/FlaxEditor -headless -null -project "$PWD/examples/engine-checks/flax" -exit -std -heroncheck -heronreload
```

- `-heronphysics` runs PhysX: floor raycast, a falling box, a rotated and scaled
  convex door, and sensor enter/exit events.
- `-heronreload` starts a fresh process and repeats the queries on the saved scene.
- Drop `-headless -null -exit` to also check rendered materials on the GPU.
- `-heronvalidate` skips model reimport.

For normal use, open `flax.flaxproj` and use **Tools > Heron Brush import**.
`Build(createCollisions: false)` and `Build(includeCollision: ...)` control colliders.

## Animation and custom games

```sh
cargo test --locked -p editor_core --lib portable_animation -- --test-threads=1
python3 examples/engine-checks/prepare_motion_lab.py flax
/path/to/FlaxEditor -headless -null -project /tmp/heron-flax-motion -exit -std -heroncheck -heronmotion
python3 examples/engine-checks/flax/prepare_custom_game.py
```

The motion check rides the shuttle, lift and rotor with `HeronBrushMotionPlayer`
and compares poses against the editor (`HERON_PORTABLE_FIXTURES` sets the fixture
folder). Run the custom game project (`/tmp/heron-flax-custom-game`) with
`-headless -null -exit -std -heroncustom` to check prefabs, scripts, two maps and
rebuild ownership.

Not covered: headless mode cannot check materials, a player build needs collision
assets baked in the editor, and old import roots are not found again after an
editor restart.
