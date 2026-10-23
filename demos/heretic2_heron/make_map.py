#!/usr/bin/env python3
"""Builds heron_wharf.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A warehouse and a yard open to the sky, joined by a door that a button opens. A
trigger in the yard's doorway wakes the plague elves and rats waiting there, prints
Corvus's line about his brethren and lights the yard's lamps; steps lead up to a
quay at the far end, where the exit sends the player to the demo's Silverspring
Docks. Every texture is one of the demo's loose .m8 files under Base/textures.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences name the
Heretic 2 profile and the demo's folder as the game path. It writes heron_wharf.map in
Heron's own dialect and heron_wharf_h2.map in Quake 2's, which build.sh compiles.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "heron_wharf.map")
HERETIC2_MAP = os.path.join(HERE, "heron_wharf_h2.map")

# Each .m8 carries its footstep material in bits 24 and 25 of its surface flags: wood
# (3) for the planks, stone (2) for the walls. The engine tells a Heretic II bsp from a
# Quake 2 one by finding both bits among its faces.
FLOOR = "Silverspring/wfloor1"
WALL = "Silverspring/oldwall1"
CEILING = "Silverspring/roof1"
YARD_FLOOR = "Silverspring/cobble13"
YARD_WALL = "Silverspring/abrickbot"
QUAY = "Silverspring/wood1"  # the quay and its steps
DOOR = "Silverspring/door1"
BUTTON = "Silverspring/0button"  # its .m8 names 1button as the next frame
CRATE = "palace/crate1"
SKY = "general/sky"  # its .m8 says sky
TRIGGER = "general/trigger"  # its .m8 says nodraw


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


def warehouse(heron):
    """The start: 512 x 512 x 192 inside, lit by four lights, with crates in a corner.

    q2tools' light needs more than ericw's for the same room: 250 left it nearly black.
    """
    heron.box([-16, -16, -16], [528, 528, 0], FLOOR)
    heron.box([-16, -16, 192], [528, 528, 208], CEILING)
    heron.box([-16, -16, 0], [0, 528, 192], WALL)
    heron.box([0, -16, 0], [512, 0, 192], WALL)
    heron.box([0, 512, 0], [512, 528, 192], WALL)
    for x, y in ((128, 128), (128, 384), (384, 128), (384, 384)):
        heron.entity("light", [x, y, 160], light="450")
    heron.box([32, 32, 0], [96, 96, 64], CRATE)
    heron.box([96, 32, 0], [160, 96, 64], CRATE)
    heron.box([48, 48, 64], [112, 112, 128], CRATE)


def wall_between(heron):
    """The east wall of the warehouse, with a doorway 64 wide and 128 high."""
    heron.box([512, -80, 0], [528, 224, 320], YARD_WALL)
    heron.box([512, 288, 0], [528, 592, 320], YARD_WALL)
    heron.box([512, 224, 128], [528, 288, 320], YARD_WALL)


def yard(heron):
    """640 x 640 inside and 320 high, under the sky, with a quay at the far end."""
    heron.box([512, -80, -16], [1184, 592, 0], YARD_FLOOR)
    heron.box([512, -80, 320], [1184, 592, 336], SKY)
    heron.box([528, -80, 0], [1168, -64, 320], YARD_WALL)
    heron.box([528, 576, 0], [1168, 592, 320], YARD_WALL)
    heron.box([1168, -80, 0], [1184, 592, 320], YARD_WALL)
    # Two dim lights on the quay, so the yard is not black before the lamps come on.
    for y in (64, 448):
        heron.entity("light", [1088, y, 160], light="250")
    # The quay, 64 high, and three steps of 16 up to it.
    heron.box([1008, -64, 0], [1168, 576, 64], QUAY)
    for step in range(3):
        x = 912 + 32 * step
        heron.box([x, 160, 0], [x + 32, 352, 16 * (step + 1)], QUAY)


def door_and_button(heron):
    # A door with a name stays shut until something fires it; angle -1 opens it upwards
    # and wait -1 keeps it open.
    heron.brush_entity(
        "func_door", [512, 224, 0], [528, 288, 128], DOOR,
        targetname="door1", angle="-1", wait="-1", speed="100", sounds="1",
    )
    # The button sits on the wall beside the door. Spawnflag 1 (TOUCH) lets walking
    # into it press it; without it Heretic II wants the action key. It moves east, into
    # the wall.
    heron.brush_entity(
        "func_button", [504, 320, 48], [512, 352, 80], BUTTON,
        target="door1", angle="0", wait="-1", spawnflags="1", sounds="1",
    )


def ambush(heron):
    # Crossing into the yard fires everything named "ambush". Monsters with spawnflag 2
    # (ASLEEP) are not in the level until then. The lamps start off (spawnflag 1) and
    # need a style of 32 or more for the game to switch them; q2tools' light takes the
    # style as written. The message is line 31 of the demo's levelmsg.txt.
    heron.brush_entity(
        "trigger_once", [544, 160, 0], [608, 352, 128], TRIGGER,
        target="ambush", message="31",
    )
    for origin in ([672, 0, 288], [672, 512, 288], [928, 0, 288], [928, 512, 288]):
        heron.entity(
            "light", origin, light="450", targetname="ambush", style="32", spawnflags="1"
        )
    heron.entity("monster_plagueElf", [800, 64, 24], angle="180", targetname="ambush",
                 spawnflags="2")
    heron.entity("monster_plagueElf", [800, 448, 24], angle="180", targetname="ambush",
                 spawnflags="2")
    for y in (192, 256, 320):
        heron.entity("monster_rat", [1088, y, 72], angle="180", targetname="ambush",
                     spawnflags="2")


def exit_and_items(heron):
    # Walking into the exit fires the changelevel, which loads the demo's docks.
    heron.brush_entity(
        "trigger_multiple", [1104, 192, 64], [1152, 320, 192], TRIGGER, target="exit",
    )
    heron.entity("target_changelevel", [1120, 256, 216], targetname="exit", map="demodocks")
    heron.entity("info_player_start", [96, 256, 24], angle="0")
    heron.entity("item_health_half", [256, 448, 16])
    heron.entity("item_ammo_hellstaff", [384, 64, 16])
    heron.entity("item_weapon_hellstaff", [256, 64, 16])
    heron.entity("item_mana_offensive_half", [1088, 64, 80])


def main():
    heron = Heron(sys.argv[1])
    heron.call("new_map")
    heron.call("set_entity_property", entity="worldspawn", key="sky", value="sky1")

    warehouse(heron)
    wall_between(heron)
    yard(heron)
    door_and_button(heron)
    ambush(heron)
    exit_and_items(heron)

    print(heron.call("save", path=MAP))
    print(heron.call("export_map", path=HERETIC2_MAP, format="quake2"))
    heron.close()


if __name__ == "__main__":
    main()
