# D-Day: Normandy: Heron Hedgerow

A small two-team field built in Heron Brush for D-Day: Normandy and compiled with
ericw-tools. The Allies start at the west end and the Axis at the east, each behind a
low wall with a spawn for every class and one for reinforcements. A concrete bunker
stands in the middle with a ramp on each side; standing on its roof takes the
objective and scores 10 points. Crates and hedges give cover on the way. The ceiling is
sky, lit by a sun.

- `heron_hedgerow.map` is the map as Heron Brush saves it (Valve 220).
  `heron_hedgerow_q2.map` is the same map exported in Quake 2's dialect, which is what
  `build.sh` compiles.
- The textures are the mod's own, under `dday/textures`; the entities come from
  `gamedata/DDayNormandy/dday.fgd`.
- `build.sh` runs `qbsp -q2bsp`, `vis` and `light` and copies the bsp into the mod's
  `dday/maps` and `playtest.cfg` into `dday`. `build.sh --remake` runs `make_map.py`
  first, which builds the map again through Heron's MCP server.
- `playtest.cfg` joins the Allies through the game's menus, spawns as infantry and
  walks onto the bunker, taking a screenshot at each step. Start the game with
  `+set deathmatch 1 +map heron_hedgerow +exec playtest.cfg`.

What to install and how to run it is in
[the game profile tests](../../docs/game-profile-tests.md#d-day-normandy).
