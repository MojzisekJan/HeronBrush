#!/usr/bin/env python3
"""Stylized ten-metre windmill with a separate rotating sail assembly."""
import json
from pathlib import Path
import redesign_gen as r
from fan_gen import brush_text, octagon
from prefabs_gen import Editor
from asset_skip import hidden_faces, FACE

STONE=(131,137,126); WALL=(224,209,174); WOOD=(102,65,42)
ROOF=(44,82,89); CLOTH=(247,230,186); GLASS=(80,147,157); GOLD=(196,146,69)

def tower(rings,color):
    r.hull([(x,y,z) for z,radius in rings for x,y in octagon(radius)],color)

def geometry():
    r.BRUSHES.clear();r.SOLIDS.clear()
    tower([(0,60),(4,64),(12,64)],STONE)
    tower([(12,60),(216,42)],WALL)
    tower([(12,61),(24,60)],STONE)
    tower([(200,45),(216,44)],WOOD)
    tower([(214,53),(220,55)],ROOF)
    tower([(220,55),(226,54),(272,12),(284,0)],ROOF)
    r.prism([(x,z+216) for x,z in octagon(5)],40,81,WOOD)
    # Arched door and broad entrance steps, sized for the two-metre player.
    r.prism([(-15,12),(15,12),(15,66),(8,78),(-8,78),(-15,66)],44,63,WOOD)
    r.prism([(-11,14),(11,14),(11,64),(6,73),(-6,73),(-11,64)],62.5,64,(151,100,54))
    r.box(6,64,38,8,66,43,GOLD)
    r.box(-21,57,0,21,78,6,STONE)
    r.box(-17,57,6,17,70,12,STONE)
    r.prism([(-11,126),(11,126),(11,147),(6,155),(-6,155),(-11,147)],42,53,WOOD)
    r.prism([(-7,130),(7,130),(7,146),(4,150),(-4,150),(-7,146)],52.5,54,GLASS)
    r.box(40,-11,108,55,11,135,WOOD)
    r.box(54.5,-7,112,56,7,131,GLASS)
    split=len(r.SOLIDS)
    def sail(poly,y0,y1,color,turn):
        points=[]
        for x,z in poly:
            for _ in range(turn):x,z=-z,x
            points.extend((x,y,z+216) for y in (y0,y1))
        r.hull(points,color)
    for turn in range(4):
        sail([(-3,4),(3,4),(3,104),(-3,104)],79,83,WOOD,turn)
        sail([(3,30),(17,38),(24,98),(3,101)],81,83,CLOTH,turn)
    r.prism([(x,z+216) for x,z in octagon(9)],76,91,WOOD)
    r.prism([(x,z+216) for x,z in octagon(5)],91,93,GOLD)
    r.box(-1,81,215,1,83,217,(255,0,255))
    return split

def build():
    split=geometry();path=Path('assets/windmill.map')
    # Keep the shared scale profile consistent with this redesign.
    profile=Path('assets/scale_profiles.json');data=json.loads(profile.read_text())
    data['assets']['windmill']['units']=[208,157,320]
    profile.write_text(json.dumps(data,indent=2)+'\n')
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    lines.extend(brush_text(i,'__TB_empty') for i in range(split))
    lines+=['}','{','"classname" "func_rotate"','"origin" "0 82 216"','"axis" "0 1 0"','"speed" "18"','"start_on" "1"']
    lines.extend(brush_text(i,'origin' if i==len(r.SOLIDS)-1 else '__TB_empty') for i in range(split,len(r.SOLIDS)))
    path.write_text('\n'.join(lines+['}'])+'\n')
    ed=Editor()
    try:
        ed.call('open_map',path=str(path))
        # Geometry already uses the intended metre scale and exact grid coordinates.
        result=ed._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
        assert not result.get('result',{}).get('isError'),result
        infos=json.loads(ed.call('list_brushes'));assert len(infos)==len(r.SOLIDS)
        brushes=[json.loads(ed.call('describe_brush',brush=b['id'])) for b in infos]
        hidden=set(hidden_faces(brushes[:split]))|set(hidden_faces(brushes[split:-1]))
        lines=path.read_text().splitlines(keepends=True);bid=None;face=0
        for i,line in enumerate(lines):
            if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16));face=0
            m=FACE.match(line.rstrip('\n'))
            if m:
                if (bid,face) in hidden:lines[i]=m[1]+'skip'+m[3]+'\n'
                face+=1
        path.write_text(''.join(lines));ed.call('open_map',path=str(path))
        ed.call('render_preview',path='assets/windmill.png',size=1000)
        print(f'Windmill: {len(infos)-1} visible brushes + origin; {len(hidden)} hidden faces; pivot (0,82,216), 18 degrees/s')
    finally:ed.close()
if __name__=='__main__':build()
