# Keyframe animation for brushes

Select a brush, or the brushes of one entity, and enable **Keyframe animation**
in the menu next to the grid size. The panel opens below the views and also
works in the separate 3D window. The **Animation editor in separate window**
icon in the panel header moves the timeline into its own resizable window with
the same map, frame and Undo history; enabling the docked panel again closes it.

Animation applies to translation and rotation of one brush entity. Rest geometry
is never changed by animating.

## Tracks and keys

In the normal timeline, translation is one **Position** track whose keys store
X, Y and Z together. Rotation has RX, RY and RZ tracks. In **Curves** view the
coordinates can be edited separately. Translation is relative to the rest
geometry in map units. Rotations are in degrees around the pivot, applied in
X, Y, Z order; 0 to 360 is one full turn, and angles are not wrapped, so several
turns work.

New timelines start at rest, with one zero-value key at time zero in each
channel.

Click a track name to make it active. **Add key** inserts a key at the playhead
with the current interpolated value. Double-clicking the timeline adds a key to
that track; in Curves view it also sets the value.

- Click selects a key; Shift or Ctrl extends the selection.
- Drag on empty space for a box selection. Ctrl+A selects visible keys.
- Dragging keys moves the whole selection (and values, in Curves view). One drag
  is one Undo step; Escape cancels it.
- Delete removes the selected keys. An empty track has value 0.
- Ctrl+C copies keys, Ctrl+V pastes them at the playhead keeping relative
  spacing and tracks. Ctrl+D does both.
- Moving or pasting never overwrites an existing key, and pasted keys must fit
  within the animation duration.
- A track holds up to 256 keys.

The inspector on the right edits the time and value of one selected key. With
several keys selected it shows their shared interpolation (empty if mixed), and
a new choice applies to all of them.

## Curves and interpolation

**Curves** shows the active track's graph. A key's interpolation controls the
segment towards the next key:

- **Linear** changes the value evenly.
- **Hold** keeps the value until the next key.
- **Bezier** uses a cubic curve with in and out tangents.

Tangents can be dragged in the graph or entered as a slope in value units per
second. Handle length is fixed at one third of the adjacent segment.
**Smooth tangents** computes slopes from neighbouring keys and switches both
adjacent segments to cubic. Cubic curves can overshoot the end key values.

**Fit** frames the whole animation and value range. Ctrl+wheel or +/− zooms;
the middle button and plain wheel pan the timeline. **Snap to frames** snaps
drags to whole frames.

## Retiming

**Retime keys** (in the **⋯** menu) changes selected keys across all tracks in
one operation. A scale of 2 doubles their spacing, 0.5 halves it. Scaling is
centred on the first selected key, or on the playhead with **Scale from
playhead**. **Offset (frames)** shifts the result by an exact amount.

**Extend duration** allows the animation to grow to fit; otherwise keys must
stay inside it. Crossing or overlapping another key, or moving before time
zero, rejects the whole operation. Bezier tangents are rescaled to keep their
normalised shape, which can also adjust the tangent of an unselected neighbour.
One Undo reverts everything.

## Playback and preview

Click or drag in the top ruler to scrub. The **Frame** field sets an exact
frame and the arrow keys step by frames. Buttons jump to the previous/next key
or the start/end. Space toggles playback when the graph has focus.

Duration and FPS are saved. Duration cannot be shorter than the last key. Modes
are **Once**, **Loop** and **Ping-pong**. Before the first key and after the
last, a track holds its end value. FPS is an editing grid; exports do not
resample curves to frames.

**Onion skinning** shows outlines of 1 to 4 previous (blue) and next (orange)
poses of the selected brushes, with configurable frame spacing. Coloured axes
show orientation around the pivot. While dragging keys only the graph updates;
the 3D preview picks up the new curve on release.

The Top, Front and Side views show the same frame as the timeline and 3D view,
including rotation and onion skinning. The frame number is shown in the view
corner. Hidden brushes stay hidden. **Preview animations** plays animations
without the timeline, using a shared time for 2D and 3D.

In animation mode each view shows **Edit animation** with the current frame. A
button in the timeline panel switches to **Edit map**, which pauses playback and
shows rest geometry with the normal editing handles. Scrubbing or editing a key
returns to animation mode. Pressing Escape in a 2D view also leaves the preview.

## Motion path

In animation mode the selected entity shows a cyan path of its pivot
translation in 2D and 3D. Dots mark key times, labelled with frame numbers.
Smooth segments are solid, Hold jumps are dashed. Pure rotation around a fixed
pivot has no path.

Click a dot in 2D to move the playhead there and select its translation keys.
Dragging a dot changes position in the two visible axes, keeping depth and
rotation; it snaps to the world grid unless snapping is off. Keys between whole
frames keep their exact time. Where dots overlap, the one nearest the current
frame is picked. With **Auto key** enabled, path dots can also be dragged in 3D
in the screen plane. One drag is one Undo step; Escape cancels.

## Direct editing in 2D

With a timeline frame active, drag the selected animated brush in Top, Front or
Side. Picking uses its animated geometry. A complete Position key is recorded
at the playhead, including the unchanged depth coordinate. Grid snapping
applies and playback pauses on pickup.

With the Rotate tool, or the panel's **Rotate** mode, drag the brush around its
pivot to key the rotation axis perpendicular to the view. Angles accumulate
across full turns and use angle snapping. Each drag is one Undo step; Escape
cancels. Clicking empty space does not change rest geometry.

## Auto key in 3D

Select brushes of one entity, set a frame and enable **Auto key**. A new
animation starts from a static rest state; an existing one is kept. Enabling it
pauses playback and turns on onion skinning. It works in the main and separate
3D views.

- **Translate**: X, Y and Z handles insert or update the Position key at the
  current frame.
- **Rotate**: ring handles for RX, RY and RZ (Euler degrees in timeline order)
  around the animated pivot.
- **Pivot**: move the pivot along X, Y or Z.
- Translation snaps to the grid and rotation to 15° steps; disable snapping
  for free movement.
- Preview, curves and onion skins update during the drag. One drag is one Undo
  step; Escape cancels. Clicking off a handle does not change geometry.

Disable Auto key for ordinary selection and geometry editing. The camera still
responds to the right and middle mouse buttons. Local transform axes are not
supported yet.

## Moving the pivot

Choose **Pivot** in the animation panel. In 2D, drag the yellow pivot marker,
even outside the brush; it moves in the two visible axes, keeps depth and snaps
to the world grid. Auto key is not needed for this. In 3D, enable **Auto key**
and drag an axis handle.

The pivot is shared by the whole animation and all clips of the entity; it is
not keyed. Changing it leaves keys and rest geometry unchanged but recomputes
rotated poses around the new centre. One Undo reverts a drag and Escape cancels
it. Exact coordinates can be entered in the Pivot fields.

## Named clips and presets

Choose the clip above the timeline. Enter a name and click **Save As** to copy
the current clip. Switching clips keeps recent edits. **Delete** removes the
chosen clip; at least one must remain. An entity can have up to 32 clips, named
with letters A–Z, digits, underscores and hyphens. Clip changes are undoable.

- **Door animations** creates `open` and `close`: a smooth 90° turn around the
  current pivot over one second, and back. It replaces existing clips with those
  names and disables **Play on start**. For a hinged door, move the pivot to the
  side edge first.
- **Translate** creates a `translate` clip moving 128 units along X over one
  second in ping-pong mode.

Other clips are kept. Selecting a group of brushes and adding keys or a preset
animates the group as one entity with a shared timeline and pivot; the group
keeps its identity and brush IDs. A selection spanning several existing
entities is not a single animation target.

For an L-shaped platform path, key Position at the start, move the platform to
the corner on a later frame and to the destination on another. Linear
interpolation gives straight segments; Ping-pong returns along the same path.

## Animation trigger panel

Select a `func_button`, `trigger_multiple` or `trigger_once` and open the
Entity inspector. The **Animation trigger** panel sits above the property list.
Choose an animated entity from the list, or use **Pick target** and click it in
any view (Escape cancels). The editor creates `targetname` when needed and draws
arrows along entity links. Missing targets or clips are reported below the
controls. A shared `targetname` activates every matching animated entity.

Actions are **Toggle**, Play, Open, Close, Pause, Resume and Stop. Play uses a
named clip or the target's selected clip. Toggle alternates `open` and `close`
when both exist, otherwise it restarts the selected clip. Stop restores the
rest pose; Pause keeps the current one. **Only once** overrides the entity class
default. **Repeat delay (s)** is a cooldown between activations, not a delay
before the first.

**Try trigger** runs the action in the shared 2D/3D preview without changing
the map or Undo history. **Reset preview** restores rest poses and clears
once/cooldown state. The preview does not simulate player collisions or game
scripts.

## Storage

Rest geometry is unchanged. Curves are stored in the `animation_curves`
property, the mode in `animation_mode` and the pivot in `animation_pivot`.
Triggers use `animation_action`, `animation_clip`, `animation_once` (`1`/`0`),
`wait` and the usual `target`/`targetname`. Older maps without these trigger
properties keep toggle behaviour and their class defaults.

Older animations in `animation_keys` load without change and are converted to
separate tracks on first edit. `animation_keys` still receives end samples for
older editor versions, which cannot play the new curves faithfully.

Scaling, event and sound tracks are not supported. Keyframe animation exports
to Godot, raylib, Defold, Bevy and Unity; other exporters warn that it is
omitted.

## Export

All exports handle named clips, independent translation and rotation tracks,
pivots, Linear/Hold/Bezier, once/loop/ping-pong, autoplay, and links from
`func_button`, `trigger_once` and `trigger_multiple` (honouring `wait`).
Invalid animation data produces a warning and the entity stays static. Exported
data is regenerated on each export; changes made in the engine do not flow back
to the map.

### Godot 4

Export the scene from the Export window or use Godot Live Link. No addon is
needed. Animations with `start_on = 1` play automatically; `start_on = 0` keeps
the entity at rest until `play()` is called.

Each animated entity gets a `HeronAnimation` child with a native
`AnimationPlayer` and `AnimationLibrary`. A short `@tool` script embedded in the
`.tscn` turns the channels into a transform around the pivot, so edits in
Godot's AnimationPlayer also show in the Godot editor. Bezier tracks
`map_x/y/z/rx/ry/rz` hold the smooth motion and constant tracks
`step_x/y/z/rx/ry/rz` add Hold jumps exactly. Times are in seconds, rotations
in map degrees and offsets in map units. Motion runs in the physics step.

`func_animate` exports as `AnimatableBody3D`, so mesh and collision move
together. An existing entity script is kept but should not write the same
transform.

```gdscript
$MovingPlatform/HeronAnimation.play()       # Restart from the beginning.
$MovingPlatform/HeronAnimation.pause()
$MovingPlatform/HeronAnimation.resume()
$MovingPlatform/HeronAnimation.seek(1.25)   # Seconds, respects the loop mode.
$MovingPlatform/HeronAnimation.stop()       # Restores the rest geometry.
$Door/HeronAnimation.play("open")
```

Once mode emits `finished` at the end. For doors, give the door a `targetname`
(for example `main_door`) and point a `func_button` or `trigger_multiple`
`target` at it. A `CharacterBody3D` entering the trigger alternates `open` and
`close`; other physics bodies do not. Buttons react to entering the area, not
to a Use key. The door preset can reverse mid-motion without a jump; custom
clips need to be exact reverses of each other for that.

The regression test `godot_animation_matches_editor_and_moves_collision` runs
the exported scenes in Godot and compares transforms with the editor preview.
Run it with `--ignored`; set `GODOT_BIN` to the Godot executable.

### Raylib

The export includes `<map>_animation.h`, a dependency-free C99 player, and
`<map>_preview.c`, a raylib 5+ free-camera preview. Build instructions are in
`<map>_raylib.md` beside the level.

Call `HbInit` once per animated entity and `HbUpdate` each frame. Apply
`HbMatrix` after the entity rest transform, or use `HbPoint` on rest world-space
vertices. Apply the same transform to collision and update your physics body.
Keep `HbTriggerState[HB_TRIGGER_COUNT+1]` alongside the player array;
`HbCheckTriggersSpan` tests a standing player's feet, middle and head against
the exported volumes, `HbCheckTriggers` a single point, and
`HbActivateTrigger` handles explicit button presses. The preview has no
collision response and activates triggers with the camera.

The C regression test compiles the exported header and compares positions and
normals with the editor. Set `HERON_TEST_RAYLIB` to a raylib distribution to
compile the preview as well.

### Defold, Bevy and Unity

These use the shared `_heron_animation` payload.

- **Defold:** each animated object's generated `#properties` script plays the
  animation. Colliders are kinematic. Send `heron_play_animation` with
  `{clip="open"}`, `heron_pause_animation`, `heron_resume_animation`,
  `heron_seek_animation` with `{time=1.5}` or `heron_stop_animation`. Triggers
  use the player collision group, or the `heron_activate` message sent to the
  trigger's `#properties`. A generated `<map>_animation.md` explains object URLs.
- **Unity:** Build Level attaches `HeronBrushAnimation` and a kinematic
  Rigidbody, with `Play(name)`, `Pause()`, `Resume()`, `Seek(seconds)` and
  `Stop()`. FixedUpdate moves the root and its colliders. Keep the generated C#
  runtime outside the Editor folder. `HeronBrushAnimationTrigger` activates on
  entry by a CharacterController or Player-tagged collider; call `Activate()`
  for explicit button presses.
- **Bevy 0.19:** add `HeronAnimationPlugin` from the exported
  `heron_animation.rs` and attach `HeronAnimation::from_json(data)` next to each
  root's Transform, where `data` is `properties._heron_animation.value` (or the
  `Value::String` in the typed loader). Trigger roots get
  `HeronAnimationTrigger::from_json`, animated roots `HeronAnimationName`, and
  `HeronActivator` marks the player. Call `trigger.activate()` for buttons. The
  `demos/bevy_fps` sample wires this up with kinematic Avian bodies; other
  physics adapters must synchronise the animated Transform themselves.

Pivots are in the coordinate frame of the exported root's parent, so put
scene-wide placement on a common parent.

## Motion lab demo

The welcome window includes `motion_lab`, an editable map with an L-path
platform, a lift, a rotating platform and a hinged door. Its bundled
`godot/project.godot` has an FPS controller and exported button links. See
[the demo instructions](../demos/motion_lab/README.md) for controls,
re-exporting through MCP and the physics regression test.
