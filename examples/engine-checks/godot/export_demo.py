#!/usr/bin/env python3
from pathlib import Path
import sys, os, shutil
ROOT=Path(__file__).resolve().parents[3]
os.chdir(ROOT)
sys.path.insert(0,str(ROOT/'assets'))
from prefabs_gen import Editor
base=Path(__file__).resolve().parent
shutil.copy2(ROOT/'demos/bevy_fps/maps/demo.map', base/'maps/demo.map')
ed=Editor()
try:
 print(ed.call('open_map',path=str(ROOT/'demos/bevy_fps/maps/demo.map')))
 print(ed.call('load_definitions',path=str(ROOT/'demos/bevy_fps/gamedata/fpskit.fgd')))
 print(ed.call('load_materials',path=str(base/'textures')))
 print(ed.call('export',engine='godot',path=str(base/'maps'),name='demo',scale=32,options={'project_dir':str(base),'texture_dir':'res://textures','material_dir':'res://materials'}))
finally: ed.close()
