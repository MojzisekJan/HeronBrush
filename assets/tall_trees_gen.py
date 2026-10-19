#!/usr/bin/env python3
"""Three tall, branching, low-brush trees. Run: python3 assets/tall_trees_gen.py
Dimensions use metres; shared scale profiles and skip processing apply on save.
"""
import math
import redesign_gen as r
from nature_gen import crown


def branch(a,b,r0,r1,color):
    """A tapered six-sided branch is one editable convex brush."""
    a=tuple(v*32 for v in a);b=tuple(v*32 for v in b)
    n=r.normal(r.sub(b,a));u=r.normal(r.cross(n,(0,0,1) if abs(n[2])<.9 else (0,1,0)));v=r.cross(n,u)
    r.hull([r.add(p,r.add(r.mul(u,rad*32*math.cos(i*math.pi/3)),r.mul(v,rad*32*math.sin(i*math.pi/3))))
            for p,rad in ((a,r0),(b,r1)) for i in range(6)],color)


def leaves(center,radii,color):
    crown(*(v*32 for v in center),*(v*32 for v in radii),color,sides=6)


def tree_elm():
    wood=(112,92,64)
    # Rising fork and secondary branches remain visible below a green crown.
    for a,b,r0,r1 in [((0,0,0),(0,0,3.8),.38,.27),
        ((0,0,3.6),(-.5,.1,6.2),.28,.18),((-.5,.1,6),(-1.8,.3,8),.18,.075),
        ((-.3,.1,4.8),(1.1,.2,6.7),.22,.12),((1.1,.2,6.6),(2.4,.6,8),.13,.06),
        ((-.5,.1,6.1),(-.4,-1.7,8.3),.17,.06),
        ((1,.2,6.5),(.5,1.7,8.2),.13,.06),((-.5,.1,6),(0,0,9),.17,.05)]:
        branch(a,b,r0,r1,wood)
    for c,rad,col in [((-2,.3,8.2),(1.9,1.6,1.35),(74,127,69)),
        ((2.3,.6,8.2),(1.7,1.55,1.3),(104,155,78)),
        ((-.4,-1.6,8.5),(1.65,1.6,1.3),(57,111,70)),
        ((.5,1.6,8.45),(1.7,1.65,1.25),(87,143,72)),
        ((0,0,8.65),(1.8,1.6,1.35),(125,171,87))]:leaves(c,rad,col)


def tree_acacia():
    wood=(139,108,69)
    # Crooked trunk, broad lateral forks, thin umbrella canopy.
    for a,b,r0,r1 in [((0,0,0),(0,0,2.7),.38,.29),
        ((0,0,2.5),(.65,.1,4.5),.3,.23),
        ((.6,.1,4.3),(-1.45,.1,5.5),.24,.14),
        ((-1.4,.1,5.4),(-3,.35,6.7),.15,.05),
        ((.6,.1,4.4),(2.6,.4,6.7),.22,.06),
        ((.4,.1,4.5),(.2,-2,6.8),.18,.055),
        ((.4,.1,4.5),(.4,2.2,6.8),.18,.055),
        ((.6,.1,4.4),(.15,.2,7.3),.18,.06)]:branch(a,b,r0,r1,wood)
    for c,rad,col in [((-2.9,.3,6.9),(2.25,1.65,.7),(106,132,66)),
        ((2.6,.4,7),(2.2,1.65,.7),(145,159,76)),
        ((.2,-1.9,7),(2.25,1.6,.7),(94,122,62)),
        ((.4,2.1,7),(2.2,1.65,.65),(124,149,70)),
        ((.1,.2,7.3),(2.35,1.9,.7),(158,171,83))]:leaves(c,rad,col)


def tree_maple():
    wood=(109,71,53)
    # Tall forked silhouette and an autumn crown with five solid leaf masses.
    for a,b,r0,r1 in [((0,0,0),(0,0,4),.45,.32),
        ((0,0,3.8),(-.65,.1,7),.31,.21),
        ((0,0,3.9),(1,.3,6.7),.3,.2),
        ((-.65,.1,6.8),(-2.4,.5,9.2),.22,.07),
        ((-.65,.1,6.9),(-.5,-1.9,9.6),.19,.06),
        ((1,.3,6.5),(2.45,.8,9.2),.2,.06),
        ((1,.3,6.6),(.55,2,9.6),.17,.06),
        ((-.65,.1,6.8),(-.3,0,10.7),.2,.065),
        ((1,.3,6.6),(-.1,0,9.4),.15,.07)]:branch(a,b,r0,r1,wood)
    for c,rad,col in [((-2.3,.5,9.25),(1.9,1.75,1.75),(196,88,43)),
        ((2.3,.8,9.35),(1.85,1.7,1.65),(231,153,53)),
        ((-.5,-1.8,9.65),(1.7,1.6,1.65),(173,64,44)),
        ((.55,1.9,9.75),(1.75,1.65,1.65),(218,116,40)),
        ((-.3,0,10.25),(1.9,1.7,1.75),(241,178,67))]:leaves(c,rad,col)


TREES={'tree_elm':tree_elm,'tree_acacia':tree_acacia,'tree_maple':tree_maple}
if __name__=='__main__':
    for name,builder in TREES.items():
        setattr(r,name,builder);r.build(name)
