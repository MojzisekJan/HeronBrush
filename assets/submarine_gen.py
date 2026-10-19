#!/usr/bin/env python3
"""Stylised 6 m submarine; 24 convex brushes at 32 units/metre.
Run from repository root: python3 assets/submarine_gen.py
"""
import math
import redesign_gen as r
from portal_frame_gen import prism
from portal_finish import rounded,finish
YELLOW=(204,155,51)
DARK=(49,67,77)
GLASS=(65,155,181)
METAL=(157,173,173)
BRASS=(181,133,67)


def hull(points,colour):r.hull([tuple(v*32 for v in p) for p in points],colour)
def rod(a,b,radius,colour,sides=10):r.rod(tuple(v*32 for v in a),tuple(v*32 for v in b),radius*32,colour,sides)


def submarine():
    # One convex hull, with a blunt rounded bow and tapered stern. +X is forward.
    hull([(x,ry*math.cos(i*math.pi/6),.75+rz*math.sin(i*math.pi/6))
          for x,ry,rz in [(-2.60,.16,.16),(-2.25,.48,.48),(-1.40,.71,.71),
                          (1.45,.75,.75),(2.15,.66,.66),(2.65,.43,.43),(2.85,.12,.12)]
          for i in range(12)],YELLOW)
    rounded([(-.67,1.32),(.61,1.32),(.53,2.08),(.29,2.25),(-.39,2.25),(-.63,2.06)],.61,DARK,.06)
    # Low oval hatch and bent periscope.
    rod((-.09,0,2.23),(-.09,0,2.31),.20,METAL,10)
    rod((.18,0,2.29),(.18,0,2.93),.055,METAL,8)
    rod((.18,0,2.90),(.44,0,2.90),.075,DARK,8)
    rod((.43,0,2.90),(.46,0,2.90),.052,GLASS,8)
    # Three portholes on each side: a dark rim and blue lens, not holes in the hull.
    for side in (-1,1):
        for x in (-.86,.16,1.16):
            rod((x,side*.68,.93),(x,side*.748,.93),.145,DARK,10)
            rod((x,side*.743,.93),(x,side*.758,.93),.105,GLASS,10)
    # Tail planes: one horizontal brush and one vertical brush.
    hull([(-2.38,-1.05,.71),(-1.65,-.44,.71),(-1.65,.44,.71),(-2.38,1.05,.71),
          (-2.38,-1.05,.79),(-1.65,-.44,.79),(-1.65,.44,.79),(-2.38,1.05,.79)],DARK)
    prism([(-2.44,.40),(-1.91,.51),(-1.77,1.02),(-2.26,1.48),(-2.44,1.30)],-.055,.055,DARK)
    rod((-2.61,0,.75),(-3.03,0,.75),.085,METAL,8)
    # Three pitched blades, each a single convex brush.
    for i in range(3):
        a=i*2*math.pi/3
        pts=[]
        for x,u,v in [(-2.90,.07,-.055),(-2.97,.42,-.06),(-3.10,.47,.10),(-3.04,.10,.08)]:
            for thickness in (-.015,.015):
                pts.append((x+thickness,u*math.cos(a)-v*math.sin(a),.75+u*math.sin(a)+v*math.cos(a)))
        hull(pts,BRASS)


def finish_submarine():
    finish('submarine',{
        YELLOW:([241,198,91],[148,104,37]),DARK:([96,124,135],[32,49,66]),
        GLASS:([159,223,231],[34,110,150]),METAL:([210,222,218],[96,122,137]),
        BRASS:([228,192,118],[127,86,43]),
    },world_gradients={YELLOW:((0,0,-1),None)})


if __name__=='__main__':
    r.submarine=submarine;r.build('submarine')
    finish_submarine()
