# Game profile tests

Heron Brush ships game profiles in `gamedata/`. Each one is tried on the real game:
install it, build a playable map in Heron following that game's own mapping guides,
compile it with the game's tools, run it in the game and fix what Heron gets wrong.
Engine export targets are checked separately, see [feature matrix](feature-matrix.md).

## Order

Games that are free to download come first, one at a time.

| Profile | Free game or data | Status |
| --- | --- | --- |
| Quake | LibreQuake data, a source port, ericw-tools | [done](#quake) |
| Neverball | Neverball and its `mapc` | planned |
| Quake3 | OpenArena, q3map2 | planned |
| Quetoo | Quetoo and its tools | planned |
| DigitalPaintball2 | Digital Paint: Paintball 2, ericw-tools | [done](#digital-paint-paintball-2) |
| Quake2 | the freely distributed demo, a source port | planned |
| DDayNormandy | the free mod on the Quake 2 demo | planned |
| Hexen2 | the freely distributed demo | planned |

Not tried, because the game has to be bought: Halflife, Kingpin, SoF, Heretic2,
Daikatana and Wrath. `Generic` and `heron_brush` are Heron's own profiles.

## Quake

Tried on 2026-10-23 with free data only: LibreQuake as the game, ironwail as the
engine and ericw-tools as the compilers. The level is
[demos/quake_libre](../demos/quake_libre/README.md): two rooms, a door that a button
opens, a trigger that wakes an ambush and switches its light on, a lift up to a ledge
and the exit. It compiles sealed (no leak), lights and plays through to the end-of-level
screen.

### What was installed

Everything is under `~/.local/opt/quake/`, without root. The archives stay in
`downloads/`.

| What | From | Unpacked to |
| --- | --- | --- |
| LibreQuake 0.09-beta, `full.zip` (the game: `id1/pak0.pak`, `pak1.pak`) | github.com/lavenderdotpet/LibreQuake releases | `librequake/` |
| LibreQuake 0.09-beta, `dev.zip` (map sources and the mapping wads) | same | `librequake-dev/` |
| ericw-tools 2.0.0-alpha11, Linux build (`qbsp`, `vis`, `light`) | github.com/ericwa/ericw-tools releases | `ericw-tools/` |
| ironwail 0.8.2, Linux x64 build (a Quakespasm descendant) | github.com/andrei-drexler/ironwail releases | `ironwail/` |

LibreQuake ships no wad inside its paks. Its mappers use the wads in `dev.zip`'s
`texture-wads/`: `lq_medieval.wad`, `lq_tech.wad`, `lq_metal.wad` and others, with
`lq_utility.wad` holding `clip`, `skip`, `trigger`, `hint` and `origin`. All are WAD2,
so they need Quake's palette, which is `gfx/palette.lmp` in `pak0.pak`.

### Setting Heron up

In the welcome window the profile is set to Quake, and in Preferences, Games, the game
path is `~/.local/opt/quake/librequake`. Heron then mounts `id1` and its paks, which is
where a WAD2's palette and the monster models come from. The Quake profile loads
`gamedata/Quake/Quake.fgd`. A wad is added from the Materials tab; with the Quake
profile it is also named in worldspawn's `wad` key, relative to the map's folder, which
is where `qbsp` looks for it.

### Rerunning it

    cd demos/quake_libre
    ./build.sh                  # copies the wads, runs qbsp, vis and light
    ~/.local/opt/quake/ironwail/ironwail -basedir ~/.local/opt/quake/librequake \
        -game heron -condebug +map libre_lift +exec playtest.cfg

`build.sh --remake` builds the map again in Heron first, through `make_map.py` and the
MCP server; set `HERON_BRUSH` to the binary and `XDG_CONFIG_HOME` to a scratch folder
whose preferences have the Quake game path. `playtest.cfg` puts the player by the
button, walks into it, then walks east through the door, the trigger, onto the lift and
into the exit, echoing a `PLAYTEST` line and taking a screenshot at each step. The
engine ran in a nested Xephyr display with Mesa's llvmpipe, which is enough for
ironwail's GL 4.3.

### What worked

- The map compiled with no leak and no texture warnings, and `light` found all ten
  lights, one of them switchable (the ambush light).
- In the engine the console shows no errors from the map. The player spawns facing
  the door; walking into the button opens the door (the door's origin goes from `0 0 0`
  to `0 0 120`, seen with `edict`); the trigger prints "Ambush!" and wakes the soldiers
  and the dog; the lift rises to the ledge; the exit ends the level.
- Textures are where Heron draws them: the brick courses run on across brush seams
  and the floor tiles and lift top line up with their brushes, as in the editor's views.
- A round trip of LibreQuake's own `lq_e0m1.map` (made in TrenchBroom, 1074 brushes,
  rotated and scaled Valve faces) through Heron and back into `qbsp` gives the same 949
  texinfos, the same planes and the same entities. The bsp differs by a few nodes:
  Heron writes grouped entities in a different order on the first save, and snaps two
  plane points that were a hair off the grid (`367.4999999999999` became `367.5`).

### What was fixed in Heron

- **Worldspawn could not be edited.** Neither the Entity tab nor MCP could reach
  worldspawn's keys, so a map could not get its `wad`, `message`, `worldtype` or
  `sounds`. With nothing selected the Entity tab now edits worldspawn, with the FGD's
  names for its keys and the wad list editor, and every change is one undo step
  (a new `WorldChanged` edit, sent to collaborators too).
- **A wad loaded from the Materials tab was not named in the map.** `qbsp` reads only
  the wads worldspawn names, so the map compiled with every texture missing. For a game
  whose configuration has `"attribute": "wad"` (Quake and the rest of its lineage),
  adding a wad now also names it in `wad`, relative to the saved map.
- **A wad named relative to the map was not found when the map opened.** Heron looked
  in the working directory and by file name only; `qbsp` resolves the name against the
  map's folder, and so does Heron now. A wad named both in the key and in Heron's own
  note on the map's material sources is no longer loaded twice, and editing the `wad`
  key loads what it names straight away.
- **`heron-brush --mcp` ignored the preferences.** The stdio server never read them, so
  it had no game and no game path, and every WAD2 texture was skipped for want of a
  palette. It now reads them as the window does.
- **MCP had no way to set an entity's keys after placing it.** New tools:
  `set_entity_property` (an entity by id, `worldspawn`, or the selection; a brush
  entity's selected brush counts as the entity) and `load_wad`. `open_map` now loads the
  wads the map names.
- **UV values gained digits on save.** Heron keeps offsets, rotation and scale as `f32`
  and wrote them as `f64`, so TrenchBroom's `0.948473` came back as `0.9484729766845703`.
  They are now written in the fewest digits that read back the same.

### Mapping mistakes, not Heron's

- Lights of 220 in rooms 192 units high were too dark; 300 to 350 reads well.
- `setpos` in ironwail turns noclip on, and the engine moves a player that is touching
  a solid back when noclip goes off, so the button test walks into the button instead
  of teleporting onto it.
- `sound/player/slimbrn2.wav` is missing from LibreQuake 0.09-beta; the console reports
  it on every map.

### What remains

- Heron writes brushes that have no UV axes of their own as Standard face lines under
  `// Format: Valve` and `"mapversion" "220"`. ericw-tools reads that fine. Whether
  TrenchBroom opens such a file was not checked.
- The entity browser lists Heron's built-in classes (`light_spot`, `prop_static`,
  `info_waypoint` and others) beside Quake's, though Quake does not know them.
- MCP has no tool to set one face's material or UV, so a brush made through MCP wears
  one material on every face.
- Placing a light writes the FGD's defaults (`"wait" "1"`, `"style" "0"`) into it.
  Harmless, but it is noise TrenchBroom does not add.
- The GUI wad list was checked in screenshots, but the file dialog route of the
  Materials tab was not driven by a script.

## Digital Paint: Paintball 2

Tried on 2026-10-23 with Paintball 2 build 48 for Linux and ericw-tools 2.0 for the
compile. The level is [demos/paintball_heron](../demos/paintball_heron/README.md): a
capture-the-flag field with a raised base at each end (the team's flag, the capture
trigger around it, a ramp and four team spawns), barrels, crates, two low walls and a
centre tower with two ramps, under a sky lit by a sun. It compiles sealed, lights, and
loads in the game as "Team Flag CTF" with both teams' spawns, flags and bases working.

### What was installed

Everything is under `~/.local/opt/paintball2/`, without root. The archives stay in
`downloads/`.

| What | From | Unpacked to |
| --- | --- | --- |
| Paintball 2 build 48, `paintball2_build048_linux_full.tar.gz` | digitalpaint.org/files (SourceForge mirror) | `paintball2/` |
| Paintball 2 mapping resources, `paintball2_mapping_resources_b48.zip` | digitalpaint.org/files | `paintball2_mapping_resources/` |
| 47 i386 Ubuntu 24.04 packages (glibc, X11, libglvnd, Mesa with llvmpipe, libjpeg62 and their dependencies), fetched with `apt-get download` | archive.ubuntu.com | unpacked with `dpkg-deb -x` into `i386-root/` |
| `libpng12-0_1.2.54-1ubuntu1.1_i386.deb` (the game links libpng12, gone since Ubuntu 18.04) | archive.ubuntu.com, xenial | `i386-root/` |
| Paintball 2 alpha 040 engine source, for reading only | SourceForge | `downloads/src/` |

The Linux build is 32-bit only (the engine, `ref_pbgl.so` and `pball/gamei386.so`), and
this machine has no 32-bit libraries. The kernel runs 32-bit programs, so
`run-paintball2` starts the game through the unpacked `i386-root/.../ld-linux.so.2` with
`LD_LIBRARY_PATH` and `LIBGL_DRIVERS_PATH` pointing into `i386-root`. ericw-tools 2.0 was
already installed for Quake, at `~/.local/opt/quake/ericw-tools`; the mapping resources
ship the Windows build of the same tools, so it is what Paintball 2 mappers use. The
mapping guides on dplogin.com were not reachable (404s and a certificate for another
host), so the mapping rules came from the shipped `pb2ents_b41.c`, the FGD, the
resources' `sample.map` and the entities of the stock maps.

### Setting Heron up

In the welcome window the profile is set to Digital Paint: Paintball 2, and in
Preferences, Games, the game path is `~/.local/opt/paintball2/paintball2`. Heron mounts
it and `pball`, finds 637 textures under `pball/textures` (mostly `.jpg`, a few `.wal`
coloured with `pball/pics/colormap.pcx`) and loads `gamedata/DigitalPaintball2/pball2.fgd`.

### How a Paintball 2 map is made

- Worldspawn names the modes it supports in `gamemode` (1 deathmatch, 2 one-flag, 4 CTF,
  8 siege, 16 KOTH, 64 pong), the team count in `maxteams` and the colours in `team1` to
  `team4`. `sky` picks one of the skies in `pball/env` (`pbsky1` to `pbsky4`, `unit1_`,
  `riverscape1`, `arctic1`).
- `info_player_deathmatch` is the spawn; `teamnumber` makes it a team's. A `flag` with a
  `teamnumber` is that team's flag, and a `base` brush entity with the same `teamnumber`,
  usually made of `pball/trigger` around the flag, is where a carried flag scores.
- Any entity can carry `gamemode` to exist only in some modes.
- Textures are `pball/<name>`. A face picks up its surface flags from a `.wal` when the
  map gives it none: `sky1`, `sky2` and `sky3` are sky, `ltwhite` and `lit2_*` give
  light. Most textures exist only as `.jpg`, which `qbsp` reads for the size.

### Rerunning it

    cd demos/paintball_heron
    ./build.sh                  # qbsp -q2bsp, vis, light; installs the bsp and playtest.cfg
    cd ~/.local/opt/paintball2 && ./run-paintball2 +set vid_fullscreen 0 +set s_initsound 0 \
        +map heron_bases +exec playtest.cfg

`build.sh --remake` builds the map again in Heron first, through `make_map.py` and the
MCP server; set `HERON_BRUSH` to the binary and `XDG_CONFIG_HOME` to a scratch folder
whose preferences have the Paintball 2 game path. The game ran in a nested Xephyr display
with Mesa's llvmpipe. `+set autojoin 0` keeps the player an observer, and `addbot` gives
the other team a player so the match starts.

### What worked

- `qbsp` reported no leak and found every texture; `sky1.wal`'s flags made the lid sky.
  `light` found the 13 point lights, the sun and the sky dome. The only warnings are `qbsp` not finding a texture
  for its own `skip`, and `light` finding no `.wal` metadata for the `.jpg` textures,
  which is expected.
- In the game the console shows the map loading as "Team Flag CTF" with no errors from
  it. The player spawns on a team's spawn facing up the field; textures, the team-colour
  barrels, the flags on the decks, the sky and the sunlight are as in Heron. Bots join
  both teams and the "match begins" countdown starts.
- The resources' `sample.map` (a TrenchBroom Quake 2 map with light flags on faces) opened
  in Heron and saved again compiles to the same texinfos, planes and faces; the only
  difference is the `mapversion` key Heron adds.

### What was fixed in Heron

- **`heron-brush --mcp` did not set a map up for the game.** `new_map` and `open_map`
  never loaded the game's entity definitions, tags or face flag names, so a `light`
  placed through MCP got the keys of Heron's own `light` class (`light_energy`,
  `omni_range`, `shadow_enabled` and five more), which the game reports as bad fields
  on every load. Both now set the map up as the window does, with the game's FGD or the
  one `.fgd` beside the map.
- **Editing a face's flags threw away the texture's.** A face with no flags of its own
  showed zeroes, and setting one value gave it `0 0 <value>`. Once a face line carries
  flags `qbsp` no longer reads the `.wal`'s, so a sky face given a light value stopped
  being sky (checked with `qbsp`: `sky1 ... 0 1 300` compiles without `is_sky`). Heron
  now reads the flags in a `.wal`'s header, shows them for a face that has none, and
  starts from them when one is changed.

### Mapping mistakes, not Heron's

- A comment in a Paintball 2 `.cfg` must not contain `;`: the console splits on it first,
  and the rest of the comment was sent to the server as chat.
- `qbsp`'s `-gamedir` has to be absolute: a relative one is taken from the basedir's
  parent, and then no texture is found and `sky1` loses its sky flag.

### What remains

- **The Linux client is kicked about eight seconds into a game,** with "failed build
  check", and is then banned from the local server for a minute. The game code sends the
  client a hidden check that this build 48 client does not answer here. It happens on the
  stock `midnight` too, so it is not the map, and it was not worked around: the
  screenshots were taken in that window, each run a new start of the game.
- Heron saves every map as `// Format: Valve` with `"mapversion" "220"`. `qbsp` reads it,
  but the game reports `mapversion` as a bad field on load (harmless). The header names
  no game and a format the Paintball 2 profile does not list ("Quake2" and "Quake2
  (Valve)"); whether TrenchBroom opens it without asking was not checked.
- MCP has no tool to set a face's flags or one face's material, so a map built through
  MCP relies on the textures' own flags (as this one does for its sky).
- Heron's tags that match on content flags (Liquid, Detail) do not yet see the flags a
  face takes from its `.wal`.
- The entity browser lists Heron's built-in classes beside Paintball 2's.
