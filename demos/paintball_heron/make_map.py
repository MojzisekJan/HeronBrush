#!/usr/bin/env python3
"""Builds heron_bases.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A capture-the-flag field for Digital Paint: Paintball 2. Red's base is at the west end
and blue's at the east, each a raised deck with its flag, a capture trigger around the
flag and four team spawns. Barrels, crates and a centre tower with two ramps give cover
on the way across. The ceiling is sky.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences select
the Paintball 2 profile and name the game's folder: Heron reads the textures and the
entity definitions from there.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "heron_bases.map")

FLOOR = "pball/pb_gras1"
WALL = "pball/stonewal"
SKY = "pball/sky1"
CRATE = "pball/box_1"
TOWER = "pball/wood1_1"
DIVIDER = "pball/brick1_1"
TRIGGER = "pball/trigger"

# Half the inside of the field: x runs from -1024 to 1024 and y from -512 to 512.
HALF_X = 1024
HALF_Y = 512
HEIGHT = 384


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
        """One shape; returns the id of its first brush."""
        text = self.call("add_shape", kind=kind, min=lo, max=hi, material=material, **options)
        return json.loads(text.split("selected: ", 1)[1])[0]

    def box(self, lo, hi, material):
        return self.shape("cuboid", lo, hi, material)

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


def shell(heron):
    """Floor, four walls and a sky lid, sealing the field."""
    x, y, t = HALF_X, HALF_Y, 16
    heron.box([-x - t, -y - t, -t], [x + t, y + t, 0], FLOOR)
    heron.box([-x - t, -y - t, HEIGHT], [x + t, y + t, HEIGHT + t], SKY)
    heron.box([-x - t, -y - t, 0], [-x, y + t, HEIGHT], WALL)
    heron.box([x, -y - t, 0], [x + t, y + t, HEIGHT], WALL)
    heron.box([-x, -y - t, 0], [x, -y, HEIGHT], WALL)
    heron.box([-x, y, 0], [x, y + t, HEIGHT], WALL)


def base(heron, team, side, colour):
    """One team's end of the field; side is -1 for the west end and 1 for the east."""
    deck = f"pball/{colour}_metal1"
    barrel = f"pball/{colour}_brl1"
    facing = "0" if side < 0 else "180"

    def span(a, b):
        """x from a to b inward from this end's wall, in increasing order."""
        lo, hi = side * (HALF_X - a), side * (HALF_X - b)
        return min(lo, hi), max(lo, hi)

    # The deck, 64 high, with a ramp down toward the middle of the field.
    x0, x1 = span(0, 256)
    heron.box([x0, -160, 0], [x1, 160, 64], deck)
    x0, x1 = span(256, 384)
    rotate = [0, 0, 180] if side < 0 else [0, 0, 0]
    heron.shape("wedge", [x0, -64, 0], [x1, 64, 64], deck, axis="x", rotate=rotate)

    # The flag stands on the deck, and the base trigger around it is where a player
    # carrying the other team's flag scores.
    flag_x = side * (HALF_X - 96)
    heron.entity("flag", [flag_x, 0, 88], teamnumber=str(team))
    heron.brush_entity(
        "base", [flag_x - 32, -32, 72], [flag_x + 32, 32, 192], TRIGGER,
        teamnumber=str(team),
    )

    # Team spawns on the floor beside the deck, looking up the field.
    for along in (64, 192):
        for y in (-352, 352):
            heron.entity(
                "info_player_deathmatch", [side * (HALF_X - along), y, 24],
                teamnumber=str(team), angle=facing,
            )

    # Inflatable barrels in the team's colour in front of the base.
    for along, y in ((512, -160), (512, 160), (640, -384), (640, 384)):
        cx = side * (HALF_X - along)
        heron.shape("cylinder", [cx - 24, y - 24, 0], [cx + 24, y + 24, 72], barrel, sides=12)

    # Paint and air by the spawns.
    heron.entity("item_pballammo", [side * (HALF_X - 128), -256, 16], type="100")
    heron.entity("item_pballco2", [side * (HALF_X - 128), 256, 16], type="12oz")


def midfield(heron):
    # Crates on both halves, mirrored.
    for x, y in ((384, 0), (256, -320), (256, 320)):
        for side in (-1, 1):
            cx = side * x
            heron.box([cx - 32, y - 32, 0], [cx + 32, y + 32, 64], CRATE)
    # A centre tower 96 high, climbed by a ramp from each side.
    heron.box([-96, -96, 0], [96, 96, 96], TOWER)
    heron.shape("wedge", [-288, -48, 0], [-96, 48, 96], TOWER, axis="x")
    heron.shape("wedge", [96, -48, 0], [288, 48, 96], TOWER, axis="x", rotate=[0, 0, 180])
    # Low walls splitting the flanks.
    for y0, y1 in ((-416, -224), (224, 416)):
        heron.box([-16, y0, 0], [16, y1, 96], DIVIDER)
    # A gun on the tower for whoever gets there first.
    heron.entity("weapon_pballgun", [0, 0, 112], type="high")
    heron.entity("item_pballhopper", [0, 48, 112], type="200")


def lights(heron):
    for x in (-896, -512, -128, 128, 512, 896):
        for y in (-320, 320):
            heron.entity("light", [x, y, 320], light="400")
    heron.entity("light", [0, 0, 256], light="300")


def main():
    heron = Heron(sys.argv[1])
    heron.call("new_map")
    for key, value in [
        ("message", "Heron Brush: Twin Bases"),
        ("sky", "pbsky2"),
        ("gamemode", "4"),
        ("maxteams", "2"),
        ("team1", "red"),
        ("team2", "blue"),
        # ericw-tools' light: a sun from the south-west and light from the sky dome.
        ("_sunlight", "180"),
        ("_sun_mangle", "60 -55 0"),
        ("_sunlight2", "60"),
    ]:
        heron.call("set_entity_property", entity="worldspawn", key=key, value=value)

    shell(heron)
    base(heron, 1, -1, "r")
    base(heron, 2, 1, "b")
    midfield(heron)
    lights(heron)
    heron.entity("info_player_intermission", [0, -448, 320], angles="25 90 0")

    print(heron.call("save", path=MAP))
    heron.close()


if __name__ == "__main__":
    main()
