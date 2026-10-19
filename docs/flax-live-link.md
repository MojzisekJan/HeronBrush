# Flax live link

The link sends unsaved Heron Brush edits into an open Flax scene. A fixed C#
editor addon imports JSON snapshots, models and collision geometry. Drawing does
not rewrite scripts or trigger script compilation.

## Setup

1. Export once into the Flax project's `Content` directory. Configure the project
   root, textures, unit scale and collision settings for that game.
2. In **Export → Flax**, click **Install Flax addon**. Let Flax compile the addon.
3. Click **Start live link** in Heron Brush.
4. Open a scene in Flax and choose **Tools → Heron Brush: attach live map**.

The addon is installed under `Source/Game/HeronBrushLive`. If there is no Game
module, a single unambiguous source module is used. Existing modified addon
files are never overwritten. Projects with several custom modules need to
choose a game module before installation can be automated.

The live assets live under `Content/HeronBrushLive`. This is a reserved,
generated directory. A pre-existing folder without the ownership marker is
rejected. The final export and hand-authored content remain separate. The
saved final-export content-path override does not redirect live resources.

Updates build a replacement branch before removing the old one. Only roots
with the `HeronBrushLiveAnchor` component belong to the link. The attachment's
placement and other scene objects are preserved. Keep custom actors and scripts
outside the generated root. Save the Flax scene yourself; generated models and
cooked collision assets are real project assets and must stay with that scene.
Reattach from Tools after script reload or reopening a scene; the anchor allows
the addon to recover the saved root.

**Tools → Heron Brush: stop live map** pauses the receiving addon. The header
indicator in Heron Brush stops publishing. Updates pause in Flax Play Mode.
Changing the source map requires stopping/restarting the publisher; an existing
attachment belonging to another map is not silently replaced.

Collision cooking uses the same importer as standalone Flax export, including
triangle meshes and convex trigger hulls. Entity metadata stays available in
`preview.json`. The link does not invent gameplay for arbitrary class names.
Immutable, content-named model files avoid changing the displayed model while
its replacement imports. Unused versions are retained; do not clean this folder
while saved scenes still reference its assets.

## Native verification

Linux native checks used a local Flax installation and the .NET 8 SDK.
A disposable copy of `examples/engine-checks/flax` is prepared using:

```sh
HERON_FLAX_LIVE_PROJECT=/absolute/path/to/test-project \
  cargo test -p editor_core --lib native_flax_live_fixtures -- --ignored --nocapture
cp examples/engine-checks/flax/LiveCheckPlugin.cs /absolute/path/to/test-project/Source/Game/
FlaxEditor -headless -null -project /absolute/path/to/test-project -std -heronlivecheck
```

The native test applies four successive snapshots: original, moved brush,
deleted brush, and undo. It checks a single owned root, preserved attachment
position and unrelated actors, replacement geometry, restored triangle count,
and native colliders. The verified demo produced 48 actors and 20 colliders at
each stage, with `HERON_LIVE_CHECK PASS` and `Total errors: 0`.

Windows/macOS and Play Mode transitions still need interactive checks. Headless
checks validate scene objects and collision assets, not GPU material appearance.
