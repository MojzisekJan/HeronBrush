#!/usr/bin/env python3
"""Rebuild the editable motion lab and its Godot export using Heron Brush MCP."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import shutil

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
PALETTE = {'floor':[34,45,61], 'white':[203,221,225], 'dark':[20,28,41],
           'teal':[40,206,181], 'orange':[255,161,65], 'blue':[91,151,255], 'pink':[235,109,158]}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,default=ROOT/'target/release/heron-brush' if (ROOT/'target/release/heron-brush').exists() else Path(shutil.which('heron-brush') or 'heron-brush'))
    parser.add_argument('--export-only',action='store_true')
    args=parser.parse_args()
    proc=subprocess.Popen([str(args.binary.resolve()),'--mcp'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    serial=0
    def rpc(method,params):
        nonlocal serial
        serial+=1
        proc.stdin.write(json.dumps(dict(jsonrpc='2.0',id=serial,method=method,params=params))+'\n');proc.stdin.flush()
        while True:
            line=proc.stdout.readline()
            if not line:raise RuntimeError('Editor MCP closed unexpectedly')
            response=json.loads(line)
            if response.get('id')!=serial:continue
            if 'error' in response or response.get('result',{}).get('isError'):raise RuntimeError(response)
            return response['result']
    def call(tool,**args):
        return '\n'.join(c.get('text','') for c in rpc('tools/call',dict(name=tool,arguments=args)).get('content',[]))
    def map_point(p):return [round(p[2]*32,6),round(p[0]*32,6),round(p[1]*32,6)]
    def box(lo,hi,color,kind='cuboid',**kw):
        call('add_shape',kind=kind,min=map_point(lo),max=map_point(hi),material='__TB_empty',**kw)
        call('set_face_color',color=PALETTE[color])
    def entity(cls):return re.search(r'\(id (\d+)\)',call('make_brush_entity',classname=cls))[1]
    def track(channel,values):return {'channel':channel,'keys':[{'time':t,'value':v,'interpolation':'linear'} for t,v in values]}
    def animation(e,duration,tracks,mode='pingpong',pivot=None):
        args=dict(entity=e,duration=duration,fps=60,mode=mode,tracks=[track(c,keys) for c,keys in tracks.items()])
        if pivot is not None:args['pivot']=map_point(pivot)
        call('set_animation',**args)
    try:
        rpc('initialize',{})
        fgd=HERE/'motion_lab.fgd'
        if not args.export_only:
            if (ROOT/'gamedata/heron_brush/heron_brush.fgd').exists():
                fgd.write_text((ROOT/'gamedata/heron_brush/heron_brush.fgd').read_text())
            call('new_map');call('load_definitions',path=str(fgd))
            box([-18,-1,-14],[18,0,14],'floor')
            # Four bays share a central walkway. No enclosing wall hides the mechanisms.
            box([-18,0,-1.5],[18,.08,1.5],'white')
            box([-.8,.08,-14],[.8,.12,14],'dark')
            for x,color in [(-14,'teal'),(-5,'teal'),(4,'orange'),(12,'orange')]:
                box([x,.085,-.12],[x+1,.1,.12],color)
            # Shuttle boarding steps and far landing.
            for i in range(4):box([-15+i*.45,0,-7.75],[-14.55+i*.45,.3*(i+1),-4.25],'white')
            box([-13.2,0,-7.75],[-11.75,1.2,-4.25],'white')
            box([-5.75,0,1.75],[-2.25,1.2,3.25],'white')
            box([-11.75,.8,-7.75],[-8.25,1.2,-4.25],'teal');shuttle=entity('func_animate')
            animation(shuttle,8,{'Y':[(0,0),(1,0),(4,192),(7,192),(8,192)],'X':[(0,0),(1,0),(4,0),(7,192),(8,192)],'Z':[(0,0),(8,0)]},pivot=[-10,1,-6])
            # The floor markings make the two straight segments explicit.
            box([-10,.01,-6.07],[-4,.025,-5.93],'teal');box([-4.07,.01,-6],[-3.93,.025,0],'teal')
            # Lift with an upper balcony and freestanding guide pillars.
            box([4.5,.2,-7.5],[7.5,.6,-4.5],'orange');lift=entity('func_animate')
            animation(lift,7,{'Z':[(0,0),(1,0),(5,192),(7,192)],'X':[(0,0),(7,0)],'Y':[(0,0),(7,0)]},pivot=[6,.4,-6])
            box([4.5,6.2,-10.5],[10.5,6.6,-7.6],'white')
            for x in [4.15,7.6]:box([x,0,-7.8],[x+.2,7,-7.6],'dark')
            for x in [4.5,10.3]:box([x,6.6,-10.5],[x+.2,7.5,-7.7],'orange')
            box([4.5,6.6,-10.5],[10.5,7.5,-10.3],'orange')
            box([9.9,0,-10],[10.3,6.2,-9.6],'dark')
            # Offset rider rotation test, visibly rectangular rather than a disk.
            box([-9,.2,5],[-5,.6,9],'blue');rotor=entity('func_animate')
            animation(rotor,12,{'RZ':[(0,0),(12,360)]},'loop',[-7,.4,7])
            # Door frame, wall returns, and a contrasting hinge side.
            for lo,hi in [([3,0,6],[6.4,4.5,6.4]),([9.6,0,6],[13,4.5,6.4]),([6.4,4,6],[9.6,4.5,6.4])]:box(lo,hi,'white')
            box([6.5,0,6],[9.5,4,6.25],'pink');door=entity('func_animate')
            call('apply_animation_recipe',entity=door,recipe='door',axis='z',amount=90,duration=2,pivot=map_point([6.5,0,6]))
            # Walk-on buttons use exactly the exported func_button target links.
            replacements={}
            for name,target,x,z,color in [('shuttle',shuttle,-14,-3,'teal'),('lift',lift,6,-3,'orange'),('rotor',rotor,-7,3,'blue'),('door',door,8,3,'pink')]:
                box([x-.6,0,z-.6],[x+.6,.15,z+.6],color)
                button=entity('func_button')
                link=json.loads(call('connect_animation_button',entity=target,button=button))
                replacements[link['targetname']]=name
            call('add_point_entity',classname='info_player_start',origin=map_point([0,1,11]))
            call('save',path=str(HERE/'motion_lab.map'))
            # Set authored startup state and stable names; MCP has no generic property setter.
            text=(HERE/'motion_lab.map').read_text()
            for old,new in replacements.items():text=text.replace('"'+old+'"','"'+new+'"')
            lines=[];depth=0;animated=-1;current=False
            for line in text.splitlines():
                if line.strip()=='{':depth+=1
                if depth==1 and line.startswith('"classname"'):
                    current=line=='"classname" "func_animate"'
                    if current:
                        animated+=1;lines.append(line);lines.append('"start_on" "'+('1' if animated in (0,2) else '0')+'"');continue
                if not(current and depth==1 and line.startswith('"start_on"')):lines.append(line)
                if line.strip()=='}':depth-=1
            (HERE/'motion_lab.map').write_text('\n'.join(lines)+'\n')
        call('open_map',path=str(HERE/'motion_lab.map'));call('load_definitions',path=str(fgd))
        result=json.loads(call('export',engine='godot',path=str(HERE/'godot/maps'),name='motion_lab',scale=32,options={'project_dir':str(HERE/'godot'),'collision':True,'cull_hidden':True,'merge_faces':True,'static_cells':False,'group_hierarchy':False,'texture_dir':'res://textures','export_on_save':False}))
        result['files']=[str(Path(file).relative_to(HERE)) for file in result['files']]
        (HERE/'export-report.json').write_text(json.dumps(result,indent=2)+'\n')
        print(f'Exported {result["triangles"]} triangles; {len(result["warnings"])} warnings')
    finally:
        proc.stdin.close();proc.wait(timeout=20)

if __name__=='__main__':main()
