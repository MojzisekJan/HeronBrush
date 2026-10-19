#!/usr/bin/env python3
"""Marble portal with a bronze-inlaid rotor; 32 units/metre, 0.125-unit grid."""
import json
import math
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import hull, box
from ishtar_gate_gen import prism, bevel_block
from fan_gen import brush_text
from prefabs_gen import Editor
from asset_skip import hidden_faces, FACE

MARBLE=(225,220,201); EDGE=(192,189,170); BRONZE=(171,124,65)
ROTOR=(111,146,139); JADE=(65,190,181)
CENTER=64


def point(radius, angle):
    return (round(radius*math.cos(angle)*8)/8,
            CENTER+round(radius*math.sin(angle)*8)/8)


def geometry():
    r.BRUSHES.clear(); r.SOLIDS.clear()
    # A rectangular wall with a polygonal bore. Shared vertices keep the arch sealed.
    for half in (-1,1):
        arc=[point(52,half*i*math.pi/8) for i in range(9)]
        edge=116 if half>0 else 0
        for a,b in zip(arc,arc[1:]):
            prism([a,b,(b[0],edge),(a[0],edge)],-12,12,MARBLE)
    for side in (-1,1):
        x0,x1=sorted((side*52,side*66))
        box(x0,-12,0,x1,12,116,MARBLE)
    bevel_block(-72,72,-20,20,0,6,EDGE,3)
    bevel_block(-48,48,-28,28,6,10,MARBLE,3)
    bevel_block(-32,32,-23,23,10,14,EDGE,2)
    # Tapered pilasters carry a continuous cornice and shallow triangular pediment.
    for x in (-60,60):
        bevel_block(x-9,x+9,-16,16,6,17,EDGE,2)
        hull([(x+dx,y,z) for z,w in ((17,7),(106,5))
              for dx in (-w,w) for y in (-15,15)],MARBLE)
        bevel_block(x-9,x+9,-17,17,106,112,BRONZE,2)
        bevel_block(x-10,x+10,-18,18,112,118,MARBLE,2)
        for side in (-1,1):
            y0,y1=sorted((side*15,side*15.5))
            # A single long inset suggests a carved flute without many small brushes.
            prism([(x-1.5,25),(x+1.5,25),(x+1,99),(x-1,99)],y0,y1,EDGE)
    bevel_block(-72,72,-18,18,116,122,EDGE,2)
    bevel_block(-73,73,-19,19,122,125,BRONZE,2)
    prism([(-71,125),(71,125),(0,146)],-15,15,MARBLE)
    for side in (-1,1):
        # Sloping marble cap rather than a rectangular top slab.
        prism([(0,146),(side*71,125),(side*74,129),(0,151)],-18,18,EDGE)
    # Bronze palmettes on both faces of the pediment.
    for front in (-1,1):
        y0,y1=sorted((front*15,front*16))
        for angle in (-60,-30,0,30,60):
            a=math.radians(angle)
            def leaf(x,z):
                return (x*math.cos(a)+z*math.sin(a),128-x*math.sin(a)+z*math.cos(a))
            prism([leaf(0,0),leaf(-2,6),leaf(0,13),leaf(2,6)],y0,y1,BRONZE)
        y0,y1=sorted((front*16,front*16.75))
        prism([(-3,128),(0,125),(3,128),(0,131)],y0,y1,JADE)
    split=len(r.SOLIDS)
    # Nearly square section with chamfered lips. A three-unit radial gap allows rotation.
    for i in range(16):
        a=i*math.pi/8; b=(i+1)*math.pi/8
        hull([(x,y,z) for y,outer,inner in
              [(-8,48,41),(-7,49,40),(7,49,40),(8,48,41)]
              for radius,angle in ((inner,a),(outer,a),(outer,b),(inner,b))
              for x,z in [point(radius,angle)]],ROTOR)
    # Raised lozenges rotate with the ring; both faces are finished for a freestanding asset.
    for i in range(8):
        a=i*math.pi/4
        polygon=[point(rad,theta) for rad,theta in
                 ((42,a),(45,a-.045),(47,a),(45,a+.045))]
        for front in (-1,1):
            y0,y1=sorted((front*8,front*9))
            prism(polygon,y0,y1,JADE if i%2==0 else BRONZE)
    box(-1,-1,CENTER-1,1,1,CENTER+1,(255,0,255))
    return split


def build():
    split=geometry(); path=Path('assets/portal_marble.map')
    assert all(abs(v*8-round(v*8))<1e-8 for points,_ in r.SOLIDS for p in points for v in p)
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    lines.extend(brush_text(i,'__TB_empty') for i in range(split))
    lines += ['}','{','"classname" "func_rotate"','"origin" "0 0 64"',
              '"axis" "0 1 0"','"speed" "12"','"start_on" "1"']
    lines.extend(brush_text(i,'origin' if i==len(r.SOLIDS)-1 else '__TB_empty')
                 for i in range(split,len(r.SOLIDS)))
    path.write_text('\n'.join(lines+['}'])+'\n')
    editor=Editor()
    try:
        editor.call('open_map',path=str(path))
        def save():
            result=editor._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
            assert not result.get('result',{}).get('isError'),result
        save()
        from asset_grid import snap_file
        snap_file(editor,path)
        infos=json.loads(editor.call('list_brushes'))
        assert len(infos)==len(r.SOLIDS),(len(infos),len(r.SOLIDS))
        for brush,(_,color) in zip(infos[:-1],r.SOLIDS[:-1]):
            # Marble remains softly polished, bronze is the only metallic material.
            for face in json.loads(editor.call('describe_brush',brush=brush['id']))['faces']:
                editor.call('set_surface_material',brush=brush['id'],face=face['face'],
                            roughness=.32 if color==MARBLE else .46,
                            metallic=.7 if color==BRONZE else 0,emission=.12 if color==JADE else 0)
        save()
        brushes=[json.loads(editor.call('describe_brush',brush=b['id'])) for b in infos]
        # Occlusion is evaluated within each rigid body, never against the other body.
        hidden=set(hidden_faces(brushes[:split])) | set(hidden_faces(brushes[split:-1]))
        lines=path.read_text().splitlines(keepends=True); bid=None; face=0
        for i,line in enumerate(lines):
            if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16)); face=0
            match=FACE.match(line.rstrip('\n'))
            if match:
                if (bid,face) in hidden:lines[i]=match[1]+'skip'+match[3]+'\n'
                face+=1
        path.write_text(''.join(lines))
        editor.call('open_map',path=str(path))
        from asset_radial import apply
        apply("portal_marble", editor)
        editor.call("open_map", path=str(path))
        # Keep the pivot brush in the map, but omit its editor marker from the thumbnail.
        editor.call('select',ids=[infos[-1]['id']])
        editor.call('delete_selection')
        editor.call('render_preview',path='assets/portal_marble.png',size=1000)
        editor.call('open_map',path=str(path))
        profile=Path('assets/scale_profiles.json'); data=json.loads(profile.read_text())
        data['assets']['portal_marble']={'units':[148,56,151],
            'anchor':'32 units/metre; ground Z=0; clear ring diameter 80; rotor axis Y at (0,0,64), 12 degrees/sec'}
        profile.write_text(json.dumps(data,indent=2)+'\n')
        print(f'Marble portal: {split} fixed + {len(infos)-split-1} rotating brushes + origin; {len(hidden)} skip faces',flush=True)
    finally:editor.close()


if __name__=='__main__':build()
