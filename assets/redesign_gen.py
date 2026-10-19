#!/usr/bin/env python3
"""Build five brush prefabs. Run from repo root: python3 assets/redesign_gen.py.
Requires numpy and Pillow for depth-buffered thumbnails; geometry is dependency-free.
"""
import itertools
import json
import math
from pathlib import Path
from prefabs_gen import Editor
BRUSHES=[]
SOLIDS=[]
def add(a, b):
    return tuple(x + y for x, y in zip(a, b))


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def mul(a, s):
    return tuple(x * s for x in a)


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])


def normal(a):
    return mul(a, 1 / math.sqrt(dot(a, a)))


def hull(points, color):
    """Supporting planes of a small convex point cloud, wound for Quake maps."""
    SOLIDS.append((points, color))
    planes = {}
    for a, b, c in itertools.combinations(points, 3):
        n = cross(sub(b, a), sub(c, a))
        if dot(n, n) < 1e-10:
            continue
        n = normal(n)
        d = dot(n, a)
        distances = [dot(n, p) - d for p in points]
        if max(distances) > 1e-6:
            if min(distances) < -1e-6:
                continue
            n, d = mul(n, -1), -d
        key = tuple(round(v, 5) for v in (*n, d))
        if key in planes:
            continue
        # Quake's map planes use the opposite winding to a standard cross product.
        u = normal(cross(n, (0, 0, 1) if abs(n[2]) < .9 else (0, 1, 0)))
        v = cross(n, u)
        anchor = mul(n, d)
        planes[key] = (anchor, add(anchor, mul(v, 64)), add(anchor, mul(u, 64)))
    assert len(planes) >= 4, 'flat brush'
    BRUSHES.append((list(planes.values()), color))



WOOD=(151,95,52); TRIM=(101,62,37); GOLD=(207,164,81)
DARK=(49,62,66); GLASS=(79,148,168); CREAM=(226,213,180)
RED=(162,48,42); TEAL=(39,112,111); STONE=(176,180,167)

def box(x0,y0,z0,x1,y1,z1,c):
    hull(list(itertools.product((x0,x1),(y0,y1),(z0,z1))),c)

def prism(poly,y0,y1,c):
    hull([(x,y,z) for x,z in poly for y in (y0,y1)],c)

def loft(stations,c,sides=12):
    for i,(a,b) in enumerate(zip(stations,stations[1:])):
        pts=[]
        for x,y,z,ry,rz in (a,b):
            pts += [(x,y+ry*math.cos(j*2*math.pi/sides),z+rz*math.sin(j*2*math.pi/sides)) for j in range(sides)]
        hull(pts,c if isinstance(c[0],int) else c[i%len(c)])

def rod(a,b,r,c,sides=8):
    n=normal(sub(b,a));u=normal(cross(n,(0,0,1) if abs(n[2])<.9 else (0,1,0)));v=cross(n,u)
    hull([add(p,add(mul(u,r*math.cos(i*2*math.pi/sides)),mul(v,r*math.sin(i*2*math.pi/sides)))) for p in (a,b) for i in range(sides)],c)

def bridge():
    # A contiguous elliptical vault. Spandrels fill exactly to a level roadway.
    n=18;spring=20;rx=138;rz=68;ox=156;oz=86;deck=114
    for x0,x1 in ((-205,-156),(156,205)):
        box(x0,-48,0,x1,48,deck,STONE)
    for i in range(n):
        a=i*math.pi/n;b=(i+1)*math.pi/n
        inner=[(rx*math.cos(t),spring+rz*math.sin(t)) for t in (a,b)]
        outer=[(ox*math.cos(t),spring+oz*math.sin(t)) for t in (a,b)]
        prism(inner+outer,-48,48,[(191,193,177),(182,184,169)][i%2])
        # Full-width mass above the arch, with no unsupported stair steps.
        prism([*outer,(outer[0][0],deck),(outer[1][0],deck)],-48,48,STONE)
    for i in range(20):
        x=-205+i*20.5
        box(x,-51,114,x+20.5,51,121,(196,196,179))
    for y in (-51,41):
        for i in range(10):
            x=-205+i*41
            box(x,y,121,x+41,y+10,145,(170,175,163))
            box(x,y-2,145,x+41,y+12,151,(205,203,183))
    for x in (-205,191):
        for y in (-54,40):
            box(x,y,121,x+14,y+14,155,(187,187,169))
            box(x-2,y-2,155,x+16,y+16,160,CREAM)

def bookshelf():
    box(-64,-17,0,64,19,7,TRIM)
    box(-59,-15,7,-52,17,174,WOOD);box(52,-15,7,59,17,174,WOOD)
    # Back divided at shelves so the editable geometry remains simple.
    for z in (7,61,115): box(-52,-16,z,52,-13,z+54,(123,78,45))
    for z in (7,61,115,169): box(-59,-16,z,59,18,z+5,WOOD)
    box(-64,-20,174,64,21,181,TRIM);box(-60,-17,181,60,18,185,WOOD)
    colors=[TEAL,RED,(90,102,130),(176,133,61),(107,79,109)]
    for row,z in enumerate((12,66,120)):
        x=-47
        for i in range(6 if row!=1 else 5):
            w=[10,9,12,8,11,9][i];h=[35,39,32,37,34,40][(i+row)%6]
            c=colors[(i+row)%5]
            # Spine and covers enclose a cream page block, no intersecting books.
            box(x,-10,z,x+1,13,z+h,c)
            box(x+w-1,-10,z,x+w,13,z+h,c)
            box(x+1,-9,z+1,x+w-1,11,z+h-1,CREAM)
            box(x,12,z,x+w,14,z+h,c)
            for zz in (z+5,z+h-6): box(x+1,14,zz,x+w-1,14.6,zz+1,GOLD)
            x+=w+2
        if row==1:
            for j in range(3):
                zz=z+j*6
                box(23,-7,zz,47,12,zz+5,colors[j])
                box(24,12,zz+1,46,12.6,zz+4,CREAM)

def airship():
    # The envelope is a smoothly tapering series of convex sections.
    xs=[-180,-160,-128,-88,-42,10,62,108,143,166,180]
    radii=[2,24,42,53,59,60,56,46,33,18,2]
    loft([(x,0,142,r,r*.87) for x,r in zip(xs,radii)],[(221,204,163),(229,214,177)],16)
    # Narrow teal gores follow the curvature, without bulky hoops.
    for theta in (.22,math.pi-.22,math.pi+.22,2*math.pi-.22):
        for k in range(1,len(xs)-2):
            pts=[]
            for idx in (k,k+1):
                for a in (theta-.065,theta+.065):
                    for lift in (.15,.6):
                        r=radii[idx]+lift
                        pts.append((xs[idx],r*math.cos(a),142+r*.87*math.sin(a)))
            hull(pts,TEAL)
    # Swept stabilizers, shaped as thin convex solids.
    for s in (-1,1):
        hull([(x,s*y,142+z) for x,y in [(-165,8),(-108,32),(-156,79),(-180,67)] for z in (-2,2)],TEAL)
        hull([(x,y,142+s*z) for x,z in [(-165,6),(-108,23),(-156,69),(-180,56)] for y in (-2,2)],TEAL)
    loft([(-62,0,49,14,11),(-42,0,48,22,16),(56,0,48,22,16),(80,0,52,12,10)],TEAL,8)
    box(-43,-20,63,57,20,80,CREAM)
    box(-48,-24,80,63,24,84,TEAL)
    for x in (-32,-8,16,40):
        for y in (-20.5,20): box(x,y,67,x+15,y+.5,77,GLASS)
    for x in (-36,46):
        for s in (-1,1): rod((x,s*17,82),(x,s*31,98),2,DARK)
    for s in (-1,1):
        rod((9,s*22,46),(9,s*42,46),3,DARK)
        loft([(-6,s*43,46,6,6),(22,s*43,46,6,6),(28,s*43,46,2,2)],DARK,8)
        rod((29,s*43,32),(29,s*43,60),2,WOOD)

def plane():
    loft([(-158,0,74,3,4),(-128,0,74,7,9),(-62,0,75,15,17),(30,0,76,22,23),(86,0,76,21,22),(116,0,76,15,17)],(214,179,74),12)
    loft([(116,0,76,17,17),(131,0,76,8,8),(136,0,76,1,1)],RED,12)
    # Airfoil wings have a thick leading edge and tapered, swept tips.
    for s in (-1,1):
        hull([(x,s*y,z) for x,y,z in [(62,12,64),(-12,12,63),(27,150,65),(-8,158,65),(61,12,71),(3,12,70),(26,150,68),(-6,157,67)]],RED)
        hull([(x,s*y,z) for x,y,z in [(-114,4,76),(-153,4,76),(-139,58,78),(-159,58,78),(-114,4,80),(-153,4,80),(-139,58,80),(-159,58,80)]],RED)
    prism([(-156,78),(-116,80),(-143,125),(-156,124)],-2,2,RED)
    hull([(-26,-13,92),(-26,13,92),(57,-15,92),(57,15,92),(-13,-10,115),(-13,10,115),(31,-10,115),(31,10,115)],GLASS)
    for y in (-1,1): rod((-14,y*10,115),(-26,y*13,93),1.4,DARK)
    rod((-13,-10,115),(-13,10,115),1.5,DARK)
    rod((31,-10,115),(31,10,115),1.5,DARK)
    rod((138,0,29),(138,0,123),2.7,DARK)
    loft([(135,0,76,5,5),(143,0,76,3,3)],GOLD,8)
    for s in (-1,1):
        rod((30,s*15,57),(20,s*42,22),3,DARK)
        rod((20,s*38,17),(20,s*48,17),12,DARK,12)
        rod((20,s*48,17),(20,s*49,17),5,GOLD,10)
    rod((-137,0,69),(-143,0,48),2,DARK)

def tram():
    outline=[(-154,-26),(-141,-37),(141,-37),(154,-26),(154,26),(141,37),(-141,37),(-154,26)]
    for z0,z1,col in [(20,29,DARK),(29,56,RED),(56,60,GOLD),(60,103,CREAM)]:
        hull([(x,y,z) for x,y in outline for z in (z0,z1)],col)
    # Solid cambered roof, not the arch primitive's open tunnel.
    profile=[(-40,104),(-38,111),(-24,118),(24,118),(38,111),(40,104)]
    hull([(x,y,z) for x in (-156,156) for y,z in profile],DARK)
    for y in (-37.4,37.1):
        for x in (-126,-91,-56,-21,14,49):
            box(x,y,67,x+27,y+.3,97,GLASS)
        # A two-leaf door fits between the window bay and the rounded end.
        box(88,y,30,126,y+.35,100,(104,41,36))
        for x in (91,109):
            box(x,y-.1,65,x+14,y+.5,96,GLASS)
            box(x,y-.1,34,x+14,y+.5,59,RED)
        box(106,y-.15,31,107,y+.6,100,GOLD)
    for x in (-154.4,154.1):
        box(x,-23,68,x+.3,23,98,GLASS)
        for y in (-20,14): box(x-.1,y,39,x+.4,y+6,46,CREAM)
        box(x-.5,-20,59,x+.5,20,62,GOLD)
    for cx in (-92,92):
        box(cx-28,-28,10,cx+28,28,24,DARK)
        for dx in (-17,17):
            for s in (-1,1): rod((cx+dx,s*27,13),(cx+dx,s*34,13),11,DARK,12)
    box(-43,-20,118,43,20,122,(74,81,77))
    for y in (-11,11):
        for a,b in [((0,y,122),(-28,y,143)),((-28,y,143),(0,y,164)),((0,y,122),(28,y,143)),((28,y,143),(0,y,164))]: rod(a,b,1.8,DARK,6)
    box(-4,-34,164,4,34,167,DARK)


def render(path,size=800):
    """Rasterize the same convex brush faces with a per-pixel depth buffer."""
    import numpy as np
    from PIL import Image
    eye=np.array([.60,.80,.57]);eye/=np.linalg.norm(eye)
    right=np.cross([0,0,1],eye);right/=np.linalg.norm(right);up=np.cross(eye,right)
    allp=np.array([p for pts,c in SOLIDS for p in pts]);xy=np.stack([allp@right,allp@up],axis=1)
    scale=(size-70)/max(np.ptp(xy,axis=0));mid=(xy.min(axis=0)+xy.max(axis=0))/2
    depth=np.full((size,size),-np.inf);rgb=np.zeros((size,size,4),dtype=np.uint8)
    light=np.array([-.35,.55,1.]);light/=np.linalg.norm(light)
    for (pts,c),(planes,_) in zip(SOLIDS,BRUSHES):
        pts=np.array(pts)
        for plane in planes:
            a,b,d=map(np.array,plane);n=np.cross(d-a,b-a);n/=np.linalg.norm(n)
            if n@eye<=0: continue
            face=pts[np.abs((pts-a)@n)<1e-4]
            if len(face)<3: continue
            center=face.mean(axis=0);u=face[0]-center;u/=np.linalg.norm(u);v=np.cross(n,u)
            face=face[np.argsort(np.arctan2((face-center)@v,(face-center)@u))]
            screen=np.stack([(face@right-mid[0])*scale+size/2,size/2-(face@up-mid[1])*scale],axis=1)
            zz=face@eye
            shade=.55+.45*max(0,n@light);col=np.array(c)*shade
            for i in range(1,len(face)-1):
                tri=screen[[0,i,i+1]];tz=zz[[0,i,i+1]]
                lo=np.maximum(np.floor(tri.min(axis=0)).astype(int),0);hi=np.minimum(np.ceil(tri.max(axis=0)).astype(int),size-1)
                if np.any(hi<lo): continue
                xx,yy=np.meshgrid(np.arange(lo[0],hi[0]+1)+.5,np.arange(lo[1],hi[1]+1)+.5)
                x0,y0=tri[0];x1,y1=tri[1];x2,y2=tri[2];den=(y1-y2)*(x0-x2)+(x2-x1)*(y0-y2)
                if abs(den)<1e-8: continue
                w0=((y1-y2)*(xx-x2)+(x2-x1)*(yy-y2))/den;w1=((y2-y0)*(xx-x2)+(x0-x2)*(yy-y2))/den;w2=1-w0-w1
                z=w0*tz[0]+w1*tz[1]+w2*tz[2];sl=np.s_[lo[1]:hi[1]+1,lo[0]:hi[0]+1]
                mask=(w0>=-1e-6)&(w1>=-1e-6)&(w2>=-1e-6)&(z>depth[sl])
                depth[sl][mask]=z[mask];rgb[sl][mask]=[*col.astype(int),255]
    Image.fromarray(rgb).save(path)

def build(name):
    BRUSHES.clear();SOLIDS.clear()
    import sys
    import asset_polish
    asset_polish.generate(name,globals()[name],sys.modules[__name__])
    from asset_scale import normalize_geometry
    normalize_geometry(name, SOLIDS, BRUSHES)
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    for planes,c in BRUSHES:
        lines+=['// heron_brush color '+' '.join(map(str,c)),'{']
        for plane in planes:
            lines.append(' '.join('( '+' '.join(f'{v:.12f}' for v in p)+' )' for p in plane)+' __TB_empty 0 0 0 1 1')
        lines.append('}')
    lines.append('}');path=Path(f'assets/{name}.map');path.write_text('\n'.join(lines)+'\n')
    ed=Editor()
    try:
        ed.call('open_map',path=str(path));count=len(json.loads(ed.call('list_brushes')))
        assert count==len(BRUSHES),(name,count,len(BRUSHES))
        ed.call('save',path=str(path));ed.call('open_map',path=str(path))
        assert len(json.loads(ed.call('list_brushes')))==count
        ed.call('render_preview',path=f'assets/{name}_editor.png',size=800)
    finally: ed.close()
    render(f'assets/{name}.png',1200)
    asset_polish.finish(name,sys.modules[__name__])
    print(f'{name}: {count} brushes validated and rendered',flush=True)

if __name__=='__main__':
    import sys
    for name in sys.argv[1:] or ['bridge','bookshelf','airship','plane','tram']: build(name)
