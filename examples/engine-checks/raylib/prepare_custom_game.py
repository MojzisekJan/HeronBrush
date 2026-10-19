#!/usr/bin/env python3
"""Export custom puzzle entities through the editor's project profile."""
import json
import re
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'assets'))
import prefabs_gen

def main():
    project=Path('/tmp/heron-raylib-custom-game');project.mkdir(exist_ok=True)
    profile=project/'.heron-brush/exports.json';profile.parent.mkdir(exist_ok=True)
    profile.write_text(json.dumps({'version':1,'targets':{'raylib':{'directory':'export','name_property':'targetname','entities':{
        'worldspawn':{'build':{'visuals':'false'}},
        'puzzle_panel':{'build':{'collision':'convex','visuals':'false'},'defaults':{'raylib_factory':'panel','armed':True,'sequence':[2,3,5]}},
        'beacon':{'defaults':{'charge':9,'raylib_factory':'beacon'}}
    }}}},indent=2))
    prefabs_gen.BIN=str(ROOT/'target/debug/heron-brush');editor=prefabs_gen.Editor()
    try:
        editor.call('new_map')
        editor.call('add_shape',kind='cuboid',min=[-160,-160,-32],max=[160,160,0],material='__TB_empty')
        editor.call('add_shape',kind='cuboid',min=[-32,-32,0],max=[32,32,64],material='__TB_empty')
        panel=re.search(r'id (\d+)',editor.call('make_brush_entity',classname='puzzle_panel'))[1]
        beacon=re.search(r'id (\d+)',editor.call('add_point_entity',classname='beacon',origin=[96,128,64],properties={'targetname':'beacon','charge':'7'}))[1]
        editor.call('link_entities',**{'from':panel,'to':beacon})
        editor.call('save',path=str(project/'workshop.map'))
        print(editor.call('export',engine='raylib',path=str(project/'export'),name='workshop'))
    finally:editor.close()
    print('Prepared',project)

if __name__=='__main__':main()
