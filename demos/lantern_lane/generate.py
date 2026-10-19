#!/usr/bin/env python3
"""Rebuild the demo geometry. Python standard library only; textures stay untouched."""
from pathlib import Path
import itertools, math, json, wave, struct, random, re
ROOT=Path(__file__).resolve().parent
WORLD=[]; ENTITIES=[]; TARGET=WORLD; OFFSET=(0,0,0); TURN=1;FRAME=0
M='kenney_fantasy/'
def sub(a,b):return tuple(x-y for x,y in zip(a,b))
def dot(a,b):return sum(x*y for x,y in zip(a,b))
def cross(a,b):return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
def fmt(v):return str(round(v,6)).rstrip('0').rstrip('.') if v%1 else str(int(v))
def xyz(p):return ' '.join(fmt(v) for v in p)
def transform(p):
 x,y,z=OFFSET[0]+TURN*p[0],OFFSET[1]+TURN*p[1],OFFSET[2]+p[2]
 for _ in range(FRAME):x,y=-y,x
 return (x,y,z)
def hull(points,mat,fit=None,align=None):
 points=[transform(tuple(round(v*8)/8 for v in p)) for p in points]
 planes={}
 for a,b,c in itertools.combinations(points,3):
  n=cross(sub(b,a),sub(c,a)); length=math.sqrt(dot(n,n))
  if length<1e-8:continue
  n=tuple(v/length for v in n);d=dot(n,a);dist=[dot(n,p)-d for p in points]
  if min(dist)<-1e-6 and max(dist)>1e-6:continue
  if max(dist)>1e-6:n=tuple(-v for v in n);d=-d;b,c=c,b
  key=tuple(round(v,6) for v in (*n,d))
  if key in planes:continue
  # Map winding is clockwise; UV projection follows the dominant normal.
  axis=max(range(3),key=lambda i:abs(n[i]))
  u,v=([(0,1,0),(0,0,-1)] if axis==0 else [(1,0,0),(0,0,-1)] if axis==1 else [(1,0,0),(0,-1,0)])
  su=sv=1.;ou=ov=0.
  if fit and axis==(0 if FRAME%2 else 1):
   tw,th=fit;us=[dot(u,p) for p in points];vs=[dot(v,p) for p in points]
   su=(max(us)-min(us))/tw;sv=(max(vs)-min(vs))/th;ou=-min(us)/su;ov=-min(vs)/sv
  if align:
   # Start the texture at the brush's top corner, so courses run whole from the cap down.
   tw,th=align;ou=-min(dot(u,p) for p in points)%tw;ov=-min(dot(v,p) for p in points)%th
  planes[key]=f'( {xyz(a)} ) ( {xyz(c)} ) ( {xyz(b)} ) {mat if mat in ("origin","skip","clip","__TB_empty") else M+mat} [ {xyz(u)} {fmt(ou)} ] [ {xyz(v)} {fmt(ov)} ] 0 {fmt(su)} {fmt(sv)}'
 assert len(planes)>=4
 TARGET.append('{\n'+'\n'.join(planes.values())+'\n}')
def box(x0,y0,z0,x1,y1,z1,mat,fit=None,align=None):hull(list(itertools.product((x0,x1),(y0,y1),(z0,z1))),mat,fit,align)
def prism(poly,y0,y1,mat,fit=None):hull([(x,y,z) for x,z in poly for y in (y0,y1)],mat,fit)
def entity(props,brushes=()):ENTITIES.append('{\n'+'\n'.join(json.dumps(k)+' '+json.dumps(str(v)) for k,v in props.items())+'\n'+'\n'.join(brushes)+'\n}')
def arc(cx,z,r):
 # Mirror the same rounded coordinates rather than rounding each side separately.
 return [(round((cx+r*math.cos(i*math.pi/8))*8)/8,round((z+r*math.sin(i*math.pi/8))*8)/8) for i in range(9)]
def arch_window(cx,y,bottom,spring,r,top,mat='wall_timber',lit=True):
 """A hollow arched opening; the surrounding wall shares exact snapped endpoints."""
 outer=arc(cx,spring,r+4);inner=arc(cx,spring,r)
 for a,b,c,d in zip(outer,outer[1:],inner,inner[1:]):prism([a,b,d,c],y-4,y+12,'floor_wood_planks_wide')
 for side in (-1,1):
  a,b=sorted((cx+side*r,cx+side*(r+4)));box(a,y-4,bottom,b,y+12,spring,'floor_wood_planks_wide')
 for a,b in zip(outer,outer[1:]):prism([a,b,(b[0],top),(a[0],top)],y,y+12,mat)
 box(cx-r-8,y-6,bottom-5,cx+r+8,y+14,bottom,'wall_brick_sand_center')
 prism([(cx-r,bottom),(cx+r,bottom)]+inner,y+7,y+9,'window_tall_rounded_lit' if lit else 'window_tall_rounded',(64,128))

def roof_x(x0,x1,y0,y1,eave,rise,texture):
 mid=(y0+y1)/2
 for profile in [[(y0,eave),(mid,eave+rise),(mid,eave+rise+8),(y0,eave+8)],[(mid,eave+rise),(y1,eave),(y1,eave+8),(mid,eave+rise+8)]]:
  hull([(x,y,z) for x in (x0,x1) for y,z in profile],texture)
 box(x0-2,mid-4,eave+rise+5,x1+2,mid+4,eave+rise+12,'floor_wood_planks_wide')
 # Triangular gable infill at the two ends.
 for x in (x0+12,x1-24):hull([(xx,y,z) for xx in (x,x+12) for y,z in [(y0+12,eave),(mid,eave+rise-8),(y1-12,eave)]],'wall_timber_structure_cross')

def townhouse(cx,front,width,depth,base,height,roof,upper=True,door=False):
 global OFFSET,TURN,TARGET
 OFFSET=(cx,front,base);TURN=-1
 w=width/2;stone='wall_brick_stone_center' if roof=='roof_clay_grey_center' else 'wall_brick_sand_center'
 # Ground floor masonry, with a true 56-unit doorway and a recessed leaf.
 box(-w,0,0,-28,12,128,stone);box(28,0,0,w,12,128,stone);box(-28,0,108,28,12,128,stone)
 for x in (-w,w-12):box(x,12,0,x+12,depth,height,stone)
 box(-w+12,depth-12,0,w-12,depth,height,stone)
 box(-w,0,-8,w,depth,0,'floor_stone');box(-w+12,12,124,w-12,depth-12,128,'floor_wood_planks')
 if door:
  saved=TARGET;TARGET=[];box(-27,4,1,27,10,107,'door_wood_window_lit',(64,128));leaf=TARGET;TARGET=saved
  entity({'classname':'func_door','targetname':f'door_{int(cx)}','origin':xyz(transform((0,7,54))),'open_direction':'-1 0 0','travel':'58','speed':'48','wait':'3','script':'res://scripts/mover.gd'},leaf)
 else:box(-27,4,1,27,10,107,'door_wood_handle',(64,128))
 for x in (-34,28):box(x,-4,0,x+6,12,114,'wall_wood_trimsheet')
 box(-34,-4,108,34,12,116,'wall_wood_trimsheet');box(-40,-16,-4,40,8,0,'floor_stone')
 box(-w-4,-5,124,w+4,depth+4,134,'wall_wood_trimsheet')
 if upper:
  # Narrow upper bays each have one centered rounded window and continuous masonry around it.
  bays=3 if width>=220 else 2;bw=width/bays
  for i in range(bays):
   mid=-w+bw*(i+.5);r=16;bottom=158;spring=height-45
   box(mid-bw/2,0,134,mid-r-4,12,height,'wall_timber')
   box(mid+r+4,0,134,mid+bw/2,12,height,'wall_timber')
   box(mid-r-4,0,134,mid+r+4,12,bottom,'wall_timber')
   arch_window(mid,0,bottom,spring,r,height,'wall_timber',i%2==0)
  for i in range(bays+1):
   x=-w+i*bw;box(x-3,-5,132,x+3,1,height,'floor_wood_planks_wide')
  for z in (138,height-8):box(-w-5,-6,z,w+5,0,z+6,'floor_wood_planks_wide')
 else:
  box(-w,0,134,w,12,height,stone)
 # A sloping front roof, with a small projecting dormer.
 roof_x(-w-12,w+12,-18,depth+14,height,100,roof)
 dy=30;dz=height+22
 # The main roof's front face rises from (y0, eave+8) to (mid, eave+rise+8); the dormer
 # ends where its parts reach it, so nothing is left standing behind it.
 y0,mid=-18,(depth-4)/2
 back=lambda z:min(mid,max(dy-2,y0+(z-height-8)*(mid-y0)/100))
 dormer=lambda poly,y,mat:hull([(x,y,z) for x,z in poly]+[(x,back(z),z) for x,z in poly],mat)
 # A gabled front under the dormer roof, and cheeks that close its sides.
 dormer([(-25,dz),(25,dz),(25,dz+60.125),(0,dz+82),(-25,dz+60.125)],dy-2,'wall_timber_structure')
 box(-18,dy-4,dz+8,18,dy-2,dz+48,'window_tall_pane_lit',(64,128))
 dormer([(-32,dz+54),(0,dz+82),(0,dz+88),(-32,dz+60)],dy-8,roof)
 dormer([(0,dz+82),(32,dz+54),(32,dz+60),(0,dz+88)],dy-8,roof)
 # Chimney in small courses of the house's brick, aligned to the stack's corner.
 box(w-42,depth-66,height+55,w-18,depth-38,height+138,stone.replace('wall_brick_','wall_brick_small_').replace('_center',''),align=(64,128))
 box(w-46,depth-70,height+134,w-14,depth-34,height+142,'wall_stone')
 OFFSET=(0,0,0);TURN=1

def edge_house(x,y,quarter,width,height,texture):
 global FRAME
 FRAME=quarter
 cx,front=x,y
 for _ in range(quarter):cx,front=front,-cx
 townhouse(cx,front,width,144,20,height,texture,True)
 FRAME=0

def stairs(x0,x1,y0,count,run,rise,z0):
 bottom=min(z0,z0+count*rise)-8
 for i in range(count):box(x0,y0+i*run,bottom,x1,y0+(i+1)*run,z0+(i+1)*rise,'floor_stone')

def crate(x,y,z,size=28):
 box(x,y,z,x+size,y+size,z+size,'timber_square_planks',(64,64))
 for yy in (y-1,y+size):
  for xx in (x,x+size-3):box(xx,yy,z,xx+3,yy+1,z+size,'floor_wood_planks_wide')

def asset_tree(name,origin,scale):
 lines=(ROOT.parents[1]/'assets'/f'{name}.map').read_text().splitlines();color='';i=0
 while i<len(lines):
  if lines[i].startswith('// heron_brush color '):color=lines[i]
  if lines[i]=='{' and i+1<len(lines) and lines[i+1].startswith('( '):
   points=set();brush_color=color;i+=1
   while i<len(lines) and lines[i]!='}':
    tint=re.search(r'// heron_brush (?:color|gradient) (\d+ \d+ \d+)',lines[i])
    if tint:brush_color='// heron_brush color '+tint.group(1)
    for m in re.finditer(r'\( ([^)]*) \)',lines[i]):
     values=[float(v) for v in m.group(1).split()]
     points.add(tuple(round((v*scale+o)*8)/8 for v,o in zip(values,origin)))
    i+=1
   hull(sorted(points),'__TB_empty')
   WORLD[-1]=brush_color+'\n'+WORLD[-1]

  i+=1

def build():
 global TARGET,OFFSET,TURN
 WORLD.clear();ENTITIES.clear();TARGET=WORLD;OFFSET=(0,0,0);TURN=1
 # Four elevation levels: canal -64, quay -40, street 16, raised terrace 64.
 box(-1100,-720,-112,704,432,-80,'wall_rock')
 box(-576,148,-80,576,252,-40,'wall_brick_stone_center')
 box(-576,148,-44,576,248,-40,'floor_stone_pattern')
 box(-576,-352,-80,576,80,8,'wall_brick_stone_center')
 box(-576,-16,8,576,72,16,'floor_ground_dirt')
 # Pavements with slab joints, proper kerbs, and an open stairway to the quay.
 box(-576,-70,8,576,-16,24,'wall_stone')
 box(-576,-70,24,576,-16,28,'floor_stone')
 for a,b in [(-576,-280),(-184,480),(560,576)]:
  box(a,72,-40,b,148,20,'wall_brick_stone_center');box(a,72,20,b,148,24,'floor_stone')
  box(a,140,24,b,150,30,'floor_stone_trimsheet')
 stairs(-280,-184,72,8,9.5,-8,24)
 # Dense frontage. Varied heights and setbacks, no isolated houses on a lawn.
 townhouse(-410,-68,216,202,28,264,'roof_clay_red_center',True,True)
 townhouse(-194,-76,216,190,28,232,'roof_thatch_center',True)
 townhouse(28,-68,228,216,28,272,'roof_thatch_center',True,True)
 townhouse(238,-76,192,206,28,156,'roof_clay_grey_center',False,True)
 # Right staircase climbs to a second street and a taller house behind the first row.
 stairs(364,420,-164,6,16,-8,64)
 box(348,-256,-20,576,-164,64,'wall_brick_stone_center');box(348,-256,64,576,-164,68,'floor_stone')
 townhouse(474,-236,172,104,68,224,'roof_clay_grey_center',True)
 # Left terrace and retaining walls, visually ties the foreground to the street.
 box(-576,36,-40,-444,136,64,'wall_brick_stone_center');box(-580,32,64,-440,140,70,'floor_stone')
 box(-576,36,70,-564,140,100,'wall_brick_stone_center')
 for x in (-552,-508):crate(x,65,70,30)
 crate(-306,-64,28);crate(-334,-62,28,24);crate(300,-60,28,26)
 crate(-316,200,-40,28)
 # A 1:2.5 ramp reaches the quay beside the stairs without a jump.
 hull([(x,y,z) for x in (480,560) for y,z in [(72,-48),(232,-48),(232,-40),(72,24)]],'floor_stone_pattern')
 # Trees reused from the editor library, planted clear of the route.
 for x,y,z in [(-534,110,70),(-440,206,-40),(426,218,-40)]:
  box(x-22,y-22,z,x+22,y+22,z+10,'wall_brick_stone_center')
  box(x-18,y-18,z+10,x+18,y+18,z+12,'floor_ground_dirt')
 asset_tree('tree_elm',(-534,110,82),.7)
 asset_tree('tree_cypress',(-440,206,-28),.9)
 asset_tree('tree_cypress',(426,218,-28),.9)
 # Recessed culvert in the canal wall, visible from the lower quay.
 box(90,246,-72,150,252,-44,'wall_stone_depth')
 for x in range(94,150,8):box(x,251,-72,x+3,254,-44,'door_metal_gate')
 # Quay bollards, timber landing and mooring posts.
 for x in (-390,-80,190,450):box(x,236,-40,x+10,246,-12,'wall_stone')
 box(180,248,-55,360,304,-48,'floor_wood_planks')
 for x in (186,346):box(x,280,-80,x+8,288,-32,'floor_wood_planks_wide')
 # The western lane opens into a small square, then loops behind the houses.
 box(-1088,-500,-80,-576,248,16,'wall_brick_stone_center')
 box(-1088,-500,16,-576,248,20,'floor_stone_pattern_small')
 box(-704,-700,-80,688,-340,16,'wall_brick_stone_center')
 box(-704,-472,16,688,-340,20,'floor_stone_pattern_small')
 box(576,-472,-80,688,248,16,'wall_brick_stone_center')
 box(576,-472,16,688,248,20,'floor_stone_pattern_small')
 # A sloping side alley joins the east terrace to the rear street.
 hull([(x,y,z) for x in (580,672) for y,z in [(-340,12),(-180,12),(-180,64),(-340,20)]],'floor_stone')
 townhouse(-858,-250,260,208,20,264,'roof_clay_red_center',True,True)
 townhouse(-284,-480,248,204,20,212,'roof_clay_grey_center',True)
 townhouse(-30,-480,260,196,20,244,'roof_thatch_center',True)
 townhouse(220,-480,240,206,20,208,'roof_clay_red_center',True)
 # A stone arch marks the transition from quay street to the square.
 for ya,yb in [(-90,-48),(48,90)]:box(-628,ya,16,-606,yb,196,'wall_brick_stone_center')
 curve=arc(0,108,48)
 for a,b in zip(curve,curve[1:]):hull([(x,y,z) for x in (-628,-606) for y,z in [a,b,(b[0],196),(a[0],196)]],'wall_brick_stone_center')
 box(-634,-96,196,-600,96,204,'floor_stone')
 # Low octagonal fountain at the center of the square, with shared mitred joints.
 for i in range(8):
  a=i*math.tau/8;b=(i+1)*math.tau/8
  hull([(-850+r*math.cos(t),-26+r*math.sin(t),z) for r,t in [(58,a),(58,b),(46,b),(46,a)] for z in (20,44)],'wall_brick_sand_center')
 hull([(-850+45*math.cos(i*math.tau/8),-26+45*math.sin(i*math.tau/8),z) for i in range(8) for z in (26,30)],'floor_ground_water')
 box(-858,-34,30,-842,-18,82,'wall_stone');box(-866,-42,78,-834,-10,86,'floor_stone')
 for x,y in [(-1020,110),(-1030,-200),(-732,176)]:
  box(x-24,y-24,20,x+24,y+24,30,'wall_brick_sand_center')
  asset_tree('tree_elm',(x,y,30),.55)
 # Covered market stalls and benches give the square a usable perimeter.
 for x,y in [(-1000,-380),(-742,-200)]:
  for dx in (-38,38):box(x+dx,y,20,x+dx+4,y+4,108,'floor_wood_planks_wide')
  roof_x(x-46,x+46,y-20,y+44,102,24,'roof_thatch_center')
  box(x-42,y+8,20,x+42,y+30,54,'floor_wood_planks')
  crate(x-34,y+10,54,18);crate(x+2,y+10,54,22)
 for x,y in [(-960,168),(-780,168)]:
  box(x-34,y-8,40,x+34,y+8,44,'floor_wood_planks')
  for dx in (-28,24):box(x+dx,y-6,20,x+dx+4,y+6,40,'wall_stone')
 # Buildings, not a perimeter wall, enclose the inland sides of the quarter.
 roofs=['roof_clay_red_center','roof_thatch_center','roof_clay_grey_center']
 for i,y in enumerate((-592,-368,-144,80)):
  edge_house(-1088,y,3,224,208+(i%3)*24,roofs[i%3])
  edge_house(688,y,1,224,192+(i%3)*24,roofs[(i+1)%3])
 for i,(x,width) in enumerate([(-972,232),(-744,224),(-520,224),(456,232),(630,116)]):
  townhouse(x,-480,width,192,20,192+(i%3)*24,roofs[i%3],True)
 # A broad river replaces the wall and fence in front of the quay.
 box(-1248,252,-96,848,960,-64,'floor_ground_water')
 box(-1248,-720,-112,-1088,252,12,'wall_rock')
 box(688,-720,-112,848,252,12,'wall_rock')
 box(-1088,248,-80,-576,252,20,'wall_brick_stone_center')
 # An animated sign/rotor above the workshop, clear of the walking corridor.
 saved=TARGET;TARGET=[];OFFSET=(-410,-47,204)
 for i in range(6):
  a=i*math.tau/6;b=a+.20
  prism([(r*math.cos(t),r*math.sin(t)) for r,t in [(8,a),(30,a),(30,b),(8,b)]],-2,2,'floor_wood_planks')
 box(-2,-2,-2,2,2,2,'origin');rotor=TARGET;TARGET=saved;OFFSET=(0,0,0)
 entity({'classname':'func_rotate','targetname':'workshop_sign','axis':'0 1 0','speed':'12','start_on':'1','script':'res://scripts/spinner.gd'},rotor)
 entity({'classname':'info_player_start','origin':'440 34 42','angles':'0 190 0'})
 entity({'classname':'ambient_sound','targetname':'canal_water','origin':'100 280 -40','stream':'res://sounds/well.wav','autoplay':'1','volume_db':'-12','unit_size':'2','max_distance':'12'})
 for x,y,z in [(-410,-28,130),(34,-26,130),(250,-30,126)]:entity({'classname':'light','origin':f'{x} {y} {z}','light_energy':'2','omni_range':'6','_color':'1 0.68 0.34'})
 entity({'classname':'light_sun','angles':'-45 35 0','light_energy':'1.4'})
 import showcase,sys
 showcase.add(sys.modules[__name__])
 header='// Game: heron_brush\n// Format: Valve\n{\n"classname" "worldspawn"\n"mapversion" "220"\n"_tb_textures" "textures"\n"message" "Lantern Lane - canal quarter"\n'
 (ROOT/'lantern_lane.map').write_text(header+'\n'.join(WORLD)+'\n}\n'+'\n'.join(ENTITIES)+'\n')
 print(f'{len(WORLD)} world brushes; {len(ENTITIES)} entities')
if __name__=='__main__':build()
