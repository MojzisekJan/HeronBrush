using System;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using HeronBrush.Demo;
[InitializeOnLoad]
public static class VerifyGameplay {
    const string Flag="HeronGameplayCheck";
    static VerifyGameplay() {
        EditorApplication.playModeStateChanged+=state=> {
            if(state!=PlayModeStateChange.EnteredPlayMode||!SessionState.GetBool(Flag,false))return;
            var probe=new GameObject("Gameplay verification").AddComponent<HeronDemoGameplayProbe>();
            probe.Completed=(ok,message)=>{SessionState.SetBool(Flag,false);Debug.Log("HERON_GAMEPLAY "+(ok?"PASS ":"FAIL ")+message);EditorApplication.Exit(ok?0:2);};
        };
    }
    public static void CreateScene() {
        EditorSceneManager.NewScene(NewSceneSetup.EmptyScene,NewSceneMode.Single);
        HeronBrushLevelImporter.Build();
        var root=GameObject.Find("preview");
        var data=AssetDatabase.LoadAssetAtPath<TextAsset>("Assets/FinalMap/preview.gameplay.json");
        if(!root||!data)throw new Exception("Export demo fixtures first");
        HeronDemoSetup.Install(root.transform,data);
        EditorSceneManager.SaveScene(UnityEngine.SceneManagement.SceneManager.GetActiveScene(),"Assets/Playable.unity");
        EditorBuildSettings.scenes=new[]{new EditorBuildSettingsScene("Assets/Playable.unity",true)};
    }
    public static void Build() {
        CreateScene();
        PlayerSettings.productName="Heron Brush Demo";PlayerSettings.companyName="Heron Brush";
        PlayerSettings.SetScriptingBackend(UnityEditor.Build.NamedBuildTarget.Standalone,ScriptingImplementation.Mono2x);
        var path=Environment.GetEnvironmentVariable("HERON_UNITY_BUILD")??"/tmp/heron-unity-player/HeronBrushDemo.x86_64";
        System.IO.Directory.CreateDirectory(System.IO.Path.GetDirectoryName(path));
        var report=BuildPipeline.BuildPlayer(new[]{"Assets/Playable.unity"},path,BuildTarget.StandaloneLinux64,BuildOptions.Development);
        if(report.summary.result!=UnityEditor.Build.Reporting.BuildResult.Succeeded)throw new Exception("Player build failed");
        Debug.Log("HERON_GAMEPLAY_BUILD PASS");
    }
    public static void Run() {
        CreateScene();SessionState.SetBool(Flag,true);
        EditorApplication.EnterPlaymode();
    }
}
