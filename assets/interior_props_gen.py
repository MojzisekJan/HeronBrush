#!/usr/bin/env python3
"""Compact coordinated interior props, authored at 32 units/metre on a 1/8 grid."""
import itertools
import json
import sys
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import box, hull
from fan_gen import brush_text
from prefabs_gen import Editor

WOOD=(174,119,70); DARKWOOD=(111,72,47); TEAL=(60,124,127)
CREAM=(229,221,192); STEEL=(155,172,173); DARK=(44,56,62)
RED=(197,75,51); GOLD=(223,173,73); BLUE=(85,153,179)
SURFACES={WOOD:(.48,0),DARKWOOD:(.58,0),TEAL:(.32,0),CREAM:(.30,0),
          STEEL:(.22,1),DARK:(.8,0),RED:(.68,0),GOLD:(.75,0),BLUE:(.18,0)}


def bevel(x0,y0,z0,x1,y1,z1,b,color):
    """A single convex brush with bevels on all twelve edges."""
    bounds=[(x0,x1),(y0,y1),(z0,z1)]
    points=[]
    for axis in range(3):
        for side in (0,1):
            for signs in itertools.product((0,1),repeat=2):
                p=[];j=0
                for k,(lo,hi) in enumerate(bounds):
                    if k==axis:p.append((lo,hi)[side])
                    else:p.append((lo+b,hi-b)[signs[j]]);j+=1
                points.append(tuple(p))
    hull(points,color)


def chair_dining():
    # The rear legs continue into the back supports; all four legs meet the seat.
    for x in (-5.5,5.5):
        for y in (-5,5):
            top=29 if y<0 else 14.5
            box(x-1,y-1,0,x+1,y+1,top,DARKWOOD)
    bevel(-7.5,-7,13.5,7.5,7,16,.5,WOOD)
    bevel(-7,-6.5,22,7,-4.5,30,.5,TEAL)
    for x in (-5.5,5.5):box(x-.5,-5,6,x+.5,5,7.25,DARKWOOD)
    box(-5.5,-5.5,6,5.5,-4.5,7.25,DARKWOOD)


def table_dining():
    for x in (-19,19):
        for y in (-9,9):
            # Square tapered legs retain broad, planar faces and a clean silhouette.
            hull([(x+dx*w,y+dy*w,z) for z,w in [(0,1),(23,1.5)]
                  for dx in (-1,1) for dy in (-1,1)],DARKWOOD)
    for y in (-9,9):box(-19,y-.75,20,19,y+.75,23.5,DARKWOOD)
    for x in (-19,19):box(x-.75,-9,20,x+.75,9,23.5,DARKWOOD)
    bevel(-24,-14,23,24,14,25,.5,WOOD)


def refrigerator():
    box(-9,-8.5,0,9,8.5,2,DARK)
    bevel(-11,-11,1,11,9,60,1,TEAL)
    # Two separated enamel doors with a visible dark gasket line.
    box(-10.25,9,3,10.25,9.5,59,DARK)
    bevel(-10.5,9.5,3,10.5,12,43,.5,CREAM)
    bevel(-10.5,9.5,43.5,10.5,12,59,.5,CREAM)
    for z0,z1 in [(32,40),(46,54)]:
        box(-7.5,12,z0,-6.5,13,z0+1,STEEL)
        box(-7.5,12,z1-1,-6.5,13,z1,STEEL)
        bevel(-7.75,12.5,z0,-6.25,14,z1,.25,STEEL)
    box(4,12,54,7.5,12.25,55,STEEL)
    # A simple rear radiator panel avoids an expensive grid of cooling fins.
    box(-8,-11.5,8,8,-11,43,DARK)


def vending_snacks():
    # Meet the flat underside at Z=2 instead of hiding a unit of solid base inside it.
    box(-14,-11,0,14,11,2,DARK)
    bevel(-17,-13,2,17,12,64,1,TEAL)
    # Recessed display surrounded by the projecting frame; no opaque glass
    # brush hides the shelves or turns the machine into a solid dark rectangle.
    box(-14,12,23,7,12.25,53,DARK)
    # Four complementary mitres, including both ends of each member.
    # Their union is the original rectangular frame, with no doubled corners.
    for profile in [
        [(-16,21),(-14,23),(-14,53),(-16,55)],
        [(9,21),(9,55),(7,53),(7,23)],
        [(-16,21),(9,21),(7,23),(-14,23)],
        [(-16,55),(-14,53),(7,53),(9,55)],
    ]:
        hull([(x,y,z) for x,z in profile for y in (12,14)],CREAM)
    for z in (24,34,44):
        box(-14,12.25,z,7,14,z+1,STEEL)
        for index,x in enumerate((-11,-4,3)):
            col=(RED,GOLD,BLUE)[(index+z//10)%3]
            bevel(x-2.25,12.25,z+1,x+2.25,13.5,z+7,.375,col)
            box(x-1.25,13.5,z+3,x+1.25,13.625,z+4,CREAM)
    # Payment strip, broad buttons and a low collection flap.
    box(10,12,24,15,13,51,DARK)
    box(10.75,13,44,14.25,13.25,49,BLUE)
    box(11,13,39,14,13.25,39.75,STEEL)
    for z in (29,33):bevel(11,13,z,14,13.75,z+2,.25,CREAM)
    box(-13,12,7,7,13,17,DARK)
    bevel(-12,13,9,6,13.75,16,.25,STEEL)
    bevel(-14,12,57,14,13,62,.25,CREAM)
    # A small three-bar badge reads at thumbnail scale without 3D lettering.
    for x in (-6,0,6):box(x-1.5,13,58,x+1.5,13.25,61,TEAL)


ASSETS={f.__name__:f for f in (chair_dining,table_dining,refrigerator,vending_snacks)}


def build(name,geometry,ed):
    r.BRUSHES.clear();r.SOLIDS.clear();geometry()
    points=[p for solid,_ in r.SOLIDS for p in solid]
    dims=[max(p[k] for p in points)-min(p[k] for p in points) for k in range(3)]
    assert all(v*8==round(v*8) for p in points for v in p)
    path=Path('assets')/(name+'.map')
    path.write_text('\n'.join(['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
                    +[brush_text(i,'__TB_empty') for i in range(len(r.SOLIDS))]+['}'])+'\n')
    import asset_scale
    info={'units':dims,'anchor':'32 units/metre; ground Z=0; authored on 0.125-unit grid'}
    asset_scale.PROFILES[name]=info
    ed.call('open_map',path=str(path))
    brushes=json.loads(ed.call('list_brushes'))
    assert len(brushes)==len(r.SOLIDS)
    for brush,(_,color) in zip(brushes,r.SOLIDS):
        roughness,metallic=SURFACES[color]
        faces=json.loads(ed.call('describe_brush',brush=brush['id']))['faces']
        for face in faces:
            ed.call('set_surface_material',brush=brush['id'],face=face['face'],roughness=roughness,metallic=metallic,emission=0)
    ed.call('save',path=str(path))
    ed.call('render_preview',path=str(path.with_suffix('.png')),size=1000)
    print(name,json.dumps(info),len(brushes),'brushes;',path.read_text().count(' skip '),'skip faces',flush=True)


if __name__=='__main__':
    ed=Editor()
    try:
        for name,geometry in ASSETS.items():
            if len(sys.argv)==1 or name in sys.argv[1:]:build(name,geometry,ed)
    finally:ed.close()
