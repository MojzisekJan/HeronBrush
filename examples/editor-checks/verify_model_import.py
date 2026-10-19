#!/usr/bin/env python3
"""Exercise native import dialogs and inspect real app snapshots and packaged assets."""
import argparse, json, os, subprocess, sys
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
parser=argparse.ArgumentParser();parser.add_argument('--root',type=Path,default=Path('/tmp/heron-native-model-check'));parser.add_argument('--multi',action='store_true');parser.add_argument('--case',action='append',choices=['import-obj','import-glb','model-rotation','model-scale']);args=parser.parse_args();root=args.root.resolve()
assets=root/'models';subprocess.run([sys.executable,str(repo/'examples/editor-checks/make_model_fixtures.py'),str(assets)]+(['--multi'] if args.multi else []),check=True)
originals={path:path.read_bytes() for path in assets.iterdir() if path.is_file()}
env={**os.environ,'HERON_BRUSH_IN_APP_FILES':'1'}
cases=[('import-obj','triangle.obj',None),('import-glb','triangle.glb',None),('model-rotation','triangle.obj','"angles" "0 90 0"'),('model-scale','triangle.obj','"modelscale" "2"')]
if args.case:cases=[case for case in cases if case[0] in args.case]
for case,asset,property_check in cases:
    output=root/case
    subprocess.run([sys.executable,str(repo/'examples/editor-checks/run_native.py'),str(repo/f'examples/editor-checks/native-{case}.script'),'--cwd',str(assets),'--shots',str(output)],env=env,check=True)
    for path,data in originals.items():assert path.read_bytes()==data,f'Source asset changed: {path.name}'
    snapshots={name:json.loads((output/f'{name}.json').read_text()) for name in ['base','changed','undo','redone']}
    assert snapshots['base']['map']==snapshots['undo']['map'],f'{case}: Undo changed other content'
    assert snapshots['changed']['map']==snapshots['redone']['map'],f'{case}: Redo did not restore the edit'
    assert snapshots['base']['imported_models']==snapshots['undo']['imported_models'],f'{case}: Undo left stale model data'
    assert snapshots['changed']['imported_models']==snapshots['redone']['imported_models'],f'{case}: Redo left stale model data'
    model=snapshots['changed']['imported_models'];assert len(model)==1
    assert model[0]['triangles']==(2 if args.multi else 1) and model[0]['skin_size']==[2,2],f'{case}: mesh or texture was not loaded'
    if args.multi:
        assert model[0]['skin_count']==2
        assert model[0]['smooth_triangles']==2
        assert model[0]['material_indices']==[0,1]
    if case=='model-scale':assert abs(model[0]['bounds'][1][0]-model[0]['bounds'][0][0]-128)<1e-6
    if case=='model-rotation':assert abs(model[0]['bounds'][0][0]+64)<1e-6
    changed=snapshots['changed']['map'];assert '"_heron_model" ".heron-brush/models/' in changed
    assert snapshots['changed']['selection']==1
    assert str(assets) not in changed,'Source folder leaked into a project model reference'
    packaged=list((output/'.heron-brush/models').glob(f'*/{asset}'));assert len(packaged)==1
    assert packaged[0].read_bytes()==(assets/asset).read_bytes()
    if asset.endswith('.obj'):
        for name in ['paint.mtl','texture.png']:assert (packaged[0].parent/name).read_bytes()==(assets/name).read_bytes()
    else:assert '"modelscale" "32"' in changed
    if property_check:
        assert property_check in changed
        reopened=json.loads((output/'reopened.json').read_text());assert property_check in reopened['map']
        assert reopened['map'].count('"_heron_model"')==1
        assert reopened['imported_models'][0]['triangles']==(2 if args.multi else 1)
        assert reopened['imported_models'][0]['skin_size']==[2,2]
        assert reopened['selection']==2,'Reloaded model could not be selected with the source brush'
    print(f'PASS: {case}: packaged assets, native preview, Undo/Redo'+(', save/reload' if property_check else ''))
print(f'PASS: {len(cases)} native model import workflows. Inspect PNGs in',root)
