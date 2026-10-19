#!/usr/bin/env python3
"""One-brush wall with tileable, mathematically matched albedo and tangent normals."""
from pathlib import Path
import json
import numpy as np
from PIL import Image

ROOT=Path(__file__).resolve().parent
SIZE=512

def textures():
    y,x=np.mgrid[:SIZE,:SIZE]
    row=y//32; column=((x+(row%2)*32)%SIZE)//64
    u=(x+(row%2)*32)%64; v=y%32
    edge=np.minimum.reduce([u,63-u,v,31-v]).astype(float)
    t=np.clip((edge-1.5)/4,0,1)
    height=t*t*(3-2*t)
    # Height derivatives wrap at the borders, so repeating the texture has no seam.
    dx=(np.roll(height,-1,axis=1)-np.roll(height,1,axis=1))*0.5*4
    dy=(np.roll(height,-1,axis=0)-np.roll(height,1,axis=0))*0.5*4
    normal=np.stack([-dx,-dy,np.ones_like(dx)],axis=-1)
    normal/=np.linalg.norm(normal,axis=-1,keepdims=True)
    normals=np.rint((normal*.5+.5)*255).clip(0,255).astype('uint8')
    palette=np.array([[158,76,49],[174,85,56],[146,67,46],[183,94,62],[161,82,58]])
    bricks=palette[(row*7+column*3)%len(palette)].astype(float)
    # Low-amplitude periodic grain stays in albedo, separate from the shallow bevel.
    grain=2*np.sin(x*2*np.pi/16)*np.cos(y*2*np.pi/8)
    mortar=np.array([151,139,116])
    blend=np.clip((edge-1)/2.5,0,1)[...,None]
    albedo=np.rint(mortar*(1-blend)+(bricks+grain[...,None])*blend).clip(0,255).astype('uint8')
    folder=ROOT/'textures'/'heron_demo';folder.mkdir(parents=True,exist_ok=True)
    Image.fromarray(albedo).save(folder/'brick_wall.png')
    Image.fromarray(normals).save(folder/'brick_wall_normal.png')

def wall():
    import redesign_gen as r
    from civic_assets_gen import box
    from fan_gen import brush_text
    r.BRUSHES.clear();r.SOLIDS.clear()
    box(-48,-4,0,48,4,80,(255,255,255))
    brush=brush_text(0,'heron_demo/brick_wall')
    # Textured faces must have no solid paint overriding their albedo.
    brush='\n'.join(line for line in brush.splitlines() if not line.startswith('// heron_brush color'))
    brush=brush.replace(' 0 0 0 1 1',' 0 0 0 0.1875 0.1875 // heron_brush pbr 88 0 0')
    path=ROOT/'wall_brick_normal.map'
    path.write_text('// Format: Standard\n// heron_brush format 1\n{\n"classname" "worldspawn"\n'+brush+'\n}\n')
    from prefabs_gen import Editor
    import asset_scale
    asset_scale.PROFILES['wall_brick_normal']={'units':[96,8,80],'anchor':'32 units/metre; one brush; ground Z=0'}
    ed=Editor()
    try:
        ed.call('open_map',path=str(path));ed.call('save',path=str(path))
        ed.call('load_materials',path=str(ROOT/'textures'))
        brushes=json.loads(ed.call('list_brushes'));assert len(brushes)==1
        faces=json.loads(ed.call('describe_brush',brush=brushes[0]['id']))['faces']
        assert len(faces)==6 and all(f['material']=='heron_demo/brick_wall' and not f.get('paint') for f in faces)
        print('Brick wall: 1 brush, 6 exposed faces, 96 x 8 x 80 units, tileable 512px textures')
    finally:ed.close()

if __name__=='__main__':textures();wall()
