#!/usr/bin/env python3
"""Closed stylised treasure chest, 32 units/metre and 1/8-unit grid."""
import json
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import build, hull, box
from prefabs_gen import Editor
from portal_finish import paint_world_gradient

WOOD=(139,77,39)
IRON=(62,72,77)
GOLD=(198,151,68)
DARK=(65,39,27)


def profile(x0,x1,points,color):
    hull([(x,y,z) for x in (x0,x1) for y,z in points],color)


def geometry():
    # The lower bevel and four short feet lift the wooden body off the floor.
    hull([(x,y,z) for z,a,b in [(2,22,12),(4,24,14),(22,24,14)]
          for x in (-a,a) for y in (-b,b)],WOOD)
    for x in (-19,19):
        for y in (-9,9):
            box(x-3,y-3,0,x+3,y+3,3,IRON)
    box(-24.25,-14.25,3,24.25,14.25,5,IRON)
    box(-24.125,-14.125,21,24.125,14.125,22,DARK)
    arch=[(-14,22),(-14,26),(-10,32),(-4,35),(4,35),(10,32),(14,26),(14,22)]
    profile(-24,24,arch,WOOD)
    # Each curved strap is a single convex brush, sharing the lid's faceted profile.
    strap=[(-14.5,22),(-14.5,26.25),(-10.25,32.5),(-4,35.5),
           (4,35.5),(10.25,32.5),(14.5,26.25),(14.5,22)]
    for x in (-16,16):
        profile(x-1.5,x+1.5,strap,IRON)
        for y in (-14.25,14.25):
            box(x-1.5,y-.25,5,x+1.5,y+.25,21,IRON)
    # Shallow dark joints suggest broad planks without subdividing the solid body.
    for y in (-14.125,14.125):
        for z in (10,16):
            box(-23.875,y-.125,z,23.875,y+.125,z+.25,DARK)
    # Brass hasp and an inset-looking keyhole on the front.
    profile(-2,2,[(14,18),(15,18),(15,27),(14,28)],GOLD)
    profile(-3,3,[(15,16),(16,16),(16,22),(15,23)],GOLD)
    box(-.5,16,17.5,.5,16.125,20,DARK)
    # Open rectangular carrying handles on both ends.
    for side in (-1,1):
        x0,x1=sorted((side*24,side*26))
        for y in (-4,4):box(x0,y-.75,13,x1,y+.75,15,IRON)
        x0,x1=sorted((side*25,side*26))
        box(x0,-4.75,12.5,x1,4.75,13.5,IRON)


if __name__=='__main__':
    editor=Editor()
    try:
        info=build('chest',geometry,editor)
        brushes=json.loads(editor.call('list_brushes'))
        for brush,(_,color) in zip(brushes,r.SOLIDS):
            if color==WOOD:
                paint_world_gradient(editor,brush,(170,103,53),(114,57,31),(0,0,-1),(-35.5,0))
        editor.call('save',path='assets/chest.map')
        editor.call('render_preview',path='assets/chest.png',size=1000)
        p=Path('assets/scale_profiles.json');data=json.loads(p.read_text())
        data['assets']['chest']=info;p.write_text(json.dumps(data,indent=2)+'\n')
        print('chest finished;',len(brushes),'brushes;',Path('assets/chest.map').read_text().count(' skip '),'skip faces',flush=True)
    finally:
        editor.close()
