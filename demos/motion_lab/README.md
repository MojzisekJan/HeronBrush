# Motion Lab

A small playable map for keyframe brush animation in Godot. The geometry, colours,
keyframes and button wiring all come from `motion_lab.map`; the Godot project only adds
the player, lights and labels.

Open `godot/project.godot` in Godot 4.7 and press F5. WASD and the mouse move, Space
jumps, Escape releases the mouse, 1 to 4 jump to a station and R restarts it. Step on a
coloured pad to trigger its object: a shuttle platform, a lift, a rotating platform and a
door.

## Edit and export

Edit `motion_lab.map` with `motion_lab.fgd` loaded, then run
`python3 build.py --export-only` (`--binary` picks another editor build). Without
`--export-only` the script generates the map again and overwrites your edits.

## Verify

```sh
godot --headless --editor --path godot --import --quit
godot --headless --path godot --fixed-fps 60 --script verify.gd
```

The check rides every platform with the real exported bodies and the game's player
controller, and prints `HERON_MOTION_LAB_OK` on success.
