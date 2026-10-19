#!/usr/bin/env python3
"""Build Relay Yard through the editor MCP; preserve original asset face paint."""
import json, subprocess, re, pathlib, math
HERE=pathlib.Path(__file__).resolve().parent
ROOT=HERE.parents[1]
p=subprocess.Popen([str(ROOT/'target/release/heron-brush'),'--mcp'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
n=0
def rpc(method,params={}):
 global n
 n+=1;p.stdin.write(json.dumps(dict(jsonrpc='2.0',id=n,method=method,params=params))+'\n');p.stdin.flush()
 while True:
  line=p.stdout.readline()
  if not line:raise RuntimeError('MCP exited')
  r=json.loads(line)
  if r.get('id')==n:
   if 'error' in r or r.get('result',{}).get('isError'):raise RuntimeError(r)
   return r['result']
def call(name,**args):
 r=rpc('tools/call',dict(name=name,arguments=args))
 return '\n'.join(x.get('text','') for x in r.get('content',[]))
rpc('initialize')
C={'white':[206,222,229],'dark':[36,49,65],'road':[62,75,91],'teal':[32,188,180],'orange':[246,148,63],'glass':[71,126,157],'light':[232,249,242]}
def shape(lo,hi,c='white',kind='cuboid',gradient=False,**kw):
 call('add_shape',kind=kind,min=lo,max=hi,material='__TB_empty',**kw)
 color=C.get(c,c) if isinstance(c,str) else c
 if gradient:call('set_face_color',gradient={'type':'linear','angle':90,'stops':[{'position':0,'color':[int(v*.62) for v in color]},{'position':1,'color':color}]})
 else:call('set_face_color',color=color)
def box(x,y,z,X,Y,Z,c='white',**kw):shape([x,y,z],[X,Y,Z],c,**kw)
def point(cls,pos,**props):call('add_point_entity',classname=cls,origin=pos,properties=props)
call('new_map')
# Floating architectural base, divided into traversable precincts.
box(-800,-600,-48,800,640,0,'dark')
box(-784,-584,0,784,624,8,'road')
box(-760,-560,8,-160,80,24,'white')
box(224,-560,8,752,64,72,'white')
box(-760,352,8,752,608,24,'white')
# Courtyard street, continuous corner bands and lane markings.
for x in range(-640,700,112):box(x,194,8,x+56,198,9,'light')
for y in (92,302):box(-760,y,8,752,y+5,10,'orange')
# Factory at the rear: actual opening, steel bays, sawtooth roof.
box(-736,-552,24,-712,-112,264,'white',gradient=True)
box(-736,-552,24,-160,-528,264,'white',gradient=True)
box(-184,-552,24,-160,-112,264,'white',gradient=True)
box(-712,-132,24,-552,-112,240,'white',gradient=True)
box(-392,-132,24,-184,-112,240,'white',gradient=True)
box(-552,-132,164,-392,-112,264,'teal',gradient=True)
for x in (-700,-572,-444,-316,-196):
 box(x,-530,24,x+12,-124,34,'dark')
 box(x,-530,24,x+12,-514,264,'dark')
 box(x,-132,24,x+12,-116,264,'dark')
for x in range(-720,-180,108):
 shape([x,-552,264],[x+108,-112,310],'white',kind='wedge',axis='x',gradient=True)
 box(x+98,-548,276,x+106,-116,304,'glass')
# Upper workshop, ribbon glazing, orange service core.
box(272,-544,72,720,-520,256,'white',gradient=True)
box(696,-520,72,720,0,256,'white',gradient=True)
box(272,-520,72,296,0,256,'white',gradient=True)
box(296,-24,72,440,0,144,'teal',gradient=True)
box(536,-24,72,696,0,144,'teal',gradient=True)
box(296,-24,192,696,0,256,'white')
for x in (304,360,552,608,664):box(x,-26,148,x+28,-20,188,'glass',gradient=True)
box(256,-560,256,736,16,268,'dark')
box(576,-480,268,688,-304,340,'orange',gradient=True)
# Connecting elevated bridge and its supports, with continuous rails.
box(-176,-360,160,280,-264,176,'dark')
for y in (-360,-270):
 box(-176,y,214,280,y+6,220,'teal')
 for x in range(-160,281,64):box(x,y,176,x+5,y+6,218,'dark')
for x in (-128,216):box(x,-352,8,x+16,-272,160,'dark')
# Accessible loading apron: stairs and vehicle ramp.
shape([64,-80,8],[224,48,72],'white',kind='stairs',axis='x',step_height=8)
shape([448,0,8],[688,176,72],'road',kind='wedge',axis='y',rotate=[0,0,180])
shape([-384,-496,24],[-192,-400,160],'dark',kind='stairs',axis='x',step_height=8)
# Low foreground logistics office keeps the view into the yard open.
box(-720,432,24,-416,568,132,'white',gradient=True)
box(-732,420,132,-404,580,144,'teal')
for x in range(-688,-430,64):box(x,429,60,x+44,432,108,'glass',gradient=True)
# Substation, overhead pipe supports, bollards and parking bays.
for x in (-64,112):
 box(x,-512,8,x+12,-496,220,'dark')
 box(x,-512,208,x+12,-144,220,'dark')
 box(x,-160,8,x+12,-144,220,'dark')
for x in range(-352,161,128):
 box(x,360,24,x+4,488,25,'light')
for x in (-740,-580,-420,-100,120,340,540,740):
 box(x,320,8,x+8,328,46,'orange')
 box(x,320,38,x+8,328,44,'light')
# Broad perimeter uses buildings and stepped service blocks, not a tall wall.
for y in range(-480,481,160):
 box(760,y,8,784,y+104,60,'teal',gradient=True)
 box(-784,y,8,-760,y+104,60,'teal',gradient=True)
# Portal plinth and circular painted landing zone.
shape([336,368,24],[560,592,32],'dark',kind='cylinder',sides=32)
shape([356,388,32],[540,572,34],'teal',kind='cylinder',sides=32)
call('set_face_color',gradient={'type':'radial','stops':[{'position':0,'color':[198,255,235]},{'position':.6,'color':[35,185,184]},{'position':1,'color':[31,76,103]}]})
point('info_player_start',[0,256,32],angle='90')
point('light',[0,0,420],_color='1 0.94 0.85',light='900')
point('light',[448,460,190],_color='0.2 0.9 1',light='350')
call('save',path=str(HERE/'architecture.map'))
base=call('get_map')
# Transform assets through MCP so face paint, bevels and planes stay native.
def blocks(text):
 depth=0;start=None;out=[]
 for line in text.splitlines(True):
  if line.strip()=='{':
   depth+=1
   if depth==2:start=[]
  if start is not None:start.append(line)
  if line.strip()=='}':
   if depth==2:out.append(''.join(start));start=None
   depth-=1
 return out
assets=[];entities=[]
def asset(name,pos,scale=1,angle=0,entity=None):
 call('open_map',path=str(ROOT/'assets'/f'{name}.map'));call('select',what='all')
 if scale!=1:call('scale_selection',factors=[scale]*3,center=[0,0,0])
 if angle:call('rotate_selection',axis=[0,0,1],degrees=angle,center=[0,0,0])
 bs=json.loads(call('list_brushes'))
 # Asset coordinates use their authored base; align the lowest plane to requested ground.
 call('translate_selection',delta=pos)
 parts=blocks(call('get_map'))
 if entity:
  entities.append('{\n'+''.join(json.dumps(k)+' '+json.dumps(str(v))+'\n' for k,v in entity.items())+''.join(parts)+'}\n')
 else:assets.extend(parts)
 print(name,len(parts),flush=True)
asset('shipping_container',[-570,290,24],1)
asset('shipping_container',[580,-382,72],.8,90)
asset('car_retro_sedan',[-270,416,24],1,90)
asset('car_retro_compact',[-100,416,24],1,90)
asset('truck',[400,130,8],.7,90)
for name,pos in [('barrel_blue',[-650,-180,24]),('barrel_yellow',[-610,-180,24]),('barrel_red',[312,-440,72]),('barrel_green',[-660,370,24])]:asset(name,pos)
asset('factory_chimney',[-650,-444,24],1.4)
for y in (-460,-364,-268):asset('pipe_straight',[24,y,208],1,90)
asset('pipe_elbow',[24,-160,208])
asset('pipe_valve',[24,-360,208])
asset('pipe_tee',[24,-440,208])
asset('portal_ring',[448,480,34],1.4,0,{'classname':'func_rotate','targetname':'relay_ring','axis':'0 1 0','speed':'20','origin':'448 480 104','script':'res://scripts/rotator.gd'})
asset('portal_frame',[-472,-420,24],1.2)
# Insert asset solids into worldspawn without changing the generated entities.
depth=0;idx=0
for line in base.splitlines(True):
 if line.strip()=='{':depth+=1
 if line.strip()=='}':
  depth-=1
  if depth==0:break
 idx+=len(line)
base=base[:idx]+ '\n'+''.join(assets)+base[idx:]+''.join(entities)
(HERE/'relay_yard.map').write_text(base)
call('open_map',path=str(HERE/'relay_yard.map'))
call('save',path=str(HERE/'relay_yard.map'))
print(call('render_preview',path=str(HERE/'preview.png'),size=1600),flush=True)
p.stdin.close();p.wait()
