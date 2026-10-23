#!/usr/bin/env python3
"""Builds heron_keep.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A hall and a chapel joined by a door that a button opens. A trigger in the chapel's
doorway wakes the imps, the archer and the spider waiting there, and the chapel's lights come
up; steps lead to a dais at the far end, where the exit sends the player to the demo's
first map. Textures come from h2demo.wad, which build.sh gathers from the demo's own
maps into wads/ first, so the map names it relative to its own folder.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences name
Hexen 2 and its game path: Heron reads the palette out of the demo's pak0.pak.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "heron_keep.map")
# The same map in Hexen II's own dialect, which Raven's qbsp needs; ericw's reads both.
HEXEN2_MAP = os.path.join(HERE, "heron_keep_hexen2.map")
FGD = os.path.join(HERE, "..", "..", "gamedata", "Hexen2", "Hexen2.fgd")
WAD = os.path.join(HERE, "wads", "h2demo.wad")

# Raven named its textures by number; these are from the demo's castle maps.
WALL = "rtex225"  # grey castle stone
FLOOR = "rtex343"  # grey flagstones
CEILING = "rtex041"  # dark planks
TRIM = "rtex067"  # pale castle blocks, for the dais and the steps
DOOR = "rtex367"  # an iron-bound wooden door
BUTTON = "+0pyr"  # the triangle switch; +apyr is its pressed frame
EXIT = "rtex499"  # a painted panel, behind the exit
TRIGGER = "rtex114"  # Raven's own TRIGGER texture; triggers are not drawn

# Line numbers in the demo's strings.txt: Hexen II messages are numbers, not text.
LEVEL_NAME = "125"  # "The Forgotten Chapel"
DOOR_OPENS = "20"  # "You hear a door across the room open"


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
    # Saved once first, so the wad is named relative to the map's folder.
    heron.call("save", path=MAP)
    print(heron.call("load_wad", path=WAD))
    for key, value in [("message", LEVEL_NAME), ("worldtype", "0"), ("CD", "3")]:
        heron.call("set_entity_property", entity="worldspawn", key=key, value=value)

    # The hall, the start: 512 x 512 x 192 inside.
    heron.box([-16, -16, -16], [528, 528, 0], FLOOR)
    heron.box([-16, -16, 192], [528, 528, 208], CEILING)
    heron.box([-16, -16, 0], [0, 528, 192], WALL)
    heron.box([0, -16, 0], [512, 0, 192], WALL)
    heron.box([0, 512, 0], [512, 528, 192], WALL)

    # The wall between the rooms, with a doorway 64 wide and 128 high.
    heron.box([512, -80, 0], [528, 224, 256], WALL)
    heron.box([512, 288, 0], [528, 592, 256], WALL)
    heron.box([512, 224, 128], [528, 288, 256], WALL)

    # The chapel: 640 x 640 x 256 inside, with a dais 64 high on its east side and
    # four steps of 16 up to it.
    heron.box([512, -80, -16], [1184, 592, 0], FLOOR)
    heron.box([512, -80, 256], [1184, 592, 272], CEILING)
    heron.box([528, -80, 0], [1168, -64, 256], WALL)
    heron.box([528, 576, 0], [1168, 576 + 16, 256], WALL)
    heron.box([1168, -80, 0], [1184, 592, 256], WALL)
    heron.box([1008, -64, 0], [1168, 576, 64], TRIM)
    for step in range(3):
        x = 912 + 32 * step
        heron.box([x, 160, 0], [x + 32, 352, 16 * (step + 1)], TRIM)
    # A painted panel on the east wall marks the way out.
    heron.box([1160, 192, 80], [1168, 320, 208], EXIT)

    # The door only opens when the button fires it, and then stays open.
    heron.brush_entity(
        "func_door", [512, 224, 0], [528, 288, 128], DOOR,
        targetname="door1", angle="-1", wait="-1", speed="100", soundtype="5",
    )
    heron.brush_entity(
        "func_button", [504, 320, 48], [512, 352, 80], BUTTON,
        target="door1", angle="180", wait="-1", message=DOOR_OPENS,
    )

    # Walking into the chapel wakes the ambush and brings its lights up. In Hexen II a
    # targeted light with a style of 32 or more fades that style from lightvalue1 to
    # lightvalue2 when it is fired; these start low (spawnflag 1), so the chapel is
    # dark but for the lamp over the dais until then.
    heron.brush_entity(
        "trigger_once", [544, 160, 0], [608, 352, 128], TRIGGER, target="ambush",
    )
    for origin in [[640, 0, 224], [640, 512, 224], [896, 0, 224], [896, 512, 224], [752, 256, 216]]:
        heron.entity(
            "light", origin, light="350", style="33", spawnflags="1",
            targetname="ambush", lightvalue1="0", lightvalue2="11", fadespeed="1",
        )
    heron.entity("monster_imp_fire", [800, 64, 96], angle="180", targetname="ambush")
    heron.entity("monster_imp_fire", [800, 448, 96], angle="180", targetname="ambush")
    heron.entity("monster_spider_red_small", [720, 256, 0], angle="180", targetname="ambush")
    heron.entity("monster_archer", [1088, 448, 64], angle="180", targetname="ambush")

    # The exit, on the dais in front of the sun disc.
    heron.brush_entity(
        "trigger_changelevel", [1104, 192, 64], [1152, 320, 160], TRIGGER, map="demo1",
    )

    heron.entity("info_player_start", [96, 336, 24], angle="0")
    heron.entity("wp_weapon2", [256, 336, 24])
    heron.entity("item_mana_blue", [256, 384, 24])
    heron.entity("item_mana_green", [320, 384, 24])
    heron.entity("art_HealthBoost", [400, 256, 24])
    heron.entity("item_armor_helmet", [1072, 64, 88])
    heron.entity("item_health", [704, 0, 24])

    # Torches on the hall's walls and a lamp over the dais.
    for origin in [[16, 128, 112], [16, 384, 112], [256, 16, 112], [256, 496, 112]]:
        heron.entity("light_torch_castle", origin, light="250")
    for origin in [[128, 256, 160], [384, 256, 160]]:
        heron.entity("light", origin, light="300")
    heron.entity("light", [1088, 256, 224], light="350")

    print(heron.call("save", path=MAP))
    print(heron.call("export_map", path=HEXEN2_MAP, format="Hexen2"))
    heron.close()


if __name__ == "__main__":
    main()
