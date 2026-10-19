#!/usr/bin/env python3
"""Four compact steel drums with identical geometry; 32 units/metre."""
import json
import math
import sys
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import build, hull
from portal_finish import paint_world_gradient
from prefabs_gen import Editor

STEEL=(120,135,140); DARK_STEEL=(72,85,91)
PALETTES={
    'barrel_green':((64,132,80),(83,155,97),(45,105,61)),
    'barrel_red':((180,60,46),(197,76,59),(154,47,39)),
    'barrel_blue':((52,113,162),(66,136,187),(42,93,140)),
    'barrel_yellow':((211,166,61),(229,187,78),(185,140,45)),
}

def profile(z0,z1,radius,color,cx=0,cy=0):
    # Integer octagon shared by every adjoining segment.
    bevel=max(1,round(radius*.4))
    ring=[(radius,bevel),(bevel,radius),(-bevel,radius),(-radius,bevel),
          (-radius,-bevel),(-bevel,-radius),(bevel,-radius),(radius,-bevel)]
    hull([(cx+x,cy+y,z) for x,y in ring for z in (z0,z1)],color)

def geometry(color):
    # Stack sections instead of burying reinforcing beads in the shell.
    for z0,z1,radius,tint in [(0,2,10,DARK_STEEL),(2,8,9,color),
            (8,10,10,STEEL),(10,18,9,color),(18,20,10,STEEL),
            (20,26,9,color),(26,28,10,DARK_STEEL),(28,29,9,color)]:
        profile(z0,z1,radius,tint)
    profile(29,30,2,STEEL,cx=4,cy=2)
    profile(29,30,1,STEEL,cx=-4,cy=-2)

if __name__=='__main__':
    editor=Editor()
    try:
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        for name,(color,light,dark) in PALETTES.items():
            if len(sys.argv)>1 and name not in sys.argv[1:]:continue
            data['assets'][name]=build(name,lambda:geometry(color),editor)
            brushes=json.loads(editor.call('list_brushes'))
            # A shared world-space ramp avoids restarting the gradient on each facet.
            for brush,(_,base) in zip(brushes,r.SOLIDS):
                if base==color:paint_world_gradient(editor,brush,light,dark,(0,0,-1),(-30,0))
            path=f'assets/{name}.map'
            editor.call('select',what='all')
            editor.call('set_surface_material',roughness=.42,metallic=.75,emission=0)
            editor.call('group_selection',name=name.replace('_',' ').title())
            editor.call('save',path=path)
            editor.call('select',what='all');editor.call('ungroup_selection')
            editor.call('render_preview',path=f'assets/{name}.png',size=800)
            data['assets'][name]['anchor']='32 units/metre; ground Z=0; authored on grid 1'
            print(name,'finished;',Path(path).read_text().count(' skip '),'skip faces',flush=True)
        profiles.write_text(json.dumps(data,indent=2)+'\n')
    finally:editor.close()
