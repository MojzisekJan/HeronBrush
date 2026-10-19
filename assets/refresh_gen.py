#!/usr/bin/env python3
"""Rebuild 13 editable brush assets and library thumbnails through Heron Brush MCP.

Run from the repository root: python3 assets/refresh_gen.py
Dragon, Steve and the imported monkey mesh are kept as they are.
"""
import json
import math
from pathlib import Path
import prefabs_gen as p
import vehicles_gen as v

WOOD = [166, 112, 65]
EDGE = [112, 73, 44]
METAL = [70, 85, 91]
BRASS = [214, 172, 88]
STONE = [186, 185, 170]
GLASS = [133, 195, 207]

class Editor(p.Editor):
    def call(self, name, **args):
        # Legacy builders finish early; the final save belongs to this script.
        if name in ('save', 'render_preview', 'add_point_entity'):
            return ''
        return super().call(name, **args)

    def finish(self, name):
        if name in ('bookshelf', 'bench'):
            self.call('select', what='all')
            self.call('rotate_selection', axis=[0,0,1], degrees=180)
        count = len(json.loads(self.call('list_brushes')))
        path = f'assets/{name}.map'
        super().call('save', path=path)
        self.call('open_map', path=path)
        assert len(json.loads(self.call('list_brushes'))) == count, name
        super().call('render_preview', path=f'assets/{name}.png', size=700)
        print(f'{name}: {count} brushes, save/reload verified', flush=True)

def box(ed, bounds, color=WOOD, material='wood'):
    p.box(ed, *bounds, color, material)

def beam(ed, a, b, width=3, color=METAL):
    # Rotate a vertical cuboid about its centre to join the two endpoints.
    mid = [(a[i]+b[i])/2 for i in range(3)]
    d = [b[i]-a[i] for i in range(3)]
    length = math.sqrt(sum(t*t for t in d))
    box(ed, (mid[0]-width/2, mid[1]-width/2, mid[2]-length/2,
             mid[0]+width/2, mid[1]+width/2, mid[2]+length/2), color, 'metal')
    axis = [-d[1], d[0], 0]
    if abs(axis[0])+abs(axis[1]) > 1e-8:
        axis = [t / math.sqrt(sum(v*v for v in axis)) for t in axis]
        ed.call('rotate_selection', axis=axis,
                degrees=math.degrees(math.acos(d[2]/length)), center=mid)

def crate(ed):
    for i in range(5):
        a=-30+i*12
        box(ed,(a,-30,2,a+11,30,4))
        box(ed,(a,-30,56,a+11,30,59))
        for y in (-30,27): box(ed,(a,y,4,a+11,y+3,56))
        for x in (-30,27): box(ed,(x,a,4,x+3,a+11,56))
    for x in (-31,25):
        for y in (-31,25): box(ed,(x,y,0,x+6,y+6,61),EDGE)
    for y in (-32,30):
        for z in (6,48): box(ed,(-31,y,z,31,y+2,z+6),METAL,'metal')
        beam(ed,(-23,y,15),(23,y,46),5,EDGE)
        for x in (-27,27):
            for z in (9,51): v.cyl(ed,x-1.5,y-1,z-1.5,x+1.5,y+3,z+1.5,BRASS,sides=8)

def bench(ed):
    for y in (-17,-7,3,13): box(ed,(-58,y,38,58,y+8,44))
    for z in (52,64,76): box(ed,(-58,20,z,58,25,z+9))
    for x in (-46,40):
        for y in (-14,15): box(ed,(x,y,0,x+6,y+6,39),METAL,'metal')
        box(ed,(x,19,35,x+6,24,86),METAL,'metal')
        box(ed,(x,-17,32,x+6,25,38),METAL,'metal')
        box(ed,(x,-14,44,x+6,-9,61),METAL,'metal')
        box(ed,(x-2,-20,61,x+8,26,66),EDGE)
    box(ed,(-40,16,15,40,21,20),METAL,'metal')

def pillar(ed):
    for b in [(-30,-30,0,30,30,8),(-25,-25,8,25,25,16),(-25,-25,134,25,25,142),(-31,-31,142,31,31,152)]: box(ed,b,STONE,'stone')
    v.cyl(ed,-20,-20,16,20,20,134,STONE,axis='z',sides=12,material='stone')
    for z in (18,125): v.cyl(ed,-23,-23,z,23,23,z+8,[151,155,147],axis='z',sides=12,material='stone')
    for i in range(12):
        a=2*math.pi*i/12
        x,y=20*math.cos(a),20*math.sin(a)
        v.cyl(ed,x-1.5,y-1.5,32,x+1.5,y+1.5,120,[208,205,188],axis='z',sides=6,material='stone')

def lamp(ed):
    v.cyl(ed,-18,-18,0,18,18,7,METAL,axis='z')
    v.cyl(ed,-10,-10,7,10,10,18,METAL,axis='z')
    v.cyl(ed,-4,-4,18,4,4,133,METAL,axis='z')
    for z in (20,110,128): v.cyl(ed,-6,-6,z,6,6,z+4,BRASS,axis='z')
    box(ed,(-16,-16,133,16,16,139),METAL,'metal')
    box(ed,(-11,-11,140,11,11,171),[249,209,124],'glass')
    for x in (-15,12):
        for y in (-15,12): box(ed,(x,y,139,x+3,y+3,174),METAL,'metal')
    box(ed,(-17,-17,173,17,17,177),METAL,'metal')
    ed.call('add_shape',kind='cone',min=[-23,-23,177],max=[23,23,198],axis='z',sides=4,material='metal')
    ed.call('set_face_color',color=METAL)
    v.sphere(ed,-4,-4,196,4,4,204,BRASS,'metal')

def bookshelf(ed):
    for x in (0,122): box(ed,(x,0,8,x+6,30,206))
    # Open back keeps the books accessible from either side.
    for z in (8,56,104,152,200): box(ed,(0,-2,z,128,30,z+6))
    for b in [(-4,-5,0,132,34,8),(-5,-5,206,133,34,214),(-2,-2,214,130,31,219)]: box(ed,b,EDGE)
    colors=[[152,62,49],[63,111,110],[68,83,122],[176,135,65],[124,79,103]]
    for row in range(4):
        base=14+row*48
        for i in range(9):
            x=10+i*12; height=28+(i*7+row*3)%12; col=colors[(i+row)%5]
            box(ed,(x,3,base,x+10,24,base+height),col,'paper')
            for z in (base+4,base+height-5): box(ed,(x,2,z,x+10,3,z+1.5),BRASS,'paper')
            box(ed,(x+2,1.8,base+height/2-3,x+8,3,base+height/2+3),[221,208,167],'paper')

def bridge(ed):
    # Short masonry piers leave a real opening beneath the arch.
    for x in (-38,330): box(ed,(x,0,0,x+92,96,66),[148,153,143],'stone')
    for i in range(15):
        angle=math.pi*(i+.5)/15
        cx=192+145*math.cos(angle); cz=18+70*math.sin(angle)
        box(ed,(cx-17,3,cz-10,cx+17,93,cz+10),STONE,'stone')
        ed.call('rotate_selection',axis=[0,1,0],degrees=-math.degrees(math.atan2(-70*math.cos(angle),145*math.sin(angle))),center=[cx,48,cz])
    # Deck follows the arch as broad shallow steps.
    for i in range(19):
        x=-36+i*24; z=66+23*math.sin(math.pi*i/18)
        box(ed,(x,0,z,x+24,96,z+9),STONE,'stone')
        for y in (0,86):
            box(ed,(x+3,y,z+9,x+11,y+10,z+35),[161,168,156],'stone')
            box(ed,(x,y-1,z+35,x+24,y+11,z+42),STONE,'stone')

def details(ed,name):
    if name=='truck':
        # Cab faces -X; its original forward details faced into the load bed.
        for z in (42,47,52): box(ed,(-4,19,z,-3,61,z+1.5),BRASS,'metal')
        box(ed,(-9,-3,29,-3,83,36),[171,182,181],'metal')
        box(ed,(3,-3,100,71,83,104),[151,43,37],'metal')
        for y in (-1,79):
            box(ed,(14,y,71,56,y+2,95),GLASS,'glass')
            box(ed,(47,y-1,58,58,y+3,61),BRASS,'metal')
        for cx in (40,184):
            for y in (-5,84): v.cyl(ed,cx-9,y,11,cx+9,y+2,29,[163,172,176],sides=12)
        for x in range(88,220,16): box(ed,(x,7,42,x+14,73,44),WOOD)
        for y in (0,75):
            for x in (94,150,208): box(ed,(x,y,43,x+4,y+5,64),METAL,'metal')
    elif name=='locomotive':
        for cx in (30,105,180):
            for y in (2,76):
                v.cyl(ed,cx-17,y,9,cx+17,y+2,43,[155,65,52],sides=12)
                v.cyl(ed,cx-5,y-1,21,cx+5,y+3,31,BRASS,sides=10)
        for y in (0,79): box(ed,(25,y,23,186,y+3,28),BRASS,'metal')
        for x in (22,80,145): v.cyl(ed,x,12,42,x+4,68,106,BRASS,axis='x',sides=16)
        for y in (6,72):
            for x in (180,211): box(ed,(x,y,91,x+21,y+2,119),GLASS,'glass')
        box(ed,(165,2,128,246,78,135),METAL,'metal')
        v.cyl(ed,-18,22,52,-14,58,94,METAL,axis='x',sides=16)
        v.cyl(ed,-23,33,85,-18,47,99,[249,217,143],axis='x')
        box(ed,(238,10,84,264,70,88),METAL,'metal')
    elif name=='tram':
        for y in (-3,73):
            box(ed,(0,y,51,300,y+2,56),[222,188,104],'metal')
            for x in (94,214):
                box(ed,(x,y,23,x+30,y+2,95),METAL,'metal')
                for dx in (3,17): box(ed,(x+dx,y-1,61,x+dx+10,y+3,90),GLASS,'glass')
        for cx in (44,76,224,256):
            for y in (2,65): v.cyl(ed,cx-10,y,-2,cx+10,y+5,18,METAL)
        for x in (-4,301):
            for y in (12,52): box(ed,(x,y,34,x+3,y+8,43),[248,217,148],'glass')
        for s in (-1,1):
            beam(ed,(116,36,115),(150,36+s*22,143),3)
            beam(ed,(150,36+s*22,143),(184,36,160),3)
        box(ed,(174,4,160,193,68,163),METAL,'metal')
    elif name=='airship':
        for x in (94,170,245): v.cyl(ed,x,-41,79,x+4,41,161,BRASS,axis='x',sides=16,material='cloth')
        for x in (132,156,180,204):
            for y in (-19,18): box(ed,(x,y,45,x+14,y+1,60),GLASS,'glass')
        for x in (136,204):
            for s in (-1,1): beam(ed,(x,s*15,72),(x,s*30,87),3,BRASS)
        for y in (-36,28):
            v.cyl(ed,164,y,47,190,y+8,59,METAL,axis='x')
            box(ed,(190,y+2,36,193,y+6,70),WOOD)
    elif name=='boat':
        box(ed,(26,10,108,114,70,113),METAL,'metal')
        for x in range(120,210,18): box(ed,(x,7,56,x+1,73,57),EDGE)
        for x in (16,120,200):
            for y in (4,74):
                box(ed,(x,y,64,x+3,y+3,85),[225,222,201],'metal')
        for y in (4,74): box(ed,(12,y,84,220,y+3,87),[225,222,201],'metal')
        beam(ed,(24,40,67),(166,40,180),2,EDGE)
        beam(ed,(270,40,56),(166,40,180),2,EDGE)
        for x in (55,90):
            for y in (-6,81): v.cyl(ed,x-8,y,35,x+8,y+4,51,METAL,sides=12)
        box(ed,(80,29,113,93,41,140),METAL,'metal')
        box(ed,(78,27,138,95,43,142),[161,65,49],'metal')
    elif name=='plane':
        for y in (-125,100): box(ed,(123,y,82,173,y+22,83),[230,214,167],'metal')
        for y in (-15,14): box(ed,(112,y,105,157,y+1,120),GLASS,'glass')
        for x in (110,136,159): box(ed,(x,-15,103,x+2,15,123),METAL,'metal')
        for s in (-1,1): beam(ed,(100,s*17,65),(126,s*58,34),4,METAL)
        for y in (-66,65): v.cyl(ed,120,y,23,132,y+2,35,BRASS,sides=10)


def main():
    simple={'crate':crate,'bench':bench,'pillar':pillar,'lamp':lamp,'bookshelf':bookshelf,'bridge':bridge}
    for name in [*simple,'truck','locomotive','tram','airship','boat','plane','windmill']:
        if name == 'windmill':
            from windmill_gen import build
            build()
            continue
        if name in ('bridge','bookshelf','airship','plane','tram'):
            from redesign_gen import build
            build(name)
            continue
        ed=Editor()
        try:
            ed.call('new_map')
            if name in simple: simple[name](ed)
            else:
                getattr(v,name)(ed)
                details(ed,name)
            ed.finish(name)
        finally: ed.close()

if __name__=='__main__': main()
