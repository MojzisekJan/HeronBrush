# Hexen II: the keep

A small Hexen II level built in Heron Brush for the Hexen II demo, compiled with
ericw-tools or with Raven's own tools. A hall and a chapel: a button opens the door
between them, a trigger in the chapel's doorway wakes the imps, the archer and the
spider and fades the chapel's lights up, steps lead to a dais, and the exit there sends
the player on to the demo's first map.

- `heron_keep.map` is the map as Heron Brush saves it (Valve 220).
  `heron_keep_hexen2.map` is the same map exported in Hexen II's own dialect, which is
  what the compilers read; Raven's `qbsp` reads no other.
- The textures come from `h2demo.wad`, which worldspawn names as `wads/h2demo.wad`.
  The demo ships no texture wad, so `extract_wad.py` gathers the textures of the demo's
  four maps from `pak0.pak` into one. Neither the wad nor the compiler output is
  committed.
- `build.sh` makes the wad, compiles the map (`--raven` for Raven's tools) and installs
  the bsp in the demo's `data1/maps`. `build.sh --remake` builds the map again in Heron
  Brush first, through `make_map.py`.
- `playtest.cfg` walks the level in the engine and takes a screenshot at each step.

What to install and how to run it is in
[the game profile tests](../../docs/game-profile-tests.md#hexen-ii).
