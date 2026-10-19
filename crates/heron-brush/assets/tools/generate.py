#!/usr/bin/env python3
"""Draws the editor's tool icons: an isometric cube with the worked element in colour.

Run from this directory. Writes select.svg, brush.svg, ... into the same folder. Needs
only the standard library.
"""
import math
import os

S = 24
OUT = os.path.dirname(__file__) or "."
C30, S30 = math.cos(math.radians(30)), math.sin(math.radians(30))


def proj(x, y, z):
    return ((x - y) * C30, (x + y) * S30 - z)


corners = [proj(x, y, z) for x in (0, 1) for y in (0, 1) for z in (0, 1)]
minx = min(p[0] for p in corners)
maxx = max(p[0] for p in corners)
miny = min(p[1] for p in corners)
maxy = max(p[1] for p in corners)
M = 0.7
SCALE = min((S - 2 * M) / (maxx - minx), (S - 2 * M) / (maxy - miny))


# The cube is taller than it is wide: centred both ways, not pushed to the left.
OX = (S - (maxx - minx) * SCALE) / 2
OY = (S - (maxy - miny) * SCALE) / 2


def P(x, y, z):
    px, py = proj(x, y, z)
    return ((px - minx) * SCALE + OX, (py - miny) * SCALE + OY)


def poly(cs, **kw):
    pts = " ".join(f"{P(*c)[0]:.2f},{P(*c)[1]:.2f}" for c in cs)
    attrs = " ".join(f'{k.replace("_", "-")}="{v}"' for k, v in kw.items())
    return f'<polygon points="{pts}" {attrs}/>'


def line(a, b, **kw):
    (x1, y1), (x2, y2) = P(*a), P(*b)
    attrs = " ".join(f'{k.replace("_", "-")}="{v}"' for k, v in kw.items())
    return f'<line x1="{x1:.2f}" y1="{y1:.2f}" x2="{x2:.2f}" y2="{y2:.2f}" {attrs}/>'


def dot(c, r, **kw):
    x, y = P(*c)
    attrs = " ".join(f'{k.replace("_", "-")}="{v}"' for k, v in kw.items())
    return f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r}" {attrs}/>'


STROKE, SW = "#8a8f98", 1.7
ACC, ACC2, ACCG = "#3584e4", "#e66100", "#2ec27e"
FACE_TOP = [(0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)]
FACE_RIGHT = [(1, 0, 0), (1, 1, 0), (1, 1, 1), (1, 0, 1)]
FACE_FRONT = [(0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1)]
HEX = [(0, 0, 1), (1, 0, 1), (1, 0, 0), (1, 1, 0), (0, 1, 0), (0, 1, 1)]


def cube(extra="", faint=True):
    parts = []
    if faint:
        for face in (FACE_TOP, FACE_RIGHT, FACE_FRONT):
            parts.append(poly(face, fill="#8a8f98", fill_opacity="0.09", stroke="none"))
    parts.append(
        f'<g fill="none" stroke="{STROKE}" stroke-width="{SW}" '
        'stroke-linejoin="round" stroke-linecap="round">'
    )
    parts.append(poly(HEX, fill="none"))
    for a, b in [((1, 1, 1), (1, 0, 1)), ((1, 1, 1), (1, 1, 0)), ((1, 1, 1), (0, 1, 1))]:
        parts.append(line(a, b))
    parts.append(extra)
    parts.append("</g>")
    return "".join(parts)


def svg(body):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {S} {S}">{body}</svg>'


p111, p101, p011, p110 = P(1, 1, 1), P(1, 0, 1), P(0, 1, 1), P(1, 1, 0)
icons = {}

icons["select"] = svg(cube(
    f'<g transform="translate({p111[0] - 1:.1f},{p111[1] - 2:.1f})">'
    f'<path d="M0 0 L0 9 L2.4 6.8 L4 10.4 L5.6 9.6 L4 6 L7 6 Z" fill="{ACC}" '
    'stroke="#0d3050" stroke-width="0.7" stroke-linejoin="round"/></g>'))

icons["brush"] = svg(
    poly([(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)], fill="none", stroke=STROKE,
         stroke_opacity="0.7", stroke_width="1.3", stroke_dasharray="2.4 1.8")
    + poly([(0, 0, 0.55), (1, 0, 0.55), (1, 1, 0.55), (0, 1, 0.55)], fill="none",
           stroke=STROKE, stroke_opacity="0.7", stroke_width="1.3", stroke_dasharray="2.4 1.8")
    + cube())

# The plane slices the cube. The piece above the cut is what a clip throws away,
# so it is drawn faint and dashed -- fading out -- while the piece kept below it
# stays solid. The plane meets each of the four vertical edges at these heights;
# on the near-left edge that is the very top corner, so the front face keeps all
# but a sliver.
_bl, _rb, _fr, _lf = (0, 0, 0.6), (1, 0, 0.2), (1, 1, 0.6), (0, 1, 1.0)
_cut = [_bl, _rb, _fr, _lf]
_keep_faces = [
    [(1, 0, 0), (1, 1, 0), _fr, _rb],       # right face, below the cut
    [(0, 1, 0), (1, 1, 0), _fr, _lf],       # front face, below the cut
]
_gone_faces = [
    FACE_TOP,
    [_rb, _fr, (1, 1, 1), (1, 0, 1)],       # right face, above the cut
    [_fr, (1, 1, 1), (0, 1, 1)],            # front face, the leftover sliver
]
icons["clip"] = svg(
    "".join(
        poly(f, fill="#8a8f98", fill_opacity="0.035", stroke="#8a8f98",
             stroke_opacity="0.30", stroke_width="1", stroke_dasharray="1.7 1.5",
             stroke_linejoin="round")
        for f in _gone_faces
    )
    + "".join(
        poly(f, fill="#8a8f98", fill_opacity="0.11", stroke=STROKE,
             stroke_width=SW, stroke_linejoin="round", stroke_linecap="round")
        for f in _keep_faces
    )
    + poly(_cut, fill=ACC2, fill_opacity="0.22", stroke=ACC2, stroke_width="1.8",
           stroke_linejoin="round"))

icons["vertex"] = svg(cube(dot((1, 1, 1), 3.2, fill=ACC, stroke="#0d3050", stroke_width="0.9")))

icons["edge"] = svg(cube(line((1, 1, 1), (1, 0, 1), stroke=ACC, stroke_width="3.6",
                              stroke_linecap="round")))

icons["face"] = svg(cube(poly(FACE_TOP, fill=ACC, fill_opacity="0.55", stroke=ACC,
                              stroke_width="1.7")))

icons["rotate"] = svg(cube(
    f'<path d="M {p101[0] - 0.5:.1f} {p101[1] - 3.2:.1f} A 9 9 0 1 1 '
    f'{p011[0] - 0.5:.1f} {p011[1] - 3.2:.1f}" fill="none" stroke="{ACC}" '
    'stroke-width="2" stroke-linecap="round"/>'
    f'<path d="M {p011[0] - 4.2:.1f} {p011[1] - 5.6:.1f} l 3.2 2 l -3.2 2.8 Z" fill="{ACC}"/>'))

cc = P(1, 1, 1)
icons["scale"] = svg(cube(
    f'<g stroke="{ACC}" stroke-width="2" stroke-linecap="round" fill="none">'
    f'<line x1="{cc[0] - 1:.1f}" y1="{cc[1] + 1:.1f}" x2="{cc[0] + 5:.1f}" y2="{cc[1] + 7:.1f}"/>'
    f'<path d="M {cc[0] - 1:.1f} {cc[1] + 1:.1f} l 4 0 m -4 0 l 0 4"/>'
    f'<path d="M {cc[0] + 5:.1f} {cc[1] + 7:.1f} l -4 0 m 4 0 l 0 -4"/></g>'))


def _skew(x, y, z, k=2.0):
    px, py = proj(x, y, z)
    px += k * z
    return ((px - minx) * SCALE + M, (py - miny) * SCALE + M)


skew_top = " ".join(f"{_skew(*p)[0]:.2f},{_skew(*p)[1]:.2f}" for p in FACE_TOP)
icons["shear"] = svg(cube(
    f'<polygon points="{skew_top}" fill="{ACC}" fill-opacity="0.34" stroke="{ACC}" stroke-width="1.6"/>'
    f'<g stroke="{ACC}" stroke-width="1.7" stroke-linecap="round" fill="none">'
    '<line x1="2" y1="4" x2="8.5" y2="4"/><path d="M8.5 4 l -2.2 -1.8 m 2.2 1.8 l -2.2 1.8"/></g>'))

piece = poly([(1, 1, 1), (1, 1, 0.45), (0.45, 1, 0.45), (0.45, 1, 1)],
             fill=ACCG, fill_opacity="0.6", stroke=ACCG, stroke_width="1.3")
icons["assemble"] = svg(cube(
    f'<g transform="translate(5,-5)">{piece}</g>'
    f'<line x1="{p110[0]:.1f}" y1="{p110[1]:.1f}" x2="{p110[0] + 5:.1f}" y2="{p110[1] - 5:.1f}" '
    f'stroke="{ACCG}" stroke-width="1.2" stroke-dasharray="1.6 1.4"/>'))

profile = poly([(0, 0, 0), (0, 0, 1), (0, 1, 1), (0, 1, 0)],
               fill=ACC, fill_opacity="0.4", stroke=ACC, stroke_width="1.6")
sa, sb = P(0, 0.5, 0.5), P(1, 0.5, 0.5)
icons["sweep"] = svg(
    profile + cube(faint=False)
    + f'<path d="M {sa[0]:.1f} {sa[1]:.1f} L {sb[0] + 2:.1f} {sb[1]:.1f}" stroke="{ACC}" '
      'stroke-width="1.7" stroke-dasharray="2.2 1.6" stroke-linecap="round"/>'
    + f'<path d="M {sb[0] + 2:.1f} {sb[1]:.1f} l -3.2 -2.3 m 3.2 2.3 l -3.2 2.3" '
      f'stroke="{ACC}" stroke-width="1.7" fill="none"/>')

icons["controlpoint"] = svg(
    f'<path d="M3.5 {S - 4} C 8 3, {S - 8} {S - 2}, {S - 3.5} 5" fill="none" '
    f'stroke="{ACC}" stroke-width="2"/>'
    f'<g stroke="{ACC2}" stroke-width="1" stroke-dasharray="1.8 1.3">'
    f'<line x1="3.5" y1="{S - 4}" x2="8" y2="3"/><line x1="{S - 3.5}" y1="5" x2="{S - 8}" y2="{S - 2}"/></g>'
    f'<g fill="{ACC2}" stroke="#7a3200" stroke-width="0.7">'
    f'<circle cx="3.5" cy="{S - 4}" r="2.2"/><circle cx="{S - 3.5}" cy="5" r="2.2"/></g>')

p100 = P(1, 0, 0)
# A filleted edge, the way a CAD app rounds one over: the cube's top-right edge, where
# the top and right faces meet, is rolled into a big quarter-round. Both faces stop short
# of where the sharp edge was; a curved band in colour spans the gap, with a highlight
# down its crown and a rounded cap at each end so it reads as a rolled edge, not a
# bevel. Radius is deliberately fat, as in the reference sketch.
FR = 0.34
_ft1, _ft2 = P(1 - FR, 1, 1), P(1 - FR, 0, 1)       # edge drawn back across the top face
_fr1, _fr2 = P(1, 1, 1 - FR), P(1, 0, 1 - FR)       # edge drawn back down the right face
_fk = FR * 0.7071
_fm1, _fm2 = P(1 - FR + _fk, 1, 1 - FR + _fk), P(1 - FR + _fk, 0, 1 - FR + _fk)  # crown


def _fbez(a, m, b):
    # a quadratic Bezier from a to b that passes through m
    cx, cy = 2 * m[0] - (a[0] + b[0]) / 2, 2 * m[1] - (a[1] + b[1]) / 2
    return f'Q {cx:.2f} {cy:.2f} {b[0]:.2f} {b[1]:.2f}'


icons["fillet"] = svg(
    cube()
    # the fillet surface: a fat quarter-round spanning the two faces, opaque so the old
    # sharp edge under it is gone
    + f'<path d="M {_ft1[0]:.2f} {_ft1[1]:.2f} L {_ft2[0]:.2f} {_ft2[1]:.2f} '
      f'{_fbez(_ft2, _fm2, _fr2)} L {_fr1[0]:.2f} {_fr1[1]:.2f} '
      f'{_fbez(_fr1, _fm1, _ft1)} Z" '
      f'fill="{ACC}" stroke="{ACC}" stroke-width="1.4" stroke-linejoin="round"/>'
    # the two edges where the round meets the flat faces
    + f'<g fill="none" stroke="{ACC}" stroke-width="1.2" stroke-linecap="round" stroke-opacity="0.65">'
    + f'<line x1="{_ft1[0]:.2f}" y1="{_ft1[1]:.2f}" x2="{_ft2[0]:.2f}" y2="{_ft2[1]:.2f}"/>'
    + f'<line x1="{_fr1[0]:.2f}" y1="{_fr1[1]:.2f}" x2="{_fr2[0]:.2f}" y2="{_fr2[1]:.2f}"/></g>')

icons["paint"] = svg(cube(
    poly(FACE_RIGHT, fill=ACC2, fill_opacity="0.45", stroke=ACC2, stroke_width="1.6")
    + f'<g transform="translate({S - 6},{S - 8.5})">'
      f'<path d="M0 5.5 C -3 2.5 -3 -0.5 0 -4.5 C 3 -0.5 3 2.5 0 5.5 Z" fill="{ACC2}" '
      'stroke="#7a3200" stroke-width="0.8"/></g>'))

for name, body in icons.items():
    with open(f"{OUT}/{name}.svg", "w") as handle:
        handle.write(body)
print("wrote", ", ".join(sorted(icons)))

# --- toolbar-row icons that are not tools: duplicate and the two flips ---
def _two_cubes():
    # a small cube, and a second offset copy of it
    def small(ox, oy, fade):
        g = [f'<g transform="translate({ox},{oy}) scale(0.62)" fill="none" '
             f'stroke="{STROKE}" stroke-opacity="{fade}" stroke-width="{SW*1.4:.1f}" '
             'stroke-linejoin="round" stroke-linecap="round">']
        g.append(poly(HEX, fill="#8a8f98", fill_opacity=f"{0.09*float(fade):.3f}"))
        for a, b in [((1,1,1),(1,0,1)), ((1,1,1),(1,1,0)), ((1,1,1),(0,1,1))]:
            g.append(line(a, b))
        g.append("</g>")
        return "".join(g)
    return svg(small(9, 9, "0.55") + small(0, 0, "1"))
open(f"{OUT}/duplicate.svg", "w").write(_two_cubes())

def _flip(horizontal):
    # a cube, its mirror ghosted across a dashed axis, and a two-headed arrow
    axis = (f'<line x1="{S/2}" y1="1" x2="{S/2}" y2="{S-1}" stroke="{STROKE}" '
            'stroke-width="1" stroke-dasharray="2 1.6"/>') if horizontal else (
            f'<line x1="1" y1="{S/2}" x2="{S-1}" y2="{S/2}" stroke="{STROKE}" '
            'stroke-width="1" stroke-dasharray="2 1.6"/>')
    tf = f'translate({S},0) scale(-1,1)' if horizontal else f'translate(0,{S}) scale(1,-1)'
    ghost = (f'<g transform="{tf}" fill="none" stroke="{STROKE}" stroke-opacity="0.4" '
             f'stroke-width="{SW}" stroke-linejoin="round">'
             + poly(HEX, fill="none")
             + "".join(line(a, b) for a, b in [((1,1,1),(1,0,1)), ((1,1,1),(1,1,0)), ((1,1,1),(0,1,1))])
             + "</g>")
    if horizontal:
        arrow = (f'<g stroke="{ACC}" stroke-width="1.8" stroke-linecap="round" fill="none">'
                 f'<line x1="4" y1="{S-4}" x2="{S-4}" y2="{S-4}"/>'
                 f'<path d="M4 {S-4} l 2.6 -2.4 m -2.6 2.4 l 2.6 2.4"/>'
                 f'<path d="M{S-4} {S-4} l -2.6 -2.4 m 2.6 2.4 l -2.6 2.4"/></g>')
    else:
        arrow = (f'<g stroke="{ACC}" stroke-width="1.8" stroke-linecap="round" fill="none">'
                 f'<line x1="{S-4}" y1="4" x2="{S-4}" y2="{S-4}"/>'
                 f'<path d="M{S-4} 4 l -2.4 2.6 m 2.4 -2.6 l 2.4 2.6"/>'
                 f'<path d="M{S-4} {S-4} l -2.4 -2.6 m 2.4 2.6 l 2.4 -2.6"/></g>')
    return svg(axis + ghost + cube() + arrow)
open(f"{OUT}/flip-horizontal.svg", "w").write(_flip(True))
open(f"{OUT}/flip-vertical.svg", "w").write(_flip(False))
print("wrote duplicate, flip-horizontal, flip-vertical")
