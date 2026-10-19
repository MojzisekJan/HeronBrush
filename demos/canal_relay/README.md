# Canal Relay

A three-chapter game made with Heron Brush and a custom Godot export profile. Open
`godot/project.godot` in Godot 4 and run it; the exported maps are included.

Collect energy cells, power the relay with F to open the bridge gate, and reach the exit
arch. WASD moves, the mouse looks, Space jumps, Escape releases the mouse and R restarts
the campaign. Progress is saved between runs.

## Edit and rebuild

`courtyard.map`, `pumphouse.map` and `beacon.map` use the game's own entities from
`game.fgd`. `.heron-brush/exports.json` maps them to Godot scripts and scenes; the
gameplay lives in `godot/scripts` and export leaves it alone.

```sh
python3 demos/canal_relay/build.py --output demos/canal_relay/godot
godot --headless --path demos/canal_relay/godot --script verify_campaign.gd
```

`--generate` replaces the maps with the original generated ones.
