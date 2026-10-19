#!/usr/bin/env python3
"""Street lantern, forklift pallet and cypress; 32 units/metre, 1/8 grid."""
import json
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import box, hull, vertical
from fan_gen import brush_text, octagon
from prefabs_gen import Editor
from portal_finish import paint_world_gradient

IRON=(50,65,71); BRASS=(183,141,76); GLOW=(250,218,151)
WOOD=(164,121,76); WOOD_LIGHT=(186,146,93); WOOD_DARK=(133,96,60)
BARK=(108,78,48); GREEN=(52,108,71); GREEN_LIGHT=(65,126,78); GREEN_DARK=(36,85,64)
SURFACES={IRON:(.58,1,0),BRASS:(.30,1,0),GLOW:(.22,0,1.2),
          WOOD:(.74,0,0),WOOD_LIGHT:(.74,0,0),WOOD_DARK:(.74,0,0),
          BARK:(.90,0,0),GREEN:(.82,0,0),GREEN_LIGHT:(.82,0,0),GREEN_DARK:(.82,0,0)}


def street_lamp():
    # Slender four-metre post with an offset hanging lantern.
    vertical(-12,0,0,3,6,IRON,top=5)
    vertical(-12,0,3,17,3.5,IRON,top=2.5)
    # Shared planar mitres join the tapered post, diagonal and horizontal arm.
    # Half-unit slopes keep every octagonal ring vertex on the 1/8 grid.
    foot=[(-12+u,v,17) for u,v in octagon(1.75)]
    elbow=[(-12+u,v,116.75-.5*u) for u,v in octagon(1.25)]
    shoulder=[(-3+.5*u,v,125.75-u) for u,v in octagon(1.25)]
    tip=[(14,v,125.75-u) for u,v in octagon(1.25)]
    hull(foot+elbow,IRON)
    vertical(-12,0,17,19,3,BRASS)
    hull(elbow+shoulder,IRON)
    hull(shoulder+tip,IRON)
    vertical(12,0,118,125,1,BRASS)
    # The glass is solid luminous paint; corner mullions have actual depth.
    hull([(12+x,y,z) for z,w in [(98,3.25),(117,5)] for x in (-w,w) for y in (-w,w)],GLOW)
    for sx in (-1,1):
        for sy in (-1,1):
            hull([(12+sx*w+dx,sy*w+dy,z) for z,w in [(98,3.25),(117,5)]
                  for dx in (-.5,.5) for dy in (-.5,.5)],IRON)
    hull([(12+x,y,z) for z,w in [(96,3.75),(99,4.25)] for x in (-w,w) for y in (-w,w)],IRON)
    hull([(12+x,y,z) for z,w in [(117,6.5),(121,2.5)] for x in (-w,w) for y in (-w,w)],IRON)
    vertical(12,0,94,97,1.5,BRASS)


def pallet_wood():
    # 1.2 x 1 metre pallet. Fork openings remain accessible from all four sides.
    for y in (-13,0,13):
        box(-19.25,y-3,0,19.25,y+3,1,WOOD_DARK)
    for x in (-16.25,0,16.25):
        for y in (-13,0,13):
            box(x-3,y-3,1,x+3,y+3,3.5,WOOD)
    for x in (-16.25,0,16.25):
        box(x-3,-16,3.5,x+3,16,4.5,WOOD)
    for index,y in enumerate((-13,-6.5,0,6.5,13)):
        box(-19.25,y-3,4.5,19.25,y+3,5.5,WOOD_LIGHT if index%2==0 else WOOD)


def tree_cypress():
    vertical(0,0,0,45,3.5,BARK,top=2.5)
    ring=[(1,0),(.75,.75),(0,1),(-.75,.75),(-1,0),(-.75,-.75),(0,-1),(.75,-.75)]
    # Convex spindle masses keep an irregular, slender crown with only six brushes.
    for x,y,z,rx,ry,h,col in [
        (0,0,26,16,13,126,GREEN),(-8,-2,32,10,9,94,GREEN_DARK),
        (7,-3,35,10,10,104,GREEN_LIGHT),(-4,5,54,11,10,103,GREEN),
        (1,-5,77,10,8,83,GREEN_LIGHT),(5,4,66,9,9,79,GREEN_DARK),
    ]:
        hull([(x+u*rx*s,y+v*ry*s,z+h*t) for t,s in [(0,.25),(.25,1),(.60,.75),(1,.0625)] for u,v in ring],col)


def build(name,geometry):
    r.BRUSHES.clear();r.SOLIDS.clear();geometry()
    dims=[max(p[k] for pts,_ in r.SOLIDS for p in pts)-min(p[k] for pts,_ in r.SOLIDS for p in pts) for k in range(3)]
    path=Path('assets')/(name+'.map')
    path.write_text('\n'.join(['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']+[brush_text(i,'__TB_empty') for i in range(len(r.SOLIDS))]+['}'])+'\n')
    import asset_scale
    asset_scale.PROFILES[name]={'units':dims,'anchor':'32 units/metre; ground Z=0; authored on 0.125-unit grid'}
    ed=Editor()
    try:
        ed.call('open_map',path=str(path))
        brushes=json.loads(ed.call('list_brushes'))
        assert len(brushes)==len(r.SOLIDS)
        for b,(_,color) in zip(brushes,r.SOLIDS):
            roughness,metallic,emission=SURFACES[color]
            if name=='tree_cypress' and color!=BARK:
                light=tuple(min(255,c+10) for c in color);dark=tuple(max(0,c-10) for c in color)
                paint_world_gradient(ed,b,light,dark,(0,0,-1),(-160,0))
            for face in json.loads(ed.call('describe_brush',brush=b['id']))['faces']:
                ed.call('set_surface_material',brush=b['id'],face=face['face'],roughness=roughness,metallic=metallic,emission=emission)
        ed.call('save',path=str(path))
        if name=='street_lamp':
            ed.call('add_point_entity',classname='light',origin=[12,0,92],properties={
                'light_color':'255 220 164','light_energy':'1.4','omni_range':'4',
                'shadow_enabled':'0','light':'192','_color':'1 0.862745 0.643137',
            })
            ed.call('save',path=str(path))
        ed.call('render_preview',path='assets/'+name+'.png',size=1000)
        print(name,len(brushes),'brushes',dims,'units',path.read_text().count(' skip '),'skip faces',flush=True)
    finally:ed.close()


if __name__=='__main__':
    for name,geometry in [('street_lamp',street_lamp),('pallet_wood',pallet_wood),('tree_cypress',tree_cypress)]:build(name,geometry)
