#!/usr/bin/env python3
"""Three framed, sliding doors using the built-in func_door properties.

Cull hidden surfaces within each rigid body only. Door travel never removes
faces on the frame or on the other leaf. Dimensions use 32 units/metre.
"""
import json
import sys
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import box, hull
from interior_props_gen import bevel
from fan_gen import brush_text, octagon
from prefabs_gen import Editor
from asset_skip import hidden_faces, FACE

WOOD=(174,119,70); DARKWOOD=(111,72,47); TEAL=(60,124,127)
CREAM=(229,221,192); STEEL=(155,172,173); DARK=(44,56,62)
BRASS=(196,151,75); ORANGE=(204,137,62)
SURFACES={WOOD:(.48,0),DARKWOOD:(.58,0),TEAL:(.32,0),CREAM:(.30,0),
          STEEL:(.36,1),DARK:(.58,1),BRASS:(.30,1),ORANGE:(.32,0)}
GROUPS=[]


def frame(width,height,color,depth=5):
    for side in (-1,1):
        x0,x1=sorted((side*width/2,side*(width/2+4)))
        # Bevels retain the exact minimum rectangular opening in the core.
        bevel(x0,-depth,0,x1,depth,height+4,.5,color)
    bevel(-width/2-4,-depth,height,width/2+4,depth,height+6,.5,color)


def group(start,direction,travel,origin):
    end=len(r.SOLIDS)
    x,y,z=origin
    box(x-1,y-1,z-1,x+1,y+1,z+1,(255,0,255))
    GROUPS.append((start,end+1,{
        'classname':'func_door','origin':' '.join(map(str,origin)),
        'open_direction':direction,'travel':str(travel),'speed':'40','wait':'3',
    }))


def door_wooden():
    frame(40,74,DARKWOOD)
    # The exposed track has room for the entire leaf when open.
    box(-62,11,80,22,14,84,DARK)
    for x in (-20,20):box(x-2,2,77,x+2,12,81,STEEL)
    fixed=len(r.SOLIDS)
    bevel(-19.5,-2,.5,19.5,2,75.5,.5,WOOD)
    # Long broad planks are indicated with shallow seams on both sides.
    for side in (-1,1):
        y0,y1=sorted((side*2,side*2.125))
        for x in (-10,0,10):box(x-.125,y0,1,x+.125,y1,75,DARKWOOD)
        y0,y1=sorted((side*2.125,side*2.75))
        for z in (12,62):box(-18,y0,z,18,y1,z+2,DARK)
        y0,y1=sorted((side*2.75,side*4))
        box(12,y0,32,14,y1,42,STEEL)
    for x in (-12,12):
        box(x-1.5,2,70,x+1.5,3.5,82,DARK)
        hull([(x+a,y,82+b) for y in (2,4) for a,b in octagon(2.5)],STEEL)
    group(fixed,'-1 0 0',40,[0,0,38])
    return fixed


def door_industrial():
    frame(40,78,TEAL,5)
    # Roller housing sits above the opening, the leaf slides vertically.
    bevel(-25,-6,84,25,5,94,1,TEAL)
    box(-20,5,87,20,5.125,90,DARK)
    for x in (-17,17):box(x-2,5.125,87,x+2,5.25,90,ORANGE)
    fixed=len(r.SOLIDS)
    bevel(-19.5,-2,.5,19.5,2,79.5,.5,STEEL)
    for side in (-1,1):
        y0,y1=sorted((side*2,side*2.5))
        for z in (16,32,48,64):box(-19,y0,z,19,y1,z+1,DARK)
        y0,y1=sorted((side*2.5,side*3.5))
        box(-19,y0,2,19,y1,5,ORANGE)
        y0,y1=sorted((side*2.5,side*4))
        box(-5,y0,9,5,y1,11,DARK)
    group(fixed,'0 0 1',82,[0,0,40])
    return fixed


def door_double():
    frame(72,78,CREAM,6)
    # A restrained brass lintel and deep side plinths suit a hotel or civic hall.
    box(-37,6,80,37,6.5,81.5,BRASS)
    for side in (-1,1):
        x0,x1=sorted((side*36,side*41))
        bevel(x0,-6.5,0,x1,6.5,9,.5,CREAM)
    fixed=len(r.SOLIDS)
    for side in (-1,1):
        start=len(r.SOLIDS)
        x0,x1=sorted((side*.25,side*35.75))
        bevel(x0,-2,.5,x1,2,79.5,.5,DARKWOOD)
        for face in (-1,1):
            y0,y1=sorted((face*2,face*2.75))
            for z0,z1 in [(6,31),(37,74)]:
                bevel(x0+3,y0,z0,x1-3,y1,z1,.25,WOOD)
            a,b=sorted((side*4,side*6))
            y0,y1=sorted((face*2,face*4))
            box(a,y0,35,b,y1,46,BRASS)
        group(start,f'{side} 0 0',37,[side*18,0,40])
    return fixed


ASSETS={f.__name__:f for f in (door_wooden,door_industrial,door_double)}


def build(name,geometry,ed):
    r.BRUSHES.clear();r.SOLIDS.clear();GROUPS.clear();fixed=geometry()
    # Leaves run in front of the jambs so the swept body never cuts through
    # the frame. Even the rear handles have clearance during opening.
    offset=9.25 if name=='door_industrial' else 12
    solids=list(r.SOLIDS)
    for start,end,props in GROUPS:
        for i in range(start,end):
            points,color=solids[i]
            solids[i]=([(x,y+offset,z) for x,y,z in points],color)
        origin=list(map(float,props['origin'].split()));origin[1]+=offset
        props['origin']=' '.join(f'{v:g}' for v in origin)
    r.BRUSHES.clear();r.SOLIDS.clear()
    for points,color in solids:hull(points,color)
    origins={end-1 for start,end,props in GROUPS}
    visible=[p for i,(solid,_) in enumerate(r.SOLIDS) if i not in origins for p in solid]
    dims=[max(p[k] for p in visible)-min(p[k] for p in visible) for k in range(3)]
    assert all(v*8==round(v*8) for p in visible for v in p)
    path=Path('assets')/(name+'.map')
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    lines.extend(brush_text(i,'__TB_empty') for i in range(fixed));lines.append('}')
    for start,end,props in GROUPS:
        lines+=['{']+[f'"{key}" "{value}"' for key,value in props.items()]
        lines.extend(brush_text(i,'origin' if i in origins else '__TB_empty') for i in range(start,end))
        lines.append('}')
    path.write_text('\n'.join(lines)+'\n')
    import asset_scale
    info={'units':dims,'anchor':'32 units/metre; ground Z=0; 0.125-unit grid; func_door leaves, separate worldspawn frame'}
    asset_scale.PROFILES[name]=info
    ed.call('open_map',path=str(path))
    infos=json.loads(ed.call('list_brushes'));assert len(infos)==len(r.SOLIDS)
    for i,(brush,(_,color)) in enumerate(zip(infos,r.SOLIDS)):
        if i in origins:continue
        roughness,metallic=SURFACES[color]
        for face in json.loads(ed.call('describe_brush',brush=brush['id']))['faces']:
            ed.call('set_surface_material',brush=brush['id'],face=face['face'],roughness=roughness,metallic=metallic,emission=0)
    # Raw save avoids rescaling origins. The native grid pass checks every body.
    result=ed._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
    assert not result.get('result',{}).get('isError'),result
    from asset_grid import snap_file
    snap_file(ed,path)
    infos=json.loads(ed.call('list_brushes'))
    brushes=[json.loads(ed.call('describe_brush',brush=b['id'])) for b in infos]
    hidden=set(hidden_faces(brushes[:fixed]))
    for start,end,props in GROUPS:hidden.update(hidden_faces(brushes[start:end-1]))
    lines=path.read_text().splitlines(keepends=True);bid=None;face=0
    for i,line in enumerate(lines):
        if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16));face=0
        match=FACE.match(line.rstrip('\n'))
        if match:
            if (bid,face) in hidden:lines[i]=match[1]+'skip'+match[3]+'\n'
            face+=1
    path.write_text(''.join(lines));ed.call('open_map',path=str(path))
    ed.call('render_preview',path=str(path.with_suffix('.png')),size=1000)
    print(name,json.dumps(info),len(infos)-len(origins),'visible brushes;',len(origins),'origins;',len(hidden),'skip faces',flush=True)


if __name__=='__main__':
    ed=Editor()
    try:
        for name,geometry in ASSETS.items():
            if len(sys.argv)==1 or name in sys.argv[1:]:build(name,geometry,ed)
    finally:ed.close()
