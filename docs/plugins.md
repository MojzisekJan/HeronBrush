# A plugin system

The goal: let someone add a batch operation, a shape generator or a small tool to the
editor without recompiling it, and without the editor trusting the plugin with the whole
machine.

The headless runner (`heron-brush --run-script`) and the **Plugins…** window are built;
the API keeps growing. What is left is noted through the doc.

## The language: Rhai

`rhai` is the pick. It is pure Rust with no `unsafe`, sandboxed by default (a script
reaches nothing but the functions we register -- no file system, no network, no process),
and it carries operation and call-depth limits so a runaway script stops itself. The
syntax is close enough to Rust that the examples read without a tutorial. It is the
language `bevy_mod_scripting` and a good part of the Rust game-tool world already use for
this.

The alternatives and why not:

- **Lua** (`mlua`): the language most editors use for plugins, and the most familiar to
  the people who write them. But it pulls in a C library, which is exactly the trust and
  build-portability cost the rest of this project avoids, and it would be the only C in
  the tree.
- **Steel** (Scheme), **Rune**, **Koto**: all pure Rust, all smaller communities. Steel's
  Lisp syntax asks more of a casual plugin author than Rhai's does. Rune is async-first,
  which buys nothing here. Koto is young.

Rhai adds about 2 MB to the binary. That is the price of the feature.

## What a plugin is

A plugin is a directory under the plugin folder, or a single `.rhai` file:

```
~/.config/heron-brush/plugins/
    desaturate/
        plugin.toml      # what it calls itself, and the API it was written against
        main.rhai        # the script that runs
    quick-stairs.rhai    # a plugin with nothing to declare is just the file
```

`plugin.toml` is read by `plugin_manifest.rs`. Every key is optional:

```toml
name = "Desaturate colours"
description = "Turn the painted colours on the selection to greyscale."
version = "1.0"
author = "A Mapper"
api = 1
```

`name` and `description` are what the Plugins window shows, instead of the folder name
and the script's first `//` line. `version` and `author` are shown and never
interpreted. The `api` field controls compatibility: the editor offers one
plugin API version at a time (`plugin_manifest::API_VERSION`, 1 today), and a plugin
asking for a different one is listed, greyed and refused rather than run against
functions it does not expect. A plugin that names no `api` is taken as written for the
current one, which is what every plugin that shipped before the manifest did.

### What a plugin asks for

A plugin with something to tune declares it, one `[[setting]]` block each, and the
Plugins window draws a control per setting under the list:

```toml
[[setting]]
key = "copies"          # what the script asks for: setting("copies")
label = "Copies"        # what the person reads beside the control
kind = "number"         # number | text | check | choice
default = 5
min = 1                 # numbers only
max = 200

[[setting]]
key = "direction"
kind = "choice"
choices = "up, down"    # comma separated, in order
default = "up"
```

`number` is a spin button and the script sees a float; `text` an entry and a string;
`check` a switch and a boolean; `choice` a drop-down and the chosen string. The script
reads an answer back by key:

```rhai
let copies = setting("copies") ?? 5.0;
```

A key nobody answered comes back as `()`, so `??` gives a script its own fallback and it
still runs where no manifest declares the setting. A setting with no `key` is dropped,
since nothing could ask for it; an unknown `kind` is a text box; a missing `label` is the
key itself; a `choice` with no `default` starts on its first option. Nothing is
remembered between runs -- open the window and every control is at the manifest's
default.

Headless, the defaults are what the script is answered with, and a `key=value` argument
moves one:

```
heron-brush --run-script assets/plugins/stack/main.rhai level.map out.map copies=12 direction=down
```

A key the manifest never declared, or a word where a number goes, stops the run with a
message rather than being quietly ignored.

The format read is a `key = "value"` subset of TOML: strings, numbers and booleans, with
`[[setting]]` and `[[action]]` arrays. That is all a manifest needs and it saves a dependency. A
`[section]` header the reader does not know is skipped along with the keys under it, so a
manifest that grows one later still reads its top keys. A manifest that cannot be parsed
is treated as absent: a broken manifest must not hide a working script.

Installing one is **Install a plugin…** in the Plugins window: pick a `.rhai` file or a
plugin folder and it is copied into the plugin folder, with the list refilled where you
can see it. That is the whole distribution story -- a plugin is a folder, so a `.zip`
and this button are all it takes to pass one around. No registry.

## The API surface

The plugin API is the same set of operations the MCP server exposes (`mcp.rs`), because
that list is already "everything an outside caller needs to drive the document" and it is
already the tested boundary. The Rhai engine registers each as a function.

What `plugin.rs` has:

```
selection()              // an array of brush handles
brushes()                // every brush handle
face_count(brush)        // how many drawn faces the brush has
brush_center(brush)      // [x, y, z]
brush_bounds(brush)      // [[minx, miny, minz], [maxx, maxy, maxz]]
face_color(brush, face)  // (), [r,g,b,a], or ["grad", [r,g,b,a], [r,g,b,a], angle]
set_face_color(brush, face, value)   // [r,g,b] / [r,g,b,a] / ["grad", ..] / ()
translate(brush, [dx, dy, dz])
add_cuboid([x,y,z], [x,y,z], material)   // material () = the current one; returns a handle
delete(brush)
select(handle_or_array)  // make those brushes the selection
csg_subtract(cutters)    // take those brushes away from every other; returns the count cut
csg_merge(brushes)       // fuse them into one convex brush; returns the new handle or ()
say("...")               // a line in the console
setting("key")           // what the person answered for a [[setting]]; () if nobody did
```

Still to add: `face_material`, `map_bounds`, a `progress` callback.

A handle is a small integer, its position in a per-run table (`Handles` in `plugin.rs`):
the first brush a script asks about is `0`, the next new one `1`. A `NodeId` carries a
random high-bit prefix so it neither fits an `i64` nor means anything to read; the handle
does both. A handle for a brush that has since been deleted comes back as a clear error.

A colour is an array, not a string: `[r, g, b]` or `[r, g, b, a]` for a solid colour,
`["grad", from, to, angle]` for a gradient (each of `from`/`to` an `[r,g,b,a]`), and `()`
for none. Rhai has no real string or hex handling, and a script doing arithmetic on
channels wants numbers anyway. A plugin never sees `FaceColor`'s Rust shape.

Multiple stops use `["linear", [[position, rgba], ...], angle]` or
`["radial", [[position, rgba], ...], angle]`, with 2 to 8 positions in 0..1. A fourth
entry `[su, sv, eu, ev]` places the gradient on the face: its start and end as fractions
of the face's bounds (a radial gradient is centred on the start).
The first stop supplies the shared opacity. Both `face_color` and `set_face_color`
use this representation.

Only faces that are actually drawn are reachable: `face_count` and `face_color` walk the
brush's geometry faces, so a redundant painted face that the hull cut away is left as it
is. That is the right thing for "recolour what I see".

In the window, a plugin's whole run is one undo press: `Document::run_as_one_edit` marks
the undo stack, runs the plugin, and `History::coalesce_since` folds every step it
recorded into one. The headless runner has no undo.

## Execution and limits

Scripts run synchronously on the editor thread. Discovery does not execute scripts.
An opted-in visible inspector panel refreshes on map/selection changes using an
isolated snapshot; geometry actions require a user click. Rhai exposes only registered APIs;
file/network imports are disabled. The runner limits operations and call depth.
It does not yet offer cancellation or background execution.

A run is grouped into one Undo step, including completed edits before an error.
An error does not automatically roll back geometry. Panel output is published only
on success, so a failed run shows its error rather than a partial report.

## Running a plugin -- done

**The window** (`plugin_dialog.rs`, "Plugins…" in the menu) lists the bundled scripts and
any `.rhai` file or `<name>/main.rhai` folder under `~/.config/heron-brush/plugins/`,
each with its first `//` line as a summary. Run goes over the selection, or the whole map
with nothing selected (put back afterwards), and is one undo step.

**Headless:** `heron-brush --run-script <script.rhai> <in.map> [out.map]` opens the map,
runs the script with the whole map selected, and writes the result. `say` lines go to
stdout; a script error goes to stderr and makes the exit code non-zero. Tests cover the
desaturate case, the add/delete/select trio, subtract and merge, the operation limit
stopping a runaway loop, a bad handle as a message, and a wrapped run undoing in one step.

## Example plugins to ship

Under `assets/plugins/`, as the starter set and the test corpus:

1. **desaturate** -- turn the painted colours on the selection to greyscale. A solid
   colour becomes its luminance (`0.299 R + 0.587 G + 0.114 B`); a gradient's two ends
   each do the same; alpha and angle are kept. The litmus test for the colour API.
2. **checker** -- paint the selected faces in two alternating greys by face index, for
   blocking out a level.
3. **stack** -- given one selected brush, make N copies each one grid step higher, for a
   quick column or staircase.
4. **recolour by height** -- map each brush's centre Z through a gradient, so a scene
   reads as a heightmap.
5. **export-face-list** -- count the selection's brushes and faces and say how many
   carry a painted colour, the "a plugin that reports rather than edits" case. It is a
   folder with a `plugin.toml` beside its `main.rhai`, so it is also the worked example
   of a manifest, and the one to try the Install button on.

## Activation and docked panels

Open **Plugins** from the menu. Active and inactive plugins have separate lists with counts. Each plugin has an activation switch that moves it between those lists. Active plugins have a collapsed Actions and settings section. Its state is
stored in `plugin-activation.json` beside the editor preferences. Stable installation
IDs distinguish bundled plugins (`builtin:<name>`) and user files/folders
(`user:<filename>`). Changing a manifest's display name preserves activation;
renaming its installation file/folder creates a different ID. Reinstalling at the
same location preserves the saved state.

New plugins are enabled by default, but loading or enabling one never runs its
script. Disabled plugins remain listed with insensitive controls and contribute no
buttons to **Map → Plugins**. Toggling activation updates open plugin windows and
map panels immediately. **Reload plugins** rescans scripts/manifests and resets
compiled sessions and output. Installing through the manager also refreshes these
views. Setting values are saved by stable installation ID in `plugin-settings.json`
beside preferences, on desktop and in the web filesystem. Reload and editor restarts
restore them, validating types and choices against the current manifest and clamping
numbers to its bounds. Corrupt files are reported and preserved; activation has its
own persistent file. An unreadable activation file does not silently enable plugins.

The GUI and MCP refuse disabled installed plugins, including execution by their
installed script path. The command-line runner checks installed paths too. This is
an editor preference, not a security boundary: copying a script elsewhere and
explicitly running it is a separate installation/execution.

## Buttons and output

Declare buttons in the manifest:

```toml
[[action]]
id = "inspect"
label = "Inspect selection"
description = "Display brush and face counts."

[[action]]
id = "raise"
label = "Move up"
```

Actions and settings can be interleaved. IDs use ASCII letters, digits, underscore
or hyphen, up to 64 bytes. Empty, invalid or duplicate IDs are ignored; each plugin
can contribute up to 16 actions. Labels default to the ID. A plugin without actions
keeps its ordinary **Run** button, so existing scripts need no changes.

Each button runs `main.rhai` from the beginning with the current card settings and
selection. `action()` returns the clicked ID, or an empty string for the legacy
runner. The UI owns layout, keyboard focus and theme colours; scripts cannot inject
UI widgets, markup, HTML or executable callbacks.

```rhai
let selected = selection();
if action() == "raise" {
    for brush in selected { translate(brush, [0.0, 0.0, 16.0]); }
    select(selected);
}
ui_text("Selection report");
ui_value("Brushes", selected.len());
ui_progress("Budget (1000 brushes)", selected.len().to_float() / 1000.0);
```

`ui_text(text)` displays selectable wrapped text. `ui_value(label, value)` displays
a labelled value. `ui_progress(label, fraction)` displays a bar with a finite
fraction clamped to 0..1. It is a result indicator after the run, not a live progress
callback. Output replaces the previous result for that card, with a maximum of 64
items and 4096 UTF-8 bytes per item. Exceeding either limit fails the script.
`say(text)` output also appears in the card and the editor log.

Try the bundled **Selection inspector** example in
`assets/plugins/selection-inspector/`: inspect, move up and move down, with a
configurable step. Its readout follows the selection automatically. With no
selection its move actions do nothing; legacy batch plugins retain their whole-map
fallback.

MCP `list_plugins` includes `id`, `enabled` and action declarations. `run_plugin`
accepts an optional `action` ID and returns both `messages` and a typed `panel`
array. Unknown IDs and disabled plugins fail before editing. The command-line
runner flattens panel output into text and supplies an empty action ID.

## Possible next extensions

Background jobs with cancellation, viewport overlays and additional toolbar/menu
locations are not implemented. The current declarative output and action IDs leave
those extensions independent of the UI toolkit's widget APIs.

## Loaded inspector panels, 14 October 2026

The Map tab's Plugins section and the management window share the same settings,
compiled scripts, state and output. Expand a plugin to show its controls. Scripts
compile on first use and stay loaded until Reload plugins, an activation change or
source replacement. `state("key")` returns the retained value or `()`;
`set_state("key", value)` stores a number, boolean, string or `()`. The state is
limited to scalar values and 64 keys, 128-byte keys and 4096-byte string values, and lasts for this
editor session. Brush handles are local to each invocation; query the current
selection again rather than retaining handles across actions. Headless/MCP invocations create a new session per request. Settings/state are not saved into maps or transferred to peers.

Add a top-level manifest key `panel_action = "inspect"` to request an automatically
refreshed readout. It receives `action() == "inspect"` and normal settings. Refresh
runs only while the card is visible, after map/selection changes settle for 150 ms,
and immediately on initial display or settings changes. It uses an isolated map
snapshot, so script edits in this action cannot affect the real map or Undo.
Errors are shown on the card and do not trigger a retry every frame.

The bundled Selection inspector demonstrates numbers, explicit Move up/Move down
buttons and live brush/face counts. Its move actions act on the selection; an empty
selection moves nothing. Legacy batch scripts still process the whole editable map
when run without a selection. User actions operate on the real document as one Undo
step. There is no unattended background mutation or unrestricted native UI API.

## Persistent settings and entity examples

The bundled **Bulk properties** panel edits one key/value on selected entities as
one Undo step; an empty selection changes nothing. **Project rules** reports missing
required fields, duplicate `targetname` values and unresolved exact `target`
references. Its automatic inspector action reads an isolated snapshot.

`entities()` returns all current entity handles; `selected_entities()` returns
entities containing selected brushes/faces or selected point entities.
`property(entity, key)` returns text or `()` for a missing field.
`set_property(entity, key, value)` requires an entity handle and string values.
Keys are at most 128 bytes without control characters; values are at most 4096
bytes. Handles remain local to the invocation. These functions let a project supply
its own rules without requiring the editor's demo game classes.

Settings are persistent; `state()`/`set_state()` retain scalar script state only for
the current editor session. Neither is serialized into maps or synchronized to peers.
The headless/MCP caller supplies settings explicitly. Restart/corruption persistence,
manifest validation, selected entity mutation with one Undo and read-only rule
reports have automated regression checks.
