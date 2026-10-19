#!/usr/bin/env python3
"""Four original low-brush vehicles; metres -> 32 map units, ground/keel Z=0.
Run from repository root: python3 assets/transport_gen.py [asset_name ...]
"""
import json, math, sys
from pathlib import Path
import redesign_gen as r
from garden_props_gen import hull, rod
from retro_cars_gen import box
from portal_finish import finish
import asset_scale

WHITE=(222,230,224); BLUE=(46,115,174); GLASS=(39,78,97)
DARK=(42,48,54); METAL=(133,150,154); RED=(176,62,46)
CREAM=(228,211,172); WOOD=(147,101, 60); GREEN=(88,118,112)


def prism(poly,depth,z,c):
    hull([(x,y,z+dz) for x,y in poly for dz in (0,depth)],c)


def side(poly,y,width,c):
    hull([(x,y+dy,z) for x,z in poly for dy in (-width/2,width/2)],c)


def fuselage(stations,c):
    # Identical end rings guarantee contact between adjacent convex sections.
    ring=[(.7,1),(1,.65),(1,-.65),(.7,-1),(-.7,-1),(-1,-.65),(-1,.65),(-.7,1)]
    for a,b in zip(stations,stations[1:]):
        hull([(x,dy*w,z+dz*h) for x,z,w,h in (a,b) for dy,dz in ring],c)


def wheel(x,y,z,radius,width=.25):
    rod((x,y-width/2,z),(x,y+width/2,z),radius,DARK,10)
    sign=1 if y>0 else -1
    rod((x,y+sign*width/2,z),(x,y+sign*(width/2+.03),z),radius*.46,METAL,8)


def airliner():
    # 40 m fuselage, 36 m span. Nose points toward +X.
    fuselage([(-20,5,.22,.35),(-15,5,1.65,1.65),(-10,5,2,2),
              (12,5,2,2),(17,4.9,1.45,1.55),(19,4.65,.8,.9),(20,4.65,.2,.3)],WHITE)
    for sign in (-1,1):
        prism([(-6,sign*1.5),(5,sign*1.5),(-8,sign*18),(-10,sign*18)],.28,3.7,BLUE)
        prism([(-18,sign*.5),(-12,sign*.7),(-18,sign*7),(-20,sign*7)],.20,5.1,WHITE)
        # Engine nacelle and a separate dark intake, no tiny fan blades.
        y=sign*6.3
        box((-3,y-.14,2.6),(.7,y+.14,3.8),METAL)
        rod((-3.3,y,2.45),(1.2,y,2.45),1.03,WHITE,12)
        rod((1.2,y,2.45),(1.27,y,2.45),.77,DARK,12)
        rod((-3.42,y,2.45),(-3.3,y,2.45),.63,METAL,10)
        # One stripe per side and eight legible windows, avoiding hundreds of brushes.
        box((-10,sign*2-.025,4.65),(12,sign*2+.025,4.86),BLUE)
        for x in (-8.5,-6,-3.5,-1,1.5,4,6.5,9):
            box((x,sign*2-.035,5.3),(x+.78,sign*2+.035,5.85),GLASS)
        hull([(x,sign*(w+offset),z) for x,w in ((14,1.78),(16.8,1.472)) for z in (5.3,5.85) for offset in (.015,.05)],GLASS)
        for x in (-3,-1.6): wheel(x,sign*2.8,.58,.58,.4)
        rod((-2.3,sign*2.8,.7),(-2.3,sign*2.2,3.4),.12,METAL,6)
    side([(-19,5.7),(-12,5.7),(-16,11),(-18.3,11)],0,.3,BLUE)
    side([(-17.7,8.4),(-16.5,8.4),(-16.9,9.9)],-.165,.025,WHITE)
    rod((13,0,.6),(13,0,3.5),.13,METAL,6)
    wheel(13,0,.5,.5,.4)


def fighter_jet():
    fuselage([(-8,1.8,.55,.55),(-4,1.9,1,.8),(2,1.9,1,.7),
              (5.3,1.65,.55,.45),(8,1.5,.06,.08)],GREEN)
    for sign in (-1,1):
        prism([(-5,sign*.8),(2,sign*.8),(-4.5,sign*5),(-6,sign*5)],.16,1.55,GREEN)
        prism([(-7.8,sign*.45),(-5,sign*.55),(-7,sign*2.7),(-8,sign*2.7)],.12,2.2,METAL)
        side([(-7,2),(-4.5,2),(-5.4,4.5),(-6.2,4.5)],sign*.65,.15,GREEN)
        box((-.6,sign*1-.04,1.2),(1.3,sign*1+.04,1.7),DARK)
        rod((-7.8,sign*.45,1.7),(-8.25,sign*.45,1.7),.4,DARK,10)
        rod((-2,sign*1.45,.35),(-2,sign*1.1,1.6),.06,METAL,6)
        wheel(-2,sign*1.45,.35,.35,.22)
        prism([(-4,sign*3),(-3,sign*3),(-4.1,sign*4.3),(-4.6,sign*4.3)],.025,1.72,CREAM)
    hull([(-1.2,-.6,2.55),(-1.2,.6,2.55),(3.4,-.45,2.45),(3.4,.45,2.45),
          (-.2,-.4,3.35),(-.2,.4,3.35),(1.4,-.4,3.35),(1.4,.4,3.35)],GLASS)
    rod((.5,-.41,3.36),(.5,.41,3.36),.035,METAL,6)
    rod((4,0,.3),(4,0,1.3),.06,METAL,6)
    wheel(4,0,.3,.3,.22)


def locomotive_diesel():
    box((-8.8,-1.5,.95),(8.8,1.5,1.4),DARK)
    box((-9,-1.62,1.4),(9,1.62,1.64),CREAM)
    # Two six-wheel bogies, long hood, raised cab and short nose.
    for center in (-5.6,5.6):
        box((center-1.8,-1.12,.55),(center+1.8,1.12,1.05),DARK)
        for x in (center-1.35,center,center+1.35):
            for y in (-1.28,1.28): wheel(x,y,.54,.54,.28)
    hull([(x,y,z) for x in (-8,2.3) for y,z in [(-1.1,1.64),(1.1,1.64),(-1.1,3.5),(1.1,3.5),(-.85,3.8),(.85,3.8)]],RED)
    box((2.3,-1.4,1.64),(5.7,1.4,4.15),RED)
    hull([(x,y,z) for x in (2.15,5.85) for y,z in [(-1.52,4.15),(1.52,4.15),(-1.28,4.55),(1.28,4.55)]],CREAM)
    box((5.7,-1.05,1.64),(8.1,1.05,2.5),RED)
    for sign in (-1,1):
        box((2.8,sign*1.4-.025,3.05),(5.2,sign*1.4+.025,3.9),GLASS)
        box((-7.8,sign*1.1-.025,2.05),(2.2,sign*1.1+.025,2.27),CREAM)
        for x in (-6.7,-4.4,-2.1): box((x,sign*1.1-.035,2.6),(x+1.35,sign*1.1+.035,3.35),DARK)
        rod((-7.9,sign*1.48,2.65),(2.2,sign*1.48,2.65),.045,CREAM,6)
        for x in (-7.9,-2.85,2.2): rod((x,sign*1.48,1.6),(x,sign*1.48,2.65),.04,CREAM,6)
    for y in (-.8,.25):box((5.705,y,3.1),(5.74,y+.55,3.9),GLASS)
    rod((-3,0,3.8),(-3,0,4.05),.3,DARK,8)
    for x in (-9,9):
        box((x-.25,-.32,.75),(x+.25,.32,1.02),DARK)
    for y in (-.9,.9):rod((8.1,y,2.1),(8.18,y,2.1),.13,CREAM,8)


def steamship():
    def outline(x0,x1,y0,y1,radius):
        return [(x0+radius,y0),(x1-radius,y0),(x1,y0+radius),(x1,y1-radius),
                (x1-radius,y1),(x0+radius,y1),(x0,y1-radius),(x0,y0+radius)]
    def cabin(x0,x1,half_y,z0,z1,radius,color):
        prism(outline(x0,x1,-half_y,half_y,radius),z1-z0,z0,color)
    def window(x0,x1,y,z0,z1):
        # A clipped-corner pane remains one brush.
        poly=outline(x0,x1,z0,z1,.12)
        side(poly,y,.06,GLASS)
    bottom=[(-12,-1.7),(-10,-2.1),(8,-2.1),(12,0),(8,2.1),(-10,2.1),(-12,1.7)]
    chine=[(-13.6,-2.1),(-11.6,-3.2),(8.8,-3.2),(13.2,0),(8.8,3.2),(-11.6,3.2),(-13.6,2.1)]
    deck=[(-14,-2.3),(-12,-3.4),(9,-3.4),(14,0),(9,3.4),(-12,3.4),(-14,2.3)]
    hull([(x,y,0) for x,y in bottom]+[(x,y,1.4) for x,y in chine],(117,56,47))
    hull([(x,y,1.4) for x,y in chine]+[(x,y,2.5) for x,y in deck],BLUE)
    prism(deck,.12,2.5,CREAM)
    prism([(x*.995,y*.985) for x,y in deck],.11,2.62,WOOD)
    cabin(-10,8,2.25,2.73,4.85,.5,CREAM)
    cabin(-10.6,8.6,2.65,4.85,5.12,.55,WHITE)
    cabin(-6.5,6,1.85,5.12,7.12,.4,WHITE)
    cabin(-7,6.45,2.05,7.12,7.38,.4,WOOD)
    cabin(3.5,6,1.6,7.38,8.85,.35,CREAM)
    cabin(3.25,6.25,1.85,8.85,9.08,.4,BLUE)
    for sign in (-1,1):
        for x in (-8,-4,0,4):window(x,x+2.25,sign*2.25,3.45,4.35)
        for x in (-5,-1.5,2):window(x,x+2,sign*1.85,5.7,6.7)
        window(4,5.6,sign*1.6,7.9,8.6)
        rod((-12,sign*3.13,3.6),(8.5,sign*3.13,3.6),.055,WHITE,6)
        for x in (-12,-5,2,8.5):rod((x,sign*3.13,2.7),(x,sign*3.13,3.6),.045,WHITE,6)
        rod((8.5,sign*3.13,3.6),(13.1,0,3.6),.055,WHITE,6)
        rod((-12,sign*3.13,3.6),(-13.5,sign*2.1,3.6),.055,WHITE,6)
        y=sign*2.38
        hull([(x,y+dy,z) for x,dy,z in [(-7,-.28,5.15),(-7,.28,5.15),(-3,-.28,5.15),(-3,.28,5.15),
              (-7.5,0,5.65),(-6.7,-.48,5.65),(-6.7,.48,5.65),(-3.3,-.48,5.65),(-3.3,.48,5.65),(-2.5,0,5.65)]],RED)
        box((-6.4,y-.22,5.63),(-3.6,y+.22,5.68),CREAM)
        # A red rescue marker with a cream inset, readable with two brushes.
        rod((-.85,sign*2.23,3.7),(-.85,sign*2.36,3.7),.36,RED,12)
        rod((-.85,sign*2.35,3.7),(-.85,sign*2.375,3.7),.19,CREAM,12)
        # Pair of low ventilation cowls beside the stack.
        rod((-4.8,sign*.95,7.38),(-4.8,sign*.95,8.1),.17,CREAM,8)
        hull([(x,y,z) for x,cy,z,radius in [(-4.8,sign*.95,8,.17),(-4.3,sign*.95,8.25,.28)]
              for y in (cy-radius,cy+radius) for x in (x-radius,x+radius)],CREAM)
    rod((-13.5,-2.1,3.6),(-13.5,2.1,3.6),.055,WHITE,6)
    rod((13.1,0,2.73),(13.1,0,3.6),.045,WHITE,6)
    box((6,-1.1,7.9),(6.035,1.1,8.6),GLASS)
    rod((-1.8,0,7.38),(-1.85,0,7.65),.96,DARK,12)
    hull([(x+radius*math.cos(i*math.pi/6),radius*math.sin(i*math.pi/6),z)
          for x,z,radius in [(-1.85,7.65,.85),(-2.3,10.4,.76)] for i in range(12)],RED)
    hull([(x+radius*math.cos(i*math.pi/6),radius*math.sin(i*math.pi/6),z)
          for x,z,radius in [(-2.3,10.4,.86),(-2.36,10.8,.86)] for i in range(12)],DARK)
    rod((10,0,2.73),(10,0,8.7),.08,WOOD,6)
    rod((10,-1.4,7.4),(10,1.4,7.4),.055,WOOD,6)
    side([(10,8.6),(8.55,8.05),(10,7.8)],0,.05,RED)


BUILDERS={f.__name__:f for f in (airliner,fighter_jet,locomotive_diesel,steamship)}

def build(name):
    r.BRUSHES.clear();r.SOLIDS.clear();BUILDERS[name]()
    points=[p for pts,_ in r.SOLIDS for p in pts]
    dims=[max(p[i] for p in points)-min(p[i] for p in points) for i in range(3)]
    path=Path('assets/scale_profiles.json');data=json.loads(path.read_text())
    profile={'units':dims,'anchor':'32 units/metre; ground or keel Z=0'}
    data['assets'][name]=profile;path.write_text(json.dumps(data,indent=2)+'\n')
    asset_scale.PROFILES[name]=profile
    setattr(r,name,BUILDERS[name]);r.build(name)
    palettes={c:([min(255,round(v+(255-v)*.08)) for v in c],[round(v*.90) for v in c]) for c in (WHITE,BLUE,RED,GREEN,CREAM)}
    # Global Z limits keep adjoining body sections in the same continuous ramp.
    finish(name,palettes,world_gradients={c:((0,0,-1),(-dims[2],0)) for c in palettes})
    print(name, 'metres:',[round(v/32,3) for v in dims], 'brushes:',len(r.BRUSHES),flush=True)

if __name__=='__main__':
    for name in sys.argv[1:] or BUILDERS:build(name)
