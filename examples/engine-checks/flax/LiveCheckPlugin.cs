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
        private Actor _previous;
        private int _previousChildren;
        public override void InitializeEditor()
        {
            base.InitializeEditor();
            if (!Engine.CommandLine.Contains("-heronlivecheck")) return;
            Scripting.Update += Tick;
        }
        private int Colliders(Actor a) { int n = a is MeshCollider ? 1 : 0; foreach (var c in a.Children) n += Colliders(c); return n; }
        private void CheckRejectedSnapshots()
        {
            var original=JObject.Parse(File.ReadAllText(Path.Combine(Globals.ProjectFolder,"LiveFixtures/0/Content/HeronBrushLive/preview.json")));
            var path=Path.Combine(Globals.ProjectFolder,"invalid-snapshot.json");
            for (int i=0;i<3;i++)
            {
                var snapshot=(JObject)original.DeepClone();
                var placements=(JArray)snapshot["placements"];
                if(i==0) placements[0]["position"][0]="NaN";
                if(i==1) placements.Add(placements[0].DeepClone());
                if(i==2) placements[0]["orientation"]=new JArray(0,0);
                File.WriteAllText(path,snapshot.ToString());
                bool rejected=false;
                try { HeronBrushLiveBuilder.LoadSnapshot(path); }
                catch(InvalidOperationException) { rejected=true; }
                if(!rejected)throw new Exception("Invalid snapshot accepted: "+i);
            }
            File.Delete(path);
            Debug.Log("HERON_LIVE_CHECK invalid snapshots PASS");
        }
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
                    CheckRejectedSnapshots();
                    Level.UnloadAllScenes();
            var scenePath = Path.Combine(Globals.ProjectContentFolder, "ExportCheck.scene");
            if (!Content.GetAssetInfo(scenePath, out var info) || Level.LoadScene(info.ID)) throw new Exception("Missing test scene");
            _unrelated = new EmptyActor { Name = "Preserve me" };
            Level.SpawnActor(_unrelated);
                    _setup=true;
                }
                if (!_published)
                {
                    var folder = Path.Combine(Globals.ProjectFolder,"LiveFixtures",Math.Min(_stage,3).ToString());
                    File.WriteAllText(Path.Combine(Globals.ProjectFolder,".heron-brush-live-busy"),"check");
                    foreach (var file in Directory.GetFiles(Path.Combine(folder,"Content"),"*",SearchOption.AllDirectories))
                    {
                        var destination=Path.Combine(Globals.ProjectFolder,file.Substring(folder.Length+1));
                        Directory.CreateDirectory(Path.GetDirectoryName(destination)); File.Copy(file,destination,true);
                    }
                    var manifestData=JObject.Parse(File.ReadAllText(Path.Combine(folder,".heron-brush-live.json")));
                    if (_stage >= 4) manifestData["revision"]="failure-recovery-"+_stage;
                    if (_stage == 4)
                    {
                        var snapshotPath=Path.Combine(Globals.ProjectFolder,"Content/HeronBrushLive/preview.json");
                        var snapshot=JObject.Parse(File.ReadAllText(snapshotPath));
                        snapshot["placements"][0]["metadata"]["properties"]["flax_property_Missing__Value"] =
                            new JObject { ["value"]=1 };
                        File.WriteAllText(snapshotPath,snapshot.ToString());
                    }
                    if (_stage == 6) { HeronBrushLiveEditor.Instance.Stop(); manifestData["source"]="second-map"; }
                    var manifest=manifestData.ToString();
                    _revision=(string)JObject.Parse(manifest)["revision"];
                    File.WriteAllText(Path.Combine(Globals.ProjectFolder,".heron-brush-live.json"),manifest);
                    File.Delete(Path.Combine(Globals.ProjectFolder,".heron-brush-live-busy"));
                    if (_stage == 0) HeronBrushLiveEditor.Instance.Attach();
                    _published = true;
                }
                var ackPath=Path.Combine(Globals.ProjectFolder,".heron-brush-live-status.json");
                if (!File.Exists(ackPath)) return;
                var ack=JObject.Parse(File.ReadAllText(ackPath));
                if ((string)ack["revision"]!=_revision) return;
                if (_stage == 4)
                {
                    if ((string)ack["state"] != "error") return;
                    var survivors=Level.GetScene(0).Children.Where(a=>a.GetScript<HeronBrushLiveAnchor>()!=null).ToArray();
                    if (survivors.Length!=1 || survivors[0]!=_previous || Level.GetScene(0).ChildrenCount!=_previousChildren)
                        throw new Exception("Failed build leaked actors or replaced previous map");
                    Debug.Log("HERON_LIVE_CHECK rollback PASS");
                    _stage++; _published=false; return;
                }
                if ((string)ack["state"]=="error") throw new Exception((string)ack["message"]);
                if ((string)ack["state"]!="updated") return;
                var roots=Level.GetScene(0).Children.Where(a=>a.GetScript<HeronBrushLiveAnchor>()!=null).ToArray();
                if (roots.Length!=(_stage==6?2:1) || _unrelated==null) throw new Exception("Root ownership failed: stage="+_stage+" roots="+roots.Length+" unrelated="+(_unrelated!=null));
                var root=roots.First(a=>a.GetScript<HeronBrushLiveAnchor>().Source==(_stage==6?"second-map":"native-flax-demo"));
                if (_stage==5 && root.IsActive) throw new Exception("Inactive attachment became active");
                if (_stage==6 && (_previous==null || _previous.IsActive)) throw new Exception("Detached map changed");
                var world=(StaticModel)root.Children.First(a=>a.Name=="worldspawn");
                // Asset headers load before streamed mesh buffers, including in Null mode.
                if (world.Model.LoadedLODs == 0 || world.Model.LODs[0].Meshes.Sum(m=>m.TriangleCount) == 0) return;
                if (Colliders(root)==0) throw new Exception("No native colliders");
                if (_stage==0) root.LocalPosition=new Vector3(300,100,200);
                else if (_stage!=6 && Vector3.Distance(root.LocalPosition,new Vector3(300,100,200))>0.01f) throw new Exception("Attachment moved");
                if (world.Model.LODs[0].Meshes.Sum(m=>m.TriangleCount)==0) return;
                if (_stage==1) { _movedModel=world.Model.ID; _movedTriangles=world.Model.LODs[0].Meshes.Sum(m=>m.TriangleCount); }
                if (_stage==2 && _movedModel==world.Model.ID) throw new Exception("Deletion did not change geometry");
                if (_stage==3 && (_movedModel!=world.Model.ID || _movedTriangles!=world.Model.LODs[0].Meshes.Sum(m=>m.TriangleCount))) throw new Exception("Undo did not restore geometry");
                if (_stage==3) { root.IsActive=false; _previous=root; _previousChildren=Level.GetScene(0).ChildrenCount; }
                if (_stage==5) _previous=root;
                if (Environment.GetEnvironmentVariable("HERON_FLAX_LIVE_MOTION") == "1")
                {
                    var snapshot = JObject.Parse(File.ReadAllText(Path.Combine(
                        Globals.ProjectContentFolder, "HeronBrushLive/preview.json")));
                    var mover = ((JArray)snapshot["placements"]).Single(p =>
                        (string)p["metadata"]["properties"]?["targetname"]?["value"] == "shuttle");
                    if (!root.Children.Any(actor => actor.Name == (string)mover["name"]))
                        throw new Exception("Animated entity missing from the scene");
                    var animation = JObject.Parse((string)mover["metadata"]["properties"]["_heron_animation"]["value"]);
                    var maximum = animation.SelectTokens("clips[*].channels[*].keys[*].value").Max(key => (double)key);
                    var expected = _stage == 0 ? 192.0 : 256.0;
                    if (Math.Abs(maximum - expected) > 0.001)
                        throw new Exception("Wrong animation payload for stage " + _stage + ": " + maximum);
                    var hasLight = root.Children.SelectMany(actor => actor.Children)
                        .OfType<PointLight>().Any(light => Math.Abs(light.Radius - 500f) < 0.1f);
                    if (hasLight != (_stage != 0))
                        throw new Exception("Point light entity did not match the snapshot");
                    Debug.Log("HERON_LIVE_MOTION stage=" + _stage + " key=" + maximum + " light=" + hasLight);
                }
                Debug.Log("HERON_LIVE_CHECK stage="+_stage+" actors="+root.ChildrenCount+" colliders="+Colliders(root));
                _stage++;_published=false;
                if (_stage==7) { Scripting.Update-=Tick; Debug.Log("HERON_LIVE_CHECK PASS"); Engine.RequestExit(0); }
            }
            catch(Exception e) { Debug.LogException(e); Scripting.Update-=Tick; Engine.RequestExit(2); }
        }
    }
}
#endif
