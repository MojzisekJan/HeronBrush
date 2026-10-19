#!/usr/bin/env python3
"""Brush heron asset, authored in metres. Run from repo root.
The application logo is independently stylised; regeneration must preserve it.
"""
import json, math
from pathlib import Path
import redesign_gen as r
from garden_props_gen import hull,rod
from portal_finish import finish
BLUE=(59,139,196)
WING=(39,102,157)
PALE=(185,214,222)
GOLD=(231,169,59)
DARK=(40,67,87)

def prism(poly,depth,colour,cy=0):
    hull([(x,cy+y,z) for x,z in poly for y in (-depth/2,depth/2)],colour)

def heron():
    # Original parallel legs, overlapping in the near-side logo projection.
    for y in (-.07,.07):
        rod((.075,y,.025),(.105,y,.55),.022,GOLD,6)
        prism([(-.085,0),(.12,0),(.13,.035),(.045,.045)],.035,GOLD,y)
    # Broad shoulders and a tapered rump, with one folded wing per side.
    hull([(-.19,0,.73),(-.10,-.12,.85),(-.10,.12,.85),(.15,-.13,.82),(.15,.13,.82),
          (.37,0,.59),(.19,-.10,.49),(.19,.10,.49),(-.025,-.13,.51),(-.025,.13,.51)],BLUE)
    for sign in (-1,1):
        hull([(-.095,sign*.13,.78),(.07,sign*.18,.76),(.335,sign*.04,.59),
              (.12,sign*.15,.535),(-.06,sign*.135,.585)],WING)
    # Three convex sections share their end faces and form a clearer folded S.
    stations=[(-.085,.72,.14),(-.29,.89,.13),(-.20,1.065,.095),(-.36,1.225,.095)]
    for (x,z,w),(xx,zz,ww) in zip(stations,stations[1:]):
        prism([(x-w/2,z),(x+w/2,z),(xx+ww/2,zz),(xx-ww/2,zz)],.105,PALE)
    prism([(-.405,1.18),(-.45,1.245),(-.38,1.29),(-.25,1.275),(-.22,1.205)],.13,BLUE)
    prism([(-.43,1.242),(-.45,1.188),(-.77,1.185)],.06,GOLD)
    prism([(-.28,1.265),(-.08,1.19),(-.235,1.21)],.065,DARK)
    for y in (-.066,.066):
        rod((-.375,y,1.241),(-.375,y+math.copysign(.008,y),1.241),.013,DARK,6)


if __name__=='__main__':
    heron()
    pts=[p for points,_ in r.SOLIDS for p in points]
    dims=[max(p[i] for p in pts)-min(p[i] for p in pts) for i in range(3)]
    path=Path('assets/scale_profiles.json');data=json.loads(path.read_text())
    data['assets']['heron']={'units':dims,'anchor':'32 units/metre; ground Z=0; standing heron 1.29m tall'}
    path.write_text(json.dumps(data,indent=2)+'\n')
    import asset_scale
    asset_scale.PROFILES['heron']=data['assets']['heron']
    r.heron=heron;r.build('heron')
    palettes={BLUE:([79,165,217],[45,117,175]),WING:([54,127,181],[31,84,133]),PALE:([214,232,235],[155,189,205])}
    finish('heron',palettes,world_gradients={c:((0,0,-1),None) for c in palettes})
