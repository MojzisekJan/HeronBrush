# Optional Unity FPS demo kit

This kit supplies example gameplay for the exported FPS demo map. It is opt-in:
ordinary Unity exports keep their geometry, lights, collisions and entity data
without installing a player or changing your game's rules.

## Setup

1. Copy this folder into your project's `Assets/HeronBrushDemo`.
2. Export from Heron Brush with the Unity target and run the generated Build Level
   menu command. Select the resulting map root in the Hierarchy.
3. Choose **Heron Brush → Demo → Create playable scene** and select the exported
   `<map>.gameplay.json` asset. Save the scene and press Play.
4. For a live map, select its **Generated** child and choose its `live.json`.
   Live synchronization remains paused in Play Mode. Leave Play Mode to receive edits.

The `Heron Brush FPS demo` object exposes movement speed, jump speed, mouse
sensitivity, maximum health, initial ammo, shot damage and fire interval.
Use a scene with no other active player camera or AudioListener. To build a game,
add the saved scene to Unity's build scene list; Restart reloads that scene.

## Controls

- WASD: move; mouse: look; Space: jump.
- Left mouse: fire the blaster. Shots consume ammo and damage enemies/barrels.
- E: use a nearby button or door.
- Escape: pause/release the mouse; the UI offers resume, respawn and restart.

Both Unity's legacy Input Manager and the Input System are supported through
Unity's compile-time input symbols.

## Entity behaviour

| Entity | Behaviour / properties |
| --- | --- |
| `info_player_start` | Player spawn and facing, metres from exported placement |
| `func_door`, `fps_lift` | Sliding collider, `open_direction`, `travel`, `speed`, `wait`, optional `key`; closing reopens when blocked by the player |
| `func_shuttle` | Repeating platform with `direction`, `travel`, `speed`, `wait`, `start_on` |
| `func_rotate` | Rotation with map-space `axis`, `speed`, `start_on` |
| `trigger_once`, `trigger_multiple` | Activate resolved targets on player entry; repeated trigger has a short cooldown |
| `fps_use_button` | E activates targets |
| `fps_portal` | Paired target, `color`, rotated velocity/facing, exit offset and cooldown |
| `fps_enemy` | Basic line-of-sight chase/attack, health and hitscan damage; no navigation mesh |
| `fps_barrel` | Health, explosion `damage` and `radius`, chain reactions |
| `fps_spawner` | Target activation spawns enemies using `count`, `interval`, `spread` |
| `fps_pickup`, `fps_key`, `fps_weapon_pickup`, `fps_health` | Inventory, keys, ammo or healing; pickups are consumed once |
| `fps_gate` | `needs_count`, `needs_kind`, `needs_key` unlock targets |
| `fps_checkpoint` | Respawn point |
| `fps_jump_pad` | Launch using `strength` |
| `fps_sign` | Title/text and display duration |
| `fps_exit` | Completion UI with `message`, optional inventory `needs` |

Distances on brush movers are map units, divided by the exported map scale.
Engine-space values such as player speed, jump-pad strength and explosion radius
are metres/seconds as in the demo's entity definitions. Target links come from the
exporter's resolved entity names, never from a global scene search at runtime.

The kit creates simple placeholder models for point entities. It does not import
Godot scenes/scripts, decorative smoke/cloud assets or provide portal render-to-texture
views. Exits show completion; loading `next_map`, flag-based quests, barrel fuse/push
and complex enemy navigation are extension points, not implemented behaviours.

## Not covered

Windows and macOS, URP and HDRP, manual controls with the new Input System, and
complex moving-platform setups.
