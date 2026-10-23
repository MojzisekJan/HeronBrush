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
| DigitalPaintball2 | Digital Paint: Paintball 2 | planned |
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
