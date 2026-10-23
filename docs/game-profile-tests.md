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
| Quake3 | OpenArena, q3map2 | planned |
| Quetoo | Quetoo and its tools | planned |
| DigitalPaintball2 | Digital Paint: Paintball 2, ericw-tools | [done](#digital-paint-paintball-2) |
| Quake2 | the freely distributed demo, Yamagi Quake II, ericw-tools | [done](#quake-2) |
| DDayNormandy | D-Day: Normandy, which needs no Quake 2 data, ericw-tools | [done](#d-day-normandy) |
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
  write `0 0 0`, as TrenchBroom does. Since the Quake 2 test that is the Quake 3
  dialect's rule, and Neverball exports in it; Quake 2's leaves the numbers off a face
  without flags (see [Quake 2](#quake-2)).
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

## Quake 2

Tried on 2026-10-23 with the Quake 2 demo that id gave away in 1998, Yamagi Quake II
8.70 built from source as the engine and ericw-tools 2.0 for the compile. The level is
[demos/quake2_heron](../demos/quake2_heron/README.md): a hall and a courtyard open to
the sky, a door that a button opens, a trigger that brings in two guards and an
enforcer and switches the courtyard's lamps on, a teleporter, steps to a platform and
the exit, which loads the demo's first map. It compiles sealed, lights and plays
through to the next map.

### What was installed

Everything is under `~/.local/opt/quake2/`, without root. The downloads stay in
`downloads/`.

| What | From | Unpacked to |
| --- | --- | --- |
| The demo, `q2-314-demo-x86.exe` (39 MB, sha256 `7ace5a43...a1e`), unpacked with `7z`: `Install/Data/baseq2/pak0.pak` and the player skins | deponie.yamagi.org/quake2/idstuff, Yamagi's copy of id's idstuff folder | `yquake2/baseq2/` |
| Yamagi Quake II 8.70 source, `QUAKE2_8_70` | github.com/yquake2/yquake2 tags | `src/`, built into `yquake2/` |
| `libsdl2-dev` from Ubuntu 24.04, fetched with `apt-get download`, for the headers | the distribution's archive | `dev-root/` |
| ericw-tools 2.0.0-alpha11, the copy from the Quake test | | `~/.local/opt/quake/ericw-tools/` |
| Seven of ericw-tools' Quake 2 test maps, for the round trip | github.com/ericwa/ericw-tools, tag 2.0.0-alpha11, `testmaps/` | `downloads/ericw-testmaps/` |

Yamagi publishes no Linux binary. The machine has SDL2 (2.30) but not its headers, so
the build takes them from the unpacked `libsdl2-dev` through a small `sdl2-config` in
`dev-root/bin`, and links the system's `libSDL2`. It is built without cURL and OpenAL
(`make WITH_CURL=no WITH_OPENAL=no`), which the demo does not need; sound is off in the
runs anyway. `run-yquake2` starts it on `yquake2/` and, with `Q2_HOME` set, uses that
folder as the home, so the config, saves and screenshots go to
`$Q2_HOME/.local/share/YamagiQ2` instead of the real home.

The demo's pak holds three maps (`demo1` to `demo3`, the first unit's three levels),
303 textures under `textures/e1u1` and a few other folders, `pics/colormap.pcx`, the
sky `env/unit1_` and the models of seven monsters: the light guard, guard and shotgun
guard (`monster_soldier*`), enforcer (`monster_infantry`), gunner, berserker, parasite,
flyer and tank. The demo ships no `.map` sources, and id never published Quake 2's.

### Setting Heron up

In the welcome window the profile is set to Quake 2, and in Preferences, Games, the game
path is `~/.local/opt/quake2/yquake2`. Heron then lists `yquake2` and `yquake2/baseq2`
as the project paths, reads the textures and the palette out of `pak0.pak` and loads
`gamedata/Quake2/Quake2.fgd`. Both steps were done with a scripted run of the window,
in a nested Xephyr display; the preferences it wrote are the ones the MCP runs use.

### How a Quake 2 map is made

- There is no wad. A face names a texture under `textures/` in the game's folder or
  pak, such as `e1u1/metal1_2`, and each `.wal` carries the texture's default surface
  flags, content flags and value. A face line that ends after the scale takes them from
  there: `e1u1/sky1` is sky, `e1u1/clip` is player and monster clip, `e1u1/trigger` is
  not drawn, and the `baselt` and `ceil1` panels give off light at the value in the
  `.wal` (5000 for `baselt_3`). A face line with its own three numbers uses those
  instead, all of them.
- Worldspawn's `sky` names the sky box under `env/` (`unit1_` in the demo). ericw's
  `light` lights through the sky faces from `_sunlight` and `_sun_mangle`, and from a
  dome with `_sunlight2`.
- A `func_door` with a `targetname` stays shut until something fires it; `angle -1`
  opens it upwards and `wait -1` keeps it open. A `func_button` is pushed in the
  direction of its `angle` and fires its `target`.
- A monster with spawnflag 2 (trigger spawn) and a `targetname` is not in the level
  until it is fired. A `light` with a `targetname` can be switched; ericw's `light`
  gives each such light a style from 32 up, which the game code needs, and spawnflag 1
  starts it off.
- The exit is a `trigger_multiple` that fires a `target_changelevel`, whose `map` is
  the next map, optionally with `$` and the `targetname` of the start spot to use there
  (`demo2$base1`).
- A `misc_teleporter` sends the player to the `misc_teleporter_dest` its `target`
  names, and turns them to the destination's `angle`.

### Rerunning it

    cd demos/quake2_heron
    ./build.sh                  # qbsp -q2bsp, vis, light; installs the bsp and the configs
    Q2_HOME=/some/scratch ~/.local/opt/quake2/run-yquake2 +set vid_fullscreen 0 \
        +set vid_renderer gl1 +set s_initsound 0 +map heron_outpost +exec playtest.cfg

`build.sh --remake` builds the map again in Heron first, through `make_map.py` and the
MCP server; set `HERON_BRUSH` to the binary and `XDG_CONFIG_HOME` to a scratch folder
whose preferences have the Quake 2 game path. It saves `heron_outpost.map` in Heron's
own Valve dialect and exports `heron_outpost_q2.map` in Quake 2's, which is what `qbsp`
is given. The configs use Yamagi's `teleport` cheat (allowed in single player) to put
the player at each step, facing east. `playtest.cfg` presses the button, looks at the
open door, walks through the door and the trigger, and walks from the platform into the
exit. `lamps.cfg` looks along the courtyard before and after the trigger, and
`teleporter.cfg` steps onto the pad. The engine ran in a nested Xephyr display with
Mesa's llvmpipe and the `gl1` renderer.

### What worked

- `qbsp` reported no leak and found all 11 textures in `pak0.pak`. `light` found the
  8 lights, the sun and the dome, made 10 surface lights of the two ceiling panels and
  one switchable style for the courtyard's lamps. The only warnings are `qbsp` not
  finding a texture for its own `skip`, and `sky1.wal` asking for sky and nodraw at once,
  which comes from id's texture.
- In the engine the map loads as "Heron Outpost" with nothing from the map in the
  console. The button opens the door. The trigger prints "Intruder alert!", the guards
  and the enforcer appear (`listentities monsters` lists all three) and the courtyard's
  south wall goes from dark to lit. The pad takes the player from `64 384` to
  `1056 447` on the platform, facing west. The exit loads `demo1`.
- Four of ericw-tools' own Quake 2 test maps (`q2_liquids`, `q2_door`, `q2_detail`,
  `q2_playerclip`, with water, lava, slime, detail and clip flags) were opened in Heron
  through MCP and exported as Quake 2 (Valve). Each compiles to the same lump sizes and
  the same texinfos, flags and axes as the original. The only change in the text is
  `-0` written as `0`.

### What was fixed in Heron

- **A map exported for Quake 2 lost its sky, its clip and its light panels.** Since the
  Neverball fix every face in the Quake 2 dialect was written with `0 0 0` after its
  scale. Explicit numbers replace the `.wal`'s, so `e1u1/sky1` compiled as a plain wall
  (texinfo flags `0x0` instead of `0x4`) and the game drew the courtyard's lid as the
  yellow sky texture, and the hall's panels gave no light (`light`: "0 surface light
  points in use", 10 after the fix). Quake 3 now has a dialect of its own, which keeps the trio on every
  face for Neverball's `mapc`. Quake 2, Quake 2 (Valve) and Daikatana write it only for a
  face that has flags, as TrenchBroom does and as ericw's test maps are written.
  "Quake3" in a profile or a header now reads as the Quake 3 dialect, and the export
  window and MCP's `export_map` offer it.
- **Mirroring a brush turned its texture upside down.** With texture lock on, the sign
  of each new scale was taken against the face's old axes, not the new ones as in
  TrenchBroom's `ParaxialUVCoordSystem::transform`. The exit sign, mirrored across Y to
  read the right way round, came out upside down instead; a face turned half round on
  itself had its turn undone by two negative scales.

### Mapping mistakes, not Heron's

- `EXIT` on a face that looks west reads backwards: Quake 2 maps such a face with U
  running north, so from the courtyard the word is mirrored. id's maps flip such faces by
  hand. Here the sign's brush is mirrored across Y, which with texture lock mirrors the
  texture with it.
- The first play-test loaded `demo1` 31 times in a row. The config was still
  waiting when the player reached the exit, the changelevel's `gamemap` command queued
  behind the waits, and the `trigger_multiple` kept firing it while the player stood in
  it. A config must end with the walk into the exit.
- A walk up the steps was blocked by the enforcer; the config now teleports onto the
  platform instead.

### What remains

- Placing a light writes the FGD's defaults, `"style" "0"` and `"_cone" "10"`. The game
  skips keys that start with `_`, and style 0 is the default, so they change nothing.
- The monster and item models in Heron's 3D view were not looked at from inside the
  level; the screenshots show it from outside.
- Heron saves in Valve 220 with `"mapversion" "220"`; the Quake 2 export leaves it out.
  The saved map itself was not compiled.

## D-Day: Normandy

Tried on 2026-10-23 with D-Day: Normandy 5.03.3.1 and ericw-tools 2.0 for the compile.
The mod is standalone: it ships its own client (q2pro r666) and data and needs nothing
from Quake 2, so the demo's limits do not matter. The level is
[demos/dday_heron](../demos/dday_heron/README.md): a field with an Allied end and an
Axis end, each behind a low wall with a spawn for every class, a bunker in the middle
whose roof is the objective, crates and hedges, under a sky lit by a sun. It compiles
sealed, and in the game a player joins the Allies through the menus, spawns as
infantry, walks onto the bunker and takes it for 10 points.

### What was installed

Everything is under `~/.local/opt/quake2/`, beside Quake 2, without root.

| What | From | Unpacked to |
| --- | --- | --- |
| D-Day: Normandy 5.03.3.1, `D-Day.Normandy5.03.3.1.zip` (279 MB): the Linux and Windows clients and dedicated servers, `dday/gamex86_64.so` and the data, 75 maps | github.com/PowaBanga/DDaynormandyFPS releases (the same file Lutris installs) | `dday-normandy/` |
| The game code's source, for reading only | github.com/InFerYes/dday | `downloads/src/dday-src/` |

The 64-bit client runs with the system's libraries. `run-dday` starts it with the
mod's folder as `basedir`; with `DDAY_HOME` set it passes `+set homedir`, and q2pro then
writes its config, logs and screenshots there, not in the real home.

### Setting Heron up

In the welcome window the profile is set to D-Day Normandy, and in Preferences, Games,
the game path is `~/.local/opt/quake2/dday-normandy`. Heron mounts `dday`, finds the
`.wal` textures under `dday/textures` with `dday/pics/colormap.pcx` and loads
`gamedata/DDayNormandy/dday.fgd`. It warns about three files: `loosebob/sky1.wal`, whose
mip offsets overlap (the file is broken), and two 24-bit `.pcx` files in `ed/` that it
does not read.

### How a D-Day map is made

The mod's mapping briefing (on dday.planetquake.gamespy.com, now only in the web
archive) and the game code say:

- Two `info_team_start` entities make the teams. `obj_owner` is 0 for the Allies and 1
  for the Axis, `message` is the team's name and is required, `dll` picks the army
  (`usa`, `grm`, `gbr`, `rus` and others), and `points` and `kills` are what wins.
- Every class has its own spawn (`info_infantry_start`, `info_officer_start` and seven
  more) with the team's `obj_owner`. A player whose class has none spawns at the team's
  `info_reinforcements_start`. `info_player_start` is where a player waits as an
  observer.
- An `objective_touch` brush is taken by whoever touches it. `obj_owner` is the team
  that holds it at the start (99 for nobody), `health` is what the taker scores, `dmg`
  what the loser gives back, and `message` names it in "... taken by".
- The stock maps also ship a briefing picture, `dday/pics/objectives/<map>.pcx`, and
  bot navigation in `dday/navigation`; without them the console says so and the game
  goes on.

### Rerunning it

    cd demos/dday_heron
    ./build.sh                  # qbsp -q2bsp, vis, light; installs the bsp and playtest.cfg
    DDAY_HOME=/some/scratch ~/.local/opt/quake2/run-dday +set vid_fullscreen 0 \
        +set s_enable 0 +set logfile 1 +set deathmatch 1 +map heron_hedgerow +exec playtest.cfg

`build.sh --remake` builds the map again in Heron first, with the D-Day profile's
preferences in `XDG_CONFIG_HOME`; it writes the Valve map and the Quake 2 export as for
Quake 2. `playtest.cfg` picks Join, the Allies and infantry in the menus with `invnext`
and `invuse`, waits for the reinforcement wave, then walks east up the ramp onto the
bunker. q2pro's `wait` takes a frame count; nested aliases of waits stop with "Runaway
alias loop". The log goes to `logs/console.log` in the homedir.

### What worked

- `qbsp` reported no leak and found all 9 textures; the warnings are the same as for
  Quake 2.
- In the game the map loads as "Heron Hedgerow". The console has only the missing
  briefing picture and bot nodes. The menus offer both teams with their counts and all
  nine classes; the player joins the Allies, becomes infantry and spawns in the gap of
  the Allied wall. On the roof the screen shows "The bunker taken by: Tuxnewbie Allies"
  and the Allies' points go to 10.

### What was fixed in Heron

- **The FGD said the Axis is team 10.** `info_team_start`'s `obj_owner` was described as
  "0-allies or 10-axis". The game code indexes its two-entry team list with it, so 10
  writes past the end; the stock maps and the briefing use 1. The spawn classes also
  gave `angle` the default `"0 0 0"`, three numbers for a key the game reads as one yaw.

### Mapping mistakes, not Heron's

- The first runs spawned the player looking at a wall. D-Day 5.03 puts a new player at
  the class spawn but keeps the view they had as an observer at `info_player_start`,
  whose `angle` was 90. That start now looks east, the Allies' way.
- A hedge on the centre line stopped the walk to the bunker; the hedges now flank it.
- The team menu's first line is Auto Select, which put the first runs on the Axis.

### What remains

- Only one player was in the game. No bots were tried and no Axis player took the bunker
  back; the map has no bot navigation.
- No briefing picture was made for the map.
- Heron does not read 24-bit `.pcx` textures. Quake 2 engines draw `.wal`, so this
  matters only for the two leftover files in `ed/`.
