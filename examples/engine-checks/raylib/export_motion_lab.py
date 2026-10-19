#!/usr/bin/env python3
"""Export demos/motion_lab for the animation check (`make anim-check`). Run from the repository root."""
import os, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
os.chdir(ROOT)
sys.path.insert(0,str(ROOT/'assets'))
from prefabs_gen import Editor
base=Path(__file__).resolve().parent
ed=Editor()
try:
 print(ed.call('open_map',path=str(ROOT/'demos/motion_lab/motion_lab.map')))
 print(ed.call('load_definitions',path=str(ROOT/'demos/motion_lab/motion_lab.fgd')))
 print(ed.call('export',engine='raylib',path=str(base/'assets/maps'),name='motion_lab',scale=32))
finally: ed.close()
