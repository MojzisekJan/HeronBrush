#!/usr/bin/env python3
"""Re-export the checked-in edited demo using the editor's real MCP commands.
Run from the repository root: python3 demos/bevy_fps/export_demo.py
"""
import json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'assets'))
from prefabs_gen import Editor
base=ROOT/'demos/bevy_fps'
ed=Editor()
try:
 print(ed.call('open_map',path=str(base/'maps/demo.map')))
 print(ed.call('load_definitions',path=str(base/'gamedata/fpskit.fgd')))
 print(ed.call('load_materials',path=str(base/'assets/textures')))
 print(ed.call('export',engine='bevy',path=str(base/'assets/maps'),name='demo',scale=32,
     options={'project_dir':str(base/'assets'),'texture_dir':'textures','material_dir':'__no_godot_materials__'}))
finally:ed.close()
