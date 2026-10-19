#!/usr/bin/env python3
"""Build the low-poly dragon prefab using convex brushes and no Python dependencies.

Run from the repository root: python3 assets/dragon_gen.py
The editor validates the saved map and renders both preview images.
"""

import itertools
import json
import math
import subprocess
from pathlib import Path

from prefabs_gen import Editor

SKIN = (55, 140, 114)
LIGHT = (76, 161, 126)
DARK = (39, 103, 92)
BELLY = (216, 186, 123)
BONE = (239, 220, 170)
WING = (191, 101, 62)
WING_LIGHT = (219, 132, 77)
MOUTH = (57, 32, 37)
BRUSHES = []


def add(a, b):
    return tuple(x + y for x, y in zip(a, b))


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def mul(a, s):
    return tuple(x * s for x in a)


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])


def normal(a):
    return mul(a, 1 / math.sqrt(dot(a, a)))


def hull(points, color):
    """Supporting planes of a small convex point cloud, wound for Quake maps."""
    planes = {}
    for a, b, c in itertools.combinations(points, 3):
        n = cross(sub(b, a), sub(c, a))
        if dot(n, n) < 1e-10:
            continue
        n = normal(n)
        d = dot(n, a)
        distances = [dot(n, p) - d for p in points]
        if max(distances) > 1e-6:
            if min(distances) < -1e-6:
                continue
            n, d = mul(n, -1), -d
        key = tuple(round(v, 5) for v in (*n, d))
        if key in planes:
            continue
        # Quake's map planes use the opposite winding to a standard cross product.
        u = normal(cross(n, (0, 0, 1) if abs(n[2]) < .9 else (0, 1, 0)))
        v = cross(n, u)
        anchor = mul(n, d)
        planes[key] = (anchor, add(anchor, mul(v, 64)), add(anchor, mul(u, 64)))
    assert len(planes) >= 4, 'flat brush'
    BRUSHES.append((list(planes.values()), color))


def ring(center, ry, rz, sides=8):
    return [(center[0], center[1]+ry*math.cos(i*2*math.pi/sides),
             center[2]+rz*math.sin(i*2*math.pi/sides)) for i in range(sides)]


def body_part(a, b, radii_a, radii_b, color):
    hull(ring(a, *radii_a) + ring(b, *radii_b), color)


def bone(a, b, r0, r1, color, sides=6):
    """Tapered limb in a local frame; joints meet regardless of direction."""
    direction = normal(sub(b, a))
    u = normal(cross(direction, (0, 0, 1) if abs(direction[2]) < .9 else (0, 1, 0)))
    v = cross(direction, u)
    points = []
    for center, radius in ((a, r0), (b, r1)):
        if radius == 0:
            points.append(center)
        else:
            for i in range(sides):
                angle = 2*math.pi*i/sides
                points.append(add(center, add(mul(u, radius*math.cos(angle)), mul(v, radius*math.sin(angle)))))
    hull(points, color)


def ellipsoid(center, size, color):
    points = []
    for x, width in ((-1, 0), (-.6, .8), (.3, 1), (1, 0)):
        c = add(center, (x*size[0], 0, 0))
        points.extend(ring(c, size[1]*width, size[2]*width) if width else [c])
    hull(points, color)


def box(lo, hi, color):
    hull(list(itertools.product(*zip(lo, hi))), color)


def membrane(a, b, c, color):
    n = mul(normal(cross(sub(b, a), sub(c, a))), .8)
    hull([add(p, n) for p in (a,b,c)] + [sub(p, n) for p in (a,b,c)], color)


def build():
    BRUSHES.clear()
    # A broad ribcage, powerful shoulders and a narrower waist.
    body_part((-62,0,54), (-24,0,64), (14,17), (28,29), DARK)
    body_part((-24,0,64), (24,0,76), (28,29), (30,34), SKIN)
    body_part((24,0,76), (46,0,96), (30,34), (18,23), LIGHT)
    # A rising, tapered neck with a continuous throat.
    bone((35,0,91), (47,0,119), 20, 16, LIGHT, 8)
    bone((47,0,119), (67,0,143), 16, 12, SKIN, 8)
    bone((44,0,85), (58,0,113), 12, 10, BELLY, 6)
    bone((58,0,113), (77,0,133), 10, 8, BELLY, 6)
    for x,z,width in ((-18,41,17),(-2,46,21),(14,51,22),(29,61,19)):
        ellipsoid((x,0,z), (11,width,4), BELLY)

    # A long wedge muzzle, separated lower jaw and small inset eyes.
    body_part((62,0,145), (91,0,149), (12,15), (20,18), LIGHT)
    body_part((91,0,149), (121,0,143), (20,13), (12,8), SKIN)
    body_part((82,0,131), (121,0,130), (13,4), (11,3), MOUTH)
    body_part((77,0,125), (117,0,124), (13,6), (10,4), DARK)
    for side in (-1,1):
        s = side
        ellipsoid((91,s*18,153), (7,3.4,4), (242,172,42))
        ellipsoid((93,s*20.7,153), (1.4,.8,3.2), (24,30,26))
        bone((80,s*19,159),(101,s*17,156),4,2.5,DARK)
        ellipsoid((115,s*10,147), (3,1.3,1.8), MOUTH)
        # Swept horns and smaller cheek horns.
        bone((72,s*13,160),(59,s*20,181),6,3.3,BONE)
        bone((59,s*20,181),(41,s*24,193),3.3,0,BONE)
        bone((72,s*15,141),(53,s*26,145),5,0,BONE)
        for x in (101,114):
            bone((x,s*10,135),(x+1,s*10,127),2.2,0,BONE,4)
        bone((108,s*9,129),(110,s*9,135),1.6,0,BONE,4)

    # Four planted feet, bent limbs and a strong rear haunch.
    for side in (-1,1):
        s=side
        ellipsoid((-38,s*22,53),(22,16,24),SKIN)
        bone((-40,s*27,50),(-53,s*39,29),13,9,DARK)
        bone((-53,s*39,29),(-33,s*43,10),9,6,LIGHT)
        ellipsoid((-24,s*44,7),(19,11,7),SKIN)
        bone((29,s*23,70),(45,s*36,40),10,8,SKIN)
        bone((45,s*36,40),(38,s*43,11),8,5.5,LIGHT)
        ellipsoid((49,s*44,7),(18,10,7),SKIN)
        for foot_x in (-12,61):
            for offset in (-6,0,6):
                bone((foot_x,s*44+offset,5),(foot_x+11,s*44+offset,1.5),3,0,BONE,4)

    # One continuous tapered tail, with an upturned tip.
    tail=[((-60,0,55),15),((-88,-2,42),12),((-118,-7,30),9),
          ((-148,-14,24),6.5),((-177,-18,31),4),((-194,-10,48),2),((-199,3,61),0)]
    for (a,r0),(b,r1) in zip(tail,tail[1:]):
        bone(a,b,r0,r1,SKIN if r0<10 else DARK,8)
    for x,z,h in ((-142,31,10),(-108,42,13),(-75,58,16),(-48,75,17),
                  (-25,91,19),(-3,102,19),(19,111,17),(36,124,14),(49,142,10)):
        hull([(x-9,-3,z),(x+7,-3,z),(x-9,3,z),(x+7,3,z),(x-8,0,z+h)],BONE)

    # Scalloped bat wings: broad sheets between fingers, no crossing struts.
    for side in (-1,1):
        def w(x,y,z):
            # Raise the nearer wing so its membrane reads in the library's isometric view.
            return (x,side*y,z + (.7*(y-20) if side == 1 else 0))
        root=w(12,20,93)
        elbow=w(3,60,128)
        wrist=w(10,105,151)
        tips=[w(-12,180,177),w(-77,162,126),w(-107,120,98),w(-85,70,82),w(-48,25,71)]
        bone(root,elbow,9,6,SKIN)
        bone(elbow,wrist,6,4.5,LIGHT)
        membrane(root,elbow,tips[0],WING_LIGHT)
        membrane(elbow,wrist,tips[0],WING_LIGHT)
        for i,(a,b) in enumerate(zip(tips,tips[1:])):
            # Inset trailing points form a concave outline between the fingers.
            middle=add(mul(add(a,b),.5),mul(sub(wrist,mul(add(a,b),.5)),.23))
            membrane(wrist,a,middle,WING_LIGHT if i%2==0 else WING)
            membrane(wrist,middle,b,WING)
        membrane(root,wrist,tips[-1],WING)
        for i,tip in enumerate(tips):
            bone(wrist,tip,3.5 if i==0 else 2.8,.9,DARK)
        bone(wrist,w(27,114,167),3,0,BONE)


def main():
    build()
    lines=['// Format: Standard', '// heron_brush format 1', '{', '"classname" "worldspawn"']
    for planes,color in BRUSHES:
        lines += ['// heron_brush color '+' '.join(map(str,color)), '{']
        for plane in planes:
            points=' '.join('( '+' '.join(f'{n:.8f}' for n in point)+' )' for point in plane)
            lines.append(points+' __TB_empty 0 0 0 1 1')
        lines.append('}')
    lines.append('}')
    path=Path('assets/dragon.map')
    path.write_text('\n'.join(lines)+'\n')
    ed=Editor()
    try:
        ed.call('open_map',path=str(path))
        count=len(json.loads(ed.call('list_brushes')))
        assert count==len(BRUSHES), f'Invalid brushes: {count}/{len(BRUSHES)} loaded'
        ed.call('save',path=str(path))
        ed.call('open_map',path=str(path))
        assert len(json.loads(ed.call('list_brushes')))==count
        ed.call('render_preview',path='assets/dragon_preview.png',size=1000)
        ed.call('render_preview',path='assets/dragon.png',size=512)
        print(f'Wrote {path}: {count} valid brushes, verified after save/reload')
    finally:
        ed.close()


if __name__=='__main__':
    main()
