using System;
using System.IO;
using System.Linq;
using System.Reflection;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using HeronBrush.Live;
public static class VerifyLiveCustom {
    static void Check(bool ok,string message){if(!ok)throw new Exception("LIVE_CUSTOM_GAME: "+message);}
    public static void Run(){
        HeronBrushLiveWindow window=null;
        try{
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene,NewSceneMode.Single);
            var root=new GameObject("Owned live attachment").AddComponent<HeronBrushLiveRoot>();
            var manifest=JsonUtility.FromJson<HeronBrushLiveWindow.Manifest>(File.ReadAllText(Path.Combine(Application.dataPath,"../.heron-brush-live.json")));
            var data=JsonUtility.FromJson<HeronBrushLiveWindow.Snapshot>(File.ReadAllText(Path.Combine(Application.dataPath,"../",manifest.scene)));
            root.source=manifest.source;
            window=ScriptableObject.CreateInstance<HeronBrushLiveWindow>();
            typeof(HeronBrushLiveWindow).GetField("target",BindingFlags.NonPublic|BindingFlags.Instance).SetValue(window,root);
            var apply=typeof(HeronBrushLiveWindow).GetMethod("Apply",BindingFlags.NonPublic|BindingFlags.Instance);
            Check((bool)apply.Invoke(window,new object[]{manifest,data}),"apply live snapshot");
            var panel=root.generated.GetComponentsInChildren<Puzzle>(true).First(p=>p.armed);
            var beacon=root.generated.GetComponentsInChildren<Puzzle>(true).First(p=>!p.armed);
            Check(panel.charge==9&&beacon.charge==7&&panel.sequence.SequenceEqual(new[]{2,3,5}),"native typed values");
            Check(panel.partner==beacon,"forward native reference");panel.Activate();Check(beacon.charge==8,"own gameplay");
            Check(PrefabUtility.IsPartOfPrefabInstance(panel),"native prefab instance");
            var body=panel.GetComponent<Rigidbody>();Check(body&&!body.isKinematic&&Math.Abs(body.mass-2)<.001,"dynamic body hint");
            var previous=root.generated;
            var entry=data.entities.First(e=>!string.IsNullOrEmpty(e.prefab));var prefab=entry.prefab;entry.prefab="Assets/Missing.prefab";
            try{apply.Invoke(window,new object[]{manifest,data});throw new Exception("missing prefab accepted");}catch(TargetInvocationException){Check(root.generated==previous&&previous,"failed update removed prior branch");}
            entry.prefab=prefab;Check((bool)apply.Invoke(window,new object[]{manifest,data}),"retry update");Check(previous==null,"old generated branch kept");
            Debug.Log("HERON_UNITY_LIVE_CUSTOM_GAME_OK: native prefab, typed fields/arrays, forward reference, dynamic body, failed update preservation and retry");EditorApplication.Exit(0);
        }catch(Exception error){Debug.LogException(error);EditorApplication.Exit(2);}
        finally{if(window)UnityEngine.Object.DestroyImmediate(window);}
    }
}
