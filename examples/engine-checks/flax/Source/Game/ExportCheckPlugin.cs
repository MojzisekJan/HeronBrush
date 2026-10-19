#if FLAX_EDITOR
using System;
using System.IO;
using System.Linq;
using FlaxEngine;
using FlaxEditor;
using FlaxEngine.Tools;
using Newtonsoft.Json.Linq;

namespace Game
{
    public sealed class ExportCheckPlugin : EditorPlugin
    {
        private float _remaining = 10;
        private void Tick()
        {
            Editor.Windows.EditWin.Viewport.ViewPosition = new Vector3(-1000, 175, 1000);
            Editor.Windows.EditWin.Viewport.ViewDirection = new Vector3(1, 0, -1);
            _remaining -= Time.UnscaledDeltaTime;
            if (_remaining <= 0) { Scripting.Update -= Tick; Engine.RequestExit(0); }
        }
        public override void InitializeEditor()
        {
            base.InitializeEditor();
            if (Engine.CommandLine.Contains("-heroncheck"))
            {
                Run();
                if (!Engine.CommandLine.Contains("-headless")) Scripting.Update += Tick;
            }
            else if (Editor.UI.MainMenu != null) Editor.UI.AddMenuButton("Tools", "Heron Brush export check", Run);
        }
        private void Run()
        {
            try
            {
                var settings = Path.Combine(Globals.ProjectContentFolder, "GameSettings.json");
                if (!File.Exists(settings)) FlaxEditor.Editor.SaveJsonAsset(settings, new FlaxEditor.Content.Settings.GameSettings());
                string scenePath = Path.Combine(Globals.ProjectContentFolder, "ExportCheck.scene");
                if (!File.Exists(scenePath)) throw new Exception("Run export.py to prepare the test scene.");
                if (!Content.GetAssetInfo(scenePath, out var info)) throw new Exception("Scene missing from content database");
                if (Level.LoadScene(info.ID)) throw new Exception("Scene failed to load");
                if (Engine.CommandLine.Contains("-heronreload"))
                {
                    CollisionCheck.ReloadOnly();
                    return;
                }
                // This entire scene is a disposable test artifact owned by this check.
                var scene = Level.GetScene(0);
                foreach (var child in scene.Children)
                    if (child.Name == "arena" || child.Name == "Floor") FlaxEngine.Object.DestroyNow(child);
                var camera = scene.FindActor<Camera>();
                if (camera != null) FlaxEngine.Object.DestroyNow(camera);
                if (!Engine.CommandLine.Contains("-headless"))
                {
                    Editor.Windows.EditWin.Viewport.ViewPosition = new Vector3(-1000, 175, 1000);
                    Editor.Windows.EditWin.Viewport.ViewDirection = new Vector3(1, 0, -1);
                }
                string folder = Path.Combine(Globals.ProjectContentFolder, "arena/arena");
                if (!Engine.CommandLine.Contains("-heronvalidate")) HeronBrushLevel.ImportModels();
                int count = 0;
                int texturedSlots = 0;
                var manifest = JObject.Parse(File.ReadAllText(Path.Combine(Globals.ProjectContentFolder, "arena/arena.level.json")));
                foreach (var entity in manifest["entities"])
                {
                    if (!(entity["surfaces"] is JArray surfaces) || surfaces.Count == 0) continue;
                    var source = Path.Combine(folder, Path.GetFileName((string)entity["mesh"]));
                    string output = Path.ChangeExtension(source, ".flax");
                    var model = Content.Load<Model>(output);
                    if (model == null) throw new Exception("Model cannot load: " + output);
                    Debug.Log("EXPORT_CHECK model=" + Path.GetFileName(source) + " bounds=" + model.GetBox());
                    foreach (var slot in model.MaterialSlots)
                    {
                        if (slot.Material == null) throw new Exception("Missing material slot");
                        slot.Material.WaitForLoaded();
                        bool hasTexture = false;
                        foreach (var parameter in slot.Material.Parameters)
                        {

                            if (parameter.Value is Texture) hasTexture = true;
                        }
                        if (hasTexture) texturedSlots++;
                    }
                    count++;
                }
                Debug.Log("EXPORT_CHECK imported=" + count + " textured_slots=" + texturedSlots);
                int metadataCount = 0;
                int collisionCount = 0;
                var actors = HeronBrushLevel.Build((actor, placement, allActors) =>
                {
                    var data = JObject.Parse(placement.MetadataJson);
                    if ((string)data["classname"] != placement.Classname) throw new Exception("Metadata classname mismatch");
                    if (data["targets"] is JArray targets)
                        foreach (var target in targets)
                            if (!allActors.ContainsKey((string)target)) throw new Exception("Unresolved target " + target);
                    if (data["collision"] is JObject collision)
                    {
                        if (!(collision["triangles"] is JArray) && !(collision["hulls"] is JArray))
                            throw new Exception("Collision geometry missing: " + placement.Name);
                        collisionCount++;
                    }
                    metadataCount++;
                });
                if (actors.Count != manifest["entities"].Count()) throw new Exception("Actor count mismatch");
                Debug.Log("EXPORT_CHECK actors=" + actors.Count + " metadata=" + metadataCount + " collision=" + collisionCount);
                if (!Engine.CommandLine.Contains("-headless") && texturedSlots != 19)
                    throw new Exception("Expected 19 textured material slots, got " + texturedSlots);
                if (Engine.CommandLine.Contains("-heronphysics")) CollisionCheck.Run(actors);
                if (Engine.CommandLine.Contains("-heronmotion")) MotionCheck.Run(actors);
                Level.SaveScene(Level.GetScene(0));
            }
            catch (Exception error)
            {
                Debug.LogException(error);
                if (Engine.CommandLine.Contains("-heroncheck")) Engine.RequestExit(1);
            }
        }
    }
}
#endif
