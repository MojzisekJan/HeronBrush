#!/usr/bin/env python3
"""Builds heron_lab.map and heron_lab2.map by driving Heron Brush over MCP.

Usage: make_map.py HERON_BRUSH_BINARY

heron_lab: a lab with the HEV suit, a pistol and a scientist, a door that a button on
its wall opens, and a yard open to the desert sky behind it. A trigger in the passage
behind the door switches on the yard's lamp and lets a monstermaker send in two
houndeyes; headcrabs and a zombie are already there. Steps lead up to a platform and a
corridor whose trigger_changelevel takes the player, through the info_landmark
"heron_gate", to heron_lab2: the same corridor, a rotating door on an ORIGIN brush and
a room with a second scientist, whose exit ends the demo the way Uplink's own maps do.

Textures come from uplink.wad, which build.sh gathers from Uplink's own maps into
wads/ first, and the tool textures (CLIP, ORIGIN, AAATRIGGER and NULL) from
SDHLT's sdhlt.wad, also copied there. The maps name both relative to their own folder.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences name the
Half-Life profile and the Uplink folder as its game path.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
WADS = [os.path.join(HERE, "wads", "uplink.wad"), os.path.join(HERE, "wads", "sdhlt.wad")]

LAB_FLOOR = "c1a1_flr1"
LAB_WALL = "c1a0_labw4"
LAB_CEILING = "fifties_ceil01"
LAB_LAMP = "+0~fifts_lght01"  # a texture light: lights.rad gives it its brightness
DOOR = "c1a0b_dr4a"
BUTTON = "+0button2"  # +abutton2 is the lit frame shown once it is pressed
YARD_FLOOR = "crete2_flr02"
YARD_WALL = "c1a4b_w1"
PLATFORM = "c3a1_w5a"
CRATE = "crate10"
EXIT_SIGN = "+0exit"
CORRIDOR = "duct_wall01"
SKY = "sky"
TRIGGER = "aaatrigger"
CLIP = "clip"
ORIGIN = "origin"
NULL = "null"


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

    def box(self, lo, hi, material, kind="cuboid", **extra):
        """One brush; returns its id."""
        text = self.call("add_shape", kind=kind, min=lo, max=hi, material=material, **extra)
        return json.loads(text.split("selected: ", 1)[1])[0]

    def face(self, brush, facing, material):
        self.call("set_face_material", brush=brush, facing=facing, material=material)

    def entity(self, classname, origin, **properties):
        text = self.call(
            "add_point_entity", classname=classname, origin=origin, properties=properties
        )
        return text.rsplit("id ", 1)[1].rstrip(")")

    def brush_entity(self, classname, brushes, **properties):
        self.call("select", ids=brushes)
        text = self.call("make_brush_entity", classname=classname)
        entity = text.rsplit("id ", 1)[1].rstrip(")")
        for key, value in properties.items():
            self.call("set_entity_property", entity=entity, key=key, value=value)
        return entity

    def new_map(self, path, message):
        self.call("new_map")
        # Saved once first, so the wads are named relative to the map's folder, which is
        # where hlcsg is run.
        self.call("save", path=path)
        for wad in WADS:
            print(self.call("load_wad", path=wad))
        for key, value in [("message", message), ("skyname", "desert")]:
            self.call("set_entity_property", entity="worldspawn", key=key, value=value)

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


def lab(heron):
    """The start: 512 x 384 inside and 144 high, lit by four ceiling lamps."""
    heron.box([-16, -16, -16], [528, 400, 0], LAB_FLOOR)
    heron.box([-16, -16, 144], [528, 400, 160], LAB_CEILING)
    heron.box([-16, 0, 0], [0, 384, 144], LAB_WALL)
    heron.box([0, -16, 0], [512, 0, 144], LAB_WALL)
    heron.box([0, 384, 0], [512, 400, 144], LAB_WALL)
    for x in (128, 384):
        for y in (96, 288):
            heron.box([x - 32, y - 16, 140], [x + 32, y + 16, 144], LAB_LAMP)
    heron.entity("light", [256, 192, 112], _light="255 240 220 120")


def wall_between(heron):
    """The yard's west wall, which is also the lab's east wall, with a 64 wide doorway."""
    heron.box([512, -144, 0], [528, 160, 336], YARD_WALL)
    heron.box([512, 224, 0], [528, 528, 336], YARD_WALL)
    heron.box([512, 160, 112], [528, 224, 336], YARD_WALL)
    # A passage on the yard's side, 128 long, as wide and high as the doorway.
    heron.box([528, 144, 0], [656, 160, 128], YARD_WALL)
    heron.box([528, 224, 0], [656, 240, 128], YARD_WALL)
    heron.box([528, 160, 112], [656, 224, 128], YARD_WALL)


def door_and_button(heron):
    # A door with a name opens only when something fires it: touching it does nothing.
    # angles -90 0 0 moves it up; wait -1 keeps it open; lip 4 leaves its edge showing.
    door = heron.box([512, 160, 0], [528, 224, 112], DOOR)
    heron.brush_entity(
        "func_door", [door],
        targetname="door1", angles="-90 0 0", wait="-1", speed="80", lip="4",
    )
    # The button sits on the lab's side of the wall, north of the door. The player
    # presses it with +use; it moves east, into the wall, and stays in (wait -1).
    button = heron.box([504, 256, 48], [512, 288, 80], BUTTON)
    heron.brush_entity("func_button", [button], target="door1", angles="0 0 0", wait="-1")


def yard(heron):
    """576 x 640 inside and 320 high, open to the sky, with a platform at the east end."""
    heron.box([512, -144, -16], [1120, 528, 0], YARD_FLOOR)
    heron.box([512, -144, 320], [1120, 528, 336], SKY)
    heron.box([528, -144, 0], [1104, -128, 320], YARD_WALL)
    heron.box([528, 512, 0], [1104, 528, 320], YARD_WALL)
    # The east wall, with an opening 64 wide over the platform.
    heron.box([1104, -144, 0], [1120, 144, 336], YARD_WALL)
    heron.box([1104, 208, 0], [1120, 528, 336], YARD_WALL)
    heron.box([1104, 144, 176], [1120, 208, 336], YARD_WALL)
    heron.box([1104, 144, 0], [1120, 208, 48], YARD_WALL)
    # The platform, 48 high, and three steps of 16 up to it. A CLIP wedge over the
    # steps makes them a ramp to walk on, so the player does not bob up them.
    heron.box([960, -128, 0], [1104, 512, 48], PLATFORM)
    for step in range(3):
        x = 912 + 16 * step
        heron.box([x, 112, 0], [x + 16, 240, 16 * (step + 1)], PLATFORM)
    heron.box([912, 112, 0], [960, 240, 48], CLIP, kind="wedge", axis="x")
    # Crates, as func_detail: VHLT keeps detail brushes out of the vis tree.
    crates = [
        heron.box([736, -96, 0], [800, -32, 64], CRATE),
        heron.box([800, -96, 0], [864, -32, 64], CRATE),
        heron.box([768, -96, 64], [832, -32, 128], CRATE),
        heron.box([640, 400, 0], [704, 464, 64], CRATE),
    ]
    heron.brush_entity("func_detail", crates)
    # The exit sign over the opening, facing the yard.
    heron.box([1096, 152, 184], [1104, 200, 200], EXIT_SIGN)
    # The sun: light_environment lights every sky face, and gives the sky's colour to
    # the faces in the open.
    heron.entity(
        "light_environment", [960, 400, 256],
        _light="255 236 200 220", angles="0 210 0", pitch="-55",
    )


def ambush(heron):
    # Walking through the passage fires "ambush": the lamp over the platform, which starts
    # dark, comes on, and the monstermaker sends in two houndeyes, a second apart.
    trigger = heron.box([560, 160, 0], [624, 224, 112], TRIGGER)
    heron.brush_entity("trigger_once", [trigger], target="ambush")
    heron.entity("light", [1024, 192, 160], targetname="ambush", spawnflags="1",
                 _light="255 200 120 300")
    heron.entity(
        "monstermaker", [1024, 384, 64], targetname="ambush",
        monstertype="monster_houndeye", monstercount="2", delay="1",
    )
    heron.entity("monster_headcrab", [800, 320, 8], angles="0 180 0")
    heron.entity("monster_headcrab", [864, 400, 8], angles="0 180 0")
    heron.entity("monster_zombie", [1024, 64, 48], angles="0 180 0")


def exit_corridor(heron):
    """A corridor beyond the opening, 64 wide; its far half is the level change."""
    heron.box([1120, 128, 32], [1264, 224, 48], CORRIDOR)
    heron.box([1120, 128, 176], [1264, 224, 192], CORRIDOR)
    heron.box([1120, 128, 48], [1264, 144, 176], CORRIDOR)
    heron.box([1120, 208, 48], [1264, 224, 176], CORRIDOR)
    heron.box([1248, 144, 48], [1264, 208, 176], CORRIDOR)
    heron.entity("light", [1184, 176, 160], _light="255 255 255 80")
    trigger = heron.box([1184, 144, 48], [1248, 208, 176], TRIGGER)
    heron.brush_entity(
        "trigger_changelevel", [trigger], map="heron_lab2", landmark="heron_gate"
    )
    heron.entity("info_landmark", [1216, 176, 80], targetname="heron_gate")


def lab_items(heron):
    heron.entity("info_player_start", [96, 176, 36], angles="0 0 0")
    heron.entity("item_suit", [192, 176, 0])
    heron.entity("weapon_crowbar", [160, 288, 0])
    heron.entity("weapon_9mmhandgun", [288, 176, 0])
    heron.entity("ammo_9mmclip", [256, 96, 0])
    heron.entity("item_healthkit", [384, 352, 0])
    heron.entity("item_battery", [416, 352, 0])
    heron.entity("monster_scientist", [400, 96, 0], angles="0 180 0")


def second_map(heron):
    """heron_lab2: the corridor again, around the same landmark, then a room."""
    # The corridor, closed at the west end where the opening was.
    heron.box([1104, 128, 32], [1392, 224, 48], CORRIDOR)
    heron.box([1104, 128, 176], [1392, 224, 192], CORRIDOR)
    heron.box([1104, 128, 48], [1392, 144, 176], CORRIDOR)
    heron.box([1104, 208, 48], [1392, 224, 176], CORRIDOR)
    heron.box([1104, 144, 48], [1120, 208, 176], CORRIDOR)
    heron.entity("info_landmark", [1216, 176, 80], targetname="heron_gate")
    # The way back, which the player never touches (USE Only): the game links two maps
    # only when each has a trigger_changelevel to the other through the same landmark,
    # and says "Can't find connection" otherwise.
    back = heron.box([1120, 144, 48], [1152, 208, 176], TRIGGER)
    heron.brush_entity(
        "trigger_changelevel", [back], map="heron_lab", landmark="heron_gate", spawnflags="2"
    )
    # Where a player starts who loads the map on its own.
    heron.entity("info_player_start", [1168, 176, 84], angles="0 0 0")
    heron.entity("light", [1184, 176, 160], _light="255 255 255 80")
    # The room: 384 x 320 inside, 192 high, with a 64 wide doorway in its west wall.
    shell = [
        (heron.box([1392, 48, 32], [1792, 400, 48], LAB_FLOOR), [0, 0, -1]),
        (heron.box([1392, 48, 240], [1792, 400, 256], LAB_CEILING), [0, 0, 1]),
        (heron.box([1392, 48, 48], [1792, 64, 240], LAB_WALL), [0, -1, 0]),
        (heron.box([1392, 384, 48], [1792, 400, 240], LAB_WALL), [0, 1, 0]),
        (heron.box([1776, 64, 48], [1792, 384, 240], LAB_WALL), [1, 0, 0]),
    ]
    # The room's outside faces are never seen: NULL keeps hlcsg from making faces of
    # them at all.
    for brush, outwards in shell:
        heron.face(brush, outwards, NULL)
    heron.box([1392, 64, 48], [1408, 144, 240], LAB_WALL)
    heron.box([1392, 208, 48], [1408, 384, 240], LAB_WALL)
    heron.box([1392, 144, 160], [1408, 208, 240], LAB_WALL)
    for x in (1504, 1664):
        heron.box([x - 32, 208, 236], [x + 32, 240, 240], LAB_LAMP)
    # A door on a hinge: the ORIGIN brush at its north edge is what it turns about.
    # It opens away from the player when touched and closes again after four seconds.
    leaf = heron.box([1396, 144, 48], [1404, 200, 160], DOOR)
    hinge = heron.box([1396, 200, 48], [1404, 208, 160], ORIGIN)
    heron.brush_entity(
        "func_door_rotating", [leaf, hinge], distance="90", speed="100", wait="4"
    )
    heron.entity("monster_scientist", [1600, 320, 48], angles="0 180 0")
    heron.entity("item_healthkit", [1600, 96, 48])
    # The end: the same trigger_endsection Uplink's own maps end the demo with.
    trigger = heron.box([1712, 64, 48], [1776, 384, 176], TRIGGER)
    heron.brush_entity("trigger_endsection", [trigger], section="_oem_end_demo")


def main():
    heron = Heron(sys.argv[1])

    path = os.path.join(HERE, "heron_lab.map")
    heron.new_map(path, "Heron Lab")
    lab(heron)
    wall_between(heron)
    door_and_button(heron)
    yard(heron)
    ambush(heron)
    exit_corridor(heron)
    lab_items(heron)
    print(heron.call("save", path=path))

    path = os.path.join(HERE, "heron_lab2.map")
    heron.new_map(path, "Heron Lab, part two")
    second_map(heron)
    print(heron.call("save", path=path))
    heron.close()


if __name__ == "__main__":
    main()
