#!/usr/bin/env python3
"""Build a few asset-library prefabs by driving `heron-brush --mcp`.

Run from the repo root after `cargo build -p heron-brush`:

    python3 assets/prefabs_gen.py

Writes assets/bookshelf.map and assets/bridge.map (and a preview PNG for each).
The dragon has its own script; the small starter prefabs (crate, pillar, bench,
lamp) were hand-written.
"""

import colorsys
import json
import random
import subprocess
import sys

BIN = "target/debug/heron-brush"


class Editor:
    def __init__(self):
        self.proc = subprocess.Popen(
            [BIN, "--mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1
        )
        self.next_id = 1
        self._rpc("initialize", {})

    def _rpc(self, method, params):
        rid = self.next_id
        self.next_id += 1
        self.proc.stdin.write(
            json.dumps({"jsonrpc": "2.0", "id": rid, "method": method, "params": params}) + "\n"
        )
        self.proc.stdin.flush()
        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError("the editor closed the connection")
            msg = json.loads(line)
            if msg.get("id") == rid:
                if (method == "tools/call" and params.get("name") == "save"
                        and not msg.get("error") and not msg.get("result", {}).get("isError")):
                    path = params.get("arguments", {}).get("path")
                    if path:
                        from asset_materials import apply_file
                        if apply_file(path):
                            self.call("open_map", path=path)
                return msg

    def call(self, tool_name, **arguments):
        if tool_name == 'save' and arguments.get('path'):
            from asset_scale import normalize_editor
            normalize_editor(self, arguments['path'])
        msg = self._rpc("tools/call", {"name": tool_name, "arguments": arguments})
        result = msg.get("result", {})
        text = result.get("content", [{}])[0].get("text", "")
        if result.get("isError"):
            raise RuntimeError(f"{tool_name}: {text}")
        if tool_name == 'save' and arguments.get('path'):
            from asset_grid import snap_file
            snap_file(self, arguments['path'])
            from asset_corners import join_file
            join_file(self, arguments['path'])
            from asset_skip import optimize_file
            optimize_file(self, arguments['path'])
        return text

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=10)


def box(ed, x0, y0, z0, x1, y1, z1, color, material="wood"):
    ed.call(
        "add_shape",
        kind="cuboid",
        min=[min(x0, x1), min(y0, y1), min(z0, z1)],
        max=[max(x0, x1), max(y0, y1), max(z0, z1)],
        material=material,
    )
    ed.call("set_face_color", color=list(color))


def hsv(h, s, v):
    r, g, b = colorsys.hsv_to_rgb(h % 1.0, s, v)
    return [round(r * 255), round(g * 255), round(b * 255)]


# --- bookshelf -------------------------------------------------------------

def bookshelf(ed):
    ed.call("new_map")
    rng = random.Random(7)

    # Light woods: the preview shades faces that turn from the light down to a
    # third of their colour, so a dark stain there just reads as black.
    wood = [170, 120, 74]
    dark = [138, 96, 58]
    w, d, h = 128, 28, 210      # outside width, depth, height
    t = 6                       # panel thickness
    shelves = 4

    box(ed, 0, 0, 0, t, d, h, wood)              # left side
    box(ed, w - t, 0, 0, w, d, h, wood)          # right side
    box(ed, 0, 0, 0, w, d, t, dark)              # bottom
    box(ed, 0, 0, h - t, w, d, h, dark)          # top
    box(ed, 0, d - 3, 0, w, d, h, dark)          # back

    inner_h = h - 2 * t
    for i in range(1, shelves):
        z = t + round(inner_h * i / shelves)
        box(ed, t, 0, z, w - t, d, z + t, wood)  # a shelf

    # Books: a run of thin uprights per shelf, colours all over. Each section is
    # inner_h / shelves tall, so the books stop short of the shelf above.
    section = inner_h / shelves
    for i in range(shelves):
        z_shelf = t + round(inner_h * i / shelves) + t
        headroom = section - t - 6
        x = t + 4
        while x < w - t - 12:
            bw = rng.choice([6, 7, 8, 9, 11])
            bh = round(headroom * rng.uniform(0.7, 0.98))
            color = hsv(rng.random(), rng.uniform(0.55, 0.9), rng.uniform(0.78, 1.0))
            box(ed, x, 4, z_shelf, x + bw, d - 5, z_shelf + bh, color, material="paper")
            x += bw + rng.choice([0, 0, 0, 1, 2])
        # a couple of books laid flat, stacked at one end
        if rng.random() < 0.7:
            fw = rng.randint(26, 40)
            stack = z_shelf
            for _ in range(rng.randint(1, 3)):
                box(ed, t + 6, 5, stack, t + 6 + fw, d - 6, stack + 7,
                    hsv(rng.random(), 0.7, 0.92), material="paper")
                stack += 7

    ed.call("add_point_entity", classname="light", origin=[w / 2, -60, h + 40],
            properties={"light": "300"})
    ed.call("save", path="assets/bookshelf.map")
    ed.call("render_preview", path="assets/bookshelf_preview.png", size=700)


# --- bridge ---------------------------------------------------------------

def bridge(ed):
    ed.call("new_map")

    span = 384
    width = 96
    deck_z = 64
    stone = [186, 180, 162]
    stone_dark = [150, 144, 128]
    rail = [174, 168, 150]

    # Two abutments and a shallow arch of voussoir blocks under the deck.
    box(ed, -40, 0, 0, 24, width, deck_z, stone_dark)
    box(ed, span - 24, 0, 0, span + 40, width, deck_z, stone_dark)

    blocks = 13
    import math
    for i in range(blocks):
        f = i / (blocks - 1)
        x = 24 + (span - 48) * f
        rise = math.sin(math.pi * f) * 46
        box(ed, x - (span - 48) / blocks / 2 - 2, 4, deck_z - 20 - rise,
            x + (span - 48) / blocks / 2 + 2, width - 4, deck_z - rise * 0.15, stone)

    # Deck.
    box(ed, -30, 0, deck_z, span + 30, width, deck_z + 10, stone)
    # Parapets down both sides, with gaps for balusters.
    for y0, y1 in ((0, 10), (width - 10, width)):
        x = -20
        while x < span + 20:
            box(ed, x, y0, deck_z + 10, x + 22, y1, deck_z + 34, rail)
            x += 34
    # A little wear: darker capstones every so often.
    for x in range(0, span, 96):
        box(ed, x, 0, deck_z + 8, x + 10, width, deck_z + 12, stone_dark)

    ed.call("add_point_entity", classname="light", origin=[span / 2, width / 2, 260],
            properties={"light": "400"})
    ed.call("save", path="assets/bridge.map")
    ed.call("render_preview", path="assets/bridge_preview.png", size=760)


def main():
    for build, name in ((bookshelf, "bookshelf"), (bridge, "bridge")):
        ed = Editor()
        try:
            build(ed)
            print(f"wrote assets/{name}.map")
        finally:
            ed.close()


if __name__ == "__main__":
    sys.exit(main())
