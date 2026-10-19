using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
public static class VerifyCustom {
    static void Check(bool ok,string message){if(!ok)throw new Exception("CUSTOM_GAME: "+message);}
    static Type Importer(string folder){
        var text=File.ReadAllText(Path.Combine(Application.dataPath,folder,"Editor/HeronBrushLevelImporter.cs"));
        var match=System.Text.RegularExpressions.Regex.Match(text,@"namespace (HeronBrush\.Level_[a-f0-9]+)");
        Check(match.Success,"namespace missing");
        return AppDomain.CurrentDomain.GetAssemblies().Select(a=>a.GetType(match.Groups[1].Value+".HeronBrushLevelImporter")).First(t=>t!=null);
    }
    static void Build(Type type)=>type.GetMethod("Build").Invoke(null,null);
    static GameObject Root(Type type)=>Resources.FindObjectsOfTypeAll<GameObject>().First(gameObject=>gameObject.scene.IsValid()&&gameObject.GetComponents<MonoBehaviour>().Any(component=>component!=null&&component.GetType().FullName==type.Namespace+".HeronBrushLevelRoot"));
    static void Prefab(){
        var node=new GameObject("Owned puzzle prefab");node.AddComponent<Puzzle>();
        PrefabUtility.SaveAsPrefabAsset(node,"Assets/Puzzle.prefab");UnityEngine.Object.DestroyImmediate(node);AssetDatabase.Refresh();
    }
    public static void Run(){
        try{
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene,NewSceneMode.Single);Prefab();
            var unrelated=new GameObject("workshop");var importer=Importer("MapA");Build(importer);var root=Root(importer);
            Check(unrelated!=null&&unrelated!=root,"same-name unrelated object deleted");
            var panel=root.GetComponentsInChildren<Puzzle>(true).First(p=>p.armed);
            var beacon=root.GetComponentsInChildren<Puzzle>(true).First(p=>!p.armed);
            Check(beacon.charge==7,"authored typed override");
            Check(panel.charge==9&&panel.sequence.SequenceEqual(new[]{2,3,5}),"typed component properties");
            Check(panel.partner==beacon,"forward component reference");panel.Activate();Check(beacon.charge==8,"own gameplay action");
            Check(PrefabUtility.IsPartOfPrefabInstance(panel),"native prefab connection lost");
            var body=panel.GetComponent<Rigidbody>();Check(body!=null&&!body.isKinematic&&Math.Abs(body.mass-2)<.001,"native dynamic mass");
            Physics.simulationMode=SimulationMode.Script;Physics.SyncTransforms();
            for(int i=0;i<240;i++)Physics.Simulate(1f/60);
            Check(Math.Abs(panel.transform.position.y-.5)<.08,"native convex body did not settle: "+panel.transform.position.y);
            var parent=new GameObject("Project placement");root.transform.SetParent(parent.transform,false);root.transform.localPosition=new Vector3(10,2,3);root.transform.localRotation=Quaternion.Euler(0,20,0);root.transform.localScale=Vector3.one*1.2f;
            var previous=root;Build(importer);root=Root(importer);
            Check(previous==null&&root.transform.parent==parent.transform&&root.transform.localPosition==new Vector3(10,2,3)&&Vector3.Distance(root.transform.localScale,Vector3.one*1.2f)<.0001,"reexport placement/ownership");
            Check(unrelated!=null,"unrelated content lost on reexport");
            var move=AssetDatabase.MoveAsset("Assets/Puzzle.prefab","Assets/Puzzle-unavailable.prefab");Check(move=="",move);
            try{Build(importer);throw new Exception("missing prefab accepted");}catch(System.Reflection.TargetInvocationException){Check(root!=null&&Root(importer)==root,"failed preflight destroyed previous root");}
            Check(AssetDatabase.MoveAsset("Assets/Puzzle-unavailable.prefab","Assets/Puzzle.prefab")=="","restore prefab");
            var second=Importer("MapB");Check(second!=importer,"two exports shared class namespace");Build(second);var other=Root(second);
            Check(root!=null&&other!=root&&unrelated!=null,"second map replaced first");
            EditorSceneManager.SaveScene(root.scene,"Assets/CustomVerified.unity");
            EditorSceneManager.OpenScene("Assets/CustomVerified.unity");var restored=Root(importer);Build(importer);
            Check(restored==null&&Root(importer)!=null&&Root(second)!=null,"ownership lost after scene reopen");
            Debug.Log("HERON_UNITY_CUSTOM_GAME_OK: native prefabs, typed values, scoped references, dynamic convex body, transactional reexport, two levels and persisted ownership");EditorApplication.Exit(0);
        }catch(Exception error){Debug.LogException(error);EditorApplication.Exit(2);}
    }
}
