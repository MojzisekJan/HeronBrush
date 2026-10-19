from pathlib import Path
import json,re
b=Path(__file__).resolve().parent; d=json.loads((b/'main/arena.json').read_text())
for e in d['entities']:
 p=b/'main/arena'/ (e['name']+'.script')
 if p.exists():
  s=p.read_text().split('-- GAME FACTORY')[0]
  s+='\n-- GAME FACTORY\nlocal game=require "main.entity_runtime"\nlocal exported_init,exported_update,exported_message=init,update,on_message\nfunction init(self) if exported_init then exported_init(self) end;game.init(self,'+json.dumps(e['name'])+') end\nfunction update(self,dt) if sys.get_config_string("check.enabled","0")=="3" then dt=1/60 end;if exported_update then exported_update(self,dt) end;game.update(self,dt) end\nfunction on_message(self,id,message,sender) if exported_message then exported_message(self,id,message,sender) end;game.message(self,id,message) end\n'
  p.write_text(s)
co='type: COLLISION_OBJECT_TYPE_TRIGGER\nmass: 0\nfriction: 0\nrestitution: 0\ngroup: "default"\nmask: "default"\nmask: "player"\nembedded_collision_shape {\n shapes { shape_type: TYPE_BOX position {} rotation {} index: 0 count: 3 }\n data: 1.05\n data: 1.5\n data: 0.25\n}\n'
(b/'main/portal.collisionobject').write_text(co)
p=b/'main/arena.collection';s=p.read_text()
def inject(m):
 data=json.loads(m.group(1))
 if '/fps_portal' in data and '/main/portal.collisionobject' not in data:
  data+='components { id: "portal_sensor" component: "/main/portal.collisionobject" }\ncomponents { id: "portal_frame" component: "/main/portal.model" }\n'
 return '  data: '+json.dumps(data)
s=re.sub(r'  data: ("(?:[^"\\]|\\.)*")',inject,s);p.write_text(s)
p=b/'game.project';s=p.read_text().replace('version = 0.1','version = 0.1\ncustom_resources = /main/arena.json') if 'custom_resources' not in p.read_text() else p.read_text();p.write_text(s)
