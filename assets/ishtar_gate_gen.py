#!/usr/bin/env python3
"""Blue glazed temple gate with stepped battlements and shallow animal reliefs."""
import json
import math
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import build, hull, box
from prefabs_gen import Editor
from portal_finish import paint_world_gradient

BLUE=(35,85,159); DARK=(25,61,112); GOLD=(219,175,79); STONE=(208,193,149)
TURQUOISE=(55,155,171)


def prism(poly,y0,y1,color):
    hull([(x,y,z) for x,z in poly for y in (y0,y1)],color)


def bevel_block(x0,x1,y0,y1,z0,z1,color,bevel=2):
    outline=[(x0+bevel,y0),(x1-bevel,y0),(x1,y0+bevel),(x1,y1-bevel),
             (x1-bevel,y1),(x0+bevel,y1),(x0,y1-bevel),(x0,y0+bevel)]
    hull([(x,y,z) for x,y in outline for z in (z0,z1)],color)


def lion(cx,cz,direction):
    def relief(poly,color=GOLD,depth=0):
        prism([(cx+direction*x,cz+z) for x,z in poly],36+depth,37+depth,color)
    relief([(-15,2),(9,2),(12,8),(7,13),(-11,12)],STONE)
    relief([(7,10),(9,17),(15,19),(19,15),(19,8),(14,5),(9,6)])
    relief([(17,10),(23,10),(24,14),(19,16)],STONE)
    relief([(11,17),(12,22),(16,19)])
    for x in (-10,9):
        relief([(x-2,-8),(x+5,-8),(x+2,-4),(x+2,5),(x-2,5)],STONE)
    relief([(-14,6),(-20,10),(-22,22),(-19,23),(-17,12),(-13,10)])
    relief([(16,14),(18,14),(18,16),(16,16)],DARK,1)


def rosette(cx,cz,y0,y1,size=8):
    # Four pointed petals surround a contrasting glazed centre.
    for turn in range(4):
        points=[]
        for x,z in [(0,2),(3,5),(0,size),(-3,5)]:
            for _ in range(turn):x,z=-z,x
            points.append((cx+x,cz+z))
        prism(points,y0,y1,GOLD)
    prism([(cx-3,cz),(cx,cz-3),(cx+3,cz),(cx,cz+3)],y0,y1+.25,TURQUOISE)


def merlon(x0,x1,y0,y1,z0,z1):
    box(x0,y0,z0,x1,y1,z0+8,BLUE)
    box(x0+4,y0,z0+8,x1-4,y1,z1,BLUE)


def geometry():
    for side in (-1,1):
        x0,x1=sorted((side*44,side*112))
        bevel_block(x0,x1,-36,36,16,208,BLUE)
        bevel_block(x0-2,x1+2,-38,38,0,8,STONE)
        bevel_block(x0-1,x1+1,-37,37,8,20,DARK)
        for z in (25,181):box(x0-.125,-36.5,z,x1+.125,36.5,z+3,GOLD)
        box(x0-2,-38,202,x1+2,38,208,GOLD)
        bevel_block(x0-1,x1+1,-37,37,208,216,BLUE)
        for x in (x0+4,x0+26,x0+48):
            for y0,y1 in [(-36,-23),(23,36)]:merlon(x,x+16,y0,y1,216,232)
        for x in (x0,x1-12):merlon(x,x+12,-8,8,216,232)
        # Projecting outer pilasters and continuous gold opening jambs, front and back.
        for front in (-1,1):
            ya,yb=sorted((front*34,front*37.875))
            outside=x1-8 if side>0 else x0
            bevel_block(outside,outside+8,ya,yb,20,181,DARK,1)
            ya,yb=sorted((front*37.875,front*38))
            box(outside+3,ya,28,outside+5,yb,177,GOLD)
            xa,xb=sorted((side*44,side*52))
            ya,yb=sorted((front*36,front*38))
            box(xa,ya,20,xb,yb,108,GOLD)
        for height in (68,122):lion(side*78,height,-side)
        # Continue the animal frieze onto the exposed outer wall of each tower.
        first=len(r.SOLIDS)
        lion(0,96,1)
        reliefs=list(r.SOLIDS[first:]);del r.SOLIDS[first:];del r.BRUSHES[first:]
        for points,color in reliefs:
            hull([(side*(112+y-36),x,z) for x,y,z in points],color)
        rosette(side*78,171,36,37.5)
        prism([(side*78-10,151),(side*78,139),(side*78+10,151),(side*78,163)],-37,-36,GOLD)
        prism([(side*78-5,151),(side*78,145),(side*78+5,151),(side*78,157)],-37.5,-37,TURQUOISE)
    inner=[(round(44*math.cos(i*math.pi/8)*8)/8,round((108+44*math.sin(i*math.pi/8))*8)/8) for i in range(9)]
    outer=[(round(52*math.cos(i*math.pi/8)*8)/8,round((108+52*math.sin(i*math.pi/8))*8)/8) for i in range(9)]
    for i in range(8):
        a,b=inner[i],inner[i+1]
        prism([a,b,(b[0],190),(a[0],190)],-36,36,BLUE)
        for y0,y1 in [(-38,-36),(36,38)]:prism([a,b,outer[i+1],outer[i]],y0,y1,GOLD)
    box(-44,-37,190,44,37,196,GOLD)
    box(-44,-36,196,44,36,200,BLUE)
    for x in (-37,-9,19):merlon(x,x+18,-32,32,200,212)
    rosette(0,176,36,37.75,10)


if __name__=='__main__':
    editor=Editor()
    try:
        name='ishtar_gate';info=build(name,geometry,editor)
        brushes=json.loads(editor.call('list_brushes'))
        solids=[json.loads(editor.call('describe_brush',brush=b['id'])) for b in brushes]
        for x in (-20,0,20):
            for y in (-40,0,40):
                for z in (2,64,108,140):
                    for solid in solids:
                        inside=all(sum(f['normal'][i]*([x,y,z][i]-f['vertices'][0][i]) for i in range(3)) < -1e-6 for f in solid['faces'])
                        assert not inside, f'blocked passage at {(x,y,z)}'
        for brush,(_,color) in zip(brushes,r.SOLIDS):
            if color==BLUE:paint_world_gradient(editor,brush,(47,105,185),(26,64,130),(0,0,-1),(-232,0))
        for colors,roughness in [((BLUE,DARK,TURQUOISE),.28),((GOLD,),.4),((STONE,),.8)]:
            ids=[brush['id'] for brush,(_,color) in zip(brushes,r.SOLIDS) if color in colors]
            editor.call('select',ids=ids)
            editor.call('set_surface_material',roughness=roughness,metallic=0,emission=0)
        path=f'assets/{name}.map';editor.call('save',path=path)
        editor.call('render_preview',path=f'assets/{name}.png',size=1000)
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        data['assets'][name]=info;profiles.write_text(json.dumps(data,indent=2)+'\n')
        print('Finished:',len(brushes),'brushes;',Path(path).read_text().count(' skip '),'skip faces',flush=True)
    finally:editor.close()
