#!/usr/bin/env python3
"""Verify entity wiring, payloads and every referenced Bevy mesh/image on disk."""
import collections,json,struct
from pathlib import Path
BASE=Path(__file__).resolve().parent
assets=BASE/'assets';level_path=assets/'maps/demo.json'
level=json.loads(level_path.read_text());entities=level['entities']
assert level['format']==2 and level['scale']==32
counts=collections.Counter(e['classname'] for e in entities)
for classname,count in {'fps_portal':2,'func_door':3,'fps_enemy':2,'fps_key_door':1,'light':5,'info_player_start':1}.items():
 assert counts[classname]==count,(classname,counts[classname])
by_name={e['name']:e for e in entities}
for e in entities:
 for target in e.get('targets',[]):assert target in by_name,(e['name'],target)
 if e['classname']=='fps_portal':
  assert len(e['targets'])==1
  dest=by_name[e['targets'][0]]
  assert dest['classname']=='fps_portal' and dest['targets']==[e['name']]
 if e['classname']=='func_door':
  assert e['properties']['travel']['value']==96
  assert e['properties']['speed']['value']>0
  assert e['collision']['kind']=='convex'
 if e['classname']=='fps_enemy':assert e['scene']=='res://objects/enemy.tscn'
 if 'mesh' not in e:continue
 path=level_path.parent/e['mesh'];data=path.read_bytes()
 magic,version,length=struct.unpack_from('<4sII',data)
 assert magic==b'glTF' and version==2 and length==len(data)
 n,tag=struct.unpack_from('<II',data,12);gltf=json.loads(data[20:20+n])
 for surface in e.get('surfaces',[]):
  assert surface['primitive']<len(gltf['meshes'][0]['primitives'])
 for material in gltf.get('materials',[]):
  texture=gltf['textures'][material['pbrMetallicRoughness']['baseColorTexture']['index']]
  image=gltf['images'][texture['source']]
  assert image['mimeType'] in ('image/png','image/jpeg')
  view=gltf['bufferViews'][image['bufferView']]
  start=20+n+8+view.get('byteOffset',0)
  payload=data[start:start+view['byteLength']]
  assert payload.startswith(b'\x89PNG\r\n\x1a\n') or payload.startswith(b'\xff\xd8\xff')
  wrap=33071 if material['name'].startswith('bbpaint_') else 10497
  sampler=gltf['samplers'][texture['sampler']]
  assert sampler['wrapS']==wrap and sampler['wrapT']==wrap
 if 'collision' in e:
  c=e['collision'];assert c['parts']==len(gltf['meshes'][c['mesh']]['primitives'])
for t in level['textures']:
 assert t.get('image'),t
 assert (assets/t['image']).is_file(),t
assert len([t for t in level['textures'] if t['name'].startswith('bbpaint_')])>=2
assert by_name['worldspawn']['collision']['kind']=='trimesh'
print(f"PASS: {len(entities)} entities, {len(level['textures'])} textures; linked portals, door payloads, enemy references, visual/collision GLB primitives and painted frames.")
print('Data validation does not execute Godot scripts or implement Bevy gameplay.')
