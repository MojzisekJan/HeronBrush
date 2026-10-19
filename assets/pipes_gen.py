#!/usr/bin/env python3
"""Modular hollow pipe fittings. Z=12 axes, radius 8 outside / 6 inside."""
import itertools
import json
from pathlib import Path
import numpy as np
import redesign_gen as r
from civic_assets_gen import hull, box, vertical, build
from prefabs_gen import Editor

PIPE=(71,125,134); METAL=(143,166,170); DARK=(57,72,79); RED=(187,64,45)


def octagon(radius):
    a=radius-4
    return [(radius,a),(a,radius),(-a,radius),(-radius,a),(-radius,-a),(-a,-radius),(a,-radius),(radius,-a)]


def ring(center,side,radius):
    return [(center[0]+u*side[0],center[1]+u*side[1],12+z) for u,z in octagon(radius)]


def shell(a,b,side_a,side_b,outer=8,inner=6,color=PIPE,outer_b=None,inner_b=None):
    rings=[ring(a,side_a,outer),ring(a,side_a,inner),ring(b,side_b,outer_b or outer),ring(b,side_b,inner_b or inner)]
    for i in range(8):
        j=(i+1)%8
        hull([points[k] for points in rings for k in (i,j)],color)


def flange(a,b,side,radius=12,bore=6):
    shell(a,b,side,side,radius,bore,METAL)


def straight():
    shell((-30,0),(30,0),(0,1),(0,1))
    flange((-32,0),(-30,0),(0,1));flange((30,0),(32,0),(0,1))


def elbow():
    # Convex lattice arc: each chord has a distinct direction. Rounding
    # evenly spaced angles to this lattice made adjacent chords collinear.
    # Shared radial vectors keep all sector seams planar and on the 1/8 grid.
    arc=[(0,16),(2,16),(6,15),(10,13),(13,10),(15,6),(16,2),(16,0)]
    sections=[]
    for x,y in arc:
        side=(-x/16,y/16)
        center=(-34-34*side[0],34-34*side[1])
        sections.append((center,side))
    for (a,sa),(b,sb) in zip(sections,sections[1:]):shell(a,b,sa,sb)
    flange((-36,0),(-34,0),(0,1));flange((0,34),(0,36),(-1,0))



def broken():
    # A ruptured crown leaves the lower pipe wall continuous and both ports intact.
    shell((-30,0),(-6,0),(0,1),(0,1))
    shell((6,0),(30,0),(0,1),(0,1))
    for i in range(8):
        j=(i+1)%8
        spans=[(-6,6)] if i>=3 else ([(-6,-3),(4,6)] if i==1 else [(-6,-1),(2,6)])
        for a,b in spans:
            hull([points[k] for points in (ring((a,0),(0,1),8),ring((a,0),(0,1),6),
                                          ring((b,0),(0,1),8),ring((b,0),(0,1),6)) for k in (i,j)],PIPE)
    flange((-32,0),(-30,0),(0,1));flange((30,0),(32,0),(0,1))


def vertices(planes):
    points={}
    normals=np.array([p[:3] for p in planes]);distances=np.array([p[3] for p in planes])
    for ids in itertools.combinations(range(len(planes)),3):
        m=normals[list(ids)]
        if abs(np.linalg.det(m))<1e-8:continue
        point=np.linalg.solve(m,distances[list(ids)])
        if np.max(normals@point-distances)<1e-6:
            points[tuple(np.round(point,8))]=point
    result=list(points.values())
    if len(result)<4 or np.linalg.matrix_rank(np.array(result)[1:]-result[0],tol=1e-6)<3:return []
    return result


def difference(points,cutters,color):
    # Split a convex wall against the cutter's planes, retaining only the outside.
    hull(points,color)
    faces=r.BRUSHES.pop()[0];r.SOLIDS.pop()
    center=np.mean(points,axis=0);planes=[]
    for a,b,c in faces:
        n=np.cross(np.array(b)-a,np.array(c)-a);n/=np.linalg.norm(n)
        d=n@a
        if n@center>d:n=-n;d=-d
        planes.append((*n,d))
    if not vertices(planes+cutters):
        hull(points,color)
        return
    for cutter in cutters:
        outside=vertices(planes+[tuple(-v for v in cutter)])
        if outside:hull(outside,color)
        planes.append(cutter)
        if not vertices(planes):break


def bore_planes(axis,radius):
    # The chamfered octagon has axis-aligned sides and 45-degree corner planes.
    result=[]
    for a,b,d in [(1,0,radius),(-1,0,radius),(0,1,radius),(0,-1,radius),
                  (1,1,2*radius-4),(1,-1,2*radius-4),(-1,1,2*radius-4),(-1,-1,2*radius-4)]:
        n=(0,a,b) if axis=='x' else (a,0,b)
        result.append((*n,d+b*12))
    return result


def tee():
    start=len(r.SOLIDS)
    shell((-30,0),(30,0),(0,1),(0,1))
    main=list(r.SOLIDS[start:]);del r.SOLIDS[start:];del r.BRUSHES[start:]
    for points,color in main:difference(points,[(0,-1,0,0)]+bore_planes('y',6),color)
    start=len(r.SOLIDS)
    shell((0,0),(0,34),(1,0),(1,0))
    branch=list(r.SOLIDS[start:]);del r.SOLIDS[start:];del r.BRUSHES[start:]
    for points,color in branch:difference(points,bore_planes('x',8),color)
    flange((-32,0),(-30,0),(0,1));flange((30,0),(32,0),(0,1));flange((0,34),(0,36),(1,0))


def valve():
    straight()
    shell((-7,0),(7,0),(0,1),(0,1),10,8,DARK)
    vertical(0,0,20,28,4,METAL)
    vertical(0,0,28,38,1.5,METAL)
    # An octagonal wheel with an open centre, two spokes and a small metal hub.
    outer,inner=octagon(9),octagon(6)
    for i in range(8):
        hull([(x,y,z) for poly in (outer,inner) for x,y in (poly[i],poly[(i+1)%8]) for z in (37,40)],RED)
    box(-6,-.75,37.5,6,.75,39,RED);box(-.75,-6,37.5,.75,6,39,RED)
    vertical(0,0,37,40,2.5,METAL)


def reducer():
    # Smaller outlet has radius 5 and bore 4.5; the standard end remains radius 8/6.
    shell((-22,0),(22,0),(0,1),(0,1),8,6,PIPE,outer_b=5,inner_b=4.5)
    flange((-24,0),(-22,0),(0,1));flange((22,0),(24,0),(0,1),radius=7,bore=4.5)


if __name__=='__main__':
    editor=Editor()
    try:
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        import sys
        selected=set(sys.argv[1:])
        for name,geometry in [('pipe_straight',straight),('pipe_elbow',elbow),('pipe_tee',tee),('pipe_valve',valve),('pipe_reducer',reducer),('pipe_broken',broken)]:
            if selected and name not in selected:continue
            info=build(name,geometry,editor)
            editor.call('select',what='all')
            editor.call('set_surface_material',roughness=.4,metallic=.65,emission=0)
            editor.call('save',path=f'assets/{name}.map')
            editor.call('render_preview',path=f'assets/{name}.png',size=1000)
            data['assets'][name]=info
        profiles.write_text(json.dumps(data,indent=2)+'\n')
    finally:editor.close()
