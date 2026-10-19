# Game assets and active mods

Models, their dependent textures, material previews, shader editor images and
palette fallbacks now use one indexed game filesystem. Configure **Game**, **Where
it is** and **Mods** in Preferences. Separate mods with semicolons, for example
`quoth;my_mod`. Relative mod paths are relative to the game installation; absolute
paths are accepted too. The final mod has the highest priority. The displayed
numbered **Project paths** list shows the highest priority first.

The mounted roots, from lowest to highest priority, are the profile's
`filesystem.searchpath` subdirectory, the installation directory, then the listed
mods. Selecting the base directory itself does not add it twice. Duplicate roots
keep their last position. Within each root, archives load in natural filename
order (`pak10` after `pak2`), followed by loose files. Thus a mod archive can
replace a base loose file, while loose files replace archives in the same mod.
Resolution is case insensitive and normalizes separators and internal `..`.
Archive dependencies cannot escape their mounted namespace. Explicit absolute
OBJ/GLB imports still read their explicitly chosen files and dependencies.

Supported containers are id PACK/PAK and ZIP/PK3 with stored or Deflate members.
Files stay inside their archives: nothing is extracted into the game directory.
Native PAK, ZIP and PK3 resources use file-backed directories and read only
requested members. The resolver limits a requested PAK member to 256 MiB and a
native PAK directory to 64 MiB. ZIP member lengths, bounds and CRC are checked;
packed and unpacked member sizes are limited to 256 MiB, and central directories
to 64 MiB. ZIP64, multidisk, encrypted packages, other compression methods and
DKPAK are unsupported. All browser archive backends still retain archive bytes
in memory. These are per-member and per-directory limits, not a total game-memory
limit.

The MDL, MD2, MD3, OBJ, GLB and SPR loaders use this filesystem, including OBJ/MTL
relative dependencies and MDL palettes. PNG, JPEG, TGA, indexed PCX and Quake II WAL previews, normal-map
companions and supported `scripts/*.shader` editor-image directives are indexed.
MD2 also decodes PCX skins; [scope and checks](md2-models.md). This does not add
DKM or MIP/M8/M32 decoding, arbitrary game
shaders, or MDL/MD2 frame/skin selection. Explicit WAD collections remain available; loose WAD
search and palette fallback use the active roots, but WAD files inside archives
are not automatically opened as collections.

Default asset-library folders have lower material priority than game assets.
Explicit project folders and WAD sources have higher priority. The index is reused
until game/path/mod preferences change or **Face → Materials → Reload** is pressed. Use Reload
after changing game files on disk; there is no filesystem watcher. Raster decoding
is lazy, with the existing 64 MiB recent-pixel cache. Mod preferences are local
editor settings, not a portable per-map mod manifest.

Exports package the winning game texture pixels rather than referencing synthetic
archive paths. Regression checks cover GLB, Godot, Three.js and OBJ/MTL. This does
not add OBJ MTL PBR extensions or prove native compatibility with every game.

## Verification, 15 October 2026

- Offline model/core/renderer/export suite: 1,448 passed, 14 ignored.
- Desktop UI suite: 113 passed, 5 ignored; default desktop build and WASM check passed.
- Integration fixtures cover root/mod/archive/loose precedence, model dependencies,
  shader flags, normal maps, MDL palettes, index reuse, Reload and portable exports.
- ZIP corruption, traversal, truncation, stored/Deflate entries, natural archive
  order and symlink cycles are tested. Fixtures are generated locally by
  `examples/editor-checks/make_game_asset_fixtures.py`.
- Manual debug benchmark: 20,000 indexed entries mounted in 107 ms; 200,000
  resolutions in 776 ms; 20,000 Deflate reads in 819 ms. Run with
  `cargo test -p map_model --lib asset_fs::tests::profile_indexed_game_asset_lookup --offline -- --ignored --nocapture`.
  These are synthetic small files, not a commercial-game memory benchmark.

Native Linux/wgpu checks on an isolated Xephyr display rendered the same map green
from PK3 and blue from a higher-priority loose mod after reversing mod order.
Screenshots and logs: `/tmp/heron-native-game-assets/`. No game assets were
downloaded and no installed engine was needed.

## Indexed PCX materials

Material folders and mounted game resources now discover version-5, single-plane,
8-bit RLE PCX images with an embedded trailing 256-colour palette. Names follow
the same texture-root rules as PNG, including nested paths and case-insensitive
extensions. PNG/JPEG/TGA retain precedence over PCX when a mounted collection
contains multiple image formats with the same stem. Loose-file/mod/archive
precedence is still resolved first for each full resource name.

The browser reads only the 128-byte header of a loose native PCX when discovering
its dimensions. Archive member reads and browser virtual files use the existing
resource path; pixels remain undecoded until requested. The shared decoder caps
dimensions at 8,192 per axis and 16,777,216 texels, handles scanline padding and
uses palette index 255 as transparent. Invalid headers become collection
warnings. Invalid RLE or missing palettes make pixel decoding fail on demand.
Other PCX versions, true-colour planes and non-RLE files are unsupported.

Six PCX decoder/material tests pass, covering lazy loose and PAK materials,
palette/alpha pixels, loose overrides, invalid input and PNG precedence. Evidence:
`/tmp/heron-pcx-material-tests.log`. This does not establish that all Quake II assets and maps are supported.

The complete native `map_model` regression suite and its offline wasm32 check
also pass. Evidence: `/tmp/heron-pcx-map-model-regression.log` and
`/tmp/heron-pcx-wasm-check.log`. The fixtures exercise the material/resource
path; a full editor GUI session with a PCX-only game installation is not yet
verified. No engine or dependency installation was needed.

## Quake II WAL materials

WAL images now participate in folder and mounted-game material discovery, after
PNG/JPEG/TGA/PCX in the existing extension preference order. The highest-resolution
indexed mip is expanded only when pixels are requested. The 100-byte loose-file
header is enough for discovery; decoding validates all four mip ranges before
allocating RGBA. Supported dimensions are 8–8,192 per axis with at most
16,777,216 texels. Header-overlapping, overlapping or truncated mip ranges fail.
This is Quake II WAL, not other games' similarly named formats.

The active profile's `materials.palette` now selects an indexed PCX or raw
768-byte RGB palette resource.
Mounted game/mod palettes take precedence over the bundled profile copy. Quake II
ships a fallback `colormap.pcx` beside its profile; that fallback is used only
when the mounted resource is absent. A malformed winning palette is reported,
rather than silently replaced by different colors. Folder collections look for
`pics/colormap.pcx` or `colormap.pcx` in their root and its parent. Missing palettes
skip WAL materials with a diagnostic; unrelated image materials still load.
Palettes are shared by the collection's lazy WAL images.

Palette index 255 becomes transparent. WAL animation chains, stored surface flags,
contents and values are not converted to animation or applied automatically to
map faces. Existing map face flags remain the authoring source. Normal-map
companions use the other supported raster formats. File-stamp-only pixel-info
cache entries are not applied to WAL, because a palette can change while the WAL
file remains identical. Reload game assets after changing either on disk.

The decoder follows id Software's [WAL header](https://github.com/id-Software/Quake-2/blob/master/qcommon/qfiles.h)
and [indexed texture upload](https://github.com/id-Software/Quake-2/blob/master/ref_gl/gl_image.c).
No game data or engine download was needed. Synthetic tests cover exact palette
colors/alpha, all truncations, invalid dimensions/offsets, lazy loose and PAK
resources, folder palette lookup, mod override and profile fallback, and missing
or invalid palettes. Evidence: `/tmp/heron-wal-tests.log`.

The document-level Quake II fixture passes with the actual bundled profile,
its fallback palette and a winning mod palette. It verifies decoded mod colors
inside the portable GLB and exported Godot/Three.js PNGs. Evidence:
`/tmp/heron-wal-document-export.log`. This is an authoring/export integration
check, not a native Quake II engine run or a visual GUI verification.

Final offline validation passes 1,453 native tests across `map_model`,
`scene_export` and `editor_core`, with fourteen intentionally ignored and no
failures. The first sandboxed broad run exposed a raw-LMP palette compatibility
issue, now fixed, and localhost permissions needed by collaboration tests; the
final host run passed those tests too. Web `editor_core` and desktop application
including test-code compilation also pass. Evidence:
`/tmp/heron-wal-native-regressions-final.log`, `/tmp/heron-wal-wasm-check.log` and
`/tmp/heron-wal-desktop-check.log`. Full native Quake II gameplay and a visual
editor session with WAL materials remain unverified.

## Native PAK residency

The shared resolver keeps an open read-only PAK handle and parsed entry metadata,
with no retained archive-body buffer. Reads are bounded and seek/read operations
are serialized for concurrent model/texture requests. The memory and file-backed
readers share directory parsing rules for names, signed numbers and ranges.
Loose/mod/package precedence and resource pseudo-paths remain the same.

Length and available modification time are checked before and after member
reads. In-place changes reject further reads through that index; use Reload.
The Linux replacement regression also verifies that atomic path replacement
leaves the old mounted handle readable until reload. Metadata checks cannot
detect rewrites that preserve both length and modification time. Existing
decoded texture/model caches still follow the editor's explicit reload behavior;
this is not an automatic file watcher.

A Linux ignored benchmark generates sparse PAKs whose unused payload is named
as 64 MiB-or-smaller entries, plus one five-byte requested resource. At 256 MiB,
the original memory mode increased process RSS by 263,932 KiB, versus 1,688 KiB
for the file-backed mode. At 512 MiB, the file-backed increase was 1,788 KiB.
This isolates archive residency; it does not measure a complete game load, all
image decoding, total editor RAM or cold physical-disk throughput. See
[performance measurements](performance.md) for commands and raw evidence.

Tests cover memory/file directory parity on truncations and bad ranges,
concurrent reads of distinct members, in-place rewrite/truncation and reload,
Linux atomic replacement, and oversized sparse member/directory rejection. The
broader native model/export/editor regression passes 1,458 tests, fifteen ignored;
the browser frontend also compiles offline. Evidence:
`/tmp/heron-file-pak-native-final.log` and `/tmp/heron-file-pak-web-check.log`.
Native Windows/macOS/Android filesystem behavior and representative many-resource
load timings remain unverified. ZIP/PK3 now have equivalent file-backed storage,
described below.

The Linux residency fixture is now an explicit CI guard. A negative control with
the original native memory route failed the RSS bound, and the restored file
route passed. The command is verified locally; the remote workflow has not run.
Native production type-checking also passes, with evidence in
`/tmp/heron-file-pak-native-check.log`.

## Native ZIP and PK3 residency

The native resolver now reads only the bounded ZIP tail, central directory and
requested local headers/payload ranges. Memory and file-backed storage share
the parser and decoder. The browser keeps its existing memory backend; its
range views borrow archive bytes instead of copying slices.

Stored and Deflate data still require matching local names/methods/flags, bounded
member ranges, exact decoded length and CRC. Deflate must consume exactly its
declared packed range. Unsupported methods fail before payload reads. Native
reads serialize seek/read operations, check length/available mtime around reads,
and retain their mounted handle across atomic replacement, following the same
reload contract and metadata limitations as PAK.

Five new native tests cover memory/file parity on truncation and byte corruption,
the maximum ZIP comment spanning the bounded tail, stored/Deflate CRC failures,
encryption and size/method rejection, concurrent member reads, in-place changes,
reload, Linux replacement and central-directory allocation limits. The full
model/export/editor suite passes 1,463 tests with sixteen ignored, and the web
frontend type-checks offline. Evidence: `/tmp/heron-file-zip-native-verified.log`,
`/tmp/heron-file-zip-unit-final.log` and `/tmp/heron-file-zip-web-verified.log`.

A sparse ZIP/PK3 fixture contains named, valid-CRC unused stored members and a
small Deflate probe which is actually read and verified. At 256 MiB, process RSS
growth falls from 262,752 KiB in memory mode to 744 KiB in native file-backed mode;
a 512 MiB fixture grows by 864 KiB. This isolates archive residency, not complete
game/editor RAM, many-resource latency, decompression budgets or physical-disk
throughput. The CI guard passes locally and fails when the original memory
route is temporarily restored. Its remote execution remains unverified.

Residency fixtures now create directories exclusively with a nonce: PID alone
is not unique across simultaneous sandbox processes. The earlier failed parallel
256 MiB run was a fixture-directory collision; the corrected final runs both
pass. Browser archive copies, installed non-Linux behavior and representative
large-library load timings remain work to do.

MD3 static entity previews retain all surfaces, frame-zero geometry and packed
vertex normals, with external raster skins. Scripted shader stages, tag assembly
and animation remain unsupported; see [MD3 scope and checks](md3-models.md).
