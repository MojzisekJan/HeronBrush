# Defold addon

The optional addon adds project commands and a managed level lifecycle to the
existing Defold export and live link. It requires a desktop Heron Brush executable
and Defold with editor scripts and the local editor HTTP API. Tested with Defold
1.13.2 on Linux and Bullet 3.25.

## Setup

1. Save your map. Export once with the Defold target and select the game project,
   unit scale, material and collision settings.
2. Click **Install Defold addon** in Heron Brush's Defold export panel.
3. In Defold, choose **Project > Reload Editor Scripts**, then
   **Project > Heron Brush: Settings**. Select the Heron Brush executable and map.
4. Add `/heron_brush/controller.go` to the game's main collection at the origin,
   with no rotation and unit scale. Keep the player,
   camera and persistent gameplay objects outside the generated map. Do not also
   include the preview collection directly, which would duplicate the map.
5. Choose **Project > Heron Brush: Re-export Map**. For the generated materials,
   choose `heron_brush_live/preview.render` in **Project settings > Bootstrap > Render**.
   Games with a custom renderer can integrate its passes and light messages instead.
6. Build and run the game. Start the live link in Heron Brush and enable
   **Hot reload running game**. Unsaved changes now rebuild and replace the managed map.

After updating Heron Brush, reinstall the addon to update its controller and build
dependencies before rebuilding the game.

Installation creates `heron_brush/`, `heron_brush_live/` and `heron_brush.json`.
It preserves an existing configuration and does not edit `game.project` or the
main collection. Both generated folders are reserved; keep custom code elsewhere.
Unowned folders and symbolic links at generated destinations are rejected.
Reinstalling updates the addon code while preserving project settings.

The same installation is available from a terminal:

```sh
heron-brush --defold-project /path/to/game install
```

## Project commands

| Command | Behavior |
| --- | --- |
| Settings | Select the executable and source map |
| Open Map | Open the source map in Heron Brush |
| Re-export Map | Export the saved source into the reserved preview folder |
| Validate Bindings | Build without publishing, check hints and project component/prefab resources |
| Reload Level | Ask the running game to replace the managed map through Defold hot reload |
| Edit Export and Bindings | Open `heron_brush.json` |

Re-export reads the saved source; live link includes unsaved edits. Reload Level
uses the last published export. It requires the game to have been started from
Defold. For command-line automation, use `--defold-project <project>` followed by
`open`, `export`, `validate` or `reload`.

## Configuration and bindings

Source paths are absolute or relative to the Defold project. `fgd` and `materials`
are optional. Export `options` can contain a partial Heron export profile. Unknown
keys and malformed bindings are errors. The preview target, folder and name are
fixed by the addon. With the addon installed, these overrides also apply to the
live link. Stop and restart the link after editing the configuration. The Heron
export panel supplies defaults for settings omitted from this file; explicit
project overrides take precedence. Stop the live link before using Re-export Map
so a saved source cannot overwrite an unsaved live snapshot.

```json
{
  "map": "maps/level.map",
  "fgd": "maps/game.fgd",
  "materials": "",
  "options": {
    "scale": 32,
    "collision": true,
    "sky": false,
    "entities": {
      "relic": {
        "defaults": {
          "defold_component_logic": "/game/relic.script",
          "defold_property_logic__charge": 9
        }
      }
    }
  }
}
```

Authored entity properties override class defaults. The validation command checks
component and prefab paths in both defaults and exported entities. It also uses
the exporter's material sampler and property validation. Bob remains responsible
for compiling the project scripts and matching their declared properties.
See [Defold binding hints](engine-export-contract.md#defold-project-owned-entity-configuration-14-october-2026)
for body settings, prefabs, typed properties and material samplers.

## Runtime messages

The controller spawns the preview with a collection factory in the existing game
world. It loads and creates a candidate while the previous map stays alive. The
candidate's components are disabled until commit. Once preparation succeeds, it
notifies the listener, deletes the old objects, waits for their deletion and
activates the replacement. Missing resources or an incomplete factory result
leave the previous objects alive, including their state and collisions.

The factory's dynamic prototype is reset for each load. This refreshes the object
list, which plain unload/load would cache. `source.collectionfactory` makes the
preview a direct build dependency for both Bob and the editor. The separate empty
`bootstrap.collection` is the reset prototype; neither is spawned directly. Replacement temporarily needs
capacity for both maps, so allow space in Defold's instance/component limits.

This protects the instances on loader failure. Shared resources may already have
been hot-reloaded by Defold, and errors inside a project's `init` do not propagate
through the factory API. Project scripts should defer irreversible initialization
side effects until `heron_level_loaded`.

Send `heron_reload_level` to the controller's `#controller` component to request a
reload from game code. Repeated requests waiting in the same phase are coalesced.
A request received while loading is serviced after that load finishes.

Set its `listener` URL property to your gameplay script to receive:

- `heron_level_unloading`, with `generation`, one frame before deletion.
- `heron_level_loaded`, with the new `generation`, after object initialization.
- `heron_level_failed`, with `reason` and `kept_previous`, when preparation fails.

Sending `heron_level_status` replies to the sender with `phase` and `generation`.
Runtime object IDs change across replacement. Store persistent game state outside
the managed map. Map entities start from their exported initial state after reload.
A failed preparation keeps the previous objects and does not advance the generation.

With the addon installed, Heron waits for a separate runtime acknowledgement after
Defold's resource hot reload. The controller posts completion or failure to a
short-lived localhost listener with a unique endpoint for that request. Heron
only reports a confirmed generation after the game replies. A timeout or a
runtime failure is an error, including whether the previous objects were kept.
Without the addon, the status remains a project/resource refresh confirmation.
The same acknowledgement is used by the CLI and the editor's Reload Level command.

The controller also reports `HERON_LEVEL_LOADED <generation>` in the game console.
Production builds can use the same controller without an acknowledgement endpoint.
External hot reload is a development operation.

## Default materials and lights

Leaving the model material at its default generates per-material metallic/roughness
shaders. They bind normal, metallic/roughness, ORM, separate roughness/metallic,
occlusion and emissive maps when present. Imported glTF factors, normal scale,
occlusion strength, alpha mode/cutoff and double-sidedness are retained. Face
surface settings and paint clamping are supported too. Explicit project materials
and sampler hints retain their own shader path.

Choose `heron_brush_live/preview.render` as **Project settings > Bootstrap > Render**,
or integrate its passes and light messages into your game's renderer. The generated
renderer draws opaque, double-sided, transparent, sky, water and GUI passes. It
uses point, spot and directional map lights, including runtime pose, colour,
energy and enable/disable changes. A static camera and unchanged lights do not
rebuild the light constants every frame.

The baseline selects up to eight lights, preferring directional lights and then
lights nearest the camera. Shader programs are shared by materials with the same
map slots. It uses direct GGX lighting and ambient light, without
shadow maps or image-based reflections. It assumes Defold's default premultiplied
texture profile; custom non-premultiplied profiles can set `extra_params.w` to zero.
OPAQUE imported albedos with source transparency are made opaque before import,
so zero source alpha cannot erase their RGB values.

## Verification

```sh
cargo test -p editor_core --lib defold_addon
DEFOLD_HOME=/path/to/Defold python3 examples/engine-checks/defold/verify_addon.py
DEFOLD_HOME=/path/to/Defold python3 examples/engine-checks/defold/verify_rendering.py
DEFOLD_HOME=/path/to/Defold python3 examples/engine-checks/defold/verify_editor_reload.py
```

The native Linux check uses a disposable project, Bob and dmengine inside Xephyr.
It checks a changed collider with raycasts, added and removed objects, deletion of
old instances, coalesced requests, notification timing, a persistent outside
object, failed preparation and a real engine-to-Rust acknowledgement. The rendering
check compares pixels for eleven light, material and transparency states and
compiles the imported model material variants. It needs an X display and Xephyr. Logs stay in the reported temporary
project. The editor check creates a fresh project, builds and starts it through
the editor HTTP API, then requires a confirmed replacement generation from the
game. Windows, macOS and other physics backends have not been verified.

Defold API references: [collection factories](https://defold.com/manuals/collection-factory/),
[editor scripts](https://defold.com/manuals/editor-scripts/) and
[hot reload](https://defold.com/manuals/hot-reload/).
