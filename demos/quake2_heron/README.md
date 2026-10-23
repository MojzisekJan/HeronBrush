# Quake 2: Heron Outpost

A small single-player level built in Heron Brush for the Quake 2 demo and compiled with
ericw-tools. A hall and a courtyard open to the sky: a button on the hall's wall opens
the door between them, a trigger in the courtyard's doorway brings in two guards and an
enforcer and switches the courtyard's lamps on, steps lead up to a platform, and the
exit there loads the demo's first map. A teleporter pad in the hall's corner puts the
player on the platform.

- `heron_outpost.map` is the map as Heron Brush saves it (Valve 220).
  `heron_outpost_q2.map` is the same map exported in Quake 2's dialect, which is what
  `build.sh` compiles.
- Every texture is in the demo's `baseq2/pak0.pak`, which `qbsp` reads directly. A face
  without flags of its own takes them from its texture's `.wal`: that is what makes
  `e1u1/sky1` sky and the `e1u1/baselt_3` panels emit light.
- `build.sh` runs `qbsp -q2bsp`, `vis` and `light` and copies the bsp into the game's
  `baseq2/maps` and the three configs into `baseq2`. `build.sh --remake` runs
  `make_map.py` first, which builds the map again through Heron's MCP server.
- `playtest.cfg` presses the button, walks through the door and the trigger and into
  the exit; `lamps.cfg` looks at the courtyard before and after the trigger;
  `teleporter.cfg` steps onto the pad. Each takes screenshots.

What to install and how to run it is in
[the game profile tests](../../docs/game-profile-tests.md#quake-2).
