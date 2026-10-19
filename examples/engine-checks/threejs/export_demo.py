#!/usr/bin/env python3
"""Export the common editor demo. Run from the repository root."""
import os, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
os.chdir(ROOT)
sys.path.insert(0,str(ROOT/'assets'))
from prefabs_gen import Editor
base=Path(__file__).resolve().parent
ed=Editor()
try:
 print(ed.call('open_map',path=str(ROOT/'demos/bevy_fps/maps/demo.map')))
 print(ed.call('load_definitions',path=str(ROOT/'demos/bevy_fps/gamedata/fpskit.fgd')))
 print(ed.call('load_materials',path=str(base/'assets/textures')))
 print(ed.call('export',engine='three',path=str(base/'assets'),name='demo',scale=32,
  options={'project_dir':str(base/'assets'),'texture_dir':'textures','material_dir':'__no_godot_materials__'}))
finally: ed.close()
