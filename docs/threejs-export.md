# Three.js playtest export

Choose **three.js** in the export dialog and export to a folder. Open
`<level-name>.html` in a desktop browser, then click **Play**. No build step or
local server is required. The HTML contains the level, GLB meshes and available
PNG/JPEG/WebP/GIF albedo textures. Three.js 0.186.0 loads from jsDelivr, so the
browser needs internet access. The JSON and GLB files are also exported for use
in another project.

- WASD moves; the mouse looks around.
- Space jumps; Shift runs.
- R returns to the starting position; Esc pauses and releases the mouse.
- If mouse capture is unavailable, **Play with drag-to-look** keeps keyboard
  movement and lets you drag the mouse to turn.

Place an `info_player_start` to choose the start location and direction.
`info_player_deathmatch`, `player_start` and `fps_player` are also recognized.
Without one, the player starts above the centre of the level, on the first
surface below it. In enclosed maps this may be the roof, so place a start entity
inside the playable room.

The player collides with exported triangle meshes and convex hulls. Trigger
volumes remain passable. With collision export disabled, the page uses free
flight: Space goes up and left Ctrl goes down. Player dimensions and movement
follow the map's export scale.

This is a geometry playtest. Entity scripts, enemies, doors and trigger actions
are not simulated. Lighting uses a hemisphere light and a directional light;
engine-specific materials and extra texture maps are not interpreted.
