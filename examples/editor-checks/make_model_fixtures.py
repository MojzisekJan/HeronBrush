#!/usr/bin/env python3
"""Create tiny textured OBJ/GLB assets for offline native import checks."""
import json, struct, zlib, argparse
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);parser.add_argument('--multi',action='store_true');args=parser.parse_args();root=args.directory;root.mkdir(parents=True,exist_ok=True)
def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
pixels=bytes([255,30,30,255,30,255,30,255,30,30,255,255,255,255,255,255])
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>2I5B',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0'+pixels[:8]+b'\0'+pixels[8:]))+chunk(b'IEND',b'')
(root/'texture.png').write_bytes(png)
(root/'paint.mtl').write_text('newmtl paint\nmap_Kd texture.png\n')
(root/'triangle.obj').write_text('mtllib paint.mtl\nv 0 0 0\nv 64 0 0\nv 0 64 32\nvt 0 0\nvt 1 0\nvt 0 1\nusemtl paint\nf 1/1 2/2 3/3\n')
binary=bytearray(struct.pack('<9f6f3H',0,0,0,1,0,0,0,1,1,0,0,1,0,0,1,0,1,2));binary.extend(b'\0'*((-len(binary))%4));image_offset=len(binary);binary.extend(png)
scene={'asset':{'version':'2.0'},'scene':0,'scenes':[{'nodes':[0]}],'nodes':[{'mesh':0}],'meshes':[{'primitives':[{'attributes':{'POSITION':0,'TEXCOORD_0':1},'indices':2,'material':0}]}],'materials':[{'pbrMetallicRoughness':{'baseColorTexture':{'index':0}}}],'textures':[{'source':0}],'images':[{'bufferView':3,'mimeType':'image/png'}],'buffers':[{'byteLength':len(binary)}],'bufferViews':[{'buffer':0,'byteOffset':0,'byteLength':36},{'buffer':0,'byteOffset':36,'byteLength':24},{'buffer':0,'byteOffset':60,'byteLength':6},{'buffer':0,'byteOffset':image_offset,'byteLength':len(png)}],'accessors':[{'bufferView':0,'componentType':5126,'count':3,'type':'VEC3'},{'bufferView':1,'componentType':5126,'count':3,'type':'VEC2'},{'bufferView':2,'componentType':5123,'count':3,'type':'SCALAR'}]}
if args.multi:
    (root/'paint.mtl').write_text('newmtl paint\nmap_Kd texture.png\nnewmtl green\nKd 0 1 0\nmap_Kd texture.png\n')
    (root/'triangle.obj').write_text('mtllib paint.mtl\nv 0 0 0\nv 64 0 0\nv 0 64 32\nv 64 64 32\nvt 0 0\nvt 1 0\nvt 0 1\nvt 1 1\nvn 0 0 1\nvn 0 1 1\nusemtl paint\nf 1/1/1 2/2/2 3/3/1\nusemtl green\nf 2/2/2 4/4/1 3/3/1\n')
    binary.extend(b'\0'*((-len(binary))%4))
    normals_offset=len(binary);binary.extend(struct.pack('<9f',0,0,1,0,1,1,0,0,1))
    scene['bufferViews'].append({'buffer':0,'byteOffset':normals_offset,'byteLength':36})
    scene['accessors'].append({'bufferView':4,'componentType':5126,'count':3,'type':'VEC3'})
    primitive=scene['meshes'][0]['primitives'][0];primitive['attributes']['NORMAL']=3
    second=json.loads(json.dumps(primitive));second['material']=1
    # A separate node places the second material beside the first triangle.
    scene['meshes'].append({'primitives':[second]});scene['nodes'].append({'mesh':1,'translation':[1,0,0]});scene['scenes'][0]['nodes'].append(1)
    scene['materials'].append({'pbrMetallicRoughness':{'baseColorTexture':{'index':0},'baseColorFactor':[0,1,0,1]}})
    scene['buffers'][0]['byteLength']=len(binary)
metadata=bytearray(json.dumps(scene,separators=(',',':')).encode());metadata.extend(b' '*((-len(metadata))%4));binary.extend(b'\0'*((-len(binary))%4));size=12+8+len(metadata)+8+len(binary)
(root/'triangle.glb').write_bytes(struct.pack('<3I',0x46546c67,2,size)+struct.pack('<2I',len(metadata),0x4e4f534a)+metadata+struct.pack('<2I',len(binary),0x004e4942)+binary)
