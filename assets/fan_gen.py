#!/usr/bin/env python3
"""Industrial fan: 20 visible convex brushes + origin brush; 32 units/metre.
Run from repository root: python3 assets/fan_gen.py
"""
import itertools
import json
import math
from pathlib import Path
import redesign_gen as r
from prefabs_gen import Editor
from asset_skip import hidden_faces, FACE

FRAME=(48,106,113); METAL=(58,67,75); BLADE=(225,209,164); HUB=(219,124,53)

def octagon(radius):
    a=round(radius*.75*8)/8
    return [(radius,a),(a,radius),(-a,radius),(-radius,a),(-radius,-a),(-a,-radius),(a,-radius),(radius,-a)]

def cylinder(radius,y0,y1,z,color):
    r.prism([(x,v+z) for x,v in octagon(radius)],y0,y1,color)

def geometry():
    r.BRUSHES.clear();r.SOLIDS.clear()
    outer=[(-36,0),(36,0),(40,4),(40,76),(36,80),(-36,80),(-40,76),(-40,4)]
    inner=[(-28,8),(28,8),(32,12),(32,68),(28,72),(-28,72),(-32,68),(-32,12)]
    for i in range(8):
        j=(i+1)%8
        r.prism([outer[i],outer[j],inner[j],inner[i]],-8,8,FRAME)
    # Rear supports, behind the rotor's entire swept volume.
    r.box(-32,5,38,32,8,42,METAL)
    r.box(-2,5,8,2,8,72,METAL)
    cylinder(6,3,8,40,METAL)
    for x,z in [(-34,6),(34,6),(-34,74),(34,74)]:
        r.hull([(x+u,y,z+v) for u,v in octagon(1.5) for y in (-9,-8)],HUB)
    split=len(r.SOLIDS)
    # Four swept, slightly pitched blades; every input vertex is on the 1/8 grid.
    for i in range(4):
        points=[]
        for x,z,y in [(3,-3,-1),(11,-7,-1),(28,-5,1),(28,3,1),(10,6,-1)]:
            for thickness in (-.75,.75):
                for _ in range(i):x,z=-z,x
                points.append((x,y+thickness,z+40))
                for _ in range(i):x,z=z,-x
        r.hull(points,BLADE)
    cylinder(6.5,-4,3,40,HUB)
    r.box(-1,-1,39,1,1,41,(255,0,255))
    # Face the asset browser's default camera; the braces remain on the rear.
    solids=list(r.SOLIDS);r.BRUSHES.clear();r.SOLIDS.clear()
    for points,color in solids:r.hull([(x,-y,z) for x,y,z in points],color)
    return split

def brush_text(index,texture):
    planes,color=r.BRUSHES[index];points=r.SOLIDS[index][0]
    lines=['// heron_brush color '+' '.join(map(str,color)),'{']
    for a,b,c in planes:
        n=r.normal(r.cross(r.sub(b,a),r.sub(c,a)));d=r.dot(n,a)
        on=[p for p in points if abs(r.dot(n,p)-d)<1e-5]
        chosen=next((triple for triple in itertools.combinations(on,3) if r.dot(r.cross(r.sub(triple[1],triple[0]),r.sub(triple[2],triple[0])),r.cross(r.sub(triple[1],triple[0]),r.sub(triple[2],triple[0])))>1e-9))
        a,b,c=chosen
        if r.dot(r.cross(r.sub(b,a),r.sub(c,a)),n)<0:b,c=c,b
        lines.append(' '.join('( '+' '.join(f'{v:g}' for v in p)+' )' for p in (a,b,c))+f' {texture} 0 0 0 1 1')
    return '\n'.join(lines+['}'])

def build():
    split=geometry();path=Path('assets/fan.map')
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    lines.extend(brush_text(i,'__TB_empty') for i in range(split));lines+=['}','{','"classname" "func_rotate"','"origin" "0 0 40"','"axis" "0 1 0"','"speed" "90"','"start_on" "1"']
    lines.extend(brush_text(i,'origin' if i==len(r.SOLIDS)-1 else '__TB_empty') for i in range(split,len(r.SOLIDS)))
    path.write_text('\n'.join(lines+['}'])+'\n')
    ed=Editor()
    try:
        ed.call('open_map',path=str(path));ed.call('save',path=str(path))
        infos=json.loads(ed.call('list_brushes'))
        assert len(infos)==21,len(infos)
        brushes=[json.loads(ed.call('describe_brush',brush=b['id'])) for b in infos]
        # Never cull against the other moving body. Exclude the non-rendered origin.
        hidden=set(hidden_faces(brushes[:split]))|set(hidden_faces(brushes[split:-1]))
        lines=path.read_text().splitlines(keepends=True);bid=None;face=0
        for i,line in enumerate(lines):
            if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16));face=0
            m=FACE.match(line.rstrip('\n'))
            if m:
                if (bid,face) in hidden:lines[i]=m[1]+'skip'+m[3]+'\n'
                face+=1
        path.write_text(''.join(lines));ed.call('open_map',path=str(path))
        ed.call('render_preview',path='assets/fan.png',size=1000)
        print(f'Fan: {len(infos)-1} visible brushes + origin, {len(hidden)} hidden faces, rotor axis Y at (0,0,40)')
    finally:ed.close()
if __name__=='__main__':build()
