#!/usr/bin/env python3
"""Builds libre_lift.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

Two rooms joined by a door that a button opens, an ambush that a trigger wakes, a lift
up to a ledge and the exit. Textures come from LibreQuake's mapping wads, which
build.sh copies into wads/ first, so the map names them relative to its own folder.

Run it with XDG_CONFIG_HOME pointing at a scratch directory: Heron reads its
preferences from there.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "libre_lift.map")
FGD = os.path.join(HERE, "..", "..", "gamedata", "Quake", "Quake.fgd")
WADS = ["lq_medieval.wad", "lq_tech.wad", "lq_utility.wad"]

WALL = "med_dbrick1"
FLOOR = "med_csl_flr2_2"
CEILING = "med_roof3"
LEDGE = "med_ebrick1"


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

    def box(self, lo, hi, material):
        """One cuboid brush; returns its id."""
        text = self.call("add_shape", kind="cuboid", min=lo, max=hi, material=material)
        return json.loads(text.split("selected: ", 1)[1])[0]

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
    heron.call("load_definitions", path=FGD)
    # Saved once first, so the wads are named relative to the map's folder.
    heron.call("save", path=MAP)
    for wad in WADS:
        print(heron.call("load_wad", path=os.path.join(HERE, "wads", wad)))
    for key, value in [
        ("message", "Heron Brush: the lift"),
        ("worldtype", "0"),
        ("sounds", "6"),
    ]:
        heron.call("set_entity_property", entity="worldspawn", key=key, value=value)

    # Room A, the start: 512 x 512 x 192 inside.
    heron.box([-16, -16, -16], [528, 528, 0], FLOOR)
    heron.box([-16, -16, 192], [528, 528, 208], CEILING)
    heron.box([-16, -16, 0], [0, 528, 192], WALL)
    heron.box([0, -16, 0], [512, 0, 192], WALL)
    heron.box([0, 512, 0], [512, 528, 192], WALL)

    # The wall between the rooms, with a doorway 64 wide and 128 high.
    heron.box([512, -80, 0], [528, 224, 256], WALL)
    heron.box([512, 288, 0], [528, 592, 256], WALL)
    heron.box([512, 224, 128], [528, 288, 256], WALL)

    # Room B, the arena: 640 x 640 x 256 inside, with a ledge 128 high on its east side.
    heron.box([512, -80, -16], [1184, 592, 0], FLOOR)
    heron.box([512, -80, 256], [1184, 592, 272], CEILING)
    heron.box([528, -80, 0], [1168, -64, 256], WALL)
    heron.box([528, 576, 0], [1168, 592, 256], WALL)
    heron.box([1168, -80, 0], [1184, 592, 256], WALL)
    heron.box([976, -64, 0], [1168, 576, 128], LEDGE)

    # The door only opens when the button fires it, and then stays open.
    heron.brush_entity(
        "func_door", [512, 224, 0], [528, 288, 128], "med_door3",
        targetname="door1", angle="-1", wait="-1", speed="100",
    )
    heron.brush_entity(
        "func_button", [504, 320, 48], [512, 352, 80], "+0med_but1",
        target="door1", angle="0",
    )

    # Walking into the arena wakes the ambush and switches its light on.
    heron.brush_entity(
        "trigger_once", [544, 160, 0], [608, 352, 128], "trigger",
        target="ambush", message="Ambush!",
    )
    heron.entity("monster_army", [800, 64, 24], angle="180", spawnflags="1", targetname="ambush")
    heron.entity("monster_army", [800, 448, 24], angle="180", spawnflags="1", targetname="ambush")
    heron.entity("monster_dog", [720, 256, 24], angle="180", spawnflags="1", targetname="ambush")
    heron.entity("light", [752, 256, 200], light="300", spawnflags="1", targetname="ambush")

    # The lift rises from the arena floor to the ledge; the exit is on the ledge.
    heron.brush_entity("func_plat", [912, 224, 0], [976, 352, 128], "plat_top1")
    heron.entity("monster_knight", [1104, 64, 152], angle="180")
    heron.brush_entity(
        "trigger_changelevel", [1104, 224, 128], [1152, 352, 224], "trigger", map="start",
    )

    heron.entity("info_player_start", [96, 256, 24], angle="0")
    heron.entity("weapon_supershotgun", [256, 128, 16])
    heron.entity("item_shells", [256, 384, 16])
    heron.entity("item_health", [400, 256, 16])
    heron.entity("item_armor1", [1056, 448, 144])
    heron.entity("item_health", [704, 0, 16])

    for origin in [[128, 128, 160], [384, 128, 160], [128, 384, 160], [384, 384, 160]]:
        heron.entity("light", origin, light="300")
    for origin in [[640, 0, 224], [640, 512, 224], [896, 0, 224], [896, 512, 224]]:
        heron.entity("light", origin, light="350")
    heron.entity("light", [1072, 288, 232], light="350")

    print(heron.call("save", path=MAP))
    heron.close()


if __name__ == "__main__":
    main()
