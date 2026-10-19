#!/usr/bin/env python3
"""Partly open double gate, 20 convex brushes, 32 units/metre.
Run from repository root: python3 assets/gate_open_gen.py
"""
import math,json
from pathlib import Path
import redesign_gen as r
from portal_finish import rounded,finish
from garden_props_gen import hull,rod
STONE=(152,165,163)
CAP=(180,188,175)
WOOD=(146,94,53)
IRON=(65,80,85)
BRASS=(190,148,73)


def box(x0,y0,z0,x1,y1,z1,c):
    r.box(x0*32,y0*32,z0*32,x1*32,y1*32,z1*32,c)


def gate_open():
    for side in (-1,1):
        x=side*1.80
        rounded([(x-.35,0),(x+.35,0),(x+.35,.16),(x+.26,.23),(x-.26,.23),(x-.35,.16)],.74,STONE,.035)
        rounded([(x-.26,.18),(x+.26,.18),(x+.24,2.46),(x-.24,2.46)],.53,STONE,.04)
        rounded([(x-.33,2.42),(x+.33,2.42),(x+.33,2.53),(x+.20,2.67),(x-.20,2.67),(x-.33,2.53)],.66,CAP,.04)
    for hinge,angle in [(-1.52,60),(1.52,145)]:
        first=len(r.SOLIDS)
        rounded([(0,.12),(1.48,.12),(1.48,1.98),(.98,2.16),(.18,2.16),(0,2.08)],.12,WOOD,.018)
        for z in (.46,1.68):
            box(-.035,-.09,z,1.49,-.055,z+.105,IRON)
            rod((0,-.035,z-.06),(0,-.035,z+.19),.057,IRON,8)
        # One diagonal iron brace, independent of the broad wooden panel.
        rod((.12,-.09,.59),(1.33,-.09,1.66),.037,IRON,4)
        box(1.27,-.125,.91,1.36,-.075,1.10,BRASS)
        a=math.radians(angle);c=math.cos(a);s=math.sin(a)
        def transform(p):
            x,y,z=p
            return (hinge*32+c*x-s*y,s*x+c*y,z)
        r.SOLIDS[first:]=[([transform(p) for p in pts],colour) for pts,colour in r.SOLIDS[first:]]
        r.BRUSHES[first:]=[([[transform(p) for p in face] for face in planes],colour)
                          for planes,colour in r.BRUSHES[first:]]


if __name__=='__main__':
    gate_open()
    pts=[p for points,_ in r.SOLIDS for p in points]
    dims=[max(p[i] for p in pts)-min(p[i] for p in pts) for i in range(3)]
    path=Path('assets/scale_profiles.json');data=json.loads(path.read_text())
    data['assets']['gate_open']={'units':dims,'anchor':'32 units/metre; pillars 2.67m; leaves open 60 and 35 degrees'}
    path.write_text(json.dumps(data,indent=2)+'\n')
    import asset_scale
    asset_scale.PROFILES['gate_open']=data['assets']['gate_open']
    r.gate_open=gate_open;r.build('gate_open')
    finish('gate_open',{
        STONE:([179,188,178],[128,145,148]), CAP:([204,210,194],[152,167,164]),
        WOOD:([174,118,68],[119,72,43]), IRON:([88,104,108],[48,61,67]),
        BRASS:([213,177,101],[153,113,55]),
    })
