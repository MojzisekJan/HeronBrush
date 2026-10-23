# Half-Life: Heron Lab

Two small single-player maps built in Heron Brush for Half-Life: Uplink, the free demo
Valve released in 1999, compiled with SDHLT and played in Xash3D FWGS. In `heron_lab`
the player picks up the HEV suit and a pistol, presses a button that lifts the lab's
door, walks through a passage whose trigger switches a lamp on and sends in two
houndeyes, past headcrabs and a zombie in a yard under the desert sky, up a ramp and
into a corridor whose `trigger_changelevel` leads through the landmark `heron_gate` to
`heron_lab2`. There a rotating door opens onto a room with a scientist, and the
`trigger_endsection` at its far end ends the demo the way Uplink's own maps do.

- `heron_lab.map` and `heron_lab2.map` are the maps as Heron Brush saves them (Valve
  220), which is what the compilers read.
- `extract_wad.py` gathers the 427 textures inside Uplink's maps into
  `wads/uplink.wad`: Uplink ships no texture wad. The tool textures (`CLIP`, `ORIGIN`,
  `AAATRIGGER`, `NULL`) come from SDHLT's `sdhlt.wad`. Neither wad is committed.
- `build.sh` makes the wads, runs `sdHLCSG -wadinclude uplink.wad`, `sdHLBSP`,
  `sdHLVIS` and `sdHLRAD` on both maps, and copies the bsps into the game's
  `valve/maps` and `playtest.cfg` into `valve`. `lights.rad` makes the ceiling lamps'
  texture give light.
  `build.sh --remake` runs `make_map.py` first, which builds both maps again through
  Heron's MCP server.
- `playtest.cfg` walks the player through both maps, pressing the button with `+use`,
  and takes a screenshot at each step; `console.py` types `exec playtest.cfg` into the
  game's console through XTest.

What to install and how to run it is in
[the game profile tests](../../docs/game-profile-tests.md#half-life-uplink).
