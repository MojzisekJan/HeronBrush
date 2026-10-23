# Getting started
Heron Brush is a brush-based level editor. You build levels from convex brushes, paint and texture their faces, place entities, animate doors and platforms, and export the result to a game engine. It also has built-in assets, parametric shapes, live links to engine editors, shared editing sessions and still-image rendering. This manual is bundled with the application and works offline.

On macOS, `Cmd` works wherever this manual says `Ctrl`, and Option is Alt.

## The welcome screen
When Heron Brush starts without a map, the welcome screen opens. On the left are **Continue** (the last map you had open), **New map** and **Browse** for opening an existing `.map`, plus **Game**, **Manual** and **Preferences**. On the right is a list of recent maps with thumbnails, followed by **Examples**. The star (**Keep at the top**) pins a map to the top of the list; the bin (**Move to the trash**) moves the file to the system wastebasket, so it can be restored. The arrow keys move through the screen and Enter opens the focused item. File → Welcome brings the screen back at any time.

## Your first map
1. Choose **New map** on the welcome screen, or File → New map (`Ctrl+N`).
2. Pick the Brush tool (`B`) in the tool strip and drag in a 2D view to draw the selected shape. Check its depth in another 2D view.
3. Select the brush and move it by dragging. Use the **Face** tab to give it a material or a colour.
4. Save the source map with `Ctrl+S`. When you are ready to use it in a game, open File → Export and write a scene for your engine.

## Files and saving
The `.map` file is your editable source. Save regularly: exporting, live links and rendering never save the map for you. File → Save as (`Ctrl+Shift+S`) adds the `.map` extension if you leave it out. File → Revert to saved map throws away unsaved changes and reloads the file. If you close a window or open another map with unsaved changes, Heron Brush asks whether to **Save**, **Discard** or **Cancel**.

While a saved map has unsaved changes, numbered backups are written every ten minutes to an `autosave` folder next to the map. The 50 newest backups are kept.

Opening a map while Heron Brush is already running hands it to the running window, which comes to the front. Use `--new-instance` on the command line when you want a separate window (see Command line).

# The interface
The main window has a title bar with the menus, a tool strip under it, the map views, the inspector on the right and a status bar along the bottom.

## Title bar and menus
The title bar shows the logo, the menus, the map name in the middle and the window buttons. On macOS the usual red, yellow and green buttons sit at the left. Drag the title bar to move the window and double-click it to maximise. While something runs in the background, an indicator appears near the end of the bar: an engine live link, a live session or **MCP active**. Click an indicator to stop the live link or to open its window.

- **File**: Welcome, New map (`Ctrl+N`), Open map (`Ctrl+O`), Save (`Ctrl+S`), Save as (`Ctrl+Shift+S`), Revert to saved map, Export, Render image, Plugins and Preferences (`Ctrl+,`).
- **Edit**: Undo (`Ctrl+Z`), Redo (`Ctrl+Shift+Z`), Cut, Copy, Paste, Duplicate (`Ctrl+D`), Delete, Select all (`Ctrl+A`), Repeat last commands (`Ctrl+R`) and Forget repeatable commands.
- **View**: 3D view in separate window, Go to, Snap to grid (`Alt+0`), Show grid (`0`), Preview animations and Show omitted faces.
- **Geometry**: Insert 3D text, Join corner, Convert faces to patches, Move selection out of its entity and Sync linked groups.
- **Animation**: Keyframe animation, Preview animations and Place camera from 3D view (`Ctrl+Alt+0`).
- **Map**: Load, Reload and Unload point file; Load, Reload and Unload portal file. These show leak traces and portals written by map compilers.
- **Collaboration**: Live session and Serve for an agent (MCP).
- **Help**: Manual and About.

## The tool strip
The tool strip carries every tool. The first group is used all the time: Select, Brush, Clip, Vertex, Edge, Face, Rotate and Scale. The second group holds Shear, Assemble, Sweep, Control point, Fillet and Paint. Hover over a button to see its name and shortcut. Clicking the active tool again returns to Select.

Enable **Show simulation preview button** in Preferences → View → Window to put a start/stop preview control beside Lock and UV in the tool strip. It uses the same preview state as Animation → Preview animations. The preference survives restart; it is off by default.

After the tools come **Flip horizontally** and **Flip vertically**, then two switches: **Lock** keeps the material mapping when a brush moves, and **UV** keeps it when a brush turns (`U` toggles it). The active tool's own settings follow: the Brush tool shows the shape and its options, and the Fillet tool shows its radius, faces and corners with **Apply**. At the right end are the tool name, the grid size (**Grid 16** and so on; click to choose another) and the **Grid settings** menu with Snap to grid, Show grid, Preview animations, Show omitted faces and Keyframe animation.

## Map views
There are four views: the 3D view and three flat views, Top (XY), Front (XZ) and Side (YZ). Preferences → View → Layout chooses One pane, Two panes, Three panes or Four panes. Drag the dividers to resize the panes. Clicking in a view gives it the keyboard. `Space` moves a pane that shows one view at a time to its next view, and `Ctrl+Space` maximises the view under the keyboard and restores the layout again.

Right-click without dragging in any view to open a context menu for the selection and what is under the pointer. It offers grouping, layers, linked groups, creating entities, moving brushes into or out of an entity, copying a material name and revealing it in the material browser.

## The inspector
The inspector on the right has four tabs. Drag its left edge to change its width.

- **Map** (`Ctrl+1`): Layers, Groups, Plugins, Issues, View filters and Tags.
- **Entity** (`Ctrl+2`): the entity browser, the entity definitions, the properties of the selected entity and, for buttons and triggers, Animation trigger.
- **Face** (`Ctrl+3`): UV, Materials, Colour, Sweep, Fillet, Patch, and Inset & bevel. Each section can be folded.
- **Assets**: the library of reusable brush models.

`Ctrl+5` hides or shows the inspector and `Ctrl+Alt+T` hides or shows the tool strip, which gives the views more room.

## The status bar and the output pane
The status bar along the bottom shows the editor's latest message. Errors are drawn in red. Click the bar, or press `Ctrl+4`, to open the output pane with the full history of messages. Most commands report what they did here, for example how many brushes were clipped.

## Dialogs and appearance
Dialogs such as Preferences, Export and Render image open as windows of their own. Preferences → View → Colour scheme chooses Follow the desktop, Light or Dark. **Panel opacity (%)** on the Overview page makes the panels see-through over the views. The window draws its own buttons in the style of your system: native on macOS, Windows 11 style on Windows and GNOME style on Linux.

# Navigation and keyboard
## Moving in the 3D view
Hold the right mouse button and drag to look around. Drag with the middle button to pan. The wheel moves forward and back. `W`, `A`, `S` and `D` fly forward, left, back and right, `Q` rises and `X` descends; hold `Shift` for larger steps. With **Alt + middle-drag moves the camera** switched on in Preferences → Mouse, Alt with the middle button moves along the view direction. Speeds, inversions and the field of view are in Preferences → Mouse.

## Moving in 2D views
Drag with the middle button to pan and use the wheel to zoom around the pointer. Panning and zooming one flat view keeps the others aligned; **Link 2D views** in Preferences → Mouse controls this. `Ctrl+Alt+Z` resets the zoom and `Ctrl+Shift+U` resets the flat views to the origin.

`Ctrl+U` frames the selection in every view. View → Go to (`Ctrl+Alt+M`) moves the camera to typed coordinates, selects what is at a line of the map file, or moves the selection by a typed amount.

## Grid and snapping
`+` and `-` change the grid size, and the number keys `1` to `9` choose a size directly, from 1 to 256 units. `0` or `#` shows or hides the grid, and `Alt+0` turns snapping on or off. `S` snaps the selected brushes' vertices to the current grid, `Ctrl+Shift+V` snaps them to whole units and `Ctrl+Alt+Shift+V` to the grid.

## Common shortcuts
- `Ctrl+S`: save the map. `Ctrl+Shift+S`: save as.
- `Ctrl+Z`: undo. `Ctrl+Shift+Z` or `Ctrl+Y`: redo.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: copy, cut and paste. Paste puts the objects at the pointer; `Ctrl+Alt+V` keeps their original coordinates. With faces selected, copy takes their material and UV settings.
- `Ctrl+D`: duplicate. `Ctrl+Shift+D`: linked duplicate of a group.
- `Delete` or `Backspace`: delete the selection.
- `Esc`: cancel the current drag, step back in the active tool, return to Select, and finally clear the selection. `Shift+Esc` returns to Select directly.
- `Ctrl+R`: repeat the last repeatable commands. `Ctrl+Shift+R`: forget them.
- `Ctrl+Alt+0` or `Ctrl+Alt+Num0`: place the render camera at the 3D view.
- `F5`: reload the materials. `F6`: reload the entity definitions.

## Selection shortcuts
- `Ctrl+A`: select all, or all handles while a vertex, edge or face tool is active. `Ctrl+Shift+A`: select nothing. `Ctrl+Alt+A`: invert the selection.
- `Ctrl+B`: select the other objects in the same entity or group.
- `Ctrl+T`: select everything the selected brushes touch. `Ctrl+E`: select everything inside them. `Ctrl+Shift+E`: select everything in their column along the view axis. These use up the selecting brushes.
- `Ctrl+I`: isolate the selection. `Ctrl+Alt+I`: hide it. `Ctrl+Shift+I`: show everything again.
- `Ctrl+G`: group. `Ctrl+Shift+G`: ungroup.

## Transform shortcuts
- Arrow keys and `Page Up` / `Page Down` move the selection by one grid step in the direction they point in the active view. With `Ctrl` they duplicate while moving, and with `Alt` they rotate.
- `Alt+X`, `Alt+Y`, `Alt+Z`: rotate a quarter turn about that axis; add `Shift` for the other direction.
- `Ctrl+F`: flip across the view; `Ctrl+Alt+F`: flip up and down. `Ctrl+Shift+X` and `Ctrl+Shift+Y` mirror along X or Y.
- `Insert`: place a point entity at the selection centre.
- `Tab`: with a brush selected, cycle Select → Vertex → Edge → Face → Select.

## The Blender keymap
Preferences → Keyboard → Keymap offers Heron Brush (the default, used throughout this manual) and Blender. With Blender: `G` selects and moves, `R` rotates, `S` scales, `E` sweeps, `1`, `2` and `3` choose the vertex, edge and face tools, `K` clips, `B` selects, `A` selects everything and `Alt+A` drops the selection, `X` deletes, `Shift+D` duplicates, `H` hides and `Alt+H` shows everything, and `Ctrl+Shift+Z` redoes. Everything else is the same in both.

# Brushes and modelling tools
Brushes are convex solids defined by planes. Build complex objects from several simple brushes rather than trying to make one brush concave. Keep the brush count low where a simple shape is enough.

## Drawing brushes
Choose the Brush tool (`B`) and drag in a view. The tool strip shows **Shape to draw**: Cuboid, Stairs, Arch, Cylinder, Cone, Spheroid (UV), Spheroid (Icosahedron), Hollow sphere and Insert 3D text. `Alt+1` to `Alt+8` choose a shape directly. Open **Shape settings** to change **Hollow** and **Thickness:**, **Step height:** and **Direction:** for stairs, **Axis:**, **Sides:** and the circle mode for round shapes, **Rings:** for UV spheres, **Accuracy:** for the icosahedron and **Brushes** for the hollow sphere. Set the options before you drag. Check the result in more than one view.

### Hollow sphere made of brushes

Select **Hollow sphere** or press `Alt+8`, open **Shape settings** and set
**Thickness:** and **Brushes**. Choose 20 to 320 convex cells in steps of four,
for example 76, 80 or 84. Opposite edges are refined together so the shell
stays centrally symmetric. Typed counts round to the nearest supported value.
Draw it like any other shape. Drawing depth follows the
reference bounds. Equal dimensions on all three axes make a sphere; stretching
the bounds makes an ellipsoid.

Each cell has an outer and inner triangular surface and three contact faces.
Adjacent contacts match exactly and use `skip`; the centre stays empty.
Thickness is radial at the vertices on the shortest semi-axis and must be
positive and smaller than that radius. It scales proportionally along the
other axes of an ellipsoid. The result is ordinary selected brushes with one
Undo step, editable and exportable through the existing targets.

### Stairs with landings and turns

Choose a straight flight, left/right L turn or U return. Straight flights accept
an optional **Landing**; turned layouts use **Flight width**. **Slab thickness**
zero makes solid stairs; a positive value makes a continuous sloped underside
with vertically measured thickness. The landing connects to both flights.
Advanced layouts use equal riser heights and at most 512 steps.

## Selecting
Click a brush to select it, and `Ctrl`-click to add to the selection. A drag from empty space draws a selection box. In the 3D view, `Shift`-click selects a single face and `Ctrl+Shift`-click adds more faces. Dragging a selected brush moves it; dragging an unselected one selects and moves just that one.

In the 3D view, `Shift`-drag a face of a selected brush to extrude it. `Alt`-click another face to copy the selected face's material and alignment onto it; `Alt+Ctrl` copies only the material and `Alt+Shift` copies the material with UV rotated to follow the target face.

If the window manager uses `Alt` to move windows, press `Ctrl+C` on the source face, select the target faces and press `Ctrl+V`. Copy/paste also accepts Face-tool handles. The transfer carries the source face's paint as well: a flat colour, or a gradient whose placed ends are fitted to the target face. When the source has no paint, the target's paint is removed so its material shows.

## Shaping tools
- **Select** (`Esc`): select and move.
- **Vertex** (`V`), **Edge** (`E`), **Face** (`F`): drag corners, edges or faces. `Shift`-drag from the middle of an edge or face adds a corner there. `Delete` removes the picked corners.
- **Rotate** (`R`), **Scale** (`T`), **Shear** (`G`): transform the selection with handles.
- **Clip** (`C`): click two or three points to define a cutting plane. `Enter` cuts, `Ctrl+Enter` chooses which side to keep, and `Backspace` or `Esc` removes the last point.
- **Assemble** (`N`): place points around a volume and press `Enter` to make a convex brush from them. `Backspace` removes the last point.
- **Sweep** (`Y`): see the Sweep section below.
- **Control point** (`P`): reshape a patch's control grid.
- **Fillet** (`O`): click an edge to pin its preview, adjust radius, faces and corners, then confirm with **Apply** or `Enter`. With edges picked in the Edge or Fillet tool, the right-click menu offers **Fillet**, which switches to the Fillet tool with those edges and opens its settings. Click a brush body to select it while the tool stays active; `Ctrl`-click a body to add or remove a brush. On edge handles, `Ctrl` adds or removes edges. Clicking empty space clears brushes and preview edges. This works in both 2D and 3D. More faces make a smoother result and more geometry.
- **Paint** (`K`): see Materials and UVs.

Open **Shape settings** from the toolbar to configure any drawn shape. Selecting another shape there opens the window automatically. Its vertical rows include numeric −/+ buttons. The window stays open while you work in the map. Changes update a provisional shape immediately; **Confirm** accepts it, **Cancel** removes the insertion. **Close** only hides the window and keeps the shape editable until the selection changes.

## Combining brushes
- `Ctrl+J`: merge the selected brushes into their convex hull.
- `Ctrl+K`: subtract the selected brushes from the brushes they touch.
- `Ctrl+Shift+K`: hollow the selection, with walls one grid step thick.
- `Ctrl+L`: intersect the selection.

## Sweep, fillet, patch and bevel
**Sweep** builds geometry along a path from selected faces. Select faces with `Shift`-click using Select, then set **Travel**, **Turn** (axis and angle), **Scale**, **Steps** and **Path** (Integer or Free alignment; Arc, Straight or S-bend) in the Face tab's Sweep section, and press **Sweep**. The line under the controls says how many brushes it will make. Source materials,
paint and PBR settings are retained, UVs carry across shared lateral edges, and
interior caps are hidden. A collapsed cell or solid self-intersection refuses
the whole operation. New cells retain their source parent, use one Undo step
and are limited to 1024 in total.

The **Fillet** section rounds the picked edges with **Radius**, **Faces** and **Corners** (Mitred, Chamfered, Rounded or Setback); one face makes a chamfer. Radius and segment count have −/+ buttons.
**Inner L corner** fills the inside of two overlapping rectangular members
with equal depth and the same parent, including unequal member widths.
The 3D Fillet preview displays the resulting solid surfaces and highlighted edges,
including inner fills. Radius and segment changes update the preview before Apply
or Enter; changing tools restores the original surfaces without editing the map.
Outer Fillet UVs unfold across their strips and continue onto the second
straight wall when materials match. Spherical multi-edge junctions can retain
separate UV seams.

**Patch** shows the selected patch's **Columns** and **Rows** with buttons to add or drop a pair. Geometry → Convert faces to patches (`Ctrl+P`) makes patches from selected faces.

**Inset & bevel** works on selected faces: **Inset** draws the face in from its edges and **Depth** pushes it out (or in, when negative). **Bevel faces** applies both; the third button adds a panel or cuts a recess, depending on the sign of Depth. **Join corner** opens a dialog that joins two brushes at a mitred or rounded corner with a preview.

In **Join corner**, the −/+ buttons disable steps beyond the actual geometry
limits. Invalid typed values retain the last valid preview. **Full-thickness
wedges** offers **Preserve wall thickness**, which moves the terminal contacts
so each straight facet has the intended perpendicular thickness. Unequal arm
widths are interpolated across the segments. Scroll over a preview to zoom,
drag to pan, and double-click or use **Reset view** to fit the whole corner.
Both previews share their zoom and pan.

**Connected face region** adjusts the shared boundary simultaneously, without
insetting a shared selected edge twice. Entire coplanar edges shared by separate
brushes are supported too. A failure refuses the whole operation; disabling the
option treats faces individually.

The same panel offers **Hollow selection** with **Inward**, **Outward** or
**Centered** thickness, **Open selected faces**, and **Join convex pair**.
Joining requires two brushes enclosing a convex solid under the same parent
and removes their original dividing wall. Openings leave no lid over the
selected faces. The Hollow shortcut retains its one-grid-step inward behaviour.

After drawing, the shape remains in **Shape preview**, outlined with the resize
handle colour. Brush settings keep editing that insertion, including hollow
spheres, cones, stairs and the other generators. **Confirm** or `Enter` finishes
parameter editing; **Cancel** or `Escape` undoes the insertion. Changing selection,
switching tools or editing geometry accepts the current result as ordinary
brushes. Insertion and all parameter changes form one Undo step. Invalid settings
retain the last valid geometry. Current valid geometry is saved and exported
normally; generator parameters are temporary and are not restored after
confirmation or reopening the map.

## 3D text
Geometry → Insert 3D text (also the last entry of the Brush tool's shape list) builds brushes from text. It uses Heron Block, a built-in polygon font with clipped corners, proportional widths and Czech accents; lowercase letters use capital forms. Type the **Text**, set **Height** (the cap height; accents rise above it) and **Depth**, watch the live preview and press **Insert**.

If an operation cannot produce valid geometry, adjust its dimensions, or undo and simplify the shape.

## Arrays
Geometry → Array copies the selection and shows the copies in the map while you change the values. Counts include the original, and an array makes at most 1024 copies.

- **Row** moves each copy by **Step X**, **Step Y** and **Step Z** from the one before and turns it by **Turn per copy (degrees)** about the chosen axis.
- **Grid** fills **Count X**, **Count Y** and **Count Z** items, **Step X**, **Step Y** and **Step Z** apart.
- **Ring** spreads the items over **Angle (degrees)** round a centre **Radius (units)** from the selection. With **Face the centre** each copy turns with the ring.
- **Scatter** puts the copies at random places within the radius, across the chosen axis. The same **Seed** gives the same layout, and **Random turn** turns each copy.

When the selection is one whole group, **Linked copies** makes each copy a linked duplicate, so an edit to one reaches them all. **Apply** keeps the copies and selects them with the original; **Cancel** removes them. The whole array undoes in one step.

## Sculpting terrain
Terrain is made of many small brushes whose surface the sculpt brush pushes and pulls. Select one or more brushes and choose Geometry → Split into columns. It cuts them along the grid lines into columns one grid step across and puts the columns in a group named Terrain. The cuts follow the world's grid, so brushes side by side are cut in the same places and share their corners. Each column stands along the brush's thin side: upright in a floor or a ceiling, sideways in a wall. A cave works the same way: hollow a box (Hollow selection) with a wall thickness of one grid step, select the walls and split them.

The **Sculpt** window, which the button at the end of the sculpt strip opens, chooses the shape of the columns and can split the selection too. **Triangle** makes two per grid cell, and a triangle's top stays flat however it is sculpted. **Square** makes one per cell, half as many brushes; its top bends along a diagonal. **Hexagon** tiles the brush with hexagons two grid steps across, each cut into six triangles round its centre, so every top stays flat.

Choose the **Sculpt** tool (`J`) and drag over the surface in the 3D view. The orange ring shows the brush. With nothing selected the brush changes every brush under it; with a selection, only the selected ones. The strip at the top of the window chooses what a stroke does:

- **Raise** and **Lower** move the surface out or in, most at the centre of the ring and nothing at its edge. Hold `Shift` to swap them.
- **Smooth** evens out bumps.
- **Flatten** levels the surface to where the stroke began.

In the same window, **Direction** chooses which way:

- **Up** moves the surface straight up and down, within a circle on the ground. This is for terrain.
- **Camera** moves it towards and away from the view, within a ball round the pointer. Look at a floor, a wall or a ceiling and it shapes that.
- **Normal** moves it out of the face under the pointer where the stroke begins.

The two numbers are **Radius (units)** and **Strength (%)**. Only the corners on the side the stroke moves move; the back stays where it is, and a column stays at least one unit thick. Where brushes share a corner they move it together, so the surface stays closed. A stroke undoes in one step. In the 2D views the tool selects as the Select tool does.

When the sculpting is done, select the terrain and choose Geometry → **Merge columns**, or the button in the **Sculpt** window. Neighbouring columns that together make a convex brush and look the same become one brush, so flat stretches turn into a few large brushes. The surface does not move, and the merge undoes in one step.

For terrain like a voxel game's, split into **Square** columns and choose **Blocks** under **Stroke** in the **Sculpt** window. A stroke then moves each column's whole top by whole grid steps and keeps its sides upright, so the ground rises in steps. Raise, Lower, Smooth and Flatten work as before; small dabs add up until a column has risen a whole step. Columns of other shapes are left alone.

**Freeform** needs no columns: it works on any brushes. Raise adds ground in a ball round the pointer and Lower takes it away, so the ground can hang over and hollow out into caves. Smooth evens it out and Flatten levels it to where the stroke began. Each dab rebuilds the brushes it reaches on the grid, rounded, and leaves everything outside its ball as it was, sharp edges included. The result is ordinary brushes, so every other tool works on it, and **Merge columns** makes them fewer afterwards. A finer grid gives a smoother surface and more brushes.

# Materials and UVs
## Loading and applying materials
Open the Face tab's **Materials** section. **Load folder** adds a folder of images and **Load .wad** adds a texture package; both appear under **Collections**, where the cross removes one. Type in **Filter materials** to narrow the list. Click a material to make it current and apply it immediately to the selected faces or brushes. Without a selection it only changes the current material. **Apply to selection** reapplies it. Assigning a texture removes solid or gradient paint on those faces while preserving their UV mapping. **Select** offers Select brushes with it and Select faces with it. **Replace** swaps one material for another across the map or only in the selection.

The **Paint** tool (`K`) applies the current material to the face under the pointer. Hold the left button and drag across faces to paint continuously; one Undo restores the whole stroke. This works in 2D and 3D, preserves UVs and clears paint hiding the texture. A click paints one face, a double click paints the whole brush, and `Shift`-click picks the material like an eyedropper.

After `Ctrl+C` on a source face, **middle-click** a target face to paste its material and UVs in 2D or 3D without changing selection. Middle-drag still pans the view. This gesture never inserts whole objects from the clipboard.

## The UV editor
Open **View → UV editor in separate window**, or use the icon with an arrow leaving a square in
the inspector's UV section. Its tooltip names the action. The window provides a larger canvas and the same
controls, sharing the map's face selection and document. **Return to main window**
restores the Face tab and shows the inspector if it was hidden. Desktop builds
use a native window; web builds use a floating dialog. Resizing reframes the face.
**Undo/Redo** use the shared history; the native window also accepts `Ctrl+Z`,
`Ctrl+Shift+Z` and `Ctrl+Y`.

The Face tab's **UV** section shows the selected face's material and outline. Mouse controls on the canvas:
- The wheel zooms around the pointer. Drag with the right or middle button to pan. Double-click the right button to frame the face again; selecting another face frames it automatically.
- Left-drag moves the material while the face outline stays in place. The ring rotates the mapping, the corner handles scale it and the centre handle sets the origin of these changes.
- `Alt`-left-drag on a tile edge shears, as does `Shift`-middle-drag.
- Each drag is one undo step, and `Esc` cancels a drag in progress.

Below the canvas are **Offset**, **Scale**, **Angle** and, when the game uses them, **Content flags**, **Surface flags** and **Surface value**. The buttons are **Reset**, **Reset to world**, **Fit**, **Flip H**, **Flip V** and the two 90° turns. **Align** turns the material along one of the face's edges (press again for the next edge, `Shift` for the previous one). The four justify buttons push the material against an edge. **Fit U** and **Fit V** scale so that a whole number of copies covers the face; press again for the next count, and hold `Ctrl` to keep a trim sheet's subdivision.

## UV keys
With faces selected, the arrow keys move the material, `[` and `]` turn it by 15°, `,` and `.` halve or double its scale, `Ctrl+F` and `Ctrl+Alt+F` flip it, `Shift+R` resets it, `Alt+Shift+R` resets it to world axes and `Ctrl+Shift+F` fits it to each face.

**Lock** in the tool strip keeps the mapping while brushes move, and **UV** (`U`) keeps it while brushes turn.

### Texture handles in the 3D view
With the Select, Face or Paint tool, a selected face that shows its material (not a paint) gets handles in the 3D view. A white outline marks one tile of the material, or a block of tiles or part of one, so that it stays a usable size on the screen.
- The blue square at the outline's corner moves the material. It snaps to the face's corners, the middles of its edges, its edges and its centre, so one drag puts a corner of the texture into a corner of the wall.
- The round handle at the next corner turns the material about the blue corner. Near a face edge it snaps into line with it.
- The far corner scales the material on both axes; the middles of the two far edges scale it along U or V only.
- `Shift` keeps a move to one axis, a turn to 15° steps and a corner scale even. `Alt` drags without snapping, and `Esc` cancels the drag.

A note beside the pointer shows the offset, angle or scale. Every selected face changes, as in the UV editor, and each drag is one undo step. After a drag the outline stays on the tile that was moved until another face is selected.

## Tool materials
- skip: removes the marked face from engine exports.
- clip: creates collision without visible geometry.
- origin: marks the pivot of a brush entity.

The names can be changed in the Export window's **Tool materials** card. Use skip for faces you know are hidden.

## Omitting faces from export
Select a face and switch on **Omit face from export** in the UV section. Orange hatching marks it in the UV preview. Its material and paint stay saved, and the brush stays closed and editable. Exported render meshes leave that face out while collision is kept. Leaving one quad of a cube produces two render triangles. This does not make faces double-sided or merge meshes.

**Omit covered faces** marks faces fully covered by static brushes, including identical touching faces and enclosed brushes. It works on the selected brushes, or on all visible unlocked world geometry when nothing is selected. Brush entities, transparent materials and brushes that already have omitted faces are skipped. Partly covered faces are not split. One undo restores everything. View → Show omitted faces hides or shows such faces in the editor views; picking still uses the whole brush.

# Colour and gradients
The Face tab's **Colour** section paints faces with flat colours or gradients on top of their materials. It is useful for blocking out a level and for stylised art. Painted faces keep their real colours in the views; the selection is shown by its outline.

## Palette and mix colour
Click one of the twelve palette swatches to paint the selected faces, or whole selected brushes, with it. **Opacity** sets how opaque the next paint is, in percent. **Mix** has a colour button that opens the colour picker; drag in the picker and the selection follows live, or press **Paint** to apply the mix colour as one step. **Back to material** removes the paint so the face shows its material again. The line at the top of the section says what is selected and what it wears.

## The colour picker
Every colour button in the editor opens the same picker. Drag in the square to choose saturation and brightness, and in the strip beside it to choose the hue. Below are **R**, **G** and **B** values and an **Opacity** slider where the colour has one. The hex field takes `#RRGGBB` or `#RRGGBBAA`; it turns red while the text is not a valid colour. **Copy** puts the hex code on the clipboard. Click outside the picker to close it.

## Gradients
**Gradient** chooses Linear gradient or Radial gradient, and the button beside it swaps the colours end for end. Each stop has a colour button and **Position (%)**, and stops between the ends have a remove button. **Add colour stop** adds a stop in the widest gap; a gradient can have up to eight stops. **Gradient angle** sets the direction of a linear gradient, with **+45°** and **−45°** buttons.

Under the preview is a rail with a marker for each stop. Click the rail to add a stop, click a marker to open its picker, drag a marker to move it, and right-click a marker to remove it. The three swatches under the rail are presets: Warm gradient, Cool gradient and Greyscale gradient. Changes to a gradient apply to the selection live.

A gradient's direction matters: orient it along a roof slope or along a curved object, not across unrelated faces. Check the model from both sides after painting.

### Handles on the face
With the Select, Face or Paint tool, painted faces that are selected show their paint as handles in the 3D view.
- A face in one colour has a dot at its centre. Click it to open the colour picker; the face follows the picker live.
- A gradient shows a line between its start and end. Drag the square ends to place the gradient anywhere on the face, even beyond it. They snap to the face's corners, the middles of its edges, its edges and its centre. `Shift` keeps the direction to 15° steps or along an edge, and `Alt` drags without snapping.
- The round handles on the line are the colour stops between the ends. Drag one along the line to move it. Click any handle, an end too, to change its colour.
- `Alt`-click the line to add a stop in the colour the gradient has there, and `Alt`-click a stop to remove it.

Faces painted alike change together, and each drag or picker session is one undo step. Placed ends set the gradient's direction in place of its angle. **Fit to face** in the Colour section, shown for such a gradient, returns it to the whole face, and so does changing **Gradient angle**.

## Surface material
**Surface material** sets **Roughness** (0 is glossy, 100 is matte), **Metallic** (0 for wood, stone or plastic, 100 for metal) and **Emission** (0 to 8; the surface glows in its own colour, including gradients, but does not light nearby objects). Changes apply live to the selected faces or brushes. **Use default surface** removes the override. All changes can be undone.

Surface effects show in the lit 3D view and in camera renders. Flat and wireframe display leave out the highlights on purpose. Bloom can add a glow around emissive surfaces. Godot and glTF/GLB receive the material values, and the Unity importer turns roughness into smoothness. See `docs/surface-materials.md` for the limits of each target.

# Assets, layers and groups
## The asset library
Open the **Assets** tab. Add a folder of `.map` models with **Add folder**; the built-in library is always there. Type in **Filter assets** to search. Click a model to select it, then double-click it, press `Enter` or click **Insert** to add it where the pointer last was. You can also drag a model into a view. Asset brushes are inserted as one group so the model moves as a whole.

**Save selection as asset** writes the selected brushes to the library as a `.map`. Library assets are ordinary editable brushes, not imported meshes.

## Layers
The Map tab's **Layers** list organises large areas. Type a name and click **Add** to make a layer, or press `Enter` to rename the current one. Each layer can be hidden, locked or left out of exports. **Move here** moves the selection into the current layer. The context menu in the views also offers Move to layer, Make active layer, Hide layers and Isolate layers.

## Groups and linked groups
`Ctrl+G` groups the selection and `Ctrl+Shift+G` ungroups it. The Map tab's **Groups** section lists groups and offers **Open**, **Rename**, **Merge**, **Ungroup**, **Take out** and **Extract**. A linked group is a set of copies that share edits: `Ctrl+Shift+D` makes a linked duplicate, Geometry → Sync linked groups updates the copies, and **Unlink** makes a copy independent. In the Entity tab, a property can be kept as one copy's own when linked groups sync.

## A shared scale
The default export scale is 32 map units per metre. The built-in player-start marker is 64 units tall, a 2-metre reference. Compare doorways, vehicles, furniture and plants against it. Change the export scale only on purpose and keep related assets consistent.

# Entities and gameplay
Common entity definitions are built in and always available: lights, player starts, doors, buttons, platforms, triggers, static props, sounds and waypoints. Loading a project FGD with **Entity definitions** in the Entity tab replaces classes with the same name and keeps the other built-ins.

## Placing entities
In the Entity tab's **Entities** browser, type in **Filter classes** and choose a class. For a point class such as a light, click **Place** or double-click the class; `Insert` places one at the selection centre. For a brush class such as a door or trigger, select the brushes and click **Make**. Classes can also be dragged into a view.

## Properties
The property list edits the selected entity. Entities of the same class are edited together. **Add property** adds a key the definition may not know; **Defaults** can put existing properties back to their defaults or add the missing ones. Properties the class defines but the entity does not have are shown dimmed; typing a value adds them. Use targetname and target to link entities.

## What an export can do
Godot's directives create native nodes, collision bodies and trigger areas. Unity and Unreal support native common lights, and Unreal also creates PlayerStart actors. Other data is kept as far as the target's importer allows.

A door entity alone does not add game behaviour. Movement, trigger responses, player controllers and project scripts still need work in the engine. Defold exports entity properties and collision objects; Flax places models and generic entities. Check the result in your engine before building gameplay on it.

# Animation
Heron Brush can preview simple motion and edit keyframe animations of brush entities, then export them to several engines.

## Preview animations
Switch on **Preview animations** in the View menu, the Animation menu, the Grid settings menu or the Map tab's View section. It plays func_rotate rotation and func_shuttle back-and-forth motion when start_on is enabled, using the entity's axis, speed, direction, travel and wait properties, and it plays keyframe clips set to Play on start. The fan and windmill assets already contain rotating parts.

Turning the preview off restores the original pose. A click in a view stops playback before selecting or editing; camera navigation keeps working. The preview does not change the map, the undo history or exports, and it does not run physics or game logic.

## The keyframe editor
Open it with Animation → Keyframe animation or from the Grid settings menu. It docks under the views, or in the 3D window while that is separate. The window icon in its heading moves it into a window of its own; it keeps the frame and the undo history. **Edit animation** switches the views to the animated pose and **Edit map** goes back to the rest geometry. In animation mode each view shows **Edit animation** and the frame number at the bottom.

Select a brush entity, or a brush or brush group, to animate it. A flat brush group shares one animation and pivot; groups that contain several entities need separate animations.

## Clips
The **Animation** selector switches between named clips. Type a name in **Animation name** and click **Save as** to copy the current clip under that name; **Delete** removes a clip. **Translate** creates a `translate` clip without rotation. **Door animations** creates smooth `open` and `close` clips with a 90-degree turn around the current pivot; move the pivot to the hinge first. **Play on start** makes the clip play when the level starts; the door preset switches it off.

## Transport and playback
The transport buttons are **Start**, **Previous key**, **Playback**, **Next key** and **End**. **Frame** shows and sets the current frame, and **Duration (s)** sets the clip length. The mode is **Once**, **Loop** or **Ping-pong**.

## The timeline
The timeline has a ruler with the playhead and a lane for **Position** (X, Y and Z keyed together) and for each rotation angle, RX, RY and RZ. Click or drag in the ruler to move the playhead and preview that pose in every view. Drag with the middle button to pan the timeline. Drag keys to move them in time; `Shift` adds to the selection; double-click a lane to insert a key; `Ctrl`+wheel zooms. **Curves** shows the value curves instead of the lanes.

The buttons above it are **Add key**, **Remove**, **Copy**, **Paste** and **Duplicate**. **Snap to frames** keeps keys on whole frames and **Fit** shows the whole clip. **Retime keys** opens **Time scale**, **Offset (frames)**, **Scale from playhead**, **Extend duration** and **Retime selection**.

The key inspector beside the timeline edits the selected key: **Frame**, **Value** (or the three coordinates of a Position key), the interpolation **Linear**, **Hold** or **Bezier**, **Incoming slope** and **Outgoing slope**, and **Smooth tangents**.

## Keys in the editor
While the timeline has the keyboard: `Space` plays or pauses, the left and right arrows step one frame, `Ctrl+A` selects all keys, `Ctrl+C`, `Ctrl+X` and `Ctrl+V` copy, cut and paste keys, `Ctrl+D` duplicates them, `Delete` removes them, `Ctrl+Z` undoes and `Esc` cancels a drag. Outside the timeline, `Esc` leaves animation mode. Undo keeps the zoom, the selection and the playhead where they were.

## Posing in the views
The row under the timeline has **Onion skinning** with a count and **Spacing**, which shows nearby poses as ghosts; the **Pivot** coordinates; **Auto key**; and the pose tools **Translate**, **Rotate** and **Pivot**.

- **Add key** turns on Auto key and pauses playback. With Auto key on and **Translate** chosen, drag a coloured axis of the selected entity's handle in the 3D view to key its position at the current frame.
- In a 2D view, drag the selected brush or a member of its group to key translation along the two visible axes, with grid snapping. Auto key does not need to be on for this.
- With **Rotate**, drag the brush in a 2D view around its orange pivot marker. Top keys RZ, Front keys RY and Side keys RX, snapped to grid angles.
- With **Pivot**, drag the yellow rotation centre in any 2D view. It snaps to the grid and keeps its depth. Keys and rest geometry stay unchanged.

The cyan path in 2D and 3D shows the pivot's travel. Click a path point to select its time and keys, or drag it to change the position; with Auto key on you can also drag path points in the 3D view. Labels show frame numbers and dashed segments mark jumps. `Esc` cancels any of these drags, and one undo reverses a whole drag.

New animations start at rest with keys at time zero. Make motion by changing the pose at another frame. For a sharp L-shaped path, key the start, the corner and the end, and use Linear with Ping-pong.

## Connecting buttons to animations
Select a func_button, trigger_multiple or trigger_once. The Entity tab shows **Animation trigger** above the properties. Choose a target from the list, or click **Pick target** and then an animated object in any view; `Esc` cancels picking. Missing targets or clips are reported under the controls. Entity links show arrows to their targets when link display is on.

**Action** chooses what activation does: **Play** starts a named clip or the selected animation; **Toggle** alternates `open` and `close` when they exist, otherwise it restarts the selected animation; **Open** and **Close** start those clips; **Pause** holds the pose, **Resume** continues and **Stop** restores the original pose. **Only once** overrides the class default. **Repeat delay (s)** sets the shortest time between activations; it does not delay the first one.

**Try trigger** runs the action in the preview without editing the map or adding undo steps. **Reset preview** restores the original poses and clears once and cooldown state. This does not simulate player collisions or game scripts.

## Animation in exports
Godot exports native AnimationPlayer and AnimationLibrary resources, including curves and moving collision. A CharacterBody3D entering a button's area activates its target. Edits made to these clips in Godot do not come back to the map and are replaced by the next export.

Raylib export adds `<map>_animation.h` with a C player and `<map>_preview.c` with a free-camera preview; follow `<map>_raylib.md` to build it. In the preview `SPACE` pauses, `R` restarts the default clips and `N` cycles through named clips. Defold embeds playback in the entity script with documented `heron_*_animation` messages. Unity's Build Level adds `HeronBrushAnimation` and a kinematic body. Bevy export includes `heron_animation.rs` and instructions. Godot, Unity, Defold, Bevy and raylib keep the trigger links, the once flag and the repeat delay. Your game still has to register the player and connect its own buttons to each runtime's API.

## The Motion lab example
Open `motion_lab` from the welcome screen's Examples. It has an L-path platform, a lift, a rotating platform and a hinged door with buttons. Open the `godot/project.godot` next to it in Godot and press `F5`. WASD and the mouse move the player, Space jumps, 1 to 4 visit the stations and R restarts the current station. Step on a coloured pad to activate its target. The included README explains re-exporting.

# Exporting to an engine
File → Export opens the Export window. The engines are listed down the left with their logos: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb, and last `.map`. The page on the right says what the target will get. **Write** writes it, **Check** builds the level without writing it and lists the problems, and **Close** closes the window.

Exporting is separate from saving: the `.map` stays your editable source. Every control saves its engine's settings as soon as it changes, so each engine keeps its own answers and the window opens as you left it.

## Common options
- **Scale**: **Units per metre**, 32 by default.
- **What to write**: **Collision geometry**.
- **Optimisations**: **Group static meshes in 1024-unit cells**, **Drop faces buried inside the geometry**, **Join faces that share a plane** and **Join collision shapes**.
- **Project paths**: where the level and its resources go inside the project.
- **Tool materials**: the names of the clip, skip and origin materials.
- **More**: engine-specific switches such as **Name nodes after**, **Rebuild layers and groups as parent nodes**, **Reverse the triangle winding**, texture-coordinate flipping and a second UV set for baked light with **Lightmap texel**.
- **Keeping up with the map**: **Write again whenever the map is saved**. It becomes available after the engine has been exported once, because that export says where to write.

Static cells assign world brushes by their centres and combine them by material within each 1024-unit cell, so engines can cull smaller parts of the map. Collision is kept and gameplay entities stay separate. It can mean more draw calls than one world mesh, so it is off by default. Standalone OBJ, glTF and GLB exports do not use cells.

## Target notes
- Godot: texture and material `res://` paths, the trigger physics layer and lightmap UV2. Choose an export folder inside the Godot project.
- Defold: the asset resource directory, the model material and the texture sampler, which must exist in the chosen material.
- Unity: OBJ meshes and an editor importer; export under the project's Assets folder.
- Unreal Engine: a generated import script and a content path, normally /Game/<level>.
- Flax Engine: a model content path relative to the project's Content folder.
- three.js, Bevy, raylib and Cocos Creator: scenes or code for those runtimes, with instructions next to the export.
- glTF / GLB: portable geometry for compatible tools; GLB is the binary form. OBJ: geometry and materials for modelling tools.
- `.map`: **Write as** chooses Valve 220, Standard, Quake 2 or Quake 2 (Valve). The window says what the chosen dialect would lose.

Read the warnings after writing, such as missing textures. **What the check found** lists them in the window.

# Live links: Godot, Unity, Defold and Flax
A live link sends unsaved edits from Heron Brush into an engine editor as you work. It is one-way: edits made in the engine do not come back. One live link can run per map window. Stop it before switching engines, and restart it to apply changed export options. The controls are in the Export window, on the engine's page, in the **Live link with** card.

The indicator in the title bar shows the link's state; click it to stop the link. Closing or replacing the map stops publishing. After an import failure, fix the problem and start the link again.

## Godot
1. Export once into a Godot project.
2. Click **Install Godot addon**.
3. Enable Heron Brush Live in Godot's Project Settings → Plugins.
4. Click **Start live link** and attach the map from the Godot dock to an open 3D scene.

The addon replaces only its generated branch and keeps the attachment's placement and other nodes. Save the Godot scene yourself, and keep your own scripts and nodes outside the generated branch.

## Unity
1. Export once under the Unity project's Assets folder.
2. Click **Install Unity addon** and let Unity compile it.
3. Open Window → Heron Brush → Live Link in Unity.
4. Start the link and attach the map to the active Unity scene.

Keep the Unity live window open. Updates pause in Play Mode and Prefab Mode. Only the Generated child of the map object is replaced. Do not edit the generated files.

## Defold
1. Export once into your Defold project and keep the project open in Defold.
2. Click **Start live link**. No addon is needed.
3. Open heron_brush_live/preview.collection in Defold, or reference it from your own scene.

The link writes to a separate generated folder and leaves the final export and game.project alone. **Hot reload running game** also updates a game started from Defold; adding objects or changing collision may require restarting the level. Connection failures are retried automatically.

## Flax
Export into the project's Content folder and click **Install Flax addon**. Let Flax compile it, start the link, then choose Tools → Heron Brush: attach live map in the open Flax scene. The generated root keeps its placement and other objects are untouched. Save the scene yourself. Updates pause in Play Mode. Tools → Heron Brush: stop live map pauses reception.

# Collaborative live sessions
Collaboration → Live session opens a window for hosting or joining another Heron Brush user. This shares map editing between people; it is separate from an engine live link.

## Host and join
Enter **Your name**. To host, choose **Host a session**, click **Start session** and send the details from **Copy connection details** to your guests. To join, choose **Join a session**, paste the details and click **Join**. Joining replaces the open map with the host's map, so save your work first.

The standard build works on the local network, and **Local network port** sets the port. Sessions over the internet (**Connect over the internet**) need a build with the optional networking feature. Participants see each other's camera markers and names in the 3D view.

Closing the window keeps the session running; the title bar indicator shows its state. **Stop session** or **Leave session** ends it. When two people edit the same object, the host's order decides the final version, and undo is a shared edit too. Work on separate areas where you can, and save checkpoints.

# 3D display and a second monitor
## Display modes
The Map tab's **View** section switches what the views draw: Brushes, Point entities, Entity models, Entity classnames, Group outlines, Patches, Materials, Show omitted faces, Preview animations, Wireframe, Edges and Brush entity outlines. Wireframe draws triangle edges, including diagonals and edges behind surfaces, without materials. It is a temporary debugging view and does not affect exports or renders.

Lighting, shadows, ambient occlusion, reflections, bloom and smoothing for the 3D view are in Preferences → Renderer. These are preview settings; final images use the render camera's own settings.

## A separate 3D window
View → 3D view in separate window moves the 3D view into a window of its own, which you can put on another monitor. It shares the camera, the document and the tools, and the flat views take the space in the main window. Its menu offers Materials, Solid view and Wireframe; a View submenu with Show omitted faces, Preview animations, Edges, Group outlines, Point entities, Entity models and Brush entity outlines; Focus on selection; Front, Side and Top, which turn the camera around the selection (or a point ahead of the camera); Place camera from 3D view; Full screen (`F11`); Render image; and Return to main window. Closing the window docks the view again.

## Procedural sky
The procedural sky is on by default in the 3D view and can be switched off in Preferences → Renderer → Light. Render cameras have their own **Procedural sky** switch, saved with the map. Sky, horizon and ground colours come from the environment settings, and the sun follows the scene lighting. The sky has no geometry and is not exported.

# Render cameras and PNG images
File → Render image opens the render window: the camera and its settings on the left and a live preview on the right that redraws as you change them. Cameras are saved in the `.map` in an editor-only Render cameras layer that is left out of engine exports.

## Placing the camera
Frame the shot in the 3D view and press `Ctrl+Alt+0` (or `Ctrl+Alt+Num0`), or click **Place camera from 3D view**. If no camera is selected, the first saved one is updated; if none exists, one is created. This can be undone and keeps the output and lighting settings. **Select camera in map**, **Load camera from map** and **Move 3D view to camera** work the other way round. **Camera frame in 3D view** shows the output boundary, and **Control camera from 3D view** lets normal navigation move the saved camera.

## Settings
- **Camera position**: X, Y, Z, yaw and pitch.
- **Output and lens**: **Presets** (Full HD, Square, Portrait), width, height, samples, **Vertical field of view** and **Far clip**.
- **Lighting**: Daylight, Sunset or Overcast, or set exposure, the sun, sky, horizon and ground colours, **Procedural sky**, **Shadows**, **Ambient occlusion** and **Bloom**.
- **Depth of field**: **Enable depth of field**, focus distance, sharp range and blur radius. **Focus on selection** focuses on the centre of a selected object in front of the camera.

**Save camera and settings** stores the form in the map; save the map to keep it on disk.

## Rendering
**Render PNG** asks where to save and starts a progressive render, which fills in sample by sample in the preview. More samples look better but take longer. **Cancel render** stops without writing a file. The renderer uses the editor's raster pipeline, not path tracing. The PNG has an opaque background, and animations are not rendered. Very large images can briefly pause the editor.

# Preferences
File → Preferences (`Ctrl+,`) opens the settings. Changes apply immediately and are saved. The sidebar lists Overview, Games, View, Colours, Mouse, Keyboard and Renderer. The search field at the top finds a setting by name or description on any page. Each changed setting has its own button to restore its default, and **Restore defaults** resets the current page.

## The pages
- **Overview**: the common settings together: Appearance (Panel opacity (%), Colour scheme, Layout, Language), Editing, View and Manual.
- **Games**: the game, where its files are, and your collaboration name. A game says which map format to write, which entity definitions to load and where its materials are. Game configurations are read from the `games` folder in the configuration folder (`~/.config/heron-brush/games` on Linux and macOS, `%LOCALAPPDATA%\heron-brush\games` on Windows) and from the `games` folder beside the application, where packages put the bundled one.
- **View**: Layout, Colour scheme, Language, world axes, grid opacity, leak-check cell size and the mapping locks.
- **Colours**: the colours of the views, geometry and handles.
- **Mouse**: look, pan and move speeds and inversions, fly speed, field of view, Link 2D views and handle sizes.
- **Keyboard**: the keymap, Heron Brush or Blender.
- **Renderer**: realistic shading, selection outline, light, shadows, ambient occlusion, reflections, bloom and surfaces.

## Language
**Language** offers Follow the desktop and every translated language. The interface changes at once. The manual opens in the interface language when a translation of it exists, and in English otherwise.

# Plugins and MCP
## Plugins
Plugins extend the editor with Rhai scripts. File → Plugins opens the Plugins window. **Install a plugin** installs a `.rhai` file and **Install a plugin folder** installs a folder containing `main.rhai`; optional `plugin.toml` metadata describes the plugin and its settings. Plugins are grouped under **Active plugins** and **Inactive plugins**, and **Enable plugin** switches one on or off; this survives a restart. **Reload plugins** reads the scripts from disk again and resets their panel values.

Active plugins also appear in the Map tab's **Plugins** section with their settings, action buttons (**Run**) and output. An action works on the selection, or on everything when nothing is selected, and is one undo step. The bundled Selection inspector shows counts and move-up and move-down actions.

## MCP
Collaboration → Serve for an agent (MCP) lets an AI agent or another tool drive the open editor over the Model Context Protocol. The window shows the local address with **Copy**, and **Stop server** ends it; while it runs, the title bar shows **MCP active**. The `--mcp` command line option runs the same interface without a window. Clients can discover tools for geometry, assets, properties, animation, export and sessions. MCP and editor plugins are different interfaces.

# Command line
The program is `heron-brush`. Without options it opens the welcome screen.

## Opening files
`heron-brush map.map [entities.fgd] [materials/]` opens a map, optionally with entity definitions and a material folder. An empty argument skips one, for example `heron-brush map.map "" materials/`. If an editor is already running, the map opens in it; `--new-instance` opens a separate window instead.

## Commands
- `--help` or `-h`: print the usage. `--version` or `-V`: print the version.
- `--mcp`: serve MCP over standard input and output, without a window.
- `--merge-map <base> <ours> <theirs> [<output>]`: a three-way merge of `.map` files for git. The output defaults to ours. Conflicts are printed and give a non-zero exit code.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: turn a mesh into brushes. Size is the length of the longest side in map units (default 256), snap is a grid for the corners (0 for none), and the words `solid` or `layered` build grid boxes or convex slabs instead of thin wedges.
- `--render-preview <in.map> <out.png> [size]`: write a PNG thumbnail of a map, 512 pixels by default.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: run a plugin script over a map with everything selected and write the result (by default back to the input). `key=value` answers the plugin's settings.

# Troubleshooting
## When something looks wrong
- Missing material: check the material folder or package and the export's project paths.
- Wrong model size: compare against the 64-unit player marker and the export scale.
- Wrong gradient direction: check the gradient angle and the face's orientation.
- Empty engine scene: read the export warnings and follow the target's import instructions.
- Live link waiting: check that the addon is installed, the scene is attached and updates are not paused.
- Only lines, no faces: switch Wireframe off in the Map tab's View section, or choose Materials or Solid view in the 3D window's menu.
- Objects missing from the views: `Ctrl+Shift+I` shows hidden objects; check the layers and the View filters.
- A key does nothing: click in the view first so it has the keyboard, and check Preferences → Keyboard for the keymap.
- An edit failed: undo, simplify the geometry, and use **Check map** and **Check for leaks** in the Map tab's **Issues** section.
- A map will not open from the file manager: it may have been handed to an editor that is already running; look for that window.

## Settings and backups
Preferences, recent maps, stars, export settings and plugins live in the configuration folder: `~/.config/heron-brush` on Linux and macOS, and `%LOCALAPPDATA%\heron-brush` on Windows. Autosave backups are in the `autosave` folder next to each map.

## Reporting a bug
Include the editor version (Help → About), your operating system, a small map that shows the problem and the exact steps. The output pane (`Ctrl+4`) often says what went wrong. Save a copy before experimenting.

## Using this manual
Choose a chapter in the contents on the left. `Ctrl+F` shows the contents and moves to the search field; `Esc` clears the search, or closes the manual when the search is empty. **Contents** hides or shows the chapter list and **Text size** changes the font size; both are remembered. The language list at the top switches between the available translations.


Desktop File → Build and run stores ordered export/copy/run profiles beside project export settings. Preferences → Keyboard → Edit shortcuts edits 30 common commands with conflict checks. Issue rows offer Hide and deterministic UV/property fixes with Undo. Help → Application updates checks the planned MojzisekJan/HeronBrush repository; startup checks are off by default. See [Build, launch, shortcuts and updates](../build-and-updates.md) for arguments, variables, limits and release packaging.

Desktop Geometry → Import model, or dropping an OBJ/GLB, inserts a static model entity. Saved maps receive copied assets beside the map, and Save as carries them into the new folder. Edit origin, angles and modelscale in the entity inspector. See [Static model import](../model-import.md) for formats, export behavior and current limitations.
