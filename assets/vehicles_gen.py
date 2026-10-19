#!/usr/bin/env python3
"""Build the vehicle prefabs by driving `heron-brush --mcp`.

Run from the repo root after `cargo build -p heron-brush`:

    python3 assets/vehicles_gen.py

Writes assets/truck.map, assets/locomotive.map, assets/tram.map and
assets/airship.map (and a preview PNG for each). The furniture is in
prefabs_gen.py and the dragon has a script of its own.
"""

import sys

from prefabs_gen import Editor, box


def cyl(ed, x0, y0, z0, x1, y1, z1, color, axis="y", sides=12, material="metal"):
    ed.call(
        "add_shape",
        kind="cylinder",
        min=[min(x0, x1), min(y0, y1), min(z0, z1)],
        max=[max(x0, x1), max(y0, y1), max(z0, z1)],
        axis=axis,
        sides=sides,
        material=material,
    )
    ed.call("set_face_color", color=list(color))


def sphere(ed, x0, y0, z0, x1, y1, z1, color, material="cloth"):
    ed.call(
        "add_shape",
        kind="sphere",
        min=[min(x0, x1), min(y0, y1), min(z0, z1)],
        max=[max(x0, x1), max(y0, y1), max(z0, z1)],
        material=material,
    )
    ed.call("set_face_color", color=list(color))


def wheel(ed, cx, y0, y1, cz, radius, color):
    cyl(ed, cx - radius, y0, cz - radius, cx + radius, y1, cz + radius, color)


# --- truck ----------------------------------------------------------------

def truck(ed):
    ed.call("new_map")

    red = [196, 60, 52]
    dark = [72, 74, 82]
    tyre = [56, 56, 60]
    glass = [150, 196, 214]
    bed = [140, 146, 152]

    # Chassis rails and bumpers, wheels under the corners.
    box(ed, -8, 8, 22, 228, 72, 34, dark, material="metal")
    for cx in (40, 184):
        for y0, y1 in ((-4, 12), (68, 84)):
            wheel(ed, cx, y0, y1, 20, 18, tyre)

    # The cab at the front: body, a windshield leaning back, a grille.
    box(ed, 0, 0, 34, 74, 80, 66, red, material="metal")
    box(ed, 8, 4, 66, 66, 76, 100, red, material="metal")
    box(ed, -2, 8, 68, 1, 72, 96, glass, material="glass")   # windshield
    box(ed, -2, 12, 38, 1, 68, 56, dark, material="metal")   # grille
    for y in (10, 62):                                        # headlights
        box(ed, -3, y, 56, 0, y + 8, 62, [255, 244, 180], material="metal")

    # The flat bed with low walls.
    box(ed, 82, 0, 34, 228, 80, 42, bed, material="metal")
    box(ed, 82, 0, 42, 228, 6, 62, bed, material="metal")
    box(ed, 82, 74, 42, 228, 80, 62, bed, material="metal")
    box(ed, 222, 6, 42, 228, 74, 62, bed, material="metal")

    ed.call("add_point_entity", classname="light", origin=[110, 40, 220],
            properties={"light": "300"})
    ed.call("save", path="assets/truck.map")
    ed.call("render_preview", path="assets/truck_preview.png", size=700)


# --- steam locomotive -----------------------------------------------------

def locomotive(ed):
    ed.call("new_map")

    body = [134, 162, 194]
    black = [92, 92, 100]
    brass = [224, 186, 96]
    cab = [150, 176, 206]
    tyre = [96, 96, 104]

    # Frame over six wheels.
    box(ed, -10, 12, 30, 250, 68, 44, black, material="metal")
    for cx in (30, 105, 180):
        for y0, y1 in ((4, 16), (64, 76)):
            wheel(ed, cx, y0, y1, 26, 24, tyre)

    # The boiler along the frame, a smokebox front, a chimney and a steam dome.
    cyl(ed, 0, 14, 44, 170, 66, 104, body, axis="x", sides=16)
    cyl(ed, -14, 16, 46, 0, 64, 102, black, axis="x", sides=16)
    cyl(ed, 18, 34, 104, 42, 46, 140, black, axis="z", sides=12)
    sphere(ed, 90, 30, 100, 122, 50, 122, brass, material="metal")

    # The cab at the back, windows front and sides, a coal bunker behind.
    box(ed, 170, 8, 44, 240, 72, 128, cab, material="metal")
    box(ed, 174, 12, 92, 196, 68, 120, [150, 196, 214], material="glass")
    box(ed, 240, 12, 44, 262, 68, 84, black, material="metal")

    # A cowcatcher of two leaning steps.
    box(ed, -26, 16, 4, -10, 64, 20, brass, material="metal")
    box(ed, -18, 24, 18, -6, 56, 32, brass, material="metal")

    ed.call("add_point_entity", classname="light", origin=[120, 40, 260],
            properties={"light": "350"})
    ed.call("save", path="assets/locomotive.map")
    ed.call("render_preview", path="assets/locomotive_preview.png", size=720)


# --- tram -----------------------------------------------------------------

def tram(ed):
    ed.call("new_map")

    cream = [222, 210, 176]
    green = [82, 142, 100]
    glass = [150, 196, 214]
    roof = [128, 132, 138]
    tyre = [60, 60, 66]

    length, width = 300, 72

    # Skirt, body band and cream upper, under a gently arched roof.
    box(ed, 0, 0, 14, length, width, 30, green, material="metal")
    box(ed, 0, 0, 30, length, width, 58, green, material="metal")
    box(ed, 0, 0, 58, length, width, 96, cream, material="metal")
    ed.call("add_shape", kind="arch", min=[0, 0, 96], max=[length, width, 116],
            axis="x", material="metal")
    ed.call("set_face_color", color=roof)

    # Windows down both sides and the driver's window at each end.
    x = 18
    while x < length - 30:
        for y0, y1 in ((-2, 2), (width - 2, width + 2)):
            if x not in (98, 218):
                box(ed, x, y0, 62, x + 26, y1, 92, glass, material="glass")
        x += 40
    for x0, x1 in ((-2, 2), (length - 2, length + 2)):
        box(ed, x0, 14, 62, x1, width - 14, 92, glass, material="glass")

    # Bogies tucked under, and the pantograph reaching up from the roof.
    for cx in (60, length - 60):
        box(ed, cx - 28, 8, 0, cx + 28, width - 8, 16, tyre, material="metal")
    box(ed, length / 2 - 3, width / 2 - 2, 116, length / 2 + 3, width / 2 + 2, 150,
        roof, material="metal")
    box(ed, length / 2 - 34, width / 2 - 2, 148, length / 2 + 34, width / 2 + 2, 154,
        roof, material="metal")

    ed.call("add_point_entity", classname="light", origin=[length / 2, width / 2, 280],
            properties={"light": "350"})
    ed.call("save", path="assets/tram.map")
    ed.call("render_preview", path="assets/tram_preview.png", size=760)


# --- airship --------------------------------------------------------------

def airship(ed):
    ed.call("new_map")

    canvas = [216, 128, 84]
    canvas_dark = [188, 102, 66]
    wood = [150, 110, 72]
    brass = [196, 158, 72]

    # The envelope: a fat cylinder, a sphere pulled over the nose, a cone for the tail.
    cyl(ed, 60, -40, 80, 280, 40, 160, canvas, axis="x", sides=16, material="cloth")
    sphere(ed, 20, -40, 80, 100, 40, 160, canvas_dark)
    ed.call("add_shape", kind="cone", min=[280, -40, 80], max=[356, 40, 160],
            axis="x", sides=16, material="cloth")
    ed.call("set_face_color", color=canvas_dark)

    # Tail fins, a cross of four riding the taper.
    box(ed, 270, -4, 140, 340, 4, 186, canvas_dark, material="cloth")
    box(ed, 270, -4, 54, 340, 4, 100, canvas_dark, material="cloth")
    box(ed, 270, -56, 116, 340, -28, 124, canvas_dark, material="cloth")
    box(ed, 270, 28, 116, 340, 56, 124, canvas_dark, material="cloth")

    # The gondola slung beneath on two straps, with a little propeller aft.
    box(ed, 120, -18, 30, 220, 18, 66, wood, material="wood")
    box(ed, 126, -14, 66, 214, 14, 78, wood, material="wood")
    for x in (136, 204):
        for y in (-14, 10):
            box(ed, x, y, 66, x + 4, y + 4, 92, brass, material="metal")
    cyl(ed, 220, -3, 42, 232, 3, 54, brass, axis="x", sides=8)
    box(ed, 232, -1, 30, 236, 1, 66, brass, material="metal")

    ed.call("add_point_entity", classname="light", origin=[170, 0, 300],
            properties={"light": "400"})
    ed.call("save", path="assets/airship.map")
    ed.call("render_preview", path="assets/airship_preview.png", size=760)


# --- fishing boat ---------------------------------------------------------

def boat(ed):
    ed.call("new_map")

    hull = [156, 74, 58]
    deck = [190, 156, 108]
    house = [232, 228, 214]
    glass = [150, 196, 214]
    mast = [140, 102, 66]

    # The hull, a pyramid prow tapering to the stem, and a strip of gunwale.
    box(ed, 0, 0, 20, 220, 80, 56, hull, material="wood")
    ed.call("add_shape", kind="cone", min=[220, 0, 20], max=[290, 80, 56],
            axis="x", sides=4, material="wood")
    ed.call("set_face_color", color=hull)
    box(ed, 0, 0, 56, 220, 6, 64, hull, material="wood")
    box(ed, 0, 74, 56, 220, 80, 64, hull, material="wood")
    box(ed, 0, 0, 56, 8, 80, 64, hull, material="wood")

    # The deck, and the wheelhouse aft with windows all round.
    box(ed, 8, 6, 52, 220, 74, 56, deck, material="wood")
    box(ed, 30, 14, 56, 110, 66, 108, house, material="wood")
    for y0, y1 in ((10, 18), (62, 70)):
        box(ed, 40, y0, 76, 100, y1, 100, glass, material="glass")
    box(ed, 104, 20, 76, 112, 60, 100, glass, material="glass")
    ed.call("add_shape", kind="wedge", min=[30, 14, 108], max=[110, 66, 124],
            axis="x", rotate=[0, 0, 180], material="wood")
    ed.call("set_face_color", color=hull)

    # A mast forward with its boom.
    cyl(ed, 160, 36, 56, 172, 44, 190, mast, axis="z", sides=8, material="wood")
    box(ed, 130, 38, 150, 210, 42, 156, mast, material="wood")

    ed.call("add_point_entity", classname="light", origin=[140, 40, 280],
            properties={"light": "400"})
    ed.call("save", path="assets/boat.map")
    ed.call("render_preview", path="assets/boat_preview.png", size=760)


# --- propeller plane ------------------------------------------------------

def plane(ed):
    ed.call("new_map")

    body = [222, 196, 96]
    trim = [178, 64, 52]
    glass = [150, 196, 214]
    dark = [72, 74, 82]

    # The fuselage: a cylinder with a cone at each end, the rear one turned around.
    cyl(ed, 60, -24, 60, 240, 24, 108, body, axis="x", sides=12)
    ed.call("add_shape", kind="cone", min=[-10, -24, 60], max=[60, 24, 108],
            axis="x", sides=12, rotate=[0, 0, 180], material="metal")
    ed.call("set_face_color", color=trim)
    ed.call("add_shape", kind="cone", min=[240, -20, 64], max=[330, 20, 104],
            axis="x", sides=12, material="metal")
    ed.call("set_face_color", color=body)

    # The canopy, the wings low on the body, and the tail.
    box(ed, 110, -14, 104, 160, 14, 122, glass, material="glass")
    box(ed, 120, -170, 70, 176, 170, 82, trim, material="metal")
    box(ed, 290, -70, 78, 322, 70, 86, trim, material="metal")
    ed.call("add_shape", kind="wedge", min=[286, -3, 86], max=[330, 3, 140],
            axis="x", material="metal")
    ed.call("set_face_color", color=trim)

    # The propeller: a spinner and two blades set at an angle.
    ed.call("add_shape", kind="cone", min=[-30, -10, 74], max=[-10, 10, 94],
            axis="x", sides=8, rotate=[0, 0, 180], material="metal")
    ed.call("set_face_color", color=dark)
    for turn in (30, 210):
        box(ed, -24, -4, 80, -16, 4, 148, dark, material="metal")
        ed.call("rotate_selection", axis=[1, 0, 0], degrees=turn,
                center=[-20, 0, 84])

    # Fixed landing gear: two legs and a tail skid.
    for y in (-60, 52):
        box(ed, 120, y, 30, 132, y + 8, 64, dark, material="metal")
        cyl(ed, 110, y - 4, 14, 142, y + 12, 46, dark, axis="y", sides=10)
    box(ed, 296, -3, 50, 306, 3, 68, dark, material="metal")

    ed.call("add_point_entity", classname="light", origin=[150, 0, 300],
            properties={"light": "400"})
    ed.call("save", path="assets/plane.map")
    ed.call("render_preview", path="assets/plane_preview.png", size=760)


# --- windmill -------------------------------------------------------------

def windmill(ed):
    from windmill_gen import build
    build()


def main():
    for build, name in (
        (truck, "truck"),
        (locomotive, "locomotive"),
        (tram, "tram"),
        (airship, "airship"),
        (boat, "boat"),
        (plane, "plane"),
        (windmill, "windmill"),
    ):
        ed = Editor()
        try:
            build(ed)
            print(f"wrote assets/{name}.map")
        finally:
            ed.close()


if __name__ == "__main__":
    sys.exit(main())
