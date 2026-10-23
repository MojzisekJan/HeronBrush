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
| Quetoo | Quetoo and its `quemap` | [done](#quetoo) |
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
- `func_group` is not read by the 1.6 `mapc`: its brushes compile into no body and
  vanish. The current source reads it.

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

- `func_group` and Heron's extra layers are written as `func_group` entities, which the
  1.6 `mapc` drops. A Neverball map has to keep its brushes in the default layer and out
  of groups, or use a newer `mapc`.
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
- The profile is still marked experimental. With the export in the Quake 2 dialect it
  produced a working level; the groups limit above is the main trap left for a user.

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

## Quetoo

Tried on 2026-10-23 with Quetoo 1.0.133 for Linux, its compiler `quemap` and its game
data 1.0.64. The level is [demos/quetoo_heron](../demos/quetoo_heron/README.md): a
deathmatch hall under the sky with a ledge along one side, reached by stairs or a jump
pad, and a side room whose teleporter puts the player back on the ledge, with six
spawns, four weapons, ammo, armour and health. It compiles with no warnings, loads with
no errors from the map, and four bots play it.

### What was installed

Everything is under `~/.local/opt/quetoo/`, without root. The archives stay in
`downloads/`.

| What | From | Unpacked to |
| --- | --- | --- |
| Quetoo 1.0.133, `quetoo-x86_64-pc-linux.tar.gz` (the engine, `quemap`, the game modules and the libraries they need) | github.com/jdolan/quetoo releases | `quetoo/` |
| Quetoo data 1.0.64, `quetoo-data.zip` (1.1 GB: textures, models, sounds, and the 25 stock maps with their `.map` sources) | github.com/jdolan/quetoo-data releases | `quetoo/share/quetoo/default/` |
| Quetoo's source, `git clone --depth 1` (commit ccc8fc2), to read `quemap`'s map reader and the game code | github.com/jdolan/quetoo | `downloads/quetoo-src/` |
| TrenchBroom's current Quetoo profile, for comparison | TrenchBroom's master branch | `downloads/tb-quetoo/` |

The Linux build bundles its libraries and runs here as it is. The engine looks for its
data in `share/quetoo` beside its `bin/`, which is where the zip goes. On every start it
asks GitHub for a newer engine and installs it over itself; an empty `.managed` file in
`quetoo/` stops that, as it does for a distribution's package. It still checks its data
against the manifest on Quetoo's server.

Quetoo keeps its log, config, screenshots and a player's own maps in
`$XDG_DATA_HOME/WickedOldGames/Quetoo/default`, and on start it may write a desktop
entry, links in `~/.local/bin` and a Discord registration. Every run here had `HOME`,
`XDG_DATA_HOME` and `XDG_CONFIG_HOME` pointing at a scratch folder, `SDL_AUDIO_DRIVER`
set to `dummy` and `cl_analyticsUrl` set to 0. One `quemap --help` run before that left
a log in the real `~/.local/share/WickedOldGames`, which was removed.

### How Quetoo maps are made

Quetoo's mapping documentation is its game code and its own maps: the wiki has nothing
on mapping, and the `QUAKED` comments in `src/game` describe every class. The stock maps
were made in TrenchBroom.

- `quemap -w DIR -bsp maps/NAME.map` compiles, lights and writes the map's manifest in
  one go; this map takes 0.3 seconds. The map is looked up on the search path, so it
  sits in `DIR/maps`. The client wants the manifest `NAME.mf` beside the bsp, and
  without it tries to download it from the server and gives up.
- Face lines are Quake 3's, with or without Valve's UV axes (the stock maps use
  "Quake3 (Valve)"), each optionally followed by content flags, surface flags and a
  value. One map cannot mix the two kinds of line. Comments and the header are ignored.
- Materials are named by their path under `textures/`, such as
  `quake3/base_wall/basewall01`; each may have a `.mat` file and `_norm`, `_spec` and
  `_glow` images beside it. The data ships loose files, not pk3s. `quemap` gives the
  tool materials their flags by name: `common/sky` is sky, `common/caulk` and
  `common/trigger` are not drawn, `common/clip`, `hint`, `skip`, `origin`, `ladder` and
  `portal` do what they say. The textures are drawn at 0.25 or 0.5: most are 512
  pixels across.
- `func_group`, `misc_dust`, `misc_sprite` and `misc_weather` are merged into the world.
  TrenchBroom's layers and groups are read, including "omit from export".
- Worldspawn: `message`, `sky` (a name in `sky/`), `ambient`, `games`, `gameplay`,
  `movement`, `items`, `min_clients`, `ai`, `frag_limit`, `time_limit`,
  `capture_limit`, `music` and `gravity`. A server setting other than `default` wins
  over the map's: `sv_minClients` is 0 unless set, so a map's `min_clients` brings no
  bots by itself.
- `info_player_deathmatch` is a spawn, `info_player_team1` to `team4` a team's, and
  `info_player_intermission` with `angles` the camera between matches.
- `light` takes `radius`, `color`, `intensity` and `style`; the lighting is done in
  real time from lights `quemap` stores in the bsp, with light volumes it computes.
- `trigger_push` throws the player along its `angles` (pitch, yaw, roll; a negative
  pitch is up) at ten times its `speed`. `misc_teleporter` is a 64 by 64 pad that sends
  the player to the `misc_teleporter_dest` its `target` names; `trigger_teleporter` is
  the brush form.

### Setting Heron up

In the welcome window the profile is set to Quetoo, and in Preferences, Games, the game
path is `~/.local/opt/quetoo/quetoo/share/quetoo`. Heron mounts it and the profile's
search path, `default`, with any pk3 there, and lists 4151 materials from
`default/textures`. A scripted run of the window did both and opened the map. Heron's
own save is what `quemap` compiles; no export is needed.

### Rerunning it

    cd demos/quetoo_heron
    ./build.sh            # quemap for both maps, then installs them in the user folder
    ~/.local/opt/quetoo/quetoo/bin/quetoo +set sv_minClients 4 +map heron_yard

`build.sh --remake` builds the map again in Heron first, through `make_map.py` and the
MCP server, with `HERON_BRUSH` and `XDG_CONFIG_HOME` set as for Quake. The game ran in a
nested Xephyr display on Mesa's lavapipe Vulkan driver (Quetoo draws through SDL's GPU
API) at 4 to 10 frames a second. Pictures of the Xephyr window come out striped, so the
screenshots are the game's own `r_screenshot`.

`playtest.py DISPLAY` plays `heron_yard_playtest`, started with `+set sv_minClients 0
+exec playtest.cfg +map heron_yard_playtest`. That map has two spawns, one on the jump
pad and one on the teleporter; the script kills and respawns the player six times and
takes a screenshot each time with the diagnostics overlay showing the origin.

### What worked

- `quemap` built both maps with no warnings. The game loads the level as "Heron Yard",
  the console has no errors from it, and its bot navigation reports "Linked 1
  teleporters, 1 jump pads". With `sv_minClients 4` four bots run the hall, the ledge
  and the side room and frag each other.
- Respawned on the teleporter the player arrives at the destination on the ledge
  (origin `96 672 216`), every time. Spawned on the jump pad it landed on the ledge in
  four of seven tries (`288 682 216`, `288 746 217`); twice it stood on the pad
  unpushed and once it hit the ledge's front. An earlier pad, at pitch -70, did take a
  player who walked into it up to the ledge (`288 670 216`).
- Textures are where Heron draws them, at the profile's scale of 0.25.
- A round trip of the stock `pits.map` (TrenchBroom, 1599 brushes, Quake3 (Valve) with
  flags on every face) through Heron's save, and through its export as Quake 2 (Valve),
  compiles to the same brushes, planes, leafs and faces. The bsp files differ only in
  the entity string, by Heron's `"mapversion" "220"`.

### What was fixed in Heron

- **Normal and specular maps were listed as materials.** Heron did not read the
  profile's `materials.excludes`, so the material list held every `_norm`, `_spec`,
  `_glow` and `_fx` image beside its texture: 2510 of Quetoo's 6658 images. The globs
  are now read and matched against each material's name.
- **New brushes ignored the profile's UV scale.** `faceattribs.defaults.scale` was read
  and never used, so a brush drawn or added through MCP got a scale of 1 and Quetoo's
  textures came out four times larger than in its own maps. The shape tools, their
  regenerated previews and MCP's `add_shape` now give new faces the profile's scale;
  this also gives Quake 3 its 0.5.
- **The profile was out of date.** The 2026.1 definitions offered worldspawn keys the
  game no longer reads (`fraglimit`, `timelimit`, `teams`, `ctf`), so a map's limits were
  lost; they lacked `func_bob` and `misc_portal`; and `trigger_push`'s sound was a file
  that does not exist. The profile now matches TrenchBroom's master branch, where
  Quetoo's author keeps it, which also adds the "Quake3 (Valve)" format, a Ladder tag
  and the `portal` and `reflect` surface flags.

### Mapping mistakes, not Heron's

- The first jump pads (pitch -80 and -70 at speed 70) threw a standing player into the
  front of the 192-high ledge. Pitch -75 at speed 72 clears it most of the time from a
  standstill; at 4 to 10 frames a second the result varies from try to try.
- `quemap` hashes the bsp it finds first on its search path for the manifest, and the
  user folder is on it: with last build's bsp installed there, the new manifest named
  the old bsp and the client refused the map. `build.sh` removes the installed copy
  before compiling.
- A player who spawns standing in the jump pad's trigger is sometimes not pushed until
  they move. Only the playtest map has a spawn there.

### What remains

- Driving the player with the movement keys through XTest was unreliable in the nested
  display: with the mouse on, the pointer turned the view, and with it off the player
  sometimes walked the opposite way to the key while the view stayed put. The playtest
  respawns instead of walking.
- Heron writes `// Format: Valve` and `"mapversion" "220"` where TrenchBroom writes
  `// Format: Quake3 (Valve)`. `quemap` ignores the header and the game ignores the key,
  but whether TrenchBroom opens Heron's file without asking was not checked.
- Items, weapons and spawns are drawn as boxes: the definitions name no model, though
  the data has OBJ models for all of them under `models/`.
- The material browser was checked through MCP's `list_materials` (no `_norm` or `_spec`
  among 4151), not looked at in the window.
- MCP has no tool to set a face's flags or UV, so a map built through MCP relies on the
  tool materials' names, as this one does for its sky and triggers.
