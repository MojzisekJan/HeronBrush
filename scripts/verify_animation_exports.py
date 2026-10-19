#!/usr/bin/env python3
"""Verify actual animation exports with optional Unity, Defold and Bevy runtimes.

Example: python3 scripts/verify_animation_exports.py --bevy \
  --unity /path/to/Unity --defold-home /path/to/Defold
All projects and logs are temporary. Requires offline Cargo dependencies and cc.
"""
from pathlib import Path
import argparse, json, subprocess, tempfile, zipfile
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--unity',type=Path)
parser.add_argument('--defold-home',type=Path)
parser.add_argument('--bevy',action='store_true')
args=parser.parse_args()
root=Path(__file__).resolve().parents[1]
subprocess.run(['cargo','test','-p','editor_core','--lib','--offline','exports_defold_bevy_unity','--','--nocapture'],cwd=root,check=True)
base=Path(tempfile.gettempdir())/'heron-portable-animation-check'
samples=json.loads((base/'expected.json').read_text())
entities=json.loads((base/'bevy/check.json').read_text())['entities']
action_names={}
for entity in entities:
    payload=entity.get('properties',{}).get('_heron_trigger',{}).get('value')
    if payload:
        trigger=json.loads(payload)
        if trigger.get('action') not in (None,'toggle'):
            action_names[trigger['action']]=entity['name']

# Unity editor integration test, using real imported assets and Unity transforms.
p=base/'unity/Assets/Tests/Editor';p.mkdir(parents=True,exist_ok=True)
code='''using UnityEngine; using UnityEditor; using System;\npublic static class AnimationCheck { public static void Run(){try {\nHeronBrushLevelImporter.Build();var go=GameObject.Find("func_animate");if(!go)throw new Exception("Missing animated object");var a=go.GetComponent<HeronBrushAnimation>();if(!a)throw new Exception("Missing player");if(!go.GetComponent<Rigidbody>().isKinematic)throw new Exception("Body is not kinematic");\n'''
for n,s in enumerate(samples):
 v=s['position']; code+=f'a.Play("{s["mode"]}");a.Pause();a.Seek({s["time"]});Check(go.transform.localPosition,new Vector3({-v[0]:.9f}f,{v[1]:.9f}f,{v[2]:.9f}f),"position {n}");\n'
 for axis,r in enumerate(s['axes']):
  v=[-r[0],r[1],r[2]]
  if axis==0:v=[-x for x in v]
  code+=f'Check(go.transform.localRotation*Vector3.{["right","up","forward"][axis]},new Vector3({v[0]:.9f}f,{v[1]:.9f}f,{v[2]:.9f}f),"axis {n}/{axis}");\n'
code+='''a.Stop();a.Activate();if(a.CurrentClip!="open")throw new Exception("Open activation");a.Seek(0.5);a.Activate();if(a.CurrentClip!="close")throw new Exception("Reverse activation");Check(go.transform.localPosition,new Vector3(-3,4,2.78125f),"reverse continuous");a.Stop();a.Activate();if(a.CurrentClip!="open")throw new Exception("Stop reset");var trigger=GameObject.Find("trigger_once").GetComponent<HeronBrushAnimationTrigger>();if(!trigger||trigger.targets.Length!=1)throw new Exception("Missing exported trigger binding");a.Stop();if(!trigger.Activate()||!a.IsPlaying||trigger.Activate())throw new Exception("Trigger once failed");a.Stop();Check(go.transform.localPosition,new Vector3(-3,4,2),"stop");Debug.Log("HERON_UNITY_ANIMATION_OK");EditorApplication.Exit(0);}catch(Exception e){Debug.LogException(e);EditorApplication.Exit(1);}}static void Check(Vector3 a,Vector3 b,string label){if(Vector3.Distance(a,b)>0.0001f)throw new Exception(label+": "+a+" != "+b);}}'''
extra=""
for action in ['play','pause','resume','stop','open','close']:
    extra+='if(!GameObject.Find('+json.dumps(action_names[action])+').GetComponent<HeronBrushAnimationTrigger>().Activate())throw new Exception("Action '+action+' not delivered");'
    if action in ('play','open','close'):
        extra+='if(a.CurrentClip!='+json.dumps('pingpong' if action=='play' else action)+')throw new Exception("Wrong clip after '+action+'");'
    if action in ('pause','stop'):
        extra+='if(a.IsPlaying)throw new Exception("Still playing after '+action+'");'
    if action=='resume':
        extra+='if(!a.IsPlaying)throw new Exception("Resume failed");'
    if action=='stop':
        extra+='Check(go.transform.localPosition,new Vector3(-3,4,2),"trigger stop");'
code=code.replace('Debug.Log("HERON_UNITY_ANIMATION_OK")',extra+'Debug.Log("HERON_UNITY_ANIMATION_OK")')
(p/'AnimationCheck.cs').write_text(code)
p=base/'unity/ProjectSettings';p.mkdir(exist_ok=True);(p/'ProjectVersion.txt').write_text('m_EditorVersion: '+(args.unity.parent.parent.name if args.unity else '6000.3.24f1')+'\n')
# Defold native message and kinematic transform test.
p=base/'defold'; (p/'game.project').write_text('''[project]\ntitle = Heron animation test\n[bootstrap]\nmain_collection = /main/main.collectionc\n[physics]\ntype = 3D\ngravity_y = 0\n[display]\nwidth = 640\nheight = 480\n[script]\nshared_state = 1\n''')
(p/'main/main.collection').write_text('''name: "main"\ncollection_instances { id: "level" collection: "/main/check.collection" }\nembedded_instances { id: "verify" data: "components { id: \\"script\\" component: \\"/main/verify.script\\" }" }\n''')
(p/'input').mkdir(exist_ok=True);(p/'input/game.input_binding').write_text('')
# Protobuf quoted newlines not required inside the tiny embedded instance.
(p/'main/main.collection').write_text('name: "main"\ncollection_instances { id: "level" collection: "/main/check.collection" }\nembedded_instances { id: "verify" data: '+json.dumps('components { id: "script" component: "/main/verify.script" }')+' }\n')
rows=[]
for s in samples:
 rows.append('{mode='+json.dumps(s['mode'])+',time='+str(s['time'])+',position={'+','.join(map(str,s['position']))+'},axes={'+','.join('{'+','.join(map(str,a))+'}' for a in s['axes'])+'}}')
lua='local samples={'+','.join(rows)+'}\n'+'''local target="/level/func_animate"\nlocal function check(v,w,label) assert(vmath.length(v-vmath.vector3(w[1],w[2],w[3]))<0.0001,label) end
function init(self) self.i=1;self.wait=0 end
function update(self,dt)
 if self.wait==0 then
  local s=samples[self.i]
  if not s then msg.post(target.."#properties","heron_stop_animation");msg.post("/level/trigger_once#properties","heron_activate");self.wait=10;return end
  msg.post(target.."#properties","heron_play_animation",{clip=s.mode});msg.post(target.."#properties","heron_pause_animation");msg.post(target.."#properties","heron_seek_animation",{time=s.time});self.wait=1
 elseif self.wait==10 then self.wait=11
 elseif self.wait==11 then self.wait=12
 elseif self.wait==12 then
  assert(vmath.length(go.get_position(target)-vmath.vector3(3,4,2))>0.00001,"trigger did not start animation")
  print("HERON_DEFOLD_ANIMATION_OK");msg.post("@system:","exit",{code=0});self.wait=99
 elseif self.wait==1 then self.wait=2
 elseif self.wait==2 then
  local s=samples[self.i];check(go.get_position(target),s.position,"position "..self.i)
  local q=go.get_rotation(target);check(vmath.rotate(q,vmath.vector3(1,0,0)),s.axes[1],"X");check(vmath.rotate(q,vmath.vector3(0,1,0)),s.axes[2],"Y");check(vmath.rotate(q,vmath.vector3(0,0,1)),s.axes[3],"Z")
  self.i=self.i+1;self.wait=0
 end
end
'''
actions='local actions={'+','.join(action+'='+json.dumps('/level/'+name+'#properties') for action,name in action_names.items())+'}\n'
expected=next(sample['position'] for sample in samples if sample['mode']=='pingpong' and sample['time']==2.5)
actions+='local named_expected={'+','.join(map(str,expected))+'}\n'
lua=actions+lua
lua=lua.replace('print("HERON_DEFOLD_ANIMATION_OK");msg.post("@system:","exit",{code=0});self.wait=99','self.wait=20')
lua=lua.replace(' elseif self.wait==1 then',''' elseif self.wait==20 then msg.post(actions.play,"heron_activate");self.wait=21
 elseif self.wait==21 then msg.post(actions.pause,"heron_activate");msg.post(target.."#properties","heron_seek_animation",{time=2.5});self.wait=22
 elseif self.wait==22 then self.wait=23
 elseif self.wait==23 then check(go.get_position(target),named_expected,"named clip trigger");self.frozen=go.get_position(target);self.wait=24
 elseif self.wait==24 then assert(vmath.length(go.get_position(target)-self.frozen)<0.00001,"pause trigger");msg.post(actions.resume,"heron_activate");self.wait=25
 elseif self.wait==25 then self.wait=26
 elseif self.wait==26 then assert(vmath.length(go.get_position(target)-self.frozen)>0.00001,"resume trigger");msg.post(actions.stop,"heron_activate");self.wait=27
 elseif self.wait==27 then self.wait=28
 elseif self.wait==28 then check(go.get_position(target),{3,4,2},"stop trigger");msg.post(actions.open,"heron_activate");self.wait=29
 elseif self.wait==29 then self.wait=30
 elseif self.wait==30 then assert(vmath.length(go.get_position(target)-vmath.vector3(3,4,2))>0.00001,"open trigger");msg.post(actions.close,"heron_activate");self.wait=31
 elseif self.wait==31 then self.wait=32
 elseif self.wait==32 then print("HERON_DEFOLD_ANIMATION_OK");msg.post("@system:","exit",{code=0});self.wait=99
 elseif self.wait==1 then''')
(p/'main/verify.script').write_text(lua)

def run(command,cwd,log,marker=None,timeout=240):
    path=base/log
    with path.open('w') as output:
        result=subprocess.run(list(map(str,command)),cwd=cwd,stdout=output,stderr=subprocess.STDOUT,timeout=timeout)
    text=path.read_text(errors='replace')
    if result.returncode or (marker and marker not in text):
        raise RuntimeError(f'Failed: {command}. See {path}\n{text[-4000:]}')
    print(f'Passed: {path}')
if args.unity:
    run([args.unity,'-batchmode','-nographics','-projectPath',base/'unity','-executeMethod','AnimationCheck.Run','-logFile','-'],root,'unity.log','HERON_UNITY_ANIMATION_OK')
if args.defold_home:
    jar=next((args.defold_home/'packages').glob('defold-*.jar'))
    java=next((args.defold_home/'packages').glob('jdk-*/bin/java'))
    run([java,'--enable-native-access=ALL-UNNAMED','-cp',jar,'com.dynamo.bob.Bob','--root',base/'defold','--max-cpu-threads','2','build'],root,'defold-build.log')
    engine=base/'dmengine'
    with zipfile.ZipFile(jar) as z: engine.write_bytes(z.read('libexec/x86_64-linux/dmengine'))
    engine.chmod(0o755)
    run([engine,'./game.projectc'],base/'defold/build/default','defold.log','HERON_DEFOLD_ANIMATION_OK',30)
if args.bevy:
    project=base/'bevy-test';(project/'src').mkdir(parents=True,exist_ok=True)
    (project/'Cargo.toml').write_text('[package]\nname="heron-bevy-animation-check"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nbevy={version="0.19.1",default-features=false,features=["std"]}\nserde={version="1",features=["derive"]}\nserde_json="1"\n')
    source='#[path="__RUNTIME__"]\nmod animation;\nuse bevy::prelude::*;\nfn main(){\n let data=std::fs::read_to_string("__FIXTURE__/data.json").unwrap();\n let samples:serde_json::Value=serde_json::from_str(&std::fs::read_to_string("__FIXTURE__/expected.json").unwrap()).unwrap();\n let mut app=App::new();app.add_plugins(MinimalPlugins).add_plugins(animation::HeronAnimationPlugin);\n let entity=app.world_mut().spawn((animation::HeronAnimation::from_json(&data).unwrap(),animation::HeronAnimationName("func_animate".into()),Transform::from_xyz(3.,4.,2.))).id();app.update();\n for s in samples.as_array().unwrap(){\n  {let mut a=app.world_mut().get_mut::<animation::HeronAnimation>(entity).unwrap();assert!(a.play(s["mode"].as_str().unwrap()));a.pause();a.seek(s["time"].as_f64().unwrap());}\n  app.update();let transform=app.world().get::<Transform>(entity).unwrap();\n  let vec=|v:&serde_json::Value|Vec3::new(v[0].as_f64().unwrap() as f32,v[1].as_f64().unwrap() as f32,v[2].as_f64().unwrap() as f32);\n  assert!(transform.translation.distance(vec(&s["position"]))<1e-4,"{s}: {:?}",transform.translation);\n  for (axis,v) in [Vec3::X,Vec3::Y,Vec3::Z].into_iter().enumerate(){assert!((transform.rotation*v).distance(vec(&s["axes"][axis]))<1e-4);}\n }\n app.world_mut().get_mut::<animation::HeronAnimation>(entity).unwrap().stop();app.update();assert_eq!(app.world().get::<Transform>(entity).unwrap().translation,Vec3::new(3.,4.,2.));let exported:serde_json::Value=serde_json::from_str(&std::fs::read_to_string("__FIXTURE__/bevy/check.json").unwrap()).unwrap();let trigger_entity=exported["entities"].as_array().unwrap().iter().find(|e|e["classname"]=="trigger_once").unwrap();let trigger_data=trigger_entity["properties"]["_heron_trigger"]["value"].as_str().unwrap();let trigger=app.world_mut().spawn((animation::HeronAnimationTrigger::from_json(trigger_data).unwrap(),GlobalTransform::from_translation(Vec3::new(1.,1.,9.)))).id();app.world_mut().spawn((animation::HeronActivator,GlobalTransform::from_translation(Vec3::new(1.,1.,9.))));app.update();assert!(app.world().get::<animation::HeronAnimation>(entity).unwrap().is_playing());app.world_mut().get_mut::<animation::HeronAnimation>(entity).unwrap().stop();app.world_mut().get_mut::<animation::HeronAnimationTrigger>(trigger).unwrap().activate();app.update();assert!(!app.world().get::<animation::HeronAnimation>(entity).unwrap().is_playing());println!("HERON_BEVY_ANIMATION_OK");\n}\n'
    extra = r'''for action in ["play","pause","resume","stop","open","close"] {
        let e=exported["entities"].as_array().unwrap().iter().find(|e|e["properties"]["animation_action"]["value"]==action).unwrap();
        let mut trigger=animation::HeronAnimationTrigger::from_json(e["properties"]["_heron_trigger"]["value"].as_str().unwrap()).unwrap();trigger.activate();
        app.world_mut().spawn((trigger,GlobalTransform::IDENTITY));app.update();
        let a=app.world().get::<animation::HeronAnimation>(entity).unwrap();
        match action {"play"=>assert_eq!(a.current_clip(),"pingpong"),"open"|"close"=>assert_eq!(a.current_clip(),action),"pause"|"stop"=>assert!(!a.is_playing()),"resume"=>assert!(a.is_playing()),_=>{}}
    }'''
    source=source.replace('println!("HERON_BEVY_ANIMATION_OK")',extra+'println!("HERON_BEVY_ANIMATION_OK")')
    (project/'src/main.rs').write_text(source.replace('__RUNTIME__',str(root/'crates/scene_export/src/writer/bevy_animation.rs.txt')).replace('__FIXTURE__',str(base)))
    run(['cargo','run','--offline'],project,'bevy.log','HERON_BEVY_ANIMATION_OK',600)
print(f'Animation verification files: {base}')
