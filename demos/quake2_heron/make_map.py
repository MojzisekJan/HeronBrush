#!/usr/bin/env python3
"""Builds heron_outpost.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A hall and a courtyard open to the sky, joined by a door that a button opens. A
trigger in the courtyard's doorway brings in the guards waiting there and switches the
courtyard's lamps on; steps lead up to a platform at the far end, where the exit sends
the player to the demo's first map. A teleporter in the hall's corner puts the player
on the platform. Every texture is in the demo's baseq2/pak0.pak.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences name the
Quake 2 profile and its game path. It writes heron_outpost.map in Heron's own dialect
and heron_outpost_q2.map in Quake 2's, which build.sh compiles.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "heron_outpost.map")
QUAKE2_MAP = os.path.join(HERE, "heron_outpost_q2.map")

FLOOR = "e1u1/floor3_1"
WALL = "e1u1/metal1_2"
CEILING = "e1u1/grate1_1"
TRIM = "e1u1/metal3_1"  # the platform and its steps
LAMP = "e1u1/baselt_3"  # a light panel: its .wal makes it emit 5000
DOOR = "e1u1/dr02_1"
BUTTON = "e1u1/btactmach"
EXIT_SIGN = "e1u1/exit1"
SKY = "e1u1/sky1"  # its .wal says sky and nodraw
TRIGGER = "e1u1/trigger"


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


def hall(heron):
    """The start: 512 x 512 x 192 inside, lit by two ceiling panels and four lights."""
    heron.box([-16, -16, -16], [528, 528, 0], FLOOR)
    heron.box([-16, -16, 192], [528, 528, 208], CEILING)
    heron.box([-16, -16, 0], [0, 528, 192], WALL)
    heron.box([0, -16, 0], [512, 0, 192], WALL)
    heron.box([0, 512, 0], [512, 528, 192], WALL)
    for y in (128, 384):
        heron.box([192, y - 16, 184], [320, y + 16, 192], LAMP)
    for x, y in ((128, 128), (128, 384), (384, 128), (384, 384)):
        heron.entity("light", [x, y, 160], light="200")


def wall_between(heron):
    """The east wall of the hall, with a doorway 64 wide and 112 high."""
    heron.box([512, -80, 0], [528, 224, 320], WALL)
    heron.box([512, 288, 0], [528, 592, 320], WALL)
    heron.box([512, 224, 112], [528, 288, 320], WALL)


def courtyard(heron):
    """640 x 640 inside and 320 high, under the sky, with a platform at the far end."""
    heron.box([512, -80, -16], [1184, 592, 0], FLOOR)
    heron.box([512, -80, 320], [1184, 592, 336], SKY)
    heron.box([528, -80, 0], [1168, -64, 320], WALL)
    heron.box([528, 576, 0], [1168, 592, 320], WALL)
    heron.box([1168, -80, 0], [1184, 592, 320], WALL)
    # The platform, 64 high, and four steps of 16 up to it.
    heron.box([1008, -64, 0], [1168, 576, 64], TRIM)
    for step in range(3):
        x = 912 + 32 * step
        heron.box([x, 160, 0], [x + 32, 352, 16 * (step + 1)], TRIM)
    # The exit sign over the way out. Quake 2 projects a texture on a face looking west
    # with u running north, so the word reads backwards from the courtyard until the
    # brush is mirrored, which with texture lock mirrors its texture too.
    sign = heron.box([1160, 224, 144], [1168, 288, 160], EXIT_SIGN)
    heron.call("select", ids=[sign])
    heron.call("flip_selection", axis="y")


def door_and_button(heron):
    # A door with a name stays shut until something fires it; wait -1 keeps it open.
    heron.brush_entity(
        "func_door", [512, 224, 0], [528, 288, 112], DOOR,
        targetname="door1", angle="-1", wait="-1", speed="100",
    )
    # The button sits on the wall beside the door and is pushed into it (east).
    heron.brush_entity(
        "func_button", [504, 320, 40], [512, 352, 72], BUTTON,
        target="door1", angle="0", wait="-1",
    )


def ambush(heron):
    # Crossing into the courtyard fires everything named "ambush": the guards appear
    # (spawnflag 2, trigger spawn) and the lamps, which start off, come on. ericw's
    # light gives each switchable light its own style from 32 up, which the game code
    # needs before it lets the light be switched.
    heron.brush_entity(
        "trigger_once", [544, 160, 0], [608, 352, 112], TRIGGER,
        target="ambush", message="Intruder alert!",
    )
    for origin in ([672, 0, 288], [672, 512, 288], [928, 0, 288], [928, 512, 288]):
        heron.entity("light", origin, light="350", targetname="ambush", spawnflags="1")
    heron.entity(
        "monster_soldier_light", [800, 64, 24], angle="180",
        targetname="ambush", spawnflags="2",
    )
    heron.entity(
        "monster_soldier", [800, 448, 24], angle="180",
        targetname="ambush", spawnflags="2",
    )
    heron.entity(
        "monster_infantry", [1088, 448, 88], angle="180",
        targetname="ambush", spawnflags="2",
    )


def exit_and_teleporter(heron):
    # Walking into the exit fires the changelevel, which loads the demo's first map.
    heron.brush_entity(
        "trigger_multiple", [1104, 192, 64], [1152, 320, 176], TRIGGER, target="exit",
    )
    heron.entity("target_changelevel", [1120, 256, 200], targetname="exit", map="demo1")
    # A teleporter pad in the hall's north-west corner lands on the platform.
    heron.entity("misc_teleporter", [64, 448, 16], target="pad")
    heron.entity("misc_teleporter_dest", [1056, 448, 80], targetname="pad", angle="180")


def items(heron):
    heron.entity("info_player_start", [96, 256, 24], angle="0")
    heron.entity("weapon_shotgun", [256, 256, 16])
    heron.entity("ammo_shells", [256, 320, 16])
    heron.entity("item_armor_jacket", [384, 64, 16])
    heron.entity("item_health", [384, 448, 16])
    heron.entity("item_health_large", [704, 0, 16])
    heron.entity("ammo_bullets", [1088, 64, 80])
    heron.entity("weapon_machinegun", [1088, 128, 80])


def main():
    heron = Heron(sys.argv[1])
    heron.call("new_map")
    for key, value in [
        ("message", "Heron Outpost"),
        ("sky", "unit1_"),
        # ericw-tools' light: sunlight through the courtyard's sky.
        ("_sunlight", "120"),
        ("_sun_mangle", "45 -60 0"),
        ("_sunlight2", "40"),
    ]:
        heron.call("set_entity_property", entity="worldspawn", key=key, value=value)

    hall(heron)
    wall_between(heron)
    courtyard(heron)
    door_and_button(heron)
    ambush(heron)
    exit_and_teleporter(heron)
    items(heron)

    print(heron.call("save", path=MAP))
    print(heron.call("export_map", path=QUAKE2_MAP, format="quake2"))
    heron.close()


if __name__ == "__main__":
    main()
