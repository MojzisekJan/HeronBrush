#!/usr/bin/env python3
"""Measure fresh MCP exports and one-brush edits; never save the source maps."""
import json,sys,time,statistics,shutil,argparse
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'assets'))
import prefabs_gen
parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,default=ROOT/'target/release/heron-brush');parser.add_argument('--static-cells',action='store_true');args=parser.parse_args()
prefabs_gen.BIN=str(args.binary.resolve())
result=[]
for name,source,fgd in [('motion_lab',ROOT/'demos/motion_lab/motion_lab.map',ROOT/'demos/motion_lab/motion_lab.fgd'),('piast_tower',ROOT/'maps/piast_tower_arch_merged.map',None),('lantern_lane',ROOT/'demos/lantern_lane/lantern_lane.map',ROOT/'demos/lantern_lane/lantern_lane.fgd')]:
 editor=prefabs_gen.Editor();out=Path('/tmp/heron-web-bench')/name;out.mkdir(parents=True,exist_ok=True)
 try:
  editor.call('open_map',path=str(source))
  if fgd:editor.call('load_definitions',path=str(fgd))
  if (source.parent/'textures').is_dir(): shutil.copytree(source.parent/'textures',out/'textures',dirs_exist_ok=True)
  export_args=dict(engine='three',path=str(out),name=name,scale=32,options={'project_dir':str(out),'texture_dir':'textures','static_cells':args.static_cells})
  timings=[]
  for _ in range(3):
   start=time.perf_counter();editor.call('export',**export_args);timings.append((time.perf_counter()-start)*1000)
  before=(out/f'{name}.html').read_bytes();(out/'before.html').write_bytes(before)
  brushes=json.loads(editor.call('list_brushes'));first=brushes[0]['id']
  editor.call('select',ids=[first]);editor.call('translate_selection',delta=[16,0,0])
  start=time.perf_counter();editor.call('export',**export_args);edit_ms=(time.perf_counter()-start)*1000
  (out/'after.html').write_bytes((out/f'{name}.html').read_bytes())
  row={'name':name,'brushes':len(brushes),'html_bytes':len(before),'export_median_ms':statistics.median(timings),'edited_export_ms':edit_ms,'directory':str(out)};result.append(row);print(json.dumps(row),flush=True)
 finally:editor.close()
Path('/tmp/heron-web-bench/results.json').write_text(json.dumps(result,indent=2)+'\n')
