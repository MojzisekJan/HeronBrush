#if FLAX_EDITOR
using System;
using System.IO;
using FlaxEngine;
using FlaxEditor;
using Newtonsoft.Json.Linq;
namespace Game
{
    public sealed class HeronBrushLiveEditor : EditorPlugin
    {
        public static HeronBrushLiveEditor Instance { get; private set; }
        private Actor _root;
        private bool _attached;
        private float _wait;
        private string _source, _revision;
        private FlaxEditor.GUI.ContextMenu.ContextMenuButton _attach, _stop;
        public override void InitializeEditor()
        {
            base.InitializeEditor();
            Instance = this;
            if (Editor.UI.MainMenu != null)
            {
                _attach = Editor.UI.AddMenuButton("Tools", "Heron Brush: attach live map", Attach);
                _stop = Editor.UI.AddMenuButton("Tools", "Heron Brush: stop live map", () => _attached = false);
            }
            Scripting.Update += Tick;
        }
        public override void DeinitializeEditor()
        {
            Scripting.Update -= Tick;
            if (Instance == this) Instance = null;
            _attach?.Dispose(); _stop?.Dispose();
            base.DeinitializeEditor();
        }
        public void Attach() { _attached = true; _wait = 0; _revision = null; }
        private void Report(JObject m, string state, string message)
        {
            var ack = new JObject { ["source"] = m["source"], ["revision"] = m["revision"], ["state"] = state, ["message"] = message };
            File.WriteAllText(Path.Combine(Globals.ProjectFolder, ".heron-brush-live-status.json"), ack.ToString());
        }
        private void Tick()
        {
            _wait -= Time.UnscaledDeltaTime;
            if (_wait > 0) return;
            _wait = 0.4f;

            var busy = Path.Combine(Globals.ProjectFolder, ".heron-brush-live-busy");
            var path = Path.Combine(Globals.ProjectFolder, ".heron-brush-live.json");
            if (File.Exists(busy) || !File.Exists(path)) return;
            JObject m = null;
            Actor candidate = null;
            try
            {
                m = JObject.Parse(File.ReadAllText(path));
                if ((string)m["scene"] != "Content/HeronBrushLive/preview.json") return;
                if ((int?)m["version"] != 1) throw new InvalidOperationException("Unsupported live manifest.");
                if (Editor.IsPlayMode) { Report(m, "waiting", "Flax live link paused during Play Mode."); return; }
                var scene = Level.ScenesCount > 0 ? Level.GetScene(0) : null;
                if (!_attached || scene == null) { Report(m, "waiting", "Open a Flax scene, then choose Tools > Heron Brush: attach live map."); return; }
                string source = (string)m["source"], revision = (string)m["revision"];
                if (revision == _revision && _root != null && _root.Scene == scene) return;
                // Recover only roots carrying our marker, never arbitrary actors with the same name.
                if (_root == null || _root.Scene != scene)
                {
                    _root = null; _source = null;
                    foreach (var child in scene.Children)
                    {
                        var anchor = child.GetScript<HeronBrushLiveAnchor>();
                        if (anchor != null && anchor.Source == source) { _root = child; _source = source; break; }
                    }
                }
                if (_root != null && _source != source) throw new InvalidOperationException("Another map is attached. Stop it before attaching this source.");
                HeronBrushLiveBuilder.LoadSnapshot(Path.Combine(Globals.ProjectContentFolder, "HeronBrushLive/preview.json"));
                HeronBrushLiveBuilder.ImportModels();
                var actors = HeronBrushLiveBuilder.Build();
                candidate = HeronBrushLiveBuilder.LastBuiltRoot;
                if (candidate == null) throw new InvalidOperationException("The live snapshot contains no root entity.");
                if (_root != null) { candidate.Parent = _root.Parent; candidate.LocalTransform = _root.LocalTransform; }
                var marker = candidate.AddScript<HeronBrushLiveAnchor>();
                marker.Source = source; marker.Revision = revision;
                // Importing can pump editor work. Do not commit a stale batch.
                if (File.Exists(busy) || (string)JObject.Parse(File.ReadAllText(path))["revision"] != revision)
                { FlaxEngine.Object.DestroyNow(candidate); return; }
                if (_root != null) FlaxEngine.Object.DestroyNow(_root);
                _root = candidate; candidate = null; _source = source; _revision = revision;
                Editor.Scene.MarkSceneEdited(scene);
                Report(m, "updated", "Flax scene updated");
            }
            catch (Exception e)
            {
                if (candidate != null) FlaxEngine.Object.DestroyNow(candidate);
                if (m != null) Report(m, "error", "Flax live link failed: " + e.Message);
                Debug.LogWarning("Heron Brush live: " + e.Message);
                _wait = 3.0f;
            }
        }
    }
}
#endif
