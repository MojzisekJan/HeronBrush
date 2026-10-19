#!/usr/bin/env python3
"""Generate a simple, editable Steve brush prefab. Run from the repository root."""

import json

from prefabs_gen import Editor, box


COLORS = {
    "skin": [177, 130, 104],
    "hair": [65, 43, 28],
    "shirt": [35, 180, 185],
    "pants": [70, 70, 168],
    "shoes": [66, 68, 84],
    "white": [225, 225, 220],
    "eyes": [70, 65, 160],
    "nose": [145, 91, 60],
    "beard": [80, 50, 36],
}


def main():
    ed = Editor()
    count = 0

    def part(bounds, color):
        nonlocal count
        box(ed, *bounds, COLORS[color], material="__TB_empty")
        count += 1

    try:
        ed.call("new_map")
        # Two units per pixel: 64 units tall, standing at z=0, facing +Y.
        for x in (-8, 0):
            part((x, -4, 0, x + 8, 4, 4), "shoes")
            part((x, -4, 4, x + 8, 4, 24), "pants")
        part((-8, -4, 24, 8, 4, 48), "shirt")
        for x in (-16, 8):
            part((x, -4, 24, x + 8, 4, 40), "skin")
            part((x, -4, 40, x + 8, 4, 48), "shirt")

        # A solid head with a separate front layer. Parts touch without overlapping.
        part((-8, -6, 48, 8, 6, 62), "skin")
        part((-8, -8, 48, 8, -6, 54), "skin")
        part((-8, -8, 54, 8, -6, 62), "hair")
        part((-8, -8, 62, 8, 8, 64), "hair")
        face = [
            "HHHHHHHH",
            "HSSSSSSH",
            "SSSSSSSS",
            "SWEPPEWS",
            "SSSNNSSS",
            "SSBSSBSS",
            "SSBBBBSS",
        ]
        palette = {"H": "hair", "S": "skin", "W": "white", "E": "eyes",
                   "P": "skin", "N": "nose", "B": "beard"}
        for row, pixels in enumerate(face):
            col = 0
            while col < 8:
                end = col + 1
                while end < 8 and palette[pixels[end]] == palette[pixels[col]]:
                    end += 1
                z = 60 - row * 2
                part((-8 + col * 2, 6, z, -8 + end * 2, 8, z + 2), palette[pixels[col]])
                col = end

        assert len(json.loads(ed.call("list_brushes"))) == count
        ed.call("save", path="assets/steve.map")
        ed.call("render_preview", path="assets/steve.png", size=512)
        print(f"Wrote assets/steve.map and assets/steve.png ({count} brushes)")
    finally:
        ed.close()


if __name__ == "__main__":
    main()
