#!/usr/bin/env python3
"""Seven translucent crystal brushes in a stone bed, plus a blue point light."""
import json
from pathlib import Path
import redesign_gen as r
from civic_assets_gen import hull
from fan_gen import brush_text
from prefabs_gen import Editor
from portal_finish import paint_world_gradient

STONE=(71,78,96)
BLUE=(64,119,221)
CYAN=(54,192,201)
PURPLE=(153,90,214)
PALETTES={BLUE:((139,207,250),(37,66,153)),CYAN:((151,244,224),(23,104,151)),PURPLE:((217,168,247),(84,48,151))}


def geometry():
    r.BRUSHES.clear();r.SOLIDS.clear()
    ring=[(1,.5),(.5,1),(-.5,1),(-1,.5),(-1,-.5),(-.5,-1),(.5,-1),(1,-.5)]
    hull([(x*rx,y*ry,z) for z,rx,ry in [(0,20,16),(3,24,20),(7,18,15)] for x,y in ring],STONE)
    hexagon=[(1,0),(.5,.875),(-.5,.875),(-1,0),(-.5,-.875),(.5,-.875)]
    for x,y,radius,height,dx,dy,color in [
        (0,2,7,64,3,0,BLUE),(-12,3,5.5,45,-9,2,PURPLE),
        (10,6,5.5,48,8,5,CYAN),(-7,-9,5,33,-7,-6,CYAN),
        (8,-9,4.5,29,8,-5,PURPLE),(-15,-5,3,20,-8,-1,BLUE),
        (0,12,4,34,-2,7,PURPLE),
    ]:
        # One convex brush includes the shaft and its pointed termination.
        hull([(x+u*radius+lean*dx,y+v*radius+lean*dy,z)
              for z,lean in [(3,0),(height-10,1)] for u,v in hexagon]
             +[(x+dx+1,y+dy,height)],color)


if __name__=='__main__':
    geometry()
    dims=[max(p[k] for points,_ in r.SOLIDS for p in points)-min(p[k] for points,_ in r.SOLIDS for p in points) for k in range(3)]
    path=Path('assets/crystal_cluster.map')
    path.write_text('\n'.join(['// Format: Standard','// heron_brush format 1','{','"classname" "worldspawn"']
                    +[brush_text(i,'__TB_empty') for i in range(len(r.SOLIDS))]+['}'])+'\n')
    import asset_scale
    info={'units':dims,'anchor':'32 units/metre; ground Z=0; authored on 0.125-unit grid'}
    asset_scale.PROFILES['crystal_cluster']=info
    ed=Editor()
    try:
        ed.call('open_map',path=str(path))
        brushes=json.loads(ed.call('list_brushes'))
        for index,brush in enumerate(brushes):
            if index==0:continue
            light,dark=PALETTES[r.SOLIDS[index][1]]
            paint_world_gradient(ed,brush,light,dark,(0,0,-1),(-64,0))
            faces=json.loads(ed.call('describe_brush',brush=brush['id']))['faces']
            for face in faces:
                paint=dict(face['paint']);paint['alpha']=185
                ed.call('set_face_color',brush=brush['id'],face=face['face'],**paint)
                ed.call('set_surface_material',brush=brush['id'],face=face['face'],roughness=.12,metallic=0,emission=.25)
        # Cull the static brushes before adding the separate point entity.
        ed.call('save',path=str(path))
        ed.call('add_point_entity',classname='light',origin=[0,-28,28],properties={
            'light_color':'90 180 255','light_energy':'1.5','omni_range':'3',
            'shadow_enabled':'0','light':'160','_color':'0.352941 0.705882 1',
        })
        # Optimise only after setting alpha: translucent crystals must not occlude each other.
        ed.call('save',path=str(path))
        ed.call('render_preview',path='assets/crystal_cluster.png',size=1000)
        profiles=Path('assets/scale_profiles.json');data=json.loads(profiles.read_text())
        data['assets']['crystal_cluster']=info;profiles.write_text(json.dumps(data,indent=2)+'\n')
        print('crystal_cluster:',len(brushes),'brushes;',dims,'units;',path.read_text().count(' skip '),'skip faces; 1 blue light',flush=True)
    finally:ed.close()
