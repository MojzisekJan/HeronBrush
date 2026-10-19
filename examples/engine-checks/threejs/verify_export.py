#!/usr/bin/env python3
"""Validate both exports against each other and their referenced GLB/PNG resources."""
import collections, json, math, struct
from pathlib import Path
BASE=Path(__file__).resolve().parent
level=json.loads((BASE/'assets/demo.json').read_text())
entities={e['name']:e for e in level['entities']}
ray=BASE.parent/'raylib'
sidecar=json.loads((ray/'assets/maps/demo.json').read_text())
assert [e['properties'] for e in sidecar['entities']]==[e['properties'] for e in level['entities']]
assert [e.get('targets',[]) for e in sidecar['entities']]==[e.get('targets',[]) for e in level['entities']]
flat={}; textures={}; current=None
for line in (ray/'assets/maps/demo.level').read_text().splitlines():
 parts=line.split()
 if not parts or parts[0].startswith('#'):continue
 key=parts[0]
 if key=='entity':
  current={'classname':parts[1],'properties':{},'values':{},'targets':[],'triangles':0,'boxes':0,'hulls':[]};flat[parts[2]]=current
 elif key=='texture':textures[parts[1]]=parts[2]
 elif key=='prop':
  current['properties'][parts[2]]=parts[1]
  current['values'][parts[2]]=line.split(None,3)[3] if len(line.split(None,3))>3 else ''
 elif key=='target':current['targets'].append(parts[1])
 elif key=='ctri':current['triangles']+=1
 elif key=='cbox':current['boxes']+=1
 elif key=='chull':
  current['hulls'].append([]); expected_hull_size=int(parts[1])
 elif key=='chv':current['hulls'][-1].extend(map(float,parts[1:]))
 elif key=='chend':assert len(current['hulls'][-1])==expected_hull_size*3
 elif key=='pos':current['pos']=list(map(float,parts[1:]))
 elif key=='rot':current['rot']=list(map(float,parts[1:]))
 elif key=='scale' and current:current['scale']=list(map(float,parts[1:]))
assert flat.keys()==entities.keys()
max_index=0; triangle_count=0; embedded_materials=0
for name,e in entities.items():
 f=flat[name]
 assert f['classname']==e['classname']
 assert f['properties']=={k:v['type'] for k,v in e['properties'].items()},name
 for key,value in e['properties'].items():
  actual=f['values'][key]; expected=value['value']; kind=value['type']
  if kind=='string':assert actual==expected.replace('\n',' ').replace('\r',' '),(name,key)
  elif kind in ('int','float','bool'):assert abs(float(actual)-float(expected))<1e-5,(name,key)
  elif kind in ('vec2','vec3','color'):
   assert all(abs(a-b)<1e-5 for a,b in zip(map(float,actual.split()),expected)),(name,key)
 assert f['targets']==e.get('targets',[]),name
 assert all(t in entities for t in f['targets'])
 for a,b in [('pos','translation'),('rot','rotation_yxz'),('scale','scale')]:
  assert all(abs(x-y)<1e-5 for x,y in zip(f[a],e['transform'][b]))
 c=e.get('collision') or {}
 if c.get('triangles'):assert f['triangles']==len(c['triangles'])//9
 if c.get('hulls'):
  assert f['boxes']==len(c['hulls'])
  assert len(f['hulls'])==len(c['hulls'])
  for actual,expected in zip(f['hulls'],c['hulls']):
   assert len(actual)==len(expected)
   assert all(abs(a-b)<1e-5 for a,b in zip(actual,expected)),name
 if not e.get('mesh'):continue
 data=(BASE/'assets'/e['mesh']).read_bytes()
 assert data[:4]==b'glTF'
 n=struct.unpack_from('<I',data,12)[0];g=json.loads(data[20:20+n]);binary=data[28+n:]
 for view in g['bufferViews']:assert view.get('byteOffset',0)+view['byteLength']<=len(binary)
 for material in g.get('materials',[]):
  texture=g['textures'][material['pbrMetallicRoughness']['baseColorTexture']['index']]
  image=g['images'][texture['source']]
  view=g['bufferViews'][image['bufferView']]
  offset=view.get('byteOffset',0)
  picture=binary[offset:offset+view['byteLength']]
  assert image['mimeType']=='image/png' and picture.startswith(b'\x89PNG\r\n\x1a\n')
  expected_wrap=33071 if material['name'].startswith('bbpaint_') else 10497
  assert g['samplers'][texture['sampler']]['wrapS']==expected_wrap
  embedded_materials+=1
 for mesh in g['meshes']:
  for prim in mesh['primitives']:
   a=g['accessors'][prim['indices']];v=g['bufferViews'][a['bufferView']]
   fmt={5123:'H',5125:'I'}[a['componentType']]
   indices=struct.unpack_from('<'+fmt*a['count'],binary,v.get('byteOffset',0)+a.get('byteOffset',0))
   pos=g['accessors'][prim['attributes']['POSITION']]
   assert max(indices)<pos['count'];max_index=max(max_index,max(indices));triangle_count+=len(indices)//3
for t in level['textures']:
 if t.get('image'):
  data=(BASE/'assets'/t['image']).read_bytes();assert data.startswith(b'\x89PNG\r\n\x1a\n')
  assert (ray/'assets'/textures[t['name']]).read_bytes()==data
paints=[t for t in level['textures'] if t['name'].startswith('bbpaint_')];assert len(paints)==2
report={'entities':len(entities),'textures':len(level['textures']),'paint_textures':len(paints),
 'target_links':sum(len(e.get('targets',[])) for e in entities.values()),'property_fields':sum(len(e['properties']) for e in entities.values()),
 'visible_triangles':triangle_count,'largest_primitive_index':max_index,
 'collision_triangles':sum(f['triangles'] for f in flat.values()),'convex_boxes':sum(f['boxes'] for f in flat.values()),'exact_hulls':sum(len(f['hulls']) for f in flat.values()),'embedded_albedo_materials':embedded_materials}
print(json.dumps(report,indent=2))
