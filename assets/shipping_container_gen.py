#!/usr/bin/env python3
"""Closed shipping container, approximately six metres long, on the 1/8 grid."""
import json
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import build, box, hull
from prefabs_gen import Editor
from portal_finish import paint_world_gradient

BLUE=(67,113,139); RIB=(77,126,151); FRAME=(48,79,95); STEEL=(174,183,174)

def geometry():
    # Closed solid core keeps this static prop inexpensive; doors are decorative.
    box(-94,-38,2,94,38,82,BLUE)
    for x in (-96,96):
        for y in (-38,38):box(x-2,y-2,0,x+2,y+2,84,FRAME)
    for y in (-38,38):
        for z in (0,80):box(-94,y-2,z,94,y+2,z+4,FRAME)
    for x in (-96,96):
        for z in (0,80):box(x-2,-36,z,x+2,36,z+4,FRAME)
    for side in (-1,1):
        y0,y1=sorted((side*.75,side*35.5))
        box(94,y0,5,96,y1,79,BLUE)
        y=side*18
        box(96,y-1,10,97.5,y+1,74,STEEL)
        box(97.5,y-4,35,98,y+4,37,STEEL)
        # Sparse trapezoidal corrugations, recessed below the protective frame.
        for x in (-77,-55,-33,-11,11,33,55,77):
            hull([(x+dx,side*y,z) for y,w in [(37.875,3),(39.5,1.5)]
                  for dx in (-w,w) for z in (7,77)],RIB)
    for y in (-22,0,22):
        box(-95.5,y-1.5,7,-94,y+1.5,77,RIB)
        box(-88,y-1.5,82,88,y+1.5,83,RIB)

if __name__=='__main__':
    editor=Editor()
    try:
        name='shipping_container'
        info=build(name,geometry,editor)
        brushes=json.loads(editor.call('list_brushes'))
        for brush,(_,color) in zip(brushes,r.SOLIDS):
            if color in (BLUE,RIB):
                light=tuple(min(255,c+8) for c in color);dark=tuple(max(0,c-12) for c in color)
                paint_world_gradient(editor,brush,light,dark,(0,0,-1),(-84,0))
        path=f'assets/{name}.map';editor.call('save',path=path)
        editor.call('render_preview',path=f'assets/{name}.png',size=1000)
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        data['assets'][name]=info;profiles.write_text(json.dumps(data,indent=2)+'\n')
        print('Finished:',len(brushes),'brushes;',Path(path).read_text().count(' skip '),'skip faces',flush=True)
    finally:editor.close()
