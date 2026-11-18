#!/usr/bin/env python3
"""Export custom puzzle entities through the editor's project profile."""
import argparse
import json
import re
import sys
import tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from export_feature_lab import Editor

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/debug/heron-brush')
    parser.add_argument('--project', type=Path, default=Path('/tmp/heron-bevy-custom-game'))
    args = parser.parse_args()
    project=args.project.resolve();project.mkdir(parents=True, exist_ok=True)
    profile=project/'.heron-brush/exports.json';profile.parent.mkdir(exist_ok=True)
    profile.write_text(json.dumps({'version':1,'targets':{'bevy':{'directory':'export','name_property':'targetname','entities':{
        'worldspawn':{'build':{'visuals':'false'}},
        'puzzle_panel':{'build':{'collision':'convex','visuals':'false'},'defaults':{'bevy_factory':'panel','physics_body':'dynamic','armed':True,'sequence':[2,3,5]}},
        'beacon':{'defaults':{'charge':9,'bevy_factory':'beacon'}}
    }}}},indent=2))
    with tempfile.TemporaryDirectory(prefix='heron-bevy-config-') as config:
        editor = Editor(args.binary.resolve(), Path(config))
        try:
            editor.call('new_map')
            editor.call('add_shape',kind='cuboid',min=[-160,-160,-32],max=[160,160,0],material='__TB_empty')
            editor.call('add_shape',kind='cuboid',min=[-16,-16,112],max=[16,16,144],material='__TB_empty')
            panel=re.search(r'id (\d+)',editor.call('make_brush_entity',classname='puzzle_panel'))[1]
            beacon=re.search(r'id (\d+)',editor.call('add_point_entity',classname='beacon',origin=[96,128,64],properties={'targetname':'beacon','charge':'7'}))[1]
            editor.call('link_entities',**{'from':panel,'to':beacon})
            editor.call('save',path=str(project/'workshop.map'))
            print(editor.call('export',engine='bevy',path=str(project/'export'),name='workshop'))
        finally:editor.close()
    print('Prepared',project)

if __name__=='__main__':main()
