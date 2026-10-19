using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using HeronBrush.Demo;
public static class HeronDemoSetup {
    [MenuItem("Heron Brush/Demo/Create playable scene")]
    public static void Create() {
        var file=EditorUtility.OpenFilePanel("Select exported .gameplay.json or live.json",Application.dataPath,"json");
        if(string.IsNullOrEmpty(file))return;
        var data=AssetDatabase.LoadAssetAtPath<TextAsset>(FileUtil.GetProjectRelativePath(file));
        if(!data){EditorUtility.DisplayDialog("Heron Brush","Select a JSON asset inside this Unity project.","OK");return;}
        var root=Selection.activeTransform;
        if(!root){EditorUtility.DisplayDialog("Heron Brush","Select the imported map root, or the Generated child for a live map.","OK");return;}
        Install(root,data);
    }
    public static HeronDemoWorld Install(Transform root,TextAsset data) {
        var existing=Object.FindFirstObjectByType<HeronDemoWorld>();
        if(existing)throw new System.InvalidOperationException("This scene already has a demo controller.");
        var controller=new GameObject("Heron Brush FPS demo");
        Undo.RegisterCreatedObjectUndo(controller,"Create playable Heron Brush scene");
        var world=controller.AddComponent<HeronDemoWorld>();world.mapRoot=root;world.definition=data;
        if(root.name=="Generated"&&root.parent&&root.parent.GetComponents<Component>().Any(c=>c&&c.GetType().FullName=="HeronBrush.Live.HeronBrushLiveRoot"))
            world.liveAttachment=root.parent;
        EditorSceneManager.MarkSceneDirty(root.gameObject.scene);
        Selection.activeGameObject=controller;
        return world;
    }
}
