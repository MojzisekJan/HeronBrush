#if FLAX_EDITOR
using System;
using System.IO;
using System.Linq;
using System.Reflection;
using FlaxEngine;
using Newtonsoft.Json.Linq;
namespace Game {
    public sealed class VerifyLiveCustom : FlaxEditor.EditorPlugin {
        static void Check(bool ok,string message){if(!ok)throw new Exception("LIVE_CUSTOM_GAME: "+message);}
        static void Tick(HeronBrushLiveEditor editor){typeof(HeronBrushLiveEditor).GetField("_wait",BindingFlags.NonPublic|BindingFlags.Instance).SetValue(editor,0f);typeof(HeronBrushLiveEditor).GetMethod("Tick",BindingFlags.NonPublic|BindingFlags.Instance).Invoke(editor,null);}
        static Actor Root(HeronBrushLiveEditor editor)=>(Actor)typeof(HeronBrushLiveEditor).GetField("_root",BindingFlags.NonPublic|BindingFlags.Instance).GetValue(editor);
        public override void InitializeEditor(){
            base.InitializeEditor();if(!Engine.CommandLine.Split(' ').Contains("-heroncustomlive"))return;
            try{
                var scenePath=Path.Combine(Globals.ProjectContentFolder,"ExportCheck.scene");Check(Content.GetAssetInfo(scenePath,out var info),"scene asset");Check(!Level.LoadScene(info.ID),"load scene");
                foreach(var child in Level.GetScene(0).Children)FlaxEngine.Object.DestroyNow(child);
                var editor=HeronBrushLiveEditor.Instance??new HeronBrushLiveEditor();editor.Attach();Tick(editor);var root=Root(editor);Check(root,"live map root");
                var panel=Level.GetScripts<Puzzle>(root).First(p=>p.armed);var beacon=Level.GetScripts<Puzzle>(root).First(p=>!p.armed);
                Check(panel.charge==9&&beacon.charge==7&&panel.sequence.SequenceEqual(new[]{2,3,5}),"native typed properties");Check(panel.partner==beacon,"forward reference");panel.Activate();Check(beacon.charge==8,"own action");
                var body=panel.Actor as RigidBody;Check(body&&!body.IsKinematic&&Math.Abs(body.Mass-2)<.001,"native dynamic mass");
                string manifestPath=Path.Combine(Globals.ProjectFolder,".heron-brush-live.json");var manifest=JObject.Parse(File.ReadAllText(manifestPath));string snapshotPath=Path.Combine(Globals.ProjectFolder,(string)manifest["scene"]);var original=File.ReadAllText(snapshotPath);var snapshot=JObject.Parse(original);
                var entry=((JArray)snapshot["placements"]).First(p=>p["metadata"]?["properties"]?["flax_prefab"]!=null);entry["metadata"]["properties"]["flax_prefab"]["value"]="Missing.prefab";
                File.WriteAllText(snapshotPath,snapshot.ToString());manifest["revision"]="missing-prefab";File.WriteAllText(manifestPath,manifest.ToString());Tick(editor);
                Check(Root(editor)==root&&root,"failed update destroyed prior root");
                File.WriteAllText(snapshotPath,original);manifest["revision"]="retry-custom";File.WriteAllText(manifestPath,manifest.ToString());Tick(editor);Check(!root&&Root(editor),"retry did not replace old root");
                Debug.Log("HERON_FLAX_LIVE_CUSTOM_GAME_OK: own prefabs/scripts, typed properties, native references, mass, failed update preservation and retry");Engine.RequestExit(0);
            }catch(Exception error){Debug.LogException(error);Engine.RequestExit(2);}
        }
    }
}
#endif
