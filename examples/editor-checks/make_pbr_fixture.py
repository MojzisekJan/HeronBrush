#!/usr/bin/env python3
"""Generate a small GLB with separate PBR map dimensions, without external assets."""
import argparse, json, struct, zlib
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('path',type=Path)
parser.add_argument('--surface-modes',action='store_true',help='Generate OPAQUE/MASK/BLEND with single/double-sided variants')
args=parser.parse_args()
def chunk(k,b):return struct.pack('>I',len(b))+k+b+struct.pack('>I',zlib.crc32(k+b)&0xffffffff)
def png(w,h,rgba):
    rows=b''.join(b'\0'+bytes(rgba)*w for _ in range(h))
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b'')
blob=bytearray();views=[]
def data(b):
    blob.extend(b'\0'*((-len(blob))%4));offset=len(blob);blob.extend(b)
    views.append({'buffer':0,'byteOffset':offset,'byteLength':len(b)});return len(views)-1
pos=data(struct.pack('<9f',0,0,0,0,0,1,1,0,0));uv=data(struct.pack('<6f',0,0,0,1,1,0));normal=data(struct.pack('<9f',0,1,0,0,1,0,0,1,0));indices=data(struct.pack('<3H',0,1,2))
images=[]
for w,h,color in [(2,2,[180,140,100,255]),(2,2,[180,110,240,255]),(2,1,[17,128,192,255]),(1,2,[64,200,30,255]),(2,2,[32,180,64,255])]:
    images.append({'bufferView':data(png(w,h,color)),'mimeType':'image/png'})
mat={'pbrMetallicRoughness':{'baseColorTexture':{'index':0},'baseColorFactor':[0.5,1,1,1],'roughnessFactor':0.6,'metallicFactor':0.4,'metallicRoughnessTexture':{'index':2}},'normalTexture':{'index':1,'scale':0.35},'occlusionTexture':{'index':3,'strength':0.25},'emissiveTexture':{'index':4},'emissiveFactor':[0.2,0.4,0.6]}
second=json.loads(json.dumps(mat));second['pbrMetallicRoughness']['roughnessFactor']=0.9
primitive={'attributes':{'POSITION':0,'TEXCOORD_0':1,'NORMAL':2},'indices':3,'material':0}
p2=json.loads(json.dumps(primitive));p2['material']=1
scene={'asset':{'version':'2.0'},'scene':0,'scenes':[{'nodes':[0,1]}],'nodes':[{'mesh':0},{'mesh':1,'translation':[1,0,0]}],'meshes':[{'primitives':[primitive]},{'primitives':[p2]}],'materials':[mat,second],'images':images,'textures':[{'source':i} for i in range(5)],'buffers':[{'byteLength':len(blob)}],'bufferViews':views,'accessors':[{'bufferView':v,'componentType':5126 if i<3 else 5123,'count':3,'type':['VEC3','VEC2','VEC3','SCALAR'][i]} for i,v in enumerate([pos,uv,normal,indices])]}
if args.surface_modes:
    # The same alpha gradient in every material makes mode changes observable.
    # Opaque ignores it, Mask uses 0.42, Blend preserves continuous coverage.
    rows=b''.join(b'\0'+b''.join(bytes([180,140,100,a]) for a in pair) for pair in [(0,96),(192,255)])
    gradient=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b'')
    scene['images'][0]={'bufferView':data(gradient),'mimeType':'image/png'}
    scene['materials']=[];scene['nodes']=[];scene['meshes']=[]
    for mode in ['OPAQUE','MASK','BLEND']:
        for double in [False,True]:
            index=len(scene['materials'])
            material=json.loads(json.dumps(mat))
            material.update(name=mode+('_double' if double else '_single'),alphaMode=mode,doubleSided=double)
            material['pbrMetallicRoughness']['baseColorFactor'][3]=0.75
            if mode=='MASK':material['alphaCutoff']=0.42
            scene['materials'].append(material)
            item=json.loads(json.dumps(primitive));item['material']=index
            scene['meshes'].append({'primitives':[item]})
            scene['nodes'].append({'mesh':index,'translation':[index*1.5,0,0]})
    scene['scenes'][0]['nodes']=list(range(6))
    scene['buffers'][0]['byteLength']=len(blob)
metadata=bytearray(json.dumps(scene,separators=(',',':')).encode());metadata.extend(b' '*((-len(metadata))%4));blob.extend(b'\0'*((-len(blob))%4));size=12+8+len(metadata)+8+len(blob)
args.path.parent.mkdir(parents=True,exist_ok=True)
args.path.write_bytes(struct.pack('<3I',0x46546c67,2,size)+struct.pack('<2I',len(metadata),0x4e4f534a)+metadata+struct.pack('<2I',len(blob),0x004e4942)+blob)
