# Godot live link

The live link uses the Godot scene builder ported from heron_brush. Unlike `heron_brush watch`, it reads the current editor document and does not require saving the `.map` file.

## Setup

1. In Heron Brush, open Export → Godot and export once into a Godot project. The project must contain `project.godot`.
2. In the same export page, click **Install Godot addon**.
3. In Godot, enable **Heron Brush Live** under Project → Project Settings → Plugins.
4. Click **Start live link** in Heron Brush.
5. Open a 3D scene in Godot. In the Heron Brush dock, click **Attach map to current scene**.

The addon requires Godot 4.4 or newer. The integration was checked with Godot 4.7.1.

Drawing, moving, deleting and undoing in Heron Brush now update the attached map branch in Godot. After a short pause, changes are queued after 300 ms. During continuous drawing a build can start once a second, provided the previous build is finished. Building happens on a worker thread; there is no growing queue of old edits. Content comparisons prevent selection-only changes from rebuilding the map.

The header's Godot indicator reports the link status; click it to stop. The Export page also has a Stop button. Closing the map window or replacing its document stops the link. Starting a link is a session action and does not automatically resume after restarting the editor. Restart the link to pick up changed export options.

## What is updated

The normal Godot export options apply, including entities defined by the loaded FGD and collision geometry. Live output is written into `heron_brush_live/` inside the selected Godot project, separate from the manually exported scene. It uses the existing Godot export material handling; it does not add new material/shader conversion capabilities.

The Godot addon creates an instanced branch named `HeronBrushMap`. On an update it replaces that branch, preserving its placement in the parent scene. Nodes created elsewhere in the Godot scene are kept. Put custom cameras, scripts and other Godot-side additions outside the generated branch: edits inside it are replaced by subsequent updates.

Save the host scene in Godot to retain the attached instance and its placement. The source map remains editable in Heron Brush and is not saved by live publishing. The connection updates the Godot editor scene; it does not inject changes into an already running game and does not send Godot edits back to Heron Brush. Use one active Heron Brush link per Godot project.

Files are replaced atomically where supported by the filesystem, unchanged files are left untouched, and the manifest is published after the scene and meshes. A publishing marker and revision check prevent the addon from applying a batch while a newer one is being written. Godot explicitly scans and reimports meshes, so focusing its window is not required.

## Verification

Rust tests cover project path validation, repeatable addon installation without overwriting local edits, debounce timing, unsaved edits, deletion and undo. A separate check ran the actual addon in a headless Godot editor, attached a cube, moved it four metres, deleted it and restored it. The imported mesh counts, positions and ownership matched after every update, and an unrelated Godot node kept its position throughout. Interactive editing on Windows and macOS remains to be tested.

The same publishing scheduler also supports [Unity](unity-live-link.md) and [Defold](defold-live-link.md). Only one
engine link can run per map window; stop it before switching targets.
