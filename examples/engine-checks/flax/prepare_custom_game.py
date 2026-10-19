#!/usr/bin/env python3
"""Build a disposable non-FPS Flax project with two map exports."""
import json
import shutil
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'assets'))
import prefabs_gen
def main():
    project=Path('/tmp/heron-flax-custom-game');(project/'Source/Game').mkdir(parents=True,exist_ok=True);(project/'Content').mkdir(exist_ok=True)
    for name in ['GameTarget.Build.cs','GameEditorTarget.Build.cs']:shutil.copyfile(HERE/'Source'/name,project/'Source'/name)
    shutil.copyfile(HERE/'Source/Game/Game.Build.cs',project/'Source/Game/Game.Build.cs')
    shutil.copyfile(HERE/'flax.flaxproj',project/'custom.flaxproj')
    shutil.copyfile(HERE/'scene-template.json',project/'Content/ExportCheck.scene')
    shutil.copyfile(HERE/'Content/GameSettings.json',project/'Content/GameSettings.json')
    for name in ['Puzzle.cs','VerifyCustom.cs']:shutil.copyfile(HERE/'custom-game'/name,project/'Source/Game'/name)
    profile=project/'.heron-brush/exports.json';profile.parent.mkdir(exist_ok=True)
    profile.write_text(json.dumps({'version':1,'targets':{'flax':{'project_dir':'.','directory':'Content/MapA','name_property':'targetname','entities':{
        'worldspawn':{'build':{'visuals':'false'}},
        'puzzle_panel':{'build':{'collision':'convex','visuals':'false'},'defaults':{'flax_prefab':'Relic.prefab','flax_body_type':'dynamic','flax_mass':2,
            'flax_property_Puzzle__charge':9,'flax_property_Puzzle__armed':True,'flax_property_Puzzle__sequence':[2,3,5],'flax_property_Puzzle__partner':'beacon'}},
        'beacon':{'defaults':{'flax_prefab':'Beacon.prefab','charge':9,'armed':False}}
    }}}},indent=2))
    prefabs_gen.BIN=str(ROOT/'target/debug/heron-brush');editor=prefabs_gen.Editor()
    try:
        editor.call('new_map');editor.call('add_shape',kind='cuboid',min=[-160,-160,-32],max=[160,160,0],material='__TB_empty')
        editor.call('add_shape',kind='cuboid',min=[-16,-16,112],max=[16,16,144],material='__TB_empty');editor.call('make_brush_entity',classname='puzzle_panel')
        editor.call('add_point_entity',classname='beacon',origin=[96,128,64],properties={'targetname':'beacon','charge':'7'});editor.call('save',path=str(project/'workshop.map'))
        for directory in ['MapA','MapB']:
            output=project/'Content'/directory
            print(editor.call('export',engine='flax',path=str(output),name='workshop',options={'content_dir':directory+'/workshop'}))
            destination=project/'Source/Game'/directory;destination.mkdir(exist_ok=True)
            for source in output.glob('*.cs'):shutil.copyfile(source,destination/source.name)
    finally:editor.close()
if __name__=='__main__':main()
