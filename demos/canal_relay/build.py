#!/usr/bin/env python3
"""Author the three campaign maps and export through the real Heron Brush MCP."""
import argparse,json,sys,shutil
from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'assets'));import prefabs_gen
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,default=ROOT/'target/debug/heron-brush');p.add_argument('--output',type=Path,default=Path('/tmp/heron-canal-relay'));p.add_argument('--generate',action='store_true');args=p.parse_args()
prefabs_gen.BIN=str(args.binary.resolve());e=prefabs_gen.Editor()
def box(lo,hi,color,classname=None,properties=None):
 e.call('add_shape',kind='cuboid',min=lo,max=hi,material='__TB_empty');e.call('set_face_color',color=color)
 if classname:
  e.call('make_brush_entity',classname=classname)
  for key,value in (properties or {}).items():e.call('run_plugin',path=str(ROOT/'assets/plugins/bulk-properties'),action='apply',settings={'key':key,'value':value})
def point(cls,origin,**props):e.call('add_point_entity',classname=cls,origin=origin,properties=props)
try:
 if args.generate:
  for index,name in enumerate(['courtyard','pumphouse','beacon']):
   e.call('new_map');e.call('load_definitions',path=str(HERE/'game.fgd'))
   floor=[154,174,174];stone=[210,205,175];dark=[43,69,76];copper=[189,102,52];teal=[45,165,154]
   # Two quays, a safe crossing and a blue canal. All dimensions are map units.
   box([-384,-256,-32],[-48,256,0],floor);box([48,-256,-32],[384,256,0],floor)
   bridge_y=136 if index==1 else 0
   box([-48,bridge_y-40,-16],[48,bridge_y+40,0],stone)
   box([-48,-256,-64],[48,256,-48],[35,105,151])
   box([-47,-255,-48],[47,255,-4],dark,'relay_zone',{'kind':'hazard'})
   for y in [-264,256]:box([-392,y,0],[392,y+8,128],stone)
   for x in [-392,384]:box([x,-264,0],[x+8,264,128],stone)
   # Pier lamps, copper parapets, gate posts and low-poly corner towers.
   for x in [-352,352]:
    for y in [-224,224]:
     box([x-24,y-24,0],[x+24,y+24,144],dark)
     box([x-32,y-32,144],[x+32,y+32,160],copper)
     box([x-16,y-16,160],[x+16,y+16,192],teal)
   for y in [bridge_y-56,bridge_y+48]:
    box([-20,y,0],[20,y+8,128],dark)
   box([-20,bridge_y-56,128],[20,bridge_y+56,144],copper)
   box([-8,bridge_y-40,0],[8,bridge_y+40,96],copper,'relay_door',{'targetname':'gate','rise':'3.5'})
   for x in [-270,270]:
    for y in [-180,180]:
     box([x-32,y-24,0],[x+32,y+24,16],dark)
     box([x-16,y-12,16],[x+16,y+12,32],teal)
   point('relay_spawn',[-280,bridge_y,2])
   positions=[[-245,-95,24],[-245,95,24]]
   if index==1:
    for step in range(4):
     box([-180-step*24,-210+step*24,0],[-132-step*24,-186+step*24,(step+1)*24],stone)
    box([-300,-128,0],[-220,-64,96],dark)
    positions=[[-260,48,24],[-150,40,24],[-260,-95,120]]
   for i,pos in enumerate(positions):point('relay_cell',pos,cell_id=f'cell-{i+1}',value='1')
   point('relay_station',[-115,bridge_y-80 if index==1 else bridge_y+90,0],station_id='relay',cost=str(len(positions)),target='gate')
   box([260,bridge_y-32,0],[324,bridge_y+32,96],teal,'relay_zone',{'kind':'exit','required':'relay','next_level':['pumphouse','beacon',''][index]})
   # Exit arch and destination beacon make the route visible from the quay.
   for y in [bridge_y-48,bridge_y+40]:box([280,y,0],[312,y+8,112],teal)
   box([280,bridge_y-48,112],[312,bridge_y+48,128],copper)
   if index==2:
    for level in range(3):box([170+level*8,-230+level*8,level*24],[250-level*8,-150-level*8,(level+1)*24],stone)
    point('relay_cell',[210,-190,98],cell_id='bonus',value='1')
   e._rpc('tools/call',{'name':'save','arguments':{'path':str(HERE/(name+'.map'))}})
 if (HERE/'godot').resolve() != args.output.resolve():
  shutil.copytree(HERE/'godot',args.output,dirs_exist_ok=True)
 (args.output/'maps').mkdir(exist_ok=True)
 profile=json.loads((HERE/'.heron-brush/exports.json').read_text())['targets']['godot']
 for name in ['courtyard','pumphouse','beacon']:
  e.call('open_map',path=str(HERE/(name+'.map')));e.call('load_definitions',path=str(HERE/'game.fgd'))
  options={**profile,'project_dir':str(args.output.resolve())};options.pop('directory',None)
  e.call('export',engine='godot',path=str((args.output/'maps').resolve()),name=name,scale=32,options=options)
  print('Exported',name,flush=True)
finally:e.close()
