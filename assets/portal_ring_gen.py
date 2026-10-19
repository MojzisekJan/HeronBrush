#!/usr/bin/env python3
"""Detailed compact ring: 31 convex brushes, 32 units/metre.
16 rounded ring sectors, three chevrons on each side with editable blue insets,
a low sill and two sculpted ring supports. Square 0.3125 m ring section; overall height about 2.76 m.
Run from repository root: python3 assets/portal_ring_gen.py
"""
import math
import redesign_gen as r
from portal_frame_gen import prism
from portal_finish import rounded

OUTER=1.40625
INNER=1.09375
CENTRE=OUTER*math.cos(math.pi/16)
METAL=(65,82,99)
LIGHT_METAL=(113,136,151)
TRIM=(155,162,161)
FOOT=(46,62,76)
PAIR_COLOUR=(88,165,245)


def point(radius,angle):
    return (radius*math.cos(angle),CENTRE+radius*math.sin(angle))


def portal_ring():
    for i in range(16):
        a=math.pi/16+i*math.pi/8;b=a+math.pi/8
        # A 0.3125 m square section, softened only at its four corners.
        points=[]
        for y,outer,inner in ((-.15625,OUTER-.0234375,INNER+.0234375),
                              (-.1328125,OUTER,INNER),(.1328125,OUTER,INNER),
                              (.15625,OUTER-.0234375,INNER+.0234375)):
            for radius,angle in ((inner,a),(outer,a),(outer,b),(inner,b)):
                x,z=point(radius,angle);points.append((x*32,y*32,z*32))
        r.hull(points,LIGHT_METAL if i%4==0 else METAL)
    rounded([(-.68,0),(.68,0),(.65,.12),(.52,.265),(-.52,.265),(-.65,.12)],.80,FOOT,.025)
    # The ring sits in two shaped sockets instead of terminating against a plain slab.
    for side in (-1,1):
        rounded([(side*x,z) for x,z in ((.44,.10),(.78,.10),(.87,.32),(.71,.47),(.54,.30))],.61,TRIM,.035)
    for angle in (math.pi/2,math.pi/4,3*math.pi/4):
        # Pointed chevron housings are distinct from the continuous ring silhouette.
        mount=[point(radius,a) for radius,a in ((1.12,angle),(1.235,angle-.075),
               (1.335,angle-.075),(1.325,angle),(1.335,angle+.075),(1.235,angle+.075))]
        # The housing must be convex: the outer centre is on the outer chord.
        mount[3]=point(1.335*math.cos(.075),angle)
        inset=[point(radius,a) for radius,a in ((1.16,angle),(1.255,angle-.042),
                                               (1.3,angle-.042),(1.3,angle+.042),(1.255,angle+.042))]
        for side in (-1,1):
            prism(mount,*sorted((side*.1328125,side*.203125)),TRIM)
            prism(inset,*sorted((side*.1953125,side*.21875)),PAIR_COLOUR)


def finish_ring():
    """Paint across the ring radius, continuous across each rounded rim.

    Generated faces use world-aligned, unit-scale Quake UVs. Solve the desired
    world-space ramp in those UVs instead of reusing one angle on every face.
    """
    import json
    import numpy as np
    from prefabs_gen import Editor
    ed=Editor()
    try:
        path='assets/portal_ring.map'
        ed.call('open_map',path=path)
        brushes=json.loads(ed.call('list_brushes'))
        for i,brush in enumerate(brushes):
            faces=json.loads(ed.call('describe_brush',brush=brush['id']))['faces']
            points=np.array([p for f in faces for p in f['vertices']])
            if i<16:
                a=math.pi/8+i*math.pi/8
                direction=np.array([math.cos(a),0,math.sin(a)])
                light,dark=np.array([139,163,177]),np.array([68,91,110])
            elif i==16:
                direction=np.array([0,0,-1.])
                light,dark=np.array([88,109,124]),np.array([43,60,75])
            else:
                centre=points.mean(axis=0)-np.array([0,0,CENTRE*32])
                direction=np.array([centre[0],0,centre[2]])
                direction/=np.linalg.norm(direction)
                blue=i>=19 and (i-19)%2==1
                light,dark=(np.array([167,228,255]),np.array([48,126,220])) if blue else (np.array([187,200,202]),np.array([113,137,150]))
            low,high=(points@direction).min(),(points@direction).max()
            def colour(value):
                t=np.clip((value-low)/max(high-low,1e-9),0,1)
                return np.rint(light*(1-t)+dark*t).astype(int).tolist()
            for face in faces:
                p=np.array(face['vertices']);normal=np.array(face['normal'])
                axis=int(np.argmax(np.abs(normal)))
                uv=p[:,[0,1]]*np.array([1,-1]) if axis==2 else p[:,[1 if axis==0 else 0,2]]*np.array([1,-1])
                values=p@direction
                args=dict(brush=brush['id'],face=face['face'])
                if np.ptp(values)<1e-6:
                    ed.call('set_face_color',**args,color=colour(values.mean()))
                else:
                    coeff=np.linalg.lstsq(uv-uv.mean(axis=0),values-values.mean(),rcond=None)[0]
                    angle=math.degrees(math.atan2(coeff[1],coeff[0]))
                    ed.call('set_face_color',**args,gradient={'from':colour(values.min()),'to':colour(values.max()),'angle':angle})
        ed.call('save',path=path)
        ed.call('open_map',path=path)
        ed.call('render_preview',path='assets/portal_ring.png',size=1200)
        ed.call('render_preview',path='assets/portal_ring_editor.png',size=800)
    finally:ed.close()


if __name__=='__main__':
    r.portal_ring=portal_ring;r.build('portal_ring')
    finish_ring()
