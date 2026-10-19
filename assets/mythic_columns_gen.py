#!/usr/bin/env python3
"""Two fantasy ornamental columns, inspired by stepped temples and an underwater city."""
import json
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import build, hull, vertical
from ornamental_columns_gen import plinth
from fan_gen import octagon
from prefabs_gen import Editor
from portal_finish import paint_world_gradient

SAND=(157,116,77); BASALT=(91,76,62); RELIEF=(204,164,105)
JADE=(47,114,116); PALE=(157,189,184); GOLD=(209,169,89); CRYSTAL=(69,197,198)

def relief(poly,turn,color,depth=14):
    points=[]
    for x,z in poly:
        for y in (depth,depth+.75):
            px,py=x,y
            for _ in range(turn):px,py=-py,px
            points.append((px,py,z))
    hull(points,color)

def rect(x0,x1,z0,z1,turn,color,depth=14):
    relief([(x0,z0),(x1,z0),(x1,z1),(x0,z1)],turn,color,depth)

def column_temple():
    plinth(28,0,8,BASALT)
    plinth(24,8,16,SAND)
    plinth(20,16,24,RELIEF)
    plinth(14,24,124,SAND)
    plinth(16,60,65,BASALT)
    plinth(20,124,132,RELIEF)
    plinth(24,132,140,SAND)
    plinth(17,140,144,BASALT)
    # Four raised stepped fret motifs, each assembled from four simple bars.
    for turn in range(4):
        for x0,x1,z0,z1 in [(-8,8,37,40),(-8,-5,40,54),(-5,7,51,54),(4,7,45,51)]:
            rect(x0,x1,z0,z1,turn,RELIEF)
    # Stylised masks on opposite faces, geometric medallions on the other two.
    for turn in (0,2):
        relief([(-11,106),(-2,102),(-2,105),(-11,110)],turn,RELIEF)
        relief([(11,106),(2,102),(2,105),(11,110)],turn,RELIEF)
        rect(-9,-3,97,100,turn,BASALT)
        rect(3,9,97,100,turn,BASALT)
        relief([(-2,99),(0,105),(2,99),(0,93)],turn,RELIEF)
        rect(-6,6,87,90,turn,JADE)
    for turn in (1,3):
        relief([(-9,99),(0,86),(9,99),(0,112)],turn,RELIEF)
        relief([(-4,99),(0,93),(4,99),(0,105)],turn,JADE,14.75)

def column_atlantis():
    plinth(26,0,8,PALE)
    vertical(0,0,8,12,22,GOLD)
    vertical(0,0,12,22,20,PALE,top=12)
    vertical(0,0,22,112,12,JADE)
    vertical(0,0,22,26,13,GOLD)
    vertical(0,0,108,116,13,GOLD,top=10)
    vertical(0,0,116,124,10,PALE,top=7)
    for turn in range(4):
        # Four rising fins surround the solid crystal finial, leaving gaps between them.
        points=[]
        for x,z in [(10,108),(16,110),(25,144),(19,139)]:
            for y in (-2,2):
                px,py=x,y
                for _ in range(turn):px,py=-py,px
                points.append((px,py,z))
        hull(points,PALE)
        relief([(-5,69),(0,54),(5,69),(0,84)],turn,GOLD,12)
        relief([(-2.5,69),(0,61),(2.5,69),(0,77)],turn,CRYSTAL,12.75)
        rect(-1,1,32,47,turn,GOLD,12)
    hull([(0,0,120),(0,0,148)]+[(x,y,132) for x,y in octagon(8)],CRYSTAL)

if __name__=='__main__':
    editor=Editor()
    try:
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        for name,geometry in [('column_temple',column_temple),('column_atlantis',column_atlantis)]:
            info=build(name,geometry,editor);brushes=json.loads(editor.call('list_brushes'))
            palettes={SAND:((176,137,92),(132,94,66)),JADE:((63,138,137),(34,88,96)),PALE:((181,207,194),(131,164,164))}
            for brush,(_,color) in zip(brushes,r.SOLIDS):
                if color in palettes:
                    light,dark=palettes[color]
                    paint_world_gradient(editor,brush,light,dark,(0,0,-1),(-info['units'][2],0))
            path=f'assets/{name}.map';editor.call('save',path=path)
            editor.call('render_preview',path=f'assets/{name}.png',size=1000)
            data['assets'][name]=info
            print(name,'finished;',len(brushes),'brushes;',Path(path).read_text().count(' skip '),'skip faces',flush=True)
        profiles.write_text(json.dumps(data,indent=2)+'\n')
    finally:editor.close()
