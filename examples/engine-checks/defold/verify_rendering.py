#!/usr/bin/env python3
"""Native pixel checks for generated Defold materials and point/spot/directional lights.
Requires the installed Defold SDK, Xephyr, xwd and Pillow. Writes only a disposable project.
"""
import argparse
import importlib.util
import os
from pathlib import Path
import queue
import re
import shutil
import subprocess
import tempfile
import threading
import time
import zipfile
from PIL import Image

HERE=Path(__file__).resolve().parent
REPO=HERE.parents[2]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path)
    parser.add_argument('--display',default=':80')
    args=parser.parse_args()
    project=args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix='heron-defold-render-'))
    if project.exists() and any(project.iterdir()): parser.error('output must be new or empty')
    project.mkdir(parents=True,exist_ok=True)
    (project/'textures').mkdir()
    for name,rgba in [('stone',(255,255,255,64)),('stone_normal',(128,128,255,255)),('stone_roughness',(255,255,255,255)),('stone_metallic',(255,255,255,255))]:
        Image.new('RGBA',(4,4),rgba).save(project/'textures'/f'{name}.png')
    subprocess.run(['cargo','test','-p','editor_core','--lib','native_render_fixture','--','--ignored','--nocapture'],cwd=REPO,env=dict(os.environ,HERON_DEFOLD_RENDER_PROJECT=str(project)),check=True)
    (project/'input').mkdir();(project/'input/game.input_binding').write_text('')
    (project/'main.collection').write_text('''name: "main"
instances { id: "level" prototype: "/heron_brush/controller.go" }
embedded_instances { id: "probe" data: "components { id: \\"check\\" component: \\"/check.script\\" }\\n" }
embedded_instances { id: "camera" data: "embedded_components { id: \\"camera\\" type: \\"camera\\" data: \\"aspect_ratio: 1.333\\\\nfov: 0.8\\\\nnear_z: 0.05\\\\nfar_z: 100.0\\\\nauto_aspect_ratio: 1\\\\n\\" }\\n" position { z: 6.0 } }
''')
    shutil.copy2(HERE/'rendering/check.script',project/'check.script')
    world=project/'heron_brush_live/preview/worldspawn.script'
    world.write_text(world.read_text()+'''\nlocal previous_init=init
function init(self) if previous_init then previous_init(self) end;msg.post("/probe#check","test_mesh",{url=msg.url(nil,go.get_id(),"mesh")}) end\n''')
    light=next((project/'heron_brush_live/preview').glob('heron_light_*.script'))
    light.write_text(light.read_text()+'''\nlocal previous_init=init
function init(self) previous_init(self);msg.post("/probe#check","test_light",{url=msg.url()}) end\n''')
    material_path=re.search(r'material: "([^"\n]+)"',(project/'heron_brush_live/preview/worldspawn.model').read_text())[1]
    material=project/material_path.lstrip('/')
    text=material.read_text()
    for name in ['sun_color','ambient_color']:
        text=re.sub(r'(name: "'+name+r'" type: CONSTANT_TYPE_USER value \{)[^}]+',r'\1x:0 y:0 z:0 w:0',text)
    material.write_text(text)
    (project/"blend.material").write_text(text.replace('tags: "model"','tags: "heron_transparent"'))
    sdk=Path(os.environ.get('DEFOLD_HOME',Path.home()/'.local/opt/Defold'))
    jar=next((sdk/'packages').glob('defold-*.jar'));java=next((sdk/'packages').glob('jdk-*/bin/java'))
    with (project/'bob.log').open('w') as log:
        result=subprocess.run([str(java),'--enable-native-access=ALL-UNNAMED','-cp',str(jar),'com.dynamo.bob.Bob','--root',str(project),'--max-cpu-threads','2','build'],stdout=log,stderr=subprocess.STDOUT)
    if result.returncode: raise RuntimeError((project/'bob.log').read_text())
    with zipfile.ZipFile(jar) as archive: (project/'dmengine').write_bytes(archive.read('libexec/x86_64-linux/dmengine'))
    (project/'dmengine').chmod(0o755)
    spec=importlib.util.spec_from_file_location('feature_lab',HERE/'run_feature_lab.py')
    feature_lab=importlib.util.module_from_spec(spec);spec.loader.exec_module(feature_lab)
    with (project/'xephyr.log').open('w') as xlog:
        nested=subprocess.Popen(['Xephyr',args.display,'-screen','320x240','-ac','-br','-noreset'],stdout=xlog,stderr=subprocess.STDOUT)
        engine=None
        try:
            time.sleep(1)
            if nested.poll() is not None: raise RuntimeError('Xephyr failed')
            engine=subprocess.Popen([str(project/'dmengine'),'./game.projectc'],cwd=project/'build/default',env=dict(os.environ,DISPLAY=args.display,DM_SERVICE_PORT='dynamic'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,bufsize=1)
            output=queue.Queue()
            def collect():
                for line in engine.stdout: output.put(line)
            threading.Thread(target=collect,daemon=True).start()
            deadline=time.monotonic()+45;lines=[];views={}
            while time.monotonic()<deadline:
                try: line=output.get(timeout=1)
                except queue.Empty:
                    if engine.poll() is not None: break
                    continue
                lines.append(line);print(line,end='',flush=True)
                match=re.search(r'HERON_RENDER_VIEW (\d+)',line)
                if match:
                    frame=project/f'view_{match[1]}.png'
                    data=subprocess.check_output(['xwd','-display',args.display,'-root','-silent'])
                    feature_lab.xwd_to_png(data,frame)
                    image=Image.open(frame).convert('RGB')
                    pixels=list(image.crop((158,118,162,122)).getdata())
                    views[int(match[1])]=tuple(sum(p[i] for p in pixels)/len(pixels) for i in range(3))
                if 'HERON_RENDER_NATIVE_OK' in line: break
            (project/'engine.log').write_text(''.join(lines))
            assert not any('ERROR:SCRIPT:' in line or 'ERROR:GRAPHICS:' in line or 'HERON_RENDER_FAIL' in line for line in lines)
            assert len(views)==11,views
            print('PIXELS',views)
            assert views[1][0]>views[1][1]+30,'red point light'
            assert views[2][1]>views[2][0]+30,'changed green point light'
            assert views[3][1]>views[4][1]+30,'spot cone direction'
            assert views[5][2]>views[5][0]+30,'blue directional light'
            assert max(views[6])<10,'disabled light'
            assert views[7][2]>views[7][0]+30,'emission without lighting'
            assert sum(views[8])>sum(views[9])+60,'metallic roughness response'
            assert views[10][0]>=18 and views[10][1]>=28,'alpha mask reveals background'
            assert views[11][2]>views[11][0]+20 and views[11][2]<views[7][2]-30,'alpha blend'
            print('HERON_RENDER_PIXELS_OK',project)
        finally:
            if engine and engine.poll() is None: engine.terminate();engine.wait(timeout=10)
            nested.terminate();nested.wait(timeout=10)
    return 0


if __name__=='__main__': raise SystemExit(main())
