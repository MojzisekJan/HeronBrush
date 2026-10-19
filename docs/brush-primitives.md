# Native Quake III brush-primitive export

The `.map` export dialog offers **Quake 3 (brush primitives)** and **Doom 3
(brushDef3)** alongside Valve, Standard and the two Quake 2 dialects. It writes native `brushDef` blocks with
three-point planes, normalized 2x3 texture matrices, material names and the
content/surface/value trio. All exported brushes use the same native syntax.
Patches and point entities remain in the map. Omitted layers contribute no geometry.
The editable document continues to use its lossless Valve representation.

`Document::export_quake3_map_text` uses the loaded width and height of each
material to convert pixel UVs to normalized coordinates. Paraxial rotation,
parallel axes, shear, offsets, negative scales and non-square textures are
supported. Removing an axis's component along the face normal contributes a
constant on the plane; that constant is folded into the matrix offset. This
preserves mappings on oblique planes as well as axis-aligned ones.

An unresolved matrix imported from `brushDef`/`brushDef3` is already normalized
and can be written without loading its texture. Other faces require actual
texture dimensions. Missing dimensions produce an error naming the material;
the writer does not assume a default image size. The UI lists missing dimensions
before export. Text generation completes before writing the selected file, so
this validation failure does not overwrite an existing map. This is not a claim
of atomic disk writes under filesystem failure.

Heron IDs, paint, PBR and hidden-face flags remain in comments, including per-face
metadata after native face lines. Quake III tools ignore those comments. The UI
warns when faces carry these Heron-only annotations; game tools do not thereby
acquire paint/PBR/render-visibility behavior. Native matrices do not depend on a
`primitive_uv` comment. The Valve `mapversion` marker is removed from worldspawn.

The low-level API is `map_model::io::map_writer::write_quake3_map`, with a caller
provided material-size lookup. It returns an error for missing dimensions,
invalid sizes or nonfinite output matrices. The existing generic `write_map`
API and four classic dialects retain their behavior.

Syntax reference is id Software's
[Q3Map parser](https://github.com/id-Software/Quake-III-Arena/blob/master/q3map/map.c).
The implementation uses the editor's existing `primitive_axis_base` function.

Local tests cover normalized-vs-texel UV evaluation on axial/oblique planes,
paraxial rotation, arbitrary parallel axes, negative scales and non-square
textures; whole-map geometry, IDs, patches, flags and pending-matrix retention;
and unavailable/invalid dimensions. The document test covers layer omission,
loaded sizes, unmodified editable state, and preservation of an existing output
file when validation fails. Native game-tool validation remains unperformed.

Doom 3 now has Version 2 reading/writing and a separate export option; its exact
scope is described below. Compiler/game rendering, real project assets and
cross-editor UV comparisons remain separate checks. These additions do not mark
all game profiles fully compatible.

Local verification on 16 October 2026 passed the complete
map_model/scene_export/editor_core regression: 1,479 tests, 16 manual benchmarks
ignored. After the final metadata checks, all three focused native-writer tests
passed again, including PBR/visibility retention and parsing after all Heron
comments are removed. Desktop and web editor/export-worker compile checks pass.
Logs: `/tmp/heron-quake3-native-regression.log`,
`/tmp/heron-quake3-writer-tests-final.log`, `/tmp/heron-quake3-document-test.log`,
`/tmp/heron-quake3-desktop-check-final.log`, `/tmp/heron-quake3-web-check.log`.
Remote CI and the live export dialog have not been exercised for this change.
The installed application and the ec4446c Linux archive remain unchanged.


## Doom 3 Version 2

The native Doom 3 option writes `Version 2` and `brushDef3` blocks with plane
normal/distance equations. Heron stores brush geometry in world coordinates;
Doom 3 brush-entity planes are local to the entity origin. Export subtracts that
origin, and the reader restores world-space planes. Worldspawn ignores its origin.
The normalized texture matrix remains in the world basis, matching the origin
contribution used by Doom 3's texture-vector evaluation. Material names are
preserved as authored, without adding a `textures/` prefix.

The reader accepts version-2 headers, `brushDef2`/`brushDef3` plane blocks and
literal idTech4 strings. Backslashes, including trailing backslashes, are not
escapes in that syntax. Internal Valve saves encode them with the existing escape
rules and restore them on reopening. Unsupported versions, malformed origins,
nonfinite planes and late origin/classname properties after local brushes are
rejected. Normalizing a nonunit plane now also scales its distance correctly.

The writer shares Quake III dimension validation and pending-matrix handling.
Patches remain in world coordinates as Doom 3 expects in the file, with quoted
literal material names. The engine ignores the numeric surface/content/value
trio and Heron paint/PBR/visibility comments; its materials determine those effects.
Quote/control characters unrepresentable by idTech4's literal strings are rejected.

Non-identity brush/patch entity rotations are not implemented in the native writer.
It rejects them before touching the destination file. Bake these transforms into
geometry and reset rotation/angle properties first. The reader preserves those
properties and warns that rotated brush previews are not applied. Point-entity
properties are retained; this is not a promise of Doom 3 model or gameplay support.
Source-map arbitrary block comments and older version-1 material semantics remain
outside this reader's verified scope. A native Doom 3 compiler/editor/render has
not been installed or run.

Reference: id Software's
[MapFile.cpp](https://github.com/id-Software/DOOM-3/blob/master/neo/idlib/MapFile.cpp),
including Version parsing, brush planes, literal lexer strings and texture vectors.

Doom 3 local verification passed the complete portable/core/export regression
with 1,484 tests and 16 manual benchmarks ignored. A subsequently added focused
patch test also passes, checking world-space control points and a literal material
name with a trailing backslash in a translated entity. Tests cover local-plane
origin recovery, scaled plane coefficients, normalized UV evaluation, literal
property strings through internal saves, invalid headers/origins/planes and late
origin declarations. Native export tests check preserved destination files on
missing dimensions or rejected rotation, and unchanged editable state.
Logs: `/tmp/heron-doom3-native-regression.log`,
`/tmp/heron-doom3-writer-tests.log`, `/tmp/heron-doom3-document-test.log`,
`/tmp/heron-doom3-patch-test.log`, `/tmp/heron-doom3-web-check.log`.
The web editor/export-worker compile check passed. No tools or engines were
installed. The user's installation and the earlier Linux candidate are unchanged.
The default desktop compile check also passed in
`/tmp/heron-doom3-desktop-check.log`. A final map-I/O check additionally verifies
case-insensitive `ClassName`/worldspawn origin handling during native export;
see `/tmp/heron-doom3-writer-tests-final.log`.
