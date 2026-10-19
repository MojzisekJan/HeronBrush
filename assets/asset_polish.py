"""Selective finish pass for generated assets, called by redesign_gen.build.
Round large painted components without adding brushes; use editable gradients.
Run from repository root: python3 assets/asset_polish.py [asset names...]
"""
import math

TARGETS=('car','car_retro_compact','car_retro_sedan','tram','bookshelf',
         'house','well','tree_oak','tree_pine','tree_maple')
GLASS={(103,177,194),(91,151,165),(79,148,168),(95,165,184)}


def generate(name,builder,core):
    if name not in TARGETS or name.startswith('tree_'):
        builder();return
    original=core.hull
    changed=0
    def hull(points,colour):
        nonlocal changed
        colour=tuple(colour)
        allowed=colour not in GLASS and max(colour)>90
        if name=='bookshelf':allowed=colour in {(151,95,52),(101,62,37)}
        zs=sorted(set(p[2] for p in points))
        # Only true straight extrusions; leave glazing, branches and shaped bodies alone.
        if allowed and len(zs)==2:
            lower=set((p[0],p[1]) for p in points if p[2]==zs[0])
            upper=set((p[0],p[1]) for p in points if p[2]==zs[1])
            if lower==upper and 3<=len(lower)<=10:
                cx=sum(p[0] for p in lower)/len(lower);cy=sum(p[1] for p in lower)/len(lower)
                poly=sorted(lower,key=lambda p:math.atan2(p[1]-cy,p[0]-cx))
                # Restrict the roundover to a small fraction of the narrowest extent.
                narrow=min(zs[1]-zs[0],max(p[0] for p in poly)-min(p[0] for p in poly),max(p[1] for p in poly)-min(p[1] for p in poly))
                if narrow>=1.5:
                    from portal_finish import inset
                    radius=min(1.5,narrow*.14)
                    profiles=[]
                    for side in (-1,1):
                        for angle in (0,math.pi/4,math.pi/2):
                            z=(zs[0]+radius-radius*math.sin(angle)) if side<0 else (zs[1]-radius+radius*math.sin(angle))
                            profiles.extend((x,y,z) for x,y in inset(poly,radius*(1-math.cos(angle))))
                    original(profiles,colour);changed+=1;return
        original(points,colour)
    core.hull=hull
    try:builder()
    finally:core.hull=original
    print(f'{name}: rounded {changed} components without adding brushes',flush=True)


def finish(name,core):
    if name not in TARGETS:return
    palettes={}
    for _,colour in core.SOLIDS:
        colour=tuple(colour)
        # Tyres and structural shadows stay dark. Tree trunks stay solid too.
        if max(colour)<85:continue
        if name.startswith('tree_') and colour[0]>colour[1]*1.15 and name!='tree_maple':continue
        strength=.10 if name.startswith('tree_') else .12
        light=[min(255,round(c+(255-c)*strength)) for c in colour]
        dark=[max(0,round(c*(1-strength))) for c in colour]
        if colour in GLASS:
            light=[170,218,228];dark=[64,117,141]
        palettes[colour]=(light,dark)
    from portal_finish import finish as native_finish
    native_finish(name,palettes,solids=core.SOLIDS)


if __name__=='__main__':
    import sys,importlib
    import redesign_gen as core
    modules={'car':'road_gen','car_retro_compact':'retro_cars_gen','car_retro_sedan':'retro_cars_gen',
             'tram':'redesign_gen','bookshelf':'redesign_gen','house':'building_gen','well':'building_gen',
             'tree_oak':'nature_gen','tree_pine':'nature_gen','tree_maple':'tall_trees_gen'}
    for name in sys.argv[1:] or TARGETS:
        if name not in modules:raise SystemExit(f'No finish profile: {name}')
        setattr(core,name,getattr(importlib.import_module(modules[name]),name));core.build(name)
