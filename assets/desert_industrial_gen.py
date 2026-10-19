#!/usr/bin/env python3
"""Build cactus and brick factory chimney as editable brush assets.
Run from the repo root: python3 assets/desert_industrial_gen.py
Uses redesign_gen.py and prefabs_gen.py; PNG previews need numpy and Pillow.
"""
import math
import random
import redesign_gen as r
GREEN=(64,127,82); RIB=(90,157,93); THORN=(218,203,144)


def upright(x,y,z0,z1,radius,color):
    # Straight fluted body with a rounded, faceted growing tip.
    n=10
    rings=[(z0,radius*.87),(z0+3,radius),(z1-radius*.8,radius),
           (z1-radius*.25,radius*.68),(z1,radius*.15)]
    for (za,ra),(zb,rb) in zip(rings,rings[1:]):
        r.hull([(x+rad*math.cos(i*2*math.pi/n),y+rad*math.sin(i*2*math.pi/n),z)
                for z,rad in ((za,ra),(zb,rb)) for i in range(n)],color)
    for i in range(n):
        a=i*2*math.pi/n
        r.rod((x+radius*math.cos(a),y+radius*math.sin(a),z0+4),
              (x+radius*math.cos(a),y+radius*math.sin(a),z1-radius),.55,RIB,5)


def cactus():
    upright(0,0,0,122,11,GREEN)
    # Unequal arms keep the silhouette readable from either side.
    for x,z,top in [(-30,45,85),(29,64,103)]:
        r.rod((0,0,z),(x,0,z),6,GREEN,10)
        upright(x,0,z-5,top,6,GREEN)
        for y in (-5.5,5.5):
            r.rod((x*.3,y,z+1),(x,y,z+1),.55,RIB,5)
    # Sparse short spines on the visible ribs, kept as real geometry.
    for cx,z0,z1,rad in [(0,20,103,11),(-30,57,76,6),(29,76,94,6)]:
        for z in range(z0,z1,17):
            for a in (.63,1.88,3.77,5.03):
                start=(cx+rad*math.cos(a),rad*math.sin(a),z)
                end=(cx+(rad+3)*math.cos(a),(rad+3)*math.sin(a),z+1.2)
                r.rod(start,end,.35,THORN,5)


def masonry(z0,z1,r0,r1,inner0,inner1,segments,offset,palette,rng,gap=.012):
    # Each voussoir is a convex solid; the centre remains empty.
    for i in range(segments):
        a=2*math.pi*(i+offset)/segments+gap
        b=2*math.pi*(i+1+offset)/segments-gap
        r.hull([(radius*math.cos(t),radius*math.sin(t),z)
                for z,outer,inner in ((z0,r0,inner0),(z1,r1,inner1))
                for radius in (inner,outer) for t in (a,b)],rng.choice(palette))


def factory_chimney():
    rng=random.Random(29)
    bricks=[(161,77,50),(178,87,55),(188,99,64),(149,70,48),(172,82,51)]
    r.box(-35,-35,0,35,35,8,(143,145,130))
    # A broad masonry plinth, then a tapered hollow shaft.
    for course in range(3):
        masonry(8+course*8,15.5+course*8,29,29,20,20,12,.5*(course%2),bricks,rng)
    for course in range(22):
        z=32+course*10
        rad=26-course*.48
        masonry(z,z+9.5,rad,rad-.456,rad-5,rad-5.456,
                14,.5*(course%2),bricks,rng)
    # Corbelled brick crown and a dark soot-stained inside lip.
    cap=[(190,103,67),(172,87,57),(182,94,61)]
    masonry(252,257,17,19,10.4,10.4,14,0,cap,rng)
    masonry(257,264,19,19,10.4,10.4,14,.5,cap,rng)
    masonry(256,263.7,10.55,10.55,10.1,10.1,14,.5,[(65,56,48)],rng,gap=.001)


def main():
    for name,builder in [('cactus',cactus),('factory_chimney',factory_chimney)]:
        setattr(r,name,builder);r.build(name)

if __name__=='__main__': main()
