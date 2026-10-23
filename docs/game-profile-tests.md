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
| DigitalPaintball2 | Digital Paint: Paintball 2 | planned |
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
