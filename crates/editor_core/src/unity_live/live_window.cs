// Data-only bridge for Unity 2022.3+ / Unity 6. No script is rewritten during live edits.
using System;
using System.IO;
using System.Collections.Generic;
using UnityEngine;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine.SceneManagement;
using Object = UnityEngine.Object;
namespace HeronBrush.Live {
    public sealed class HeronBrushLiveWindow : EditorWindow {
        [Serializable] public class Manifest { public int version; public string revision, source, scene; public string[] meshes; }
        [Serializable] public class Snapshot { public int version; public string name; public Entry[] entities; }
        [Serializable] public class Entry { public string name, mesh, metadata, prefab, bodyType; public float mass; public Property[] properties; public float[] position, rotation, scale; public Lamp light; public Shape[] shapes; public bool sensor; }
        [Serializable] public class Property { public string name, kind, value; public Property[] items; }
        static object Plain(Property property) {
            var culture=System.Globalization.CultureInfo.InvariantCulture;
            switch(property.kind){
                case "bool":return bool.Parse(property.value);
                case "int":return long.Parse(property.value,culture);
                case "float":return double.Parse(property.value,culture);
                case "array":case "vec2":case "vec3":case "color":
                    var items=new object[property.items?.Length??0];for(int i=0;i<items.Length;i++)items[i]=Plain(property.items[i]);return items;
                default:return property.value;
            }
        }
        [Serializable] public class Lamp { public string kind; public float[] color; public float energy, range, cone; public bool shadows; }
        [Serializable] public class Shape { public bool convex; public float[] points; }
        [Serializable] public class Ack { public string source, revision, state, message; }
        [SerializeField] HeronBrushLiveRoot target;
        [SerializeField] bool syncing = true;
        string status = "Start the live link in Heron Brush, then attach the map.";
        string importedRevision, failedRevision;
        double nextPoll;
        static string Project => Directory.GetParent(Application.dataPath).FullName;
        static string ManifestPath => Path.Combine(Project, ".heron-brush-live.json");
        static bool Busy => File.Exists(Path.Combine(Project, ".heron-brush-live-busy"));
        [MenuItem("Window/Heron Brush/Live Link")]
        public static void Open() { GetWindow<HeronBrushLiveWindow>("Heron Brush Live"); }
        void OnEnable() { EditorApplication.update += Tick; }
        void OnDisable() { EditorApplication.update -= Tick; }
        void OnGUI() {
            EditorGUILayout.LabelField("Heron Brush → Unity", EditorStyles.boldLabel);
            EditorGUILayout.HelpBox("Unsaved edits update only the generated map branch. Keep your own objects outside it. Save the Unity scene to retain the result. Keep this window open; Play Mode pauses updates.", MessageType.Info);
            target = (HeronBrushLiveRoot)EditorGUILayout.ObjectField("Attached map", target, typeof(HeronBrushLiveRoot), true);
            syncing = EditorGUILayout.Toggle("Live updates", syncing);
            using (new EditorGUI.DisabledScope(EditorApplication.isPlayingOrWillChangePlaymode || PrefabStageUtility.GetCurrentPrefabStage() != null)) {
                if (GUILayout.Button("Attach map to active scene", GUILayout.Height(32))) Attach();
            }
            if (GUILayout.Button("Retry latest update")) { failedRevision = null; importedRevision = null; if(target) target.revision = ""; }
            EditorGUILayout.HelpBox(status, MessageType.None);
        }
        Manifest ReadManifest() {
            if (Busy || !File.Exists(ManifestPath)) return null;
            var m = JsonUtility.FromJson<Manifest>(File.ReadAllText(ManifestPath));
            if (m == null || m.version != 1 || string.IsNullOrEmpty(m.revision) || string.IsNullOrEmpty(m.source)) throw new InvalidDataException("Unsupported live manifest.");
            SafeAsset(m.scene);
            return m;
        }
        // Restrict the protocol to the dedicated generated folder, not arbitrary files.
        static string SafeAsset(string relative) {
            if (string.IsNullOrEmpty(relative) || Path.IsPathRooted(relative)) throw new InvalidDataException("Invalid live asset path.");
            string full = Path.GetFullPath(Path.Combine(Project, relative));
            string allowed = Path.GetFullPath(Path.Combine(Application.dataPath, "HeronBrushLive")) + Path.DirectorySeparatorChar;
            if (!full.StartsWith(allowed, StringComparison.Ordinal)) throw new InvalidDataException("Live asset outside Assets/HeronBrushLive.");
            return full.Substring(Project.Length + 1).Replace('\\','/');
        }
        void Attach() {
            try {
                var m = ReadManifest();
                if (m == null) { status = "Waiting for Heron Brush to publish the map."; return; }
                var scene = SceneManager.GetActiveScene();
                if (!scene.IsValid() || !scene.isLoaded || EditorSceneManager.IsPreviewScene(scene)) throw new InvalidOperationException("Open a normal scene first.");
                if (target && target.gameObject.scene != scene) target = null;
                if (!target) {
                    // Reuse only a root explicitly owned by this source, in this scene.
                    foreach (var root in scene.GetRootGameObjects()) {
                        var candidate = root.GetComponent<HeronBrushLiveRoot>();
                        if (candidate && candidate.source == m.source) { target = candidate; break; }
                    }
                }
                if (!target) {
                    var root = new GameObject("HeronBrushMap");
                    SceneManager.MoveGameObjectToScene(root, scene);
                    Undo.RegisterCreatedObjectUndo(root, "Attach Heron Brush map");
                    target = root.AddComponent<HeronBrushLiveRoot>();
                }
                target.source = m.source; target.revision = "";
                syncing = true; importedRevision = null; failedRevision = null;
                EditorSceneManager.MarkSceneDirty(target.gameObject.scene);
                status = "Attached. Waiting for imported meshes.";
            } catch (Exception e) { status = e.Message; }
        }
        void Tick() {
            if (EditorApplication.timeSinceStartup < nextPoll) return;
            nextPoll = EditorApplication.timeSinceStartup + 0.2;
            Manifest m = null;
            try {
                if (EditorApplication.isCompiling || EditorApplication.isUpdating || Busy) return;
                m = ReadManifest();
                if (m == null) { status = "Link stopped or waiting for Heron Brush."; Repaint(); return; }
                if (!syncing || EditorApplication.isPlayingOrWillChangePlaymode || PrefabStageUtility.GetCurrentPrefabStage() != null) {
                    Report(m, "waiting", "Unity live updates paused."); return;
                }
                if (!target || !target.gameObject.scene.IsValid() || !target.gameObject.scene.isLoaded || target.source != m.source) {
                    Report(m, "waiting", "Attach this map in Window > Heron Brush > Live Link."); return;
                }
                if (target.revision == m.revision) { Report(m, "updated", "Unity scene updated."); return; }
                if (failedRevision == m.revision) return;
                if (importedRevision != m.revision) {
                    // Explicit import works even when Unity has background auto-refresh disabled.
                    foreach (var mesh in m.meshes ?? Array.Empty<string>()) {
                        string asset = SafeAsset(mesh);
                        if (!AssetDatabase.LoadMainAssetAtPath(asset))
                            AssetDatabase.ImportAsset(asset, ImportAssetOptions.ForceUpdate | ImportAssetOptions.ForceSynchronousImport);
                    }
                    importedRevision = m.revision;
                    return;
                }
                var data = JsonUtility.FromJson<Snapshot>(File.ReadAllText(Path.Combine(Project, SafeAsset(m.scene))));
                if (data == null || data.version != 1 || data.entities == null) throw new InvalidDataException("Unsupported live snapshot.");
                if (Busy || ReadManifest()?.revision != m.revision) return;
                if (Apply(m, data)) Report(m, "updated", "Unity scene updated.");
            } catch (Exception e) {
                if (m != null) { failedRevision = m.revision; Report(m, "error", "Unity live update failed: " + e.Message); }
                else { status = e.Message; Repaint(); }
            }
        }
        static Vector3 V(float[] v) {
            if (v == null || v.Length != 3) throw new InvalidDataException("Invalid vector.");
            foreach(float f in v) if(float.IsNaN(f) || float.IsInfinity(f)) throw new InvalidDataException("Non-finite vector.");
            return new Vector3(v[0],v[1],v[2]);
        }
        bool Apply(Manifest manifest, Snapshot data) {
            var staging = new GameObject("Generated");
            staging.SetActive(false);
            SceneManager.MoveGameObjectToScene(staging, target.gameObject.scene);
            staging.transform.SetParent(target.transform, false);
            try {
                var namedNodes=new Dictionary<string,GameObject>();
                for (int i = 0; i < data.entities.Length; ++i) {
                    var e = data.entities[i];
                    GameObject node;
                    if(string.IsNullOrEmpty(e.prefab))node=new GameObject(e.name);
                    else {
                        var prefab=AssetDatabase.LoadAssetAtPath<GameObject>(e.prefab);
                        if(!prefab)throw new InvalidDataException("Project prefab not imported: "+e.prefab);
                        node=(GameObject)PrefabUtility.InstantiatePrefab(prefab,staging.transform);node.name=e.name;
                    }
                    namedNodes.Add(e.name,node);
                    node.transform.SetParent(staging.transform, false);
                    node.transform.localPosition = V(e.position);
                    node.transform.localRotation = Quaternion.Euler(V(e.rotation));
                    node.transform.localScale = V(e.scale);
                    node.AddComponent<HeronBrushLiveEntity>().entityJson = e.metadata;
                    if (!string.IsNullOrEmpty(e.mesh)) {
                        var meshPath = SafeAsset("Assets/HeronBrushLive/" + e.mesh);
                        var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(meshPath);
                        if (!prefab) throw new InvalidDataException("Mesh not imported: " + meshPath);
                        var placed = Object.Instantiate(prefab, node.transform);
                        placed.transform.localPosition = Vector3.zero;
                        placed.transform.localRotation = Quaternion.identity;
                        placed.transform.localScale = Vector3.one;
                    }
                    if (e.light != null && !string.IsNullOrEmpty(e.light.kind)) {
                        var l = node.AddComponent<Light>();
                        l.type = e.light.kind == "Spot" ? LightType.Spot : e.light.kind == "Directional" ? LightType.Directional : LightType.Point;
                        var c = e.light.color;
                        l.color = new Color(c[0],c[1],c[2],c[3]); l.intensity=e.light.energy;
                        l.range=e.light.range; l.spotAngle=e.light.cone;
                        l.shadows=e.light.shadows ? LightShadows.Soft : LightShadows.None;
                        l.transform.Rotate(0,180,0,Space.Self);
                    }
                    for (int j=0; j<(e.shapes?.Length ?? 0); ++j) AddCollision(node, e.shapes[j], e.sensor, "Assets/HeronBrushLive");
                    var body=node.GetComponent<Rigidbody>();
                    if(e.bodyType=="static"&&body){Object.DestroyImmediate(body);body=null;}
                    if(e.bodyType=="dynamic"||e.bodyType=="kinematic"){
                        if(!body)body=node.AddComponent<Rigidbody>();body.isKinematic=e.bodyType=="kinematic";body.useGravity=e.bodyType=="dynamic";
                    }
                    if(e.mass>0){if(!body)throw new InvalidDataException("Mass requires a Rigidbody: "+e.name);body.mass=e.mass;}
                }
                foreach(var entry in data.entities)foreach(var property in entry.properties??new Property[0])SetProperty(namedNodes[entry.name],property.name,Plain(property),namedNodes);
                if (Busy || ReadManifest()?.revision != manifest.revision) { Object.DestroyImmediate(staging); return false; }
                var previous = target.generated;
                target.generated = staging; target.revision = manifest.revision;
                staging.SetActive(true);
                if (previous) Object.DestroyImmediate(previous);
                EditorUtility.SetDirty(target);
                EditorSceneManager.MarkSceneDirty(target.gameObject.scene);
                SceneView.RepaintAll(); EditorApplication.QueuePlayerLoopUpdate();
                return true;
            } catch { Object.DestroyImmediate(staging); throw; }
        }
        // HERON_COLLISION_HELPER
        // HERON_BINDINGS_HELPER
        void Report(Manifest m, string state, string message) {
            status=message; Repaint();
            var ack=new Ack {source=m.source,revision=m.revision,state=state,message=message};
            string path=Path.Combine(Project,".heron-brush-live-status.json");
            string text=JsonUtility.ToJson(ack);
            if(File.Exists(path) && File.ReadAllText(path)==text) return;
            string temp=path+".tmp";
            File.WriteAllText(temp,text);
            if(File.Exists(path)) File.Delete(path);
            File.Move(temp,path);
        }
    }
}
