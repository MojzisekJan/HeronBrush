#if FLAX_EDITOR
using System;
using System.IO;
using System.Linq;
using FlaxEngine;
using FlaxEditor;
public sealed class WaterCheckPlugin:EditorPlugin {
 static void Check(bool ok,string message){if(!ok)throw new Exception("WATER_CHECK "+message);}
 public override void InitializeEditor(){base.InitializeEditor();if(!Engine.CommandLine.Contains("-heronwater"))return;try{
  if(!Content.GetAssetInfo(Path.Combine(Globals.ProjectContentFolder,"ExportCheck.scene"),out var info))throw new Exception("Missing scene");Check(!Level.LoadScene(info.ID),"load scene");HeronBrushLevel.ImportModels();var actors=HeronBrushLevel.Build();var waters=actors.Values.Select(a=>a.GetScript<HeronBrushWater>()).Where(w=>w!=null).ToArray();Check(waters.Length==2,"enabled volumes");
  foreach(var water in waters)if(water.Settings.surface_material){var model=water.Actor as StaticModel;var original=model?.Model;water.BuildSurfaces();if(model!=null)Check(model.Model!=original&&model.Model.IsVirtual,"private animated surface mesh");water.OnUpdate();}
  var camera=new EmptyActor();camera.Parent=actors.Values.First().Parent;camera.Position=new Vector3(300,-200,0);var view=camera.AddScript<HeronBrushWaterView>();view.Waters=waters;view.OnStart();view.OnUpdate();Check(view.Underwater,"underwater view");camera.Position=new Vector3(800,100,0);view.OnUpdate();Check(!view.Underwater,"dry view");FlaxEngine.Object.DestroyNow(camera);
  var point=new Vector3(0,-200,0);var wet=HeronBrushWater.Sample(waters,point,out float depth);Check(wet!=null&&wet.Settings.priority==10,"overlap priority");Check(HeronBrushWater.Sample(waters,new Vector3(0,100,0),out depth)==null,"above");Check(HeronBrushWater.Sample(waters,new Vector3(600,-200,0),out depth)==null,"outside hull");
  Check(waters.SelectMany(w=>w.Actor.Children).OfType<Collider>().All(c=>c.IsTrigger),"water sensors");
  var body=new CharacterController{Parent=actors.Values.First().Parent,Height=180,Radius=35,Center=new Vector3(0,90,0),Layer=31};
  using(var player=new HeronBrushMotionPlayer(actors,body)){
   player.Place(new Vector3(0,-300,0));for(int i=0;i<20;i++)player.Step(1f/60,Vector3.Zero,true);Check(body.Position.Y> -275,"rise");Check(player.Water!=null,"swim state");
   player.Place(new Vector3(0,-200,0));for(int i=0;i<20;i++)player.Step(1f/60,Vector3.Zero,false,true);Check(body.Position.Y < -225,"dive");
   player.Place(new Vector3(300,-200,0));for(int i=0;i<20;i++)player.Step(1f/60);Check(body.Position.Y < -210,"idle sink");
   player.Place(new Vector3(300,-200,0));for(int i=0;i<20;i++)player.Step(1f/60,Vector3.Forward);Check(Math.Abs(body.Position.Z)>30,"horizontal swimming");
   player.Place(new Vector3(800,100,0));player.Step(1f/60);Check(player.Water==null&&!player.Underwater,"dry state");
  }
  var root=actors.Values.First().Parent;var local=root.Transform.WorldToLocal(point);root.Position=new Vector3(500,300,700);root.Orientation=Quaternion.Euler(0,37,0);root.Scale=Vector3.One*1.5f;Check(HeronBrushWater.Sample(waters,root.Transform.LocalToWorld(local),out depth)!=null,"parent transform");
  Debug.Log("WATER_CHECK Flax PASS volumes=2 overlap swim rise dive sink exit parent sensors");Engine.RequestExit(0);
 }catch(Exception e){Debug.LogException(e);Engine.RequestExit(2);}}
}
#endif
