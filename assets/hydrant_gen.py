#!/usr/bin/env python3
"""Small editable hydrant, 14 convex brushes, 32 units/metre.
Run from repository root: python3 assets/hydrant_gen.py
"""
import math
import redesign_gen as r
from portal_finish import finish
RED=(191,48,38)
TRIM=(220,181,105)
DARK=(64,72,78)


def rod(a,b,radius,colour,sides=10):
    r.rod(tuple(v*32 for v in a),tuple(v*32 for v in b),radius*32,colour,sides)


def profile(rings,colour):
    r.hull([(radius*math.cos(i*math.pi/6)*32,radius*math.sin(i*math.pi/6)*32,z*32)
            for z,radius in rings for i in range(12)],colour)


def hydrant():
    profile([(0,.20),(.018,.22),(.07,.22),(.09,.19)],DARK)
    profile([(.065,.155),(.13,.145),(.76,.145),(.79,.16)],RED)
    profile([(.755,.17),(.77,.185),(.815,.185),(.83,.17)],TRIM)
    # One convex dome with a rounded profile, no stack of separate brushes.
    profile([(.81,.173),(.86,.173),(.93,.14),(.985,.07),(1.005,.035)],RED)
    rod((0,0,.985),(0,0,1.05),.042,DARK,6)
    for side in (-1,1):
        rod((side*.125,0,.62),(side*.265,0,.62),.077,RED)
        rod((side*.25,0,.62),(side*.305,0,.62),.103,TRIM,12)
        rod((side*.30,0,.62),(side*.335,0,.62),.035,DARK,6)
    rod((0,.12,.43),(0,.24,.43),.10,RED,12)
    rod((0,.22,.43),(0,.28,.43),.122,RED,12)
    rod((0,.275,.43),(0,.31,.43),.044,TRIM,6)


def finish_hydrant():
    finish('hydrant',{
        RED:([234,81,57],[132,29,33]),
        TRIM:([245,216,155],[154,114,61]),
        DARK:([103,119,126],[43,52,61]),
    },world_gradients={
        # One height ramp across the body, dome and red outlets.
        RED:((0,0,-1),(-1.005*32,-.065*32)),
        TRIM:((0,0,-1),None),
        DARK:((0,0,-1),None),
    })


if __name__=='__main__':
    r.hydrant=hydrant;r.build('hydrant')
    finish_hydrant()
