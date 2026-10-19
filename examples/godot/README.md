# The Godot example project

A small Godot 4 project that exists to answer one question: does what this
editor writes actually work in the engine? It came over from the `heron_brush` project,
which built the same exporter for `.map` levels, so the maps, the textures and the
checks are the ones that exporter was measured against.

## What is in it

`maps/example.map` is the wide test: worldspawn, a door, a physics brush, an
illusionary brush on its own layer, five kinds of light, a trigger, a sound, a
prop that instances another scene, and a player start. `maps/wiring.map` is the
narrow one: a trigger and a button that both drive the same door.

Beside each map is what an export wrote — `example.tscn` and `example/*.obj` —
and those are committed, so the project opens and runs without exporting first.
They are also the reference: a change to the exporter that alters them shows up
as a diff.

## Building the maps

Anything that can drive the editor can do it. Over the MCP server:

    target/debug/heron-brush --mcp

then `open_map` the `.map`, `load_definitions` on `gamedata/heron_brush/heron_brush.fgd`, and
`export` with `engine: "godot"`, `path: "examples/godot/maps"` and the map's
name. The FGD matters: without it every entity falls back to a plain marker,
because what a `func_door` becomes in Godot is something the FGD says with a
`// @heron_brush node=` comment above the class.

## Running the checks

    godot --headless --path examples/godot --editor --quit   # import once
    godot --headless --path examples/godot --script verify.gd
    godot --headless --path examples/godot --script verify_wiring.gd
    godot --headless --path examples/godot --script verify_gameplay.gd

`verify.gd` walks the scene and the mesh: the node class each entity became, the
properties that survived, the collision shape, the material and its texture, the
winding of every triangle, and the texture coordinate of one known corner.
`verify_wiring.gd` calls the door's signals directly. `verify_gameplay.gd` walks
a body into the trigger with real physics running, which is the one that catches
a collision mask that does not line up.

The first two pass. `verify_gameplay.gd` has one failure on Godot 4.7.1 — the
door does not close again inside the frames the script waits — and the same
check fails the same way on a scene built by `heron_brush` itself, so it is the script's
timing against this version of the engine rather than anything the exporter did.
