#!/usr/bin/env python3
"""Stylised palm: 12 convex brushes, 32 units/metre.
Run from repository root: python3 assets/palm_gen.py
"""
import math,json
from pathlib import Path
import redesign_gen as r
from garden_props_gen import hull,rod
from portal_finish import finish
BARK=(153,116,72)
LEAF=(61,128,68)
LIGHT=(88,149,71)
NUT=(103,73,45)


def palm():
    # Three tapered sections share identical rings; hidden joint faces become skip.
    rings=[(0,0,0,.22),(.12,0,1.55,.18),(.37,.035,3.05,.135),(.64,.06,4.35,.10)]
    for a,b in zip(rings,rings[1:]):
        hull([(x+radius*math.cos(i*math.pi/4),y+radius*math.sin(i*math.pi/4),z)
              for x,y,z,radius in [a,b] for i in range(8)],BARK)
    cx,cy,cz=.64,.06,4.30
    # Each leaf is one thin, folded convex blade with a broad shoulder and drooping tip.
    for i in range(7):
        angle=i*2*math.pi/7
        length=1.65 if i%2 else 1.95
        radial=(math.cos(angle),math.sin(angle));across=(-radial[1],radial[0])
        points=[]
        for distance,width,height in [(0,0,.03),(.55,.25,.40),(length,0,-.48)]:
            for side in (-1,1):
                points.append((cx+radial[0]*distance+across[0]*width*side,
                               cy+radial[1]*distance+across[1]*width*side,cz+height))
        points.append((cx+radial[0]*.60,cy+radial[1]*.60,cz+.49))
        hull(points,LIGHT if i%3==0 else LEAF)
    for dx,dy,dz in [(-.10,-.09,-.12),(.13,.02,-.18)]:
        # Two small faceted coconuts tucked under the crown.
        x,y,z=cx+dx,cy+dy,cz+dz
        hull([(x+.12,y,z),(x-.12,y,z),(x,y+.12,z),(x,y-.12,z),
              (x,y,z+.15),(x,y,z-.15)],NUT)


if __name__=='__main__':
    palm()
    pts=[p for points,_ in r.SOLIDS for p in points]
    dims=[max(p[i] for p in pts)-min(p[i] for p in pts) for i in range(3)]
    path=Path('assets/scale_profiles.json');data=json.loads(path.read_text())
    data['assets']['tree_palm']={'units':dims,'anchor':'32 units/metre; ground Z=0; height 4.79m'}
    path.write_text(json.dumps(data,indent=2)+'\n')
    import asset_scale
    asset_scale.PROFILES['tree_palm']=data['assets']['tree_palm']
    r.tree_palm=palm;r.build('tree_palm')
    finish('tree_palm',{
        BARK:([180,143,90],[127,92,56]),LEAF:([78,146,74],[43,109,60]),
        LIGHT:([111,168,82],[66,128,59]),NUT:([132,96,56],[81,54,35]),
    })
