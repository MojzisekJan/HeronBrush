#!/usr/bin/env python3
"""Minimal-brush ground props, authored in metres at 32 units per metre.
Run from repository root: python3 assets/ground_props_gen.py
"""
import math
import redesign_gen as r
from nature_gen import crown


def hull_metres(points,color):
    r.hull([tuple(v*32 for v in p) for p in points],color)


def rock_boulder():
    # One convex brush, with a grounded footprint and an asymmetric broken top.
    hull_metres([(-.8,-.36,0),(-.36,-.59,0),(.55,-.48,0),(.85,.08,0),(.42,.6,0),(-.6,.47,0),
        (-.68,-.31,.55),(-.3,-.47,.78),(.59,-.28,.57),(.7,.14,.42),(.38,.48,.65),(-.52,.38,.62),
        (-.22,-.14,1.1),(.32,-.05,.94),(.02,.29,.99)],(132,143,144))


def shrub():
    # Two intersecting leaf masses; no hidden supporting trunk is needed.
    crown(0,0,.4*32,.65*32,.5*32,.5*32,(75,123,64),sides=6)
    crown(.57*32,.12*32,.32*32,.48*32,.4*32,.4*32,(112,153,76),sides=6)


def stump():
    # Flared bark and a shallow inset cut end. Two brushes, no decorative rings.
    hull_metres([(rad*math.cos(i*math.pi/4),rad*math.sin(i*math.pi/4),z)
                 for z,rad in ((0,.43),(.58,.29)) for i in range(8)],(117,82,53))
    hull_metres([(.255*math.cos(i*math.pi/4),.255*math.sin(i*math.pi/4),z)
                 for z in (.54,.6) for i in range(8)],(209,174,113))


PROPS={'rock_boulder':rock_boulder,'shrub':shrub,'stump':stump}
if __name__=='__main__':
    for name,builder in PROPS.items():
        setattr(r,name,builder);r.build(name)
