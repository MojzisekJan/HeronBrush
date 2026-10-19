#!/usr/bin/env python3
"""Low-brush garden props, authored in metres and saved at 32 units/metre.
Run from the repository root: python3 assets/garden_props_gen.py
"""
import math
import json
from pathlib import Path
import redesign_gen as r
from portal_finish import finish

BARK=(100,67,43)
CUT=(192,153,92)
LEAF=(63,121,64)
LEAF_LIGHT=(88,143,72)
FLOWER=(224,148,134)
STONE=(163,175,174)
WATER=(57,151,194)


def hull(points, colour):
    r.hull([tuple(v*32 for v in p) for p in points],colour)


def rod(a,b,radius,colour,sides=8):
    r.rod(tuple(v*32 for v in a),tuple(v*32 for v in b),radius*32,colour,sides)


def profile(rings,colour,cx=0,cy=0,sides=8):
    hull([(cx+radius*math.cos(i*2*math.pi/sides),cy+radius*math.sin(i*2*math.pi/sides),z)
          for z,radius in rings for i in range(sides)],colour)


def crown(cx,cy,z,rx,ry,h,colour):
    hull([(cx+rx*scale*math.cos(i*math.pi/4),cy+ry*scale*math.sin(i*math.pi/4),z+height*h)
          for height,scale in [(0,.48),(.35,1),(.72,.85),(1,.32)] for i in range(8)],colour)


def bush_flowering():
    rod((0,0,0),(0,0,.60),.095,BARK,6)
    crown(-.34,0,.18,.55,.46,.68,LEAF)
    crown(.30,.10,.20,.57,.45,.84,LEAF_LIGHT)
    crown(.02,-.31,.16,.49,.38,.66,LEAF)
    for x,y,z in [(-.46,-.26,.76),(.37,-.09,.99),(.04,-.49,.72)]:
        crown(x,y,z,.105,.09,.095,FLOWER)


def fallen_log():
    # Octagonal trunk, inset end-grain discs and one broken branch.
    rod((-1.25,0,.30),(1.25,0,.30),.30,BARK)
    for sign in (-1,1):
        rod((sign*1.247,0,.30),(sign*1.258,0,.30),.248,CUT)
    a=(.28,0,.43);b=(.55,.20,.91)
    rod(a,b,.105,BARK,6)
    direction=r.normal(r.sub(b,a))
    rod(r.add(b,r.mul(direction,-.005)),r.add(b,r.mul(direction,.008)),.080,CUT,6)


def fountain():
    profile([(0,1.01),(.05,1.12),(.12,1.12),(.16,1.02)],STONE)
    # Eight convex wall sectors leave a real open basin around the central pedestal.
    for i in range(8):
        a=i*math.pi/4;b=a+math.pi/4
        hull([(radius*math.cos(t),radius*math.sin(t),z)
              for z,outer,inner in [(.12,1.02,.87),(.36,1.12,.88),(.44,1.08,.91)]
              for radius,t in [(inner,a),(outer,a),(outer,b),(inner,b)]],STONE)
    profile([(.16,.88),(.265,.88)],WATER)
    profile([(.15,.25),(.27,.25),(.87,.14),(.92,.18)],STONE)
    profile([(.87,.15),(1.00,.41),(1.055,.41)],STONE)
    profile([(1.053,.34),(1.062,.34)],WATER)
    profile([(1.06,.055),(1.34,.035)],WATER,sides=6)
    crown(0,0,1.36,.065,.065,.14,WATER)


if __name__=='__main__':
    profiles=Path('assets/scale_profiles.json')
    data=json.loads(profiles.read_text())
    for name,generator in [('bush_flowering',bush_flowering),('fallen_log',fallen_log),('fountain',fountain)]:
        r.BRUSHES.clear();r.SOLIDS.clear();generator()
        points=[p for pts,_ in r.SOLIDS for p in pts]
        dims=[max(p[i] for p in points)-min(p[i] for p in points) for i in range(3)]
        data['assets'][name]={'units':dims,'anchor':'32 units/metre; ground at Z=0'}
        profiles.write_text(json.dumps(data,indent=2)+'\n')
        # The scale module can already have been imported by an earlier asset.
        import asset_scale
        asset_scale.PROFILES[name]=data['assets'][name]
        setattr(r,name,generator);r.build(name)
        finish(name,{
            BARK:([119,84,53],[83,53,37]), CUT:([213,179,117],[167,128,78]),
            LEAF:([82,143,73],[45,99,58]), LEAF_LIGHT:([112,163,88],[65,120,60]),
            FLOWER:([244,184,162],[206,115,113]),
            STONE:([188,199,193],[133,150,153]), WATER:([99,197,221],[40,125,181]),
        })
