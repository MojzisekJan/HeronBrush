#!/usr/bin/env python3
"""Compact ten-metre steel lattice power pole with four insulator strings."""
import json
from pathlib import Path
from civic_assets_gen import build, hull, box, vertical
from prefabs_gen import Editor

STEEL=(153,172,181); BRACE=(119,145,158); CERAMIC=(112,170,174); BASE=(151,152,143)

def beam(a,b,width,color):
    axis=max(range(3),key=lambda i:abs(b[i]-a[i]))
    u,v=[i for i in range(3) if i!=axis]
    points=[]
    for p in (a,b):
        for du in (-width/2,width/2):
            for dv in (-width/2,width/2):
                q=list(p);q[u]+=du;q[v]+=dv;points.append(q)
    hull(points,color)

def corner(sx,sy,z):
    t=(z-8)/280
    return (sx*(24-17*t),sy*(16-9*t),z)

def geometry():
    for sx in (-1,1):
        for sy in (-1,1):
            x,y=sx*24,sy*16
            box(x-5,y-5,0,x+5,y+5,8,BASE)
            beam(corner(sx,sy,8),corner(sx,sy,288),3.5,STEEL)
    # A single diagonal per bay and face keeps the lattice open and inexpensive.
    levels=(12,104,196,280)
    for i,(z0,z1) in enumerate(zip(levels,levels[1:])):
        direction=1 if i%2 else -1
        for side in (-1,1):
            beam(corner(-direction,side,z0),corner(direction,side,z1),2,BRACE)
            beam(corner(side,-direction,z0),corner(side,direction,z1),2,BRACE)
    for z,span in [(230,64),(282,46)]:
        box(-span,-4,z,span,4,z+6,STEEL)
        for side in (-1,1):
            beam((0,0,z-22),(side*(span-14),0,z),2.5,BRACE)
            # Three broad porcelain sheds per hanging string, no tiny screw detail.
            for j in range(3):
                vertical(side*(span-5),0,z-8*(j+1),z-8*j,4,CERAMIC,top=2)
    vertical(0,0,288,320,3,STEEL,top=1.5)

if __name__=='__main__':
    editor=Editor()
    try:
        name='utility_pole_metal';info=build(name,geometry,editor)
        path=Path('assets/scale_profiles.json');data=json.loads(path.read_text())
        data['assets'][name]=info;path.write_text(json.dumps(data,indent=2)+'\n')
    finally:editor.close()
