#!/usr/bin/env python3
"""A seated, low-brush beaver. Run from repository root."""
import json
import math
from pathlib import Path
import redesign_gen as r
from garden_props_gen import hull
from portal_finish import finish

FUR=(133,85,48)
MUZZLE=(186,139,88)
DARK=(66,45,34)
BLACK=(24,24,23)
IVORY=(242,225,182)


def pebble(center, size, colour):
    # Three octagonal rings give each rounded part a single convex brush.
    x,y,z=center
    rx,ry,rz=size
    hull([(-(x+rx*s*math.cos(i*math.pi/4)),-(y+ry*s*math.sin(i*math.pi/4)),z+rz*h)
          for h,s in [(-1,.55),(0,1),(1,.55)] for i in range(8)],colour)


def beaver():
    # Author facing -Y, then rotate in pebble() for a front three-quarter library preview.
    pebble((0,.39,.055),(.19,.34,.045),DARK)
    pebble((0,.035,.29),(.245,.235,.26),FUR)
    for x in (-.17,.17):
        pebble((x,-.12,.045),(.10,.16,.045),DARK)
    pebble((0,-.125,.545),(.215,.175,.175),FUR)
    for x in (-.162,.162):
        pebble((x,-.075,.715),(.068,.045,.068),FUR)
    # A wide light muzzle and separate incisors read clearly at asset thumbnail size.
    pebble((0,-.277,.49),(.145,.080,.075),MUZZLE)
    pebble((0,-.338,.548),(.052,.033,.03),BLACK)
    for x in (-.035,.035):
        pebble((x,-.339,.423),(.029,.022,.053),IVORY)
    for x in (-.122,.122):
        pebble((x,-.273,.602),(.027,.025,.030),BLACK)
    for x in (-.16,.16):
        pebble((x,-.203,.285),(.066,.072,.115),FUR)


if __name__=='__main__':
    beaver()
    pts=[p for points,_ in r.SOLIDS for p in points]
    dims=[max(p[i] for p in pts)-min(p[i] for p in pts) for i in range(3)]
    path=Path('assets/scale_profiles.json');data=json.loads(path.read_text())
    data['assets']['beaver']={'units':dims,'anchor':'32 units/metre; ground Z=0; seated beaver 0.783m tall including ears'}
    path.write_text(json.dumps(data,indent=2)+'\n')
    import asset_scale
    asset_scale.PROFILES['beaver']=data['assets']['beaver']
    r.beaver=beaver;r.build('beaver')
    palettes={FUR:([156,104,60],[111,69,40]),MUZZLE:([204,160,108],[171,119,70]),DARK:([86,61,44],[56,38,30])}
    finish('beaver',palettes,world_gradients={c:((0,0,-1),None) for c in palettes})
