#!/usr/bin/env python3
"""Build crane, house and well brush assets and PNG thumbnails.
Run from the repository root: python3 assets/building_gen.py
Uses redesign_gen.py and prefabs_gen.py; previews need numpy and Pillow.
"""
import math
import redesign_gen as r
YELLOW=(223,166,43); DARK=(57,68,73); STEEL=(151,161,159)
WOOD=(139,89,48); ROOF=(165,65,46); CREAM=(225,208,167)
GLASS=(95,165,184); STONE=(172,179,172)


def crane():
    # Tower crane with an open lattice tower and a longer working jib.
    r.box(-38,-38,0,38,38,9,STONE)
    for x in (-29,17):
        for y in (-29,17): r.box(x,y,9,x+12,y+12,18,DARK)
    for x in (-11,11):
        for y in (-11,11):
            r.rod((x*2,y*2,14),(x,y,40),2.5,YELLOW,6)
            r.rod((x,y,18),(x,y,222),2.5,YELLOW,6)
    for z in range(30,222,32):
        for s in (-1,1):
            r.rod((-11,s*11,z),(11,s*11,z),2,YELLOW,6)
            r.rod((s*11,-11,z),(s*11,11,z),2,YELLOW,6)
            if z+32<=222:
                r.rod((-11,s*11,z),(11,s*11,z+32),1.6,YELLOW,6)
                r.rod((s*11,-11,z),(s*11,11,z+32),1.6,YELLOW,6)
    r.box(-18,-18,217,18,18,224,DARK)
    # Jib triangle and repeating diagonal webs.
    for a,b in [((-74,-10,228),(216,-10,228)),((-74,10,228),(216,10,228)),((-74,0,247),(216,0,247))]:
        r.rod(a,b,2.4,YELLOW,6)
    for i,x in enumerate(range(-74,216,29)):
        nxt=x+29
        for s in (-1,1):
            r.rod((x,s*10,228),(x,0,247),1.5,YELLOW,6)
            r.rod((x,s*10,228),(nxt,0,247),1.5,YELLOW,6)
        r.rod((x,-10,228),(x,10,228),1.5,YELLOW,6)
    for y in (-10,10): r.rod((216,y,228),(216,0,247),1.5,YELLOW,6)
    r.rod((216,-10,228),(216,10,228),1.5,YELLOW,6)
    # Tower head and tension stays.
    r.rod((0,0,224),(0,0,282),3,YELLOW,6)
    for x in (-70,166): r.rod((0,0,282),(x,0,247),1,DARK,6)
    for x in (-71,-58,-45): r.box(x,-17,211,x+11,17,229,STONE)
    # Cabin facing the working end.
    r.box(17,-17,197,43,17,222,YELLOW)
    for y in (-17.4,17.1): r.box(22,y,204,39,y+.3,218,GLASS)
    r.box(43,-13,204,43.5,13,219,GLASS)
    r.box(14,-20,222,47,20,226,YELLOW)
    # Trolley, hoist cable and an open hook.
    r.box(148,-13,223,164,13,230,DARK)
    for y in (-3,3): r.rod((156,y,224),(156,y,123),.8,DARK,6)
    r.box(150,-6,115,162,6,125,YELLOW)
    for a,b in [((156,0,115),(156,0,106)),((156,0,106),(161,0,101)),((161,0,101),(167,0,104)),((167,0,104),(168,0,109))]:
        r.rod(a,b,1.8,DARK,6)


def roof(y0,y1,half,eave,ridge):
    # Two solid roof slopes, not a filled triangular attic.
    for s in (-1,1):
        r.prism([(0,ridge),(s*half,eave),(s*half,eave-5),(0,ridge-5)],y0,y1,ROOF)
    r.rod((0,y0,ridge),(0,y1,ridge),2,ROOF,6)


def house():
    r.box(-64,-50,0,64,50,7,STONE)
    r.box(-60,-46,7,60,46,77,CREAM)
    r.prism([(-60,77),(60,77),(0,117)],-46,46,CREAM)
    roof(-53,53,69,77,123)
    # Wooden corner trim and low foundation course.
    for x in (-60.5,57.5):
        for y in (-46.5,43.5): r.box(x,y,7,x+3,y+3,77,WOOD)
    r.box(-61,-47,7,61,47,14,(153,158,147))
    # Front door and doorstep.
    r.box(-14,46,14,12,48,57,WOOD)
    r.box(-11,48,16,9,49,54,(104,66,39))
    r.box(-7,49,38,5,49.4,50,GLASS)
    r.rod((5,49,30),(5,51,30),1.2,(215,174,76),8)
    r.box(-19,47,0,17,65,5,STONE)
    r.box(-16,47,5,14,58,10,STONE)
    # Paired front windows and a smaller attic window.
    for x in (-46,28):
        r.box(x,46,31,x+22,48,55,WOOD)
        r.box(x+2,48,33,x+20,48.4,53,GLASS)
        r.box(x+10,48.4,33,x+12,49,53,CREAM)
        r.box(x+2,48.4,42,x+20,49,44,CREAM)
        r.box(x-2,46,29,x+24,52,32,WOOD)
    r.box(-9,46,84,9,47.5,101,WOOD)
    r.box(-7,47.5,86,7,48,99,GLASS)
    for s in (-1,1):
        for y in (-28,10):
            r.box(s*60-.3,y,32,s*60+.3,y+20,55,WOOD)
            r.box(s*60.5-.2,y+2,34,s*60.5+.2,y+18,53,GLASS)
    # Chimney rises through the rear roof slope.
    r.box(28,-31,88,42,-17,130,(132,82,63))
    r.box(26,-33,130,44,-15,135,(100,77,65))
    r.box(30,-29,135,40,-19,135.5,DARK)


def well():
    # Three staggered masonry courses around a real open shaft.
    n=12
    for row in range(3):
        for i in range(n):
            a=2*math.pi*(i+.5*(row%2))/n+.012
            b=2*math.pi*(i+1+.5*(row%2))/n-.012
            pts=[(rad*math.cos(t),rad*math.sin(t),z)
                 for rad in (25,34) for t in (a,b) for z in (row*12,row*12+11.5)]
            r.hull(pts,[(177,182,172),(157,166,159),(190,190,173)][(i+row)%3])
    for i in range(n):
        a=2*math.pi*i/n+.008;b=2*math.pi*(i+1)/n-.008
        r.hull([(rad*math.cos(t),rad*math.sin(t),z) for rad in (24,36) for t in (a,b) for z in (36,41)],(201,197,176))
    r.rod((0,0,4),(0,0,5),24,(63,126,146),16)
    # Timber supports, roof and winch.
    for x in (-42,35):
        r.box(x,-5,0,x+7,5,105,WOOD)
        r.box(x-2,-7,0,x+9,7,5,DARK)
    roof(-35,35,53,103,132)
    r.box(-43,-4,101,43,4,106,WOOD)
    r.rod((-45,0,74),(49,0,74),4,(107,69,42),12)
    r.rod((-17,0,74),(17,0,74),6,(166,133,88),12)
    for x in (-17,14): r.rod((x,0,74),(x+3,0,74),8,DARK,12)
    r.rod((48,0,74),(48,0,60),2,DARK)
    r.rod((48,0,60),(58,0,60),2,WOOD)
    r.rod((0,0,69),(0,0,28),.9,(188,159,111),6)
    # Small wooden bucket suspended inside the shaft.
    r.rod((0,0,20),(0,0,30),6,WOOD,10)
    r.rod((0,0,29),(0,0,31),6.5,DARK,10)
    for y in (-5,5): r.rod((0,y,30),(0,0,36),.7,DARK,6)


def main():
    for name,builder in [('crane',crane),('house',house),('well',well)]:
        setattr(r,name,builder);r.build(name)

if __name__=='__main__':main()
