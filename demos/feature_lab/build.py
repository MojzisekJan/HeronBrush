#!/usr/bin/env python3
"""Rebuild feature_lab.map, one station per export feature, through the editor's MCP.

Run from any directory after `cargo build -p heron-brush`:

    python3 demos/feature_lab/build.py

The script writes the brushes, materials, UVs, paint and entities as .map text,
opens that in the editor, adds the animations with the MCP animation tools and
saves the result over feature_lab.map. Every station is an entity whose
`targetname` starts with `fl_`, so a checker can find it in any export.
"""
import argparse
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MAP = HERE / "feature_lab.map"


def face_lines(lo, hi, faces):
    """Six standard-format face lines of an axis-aligned box.

    Order: -X, +X, -Y, +Y, +Z (top), -Z (bottom), the same order the editor's
    face indices use for a cuboid. Each face is (material, ox, oy, rot, sx, sy, note).
    """
    x0, y0, z0 = lo
    x1, y1, z1 = hi
    planes = [
        f"( {x0} {y0} {z0} ) ( {x0} {y0 + 1} {z0} ) ( {x0} {y0} {z0 + 1} )",
        f"( {x1} {y1} {z1} ) ( {x1} {y1} {z1 + 1} ) ( {x1} {y1 + 1} {z1} )",
        f"( {x0} {y0} {z0} ) ( {x0} {y0} {z0 + 1} ) ( {x0 + 1} {y0} {z0} )",
        f"( {x1} {y1} {z1} ) ( {x1 + 1} {y1} {z1} ) ( {x1} {y1} {z1 + 1} )",
        f"( {x1} {y1} {z1} ) ( {x1} {y1 + 1} {z1} ) ( {x1 + 1} {y1} {z1} )",
        f"( {x0} {y0} {z0} ) ( {x0 + 1} {y0} {z0} ) ( {x0} {y0 + 1} {z0} )",
    ]
    lines = []
    for plane, face in zip(planes, faces):
        material, ox, oy, rot, sx, sy, note = face
        line = f"{plane} {material} {ox} {oy} {rot} {sx} {sy}"
        if note:
            line += f" // heron_brush {note}"
        lines.append(line)
    return lines


def plain(material, note=None):
    return (material, 0, 0, 0, 1, 1, note)


def brush(lo, hi, faces):
    if not isinstance(faces, list):
        faces = [faces] * 6
    return "{\n" + "\n".join(face_lines(lo, hi, faces)) + "\n}"


def entity(properties, brushes=()):
    body = "\n".join(f'"{k}" "{v}"' for k, v in properties.items())
    return "{\n" + body + ("\n" if brushes else "") + "\n".join(brushes) + "\n}"


def station_box(x, y, size=64, height=64, z=8):
    """A station block, lifted off the floor so no face of it is buried."""
    return (x, y, z), (x + size, y + size, z + height)


def map_text():
    world = {
        "mapversion": "220",
        "classname": "worldspawn",
        "sky": "1",
        "sky_top_color": "40 90 200",
        "sky_horizon_color": "230 200 160",
        "fog_enabled": "1",
        "fog_light_color": "180 190 200",
        "fog_density": "0.02",
    }
    floor = brush((-1024, -512, -16), (1024, 512, 0), plain("concrete"))
    out = ["// heron material-sources [[\"folder\",\"textures\"]]", "// Format: Valve",
           entity(world, [floor])]

    # Row one, y = -320: surface appearance.
    lo, hi = station_box(-896, -320)
    out.append(entity({"classname": "func_detail", "targetname": "fl_textures"}, [
        brush(lo, hi, [
            ("wood", 0, 0, 0, 1, 1, None),
            ("metal", 16, 8, 0, 1, 1, None),
            ("crate", 0, 0, 45, 1, 1, None),
            ("wall", 0, 0, 0, 0.5, 2, None),
            ("dirt", 32, 0, 90, 2, 2, None),
            ("hazard", 0, 0, 0, 1, 1, None),
        ]),
        brush((lo[0], lo[1] + 96, 8), (hi[0], hi[1] + 96, 40), plain("grass")),
    ]))
    lo, hi = station_box(-704, -320)
    out.append(entity({"classname": "func_detail", "targetname": "fl_colours"}, [
        brush(lo, hi, [
            plain("wood", "color 220 30 40"),
            plain("wood", "color 30 200 60"),
            plain("wood", "color 40 60 230"),
            plain("wood", "color 240 220 30"),
            plain("wood", "color 255 255 255"),
            plain("wood", "color 30 200 220 128"),
        ]),
    ]))
    lo, hi = station_box(-512, -320)
    out.append(entity({"classname": "func_detail", "targetname": "fl_gradients"}, [
        brush(lo, hi, [
            plain("wood", "gradient 255 0 0 0 0 255 0"),
            plain("wood", "linear 45 3 0 255 255 0 0.5 0 255 0 1 0 0 0"),
            plain("wood", "radial 0 3 0 255 255 255 0.5 255 128 0 1 64 0 128"),
            plain("wood", "gradient 0 0 0 255 255 255 90"),
            plain("wood", "color 128 128 128"),
            plain("wood", "color 128 128 128"),
        ]),
    ]))
    lo, hi = station_box(-320, -320)
    out.append(entity({"classname": "func_detail", "targetname": "fl_surface"}, [
        brush(lo, hi, [
            plain("metal", "pbr 20 90 0"),
            plain("metal", "pbr 90 0 0"),
            plain("wood", "color 255 120 0 pbr 65 0 300"),
            plain("metal"),
            plain("metal", "pbr 5 100 0"),
            plain("metal"),
        ]),
    ]))

    # brick_wall has a brick_wall_normal companion in textures/.
    lo, hi = station_box(-128, -320)
    out.append(entity({"classname": "func_detail", "targetname": "fl_normal"},
                      [brush(lo, hi, plain("brick_wall"))]))

    # Row two, y = 0: collision.
    lo, hi = station_box(-896, 0)
    out.append(entity({"classname": "func_wall", "targetname": "fl_solid"},
                      [brush(lo, hi, plain("wall"))]))
    lo, hi = station_box(-704, 0, height=96)
    out.append(entity({"classname": "func_clip", "targetname": "fl_clip"},
                      [brush(lo, hi, plain("clip"))]))
    lo, hi = station_box(-512, 0)
    out.append(entity({"classname": "func_detail", "targetname": "fl_skip"}, [
        brush(lo, hi, [plain("metal"), plain("metal"), plain("metal"), plain("metal"),
                       plain("skip"), plain("metal")]),
    ]))
    lo, hi = station_box(-320, 0)
    out.append(entity({"classname": "func_detail", "targetname": "fl_hidden"}, [
        brush(lo, hi, [plain("wood"), plain("wood", "export_hidden"), plain("wood"),
                       plain("wood"), plain("wood"), plain("wood")]),
    ]))
    lo, hi = station_box(-128, 0, size=96, height=96)
    out.append(entity({"classname": "trigger_multiple", "targetname": "fl_trigger",
                       "target": "fl_door", "wait": "1.5"},
                      [brush(lo, hi, plain("hazard"))]))

    # Row three, y = 320: animation and brush entities with links.
    lo, hi = station_box(-896, 320, size=96, height=16)
    out.append(entity({"classname": "func_animate", "targetname": "fl_mover", "start_on": "1"},
                      [brush(lo, hi, plain("metal"))]))
    out.append(entity({"classname": "func_animate", "targetname": "fl_door"},
                      [brush((-640, 320, 8), (-632, 384, 104), plain("crate"))]))
    out.append(entity({"classname": "func_button", "targetname": "fl_button",
                       "target": "fl_door"},
                      [brush((-576, 320, 8), (-560, 336, 24), plain("hazard"))]))
    out.append(entity({"classname": "water_volume", "targetname": "fl_water",
                       "water_color": "0.1 0.4 0.6", "swim_speed": "2.5",
                       "water_priority": "3"},
                      [brush((-384, 320, 8), (-256, 448, 56), plain("wood"))]))

    # Point entities.
    out.append(entity({"classname": "info_player_start", "targetname": "fl_start",
                       "origin": "0 -160 32", "angles": "0 90 0"}))
    out.append(entity({"classname": "light", "targetname": "fl_light",
                       "origin": "-160 -288 160", "light_color": "255 200 120",
                       "light_energy": "2.5", "omni_range": "8"}))
    out.append(entity({"classname": "light_spot", "targetname": "fl_spot",
                       "origin": "-480 64 192", "angles": "-60 45 0",
                       "light_color": "120 180 255", "light_energy": "3",
                       "spot_range": "10", "spot_angle": "30"}))
    out.append(entity({"classname": "info_waypoint", "targetname": "fl_wp_a",
                       "target": "fl_wp_b", "origin": "128 -160 32"}))
    out.append(entity({"classname": "info_waypoint", "targetname": "fl_wp_b",
                       "origin": "256 -160 32"}))
    out.append(entity({"classname": "info_camera", "targetname": "fl_camera",
                       "origin": "384 -160 64", "angles": "0 180 0", "fov": "62.5",
                       "current": "1"}))
    out.append(entity({"classname": "fl_probe", "targetname": "fl_probe",
                       "target": "fl_wp_a", "origin": "512 -160 32", "charge": "7",
                       "note": "free text, with a comma"}))

    # Imported model, placed twice so exporters can share one mesh.
    out.append(entity({"classname": "heron_model", "targetname": "fl_model_a",
                       "_heron_model": "models/pbr.glb", "origin": "128 160 32",
                       "angles": "0 0 0", "modelscale": "32"}))
    out.append(entity({"classname": "heron_model", "targetname": "fl_model_b",
                       "_heron_model": "models/pbr.glb", "origin": "256 160 32",
                       "angles": "0 45 0", "modelscale": "48"}))
    return "\n".join(out) + "\n"


class Editor:
    def __init__(self, binary, scratch):
        env = dict(os.environ, XDG_CONFIG_HOME=str(scratch / "config"),
                   XDG_DATA_HOME=str(scratch / "data"), XDG_CACHE_HOME=str(scratch / "cache"))
        self.proc = subprocess.Popen([str(binary), "--mcp"], stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, text=True, bufsize=1, env=env)
        self.serial = 0
        self.rpc("initialize", {})

    def rpc(self, method, params):
        self.serial += 1
        self.proc.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.serial,
                                          "method": method, "params": params}) + "\n")
        self.proc.stdin.flush()
        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError("the editor closed the connection")
            message = json.loads(line)
            if message.get("id") == self.serial:
                return message

    def call(self, tool, **arguments):
        result = self.rpc("tools/call", {"name": tool, "arguments": arguments}).get("result", {})
        text = "\n".join(c.get("text", "") for c in result.get("content", []))
        if result.get("isError"):
            raise RuntimeError(f"{tool}: {text}")
        return text

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=30)


def entity_ids(editor):
    """targetname -> entity id, from the editor's entity graph."""
    graph = json.loads(editor.call("entity_graph"))
    ids = {}
    for node in graph["nodes"]:
        if node.get("name"):
            ids[node["name"]] = str(node["id"])
    return ids


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    args = parser.parse_args()
    MAP.write_text(map_text())
    with tempfile.TemporaryDirectory(prefix="heron-feature-lab-") as scratch:
        editor = Editor(args.binary.resolve(), Path(scratch))
        try:
            print(editor.call("open_map", path=str(MAP)))
            ids = entity_ids(editor)
            # A loop with every interpolation: linear X, a held Z step, a bezier turn.
            print(editor.call("set_animation", entity=ids["fl_mover"], duration=4, fps=30,
                              mode="loop", tracks=[
                                  {"channel": "X", "keys": [
                                      {"time": 0, "value": 0, "interpolation": "linear"},
                                      {"time": 2, "value": 128, "interpolation": "linear"},
                                      {"time": 4, "value": 0, "interpolation": "linear"}]},
                                  {"channel": "Z", "keys": [
                                      {"time": 0, "value": 0, "interpolation": "hold"},
                                      {"time": 1, "value": 32, "interpolation": "hold"},
                                      {"time": 3, "value": 0, "interpolation": "hold"}]},
                                  {"channel": "RZ", "keys": [
                                      {"time": 0, "value": 0, "interpolation": "bezier"},
                                      {"time": 4, "value": 90, "interpolation": "bezier"}]},
                              ]))
            # The door turns about its hinge edge, so the pivot goes there before the
            # recipe keys the open and close clips.
            print(editor.call("apply_animation_recipe", entity=ids["fl_door"], recipe="door",
                              duration=1, amount=90, pivot=[-636, 320, 8]))
            print(editor.call("configure_animation_trigger", source=ids["fl_button"],
                              action="toggle"))
            print(editor.call("configure_animation_trigger", source=ids["fl_trigger"],
                              action="open", wait=1.5))
            result = editor.rpc("tools/call", {"name": "save", "arguments": {"path": str(MAP)}})
            if result.get("error") or result.get("result", {}).get("isError"):
                raise RuntimeError(f"save: {result}")
            print(editor.call("open_map", path=str(MAP)))
        finally:
            editor.close()


if __name__ == "__main__":
    main()
