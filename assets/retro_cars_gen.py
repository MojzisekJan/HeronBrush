#!/usr/bin/env python3
"""Two original retro cars, editable brushes at 32 units/metre.
Run from repository root: python3 assets/retro_cars_gen.py
"""
import redesign_gen as r

RUBBER=(36,42,43);CHROME=(186,195,187);GLASS=(91,151,165)
CREAM=(228,218,180);LIGHT=(252,230,160);RED=(155,43,32)


def box(a,b,color):r.box(*(v*32 for v in a),*(v*32 for v in b),color)
def hull(points,color):r.hull([tuple(v*32 for v in p) for p in points],color)
def rod(a,b,radius,color,sides=8):r.rod(tuple(v*32 for v in a),tuple(v*32 for v in b),radius*32,color,sides)


def wheels(axles,half_width,radius=.29):
    for x in axles:
        for s in (-1,1):
            rod((x,s*(half_width-.16),radius),(x,s*half_width,radius),radius,RUBBER,12)
            rod((x,s*half_width,radius),(x,s*(half_width+.012),radius),radius*.56,CHROME,10)


def cabin(x0,x1,roof0,roof1,width,roofwidth,waist,top,color):
    hull([(x,y,waist) for x in (x0,x1) for y in (-width,width)]+
         [(x,y,top) for x in (roof0,roof1) for y in (-roofwidth,roofwidth)],GLASS)
    hull([(x,y,z) for x in (roof0-.025,roof1+.025) for y in (-roofwidth-.025,roofwidth+.025) for z in (top,top+.055)],CREAM)
    for s in (-1,1):
        rod((x0,s*width,waist),(roof0,s*roofwidth,top),.032,color,4)
        rod((x1,s*width,waist),(roof1,s*roofwidth,top),.032,color,4)
        rod((-.04,s*width,waist),(-.04,s*roofwidth,top),.035,color,4)


def car_retro_compact():
    mint=(103,157,128)
    outline=[(-1.69,-.53),(-1.53,-.71),(1.43,-.71),(1.68,-.52),(1.68,.52),(1.43,.71),(-1.53,.71),(-1.69,.53)]
    hull([(x,y,.34) for x,y in outline]+[(x*.99,y*.99,.85) for x,y in outline],mint)
    box((-1.2,-.46,.23),(1.18,.46,.35),RUBBER)
    wheels((-1.07,1.03),.79,.285)
    cabin(-1.32,.66,-.99,.34,.66,.56,.84,1.445,mint)
    # Short sloping bonnet, round lamps and bright simple bumpers.
    hull([(.66,-.64,.84),(.66,.64,.84),(1.55,-.53,.78),(1.55,.53,.78),
          (.66,-.64,.88),(.66,.64,.88),(1.55,-.53,.82),(1.55,.53,.82)],mint)
    box((1.68,-.57,.34),(1.775,.57,.43),CHROME)
    box((-1.775,-.57,.34),(-1.68,.57,.43),CHROME)
    box((1.681,-.24,.49),(1.7,.24,.62),RUBBER)
    for y in (-.43,.43):rod((1.66,y,.69),(1.715,y,.69),.108,LIGHT,10)
    for y in (-.51,.41):box((-1.704,y,.53),(-1.686,y+.1,.72),RED)
    for s in (-1,1):box((-.24,s*.706-.013,.72),(-.08,s*.706+.013,.753),CHROME)
    box((-1.787,-.14,.355),(-1.775,.14,.415),CREAM)


def car_retro_sedan():
    ochre=(172,91,51)
    outline=[(-2.36,-.63),(-2.2,-.84),(2.23,-.84),(2.36,-.64),(2.36,.64),(2.23,.84),(-2.2,.84),(-2.36,.63)]
    hull([(x,y,.33) for x,y in outline]+[(x,y,.88) for x,y in outline],ochre)
    wheels((-1.48,1.43),.90,.30)
    cabin(-1.38,1.07,-.91,.65,.77,.64,.875,1.425,ochre)
    box((1.075,-.74,.878),(2.25,.74,.905),ochre)
    box((-2.25,-.73,.878),(-1.38,.73,.905),ochre)
    box((2.36,-.75,.34),(2.45,.75,.43),CHROME)
    box((-2.45,-.75,.34),(-2.36,.75,.43),CHROME)
    box((2.361,-.31,.53),(2.382,.31,.73),RUBBER)
    box((2.382,-.29,.62),(2.389,.29,.64),CHROME)
    for y in (-.60,-.405,.405,.60):rod((2.35,y,.65),(2.395,y,.65),.085,LIGHT,10)
    for y in (-.64,.36):box((-2.384,y,.54),(-2.362,y+.28,.69),RED)
    for s in (-1,1):
        box((-2.18,s*.839-.012,.75),(2.20,s*.839+.012,.777),CHROME)
        for x in (-1.04,.1):box((x,s*.847-.012,.80),(x+.14,s*.847+.012,.826),CHROME)


CARS={'car_retro_compact':car_retro_compact,'car_retro_sedan':car_retro_sedan}
if __name__=='__main__':
    for name,builder in CARS.items():
        setattr(r,name,builder);r.build(name)
