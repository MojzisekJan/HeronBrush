#!/usr/bin/env python3
"""Export this demo through the editor's real MCP API, without modifying the map."""
import argparse,json,subprocess,shutil
from pathlib import Path
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
RUNTIME=HERE/'godot'
GENERATED=lambda directory,names:[n for n in names if Path(directory)==RUNTIME and n in ('maps','export-report.json','.godot')]
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,default=ROOT/'target/release/heron-brush');parser.add_argument('--output',type=Path,default=HERE);parser.add_argument('--engines',nargs='+',default=['three.js','godot','glb']);args=parser.parse_args()
 proc=subprocess.Popen([str(args.binary),'--mcp'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
 counter=0
 def rpc(method,params):
  nonlocal counter
  counter+=1;proc.stdin.write(json.dumps(dict(jsonrpc='2.0',id=counter,method=method,params=params))+'\n');proc.stdin.flush()
  while True:
   line=proc.stdout.readline()
   if not line:raise RuntimeError('Editor closed unexpectedly')
   result=json.loads(line)
   if result.get('id')==counter:
    if 'error' in result or result.get('result',{}).get('isError'):raise RuntimeError(result)
    return result.get('result',{})
 def call(tool,**arguments):return rpc('tools/call',dict(name=tool,arguments=arguments))
 try:
  rpc('initialize',{});print(call('open_map',path=str(HERE/'lantern_lane.map')));print(call('load_definitions',path=str(HERE/'lantern_lane.fgd')))
  for engine in args.engines:
   output=(args.output/engine.replace('.','')).resolve();output.mkdir(parents=True,exist_ok=True)
   # The Godot project in godot/ is the runtime template: an export elsewhere gets a copy of
   # it without the generated map and report; an export into it keeps it as it is.
   if output!=RUNTIME.resolve():shutil.copytree(RUNTIME,output,dirs_exist_ok=True,ignore=GENERATED)
   shutil.copytree(HERE/'textures',output/'textures',dirs_exist_ok=True)
   shutil.copytree(HERE/'sounds',output/'sounds',dirs_exist_ok=True)
   result=call('export',engine=engine,path=str(output/'maps') if engine=='godot' else str(output),name='lantern_lane',scale=32,options={'project_dir':str(output),'texture_dir':'res://textures' if engine=='godot' else 'textures'})
   if engine=='three.js' and args.output.resolve()==HERE:
    shutil.copy2(output/'lantern_lane.html',HERE/'play.html')
   (output/'export-report.json').write_text(json.dumps(result,indent=2)+'\n');print(engine,result)
 finally:proc.stdin.close();proc.wait(timeout=20)
if __name__=='__main__':main()
