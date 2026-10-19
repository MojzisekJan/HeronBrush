#!/usr/bin/env python3
"""Fill an existing disposable Unity project with two independent map exports."""
import json
import shutil
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'assets'))
import prefabs_gen
def main():
    project=Path('/tmp/heron-unity-custom-game')
    assert (project/'ProjectSettings/ProjectVersion.txt').is_file(),'Create the disposable Unity project first'
    (project/'Assets/Editor').mkdir(parents=True,exist_ok=True)
    source=Path(__file__).resolve().parent/'custom-game'
    shutil.copyfile(source/'Puzzle.cs',project/'Assets/Puzzle.cs')
    shutil.copyfile(source/'VerifyCustom.cs',project/'Assets/Editor/VerifyCustom.cs')
    profile=project/'.heron-brush/exports.json';profile.parent.mkdir(exist_ok=True)
    profile.write_text(json.dumps({'version':1,'targets':{'unity':{'project_dir':'.','directory':'Assets/MapA','name_property':'targetname','entities':{
        'worldspawn':{'build':{'visuals':'false'}},
        'puzzle_panel':{'build':{'collision':'convex','visuals':'false'},'defaults':{'unity_prefab':'Assets/Puzzle.prefab','unity_body_type':'dynamic','unity_mass':2,
            'unity_property_Puzzle__charge':9,'unity_property_Puzzle__armed':True,'unity_property_Puzzle__sequence':[2,3,5],'unity_property_Puzzle__partner':'beacon'}},
        'beacon':{'defaults':{'unity_prefab':'Assets/Puzzle.prefab','charge':9,'armed':False}}
    }}}},indent=2))
    prefabs_gen.BIN=str(ROOT/'target/debug/heron-brush');editor=prefabs_gen.Editor()
    try:
        editor.call('new_map')
        editor.call('add_shape',kind='cuboid',min=[-160,-160,-32],max=[160,160,0],material='__TB_empty')
        editor.call('add_shape',kind='cuboid',min=[-16,-16,112],max=[16,16,144],material='__TB_empty')
        editor.call('make_brush_entity',classname='puzzle_panel')
        editor.call('add_point_entity',classname='beacon',origin=[96,128,64],properties={'targetname':'beacon','charge':'7'})
        editor.call('save',path=str(project/'workshop.map'))
        for directory in ['MapA','MapB']:
            print(editor.call('export',engine='unity',path=str(project/'Assets'/directory),name='workshop'))
    finally:editor.close()
if __name__=='__main__':main()
