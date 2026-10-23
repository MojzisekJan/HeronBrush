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
| Neverball | Neverball and its `mapc` | [done](#neverball) |
| Quake3 | OpenArena, q3map2 | [done](#openarena-quake-3) |
| Quetoo | Quetoo and its tools | planned |
| DigitalPaintball2 | Digital Paint: Paintball 2, ericw-tools | [done](#digital-paint-paintball-2) |
| Quake2 | the freely distributed demo, a source port | planned |
| DDayNormandy | the free mod on the Quake 2 demo | planned |
| Hexen2 | the freely distributed demo | [done](#hexen-ii) |

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

## Neverball

Tried on 2026-10-23 with Debian's Neverball 1.6.0 (a 2018 snapshot), its `mapc` and
its data. The level is [demos/neverball_heron](../demos/neverball_heron/README.md): a
start pad, a ramp up to a deck, a switch that starts a lift across a gap, a teleporter
as a shortcut, and the goal on the far deck, with ten coins to open it and 90 seconds.
It compiles, loads, and a scripted run plays it to the goal: the switch starts the
lift, the ball rides it across, and the goal ends the level with 11 coins in 33
seconds.

### What was installed

Everything is under `~/.local/opt/neverball/`, without root. The packages stay in
`downloads/` and are unpacked with `dpkg -x` into `root/`.

| What | From | Unpacked to |
| --- | --- | --- |
| `neverball` 1.6.0+git20180603-3build2 (the game and `mapc`) | Ubuntu 24.04, `apt-get download` | `root/usr/games/` |
| `neverball-common`, `neverball-data`, same version (the data: `textures/`, `map-*/*.sol`) | same | `root/usr/share/games/neverball/` |
| `libsdl2-ttf-2.0-0`, `libopenhmd0`, `libhidapi-libusb0` (libraries the game needs that the system lacks) | same | `root/usr/lib/x86_64-linux-gnu/` |
| Neverball's source, `git clone --depth 1` (commit b1a383a), for the `.map` sources and `mapc` | github.com/Neverball/neverball | `downloads/neverball-src/` |

The game runs with `LD_LIBRARY_PATH=root/usr/lib/x86_64-linux-gnu`. The data's two
fonts are symlinks into `/usr/share/fonts`, which `dpkg -x` leaves dangling; a link
`root/usr/share/fonts/truetype/dejavu` to the system's DejaVu folder brings the text
back. The packaged data has only compiled `.sol` levels, so the `.map` sources come
from the source tree. Its `mapc` is newer than the packaged one: the 1.6 `mapc` was
read from the source as of the package's date, commit 4ecb092.

### How Neverball levels are made

A level is a Quake 3 style `.map` that `mapc map.map data-folder` turns into a `.sol`.
`mapc` reads line by line: an entity's `{` and its keys start in the first column, and a
face is `( p1 ) ( p2 ) ( p3 ) material xoff yoff rot xscale yscale contents flags value`.
It needs all three numbers after the scale; any content flag makes the brush detail
(no collision). Materials are `mtrl/...`, files under `textures/` that name an image.
The 1.6 `mapc` knows these classes:

- `worldspawn` keys: `time` (hundredths of a second), `goal` (coins to open the goal),
  `message` (`\` breaks a line), `back` and `grad` (background level and gradient),
  `song`, `version`, `author`.
- `info_player_start` is the ball, 24 units above the floor; `info_player_deathmatch`
  the goal, also 24 up; `light` a coin worth its `light` key; `item_health_large` and
  `item_health_small` grow and shrink.
- `func_train` is a moving body that follows the `path_corner` its `target` names; a
  corner's `speed` is the seconds to the next one, and two corners in one place make a
  pause. The body's brushes are placed where the path starts.
- `info_camp` is a switch: rolling into it toggles the whole path loop its `target`
  names, `state` 0 being off. `target_teleporter` jumps the ball to a `target_position`.
- `info_player_intermission` with a `target_position` is the camera before the start.
- `func_group` is not a class the 1.6 `mapc` knows. It still reads the brushes, but
  puts them in no body, so they are neither drawn nor collided with: in the level they
  are gone, and `mapc` prints the same counts as if they were there. The current source
  makes a body of a `func_group` with brushes.

A level does not need a set: `neverball -l map-heron/heron_lift.sol` plays one `.sol`
standalone, from the data folder or from `~/.neverball/`. The game also records every
run to `Replays/Last.nbr`, which `neverball -r /full/path.nbr` plays back.

### Setting Heron up

In the welcome window the profile is set to Neverball, and in Preferences, Games, the
game path is `~/.local/opt/neverball/root/usr/share/games/neverball`. Heron then lists
Neverball's `mtrl/...` materials, and a new map starts from the profile's initial map.
Heron saves in Valve 220, which `mapc` cannot read: the level for `mapc` is written with
File, Export, Map in the Quake 2 dialect, which the export window now offers first for
Neverball, or with the MCP tool `export_map`.

Groups and layers are written as `func_group` entities, which the 1.6 `mapc` loses (see
above). The profile's `heron.mergeGroupsOnExport` key makes the export merge them: every
brush of a group or an extra layer goes into the worldspawn, and a `func_train` in a
group stays a `func_train` with its own brushes. The export window and `export_map` say
how many groups and layers were merged. Save does not merge: the saved map is the one
you keep editing, and it keeps its groups and layers. That is why `mapc` is always given
the export, never the saved file, which it could not read anyway.

### Rerunning it

    cd demos/neverball_heron
    ./build.sh            # mapc, then copies the .sol to ~/.neverball/map-heron/
    LD_LIBRARY_PATH=~/.local/opt/neverball/root/usr/lib/x86_64-linux-gnu \
        ~/.local/opt/neverball/root/usr/games/neverball \
        --data ~/.local/opt/neverball/root/usr/share/games/neverball \
        -l map-heron/heron_lift.sol

`build.sh --remake` builds the map again in Heron first, through `make_map.py` and the
MCP server; set `HERON_BRUSH` to the binary and `XDG_CONFIG_HOME` to a scratch folder
whose preferences have the Neverball profile and game path. `NEVERBALL_HOME` puts the
level in another home's `.neverball`. The game ran in a nested Xephyr display with
Mesa's llvmpipe, with `HOME` pointing at a scratch folder.

`playtest.py DISPLAY SHOTS_DIR` plays the level on that display with XTest key presses,
once the level's intro is showing and the neverballrc has a top-down camera
(`view_fov 50`, `view_dp 900`, `view_dc 0`, `view_dz 250`). The ball stays at the centre
of the screen, so the script measures how far the floor slides under it between frames
and steers by that; on the lift it keeps the ball over the lift's middle, found by its
colour. It saves a screenshot at each step.

### What worked

- The exported map compiles with no warnings: 7 materials, 11 coins, 1 goal, 1 switch,
  1 teleporter, 4 path corners, 2 bodies. All the brushes made in Heron arrive with
  their textures, turf on top and grey on the sides.
- In the game the intro shows the three lines of the message, the ball spawns on the
  start pad, the coins count up, the goal opens at ten, the switch starts the lift, the
  teleporter puts the ball on the deck, and the goal ends the level ("New Record",
  0:33, 11 coins). The run's replay plays back to the end.
- With the profile's assets mounted, the editor draws the ball start and the goal ring
  of the initial map with the profile's models instead of empty boxes. The coin,
  switch and teleporter models are converted the same way but were not looked at.

### What was fixed in Heron

- **Brushes made in Heron vanished in `mapc`.** In the Quake 2 dialect a face with no
  surface flags was written without them, and `mapc` skips a face line without all
  three numbers: the level compiled with no geometry and no warning. Such faces now
  write `0 0 0`, as TrenchBroom does.
- **The Neverball profile had no format.** "Quake3", "Quake2" and "Daikatana" in a
  profile were not recognised, so a new Neverball map did not start from the profile's
  initial map, and the export window offered Valve 220, which `mapc` cannot read. They
  now read as Quake 2 face lines, and the export window offers the game's first format
  for a map in Heron's own dialect.
- **Messages gained empty lines.** Heron wrote every backslash in a value doubled, so a
  Neverball message `a\b` showed an empty line between the two. A backslash is now
  doubled only where the reader would take it for an escape. The same change fixes
  Quake messages that use `\n`.
- **59 of Neverball's 368 maps would not open.** Their messages end in a backslash,
  `...coins.\"`, and Heron read `\"` as an escaped quote. A `\"` at the end of a line
  now closes the string, as TrenchBroom reads it.
- **The profile's editor models were never found.** A profile's `assets` folder is now
  mounted under the game's files, as TrenchBroom does. Neverball's models there were in
  metres and Y-up, a quarter of a unit across in Heron; they are converted to map units.
- **MCP could not put a material on one face or export a dialect.** New tools:
  `set_face_material` (one face by index, or every face facing a direction) and
  `export_map`.
- **Grouped and layered brushes vanished in the level.** Heron wrote groups and extra
  layers as `func_group`, whose brushes the 1.6 `mapc` puts in no body. An export for
  Neverball now merges them into the worldspawn; the saved map keeps them. The demo
  has its start walls in a group and the goal deck on a layer of its own, made with the
  new MCP tool `move_selection_to_layer`. Exported without the merge, the compiled
  `.sol` had 22 brushes of which 9 were in no body (the four start walls and the five
  goal deck brushes): the goal deck was missing. With the merge, all 22 are in a body,
  the same as before the demo had a group or a layer. The count comes from reading the
  bodies' lump ranges out of the `.sol`; `mapc`'s own summary is the same either way.

### Mapping mistakes, not Heron's

- A first layout had the lift cross a gap only 128 wide and wait nowhere: the ball
  rolled straight over the gap at speed, and the lift left as the ball arrived. The gap
  is now 384 wide, the lift waits three seconds at each end, and its platform is 192
  square so the ball stays on while it speeds up.
- Holding the tilt key all the way up the ramp sends the ball over the whole level in
  six seconds. Neverball's physics reward a light touch.
- The textures are at Heron's default scale of 1; Neverball's own maps use 0.5, which
  makes the turf finer.

### What remains

- MCP's `new_map` starts an empty Valve map without the game's initial map, FGD or
  `// Game:` header; the script loads the FGD itself.
- Neverball's TrenchBroom profile names no default UV scale, so new faces get 1, not the
  0.5 Neverball's own maps use. The official gamepack in Neverball's source sets
  `"faceattribs": { "defaults": { "scale": [0.5, 0.5] } }`; whether Heron honours that key
  was not checked.
- MCP cannot set content flags, so a decal (the goal's target, the start arrow) cannot
  be made detail through MCP; the level has none.
- The `__tb_fancy_path_corner.obj` the FGD names for `path_corner` is not in
  TrenchBroom's copy of the profile either, so path corners stay boxes.
- The profile is no longer marked experimental: following this section, the export
  gives a level that compiles and plays, with groups and layers. The merge was checked
  with `mapc` and the `.sol`; the regrouped demo was not played through again, since its
  brushes and bodies are the same as the version that was.

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

## Hexen II

Tried on 2026-10-23 with the freely distributable Hexen II demo (November 1997, v1.11)
on Hammer of Thyrion 1.5.9, compiled once with ericw-tools and once with Raven's own
tools as Hammer of Thyrion ships them. The level is
[demos/hexen2_heron](../demos/hexen2_heron/README.md): a hall and a chapel, a door that a
button opens, a trigger that wakes an ambush of imps, an archer and a spider and fades
the chapel's lights up, steps to a dais and the exit, which sends the player on to the
demo's first map. Both builds compile sealed, light and play through to the exit.

### What was installed

Everything is under `~/.local/opt/hexen2/`, without root. The archives stay in
`downloads/`; all of them are on sourceforge.net/projects/uhexen2.

| What | From | Unpacked to |
| --- | --- | --- |
| The demo for Linux x86_64, `hexen2demo_nov1997-linux-x86_64.tgz`: the demo's `data1/pak0.pak` and Hammer of Thyrion built for it (`glhexen2`) | Hexen2Demo-Nov.1997 | `hexen2demo_nov1997/` |
| `libsdl1.2debian` (sdl12-compat) and `libmad0` from Ubuntu 24.04, fetched with `apt-get download` | the distribution's archive | `lib/` |
| Hammer of Thyrion utilities 1.5.9 (`qbsp`, `vis`, `light`, `pakx` and others) | Hammer of Thyrion/1.5.9/Linux-x86_64 | `hot-utils/utils/` |
| Hammer of Thyrion source 1.5.9 and the HexenC archive, to read the compiler's map parser and the game code | Hammer of Thyrion/1.5.9/Source, Hexen2 GameData/hcode_archive | `src/`, `hcode/` |
| ericw-tools 2.0.0-alpha11, the copy from the Quake test | | `~/.local/opt/quake/ericw-tools/` |

The engine is linked against SDL 1.2 and libmad, which this machine does not have, so
`run.sh` puts `lib/` on the library path and starts `glhexen2`. The demo build refuses
`-game` ("You must have the full version of Hexen II to play modified games"), so the
map is installed in the demo's own `data1/maps`. The engine keeps its config and
screenshots in `~/.hexen2demo`, which it finds through the password database, so
setting `HOME` does not move it.

The demo has no texture wad. Raven's maps name `gfx/castle.wad` and others in
worldspawn, but those files stayed at Raven. What Hexen II mappers have is the textures
inside the game's bsp files, pulled out with a tool into a WAD2. The demo's four maps
hold 179 textures, among them Raven's own `clip`, `origin` and a `TRIGGER` texture
called `rtex114`. `demos/hexen2_heron/extract_wad.py` gathers them into `h2demo.wad`.
It is WAD2, so it needs Hexen II's palette, `gfx/palette.lmp` in `pak0.pak`.

### Setting Heron up

In the welcome window the profile is set to Hexen 2, and in Preferences, Games, the game
path is `~/.local/opt/hexen2/hexen2demo_nov1997`. Heron then mounts it and `data1` with
its pak, which is where the palette and the monster models come from. The profile
loads `gamedata/Hexen2/Hexen2.fgd`. The wad is added from the Materials tab and named in
worldspawn's `wad` key relative to the map, as for Quake.

### Rerunning it

    cd demos/hexen2_heron
    ./build.sh                  # makes the wad, runs qbsp -hexen2, vis and light
    ./build.sh --raven          # the same with Raven's qbsp, vis and light
    ~/.local/opt/hexen2/run.sh -window -width 800 -height 600 \
        +map heron_keep +exec playtest.cfg

`build.sh --remake` builds the map again in Heron first, through `make_map.py` and the
MCP server, with `HERON_BRUSH` and `XDG_CONFIG_HOME` set as for Quake. It saves
`heron_keep.map` in Heron's own Valve dialect and exports `heron_keep_hexen2.map` in
Hexen II's, which is what both compilers are given. `playtest.cfg` sets
`host_framerate 0.02`, so every frame is 1/50 s and a key held for a number of waits
moves the player the same distance on every run. It walks into the button, prints the
door's edict, steps in front of the doorway and walks east through the trigger, up the
steps and into the exit, printing `viewpos` and taking a screenshot at each step. The
engine has no `setpos`. It ran in a nested Xephyr display with Mesa's llvmpipe.

### What worked

- Both compilers built the map with no leak. Raven's `light` keeps the ambush lights'
  style 33; ericw's `light` moves the targeted lights to style 32 and writes that back
  into the entities, so the game code fades style 32 instead. Both fade up in the game.
- In the engine the console shows the level's name, "The Forgotten Chapel", and nothing
  from the map. The button prints "You hear a door across the room open" and the door
  goes from `0 0 0` to `0 0 119.9`; the chapel is dark from the doorway and lit once
  the trigger fires; the monsters spawn and attack; the steps take the player onto the
  dais and the exit loads Blackmarsh.
- Textures and the HUD come out in the right colours, in the editor and in the game,
  through the palette from `pak0.pak`. The demo's imp, archer, spider and torch models
  are in Quake's `IDPO` version 6 layout, the one Heron's model reader takes; this was
  checked by walking the files, not by looking at them in the editor.

### What was fixed in Heron

- **A Hexen II map could not be opened.** Hexen II's dialect is the Standard one with
  one more number at the end of every face line. Heron read the number as the start of
  the next face and stopped with a parse error, so no map in TrenchBroom's Hexen 2
  format or Raven's would open. The reader now takes the number and says the map is Hexen II, with or
  without a `// Format: Hexen2` header.
- **Raven's qbsp could not compile anything Heron wrote.** It reads that number as the
  brush's light value and stops with "Line 9 is incomplete" without it, on Valve and
  Standard maps alike. Hexen 2 is now a dialect: the export window's `.map` tab offers
  it, and it writes `0` there, as TrenchBroom does. A scripted run of the window wrote
  the same file as MCP's export, ids aside. The profile's `"format": "Hexen2"`
  entry, which the config reader skipped, now means it.
- **`'mapversion' is not a field`.** A Valve map carries `"mapversion" "220"` in
  worldspawn, the compilers pass it into the bsp, and the Hexen II engine prints that
  line on every load. A dialect without a map version, Hexen 2 among them, now leaves
  a carried `mapversion` out instead of claiming to be Valve 220.
- **MCP could only save in Valve.** A new `export_map` tool writes a copy in another
  dialect (`Hexen2`, `Standard`, `Quake2`, `Quake2 (Valve)`) and says what it lost; the
  open map stays as it was. `make_map.py` uses it for `heron_keep_hexen2.map`.
- **The FGD's Egyptian torch spawned nothing.** Raven's game code spells it
  `light_torch_eqypt`; the definition now does too. The `message` keys of worldspawn,
  buttons, doors and triggers are integers in the FGD now, since Hexen II reads them as
  line numbers in `strings.txt`.

### Mapping mistakes, not Heron's

- Hexen II's messages are numbers, not text. `"message" "Ambush!"` reads as 0 and prints
  nothing; `"message" "20"` prints line 20 of `strings.txt`. The level's name is line 125,
  "The Forgotten Chapel", because the demo's strings had nothing better.
- A light toggled by a trigger is targeted like a monster: the trigger's `target` finds
  the light by `targetname`, and a style of 32 or more makes the game fade it from
  `lightvalue1` to `lightvalue2`. The trigger's own `style` and its "light toggle" flag
  look like the way to do it, but `SUB_UseTargets` clears the trigger's style before it
  looks at the targets.
- `trigger_changelevel` takes `map` and, as a second key, the `targetname` of the
  `info_player_start` to arrive at. Without it the game picks the first start.
- Only monsters whose models are in the demo's pak can be placed: imps, archers,
  spiders, skull wizards, hydras, rats and the sheep. The FGD also offers golems,
  medusas and others whose models the demo does not have; placing one was not tried.
- ericw's `qbsp -hexen2` warns that it cannot find a `skip` texture, and its `light`
  then warns about a texture with no name. Neither comes from the map.

### What remains

- The Hexen II number is read and dropped, and written as 0. Raven's `qbsp` passes it on
  as a light value per face; no map tested here sets it, but a Raven source that did
  would lose it in Heron.
- The editor's view of the monster models from inside the level was not looked at;
  the 3D view in the screenshots shows the level from outside.
- Placing a light still writes the FGD's defaults (`style`, `lightvalue1`, `lightvalue2`,
  `fadespeed`). For Hexen II they are the game's own defaults, so they change nothing.

## OpenArena (Quake 3)

Tried on 2026-10-23 with OpenArena 0.8.8, the free game on the Quake 3 engine, and the
q3map2 and mbspc that ship with NetRadiant-custom. The level is
[demos/openarena_heron](../demos/openarena_heron/README.md): a courtyard under a sky
shader with a ledge along one side, a jump pad and stairs up to the ledge, and a room
whose teleporter puts the player on it; five spawns, four weapons, armour, health and
ammunition. It compiles sealed, lights from the sky and the room's lamps, and plays with
bots: the jump pad lands the player on the ledge, the teleporter on its destination,
and three bots find their way about and frag each other.

### What was installed

Everything is under `~/.local/opt/openarena/`, without root. The archives stay in
`downloads/`.

| What | From | Unpacked to |
| --- | --- | --- |
| OpenArena 0.8.8, `openarena-0.8.8.zip` (the game with `openarena.x86_64`, and `baseoa/*.pk3`) | download.tuxfamily.org/openarena/rel/088 (the official mirror) | `openarena-0.8.8/` |
| `libsdl1.2debian` 1.2.68-2 (sdl12-compat), which the game links and the system lacks | Ubuntu 24.04, `apt-get download` | `root/usr/lib/x86_64-linux-gnu/` |
| NetRadiant-custom 20260114, Linux x86_64 AppImage, extracted with `--appimage-extract` (q3map2 2.5.17n, mbspc and the editor's OpenArena gamepack) | github.com/Garux/netradiant-custom releases | `netradiant-custom/squashfs-root/usr/bin/` |
| The sources of OpenArena's stock map "Aggressor" (`oad_aggressor.map`, brush primitives, and `aggressor+++.map`, with patches), for the round trip | github.com/NeonKnightOA/nk-oa-assets, `maps/DM/aggressor` | `downloads/nk-oa-assets/` |

`run-openarena` starts the game with `root/` on the library path and the install as
`fs_basepath`. The game ran with `+set fs_homepath` pointing at a scratch folder, so
`~/.openarena` was never made. It keeps a pid file in `$TMPDIR` (`/tmp/ioq3+oa.pid`
by default); after a run that was killed rather than quit, the next start waits on a
zenity question about safe video settings, so the runs set `TMPDIR` to a scratch folder
too and end with `quit`.

### How OpenArena maps are made

The rules come from q3map2's own documentation in NetRadiant-custom's `docs/`, the
NetRadiant-custom OpenArena gamepack and the stock maps' sources.

- A face names a shader as `skies/q3dm0`; q3map2 puts `textures/` in front. A shader is
  declared in `scripts/*.shader` inside a pk3: its `qer_editorimage` is what an editor
  shows, its `surfaceparm`s make it sky, water, nodraw and so on, and `q3map_sun` and
  `q3map_surfacelight` make it give light. A texture with no shader is an image under
  `textures/`. OpenArena's `scripts/shaderlist.txt` decides which shader files q3map2
  reads; the engine reads them all.
- `common/caulk` is drawn nowhere and lit by nothing, so every face no player sees wears
  it. `common/trigger` is for trigger brushes, `common/clip` and its kin block without
  being drawn. Small or decorative brushes are made detail (content flag 0x8000000) so
  they do not split the vis tree.
- Spawns are `info_player_deathmatch`; weapons, ammunition and items are point entities
  16 above the floor, and fall onto it. A jump pad is a `trigger_push` brush whose
  `target` is a `target_position` at the top of the arc. A teleporter is a
  `trigger_teleport` brush aimed at a `misc_teleporter_dest`, whose `angle` is the way
  the player faces on arrival. Lights are `light` entities (`light` and `_color`), or
  shaders that emit light; a sky shader with `q3map_sun` lights the outdoors.
- The compile is three runs of q3map2 on the same file: `-meta` (the bsp), `-vis` and
  `-light`. q3map2 has no OpenArena game of its own: it is `-game quake3` with
  `-fs_basegame baseoa`. Bots need an `.aas` file from mbspc's `-bsp2aas`. A map is
  shipped as a pk3 with the bsp, the aas and a `scripts/<map>.arena` file that names its
  game types.
- q3map2 reads three kinds of face line: Quake 3's own (the Quake 2 one with the surface
  flags trio), brush primitives (`brushDef`, a texture matrix per face) and, in
  NetRadiant-custom's build, Valve 220. Patches are `patchDef2`.

### Setting Heron up

In the welcome window the profile is set to Quake 3. In Preferences, Games, "Where it is"
is the OpenArena folder, `~/.local/opt/openarena/openarena-0.8.8`, and Mods is `baseoa`:
the profile's search path is Quake 3's `baseq3`, which OpenArena does not have, and the
Mods field is how its own folder is named, as in TrenchBroom. The project paths then
list `baseoa`, the install and the missing `baseq3`, and Heron mounts the nine pk3s in
`baseoa`. The material browser lists OpenArena's textures and shaders under the names a
map uses (`skies/q3dm0`, `common/caulk`), and the profile loads
`gamedata/Quake3/entities.ent`.

### Rerunning it

    cd demos/openarena_heron
    OA_HOME=/scratch/oahome ./build.sh   # q3map2, mbspc, then heron_court.pk3 into $OA_HOME/baseoa
    TMPDIR=/scratch/tmp ~/.local/opt/openarena/run-openarena +set fs_homepath /scratch/oahome \
        +set r_fullscreen 0 +devmap heron_court +exec playtest.cfg

Without `OA_HOME` the pk3 goes into `~/.openarena/baseoa`, where the game looks by
default. `build.sh --remake` builds the map again in Heron first, through `make_map.py`
and the MCP server; set `HERON_BRUSH` to the binary and `XDG_CONFIG_HOME` to a scratch
folder whose preferences have the Quake 3 profile, the game path and the mod.
`build.sh --primitives` compiles the brush-primitive export instead. `playtest.cfg`
turns god mode on, uses `setviewpos` to drop the player on the jump pad and into the
teleporter, prints `viewpos` after each, adds three bots and takes screenshots. The game
ran in a nested Xephyr display with Mesa's llvmpipe at 30 frames a second.

### What worked

- q3map2 compiled the map with no leak and no warning: every shader was found, the light
  stage counted 8 point lights, 17 area lights from the shaders and the sky's sun. mbspc
  found the jump pad and the teleporter as bot routes.
- In the game the console shows nothing from the map. The jump pad throws the player
  from `0 96` to the ledge (`viewpos` reads `0 496 202` on landing), and the teleporter
  puts the player at the destination, `-405 384 210`, facing east. The bots load the
  `.aas`, run about the yard and the ledge and frag each other, one of them with the
  rocket launcher from the ledge. Textures, the sky
  and the light panels are where Heron draws them.
- The map saved in Heron's Valve dialect and its brush-primitive export compile to the
  same bsp: the same planes, brushes, surfaces, vertices and texture coordinates; only
  the lightmaps differ, as they do between two runs of the light stage.
- Round trip of OpenArena's own sources: `oad_aggressor.map` (835 `brushDef` brushes)
  opened in Heron, saved as Valve 220 and exported again as brush primitives, compiles
  to the same bsp as the original in both cases. Texture coordinates differ by at most
  0.000002; the only other change is `"mapversion" "220"` in the Valve save's entities.
  `aggressor+++.map` (Quake 3 face lines, four patches) gives the same bsp through Heron
  too, patches included. Exporting the brush-primitive copy as Quake 3 face lines
  instead changes the texture alignment, as Heron warns it will (4892 faces lose their
  own axes).

### What was fixed in Heron

- **Shaders had the wrong names.** Heron listed a game's shaders under their full names,
  `textures/skies/q3dm0`, while images and maps name them without the texture root. A
  face naming `skies/q3dm0` found no material, a sky picked in the browser would have
  been written as `textures/skies/q3dm0`, which q3map2 reads as
  `textures/textures/skies/q3dm0` (no sky, no sun), and caulk and the sky were listed
  twice, once without their surfaceparms. Shaders are now named as maps name them and
  replace the image of the same name; the model, icon and effect shaders outside
  `textures/` are no longer offered as brush materials.
- **MCP could not write brush primitives.** `export_map` takes `quake3_brushdef`, the
  format the export window offers as "Quake 3 (brush primitives)", which is what
  `make_map.py` uses for `heron_court_brushdef.map`.
- **The entity definitions warned on every start.** Four classes declared `targetname`
  twice, and `gridsize` and `_blocksize` were integers with three numbers as their
  default; the status bar showed it in red. The duplicates are gone and the two keys are
  strings.

### Mapping mistakes, not Heron's

- A weapon placed next to the teleporter pad stood on it; items drop onto whatever is
  below them, so they want a clear floor.
- `screenshot` in the console is taken at the end of a later frame, so a script that
  moves the player straight after it photographs the new place. The play-test waits a
  few frames after each one.
- `god` in a config run with `+devmap` does nothing before the player has entered; it
  goes after the first wait.
- mbspc garbles a long path to the bsp ("error opening" a name with junk in it), so
  `build.sh` runs it from the bsp's folder.

### What remains

- New faces get a UV scale of 1. The profile asks for 0.5 (`faceattribs.defaults`), the
  scale Quake 3's textures are drawn at, and Heron read the key without using it. The
  Quetoo profile test's branch has a change that makes the shape tools honour it. Until that is merged the
  textures in this map are twice the size a Quake 3 mapper would give them.
- MCP cannot set content flags, so `make_map.py` cannot make its pillars detail.
  Whether the GUI's face attribute editor sets the profile's `detail` flag and q3map2
  then treats the brush as detail was not checked.
- The brush-primitive export refuses the whole map when one texture has no size: the
  Aggressor variant with patches names a `NULL` texture that is not in OpenArena, and
  could not be exported that way. q3map2 takes a missing image as 64 by 64.
- The profile's search path is `baseq3`, so every OpenArena setup lists a missing folder
  among the project paths and warns that it is not there.
- `gamedata/Quake3/entities.ent` is Quake 3's: OpenArena's extra classes (nail gun, chain
  gun, prox launcher, the Team Arena items, domination points) are not in it.
  NetRadiant-custom's gamepack has them in `oa.game/baseoa/entities.ent`.
- Placing an entity writes the definition's defaults into it (`wait`, `random`, `count` on
  weapons, `radius`, `fade` and five more on lights). They are the game's own defaults,
  but a weapon's `wait` overrides the server's `g_weaponRespawn`.
- A map saved by Heron carries `"mapversion" "220"` into the bsp's entities. OpenArena
  ignores it.
- The editor's 3D view was only looked at from outside the level with the roofs deleted;
  flying the camera through it from a script was not worked out.

The profile could drop "experimental": OpenArena's pk3s, shaders and textures load,
Heron's Valve save and its brush-primitive export both compile with q3map2 to exactly
what the original map gives, and patches survive the round trip. The UV scale default
is the one thing a Quake 3 mapper would notice at once, so I would drop the flag once
that change is merged.
