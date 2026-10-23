#if FLAX_EDITOR
// Feature lab runtime check for the generated Flax importer. run_feature_lab.py
// installs it next to the generated scripts and starts the editor with
// -heronfeaturelab. Each check logs "FEATURE_LAB PASS <label>" or
// "FEATURE_LAB FAIL <label>"; the run ends with "FEATURE_LAB_DONE <failures>".
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using FlaxEditor;
using FlaxEngine;
using Newtonsoft.Json.Linq;

namespace Game
{
    public sealed class FeatureLabCheck : EditorPlugin
    {
        private int _failures;

        private void Check(bool ok, string label, string detail = null)
        {
            if (!ok) _failures++;
            Debug.Log("FEATURE_LAB " + (ok ? "PASS " : "FAIL ") + label + (detail != null ? " (" + detail + ")" : ""));
        }

        // Level frame (metres, Y up) to Flax (centimetres, Z reflected).
        private static Vector3 V(double x, double y, double z) => new Vector3((float)(x * 100), (float)(y * 100), (float)(-z * 100));
        private static Vector3 V(JToken a) => V((double)a[0], (double)a[1], (double)a[2]);

        private static void Step(int count)
        {
            Physics.FlushRequests();
            for (int i = 0; i < count; i++) { Physics.Simulate(1.0f / 60); Physics.CollectResults(); }
        }

        // A 40 cm box on its own rigid body, set up as CollisionCheck.cs does.
        private RigidBody Box(Vector3 position, bool gravity)
        {
            var body = new RigidBody { Name = "Feature lab body", Position = position, EnableGravity = gravity, EnableSimulation = true };
            if (Level.SpawnActor(body)) throw new Exception("cannot spawn a rigid body");
            var box = new BoxCollider { Position = position, Size = new Vector3(40), Name = "Feature lab box" };
            if (Level.SpawnActor(box, body)) throw new Exception("cannot spawn a box collider");
            box.LocalPosition = Vector3.Zero;
            return body;
        }

        public override void InitializeEditor()
        {
            base.InitializeEditor();
            if (!Engine.CommandLine.Contains("-heronfeaturelab")) return;
            try { Run(); }
            catch (Exception error) { _failures++; Debug.LogException(error); }
            if (Engine.CommandLine.Contains("-headless"))
            {
                Debug.Log("FEATURE_LAB_DONE " + _failures);
                Engine.RequestExit(_failures == 0 ? 0 : 1);
            }
            else Scripting.Update += ShowViews;
        }

        // With a GPU: point the editor viewport at each station in turn and save what it
        // draws to HERON_FEATURE_LAB_SHOTS. Views are eye and target in the level
        // frame; the first waits longer while Flax compiles the materials.
        private static readonly (string, Vector3, Vector3)[] Views =
        {
            ("overview", V(24, 18, 6), V(1, 0, -14)),
            ("textures", V(-5, 3.2, -23.5), V(-9, 1.25, -27)),
            ("colours_front", V(-5, 3.2, -17.5), V(-9, 1.25, -21)),
            ("colours_back", V(-13, 3.2, -24.5), V(-9, 1.25, -21)),
            ("gradients_front", V(-5, 3.2, -11.5), V(-9, 1.25, -15)),
            ("gradients_back", V(-13, 3.2, -18.5), V(-9, 1.25, -15)),
            ("surface", V(-5, 3.2, -5.5), V(-9, 1.25, -9)),
            ("surface_back", V(-13, 3.2, -12.5), V(-9, 1.25, -9)),
            ("normal", V(-5, 3.2, 0.5), V(-9, 1.25, -3)),
            ("hidden_face", V(1, 1.6, -5.2), V(1, 1.25, -9)),
            ("skip_top", V(1.01, 6, -15), V(1, 1.25, -15)),
            ("hidden_top", V(1.01, 6, -9), V(1, 1.25, -9)),
            ("water", V(16, 3.5, -5), V(12, 1, -10)),
            ("models", V(9, 3, 6), V(5, 1, 6)),
        };
        private float _clock;
        private int _view = -1;
        private void ShowViews()
        {
            _clock += Time.UnscaledDeltaTime;
            const float start = 30, each = 4;
            int view = _clock < start ? 0 : (int)((_clock - start) / each);
            if (view >= Views.Length)
            {
                Scripting.Update -= ShowViews;
                Debug.Log("FEATURE_LAB_DONE " + _failures);
                Engine.RequestExit(_failures == 0 ? 0 : 1);
                return;
            }
            var viewport = Editor.Windows.EditWin.Viewport;
            viewport.ViewPosition = Views[view].Item2;
            viewport.ViewDirection = Vector3.Normalize(Views[view].Item3 - Views[view].Item2);
            // Capture the viewport once the view has had a few frames to draw.
            float shown = _clock < start ? 0 : _clock - start - view * each;
            if (view != _view && shown > each - 1.5f)
            {
                _view = view;
                var folder = Environment.GetEnvironmentVariable("HERON_FEATURE_LAB_SHOTS");
                if (folder != null) Screenshot.Capture(viewport.Task, Path.Combine(folder, Views[view].Item1 + ".png"));
                Debug.Log("FEATURE_LAB VIEW " + Views[view].Item1);
            }
        }

        private void Run()
        {
            var scenePath = Path.Combine(Globals.ProjectContentFolder, "FeatureLab.scene");
            if (!Content.GetAssetInfo(scenePath, out var info)) throw new Exception("FeatureLab.scene missing from the content database");
            if (Level.LoadScene(info.ID)) throw new Exception("FeatureLab.scene failed to load");
            HeronBrushLevel.ImportModels();
            var placements = new Dictionary<string, HeronBrushLevel.Placement>();
            var actors = HeronBrushLevel.Build((actor, placement, all) => placements[placement.Name] = placement);
            var metadata = placements.ToDictionary(p => p.Key, p => JObject.Parse(p.Value.MetadataJson));
            Check(actors.Count == 24, "importer built all 24 entities", actors.Count + " actors");

            Colliders(actors);
            Materials(actors);
            HiddenFace(actors);
            Sensors(actors);
            Animation(actors);
            Doors(actors);
            Water(actors);
            Properties(actors, metadata);
        }

        private static MeshCollider[] CollidersOf(Actor actor) => actor.Children.OfType<MeshCollider>().ToArray();

        private void Colliders(Dictionary<string, Actor> actors)
        {
            void Kind(string name, CollisionDataType type, bool trigger)
            {
                var colliders = CollidersOf(actors[name]);
                var ok = colliders.Length == 1 && colliders[0].CollisionData != null && !colliders[0].CollisionData.IsVirtual
                         && colliders[0].CollisionData.Options.Type == type && colliders[0].IsTrigger == trigger;
                Check(ok, name + " has one " + (trigger ? "trigger " : "") + type + " collider",
                      colliders.Length == 0 ? "none" : colliders[0].CollisionData?.Options.Type + ", trigger " + colliders[0].IsTrigger);
            }
            // Inline hulls for solid brush entities and volumes, triangles for the rest.
            Kind("func_wall", CollisionDataType.ConvexMesh, false);
            Kind("func_clip", CollisionDataType.ConvexMesh, false);
            Kind("func_animate", CollisionDataType.ConvexMesh, false);
            Kind("func_animate_1", CollisionDataType.ConvexMesh, false);
            Kind("trigger_multiple", CollisionDataType.ConvexMesh, true);
            Kind("func_button", CollisionDataType.ConvexMesh, true);
            Kind("water_volume", CollisionDataType.ConvexMesh, true);
            Kind("worldspawn", CollisionDataType.TriangleMesh, false);
            Kind("func_detail_5", CollisionDataType.TriangleMesh, false);
            Kind("func_detail_6", CollisionDataType.TriangleMesh, false);
            Check(actors["func_clip"].Children.OfType<StaticModel>().Count() == 0 && !(actors["func_clip"] is StaticModel),
                  "clip brush draws nothing");
            Check(!(actors["trigger_multiple"] is StaticModel), "trigger draws nothing");

            Physics.FlushRequests();
            void Top(string name, double x, double z, double top, string label)
            {
                bool hit = Physics.RayCast(V(x, 8, z), Vector3.Down, out RayCastHit h, 900, uint.MaxValue, false);
                bool ok = hit && h.Collider.Parent == actors[name] && Math.Abs(h.Point.Y - top * 100) < 0.5f;
                Check(ok, label, hit ? h.Collider.Parent.Name + " at y " + h.Point.Y / 100 : "no hit");
            }
            Top("func_wall", 1, -27, 2.25, "fl_solid convex hull, top at 2.25 m");
            Top("func_clip", 1, -21, 3.25, "fl_clip convex hull, 2 by 3 by 2 m, top at 3.25 m");
            Top("func_detail_6", 1, -9, 2.25, "fl_hidden triangle collision keeps the hidden face's neighbours, top at 2.25 m");
            Top("worldspawn", 1, -2.5, 0, "a ray through the trigger ignores it and reaches the floor");
            // The skip top has no collision triangles: the ray passes into the brush.
            bool skipHit = Physics.RayCast(V(1, 8, -15), Vector3.Down, out RayCastHit skip, 900, uint.MaxValue, false);
            Check(!skipHit || skip.Point.Y < 224, "fl_skip's top face has no collision",
                  skipHit ? skip.Collider.Parent.Name + " at y " + skip.Point.Y / 100 : "no hit");
            bool sideHit = Physics.RayCast(V(1, 1.25, -12), new Vector3(0, 0, 1), out RayCastHit side, 900, uint.MaxValue, false);
            Check(sideHit && side.Collider.Parent == actors["func_detail_5"] && Math.Abs(side.Point.Z - 1400) < 0.5f,
                  "fl_skip's side faces still collide", sideHit ? side.Collider.Parent.Name + " at z " + side.Point.Z / 100 : "no hit");
            bool hiddenHit = Physics.RayCast(V(4, 1.25, -9), new Vector3(-1, 0, 0), out RayCastHit hidden, 900, uint.MaxValue, false);
            Check(hiddenHit && hidden.Collider.Parent == actors["func_detail_6"],
                  "fl_hidden's sides collide", hiddenHit ? hidden.Collider.Parent.Name + " at x " + hidden.Point.X / 100 : "no hit");
            bool hiddenFace = Physics.RayCast(V(1, 1.25, -5), new Vector3(0, 0, 1), out RayCastHit face, 900, uint.MaxValue, false);
            Check(hiddenFace && face.Collider.Parent == actors["func_detail_6"] && Math.Abs(face.Point.Z - 800) < 0.5f,
                  "fl_hidden's hidden face still collides", hiddenFace ? face.Collider.Parent.Name + " at z " + face.Point.Z / 100 : "no hit");

            // A falling box settles on the static clip hull.
            // Rigid bodies: a 40 cm box dropped on hulls and on triangle collision rests on top.
            void Drops(string when)
            {
                foreach (var (name, at, rest) in new[] { ("func_clip", V(1, 5, -21), 345f), ("func_wall", V(1, 5, -27), 245f),
                                                          ("func_detail_6", V(1, 5, -9), 245f), ("func_animate_1", V(11, 5, -19.875), 345f) })
                {
                    var body = Box(at, true);
                    Step(180);
                    Check(Math.Abs(body.Position.Y - rest) < 3, "a dynamic box rests on " + name + when, "y " + body.Position.Y / 100);
                    FlaxEngine.Object.DestroyNow(body);
                }
            }
            Drops("");
            // Turning the level off and on again keeps its colliders in the simulation.
            var root = actors["worldspawn"].Parent;
            root.IsActive = false;
            root.IsActive = true;
            Drops(" after the level root is toggled");
        }

        // The material and every asset it references, with type names, so the log
        // shows which texture each imported material samples.
        private static List<string> Describe(MaterialBase material)
        {
            var lines = new List<string>();
            if (material == null) { lines.Add("no material"); return lines; }
            material.WaitForLoaded();
            lines.Add(Path.GetFileName(material.Path) + " (" + material.GetType().Name + ")");
            foreach (var id in material.GetReferences())
            {
                var asset = Content.LoadAsync<Asset>(id);
                if (asset == null) { lines.Add("  missing " + id); continue; }
                asset.WaitForLoaded();
                lines.Add("  uses " + asset.GetType().Name + " " + Path.GetFileName(asset.Path));
            }
            foreach (var parameter in material.Parameters)
                lines.Add("  parameter " + parameter.Name + " = " + (parameter.Value is Asset value ? Path.GetFileName(value.Path) : parameter.Value));
            return lines;
        }

        private void Materials(Dictionary<string, Actor> actors)
        {
            int textured = 0, slots = 0;
            var seen = new HashSet<string>();
            foreach (var name in new[] { "func_detail", "func_detail_1", "func_detail_2", "func_detail_3", "func_detail_4", "func_wall", "heron_model" })
            {
                var model = (actors[name] as StaticModel)?.Model;
                if (model == null) { Check(false, name + " has a model"); continue; }
                model.WaitForLoaded();
                foreach (var slot in model.MaterialSlots)
                {
                    slots++;
                    var lines = Describe(slot.Material);
                    if (lines.Any(l => l.StartsWith("  uses Texture"))) textured++;
                    if (seen.Add(slot.Material?.Path ?? "none"))
                    {
                        Debug.Log("FEATURE_LAB MATERIAL " + name + " slot " + slot.Name + ":");
                        foreach (var line in lines) Debug.Log("FEATURE_LAB MATERIAL " + line);
                    }
                }
            }
            Check(slots > 0 && textured == slots, "every brush and model material slot samples a texture", textured + " of " + slots);
            int Distinct(string name) => Describe((actors[name] as StaticModel).Model.MaterialSlots[0].Material)
                .Where(l => l.StartsWith("  uses Texture")).Distinct().Count();
            // brick_wall: colour and normal map. The model: colour, metallic-roughness,
            // normal and emissive; Flax's glTF import leaves the occlusion map out.
            Check(Distinct("func_detail_4") == 2, "brick_wall material samples its colour and normal map", Distinct("func_detail_4") + " textures");
            Check(Distinct("heron_model") == 4, "imported model material samples four of its five maps", Distinct("heron_model") + " textures");
        }

        private void HiddenFace(Dictionary<string, Actor> actors)
        {
            int Triangles(string name)
            {
                var model = (actors[name] as StaticModel).Model;
                model.WaitForLoaded();
                return model.LODs[0].Meshes.Sum(m => m.DownloadIndexBuffer().Length / 3);
            }
            Check(Triangles("func_detail_6") == 10, "fl_hidden's model has 10 triangles: the hidden face is absent", Triangles("func_detail_6") + " triangles");
            Check(Triangles("func_wall") == 12, "fl_solid's model has all 12 triangles", Triangles("func_wall") + " triangles");
            Check(Triangles("func_detail_5") == 10, "fl_skip's model has 10 triangles: the skip top is not drawn", Triangles("func_detail_5") + " triangles");
        }

        private void Sensors(Dictionary<string, Actor> actors)
        {
            foreach (var name in new[] { "trigger_multiple", "func_button", "water_volume" })
            {
                var sensor = CollidersOf(actors[name]).First();
                var centre = sensor.Transform.LocalToWorld(sensor.CollisionData.Options.Box.Center);
                var visitor = Box(centre + new Vector3(5000, 0, 0), false);
                int enters = 0, exits = 0;
                Action<PhysicsColliderActor> enter = other => enters++;
                Action<PhysicsColliderActor> exit = other => exits++;
                sensor.TriggerEnter += enter;
                sensor.TriggerExit += exit;
                Step(2);
                visitor.Position = centre;
                Step(5);
                visitor.Position = centre + new Vector3(5000, 0, 0);
                Step(5);
                sensor.TriggerEnter -= enter;
                sensor.TriggerExit -= exit;
                FlaxEngine.Object.DestroyNow(visitor);
                Check(enters > 0 && exits > 0, name + " sensor reports enter and exit", enters + "/" + exits);
            }
        }

        // Compares an actor's pose with an editor sample (positions in metres, three axes).
        private static float PoseError(Actor actor, JToken pose)
        {
            float error = (actor.Position - V(pose["position"])).Length / 100;
            for (int axis = 0; axis < 3; axis++)
            {
                var source = pose["axes"][axis];
                var sign = axis == 2 ? -1 : 1;
                var target = new Vector3((float)source[0] * sign, (float)source[1] * sign, -(float)source[2] * sign);
                var unit = axis == 0 ? Vector3.Right : axis == 1 ? Vector3.Up : new Vector3(0, 0, 1);
                error = Math.Max(error, (Vector3.Transform(unit, actor.Orientation) - target).Length);
            }
            return error;
        }

        private void Animation(Dictionary<string, Actor> actors)
        {
            var path = Environment.GetEnvironmentVariable("HERON_FEATURE_LAB_POSES");
            var poses = JObject.Parse(File.ReadAllText(path));
            var mover = actors["func_animate"];
            var animation = mover.GetScript<HeronBrushAnimation>();
            Check(animation != null, "fl_mover has HeronBrushAnimation");
            if (animation == null) return;
            Check(animation.Play(), "fl_mover plays its selected clip");
            float worst = 0;
            var frames = (JArray)poses["frames"];
            Check(PoseError(mover, frames[0]) < 1e-4f, "fl_mover starts at the editor's pose");
            // Summing 1/60 drifts from k/60 by a few ulps, which matters exactly on a held
            // key's time: at frame 180 the clock reads 2.9999999999999942 and the hold
            // still shows the 1 s key. Such frames match the editor a nanosecond earlier.
            var before = (JArray)poses["frames_before"];
            int early = 0;
            for (int frame = 1; frame <= 240; frame++)
            {
                animation.Advance(1.0 / 60);
                float error = PoseError(mover, frames[frame]);
                float earlier = PoseError(mover, before[frame]);
                if (error > 1e-4f && earlier <= 1e-4f) early++;
                worst = Math.Max(worst, Math.Min(error, earlier));
            }
            Check(worst < 1e-4f, "fl_mover playback follows 240 editor frames over 4 s",
                  "max error " + worst.ToString("E2") + ", " + early + " frames matched the pose just before a held key");
            animation.Pause();
            foreach (var sample in (JArray)poses["samples"])
            {
                animation.Seek((double)sample["time"]);
                float error = PoseError(mover, sample);
                Check(error < 1e-4f, "fl_mover pose at " + ((double)sample["time"]).ToString("0.000") + " s", "error " + error.ToString("E2"));
            }
            animation.Stop();
        }

        private void Doors(Dictionary<string, Actor> actors)
        {
            var door = actors["func_animate_1"];
            var animation = door.GetScript<HeronBrushAnimation>();
            var trigger = actors["trigger_multiple"].GetScript<HeronBrushAnimationTrigger>();
            var button = actors["func_button"].GetScript<HeronBrushAnimationTrigger>();
            Check(animation != null && trigger != null && button != null, "door, trigger and button scripts attached");
            if (animation == null || trigger == null || button == null) return;
            Check(trigger.Targets.Contains(door) && button.Targets.Contains(door), "trigger and button target fl_door");
            Vector3 Axis() => Vector3.Transform(Vector3.Right, door.Orientation);
            bool Closed() => (Axis() - Vector3.Right).Length < 1e-3f;
            // Flax reflects Z, so the level's -Z is Flax +Z.
            bool Open() => (Axis() - new Vector3(0, 0, 1)).Length < 1e-3f;
            bool DoorwayBlocked()
            {
                Physics.FlushRequests();
                return Physics.RayCast(V(11.5, 2.5, -17), new Vector3(0, 0, 1), out RayCastHit h, 600, uint.MaxValue, false) && h.Collider.Parent == door;
            }
            bool OpenSideBlocked()
            {
                Physics.FlushRequests();
                return Physics.RayCast(V(13, 2.5, -21), Vector3.Left, out RayCastHit h, 500, uint.MaxValue, false) && h.Collider.Parent == door;
            }
            Check(Closed() && DoorwayBlocked() && !OpenSideBlocked(), "fl_door starts closed across the doorway");
            var player = new Dictionary<Guid, Vector3>();
            var id = Guid.NewGuid();
            void Run(double seconds, Vector3? at)
            {
                for (int i = 0; i < (int)(seconds * 60); i++)
                {
                    player.Clear();
                    if (at.HasValue) player[id] = at.Value;
                    trigger.Advance(1.0 / 60, player);
                    button.Advance(1.0 / 60, player);
                    animation.Advance(1.0 / 60);
                }
            }
            Run(0.2, null);
            Run(1.5, V(1.5, 1.75, -2.5));
            Check(Open(), "walking into fl_trigger opens fl_door", "axis " + Axis());
            Check(OpenSideBlocked() && !DoorwayBlocked(), "door collider moved with the door");
            Run(0.5, null);
            Run(1.5, V(10.25, 0.5, -17.75));
            Check(Closed() && DoorwayBlocked(), "stepping on fl_button toggles fl_door closed", "axis " + Axis());
            Run(0.5, null);
            Run(1.5, V(10.25, 0.5, -17.75));
            Check(Open(), "stepping on fl_button again opens fl_door", "axis " + Axis());
            Run(0.5, null);
            animation.Stop();
        }

        private void Water(Dictionary<string, Actor> actors)
        {
            var water = actors["water_volume"].GetScript<HeronBrushWater>();
            Check(water != null, "fl_water has HeronBrushWater");
            if (water == null) return;
            bool inside = water.Sample(V(12, 1, -10), out float depth);
            Check(inside && Math.Abs(depth - 75) < 0.1f && Math.Abs(water.Settings.swim_speed - 2.5f) < 1e-6f,
                  "water sampled with swim_speed 2.5 and depth 0.75 m", "depth " + depth + " cm");
            Check(!water.Sample(V(12, 3, -10), out _), "above the water is dry");
            Check(water.Settings.priority == 3, "water priority 3");
        }

        private void Properties(Dictionary<string, Actor> actors, Dictionary<string, JObject> metadata)
        {
            JToken P(string name, string key) => metadata[name]["properties"]?[key];
            var probe = P("fl_probe", "charge");
            Check((string)probe["type"] == "int" && (int)probe["value"] == 7, "probe charge is the int 7, not the default 9");
            Check((string)P("fl_probe", "armed")["type"] == "bool" && (bool)P("fl_probe", "armed")["value"], "probe armed bool from the class default");
            var sequence = (JArray)P("fl_probe", "sequence")["value"];
            Check(sequence.Count == 3 && sequence.All(v => (string)v["type"] == "int") && (int)sequence[2]["value"] == 5, "probe sequence array of ints");
            Check((string)P("fl_probe", "note")["value"] == "free text, with a comma", "string with a comma survives");
            Check((string)P("light", "light_energy")["type"] == "float" && (double)P("light", "light_energy")["value"] == 2.5, "light energy float 2.5");
            Check((string)P("info_camera", "fov")["type"] == "float" && (string)P("info_camera", "current")["type"] == "int", "camera float and int");
            Check((string)P("water_volume", "swim_speed")["type"] == "float" && (string)P("water_volume", "water_priority")["type"] == "int", "water float and int");
            void Link(string name, string target, string label)
            {
                var targets = metadata[name]["targets"] as JArray;
                bool ok = targets != null && targets.Any(t => (string)t == target) && actors.ContainsKey(target);
                Check(ok, label, ok ? "at " + actors[target].Position : null);
            }
            Link("fl_probe", "info_waypoint", "probe -> fl_wp_a");
            Link("info_waypoint", "info_waypoint_1", "fl_wp_a -> fl_wp_b");
            Link("func_button", "func_animate_1", "button -> fl_door");
            Link("trigger_multiple", "func_animate_1", "trigger -> fl_door");
            Check((actors["info_waypoint"].Position - V(-5, 1, 4)).Length < 0.01f && (actors["info_waypoint_1"].Position - V(-5, 1, 8)).Length < 0.01f,
                  "waypoints at their map positions");
            var a = actors["heron_model"] as StaticModel;
            var b = actors["heron_model_1"] as StaticModel;
            Check(a != null && b != null && a.Model == b.Model, "both model copies share one imported model");
        }
    }
}
#endif
