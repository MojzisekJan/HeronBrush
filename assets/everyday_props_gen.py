#!/usr/bin/env python3
"""Ten everyday props. 32 units/metre, ground Z=0, 1/8-unit vertices.
Run from the repository root; optional arguments select asset names.
"""
import json
import sys
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import box, hull, vertical
from fan_gen import brush_text, octagon
from interior_props_gen import bevel
from prefabs_gen import Editor

WOOD=(151,101,62); FABRIC=(63,119,126); CUSHION=(201,153,86)
CREAM=(230,223,202); WHITE=(228,237,231); DARK=(43,54,59)
STEEL=(155,175,182); PAINT=(75,116,133); ORANGE=(224,100,42)

def feet(xs,ys,z=5):
    for x in xs:
        for y in ys:box(x-1.5,y-1.5,0,x+1.5,y+1.5,z,WOOD)

def sofa():
    feet((-25,25),(-10,10))
    bevel(-31,-14,4,31,14,10,1.5,FABRIC)
    bevel(-31,-14,9,31,-8,32,2,FABRIC)
    for x in (-31,25):bevel(x,-10,9,x+6,14,24,1.5,FABRIC)
    for x in (-24.5,.5):bevel(x,-8,10,x+24,13,16,1.5,CREAM)
    bevel(13,-8,18,24,-3,29,1.5,CUSHION)

def armchair():
    feet((-11,11),(-10,10))
    bevel(-16,-14,4,16,14,10,1.5,FABRIC)
    bevel(-16,-14,9,16,-8,33,2,FABRIC)
    for x in (-16,10):bevel(x,-10,9,x+6,14,24,1.5,FABRIC)
    bevel(-9.5,-8,10,9.5,13,16,1.5,CUSHION)
    bevel(-10,-8,17,10,-3,30,1.5,FABRIC)

def bed_single():
    feet((-13,13),(-27,27),8)
    bevel(-16,-32,6,16,32,11,.5,WOOD)
    bevel(-16,-33,7,16,-30,32,1,WOOD)
    bevel(-16,30,7,16,33,17,1,WOOD)
    bevel(-15,-30,11,15,30,17,1.5,CREAM)
    bevel(-15.25,-10,16,15.25,29,19,1,FABRIC)
    bevel(-11,-27,17,11,-15,21,1.5,WHITE)

def desk_office():
    # Pedestal drawers on one side; free knee space on the other.
    for y in (-10,10):box(-26,y-1,0,-24,y+1,24,STEEL)
    box(-26,-11,3,-24,11,5,STEEL)
    bevel(13,-11,1,26,11,24,.5,PAINT)
    for z in (3,10,17):
        bevel(14,11,z,25,12,z+6,.375,CREAM)
        box(17,12,z+3.5,22,12.75,z+4.25,STEEL)
    bevel(-28,-14,24,28,14,26,.5,WOOD)
    box(-24,-12,17,13,-10.5,24,PAINT)

def locker_metal():
    box(-10,-8,0,10,8,3,DARK)
    bevel(-12,-10,2,12,10,60,.75,PAINT)
    box(-10.5,10,4,10.5,10.25,58,DARK)
    bevel(-10,10.25,4.5,10,11,57.5,.5,PAINT)
    for z in (7,9,11,50,52,54):box(-6,11,z,6,11.125,z+.75,DARK)
    box(5,11,29,8,11.5,36,STEEL)
    bevel(5.75,11.5,30,7.25,12.5,35,.25,DARK)
    box(-4,11,42,4,11.25,45,CREAM)

def bowl(cx,cy,z0,z1,rx,ry,inner_x,inner_y,color):
    # Eight convex sectors form a symmetric open basin, with a solid bottom.
    ring=octagon(1)
    for i,(u,v) in enumerate(ring):
        a,b=ring[(i+1)%8]
        hull([(cx+x*sx,cy+y*sy,z) for x,y in ((u,v),(a,b))
              for sx,sy,z in ((rx*.625,ry*.625,z0),(rx,ry,z1),
                              (inner_x,inner_y,z1),(inner_x*.5,inner_y*.5,z0+1))],color)
    hull([(cx+u*inner_x*.5,cy+v*inner_y*.5,z) for u,v in ring for z in (z0,z0+1)],color)

def toilet():
    bevel(-6,-7,0,6,9,3,1,WHITE)
    hull([(x*w,y*d,z) for z,w,d in ((2,4,5),(10,6,7)) for x in (-1,1) for y in (-1,1)],WHITE)
    bowl(0,3,9,16,9,11,6,8,WHITE)
    bevel(-9,-12,3,9,-7,26,1,WHITE)
    bevel(-9.5,-12.5,26,9.5,-6.5,28,.5,CREAM)
    box(5,-6.5,23,8,-5.75,24,STEEL)

def sink_pedestal():
    bevel(-7,-6,0,7,6,2,.75,WHITE)
    hull([(u*w,v*d,z) for z,w,d in ((2,5,4),(20,3,3),(25,6,5)) for u,v in octagon(1)],WHITE)
    bowl(0,1,23,28,12,9,9,6,WHITE)
    bevel(-12,-9,25,12,-6,28,.5,WHITE)
    box(-1,-8,28,1,-6,35,STEEL)
    bevel(-1,-7,33,1,0,35,.375,STEEL)
    for x in (-7,7):bevel(x-1.5,-8,28,x+1.5,-5,30,.375,STEEL)

def traffic_cone():
    bevel(-6,-6,0,6,6,1.5,.5,DARK)
    for z0,z1,r0,r1,c in ((1.5,8,4.5,3,ORANGE),(8,11,3,2.25,WHITE),(11,16,2.25,1,ORANGE)):
        vertical(0,0,z0,z1,r0,c,top=r1)

def bollard():
    bevel(-5,-5,0,5,5,2,.5,DARK)
    for z0,z1,c in ((2,23,PAINT),(23,26,CREAM),(26,31,PAINT)):
        vertical(0,0,z0,z1,2.5,c)
    vertical(0,0,31,32,2.5,STEEL,top=1.5)

def trash_bin():
    bevel(-9,-8,0,9,8,2,.5,DARK)
    bevel(-10,-9,2,10,9,25,1,PAINT)
    # Two front opening slots remain empty beneath a raised lid.
    box(-10,-9,25,10,-7,32,PAINT)
    for x in (-10,8):box(x,-7,25,x+2,9,32,PAINT)
    box(-1,7,25,1,9,32,PAINT)
    bevel(-11,-10,32,11,10,35,1,STEEL)
    for x in (-5,0,5):box(x-.5,9,6,x+.5,9.25,20,STEEL)

ASSETS={f.__name__:f for f in (sofa,armchair,bed_single,desk_office,locker_metal,toilet,sink_pedestal,traffic_cone,bollard,trash_bin)}

def build(name,geometry,ed):
    r.BRUSHES.clear();r.SOLIDS.clear();geometry()
    pts=[p for points,_ in r.SOLIDS for p in points]
    assert all(v*8==round(v*8) for p in pts for v in p)
    assert min(p[2] for p in pts)==0
    dims=[max(p[k] for p in pts)-min(p[k] for p in pts) for k in range(3)]
    path=Path('assets')/(name+'.map')
    path.write_text('\n'.join(['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']+[brush_text(i,'__TB_empty') for i in range(len(r.SOLIDS))]+['}'])+'\n')
    import asset_scale
    info={'units':dims,'anchor':'32 units/metre; ground Z=0; authored on 0.125-unit grid'}
    asset_scale.PROFILES[name]=info
    ed.call('open_map',path=str(path))
    ed.call('save',path=str(path))
    brushes=json.loads(ed.call('list_brushes'))
    assert len(brushes)==len(r.SOLIDS)
    ed.call('render_preview',path=str(path.with_suffix('.png')),size=800)
    print(name,len(brushes),'brushes',dims,'units',path.read_text().count(' skip '),'skip faces',flush=True)
    return info

if __name__=='__main__':
    profiles=Path('assets/scale_profiles.json')
    data=json.loads(profiles.read_text())
    ed=Editor()
    try:
        for name,geometry in ASSETS.items():
            if len(sys.argv)==1 or name in sys.argv[1:]:data['assets'][name]=build(name,geometry,ed)
    finally:ed.close()
    profiles.write_text(json.dumps(data,indent=2)+'\n')
