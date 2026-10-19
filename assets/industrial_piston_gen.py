#!/usr/bin/env python3
"""Factory hydraulic ram with a hollow barrel and an automatic one-metre stroke."""
import json
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import hull, box
from fan_gen import brush_text, octagon
from prefabs_gen import Editor
from asset_skip import hidden_faces, FACE

BODY=(197,145,62); STEEL=(176,192,195); DARK=(54,73,79); RED=(176,67,44)

def cylinder(x0,x1,radius,color):
    hull([(x,y,40+z) for x in (x0,x1) for y,z in octagon(radius)],color)

def geometry():
    r.BRUSHES.clear();r.SOLIDS.clear()
    r.prism([(-62,0),(78,0),(78,4),(74,8),(-58,8),(-62,4)],-26,26,DARK)
    for x in (-42,-4):box(x-5,-14,8,x+5,14,25,DARK)
    for x in (-54,68):
        for y in (-19,19):box(x-2,y-2,8,x+2,y+2,10,STEEL)
    cylinder(-52,-48,19,DARK)
    outer,inner=octagon(17),octagon(7.5)
    # Eight convex segments leave a real bore; the rod has 2.5 units of clearance.
    for i in range(8):
        j=(i+1)%8
        hull([(x,y,40+z) for x in (-48,12) for poly in (outer,inner)
              for y,z in (poly[i],poly[j])],BODY)
    box(-43,-3,56,-37,3,65,DARK)
    box(-46,-4,63,-34,4,67,RED)
    fixed=len(r.SOLIDS)
    # At full extension, the rear of the rod remains inside the bore.
    cylinder(-26,24,5,STEEL)
    cylinder(24,30,14,DARK)
    cylinder(30,31,11,RED)
    box(-1,-1,39,1,1,41,(255,0,255))
    return fixed

def build():
    fixed=geometry();path=Path('assets/industrial_piston.map')
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    lines.extend(brush_text(i,'__TB_empty') for i in range(fixed))
    lines+=['}','{','"classname" "func_shuttle"','"origin" "0 0 40"','"direction" "1 0 0"','"travel" "32"','"speed" "24"','"wait" "0.5"','"start_on" "1"']
    lines.extend(brush_text(i,'origin' if i==len(r.SOLIDS)-1 else '__TB_empty') for i in range(fixed,len(r.SOLIDS)))
    path.write_text('\n'.join(lines+['}'])+'\n')
    editor=Editor()
    try:
        editor.call('open_map',path=str(path))
        saved=editor._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
        assert not saved.get('result',{}).get('isError'),saved
        from asset_grid import snap_file
        snap_file(editor,path)
        infos=json.loads(editor.call('list_brushes'));assert len(infos)==len(r.SOLIDS)
        brushes=[json.loads(editor.call('describe_brush',brush=b['id'])) for b in infos]
        # Cross-body culling would erase rod surfaces revealed during extension.
        hidden=set(hidden_faces(brushes[:fixed]))|set(hidden_faces(brushes[fixed:-1]))
        lines=path.read_text().splitlines(keepends=True);bid=None;face=0
        for i,line in enumerate(lines):
            if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16));face=0
            match=FACE.match(line.rstrip('\n'))
            if match:
                if (bid,face) in hidden:lines[i]=match[1]+'skip'+match[3]+'\n'
                face+=1
        path.write_text(''.join(lines));editor.call('open_map',path=str(path))
        editor.call('render_preview',path='assets/industrial_piston.png',size=1000)
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        data['assets']['industrial_piston']={'units':[140,52,67],'anchor':'32 units/metre; ground Z=0; X-axis stroke 32 units at 24 units/sec, 0.5 sec endpoint wait'}
        profiles.write_text(json.dumps(data,indent=2)+'\n')
        print(f'Piston: {len(infos)-1} visible brushes + origin; {len(hidden)} skip faces; 32-unit stroke',flush=True)
    finally:editor.close()

if __name__=='__main__':build()
