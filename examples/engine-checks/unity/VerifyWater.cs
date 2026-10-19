using System;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
public static class VerifyWater {
 static void Check(bool ok,string message){if(!ok)throw new Exception("WATER_CHECK "+message);}
 public static void Run(){try{
  EditorSceneManager.NewScene(NewSceneSetup.EmptyScene,NewSceneMode.Single);HeronBrushLevelImporter.Build();Physics.SyncTransforms();
  var root=GameObject.Find("water");var waters=root.GetComponentsInChildren<HeronBrushWater>();Check(waters.Length==2,"enabled volumes");
  var point=new Vector3(0,-2,0);var wet=HeronBrushWater.Sample(waters,point,out float depth);Check(wet&&wet.Settings.priority==10,"overlap priority");
  Check(HeronBrushWater.Sample(waters,new Vector3(0,1,0),out depth)==null,"above surface");Check(HeronBrushWater.Sample(waters,new Vector3(6,-2,0),out depth)==null,"outside hull");
  Check(root.GetComponentsInChildren<Collider>().Where(c=>c.GetComponentInParent<HeronBrushWater>()).All(c=>c.isTrigger),"water collision is nonblocking");
  var bodyObject=new GameObject("water player");bodyObject.layer=31;var body=bodyObject.AddComponent<CharacterController>();body.height=1.8f;body.radius=.35f;body.center=Vector3.up*.9f;body.skinWidth=.01f;body.minMoveDistance=0;
  using(var player=new HeronBrushMotionPlayer(root.transform,body)){
   player.Place(new Vector3(0,-3,0));for(int i=0;i<20;i++)player.Step(1f/60,Vector3.zero,true);Check(body.transform.position.y> -2.75f,"rise");Check(player.Water,"swimming state");
   player.Place(new Vector3(0,-2,0));for(int i=0;i<20;i++)player.Step(1f/60,Vector3.zero,false,true);Check(body.transform.position.y < -2.25f,"dive");
   player.Place(new Vector3(-3,-2,0));for(int i=0;i<20;i++)player.Step(1f/60,Vector3.zero);Check(body.transform.position.y < -2.1f,"idle sink");
   player.Place(new Vector3(-3,-2,0));for(int i=0;i<20;i++)player.Step(1f/60,Vector3.forward);Check(body.transform.position.z>.3f,"horizontal swimming");
   player.Place(new Vector3(8,1,0));player.Step(1f/60);Check(!player.Water&&!player.Underwater,"dry state restored");
  }
  var local=root.transform.InverseTransformPoint(point);root.transform.position=new Vector3(5,3,7);root.transform.rotation=Quaternion.Euler(0,37,0);root.transform.localScale=Vector3.one*1.5f;
  Check(HeronBrushWater.Sample(waters,root.transform.TransformPoint(local),out depth)!=null,"moved rotated scaled level");root.transform.position=Vector3.zero;root.transform.rotation=Quaternion.identity;root.transform.localScale=Vector3.one;
  var cameraObject=new GameObject("water camera");cameraObject.transform.position=point;var view=cameraObject.AddComponent<HeronBrushWaterView>();view.Waters=waters;bool old=RenderSettings.fog;view.SendMessage("LateUpdate");Check(RenderSettings.fog,"underwater fog");cameraObject.transform.position=new Vector3(8,1,0);view.SendMessage("LateUpdate");Check(RenderSettings.fog==old,"dry fog restored");
  var shader=Shader.Find("HeronBrush/Water");Check(shader&&!ShaderUtil.ShaderHasError(shader),"water shader compiled");
  foreach(var water in waters)if(water.Settings.surface_material){water.SendMessage("Start");Check(water.GetComponentsInChildren<MeshRenderer>().All(r=>r.sharedMaterials.All(m=>m.shader==shader)),"water surface material");}
  Debug.Log("WATER_CHECK Unity PASS volumes=2 overlap swim rise dive sink exit parent shader fog");EditorApplication.Exit(0);
 }catch(Exception e){Debug.LogException(e);EditorApplication.Exit(2);}}
}
