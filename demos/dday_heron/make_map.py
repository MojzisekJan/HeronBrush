#!/usr/bin/env python3
"""Builds heron_hedgerow.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A small two-team field for D-Day: Normandy. The Allies start at the west end and the
Axis at the east, each behind a low wall with a spawn for every class and one for
reinforcements. A concrete bunker stands in the middle, and its roof is the objective:
whoever stands on it takes it and scores. Crates and hedges give cover on the way. The
ceiling is sky. Every texture is the mod's own.

Run it with XDG_CONFIG_HOME pointing at a scratch directory whose preferences name the
D-Day: Normandy profile and the mod's folder. It writes heron_hedgerow.map in Heron's
own dialect and heron_hedgerow_q2.map in Quake 2's, which build.sh compiles.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "heron_hedgerow.map")
QUAKE2_MAP = os.path.join(HERE, "heron_hedgerow_q2.map")

GRASS = "dday/grass1"
WALL = "dday/bricks2"
LOW_WALL = "dday/bricks7"
CONCRETE = "dday/base1"
CRATE = "dday/crate1"
HEDGE = "dday/hedge"
SKY = "e1u1/sky1"  # the mod ships id's sky texture: its .wal says sky
TRIGGER = "e1u1/trigger"

# Half the inside of the field: x runs from -1024 to 1024 and y from -512 to 512.
HALF_X = 1024
HALF_Y = 512
HEIGHT = 448

# In the order they stand behind the wall; infantry, the class a new player gets
# offered first, is in the middle, in line with the gap in the wall and the bunker.
CLASSES = [
    "info_lgunner_start",
    "info_officer_start",
    "info_infantry_start",
    "info_hgunner_start",
    "info_sniper_start",
    "info_special_start",
    "info_engineer_start",
    "info_medic_start",
    "info_flamethrower_start",
]


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


def shell(heron):
    """Floor, four walls and a sky lid, sealing the field."""
    x, y, t = HALF_X, HALF_Y, 16
    heron.box([-x - t, -y - t, -t], [x + t, y + t, 0], GRASS)
    heron.box([-x - t, -y - t, HEIGHT], [x + t, y + t, HEIGHT + t], SKY)
    heron.box([-x - t, -y - t, 0], [-x, y + t, HEIGHT], WALL)
    heron.box([x, -y - t, 0], [x + t, y + t, HEIGHT], WALL)
    heron.box([-x, -y - t, 0], [x, -y, HEIGHT], WALL)
    heron.box([-x, y, 0], [x, y + t, HEIGHT], WALL)


def team_end(heron, team, side, name, dll):
    """One team's end; team is 0 for the Allies and 1 for the Axis, side -1 is west."""
    facing = "0" if side < 0 else "180"
    # The team itself: its name, its army, and what wins the map for it.
    heron.entity(
        "info_team_start", [side * (HALF_X - 64), 0, 64],
        obj_owner=str(team), message=name, dll=dll, points="20", kills="30",
        nextmap="heron_hedgerow",
    )
    # A low wall to start behind, with a gap in the middle.
    x0, x1 = sorted((side * (HALF_X - 256), side * (HALF_X - 272)))
    heron.box([x0, -HALF_Y, 0], [x1, -64, 48], LOW_WALL)
    heron.box([x0, 64, 0], [x1, HALF_Y, 48], LOW_WALL)
    # Reinforcements and a spawn for every class, in two rows behind the wall.
    heron.entity(
        "info_reinforcements_start", [side * (HALF_X - 128), 0, 24],
        obj_owner=str(team), angle=facing,
    )
    for index, classname in enumerate(CLASSES):
        row, column = divmod(index, 5)
        x = side * (HALF_X - 64 - 96 * row)
        y = -384 + 192 * column if row == 0 else -288 + 192 * column
        heron.entity(classname, [x, y, 24], obj_owner=str(team), angle=facing)


def middle(heron):
    # The bunker, 256 square and 96 high, with a ramp up its west and east sides.
    heron.box([-128, -128, 0], [128, 128, 96], CONCRETE)
    heron.call(
        "add_shape", kind="wedge", min=[-320, -64, 0], max=[-128, 64, 96],
        material=CONCRETE, axis="x",
    )
    heron.call(
        "add_shape", kind="wedge", min=[128, -64, 0], max=[320, 64, 96],
        material=CONCRETE, axis="x", rotate=[0, 0, 180],
    )
    # Standing on the roof takes it: 99 means nobody holds it at the start, health is
    # the points the taker scores, dmg what the loser gives back.
    heron.brush_entity(
        "objective_touch", [-96, -96, 96], [96, 96, 160], TRIGGER,
        obj_owner="99", message="The bunker", health="10", dmg="5",
    )
    # Crates and hedges on both halves, mirrored.
    for side in (-1, 1):
        for x, y in ((512, -288), (448, 256)):
            cx = side * x
            heron.box([cx - 32, y - 32, 0], [cx + 32, y + 32, 64], CRATE)
        for y0, y1 in ((-320, -128), (128, 320)):
            heron.box([side * 640 - 16, y0, 0], [side * 640 + 16, y1, 80], HEDGE)


def lights(heron):
    for x in (-768, -256, 256, 768):
        for y in (-320, 320):
            heron.entity("light", [x, y, 384], light="300")


def main():
    heron = Heron(sys.argv[1])
    heron.call("new_map")
    for key, value in [
        ("message", "Heron Hedgerow"),
        ("sky", "sr"),
        # ericw-tools' light: a low sun from the south-west and light from the sky.
        ("_sunlight", "150"),
        ("_sun_mangle", "60 -40 0"),
        ("_sunlight2", "60"),
    ]:
        heron.call("set_entity_property", entity="worldspawn", key=key, value=value)

    shell(heron)
    team_end(heron, 0, -1, "Allies", "usa")
    team_end(heron, 1, 1, "Axis", "grm")
    middle(heron)
    lights(heron)
    # Where a player waits while dead or before joining a team. The game keeps the view
    # a player had here when it puts them in at a team's spawn, so this looks east, the
    # way the Allies go.
    heron.entity("info_player_start", [0, -384, 256], angle="0")

    print(heron.call("save", path=MAP))
    print(heron.call("export_map", path=QUAKE2_MAP, format="quake2"))
    heron.close()


if __name__ == "__main__":
    main()
