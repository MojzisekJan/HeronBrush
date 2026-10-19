#!/usr/bin/env python3
"""An empty stylised portal frame. 32 units/metre, 22 convex brushes.
Run from repository root: python3 assets/portal_frame_gen.py
The six turquoise inlays share one brush colour for easy pair recolouring.
"""
import json
from pathlib import Path
import redesign_gen as r

STONE=(163,169,151)
EDGE=(183,185,159)
BASE=(75,95,88)
BRONZE=(153,123,73)
PAIR_COLOUR=(46,208,190)


def prism(poly,y0,y1,colour):
    r.prism([(x*32,z*32) for x,z in poly],y0*32,y1*32,colour)


def bevel(poly,depth,colour):
    # One convex solid, bevelled on both faces, with no extra trim brushes.
    cx=sum(x for x,z in poly)/len(poly)
    cz=sum(z for x,z in poly)/len(poly)
    points=[]
    for y,scale in ((-depth/2,.94),(-depth/2+.055,1),
                    (depth/2-.055,1),(depth/2,.94)):
        points.extend(((cx+(x-cx)*scale)*32,y*32,(cz+(z-cz)*scale)*32) for x,z in poly)
    r.hull(points,colour)


def portal_frame():
    from portal_finish import rounded
    # Light weathered stone, split tapered columns, rounded rim mouldings.
    for side in (-1,1):
        mirror=lambda poly:[(side*x,z) for x,z in poly]
        rounded(mirror([(.90,0),(1.50,0),(1.42,.33),(.90,.33)]),.86,BASE,.07)
        rounded(mirror([(.94,.27),(1.36,.27),(1.30,1.25),(.94,1.25)]),.64,STONE,.055)
        rounded(mirror([(.94,1.25),(1.30,1.25),(1.35,2.35),(.90,2.05)]),.64,STONE,.055)
        rounded(mirror([(.90,2.05),(1.35,2.35),(.80,3.12),(.55,2.65)]),.72,EDGE,.07)
        rounded(mirror([(.89,1.18),(1.37,1.18),(1.37,1.32),(.89,1.32)]),.76,BRONZE,.025)
        for front in (-1,1):
            # Dark inset mount and a pointed coloured crystal. Both editable brushes.
            prism(mirror([(1.075,.47),(1.27,.69),(1.23,1.09),(1.075,1.14),(.995,.89)]),
                  *sorted((front*.285,front*.342)),BASE)
            prism(mirror([(1.105,.56),(1.20,.70),(1.17,1.03),(1.085,1.06),(1.06,.86)]),
                  *sorted((front*.328,front*.383)),PAIR_COLOUR)
    rounded([(-.55,2.65),(.55,2.65),(.80,3.12),(-.80,3.12)],.72,STONE,.065)
    rounded([(0,2.50),(.25,2.90),(0,3.30),(-.25,2.90)],.78,BRONZE,.035)
    for front in (-1,1):
        prism([(0,2.65),(.14,2.90),(0,3.15),(-.14,2.90)],*sorted((front*.36,front*.42)),PAIR_COLOUR)


if __name__=='__main__':
    r.portal_frame=portal_frame
    r.build('portal_frame')
    from portal_finish import finish
    finish('portal_frame',{
        STONE:([197,202,177],[116,136,125]), EDGE:([219,214,177],[144,161,139]),
        BASE:([113,131,114],[55,76,74]), BRONZE:([215,180,109],[108,81,48]),
        PAIR_COLOUR:([147,251,220],[25,126,126]),
    })

    from asset_radial import apply
    editor=Editor()
    try:
        apply("portal_frame", editor)
        editor.call("open_map", path="assets/portal_frame.map")
        editor.call("render_preview", path="assets/portal_frame.png", size=1200)
        editor.call("render_preview", path="assets/portal_frame_editor.png", size=800)
    finally:
        editor.close()
