// Optional adapter over entity-local convex planes. No project player type is required.
using System;
using UnityEngine;
public sealed class HeronBrushWater:MonoBehaviour {
    [Serializable] public sealed class Plane {public float x,y,z,d;}
    [Serializable] public sealed class Volume {public Plane[] planes;public float surface;}
    [Serializable] public sealed class Data {public int version,priority;public Volume[] volumes;public float swim_speed,rise_speed,sink_speed,drag,opacity,fog_density,wave_height,wave_speed;public float[] color;public bool surface_material;}
    public string dataJson;
    public Data Settings {get{if(data==null){data=JsonUtility.FromJson<Data>(dataJson);if(data==null||data.version!=1)throw new InvalidOperationException("Unsupported water payload");}return data;}}
    Data data;Material[] generated;
    public bool Sample(Vector3 world,out float depth){depth=-1;var p=transform.InverseTransformPoint(world);foreach(var v in Settings.volumes){bool inside=v.planes.Length>=4;foreach(var plane in v.planes)if(plane.x*p.x+plane.y*p.y+plane.z*p.z>plane.d+1e-5f){inside=false;break;}if(inside)depth=Mathf.Max(depth,(v.surface-p.y)*Mathf.Abs(transform.lossyScale.y));}return depth>=0;}
    public static HeronBrushWater Sample(HeronBrushWater[] waters,Vector3 point,out float depth){depth=-1;HeronBrushWater best=null;foreach(var water in waters)if(water&&water.isActiveAndEnabled&&water.Sample(point,out float d)&&(!best||water.Settings.priority>best.Settings.priority||(water.Settings.priority==best.Settings.priority&&d>depth))){best=water;depth=d;}return best;}
    public Color Tint {get{var c=Settings.color;return new Color(c[0],c[1],c[2],Settings.opacity);}}
    void Start(){if(!Settings.surface_material)return;var shader=Shader.Find("HeronBrush/Water");if(!shader)return;var list=new System.Collections.Generic.List<Material>();foreach(var renderer in GetComponentsInChildren<MeshRenderer>()){var materials=renderer.sharedMaterials;for(int i=0;i<materials.Length;i++){var m=new Material(shader);if(materials[i]&&materials[i].HasProperty("_MainTex"))m.mainTexture=materials[i].mainTexture;m.SetColor("_Color",Tint);m.SetFloat("_WaveHeight",Settings.wave_height);m.SetFloat("_WaveSpeed",Settings.wave_speed);materials[i]=m;list.Add(m);}renderer.sharedMaterials=materials;}generated=list.ToArray();}
    void OnDestroy(){if(generated!=null)foreach(var material in generated)if(material){if(Application.isPlaying)Destroy(material);else DestroyImmediate(material);}}
}
