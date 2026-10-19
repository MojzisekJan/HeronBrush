"""Writes curves.map beside this script: round shapes out of flat brushes.

A brush has flat faces, so anything curved is drawn as segments. That costs
less than it looks: the faces where neighbouring segments meet are exactly
equal and opposite, and the exporter throws every one of them away. What is left is the
outside of the shape and nothing else. `_phong` then averages the normals along
the seams, so sixteen flat faces shade like one round wall.
"""

import math

OUT = str(__import__("pathlib").Path(__file__).with_name("curves.map"))
parts = []

# Texture axes per dominant face normal, the way Quake picks them.
AXES = [
    ((0, 0, 1), (1, 0, 0), (0, -1, 0)),
    ((0, 0, -1), (1, 0, 0), (0, -1, 0)),
    ((1, 0, 0), (0, 1, 0), (0, 0, -1)),
    ((-1, 0, 0), (0, 1, 0), (0, 0, -1)),
    ((0, 1, 0), (1, 0, 0), (0, 0, -1)),
    ((0, -1, 0), (1, 0, 0), (0, 0, -1)),
]


def face(p1, p2, p3, tex):
    u = [p2[i] - p1[i] for i in range(3)]
    v = [p3[i] - p1[i] for i in range(3)]
    n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
    best = max(AXES, key=lambda a: sum(a[0][i] * n[i] for i in range(3)))
    return "( %.6g %.6g %.6g ) ( %.6g %.6g %.6g ) ( %.6g %.6g %.6g ) %s [ %g %g %g 0 ] [ %g %g %g 0 ] 0 1 1" % (
        p1 + p2 + p3 + (tex,) + best[1] + best[2])


def brush(faces):
    return "{\n" + "\n".join(faces) + "\n}"


def entity(classname, props=None, brushes=()):
    lines = ["{", '"classname" "%s"' % classname]
    for key, value in (props or {}).items():
        lines.append('"%s" "%s"' % (key, value))
    lines.extend(brushes)
    parts.append("\n".join(lines + ["}"]))


def box(x0, y0, z0, x1, y1, z1, tex, top=None):
    t = top or tex
    return brush([
        face((x0, y0, z0), (x0, y0 + 1, z0), (x0, y0, z0 + 1), tex),
        face((x0, y0, z0), (x0, y0, z0 + 1), (x0 + 1, y0, z0), tex),
        face((x0, y0, z0), (x0 + 1, y0, z0), (x0, y0 + 1, z0), tex),
        face((x0, y0, z1), (x0, y0 + 1, z1), (x0 + 1, y0, z1), t),
        face((x0, y1, z0), (x0 + 1, y1, z0), (x0, y1, z0 + 1), tex),
        face((x1, y0, z0), (x1, y0, z0 + 1), (x1, y0 + 1, z0), tex),
    ])


def ring(cx, cy, inner, outer, z0, z1, tex, segments=16, top_inner=None, top_outer=None):
    """A curved wall: one wedge brush per segment, laid round a centre.

    Give `top_inner` and `top_outer` to make the wall lean, which is what turns
    a stack of these into a dome rather than a wedding cake. A cone's surface is
    ruled, so each leaning face is still flat and the brush stays convex.
    """
    ti = inner if top_inner is None else top_inner
    to = outer if top_outer is None else top_outer
    out = []
    for i in range(segments):
        a0 = math.radians(360 * i / segments)
        a1 = math.radians(360 * (i + 1) / segments)
        at = lambda r, a: (cx + r * math.cos(a), cy + r * math.sin(a))
        i0, i1 = at(inner, a0), at(inner, a1)
        o0, o1 = at(outer, a0), at(outer, a1)
        j0, j1 = at(ti, a0), at(ti, a1)
        p0, p1 = at(to, a0), at(to, a1)
        out.append(brush([
            face((i0[0], i0[1], z0), (o0[0], o0[1], z0), (i1[0], i1[1], z0), tex),
            face((j0[0], j0[1], z1), (j1[0], j1[1], z1), (p0[0], p0[1], z1), tex),
            face((i0[0], i0[1], z0), (j0[0], j0[1], z1), (o0[0], o0[1], z0), tex),
            face((i1[0], i1[1], z0), (o1[0], o1[1], z0), (j1[0], j1[1], z1), tex),
            face((i0[0], i0[1], z0), (i1[0], i1[1], z0), (j0[0], j0[1], z1), tex),
            face((o0[0], o0[1], z0), (p0[0], p0[1], z1), (o1[0], o1[1], z0), tex),
        ]))
    return out


def arch(cx, cy, y0, y1, inner, outer, tex, segments=12):
    """Half a ring stood on its side: a doorway with a round top."""
    out = []
    for i in range(segments):
        a0 = math.radians(180 * i / segments)
        a1 = math.radians(180 * (i + 1) / segments)
        i0 = (cx + inner * math.cos(a0), inner * math.sin(a0))
        i1 = (cx + inner * math.cos(a1), inner * math.sin(a1))
        o0 = (cx + outer * math.cos(a0), outer * math.sin(a0))
        o1 = (cx + outer * math.cos(a1), outer * math.sin(a1))
        # Wound the other way round from `ring`: the arc turns in xz while the
        # extrusion runs along y, and that swaps the handedness.
        out.append(brush([
            face((i0[0], y0, i0[1]), (o0[0], y0, o0[1]), (i0[0], y1, i0[1]), tex),
            face((i1[0], y0, i1[1]), (i1[0], y1, i1[1]), (o1[0], y0, o1[1]), tex),
            face((i0[0], y0, i0[1]), (i1[0], y0, i1[1]), (o0[0], y0, o0[1]), tex),
            face((i0[0], y1, i0[1]), (o0[0], y1, o0[1]), (i1[0], y1, i1[1]), tex),
            face((i0[0], y0, i0[1]), (i0[0], y1, i0[1]), (i1[0], y0, i1[1]), tex),
            face((o0[0], y0, o0[1]), (o1[0], y0, o1[1]), (o0[0], y1, o0[1]), tex),
        ]))
    return out


def prism(cx, cy, radius, z0, z1, tex, segments=16):
    """A solid convex column of `segments` sides. One brush, not a ring."""
    sides = []
    for i in range(segments):
        a0 = math.radians(360 * i / segments)
        a1 = math.radians(360 * (i + 1) / segments)
        p0 = (cx + radius * math.cos(a0), cy + radius * math.sin(a0))
        p1 = (cx + radius * math.cos(a1), cy + radius * math.sin(a1))
        sides.append(face((p0[0], p0[1], z0), (p1[0], p1[1], z0), (p0[0], p0[1], z1), tex))
    sides.append(face((cx, cy, z0), (cx + 1, cy, z0), (cx, cy + 1, z0), tex))
    sides.append(face((cx, cy, z1), (cx, cy + 1, z1), (cx + 1, cy, z1), tex))
    return [brush(sides)]


def dome(cx, cy, radius, thickness, tex, segments=16, rings=6):
    """Half a sphere, as rings that get smaller towards the top.

    The last ring is left off and capped instead: at the pole the inner radius
    reaches zero and the wedges collapse into nothing.
    """
    out = []
    for r in range(rings - 1):
        t0 = math.radians(90 * r / rings)
        t1 = math.radians(90 * (r + 1) / rings)
        out.extend(ring(cx, cy,
                        radius * math.cos(t0), (radius + thickness) * math.cos(t0),
                        radius * math.sin(t0), radius * math.sin(t1),
                        tex, segments,
                        top_inner=radius * math.cos(t1),
                        top_outer=(radius + thickness) * math.cos(t1)))
    t = math.radians(90 * (rings - 1) / rings)
    out.extend(prism(cx, cy, (radius + thickness) * math.cos(t),
                     radius * math.sin(t), radius + thickness * 0.5, tex, segments))
    return out


X0, X1 = -640, 640
Y0, Y1 = -512, 512
CEIL = 448

# The room.
entity("worldspawn", {"_phong": "0", "_tb_textures": "textures"}, [
    box(X0, Y0, -32, X1, Y1, 0, "grass"),
    box(X0, Y0, CEIL, X1, Y1, CEIL + 32, "concrete"),
    box(X0 - 32, Y0, 0, X0, Y1, CEIL, "concrete"),
    box(X1, Y0, 0, X1 + 32, Y1, CEIL, "concrete"),
    box(X0, Y0 - 32, 0, X1, Y0, CEIL, "concrete"),
    box(X0, Y1, 0, X1, Y1 + 32, CEIL, "concrete"),
])

# A round tower to walk into, smooth shaded. Sixteen wedges, and the faces
# where they meet are gone before Godot ever sees them.
entity("func_geo", {"_phong": "1", "_phong_angle": "60"},
       ring(-320, 0, 176, 208, 0, 320, "metal", segments=16))

# The same shape left faceted, so the two can be compared side by side.
entity("func_geo", {"_phong": "0"},
       ring(320, 0, 176, 208, 0, 320, "metal", segments=16))

# An arch to walk under.
entity("func_geo", {"_phong": "1", "_phong_angle": "60"},
       arch(0, 0, -48, 48, 128, 176, "concrete", segments=12))

# And a dome, which is rings of decreasing radius stacked up.
entity("func_geo", {"_phong": "1", "_phong_angle": "60"},
       dome(0, 384, 160, 24, "dirt", segments=16, rings=6))

# And the other way to curve something: a bezier patch, which is a grid of
# control points rather than brushes. Editors offer these when the map is
# in the Quake 3 (Valve) format.
parts.append("""{
"classname" "func_geo"
{
patchDef2
{
metal
( 3 3 0 0 0 )
(
( ( -192 -448 0 0 0 ) ( -192 -448 224 0 -2 ) ( -192 -192 288 0 -4 ) )
( ( 0 -512 0 2 0 ) ( 0 -512 288 2 -2 ) ( 0 -192 352 2 -4 ) )
( ( 192 -448 0 4 0 ) ( 192 -448 224 4 -2 ) ( 192 -192 288 4 -4 ) )
)
}
}
}""")

entity("info_player_start", {"origin": "0 -384 32", "angles": "0 90 0"})
entity("light_sun", {"angles": "-50 130 0", "light_energy": "1.1",
                     "light_color": "255 244 214"})
entity("light", {"origin": "-320 0 260", "light_energy": "3",
                 "light_color": "180 220 255"})
entity("light", {"origin": "320 0 260", "light_energy": "3",
                 "light_color": "180 220 255"})
entity("light", {"origin": "0 384 200", "light_energy": "3",
                 "light_color": "255 220 180"})

open(OUT, "w").write("// Generated by make_curves.py\n" + "\n".join(parts) + "\n")
print("wrote %s" % OUT)
