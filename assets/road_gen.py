#!/usr/bin/env python3
"""Generate car.map and motorcycle.map with matching thumbnails.
Run from the repo root: python3 assets/road_gen.py
Uses the convex brush writer and depth-buffered renderer in redesign_gen.py.
"""
import math
import redesign_gen as r

BLUE=(51,119,154)
GLASS=(103,177,194)
RUBBER=(36,42,46)
STEEL=(155,169,171)
DARK=(60,73,79)
RED=(180,61,44)
CREAM=(242,223,169)


def car():
    # Compact two-door car, +X forward, tyres resting at Z=0.
    r.box(-87,-28,23,88,28,30,DARK)
    r.hull([(x,y,z) for x,y,z in [
        (-94,-27,32),(-94,27,32),(-89,-32,50),(-89,32,50),
        (86,-32,50),(86,32,50),(96,-27,35),(96,27,35),
        (91,-27,29),(91,27,29),(-89,-27,29),(-89,27,29)]],BLUE)
    # Sills between wheel wells; wheels sit outboard of the body.
    for y in (-33,29): r.box(-34,y,24,34,y+4,30,BLUE)
    for x in (-60,60):
        for s in (-1,1):
            r.rod((x,s*27,17),(x,s*37,17),17,RUBBER,16)
            r.rod((x,s*37,17),(x,s*37.8,17),10,STEEL,12)
            r.rod((x,s*37.8,17),(x,s*38.4,17),4,DARK,10)
    # The cabin is a trapezoid with a swept windscreen and rear hatch.
    r.hull([(x,y,z) for x,y,z in [(-64,-28,51),(-64,28,51),(49,-28,51),(49,28,51),
        (-40,-23,80),(-40,23,80),(21,-23,80),(21,23,80)]],GLASS)
    r.hull([(x,y,z) for x in (-42,23) for y in (-24,24) for z in (80,84)],BLUE)
    for s in (-1,1):
        # Pillars meet the roof and waist, and divide the side glazing.
        r.rod((-64,s*28,51),(-40,s*23,80),2.4,BLUE,4)
        r.rod((49,s*28,51),(21,s*23,80),2.4,BLUE,4)
        r.rod((-12,s*28,51),(-12,s*23,80),2,DARK,4)
        r.box(-63,s*29-1,49,49,s*29+1,52,BLUE)
        r.box(-4,s*32.2-1,42,7,s*32.2+1,44,STEEL)
        r.rod((38,s*28,56),(37,s*37,58),1.6,DARK)
        r.box(31,s*37-3,56,41,s*37+3,62,BLUE)
    # Short bonnet, distinct bumpers and paired lamps.
    r.box(49,-27,50,84,27,52,BLUE)
    r.box(94,-25,31,98,25,36,DARK)
    r.box(-97,-26,31,-93,26,36,DARK)
    r.box(94,-10,37,96,10,43,RUBBER)
    for y in (-25,15): r.box(91,y,41,94,y+10,48,CREAM)
    for y in (-25,17): r.box(-95,y,39,-93,y+8,47,RED)
    r.box(-97,-9,36,-96,9,41,CREAM)


def wheel(cx):
    # Annular tyre and rim leave real openings between the spokes.
    n=16;cz=22
    for i in range(n):
        a=2*math.pi*i/n;b=2*math.pi*(i+1)/n
        for inner,outer,half,c in [(16,22,5,RUBBER),(14.5,16,3.5,STEEL)]:
            pts=[(cx+rad*math.cos(t),y,cz+rad*math.sin(t))
                 for rad in (inner,outer) for t in (a,b) for y in (-half,half)]
            r.hull(pts,c)
    r.rod((cx,-6,cz),(cx,6,cz),4.5,DARK,10)
    for i in range(6):
        a=2*math.pi*i/6
        r.rod((cx,0,cz),(cx+15*math.cos(a),0,cz+15*math.sin(a)),1.3,STEEL,6)


def motorcycle():
    # A small standard road bike, +X forward.
    wheel(-48);wheel(53)
    for y in (-6,6):
        for a,b in [((-48,y,22),(-13,y,40)),((-13,y,40),(25,y,74)),
                    ((25,y,74),(-29,y,60)),((-29,y,60),(-48,y,22)),
                    ((-13,y,40),(-29,y,60))]:
            r.rod(a,b,2.4,DARK)
        r.rod((-48,y,22),(-27,y,62),2.8,STEEL)
        # Telescopic front fork.
        r.rod((53,y,22),(32,y,70),2.7,STEEL)
        r.rod((32,y,70),(25,y,84),3,DARK)
    # Engine block with cooling fins, below a faceted red fuel tank.
    r.box(-18,-9,36,9,9,47,DARK)
    for z in (48,52,56): r.box(-10,-10,z,7,10,z+2,STEEL)
    r.prism([(-14,60),(22,62),(13,77),(-8,73)],-10,10,RED)
    r.rod((6,0,77),(6,0,78.5),3,STEEL)
    # Seat and rear mudguard.
    r.hull([(-44,-9,62),(-44,9,62),(-11,-9,65),(-11,9,65),
            (-43,-9,67),(-43,9,67),(-11,-9,69),(-11,9,69)],RUBBER)
    r.prism([(-65,48),(-46,52),(-29,49),(-29,52),(-46,55),(-65,51)],-7,7,RED)
    r.prism([(33,42),(50,48),(68,44),(68,47),(50,51),(33,45)],-7,7,RED)
    # Exhaust and tail lamp.
    r.rod((6,11,39),(-35,11,30),3,DARK)
    r.rod((-35,11,30),(-57,11,32),4,STEEL)
    r.box(-66,-5,48,-64,5,53,(202,44,36))
    # Headlight and wide handlebars.
    r.rod((29,0,75),(40,0,75),7,DARK,12)
    r.rod((40,0,75),(41,0,75),5.5,CREAM,12)
    r.rod((25,0,80),(22,0,89),2.4,STEEL)
    for s in (-1,1):
        r.rod((22,0,89),(17,s*19,90),2,STEEL)
        r.rod((17,s*19,90),(13,s*25,90),2.5,RUBBER)
        r.rod((18,s*16,90),(21,s*19,103),1.2,DARK,6)
        r.box(17,s*19-3,102,25,s*19+3,106,STEEL)


def main():
    for name,builder in [('car',car),('motorcycle',motorcycle)]:
        setattr(r,name,builder)
        r.build(name)

if __name__=='__main__': main()
