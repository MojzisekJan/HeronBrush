#!/usr/bin/env python3
"""Builds heron_yard.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A small Quetoo deathmatch level: an open hall under the sky with a raised ledge along its
north side, stairs and a jump pad up to the ledge, and a side room through a doorway in
the east wall whose teleporter puts the player back on the ledge. Six spawns, four
weapons with their ammo, health, armour and a mega health. heron_yard_playtest.map is
the same with no bots and two spawns, on the jump pad and on the teleporter, for
playtest.py.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences name the
Quetoo profile and its game path (the folder holding `default/`): new brushes then get
the profile's UV scale of 0.25 and the materials come from `default/textures`.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "heron_yard.map")
# The same level with its spawns on the jump pad and on the teleporter, for playtest.py.
PLAYTEST_MAP = os.path.join(HERE, "heron_yard_playtest.map")

# Quetoo names a material by its path under textures/, without the extension.
WALL = "quake3/base_wall/basewall01"
FLOOR = "quake3/base_floor/diamond2c"
TRIM = "quake3/base_trim/border11"
LEDGE = "quake3/base_trim/pewter"
PAD = "quake3/base_floor/clanggrate"
LAMP = "quake3/base_light/ceil1_30"
STEPS = "quake3/base_floor/clang_floor"
# quemap gives these their flags by name: sky is sky, trigger is not drawn.
SKY = "common/sky"
TRIGGER = "common/trigger"


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

    def shape(self, lo, hi, material, **options):
        """Brushes of one shape; returns their ids."""
        text = self.call("add_shape", min=lo, max=hi, material=material, **options)
        return json.loads(text.split("selected: ", 1)[1])

    def box(self, lo, hi, material, top=None):
        """One cuboid brush, optionally with another material on its top; returns its id."""
        brush = self.shape(lo, hi, material, kind="cuboid")[0]
        if top:
            self.call("set_face_material", brush=str(brush), facing=[0, 0, 1], material=top)
        return brush

    def entity(self, classname, origin, **properties):
        text = self.call(
            "add_point_entity", classname=classname, origin=origin, properties=properties
        )
        return text.rsplit("id ", 1)[1].rstrip(")")

    def brush_entity(self, classname, lo, hi, material, **properties):
        brush = self.box(lo, hi, material)
        self.call("select", ids=[brush])
        text = self.call("make_brush_entity", classname=classname)
        entity = text.rsplit("id ", 1)[1].rstrip(")")
        for key, value in properties.items():
            self.call("set_entity_property", entity=entity, key=key, value=value)
        return entity

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


def main():
    heron = Heron(sys.argv[1])
    heron.call("new_map")
    for key, value in [
        ("message", "Heron Yard"),
        ("sky", "dragonheart"),
        ("ambient", "0.4"),
        ("games", "dm"),
        ("min_clients", "4"),
        ("author", "Heron Brush"),
    ]:
        heron.call("set_entity_property", entity="worldspawn", key=key, value=value)

    # The hall: 1024 x 768 inside, open to the sky 384 up.
    heron.box([-16, -16, -16], [1040, 784, 0], FLOOR)
    heron.box([-16, -16, 384], [1040, 784, 400], SKY)
    heron.box([-16, -16, 0], [0, 784, 384], WALL)
    heron.box([0, -16, 0], [1040, 0, 384], WALL)
    heron.box([0, 768, 0], [1040, 784, 384], WALL)
    # The east wall, with a doorway 128 wide and 128 high into the side room.
    heron.box([1024, 0, 0], [1040, 320, 384], WALL)
    heron.box([1024, 448, 0], [1040, 768, 384], WALL)
    heron.box([1024, 320, 128], [1040, 448, 384], WALL)
    # A trim band around the hall at the ledge's height.
    for lo, hi in [
        ([0, 0, 192], [1024, 8, 208]),
        ([0, 8, 192], [8, 576, 208]),
        ([1016, 8, 192], [1024, 320, 208]),
    ]:
        heron.box(lo, hi, TRIM)

    # The ledge along the north wall, 192 up, and the stairs to it from the east.
    heron.box([0, 576, 0], [640, 768, 192], TRIM, top=LEDGE)
    heron.shape(
        [640, 576, 0], [832, 768, 192], STEPS,
        kind="stairs", axis="x", ascending=False, step_height=16,
    )

    # The jump pad: a grate in the floor and a push just above it, throwing the player
    # north and up onto the ledge. Quetoo's push speed is a tenth of the velocity.
    heron.box([256, 448, 0], [320, 512, 8], PAD)
    heron.brush_entity(
        "trigger_push", [256, 448, 8], [320, 512, 48], TRIGGER,
        angles="-75 90 0", speed="72", spawnflags="2",
    )

    # The side room east of the hall, 512 x 512 and 256 high, lit by two lamps.
    heron.box([1040, 112, -16], [1568, 656, 0], FLOOR)
    heron.box([1040, 112, 256], [1568, 656, 272], WALL)
    heron.box([1040, 112, 0], [1568, 128, 256], WALL)
    heron.box([1040, 640, 0], [1568, 656, 256], WALL)
    heron.box([1552, 128, 0], [1568, 640, 256], WALL)
    for x in [1168, 1424]:
        heron.box([x - 32, 352, 248], [x + 32, 416, 256], LAMP)
        heron.entity("light", [x, 384, 224], radius="400", intensity="1.5")

    # The teleporter against the room's east wall goes back to the west end of the ledge.
    heron.box([1488, 352, 0], [1552, 416, 4], PAD)
    heron.entity("misc_teleporter", [1520, 384, 28], target="ledge")
    heron.entity("misc_teleporter_dest", [96, 672, 216], targetname="ledge", angle="0")

    # Lights in the hall, warm over the ledge.
    for origin in [[256, 192, 288], [768, 192, 288], [512, 448, 320]]:
        heron.entity("light", origin, radius="500", intensity="2")
    for origin in [[160, 672, 320], [480, 672, 320]]:
        heron.entity("light", origin, radius="350", color="1 0.8 0.6")

    spawns = []
    for origin, angle in [
        ([128, 128, 32], "45"),
        ([896, 128, 32], "135"),
        ([512, 320, 32], "90"),
        ([320, 672, 224], "270"),
        ([1296, 224, 32], "180"),
        ([1296, 544, 32], "180"),
    ]:
        spawns.append(heron.entity("info_player_deathmatch", origin, angle=angle))
    heron.entity("info_player_intermission", [512, 64, 320], angles="20 90 0")

    for classname, origin in [
        ("weapon_rocketlauncher", [480, 672, 208]),
        ("ammo_rockets", [544, 672, 208]),
        ("item_armor_jacket", [192, 672, 208]),
        ("weapon_railgun", [1472, 576, 16]),
        ("ammo_slugs", [1408, 576, 16]),
        ("item_health_mega", [1472, 192, 16]),
        ("weapon_shotgun", [512, 128, 16]),
        ("ammo_shells", [576, 128, 16]),
        ("weapon_hyperblaster", [128, 384, 16]),
        ("ammo_cells", [128, 448, 16]),
        ("item_armor_combat", [768, 384, 16]),
        ("item_health", [896, 640, 16]),
        ("item_health", [960, 640, 16]),
        ("item_health", [1168, 224, 16]),
    ]:
        heron.entity(classname, origin)

    print(heron.call("save", path=MAP))

    # The playtest's two spawns stand on the jump pad and on the teleporter, so that
    # each respawn tries one of them without walking there.
    heron.call("select", ids=spawns)
    heron.call("delete_selection")
    heron.entity("info_player_deathmatch", [288, 480, 40], angle="90")
    heron.entity("info_player_deathmatch", [1520, 384, 32], angle="180")
    heron.call("set_entity_property", entity="worldspawn", key="min_clients", value="0")
    print(heron.call("save", path=PLAYTEST_MAP))
    heron.close()


if __name__ == "__main__":
    main()
