#!/usr/bin/env python3
"""Builds heron_lift.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A start pad, a ramp up to a deck, a switch that starts a lift across a gap, a shortcut
teleporter and the goal on the far deck. Fifteen coins' worth of coins, ten to open the
goal, and 90 seconds.

The map is saved in Heron's own dialect (Valve 220) and exported in the Quake 2 one,
which is what Neverball's mapc reads: heron_lift.map is the export, and
heron_lift.heron.map is what Heron edits.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences have the
Neverball profile and its data folder as the game path: Heron loads the textures from
there.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SOURCE = os.path.join(HERE, "heron_lift.heron.map")
EXPORT = os.path.join(HERE, "heron_lift.map")
FGD = os.path.join(HERE, "..", "..", "gamedata", "Neverball", "neverball.fgd")

TURF = "mtrl/turf-green-small"
RAMP = "mtrl/turf-green-check"
DECK = "mtrl/turf-green"
SIDE = "mtrl/turf-grey"
WALL = "mtrl/wood-light"
LIFT = "mtrl/wood-check-glossy"
GOAL_DECK = "mtrl/turf-blue"

UP = [0, 0, 1]


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

    def box(self, lo, hi, material, top=None, kind="cuboid", facing=UP, **shape):
        """One brush; returns its id. `top` is the material of its faces along `facing`."""
        text = self.call("add_shape", kind=kind, min=lo, max=hi, material=material, **shape)
        brush = json.loads(text.split("selected: ", 1)[1])[0]
        if top:
            self.call("set_face_material", brush=brush, facing=facing, material=top)
        return brush

    def floor(self, lo, hi, top):
        """A slab whose top is turf and whose sides are grey."""
        return self.box(lo, hi, SIDE, top=top)

    def entity(self, classname, origin, **properties):
        text = self.call(
            "add_point_entity", classname=classname, origin=origin, properties=properties
        )
        return text.rsplit("id ", 1)[1].rstrip(")")

    def brush_entity(self, classname, lo, hi, material, top=None, **properties):
        brush = self.box(lo, hi, material, top=top)
        self.call("select", ids=[brush])
        text = self.call("make_brush_entity", classname=classname)
        entity = text.rsplit("id ", 1)[1].rstrip(")")
        for key, value in properties.items():
            self.call("set_entity_property", entity=entity, key=key, value=value)
        return entity

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


def walls(heron, x0, x1, y0, y1, z0, z1, gaps=()):
    """Walls 8 thick around the rectangle x0..x1, y0..y1, except the sides in `gaps`.

    A gap is (side, a, b): side "front" (y0) or "back" (y1), open from x = a to b.
    """
    heron.box([x0 - 8, y0 - 8, z0], [x0, y1 + 8, z1], WALL)
    heron.box([x1, y0 - 8, z0], [x1 + 8, y1 + 8, z1], WALL)
    for side, y in (("front", y0 - 8), ("back", y1)):
        opening = [gap for gap in gaps if gap[0] == side]
        if not opening:
            heron.box([x0, y, z0], [x1, y + 8, z1], WALL)
            continue
        _, a, b = opening[0]
        if a > x0:
            heron.box([x0, y, z0], [a, y + 8, z1], WALL)
        if b < x1:
            heron.box([b, y, z0], [x1, y + 8, z1], WALL)


def main():
    heron = Heron(sys.argv[1])
    heron.call("new_map")
    heron.call("load_definitions", path=FGD)
    for key, value in [
        ("message", "Built in Heron Brush.\\Roll over the switch to start the lift.\\"
         "The pad in the corner is a shortcut."),
        ("author", "Heron Brush"),
        ("version", "1"),
        ("time", "9000"),
        ("goal", "10"),
        ("back", "map-back/clouds.sol"),
        ("grad", "back/land.png"),
        ("song", "bgm/track1.ogg"),
    ]:
        heron.call("set_entity_property", entity="worldspawn", key=key, value=value)

    # The start pad, 256 square with its top at 0, walled on three sides.
    heron.floor([-128, -128, -16], [128, 128, 0], TURF)
    walls(heron, -128, 128, -128, 128, -16, 32, gaps=[("back", -128, 128)])

    # The ramp climbs 64 over 384 to the deck, on a slab, with a wall each side.
    heron.box([-128, 128, 0], [128, 512, 64], SIDE, top=RAMP, kind="wedge", axis="y",
              facing=[0, -64, 384])
    heron.box([-128, 128, -16], [128, 512, 0], SIDE)
    heron.box([-136, 128, -16], [-128, 512, 96], WALL)
    heron.box([128, 128, -16], [136, 512, 96], WALL)

    # The deck, 512 by 384 at 64. The ramp comes in at the front, and the lift leaves
    # from the middle of the back.
    heron.floor([-256, 512, 48], [256, 896, 64], DECK)
    walls(heron, -256, 256, 512, 896, 48, 96,
          gaps=[("front", -128, 128), ("back", -96, 96)])

    # The lift: a platform that rides between the decks once the switch starts its path.
    # Its brush sits where the path begins, its top at the path_corner's origin. A
    # corner's speed is the seconds to the next one: three to cross, and two corners at
    # each end make it wait there for three. The switch turns the whole loop on.
    heron.brush_entity("func_train", [-96, 896, 48], [96, 1088, 64], LIFT, target="lift_a")
    for name, origin, after, speed in [("lift_a", [0, 992, 64], "lift_b", "3"),
                                       ("lift_b", [0, 1184, 64], "lift_c", "3"),
                                       ("lift_c", [0, 1184, 64], "lift_d", "3"),
                                       ("lift_d", [0, 992, 64], "lift_a", "3")]:
        heron.entity("path_corner", origin, targetname=name, target=after, speed=speed,
                     state="0")
    heron.entity("info_camp", [0, 640, 64], target="lift_a", radius="0.75")

    # The goal deck, 384 across the gap: too far to roll over while the lift is away.
    heron.floor([-256, 1280, 48], [256, 1664, 64], GOAL_DECK)
    walls(heron, -256, 256, 1280, 1664, 48, 96, gaps=[("front", -96, 96)])
    heron.entity("info_player_deathmatch", [0, 1536, 88])

    # The shortcut: a teleporter in the start pad's corner, out onto the deck.
    heron.entity("target_teleporter", [96, -96, 0], target="deck_landing")
    heron.entity("target_position", [160, 704, 64], targetname="deck_landing")

    heron.entity("info_player_start", [0, -64, 24])
    heron.entity("target_position", [0, 704, 64], targetname="view")
    heron.entity("info_player_intermission", [-640, -384, 640], target="view")

    # Coins: ones, and a red five on the deck. The goal wants ten, and the straight line
    # up the middle passes eleven.
    for origin in [[0, 32, 16], [0, 96, 16],
                   [0, 256, 37], [0, 320, 48], [0, 384, 59],
                   [0, 560, 80],
                   [-192, 576, 80], [192, 576, 80], [-192, 832, 80], [192, 832, 80]]:
        heron.entity("light", origin, light="1")
    heron.entity("light", [0, 800, 80], light="5")

    print(heron.call("save", path=SOURCE))
    print(heron.call("export_map", path=EXPORT, format="quake2"))
    heron.close()


if __name__ == "__main__":
    main()
