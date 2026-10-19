#!/usr/bin/env python3
"""Export one custom map as portable glTF, GLB and OBJ through real MCP."""
import json
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'assets'))
import prefabs_gen
project=Path('/tmp/heron-interchange-custom');project.mkdir(exist_ok=True)
profile=project/'.heron-brush/exports.json';profile.parent.mkdir(exist_ok=True)
profile.write_text(json.dumps({'version':1,'targets':{engine:{'directory':engine,'entities':{'beacon':{'defaults':{'charge':9,'armed':True,'sequence':[2,3,5]}}}} for engine in ['gltf','glb','obj']}}))
prefabs_gen.BIN=str(ROOT/'target/debug/heron-brush');editor=prefabs_gen.Editor()
try:
    editor.call('new_map')
    editor.call('add_shape',kind='cuboid',min=[-160,-160,-32],max=[160,160,0],material='__TB_empty')
    editor.call('add_point_entity',classname='beacon',origin=[96,128,64],properties={'targetname':'beacon','charge':'7'})
    editor.call('save',path=str(project/'workshop.map'))
    for engine in ['gltf','glb','obj']:print(editor.call('export',engine=engine,path=str(project/engine),name='workshop'))
finally:editor.close()
