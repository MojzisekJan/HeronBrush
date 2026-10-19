"""Writes demo.map beside this script: one room with a demo of everything."""

OUT = str(__import__("pathlib").Path(__file__).with_name("demo.map"))
parts = []


def box(x0, y0, z0, x1, y1, z1, tex, top=None):
    """An axis aligned brush. `top` textures the +z face differently."""
    t = tex if top is None else top
    return """{
( %d %d %d ) ( %d %d %d ) ( %d %d %d ) %s [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( %d %d %d ) ( %d %d %d ) ( %d %d %d ) %s [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( %d %d %d ) ( %d %d %d ) ( %d %d %d ) %s [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( %d %d %d ) ( %d %d %d ) ( %d %d %d ) %s [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( %d %d %d ) ( %d %d %d ) ( %d %d %d ) %s [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( %d %d %d ) ( %d %d %d ) ( %d %d %d ) %s [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}""" % (
        x0, y0, z0, x0, y0 + 1, z0, x0, y0, z0 + 1, tex,
        x0, y0, z0, x0, y0, z0 + 1, x0 + 1, y0, z0, tex,
        x0, y0, z0, x0 + 1, y0, z0, x0, y0 + 1, z0, tex,
        x0, y0, z1, x0, y0 + 1, z1, x0 + 1, y0, z1, t,
        x0, y1, z0, x0 + 1, y1, z0, x0, y1, z0 + 1, tex,
        x1, y0, z0, x1, y0, z0 + 1, x1, y0 + 1, z0, tex)


def entity(classname, props=None, brushes=()):
    lines = ["{", '"classname" "%s"' % classname]
    for key, value in (props or {}).items():
        lines.append('"%s" "%s"' % (key, value))
    lines.extend(brushes)
    lines.append("}")
    parts.append("\n".join(lines))


# The room: 1024 by 768, three metres and a bit high, with a pool cut out of
# the floor and a ledge along the north end.
X0, X1 = -512, 512
Y0, Y1 = -384, 384
CEIL = 320
POOL = (192, -128, 448, 128)          # x0 y0 x1 y1 of the hole
LEDGE_Y = 192                          # everything north of this is 96 up

world = []
# Floor in four slabs so the pool has somewhere to be.
world.append(box(X0, Y0, -16, POOL[0], Y1, 0, "concrete", top="grass"))
world.append(box(POOL[2], Y0, -16, X1, Y1, 0, "concrete", top="grass"))
world.append(box(POOL[0], Y0, -16, POOL[2], POOL[1], 0, "concrete", top="grass"))
world.append(box(POOL[0], POOL[3], -16, POOL[2], Y1, 0, "concrete", top="grass"))
# Pool basin.
world.append(box(POOL[0], POOL[1], -112, POOL[2], POOL[3], -96, "dirt"))
for a, b, c, d in [(POOL[0] - 16, POOL[1] - 16, POOL[0], POOL[3] + 16),
                   (POOL[2], POOL[1] - 16, POOL[2] + 16, POOL[3] + 16),
                   (POOL[0], POOL[1] - 16, POOL[2], POOL[1]),
                   (POOL[0], POOL[3], POOL[2], POOL[3] + 16)]:
    world.append(box(a, b, -112, c, d, -16, "dirt"))
# Walls and ceiling.
world.append(box(X0 - 16, Y0 - 16, 0, X0, Y1 + 16, CEIL, "wall"))
world.append(box(X1, Y0 - 16, 0, X1 + 16, Y1 + 16, CEIL, "wall"))
world.append(box(X0, Y0 - 16, 0, X1, Y0, CEIL, "wall"))
world.append(box(X0, Y1, 0, X1, Y1 + 16, CEIL, "wall"))
world.append(box(X0 - 16, Y0 - 16, CEIL, X1 + 16, Y1 + 16, CEIL + 16, "wall"))
# The ledge along the north end, and the wall holding it up.
world.append(box(X0, LEDGE_Y, 80, X1, Y1, 96, "concrete", top="metal"))
world.append(box(X0, LEDGE_Y, 0, -128, LEDGE_Y + 16, 80, "wall"))
world.append(box(64, LEDGE_Y, 0, X1, LEDGE_Y + 16, 80, "wall"))
# A low wall to hide behind, and a step to show step climbing.
world.append(box(-320, -64, 0, -288, 64, 48, "crate"))
world.append(box(-448, 0, 0, -384, 64, 12, "crate"))
world.append(box(-448, 64, 0, -384, 128, 24, "crate"))

entity("worldspawn", {
    "mapversion": "220",
    "_tb_textures": "textures",
    "sky": "1",
    "sky_top_color": "60 110 200",
    "sky_horizon_color": "220 205 180",
    "fog_enabled": "1",
    "fog_light_color": "200 205 220",
    "fog_density": "0.008",
    "ambient_light_color": "175 180 195",
    "ambient_light_energy": "1.0",
    # A room with walls: worth telling the renderer what hides what. On an open
    # level this would cost and save nothing, which is why it is off by default.
    "occluders": "1",
    "occluder_min_size": "96",
}, world)

# Where you start, and what to shoot with.
entity("info_player_start", {"origin": "-448 -320 24", "angles": "0 45 0"})
entity("fps_weapon_pickup", {"origin": "-384 -320 24", "weapon": "blaster-repeater"})
entity("light_sun", {"origin": "0 0 288", "angles": "-50 135 0",
                     "light_energy": "0.8", "light_color": "255 250 235"})
for i, (x, y) in enumerate([(-320, -256), (0, -256), (320, -256), (0, 96), (-256, 288)]):
    entity("light", {"origin": "%d %d 240" % (x, y), "light_energy": "1.6",
                     "omni_range": "12.0", "style": "1" if i == 4 else "0"})

# A row of things to pick up.
for i in range(6):
    props = {"origin": "%d -256 32" % (-416 + i * 48), "value": "1"}
    if i < 3:
        props["kind"] = "present"
    entity("fps_pickup", props)

# Door on a trigger, and door on a button, side by side.
entity("func_door", {"script": "res://scripts/mover.gd", "targetname": "door_trigger", "open_direction": "0 0 1",
                     "travel": "96", "speed": "180", "wait": "3"},
       [box(-224, -320, 0, -128, -288, 96, "metal")])
entity("trigger_multiple", {"target": "door_trigger"},
       [box(-224, -256, 0, -128, -192, 96, "clip")])
entity("func_door", {"script": "res://scripts/mover.gd", "targetname": "door_button", "open_direction": "0 0 1",
                     "travel": "96", "speed": "180", "wait": "5"},
       [box(-64, -320, 0, 32, -288, 96, "metal")])
entity("func_button", {"target": "door_button"},
       [box(-64, -224, 0, 32, -160, 8, "hazard")])

# Panel you look at and press Use, working the same door.
entity("fps_use_button", {"target": "door_button"},
       [box(48, -300, 40, 56, -268, 72, "hazard")])

# Key, and the door that wants it.
entity("fps_key", {"origin": "-448 288 32", "key": "red", "color": "255 60 60"})
entity("fps_key_door", {"targetname": "vault", "key": "red", "open_direction": "0 1 0",
                        "travel": "64", "speed": "120", "wait": "6"},
       [box(384, -352, 0, 448, -288, 96, "metal")])

# A pair of portals: one by the spawn, one on the ledge.
entity("fps_portal", {"targetname": "portal_low", "target": "portal_high",
                      "origin": "-448 -128 32", "angles": "0 0 0"})
entity("fps_portal", {"targetname": "portal_high", "target": "portal_low",
                      "origin": "-448 320 128", "angles": "0 -90 0"})

# Up to the ledge three ways: a pad, a ladder and a lift.
entity("fps_jump_pad", {"strength": "17"}, [box(-96, 96, 0, -32, 160, 8, "hazard")])
entity("fps_ladder", {"targetname": "ladder"}, [box(-32, LEDGE_Y - 32, 0, 32, LEDGE_Y, 96, "skip")])
entity("fps_lift", {"targetname": "lift", "open_direction": "0 0 1", "travel": "96",
                    "speed": "90", "wait": "4"},
       [box(160, 128, 0, 224, 192, 16, "metal")])
entity("trigger_multiple", {"target": "lift"}, [box(160, 64, 0, 224, 120, 64, "clip")])

# Water in the pool. Only the top face is water; the rest is skip.
pool_brush = box(POOL[0], POOL[1], -96, POOL[2], POOL[3], -32, "skip")
pool_brush = pool_brush.replace("( %d %d -32 ) ( %d %d -32 ) ( %d %d -32 ) skip"
                                % (POOL[0], POOL[1], POOL[0], POOL[1] + 1, POOL[0] + 1, POOL[1]),
                                "( %d %d -32 ) ( %d %d -32 ) ( %d %d -32 ) water"
                                % (POOL[0], POOL[1], POOL[0], POOL[1] + 1, POOL[0] + 1, POOL[1]))
entity("fps_water", {"targetname": "pool", "sink_speed": "0.8", "rise_speed": "3.5",
                     "swim_speed": "3.0"}, [pool_brush])

# Lava, and things that go off.
entity("fps_hurt", {"damage": "25", "interval": "0.5", "message": "That burns"},
       [box(224, 224, 96, 352, 352, 144, "hazard")])
for i in range(3):
    entity("fps_barrel", {"origin": "%d -96 0" % (128 + i * 48), "health": "20",
                          "damage": "55", "radius": "4"})
for i in range(2):
    entity("fps_enemy", {"origin": "%d -32 96" % (192 + i * 96), "angles": "0 180 0"})

# Machinery.
entity("func_rotate", {"script": "res://scripts/spinner.gd", "axis": "0 0 1", "speed": "140"},
       [box(-80, -32, 208, 80, 0, 224, "metal")])
entity("func_shuttle", {"script": "res://scripts/shuttle.gd", "targetname": "piston", "direction": "0 1 0", "travel": "160",
                        "speed": "70", "wait": "0.8"},
       [box(288, -32, 0, 352, 32, 16, "crate")])

# Progress: a checkpoint, and a spawner a trigger wakes up.
entity("fps_checkpoint", {"targetname": "halfway"}, [box(-32, 32, 0, 32, 96, 96, "clip")])
entity("fps_spawner", {"targetname": "ambush", "count": "3", "interval": "0.8", "spread": "2.5",
                       "origin": "96 224 128"})
entity("trigger_once", {"target": "ambush"}, [box(64, 160, 96, 160, 192, 192, "clip")])

# Something to look at.
entity("fps_smoke", {"origin": "-288 224 96", "amount": "26", "size": "0.7",
                     "grow": "2.4", "speed": "1.1", "color": "200 200 205"})
entity("fps_particles", {"origin": "352 96 24", "amount": "40", "lifetime": "1.2",
                         "size": "0.1", "speed": "4", "spread": "40",
                         "gravity": "0 -6 0", "color": "255 190 90", "start_on": "1"})
entity("fps_cloud", {"origin": "0 0 288"})
entity("prop_static", {"origin": "448 224 96", "scene": "res://objects/platform.tscn",
                       "scale": "1"})

# A door with nothing pointing at it in the map: the gate below opens it.
entity("func_door", {"script": "res://scripts/mover.gd", "targetname": "side_door", "open_direction": "0 0 1",
                     "travel": "96", "speed": "160", "wait": "0"},
       [box(352, 320, 0, 448, 352, 96, "wood")])

# A sign that says where you are, and a gate that opens the way out once three
# of the presents are in hand. The gate points at the door, and the door has no
# idea why it opened.
entity("fps_sign", {"title": "The ledge", "text": "Three presents and the door opens",
                    "seconds": "4"},
       [box(352, 224, 96, 448, 320, 192, "clip")])
entity("fps_gate", {"targetname": "present_gate", "target": "side_door",
                    "needs_count": "3", "needs_kind": "present",
                    "message": "The door swings open", "origin": "352 128 128"})

# The way out, on the ledge.
entity("fps_exit", {"targetname": "way_out", "next_map": "arena",
                    "message": "Off to the arena"},
       [box(416, 288, 96, 512, 384, 192, "clip")])

header = """// Game: heron_brush
// Format: Valve
// A demo of everything the kit can do, laid out in one room.
// Build and play it with:
//   heron_brush play maps/demo.map
"""
open(OUT, "w").write(header + "\n".join(parts) + "\n")
print("napsano:", OUT, len(parts), "entit")
