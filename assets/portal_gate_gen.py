#!/usr/bin/env python3
"""Empty bronze-and-slate portal on a stone dais, 23 convex brushes.
32 units/metre. Both approaches have a 4-unit step up to the 8-unit dais.
The six amber inlays share one colour for portal-pair recolouring.
Run from the repository root: python3 assets/portal_gate_gen.py
"""
import redesign_gen as r
from portal_frame_gen import prism
from portal_finish import rounded as bevel

SLATE=(53,72,84)
BRONZE=(177,127,70)
STONE=(131,126,113)
STEP=(106,105,98)
PAIR_COLOUR=(250,179,65)


def hull(points,colour):
    r.hull([tuple(v*32 for v in point) for point in points],colour)


def portal_gate():
    # A flat-topped octagonal dais; steps meet it from both approaches.
    plan=[(-1.50,-1.0),(1.50,-1.0),(1.625,-.875),(1.625,.875),
          (1.50,1.0),(-1.50,1.0),(-1.625,.875),(-1.625,-.875)]
    hull([(x,y,z) for z,scale in ((0,1),(.25,1)) for x,y in [(x*scale,y*scale) for x,y in plan]],STONE)
    for side in (-1,1):
        plan_step=[(-1.25,.875),(1.25,.875),(1.25,1.375),(1.125,1.50),(-1.125,1.50),(-1.25,1.375)]
        hull([(x,side*y,z) for z in (0,.125) for x,y in plan_step],STEP)
    for side in (-1,1):
        mirror=lambda poly:[(side*x,z) for x,z in poly]
        bevel(mirror([(.95,.25),(1.40,.25),(1.40,2.95),(1.18,3.02),(.95,2.80)]),.70,SLATE)
        # Bronze cuffs and an inset dark socket give each column a layered silhouette.
        bevel(mirror([(.89,.25),(1.47,.25),(1.42,.49),(.94,.49)]),.84,BRONZE,.035)
        bevel(mirror([(.93,2.43),(1.44,2.43),(1.44,2.62),(.93,2.62)]),.84,BRONZE,.035)
        # Small bronze corner brackets leave the human-sized corridor clear.
        bevel(mirror([(.68,2.75),(.95,2.40),(1.13,2.57),(.95,2.94)]),.82,BRONZE)
        for face in (-1,1):
            prism(mirror([(1.025,.61),(1.30,.76),(1.30,2.35),(1.025,2.49)]),
                  *sorted((face*.29,face*.375)),SLATE)
            prism(mirror([(1.09,.68),(1.23,.82),(1.23,2.32),(1.09,2.46)]),
                  *sorted((face*.36,face*.415)),PAIR_COLOUR)
    bevel([(-1.58,2.76),(1.58,2.76),(1.48,3.12),(1.12,3.20),
           (-1.12,3.20),(-1.48,3.12)],.84,BRONZE)
    # Raised centre crest, with a gently faceted curved profile.
    bevel([(-.72,3.08),(.72,3.08),(.55,3.22),(.30,3.25),(-.30,3.25),(-.55,3.22)],.63,SLATE,.045)
    for face in (-1,1):
        prism([(-.56,2.92),(.56,2.92),(.67,3.02),(-.67,3.02)],
              *sorted((face*.395,face*.455)),PAIR_COLOUR)


if __name__=='__main__':
    r.portal_gate=portal_gate
    r.build('portal_gate')
    from portal_finish import finish
    finish('portal_gate',{
        SLATE:([99,129,141],[32,49,65]), BRONZE:([230,184,113],[121,77,47]),
        STONE:([159,158,145],[99,103,103]), STEP:([135,139,132],[77,88,94]),
        PAIR_COLOUR:([255,237,155],[220,117,30]),
    })
