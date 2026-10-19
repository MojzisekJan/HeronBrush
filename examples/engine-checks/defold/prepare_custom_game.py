#!/usr/bin/env python3
"""Prepare a disposable non-FPS game using its own FGD and component."""
import shutil
import json
import sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'assets'))
import prefabs_gen

def main():
    source=Path(__file__).resolve().parent/'custom-game'
    project=Path('/tmp/heron-defold-custom-game')
    shutil.copytree(source,project,dirs_exist_ok=True)
    profile=project/'.heron-brush/exports.json'
    profile.parent.mkdir(exist_ok=True)
    profile.write_text(json.dumps({'version':1,'targets':{'defold':{'project_dir':'.','directory':'main','entities':{
        'relic':{'defaults':{'defold_property_logic__charge':9,'defold_property_logic__armed':True,'defold_property_logic__offset':[1,2,3]}},
        'beacon':{'defaults':{'defold_go':'/main/beacon.go','defold_property_logic__charge':11,
            'defold_property_logic__destination':'/relic#logic','defold_property_type_logic__destination':'url',
            'defold_property_logic__orientation':[0,0,0,1],'defold_property_type_logic__orientation':'quat'}}
    }}}},indent=2))
    prefabs_gen.BIN=str(ROOT/'target/debug/heron-brush')
    editor=prefabs_gen.Editor()
    try:
        editor.call('new_map')
        editor.call('load_definitions',path=str(project/'game.fgd'))
        editor.call('add_shape',kind='cuboid',min=[-160,-160,-32],max=[160,160,0],material='__TB_empty')
        editor.call('add_shape',kind='cuboid',min=[-16,-16,112],max=[16,16,144],material='__TB_empty')
        editor.call('make_brush_entity',classname='relic')
        editor.call('add_point_entity',classname='beacon',origin=[96,128,64])
        editor.call('save',path=str(project/'workshop.map'))
        print(editor.call('export',engine='defold',path=str(project/'main'),name='workshop',scale=32,options={'project_dir':str(project)}))
    finally:
        editor.close()
    print('Prepared',project)

if __name__=='__main__':
    main()
