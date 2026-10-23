# The map file

Heron Brush saves a `.map`: the Quake-lineage text format, in the Valve 220 dialect,
with a handful of the editor's own notes carried in comments. It is the format the whole
lineage reads, so a map is never trapped in this editor, and it is plain text, so two
people can share one in a repository.

`crates/map_model/tests/format_freeze.rs` holds the writer to this document, against
`crates/map_model/tests/fixtures/format.map` — a map carrying one of everything below. A change
to the writer that alters a byte of it fails there. Changing the format is allowed;
changing it without noticing is not.

## Why one dialect

The lineage has four flavours and they do not hold the same things. The Standard dialect
has nowhere on a face line to put the face's own UV axes, so a face that carries them
comes back deriving them from its dominant plane instead — the shear tool and the
free-axis half of the UV editor are shut off from the first minute in a map written that
way.

Valve 220 has the room, and both the reader and the writer decide **per face** rather than
per file: the reader by whether the line carries `[ ... ]` axes, the writer by asking the
face. So one brush holds faces of both kinds and each comes back as what it was. The
Quake 2 surface flag trio rides along wherever a face has one, ungated by the dialect. With
the axes and the flags both accounted for there is nothing left that another flavour can
say and this one cannot, which is what makes it safe to always save in it.

Hexen II's dialect is the Standard one with one more number at the end of each face
line, which Raven's `qbsp` reads as a light value and refuses to do without. The reader
takes the number and drops it; the writer puts 0 there, as TrenchBroom does.

So the flavour is not something to choose before there is a map. It is an output format
like Godot or raylib, and it lives on the export window's `.map` tab, where each one says
what it would drop from *this* map before it writes.

## What a save is

- **A fixed point.** The same document written twice is the same bytes. A map from
  another editor is normalised once, on its first save, and never moves again.
- **Local.** Adding a brush adds nine lines in one place. A face edit changes that face's
  line. Nothing else in the file moves, which is what makes a diff worth reading and a
  merge possible.
- **Named.** Every entity, brush and patch carries an id, so the same object is
  recognisably the same object in both people's copies.
- **Ordered by the tree.** Entities come out in the order the editor holds them, brushes
  in the order they sit in their entity. Nothing is sorted on the way out, so nothing is
  reshuffled by an edit somewhere else.

## The editor's own notes

Everything the lineage's format cannot say is written as a comment, so another tool reads
the map and ignores the note. Each begins with `heron_brush`, so an ordinary comment in a map is
not taken for one.

    // heron_brush format 3
    // heron_brush id <hex>
    // heron_brush color R G B [A]
    // heron_brush gradient R G B R G B ANGLE [A]
    // heron_brush linear ANGLE COUNT POS R G B ... [A]
    // heron_brush radial ANGLE COUNT POS R G B ... [A]
    // heron_brush primitive_uv A B C D E F

`format` is a header line, beside `// Format: Valve`, and says which version of the notes
below it the file carries. It is what lets a reader meeting a higher number say the file
holds something it does not understand, instead of quietly dropping it on the next save.
A file without one is from another editor, or from before the line existed.

`id` sits on its own line before the entity, brush or patch it names, and says what that
object is across saves and across editors. It is 64 bits whose high end is a random per-process
prefix, so a brush one person adds while offline and a brush the other adds while offline
are never the same brush by accident.

It exists for the merge. Without one, a three-way merge cannot tell an edited brush from
one removed and one added, and has to guess from how much of the geometry still lines up;
an entity is matched by its classname and its position among the others of that
classname, so a light added above another makes every light below it look like a
different light. With an id it knows. A file from another editor carries none, and one is
minted for each of its objects when it is opened.

The worldspawn does not carry one: it is matched as the single entity with that
classname. Nor does a layer or a group, which already carries the `_tb_id` its members
point back at.

`color` is a colour painted on a face, drawn instead of its material, and `gradient` two
colours and the angle between them. `A` is 0 to 255 and absent means opaque.

Version 2 adds `linear` and `radial`, each with 2 to 8 stops. Each stop stores a
position from 0 to 1 and RGB channels from 0 to 255. Stops are sorted by position.
Linear paint follows the face UV direction at `ANGLE`. Radial paint uses an ellipse
centred in the UV bounds; position 0 is its centre and position 1 reaches the bounds
at the horizontal and vertical midpoints. Its angle is retained but unused.
The older two-colour `gradient` syntax remains supported.

Either goes on the face's own line, at the end of it — or, when every face of the brush
carries the same one, on a line of its own before the brush and on no face at all.
Painting in the editor paints what is selected, so a brush is normally uniform: on the
dragon in `assets/`, all ninety-nine of its brushes are and none is mixed, which is why
that is worth the second place to look. Recolouring a brush is then one changed line
rather than one per face. A face that carries its own colour keeps it whatever the brush
above it says.

`// entity N` and `// brush N` are the lineage's own numbering comments, written for the
sake of a compiler's error messages, and read by nothing here.

## What the lineage's own keys carry

Layers and groups are entities, as they are upstream: a `func_group` with
`_tb_type` of `_tb_layer` or `_tb_group`, an `_tb_id`, an `_tb_name`, and the members
pointing back with `_tb_layer` or `_tb_group`. `_tb_layer_omit_from_export` marks a layer
left out of an export, `_tb_layer_locked` and `_tb_layer_hidden` its two switches, and
`_tb_protected_properties` names the keys of an entity in a linked group that a sync from
its siblings leaves alone.

## Two people, one map

Two ways, and they are for different things.

**In a repository.** The file is text and a save is local, so git handles it. Set the
merge driver up so a merge is done by the editor rather than by lines
(`heron-brush --merge-map`): it merges by brush and by entity, keeps
both sides' additions, and reports the ones it could not decide rather than leaving
conflict markers in a file no editor can then open.

**At the same moment.** One editor hosts a session and the other joins; each edit goes
over the wire as it happens, and an edit made while disconnected replays on rejoin. Each
participant's selection is drawn in their own colour. `crates/editor_core/src/collab/`.


## Deferred brush-primitive UVs

`brushDef` and `brushDef3` matrices describe normalized texture coordinates.
Their conversion to the editor's pixel-based parallel axes needs the material's
width and height. If a material is absent, the original 2x3 matrix remains pending.
Saving a face now retains it as `primitive_uv A B C D E F` in the same face comment
as color, PBR and export visibility. Version 3 of the notes identifies this payload;
older Heron readers warn about the newer notes version. Ordinary foreign comments,
truncated matrices and nonfinite metadata do not override face mappings.

The pending matrix survives save/reopen until material discovery converts it to
parallel UV axes. Paint edits retain it. Explicit UV edits replace the placeholder
alignment and clear the old matrix so a later texture load cannot undo the edit;
undo restores the original pending matrix. Reset UV also clears it even when the
placeholder already had default attributes.

These comments preserve data for Heron; other editors ignore them. Native Quake III `brushDef` export is now available separately with actual texture
sizes; see [native export](brush-primitives.md). Doom 3 Version 2 `brushDef3` output is also available with origin handling; non-identity brush/patch entity transforms and cross-editor verification remain open. A missing material still uses placeholder alignment
until its dimensions become available. Resolved mappings use the existing Valve
parallel-axis representation. Standard/Quake 2 export cannot encode arbitrary
parallel axes and remains potentially lossy.

Local checks on 16 October 2026 cover all four dialects, solid/multistop/no paint,
PBR and hidden-face combinations, stable repeated saves, malformed/foreign
comments and later non-square texture resolution. The document regression also
checks paint retention, explicit UV edits, reset and undo before loading textures.
The format-freeze fixture deliberately advances to notes version 3 and now includes
a pending matrix. The full map_model/scene_export/editor_core test run passed
1,475 tests with 16 manual benchmarks ignored. The editor and export-worker wasm32
compile check passed. Logs: `/tmp/heron-primitive-save-full.log`,
`/tmp/heron-primitive-save-document.log`, `/tmp/heron-primitive-save-web.log`.
Native cross-editor and GUI mapping verification remain open.
The default desktop compile check also passed in
`/tmp/heron-primitive-save-desktop.log`. The installed application and the earlier
Linux archive were not rebuilt for this change.
