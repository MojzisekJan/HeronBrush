# Heretic II: Heron Wharf

A small single-player level built in Heron Brush for the Heretic II demo and compiled
with q2tools-220. A warehouse and a yard open to the sky: a button beside the
warehouse door opens it, a trigger in the yard's doorway wakes two plague elves and
three rats, prints "My brethren are insane with bloodlust!" and lights the yard's
lamps, steps lead up to a quay, and the exit there loads the demo's Silverspring Docks.

- `heron_wharf.map` is the map as Heron Brush saves it (Valve 220).
  `heron_wharf_h2.map` is the same map exported in Quake 2's dialect, which is what
  `build.sh` compiles.
- Every texture is one of the demo's loose `.m8` files under `Base/textures`. No face
  writes flags of its own, so each takes them from its `.m8`: that makes `general/sky`
  sky and `general/trigger` nodraw, and gives every face its footstep material.
- `build.sh` runs `q2tool -bsp -vis -rad -h2tex` (q2tools-220 with two fixes for
  Heretic II, see the profile tests) and copies the bsp and the configs where the
  engine reads them. `build.sh --remake` runs `make_map.py` first, which builds the map
  again through Heron's MCP server.
- `playtest.cfg` presses the button, walks through the door and the trigger and into
  the exit; `lamps.cfg` looks along the yard before and after the trigger. Each takes
  screenshots.

What to install and how to run it is in
[the game profile tests](../../docs/game-profile-tests.md#heretic-ii).
