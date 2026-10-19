#!/usr/bin/env python3
"""A non-FPS Godot project configured by a portable export profile."""
import json
import re
import shutil
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'assets'))
import prefabs_gen
def main():
    project=Path('/tmp/heron-godot-custom-game')
    shutil.copytree(Path(__file__).resolve().parent/'custom-game',project,dirs_exist_ok=True)
    profile=project/'.heron-brush/exports.json';profile.parent.mkdir(exist_ok=True)
    profile.write_text(json.dumps({'version':1,'targets':{'godot':{'project_dir':'.','directory':'maps','name_property':'targetname','entities':{
        'worldspawn':{'build':{'visuals':'false'}},
        'sensor_panel':{'build':{'node':'Area3D','collision':'convex','visuals':'false','groups':'interactions','connect':'body_entered:on_puzzle_enter'}},
        'beacon':{'build':{'scene':'res://beacon.tscn','auto':'true'},'defaults':{'charge':9,'armed':True,'sequence':[2,3,5]}}
    }}}},indent=2))
    prefabs_gen.BIN=str(ROOT/'target/debug/heron-brush');editor=prefabs_gen.Editor()
    try:
        editor.call('new_map')
        editor.call('add_shape',kind='cuboid',min=[-160,-160,-32],max=[160,160,0],material='__TB_empty')
        editor.call('add_shape',kind='cuboid',min=[-32,-32,0],max=[32,32,64],material='__TB_empty')
        panel=re.search(r'id (\d+)',editor.call('make_brush_entity',classname='sensor_panel'))[1]
        beacon=re.search(r'id (\d+)',editor.call('add_point_entity',classname='beacon',origin=[96,128,64],properties={'targetname':'beacon','charge':'7'}))[1]
        editor.call('link_entities',**{'from':panel,'to':beacon})
        editor.call('save',path=str(project/'workshop.map'))
        print(editor.call('export',engine='godot',path=str(project/'maps'),name='workshop'))
    finally:editor.close()
    print('Prepared',project)
if __name__=='__main__':main()
