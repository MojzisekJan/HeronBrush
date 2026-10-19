// Optional water queries, surface animation and camera tint. Flax coordinates are cm, +Z reflected.
using System;
using System.Collections.Generic;
using FlaxEngine;
using Newtonsoft.Json;
namespace Game {
 public sealed class HeronBrushWater:Script {
  public sealed class Plane{public float x,y,z,d;}
  public sealed class Volume{public Plane[] planes;public float surface;}
  public sealed class Vertex{public float x,y,z,nx,ny,nz,u,v;}
  public sealed class Data{public Vertex[] surface_vertices;public uint[] surface_indices;public int version,priority;public Volume[] volumes;public float swim_speed,rise_speed,sink_speed,drag,opacity,fog_density,wave_height,wave_speed;public float[] color;public bool surface_material;}
  public string DataJson;Data data;
  public Data Settings{get{if(data==null){data=JsonConvert.DeserializeObject<Data>(DataJson);if(data==null||data.version!=1)throw new InvalidOperationException("Unsupported water payload");}return data;}}
  public bool Sample(Vector3 world,out float depth){depth=-1;var local=Actor.Transform.WorldToLocal(world);var p=new Vector3(local.X*.01f,local.Y*.01f,-local.Z*.01f);foreach(var v in Settings.volumes){bool inside=v.planes.Length>=4;foreach(var plane in v.planes)if(plane.x*p.X+plane.y*p.Y+plane.z*p.Z>plane.d+1e-5f){inside=false;break;}if(inside)depth=Math.Max(depth,(v.surface-p.Y)*100*Math.Abs(Actor.Scale.Y));}return depth>=0;}
  public static HeronBrushWater Sample(IEnumerable<HeronBrushWater> waters,Vector3 point,out float depth){depth=-1;HeronBrushWater best=null;foreach(var water in waters)if(water!=null&&water.Enabled&&water.Sample(point,out float d)&&(best==null||water.Settings.priority>best.Settings.priority||(water.Settings.priority==best.Settings.priority&&d>depth))){best=water;depth=d;}return best;}
  public Color Tint{get{var c=Settings.color;return new Color(c[0],c[1],c[2],Settings.opacity);}}
  sealed class Surface{public StaticModel Actor;public Model Original,Model;public Vertex[] Rest;public uint[] Indices;public Float3[] Positions,Normals;public Float2[] Uv;}
  readonly List<Surface> surfaces=new List<Surface>();
  public override void OnStart(){BuildSurfaces();}
  public void BuildSurfaces(){
   if(!Settings.surface_material||surfaces.Count>0)return;
   var models=new List<StaticModel>();if(Actor is StaticModel own)models.Add(own);models.AddRange(Actor.GetChildren<StaticModel>());
   foreach(var actor in models){var source=actor.Model;if(source==null||source.WaitForLoaded()||source.LODs.Length==0)continue;
    if(Settings.surface_vertices==null||Settings.surface_vertices.Length<3||Settings.surface_indices==null||Settings.surface_indices.Length<3)continue;
    var virtualModel=Content.CreateVirtualAsset<Model>();virtualModel.SetupLODs(new[]{1});virtualModel.SetupMaterialSlots(1);virtualModel.MaterialSlots[0].Material=source.MaterialSlots[0].Material;
    var surface=new Surface{Actor=actor,Original=source,Model=virtualModel,Rest=Settings.surface_vertices,Indices=Settings.surface_indices,Positions=new Float3[Settings.surface_vertices.Length],Normals=new Float3[Settings.surface_vertices.Length],Uv=new Float2[Settings.surface_vertices.Length]};
    surfaces.Add(surface);UpdateSurface(surface,0);actor.Model=virtualModel;
   }
  }
  void UpdateSurface(Surface surface,float time){var rest=surface.Rest;var positions=surface.Positions;var normals=surface.Normals;var uv=surface.Uv;for(int j=0;j<rest.Length;j++){var v=rest[j];var p=new Float3(v.x*100,v.y*100,-v.z*100);float t=time*Settings.wave_speed;if(v.ny>.5f)p.Y+=Settings.wave_height*100*(float)(Math.Sin(v.x*1.7f+t)*Math.Cos(v.z*1.3f-t*.8f));positions[j]=p;normals[j]=new Float3(v.nx,v.ny,-v.nz);uv[j]=new Float2(v.u,v.v);}surface.Model.LODs[0].Meshes[0].UpdateMesh(positions,surface.Indices,normals,null,uv,null);}
  public override void OnUpdate(){foreach(var surface in surfaces)UpdateSurface(surface,Time.GameTime);}
  public override void OnDestroy(){foreach(var surface in surfaces){if(surface.Actor!=null)surface.Actor.Model=surface.Original;Destroy(surface.Model);}surfaces.Clear();}
 }
 public sealed class HeronBrushWaterView:Script {
  public HeronBrushWater[] Waters=Array.Empty<HeronBrushWater>();UICanvas canvas;FlaxEngine.GUI.Image image;
  public bool Underwater{get;private set;}
  public override void OnStart(){canvas=new UICanvas{Parent=Actor,RenderMode=CanvasRenderMode.ScreenSpace};image=new FlaxEngine.GUI.Image{Parent=canvas.GUI,Brush=new FlaxEngine.GUI.SolidColorBrush(Color.White),AutoFocus=false};image.SetAnchorPreset(FlaxEngine.GUI.AnchorPresets.StretchAll,true,true);image.Visible=false;}
  public override void OnUpdate(){var water=HeronBrushWater.Sample(Waters,Actor.Position,out float depth);Underwater=water!=null;if(image==null)return;image.Visible=Underwater;if(water!=null){var tint=water.Tint;tint.A=1-(float)Math.Exp(-water.Settings.fog_density*Math.Max(depth*.01f,1));image.Color=tint;}}
  public override void OnDestroy(){if(canvas!=null)Destroy(canvas);}
 }
}
