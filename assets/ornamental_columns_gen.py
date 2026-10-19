#!/usr/bin/env python3
"""Two decorative, static brush columns; four metres tall, on the 1/8 grid."""
import json
from pathlib import Path
from civic_assets_gen import build, hull, box, vertical, wheel
from fan_gen import octagon
from prefabs_gen import Editor
from portal_finish import paint_world_gradient
import redesign_gen as r

STONE=(220,210,186); TRIM=(186,171,143)
JADE=(56,124,118); JADE_LIGHT=(72,143,133); GOLD=(201,158,73)

def plinth(radius,z0,z1,color,bevel=2):
    # Square bevelled corners, with an inset top rim, in one convex brush.
    def corners(r):
        return [(r,r-bevel),(r-bevel,r),(-r+bevel,r),(-r,r-bevel),
                (-r,-r+bevel),(-r+bevel,-r),(r-bevel,-r),(r,-r+bevel)]
    hull([(x,y,z) for z,size in [(z0,radius),(z1-2,radius),(z1,radius-2)] for x,y in corners(size)],color)

def column_classical():
    plinth(24,0,8,TRIM)
    vertical(0,0,8,12,20,STONE,top=20)
    vertical(0,0,12,20,20,STONE,top=14)
    vertical(0,0,20,108,13,STONE,top=11)
    vertical(0,0,108,112,13,TRIM)
    vertical(0,0,112,122,13,STONE,top=20)
    # Four restrained scroll ends, two on each side of the capital.
    for x in (-15,15):
        for side in (-1,1):
            y0,y1=sorted((side*12,side*19))
            wheel(x,y0,y1,117,5,STONE)
    plinth(24,122,128,STONE)

def column_lotus():
    plinth(23,0,8,GOLD)
    vertical(0,0,8,14,18,JADE,top=14)
    vertical(0,0,14,96,14,JADE,top=10)
    vertical(0,0,14,18,14.5,GOLD,top=14.25)
    # Gold inlays follow the taper on the four broad faces of the octagonal shaft.
    for turn in range(4):
        points=[]
        for z,y in [(23,14-4*9/82),(86,14-4*72/82)]:
            for x in (-1,1):
                for thickness in (0,.5):
                    px,py=x,y-.25+thickness
                    for _ in range(turn):px,py=-py,px
                    points.append((px,py,z))
        hull(points,GOLD)
    vertical(0,0,92,98,11,GOLD)
    vertical(0,0,98,122,8,JADE)
    # Eight shared wedge faces form a spreading lotus capital.
    for i in range(8):
        j=(i+1)%8
        points=[]
        for z,outer,inner in [(98,11,8),(122,23,8)]:
            for radius in (outer,inner):
                poly=octagon(radius)
                for k in (i,j):points.append((*poly[k],z))
        hull(points,JADE_LIGHT if i%2 else JADE)
    vertical(0,0,122,128,23,GOLD)


def finish(name,editor):
    brushes=json.loads(editor.call('list_brushes'))
    palettes={STONE:((233,225,204),(210,197,169)),JADE:((68,140,132),(45,107,104)),JADE_LIGHT:((84,157,144),(62,128,120))}
    for brush,(_,color) in zip(brushes,r.SOLIDS):
        if color in palettes:
            light,dark=palettes[color]
            paint_world_gradient(editor,brush,light,dark,(0,0,-1),(-128,0))
    path=f'assets/{name}.map'
    editor.call('save',path=path)
    editor.call('render_preview',path=f'assets/{name}.png',size=1000)
    print(name, 'finished:',len(brushes),'brushes,',Path(path).read_text().count(' skip '),'skip faces',flush=True)

if __name__=='__main__':
    editor=Editor()
    try:
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        for name,geometry in [('column_classical',column_classical),('column_lotus',column_lotus)]:
            data['assets'][name]=build(name,geometry,editor)
            finish(name,editor)
        profiles.write_text(json.dumps(data,indent=2)+'\n')
    finally:editor.close()
