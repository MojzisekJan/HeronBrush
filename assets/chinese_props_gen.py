#!/usr/bin/env python3
"""Bamboo clump and hanging lantern, low-brush editable geometry at 32 units/m."""
import math,json
from pathlib import Path
import redesign_gen as r
from garden_props_gen import hull,rod,profile
from portal_finish import finish
GREEN=(74,130,66)
RING=(141,169,93)
LEAF=(47,107,65)
RED=(194,49,42)
GOLD=(205,162,76)
CORD=(81,62,44)


def bamboo_clump():
    # Three solid stalks. Collars make the bamboo nodes readable without
    # dividing every stalk into separate brush segments.
    for x,y,h,lean in [(-.27,.08,2.8,-.19),(.08,.14,3.2,.15),(.27,-.16,2.45,.27)]:
        a=(x,y,0);b=(x+lean,y,h)
        rod(a,b,.065,GREEN,6)
        for t in (.28,.56,.82):
            c=(x+lean*t,y,h*t)
            rod((c[0],c[1],c[2]-.025),(c[0],c[1],c[2]+.025),.083,RING,6)
        # Two tapered leaf sprays per stalk, each a single convex brush.
        for side,t in [(-1,.68),(1,.87)]:
            bx=x+lean*t;bz=h*t
            hull([(bx,y,bz),(bx+side*.30,y-.055,bz+.22),
                  (bx+side*.69,y,bz+.27),(bx+side*.31,y+.055,bz+.09),
                  (bx+side*.30,y,bz+.26)],LEAF)


def lantern_red():
    # One faceted convex body, two brass collars, tassel, knot and hanging cord.
    profile([(.19,.13),(.28,.245),(.48,.285),(.69,.245),(.78,.13)],RED,sides=12)
    profile([(.15,.12),(.20,.16),(.25,.14)],GOLD,sides=12)
    profile([(.735,.14),(.79,.16),(.82,.12)],GOLD,sides=12)
    rod((0,0,.81),(0,0,1.02),.018,CORD,6)
    profile([(.095,.042),(.145,.04),(.17,.02)],GOLD,sides=8)
    profile([(0,.055),(.025,.063),(.11,.023)],RED,sides=8)
    # Small round brass medallions sit on the front and back of the lantern.
    for side in (-1,1):
        rod((0,side*.27,.48),(0,side*.30,.48),.062,GOLD,8)


if __name__=='__main__':
    path=Path('assets/scale_profiles.json');data=json.loads(path.read_text())
    for name,generator in [('bamboo_clump',bamboo_clump),('lantern_red',lantern_red)]:
        r.SOLIDS.clear();r.BRUSHES.clear();generator()
        pts=[p for points,_ in r.SOLIDS for p in points]
        dims=[max(p[i] for p in pts)-min(p[i] for p in pts) for i in range(3)]
        data['assets'][name]={'units':dims,'anchor':'32 units/metre; lowest point at Z=0'}
        path.write_text(json.dumps(data,indent=2)+'\n')
        import asset_scale
        asset_scale.PROFILES[name]=data['assets'][name]
        setattr(r,name,generator);r.build(name)
        finish(name,{
            GREEN:([102,155,78],[55,110,57]),RING:([164,186,111],[116,145,77]),
            LEAF:([67,132,73],[32,89,54]), RED:([232,77,49],[154,28,38]),
            GOLD:([235,196,112],[164,119,47]),CORD:([106,85,55],[63,47,32]),
        })
