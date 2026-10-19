#!/usr/bin/env python3
"""Industrial gearbox with two counter-rotating twelve-tooth gears."""
import json
import math
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import hull, box, wheel, vertical
from fan_gen import brush_text
from prefabs_gen import Editor
from asset_skip import hidden_faces, FACE
from portal_finish import paint_world_gradient

GEAR_X=31.5
GEAR_Z=73

TEAL=(62,112,112); DARK=(48,62,67); BRASS=(204,155,74); STEEL=(165,186,188)

def profile(points,y0,y1,color):
    hull([(x,y,z) for x,z in points for y in (y0,y1)],color)

def gear(cx,phase,color):
    def point(radius,degrees):
        a=math.radians(degrees+phase)
        return (cx+radius*math.cos(a),GEAR_Z+radius*math.sin(a))
    # Six convex rim sectors retain twelve outer facets and leave a real central hole.
    for i in range(6):
        a=i*60
        profile([point(25.9,a-30),point(25.9,a),point(25.9,a+30),
                 point(19,a+30),point(19,a-30)],14,22,color)
    for i in range(12):
        a=i*30
        profile([point(24.5,a-9),point(35,a-4),point(35,a+4),point(24.5,a+9)],14,22,color)
    for i in range(3):
        a=math.radians(phase+i*120);c,t=math.cos(a),math.sin(a)
        profile([(cx+radius*c-width*t,GEAR_Z+radius*t+width*c)
                 for radius,width in [(3,-2.5),(21,-3),(21,3),(3,2.5)]],14,22,color)
    wheel(cx,14,25,GEAR_Z,6,DARK)
    box(cx-1,17,GEAR_Z-1,cx+1,19,GEAR_Z+1,(255,0,255))

def geometry():
    r.BRUSHES.clear();r.SOLIDS.clear()
    profile([(-70,0),(70,0),(70,5),(64,10),(-64,10),(-70,5)],-40,24,DARK)
    # Low sloping chassis, with separate bearing towers instead of a tall solid box.
    profile([(-60,10),(60,10),(58,27),(44,40),(-45,40),(-58,28)],-32,8,TEAL)
    for x in (-GEAR_X,GEAR_X):
        profile([(x-16,34),(x+16,34),(x+11,73),(x+8,80),(x-8,80),(x-11,73)],-8,7,TEAL)
        wheel(x,7,10,GEAR_Z,13,DARK)
        wheel(x,10,14,GEAR_Z,6,STEEL)
    def motor(x0,x1,radius,color):
        hull([(x,-20+radius*math.cos(i*math.pi/6),45+radius*math.sin(i*math.pi/6))
              for x in (x0,x1) for i in range(12)],color)
    motor(-55,5,18,TEAL)
    motor(-58,-55,19,DARK)
    motor(5,8,19,DARK)
    for x in (-45,-28,-11):motor(x,x+2,19,STEEL)
    # A lower rear control housing balances the motor without hiding the gear silhouettes.
    profile([(20,34),(54,34),(54,56),(49,62),(25,62),(20,57)],-30,-8,TEAL)
    box(25,-29,62,49,-9,64,DARK)
    motor(8,23,3,STEEL)
    vertical(43,-18,64,67,6,DARK)
    vertical(43,-18,67,76,4,(228,164,49))
    for z in (17,23,29):box(-48,8,z,-16,8.5,z+2,DARK)
    profile([(24,17),(48,17),(49,29),(46,33),(26,33),(23,29)],8,11,STEEL)
    wheel(31,11,13,25,3,(182,55,41))
    wheel(42,11,13,25,3,(100,187,124))
    fixed=len(r.SOLIDS)
    gear(-GEAR_X,0,BRASS);second=len(r.SOLIDS)
    gear(GEAR_X,15,STEEL)
    return fixed,second

def convex(points):
    pts=sorted(set(points))
    def cross(o,a,b):return (a[0]-o[0])*(b[1]-o[1])-(a[1]-o[1])*(b[0]-o[0])
    lower=[];upper=[]
    for p in pts:
        while len(lower)>1 and cross(lower[-2],lower[-1],p)<=0:lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper)>1 and cross(upper[-2],upper[-1],p)<=0:upper.pop()
        upper.append(p)
    return lower[:-1]+upper[:-1]

def check_clearance(fixed,second):
    # Check all convex tooth/body pairs over one full tooth period after grid rounding.
    profiles=[[convex([(x,z) for x,_,z in points]) for points,_ in r.SOLIDS[a:b-2]]
              for a,b in [(fixed,second),(second,len(r.SOLIDS))]]
    def rotate(poly,cx,angle):
        c,s=math.cos(angle),math.sin(angle)
        return [(cx+(x-cx)*c-(z-GEAR_Z)*s,GEAR_Z+(x-cx)*s+(z-GEAR_Z)*c) for x,z in poly]
    def intersects(a,b):
        for poly in (a,b):
            for p,q in zip(poly,poly[1:]+poly[:1]):
                nx,ny=q[1]-p[1],p[0]-q[0]
                pa=[x*nx+y*ny for x,y in a];pb=[x*nx+y*ny for x,y in b]
                if min(max(pa),max(pb))-max(min(pa),min(pb))<=1e-7:return False
        return True
    for step in range(121):
        angle=math.radians(step*.25)
        left=[rotate(p,-GEAR_X,-angle) for p in profiles[0]]
        right=[rotate(p,GEAR_X,angle) for p in profiles[1]]
        assert not any(intersects(a,b) for a in left for b in right),f'teeth intersect at {step*.25} degrees'
    print('Gear clearance: 121 phases checked without intersection',flush=True)

def build():
    fixed,second=geometry();check_clearance(fixed,second)
    path=Path('assets/gear_machine.map')
    lines=['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
    lines.extend(brush_text(i,'__TB_empty') for i in range(fixed));lines.append('}')
    for start,end,cx,speed in [(fixed,second,-GEAR_X,30),(second,len(r.SOLIDS),GEAR_X,-30)]:
        lines+=['{','"classname" "func_rotate"',f'"origin" "{cx} 18 {GEAR_Z}"','"axis" "0 1 0"',f'"speed" "{speed}"','"start_on" "1"']
        lines.extend(brush_text(i,'origin' if i==end-1 else '__TB_empty') for i in range(start,end));lines.append('}')
    path.write_text('\n'.join(lines)+'\n')
    editor=Editor()
    try:
        editor.call('open_map',path=str(path))
        saved=editor._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
        assert not saved.get('result',{}).get('isError'),saved
        from asset_grid import snap_file
        snap_file(editor,path)
        infos=json.loads(editor.call('list_brushes'));assert len(infos)==len(r.SOLIDS)
        palettes={TEAL:((78,132,129),(47,89,94)),BRASS:((225,186,109),(174,120,52)),STEEL:((191,207,207),(139,160,168))}
        for index,(brush,(_,color)) in enumerate(zip(infos,r.SOLIDS)):
            if color in palettes:
                light,dark=palettes[color]
                limits=(-112,0) if index<fixed else (-108,-38)
                paint_world_gradient(editor,brush,light,dark,(0,0,-1),limits)
        saved=editor._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
        assert not saved.get('result',{}).get('isError'),saved
        brushes=[json.loads(editor.call('describe_brush',brush=b['id'])) for b in infos]
        hidden=set()
        for a,b in [(0,fixed),(fixed,second-1),(second,len(brushes)-1)]:hidden.update(hidden_faces(brushes[a:b]))
        lines=path.read_text().splitlines(keepends=True);bid=None;face=0
        for i,line in enumerate(lines):
            if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16));face=0
            match=FACE.match(line.rstrip('\n'))
            if match:
                if (bid,face) in hidden:lines[i]=match[1]+'skip'+match[3]+'\n'
                face+=1
        path.write_text(''.join(lines));editor.call('open_map',path=str(path))
        editor.call('render_preview',path='assets/gear_machine.png',size=1000)
        profile_path=Path('assets/scale_profiles.json');data=json.loads(profile_path.read_text())
        points=[p for solid,_ in r.SOLIDS for p in solid]
        dimensions=[max(p[i] for p in points)-min(p[i] for p in points) for i in range(3)]
        data['assets']['gear_machine']={'units':dimensions,'anchor':'32 units/metre; ground Z=0; two Y-axis rotors at +/-30 degrees/sec'}
        profile_path.write_text(json.dumps(data,indent=2)+'\n')
        print(f'Gear machine: {len(infos)-2} visible brushes + 2 origins; {len(hidden)} skip faces',flush=True)
    finally:editor.close()

if __name__=='__main__':build()
