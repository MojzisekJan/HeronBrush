#if FLAX_EDITOR
using System;
using System.IO;
using System.Linq;
using System.Collections.Generic;
using FlaxEngine;
namespace Game {
    public sealed class VerifyCustom : FlaxEditor.EditorPlugin {
        static void Check(bool ok,string message){if(!ok)throw new Exception("CUSTOM_GAME: "+message);}
        static Type Importer(string folder){
            var text=File.ReadAllText(Path.Combine(Globals.ProjectContentFolder,folder,"HeronBrushLevel.cs"));
            var line=text.Split('\n').First(value=>value.StartsWith("namespace Game.HeronLevel_"));
            var name=line.Substring("namespace ".Length).Trim()+".HeronBrushLevel";
            return AppDomain.CurrentDomain.GetAssemblies().Select(a=>a.GetType(name)).First(t=>t!=null);
        }
        static Dictionary<string,Actor> Build(Type type,string method="Rebuild")=> (Dictionary<string,Actor>)type.GetMethod(method).Invoke(null,method=="Build"?new object[]{null,true,null}:new object[]{null});
        static void Prefab(Actor actor,string name){
            actor.AddScript<Puzzle>();Check(!Level.SpawnActor(actor),"spawn prefab template");
            Check(!PrefabManager.CreatePrefab(actor,Path.Combine(Globals.ProjectContentFolder,name),false),"create prefab");FlaxEngine.Object.DestroyNow(actor);
        }
        public override void InitializeEditor(){
            base.InitializeEditor();if(!Engine.CommandLine.Split(' ').Contains("-heroncustom"))return;
            try{
                var path=Path.Combine(Globals.ProjectContentFolder,"ExportCheck.scene");Check(Content.GetAssetInfo(path,out var info),"scene asset");Check(!Level.LoadScene(info.ID),"load scene");
                Prefab(new RigidBody{EnableGravity=false},"Relic.prefab");Prefab(new EmptyActor(),"Beacon.prefab");
                var unrelated=new EmptyActor{Name="workshop"};Check(!Level.SpawnActor(unrelated),"spawn unrelated");
                var importer=Importer("MapA");var actors=Build(importer);var root=actors.Values.First().Parent;
                var panel=actors.Values.Select(actor=>actor.GetScript<Puzzle>()).First(script=>script!=null&&script.armed);
                var beacon=actors["beacon"].GetScript<Puzzle>();
                Check(panel.charge==9&&panel.sequence.SequenceEqual(new[]{2,3,5})&&beacon.charge==7,"typed defaults/authored override");
                Check(panel.partner==beacon,"forward script reference");panel.Activate();Check(beacon.charge==8,"own gameplay action");
                var body=panel.Actor as RigidBody;Check(body!=null&&!body.IsKinematic&&Math.Abs(body.Mass-2)<.001,"native rigidbody mass");
                Physics.FlushRequests();for(int i=0;i<240;i++){Physics.Simulate(1f/60);Physics.CollectResults();}
                Check(Math.Abs(body.Position.Y-50)<8,"native body did not settle: "+body.Position.Y);
                var parent=new EmptyActor{Name="Project placement"};Check(!Level.SpawnActor(parent),"spawn parent");root.Parent=parent;root.LocalPosition=new Vector3(1000,200,300);root.LocalScale=Vector3.One*1.2f;
                var previous=root;actors=Build(importer);root=actors.Values.First().Parent;
                Check(!previous&&root.Parent==parent&&Vector3.Distance(root.LocalPosition,new Vector3(1000,200,300))<.01,"reexport placement/ownership");Check(unrelated,"same-name unrelated root deleted");
                var second=Importer("MapB");Check(second!=importer,"duplicate namespace");var other=Build(second).Values.First().Parent;Check(root&&other&&root!=other,"second map replaced first");
                var scene=Level.GetScene(0);var id=scene.ID;Check(!Level.SaveScene(scene),"save scene");Check(!Level.UnloadScene(scene),"unload scene");Check(!Level.LoadScene(id),"reload scene");
                actors=Build(importer);Check(actors.Values.First().Parent.Parent.Name=="Project placement","persisted root ownership");
                Debug.Log("HERON_FLAX_CUSTOM_GAME_OK: native prefabs, typed scripts, references, dynamic hull, two levels and persisted reexport ownership");Engine.RequestExit(0);
            }catch(Exception error){Debug.LogException(error);Engine.RequestExit(2);}
        }
    }
}
#endif
