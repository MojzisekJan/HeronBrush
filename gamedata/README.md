# Bundled game profiles

Heron Brush is the general profile for projects exported to Godot, Bevy, Three.js
and the other engine targets. Its identifier and directory are `heron_brush`. A fresh profile selects it.
An explicitly saved `None` selection remains available. TrenchBroom's `Generic`
configuration is retained below as upstream reference data, but is not offered
as a second general profile. Existing `Generic` preferences use `heron_brush`.

The Profile selector chooses entity definitions, material locations, tags and flag
names. It does not select an export engine or prescribe a game's runtime logic.
New maps use the editor's lossless working MAP dialect. Existing maps retain their
read dialect; export chooses the output format. Engine export settings and project
class bindings are separate from these profiles. Custom FGD definitions can replace
the built-in entity vocabulary.

All 15 profiles shipped by the locally installed TrenchBroom 2026.1 are included,
with the same game names:

- D-Day Normandy
- Daikatana
- Digital Paint: Paintball 2
- Generic (reference configuration; the selector uses Heron Brush)
- Half-Life
- Heretic 2
- Hexen 2
- Kingpin
- Neverball
- Quake
- Quake 2
- Quake 3
- Quetoo
- SoF
- Wrath

The welcome and preferences selectors show each profile's configured PNG icon,
including the selected profile. Heron Brush uses the application logo. A missing
or unreadable icon leaves the profile usable as a text entry. Custom profiles
resolve the `icon` field relative to their `GameConfig.cfg` directory.

Together with Heron Brush this makes 16 bundled profiles. Each profile includes
all its supplied entity definition files, including Daikatana's shared definitions
and four episodes, and Quake's Quoth, Rubicon and Teamfortress variants. The first
listed definition is the profile's default. Other definitions can be loaded through
the editor's entity-definition file chooser.

These are editing presets, not a claim of complete native game compatibility.
Supply your own game assets and compiler/build profile. WAD loading and supported
texture directories are available. Shared [game asset lookup](../docs/game-assets.md)
adds ordered mods, PAK/ZIP/PK3, supported raster previews and shader editor images.
Palette-indexed game texture formats and other TrenchBroom features still have
limits described there. Heron Brush
uses the map dialects its reader/writer supports. A profile with no supported
dialect falls back to the working Valve dialect. Check the export dialect and
compiler required by the game before building a native level. TrenchBroom's runtime
scale expressions are stored as strings and are not evaluated by this config reader.

## Source and license

The game profile directories, excluding `heron_brush`, come from the locally
installed TrenchBroom 2026.1 Flatpak, application
`com.kristianduske.TrenchBroom`, deployment
`c8f638abf9fe90bf98ed9ddfdc4ce3881173aa3c1e23e39c846d21acbb20c892`.
TrenchBroom's GPLv3 license is preserved in `TrenchBroom-LICENSE.txt`; author
comments in definition files are preserved. The package includes TrenchBroom's
profile icons, palettes and editor helper assets. It does not bundle installed
commercial games or download their assets.

Configuration adaptations quote Generic's unquoted keys, escape literal glob
backslashes for JSON, quote runtime scale identifiers, and omit Generic's missing
initial-map references. Definition-file line endings, indentation and trailing
whitespace are normalized. Neverball's starting map is kept with its profile, and a
new Neverball map starts from it: the profile's "Quake3" format reads as Quake 2 face
lines, which is what Neverball's `mapc` compiles.

The Half-Life profile is no longer marked experimental, after the Half-Life: Uplink run
in `docs/game-profile-tests.md`; only Xash3D FWGS and SDHLT were tried.

The Quake 3 profile is no longer marked experimental either, after the OpenArena run
in `docs/game-profile-tests.md`.

Neverball's profile is no longer marked experimental, and it has a section of Heron's
own, `"heron": { "mergeGroupsOnExport": true }`. Neverball 1.6's `mapc` puts the
brushes of a `func_group` in no body, so they are not in the level; with the key set,
File, Export, Map and the MCP tool `export_map` write groups and extra layers into the
worldspawn. Saving keeps them. A profile without the key exports groups and layers as
`func_group` entities, as TrenchBroom does. TrenchBroom's copy of the profile has no such
section.

A profile's own `assets` folder is mounted under the game's files, as TrenchBroom
does, so the editor models its definitions name are found. Neverball's models there
are converted from TrenchBroom's copies, which are Y-up and in metres, to Z-up map
units (64 to the metre), since Heron reads an OBJ as it is and the profile sets no
model scale.

Hexen 2's definitions were corrected against Raven's game code: the Egyptian torch is
`light_torch_eqypt`, the spelling the game spawns, and the `message` keys of worldspawn,
buttons, doors and triggers are integers, because Hexen II reads them as line numbers
in `strings.txt`.

Quetoo's configuration and definitions come from TrenchBroom's master branch as of
2026-10-17 instead of the 2026.1 release, since Quetoo changes quickly and its author
keeps them there. The 2026.1 definitions named worldspawn keys the game no longer reads
(`fraglimit`, `timelimit`, `teams`, `ctf`), so a map made with them got none of its
limits, and lacked `func_bob` and `misc_portal`. The newer configuration lists the
"Quake3 (Valve)" format the game's own maps are saved in, a Ladder tag, and the
`portal` and `reflect` surface flags.

Heretic 2's configuration names surface flag bits 24 and 25, `metal` and `stone`, which
TrenchBroom's leaves out. They are the footstep material Raven's `qfiles.h` calls
`SURF_TYPE_*` (both set is wood), and nearly every `.m8` sets them. A face that writes
flags of its own replaces the texture's; the face panel starts those from the texture's,
so the bits were kept, but they had no tick to see or change them by. They also matter
beyond footsteps: Yamagi-based engines tell a Heretic II bsp from a Quake 2 one by
finding both bits among its faces. Its definitions' `item_spitter` model chose by `target =>`,
which the model-expression parser rejects, so every load warned and the spawner had no
model; it is `target ==` now.
