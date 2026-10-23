#!/usr/bin/env python3
"""Builds heron_court.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A small deathmatch map for OpenArena. A courtyard open to the sky has a raised ledge
along its north side, reached by a jump pad in the middle of the yard or by stairs in
the east corner. A doorway in the south wall leads into a lit room with a teleporter
that puts the player on the ledge. The rocket launcher waits on the ledge, a plasma gun
in the room, a shotgun and a railgun in the yard.

Every brush starts as `common/caulk` and only the faces a player can see get a texture,
which is how Quake 3 maps are made: caulk is drawn nowhere and lit by nothing.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences select
the Quake 3 profile, name the OpenArena folder as the game path and `baseoa` as the
mod: Heron reads the textures, the shaders and the entity definitions from there.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "heron_court.map")
# The same map with Quake 3 brush primitives, the format GtkRadiant and NetRadiant
# write by default; build.sh --primitives compiles this one instead.
PRIMITIVE_MAP = os.path.join(HERE, "heron_court_brushdef.map")

CAULK = "common/caulk"
TRIGGER = "common/trigger"
# The sky shader has q3map_sun and q3map_surfacelight, so the yard is lit by the sky.
SKY = "skies/q3dm0"
FLOOR = "gothic_floor/largerblock3b"
WALL = "gothic_block/blocks18c"
LEDGE_TOP = "gothic_floor/metalbridge06"
LEDGE_WALL = "gothic_block/blocks15"
TRIM = "gothic_trim/baseboard09"
STEP = "gothic_floor/q1metal7_99"
PILLAR = "gothic_trim/metalsupport4b"
ROOM_FLOOR = "base_floor/clang_floor"
ROOM_WALL = "base_wall/basewall01"
ROOM_CEILING = "base_wall/metalfloor_wall_10"
CEILING_LIGHT = "base_light/ceil1_34_5k"
JUMP_PAD = "sfx/bouncepad01_block17"
TELEPORTER = "base_light/ceil1_34_5k"

# The yard: x from -640 to 640, y from -512 to 512, sky at 448.
YARD_X = 640
YARD_Y = 512
SKY_Z = 448
LEDGE_Y = 256
LEDGE_Z = 160
# The room south of the yard.
ROOM_X = 256
ROOM_Y = -896
ROOM_Z = 192
T = 16

EAST = [1, 0, 0]
WEST = [-1, 0, 0]
NORTH = [0, 1, 0]
SOUTH = [0, -1, 0]
UP = [0, 0, 1]
DOWN = [0, 0, -1]


class Heron:
    def __init__(self, binary):
        self.proc = subprocess.Popen(
            [binary, "--mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True
        )
        self.next_id = 0
        self.request("initialize", {})

    def request(self, method, params):
        self.next_id += 1
        line = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}
        self.proc.stdin.write(json.dumps(line) + "\n")
        self.proc.stdin.flush()
        return json.loads(self.proc.stdout.readline())

    def call(self, name, **arguments):
        result = self.request("tools/call", {"name": name, "arguments": arguments})["result"]
        text = " | ".join(part["text"] for part in result["content"])
        if result.get("isError"):
            sys.exit(f"{name} failed: {text}")
        return text

    def shape(self, kind, lo, hi, material, **options):
        """One shape; returns the ids of its brushes."""
        text = self.call("add_shape", kind=kind, min=lo, max=hi, material=material, **options)
        return json.loads(text.split("selected: ", 1)[1])

    def box(self, lo, hi, faces=None, material=CAULK):
        """A cuboid in `material` whose faces in `faces` ({direction: material}) get
        their own; returns its id."""
        brush = self.shape("cuboid", lo, hi, material)[0]
        for facing, wanted in (faces or {}).items():
            self.call("set_face_material", brush=brush, facing=list(facing), material=wanted)
        return brush

    def entity(self, classname, origin, **properties):
        text = self.call(
            "add_point_entity", classname=classname, origin=origin, properties=properties
        )
        return text.rsplit("id ", 1)[1].rstrip(")")

    def brush_entity(self, classname, lo, hi, material, **properties):
        brush = self.box(lo, hi, material=material)
        self.call("select", ids=[brush])
        text = self.call("make_brush_entity", classname=classname)
        entity = text.rsplit("id ", 1)[1].rstrip(")")
        for key, value in properties.items():
            self.call("set_entity_property", entity=entity, key=key, value=value)
        return entity

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


def faces(**by_direction):
    """{direction tuple: material} from keyword arguments named after the directions."""
    names = {"east": EAST, "west": WEST, "north": NORTH, "south": SOUTH, "up": UP, "down": DOWN}
    return {tuple(names[name]): material for name, material in by_direction.items()}


def yard(heron):
    x, y = YARD_X, YARD_Y
    heron.box([-x - T, -y - T, -T], [x + T, y + T, 0], faces(up=FLOOR))
    heron.box([-x - T, -y - T, SKY_Z], [x + T, y + T, SKY_Z + T], faces(down=SKY))
    heron.box([-x - T, -y - T, 0], [-x, y + T, SKY_Z], faces(east=WALL))
    heron.box([x, -y - T, 0], [x + T, y + T, SKY_Z], faces(west=WALL))
    heron.box([-x, y, 0], [x, y + T, SKY_Z], faces(south=WALL))
    # The south wall, with a doorway 128 wide and 128 high into the room. Its south side
    # is the room's north wall.
    door = 64
    heron.box([-x, -y - T, 0], [-door, -y, SKY_Z], faces(north=WALL, east=TRIM, south=ROOM_WALL))
    heron.box([door, -y - T, 0], [x, -y, SKY_Z], faces(north=WALL, west=TRIM, south=ROOM_WALL))
    heron.box(
        [-door, -y - T, 128], [door, -y, SKY_Z], faces(north=WALL, down=TRIM, south=ROOM_WALL)
    )

    # The ledge along the north wall, and stairs up to it in the east corner.
    heron.box([-x, LEDGE_Y, 0], [x, y, LEDGE_Z], faces(up=LEDGE_TOP, south=LEDGE_WALL))
    heron.shape(
        "stairs", [x - 128, LEDGE_Y - 320, 0], [x, LEDGE_Y, LEDGE_Z], STEP, axis="y",
        step_height=16,
    )

    # Two pillars for cover in the south half.
    for cx in (-320, 320):
        heron.box([cx - 32, -224, 0], [cx + 32, -160, 256], material=PILLAR)

    # The jump pad: a glowing plate and a trigger_push above it that throws the player
    # at the target_position over the ledge.
    heron.box([-64, 32, 0], [64, 160, 8], faces(up=JUMP_PAD, north=TRIM, south=TRIM,
                                                 east=TRIM, west=TRIM))
    heron.brush_entity("trigger_push", [-48, 48, 8], [48, 144, 40], TRIGGER, target="ledge_apex")
    heron.entity("target_position", [0, 384, LEDGE_Z + 96], targetname="ledge_apex")


def room(heron):
    x, y0, y1 = ROOM_X, ROOM_Y, -YARD_Y - T
    heron.box([-x - T, y0 - T, -T], [x + T, y1, 0], faces(up=ROOM_FLOOR))
    heron.box([-x - T, y0 - T, ROOM_Z], [x + T, y1, ROOM_Z + T], faces(down=ROOM_CEILING))
    heron.box([-x - T, y0 - T, 0], [-x, y1, ROOM_Z], faces(east=ROOM_WALL))
    heron.box([x, y0 - T, 0], [x + T, y1, ROOM_Z], faces(west=ROOM_WALL))
    heron.box([-x, y0 - T, 0], [x, y0, ROOM_Z], faces(north=ROOM_WALL))
    # Light panels set into the ceiling; the shader gives off the light.
    for cx in (-160, 160):
        heron.box([cx - 32, -736, ROOM_Z - 8], [cx + 32, -672, ROOM_Z],
                  faces(down=CEILING_LIGHT, east=TRIM, west=TRIM, north=TRIM, south=TRIM))

    # The teleporter in the far end: a glowing plate, and the trigger that sends the
    # player to the destination on the ledge.
    heron.box([-48, y0 + 16, 0], [48, y0 + 112, 8],
              faces(up=TELEPORTER, north=TRIM, east=TRIM, west=TRIM))
    heron.brush_entity(
        "trigger_teleport", [-40, y0 + 24, 8], [40, y0 + 104, 72], TRIGGER, target="on_ledge"
    )
    heron.entity("misc_teleporter_dest", [-512, 384, LEDGE_Z + 32], targetname="on_ledge",
                 angle="0")


def things(heron):
    # Spawns, 32 above the floor, facing into the yard.
    for origin, angle in (
        ([-448, -352, 32], "45"),
        ([448, -352, 32], "135"),
        ([-512, 64, 32], "0"),
        ([-160, -640, 32], "90"),
        ([448, 384, LEDGE_Z + 32], "180"),
    ):
        heron.entity("info_player_deathmatch", origin, angle=angle)
    heron.entity("info_player_intermission", [0, -448, 320], angles="20 90 0")

    # Weapons and their ammunition; items stand 16 above the floor and drop onto it.
    heron.entity("weapon_rocketlauncher", [0, 448, LEDGE_Z + 16])
    heron.entity("ammo_rockets", [96, 448, LEDGE_Z + 16])
    heron.entity("weapon_shotgun", [-448, -96, 16])
    heron.entity("ammo_shells", [-384, -96, 16])
    heron.entity("weapon_railgun", [512, -448, 16])
    heron.entity("ammo_slugs", [448, -448, 16])
    heron.entity("weapon_plasmagun", [0, -640, 16])
    heron.entity("ammo_cells", [96, -640, 16])
    heron.entity("item_armor_combat", [0, -256, 16])
    for origin in ([-192, -864, 16], [192, -864, 16], [-512, 448, LEDGE_Z + 16]):
        heron.entity("item_health", origin)
    heron.entity("item_health_large", [448, 64, 16])

    # Point lights in the room and along the yard's walls; the sky lights the rest.
    for cx in (-160, 160):
        for cy in (-832, -608):
            heron.entity("light", [cx, cy, ROOM_Z - 32], light="250")
    for cx in (-448, 0, 448):
        heron.entity("light", [cx, -400, 320], light="300")
    heron.entity("light", [0, 96, 64], light="200", _color="0.4 0.7 1")


def main():
    heron = Heron(sys.argv[1])
    heron.call("new_map")
    heron.call("set_entity_property", entity="worldspawn", key="message",
               value="Heron Brush: Courtyard")
    yard(heron)
    room(heron)
    things(heron)
    print(heron.call("save", path=MAP))
    print(heron.call("export_map", path=PRIMITIVE_MAP, format="quake3_brushdef"))
    heron.close()


if __name__ == "__main__":
    main()
