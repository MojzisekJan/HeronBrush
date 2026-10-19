#!/usr/bin/env python3
"""Open red gateway matching the user's reference: 33 brushes, 32 units/metre.
Run from repository root: python3 assets/gate_paifang_gen.py
"""
import json
from pathlib import Path
import redesign_gen as r
from portal_finish import rounded,finish
from garden_props_gen import hull,rod,profile
from gate_open_gen import box
RED=(189,57,35)
DARK=(68,73,70)
COLLAR=(116,121,115)
JADE=(49,118,103)
GOLD=(185,145,75)


def gate_paifang():
    for side in (-1,1):
        x=side*1.20
        # Square dark foot, tapered red lower leg and broad grey collar.
        rounded([(x-.235,0),(x+.235,0),(x+.235,.12),(x+.20,.17),(x-.20,.17),(x-.235,.12)],.49,DARK,.022)
        rounded([(x-.18,.12),(x+.18,.12),(x+.14,.99),(x-.14,.99)],.35,RED,.014)
        rounded([(x-.25,.94),(x+.25,.94),(x+.25,1.015),(x-.25,1.015)],.49,COLLAR,.015)
        # Faceted cylinders give round pillars without separate fillet brushes.
        rod((x,0,1.005),(x,0,3.25),.145,RED,12)
        profile([(3.20,.15),(3.235,.21),(3.29,.21),(3.31,.17)],RED,cx=x,sides=12)
    # The lower crossbar stays above a two-metre player. The middle post joins it
    # to the upper lintel, leaving two small rectangular openings as in the reference.
    rounded([(-1.91,2.59),(1.91,2.59),(1.91,2.82),(-1.91,2.82)],.27,RED,.014)
    box(-.115,-.135,2.81,.115,.135,3.26,RED)
    hull([(x,y,z) for x in [-1.82,1.82]
          for y,z in [(-.20,3.22),(.20,3.22),(.20,3.70),(0,3.82),(-.20,3.70)]],RED)
    # Five joined convex spans approximate the shallow upturned lintel. This is a
    # shallow green roof above the reference's open frame. Shared ends become skip.
    xs=[-2.23,-1.52,-.65,.65,1.52,2.23]
    def rise(x):return .22*(abs(x)/2.23)**2
    for a,b in zip(xs,xs[1:]):
        hull([(x,y,3.40+rise(x)+z) for x in [a,b] for y in [-.225,.225]
              for z in [0,.125]],RED)
        for side in (-1,1):
            hull([(x,side*y,3.53+rise(x)+lift+t)
                  for x in [a,b] for y,lift in [(0,.26),(.57,0)]
                  for t in [0,.075]],JADE)
    # A fine raised ridge ties the roof together without hiding the red lintel.
    for a,b in zip(xs,xs[1:]):
        hull([(x,y,3.85+rise(x)+z) for x in [a,b] for y in [-.045,.045]
              for z in [0,.045]],GOLD)


def finish_gate():
    finish('gate_paifang',{
        RED:([224,79,43],[161,39,27]), DARK:([89,96,88],[49,55,53]),
        COLLAR:([149,155,143],[94,101,95]), JADE:([67,142,116],[34,96,87]),
        GOLD:([209,177,105],[153,113,53]),
    },world_gradients={JADE:(lambda brush:(0,1 if brush['min'][1]+brush['max'][1]>0 else -1,0),(0,.57*32))})


if __name__=='__main__':
    gate_paifang()
    pts=[p for points,_ in r.SOLIDS for p in points]
    dims=[max(p[i] for p in pts)-min(p[i] for p in pts) for i in range(3)]
    path=Path('assets/scale_profiles.json');data=json.loads(path.read_text())
    data['assets']['gate_paifang']={'units':dims,'anchor':'32 units/metre; reference-shaped red gate; clear height 2.59m'}
    path.write_text(json.dumps(data,indent=2)+'\n')
    import asset_scale
    asset_scale.PROFILES['gate_paifang']=data['assets']['gate_paifang']
    r.gate_paifang=gate_paifang;r.build('gate_paifang')
    finish_gate()
