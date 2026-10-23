#!/usr/bin/env python3
"""Builds heron_marsh.map by driving Heron Brush over MCP (`heron-brush --mcp`).

Usage: make_map.py HERON_BRUSH_BINARY

A hall and a courtyard open to the sky, joined by a door that a button opens. A
trigger in the courtyard's doorway switches the courtyard's lamps on; froginators and
protopods wait there, and steps lead up to a platform at the far end, where the exit
loads the demo's first map. Every texture is in the demo's data/pak1.pak, under
textures/e1m1, and every monster and item is one the demo's first episode has.

Daikatana's .wal files give their faces no flags, so the sky and the light panels get
theirs on the face, with set_face_flags. Run it with XDG_CONFIG_HOME pointing at a
scratch directory whose preferences name the Daikatana profile and its game path. It
writes heron_marsh.map in Heron's own dialect and heron_marsh_dk.map in the Quake 2
face lines Daikatana's qbsp3 reads, which build.sh compiles.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MAP = os.path.join(HERE, "heron_marsh.map")
DAIKATANA_MAP = os.path.join(HERE, "heron_marsh_dk.map")

FLOOR = "e1m1/swfortile2"
WALL = "e1m1/swfort1"
CEILING = "e1m1/swroofgrate1"
TRIM = "e1m1/swfortrf1b"  # the platform and its steps
LAMP = "e1m1/lightw"  # a light panel, given light on its face
ROCK = "e1m1/mossrk_04"  # the courtyard's walls
GROUND = "e1m1/swgrasspatch2"
DOOR = "e1m1/swoutdoor6"
BUTTON = "e1m1/swbulblight2"
SKY = "e1m1/sky2a"
TRIGGER = "e1m1/trigger"

# Surface flags from the profile: 1 light, 4 sky.
LIGHT = 1
SKY_FLAG = 4


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
        panel = heron.box([224, y - 32, 184], [288, y + 32, 192], LAMP)
        # The underside gives off a warm white, as the DK tools' example sky does.
        heron.call(
            "set_face_flags", brush=panel, facing=[0, 0, -1],
            surface=LIGHT, value=3000, color=[255, 240, 200],
        )
    for x, y in ((128, 128), (128, 384), (384, 128), (384, 384)):
        heron.entity("light", [x, y, 160], light="400")


def wall_between(heron):
    """The east wall of the hall, with a doorway 64 wide and 128 high."""
    heron.box([512, -80, 0], [528, 224, 320], WALL)
    heron.box([512, 288, 0], [528, 592, 320], WALL)
    heron.box([512, 224, 128], [528, 288, 320], WALL)


def courtyard(heron):
    """640 x 640 inside and 320 high, under the sky, with a platform at the far end."""
    heron.box([512, -80, -16], [1184, 592, 0], GROUND)
    lid = heron.box([512, -80, 320], [1184, 592, 336], SKY)
    # Sky and light, as e1m1's own sky faces are: the sky lights the courtyard.
    heron.call(
        "set_face_flags", brush=lid, facing=[0, 0, -1],
        surface=LIGHT | SKY_FLAG, value=300, color=[255, 255, 204],
    )
    heron.box([528, -80, 0], [1168, -64, 320], ROCK)
    heron.box([528, 576, 0], [1168, 592, 320], ROCK)
    heron.box([1168, -80, 0], [1184, 592, 320], ROCK)
    # The platform, 64 high, and four steps of 16 up to it.
    heron.box([1008, -64, 0], [1168, 576, 64], TRIM)
    for step in range(3):
        x = 912 + 32 * step
        heron.box([x, 160, 0], [x + 32, 352, 16 * (step + 1)], TRIM)


def door_and_button(heron):
    # A door with a targetname gets no touch field and waits for the button; angle -1
    # opens it upwards and wait -1 keeps it open. The sounds are e1m1's own door's.
    heron.brush_entity(
        "func_door", [512, 224, 0], [528, 288, 128], DOOR,
        targetname="door1", angle="-1", wait="-1", speed="100",
        sound_opening="doors/e1/door1loop.wav",
        sound_open_finish="doors/e1/door1openend.wav",
    )
    # The button sits on the wall beside the door and is pushed into it (east).
    # PUSH_TOUCH (1) lets the player push it by walking into it, Quake style.
    heron.brush_entity(
        "func_button", [504, 320, 40], [512, 352, 72], BUTTON,
        target="door1", angle="0", wait="-1", spawnflags="1",
    )


def ambush(heron):
    # Crossing into the courtyard fires "lamps": the four lights, which start off
    # (START_OFF, 1), come on. qrad3 gives each switchable light a style of its own.
    heron.brush_entity(
        "trigger_once", [544, 160, 0], [608, 352, 128], TRIGGER,
        target="lamps", message="The lamps come on.",
    )
    for origin in ([672, 0, 288], [672, 512, 288], [928, 0, 288], [928, 512, 288]):
        heron.entity("light", origin, light="300", targetname="lamps", spawnflags="1")
    heron.entity("monster_froginator", [800, 64, 24], angle="180")
    heron.entity("monster_froginator", [800, 448, 24], angle="180")
    heron.entity("monster_protopod", [960, 96, 0], angle="180")
    heron.entity("monster_protopod", [960, 416, 0], angle="180")


def exit_trigger(heron):
    # The exit loads the demo's first map at its start, named "primary" there.
    heron.brush_entity(
        "trigger_changelevel", [1104, 192, 64], [1152, 320, 176], TRIGGER,
        map="e1m1a", target="primary",
    )


def items(heron):
    heron.entity("info_player_start", [96, 256, 24], angle="0")
    heron.entity("weapon_ionblaster", [256, 256, 0])
    heron.entity("ammo_ionpack", [256, 320, 0])
    heron.entity("item_chromatic_armor", [384, 64, 0])
    heron.entity("item_health_25", [384, 448, 0])
    heron.entity("item_savegem", [1088, 64, 64])
    heron.entity("item_health_50", [1088, 448, 64])


def main():
    heron = Heron(sys.argv[1])
    heron.call("new_map")
    for key, value in [
        ("mapname", "Heron Marsh Gate"),
        # The texture set whose colormap the engine uses, and the episode, which picks
        # the weapons. The DK tools' readme asks for both.
        ("palette", "e1m1"),
        ("episode", "1"),
        ("sky", "e1m1"),
        ("cloudname", "e1m1tile"),
        ("cloud1speed", ".5"),
        ("cloud2speed", "2"),
        ("ambient", "5"),
    ]:
        heron.call("set_entity_property", entity="worldspawn", key=key, value=value)

    hall(heron)
    wall_between(heron)
    courtyard(heron)
    door_and_button(heron)
    ambush(heron)
    exit_trigger(heron)
    items(heron)

    print(heron.call("save", path=MAP))
    print(heron.call("export_map", path=DAIKATANA_MAP, format="quake2"))
    heron.close()


if __name__ == "__main__":
    main()
