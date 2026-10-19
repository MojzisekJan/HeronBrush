#if FLAX_EDITOR
using System;
using System.IO;
using System.Linq;
using FlaxEngine;
using FlaxEditor;
using Newtonsoft.Json.Linq;
namespace Game
{
    // Runs only in a disposable copy prepared by native_flax_live_fixtures.
    public sealed class LiveCheckPlugin : EditorPlugin
    {
        private int _stage;
        private bool _published, _setup;
        private float _elapsed;
        private Actor _unrelated;
        private Guid _movedModel;
        private int _movedTriangles;
        private string _revision;
        public override void InitializeEditor()
        {
            base.InitializeEditor();
            if (!Engine.CommandLine.Contains("-heronlivecheck")) return;
            Scripting.Update += Tick;
        }
        private int Colliders(Actor a) { int n = a is MeshCollider ? 1 : 0; foreach (var c in a.Children) n += Colliders(c); return n; }
        private void Tick()
        {
            try
            {
                _elapsed += Time.UnscaledDeltaTime;
                if (_elapsed > 150) throw new Exception("Flax live check timed out");
                if (HeronBrushLiveEditor.Instance == null) return;
                if (_elapsed < 2) return;
                if (!_setup)
                {
                    Level.UnloadAllScenes();
            var scenePath = Path.Combine(Globals.ProjectContentFolder, "ExportCheck.scene");
            if (!Content.GetAssetInfo(scenePath, out var info) || Level.LoadScene(info.ID)) throw new Exception("Missing test scene");
            _unrelated = new EmptyActor { Name = "Preserve me" };
            Level.SpawnActor(_unrelated);
                    _setup=true;
                }
                if (!_published)
                {
                    var folder = Path.Combine(Globals.ProjectFolder,"LiveFixtures",_stage.ToString());
                    File.WriteAllText(Path.Combine(Globals.ProjectFolder,".heron-brush-live-busy"),"check");
                    foreach (var file in Directory.GetFiles(Path.Combine(folder,"Content"),"*",SearchOption.AllDirectories))
                    {
                        var destination=Path.Combine(Globals.ProjectFolder,file.Substring(folder.Length+1));
                        Directory.CreateDirectory(Path.GetDirectoryName(destination)); File.Copy(file,destination,true);
                    }
                    var manifest=File.ReadAllText(Path.Combine(folder,".heron-brush-live.json"));
                    _revision=(string)JObject.Parse(manifest)["revision"];
                    File.WriteAllText(Path.Combine(Globals.ProjectFolder,".heron-brush-live.json"),manifest);
                    File.Delete(Path.Combine(Globals.ProjectFolder,".heron-brush-live-busy"));
                    HeronBrushLiveEditor.Instance.Attach(); _published=true;
                }
                var ackPath=Path.Combine(Globals.ProjectFolder,".heron-brush-live-status.json");
                if (!File.Exists(ackPath)) return;
                var ack=JObject.Parse(File.ReadAllText(ackPath));
                if ((string)ack["revision"]!=_revision) return;
                if ((string)ack["state"]=="error") throw new Exception((string)ack["message"]);
                if ((string)ack["state"]!="updated") return;
                var roots=Level.GetScene(0).Children.Where(a=>a.GetScript<HeronBrushLiveAnchor>()!=null).ToArray();
                if (roots.Length!=1 || _unrelated==null) throw new Exception("Root ownership failed: stage="+_stage+" roots="+roots.Length+" unrelated="+(_unrelated!=null));
                var root=roots[0];
                var world=(StaticModel)root.Children.First(a=>a.Name=="worldspawn");
                if (Colliders(root)==0) throw new Exception("No native colliders");
                if (_stage==0) root.LocalPosition=new Vector3(300,100,200);
                else if (Vector3.Distance(root.LocalPosition,new Vector3(300,100,200))>0.01f) throw new Exception("Attachment moved");
                if (_stage==1) { _movedModel=world.Model.ID; _movedTriangles=world.Model.LODs[0].Meshes.Sum(m=>m.TriangleCount); }
                if (_stage==2 && _movedModel==world.Model.ID) throw new Exception("Deletion did not change geometry");
                if (_stage==3 && _movedTriangles!=world.Model.LODs[0].Meshes.Sum(m=>m.TriangleCount)) throw new Exception("Undo did not restore geometry");
                Debug.Log("HERON_LIVE_CHECK stage="+_stage+" actors="+root.ChildrenCount+" colliders="+Colliders(root));
                _stage++;_published=false;
                if (_stage==4) { Scripting.Update-=Tick; Debug.Log("HERON_LIVE_CHECK PASS"); Engine.RequestExit(0); }
            }
            catch(Exception e) { Debug.LogException(e); Scripting.Update-=Tick; Engine.RequestExit(2); }
        }
    }
}
#endif
