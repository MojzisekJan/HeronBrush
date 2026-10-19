import subprocess,json,sys,pathlib
root=pathlib.Path(__file__).resolve().parents[3]
p=subprocess.Popen([str(root/'target/debug/heron-brush'),'--mcp'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=open('/tmp/defold-flax-mcp.stderr','w'),text=True)
n=0
transcript=[]
def call(name,args):
 global n
 n+=1
 request={'jsonrpc':'2.0','id':n,'method':'tools/call','params':{'name':name,'arguments':args}}
 p.stdin.write(json.dumps(request)+'\n');p.stdin.flush()
 while True:
  line=p.stdout.readline()
  if not line:raise RuntimeError('MCP closed')
  reply=json.loads(line)
  if reply.get('id')==n:break
 transcript.append({'request':request,'response':reply})
 if reply.get('error') or reply.get('result',{}).get('isError'):raise RuntimeError(reply)
 print(name,json.dumps(reply)[:300],flush=True)
call('open_map',{'path':str(root/'demos/bevy_fps/maps/demo.map')})
call('load_definitions',{'path':str(root/'demos/bevy_fps/gamedata/fpskit.fgd')})
call('load_materials',{'path':str(root/'demos/bevy_fps/assets/textures')})
engine='defold'; out=root/f'examples/engine-checks/{engine}'
import shutil
shutil.copytree(root/'demos/bevy_fps/assets/textures', out/'textures', dirs_exist_ok=True)
options={'project_dir':str(out),'texture_dir':'textures','name':'arena','scale':32.0,'collision':True}
if engine=='defold':options.update(model_material='/main/model.material',model_sampler='tex0',asset_dir='/main/arena')
else:options.update(content_dir='arena/arena')
call('export',{'engine':engine,'path':str(out/('main' if engine=='defold' else 'Content/arena')),'options':options})
(out/'mcp-export.json').write_text(json.dumps(transcript,indent=2)+'\n')
p.stdin.close();p.wait()

# Game-owned adapter consumes complete exported metadata after every export.
subprocess.run([sys.executable,str(out/"install_game.py")],check=True)
