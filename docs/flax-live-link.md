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
files are never overwritten. A fingerprint manifest records generated versions, so
unchanged generated files can be upgraded on a later installation. Older addons
without a manifest are accepted only when their bytes match the current version. Projects with several custom modules need to
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
Imported models now use the same complete export pipeline as final exports.
Material reloads also trigger publication when the map text is unchanged.
Authored environments, camera settings and reflection probes travel in snapshots;
external HDR file changes are polled twice per second. Camera and probe settings
also travel when sky export is disabled.

Immutable, content-named model and cube files avoid changing the displayed model while
its replacement imports. Unused versions are retained; do not clean this folder
while saved scenes still reference its assets.

## Native verification

Linux native checks used a local Flax installation and the .NET 8 SDK.
Prepare a disposable project from `examples/engine-checks/flax`. Use the scene template
as `Content/ExportCheck.scene` and omit `CollisionCheck.cs`, `ExportCheckPlugin.cs`,
`MotionCheck.cs` and `WaterCheckPlugin.cs` from the copied source. Those scripts
require the runtime classes from a final export. Then run:

```sh
HERON_FLAX_LIVE_PROJECT=/absolute/path/to/test-project \
  cargo test -p editor_core --lib native_flax_live_fixtures -- --ignored --nocapture
cp examples/engine-checks/flax/LiveCheckPlugin.cs /absolute/path/to/test-project/Source/Game/
FlaxEditor -headless -null -project /absolute/path/to/test-project -std -heronlivecheck
```

Set `HERON_FLAX_LIVE_MOTION=1` for both the fixture publisher and the editor to
use Motion Lab. That variant checks the new point-light actor and the revised
animation payload after the editor acknowledges the snapshot. The live scene
stays at its authored pose. Use the final Motion Lab export to test playback.

The native test applies four successive snapshots: original, moved brush,
deleted brush, and undo. It checks a single owned root, preserved attachment
position and unrelated actors, replacement geometry, restored triangle count,
and native colliders. The verified demo produced 48 actors and 20 colliders at
each stage, with `HERON_LIVE_CHECK PASS` and `Total errors: 0`.

macOS 1.13 native checks also apply the four snapshots, preserve the moved root
and unrelated actor, and retain 20 colliders through deletion and undo. The
updated fixture waits for streamed mesh buffers before comparing triangle counts.
With environment and camera actors the demo has 52 root children.

Windows and Play Mode transitions still need interactive checks. Headless
checks validate scene objects and collision assets, not GPU material appearance.
