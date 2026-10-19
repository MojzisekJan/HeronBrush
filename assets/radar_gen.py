#!/usr/bin/env python3
"""Rotating tilted radar dish with twenty-four curved reflector panels, 32 units/metre and a 1/8-unit grid."""
import json
import math
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import hull, box, vertical
from ornamental_columns_gen import plinth
from fan_gen import brush_text, octagon
from prefabs_gen import Editor
from asset_skip import hidden_faces, FACE
from utility_pole_metal_gen import beam
from portal_finish import paint_world_gradient

METAL=(71,100,104); DISH=(191,209,194); DARK=(49,61,64); GOLD=(210,157,66)

def geometry():
    r.BRUSHES.clear();r.SOLIDS.clear()
    plinth(24,0,8,(139,150,147))
    vertical(0,0,8,54,13,METAL,top=9)
    vertical(0,0,54,60,12,DARK)
    for x in (-17,17):
        for y in (-17,17):box(x-2,y-2,8,x+2,y+2,10,GOLD)
    split=len(r.SOLIDS)
    vertical(0,0,60,66,13,METAL)
    # A U-shaped cradle carries the tilted reflector; everything above the bearing rotates.
    for x in (-15,15):
        box(x-2,-10,66,x+2,-4,105,METAL)
        hull([(px,-6+y,100+z) for px in (x-3,x+3) for y,z in octagon(5)],GOLD)
    box(-17,-10,99,17,-3,105,METAL)
    def tilt(p):
        x,y,z=p;c=math.cos(math.radians(20));t=math.sin(math.radians(20))
        return (x,y*c-z*t,106+y*t+z*c)
    def point(radius,angle,depth):
        return (44*radius*math.cos(angle),depth,36*radius*math.sin(angle))
    # Two radial courses approximate a parabola; twelve sectors keep a rounded silhouette.
    for i in range(12):
        a=i*math.pi/6;b=(i+1)*math.pi/6
        hull([tilt((0,y,0)) for y in (-3,0)] +
             [tilt(point(.5,angle,y)) for angle in (a,b) for y in (1,4)],DISH)
        hull([tilt(point(radius,angle,y)) for radius,depth in [(.5,4),(1,18)]
              for angle in (a,b) for y in (depth-3,depth)],DISH)
    # Three slender arms support a flared feed horn at the focus.
    for i in (2,6,10):
        beam(tilt(point(1,i*math.pi/6,18)),tilt((0,32,0)),1.5,METAL)
    hull([tilt((x,y,z)) for y,radius in [(23,6),(29,3)] for x,z in octagon(radius)],GOLD)
    hull([tilt((x,y,z)) for y in (29,36) for x,z in octagon(3.5)],DARK)
    # Non-rendered pivot marker belongs to the rotor, never the fixed support.
    box(-1,-1,59,1,1,61,(255,0,255))
    return split

def build():
    split=geometry();path=Path('assets/radar.map')
    assert all(abs(v*8-round(v*8))<1e-8 for points,_ in r.SOLIDS for p in points for v in p)
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    lines.extend(brush_text(i,'__TB_empty') for i in range(split))
    lines+=['}','{','"classname" "func_rotate"','"origin" "0 0 60"','"axis" "0 0 1"','"speed" "30"','"start_on" "1"']
    lines.extend(brush_text(i,'origin' if i==len(r.SOLIDS)-1 else '__TB_empty') for i in range(split,len(r.SOLIDS)))
    path.write_text('\n'.join(lines+['}'])+'\n')
    editor=Editor()
    try:
        editor.call('open_map',path=str(path))
        # Save without static cross-body occlusion: rotor faces can turn into view.
        saved=editor._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
        assert not saved.get('result',{}).get('isError'),saved
        from asset_grid import snap_file
        snap_file(editor,path)
        infos=json.loads(editor.call('list_brushes'));assert len(infos)==len(r.SOLIDS)
        for brush,(_,color) in zip(infos,r.SOLIDS):
            if color==METAL:
                paint_world_gradient(editor,brush,(90,127,130),(55,83,91),(0,0,-1),(-150,0))
        saved=editor._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
        assert not saved.get('result',{}).get('isError'),saved
        brushes=[json.loads(editor.call('describe_brush',brush=b['id'])) for b in infos]
        hidden=set(hidden_faces(brushes[:split]))|set(hidden_faces(brushes[split:-1]))
        lines=path.read_text().splitlines(keepends=True);bid=None;face=0
        for i,line in enumerate(lines):
            if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16));face=0
            match=FACE.match(line.rstrip('\n'))
            if match:
                if (bid,face) in hidden:lines[i]=match[1]+'skip'+match[3]+'\n'
                face+=1
        path.write_text(''.join(lines));editor.call('open_map',path=str(path))
        editor.call('render_preview',path='assets/radar.png',size=1000)
        profile=Path('assets/scale_profiles.json');data=json.loads(profile.read_text())
        points=[p for solid,_ in r.SOLIDS for p in solid]
        dimensions=[max(p[i] for p in points)-min(p[i] for p in points) for i in range(3)]
        data['assets']['radar']={'units':dimensions,'anchor':'32 units/metre; ground Z=0; rotor axis Z at (0,0,60), 30 degrees/sec'}
        profile.write_text(json.dumps(data,indent=2)+'\n')
        print(f'Radar: {len(infos)-1} visible brushes + origin; {len(hidden)} skip faces; 30 degrees/sec',flush=True)
    finally:editor.close()

if __name__=='__main__':build()
