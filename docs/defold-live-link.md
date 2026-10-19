# Defold live link

The link updates a Defold collection while you edit in Heron Brush, including
unsaved changes. It uses the same debounce and background geometry builder as
the Godot and Unity links. It does not require a Defold addon.

## Setup

1. Open your game project in a recent Defold editor. The integration requires
   the editor's authenticated `/eval` API and `.internal/editor.token`.
2. Export the map once using the **Defold** target. Set the project directory,
   unit scale, texture directory, model material, sampler and collision options.
3. In the Defold export panel, click **Start live link**.
4. Open `heron_brush_live/preview.collection` in Defold. You can also reference
   this collection from your own scene. The preview's path stays fixed when the
   map is renamed.

The preview uses a separate, reserved folder. It does not overwrite the final
export or modify `game.project`. An existing folder without the live-link marker
is rejected. Keep your gameplay scripts and hand-authored scenes outside this
folder: its contents are generated. The preview uses its actual resource paths,
not the final export's optional asset-directory override.

Only one map window can publish into the same project at a time. The header
indicator shows the connection state and stops publishing when clicked. Closing
or replacing the source map stops the link. A closed/restarted Defold editor is
retried automatically, with its port and token read again each time. Network
requests run off the UI thread and only connect to IPv4 localhost.

## Running game

Enable **Hot reload running game** to request a runtime refresh after each
published batch. This switch applies to the current editor session. Start the
game from Defold first, and make sure it includes the live preview collection.

Defold's hot reload does not guarantee that existing instances are recreated.
Adding or removing objects, changing transforms or collision shapes may require
restarting the level or game. A game-owned level controller can provide its own
reload lifecycle; this link does not inject one into an arbitrary game.

Heron Brush distinguishes confirmed project refresh, an accepted asynchronous
hot-reload request, and confirmed hot-reload completion. Older Defold versions
return HTTP 202; the UI does not present that as proof of runtime completion.
Build errors and connection failures appear in the live-link status. Hot reload
may include other modified game resources, according to Defold's own behavior.

## Verification

Automated tests cover project boundaries, preserving existing folders, full
snapshots after moving/deleting/undoing geometry, collision resources, paint
textures, native resource paths, HTTP truncation and version-dependent command
responses. Optional tests exercise an open Defold editor:

```sh
HERON_DEFOLD_LIVE_PROJECT=/absolute/path/to/isolated/defold/project \
  cargo test -p editor_core --lib native_editor_refresh -- --ignored --nocapture
```

`native_defold_demo_snapshot` publishes the repository's demo level into that
project. The project must contain the demo textures and `/main/model.material`.
Set `HERON_DEFOLD_EDIT` to `paint`, `move` or `delete` to publish an edited
snapshot, and `HERON_DEFOLD_HOT_RELOAD=1` to request hot reload. Use a disposable
copy of `examples/engine-checks/defold`, not a production game.

On Linux with Defold 1.13.0, the native checks published the full demo (96
files), built it in the editor, and sent paint, move and deletion updates. The
runtime console confirmed reloads of `worldspawn.modelc` and
`worldspawn.collisionobjectc`. Those resource messages do not prove that every
existing physics body or gameplay instance was recreated. Windows and macOS
live-link checks are still pending.

API references: [Defold editor HTTP API](https://defold.com/manuals/editor-http-api/)
and [hot reload](https://defold.com/manuals/hot-reload/).
