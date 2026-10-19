"""Feature stations placed in the existing streets, square, quay and workshop."""
def add(g):
 def volume(cls,name,bounds,**props):
  old=g.TARGET;g.TARGET=[];g.box(*bounds,'clip');parts=g.TARGET;g.TARGET=old
  g.entity(dict(classname=cls,targetname=name,**props),parts)
 def solid(cls,name,bounds,mat='floor_wood_planks',**props):
  if cls in ('func_shuttle','func_platform'):
   bounds=(*bounds[:2],bounds[2]+.125,*bounds[3:])
  old=g.TARGET;g.TARGET=[];g.box(*bounds,mat);parts=g.TARGET;g.TARGET=old
  if cls not in ('fps_lift','func_shuttle','func_platform','func_physics'):
   import re
   def hide_bottom(line):
    points=re.findall(r'\( ([^)]*) \)',line)
    if points and all(float(p.split()[2])==bounds[2] for p in points):
     return line.replace(' '+g.M+mat+' ',' skip ',1)
    return line
   parts=['\n'.join(hide_bottom(line) for line in part.splitlines()) for part in parts]
  g.entity(dict(classname=cls,targetname=name,**props),parts)
 def sign(name,at,title,text):
  x,y,z=at;volume('fps_sign',name,(x-36,y-24,z,x+36,y+24,z+80),title=title,text=text,seconds='7',once='0')
 def point(cls,name,at,**props):g.entity(dict(classname=cls,targetname=name,origin=g.xyz(at),**props))
 # Spacious, bidirectional portals: one in the square, the other on the raised terrace.
 for name,target,at,color in [('portal_square','portal_terrace',(-990,-90,20),'70 190 255'),('portal_terrace','portal_square',(520,-186,68),'200 130 255')]:
  x,y,z=at;g.OFFSET=at
  outer=g.arc(0,80,38);inner=g.arc(0,80,30)
  for a,b,c,d in zip(outer,outer[1:],inner,inner[1:]):g.prism([a,b,d,c],-8,8,'wall_brick_sand_center')
  for xx in (-38,30):g.box(xx,-8,0,xx+8,8,80,'wall_brick_sand_center')
  g.box(-44,-14,-4,44,14,0,'floor_stone')
  g.OFFSET=(0,0,0)
  point('fps_portal',name,at,target=target,color=color,angles='0 0 0')
 sign('station_portals',(-930,-85,20),'01 / PORTALS','The blue portal links the market square to the upper terrace. Walk through, then return through the violet portal.')
 # An actual linked door, trigger and wall button, rather than unrelated props.
 volume('trigger_multiple','bakery_sensor',(-454,-56,28,-366,-16,108),target='door_-410',wait='1')
 solid('func_button','bakery_floor_button',(-454,-8,16,-426,12,20),'floor_tiles_blue_small',target='door_-410')
 solid('fps_use_button','workshop_use_button',(-10,-58,64,6,-52,82),'door_metal_gate_lock',target='door_28')
 point('fps_key','amber_key',(-910,90,48),key='amber',color='255 190 60')
 # The workshop door becomes a key-door; its travel, script and target name stay intact.
 g.ENTITIES[:]=[e.replace('"classname" "func_door"','"classname" "fps_key_door"').replace('"targetname" "door_28"','"key" "amber"\n"targetname" "door_28"') if '"targetname" "door_28"' in e else e for e in g.ENTITIES]
 volume('trigger_multiple','workshop_sensor',(-14,-52,28,70,-16,108),target='door_28')
 sign('station_doors',(-330,30,16),'02 / DOORS AND TARGETS','The bakery has a floor trigger. Find the amber key in the square, then use the workshop panel. Targets connect sensors, buttons and doors.')
 # Quay machinery: a lift, a continuous shuttle, and rigid-body cargo.
 solid('fps_lift','quay_lift',(64,172,-48,128,236,-40),travel='64',speed='48',wait='4',open_direction='0 0 1',script='res://scripts/mover.gd')
 volume('trigger_multiple','lift_sensor',(64,172,-40,128,236,32),target='quay_lift')
 solid('func_shuttle','cargo_shuttle',(250,168,-40,290,208,-30),direction='1 0 0',travel='96',speed='24',wait='1',start_on='1',script='res://scripts/shuttle.gd')
 solid('func_physics','cargo_crate',(306,174,-28,330,198,-4),'timber_square_planks',mass='3')
 solid('func_platform','platform_sample',(170,164,-40,202,196,-32),height='32',travel='32',speed='32',open_direction='0 0 1',script='res://scripts/mover.gd')
 volume('trigger_once','platform_sensor',(152,164,-40,214,214,24),target='platform_sample')
 # Separate ladder and launch-pad stations leave the ordinary stairs clear.
 for x in (-46,-18):g.box(x,150,-40,x+4,156,40,'floor_wood_planks_wide')
 for z in range(-32,41,10):g.box(-46,148,z,-14,154,z+3,'floor_wood_planks_wide')
 volume('fps_ladder','quay_ladder',(-50,144,-40,-10,174,44),speed='2.5')
 solid('fps_jump_pad','quay_jump',(376,168,-40,408,200,-36),'floor_tiles_blue_small',strength='10')
 sign('station_movement',(214,214,-40),'03 / MOVEMENT','Try the cargo shuttle, lift, ladder and jump pad. The ramp and stone stairs remain an ordinary route back up.')
 # The river is a nonblocking water volume in the FPS runtime.
 old=g.TARGET;g.TARGET=[];g.box(-1248,252,-96,848,960,-64,'floor_ground_water');water=g.TARGET;g.TARGET=old
 # Remove the original water brush so there is no solid floor at its surface.
 g.WORLD[:]=[b for b in g.WORLD if not ('( -1248 252 -96 )' in b and 'floor_ground_water' in b)]
 g.entity(dict(classname='fps_water',targetname='river',sink_speed='.8',swim_speed='3',rise_speed='3.5'),water)
 # Checkpoint and collectibles guide visitors through the square.
 volume('fps_checkpoint','square_checkpoint',(-820,100,20,-760,148,100))
 for i,(x,y) in enumerate([(-930,132),(-862,132),(-790,132)]):point('fps_pickup',f'market_token_{i}',(x,y,44),kind='market',value='1')
 point('fps_gate','market_gate',(-770,100,44),needs_count='3',needs_kind='market',target='door_-858',message='Three market tokens collected. The hall is open.')
 sign('station_progress',(-800,100,20),'04 / PROGRESS','This square is a checkpoint. Collect three market tokens to open the hall. The amber key is a separate door condition.')
 # A contained, optional combat/workshop station in the rear lane.
 point('fps_weapon_pickup','demo_weapon',(420,-412,44),weapon='blaster')
 point('fps_enemy','demo_flyer',(492,-418,106),angles='0 180 0')
 point('fps_walker','demo_walker',(548,-416,44))
 point('fps_spawner','demo_spawner',(616,-410,44),count='2',interval='2',spread='1')
 solid('fps_use_button','spawner_panel',(390,-462,64,410,-456,88),'door_metal_gate_lock',target='demo_spawner')
 point('fps_barrel','demo_barrel',(572,-440,46),health='30',damage='30',radius='2')
 volume('fps_hurt','hot_vent',(438,-468,20,462,-444,28),damage='5',message='Hot vent: damage volumes are kept off the main route.')
 g.box(436,-470,20,464,-442,24,'floor_stone_grate')
 point('fps_particles','vent_steam',(450,-455,30),color='220 230 240',amount='12',start_on='1')
 point('fps_smoke','chimney_smoke',(-322,-205,430),amount='12',start_on='1')
 point('fps_cloud','demo_cloud',(-600,-120,620))
 sign('station_combat',(416,-374,20),'05 / OPTIONAL WORKSHOP','Weapon pickup, enemy types, barrel, wave button and damage volume. This side lane keeps the main sightseeing route clear.')
 # Visible examples of collision modes and materials, on a small display bench.
 for x in (-748,-704,-660):g.box(x,-414,20,x+32,-382,48,'wall_stone')
 solid('func_detail_illusionary','walkthrough_panel',(-748,-408,48,-716,-402,88),'window_square_pane_lit')
 solid('func_wall','solid_panel',(-704,-408,48,-672,-402,88),'window_square_metal')
 volume('func_clip','invisible_rail',(-660,-408,48,-628,-402,88))
 # Gradient and PBR are editor-authored brush attributes, baked by exporters.
 for i,kind in enumerate(('multigradient','radial')):
  x=-748+i*64;g.box(x,-386,50,x+48,-378,94,'__TB_empty')
  token='multigradient' if kind=='multigradient' else 'radial'
  # The writer uses "linear" for a multi-stop linear gradient.
  token='linear' if token=='multigradient' else token
  metadata=f' // heron_brush {token} 35 3 0 30 110 180 0.5 70 220 190 1 245 205 90 pbr 35 40 15'
  g.WORLD[-1]='\n'.join(line+metadata if line.startswith('( ') else line for line in g.WORLD[-1].splitlines())
 sign('station_materials',(-706,-354,20),'06 / MATERIALS AND COLLISION','Compare solid, detail and invisible collision. The display includes multi-stop linear/radial gradients and PBR paint.')
 point('light_spot','gallery_spot',(-690,-354,154),angles='45 270 0',light_energy='2',spot_range='8')
 point('reflection_probe','water_reflection',(200,240,40),size='12 6 8')
 point('info_waypoint','tour_square',(-850,100,20))
 point('info_camera','tour_camera',(440,80,104),angles='10 190 0')
 point('info_player_deathmatch','alternate_spawn',(-780,100,46),angles='0 0 0')
 volume('fps_exit','tour_exit',(-986,-226,20,-930,-190,110),message='Lantern Lane tour complete. Try another engine export.')
 sign('station_start',(370,34,16),'LANTERN LANE / FEATURE TOUR','Follow the street to the market portal. The quay tests movement and sound; the rear lane contains materials, physics and an optional combat station.')
 # Generic built-ins not needing an interactive station: grouped/detail geometry.
 solid('func_detail','detail_trim',(-986,-238,20,-930,-234,24),'wall_wood_trimsheet')
 solid('func_geo','occluding_plinth',(-1010,-230,20,-994,-214,52),'wall_stone')
 solid('func_illusionary','decorative_panel',(-1012,-228,54,-1010,-216,70),'window_square_pane_lit')

 point('prop_static','model_display',(-660,-396,54),scene='res://models/grass-small.glb',scale='1')
 solid('func_group','grouped_sample',(-630,-414,20,-610,-394,36),'wall_stone')

 # The library sailboat rides beside the landing, with only a four-unit vertical travel.
 start=len(g.WORLD)
 g.asset_tree('sailboat',(270,354,-80),1.25)
 boat=g.WORLD[start:];del g.WORLD[start:]
 g.entity(dict(classname='func_shuttle',targetname='moored_sailboat',direction='0 0 1',travel='4',speed='1',wait='0',start_on='1',script='res://scripts/shuttle.gd'),boat)
