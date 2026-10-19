#!/usr/bin/env python3
"""Fire engine, donkey and utility pole; 32 units/metre, 1/8-unit grid.
Run from the repository root: python3 assets/civic_assets_gen.py
"""
import itertools
import json
import math
from pathlib import Path
import redesign_gen as r
from fan_gen import brush_text, octagon
from prefabs_gen import Editor

RED=(194,47,39); DARK=(43,49,53); SILVER=(166,184,187)
GLASS=(69,130,155); CREAM=(237,224,190); BLUE=(47,135,232)

def hull(points, color):
    r.hull([tuple(round(v*8)/8 for v in p) for p in points],color)

def box(x0,y0,z0,x1,y1,z1,color):
    hull(list(itertools.product((x0,x1),(y0,y1),(z0,z1))),color)

def rounded(center, radii, color):
    x,y,z=center;rx,ry,rz=radii
    hull([(x+rx*s*math.cos(i*math.pi/4),y+ry*s*math.sin(i*math.pi/4),z+rz*h)
          for h,s in [(-1,.625),(0,1),(1,.625)] for i in range(8)],color)

def wheel(x,y0,y1,z,radius,color):
    hull([(x+a,y,z+b) for a,b in octagon(radius) for y in (y0,y1)],color)

def vertical(x,y,z0,z1,radius,color,top=None):
    hull([(x+a,y+b,z) for z,rad in [(z0,radius),(z1,radius if top is None else top)]
          for a,b in octagon(rad)],color)

def fire_engine():
    box(-108,-30,17,108,30,30,DARK)
    # A clipped cab roof and a separate equipment body keep the silhouette readable.
    from portal_finish import inset
    cab=[(36,28),(108,28),(108,80),(102,86),(42,86),(36,80)]
    hull([(x,y,z) for y,d in [(-32,3),(-29,0),(29,0),(32,3)] for x,z in inset(cab,d)],RED)
    r.prism([(-108,30),(36,30),(36,76),(30,82),(-102,82),(-108,76)],-34,34,RED)
    for x in (-72,72):
        for side in (-1,1):
            y0,y1=sorted((side*29,side*40))
            hull([(x+radius*math.cos(i*math.pi/6),y,16+radius*math.sin(i*math.pi/6))
                  for y,radius in [(y0,14),(y0+2,16),(y1-2,16),(y1,14)] for i in range(12)],DARK)
            y0,y1=sorted((side*40,side*41))
            hull([(x+7*math.cos(i*math.pi/6),y,16+7*math.sin(i*math.pi/6)) for y in (y0,y1) for i in range(12)],SILVER)
            a,b=sorted((side*40.75,side*41.25));wheel(x,a,b,16,3,DARK)
    box(108,-31,23,112,31,30,SILVER)
    box(-112,-31,23,-108,31,30,SILVER)
    box(108,-27,53,108.5,27,75,GLASS)
    box(108,-19,34,108.5,19,46,DARK)
    for side in (-1,1):
        y0,y1=sorted((side*32,side*32.5))
        box(48,y0,55,99,y1,77,GLASS)
        box(49,y0,51,57,y1+.25,52.5,SILVER)
        a,b=sorted((side*32,side*36));box(97,a,68,100,b,71,DARK)
        a,b=sorted((side*35,side*38));box(96,a,65,102,b,76,DARK)
        box(42,y0,44,103,y1,49,CREAM)
        y0,y1=sorted((side*34,side*34.5))
        for x0,x1 in [(-99,-60),(-56,-17),(-13,26)]:
            box(x0,y0,37,x1,y1,69,SILVER)
            mid=(x0+x1)/2
            a,b=sorted((side*34.5,side*35));box(mid-4,a,41,mid+4,b,43,DARK)
        box(-102,y0,72,29,y1,76,CREAM)
        yy=side*25
        box(108,yy-4,37,109,yy+4,45,CREAM)
        box(-109,yy-3,35,-108,yy+3,41,RED)
    # Six widely spaced rungs instead of a dense mesh of ladder detail.
    for y in (-12,12):box(-96,y-1.5,85,20,y+1.5,89,SILVER)
    for x in (-91,-70,-49,-28,-7,14):box(x-1.5,-12,85,x+1.5,12,88,SILVER)
    box(57,-25,86,75,25,89,DARK)
    for y in (-19,19):rounded((66,y,92),(7,6,3),BLUE)

def donkey():
    fur=(143,139,130); muzzle=(218,205,176); hoof=(62,58,53)
    rounded((-1,0,32),(22,10.5,11),fur)
    for x in (-15,15):
        for y in (-6,6):
            knee=x+(-2 if x>0 else 2)
            hull([(cx+dx,y+dy,z) for z,cx,w in [(4,x,2),(15,knee,2.75)]
                  for dx in (-w,w) for dy in (-2.5,2.5)],fur)
            # The upper attachment sits inside the belly, with no exposed flat cap.
            hip=10 if x>0 else -12
            hull([(cx+dx,cy+dy,z) for z,cx,cy,w,d in
                  [(15,knee,y,2.75,2.5),(31,hip,4.5 if y>0 else -4.5,3.5,1.75)]
                  for dx in (-w,w) for dy in (-d,d)],fur)
            hull([(x+.5+dx,y+dy,z) for z,w in [(0,3),(3.5,3.5),(5,2.75)] for dx in (-w,w) for dy in (-2.75,2.75)],hoof)
    # A forward-sloping neck widens into the chest and tapers into the head.
    neck_ring=[(8,0),(6,4.5),(0,6),(-6,4.5),(-8,0),(-6,-4.5),(0,-6),(6,-4.5)]
    hull([(cx+dx*scale,dy*scale,z) for z,cx,scale in [(32,10,1),(51,24,.5)]
          for dx,dy in neck_ring],fur)
    rounded((26,0,49),(8,6.5,10),fur)
    rounded((35,0,43),(9,6.5,5.5),muzzle)
    for side in (-1,1):
        # Slender ears tilt apart, with narrow pink insets on the front.
        hull([(x,side*cy+dy,z) for z,cy,w in [(55,4,2),(63,7,2.5),(69,8,.75)]
              for x in (22,25) for dy in (-w,w)],fur)
        hull([(x,side*cy+dy,z) for z,cy,w in [(57,4.75,1),(63,7,1.5),(67.5,7.75,.375)]
              for x in (24.875,25.125) for dy in (-w,w)],(187,155,145))
        rounded((28.5,side*5.2,52),(1.75,.625,2.25),muzzle)
        rounded((29,side*5.65,52),(.875,.25,1.25),hoof)
        rounded((42,side*2.5,44),(.625,.875,.625),hoof)
    hull([(x,y,z) for x,z in [(9,39),(18,52),(21,54),(12,39)] for y in (-1.5,1.5)],hoof)
    hull([(-23+dx,y,z) for z,dx in [(31,0),(22,-5)] for y in (-1,1) for dx in (dx-1,dx+1)],fur)
    hull([(-29+dx,y,z) for z,dx in [(23,0),(16,-1)] for y in (-1,1) for dx in (dx-1,dx+1)],fur)
    rounded((-30,0,15),(2,2,3.5),hoof)

def utility_pole():
    wood=(160,117,72); metal=(84,104,108); ceramic=(187,220,209)
    vertical(0,0,0,28,7,(149,151,143),top=6)
    vertical(0,0,28,241,5.5,wood,top=3.5)
    box(-44,-4,223,44,4,229,wood)
    # Diagonal braces are planar wedges, leaving the space below the arm open.
    r.prism([(-33,223),(-29,223),(-3,195),(-3,191)],-2,2,metal)
    r.prism([(33,223),(29,223),(3,195),(3,191)],-2,2,metal)
    for x in (-36,0,36):
        dz=12 if x==0 else 0
        vertical(x,0,229+dz,234+dz,2,metal)
        vertical(x,0,234+dz,239+dz,4.5,ceramic,top=3)
        vertical(x,0,239+dz,244+dz,4.5,ceramic,top=2.5)
    # Compact transformer mounted to the side of the pole.
    box(-8,-10,181,8,-4,185,metal)
    vertical(0,-17,185,215,10,(139,163,164))
    vertical(0,-17,215,218,11,metal)
    vertical(0,-17,218,225,3,ceramic,top=2)


def build(name, geometry, editor):
    r.BRUSHES.clear();r.SOLIDS.clear();geometry()
    path=Path('assets')/(name+'.map')
    points=[p for solid,_ in r.SOLIDS for p in solid]
    dims=[max(p[i] for p in points)-min(p[i] for p in points) for i in range(3)]
    assert all(abs(v*8-round(v*8))<1e-8 for p in points for v in p)
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    lines += [brush_text(i,'__TB_empty') for i in range(len(r.SOLIDS))]
    path.write_text('\n'.join(lines+['}'])+'\n')
    import asset_scale
    asset_scale.PROFILES[name]={"units":dims}
    expected=len(r.SOLIDS)
    editor.call('open_map',path=str(path));editor.call('save',path=str(path))
    brushes=json.loads(editor.call('list_brushes'))
    assert len(brushes)==expected,(name,expected,len(brushes))
    if name in ('fire_engine','donkey'):
        from portal_finish import paint_world_gradient
        palettes={RED:((210,65,47),(159,35,32)),SILVER:((192,205,204),(143,163,172)),
                  GLASS:((103,172,190),(45,102,128)),(143,139,130):((168,164,150),(117,114,109)),
                  (218,205,176):((232,219,188),(202,186,158))}
        for brush,(_,color) in zip(brushes,r.SOLIDS):
            if color in palettes:
                light,dark=palettes[color]
                paint_world_gradient(editor,brush,light,dark,(0,0,-1),(-dims[2],0))
        editor.call('save',path=str(path))
    editor.call('render_preview',path=str(path.with_suffix('.png')),size=1000)
    print(name,expected,'brushes;',dims,'units;',path.read_text().count(' skip '),'skip faces',flush=True)
    return {'units':dims,'anchor':'32 units/metre; ground Z=0; authored on 0.125-unit grid'}

if __name__=='__main__':
    editor=Editor()
    try:
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        import sys
        selected=set(sys.argv[1:])
        for name,geometry in [('fire_engine',fire_engine),('donkey',donkey),('utility_pole',utility_pole)]:
            if selected and name not in selected:continue
            data['assets'][name]=build(name,geometry,editor)
        profiles.write_text(json.dumps(data,indent=2)+'\n')
    finally:editor.close()
