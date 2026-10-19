#!/usr/bin/env python3
"""Generate sailboat and three stylized trees as editable brush prefabs.
Run from the repository root: python3 assets/nature_gen.py
Uses redesign_gen.py and prefabs_gen.py; previews need numpy and Pillow.
"""
import math
import redesign_gen as r
WOOD=(135,87,47); DARK=(67,65,56); CREAM=(239,226,185)
TEAL=(47,115,129); ROPE=(151,128,87)


def crown(x,y,z,rx,ry,rz,color,sides=8):
    # Staggered rings make broad triangular facets, rather than a smooth sphere.
    pts=[(x,y,z+rz),(x,y,z-rz*.8)]
    for height,rad,offset in ((-.42,.84,0),(.35,.9,.5)):
        for i in range(sides):
            a=(i+offset)*2*math.pi/sides
            pts.append((x+rx*rad*math.cos(a),y+ry*rad*math.sin(a),z+rz*height))
    r.hull(pts,color)


def tree_oak():
    r.rod((0,0,0),(2,1,69),6,WOOD,7)
    for a,b in [((1,0,40),(-25,3,77)),((2,0,50),(23,10,87)),((2,0,58),(6,-24,87))]:
        r.rod(a,b,3.5,WOOD,6)
    for a in range(0,360,90):
        t=math.radians(a)
        r.hull([(0,0,0),(12*math.cos(t),12*math.sin(t),0),
                (5*math.cos(t+.5),5*math.sin(t+.5),0),(0,0,16)],WOOD)
    crown(-25,3,83,30,29,29,(72,123,68))
    crown(22,12,91,32,29,32,(88,143,70))
    crown(7,-23,91,30,27,31,(63,119,71))
    crown(-3,0,111,37,33,34,(101,152,76))


def tree_pine():
    r.rod((0,0,0),(0,0,133),5,WOOD,8)
    for z,rad,top,col in [(22,40,86,(43,100,78)),(49,33,113,(48,116,82)),
                          (79,25,137,(57,131,89)),(108,17,155,(74,143,95))]:
        pts=[(rad*math.cos(i*math.pi/4),rad*math.sin(i*math.pi/4),z) for i in range(8)]
        r.hull(pts+[(0,0,top)],col)


def tree_birch():
    pale=(220,220,194)
    r.rod((0,0,0),(0,0,115),4,pale,8)
    for a,b in [((0,0,51),(-19,2,93)),((0,0,71),(17,2,113)),((0,0,86),(-5,-12,125))]:
        r.rod(a,b,2,pale,6)
    # Short dark bark patches on different sides of the trunk.
    for i,z in enumerate((9,21,35,48,64,79,95)):
        a=(i%4)*math.pi/2
        r.hull([(rad*math.cos(t),rad*math.sin(t),zz)
                for rad in (3.8,4.15) for t in (a-.3,a+.3) for zz in (z,z+2)],(77,78,68))
    crown(-18,2,99,22,19,34,(144,165,67))
    crown(17,2,117,23,21,32,(164,183,77))
    crown(-5,-13,126,21,20,34,(129,161,69))
    crown(0,2,143,21,20,29,(180,194,87))


def sailboat():
    # Tapered hull with a narrow keel, broad deck and pointed bow at +X.
    stations=[(-82,13),(-61,25),(40,26),(85,.8)]
    for (x0,w0),(x1,w1) in zip(stations,stations[1:]):
        r.hull([(x,s*w,z) for x,w in ((x0,w0),(x1,w1)) for s in (-1,1) for z in (23,28)],CREAM)
        r.hull([(x,s*w,23) for x,w in ((x0,w0),(x1,w1)) for s in (-1,1)]+
               [(x,s*w*.52,5) for x,w in ((x0,w0),(x1,w1)) for s in (-1,1)],TEAL)
    plan=[(-81,-12),(-60,-24),(40,-25),(84,0),(40,25),(-60,24),(-81,12)]
    r.hull([(x,y,z) for x,y in plan for z in (28,30)],(181,141,87))
    for i,a in enumerate(plan):
        b=plan[(i+1)%len(plan)]
        r.rod((*a,31),(*b,31),1.8,WOOD,6)
    # Shallow cabin and aft cockpit seat.
    r.hull([(-23,-13,30),(-23,13,30),(22,-13,30),(22,13,30),
            (-18,-10,43),(-18,10,43),(17,-10,43),(17,10,43)],CREAM)
    for y in (-11.5,11.5):
        r.rod((-12,y,37),(11,y,37),2.3,TEAL,6)
    r.box(-67,-17,30,-34,17,34,WOOD)
    r.box(-64,-14,34,-37,14,35,DARK)
    # Mast and boom support two separate, slightly billowed sails.
    r.rod((-8,0,30),(-8,0,170),2.3,WOOD,8)
    r.rod((-8,0,60),(-72,0,60),1.8,WOOD,6)
    r.hull([(-10,-.8,162),(-10,.8,162),(-10,-.8,64),(-10,.8,64),
            (-69,-.8,64),(-69,.8,64),(-28,5,98)],CREAM)
    r.hull([(-4,-.7,151),(-4,.7,151),(1,-.7,65),(1,.7,65),
            (72,-.7,42),(72,.7,42),(22,4,86)],(221,208,162))
    # Standing rigging and a small pennant.
    for b in [(83,0,32),(-81,0,32),(-8,-23,32),(-8,23,32)]:
        r.rod((-8,0,165),b,.55,ROPE,6)
    r.prism([(-8,168),(-8,179),(12,174)],-.7,.7,(170,67,47))
    # Transom rudder and tiller, both visibly attached.
    r.box(-88,-2,2,-83,2,28,WOOD)
    r.rod((-85,0,28),(-59,0,39),1.6,WOOD,6)


def main():
    for name,builder in [('sailboat',sailboat),('tree_oak',tree_oak),('tree_pine',tree_pine),('tree_birch',tree_birch)]:
        setattr(r,name,builder);r.build(name)

if __name__=='__main__':main()
