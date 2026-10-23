# Defold export check

A Defold project that loads the shared demo map. `export.py` exports it through the
editor's MCP commands (and saves the transcript to `mcp-export.json`), then
`install_game.py` adds game-owned door and portal logic on top of the export. The
full entity data is read from `main/arena.json`, since `go.property` cannot hold
arrays or every target.

## Export and run

From the repository root:

```sh
cargo build -p heron-brush
python3 examples/engine-checks/defold/export.py
java -jar /path/to/bob.jar --root examples/engine-checks/defold --max-cpu-threads 2 build
cd examples/engine-checks/defold/build/default
/path/to/dmengine --config=check.enabled=1 ./game.projectc
```

Or open `game.project` in Defold. `check.enabled=1` runs a five-second collision
check; leave it out to play (WASD, mouse, Shift, Space). The Defold editor archive
contains Bob, dmengine and a JDK (`java -cp defold-<version>.jar com.dynamo.bob.Bob`).

## Gameplay, animation and custom games

```sh
DEFOLD_HOME=/path/to/Defold python3 examples/engine-checks/defold/verify.py
python3 examples/engine-checks/prepare_motion_lab.py defold
python3 examples/engine-checks/defold/prepare_custom_game.py
```

- `verify.py` re-exports, builds and runs the native engine with
  `check.enabled=2`: doors, door colliders and both portal directions, using real
  trigger and contact events. Output goes to `evidence/`.
- The motion lab project (`/tmp/heron-defold-motion`) runs with
  `--config=check.enabled=3 --config=display.vsync=1 --config=display.update_frequency=60`
  and checks shuttle, lift, rotor, pads and doors with player carrying.
- The custom game (`/tmp/heron-defold-custom-game`) has no FPS classes and checks
  a project script, typed properties, mass and collision groups. It prints
  `HERON_DEFOLD_CUSTOM_GAME_OK`.

Not covered: Windows, other physics backends, and exported lights (the material
uses simple directional shading). Enemies and other FPS classes stay as data.

Credits: controller and shaders are MIT code from the earlier tbx project, see
`LICENSE-TBX`. Textures are Kenney assets under their own licence.

## Feature lab

```sh
python3 examples/engine-checks/defold/run_feature_lab.py --root /tmp/feature-runs
```

Exports `demos/feature_lab/feature_lab.map`, builds it with Bob and runs it in
dmengine inside Xephyr with the check in `feature_lab/`. See
[feature lab in Defold](../../../docs/feature-runtime-defold.md).
