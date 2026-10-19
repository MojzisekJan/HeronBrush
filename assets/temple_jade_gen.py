#!/usr/bin/env python3
"""Reference-inspired tiered temple. Every reconstructed vertex uses grid 1."""
import json,sys,itertools
from pathlib import Path
import numpy as np
import redesign_gen as r
from fan_gen import brush_text
import prefabs_gen
from prefabs_gen import Editor
from asset_skip import hidden_faces,opaque,FACE
ROOT=Path(__file__).resolve().parent
RED=(184,38,32);SCARLET=(223,57,35);GOLD=(240,190,65);CREAM=(238,219,158)
JADE=(35,120,108);DARK=(23,74,74);STONE=(99,133,109)

def hull(points,color):
    points=[tuple(int(round(v)) for v in p) for p in points]
    r.hull(points,color)
def box(x0,y0,z0,x1,y1,z1,color):hull(list(itertools.product((x0,x1),(y0,y1),(z0,z1))),color)
def prism(poly,y0,y1,color):hull([(x,y,z) for x,z in poly for y in (y0,y1)],color)
def roof(xs,lifts,ys,heights,base):
    for i,(a,b) in enumerate(zip(xs,xs[1:])):
        for side in (-1,1):
            for j,(c,d) in enumerate(zip(ys,ys[1:])):
                points=[(x,side*y,base+lift+height+t) for x,lift in [(a,lifts[i]),(b,lifts[i+1])] for y,height in [(c,heights[j]),(d,heights[j+1])] for t in (0,4)]
                hull(points,JADE if j else DARK)
            # Red underside rim, sharing the roof plane at the outermost span.
            c,d=ys[-2:];hc,hd=heights[-2:]
            hull([(x,side*y,base+lift+height+t) for x,lift in [(a,lifts[i]),(b,lifts[i+1])] for y,height in [(c,hc),(d,hd)] for t in (-5,0)],SCARLET)
        # Ridge follows the upward sweep in X.
        cut=92 if base==160 else (48 if base==220 else 0)
        spans=[(a,b)]
        if cut and a<cut and b>-cut:spans=[(a,min(b,-cut)),(max(a,cut),b)]
        for left,right in spans:
            if left>=right:continue
            hull([(x,y,base+int(lifts[i]+(lifts[i+1]-lifts[i])*(x-a)/(b-a))+heights[0]+4+t) for x in (left,right) for y in (-2,2) for t in (0,3)],GOLD)

def geometry():
    # Broad plinth with four solid, non-overlapping stair treads.
    box(-216,-112,0,216,112,32,STONE)
    for i in range(4):box(-136,-176+i*16,0,136,-160+i*16,(i+1)*8,SCARLET)
    # Main body follows the bottom surface of the lower roof exactly.
    xs=[-192,-96,96,192];fx=[10,0,0,10]
    ys=[-96,-80,0,80,96];fy=[4,6,26,6,4]
    for i in range(3):
        for j in range(4):
            hull([(x,y,z) for x,lift in [(xs[i],fx[i]),(xs[i+1],fx[i+1])] for y,rise in [(ys[j],fy[j]),(ys[j+1],fy[j+1])] for z in (152,160+lift+rise)],RED)
    roof([-240,-192,-96,96,192,240],[28,10,0,0,10,28],[0,80,128,144],[26,6,0,8],160)
    # Central raised volumes seat on the sloping roofs; no floating boxes.
    for a,b,ha,hb in [(-56,0,176,190),(0,56,190,176)]:
        hull([(x,y,z) for x in (-92,92) for y,bottom in [(a,ha),(b,hb)] for z in (bottom,226 if y else 240)],RED)
    roof([-144,-112,-96,96,112,144],[22,8,0,0,8,22],[0,64,80,88],[20,4,0,8],220)
    for a,b,ha,hb in [(-40,0,234,244),(0,40,244,234)]:
        hull([(x,y,z) for x in (-48,48) for y,bottom in [(a,ha),(b,hb)] for z in (bottom,272 if y else 288)],CREAM)
    roof([-96,-72,-56,56,72,96],[24,8,0,0,8,24],[0,40,56,64],[20,4,0,8],268)
    # Stylized ridge finial.
    prism([(-4,295),(4,295),(0,312)],-3,3,GOLD)
    # Gold brackets articulate the walls between the three roof tiers.
    for sign in (-1,1):
        y0,y1=sorted((sign*56,sign*59))
        box(-88,y0,202,88,y1,206,GOLD)
        for x in (-80,-40,0,40,80):
            box(x-3,y0,184,x+3,y1,202,GOLD)
            box(x-3,y0,206,x+3,y1,224,GOLD)
        y0,y1=sorted((sign*40,sign*42))
        for x in (-40,-20,0,20,40):box(x-2,y0,240,x+2,y1,270,JADE)
    # Eight real columns leave the lower hall open on every side.
    for sign in (-1,1):
        for x in (-180,-80,80,180):
            y=sign*96
            box(x-10,y-10,32,x+10,y+10,40,STONE)
            box(x-8,y-8,40,x+8,y+8,48,GOLD)
            box(x-6,y-6,48,x+6,y+6,140,SCARLET)
            box(x-9,y-9,140,x+9,y+9,146,CREAM)
            box(x-11,y-11,146,x+11,y+11,152,GOLD)
    # Hanging fretwork retains the red and gold rhythm of the reference facade.
    # The 80-unit clear height beneath it accommodates a two-metre player.
    for sign in (-1,1):
        def front_box(x0,x1,z0,z1,d0,d1,color):
            y0,y1=sorted((sign*d0,sign*d1))
            box(x0,y0,z0,x1,y1,z1,color)
        for x0,x1 in [(-164,-96),(-68,68),(96,164)]:
            front_box(x0,x1,112,152,96,99,RED)
            for x in (x0,x1-3):front_box(x,x+3,112,152,99,101,GOLD)
            for z in (112,149):front_box(x0+3,x1-3,z,z+3,99,101,GOLD)
            for x in range(x0+12,x1-6,22):
                outer=[(x,142),(x+7,132),(x,122),(x-7,132)]
                inner=[(x,137),(x+3,132),(x,127),(x-3,132)]
                y0,y1=sorted((sign*101,sign*103))
                for i in range(4):prism([outer[i],outer[(i+1)%4],inner[(i+1)%4],inner[i]],y0,y1,GOLD)
        front_box(-190,190,152,156,96,103,DARK)
        for x in (-180,-80,80,180):front_box(x-4,x+4,156,162,96,104,GOLD)
    # Side lintels link the columns without closing the pavilion.
    for sign in (-1,1):
        x0,x1=sorted((sign*186,sign*192))
        box(x0,-85,144,x1,85,152,RED)
        x0,x1=sorted((sign*192,sign*195))
        box(x0,-85,148,x1,85,152,GOLD)

def save(ed,path):
    result=ed._rpc('tools/call',{'name':'save','arguments':{'path':str(path)}})
    assert not result.get('error') and not result.get('result',{}).get('isError'),result

def main():
    geometry()
    solids=list(r.SOLIDS);r.SOLIDS.clear();r.BRUSHES.clear()
    for points,color in solids:hull([(x,-y,z) for x,y,z in points],color)
    path=ROOT/'temple_jade.map'
    # Nonconvex ornamental outlines are intentionally avoided by hull decomposition.
    points=[p for pts,_ in r.SOLIDS for p in pts];dims=[max(p[k] for p in points)-min(p[k] for p in points) for k in range(3)]
    path.write_text('// Format: Standard\n// heron_brush format 1\n{\n"classname" "worldspawn"\n'+ '\n'.join(brush_text(i,'__TB_empty') for i in range(len(r.SOLIDS)))+'\n}\n')
    ed=Editor()
    try:
        ed.call('open_map',path=str(path));save(ed,path)
        infos=json.loads(ed.call('list_brushes'));brushes=[json.loads(ed.call('describe_brush',brush=b['id'])) for b in infos]
        assert len(brushes)==len(r.SOLIDS)
        for b in brushes:
            assert all(abs(v-round(v))<1e-4 for f in b['faces'] for p in f['vertices'] for v in p),('Off grid',b['id'])
        # Compute only the target faces; neighbors retain their occluding planes.
        occluders=[dict(b,faces=[dict(f,material='skip') for f in b['faces']]) if opaque(b) else None for b in brushes]
        lo=np.array([b['min'] for b in brushes]);hi=np.array([b['max'] for b in brushes]);hidden=set()
        for i,b in enumerate(brushes):
            neighbors=np.flatnonzero(np.all(hi>=lo[i]-1e-4,axis=1)&np.all(lo<=hi[i]+1e-4,axis=1))
            hidden.update(hidden_faces([b]+[occluders[j] for j in neighbors if i!=j and occluders[j] is not None]))
        ls=path.read_text().splitlines(keepends=True);bid=None;face=0
        for i,line in enumerate(ls):
            if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16));face=0
            m=FACE.match(line.rstrip('\n'))
            if m:
                if (bid,face) in hidden:ls[i]=m[1]+'skip'+m[3]+'\n'
                face+=1
        path.write_text(''.join(ls));ed.call('open_map',path=str(path))
        for color in (JADE,DARK,GOLD):
            ids=[b['id'] for b,(_,c) in zip(brushes,r.SOLIDS) if c==color]
            ed.call('select',ids=ids);ed.call('set_surface_material',roughness=.45 if color==GOLD else .55,metallic=.65 if color==GOLD else 0,emission=0)
        ed.call('select',what='all');ed.call('group_selection',name='Jade Temple');save(ed,path)
        ed.call('select',what='all');ed.call('ungroup_selection');ed.call('render_preview',path=str(ROOT/'temple_jade.png'),size=1200)
        report={'brushes':len(brushes),'grid':1,'off_grid_vertices':0,'skip_faces':len(hidden),'units':dims,'metres':[v/32 for v in dims]}
        (ROOT/'temple_jade.validation.json').write_text(json.dumps(report,indent=2)+'\n')
        profiles=ROOT/'scale_profiles.json';data=json.loads(profiles.read_text());data['assets']['temple_jade']={'units':dims,'anchor':'32 units/metre; three-tier jade temple, grid 1; base Z=0'};profiles.write_text(json.dumps(data,indent=2)+'\n')
        print(report,flush=True)
    finally:ed.close()
if __name__=='__main__':main()
