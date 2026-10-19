# Unity live link

The bridge publishes unsaved map edits to Unity's editor scene, using the same
300 ms debounce and background build worker as the Godot link. It is a one-way
editor connection; it does not update Play Mode or send Unity edits back.
The addon uses Unity 2022.3+/Unity 6 editor APIs. Native import and live synchronization were verified on Linux with Unity 6000.3.24f1.
The test project uses the Built-in Render Pipeline. Windows, macOS, URP and HDRP
remain unverified.

## Setup

1. Export the map once with the **Unity** target into a folder under the Unity
   project's `Assets` directory. The project must have
   `ProjectSettings/ProjectVersion.txt`.
2. In Export → Unity, click **Install Unity addon**. The installer adds an editor
   window and two metadata components. It refuses to overwrite locally edited
   addon files.
3. Let Unity import/compile the addon, then open **Window → Heron Brush → Live Link**.
4. In Heron Brush, click **Start live link**. In the Unity window, click
   **Attach map to active scene**.
5. Edit the map in Heron Brush. The Unity window must remain open (it can be docked).
   Mesh import is explicit, so Unity does not need to regain focus for each change.

The Heron Brush header shows whether the selected engine is building, waiting or
has acknowledged the update. Click it to stop. Only one live engine can be active
per Heron Brush map window, and one publisher per target project. Restart the
link to apply changed export options. A stopped publisher leaves the last Unity
scene result intact. Closing/replacing the source map stops publishing.

## Scene ownership

Attachment creates/reuses a `HeronBrushMap` root with `HeronBrushLiveRoot` metadata.
Only its `Generated` child is replaced. The attachment's transform and objects
outside that child are retained. Put custom cameras and gameplay scripts outside
`Generated`; user edits inside it are overwritten by later updates. The addon
pauses in Play Mode and Prefab Mode and refuses to attach to a preview scene.
Use **Live updates** to pause, and **Retry latest update** after fixing an import
problem. A new map source requires explicit attachment.

Save the Unity scene yourself to keep the result. The addon marks the scene dirty
but does not save the user's scene or source `.map`. Metadata components and
collision mesh assets survive saving and reopening the scene.

## Data and limits

- Geometry uses the existing Unity OBJ export with Unity axis conversion and metre
  scale. Each material uses a separate OBJ group. Loaded editor textures, solid
  colours and gradients are exported as portable PNG albedo images with an MTL
  library. Custom shaders and URP/HDRP material conversion are not implemented.
- Native point/spot/directional lights are created. Typed entity properties and
  target links are retained in `HeronBrushLiveEntity.entityJson`. This does not
  automatically install gameplay scripts. The optional kit in
  `examples/engine-checks/unity/Gameplay` provides an FPS player, doors, triggers,
  portals, enemies and HUD. Both `live.json` and the final `.gameplay.json`
  sidecar expose flat property arrays, class names, target names and map scale
  that Unity JsonUtility can read without an extra JSON package.
- Static triangle collision and per-brush convex hulls are included, even for
  invisible trigger volumes. Convex colliders are subject to Unity/PhysX limits.
- The map hierarchy is rebuilt under one generated branch, matching the existing
  Unity importer's flat entity layout.

Live data is written under `Assets/HeronBrushLive`, separate from final exports.
Mesh filenames depend on their contents; a new revision cannot mutate mesh assets
still used by the previous scene branch. Collider assets are also content-addressed.
Unchanged geometry is reused. Old generated assets are retained because saved
Unity scenes can still reference them; remove unused cached assets manually only
when you no longer need those scenes. Do not edit generated cache files.

The shared publisher writes a busy marker, then complete files, then the manifest
`.heron-brush-live.json` in the project root. The addon checks the revision again
before replacing `Generated` and writes an acknowledgement to
`.heron-brush-live-status.json`. Ordinary updates write data, not C# sources, and
therefore do not cause a C# recompilation on every edit.

## Verification

Rust tests cover project containment, installer idempotence/local edit protection,
unsaved geometry, deletion/undo, native light data, collision data and the absence
of generated scripts in update batches. The shared Godot publishing tests remain
in place. The native demo test also checks matching final/live colliders and invisible
triggers, material textures, physics raycasts, pause/resume, recovery after a missing
mesh, preservation of unrelated objects and root transforms, and scene save/reload.
A separate GPU capture verifies rendering. Test sources and reproduction commands
are in `examples/engine-checks/unity`. Remaining manual/platform checks:

1. Attach a cube to a scene containing an unrelated camera and object.
2. Move, resize, recolour/reassign an existing material, delete and undo the cube
   without saving the source map; inspect geometry, UVs and metre scale.
3. Add point/spot/sun lights and check direction, colour, cone and range.
4. Add a trigger brush and inspect its invisible collider and metadata.
5. Move the attachment root; confirm later updates preserve that transform and the
   unrelated objects. Save/reopen the Unity scene and verify collision persists.
6. Pause/resume, enter/exit Play Mode, close/reopen the live window, stop/restart the
   publisher, replace the source map and retry an intentionally missing mesh.
7. Repeat on Windows/macOS, including spaces and non-ASCII project paths.
